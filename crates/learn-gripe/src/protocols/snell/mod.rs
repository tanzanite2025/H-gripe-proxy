//! Snell outbound (TCP relay).
//!
//! Snell is Surge's closed-source proxy protocol. Its wire format is
//! reconstructed here from the reference `mihomo` implementation (this repo
//! migrates off mihomo, so it is the authority for the framing):
//!
//! * The session runs over plain TCP wrapped in the **Shadowsocks AEAD** chunk
//!   stream (the "shadowaead" framing) — `salt | chunk | chunk | …` where each
//!   `chunk = AEAD(len)(2+16) | AEAD(payload)(len+16)` with a 12-byte
//!   little-endian counter nonce starting at 0. The only departures from
//!   Shadowsocks-2017 are the fixed **16-byte salt** and the session subkey,
//!   which Snell derives with **Argon2id** (`argon2id(psk, salt, t=3, m=8 KiB,
//!   p=1, 32)` truncated to the cipher key length) instead of HKDF-SHA1.
//! * Cipher by protocol version: v1 uses ChaCha20-Poly1305 (32-byte key); v2/v3
//!   use AES-128-GCM (16-byte key).
//! * The first plaintext bytes the client sends are the Snell request header
//!   (`0x01 | command | clientID-len(0) | host-len | host | port(u16 BE)`); the
//!   first plaintext byte the server sends back is the command response
//!   (`Tunnel(0)` = ok, `Error(2)` = `code | msg-len | msg`).
//!
//! UDP (`CommandUDP`) is carried over the same shadowaead chunk stream as TCP
//! ([`SnellUdp`], v3 only): the handshake header becomes `0x01 | CommandUDP |
//! clientID-len(0)` and every datagram is one AEAD chunk whose plaintext is
//! `UDPForward(0x01) | addr | payload` (client->server) or `addr | payload`
//! (server->client). One chunk == one datagram, so the AEAD boundary preserves
//! packet boundaries.
//!
//! **Sequential session reuse** rides one TCP across logical streams: after a
//! stream finishes, both sides exchange Snell's half-close (a zero-length AEAD
//! chunk on v1-v3, a zero-payload frame on v4/v5) without closing the TCP, and
//! the next request rides the same connection with continuous cipher/nonce
//! state. Such connections are parked in a per-server pool (keyed by
//! `{server, port, version, psk, obfs}`) and preferred over a fresh dial,
//! mirroring the AnyTLS session registry. It is on for **v2** always, and for
//! **v4/v5** when `reuse` is configured (both negotiate `CommandConnectV2`);
//! v1/v3 are always one-shot.
//!
//! **v4/v5** replace the shadowaead chunk framing with a distinct framed
//! stream ([`SnellV4Stream`]) — v5 is identical on the wire (upstream maps a v5
//! config to a v4 client, since v5 servers are backward-compatible with v4
//! clients). It keeps the same Argon2id KDF / AES-128-GCM / counter nonce and
//! the same request-header + command-response handshake, but each frame is
//! `AEAD(7-byte header) | [padding] | AEAD(payload)` where the header is
//! `0x04 | 0 | 0 | padding-len(u16 BE) | payload-len(u16 BE)`. The first frame
//! is preceded by the 16-byte salt and carries an initial random padding block
//! (length in `[0x100, 0x200)`) byte-interleaved ("swapped") with the payload
//! ciphertext for traffic obfuscation; a `payload-len == 0` frame is the
//! logical EOF (`ErrZeroChunk`). v4/v5 UDP rides the same frame stream (one
//! frame per datagram, [`SnellV4Udp`]).

mod crypto;
mod pool;
mod stream;
mod udp;
mod v4;

#[cfg(test)]
mod tests;

use anyhow::{Context, Result, anyhow, bail};
use tokio::io::AsyncWriteExt;
use tokio::net::TcpStream;

use crate::address::TargetAddr;
use crate::config::outbound_opts::{ObfsOpts, ProxyEntry};
use crate::outbound::BoxedStream;
use crate::transport::simple_obfs;

pub use udp::SnellUdpAssoc;

use crypto::{AeadCipher, SnellCipher, random_bytes, snell_kdf};
use pool::{PooledSession, SnellServerKey, pool_take};
use stream::SnellStream;
use v4::SnellV4Stream;

/// Snell protocol byte that prefixes every client request header (constant 1).
const SNELL_PROTO_BYTE: u8 = 1;
/// Request command: open a one-shot TCP relay (no reuse).
const COMMAND_CONNECT: u8 = 1;
/// Request command: open a reuse-capable TCP relay (sent for v2).
const COMMAND_CONNECT_V2: u8 = 5;
/// Request command: relay UDP datagrams (UDP-over-TCP). Requires protocol v3.
const COMMAND_UDP: u8 = 6;
/// Per-packet command byte the client prefixes to each forwarded datagram.
const UDP_FORWARD: u8 = 1;
/// Reply address type: the server's source address is IPv4 (`type | 4B | port`).
const UDP_ADDR_IPV4: u8 = 4;
/// Reply address type: the server's source address is IPv6 (`type | 16B | port`).
const UDP_ADDR_IPV6: u8 = 6;
/// Response command: the relay tunnel was established.
const RESP_TUNNEL: u8 = 0;
/// Response command: the server rejected the request (`code | len | msg`).
const RESP_ERROR: u8 = 2;
/// Snell's salt is a fixed 16 bytes regardless of cipher key length.
const SALT_LEN: usize = 16;
/// AEAD tag length for both supported ciphers.
const TAG_LEN: usize = 16;
/// Largest plaintext carried in a single AEAD chunk (length field is capped at
/// 0x3FFF, matching the Shadowsocks framing Snell reuses).
const MAX_CHUNK: usize = 0x3fff;

/// simple-obfs (`obfs-opts`) transport that wraps the Snell shadowaead stream,
/// disguising it as innocuous HTTP or TLS 1.2 traffic. The framing is the same
/// one-shot-header simple-obfs the Shadowsocks plugin uses, applied beneath the
/// AEAD layer so it covers both TCP and UDP-over-TCP.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum SnellObfs {
    /// http mode: a fake WebSocket-upgrade request; `host`/`path` populate it.
    Http { host: String, path: String },
    /// tls mode: a fake TLS 1.2 handshake; `host` is sent as the SNI.
    Tls { host: String },
}

impl SnellObfs {
    /// Resolve `obfs-opts` into an obfs transport, or `None` when unset.
    /// Unknown modes are rejected so traffic is never silently mis-framed.
    fn parse(opts: Option<&ObfsOpts>) -> Result<Option<Self>> {
        let opts = match opts {
            None => return Ok(None),
            Some(o) => o,
        };
        let host = opts
            .host
            .clone()
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| "bing.com".to_string());
        match opts.mode.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
            None => Ok(None),
            Some("http") => Ok(Some(SnellObfs::Http {
                host,
                path: "/".to_string(),
            })),
            Some("tls") => Ok(Some(SnellObfs::Tls { host })),
            Some(other) => bail!("snell: unknown obfs mode {other:?} (use http or tls)"),
        }
    }

    /// Wrap an established TCP stream in the obfs framing.
    async fn wrap(&self, tcp: TcpStream) -> Result<BoxedStream> {
        match self {
            SnellObfs::Http { host, path } => Ok(Box::new(simple_obfs::connect_http(tcp, host, path).await?)),
            SnellObfs::Tls { host } => Ok(Box::new(simple_obfs::connect_tls(tcp, host).await?)),
        }
    }
}

/// Fully-resolved Snell outbound parameters.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SnellOutboundConfig {
    pub server: String,
    pub port: u16,
    /// Pre-shared key bytes (the `psk` string used verbatim as Argon2 input).
    pub psk: Vec<u8>,
    /// Protocol version (1..=5). v5 is normalised to v4 (identical on the wire;
    /// v5 servers are backward-compatible with v4 clients).
    pub version: u8,
    /// simple-obfs transport (`obfs-opts`), if any. `None` dials the raw socket.
    pub obfs: Option<SnellObfs>,
    /// Whether v4/v5 session reuse (`reuse`) is enabled: negotiate
    /// `CommandConnectV2` and ride the per-server connection pool.
    pub reuse: bool,
}

impl SnellOutboundConfig {
    /// Build an outbound config from a parsed `snell` proxy entry, rejecting
    /// versions / features that are not implemented so traffic is never
    /// mis-framed.
    pub fn from_proxy(entry: &ProxyEntry) -> Result<Self> {
        let opts = &entry.options;
        let server = opts
            .server
            .clone()
            .filter(|s| !s.is_empty())
            .context("snell: missing server")?;
        let port = opts.port.context("snell: missing port")?;
        let psk = opts
            .psk
            .as_deref()
            .filter(|s| !s.is_empty())
            .context("snell: missing psk")?
            .as_bytes()
            .to_vec();
        // mihomo defaults an unset `version` to 1, and normalises v5 to v4
        // (v5 servers accept v4 clients, so the wire framing is identical).
        let version = match opts.version.unwrap_or(1) {
            5 => 4,
            v @ 1..=4 => v as u8,
            other => bail!("snell: version {other} not supported (use 1..=5)"),
        };
        let obfs = SnellObfs::parse(opts.obfs_opts.as_ref())?;
        let reuse = opts.reuse.unwrap_or(false);
        Ok(Self {
            server,
            port,
            psk,
            version,
            obfs,
            reuse,
        })
    }

    fn cipher(&self) -> SnellCipher {
        match self.version {
            1 => SnellCipher::Chacha20Poly1305,
            _ => SnellCipher::Aes128Gcm,
        }
    }

    /// The request command: a reuse-capable session negotiates
    /// `CommandConnectV2` (so it can ride / be parked in the pool); a one-shot
    /// session uses the plain connect command. v2 is always reuse-capable;
    /// v4/v5 are when `reuse` is set. v1/v3 are always one-shot.
    fn command(&self) -> u8 {
        if self.reuse_capable() {
            COMMAND_CONNECT_V2
        } else {
            COMMAND_CONNECT
        }
    }

    /// Whether this outbound reuses one TCP connection across logical streams
    /// (`CommandConnectV2` + half-close + connection pool): v2 always, v4/v5
    /// when `reuse` is configured.
    fn reuse_capable(&self) -> bool {
        self.version == 2 || (self.uses_v4_framing() && self.reuse)
    }

    /// Whether this outbound can carry UDP. `CommandUDP` UDP-over-TCP is
    /// implemented for v3 (shadowaead chunk per datagram) and v4/v5 (one v4
    /// frame per datagram); v1/v2 carry TCP only and reject UDP.
    pub fn supports_udp(&self) -> bool {
        self.version >= 3
    }

    /// Whether this version uses the v4 frame stream instead of shadowaead.
    fn uses_v4_framing(&self) -> bool {
        self.version >= 4
    }
}

/// Connect a Snell outbound to `target` and return a relay-ready stream. A
/// reuse-capable outbound (v2, or v4/v5 with `reuse`) first tries to ride a
/// pooled session — writing only the new request header on the live stream —
/// and otherwise dials fresh; one-shot versions always dial. The salt (fresh
/// dials only) and the AEAD-sealed request header are sent before the stream is
/// handed back; the server's command response is consumed transparently on
/// first read.
pub async fn connect(config: &SnellOutboundConfig, target: &TargetAddr) -> Result<BoxedStream> {
    // v4/v5 use a distinct frame stream; the salt + initial padding ride the
    // first frame, so the request header is just the first write.
    if config.uses_v4_framing() {
        // With `reuse`, ride / park a pooled v4 session (CommandConnectV2 +
        // zero-payload-frame half-close), mirroring the v2 shadowaead pool.
        if config.reuse {
            let key = SnellServerKey::from_config(config);
            if let Some(PooledSession::V4(pooled)) = pool_take(&key) {
                let mut stream = SnellV4Stream::from_pooled(pooled, key);
                let header = build_request_header(COMMAND_CONNECT_V2, target)?;
                stream
                    .write_all(&header)
                    .await
                    .context("snell v4: send reuse request header")?;
                return Ok(Box::new(stream));
            }
            let transport = connect_transport(config).await?;
            let mut stream = SnellV4Stream::new(transport, config.psk.clone(), Some(key))?;
            let header = build_request_header(COMMAND_CONNECT_V2, target)?;
            stream
                .write_all(&header)
                .await
                .context("snell v4: send request header")?;
            return Ok(Box::new(stream));
        }
        let transport = connect_transport(config).await?;
        let mut stream = SnellV4Stream::new(transport, config.psk.clone(), None)?;
        let header = build_request_header(config.command(), target)?;
        stream
            .write_all(&header)
            .await
            .context("snell v4: send request header")?;
        return Ok(Box::new(stream));
    }

    // v2 is the reuse-capable shadowaead version (CommandConnectV2 + half-close).
    if config.version == 2 {
        let key = SnellServerKey::from_config(config);
        if let Some(PooledSession::Shadowaead(pooled)) = pool_take(&key) {
            let mut stream = SnellStream::from_pooled(pooled, key);
            let header = build_request_header(COMMAND_CONNECT_V2, target)?;
            stream
                .write_all(&header)
                .await
                .context("snell: send reuse request header")?;
            return Ok(Box::new(stream));
        }
        let mut stream = handshake(config, Some(key)).await?;
        let header = build_request_header(COMMAND_CONNECT_V2, target)?;
        stream.write_all(&header).await.context("snell: send request header")?;
        return Ok(Box::new(stream));
    }

    let mut stream = handshake(config, None).await?;
    let header = build_request_header(config.command(), target)?;
    stream.write_all(&header).await.context("snell: send request header")?;
    Ok(Box::new(stream))
}

/// Dial a fresh Snell session: open the (optionally obfuscated) transport, send
/// the client salt and derive the write cipher, returning a stream ready for its
/// request header. `reuse_key` (set for v2) marks it poolable on a clean close.
async fn handshake(config: &SnellOutboundConfig, reuse_key: Option<SnellServerKey>) -> Result<SnellStream> {
    let mut transport = connect_transport(config).await?;

    let cipher = config.cipher();
    let mut salt = [0u8; SALT_LEN];
    random_bytes(&mut salt);
    transport.write_all(&salt).await.context("snell: send salt")?;

    let subkey = snell_kdf(&config.psk, &salt, cipher.key_size());
    let write_cipher = AeadCipher::new(cipher, &subkey)?;

    Ok(SnellStream::new(
        transport,
        cipher,
        config.psk.clone(),
        write_cipher,
        reuse_key,
    ))
}

/// Dial `config.server:port` and wrap the socket in the configured simple-obfs
/// transport (if any), returning the byte stream the shadowaead layer runs over.
async fn connect_transport(config: &SnellOutboundConfig) -> Result<BoxedStream> {
    let tcp = TcpStream::connect((config.server.as_str(), config.port))
        .await
        .with_context(|| format!("snell: connect {}:{}", config.server, config.port))?;
    match &config.obfs {
        None => Ok(Box::new(tcp)),
        Some(obfs) => obfs
            .wrap(tcp)
            .await
            .with_context(|| format!("snell: obfs connect {}:{}", config.server, config.port)),
    }
}

/// Build the Snell request header:
/// `proto(1) | command(1) | clientID-len(0) | host-len(1) | host | port(u16 BE)`.
fn build_request_header(command: u8, target: &TargetAddr) -> Result<Vec<u8>> {
    let (host, port) = match target {
        TargetAddr::Domain(host, port) => (host.clone(), *port),
        TargetAddr::Ip(addr) => (addr.ip().to_string(), addr.port()),
    };
    let host_len = u8::try_from(host.len()).map_err(|_| anyhow!("snell: host longer than 255 bytes"))?;

    let mut header = Vec::with_capacity(1 + 1 + 1 + 1 + host.len() + 2);
    header.push(SNELL_PROTO_BYTE);
    header.push(command);
    header.push(0); // client ID length (unused)
    header.push(host_len);
    header.extend_from_slice(host.as_bytes());
    header.extend_from_slice(&port.to_be_bytes());
    Ok(header)
}
