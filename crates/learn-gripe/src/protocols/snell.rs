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

use std::collections::HashMap;
use std::io;
use std::net::SocketAddr;
use std::pin::Pin;
use std::sync::Mutex as StdMutex;
use std::task::{Context as TaskContext, Poll, ready};
use std::time::{Duration, Instant};

use aes_gcm::Aes128Gcm;
use aes_gcm::aead::generic_array::GenericArray;
use aes_gcm::aead::{Aead, KeyInit, Payload};
use anyhow::{Context, Result, anyhow, bail};
use argon2::{Algorithm, Argon2, Params, Version};
use chacha20poly1305::ChaCha20Poly1305;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt, ReadBuf, ReadHalf, WriteHalf};
use tokio::net::TcpStream;
use tokio::sync::Mutex;

use crate::address::TargetAddr;
use crate::config::outbound_opts::{ObfsOpts, ProxyEntry};
use crate::outbound::BoxedStream;
use crate::transport::simple_obfs;

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

/// v4 frame header plaintext: `0x04 | 0 | 0 | padding-len(u16 BE) | payload-len(u16 BE)`.
const V4_HEADER_PLAIN: usize = 7;
/// v4 sealed header is the 7-byte plaintext plus the AEAD tag.
const V4_HEADER_CIPHER: usize = V4_HEADER_PLAIN + TAG_LEN;
/// Marker byte at the start of every v4 frame header.
const V4_FRAME_BYTE: u8 = 4;
/// Inclusive lower bound for the random initial-padding block on the first frame.
const V4_INITIAL_PADDING_MIN: usize = 0x100;
/// Width of the initial-padding range; the length is `MIN + rand(0..SPAN)`.
const V4_INITIAL_PADDING_SPAN: usize = 0x100;

/// AEAD cipher selected by Snell protocol version.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SnellCipher {
    /// v1: ChaCha20-Poly1305 with a 32-byte key.
    Chacha20Poly1305,
    /// v2/v3: AES-128-GCM with a 16-byte key.
    Aes128Gcm,
}

impl SnellCipher {
    fn key_size(self) -> usize {
        match self {
            SnellCipher::Chacha20Poly1305 => 32,
            SnellCipher::Aes128Gcm => 16,
        }
    }
}

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

/// How long an idle reusable Snell session stays in the pool before it is
/// evicted (and its TCP closed) on the next access. Matches mihomo's snell pool
/// connection age (15s).
const SESSION_IDLE_TTL: Duration = Duration::from_secs(15);
/// Cap on idle reusable sessions kept per server endpoint, bounding fd/memory
/// use (mihomo's snell pool size).
const SESSION_POOL_MAX: usize = 10;

/// Identifies a Snell server endpoint for the reuse pool. A session is only
/// reusable for an identical endpoint *and* crypto/transport config (version,
/// psk and obfs all change the bytes on the wire), so all are part of the key.
#[derive(Clone, PartialEq, Eq, Hash)]
struct SnellServerKey {
    server: String,
    port: u16,
    version: u8,
    psk: Vec<u8>,
    obfs: Option<SnellObfs>,
}

impl SnellServerKey {
    fn from_config(config: &SnellOutboundConfig) -> Self {
        Self {
            server: config.server.clone(),
            port: config.port,
            version: config.version,
            psk: config.psk.clone(),
            obfs: config.obfs.clone(),
        }
    }
}

/// A live v1-v3 shadowaead session parked for sequential reuse: the established
/// transport plus the *continuous* shadowaead state (both ciphers and their
/// counter nonces keep advancing across logical streams). `read_cipher` is
/// always set because a session is only parked after its first stream consumed
/// the server salt.
struct PooledSnell {
    inner: BoxedStream,
    cipher: SnellCipher,
    psk: Vec<u8>,
    write_cipher: AeadCipher,
    write_nonce: [u8; 12],
    read_cipher: AeadCipher,
    read_nonce: [u8; 12],
    idle_since: Instant,
}

/// A live v4/v5 frame session parked for sequential reuse: the established
/// transport plus the continuous v4 frame state. The salt was already sent /
/// consumed on the first stream, so reads resume at a frame-header boundary and
/// no further salt or initial padding is emitted. v4 is always AES-128-GCM, so
/// (unlike [`PooledSnell`]) the cipher family is implied.
struct PooledSnellV4 {
    inner: BoxedStream,
    psk: Vec<u8>,
    write_cipher: AeadCipher,
    write_nonce: [u8; 12],
    read_cipher: AeadCipher,
    read_nonce: [u8; 12],
    idle_since: Instant,
}

/// A pooled idle session: a v1-v3 shadowaead session or a v4/v5 frame session.
/// The server key embeds the protocol version, so the two never share a bucket.
enum PooledSession {
    Shadowaead(PooledSnell),
    V4(PooledSnellV4),
}

impl PooledSession {
    fn idle_since(&self) -> Instant {
        match self {
            Self::Shadowaead(s) => s.idle_since,
            Self::V4(s) => s.idle_since,
        }
    }
}

/// Process-wide pool of idle reusable sessions, keyed by server endpoint, using
/// the same lazily-initialised `Mutex<Option<HashMap>>` idiom as the AnyTLS
/// session registry.
static SESSION_POOL: StdMutex<Option<HashMap<SnellServerKey, Vec<PooledSession>>>> = StdMutex::new(None);

/// Take a still-fresh idle session for `key`, dropping any that have outlived
/// [`SESSION_IDLE_TTL`]. `None` means a new connection must be dialled.
fn pool_take(key: &SnellServerKey) -> Option<PooledSession> {
    let mut guard = SESSION_POOL.lock().expect("snell session pool");
    let map = guard.as_mut()?;
    let list = map.get_mut(key)?;
    list.retain(|s| s.idle_since().elapsed() <= SESSION_IDLE_TTL);
    let taken = list.pop();
    if list.is_empty() {
        map.remove(key);
    }
    taken
}

/// Park a cleanly half-closed session for later reuse, bounded by
/// [`SESSION_POOL_MAX`]; over-capacity sessions are dropped (TCP closed).
fn pool_put(key: SnellServerKey, session: PooledSession) {
    let mut guard = SESSION_POOL.lock().expect("snell session pool");
    let map = guard.get_or_insert_with(HashMap::new);
    let list = map.entry(key).or_default();
    list.retain(|s| s.idle_since().elapsed() <= SESSION_IDLE_TTL);
    if list.len() < SESSION_POOL_MAX {
        list.push(session);
    }
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

/// A Snell UDP-over-TCP association (one per destination, mirroring the other
/// UDP egresses' `connect` / `send` / `recv` shape). It runs over the same
/// shadowaead chunk stream as TCP, but the handshake uses `CommandUDP` and each
/// datagram is a single AEAD chunk so packet boundaries survive. The TCP stream
/// is split so `send` and `recv` can run concurrently in the egress `select!`;
/// each half guards its own cipher + counter nonce behind a mutex.
pub struct SnellUdp {
    /// The fixed destination sealed into every packet sent on this association.
    target: TargetAddr,
    /// The Snell PSK, used to derive the read subkey from the server's salt.
    psk: Vec<u8>,
    /// The AEAD cipher family (v3 => AES-128-GCM).
    cipher: SnellCipher,
    write: Mutex<UdpWriteSide>,
    read: Mutex<UdpReadSide>,
}

struct UdpWriteSide {
    writer: WriteHalf<BoxedStream>,
    cipher: AeadCipher,
    nonce: [u8; 12],
}

struct UdpReadSide {
    reader: ReadHalf<BoxedStream>,
    /// Derived from the server's salt on the first `recv`; `None` until then.
    cipher: Option<AeadCipher>,
    nonce: [u8; 12],
    salt_done: bool,
}

impl SnellUdp {
    /// Open a Snell UDP association to `config.server` for datagrams destined to
    /// `target`. Sends the client salt and the `CommandUDP` handshake header
    /// (one AEAD chunk) before returning. Requires protocol v3.
    pub async fn connect(config: &SnellOutboundConfig, target: &TargetAddr) -> Result<Self> {
        if config.version != 3 {
            bail!("snell udp (shadowaead): requires version 3 (got v{})", config.version);
        }
        let cipher = config.cipher();
        let transport = connect_transport(config).await?;
        let (reader, mut writer) = tokio::io::split(transport);

        let mut salt = [0u8; SALT_LEN];
        random_bytes(&mut salt);
        writer.write_all(&salt).await.context("snell udp: send salt")?;

        let subkey = snell_kdf(&config.psk, &salt, cipher.key_size());
        let write_cipher = AeadCipher::new(cipher, &subkey)?;
        let mut write_nonce = [0u8; 12];

        // UDP handshake header: `proto(1) | CommandUDP | clientID-len(0)`; no
        // host/port (every datagram carries its own address).
        let header = [SNELL_PROTO_BYTE, COMMAND_UDP, 0];
        write_packet_chunk(&mut writer, &write_cipher, &mut write_nonce, &header)
            .await
            .context("snell udp: send handshake header")?;

        Ok(Self {
            target: target.clone(),
            psk: config.psk.clone(),
            cipher,
            write: Mutex::new(UdpWriteSide {
                writer,
                cipher: write_cipher,
                nonce: write_nonce,
            }),
            read: Mutex::new(UdpReadSide {
                reader,
                cipher: None,
                nonce: [0u8; 12],
                salt_done: false,
            }),
        })
    }

    /// Seal `payload` as one datagram (`UDPForward | addr | payload`) and send it
    /// to the server as a single AEAD chunk.
    pub async fn send(&self, payload: &[u8]) -> Result<()> {
        let mut plain = Vec::with_capacity(1 + 1 + 16 + 2 + payload.len());
        plain.push(UDP_FORWARD);
        encode_udp_addr(&mut plain, &self.target)?;
        plain.extend_from_slice(payload);
        if plain.len() > MAX_CHUNK {
            bail!(
                "snell udp: packet too large for one chunk ({} > {MAX_CHUNK})",
                plain.len()
            );
        }
        let mut w = self.write.lock().await;
        let UdpWriteSide { writer, cipher, nonce } = &mut *w;
        write_packet_chunk(writer, cipher, nonce, &plain).await
    }

    /// Receive one reply datagram (one AEAD chunk), strip the server's source
    /// address, and return the application payload.
    pub async fn recv(&self) -> Result<Vec<u8>> {
        let mut r = self.read.lock().await;
        if !r.salt_done {
            let mut salt = [0u8; SALT_LEN];
            r.reader.read_exact(&mut salt).await.context("snell udp: read salt")?;
            let subkey = snell_kdf(&self.psk, &salt, self.cipher.key_size());
            r.cipher = Some(AeadCipher::new(self.cipher, &subkey)?);
            r.salt_done = true;
        }
        let UdpReadSide {
            reader, cipher, nonce, ..
        } = &mut *r;
        let cipher = cipher.as_ref().ok_or_else(|| anyhow!("snell udp: read cipher unset"))?;
        let plain = read_packet_chunk(reader, cipher, nonce).await?;
        decode_udp_reply(&plain)
    }
}

/// A Snell **v4/v5** UDP-over-TCP association. The datagram framing (handshake
/// `CommandUDP` header, per-packet `UDPForward | addr | payload`, reply `addr |
/// payload`) is identical to v3; only the transport differs: each datagram is
/// carried as one v4 frame (`AEAD(header) | [padding] | AEAD(payload)`) instead
/// of a shadowaead chunk, the salt + initial padding ride the handshake frame,
/// and — unlike v3 — v4 sends a one-byte command response that is consumed
/// before the first reply datagram (mirroring upstream's `ReadReply`).
pub struct SnellV4Udp {
    /// The fixed destination sealed into every packet sent on this association.
    target: TargetAddr,
    write: Mutex<UdpV4WriteSide>,
    read: Mutex<UdpV4ReadSide>,
}

struct UdpV4WriteSide {
    writer: WriteHalf<BoxedStream>,
    cipher: AeadCipher,
    nonce: [u8; 12],
}

struct UdpV4ReadSide {
    reader: ReadHalf<BoxedStream>,
    psk: Vec<u8>,
    /// Derived from the server's salt on the first `recv`; `None` until then.
    cipher: Option<AeadCipher>,
    nonce: [u8; 12],
    salt_done: bool,
    /// Whether the v4 command-response byte has been consumed.
    reply_done: bool,
    /// A datagram that shared the reply frame (if the server coalesced the
    /// command byte with its first response), surfaced by the next `recv`.
    pending: Option<Vec<u8>>,
}

impl SnellV4Udp {
    /// Open a v4/v5 UDP association to `config.server` for datagrams destined to
    /// `target`. Sends the `CommandUDP` handshake header as the first v4 frame
    /// (carrying the client salt + initial padding). Requires v4/v5.
    pub async fn connect(config: &SnellOutboundConfig, target: &TargetAddr) -> Result<Self> {
        if !config.uses_v4_framing() {
            bail!("snell v4 udp: requires version >= 4 (got v{})", config.version);
        }
        let transport = connect_transport(config).await?;
        let (reader, mut writer) = tokio::io::split(transport);

        // v4 is always AES-128-GCM.
        let mut salt = [0u8; SALT_LEN];
        random_bytes(&mut salt);
        let subkey = snell_kdf(&config.psk, &salt, SnellCipher::Aes128Gcm.key_size());
        let write_cipher = AeadCipher::new(SnellCipher::Aes128Gcm, &subkey)?;
        let mut write_nonce = [0u8; 12];

        let mut delta = [0u8; 2];
        random_bytes(&mut delta);
        let initial_padding = V4_INITIAL_PADDING_MIN + (u16::from_le_bytes(delta) as usize) % V4_INITIAL_PADDING_SPAN;

        // UDP handshake header `proto | CommandUDP | clientID-len(0)` rides the
        // first v4 frame, which prepends the salt + initial padding.
        let header = [SNELL_PROTO_BYTE, COMMAND_UDP, 0];
        let frame = build_v4_frame(&write_cipher, &mut write_nonce, &header, initial_padding, Some(&salt))?;
        writer
            .write_all(&frame)
            .await
            .context("snell v4 udp: send handshake header")?;
        writer.flush().await.context("snell v4 udp: flush handshake header")?;

        Ok(Self {
            target: target.clone(),
            write: Mutex::new(UdpV4WriteSide {
                writer,
                cipher: write_cipher,
                nonce: write_nonce,
            }),
            read: Mutex::new(UdpV4ReadSide {
                reader,
                psk: config.psk.clone(),
                cipher: None,
                nonce: [0u8; 12],
                salt_done: false,
                reply_done: false,
                pending: None,
            }),
        })
    }

    /// Seal `payload` as one datagram (`UDPForward | addr | payload`) and send it
    /// as a single v4 frame (no padding: the salt already rode the handshake).
    pub async fn send(&self, payload: &[u8]) -> Result<()> {
        let mut plain = Vec::with_capacity(1 + 1 + 16 + 2 + payload.len());
        plain.push(UDP_FORWARD);
        encode_udp_addr(&mut plain, &self.target)?;
        plain.extend_from_slice(payload);
        if plain.len() > MAX_CHUNK {
            bail!(
                "snell v4 udp: packet too large for one frame ({} > {MAX_CHUNK})",
                plain.len()
            );
        }
        let mut w = self.write.lock().await;
        let UdpV4WriteSide { writer, cipher, nonce } = &mut *w;
        let frame = build_v4_frame(cipher, nonce, &plain, 0, None)?;
        writer.write_all(&frame).await.context("snell v4 udp: write datagram")?;
        writer.flush().await.context("snell v4 udp: flush datagram")?;
        Ok(())
    }

    /// Receive one reply datagram (one v4 frame), stripping the server's source
    /// address. Lazily reads the server salt and the one-byte command response
    /// on the first call.
    pub async fn recv(&self) -> Result<Vec<u8>> {
        let mut r = self.read.lock().await;
        if !r.salt_done {
            let mut salt = [0u8; SALT_LEN];
            r.reader
                .read_exact(&mut salt)
                .await
                .context("snell v4 udp: read salt")?;
            let subkey = snell_kdf(&r.psk, &salt, SnellCipher::Aes128Gcm.key_size());
            r.cipher = Some(AeadCipher::new(SnellCipher::Aes128Gcm, &subkey)?);
            r.salt_done = true;
        }

        if !r.reply_done {
            let frame = {
                let UdpV4ReadSide {
                    reader, cipher, nonce, ..
                } = &mut *r;
                let cipher = cipher
                    .as_ref()
                    .ok_or_else(|| anyhow!("snell v4 udp: read cipher unset"))?;
                read_v4_frame(reader, cipher, nonce).await?
            };
            match frame.first().copied() {
                Some(RESP_TUNNEL) => {}
                Some(RESP_ERROR) => bail!("snell v4 udp: server reported error"),
                Some(other) => bail!("snell v4 udp: unexpected command response {other}"),
                None => bail!("snell v4 udp: empty reply frame"),
            }
            if frame.len() > 1 {
                r.pending = Some(frame[1..].to_vec());
            }
            r.reply_done = true;
        }

        if let Some(pending) = r.pending.take() {
            return decode_udp_reply(&pending);
        }

        let frame = {
            let UdpV4ReadSide {
                reader, cipher, nonce, ..
            } = &mut *r;
            let cipher = cipher
                .as_ref()
                .ok_or_else(|| anyhow!("snell v4 udp: read cipher unset"))?;
            read_v4_frame(reader, cipher, nonce).await?
        };
        if frame.is_empty() {
            bail!("snell v4 udp: server closed the association");
        }
        decode_udp_reply(&frame)
    }
}

/// A Snell UDP association over either the v3 shadowaead stream or the v4/v5
/// frame stream, so the UDP egress loop can stay version-agnostic.
pub enum SnellUdpAssoc {
    V3(SnellUdp),
    V4(SnellV4Udp),
}

impl SnellUdpAssoc {
    /// Open the association, dispatching on the protocol version: v4/v5 use the
    /// v4 frame stream, v3 uses the shadowaead chunk stream.
    pub async fn connect(config: &SnellOutboundConfig, target: &TargetAddr) -> Result<Self> {
        if config.uses_v4_framing() {
            Ok(Self::V4(SnellV4Udp::connect(config, target).await?))
        } else {
            Ok(Self::V3(SnellUdp::connect(config, target).await?))
        }
    }

    pub async fn send(&self, payload: &[u8]) -> Result<()> {
        match self {
            Self::V3(assoc) => assoc.send(payload).await,
            Self::V4(assoc) => assoc.send(payload).await,
        }
    }

    pub async fn recv(&self) -> Result<Vec<u8>> {
        match self {
            Self::V3(assoc) => assoc.recv().await,
            Self::V4(assoc) => assoc.recv().await,
        }
    }
}

/// Seal `plaintext` as one length-prefixed AEAD chunk and write it (with a
/// flush) to `writer`, advancing the counter nonce twice.
async fn write_packet_chunk<W: AsyncWrite + Unpin>(
    writer: &mut W,
    cipher: &AeadCipher,
    nonce: &mut [u8; 12],
    plaintext: &[u8],
) -> Result<()> {
    let len = u16::try_from(plaintext.len()).map_err(|_| anyhow!("snell udp: chunk too large"))?;
    let sealed_len = cipher.seal(nonce, &len.to_be_bytes())?;
    increment_nonce(nonce);
    let sealed_payload = cipher.seal(nonce, plaintext)?;
    increment_nonce(nonce);

    let mut out = Vec::with_capacity(sealed_len.len() + sealed_payload.len());
    out.extend_from_slice(&sealed_len);
    out.extend_from_slice(&sealed_payload);
    writer.write_all(&out).await.context("snell udp: write chunk")?;
    writer.flush().await.context("snell udp: flush chunk")?;
    Ok(())
}

/// Read exactly one length-prefixed AEAD chunk and return its plaintext,
/// advancing the counter nonce twice.
async fn read_packet_chunk<R: AsyncRead + Unpin>(
    reader: &mut R,
    cipher: &AeadCipher,
    nonce: &mut [u8; 12],
) -> Result<Vec<u8>> {
    let mut sealed_len = [0u8; 2 + TAG_LEN];
    reader
        .read_exact(&mut sealed_len)
        .await
        .context("snell udp: read chunk length")?;
    let len_plain = cipher.open(nonce, &sealed_len)?;
    increment_nonce(nonce);
    let clen = u16::from_be_bytes([len_plain[0], len_plain[1]]) as usize;
    if clen == 0 || clen > MAX_CHUNK {
        bail!("snell udp: invalid chunk length {clen}");
    }
    let mut sealed = vec![0u8; clen + TAG_LEN];
    reader
        .read_exact(&mut sealed)
        .await
        .context("snell udp: read chunk payload")?;
    let plain = cipher.open(nonce, &sealed)?;
    increment_nonce(nonce);
    Ok(plain)
}

/// Encode a Snell UDP destination address into `buf`. The wire form differs from
/// SOCKS5: a domain is `len(1) | host | port(2 BE)`; an IP is `0x00 | family |
/// addr | port(2 BE)` where `family` is `4` (IPv4) or `6` (IPv6).
fn encode_udp_addr(buf: &mut Vec<u8>, target: &TargetAddr) -> Result<()> {
    match target {
        TargetAddr::Domain(host, port) => {
            let host_len = u8::try_from(host.len()).map_err(|_| anyhow!("snell udp: host longer than 255 bytes"))?;
            buf.push(host_len);
            buf.extend_from_slice(host.as_bytes());
            buf.extend_from_slice(&port.to_be_bytes());
        }
        TargetAddr::Ip(SocketAddr::V4(addr)) => {
            buf.push(0);
            buf.push(UDP_ADDR_IPV4);
            buf.extend_from_slice(&addr.ip().octets());
            buf.extend_from_slice(&addr.port().to_be_bytes());
        }
        TargetAddr::Ip(SocketAddr::V6(addr)) => {
            buf.push(0);
            buf.push(UDP_ADDR_IPV6);
            buf.extend_from_slice(&addr.ip().octets());
            buf.extend_from_slice(&addr.port().to_be_bytes());
        }
    }
    Ok(())
}

/// Strip the server's source address from a reply chunk and return the payload.
/// Replies are `type | addr | payload` with `type` = `4` (IPv4) or `6` (IPv6).
fn decode_udp_reply(plain: &[u8]) -> Result<Vec<u8>> {
    let kind = *plain.first().ok_or_else(|| anyhow!("snell udp: empty reply"))?;
    let payload_off = match kind {
        UDP_ADDR_IPV4 => 1 + 4 + 2,
        UDP_ADDR_IPV6 => 1 + 16 + 2,
        other => bail!("snell udp: unexpected reply address type {other}"),
    };
    if plain.len() < payload_off {
        bail!("snell udp: reply truncated");
    }
    Ok(plain[payload_off..].to_vec())
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

/// Snell's session-subkey KDF: `argon2id(psk, salt, t=3, m=8 KiB, p=1, 32)`
/// truncated to the cipher key length.
fn snell_kdf(psk: &[u8], salt: &[u8], key_size: usize) -> Vec<u8> {
    let params = Params::new(8, 3, 1, Some(32)).expect("valid snell argon2 params");
    let argon2 = Argon2::new(Algorithm::Argon2id, Version::V0x13, params);
    let mut out = [0u8; 32];
    argon2
        .hash_password_into(psk, salt, &mut out)
        .expect("snell argon2 kdf");
    out[..key_size].to_vec()
}

/// Fill `buf` with cryptographically secure random bytes from the OS.
fn random_bytes(buf: &mut [u8]) {
    if getrandom::fill(buf).is_err() {
        panic!("snell: system RNG unavailable");
    }
}

/// Increment a 12-byte little-endian counter nonce.
fn increment_nonce(nonce: &mut [u8; 12]) {
    for byte in nonce.iter_mut() {
        *byte = byte.wrapping_add(1);
        if *byte != 0 {
            break;
        }
    }
}

/// An AEAD cipher instance keyed with a per-session subkey.
enum AeadCipher {
    Aes128(Box<Aes128Gcm>),
    Chacha(Box<ChaCha20Poly1305>),
}

impl AeadCipher {
    fn new(cipher: SnellCipher, subkey: &[u8]) -> Result<Self> {
        match cipher {
            SnellCipher::Aes128Gcm => Ok(AeadCipher::Aes128(Box::new(
                Aes128Gcm::new_from_slice(subkey).map_err(|_| anyhow!("snell: invalid aes-128 key"))?,
            ))),
            SnellCipher::Chacha20Poly1305 => Ok(AeadCipher::Chacha(Box::new(
                ChaCha20Poly1305::new_from_slice(subkey).map_err(|_| anyhow!("snell: invalid chacha key"))?,
            ))),
        }
    }

    fn seal(&self, nonce: &[u8; 12], plaintext: &[u8]) -> Result<Vec<u8>> {
        let payload = Payload {
            msg: plaintext,
            aad: &[],
        };
        let result = match self {
            AeadCipher::Aes128(c) => c.encrypt(GenericArray::from_slice(nonce), payload),
            AeadCipher::Chacha(c) => c.encrypt(GenericArray::from_slice(nonce), payload),
        };
        result.map_err(|_| anyhow!("snell: AEAD seal failed"))
    }

    fn open(&self, nonce: &[u8; 12], ciphertext: &[u8]) -> Result<Vec<u8>> {
        let payload = Payload {
            msg: ciphertext,
            aad: &[],
        };
        let result = match self {
            AeadCipher::Aes128(c) => c.decrypt(GenericArray::from_slice(nonce), payload),
            AeadCipher::Chacha(c) => c.decrypt(GenericArray::from_slice(nonce), payload),
        };
        result.map_err(|_| anyhow!("snell: AEAD open failed"))
    }
}

/// Read-side framing state machine.
enum ReadState {
    /// Waiting for the server's 16-byte salt (derives the read cipher).
    Salt,
    /// Waiting for the 18-byte AEAD-sealed chunk length.
    Len,
    /// Waiting for a `clen + 16`-byte sealed payload chunk.
    Data(usize),
    /// Clean EOF (the peer closed the connection).
    Eof,
}

/// Wraps the raw TCP transport in the Snell AEAD chunk stream. Writes seal
/// application data into chunks; reads strip the server salt, derive the read
/// cipher, consume the one-byte command response, then decrypt length-prefixed
/// chunks. The client salt and sealed request header are sent at connect time.
struct SnellStream {
    /// `Option` only so [`Drop`] can move it out when parking the session for
    /// reuse; it is always `Some` during normal operation.
    inner: Option<BoxedStream>,
    cipher: SnellCipher,
    psk: Vec<u8>,
    // Write side.
    write_cipher: Option<AeadCipher>,
    write_nonce: [u8; 12],
    write_buf: Vec<u8>,
    write_pos: usize,
    // Read side.
    read_cipher: Option<AeadCipher>,
    read_nonce: [u8; 12],
    read_state: ReadState,
    read_raw: Vec<u8>,
    /// Whether the server's leading command-response byte has been consumed.
    reply_done: bool,
    plain: Vec<u8>,
    plain_pos: usize,
    /// Pool key if this stream may be parked for reuse (v2); `None` = one-shot.
    reuse_key: Option<SnellServerKey>,
    /// The server sent its zero-length chunk (a clean logical EOF, distinct from
    /// a transport close), so the session can be reused.
    read_saw_zero: bool,
    /// We sent our zero-length chunk (half-close) on shutdown.
    write_closed: bool,
}

impl SnellStream {
    fn new(
        inner: BoxedStream,
        cipher: SnellCipher,
        psk: Vec<u8>,
        write_cipher: AeadCipher,
        reuse_key: Option<SnellServerKey>,
    ) -> Self {
        Self {
            inner: Some(inner),
            cipher,
            psk,
            write_cipher: Some(write_cipher),
            write_nonce: [0u8; 12],
            write_buf: Vec::new(),
            write_pos: 0,
            read_cipher: None,
            read_nonce: [0u8; 12],
            read_state: ReadState::Salt,
            read_raw: Vec::new(),
            reply_done: false,
            plain: Vec::new(),
            plain_pos: 0,
            reuse_key,
            read_saw_zero: false,
            write_closed: false,
        }
    }

    /// Rebuild a stream on a pooled (reused) session: the salt was consumed on
    /// the first stream so reads resume at a chunk length boundary with the
    /// session's continuous ciphers/nonces, and the server sends a fresh
    /// command-response byte for this request (`reply_done` reset).
    fn from_pooled(pooled: PooledSnell, key: SnellServerKey) -> Self {
        Self {
            inner: Some(pooled.inner),
            cipher: pooled.cipher,
            psk: pooled.psk,
            write_cipher: Some(pooled.write_cipher),
            write_nonce: pooled.write_nonce,
            write_buf: Vec::new(),
            write_pos: 0,
            read_cipher: Some(pooled.read_cipher),
            read_nonce: pooled.read_nonce,
            read_state: ReadState::Len,
            read_raw: Vec::new(),
            reply_done: false,
            plain: Vec::new(),
            plain_pos: 0,
            reuse_key: Some(key),
            read_saw_zero: false,
            write_closed: false,
        }
    }

    /// Flush any pending sealed bytes to the inner stream.
    fn poll_drain(&mut self, cx: &mut TaskContext<'_>) -> Poll<io::Result<()>> {
        let inner = self.inner.as_mut().expect("snell stream inner");
        while self.write_pos < self.write_buf.len() {
            let n = ready!(Pin::new(&mut *inner).poll_write(cx, &self.write_buf[self.write_pos..]))?;
            if n == 0 {
                return Poll::Ready(Err(io::Error::new(io::ErrorKind::WriteZero, "snell: write zero")));
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
            .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "snell: chunk too large"))?;
        let cipher = self.write_cipher.as_ref().expect("snell write cipher");
        let sealed_len = cipher
            .seal(&self.write_nonce, &len.to_be_bytes())
            .map_err(|e| io::Error::other(e.to_string()))?;
        increment_nonce(&mut self.write_nonce);
        let cipher = self.write_cipher.as_ref().expect("snell write cipher");
        let sealed_payload = cipher
            .seal(&self.write_nonce, plaintext)
            .map_err(|e| io::Error::other(e.to_string()))?;
        increment_nonce(&mut self.write_nonce);

        self.write_buf.clear();
        self.write_pos = 0;
        self.write_buf.extend_from_slice(&sealed_len);
        self.write_buf.extend_from_slice(&sealed_payload);
        Ok(())
    }

    /// Queue Snell's half-close: a single zero-length AEAD chunk — only the
    /// sealed length field `0x0000` (one nonce step), no payload block — matching
    /// mihomo's `writeZeroChunk` / shadowaead empty write. The peer decrypts the
    /// length, sees `0`, and treats it as a logical EOF (its `ErrZeroChunk`).
    fn queue_zero_chunk(&mut self) -> io::Result<()> {
        let cipher = self.write_cipher.as_ref().expect("snell write cipher");
        let sealed_len = cipher
            .seal(&self.write_nonce, &[0u8, 0u8])
            .map_err(|e| io::Error::other(e.to_string()))?;
        increment_nonce(&mut self.write_nonce);
        self.write_buf.clear();
        self.write_pos = 0;
        self.write_buf.extend_from_slice(&sealed_len);
        Ok(())
    }

    /// Strip the leading command-response byte from freshly-decrypted plaintext,
    /// returning an error if the server reported `RESP_ERROR`.
    fn consume_reply(&mut self) -> io::Result<()> {
        // `plain` holds the just-decrypted chunk starting at `plain_pos`.
        if self.plain_pos >= self.plain.len() {
            return Ok(());
        }
        self.plain_pos += consume_command_reply(&self.plain[self.plain_pos..])?;
        self.reply_done = true;
        Ok(())
    }
}

/// Inspect the server's leading command-response byte and return the offset of
/// the application data that follows it (`Tunnel` = 1), or an error if the
/// server reported `RESP_ERROR` / an unknown command. Shared by the shadowaead
/// ([`SnellStream`]) and v4 ([`SnellV4Stream`]) read paths.
fn consume_command_reply(plain: &[u8]) -> io::Result<usize> {
    match plain.first() {
        None => Ok(0),
        Some(&RESP_TUNNEL) => Ok(1),
        Some(&RESP_ERROR) => {
            // `code | msg-len | msg`; best-effort decode for diagnostics
            // (the connection fails regardless of how much is buffered).
            let rest = &plain[1..];
            let code = rest.first().copied().unwrap_or(0);
            let msg = rest
                .get(2..)
                .map(|m| String::from_utf8_lossy(m).into_owned())
                .unwrap_or_default();
            Err(io::Error::new(
                io::ErrorKind::ConnectionRefused,
                format!("snell: server error code {code}: {msg}"),
            ))
        }
        Some(&other) => Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("snell: unexpected command response {other}"),
        )),
    }
}

/// v4's initial-padding obfuscation: swap every even-indexed byte between the
/// padding block and the payload ciphertext, up to the shorter of the two. It
/// is its own inverse, so the reader applies the same swap to recover the
/// payload ciphertext before decrypting.
fn swap_padding(padding: &mut [u8], payload_cipher: &mut [u8]) {
    let limit = padding.len().min(payload_cipher.len());
    let mut i = 0;
    while i < limit {
        std::mem::swap(&mut padding[i], &mut payload_cipher[i]);
        i += 2;
    }
}

/// Read-side framing state for the v4 frame stream.
enum V4ReadState {
    /// Waiting for the server's 16-byte salt (derives the read cipher).
    Salt,
    /// Waiting for the 23-byte AEAD-sealed frame header.
    Header,
    /// Waiting for `padding + payload + TAG_LEN` bytes of frame body.
    Body { padding: usize, payload: usize },
    /// Clean EOF (a zero-payload frame or a transport close).
    Eof,
}

/// Wraps the raw transport in the Snell **v4** frame stream (v4/v5). Like
/// [`SnellStream`] it carries the request header / command response and uses
/// Argon2id + AES-128-GCM + a counter nonce, but each frame is
/// `AEAD(header) | [padding] | AEAD(payload)`; the first frame is prefixed with
/// the client salt and an initial random padding block (see the module docs).
struct SnellV4Stream {
    /// `Option` only so [`Drop`] can move it out when parking the session for
    /// reuse; always `Some` during normal operation.
    inner: Option<BoxedStream>,
    psk: Vec<u8>,
    // Write side.
    /// Same as `inner`: `Option` only so a reusable session can be parked.
    write_cipher: Option<AeadCipher>,
    write_salt: [u8; SALT_LEN],
    write_nonce: [u8; 12],
    /// Whether the salt (and, with it, the first frame's initial padding) has
    /// been emitted; gates the one-time salt prefix and initial padding.
    salt_sent: bool,
    initial_padding: usize,
    write_buf: Vec<u8>,
    write_pos: usize,
    // Read side.
    read_cipher: Option<AeadCipher>,
    read_nonce: [u8; 12],
    read_state: V4ReadState,
    read_raw: Vec<u8>,
    reply_done: bool,
    plain: Vec<u8>,
    plain_pos: usize,
    /// Pool key if this stream may be parked for reuse (v4/v5 + `reuse`);
    /// `None` = one-shot.
    reuse_key: Option<SnellServerKey>,
    /// The server sent its zero-payload frame (a clean logical EOF, distinct
    /// from a transport close), so the session can be reused.
    read_saw_zero: bool,
    /// We sent our zero-payload frame (half-close) on shutdown.
    write_closed: bool,
}

impl SnellV4Stream {
    fn new(inner: BoxedStream, psk: Vec<u8>, reuse_key: Option<SnellServerKey>) -> Result<Self> {
        // v4 is always AES-128-GCM.
        let mut salt = [0u8; SALT_LEN];
        random_bytes(&mut salt);
        let subkey = snell_kdf(&psk, &salt, SnellCipher::Aes128Gcm.key_size());
        let write_cipher = AeadCipher::new(SnellCipher::Aes128Gcm, &subkey)?;

        let mut delta = [0u8; 2];
        random_bytes(&mut delta);
        let initial_padding = V4_INITIAL_PADDING_MIN + (u16::from_le_bytes(delta) as usize) % V4_INITIAL_PADDING_SPAN;

        Ok(Self {
            inner: Some(inner),
            psk,
            write_cipher: Some(write_cipher),
            write_salt: salt,
            write_nonce: [0u8; 12],
            salt_sent: false,
            initial_padding,
            write_buf: Vec::new(),
            write_pos: 0,
            read_cipher: None,
            read_nonce: [0u8; 12],
            read_state: V4ReadState::Salt,
            read_raw: Vec::new(),
            reply_done: false,
            plain: Vec::new(),
            plain_pos: 0,
            reuse_key,
            read_saw_zero: false,
            write_closed: false,
        })
    }

    /// Rebuild a stream on a pooled (reused) v4 session: the salt was already
    /// sent / consumed on the first stream, so writes emit no salt or initial
    /// padding and reads resume at a frame-header boundary with the session's
    /// continuous ciphers/nonces; the server sends a fresh command-response byte
    /// for this request (`reply_done` reset).
    fn from_pooled(pooled: PooledSnellV4, key: SnellServerKey) -> Self {
        Self {
            inner: Some(pooled.inner),
            psk: pooled.psk,
            write_cipher: Some(pooled.write_cipher),
            write_salt: [0u8; SALT_LEN],
            write_nonce: pooled.write_nonce,
            salt_sent: true,
            initial_padding: 0,
            write_buf: Vec::new(),
            write_pos: 0,
            read_cipher: Some(pooled.read_cipher),
            read_nonce: pooled.read_nonce,
            read_state: V4ReadState::Header,
            read_raw: Vec::new(),
            reply_done: false,
            plain: Vec::new(),
            plain_pos: 0,
            reuse_key: Some(key),
            read_saw_zero: false,
            write_closed: false,
        }
    }

    /// Flush any pending sealed bytes to the inner stream.
    fn poll_drain(&mut self, cx: &mut TaskContext<'_>) -> Poll<io::Result<()>> {
        let inner = self.inner.as_mut().expect("snell v4 stream inner");
        while self.write_pos < self.write_buf.len() {
            let n = ready!(Pin::new(&mut *inner).poll_write(cx, &self.write_buf[self.write_pos..]))?;
            if n == 0 {
                return Poll::Ready(Err(io::Error::new(io::ErrorKind::WriteZero, "snell v4: write zero")));
            }
            self.write_pos += n;
        }
        self.write_buf.clear();
        self.write_pos = 0;
        Poll::Ready(Ok(()))
    }

    /// Seal `payload` (at most [`MAX_CHUNK`] bytes) into one v4 frame queued for
    /// writing. The first frame prepends the salt and an initial padding block
    /// (interleaved with the payload ciphertext via [`swap_padding`]).
    fn queue_frame(&mut self, payload: &[u8]) -> io::Result<()> {
        let first = !self.salt_sent;
        let padding_len = if first && !payload.is_empty() {
            self.initial_padding
        } else {
            0
        };
        let salt = if first { Some(&self.write_salt[..]) } else { None };
        let cipher = self.write_cipher.as_ref().expect("snell v4 write cipher");
        let frame = build_v4_frame(cipher, &mut self.write_nonce, payload, padding_len, salt)
            .map_err(|e| io::Error::other(e.to_string()))?;
        self.salt_sent = true;
        self.write_buf = frame;
        self.write_pos = 0;
        Ok(())
    }

    /// Queue Snell's v4 half-close: a single zero-payload frame (only the sealed
    /// header, `payLen == 0`, no padding, one nonce step). The peer decodes the
    /// header, sees a zero payload, and treats it as a logical EOF
    /// (`ErrZeroChunk`). Never prepends the salt — a reusable stream has already
    /// sent it.
    fn queue_zero_frame(&mut self) -> io::Result<()> {
        let cipher = self.write_cipher.as_ref().expect("snell v4 write cipher");
        let frame =
            build_v4_frame(cipher, &mut self.write_nonce, &[], 0, None).map_err(|e| io::Error::other(e.to_string()))?;
        self.salt_sent = true;
        self.write_buf = frame;
        self.write_pos = 0;
        Ok(())
    }
}

/// Serialise one v4 frame `AEAD(header) | [padding] | AEAD(payload)`, advancing
/// `nonce` once per AEAD seal. When `salt` is `Some` (the first frame on a
/// stream) it is prepended and `padding_len` random padding bytes are
/// interleaved with the payload ciphertext via [`swap_padding`]. Shared by the
/// v4 TCP stream ([`SnellV4Stream`]) and the v4 UDP path ([`SnellV4Udp`]).
fn build_v4_frame(
    cipher: &AeadCipher,
    nonce: &mut [u8; 12],
    payload: &[u8],
    padding_len: usize,
    salt: Option<&[u8]>,
) -> Result<Vec<u8>> {
    let mut header = [0u8; V4_HEADER_PLAIN];
    header[0] = V4_FRAME_BYTE;
    header[3..5].copy_from_slice(&(padding_len as u16).to_be_bytes());
    header[5..7].copy_from_slice(&(payload.len() as u16).to_be_bytes());
    let sealed_header = cipher.seal(nonce, &header)?;
    increment_nonce(nonce);

    let mut payload_cipher = if payload.is_empty() {
        Vec::new()
    } else {
        let pc = cipher.seal(nonce, payload)?;
        increment_nonce(nonce);
        pc
    };

    let mut out = Vec::new();
    if let Some(salt) = salt {
        out.extend_from_slice(salt);
    }
    out.extend_from_slice(&sealed_header);
    if padding_len > 0 {
        let mut padding = vec![0u8; padding_len];
        random_bytes(&mut padding);
        swap_padding(&mut padding, &mut payload_cipher);
        out.extend_from_slice(&padding);
    }
    out.extend_from_slice(&payload_cipher);
    Ok(out)
}

/// Read exactly one v4 frame from `reader` and return its decrypted payload,
/// advancing `nonce` once per AEAD open. A zero-payload frame (Snell's logical
/// EOF) returns an empty `Vec`.
async fn read_v4_frame<R: AsyncRead + Unpin>(
    reader: &mut R,
    cipher: &AeadCipher,
    nonce: &mut [u8; 12],
) -> Result<Vec<u8>> {
    let mut header_cipher = [0u8; V4_HEADER_CIPHER];
    reader
        .read_exact(&mut header_cipher)
        .await
        .context("snell v4 udp: read frame header")?;
    let header = cipher.open(nonce, &header_cipher)?;
    increment_nonce(nonce);
    if header.len() != V4_HEADER_PLAIN || header[0] != V4_FRAME_BYTE {
        bail!("snell v4 udp: invalid frame header");
    }
    let padding = u16::from_be_bytes([header[3], header[4]]) as usize;
    let payload = u16::from_be_bytes([header[5], header[6]]) as usize;
    if payload == 0 {
        return Ok(Vec::new());
    }
    if payload > MAX_CHUNK || padding > MAX_CHUNK {
        bail!("snell v4 udp: frame too large");
    }
    let mut frame = vec![0u8; padding + payload + TAG_LEN];
    reader
        .read_exact(&mut frame)
        .await
        .context("snell v4 udp: read frame body")?;
    if padding > 0 {
        let (pad_part, pay_part) = frame.split_at_mut(padding);
        swap_padding(pad_part, pay_part);
    }
    let plain = cipher.open(nonce, &frame[padding..])?;
    increment_nonce(nonce);
    Ok(plain)
}

impl AsyncRead for SnellV4Stream {
    fn poll_read(self: Pin<&mut Self>, cx: &mut TaskContext<'_>, buf: &mut ReadBuf<'_>) -> Poll<io::Result<()>> {
        let this = self.get_mut();
        loop {
            if this.plain_pos < this.plain.len() {
                let n = buf.remaining().min(this.plain.len() - this.plain_pos);
                buf.put_slice(&this.plain[this.plain_pos..this.plain_pos + n]);
                this.plain_pos += n;
                return Poll::Ready(Ok(()));
            }
            if matches!(this.read_state, V4ReadState::Eof) {
                return Poll::Ready(Ok(()));
            }

            let need = match this.read_state {
                V4ReadState::Salt => SALT_LEN,
                V4ReadState::Header => V4_HEADER_CIPHER,
                V4ReadState::Body { padding, payload } => padding + payload + TAG_LEN,
                V4ReadState::Eof => unreachable!(),
            };

            if this.read_raw.len() < need {
                let mut scratch = [0u8; 4096];
                let mut read_buf = ReadBuf::new(&mut scratch);
                let inner = this.inner.as_mut().expect("snell v4 stream inner");
                ready!(Pin::new(inner).poll_read(cx, &mut read_buf))?;
                let filled = read_buf.filled();
                if filled.is_empty() {
                    this.read_state = V4ReadState::Eof;
                    return Poll::Ready(Ok(()));
                }
                this.read_raw.extend_from_slice(filled);
                continue;
            }

            match this.read_state {
                V4ReadState::Salt => {
                    let salt: Vec<u8> = this.read_raw.drain(..SALT_LEN).collect();
                    let subkey = snell_kdf(&this.psk, &salt, SnellCipher::Aes128Gcm.key_size());
                    let cipher = AeadCipher::new(SnellCipher::Aes128Gcm, &subkey).map_err(decrypt_err)?;
                    this.read_cipher = Some(cipher);
                    this.read_state = V4ReadState::Header;
                }
                V4ReadState::Header => {
                    let sealed: Vec<u8> = this.read_raw.drain(..V4_HEADER_CIPHER).collect();
                    let Some(cipher) = this.read_cipher.as_ref() else {
                        return Poll::Ready(Err(read_cipher_unset()));
                    };
                    let header = cipher.open(&this.read_nonce, &sealed).map_err(decrypt_err)?;
                    increment_nonce(&mut this.read_nonce);
                    if header.len() != V4_HEADER_PLAIN || header[0] != V4_FRAME_BYTE {
                        return Poll::Ready(Err(io::Error::new(
                            io::ErrorKind::InvalidData,
                            "snell v4: invalid frame header",
                        )));
                    }
                    let padding = u16::from_be_bytes([header[3], header[4]]) as usize;
                    let payload = u16::from_be_bytes([header[5], header[6]]) as usize;
                    if payload == 0 {
                        if padding != 0 {
                            return Poll::Ready(Err(io::Error::new(
                                io::ErrorKind::InvalidData,
                                "snell v4: zero chunk with padding",
                            )));
                        }
                        // Zero-payload frame = Snell's logical EOF (ErrZeroChunk):
                        // a clean half-close on a (reusable) stream, distinct
                        // from a transport close.
                        this.read_saw_zero = true;
                        this.read_state = V4ReadState::Eof;
                        return Poll::Ready(Ok(()));
                    }
                    if payload > MAX_CHUNK || padding > MAX_CHUNK {
                        return Poll::Ready(Err(io::Error::new(
                            io::ErrorKind::InvalidData,
                            "snell v4: frame too large",
                        )));
                    }
                    this.read_state = V4ReadState::Body { padding, payload };
                }
                V4ReadState::Body { padding, payload } => {
                    let mut frame: Vec<u8> = this.read_raw.drain(..padding + payload + TAG_LEN).collect();
                    if padding > 0 {
                        let (pad_part, pay_part) = frame.split_at_mut(padding);
                        swap_padding(pad_part, pay_part);
                    }
                    let Some(cipher) = this.read_cipher.as_ref() else {
                        return Poll::Ready(Err(read_cipher_unset()));
                    };
                    let plain = cipher.open(&this.read_nonce, &frame[padding..]).map_err(decrypt_err)?;
                    increment_nonce(&mut this.read_nonce);
                    this.plain = plain;
                    this.plain_pos = 0;
                    this.read_state = V4ReadState::Header;
                    if !this.reply_done {
                        this.plain_pos = consume_command_reply(&this.plain)?;
                        this.reply_done = true;
                    }
                }
                V4ReadState::Eof => unreachable!(),
            }
        }
    }
}

impl AsyncWrite for SnellV4Stream {
    fn poll_write(self: Pin<&mut Self>, cx: &mut TaskContext<'_>, buf: &[u8]) -> Poll<io::Result<usize>> {
        let this = self.get_mut();
        ready!(this.poll_drain(cx))?;
        if buf.is_empty() {
            return Poll::Ready(Ok(0));
        }
        let take = buf.len().min(MAX_CHUNK);
        this.queue_frame(&buf[..take])?;
        if let Poll::Ready(Err(e)) = this.poll_drain(cx) {
            return Poll::Ready(Err(e));
        }
        Poll::Ready(Ok(take))
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut TaskContext<'_>) -> Poll<io::Result<()>> {
        let this = self.get_mut();
        ready!(this.poll_drain(cx))?;
        Pin::new(this.inner.as_mut().expect("snell v4 stream inner")).poll_flush(cx)
    }

    fn poll_shutdown(self: Pin<&mut Self>, cx: &mut TaskContext<'_>) -> Poll<io::Result<()>> {
        let this = self.get_mut();
        // On a reuse-capable stream, shutdown means *half-close*: send our
        // zero-payload frame so the peer ends this logical stream, then flush
        // but keep the TCP open — the session returns to the pool on drop.
        if this.reuse_key.is_some() {
            ready!(this.poll_drain(cx))?;
            if !this.write_closed {
                this.queue_zero_frame()?;
                this.write_closed = true;
            }
            ready!(this.poll_drain(cx))?;
            return Pin::new(this.inner.as_mut().expect("snell v4 stream inner")).poll_flush(cx);
        }
        ready!(this.poll_drain(cx))?;
        Pin::new(this.inner.as_mut().expect("snell v4 stream inner")).poll_shutdown(cx)
    }
}

impl Drop for SnellV4Stream {
    fn drop(&mut self) {
        let Some(key) = self.reuse_key.take() else {
            return;
        };
        // Only park a session that half-closed cleanly in both directions and
        // carries no buffered/leftover bytes, so the next stream starts on a
        // clean frame boundary with continuous nonces; otherwise the TCP is
        // closed by dropping the fields.
        if !(self.read_saw_zero && self.write_closed) {
            return;
        }
        if self.write_pos < self.write_buf.len() || !self.read_raw.is_empty() || self.plain_pos < self.plain.len() {
            return;
        }
        let (Some(inner), Some(write_cipher), Some(read_cipher)) =
            (self.inner.take(), self.write_cipher.take(), self.read_cipher.take())
        else {
            return;
        };
        pool_put(
            key,
            PooledSession::V4(PooledSnellV4 {
                inner,
                psk: std::mem::take(&mut self.psk),
                write_cipher,
                write_nonce: self.write_nonce,
                read_cipher,
                read_nonce: self.read_nonce,
                idle_since: Instant::now(),
            }),
        );
    }
}

fn decrypt_err(e: anyhow::Error) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, e.to_string())
}

fn read_cipher_unset() -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, "snell: read cipher unset")
}

impl AsyncRead for SnellStream {
    fn poll_read(self: Pin<&mut Self>, cx: &mut TaskContext<'_>, buf: &mut ReadBuf<'_>) -> Poll<io::Result<()>> {
        let this = self.get_mut();
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
                ReadState::Salt => SALT_LEN,
                ReadState::Len => 2 + TAG_LEN,
                ReadState::Data(clen) => clen + TAG_LEN,
                ReadState::Eof => unreachable!(),
            };

            if this.read_raw.len() < need {
                let mut scratch = [0u8; 4096];
                let mut read_buf = ReadBuf::new(&mut scratch);
                let inner = this.inner.as_mut().expect("snell stream inner");
                ready!(Pin::new(inner).poll_read(cx, &mut read_buf))?;
                let filled = read_buf.filled();
                if filled.is_empty() {
                    // Snell (like Shadowsocks) signals end-of-stream by closing
                    // the TCP connection, with no terminating chunk.
                    this.read_state = ReadState::Eof;
                    return Poll::Ready(Ok(()));
                }
                this.read_raw.extend_from_slice(filled);
                continue;
            }

            match this.read_state {
                ReadState::Salt => {
                    let salt: Vec<u8> = this.read_raw.drain(..SALT_LEN).collect();
                    let subkey = snell_kdf(&this.psk, &salt, this.cipher.key_size());
                    let cipher = AeadCipher::new(this.cipher, &subkey).map_err(decrypt_err)?;
                    this.read_cipher = Some(cipher);
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
                    if clen == 0 {
                        // Zero-length chunk = Snell half-close: a clean logical
                        // EOF on a (reusable) stream, not a transport close.
                        this.read_saw_zero = true;
                        this.read_state = ReadState::Eof;
                        return Poll::Ready(Ok(()));
                    }
                    if clen > MAX_CHUNK {
                        return Poll::Ready(Err(io::Error::new(
                            io::ErrorKind::InvalidData,
                            "snell: invalid chunk length",
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
                    if !this.reply_done {
                        this.consume_reply()?;
                    }
                }
                ReadState::Eof => unreachable!(),
            }
        }
    }
}

impl AsyncWrite for SnellStream {
    fn poll_write(self: Pin<&mut Self>, cx: &mut TaskContext<'_>, buf: &[u8]) -> Poll<io::Result<usize>> {
        let this = self.get_mut();
        ready!(this.poll_drain(cx))?;
        if buf.is_empty() {
            return Poll::Ready(Ok(0));
        }
        let take = buf.len().min(MAX_CHUNK);
        this.queue_chunk(&buf[..take])?;
        if let Poll::Ready(Err(e)) = this.poll_drain(cx) {
            return Poll::Ready(Err(e));
        }
        Poll::Ready(Ok(take))
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut TaskContext<'_>) -> Poll<io::Result<()>> {
        let this = self.get_mut();
        ready!(this.poll_drain(cx))?;
        Pin::new(this.inner.as_mut().expect("snell stream inner")).poll_flush(cx)
    }

    fn poll_shutdown(self: Pin<&mut Self>, cx: &mut TaskContext<'_>) -> Poll<io::Result<()>> {
        let this = self.get_mut();
        // On a reuse-capable stream, shutdown means *half-close*: send our
        // zero-length chunk so the peer ends this logical stream, then flush but
        // keep the TCP open — the session returns to the pool on drop.
        if this.reuse_key.is_some() {
            ready!(this.poll_drain(cx))?;
            if !this.write_closed {
                this.queue_zero_chunk()?;
                this.write_closed = true;
            }
            ready!(this.poll_drain(cx))?;
            return Pin::new(this.inner.as_mut().expect("snell stream inner")).poll_flush(cx);
        }
        ready!(this.poll_drain(cx))?;
        Pin::new(this.inner.as_mut().expect("snell stream inner")).poll_shutdown(cx)
    }
}

impl Drop for SnellStream {
    fn drop(&mut self) {
        let Some(key) = self.reuse_key.take() else {
            return;
        };
        // Only park a session that half-closed cleanly in both directions and
        // carries no buffered/leftover bytes, so the next stream starts on a
        // clean chunk boundary with continuous nonces; otherwise the TCP is
        // closed by dropping the fields.
        if !(self.read_saw_zero && self.write_closed) {
            return;
        }
        if self.write_pos < self.write_buf.len() || !self.read_raw.is_empty() || self.plain_pos < self.plain.len() {
            return;
        }
        let (Some(inner), Some(write_cipher), Some(read_cipher)) =
            (self.inner.take(), self.write_cipher.take(), self.read_cipher.take())
        else {
            return;
        };
        pool_put(
            key,
            PooledSession::Shadowaead(PooledSnell {
                inner,
                cipher: self.cipher,
                psk: std::mem::take(&mut self.psk),
                write_cipher,
                write_nonce: self.write_nonce,
                read_cipher,
                read_nonce: self.read_nonce,
                idle_since: Instant::now(),
            }),
        );
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
    fn parses_snell_entry_defaults_to_v1() {
        let entry = parse_entry("name: s\ntype: snell\nserver: example.com\nport: 443\npsk: secret\n");
        let config = SnellOutboundConfig::from_proxy(&entry).unwrap();
        assert_eq!(config.server, "example.com");
        assert_eq!(config.port, 443);
        assert_eq!(config.psk, b"secret");
        assert_eq!(config.version, 1);
        assert_eq!(config.cipher(), SnellCipher::Chacha20Poly1305);
        assert_eq!(config.command(), COMMAND_CONNECT);
    }

    #[test]
    fn version_selects_cipher_and_command() {
        let v2 = parse_entry("name: s\ntype: snell\nserver: h\nport: 1\npsk: p\nversion: 2\n");
        let v2 = SnellOutboundConfig::from_proxy(&v2).unwrap();
        assert_eq!(v2.cipher(), SnellCipher::Aes128Gcm);
        assert_eq!(v2.command(), COMMAND_CONNECT_V2);

        let v3 = parse_entry("name: s\ntype: snell\nserver: h\nport: 1\npsk: p\nversion: 3\n");
        let v3 = SnellOutboundConfig::from_proxy(&v3).unwrap();
        assert_eq!(v3.cipher(), SnellCipher::Aes128Gcm);
        assert_eq!(v3.command(), COMMAND_CONNECT);
    }

    #[test]
    fn v4_and_v5_select_frame_path_and_v5_normalises_to_v4() {
        for version in [4, 5] {
            let entry = parse_entry(&format!(
                "name: s\ntype: snell\nserver: h\nport: 1\npsk: p\nversion: {version}\n"
            ));
            let config = SnellOutboundConfig::from_proxy(&entry).unwrap();
            // v5 dials as v4 (identical on the wire).
            assert_eq!(config.version, 4);
            assert!(config.uses_v4_framing());
            assert_eq!(config.cipher(), SnellCipher::Aes128Gcm);
            assert_eq!(config.command(), COMMAND_CONNECT);
            // v4/v5 carry UDP over the v4 frame stream.
            assert!(config.supports_udp());
        }
    }

    #[test]
    fn rejects_missing_psk_and_bad_version() {
        let no_psk = parse_entry("name: s\ntype: snell\nserver: h\nport: 1\n");
        assert!(SnellOutboundConfig::from_proxy(&no_psk).is_err());
        let bad_version = parse_entry("name: s\ntype: snell\nserver: h\nport: 1\npsk: p\nversion: 6\n");
        assert!(SnellOutboundConfig::from_proxy(&bad_version).is_err());
    }

    #[test]
    fn request_header_encodes_host_and_port() {
        let target = TargetAddr::Domain("example.com".to_string(), 443);
        let header = build_request_header(COMMAND_CONNECT, &target).unwrap();
        let mut expected = vec![SNELL_PROTO_BYTE, COMMAND_CONNECT, 0, 11];
        expected.extend_from_slice(b"example.com");
        expected.extend_from_slice(&443u16.to_be_bytes());
        assert_eq!(header, expected);
    }

    #[test]
    fn supports_udp_on_v3_and_v4() {
        let cfg = |v: u8| SnellOutboundConfig {
            server: "h".into(),
            port: 1,
            psk: b"p".to_vec(),
            version: v,
            obfs: None,
            reuse: false,
        };
        assert!(!cfg(1).supports_udp());
        assert!(!cfg(2).supports_udp());
        assert!(cfg(3).supports_udp());
        // v4 (and v5, normalised to 4) carry UDP over the v4 frame stream.
        assert!(cfg(4).supports_udp());
    }

    #[test]
    fn encodes_udp_addr_per_family() {
        let mut domain = Vec::new();
        encode_udp_addr(&mut domain, &TargetAddr::Domain("ex.com".into(), 443)).unwrap();
        let mut expected = vec![6u8];
        expected.extend_from_slice(b"ex.com");
        expected.extend_from_slice(&443u16.to_be_bytes());
        assert_eq!(domain, expected);

        let mut v4 = Vec::new();
        encode_udp_addr(&mut v4, &TargetAddr::Ip("1.2.3.4:443".parse().unwrap())).unwrap();
        assert_eq!(v4, vec![0, 4, 1, 2, 3, 4, 0x01, 0xbb]);

        let mut v6 = Vec::new();
        encode_udp_addr(&mut v6, &TargetAddr::Ip("[::1]:53".parse().unwrap())).unwrap();
        let mut expected_v6 = vec![0u8, 6];
        expected_v6.extend_from_slice(&std::net::Ipv6Addr::LOCALHOST.octets());
        expected_v6.extend_from_slice(&53u16.to_be_bytes());
        assert_eq!(v6, expected_v6);
    }

    #[test]
    fn decodes_udp_reply_strips_source_address() {
        let mut v4 = vec![UDP_ADDR_IPV4, 9, 9, 9, 9, 0x00, 0x35];
        v4.extend_from_slice(b"payload");
        assert_eq!(decode_udp_reply(&v4).unwrap(), b"payload");

        let mut v6 = vec![UDP_ADDR_IPV6];
        v6.extend_from_slice(&std::net::Ipv6Addr::LOCALHOST.octets());
        v6.extend_from_slice(&53u16.to_be_bytes());
        v6.extend_from_slice(b"reply6");
        assert_eq!(decode_udp_reply(&v6).unwrap(), b"reply6");

        assert!(decode_udp_reply(&[0x03, 1, 2, 3]).is_err());
        assert!(decode_udp_reply(&[]).is_err());
    }

    #[test]
    fn snell_kdf_truncates_argon2_output() {
        let psk = b"password";
        let salt = [0x11u8; SALT_LEN];
        let k16 = snell_kdf(psk, &salt, 16);
        let k32 = snell_kdf(psk, &salt, 32);
        assert_eq!(k16.len(), 16);
        assert_eq!(k32.len(), 32);
        // Truncation: the 16-byte key is the prefix of the 32-byte derivation.
        assert_eq!(&k32[..16], &k16[..]);
    }

    #[test]
    fn obfs_parses_modes_and_rejects_unknown() {
        assert_eq!(SnellObfs::parse(None).unwrap(), None);
        // Empty `obfs-opts` (no mode) means no obfs.
        assert_eq!(SnellObfs::parse(Some(&ObfsOpts::default())).unwrap(), None);

        let http = SnellObfs::parse(Some(&ObfsOpts {
            mode: Some("http".into()),
            host: Some("a.example".into()),
        }))
        .unwrap();
        assert_eq!(
            http,
            Some(SnellObfs::Http {
                host: "a.example".into(),
                path: "/".into(),
            })
        );

        // Unset host defaults to a plausible value.
        let tls = SnellObfs::parse(Some(&ObfsOpts {
            mode: Some("tls".into()),
            host: None,
        }))
        .unwrap();
        assert_eq!(
            tls,
            Some(SnellObfs::Tls {
                host: "bing.com".into()
            })
        );

        let err = SnellObfs::parse(Some(&ObfsOpts {
            mode: Some("quic".into()),
            host: None,
        }))
        .unwrap_err();
        assert!(err.to_string().contains("unknown obfs mode"), "got: {err}");
    }

    // ---- v2 session reuse (CommandConnectV2 + half-close) -----------------

    use std::net::{Ipv4Addr, SocketAddr};
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::{TcpListener, TcpStream};

    const REUSE_PSK: &[u8] = b"snell-reuse-psk";

    /// One framing event read off the fake server's socket.
    enum SrvChunk {
        Data(Vec<u8>),
        /// A zero-length chunk: the client's half-close.
        Zero,
        /// The transport closed.
        Eof,
    }

    async fn srv_read_chunk(stream: &mut TcpStream, cipher: &AeadCipher, nonce: &mut [u8; 12]) -> SrvChunk {
        let mut sealed_len = [0u8; 2 + TAG_LEN];
        if stream.read_exact(&mut sealed_len).await.is_err() {
            return SrvChunk::Eof;
        }
        let len_plain = cipher.open(nonce, &sealed_len).expect("server: decrypt chunk length");
        increment_nonce(nonce);
        let clen = u16::from_be_bytes([len_plain[0], len_plain[1]]) as usize;
        if clen == 0 {
            return SrvChunk::Zero;
        }
        let mut sealed = vec![0u8; clen + TAG_LEN];
        stream.read_exact(&mut sealed).await.expect("server: read chunk body");
        let plain = cipher.open(nonce, &sealed).expect("server: decrypt chunk body");
        increment_nonce(nonce);
        SrvChunk::Data(plain)
    }

    async fn srv_write_chunk(stream: &mut TcpStream, cipher: &AeadCipher, nonce: &mut [u8; 12], plaintext: &[u8]) {
        let len = (plaintext.len() as u16).to_be_bytes();
        let sealed_len = cipher.seal(nonce, &len).unwrap();
        increment_nonce(nonce);
        let sealed = cipher.seal(nonce, plaintext).unwrap();
        increment_nonce(nonce);
        stream.write_all(&sealed_len).await.unwrap();
        stream.write_all(&sealed).await.unwrap();
        stream.flush().await.unwrap();
    }

    /// The server's half-close: a single sealed zero-length field, no payload.
    async fn srv_write_zero(stream: &mut TcpStream, cipher: &AeadCipher, nonce: &mut [u8; 12]) {
        let sealed_len = cipher.seal(nonce, &[0u8, 0u8]).unwrap();
        increment_nonce(nonce);
        stream.write_all(&sealed_len).await.unwrap();
        stream.flush().await.unwrap();
    }

    /// A reuse-capable fake Snell v2 server: it handshakes once, then loops
    /// serving sequential logical streams on the *same* connection, echoing each
    /// until the client's zero-length half-close, replying with its own zero
    /// chunk so the client sees a clean logical EOF. It records each request's
    /// command byte so tests can assert CommandConnectV2 was sent.
    async fn serve_reuse(mut stream: TcpStream, commands: Arc<std::sync::Mutex<Vec<u8>>>) {
        let cipher = SnellCipher::Aes128Gcm; // v2
        let ks = cipher.key_size();

        let mut salt = [0u8; SALT_LEN];
        if stream.read_exact(&mut salt).await.is_err() {
            return;
        }
        let read_cipher = AeadCipher::new(cipher, &snell_kdf(REUSE_PSK, &salt, ks)).unwrap();
        let mut read_nonce = [0u8; 12];

        let mut salt_w = [0u8; SALT_LEN];
        for (i, b) in salt_w.iter_mut().enumerate() {
            *b = (i as u8).wrapping_mul(11).wrapping_add(3);
        }
        stream.write_all(&salt_w).await.unwrap();
        let write_cipher = AeadCipher::new(cipher, &snell_kdf(REUSE_PSK, &salt_w, ks)).unwrap();
        let mut write_nonce = [0u8; 12];

        loop {
            // Each logical stream starts with a request header chunk.
            let header = match srv_read_chunk(&mut stream, &read_cipher, &mut read_nonce).await {
                SrvChunk::Data(h) => h,
                SrvChunk::Zero | SrvChunk::Eof => return,
            };
            commands.lock().unwrap().push(header[1]);
            srv_write_chunk(&mut stream, &write_cipher, &mut write_nonce, &[RESP_TUNNEL]).await;

            // Echo until the client half-closes this logical stream.
            loop {
                match srv_read_chunk(&mut stream, &read_cipher, &mut read_nonce).await {
                    SrvChunk::Data(d) => srv_write_chunk(&mut stream, &write_cipher, &mut write_nonce, &d).await,
                    SrvChunk::Zero => {
                        srv_write_zero(&mut stream, &write_cipher, &mut write_nonce).await;
                        break;
                    }
                    SrvChunk::Eof => return,
                }
            }
        }
    }

    async fn spawn_reuse_server() -> (SocketAddr, Arc<AtomicUsize>, Arc<std::sync::Mutex<Vec<u8>>>) {
        let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).await.unwrap();
        let addr = listener.local_addr().unwrap();
        let conns = Arc::new(AtomicUsize::new(0));
        let commands = Arc::new(std::sync::Mutex::new(Vec::new()));
        let conns_task = conns.clone();
        let commands_task = commands.clone();
        tokio::spawn(async move {
            while let Ok((stream, _)) = listener.accept().await {
                conns_task.fetch_add(1, Ordering::SeqCst);
                tokio::spawn(serve_reuse(stream, commands_task.clone()));
            }
        });
        (addr, conns, commands)
    }

    fn reuse_config(addr: SocketAddr) -> SnellOutboundConfig {
        SnellOutboundConfig {
            server: addr.ip().to_string(),
            port: addr.port(),
            psk: REUSE_PSK.to_vec(),
            version: 2,
            obfs: None,
            reuse: false,
        }
    }

    fn pool_len(key: &SnellServerKey) -> usize {
        SESSION_POOL
            .lock()
            .expect("snell session pool")
            .as_ref()
            .and_then(|m| m.get(key))
            .map_or(0, |v| v.len())
    }

    /// Relay-style round trip then a clean half-close (our zero chunk, then read
    /// the server's zero chunk to EOF), leaving the session reusable once
    /// dropped — the shape `copy_bidirectional` produces in the real relay.
    async fn round_trip_and_close(stream: &mut BoxedStream, payload: &[u8]) {
        stream.write_all(payload).await.unwrap();
        stream.flush().await.unwrap();
        let mut buf = vec![0u8; payload.len()];
        stream.read_exact(&mut buf).await.unwrap();
        assert_eq!(buf, payload);
        stream.shutdown().await.unwrap();
        let mut tail = Vec::new();
        stream.read_to_end(&mut tail).await.unwrap();
        assert!(tail.is_empty(), "no application bytes after the echo");
    }

    #[tokio::test]
    async fn v2_reuses_pooled_session_sequentially() {
        let (addr, conns, commands) = spawn_reuse_server().await;
        let config = reuse_config(addr);
        let key = SnellServerKey::from_config(&config);
        let target = TargetAddr::Domain("example.com".to_string(), 443);

        // First stream over a fresh connection; a clean half-close parks it.
        {
            let mut s = connect(&config, &target).await.unwrap();
            round_trip_and_close(&mut s, b"first").await;
        }
        assert_eq!(pool_len(&key), 1, "clean half-close parks the session for reuse");

        // Second and third streams must ride the same connection.
        {
            let mut s = connect(&config, &target).await.unwrap();
            round_trip_and_close(&mut s, b"second").await;
        }
        {
            let mut s = connect(&config, &target).await.unwrap();
            round_trip_and_close(&mut s, b"third").await;
        }

        assert_eq!(pool_len(&key), 1, "the session stays parked between reuses");
        assert_eq!(
            conns.load(Ordering::SeqCst),
            1,
            "all three streams shared one TCP connection"
        );
        assert_eq!(
            *commands.lock().unwrap(),
            vec![COMMAND_CONNECT_V2, COMMAND_CONNECT_V2, COMMAND_CONNECT_V2],
            "every reuse request used CommandConnectV2",
        );
    }

    #[tokio::test]
    async fn pool_take_evicts_idle_expired_sessions() {
        let key = SnellServerKey {
            server: "ttl-test.invalid".to_string(),
            port: 1,
            version: 2,
            psk: b"p".to_vec(),
            obfs: None,
        };
        let (dummy, _peer) = tokio::io::duplex(64);
        let expired = PooledSnell {
            inner: Box::new(dummy),
            cipher: SnellCipher::Aes128Gcm,
            psk: b"p".to_vec(),
            write_cipher: AeadCipher::new(SnellCipher::Aes128Gcm, &[0u8; 16]).unwrap(),
            write_nonce: [0u8; 12],
            read_cipher: AeadCipher::new(SnellCipher::Aes128Gcm, &[0u8; 16]).unwrap(),
            read_nonce: [0u8; 12],
            idle_since: Instant::now() - SESSION_IDLE_TTL - Duration::from_secs(1),
        };
        pool_put(key.clone(), PooledSession::Shadowaead(expired));
        // The parked session has outlived the idle TTL: it is evicted on access
        // and no session is handed back for reuse.
        assert!(pool_take(&key).is_none(), "an idle-expired session is not reused");
        assert_eq!(pool_len(&key), 0, "the expired entry is evicted");
    }
}
