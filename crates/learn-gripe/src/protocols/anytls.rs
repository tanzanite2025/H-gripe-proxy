//! AnyTLS outbound.
//!
//! AnyTLS ("any TLS") rides a normal TLS connection and runs a small session
//! layer on top whose purpose is traffic-shaping: it multiplexes logical
//! streams inside one TLS connection and (optionally) pads records so their
//! sizes do not leak the proxied protocol. The transport (tcp/ws/grpc/…) and
//! security (tls/reality) layers are provided by [`crate::transport`] via the
//! shared [`crate::transport::build_layers`]; this module is purely the AnyTLS
//! session framing on top. AnyTLS is TLS-by-default (the whole point), so
//! security defaults to TLS unless overridden.
//!
//! Wire format (after the TLS handshake completes), per the upstream spec
//! (`anytls/anytls-go` `docs/protocol.md`):
//!
//! 1. **Authentication** — the client immediately sends
//!    `SHA256(password) (32) | padding0_len (u16 BE) | padding0`.
//! 2. **Session frames** — `cmd(1) | streamId(u32 BE) | len(u16 BE) | data`.
//!    The client must send `cmdSettings` first, then opens a stream with
//!    `cmdSYN`, writes the proxy target as a SOCKS5 address (RFC 1928 §5) in a
//!    `cmdPSH`, and relays the payload as further `cmdPSH` frames. `cmdFIN`
//!    marks EOF. A v2 server answers `cmdSYN` with `cmdSYNACK` (empty = ok, data
//!    = error text) and `cmdSettings` with `cmdServerSettings`.
//!
//! The kernel opens one TLS connection (one session, one stream) per outbound
//! connection — analogous to Trojan — rather than pooling/reusing sessions; a
//! fresh session always creating a new stream is conformant (it is just the
//! "no idle session" branch of the reuse rule).
//!
//! **Padding-scheme traffic shaping** is applied, matching upstream
//! (`anytls-go` `proxy/session/session.go` `writeConn` + `proxy/padding`). The
//! client advertises the default scheme's `padding-md5` and shapes its writes
//! by it: `padding0` zero bytes ride the auth header (packet 0), and the first
//! `stop` "TLS packets" (a packet = one `writeConn` flush) are split/padded to
//! the scheme's per-packet record sizes, inserting `cmdWaste` frames to fill
//! short writes and emitting standalone `cmdWaste` records where the scheme
//! calls for pure padding. Packet 1 is the combined `cmdSettings` +
//! `cmdSYN` + `cmdPSH(target)` flush; packet 2 onward are the relay's data
//! writes. Padding is byte-level shaping the server discards transparently
//! (`cmdWaste` is dropped, frame boundaries are recovered from the length
//! field), so it never affects interop — it only obscures record sizes. A
//! server-pushed `cmdUpdatePaddingScheme` is parsed and stored per server (keyed
//! by `server:port`): the current connection keeps shaping by its own scheme,
//! but subsequent connections to that server advertise and shape by the updated
//! scheme, exactly as anytls-go's per-server `Client` does. A stock server does
//! not push one anyway since the advertised md5 already matches the default.
//! UDP rides sing-box udp-over-tcp v2 (see [`connect_udp`]).

use std::collections::HashMap;
use std::collections::VecDeque;
use std::io;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::task::{Context as TaskContext, Poll, ready};

use anyhow::{Context, Result};
use md5::Md5;
use sha2::{Digest, Sha256};
use tokio::io::{AsyncRead, AsyncWrite, AsyncWriteExt, ReadBuf};

use crate::address::TargetAddr;
use crate::config::outbound_opts::ProxyEntry;
use crate::inbound::socks5;
use crate::outbound::BoxedStream;
use crate::transport::{self, Security, Transport};

// Session-layer commands (anytls protocol, "since version 1" + "since version 2").
const CMD_WASTE: u8 = 0;
const CMD_SYN: u8 = 1;
const CMD_PSH: u8 = 2;
const CMD_FIN: u8 = 3;
const CMD_SETTINGS: u8 = 4;
const CMD_ALERT: u8 = 5;
const CMD_UPDATE_PADDING_SCHEME: u8 = 6;
const CMD_SYNACK: u8 = 7;
const CMD_HEART_REQUEST: u8 = 8;
const CMD_HEART_RESPONSE: u8 = 9;
const CMD_SERVER_SETTINGS: u8 = 10;

const FRAME_HEADER_LEN: usize = 7;
/// The single stream opened per session. anytls stream ids are monotonic within
/// a session; with one stream per connection it is always the first id.
const STREAM_ID: u32 = 1;
/// Cap on a single `cmdPSH` payload (the frame length field is a `u16`); a
/// comfortable margin keeps frames small without excessive overhead.
const MAX_PSH_CHUNK: usize = 8192;

/// Implemented protocol version reported in `cmdSettings` (`v=2`).
const PROTOCOL_VERSION: u8 = 2;

/// udp-over-tcp v2 magic destination (sing `common/uot`). A `cmdPSH` to this
/// FQDN tells the server the stream carries UoT-framed datagrams rather than a
/// raw TCP relay.
const UOT_MAGIC_ADDRESS: &str = "sp.v2.udp-over-tcp.arpa";
/// `client` identifier reported in `cmdSettings` (real name, per the spec —
/// spoofing it is pointless).
const CLIENT_NAME: &str = concat!("learn-gripe/", env!("CARGO_PKG_VERSION"));

/// The upstream default padding scheme (anytls-go `proxy/padding/padding.go`).
/// We both advertise its md5 and shape traffic by it. No trailing newline — it
/// must hash identically to the upstream bytes.
const DEFAULT_PADDING_SCHEME: &str = "stop=8\n\
0=30-30\n\
1=100-400\n\
2=400-500,c,500-1000,c,500-1000,c,500-1000,c,500-1000\n\
3=9-9,500-1000\n\
4=500-1000\n\
5=500-1000\n\
6=500-1000\n\
7=500-1000";

/// Lowercase-hex md5 of [`DEFAULT_PADDING_SCHEME`] — what we advertise in
/// `cmdSettings`, and the baseline against which a pushed scheme is judged
/// "different" (so a stock server's matching scheme is a no-op).
const DEFAULT_PADDING_MD5: &str = "75cff2ad89aadf5e257059ee571ebe11";

/// Sentinel returned by [`PaddingScheme::record_payload_sizes`] for the scheme's
/// `c` token (anytls `padding.CheckMark`): "if the user payload is exhausted,
/// stop emitting padding records for this packet; otherwise carry on".
const CHECK_MARK: i64 = -1;

/// One token of a padding-scheme packet entry: either a `min-max` byte-size
/// range or the `c` check mark.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SizeToken {
    /// A `min-max` range; `record_payload_sizes` resolves it to a random size in
    /// `[min, max)` (upper-exclusive, matching anytls `rand.Int(max-min)+min`),
    /// or exactly `min` when `min == max`.
    Range(i64, i64),
    /// The `c` check mark.
    Check,
}

/// A parsed anytls padding scheme (anytls-go `proxy/padding`): the lowercase-hex
/// md5 of the raw bytes (advertised in `cmdSettings`), the `stop` packet count,
/// and the per-packet size-token lists keyed by packet index.
#[derive(Debug, Clone)]
struct PaddingScheme {
    md5_hex: String,
    stop: u32,
    packets: HashMap<u32, Vec<SizeToken>>,
}

impl PaddingScheme {
    /// Parse a raw scheme (`key=value` lines, `\n`-separated, per
    /// `util.StringMapFromBytes`). Returns `None` if there is no usable `stop`
    /// line — matching anytls `NewPaddingFactory`, which rejects such schemes.
    fn parse(raw: &[u8]) -> Option<Self> {
        let md5_hex = md5_hex(raw);
        let text = String::from_utf8_lossy(raw);
        let mut stop = None;
        let mut packets: HashMap<u32, Vec<SizeToken>> = HashMap::new();
        for line in text.split('\n') {
            let Some((key, value)) = line.split_once('=') else {
                continue;
            };
            if key == "stop" {
                stop = value.trim().parse::<u32>().ok();
            } else if let Ok(pkt) = key.parse::<u32>() {
                let tokens = value
                    .split(',')
                    .filter_map(|tok| match tok {
                        "c" => Some(SizeToken::Check),
                        _ => {
                            let (lo, hi) = tok.split_once('-')?;
                            let (lo, hi) = (lo.parse::<i64>().ok()?, hi.parse::<i64>().ok()?);
                            let (lo, hi) = (lo.min(hi), lo.max(hi));
                            // anytls skips non-positive ranges.
                            (lo > 0 && hi > 0).then_some(SizeToken::Range(lo, hi))
                        }
                    })
                    .collect();
                packets.insert(pkt, tokens);
            }
        }
        stop.map(|stop| Self { md5_hex, stop, packets })
    }

    /// The built-in default scheme (always parses).
    fn default_scheme() -> Self {
        Self::parse(DEFAULT_PADDING_SCHEME.as_bytes()).expect("default padding scheme parses")
    }

    /// Resolve packet `pkt`'s tokens to concrete record payload sizes, picking a
    /// fresh random length within each range and mapping `c` to [`CHECK_MARK`].
    /// An undefined packet yields an empty list (anytls sends it unshaped).
    fn record_payload_sizes(&self, pkt: u32) -> Vec<i64> {
        let Some(tokens) = self.packets.get(&pkt) else {
            return Vec::new();
        };
        tokens
            .iter()
            .map(|tok| match *tok {
                SizeToken::Check => CHECK_MARK,
                SizeToken::Range(lo, hi) if lo == hi => lo,
                // Upper-exclusive, like anytls `rand.Int(big.NewInt(max-min))+min`.
                SizeToken::Range(lo, hi) => lo + random_below((hi - lo) as u64) as i64,
            })
            .collect()
    }
}

/// Lowercase-hex md5 of `data`.
fn md5_hex(data: &[u8]) -> String {
    let digest = Md5::digest(data);
    let mut out = String::with_capacity(32);
    for byte in digest {
        out.push_str(&format!("{byte:02x}"));
    }
    out
}

/// A uniform-ish random integer in `[0, n)` (0 when `n == 0`). Modulo bias is
/// irrelevant for traffic-padding sizes (`n` is at most a few thousand).
fn random_below(n: u64) -> u64 {
    if n == 0 {
        return 0;
    }
    let mut bytes = [0u8; 8];
    getrandom::fill(&mut bytes).expect("os rng");
    u64::from_le_bytes(bytes) % n
}

/// Identifies an AnyTLS server endpoint for the per-server padding-scheme store.
/// A server-pushed `cmdUpdatePaddingScheme` applies only to connections to that
/// same server (anytls-go stores it on the per-server `Client`).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct ServerKey {
    server: String,
    port: u16,
}

/// Per-server override schemes learned from `cmdUpdatePaddingScheme`; a missing
/// entry means "use the built-in default". Process-wide because outbound
/// `connect`s are independent calls with no other shared state, matching
/// anytls-go's per-server `Client` storage. `None` is the lazily-initialised
/// empty map (the `static Mutex<Option<HashMap>>` idiom used elsewhere in the
/// crate avoids a separate lazy-init dependency).
static SCHEME_STORE: Mutex<Option<HashMap<ServerKey, Arc<PaddingScheme>>>> = Mutex::new(None);

/// The padding scheme a new connection to `key` should use: the server's pushed
/// scheme if one has been learned, else the built-in default.
fn current_scheme(key: &ServerKey) -> Arc<PaddingScheme> {
    let store = SCHEME_STORE.lock().expect("anytls scheme store");
    store
        .as_ref()
        .and_then(|map| map.get(key).cloned())
        .unwrap_or_else(|| Arc::new(PaddingScheme::default_scheme()))
}

/// Apply a server-pushed `cmdUpdatePaddingScheme` for `key`: parse it and, when
/// it is valid and its md5 differs from the scheme currently in effect, store it
/// so subsequent connections to that server advertise and shape by it (anytls-go
/// `UpdatePaddingScheme`). The connection that received it keeps its own scheme.
fn apply_scheme_update(key: &ServerKey, raw: &[u8]) {
    let Some(scheme) = PaddingScheme::parse(raw) else {
        return;
    };
    let mut store = SCHEME_STORE.lock().expect("anytls scheme store");
    let map = store.get_or_insert_with(HashMap::new);
    let current_md5 = map.get(key).map_or(DEFAULT_PADDING_MD5, |s| s.md5_hex.as_str());
    if scheme.md5_hex != current_md5 {
        map.insert(key.clone(), Arc::new(scheme));
    }
}

/// Fully-resolved AnyTLS outbound parameters.
///
/// `security` and `transport` are orthogonal layers (see [`crate::transport`]).
/// The password is pre-hashed into its 32-byte `SHA256` form — exactly the
/// on-wire authenticator — so the dial path never touches the raw secret again.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AnyTlsOutboundConfig {
    pub server: String,
    pub port: u16,
    pub password_sha256: [u8; 32],
    pub security: Security,
    pub transport: Transport,
}

impl AnyTlsOutboundConfig {
    /// Build an outbound config from a parsed `anytls` proxy entry.
    pub fn from_proxy(entry: &ProxyEntry) -> Result<Self> {
        let opts = &entry.options;
        let server = opts
            .server
            .clone()
            .filter(|s| !s.is_empty())
            .context("anytls: missing server")?;
        let port = opts.port.context("anytls: missing port")?;
        let password = opts
            .password
            .as_deref()
            .filter(|s| !s.is_empty())
            .context("anytls: missing password")?;
        let password_sha256 = Sha256::digest(password.as_bytes()).into();

        // AnyTLS always rides TLS; security and transport are orthogonal to the
        // session framing and are built by the shared layer helper.
        let (security, transport) = transport::build_layers(opts, "anytls", true, false)?;

        Ok(Self {
            server,
            port,
            password_sha256,
            security,
            transport,
        })
    }
}

/// Connect an AnyTLS outbound to `target`: establish the TLS transport, send the
/// auth header (packet 0, padding0 from the scheme), then the padded packet-1
/// flush of `cmdSettings` + `cmdSYN` + `cmdPSH`(target address), and hand back a
/// stream that frames relay traffic as `cmdPSH` and decodes the server's frames.
pub async fn connect(config: &AnyTlsOutboundConfig, target: &TargetAddr) -> Result<BoxedStream> {
    let key = ServerKey {
        server: config.server.clone(),
        port: config.port,
    };
    let scheme = current_scheme(&key);
    let mut transport = transport::establish(&config.server, config.port, &config.security, &config.transport).await?;
    transport
        .write_all(&build_auth_header(&config.password_sha256, &scheme))
        .await
        .context("anytls: send auth header")?;

    // Packet 1: the buffered `cmdSettings` + `cmdSYN` + `cmdPSH`(target) flush,
    // shaped together as anytls-go does after `OpenStream` clears buffering.
    let init = build_session_init(&scheme, target);
    let mut anytls = AnyTlsStream::new(transport, (*scheme).clone(), key);
    anytls.enqueue_session_unit(init);
    anytls.flush().await.context("anytls: send settings + open stream")?;
    Ok(Box::new(anytls))
}

/// Open an AnyTLS outbound for UDP datagrams to `target` via udp-over-tcp v2
/// (sing `common/uot`). The session stream is opened to the UoT magic address;
/// the first application bytes are the UoT *connect* request (`IsConnect=1` +
/// SOCKS5 destination), after which every datagram is framed as `len(u16 BE) |
/// payload` in both directions (connect mode carries no per-packet address).
/// One stream is opened per destination, matching the relay's per-target model.
pub async fn connect_udp(config: &AnyTlsOutboundConfig, target: &TargetAddr) -> Result<BoxedStream> {
    let key = ServerKey {
        server: config.server.clone(),
        port: config.port,
    };
    let scheme = current_scheme(&key);
    let magic = TargetAddr::Domain(UOT_MAGIC_ADDRESS.to_string(), 0);
    let mut transport = transport::establish(&config.server, config.port, &config.security, &config.transport).await?;
    transport
        .write_all(&build_auth_header(&config.password_sha256, &scheme))
        .await
        .context("anytls udp: send auth header")?;

    // Packet 1: settings + SYN + PSH(UoT magic address).
    let init = build_session_init(&scheme, &magic);
    let mut anytls = AnyTlsStream::new(transport, (*scheme).clone(), key);
    anytls.enqueue_session_unit(init);
    // UoT v2 request: IsConnect (1) + SOCKS5-encoded destination. Sent as the
    // stream's first `cmdPSH` payload.
    let mut request = Vec::with_capacity(1 + 1 + 256 + 2);
    request.push(1u8); // IsConnect = true (fixed destination per stream)
    socks5::encode_address(&mut request, target);
    anytls
        .write_all(&request)
        .await
        .context("anytls udp: send uot request")?;
    anytls.flush().await.context("anytls udp: flush uot request")?;
    Ok(Box::new(anytls))
}

/// Append one session frame (`cmd | streamId | len | data`) to `buf`.
fn push_frame(buf: &mut Vec<u8>, cmd: u8, stream_id: u32, data: &[u8]) {
    buf.push(cmd);
    buf.extend_from_slice(&stream_id.to_be_bytes());
    buf.extend_from_slice(&(data.len() as u16).to_be_bytes());
    buf.extend_from_slice(data);
}

/// Append a `cmdWaste` frame carrying `payload_len` zero bytes of padding.
fn push_waste(buf: &mut Vec<u8>, payload_len: usize) {
    buf.push(CMD_WASTE);
    buf.extend_from_slice(&0u32.to_be_bytes());
    buf.extend_from_slice(&(payload_len as u16).to_be_bytes());
    buf.resize(buf.len() + payload_len, 0);
}

/// Build the auth header (packet 0): `SHA256(password)`, the `padding0` length,
/// then that many zero bytes. `padding0` is the scheme's packet-0 size (anytls
/// `GenerateRecordPayloadSizes(0)[0]`); the default scheme yields 30 bytes.
fn build_auth_header(password_sha256: &[u8; 32], scheme: &PaddingScheme) -> Vec<u8> {
    let padding0 = match scheme.record_payload_sizes(0).first().copied() {
        Some(size) if size > 0 => size as usize,
        _ => 0,
    };
    let mut buf = Vec::with_capacity(32 + 2 + padding0);
    buf.extend_from_slice(password_sha256);
    buf.extend_from_slice(&(padding0 as u16).to_be_bytes());
    buf.resize(buf.len() + padding0, 0);
    buf
}

/// Build the packet-1 session bytes: `cmdSettings` (advertising the scheme md5),
/// `cmdSYN` opening the stream, and the `cmdPSH` carrying the SOCKS5-encoded
/// proxy target. The caller feeds the whole blob through the padding shaper as a
/// single `writeConn` unit.
fn build_session_init(scheme: &PaddingScheme, target: &TargetAddr) -> Vec<u8> {
    let mut buf = Vec::with_capacity(64 + FRAME_HEADER_LEN * 2 + 64);
    let settings = format!(
        "v={PROTOCOL_VERSION}\nclient={CLIENT_NAME}\npadding-md5={}",
        scheme.md5_hex
    );
    push_frame(&mut buf, CMD_SETTINGS, 0, settings.as_bytes());
    push_frame(&mut buf, CMD_SYN, STREAM_ID, &[]);
    let mut addr = Vec::with_capacity(1 + 256 + 2);
    socks5::encode_address(&mut addr, target);
    push_frame(&mut buf, CMD_PSH, STREAM_ID, &addr);
    buf
}

/// Drives the anytls padding scheme over the outgoing frame stream. Each call to
/// [`PaddingShaper::shape`] is one anytls "TLS packet" (`writeConn` flush): the
/// packet counter advances, and while it is below the scheme's `stop` the frame
/// bytes are split into records of the scheme's sizes — emitting `cmdWaste`
/// frames to fill short writes — exactly as anytls-go `Session.writeConn` does.
struct PaddingShaper {
    scheme: PaddingScheme,
    /// Number of packets (flushes) shaped so far; the next flush is `pkt + 1`.
    pkt: u32,
    /// Cleared once the packet counter reaches `stop`; thereafter frames pass
    /// through unshaped (matching anytls clearing `sendPadding`).
    send_padding: bool,
}

impl PaddingShaper {
    fn new(scheme: PaddingScheme) -> Self {
        Self {
            scheme,
            pkt: 0,
            send_padding: true,
        }
    }

    /// Shape one `writeConn` unit of complete frame bytes into the record queue
    /// `out`, appending `cmdWaste` padding per the scheme for the current packet.
    fn shape(&mut self, out: &mut VecDeque<Vec<u8>>, frame_bytes: Vec<u8>) {
        if self.send_padding {
            self.pkt += 1;
            if self.pkt < self.scheme.stop {
                self.shape_packet(out, frame_bytes);
                return;
            }
            self.send_padding = false;
        }
        out.push_back(frame_bytes);
    }

    /// The padded-packet branch of anytls `writeConn`: walk the scheme's record
    /// sizes, chopping `frame_bytes` into records and inserting `cmdWaste`.
    fn shape_packet(&self, out: &mut VecDeque<Vec<u8>>, frame_bytes: Vec<u8>) {
        let mut pos = 0usize;
        for size in self.scheme.record_payload_sizes(self.pkt) {
            let remain = frame_bytes.len() - pos;
            if size == CHECK_MARK {
                // Stop padding once the payload is drained; else keep going.
                if remain == 0 {
                    break;
                }
                continue;
            }
            let size = size as usize;
            if remain > size {
                // Record is entirely real payload (a prefix; may cut mid-frame,
                // which is transparent once the peer reassembles by length).
                out.push_back(frame_bytes[pos..pos + size].to_vec());
                pos += size;
            } else if remain > 0 {
                // Last of the payload, padded up to `size` with one `cmdWaste`.
                let mut record = frame_bytes[pos..].to_vec();
                pos = frame_bytes.len();
                let pad = size as isize - remain as isize - FRAME_HEADER_LEN as isize;
                if pad > 0 {
                    push_waste(&mut record, pad as usize);
                }
                out.push_back(record);
            } else {
                // Payload exhausted: a standalone `cmdWaste` record of `size`.
                let mut record = Vec::with_capacity(FRAME_HEADER_LEN + size);
                push_waste(&mut record, size);
                out.push_back(record);
            }
        }
        // Any payload the scheme did not cover is sent as a final record.
        if pos < frame_bytes.len() {
            out.push_back(frame_bytes[pos..].to_vec());
        }
    }
}

/// Session-layer stream over the TLS transport: relay writes become `cmdPSH`
/// frames; reads strip the framing and surface only this stream's `cmdPSH`
/// payload, handling the control frames (`cmdSYNACK`/`cmdFIN`/`cmdAlert`/
/// `cmdHeartRequest`/padding) transparently.
struct AnyTlsStream<S> {
    inner: S,
    /// Outgoing records pending write to the inner transport. Each entry is one
    /// intended inner write (one TLS record) produced by the padding shaper, so
    /// record sizes follow the scheme rather than leaking frame boundaries.
    out: VecDeque<Vec<u8>>,
    /// Bytes already written from `out.front()` (partial-write resume point).
    out_pos: usize,
    /// Padding-scheme state shaping the outgoing record stream.
    shaper: PaddingShaper,
    /// Server endpoint, so a `cmdUpdatePaddingScheme` received on this connection
    /// is stored against the right server for its future connections.
    server_key: ServerKey,
    /// Raw bytes read from the inner transport not yet parsed into frames.
    read_raw: Vec<u8>,
    /// Decoded `cmdPSH` payload pending delivery to the reader.
    plain: Vec<u8>,
    plain_pos: usize,
    eof: bool,
    fin_sent: bool,
}

impl<S> AnyTlsStream<S> {
    fn new(inner: S, scheme: PaddingScheme, server_key: ServerKey) -> Self {
        Self {
            inner,
            out: VecDeque::new(),
            out_pos: 0,
            shaper: PaddingShaper::new(scheme),
            server_key,
            read_raw: Vec::new(),
            plain: Vec::new(),
            plain_pos: 0,
            eof: false,
            fin_sent: false,
        }
    }

    /// Enqueue a multi-frame `writeConn` unit (e.g. the packet-1 settings + SYN +
    /// PSH blob) through the padding shaper.
    fn enqueue_session_unit(&mut self, frame_bytes: Vec<u8>) {
        self.shaper.shape(&mut self.out, frame_bytes);
    }
}

impl<S: AsyncWrite + Unpin> AnyTlsStream<S> {
    /// Flush queued outgoing records to the inner transport, one record per
    /// `poll_write` so each becomes its own TLS record.
    fn poll_drain(&mut self, cx: &mut TaskContext<'_>) -> Poll<io::Result<()>> {
        while let Some(front) = self.out.front() {
            if self.out_pos >= front.len() {
                self.out.pop_front();
                self.out_pos = 0;
                continue;
            }
            let n = ready!(Pin::new(&mut self.inner).poll_write(cx, &front[self.out_pos..]))?;
            if n == 0 {
                return Poll::Ready(Err(io::Error::new(io::ErrorKind::WriteZero, "anytls: write zero")));
            }
            self.out_pos += n;
        }
        Poll::Ready(Ok(()))
    }
}

impl<S: AsyncRead + AsyncWrite + Unpin> AsyncRead for AnyTlsStream<S> {
    fn poll_read(self: Pin<&mut Self>, cx: &mut TaskContext<'_>, buf: &mut ReadBuf<'_>) -> Poll<io::Result<()>> {
        let this = self.get_mut();
        // Best-effort flush of any queued control replies (e.g. heart responses
        // produced while parsing). Errors/pending here do not block the read.
        let _ = this.poll_drain(cx);

        loop {
            if this.plain_pos < this.plain.len() {
                let n = buf.remaining().min(this.plain.len() - this.plain_pos);
                buf.put_slice(&this.plain[this.plain_pos..this.plain_pos + n]);
                this.plain_pos += n;
                return Poll::Ready(Ok(()));
            }
            if this.eof {
                return Poll::Ready(Ok(()));
            }

            // Need a full frame: a 7-byte header, then its `len` body bytes.
            let need = if this.read_raw.len() < FRAME_HEADER_LEN {
                FRAME_HEADER_LEN
            } else {
                FRAME_HEADER_LEN + u16::from_be_bytes([this.read_raw[5], this.read_raw[6]]) as usize
            };
            if this.read_raw.len() < need {
                let mut scratch = [0u8; 4096];
                let mut read_buf = ReadBuf::new(&mut scratch);
                ready!(Pin::new(&mut this.inner).poll_read(cx, &mut read_buf))?;
                let filled = read_buf.filled();
                if filled.is_empty() {
                    // Peer closed the transport; treat as end of this stream.
                    this.eof = true;
                    return Poll::Ready(Ok(()));
                }
                this.read_raw.extend_from_slice(filled);
                continue;
            }

            let cmd = this.read_raw[0];
            let stream_id =
                u32::from_be_bytes([this.read_raw[1], this.read_raw[2], this.read_raw[3], this.read_raw[4]]);
            let len = u16::from_be_bytes([this.read_raw[5], this.read_raw[6]]) as usize;
            let data: Vec<u8> = this.read_raw[FRAME_HEADER_LEN..FRAME_HEADER_LEN + len].to_vec();
            this.read_raw.drain(..FRAME_HEADER_LEN + len);

            match cmd {
                CMD_PSH if stream_id == STREAM_ID => {
                    this.plain = data;
                    this.plain_pos = 0;
                }
                CMD_FIN if stream_id == STREAM_ID => {
                    this.eof = true;
                    return Poll::Ready(Ok(()));
                }
                CMD_SYNACK if !data.is_empty() => {
                    return Poll::Ready(Err(io::Error::new(
                        io::ErrorKind::ConnectionRefused,
                        format!("anytls: stream rejected: {}", String::from_utf8_lossy(&data)),
                    )));
                }
                CMD_ALERT => {
                    return Poll::Ready(Err(io::Error::other(format!(
                        "anytls: server alert: {}",
                        String::from_utf8_lossy(&data)
                    ))));
                }
                CMD_HEART_REQUEST => {
                    let mut frame = Vec::with_capacity(FRAME_HEADER_LEN);
                    push_frame(&mut frame, CMD_HEART_RESPONSE, stream_id, &[]);
                    this.shaper.shape(&mut this.out, frame);
                    let _ = this.poll_drain(cx);
                }
                // Store a server-pushed scheme for this server's future
                // connections; the current session keeps its own scheme.
                CMD_UPDATE_PADDING_SCHEME => apply_scheme_update(&this.server_key, &data),
                // Padding, settings, heart responses, the stream's own
                // SYN/SYNACK(ok), and frames for other streams carry nothing this
                // single-stream relay needs: read past them.
                CMD_WASTE | CMD_SETTINGS | CMD_SERVER_SETTINGS | CMD_HEART_RESPONSE | CMD_SYN | CMD_SYNACK
                | CMD_PSH | CMD_FIN => {}
                _ => {}
            }
        }
    }
}

impl<S: AsyncWrite + Unpin> AsyncWrite for AnyTlsStream<S> {
    fn poll_write(self: Pin<&mut Self>, cx: &mut TaskContext<'_>, buf: &[u8]) -> Poll<io::Result<usize>> {
        let this = self.get_mut();
        ready!(this.poll_drain(cx))?;
        if buf.is_empty() {
            return Poll::Ready(Ok(0));
        }
        let take = buf.len().min(MAX_PSH_CHUNK);
        let mut frame = Vec::with_capacity(FRAME_HEADER_LEN + take);
        push_frame(&mut frame, CMD_PSH, STREAM_ID, &buf[..take]);
        this.shaper.shape(&mut this.out, frame);
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
        if !this.fin_sent {
            let mut frame = Vec::with_capacity(FRAME_HEADER_LEN);
            push_frame(&mut frame, CMD_FIN, STREAM_ID, &[]);
            this.shaper.shape(&mut this.out, frame);
            this.fin_sent = true;
        }
        ready!(this.poll_drain(cx))?;
        Pin::new(&mut this.inner).poll_shutdown(cx)
    }
}

#[cfg(test)]
mod tests {
    use std::net::{Ipv4Addr, SocketAddr};

    use super::*;
    use crate::config::outbound_opts::ProxyEntry;
    use crate::transport::tls::ClientFingerprint;

    fn parse_entry(yaml: &str) -> ProxyEntry {
        serde_yaml_ng::from_str(yaml).expect("parse proxy entry")
    }

    #[test]
    fn padding_md5_matches_upstream_default_scheme() {
        // Cross-checked against `md5sum` of anytls-go's default padding scheme.
        assert_eq!(DEFAULT_PADDING_MD5, "75cff2ad89aadf5e257059ee571ebe11");
        assert_eq!(PaddingScheme::default_scheme().md5_hex, DEFAULT_PADDING_MD5);
    }

    #[test]
    fn default_scheme_parses_stop_and_packet_tokens() {
        let scheme = PaddingScheme::default_scheme();
        assert_eq!(scheme.stop, 8);
        // Packet 0 is the fixed 30-byte auth padding0.
        assert_eq!(scheme.record_payload_sizes(0), vec![30]);
        // Packet 1 is a single range in [100, 400).
        let one = scheme.record_payload_sizes(1);
        assert_eq!(one.len(), 1);
        assert!((100..400).contains(&one[0]), "{one:?}");
        // Packet 2: 5 ranges interleaved with 4 check marks.
        let two = scheme.record_payload_sizes(2);
        assert_eq!(two.len(), 9);
        assert_eq!(two.iter().filter(|&&s| s == CHECK_MARK).count(), 4, "{two:?}");
        assert!((400..500).contains(&two[0]), "{two:?}");
        // An undefined packet shapes nothing.
        assert!(scheme.record_payload_sizes(99).is_empty());
    }

    #[test]
    fn auth_header_carries_password_and_padding0() {
        let password_sha256: [u8; 32] = Sha256::digest(b"secret").into();
        let scheme = PaddingScheme::default_scheme();
        let auth = build_auth_header(&password_sha256, &scheme);
        // SHA256(password) | padding0_len(=30) | 30 zero bytes.
        assert_eq!(&auth[..32], &password_sha256);
        assert_eq!(&auth[32..34], &30u16.to_be_bytes());
        assert_eq!(auth.len(), 34 + 30);
        assert!(auth[34..].iter().all(|&b| b == 0), "padding0 must be zero");
    }

    #[test]
    fn session_init_carries_settings_syn_and_target() {
        let target = TargetAddr::Domain("example.com".to_string(), 443);
        let scheme = PaddingScheme::default_scheme();
        let init = build_session_init(&scheme, &target);

        // cmdSettings frame (sid 0) with v / client / padding-md5.
        let mut pos = 0;
        assert_eq!(init[pos], CMD_SETTINGS);
        assert_eq!(&init[pos + 1..pos + 5], &0u32.to_be_bytes());
        let settings_len = u16::from_be_bytes([init[pos + 5], init[pos + 6]]) as usize;
        let settings = &init[pos + FRAME_HEADER_LEN..pos + FRAME_HEADER_LEN + settings_len];
        let settings = std::str::from_utf8(settings).unwrap();
        assert!(settings.contains("v=2"), "{settings}");
        assert!(
            settings.contains("padding-md5=75cff2ad89aadf5e257059ee571ebe11"),
            "{settings}"
        );
        pos += FRAME_HEADER_LEN + settings_len;

        // cmdSYN frame for the stream, no data.
        assert_eq!(init[pos], CMD_SYN);
        assert_eq!(&init[pos + 1..pos + 5], &STREAM_ID.to_be_bytes());
        assert_eq!(&init[pos + 5..pos + 7], &0u16.to_be_bytes());
        pos += FRAME_HEADER_LEN;

        // cmdPSH frame carrying the SOCKS5-encoded target.
        assert_eq!(init[pos], CMD_PSH);
        assert_eq!(&init[pos + 1..pos + 5], &STREAM_ID.to_be_bytes());
        let addr_len = u16::from_be_bytes([init[pos + 5], init[pos + 6]]) as usize;
        let mut expected = Vec::new();
        socks5::encode_address(&mut expected, &target);
        assert_eq!(
            &init[pos + FRAME_HEADER_LEN..pos + FRAME_HEADER_LEN + addr_len],
            &expected[..]
        );
    }

    /// Concatenate the shaper's output records back into the byte stream the peer
    /// receives.
    fn drain(out: &VecDeque<Vec<u8>>) -> Vec<u8> {
        out.iter().flatten().copied().collect()
    }

    /// Walk a frame stream, returning the `(cmd, payload-len)` of each frame.
    /// Panics on a truncated trailer, proving the stream is exactly frame-aligned.
    fn frames(bytes: &[u8]) -> Vec<(u8, usize)> {
        let mut out = Vec::new();
        let mut pos = 0;
        while pos < bytes.len() {
            assert!(pos + FRAME_HEADER_LEN <= bytes.len(), "truncated frame header");
            let cmd = bytes[pos];
            let len = u16::from_be_bytes([bytes[pos + 5], bytes[pos + 6]]) as usize;
            assert!(pos + FRAME_HEADER_LEN + len <= bytes.len(), "truncated frame body");
            out.push((cmd, len));
            pos += FRAME_HEADER_LEN + len;
        }
        out
    }

    /// Parse a scheme with deterministic fixed-size records (min == max).
    fn fixed_scheme(raw: &str) -> PaddingScheme {
        PaddingScheme::parse(raw.as_bytes()).expect("scheme parses")
    }

    #[test]
    fn shaper_pads_short_payload_up_to_record_size() {
        // Packet 1 is one fixed 100-byte record, so a small frame is padded with
        // a trailing cmdWaste to reach exactly 100 bytes.
        let mut shaper = PaddingShaper::new(fixed_scheme("stop=8\n1=100-100"));
        let mut out = VecDeque::new();
        let mut frame = Vec::new();
        push_frame(&mut frame, CMD_PSH, STREAM_ID, b"hi"); // 7 + 2 = 9 bytes
        shaper.shape(&mut out, frame);

        let stream = drain(&out);
        assert_eq!(stream.len(), 100, "record padded to scheme size");
        // Real PSH(2) then a cmdWaste filling the rest: 100 - 9 - 7 = 84 bytes.
        assert_eq!(frames(&stream), vec![(CMD_PSH, 2), (CMD_WASTE, 84)]);
    }

    #[test]
    fn shaper_emits_standalone_waste_when_payload_exhausted() {
        // Two ranges but a tiny payload: the first record carries the payload +
        // padding, the second is pure cmdWaste (no check mark stops it).
        let mut shaper = PaddingShaper::new(fixed_scheme("stop=8\n1=50-50,60-60"));
        let mut out = VecDeque::new();
        let mut frame = Vec::new();
        push_frame(&mut frame, CMD_PSH, STREAM_ID, b"x"); // 8 bytes
        shaper.shape(&mut out, frame);

        assert_eq!(out.len(), 2, "two records");
        // The payload+padding record is exactly the scheme size (50). The pure
        // cmdWaste record is `header + size` (= 7 + 60), matching upstream's
        // `make([]byte, headerOverHeadSize+l)` for the all-padding branch.
        assert_eq!(out[0].len(), 50);
        assert_eq!(out[1].len(), FRAME_HEADER_LEN + 60);
        assert_eq!(frames(&out[0]), vec![(CMD_PSH, 1), (CMD_WASTE, 50 - 8 - 7)]);
        assert_eq!(frames(&out[1]), vec![(CMD_WASTE, 60)]);
    }

    #[test]
    fn shaper_check_mark_stops_padding_when_drained() {
        // After the first range consumes the payload, the `c` check mark stops
        // further padding records for this packet.
        let mut shaper = PaddingShaper::new(fixed_scheme("stop=8\n1=50-50,c,500-500"));
        let mut out = VecDeque::new();
        let mut frame = Vec::new();
        push_frame(&mut frame, CMD_PSH, STREAM_ID, b"x");
        shaper.shape(&mut out, frame);

        assert_eq!(out.len(), 1, "check mark halted padding");
        assert_eq!(out[0].len(), 50);
    }

    #[test]
    fn shaper_splits_large_payload_and_keeps_bytes_intact() {
        // A payload larger than the single record size is split: one record of
        // the scheme size, then the remainder. No bytes are added or lost.
        let mut shaper = PaddingShaper::new(fixed_scheme("stop=8\n1=40-40"));
        let mut out = VecDeque::new();
        let mut frame = Vec::new();
        let payload: Vec<u8> = (0..200u32).map(|i| i as u8).collect();
        push_frame(&mut frame, CMD_PSH, STREAM_ID, &payload);
        let expected = frame.clone();
        shaper.shape(&mut out, frame);

        assert_eq!(out[0].len(), 40, "first record is the scheme size");
        assert!(out.len() >= 2, "remainder spilled into more records");
        assert_eq!(drain(&out), expected, "payload bytes unchanged, just rechunked");
    }

    #[test]
    fn shaper_stops_after_scheme_stop_packet() {
        let mut shaper = PaddingShaper::new(fixed_scheme("stop=2\n1=100-100"));
        let mut out = VecDeque::new();
        // Packet 1 is shaped (padded to 100).
        let mut f1 = Vec::new();
        push_frame(&mut f1, CMD_PSH, STREAM_ID, b"a");
        shaper.shape(&mut out, f1);
        assert_eq!(out[0].len(), 100);
        out.clear();
        // Packet 2 reaches `stop`: passed through verbatim, padding disabled.
        let mut f2 = Vec::new();
        push_frame(&mut f2, CMD_PSH, STREAM_ID, b"b");
        let raw = f2.clone();
        shaper.shape(&mut out, f2);
        assert!(!shaper.send_padding);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0], raw);
    }

    #[test]
    fn defaults_to_tls_security() {
        let yaml = "name: a\ntype: anytls\nserver: example.com\nport: 443\npassword: secret\n";
        let cfg = AnyTlsOutboundConfig::from_proxy(&parse_entry(yaml)).unwrap();
        assert!(matches!(cfg.security, Security::Tls(_)));
        assert!(matches!(cfg.transport, Transport::Tcp));
        assert_eq!(cfg.password_sha256, <[u8; 32]>::from(Sha256::digest(b"secret")));
    }

    #[test]
    fn missing_password_is_rejected() {
        let yaml = "name: a\ntype: anytls\nserver: example.com\nport: 443\n";
        let err = AnyTlsOutboundConfig::from_proxy(&parse_entry(yaml)).unwrap_err();
        assert!(err.to_string().contains("password"), "got: {err}");
    }

    #[test]
    fn missing_server_is_rejected() {
        let yaml = "name: a\ntype: anytls\nport: 443\npassword: secret\n";
        let err = AnyTlsOutboundConfig::from_proxy(&parse_entry(yaml)).unwrap_err();
        assert!(err.to_string().contains("server"), "got: {err}");
    }

    #[test]
    fn sni_and_skip_cert_verify_flow_into_tls() {
        let yaml = "name: a\ntype: anytls\nserver: example.com\nport: 443\npassword: secret\n\
             sni: real.example\nskip-cert-verify: true\nclient-fingerprint: chrome\n";
        let cfg = AnyTlsOutboundConfig::from_proxy(&parse_entry(yaml)).unwrap();
        match cfg.security {
            Security::Tls(tls) => {
                assert_eq!(tls.server_name.as_deref(), Some("real.example"));
                assert!(tls.skip_cert_verify);
                assert_eq!(tls.client_fingerprint, Some(ClientFingerprint::Chrome));
            }
            other => panic!("expected TLS security, got {other:?}"),
        }
    }

    #[test]
    fn scheme_update_is_stored_per_server_and_applied_to_new_connections() {
        let key = ServerKey {
            server: "scheme-update-apply.invalid".to_string(),
            port: 443,
        };
        // An unknown server falls back to the built-in default scheme.
        assert_eq!(current_scheme(&key).md5_hex, DEFAULT_PADDING_MD5);

        // A pushed scheme with a different md5 is adopted for future connections.
        let pushed = b"stop=4\n0=20-20\n1=120-120";
        apply_scheme_update(&key, pushed);
        let now = current_scheme(&key);
        assert_eq!(now.md5_hex, md5_hex(pushed));
        assert_eq!(now.stop, 4);
        assert_eq!(now.record_payload_sizes(0), vec![20]);

        // Storage is per server: another endpoint is unaffected.
        let other = ServerKey {
            server: "scheme-update-other.invalid".to_string(),
            port: 443,
        };
        assert_eq!(current_scheme(&other).md5_hex, DEFAULT_PADDING_MD5);
    }

    #[test]
    fn scheme_update_ignores_unchanged_and_invalid_schemes() {
        let key = ServerKey {
            server: "scheme-update-noop.invalid".to_string(),
            port: 1,
        };
        // Re-pushing the default scheme (same md5) is a no-op: still default.
        apply_scheme_update(&key, DEFAULT_PADDING_SCHEME.as_bytes());
        assert_eq!(current_scheme(&key).md5_hex, DEFAULT_PADDING_MD5);

        // A scheme without a `stop` line fails to parse and is ignored.
        apply_scheme_update(&key, b"0=10-10");
        assert_eq!(current_scheme(&key).md5_hex, DEFAULT_PADDING_MD5);
    }
}
