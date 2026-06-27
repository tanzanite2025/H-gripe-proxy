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
//! Scoped out for follow-up (kept explicit so traffic is never mis-framed):
//! session reuse / connection pooling, the v4/v5 connection types, and
//! `obfs-opts` (http / tls simple-obfs).

use std::io;
use std::net::SocketAddr;
use std::pin::Pin;
use std::task::{Context as TaskContext, Poll, ready};

use aes_gcm::Aes128Gcm;
use aes_gcm::aead::generic_array::GenericArray;
use aes_gcm::aead::{Aead, KeyInit, Payload};
use anyhow::{Context, Result, anyhow, bail};
use argon2::{Algorithm, Argon2, Params, Version};
use chacha20poly1305::ChaCha20Poly1305;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt, ReadBuf};
use tokio::net::TcpStream;
use tokio::net::tcp::{OwnedReadHalf, OwnedWriteHalf};
use tokio::sync::Mutex;

use crate::address::TargetAddr;
use crate::config::outbound_opts::ProxyEntry;
use crate::outbound::BoxedStream;

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

/// Fully-resolved Snell outbound parameters.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SnellOutboundConfig {
    pub server: String,
    pub port: u16,
    /// Pre-shared key bytes (the `psk` string used verbatim as Argon2 input).
    pub psk: Vec<u8>,
    /// Protocol version (1, 2 or 3).
    pub version: u8,
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
        // mihomo defaults an unset `version` to 1.
        let version = match opts.version.unwrap_or(1) {
            v @ 1..=3 => v as u8,
            other => bail!("snell: version {other} not supported (use 1, 2 or 3)"),
        };
        Ok(Self {
            server,
            port,
            psk,
            version,
        })
    }

    fn cipher(&self) -> SnellCipher {
        match self.version {
            1 => SnellCipher::Chacha20Poly1305,
            _ => SnellCipher::Aes128Gcm,
        }
    }

    /// The request command: v2 negotiates the reuse-capable command, v1/v3 use
    /// the plain connect command (reuse / half-close is not implemented).
    fn command(&self) -> u8 {
        if self.version == 2 {
            COMMAND_CONNECT_V2
        } else {
            COMMAND_CONNECT
        }
    }

    /// Whether this outbound can carry UDP. Snell's `CommandUDP` framing is only
    /// defined for protocol v3, so v1/v2 reject UDP rather than mis-frame it.
    pub fn supports_udp(&self) -> bool {
        self.version >= 3
    }
}

/// Connect a Snell outbound to `target` and return a relay-ready stream. The
/// salt and the AEAD-sealed request header are sent before the stream is handed
/// back; the server's command response is consumed transparently on first read.
pub async fn connect(config: &SnellOutboundConfig, target: &TargetAddr) -> Result<BoxedStream> {
    let mut transport: BoxedStream = Box::new(
        TcpStream::connect((config.server.as_str(), config.port))
            .await
            .with_context(|| format!("snell: connect {}:{}", config.server, config.port))?,
    );

    let cipher = config.cipher();
    let mut salt = [0u8; SALT_LEN];
    random_bytes(&mut salt);
    transport.write_all(&salt).await.context("snell: send salt")?;

    let subkey = snell_kdf(&config.psk, &salt, cipher.key_size());
    let write_cipher = AeadCipher::new(cipher, &subkey)?;

    let mut stream = SnellStream::new(transport, cipher, config.psk.clone(), write_cipher);

    let header = build_request_header(config.command(), target)?;
    stream.write_all(&header).await.context("snell: send request header")?;

    Ok(Box::new(stream))
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
    writer: OwnedWriteHalf,
    cipher: AeadCipher,
    nonce: [u8; 12],
}

struct UdpReadSide {
    reader: OwnedReadHalf,
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
        if !config.supports_udp() {
            bail!("snell udp: requires version 3 (got v{})", config.version);
        }
        let cipher = config.cipher();
        let tcp = TcpStream::connect((config.server.as_str(), config.port))
            .await
            .with_context(|| format!("snell udp: connect {}:{}", config.server, config.port))?;
        let (reader, mut writer) = tcp.into_split();

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

/// Seal `plaintext` as one length-prefixed AEAD chunk and write it (with a
/// flush) to `writer`, advancing the counter nonce twice.
async fn write_packet_chunk(
    writer: &mut OwnedWriteHalf,
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
async fn read_packet_chunk(reader: &mut OwnedReadHalf, cipher: &AeadCipher, nonce: &mut [u8; 12]) -> Result<Vec<u8>> {
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
    inner: BoxedStream,
    cipher: SnellCipher,
    psk: Vec<u8>,
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
    /// Whether the server's leading command-response byte has been consumed.
    reply_done: bool,
    plain: Vec<u8>,
    plain_pos: usize,
}

impl SnellStream {
    fn new(inner: BoxedStream, cipher: SnellCipher, psk: Vec<u8>, write_cipher: AeadCipher) -> Self {
        Self {
            inner,
            cipher,
            psk,
            write_cipher,
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
        }
    }

    /// Flush any pending sealed bytes to the inner stream.
    fn poll_drain(&mut self, cx: &mut TaskContext<'_>) -> Poll<io::Result<()>> {
        while self.write_pos < self.write_buf.len() {
            let n = ready!(Pin::new(&mut self.inner).poll_write(cx, &self.write_buf[self.write_pos..]))?;
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

    /// Strip the leading command-response byte from freshly-decrypted plaintext,
    /// returning an error if the server reported `RESP_ERROR`.
    fn consume_reply(&mut self) -> io::Result<()> {
        // `plain` holds the just-decrypted chunk starting at `plain_pos`.
        if self.plain_pos >= self.plain.len() {
            return Ok(());
        }
        match self.plain[self.plain_pos] {
            RESP_TUNNEL => {
                self.plain_pos += 1;
                self.reply_done = true;
                Ok(())
            }
            RESP_ERROR => {
                // `code | msg-len | msg`; best-effort decode for diagnostics
                // (the connection fails regardless of how much is buffered).
                let rest = &self.plain[self.plain_pos + 1..];
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
            other => Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("snell: unexpected command response {other}"),
            )),
        }
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
                ready!(Pin::new(&mut this.inner).poll_read(cx, &mut read_buf))?;
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
                    if clen == 0 || clen > MAX_CHUNK {
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
        Pin::new(&mut this.inner).poll_flush(cx)
    }

    fn poll_shutdown(self: Pin<&mut Self>, cx: &mut TaskContext<'_>) -> Poll<io::Result<()>> {
        let this = self.get_mut();
        ready!(this.poll_drain(cx))?;
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
    fn rejects_missing_psk_and_bad_version() {
        let no_psk = parse_entry("name: s\ntype: snell\nserver: h\nport: 1\n");
        assert!(SnellOutboundConfig::from_proxy(&no_psk).is_err());
        let bad_version = parse_entry("name: s\ntype: snell\nserver: h\nport: 1\npsk: p\nversion: 4\n");
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
    fn supports_udp_only_on_v3() {
        let cfg = |v: u8| SnellOutboundConfig {
            server: "h".into(),
            port: 1,
            psk: b"p".to_vec(),
            version: v,
        };
        assert!(!cfg(1).supports_udp());
        assert!(!cfg(2).supports_udp());
        assert!(cfg(3).supports_udp());
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
}
