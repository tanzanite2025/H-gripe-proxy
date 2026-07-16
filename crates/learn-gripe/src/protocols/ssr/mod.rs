//! ShadowsocksR (SSR) outbound (TCP relay).
//!
//! SSR is the legacy fork of Shadowsocks that adds three extra layers on top of
//! the raw stream:
//!
//! 1. **Stream cipher** — legacy (non-AEAD) ciphers: `aes-128-cfb`,
//!    `aes-256-cfb`, `chacha20-ietf`, `rc4-md5`, `none`. Key derivation uses
//!    the same `EVP_BytesToKey` as classic Shadowsocks; a random IV is
//!    prepended to the stream in the clear.
//!
//! 2. **Protocol** — authentication / framing layer that wraps the encrypted
//!    payload: `origin` (pass-through), `auth_aes128_sha1`, `auth_aes128_md5`,
//!    `auth_chain_a`.
//!
//! 3. **Obfuscation** — transport-level disguise: `plain` (pass-through),
//!    `http_simple` (fake HTTP GET), `tls1.2_ticket_auth` (fake TLS handshake).
//!
//! Data flow (client write):
//! ```text
//! app data → protocol.pre_encrypt(socks5_addr + data)
//!          → stream_cipher.encrypt(protocol_output)
//!          → IV ++ encrypted  (IV only for the first write)
//!          → obfs.encode(wire_data)
//!          → TCP send
//! ```
//!
//! These are intentionally weak constructions that were deliberately excluded
//! from the AEAD-only kernel. They are re-introduced here solely to enable SSR
//! interop with existing deployments.

mod cipher;
mod crypto;
mod obfs;
mod protocol;
mod stream;
mod udp;

#[cfg(test)]
mod tests;

use anyhow::{Context, Result, bail};
use tokio::net::TcpStream;

use crate::address::TargetAddr;
use crate::config::outbound_opts::ProxyEntry;
use crate::inbound::socks5;
use crate::outbound::BoxedStream;

pub use cipher::SsrCipher;
pub use obfs::SsrObfs;
pub use protocol::SsrProtocol;
pub use udp::SsrUdp;

use cipher::StreamCryptor;
use crypto::{evp_bytes_to_key, random_bytes};
use obfs::ObfsState;
use protocol::ProtocolState;
use stream::SsrStream;

// ---------------------------------------------------------------------------
// Config + connect
// ---------------------------------------------------------------------------

/// Fully-resolved ShadowsocksR outbound parameters.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SsrOutboundConfig {
    pub server: String,
    pub port: u16,
    pub cipher: SsrCipher,
    pub key: Vec<u8>,
    pub protocol: SsrProtocol,
    pub protocol_param: String,
    pub obfs: SsrObfs,
    pub obfs_param: String,
}

impl SsrOutboundConfig {
    /// Build from a parsed `ssr` proxy entry.
    pub fn from_proxy(entry: &ProxyEntry) -> Result<Self> {
        let opts = &entry.options;
        let server = opts
            .server
            .clone()
            .filter(|s| !s.is_empty())
            .context("ssr: missing server")?;
        let port = opts.port.context("ssr: missing port")?;
        let password = opts
            .password
            .as_deref()
            .filter(|s| !s.is_empty())
            .context("ssr: missing password")?;

        let cipher = match opts.cipher.as_deref() {
            Some("aes-128-cfb") => SsrCipher::Aes128Cfb,
            Some("aes-256-cfb") => SsrCipher::Aes256Cfb,
            Some("chacha20-ietf") => SsrCipher::Chacha20Ietf,
            Some("rc4-md5") => SsrCipher::Rc4Md5,
            Some("none") => SsrCipher::None,
            None | Some("") => bail!("ssr: missing cipher"),
            Some(other) => bail!(
                "ssr: cipher {other:?} not supported \
                 (use aes-128-cfb / aes-256-cfb / chacha20-ietf / rc4-md5 / none)"
            ),
        };

        let protocol = match opts.protocol.as_deref() {
            Some("origin") | None | Some("") => SsrProtocol::Origin,
            Some("auth_aes128_sha1") => SsrProtocol::AuthAes128Sha1,
            Some("auth_aes128_md5") => SsrProtocol::AuthAes128Md5,
            Some("auth_chain_a") => SsrProtocol::AuthChainA,
            Some(other) => bail!(
                "ssr: protocol {other:?} not supported \
                 (use origin / auth_aes128_sha1 / auth_aes128_md5 / auth_chain_a)"
            ),
        };

        let obfs = match opts.obfs.as_deref() {
            Some("plain") | None | Some("") => SsrObfs::Plain,
            Some("http_simple") => SsrObfs::HttpSimple,
            Some("tls1.2_ticket_auth") => SsrObfs::Tls12TicketAuth,
            Some(other) => bail!(
                "ssr: obfs {other:?} not supported \
                 (use plain / http_simple / tls1.2_ticket_auth)"
            ),
        };

        let key = evp_bytes_to_key(password.as_bytes(), cipher.key_size());

        let protocol_param = opts.protocol_param.clone().unwrap_or_default();
        let obfs_param = opts.obfs_param.clone().unwrap_or_default();

        Ok(Self {
            server,
            port,
            cipher,
            key,
            protocol,
            protocol_param,
            obfs,
            obfs_param,
        })
    }
}

/// Connect a ShadowsocksR outbound to `target` and return a relay-ready stream.
pub async fn connect(config: &SsrOutboundConfig, target: &TargetAddr) -> Result<BoxedStream> {
    let transport: BoxedStream = Box::new(
        TcpStream::connect((config.server.as_str(), config.port))
            .await
            .with_context(|| format!("ssr: connect {}:{}", config.server, config.port))?,
    );

    // Generate random client IV.
    let iv_len = config.cipher.iv_size();
    let mut client_iv = vec![0u8; iv_len];
    random_bytes(&mut client_iv);

    // Create the write-side stream cipher (encrypt).
    let write_cipher = StreamCryptor::new_encrypt(config.cipher, &config.key, &client_iv);

    // Create the protocol layer.
    let protocol = ProtocolState::new(config.protocol, &config.key, &client_iv, &config.protocol_param);

    // Create the obfuscation layer.
    let obfs = ObfsState::new(config.obfs, &config.server, config.port, &config.obfs_param);

    // Prepare the first write: IV + encrypted(protocol(socks5_addr)).
    let mut addr_buf = Vec::with_capacity(1 + 256 + 2);
    socks5::encode_address(&mut addr_buf, target);

    let stream = SsrStream::new(
        transport,
        config.cipher,
        config.key.clone(),
        write_cipher,
        client_iv,
        protocol,
        obfs,
        addr_buf,
    );

    Ok(Box::new(stream))
}
