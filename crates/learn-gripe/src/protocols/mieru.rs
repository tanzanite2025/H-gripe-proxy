//! Mieru outbound (TCP underlay, single logical stream).
//!
//! [mieru](https://github.com/enfein/mieru) is a self-designed reliable
//! transport (not a thin SOCKS5/CONNECT wrapper). This module implements the
//! **client** side of its TCP underlay carrying a single logical stream — the
//! common case for a `mieru` proxy node — and deliberately leaves the UDP
//! underlay and the multiplexed (MUX) mode to follow-up work, the same
//! TCP-first staging used for the SSH / Hysteria / GOST outbounds.
//!
//! ## Why a faithful client only needs the cipher + framing
//!
//! mieru layers an ARQ reliable transport (seq/ack/window/retransmit) on top of
//! its underlay so it can run over an unreliable UDP datagram path. Over a
//! *stream* underlay (TCP) the bytes are already ordered and reliable, so the
//! upstream server bypasses that machinery entirely (`inputAck` is a no-op,
//! `inputData` is delivered straight to the receive queue). A correct TCP
//! client therefore reduces to:
//!
//! 1. **Key derivation** — `key = PBKDF2-HMAC-SHA256(HashPassword(password,
//!    username), salt, iter=64, 32)` where `HashPassword(p,u) = SHA256(p ‖ 0x00
//!    ‖ u)` and `salt = SHA256(BE_u64(unix_seconds rounded to 2 min))`. The salt
//!    rotates every 2 minutes; the server accepts the previous / current / next
//!    salt, so the client sends with the current one.
//! 2. **Stateful XChaCha20-Poly1305** — 24-byte nonce, 16-byte tag. The first
//!    ciphertext on each direction carries the full random nonce (with a 4-byte
//!    user hint, see below); every subsequent ciphertext omits it and both ends
//!    increment the nonce in lock-step ("implicit nonce").
//! 3. **32-byte encrypted segment metadata** — each segment is
//!    `Encrypt(metadata)` followed by `Encrypt(payload)` (and optional padding).
//! 4. **openSession handshake** — the client sends an `openSessionRequest`
//!    segment (seq 0) that also carries the first payload (≤ 1024 bytes), the
//!    server replies with `openSessionResponse`; afterwards both sides exchange
//!    `data*` segments.
//! 5. **Inner SOCKS5** — the payload carried inside the session is a bare SOCKS5
//!    `CONNECT` request (no method negotiation; the server runs with
//!    *client-side authentication*) answered by a bare SOCKS5 reply, after which
//!    bytes relay transparently.
//!
//! The 4-byte **user hint** written into the nonce tail
//! (`SHA256(username ‖ nonce[..16])[..4]`) lets the multi-user server pick the
//! right key without trial-decrypting every user.

use std::cmp::min;
use std::io;
use std::pin::Pin;
use std::task::{Context as TaskContext, Poll};
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result, anyhow, bail};
use chacha20poly1305::aead::{Aead, KeyInit};
use chacha20poly1305::{XChaCha20Poly1305, XNonce};
use hmac::{Hmac, Mac};
use sha2::{Digest, Sha256};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt, ReadBuf};

use crate::address::TargetAddr;
use crate::config::outbound_opts::ProxyEntry;
use crate::inbound::socks5;
use crate::outbound::BoxedStream;
use crate::transport::{self, Security, Transport};

/// Bytes of metadata before encryption (one fixed-size struct per segment).
const METADATA_LEN: usize = 32;
/// XChaCha20-Poly1305 nonce size.
const NONCE_SIZE: usize = 24;
/// XChaCha20-Poly1305 authentication-tag overhead.
const OVERHEAD: usize = 16;
/// Derived session-key length (256-bit).
const KEY_LEN: usize = 32;
/// PBKDF2 iteration count fixed by the mieru wire protocol.
const PBKDF2_ITER: u32 = 64;
/// Salt rotation period (the salt is a hash of the rounded unix time).
const KEY_REFRESH_SECS: u64 = 120;
/// Largest payload an open-session segment may carry.
const MAX_SESSION_OPEN_PAYLOAD: usize = 1024;
/// Largest payload a single data segment carries (no fragmentation on a stream).
const MAX_PDU: usize = 32 * 1024;

// Segment protocol types (`metadata[0]`).
const PROTO_CLOSE_SESSION_REQUEST: u8 = 4;
const PROTO_CLOSE_SESSION_RESPONSE: u8 = 5;
const PROTO_OPEN_SESSION_REQUEST: u8 = 2;
const PROTO_OPEN_SESSION_RESPONSE: u8 = 3;
const PROTO_DATA_CLIENT_TO_SERVER: u8 = 6;
const PROTO_DATA_SERVER_TO_CLIENT: u8 = 7;
const PROTO_ACK_CLIENT_TO_SERVER: u8 = 8;
const PROTO_ACK_SERVER_TO_CLIENT: u8 = 9;

/// User-hint input prefix length (`SHA256(username ‖ nonce[..16])`).
const USER_HINT_PREFIX: usize = 16;
/// User-hint output length written to the nonce tail.
const USER_HINT_SUFFIX: usize = 4;

/// Fully-resolved mieru outbound parameters.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MieruOutboundConfig {
    pub server: String,
    pub port: u16,
    /// mieru identifies the user by name (drives the nonce user hint and the
    /// server-side key selection); always required.
    pub username: String,
    /// Raw account password (mixed into the key via `HashPassword`).
    pub password: String,
}

impl MieruOutboundConfig {
    /// Build an outbound config from a parsed `mieru` proxy entry.
    ///
    /// Only the TCP underlay single-stream mode is implemented: a non-TCP
    /// `transport`, any multiplexing other than `MULTIPLEXING_OFF`, and
    /// `port-range` port hopping are rejected up front rather than silently
    /// mis-handled.
    pub fn from_proxy(entry: &ProxyEntry) -> Result<Self> {
        let opts = &entry.options;
        let server = opts
            .server
            .clone()
            .filter(|s| !s.is_empty())
            .context("mieru: missing server")?;
        let port = opts.port.context("mieru: missing port")?;

        // mieru's reliable transport runs over either a TCP or a UDP underlay;
        // this client implements the TCP underlay only.
        if let Some(transport) = opts.transport.as_deref().filter(|s| !s.is_empty()) {
            if !transport.eq_ignore_ascii_case("tcp") {
                bail!("mieru: only the TCP underlay is supported (transport={transport:?})");
            }
        }
        // Port hopping (`port-range`) and multiplexing are deferred to follow-up
        // work; a single fixed port and a single logical stream are used.
        if opts.port_range.as_deref().is_some_and(|s| !s.is_empty()) {
            bail!("mieru: port-range (port hopping) is not supported yet");
        }
        if let Some(mux) = opts.multiplexing.as_deref().filter(|s| !s.is_empty()) {
            if !mux.eq_ignore_ascii_case("MULTIPLEXING_OFF") {
                bail!("mieru: multiplexing ({mux}) is not supported yet; use MULTIPLEXING_OFF");
            }
        }

        let username = opts
            .username
            .clone()
            .filter(|s| !s.is_empty())
            .context("mieru: missing username")?;
        let password = opts
            .password
            .clone()
            .filter(|s| !s.is_empty())
            .context("mieru: missing password")?;

        Ok(Self {
            server,
            port,
            username,
            password,
        })
    }
}

/// Connect through the mieru server to `target` and return a relay-ready
/// stream. Dials TCP, opens a mieru session whose first payload is the inner
/// SOCKS5 `CONNECT` request, validates the SOCKS5 reply, and hands back a
/// transparent stream wrapping the session framing.
pub async fn connect(config: &MieruOutboundConfig, target: &TargetAddr) -> Result<BoxedStream> {
    let key = derive_session_key(config.password.as_bytes(), config.username.as_bytes(), now_unix());
    let inner = transport::establish(&config.server, config.port, &Security::None, &Transport::Tcp)
        .await
        .context("mieru: dial server")?;

    // The send cipher carries the user hint; the receive cipher only opens the
    // server's segments (it learns the nonce from the first ciphertext).
    let send = MieruCipher::new(&key, config.username.as_bytes());
    let recv = MieruCipher::new(&key, &[]);
    let mut stream = MieruStream::new(inner, send, recv, random_u32());

    // The session's first payload is a bare SOCKS5 CONNECT (the server runs with
    // client-side authentication, so there is no method-negotiation greeting).
    let request = build_socks5_connect(target)?;
    stream
        .write_all(&request)
        .await
        .context("mieru: send openSession + inner SOCKS5 request")?;
    read_socks5_reply(&mut stream)
        .await
        .with_context(|| format!("mieru: inner SOCKS5 CONNECT to {target}"))?;
    Ok(Box::new(stream))
}

/// Build the bare inner SOCKS5 `CONNECT` request (`VER CMD RSV ATYP ADDR PORT`).
fn build_socks5_connect(target: &TargetAddr) -> Result<Vec<u8>> {
    if matches!(target, TargetAddr::Domain(host, _) if host.len() > 0xFF) {
        bail!("mieru: target host exceeds 255 bytes");
    }
    let mut buf = vec![socks5::VERSION, socks5::CMD_CONNECT, socks5::RSV];
    socks5::encode_address(&mut buf, target);
    Ok(buf)
}

/// Read the bare inner SOCKS5 reply (`VER REP RSV ATYP BND PORT`), requiring a
/// success status and draining the bound address so the stream is positioned at
/// relay data.
async fn read_socks5_reply<S>(stream: &mut S) -> Result<()>
where
    S: AsyncRead + Unpin,
{
    let mut head = [0u8; 4];
    stream.read_exact(&mut head).await.context("mieru: read SOCKS5 reply")?;
    if head[0] != socks5::VERSION {
        bail!("mieru: inner SOCKS5 reply bad version 0x{:02x}", head[0]);
    }
    if head[1] != socks5::REP_SUCCEEDED {
        bail!("mieru: inner SOCKS5 CONNECT failed, reply 0x{:02x}", head[1]);
    }
    // ATYP is head[3]; consume the bound address + port that follow.
    socks5::read_address(stream, head[3])
        .await
        .context("mieru: read SOCKS5 reply bound address")?;
    Ok(())
}

/// PBKDF2-HMAC-SHA256 (the small subset mieru needs: a single derived block is
/// enough for a 32-byte key, but the generic loop is kept for clarity).
fn pbkdf2_hmac_sha256(password: &[u8], salt: &[u8], iterations: u32, out_len: usize) -> Vec<u8> {
    type HmacSha256 = Hmac<Sha256>;
    let mut out = Vec::with_capacity(out_len);
    let mut block_index: u32 = 1;
    while out.len() < out_len {
        let mut mac = <HmacSha256 as Mac>::new_from_slice(password).expect("HMAC accepts any key length");
        mac.update(salt);
        mac.update(&block_index.to_be_bytes());
        let mut u = mac.finalize().into_bytes();
        let mut t = u;
        for _ in 1..iterations {
            let mut mac = <HmacSha256 as Mac>::new_from_slice(password).expect("HMAC accepts any key length");
            mac.update(&u);
            u = mac.finalize().into_bytes();
            for (ti, ui) in t.iter_mut().zip(u.iter()) {
                *ti ^= *ui;
            }
        }
        out.extend_from_slice(&t);
        block_index += 1;
    }
    out.truncate(out_len);
    out
}

/// `HashPassword(password, username) = SHA256(password ‖ 0x00 ‖ username)`.
fn hash_password(password: &[u8], username: &[u8]) -> [u8; 32] {
    let mut h = Sha256::new();
    h.update(password);
    h.update([0x00]);
    h.update(username);
    h.finalize().into()
}

/// The salt for the current 2-minute window: `SHA256(BE_u64(rounded_unix))`,
/// rounding to the nearest multiple of 120 s (ties up), matching Go's
/// `time.Round` on the 120-second grid (the year-1→1970 offset is a multiple of
/// 120 s, so rounding the unix seconds is equivalent).
fn current_salt(now_secs: u64) -> [u8; 32] {
    let rounded = ((now_secs + KEY_REFRESH_SECS / 2) / KEY_REFRESH_SECS) * KEY_REFRESH_SECS;
    let mut h = Sha256::new();
    h.update(rounded.to_be_bytes());
    h.finalize().into()
}

/// Derive the 32-byte session key the client sends with (the server accepts the
/// previous / current / next salt, so the current one is always in range).
fn derive_session_key(password: &[u8], username: &[u8], now_secs: u64) -> [u8; KEY_LEN] {
    let hashed = hash_password(password, username);
    let salt = current_salt(now_secs);
    let key = pbkdf2_hmac_sha256(&hashed, &salt, PBKDF2_ITER, KEY_LEN);
    let mut out = [0u8; KEY_LEN];
    out.copy_from_slice(&key);
    out
}

fn now_unix() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// Big-endian +1 on the nonce (mieru's `increaseNonce`).
fn increment_nonce(nonce: &mut [u8; NONCE_SIZE]) {
    for byte in nonce.iter_mut().rev() {
        *byte = byte.wrapping_add(1);
        if *byte != 0 {
            break;
        }
    }
}

/// Write the 4-byte user hint `SHA256(username ‖ nonce[..16])[..4]` into the
/// nonce tail. No-op for an empty username.
fn add_user_hint(nonce: &mut [u8; NONCE_SIZE], username: &[u8]) {
    if username.is_empty() {
        return;
    }
    let mut h = Sha256::new();
    h.update(username);
    h.update(&nonce[..USER_HINT_PREFIX]);
    let out = h.finalize();
    nonce[NONCE_SIZE - USER_HINT_SUFFIX..].copy_from_slice(&out[..USER_HINT_SUFFIX]);
}

/// A stateful XChaCha20-Poly1305 cipher with mieru's implicit-nonce behaviour:
/// the first ciphertext embeds the full 24-byte nonce; every later one omits it
/// and the nonce is incremented in step on both ends.
struct MieruCipher {
    aead: XChaCha20Poly1305,
    username: Vec<u8>,
    nonce: Option<[u8; NONCE_SIZE]>,
}

impl MieruCipher {
    fn new(key: &[u8; KEY_LEN], username: &[u8]) -> Self {
        Self {
            aead: XChaCha20Poly1305::new_from_slice(key).expect("32-byte key"),
            username: username.to_vec(),
            nonce: None,
        }
    }

    /// Encrypt `plaintext`. The first call generates a random nonce (with user
    /// hint) and returns `nonce ‖ ciphertext`; later calls increment the nonce
    /// and return the ciphertext alone.
    fn encrypt(&mut self, plaintext: &[u8]) -> Vec<u8> {
        match self.nonce {
            None => {
                let mut nonce = [0u8; NONCE_SIZE];
                fill_random(&mut nonce);
                add_user_hint(&mut nonce, &self.username);
                self.nonce = Some(nonce);
                let ct = self
                    .aead
                    .encrypt(XNonce::from_slice(&nonce), plaintext)
                    .expect("XChaCha20-Poly1305 seal");
                let mut out = Vec::with_capacity(NONCE_SIZE + ct.len());
                out.extend_from_slice(&nonce);
                out.extend_from_slice(&ct);
                out
            }
            Some(mut nonce) => {
                increment_nonce(&mut nonce);
                self.nonce = Some(nonce);
                self.aead
                    .encrypt(XNonce::from_slice(&nonce), plaintext)
                    .expect("XChaCha20-Poly1305 seal")
            }
        }
    }

    /// Decrypt one ciphertext. On the first call `data` is `nonce ‖ ciphertext`;
    /// afterwards it is the ciphertext alone and the nonce is incremented.
    fn decrypt(&mut self, data: &[u8]) -> Result<Vec<u8>> {
        match self.nonce {
            Some(mut nonce) => {
                increment_nonce(&mut nonce);
                self.nonce = Some(nonce);
                self.aead
                    .decrypt(XNonce::from_slice(&nonce), data)
                    .map_err(|_| anyhow!("mieru: AEAD open failed"))
            }
            None => {
                if data.len() < NONCE_SIZE {
                    bail!("mieru: first ciphertext too short for nonce");
                }
                let mut nonce = [0u8; NONCE_SIZE];
                nonce.copy_from_slice(&data[..NONCE_SIZE]);
                self.nonce = Some(nonce);
                self.aead
                    .decrypt(XNonce::from_slice(&nonce), &data[NONCE_SIZE..])
                    .map_err(|_| anyhow!("mieru: AEAD open failed"))
            }
        }
    }
}

/// Per-segment lengths decoded from the 32-byte metadata: prefix padding,
/// payload, suffix padding, and whether it closes the session.
struct SegmentLens {
    prefix_len: usize,
    payload_len: usize,
    suffix_len: usize,
    is_close: bool,
}

fn segment_lens(meta: &[u8]) -> Result<SegmentLens> {
    match meta[0] {
        PROTO_OPEN_SESSION_REQUEST
        | PROTO_OPEN_SESSION_RESPONSE
        | PROTO_CLOSE_SESSION_REQUEST
        | PROTO_CLOSE_SESSION_RESPONSE => Ok(SegmentLens {
            prefix_len: 0,
            payload_len: u16::from_be_bytes([meta[15], meta[16]]) as usize,
            suffix_len: meta[17] as usize,
            is_close: meta[0] == PROTO_CLOSE_SESSION_REQUEST || meta[0] == PROTO_CLOSE_SESSION_RESPONSE,
        }),
        PROTO_DATA_CLIENT_TO_SERVER
        | PROTO_DATA_SERVER_TO_CLIENT
        | PROTO_ACK_CLIENT_TO_SERVER
        | PROTO_ACK_SERVER_TO_CLIENT => Ok(SegmentLens {
            prefix_len: meta[21] as usize,
            payload_len: u16::from_be_bytes([meta[22], meta[23]]) as usize,
            suffix_len: meta[24] as usize,
            is_close: false,
        }),
        other => bail!("mieru: unknown segment protocol {other}"),
    }
}

/// Big-endian unix-minutes timestamp written into every metadata header.
fn timestamp_minutes() -> u32 {
    (now_unix() / 60) as u32
}

/// Marshal a session-struct metadata header (open/close session).
fn marshal_session_meta(protocol: u8, session_id: u32, seq: u32, payload_len: u16) -> [u8; METADATA_LEN] {
    let mut meta = [0u8; METADATA_LEN];
    meta[0] = protocol;
    meta[2..6].copy_from_slice(&timestamp_minutes().to_be_bytes());
    meta[6..10].copy_from_slice(&session_id.to_be_bytes());
    meta[10..14].copy_from_slice(&seq.to_be_bytes());
    // meta[14] statusCode = 0
    meta[15..17].copy_from_slice(&payload_len.to_be_bytes());
    // meta[17] suffixLen = 0 (no padding sent)
    meta
}

/// Marshal a data/ack metadata header.
fn marshal_data_meta(protocol: u8, session_id: u32, seq: u32, payload_len: u16) -> [u8; METADATA_LEN] {
    let mut meta = [0u8; METADATA_LEN];
    meta[0] = protocol;
    meta[2..6].copy_from_slice(&timestamp_minutes().to_be_bytes());
    meta[6..10].copy_from_slice(&session_id.to_be_bytes());
    meta[10..14].copy_from_slice(&seq.to_be_bytes());
    // meta[14..18] unAckSeq = 0
    meta[18..20].copy_from_slice(&1024u16.to_be_bytes()); // windowSize (ignored on a stream)
    // meta[20] fragment = 0, meta[21] prefixLen = 0
    meta[22..24].copy_from_slice(&payload_len.to_be_bytes());
    // meta[24] suffixLen = 0
    meta
}

/// Read-side framing state machine.
enum ReadState {
    /// Awaiting (and then decrypting) a segment's encrypted metadata.
    NeedMeta,
    /// Metadata decoded; awaiting the segment body (prefix pad, payload, suffix
    /// pad) before decrypting the payload.
    NeedBody(SegmentLens),
}

/// An `AsyncRead`/`AsyncWrite` view over a single mieru session: writes are
/// encrypted into data segments, reads decrypt incoming segments and surface
/// their payloads. The opening handshake is driven by the first write (an
/// `openSessionRequest` carrying the initial payload).
struct MieruStream {
    inner: BoxedStream,
    send: MieruCipher,
    recv: MieruCipher,
    session_id: u32,
    next_seq: u32,
    /// Whether the `openSessionRequest` has been emitted yet.
    opened: bool,

    // Write staging: a fully-framed segment being flushed, plus how many input
    // bytes it consumed (returned once flushed).
    write_buf: Vec<u8>,
    write_pos: usize,
    write_consumed: usize,

    // Read staging.
    raw: Vec<u8>,
    rstate: ReadState,
    plain: Vec<u8>,
    plain_pos: usize,
    eof: bool,
}

impl MieruStream {
    fn new(inner: BoxedStream, send: MieruCipher, recv: MieruCipher, session_id: u32) -> Self {
        Self {
            inner,
            send,
            recv,
            session_id,
            next_seq: 0,
            opened: false,
            write_buf: Vec::new(),
            write_pos: 0,
            write_consumed: 0,
            raw: Vec::new(),
            rstate: ReadState::NeedMeta,
            plain: Vec::new(),
            plain_pos: 0,
            eof: false,
        }
    }

    /// Frame the next outgoing segment from `buf`, returning the wire bytes and
    /// the number of input bytes consumed.
    fn frame_outgoing(&mut self, buf: &[u8]) -> (Vec<u8>, usize) {
        if !self.opened {
            self.opened = true;
            let take = min(buf.len(), MAX_SESSION_OPEN_PAYLOAD);
            let meta = marshal_session_meta(PROTO_OPEN_SESSION_REQUEST, self.session_id, 0, take as u16);
            self.next_seq = 1;
            (self.seal_segment(&meta, &buf[..take]), take)
        } else {
            let take = min(buf.len(), MAX_PDU);
            let seq = self.next_seq;
            self.next_seq = self.next_seq.wrapping_add(1);
            let meta = marshal_data_meta(PROTO_DATA_CLIENT_TO_SERVER, self.session_id, seq, take as u16);
            (self.seal_segment(&meta, &buf[..take]), take)
        }
    }

    /// `Encrypt(metadata) ‖ Encrypt(payload)` (no padding is added on send).
    fn seal_segment(&mut self, meta: &[u8], payload: &[u8]) -> Vec<u8> {
        let mut out = self.send.encrypt(meta);
        if !payload.is_empty() {
            out.extend_from_slice(&self.send.encrypt(payload));
        }
        out
    }

    /// Ensure `self.raw` holds at least `need` bytes, reading from `inner`.
    fn poll_fill(&mut self, cx: &mut TaskContext<'_>, need: usize) -> Poll<io::Result<bool>> {
        while self.raw.len() < need {
            let mut tmp = [0u8; 8192];
            let mut rb = ReadBuf::new(&mut tmp);
            match Pin::new(&mut self.inner).poll_read(cx, &mut rb) {
                Poll::Pending => return Poll::Pending,
                Poll::Ready(Err(e)) => return Poll::Ready(Err(e)),
                Poll::Ready(Ok(())) => {
                    let filled = rb.filled();
                    if filled.is_empty() {
                        // Inner stream ended before `need` bytes arrived.
                        return Poll::Ready(Ok(false));
                    }
                    self.raw.extend_from_slice(filled);
                }
            }
        }
        Poll::Ready(Ok(true))
    }

    /// Copy as much buffered plaintext as fits into `dst`; returns true if any
    /// was delivered.
    fn drain_plain(&mut self, dst: &mut ReadBuf<'_>) -> bool {
        if self.plain_pos >= self.plain.len() {
            return false;
        }
        let n = min(dst.remaining(), self.plain.len() - self.plain_pos);
        dst.put_slice(&self.plain[self.plain_pos..self.plain_pos + n]);
        self.plain_pos += n;
        if self.plain_pos >= self.plain.len() {
            self.plain.clear();
            self.plain_pos = 0;
        }
        n > 0
    }
}

fn invalid_data<E: std::fmt::Display>(e: E) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, e.to_string())
}

impl AsyncWrite for MieruStream {
    fn poll_write(self: Pin<&mut Self>, cx: &mut TaskContext<'_>, buf: &[u8]) -> Poll<io::Result<usize>> {
        let me = self.get_mut();
        loop {
            if !me.write_buf.is_empty() {
                while me.write_pos < me.write_buf.len() {
                    match Pin::new(&mut me.inner).poll_write(cx, &me.write_buf[me.write_pos..]) {
                        Poll::Pending => return Poll::Pending,
                        Poll::Ready(Err(e)) => return Poll::Ready(Err(e)),
                        Poll::Ready(Ok(0)) => {
                            return Poll::Ready(Err(io::Error::new(
                                io::ErrorKind::WriteZero,
                                "mieru: inner stream closed",
                            )));
                        }
                        Poll::Ready(Ok(n)) => me.write_pos += n,
                    }
                }
                me.write_buf.clear();
                me.write_pos = 0;
                let consumed = me.write_consumed;
                me.write_consumed = 0;
                return Poll::Ready(Ok(consumed));
            }
            if buf.is_empty() {
                return Poll::Ready(Ok(0));
            }
            let (segment, consumed) = me.frame_outgoing(buf);
            me.write_buf = segment;
            me.write_pos = 0;
            me.write_consumed = consumed;
        }
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut TaskContext<'_>) -> Poll<io::Result<()>> {
        Pin::new(&mut self.get_mut().inner).poll_flush(cx)
    }

    fn poll_shutdown(self: Pin<&mut Self>, cx: &mut TaskContext<'_>) -> Poll<io::Result<()>> {
        Pin::new(&mut self.get_mut().inner).poll_shutdown(cx)
    }
}

impl AsyncRead for MieruStream {
    fn poll_read(self: Pin<&mut Self>, cx: &mut TaskContext<'_>, dst: &mut ReadBuf<'_>) -> Poll<io::Result<()>> {
        let me = self.get_mut();
        if me.drain_plain(dst) {
            return Poll::Ready(Ok(()));
        }
        if me.eof {
            return Poll::Ready(Ok(()));
        }
        loop {
            match &me.rstate {
                ReadState::NeedMeta => {
                    let meta_total = if me.recv.nonce.is_none() {
                        METADATA_LEN + OVERHEAD + NONCE_SIZE
                    } else {
                        METADATA_LEN + OVERHEAD
                    };
                    match me.poll_fill(cx, meta_total) {
                        Poll::Pending => return Poll::Pending,
                        Poll::Ready(Err(e)) => return Poll::Ready(Err(e)),
                        // Clean stream end at a segment boundary.
                        Poll::Ready(Ok(false)) => {
                            me.eof = true;
                            return Poll::Ready(Ok(()));
                        }
                        Poll::Ready(Ok(true)) => {}
                    }
                    let raw_meta: Vec<u8> = me.raw.drain(..meta_total).collect();
                    let meta = me.recv.decrypt(&raw_meta).map_err(invalid_data)?;
                    if meta.len() != METADATA_LEN {
                        return Poll::Ready(Err(invalid_data("mieru: bad metadata length")));
                    }
                    let lens = segment_lens(&meta).map_err(invalid_data)?;
                    me.rstate = ReadState::NeedBody(lens);
                }
                ReadState::NeedBody(lens) => {
                    // Copy the lengths out so the immutable borrow of `rstate`
                    // ends before `poll_fill` borrows `*me` mutably.
                    let prefix_len = lens.prefix_len;
                    let payload_len = lens.payload_len;
                    let suffix_len = lens.suffix_len;
                    let is_close = lens.is_close;
                    let payload_on_wire = if payload_len > 0 { payload_len + OVERHEAD } else { 0 };
                    let body_total = prefix_len + payload_on_wire + suffix_len;
                    if body_total > 0 {
                        match me.poll_fill(cx, body_total) {
                            Poll::Pending => return Poll::Pending,
                            Poll::Ready(Err(e)) => return Poll::Ready(Err(e)),
                            Poll::Ready(Ok(false)) => {
                                return Poll::Ready(Err(io::Error::new(
                                    io::ErrorKind::UnexpectedEof,
                                    "mieru: truncated segment body",
                                )));
                            }
                            Poll::Ready(Ok(true)) => {}
                        }
                    }
                    let body: Vec<u8> = me.raw.drain(..body_total).collect();
                    if payload_len > 0 {
                        let start = prefix_len;
                        let end = start + payload_len + OVERHEAD;
                        let plaintext = me.recv.decrypt(&body[start..end]).map_err(invalid_data)?;
                        me.plain = plaintext;
                        me.plain_pos = 0;
                    }
                    me.rstate = ReadState::NeedMeta;
                    if is_close {
                        me.eof = true;
                    }
                    if me.drain_plain(dst) {
                        return Poll::Ready(Ok(()));
                    }
                    if me.eof {
                        return Poll::Ready(Ok(()));
                    }
                    // Empty segment (ack / open-session response): read the next.
                }
            }
        }
    }
}

fn fill_random(buf: &mut [u8]) {
    getrandom::fill(buf).expect("mieru: system RNG unavailable");
}

fn random_u32() -> u32 {
    let mut bytes = [0u8; 4];
    fill_random(&mut bytes);
    u32::from_be_bytes(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::outbound_opts::ProxyEntry;

    fn parse_entry(yaml: &str) -> ProxyEntry {
        serde_yaml_ng::from_str(yaml).expect("parse proxy entry")
    }

    #[test]
    fn parses_minimal_mieru() {
        let cfg = MieruOutboundConfig::from_proxy(&parse_entry(
            "name: m\ntype: mieru\nserver: m.example\nport: 2999\nusername: bob\npassword: secret\n",
        ))
        .unwrap();
        assert_eq!(cfg.server, "m.example");
        assert_eq!(cfg.port, 2999);
        assert_eq!(cfg.username, "bob");
        assert_eq!(cfg.password, "secret");
    }

    #[test]
    fn tcp_transport_is_accepted() {
        let cfg = MieruOutboundConfig::from_proxy(&parse_entry(
            "name: m\ntype: mieru\nserver: m.example\nport: 2999\nusername: bob\npassword: secret\ntransport: TCP\n",
        ));
        assert!(cfg.is_ok());
    }

    #[test]
    fn udp_transport_is_rejected() {
        let err = MieruOutboundConfig::from_proxy(&parse_entry(
            "name: m\ntype: mieru\nserver: m.example\nport: 2999\nusername: bob\npassword: secret\ntransport: UDP\n",
        ))
        .unwrap_err();
        assert!(err.to_string().contains("TCP underlay"), "{err}");
    }

    #[test]
    fn multiplexing_on_is_rejected() {
        let err = MieruOutboundConfig::from_proxy(&parse_entry(
            "name: m\ntype: mieru\nserver: m.example\nport: 2999\nusername: bob\npassword: p\nmultiplexing: MULTIPLEXING_HIGH\n",
        ))
        .unwrap_err();
        assert!(err.to_string().contains("multiplexing"), "{err}");
    }

    #[test]
    fn multiplexing_off_is_accepted() {
        let cfg = MieruOutboundConfig::from_proxy(&parse_entry(
            "name: m\ntype: mieru\nserver: m.example\nport: 2999\nusername: bob\npassword: p\nmultiplexing: MULTIPLEXING_OFF\n",
        ));
        assert!(cfg.is_ok());
    }

    #[test]
    fn port_range_is_rejected() {
        let err = MieruOutboundConfig::from_proxy(&parse_entry(
            "name: m\ntype: mieru\nserver: m.example\nport: 2999\nusername: bob\npassword: p\nport-range: 2000-3000\n",
        ))
        .unwrap_err();
        assert!(err.to_string().contains("port-range"), "{err}");
    }

    #[test]
    fn missing_credentials_are_rejected() {
        let err = MieruOutboundConfig::from_proxy(&parse_entry(
            "name: m\ntype: mieru\nserver: m.example\nport: 2999\npassword: p\n",
        ))
        .unwrap_err();
        assert!(err.to_string().contains("username"), "{err}");
    }

    #[test]
    fn hash_password_matches_known_vector() {
        // SHA256("secret" ‖ 0x00 ‖ "bob").
        let got = hash_password(b"secret", b"bob");
        let mut h = Sha256::new();
        h.update(b"secret");
        h.update([0x00]);
        h.update(b"bob");
        let want: [u8; 32] = h.finalize().into();
        assert_eq!(got, want);
    }

    #[test]
    fn derive_session_key_matches_reference() {
        // Cross-checked against Python:
        //   pw = sha256(b"secret" + b"\x00" + b"bob")
        //   salt = sha256(struct.pack(">Q", 1782662760))
        //   pbkdf2_hmac("sha256", pw, salt, 64, 32)
        // 1782662760 is a multiple of 120, so it is its own rounded window.
        let key = derive_session_key(b"secret", b"bob", 1782662760);
        let want = [
            0x4b, 0xee, 0x3a, 0x73, 0x83, 0xde, 0x56, 0xba, 0xa0, 0x01, 0x55, 0x1a, 0x9f, 0x1c, 0x8f, 0x48, 0x19, 0x85,
            0xb2, 0xe9, 0x5c, 0xf3, 0x3e, 0xf1, 0x03, 0xce, 0x97, 0x82, 0xa6, 0xb8, 0xf7, 0xdd,
        ];
        assert_eq!(key, want);
    }

    #[test]
    fn salt_rounds_to_two_minute_grid() {
        // 13:00:59 rounds down to 13:00:00; 13:01:00 rounds up to 13:02:00.
        assert_eq!(current_salt(59), current_salt(0));
        assert_eq!(current_salt(60), current_salt(120));
        assert_ne!(current_salt(0), current_salt(120));
    }

    #[test]
    fn nonce_increment_carries() {
        let mut n = [0u8; NONCE_SIZE];
        n[NONCE_SIZE - 1] = 0xff;
        increment_nonce(&mut n);
        assert_eq!(n[NONCE_SIZE - 1], 0x00);
        assert_eq!(n[NONCE_SIZE - 2], 0x01);
    }

    #[test]
    fn user_hint_is_deterministic_and_in_tail() {
        let mut nonce = [7u8; NONCE_SIZE];
        let before = nonce;
        add_user_hint(&mut nonce, b"alice");
        // Only the last 4 bytes change.
        assert_eq!(
            nonce[..NONCE_SIZE - USER_HINT_SUFFIX],
            before[..NONCE_SIZE - USER_HINT_SUFFIX]
        );
        let mut h = Sha256::new();
        h.update(b"alice");
        h.update(&before[..USER_HINT_PREFIX]);
        let out = h.finalize();
        assert_eq!(&nonce[NONCE_SIZE - USER_HINT_SUFFIX..], &out[..USER_HINT_SUFFIX]);
    }

    #[test]
    fn empty_username_leaves_nonce_untouched() {
        let mut nonce = [3u8; NONCE_SIZE];
        let before = nonce;
        add_user_hint(&mut nonce, b"");
        assert_eq!(nonce, before);
    }

    #[test]
    fn cipher_round_trip_with_implicit_nonce() {
        let key = [9u8; KEY_LEN];
        let mut enc = MieruCipher::new(&key, b"bob");
        let mut dec = MieruCipher::new(&key, &[]);
        // A sequence of messages must decrypt in the same order (the first
        // carries the nonce; the rest ride the incremented implicit nonce).
        for i in 0..8u8 {
            let msg = vec![i; (i as usize + 1) * 10];
            let sealed = enc.encrypt(&msg);
            if i == 0 {
                assert!(sealed.len() >= NONCE_SIZE + OVERHEAD);
            }
            let opened = dec.decrypt(&sealed).expect("decrypt");
            assert_eq!(opened, msg);
        }
    }

    #[test]
    fn wrong_key_fails_to_decrypt() {
        let mut enc = MieruCipher::new(&[1u8; KEY_LEN], &[]);
        let mut dec = MieruCipher::new(&[2u8; KEY_LEN], &[]);
        let sealed = enc.encrypt(b"hello");
        assert!(dec.decrypt(&sealed).is_err());
    }

    #[test]
    fn session_meta_layout() {
        let meta = marshal_session_meta(PROTO_OPEN_SESSION_REQUEST, 0x01020304, 0, 11);
        assert_eq!(meta[0], PROTO_OPEN_SESSION_REQUEST);
        assert_eq!(&meta[6..10], &0x01020304u32.to_be_bytes());
        assert_eq!(u16::from_be_bytes([meta[15], meta[16]]), 11);
        let lens = segment_lens(&meta).unwrap();
        assert_eq!(lens.payload_len, 11);
        assert_eq!(lens.prefix_len, 0);
        assert!(!lens.is_close);
    }

    #[test]
    fn data_meta_layout() {
        let meta = marshal_data_meta(PROTO_DATA_CLIENT_TO_SERVER, 7, 3, 4096);
        assert_eq!(meta[0], PROTO_DATA_CLIENT_TO_SERVER);
        assert_eq!(&meta[10..14], &3u32.to_be_bytes());
        let lens = segment_lens(&meta).unwrap();
        assert_eq!(lens.payload_len, 4096);
    }

    #[test]
    fn close_segment_is_flagged() {
        let meta = marshal_session_meta(PROTO_CLOSE_SESSION_REQUEST, 1, 5, 0);
        assert!(segment_lens(&meta).unwrap().is_close);
    }

    #[test]
    fn build_socks5_connect_domain() {
        let req = build_socks5_connect(&TargetAddr::Domain("example.com".to_string(), 443)).unwrap();
        assert_eq!(req[0], socks5::VERSION);
        assert_eq!(req[1], socks5::CMD_CONNECT);
        assert_eq!(req[3], socks5::ATYP_DOMAIN);
        assert_eq!(req[4] as usize, "example.com".len());
        assert_eq!(&req[5..5 + 11], b"example.com");
        assert_eq!(&req[16..18], &443u16.to_be_bytes());
    }
}
