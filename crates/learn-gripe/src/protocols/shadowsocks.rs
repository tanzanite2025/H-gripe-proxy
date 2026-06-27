//! Shadowsocks outbound (AEAD ciphers).
//!
//! Implements two generations of the Shadowsocks **AEAD** stream over plain TCP:
//! * the 2017 AEAD methods `aes-128-gcm`, `aes-256-gcm`,
//!   `chacha20-ietf-poly1305`; and
//! * the Shadowsocks 2022 methods `2022-blake3-aes-128-gcm`,
//!   `2022-blake3-aes-256-gcm`, `2022-blake3-chacha20-poly1305`.
//!
//! The legacy stream ciphers (`aes-*-cfb`, `rc4-md5`, …) use a different
//! construction and are rejected by [`ShadowsocksOutboundConfig::from_proxy`]
//! rather than silently mis-encoded.
//!
//! SIP003 `plugin` transports are supported via [`crate::protocols::ss_plugin`]:
//! the Shadowsocks stream runs over the plugin transport (simple-obfs HTTP, or
//! v2ray-plugin WebSocket optionally over TLS) instead of the raw socket. The
//! simple-obfs fake-TLS mode and v2ray-plugin's non-WebSocket modes are
//! rejected rather than mis-framed.
//!
//! UDP is supported for both generations. The 2017 methods use the stateless,
//! per-packet salted construction; the 2022 methods use the SIP022 UDP packet
//! format (AES separate-header for the AES methods, XChaCha20-Poly1305 for the
//! chacha method) implemented in [`ShadowsocksUdp`].
//!
//! All cryptographic primitives are delegated to vetted RustCrypto crates
//! (`aes-gcm`, `chacha20poly1305`, `md-5`, `sha1`, `blake3`); the only things
//! assembled here are the Shadowsocks key schedule and the on-wire framing.
//!
//! Both generations share the same body chunk framing:
//! ```text
//! salt (key_len bytes, in clear) | <header> | chunk | chunk | ...
//! chunk = AEAD(len)(2+16) | AEAD(payload)(len+16)
//! ```
//! with a 12-byte little-endian counter nonce that starts at 0 and increments
//! after every seal/open. They differ in the per-session subkey derivation and
//! the header that precedes the body:
//! * **2017**: `subkey = HKDF-SHA1(master_key, salt, "ss-subkey")`; the first
//!   plaintext bytes are the SOCKS5-format target address, then app data.
//! * **2022 (SIP022)**: `subkey = BLAKE3-derive_key("shadowsocks 2022 session
//!   subkey", PSK || salt)`; the request carries a fixed-length header
//!   (`type | timestamp | varlen`) followed by a variable-length header
//!   (`socks_addr | padding`), and the response header echoes the request salt
//!   and a freshness timestamp before the first payload chunk.

use std::io;
use std::pin::Pin;
use std::sync::atomic::{AtomicU64, Ordering};
use std::task::{Context as TaskContext, Poll, ready};
use std::time::{SystemTime, UNIX_EPOCH};

use aes::cipher::{BlockDecrypt, BlockEncrypt};
use aes::{Aes128, Aes256};
use aes_gcm::aead::generic_array::GenericArray;
use aes_gcm::aead::{Aead, KeyInit, Payload};
use aes_gcm::{Aes128Gcm, Aes256Gcm};
use anyhow::{Context, Result, anyhow, bail};
use chacha20poly1305::{ChaCha20Poly1305, XChaCha20Poly1305, XNonce};
use md5::Md5;
use sha1::{Digest, Sha1};
use tokio::io::{AsyncRead, AsyncWrite, AsyncWriteExt, ReadBuf};
use tokio::net::{TcpStream, UdpSocket, lookup_host};

use crate::address::TargetAddr;
use crate::config::outbound_opts::ProxyEntry;
use crate::inbound::socks5;
use crate::outbound::BoxedStream;
use crate::protocols::ss_plugin::SsPlugin;

/// HKDF `info` string that derives the per-session subkey from the master key.
const SS_SUBKEY_INFO: &[u8] = b"ss-subkey";
/// Largest plaintext carried in a single AEAD chunk (Shadowsocks caps the
/// length field at 0x3FFF so it never collides with the high bit).
const MAX_CHUNK: usize = 0x3fff;
/// The AEAD tag length for every supported cipher.
const TAG_LEN: usize = 16;
/// Upper bound on a received Shadowsocks UDP packet (salt + sealed payload).
const MAX_UDP_PACKET: usize = 64 * 1024;
/// BLAKE3 `derive_key` context that turns a Shadowsocks 2022 PSK + session salt
/// into the per-session AEAD subkey.
const SS2022_SUBKEY_CONTEXT: &str = "shadowsocks 2022 session subkey";
/// Shadowsocks 2022 stream header type for the client request direction.
const SS2022_HEADER_TYPE_REQUEST: u8 = 0;
/// Shadowsocks 2022 stream header type for the server response direction.
const SS2022_HEADER_TYPE_RESPONSE: u8 = 1;
/// Largest tolerated skew (seconds) between the response header timestamp and
/// the local clock; mirrors the reference implementations' replay window.
const SS2022_MAX_TIME_DIFF: u64 = 30;
/// Shadowsocks 2022 UDP main-header type for the client-to-server direction.
const SS2022_UDP_HEADER_TYPE_CLIENT: u8 = 0;
/// Shadowsocks 2022 UDP main-header type for the server-to-client direction.
const SS2022_UDP_HEADER_TYPE_SERVER: u8 = 1;
/// Length of the Shadowsocks 2022 UDP session ID (and the separate-header
/// session-ID field).
const SS2022_SESSION_ID_LEN: usize = 8;
/// Length of the AES separate header (`session ID | packet ID`), which is also
/// the AES block size.
const SS2022_SEPARATE_HEADER_LEN: usize = 16;
/// Length of the XChaCha20-Poly1305 per-packet nonce used by the Shadowsocks
/// 2022 chacha UDP construction.
const SS2022_XNONCE_LEN: usize = 24;

/// Shadowsocks AEAD method. The `Blake3*` variants are the Shadowsocks 2022
/// (SIP022) methods; the others are the 2017 AEAD methods.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShadowsocksCipher {
    Aes128Gcm,
    Aes256Gcm,
    Chacha20IetfPoly1305,
    /// `2022-blake3-aes-128-gcm`.
    Blake3Aes128Gcm,
    /// `2022-blake3-aes-256-gcm`.
    Blake3Aes256Gcm,
    /// `2022-blake3-chacha20-poly1305`.
    Blake3Chacha20Poly1305,
}

impl ShadowsocksCipher {
    /// Key length in bytes; the salt length equals the key length for every
    /// supported AEAD method.
    fn key_size(self) -> usize {
        match self {
            ShadowsocksCipher::Aes128Gcm | ShadowsocksCipher::Blake3Aes128Gcm => 16,
            ShadowsocksCipher::Aes256Gcm
            | ShadowsocksCipher::Chacha20IetfPoly1305
            | ShadowsocksCipher::Blake3Aes256Gcm
            | ShadowsocksCipher::Blake3Chacha20Poly1305 => 32,
        }
    }

    /// Whether this is a Shadowsocks 2022 (`2022-blake3-*`) method, which uses
    /// BLAKE3 key derivation and the SIP022 stream header.
    fn is_2022(self) -> bool {
        matches!(
            self,
            ShadowsocksCipher::Blake3Aes128Gcm
                | ShadowsocksCipher::Blake3Aes256Gcm
                | ShadowsocksCipher::Blake3Chacha20Poly1305
        )
    }
}

/// Fully-resolved Shadowsocks outbound parameters. For the 2017 methods the
/// password is pre-expanded into the master `key` via `EVP_BytesToKey`; for the
/// 2022 methods `key` holds the raw PSK (the Base64-decoded password). Either
/// way the dial path never touches the raw secret again.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShadowsocksOutboundConfig {
    pub server: String,
    pub port: u16,
    pub cipher: ShadowsocksCipher,
    pub key: Vec<u8>,
    /// SIP003 plugin transport the Shadowsocks stream runs over, if any
    /// (simple-obfs / v2ray-plugin). `None` dials the raw socket directly.
    pub plugin: Option<SsPlugin>,
}

impl ShadowsocksOutboundConfig {
    /// Build an outbound config from a parsed `ss` proxy entry, rejecting
    /// methods and features that are not implemented yet so traffic is never
    /// mis-framed.
    pub fn from_proxy(entry: &ProxyEntry) -> Result<Self> {
        let opts = &entry.options;
        let server = opts
            .server
            .clone()
            .filter(|s| !s.is_empty())
            .context("shadowsocks: missing server")?;
        let port = opts.port.context("shadowsocks: missing port")?;
        let password = opts
            .password
            .as_deref()
            .filter(|s| !s.is_empty())
            .context("shadowsocks: missing password")?;

        let cipher = match opts.cipher.as_deref() {
            Some("aes-128-gcm") => ShadowsocksCipher::Aes128Gcm,
            Some("aes-256-gcm") => ShadowsocksCipher::Aes256Gcm,
            Some("chacha20-ietf-poly1305") | Some("chacha20-poly1305") => ShadowsocksCipher::Chacha20IetfPoly1305,
            Some("2022-blake3-aes-128-gcm") => ShadowsocksCipher::Blake3Aes128Gcm,
            Some("2022-blake3-aes-256-gcm") => ShadowsocksCipher::Blake3Aes256Gcm,
            Some("2022-blake3-chacha20-poly1305") => ShadowsocksCipher::Blake3Chacha20Poly1305,
            None | Some("") => bail!("shadowsocks: missing cipher"),
            Some(other) => bail!(
                "shadowsocks: cipher {other:?} not supported \
                 (use aes-128-gcm / aes-256-gcm / chacha20-ietf-poly1305 / 2022-blake3-aes-128-gcm / \
                 2022-blake3-aes-256-gcm / 2022-blake3-chacha20-poly1305)"
            ),
        };

        // SIP003 plugin transport (obfs / v2ray-plugin), if configured. The
        // parser rejects unsupported plugins/modes so traffic is never
        // mis-framed.
        let plugin = SsPlugin::parse(opts.plugin.as_deref(), opts.plugin_opts.as_ref())?;

        let key = if cipher.is_2022() {
            // Shadowsocks 2022 PSK: the password is Base64-encoded raw key bytes
            // used directly (no EVP_BytesToKey expansion) and must be exactly the
            // cipher key length.
            let psk = base64_decode(password).context("shadowsocks: invalid 2022 PSK base64")?;
            if psk.len() != cipher.key_size() {
                bail!(
                    "shadowsocks: 2022 PSK must be {} bytes, got {} (check the Base64 password)",
                    cipher.key_size(),
                    psk.len()
                );
            }
            psk
        } else {
            evp_bytes_to_key(password.as_bytes(), cipher.key_size())
        };

        Ok(Self {
            server,
            port,
            cipher,
            key,
            plugin,
        })
    }
}

/// Connect a Shadowsocks outbound to `target` and return a relay-ready stream.
/// The salt and the AEAD-sealed target address are sent before the stream is
/// handed back, so reads/writes thereafter carry application data only.
pub async fn connect(config: &ShadowsocksOutboundConfig, target: &TargetAddr) -> Result<BoxedStream> {
    // The Shadowsocks stream runs over the raw socket, or, when a SIP003 plugin
    // is configured, over the plugin's transport (simple-obfs / v2ray-plugin).
    let mut transport: BoxedStream = match &config.plugin {
        None => Box::new(
            TcpStream::connect((config.server.as_str(), config.port))
                .await
                .with_context(|| format!("shadowsocks: connect {}:{}", config.server, config.port))?,
        ),
        Some(plugin) => plugin
            .connect(&config.server, config.port)
            .await
            .with_context(|| format!("shadowsocks: plugin connect {}:{}", config.server, config.port))?,
    };

    let salt_len = config.cipher.key_size();
    let mut salt = vec![0u8; salt_len];
    random_bytes(&mut salt);
    transport.write_all(&salt).await.context("shadowsocks: send salt")?;

    let subkey = session_subkey(config.cipher, &config.key, &salt);
    let write_cipher = AeadCipher::new(config.cipher, &subkey)?;

    if config.cipher.is_2022() {
        return connect_2022(transport, config, target, salt, write_cipher).await;
    }

    let mut stream = ShadowsocksStream::new(transport, config.cipher, config.key.clone(), write_cipher);

    // The first plaintext bytes are the SOCKS5-format destination address.
    let mut addr = Vec::with_capacity(1 + 256 + 2);
    socks5::encode_address(&mut addr, target);
    stream
        .write_all(&addr)
        .await
        .context("shadowsocks: send target address")?;

    Ok(Box::new(stream))
}

/// Send the Shadowsocks 2022 request header on `tcp` (the session salt has
/// already been written) and return a relay-ready stream. The request is a
/// fixed-length header (`type | timestamp | variable-header length`) followed by
/// a variable-length header (`socks_addr | padding length | padding`), each
/// sealed as its own AEAD chunk; application data then flows as ordinary body
/// chunks with the write nonce continuing past the two header chunks.
async fn connect_2022(
    mut tcp: BoxedStream,
    config: &ShadowsocksOutboundConfig,
    target: &TargetAddr,
    request_salt: Vec<u8>,
    write_cipher: AeadCipher,
) -> Result<BoxedStream> {
    // Variable-length header: SOCKS5 target address + a zero-length padding
    // field (no initial payload).
    let mut var_header = Vec::with_capacity(1 + 256 + 2 + 2);
    socks5::encode_address(&mut var_header, target);
    var_header.extend_from_slice(&0u16.to_be_bytes());
    let var_len = u16::try_from(var_header.len()).map_err(|_| anyhow!("shadowsocks 2022: header too large"))?;

    // Fixed-length header: request type, timestamp, variable-header length.
    let mut fixed_header = Vec::with_capacity(1 + 8 + 2);
    fixed_header.push(SS2022_HEADER_TYPE_REQUEST);
    fixed_header.extend_from_slice(&unix_timestamp().to_be_bytes());
    fixed_header.extend_from_slice(&var_len.to_be_bytes());

    let mut write_nonce = [0u8; 12];
    let sealed_fixed = write_cipher.seal(&write_nonce, &fixed_header)?;
    increment_nonce(&mut write_nonce);
    let sealed_var = write_cipher.seal(&write_nonce, &var_header)?;
    increment_nonce(&mut write_nonce);

    let mut head = Vec::with_capacity(sealed_fixed.len() + sealed_var.len());
    head.extend_from_slice(&sealed_fixed);
    head.extend_from_slice(&sealed_var);
    tcp.write_all(&head)
        .await
        .context("shadowsocks 2022: send request header")?;

    let stream = ShadowsocksStream::new_2022(
        tcp,
        config.cipher,
        config.key.clone(),
        write_cipher,
        write_nonce,
        request_salt,
    );
    Ok(Box::new(stream))
}

/// A Shadowsocks UDP association to a single destination.
///
/// Shadowsocks UDP frames each datagram independently — unlike the TCP stream
/// there is no length-prefixing or running chunk nonce. The 2017 and 2022
/// generations use different per-packet constructions:
///
/// **2017 AEAD** (`salt`-based, stateless):
/// ```text
/// salt (key_len bytes, in clear) | AEAD(subkey, nonce=0, socks5_addr | payload)
/// ```
/// with a fresh random `salt` per packet (so the all-zero nonce is never reused
/// under one key) and `subkey = HKDF-SHA1(master_key, salt, "ss-subkey")`.
///
/// **2022 (SIP022)**: every association has a random 8-byte session ID and a
/// monotonic packet-ID counter. The AES methods use a *separate header*
/// (`session ID | packet ID`) encrypted with the PSK via single-block AES-ECB,
/// then an AES-GCM body keyed by `BLAKE3(PSK || session ID)` with the nonce
/// taken from the plaintext header's last 12 bytes; the chacha method uses
/// XChaCha20-Poly1305 with the PSK directly and a random 24-byte nonce, merging
/// the session/packet IDs into the body's main header. Either way the sealed
/// body is `type | timestamp | … | socks5_addr | payload`; replies are
/// validated (type, timestamp window, echoed client session ID) and the
/// SOCKS5 address is stripped before the payload is returned.
pub struct ShadowsocksUdp {
    socket: UdpSocket,
    cipher: ShadowsocksCipher,
    key: Vec<u8>,
    target: TargetAddr,
    /// Shadowsocks 2022 client session ID: random, fixed for the lifetime of
    /// the association. Unused by the 2017 methods.
    session_id: [u8; SS2022_SESSION_ID_LEN],
    /// Shadowsocks 2022 monotonic packet counter, starting at 0 and incremented
    /// per sent datagram. Unused by the 2017 methods.
    packet_id: AtomicU64,
}

impl ShadowsocksUdp {
    /// Bind a UDP socket and connect it to the Shadowsocks server. `target` is
    /// the eventual destination sealed into every datagram sent on this socket.
    pub async fn connect(config: &ShadowsocksOutboundConfig, target: &TargetAddr) -> Result<Self> {
        let server = lookup_host((config.server.as_str(), config.port))
            .await
            .with_context(|| format!("shadowsocks udp: resolve {}:{}", config.server, config.port))?
            .next()
            .ok_or_else(|| anyhow!("shadowsocks udp: no address for {}:{}", config.server, config.port))?;
        let socket = crate::udp::bind_egress(server).await?;
        socket
            .connect(server)
            .await
            .with_context(|| format!("shadowsocks udp: connect {server}"))?;
        // A fresh random session ID per association; only consulted by the 2022
        // methods, but cheap enough to always generate.
        let mut session_id = [0u8; SS2022_SESSION_ID_LEN];
        random_bytes(&mut session_id);
        Ok(Self {
            socket,
            cipher: config.cipher,
            key: config.key.clone(),
            target: target.clone(),
            session_id,
            packet_id: AtomicU64::new(0),
        })
    }

    /// Seal `payload` for the destination and send it to the server.
    pub async fn send(&self, payload: &[u8]) -> Result<()> {
        let packet = self.seal(payload)?;
        self.socket.send(&packet).await.context("shadowsocks udp: send")?;
        Ok(())
    }

    /// Receive one reply datagram, open it, and return the application payload
    /// (the source-address prefix is discarded).
    pub async fn recv(&self) -> Result<Vec<u8>> {
        let mut buf = vec![0u8; MAX_UDP_PACKET];
        let n = self.socket.recv(&mut buf).await.context("shadowsocks udp: recv")?;
        self.open(&buf[..n])
    }

    fn seal(&self, payload: &[u8]) -> Result<Vec<u8>> {
        if self.cipher.is_2022() {
            return self.seal_2022(payload);
        }
        let salt_len = self.cipher.key_size();
        let mut salt = vec![0u8; salt_len];
        random_bytes(&mut salt);
        let subkey = hkdf_sha1(&self.key, &salt, SS_SUBKEY_INFO, salt_len);
        let cipher = AeadCipher::new(self.cipher, &subkey)?;
        let mut plain = Vec::with_capacity(1 + 256 + 2 + payload.len());
        socks5::encode_address(&mut plain, &self.target);
        plain.extend_from_slice(payload);
        let sealed = cipher.seal(&[0u8; 12], &plain)?;
        let mut packet = salt;
        packet.extend_from_slice(&sealed);
        Ok(packet)
    }

    fn open(&self, datagram: &[u8]) -> Result<Vec<u8>> {
        if self.cipher.is_2022() {
            return self.open_2022(datagram);
        }
        let salt_len = self.cipher.key_size();
        if datagram.len() < salt_len + TAG_LEN {
            bail!("shadowsocks udp: datagram too short");
        }
        let (salt, sealed) = datagram.split_at(salt_len);
        let subkey = hkdf_sha1(&self.key, salt, SS_SUBKEY_INFO, salt_len);
        let cipher = AeadCipher::new(self.cipher, &subkey)?;
        let plain = cipher.open(&[0u8; 12], sealed)?;
        let (_source, offset) = socks5::decode_address(&plain)?;
        Ok(plain[offset..].to_vec())
    }

    /// Seal one Shadowsocks 2022 UDP datagram, dispatching to the AES
    /// separate-header construction or the XChaCha20-Poly1305 construction.
    fn seal_2022(&self, payload: &[u8]) -> Result<Vec<u8>> {
        let packet_id = self.packet_id.fetch_add(1, Ordering::Relaxed);
        match self.cipher {
            ShadowsocksCipher::Blake3Chacha20Poly1305 => self.seal_2022_chacha(payload, packet_id),
            _ => self.seal_2022_aes(payload, packet_id),
        }
    }

    /// Open one Shadowsocks 2022 UDP datagram, dispatching by cipher family.
    fn open_2022(&self, datagram: &[u8]) -> Result<Vec<u8>> {
        match self.cipher {
            ShadowsocksCipher::Blake3Chacha20Poly1305 => self.open_2022_chacha(datagram),
            _ => self.open_2022_aes(datagram),
        }
    }

    /// SIP022 AES-GCM UDP: a 16-byte separate header (`session ID | packet ID`)
    /// encrypted with the PSK as a single AES-ECB block, followed by an AES-GCM
    /// body whose nonce is the plaintext header's last 12 bytes.
    fn seal_2022_aes(&self, payload: &[u8], packet_id: u64) -> Result<Vec<u8>> {
        let mut separate_header = [0u8; SS2022_SEPARATE_HEADER_LEN];
        separate_header[..SS2022_SESSION_ID_LEN].copy_from_slice(&self.session_id);
        separate_header[SS2022_SESSION_ID_LEN..].copy_from_slice(&packet_id.to_be_bytes());

        let subkey = session_subkey(self.cipher, &self.key, &self.session_id);
        let aead = AeadCipher::new(self.cipher, &subkey)?;
        let mut nonce = [0u8; 12];
        nonce.copy_from_slice(&separate_header[4..]);
        let sealed_body = aead.seal(&nonce, &self.build_request_body(packet_id, false, payload))?;

        let block = Ss2022BlockCipher::new(self.cipher, &self.key)?;
        let mut encrypted_header = separate_header;
        block.encrypt_block(&mut encrypted_header);

        let mut packet = Vec::with_capacity(SS2022_SEPARATE_HEADER_LEN + sealed_body.len());
        packet.extend_from_slice(&encrypted_header);
        packet.extend_from_slice(&sealed_body);
        Ok(packet)
    }

    /// SIP022 XChaCha20-Poly1305 UDP: a random 24-byte nonce, then a body sealed
    /// with the PSK directly. The session/packet IDs are merged into the body's
    /// main header rather than a separate header.
    fn seal_2022_chacha(&self, payload: &[u8], packet_id: u64) -> Result<Vec<u8>> {
        let mut nonce = [0u8; SS2022_XNONCE_LEN];
        random_bytes(&mut nonce);
        let cipher = XChaCha20Poly1305::new_from_slice(&self.key)
            .map_err(|_| anyhow!("shadowsocks 2022 udp: invalid chacha key"))?;
        let sealed = cipher
            .encrypt(
                XNonce::from_slice(&nonce),
                Payload {
                    msg: &self.build_request_body(packet_id, true, payload),
                    aad: &[],
                },
            )
            .map_err(|_| anyhow!("shadowsocks 2022 udp: seal failed"))?;
        let mut packet = Vec::with_capacity(SS2022_XNONCE_LEN + sealed.len());
        packet.extend_from_slice(&nonce);
        packet.extend_from_slice(&sealed);
        Ok(packet)
    }

    /// Build the plaintext body for a client-to-server UDP message. With
    /// `merged_session` (the chacha construction) the body is prefixed with the
    /// session ID and packet ID; the AES construction carries those in the
    /// separate header instead. The remainder is the main header
    /// (`type | timestamp | padding length(0) | socks_addr`) followed by the
    /// application payload.
    fn build_request_body(&self, packet_id: u64, merged_session: bool, payload: &[u8]) -> Vec<u8> {
        let mut body = Vec::with_capacity(8 + 8 + 1 + 8 + 2 + 1 + 256 + 2 + payload.len());
        if merged_session {
            body.extend_from_slice(&self.session_id);
            body.extend_from_slice(&packet_id.to_be_bytes());
        }
        body.push(SS2022_UDP_HEADER_TYPE_CLIENT);
        body.extend_from_slice(&unix_timestamp().to_be_bytes());
        body.extend_from_slice(&0u16.to_be_bytes());
        socks5::encode_address(&mut body, &self.target);
        body.extend_from_slice(payload);
        body
    }

    fn open_2022_aes(&self, datagram: &[u8]) -> Result<Vec<u8>> {
        if datagram.len() < SS2022_SEPARATE_HEADER_LEN + TAG_LEN {
            bail!("shadowsocks 2022 udp: datagram too short");
        }
        let block = Ss2022BlockCipher::new(self.cipher, &self.key)?;
        let mut separate_header = [0u8; SS2022_SEPARATE_HEADER_LEN];
        separate_header.copy_from_slice(&datagram[..SS2022_SEPARATE_HEADER_LEN]);
        block.decrypt_block(&mut separate_header);

        let server_session_id = &separate_header[..SS2022_SESSION_ID_LEN];
        let subkey = session_subkey(self.cipher, &self.key, server_session_id);
        let aead = AeadCipher::new(self.cipher, &subkey)?;
        let mut nonce = [0u8; 12];
        nonce.copy_from_slice(&separate_header[4..]);
        let body = aead.open(&nonce, &datagram[SS2022_SEPARATE_HEADER_LEN..])?;
        self.parse_response_body(&body)
    }

    fn open_2022_chacha(&self, datagram: &[u8]) -> Result<Vec<u8>> {
        if datagram.len() < SS2022_XNONCE_LEN + TAG_LEN {
            bail!("shadowsocks 2022 udp: datagram too short");
        }
        let (nonce, sealed) = datagram.split_at(SS2022_XNONCE_LEN);
        let cipher = XChaCha20Poly1305::new_from_slice(&self.key)
            .map_err(|_| anyhow!("shadowsocks 2022 udp: invalid chacha key"))?;
        let body = cipher
            .decrypt(XNonce::from_slice(nonce), Payload { msg: sealed, aad: &[] })
            .map_err(|_| anyhow!("shadowsocks 2022 udp: open failed"))?;
        // The chacha server-to-client body is prefixed with the server session
        // ID and packet ID before the shared main header.
        let header_start = SS2022_SESSION_ID_LEN + 8;
        if body.len() < header_start {
            bail!("shadowsocks 2022 udp: response header truncated");
        }
        self.parse_response_body(&body[header_start..])
    }

    /// Validate a server-to-client main header and return the trailing payload.
    /// `body` must start at the `type` field, i.e. any leading session/packet ID
    /// has already been stripped:
    /// `type | timestamp | client session ID | padding length | padding | socks_addr | payload`.
    fn parse_response_body(&self, body: &[u8]) -> Result<Vec<u8>> {
        let header_min = 1 + 8 + SS2022_SESSION_ID_LEN + 2;
        if body.len() < header_min {
            bail!("shadowsocks 2022 udp: response header truncated");
        }
        if body[0] != SS2022_UDP_HEADER_TYPE_SERVER {
            bail!("shadowsocks 2022 udp: unexpected response header type");
        }
        let mut ts = [0u8; 8];
        ts.copy_from_slice(&body[1..9]);
        if unix_timestamp().abs_diff(u64::from_be_bytes(ts)) > SS2022_MAX_TIME_DIFF {
            bail!("shadowsocks 2022 udp: response timestamp outside replay window");
        }
        if body[9..9 + SS2022_SESSION_ID_LEN] != self.session_id[..] {
            bail!("shadowsocks 2022 udp: response client session id mismatch");
        }
        let pad_at = 9 + SS2022_SESSION_ID_LEN;
        let pad_len = u16::from_be_bytes([body[pad_at], body[pad_at + 1]]) as usize;
        let addr_at = pad_at + 2 + pad_len;
        if body.len() < addr_at {
            bail!("shadowsocks 2022 udp: padding exceeds packet");
        }
        let (_source, offset) = socks5::decode_address(&body[addr_at..])?;
        Ok(body[addr_at + offset..].to_vec())
    }
}

/// Current Unix time in whole seconds, used for the Shadowsocks 2022 header
/// timestamp. A clock before the epoch is clamped to 0.
fn unix_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// Derive the per-session AEAD subkey from the session `salt`. The 2017 methods
/// run HKDF-SHA1 over the EVP master key; the 2022 methods run BLAKE3
/// `derive_key` over the raw PSK, taking the leading `key_size` bytes.
fn session_subkey(cipher: ShadowsocksCipher, key: &[u8], salt: &[u8]) -> Vec<u8> {
    let key_len = cipher.key_size();
    if cipher.is_2022() {
        let mut material = Vec::with_capacity(key.len() + salt.len());
        material.extend_from_slice(key);
        material.extend_from_slice(salt);
        let derived = blake3::derive_key(SS2022_SUBKEY_CONTEXT, &material);
        derived[..key_len].to_vec()
    } else {
        hkdf_sha1(key, salt, SS_SUBKEY_INFO, key_len)
    }
}

/// Decode standard or URL-safe Base64 (padding and ASCII whitespace ignored),
/// used for the Shadowsocks 2022 PSK. Trailing bits that do not complete a byte
/// are discarded, matching canonical Base64.
fn base64_decode(input: &str) -> Result<Vec<u8>> {
    fn sextet(c: u8) -> Option<u8> {
        match c {
            b'A'..=b'Z' => Some(c - b'A'),
            b'a'..=b'z' => Some(c - b'a' + 26),
            b'0'..=b'9' => Some(c - b'0' + 52),
            b'+' | b'-' => Some(62),
            b'/' | b'_' => Some(63),
            _ => None,
        }
    }
    let mut out = Vec::with_capacity(input.len() / 4 * 3);
    let mut acc = 0u32;
    let mut bits = 0u32;
    for &c in input.as_bytes() {
        if c == b'=' || c.is_ascii_whitespace() {
            continue;
        }
        let v = sextet(c).ok_or_else(|| anyhow!("invalid base64 character {:?}", c as char))?;
        acc = (acc << 6) | u32::from(v);
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push((acc >> bits) as u8);
        }
    }
    Ok(out)
}

/// Fill `buf` with cryptographically secure random bytes from the OS.
fn random_bytes(buf: &mut [u8]) {
    if getrandom::fill(buf).is_err() {
        panic!("shadowsocks: system RNG unavailable");
    }
}

/// OpenSSL `EVP_BytesToKey` with MD5, count 1, no salt — the Shadowsocks master
/// key derivation from a password.
fn evp_bytes_to_key(password: &[u8], key_len: usize) -> Vec<u8> {
    let mut key = Vec::with_capacity(key_len);
    let mut prev: Vec<u8> = Vec::new();
    while key.len() < key_len {
        let mut hasher = Md5::new();
        hasher.update(&prev);
        hasher.update(password);
        let digest: [u8; 16] = hasher.finalize().into();
        key.extend_from_slice(&digest);
        prev = digest.to_vec();
    }
    key.truncate(key_len);
    key
}

// --- HKDF-SHA1 ------------------------------------------------------------
//
// Shadowsocks derives the per-session subkey with HKDF-SHA1. The construction
// is standard (RFC 5869) but no HKDF crate is in the dependency tree, so it is
// assembled here on top of the vetted `sha1` primitive, mirroring how the VMess
// module hand-rolls its KDF over `sha2`.

const SHA1_BLOCK: usize = 64;
const SHA1_OUT: usize = 20;

/// SHA-1 over the concatenation of `parts`.
fn sha1(parts: &[&[u8]]) -> [u8; SHA1_OUT] {
    let mut hasher = Sha1::new();
    for part in parts {
        hasher.update(part);
    }
    hasher.finalize().into()
}

/// Standard HMAC-SHA1.
fn hmac_sha1(key: &[u8], msg: &[u8]) -> [u8; SHA1_OUT] {
    let mut block = [0u8; SHA1_BLOCK];
    if key.len() > SHA1_BLOCK {
        block[..SHA1_OUT].copy_from_slice(&sha1(&[key]));
    } else {
        block[..key.len()].copy_from_slice(key);
    }
    let mut ipad = [0u8; SHA1_BLOCK];
    let mut opad = [0u8; SHA1_BLOCK];
    for i in 0..SHA1_BLOCK {
        ipad[i] = block[i] ^ 0x36;
        opad[i] = block[i] ^ 0x5c;
    }
    let inner = sha1(&[&ipad, msg]);
    sha1(&[&opad, &inner])
}

/// HKDF-SHA1 (extract + expand) producing `length` bytes of output.
fn hkdf_sha1(ikm: &[u8], salt: &[u8], info: &[u8], length: usize) -> Vec<u8> {
    let prk = hmac_sha1(salt, ikm);
    let mut okm = Vec::with_capacity(length);
    let mut prev: Vec<u8> = Vec::new();
    let mut counter: u8 = 1;
    while okm.len() < length {
        let mut input = Vec::with_capacity(prev.len() + info.len() + 1);
        input.extend_from_slice(&prev);
        input.extend_from_slice(info);
        input.push(counter);
        let block = hmac_sha1(&prk, &input);
        okm.extend_from_slice(&block);
        prev = block.to_vec();
        counter = counter.wrapping_add(1);
    }
    okm.truncate(length);
    okm
}

/// Increment a 12-byte little-endian counter nonce in place.
fn increment_nonce(nonce: &mut [u8; 12]) {
    for byte in nonce.iter_mut() {
        *byte = byte.wrapping_add(1);
        if *byte != 0 {
            break;
        }
    }
}

/// A keyed Shadowsocks AEAD cipher (one direction).
enum AeadCipher {
    Aes128(Box<Aes128Gcm>),
    Aes256(Box<Aes256Gcm>),
    Chacha(Box<ChaCha20Poly1305>),
}

impl AeadCipher {
    fn new(cipher: ShadowsocksCipher, subkey: &[u8]) -> Result<Self> {
        match cipher {
            ShadowsocksCipher::Aes128Gcm | ShadowsocksCipher::Blake3Aes128Gcm => Ok(AeadCipher::Aes128(Box::new(
                Aes128Gcm::new_from_slice(subkey).map_err(|_| anyhow!("shadowsocks: invalid aes-128 key"))?,
            ))),
            ShadowsocksCipher::Aes256Gcm | ShadowsocksCipher::Blake3Aes256Gcm => Ok(AeadCipher::Aes256(Box::new(
                Aes256Gcm::new_from_slice(subkey).map_err(|_| anyhow!("shadowsocks: invalid aes-256 key"))?,
            ))),
            ShadowsocksCipher::Chacha20IetfPoly1305 | ShadowsocksCipher::Blake3Chacha20Poly1305 => {
                Ok(AeadCipher::Chacha(Box::new(
                    ChaCha20Poly1305::new_from_slice(subkey).map_err(|_| anyhow!("shadowsocks: invalid chacha key"))?,
                )))
            }
        }
    }

    fn seal(&self, nonce: &[u8; 12], plaintext: &[u8]) -> Result<Vec<u8>> {
        let payload = Payload {
            msg: plaintext,
            aad: &[],
        };
        let result = match self {
            AeadCipher::Aes128(c) => c.encrypt(GenericArray::from_slice(nonce), payload),
            AeadCipher::Aes256(c) => c.encrypt(GenericArray::from_slice(nonce), payload),
            AeadCipher::Chacha(c) => c.encrypt(GenericArray::from_slice(nonce), payload),
        };
        result.map_err(|_| anyhow!("shadowsocks: AEAD seal failed"))
    }

    fn open(&self, nonce: &[u8; 12], ciphertext: &[u8]) -> Result<Vec<u8>> {
        let payload = Payload {
            msg: ciphertext,
            aad: &[],
        };
        let result = match self {
            AeadCipher::Aes128(c) => c.decrypt(GenericArray::from_slice(nonce), payload),
            AeadCipher::Aes256(c) => c.decrypt(GenericArray::from_slice(nonce), payload),
            AeadCipher::Chacha(c) => c.decrypt(GenericArray::from_slice(nonce), payload),
        };
        result.map_err(|_| anyhow!("shadowsocks: AEAD open failed"))
    }
}

/// A keyed AES block cipher used for the Shadowsocks 2022 UDP separate header,
/// which is encrypted/decrypted with the PSK as a single ECB block (no chaining
/// or padding). Only the AES 2022 methods use a separate header; the chacha
/// method has none.
enum Ss2022BlockCipher {
    Aes128(Box<Aes128>),
    Aes256(Box<Aes256>),
}

impl Ss2022BlockCipher {
    fn new(cipher: ShadowsocksCipher, key: &[u8]) -> Result<Self> {
        match cipher {
            ShadowsocksCipher::Blake3Aes128Gcm => Ok(Ss2022BlockCipher::Aes128(Box::new(
                Aes128::new_from_slice(key).map_err(|_| anyhow!("shadowsocks 2022 udp: invalid aes-128 key"))?,
            ))),
            ShadowsocksCipher::Blake3Aes256Gcm => Ok(Ss2022BlockCipher::Aes256(Box::new(
                Aes256::new_from_slice(key).map_err(|_| anyhow!("shadowsocks 2022 udp: invalid aes-256 key"))?,
            ))),
            other => Err(anyhow!(
                "shadowsocks 2022 udp: separate header requires an AES method, got {other:?}"
            )),
        }
    }

    fn encrypt_block(&self, block: &mut [u8; SS2022_SEPARATE_HEADER_LEN]) {
        let ga = GenericArray::from_mut_slice(block);
        match self {
            Ss2022BlockCipher::Aes128(c) => c.encrypt_block(ga),
            Ss2022BlockCipher::Aes256(c) => c.encrypt_block(ga),
        }
    }

    fn decrypt_block(&self, block: &mut [u8; SS2022_SEPARATE_HEADER_LEN]) {
        let ga = GenericArray::from_mut_slice(block);
        match self {
            Ss2022BlockCipher::Aes128(c) => c.decrypt_block(ga),
            Ss2022BlockCipher::Aes256(c) => c.decrypt_block(ga),
        }
    }
}

/// Read-side framing state machine.
enum ReadState {
    /// Waiting for the peer's `salt_len`-byte salt (derives the read cipher).
    Salt,
    /// Shadowsocks 2022 only: waiting for the fixed-length response header
    /// (`type | timestamp | request salt | first-chunk length`).
    RespHeader,
    /// Shadowsocks 2022 only: waiting for the first payload chunk, whose length
    /// (`clen`) was carried in the response header; `clen + 16` bytes.
    FirstChunk(usize),
    /// Waiting for the 18-byte AEAD-sealed payload length.
    Len,
    /// Waiting for a `clen + 16`-byte sealed payload chunk.
    Data(usize),
    /// Clean EOF (the peer closed the connection).
    Eof,
}

/// Wraps the underlying transport stream (raw TCP, or a SIP003 plugin
/// transport): writes seal application data into Shadowsocks AEAD chunks; reads
/// strip the peer salt, then decrypt length-prefixed chunks. The client salt
/// and sealed target address are sent at connect time.
struct ShadowsocksStream {
    inner: BoxedStream,
    cipher: ShadowsocksCipher,
    master_key: Vec<u8>,
    /// Whether this is a Shadowsocks 2022 session (BLAKE3 subkey + SIP022
    /// response header). For 2017 sessions `request_salt` is empty.
    is_2022: bool,
    /// The client's request salt, echoed back in the 2022 response header and
    /// validated on read to bind the response to this request.
    request_salt: Vec<u8>,
    // Write side.
    write_cipher: AeadCipher,
    write_nonce: [u8; 12],
    write_buf: Vec<u8>,
    write_pos: usize,
    // Read side.
    read_cipher: Option<AeadCipher>,
    read_nonce: [u8; 12],
    read_state: ReadState,
    read_raw: Vec<u8>,
    plain: Vec<u8>,
    plain_pos: usize,
}

impl ShadowsocksStream {
    fn new(inner: BoxedStream, cipher: ShadowsocksCipher, master_key: Vec<u8>, write_cipher: AeadCipher) -> Self {
        Self {
            inner,
            cipher,
            master_key,
            is_2022: false,
            request_salt: Vec::new(),
            write_cipher,
            write_nonce: [0u8; 12],
            write_buf: Vec::new(),
            write_pos: 0,
            read_cipher: None,
            read_nonce: [0u8; 12],
            read_state: ReadState::Salt,
            read_raw: Vec::new(),
            plain: Vec::new(),
            plain_pos: 0,
        }
    }

    /// Construct a Shadowsocks 2022 stream. The request header (and salt) have
    /// already been written, so `write_nonce` continues past the two header
    /// chunks and `request_salt` is retained to validate the response header.
    fn new_2022(
        inner: BoxedStream,
        cipher: ShadowsocksCipher,
        psk: Vec<u8>,
        write_cipher: AeadCipher,
        write_nonce: [u8; 12],
        request_salt: Vec<u8>,
    ) -> Self {
        Self {
            inner,
            cipher,
            master_key: psk,
            is_2022: true,
            request_salt,
            write_cipher,
            write_nonce,
            write_buf: Vec::new(),
            write_pos: 0,
            read_cipher: None,
            read_nonce: [0u8; 12],
            read_state: ReadState::Salt,
            read_raw: Vec::new(),
            plain: Vec::new(),
            plain_pos: 0,
        }
    }

    /// Flush any pending sealed bytes to the inner stream.
    fn poll_drain(&mut self, cx: &mut TaskContext<'_>) -> Poll<io::Result<()>> {
        while self.write_pos < self.write_buf.len() {
            let n = ready!(Pin::new(&mut self.inner).poll_write(cx, &self.write_buf[self.write_pos..]))?;
            if n == 0 {
                return Poll::Ready(Err(io::Error::new(io::ErrorKind::WriteZero, "shadowsocks: write zero")));
            }
            self.write_pos += n;
        }
        self.write_buf.clear();
        self.write_pos = 0;
        Poll::Ready(Ok(()))
    }

    /// Seal `plaintext` (at most [`MAX_CHUNK`] bytes) into a length-prefixed AEAD
    /// chunk queued for writing.
    fn queue_chunk(&mut self, plaintext: &[u8]) -> io::Result<()> {
        let len = u16::try_from(plaintext.len())
            .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "shadowsocks: chunk too large"))?;
        let sealed_len = self
            .write_cipher
            .seal(&self.write_nonce, &len.to_be_bytes())
            .map_err(|e| io::Error::other(e.to_string()))?;
        increment_nonce(&mut self.write_nonce);
        let sealed_payload = self
            .write_cipher
            .seal(&self.write_nonce, plaintext)
            .map_err(|e| io::Error::other(e.to_string()))?;
        increment_nonce(&mut self.write_nonce);

        self.write_buf.clear();
        self.write_pos = 0;
        self.write_buf.extend_from_slice(&sealed_len);
        self.write_buf.extend_from_slice(&sealed_payload);
        Ok(())
    }
}

fn decrypt_err(e: anyhow::Error) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, e.to_string())
}

/// The read cipher is created from the peer salt before any chunk is decrypted,
/// so reaching a chunk state without it is a logic error rather than a
/// recoverable condition; surface it as an error instead of panicking.
fn read_cipher_unset() -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, "shadowsocks: read cipher unset")
}

impl AsyncRead for ShadowsocksStream {
    fn poll_read(self: Pin<&mut Self>, cx: &mut TaskContext<'_>, buf: &mut ReadBuf<'_>) -> Poll<io::Result<()>> {
        let this = self.get_mut();
        let salt_len = this.cipher.key_size();
        loop {
            if this.plain_pos < this.plain.len() {
                let n = buf.remaining().min(this.plain.len() - this.plain_pos);
                buf.put_slice(&this.plain[this.plain_pos..this.plain_pos + n]);
                this.plain_pos += n;
                return Poll::Ready(Ok(()));
            }
            if matches!(this.read_state, ReadState::Eof) {
                return Poll::Ready(Ok(()));
            }

            let need = match this.read_state {
                ReadState::Salt => salt_len,
                ReadState::RespHeader => 1 + 8 + salt_len + 2 + TAG_LEN,
                ReadState::FirstChunk(clen) => clen + TAG_LEN,
                ReadState::Len => 2 + TAG_LEN,
                ReadState::Data(clen) => clen + TAG_LEN,
                ReadState::Eof => unreachable!(),
            };

            if this.read_raw.len() < need {
                let mut scratch = [0u8; 4096];
                let mut read_buf = ReadBuf::new(&mut scratch);
                ready!(Pin::new(&mut this.inner).poll_read(cx, &mut read_buf))?;
                let filled = read_buf.filled();
                if filled.is_empty() {
                    // TCP FIN: Shadowsocks marks end-of-stream by closing, with
                    // no terminating chunk, so a clean EOF here is expected.
                    this.read_state = ReadState::Eof;
                    return Poll::Ready(Ok(()));
                }
                this.read_raw.extend_from_slice(filled);
                continue;
            }

            match this.read_state {
                ReadState::Salt => {
                    let salt: Vec<u8> = this.read_raw.drain(..salt_len).collect();
                    let subkey = session_subkey(this.cipher, &this.master_key, &salt);
                    let cipher = AeadCipher::new(this.cipher, &subkey).map_err(decrypt_err)?;
                    this.read_cipher = Some(cipher);
                    this.read_state = if this.is_2022 {
                        ReadState::RespHeader
                    } else {
                        ReadState::Len
                    };
                }
                ReadState::RespHeader => {
                    let sealed: Vec<u8> = this.read_raw.drain(..1 + 8 + salt_len + 2 + TAG_LEN).collect();
                    let Some(cipher) = this.read_cipher.as_ref() else {
                        return Poll::Ready(Err(read_cipher_unset()));
                    };
                    let plain = cipher.open(&this.read_nonce, &sealed).map_err(decrypt_err)?;
                    increment_nonce(&mut this.read_nonce);
                    if plain.len() != 1 + 8 + salt_len + 2 {
                        return Poll::Ready(Err(io::Error::new(
                            io::ErrorKind::InvalidData,
                            "shadowsocks 2022: malformed response header",
                        )));
                    }
                    if plain[0] != SS2022_HEADER_TYPE_RESPONSE {
                        return Poll::Ready(Err(io::Error::new(
                            io::ErrorKind::InvalidData,
                            "shadowsocks 2022: unexpected response header type",
                        )));
                    }
                    let mut ts_bytes = [0u8; 8];
                    ts_bytes.copy_from_slice(&plain[1..9]);
                    let ts = u64::from_be_bytes(ts_bytes);
                    if unix_timestamp().abs_diff(ts) > SS2022_MAX_TIME_DIFF {
                        return Poll::Ready(Err(io::Error::new(
                            io::ErrorKind::InvalidData,
                            "shadowsocks 2022: response timestamp outside replay window",
                        )));
                    }
                    if plain[9..9 + salt_len] != this.request_salt[..] {
                        return Poll::Ready(Err(io::Error::new(
                            io::ErrorKind::InvalidData,
                            "shadowsocks 2022: response salt does not match request",
                        )));
                    }
                    let clen = u16::from_be_bytes([plain[9 + salt_len], plain[10 + salt_len]]) as usize;
                    if clen == 0 || clen > MAX_CHUNK {
                        return Poll::Ready(Err(io::Error::new(
                            io::ErrorKind::InvalidData,
                            "shadowsocks 2022: invalid first-chunk length",
                        )));
                    }
                    this.read_state = ReadState::FirstChunk(clen);
                }
                ReadState::FirstChunk(clen) => {
                    let sealed: Vec<u8> = this.read_raw.drain(..clen + TAG_LEN).collect();
                    let Some(cipher) = this.read_cipher.as_ref() else {
                        return Poll::Ready(Err(read_cipher_unset()));
                    };
                    let plain = cipher.open(&this.read_nonce, &sealed).map_err(decrypt_err)?;
                    increment_nonce(&mut this.read_nonce);
                    this.plain = plain;
                    this.plain_pos = 0;
                    this.read_state = ReadState::Len;
                }
                ReadState::Len => {
                    let sealed: Vec<u8> = this.read_raw.drain(..2 + TAG_LEN).collect();
                    let Some(cipher) = this.read_cipher.as_ref() else {
                        return Poll::Ready(Err(read_cipher_unset()));
                    };
                    let plain = cipher.open(&this.read_nonce, &sealed).map_err(decrypt_err)?;
                    increment_nonce(&mut this.read_nonce);
                    let clen = u16::from_be_bytes([plain[0], plain[1]]) as usize;
                    if clen == 0 || clen > MAX_CHUNK {
                        return Poll::Ready(Err(io::Error::new(
                            io::ErrorKind::InvalidData,
                            "shadowsocks: invalid chunk length",
                        )));
                    }
                    this.read_state = ReadState::Data(clen);
                }
                ReadState::Data(clen) => {
                    let sealed: Vec<u8> = this.read_raw.drain(..clen + TAG_LEN).collect();
                    let Some(cipher) = this.read_cipher.as_ref() else {
                        return Poll::Ready(Err(read_cipher_unset()));
                    };
                    let plain = cipher.open(&this.read_nonce, &sealed).map_err(decrypt_err)?;
                    increment_nonce(&mut this.read_nonce);
                    this.plain = plain;
                    this.plain_pos = 0;
                    this.read_state = ReadState::Len;
                }
                ReadState::Eof => unreachable!(),
            }
        }
    }
}

impl AsyncWrite for ShadowsocksStream {
    fn poll_write(self: Pin<&mut Self>, cx: &mut TaskContext<'_>, buf: &[u8]) -> Poll<io::Result<usize>> {
        let this = self.get_mut();
        ready!(this.poll_drain(cx))?;
        if buf.is_empty() {
            return Poll::Ready(Ok(0));
        }
        let take = buf.len().min(MAX_CHUNK);
        this.queue_chunk(&buf[..take])?;
        // Best-effort flush; remaining bytes drain on the next poll.
        if let Poll::Ready(Err(e)) = this.poll_drain(cx) {
            return Poll::Ready(Err(e));
        }
        Poll::Ready(Ok(take))
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut TaskContext<'_>) -> Poll<io::Result<()>> {
        let this = self.get_mut();
        ready!(this.poll_drain(cx))?;
        Pin::new(&mut this.inner).poll_flush(cx)
    }

    fn poll_shutdown(self: Pin<&mut Self>, cx: &mut TaskContext<'_>) -> Poll<io::Result<()>> {
        let this = self.get_mut();
        ready!(this.poll_drain(cx))?;
        // Shadowsocks has no terminating chunk: closing the TCP stream (FIN) is
        // the end-of-stream signal.
        Pin::new(&mut this.inner).poll_shutdown(cx)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::outbound_opts::ProxyEntry;

    fn parse_entry(yaml: &str) -> ProxyEntry {
        serde_yaml_ng::from_str(yaml).expect("parse proxy entry")
    }

    #[test]
    fn evp_bytes_to_key_single_block_is_md5() {
        // For a 16-byte key the derivation is exactly MD5(password).
        // MD5("foobar") = 3858f62230ac3c915f300c664312c63f.
        let key = evp_bytes_to_key(b"foobar", 16);
        let expected = [
            0x38, 0x58, 0xf6, 0x22, 0x30, 0xac, 0x3c, 0x91, 0x5f, 0x30, 0x0c, 0x66, 0x43, 0x12, 0xc6, 0x3f,
        ];
        assert_eq!(key, expected);
    }

    #[test]
    fn evp_bytes_to_key_extends_to_requested_length() {
        // A 32-byte key chains a second MD5 block; the first 16 bytes still
        // equal MD5(password).
        let key = evp_bytes_to_key(b"foobar", 32);
        assert_eq!(key.len(), 32);
        assert_eq!(&key[..16], &evp_bytes_to_key(b"foobar", 16)[..]);
    }

    #[test]
    fn hmac_sha1_matches_rfc2202_case2() {
        // RFC 2202 test case 2: key="Jefe", data="what do ya want for nothing?".
        let mac = hmac_sha1(b"Jefe", b"what do ya want for nothing?");
        let expected = [
            0xef, 0xfc, 0xdf, 0x6a, 0xe5, 0xeb, 0x2f, 0xa2, 0xd2, 0x74, 0x16, 0xd5, 0xf1, 0x84, 0xdf, 0x9c, 0x25, 0x9a,
            0x7c, 0x79,
        ];
        assert_eq!(mac, expected);
    }

    #[test]
    fn hkdf_sha1_matches_rfc5869_case4() {
        // RFC 5869 Appendix A.4 (SHA-1).
        let ikm = [0x0bu8; 11];
        let salt: Vec<u8> = (0u8..=0x0c).collect();
        let info: Vec<u8> = (0xf0u8..=0xf9).collect();
        let okm = hkdf_sha1(&ikm, &salt, &info, 42);
        let expected = [
            0x08, 0x5a, 0x01, 0xea, 0x1b, 0x10, 0xf3, 0x69, 0x33, 0x06, 0x8b, 0x56, 0xef, 0xa5, 0xad, 0x81, 0xa4, 0xf1,
            0x4b, 0x82, 0x2f, 0x5b, 0x09, 0x15, 0x68, 0xa9, 0xcd, 0xd4, 0xf1, 0x55, 0xfd, 0xa2, 0xc2, 0x2e, 0x42, 0x24,
            0x78, 0xd3, 0x05, 0xf3, 0xf8, 0x96,
        ];
        assert_eq!(okm, expected);
    }

    #[test]
    fn nonce_increments_little_endian_with_carry() {
        let mut nonce = [0u8; 12];
        increment_nonce(&mut nonce);
        assert_eq!(nonce[0], 1);
        nonce[0] = 0xff;
        increment_nonce(&mut nonce);
        assert_eq!(nonce[0], 0);
        assert_eq!(nonce[1], 1);
    }

    #[test]
    fn parses_aes_256_gcm_entry() {
        let yaml = "name: s\ntype: ss\nserver: example.com\nport: 8388\ncipher: aes-256-gcm\npassword: secret\n";
        let cfg = ShadowsocksOutboundConfig::from_proxy(&parse_entry(yaml)).unwrap();
        assert_eq!(cfg.server, "example.com");
        assert_eq!(cfg.port, 8388);
        assert_eq!(cfg.cipher, ShadowsocksCipher::Aes256Gcm);
        assert_eq!(cfg.key, evp_bytes_to_key(b"secret", 32));
    }

    #[test]
    fn chacha_alias_is_accepted() {
        let yaml = "name: s\ntype: ss\nserver: h\nport: 1\ncipher: chacha20-poly1305\npassword: p\n";
        let cfg = ShadowsocksOutboundConfig::from_proxy(&parse_entry(yaml)).unwrap();
        assert_eq!(cfg.cipher, ShadowsocksCipher::Chacha20IetfPoly1305);
    }

    #[test]
    fn legacy_stream_cipher_is_rejected() {
        let yaml = "name: s\ntype: ss\nserver: h\nport: 1\ncipher: aes-256-cfb\npassword: p\n";
        let err = ShadowsocksOutboundConfig::from_proxy(&parse_entry(yaml)).unwrap_err();
        assert!(err.to_string().contains("not supported"), "got: {err}");
    }

    #[test]
    fn parses_2022_aes_256_gcm_entry() {
        // PSK is Base64 of the 32 bytes 0x00..=0x1f and is used as the key
        // directly (no EVP_BytesToKey expansion).
        let yaml = "name: s\ntype: ss\nserver: h\nport: 8388\ncipher: 2022-blake3-aes-256-gcm\npassword: AAECAwQFBgcICQoLDA0ODxAREhMUFRYXGBkaGxwdHh8=\n";
        let cfg = ShadowsocksOutboundConfig::from_proxy(&parse_entry(yaml)).unwrap();
        assert_eq!(cfg.cipher, ShadowsocksCipher::Blake3Aes256Gcm);
        assert_eq!(cfg.key, (0u8..32).collect::<Vec<_>>());
    }

    #[test]
    fn parses_2022_aes_128_gcm_entry() {
        let yaml = "name: s\ntype: ss\nserver: h\nport: 1\ncipher: 2022-blake3-aes-128-gcm\npassword: AAECAwQFBgcICQoLDA0ODw==\n";
        let cfg = ShadowsocksOutboundConfig::from_proxy(&parse_entry(yaml)).unwrap();
        assert_eq!(cfg.cipher, ShadowsocksCipher::Blake3Aes128Gcm);
        assert_eq!(cfg.key, (0u8..16).collect::<Vec<_>>());
    }

    #[test]
    fn rejects_2022_psk_with_wrong_length() {
        // A 16-byte PSK for a 32-byte method must be rejected, not zero-padded.
        let yaml = "name: s\ntype: ss\nserver: h\nport: 1\ncipher: 2022-blake3-aes-256-gcm\npassword: AAECAwQFBgcICQoLDA0ODw==\n";
        let err = ShadowsocksOutboundConfig::from_proxy(&parse_entry(yaml)).unwrap_err();
        assert!(err.to_string().contains("PSK"), "got: {err}");
    }

    #[test]
    fn base64_decode_matches_rfc4648_vectors() {
        assert_eq!(base64_decode("").unwrap(), Vec::<u8>::new());
        assert_eq!(base64_decode("Zg==").unwrap(), b"f".to_vec());
        assert_eq!(base64_decode("Zm8=").unwrap(), b"fo".to_vec());
        assert_eq!(base64_decode("Zm9v").unwrap(), b"foo".to_vec());
        assert_eq!(base64_decode("Zm9vYmFy").unwrap(), b"foobar".to_vec());
        // Padding and whitespace are ignored.
        assert_eq!(base64_decode("Zm9v\nYmFy").unwrap(), b"foobar".to_vec());
        assert!(base64_decode("not base64!").is_err());
    }

    #[test]
    fn session_subkey_2022_is_blake3_derive_key() {
        let psk: Vec<u8> = (0u8..32).collect();
        let salt: Vec<u8> = (100u8..132).collect();
        let expected = blake3::derive_key(SS2022_SUBKEY_CONTEXT, &[psk.clone(), salt.clone()].concat());
        // 32-byte method uses the full derived key.
        assert_eq!(
            session_subkey(ShadowsocksCipher::Blake3Aes256Gcm, &psk, &salt),
            expected.to_vec()
        );
        // 16-byte method takes the leading 16 bytes of the same XOF output.
        let psk16: Vec<u8> = (0u8..16).collect();
        let expected16 = blake3::derive_key(SS2022_SUBKEY_CONTEXT, &[psk16.clone(), salt.clone()].concat());
        assert_eq!(
            session_subkey(ShadowsocksCipher::Blake3Aes128Gcm, &psk16, &salt),
            expected16[..16].to_vec()
        );
    }

    #[test]
    fn supported_plugin_is_parsed() {
        let yaml = "name: s\ntype: ss\nserver: h\nport: 1\ncipher: aes-128-gcm\npassword: p\nplugin: obfs\n";
        let cfg = ShadowsocksOutboundConfig::from_proxy(&parse_entry(yaml)).unwrap();
        assert!(matches!(cfg.plugin, Some(SsPlugin::ObfsHttp { .. })));
    }

    #[test]
    fn unsupported_plugin_is_rejected() {
        let yaml = "name: s\ntype: ss\nserver: h\nport: 1\ncipher: aes-128-gcm\npassword: p\nplugin: kcptun\n";
        let err = ShadowsocksOutboundConfig::from_proxy(&parse_entry(yaml)).unwrap_err();
        assert!(err.to_string().contains("not supported"), "got: {err}");
    }

    #[test]
    fn missing_cipher_is_rejected() {
        let yaml = "name: s\ntype: ss\nserver: h\nport: 1\npassword: p\n";
        let err = ShadowsocksOutboundConfig::from_proxy(&parse_entry(yaml)).unwrap_err();
        assert!(err.to_string().contains("cipher"), "got: {err}");
    }

    #[test]
    fn missing_password_is_rejected() {
        let yaml = "name: s\ntype: ss\nserver: h\nport: 1\ncipher: aes-128-gcm\n";
        let err = ShadowsocksOutboundConfig::from_proxy(&parse_entry(yaml)).unwrap_err();
        assert!(err.to_string().contains("password"), "got: {err}");
    }

    #[test]
    fn missing_server_is_rejected() {
        let yaml = "name: s\ntype: ss\nport: 1\ncipher: aes-128-gcm\npassword: p\n";
        let err = ShadowsocksOutboundConfig::from_proxy(&parse_entry(yaml)).unwrap_err();
        assert!(err.to_string().contains("server"), "got: {err}");
    }
}
