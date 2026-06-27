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
//! The kernel pools sessions per server: each outbound connection runs one
//! logical stream, but when that stream closes cleanly the TLS connection is
//! returned to a per-server idle pool and the next connection to the same server
//! reuses it — opening a fresh stream with the next `cmdSYN` instead of a new TLS
//! handshake + auth (anytls-go's idle-session reuse). Streams are sequential per
//! connection (one active stream at a time); concurrent multiplexing of several
//! streams over one connection is left for later. A reused connection is
//! liveness-probed first and pooled entries expire on an idle TTL, so a server
//! that dropped an idle connection at most costs one wasted reuse attempt.
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
use std::future::poll_fn;
use std::io;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::task::{Context as TaskContext, Poll, ready};
use std::time::{Duration, Instant};

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
/// The first stream id opened on a fresh session. anytls stream ids are
/// monotonic within a session; reusing a pooled connection opens the next id,
/// so leftover frames from an earlier (closed) stream carry an older id and are
/// skipped by the new stream.
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

/// Per-server pool of idle AnyTLS sessions: live TLS connections whose current
/// stream has closed cleanly, available for the next connection to that server
/// to reuse (opening a fresh stream id) instead of doing another TLS handshake +
/// auth — anytls-go's idle-session reuse. Each entry records when it was returned
/// so stale ones are evicted on access. Process-wide, like [`SCHEME_STORE`], with
/// the same lazily-initialised `Mutex<Option<HashMap>>` idiom.
static SESSION_POOL: Mutex<Option<HashMap<ServerKey, Vec<(Instant, SessionState)>>>> = Mutex::new(None);

/// How long an idle pooled session may live before it is discarded on the next
/// access. Comfortably under typical anytls server idle timeouts; since a reused
/// session is also liveness-probed, an over-long TTL costs at most one wasted
/// reuse attempt, never a relayed connection.
const SESSION_IDLE_TTL: Duration = Duration::from_secs(30);
/// Cap on idle sessions pooled per server, bounding memory and fd use.
const SESSION_POOL_MAX: usize = 8;

/// Take a live idle session for `key` from the pool, or `None` if there is no
/// reusable one. Entries idle past [`SESSION_IDLE_TTL`] are evicted, and each
/// candidate is liveness-probed (a closed/broken connection is discarded and the
/// next one tried) so a returned session is ready for a fresh stream.
async fn take_pooled(key: &ServerKey) -> Option<SessionState> {
    loop {
        let candidate = {
            let mut guard = SESSION_POOL.lock().expect("anytls session pool");
            let map = guard.as_mut()?;
            let list = map.get_mut(key)?;
            list.retain(|(since, _)| since.elapsed() <= SESSION_IDLE_TTL);
            // Most-recently returned first: likelier to still be alive.
            list.pop().map(|(_, session)| session)
        };
        let mut session = candidate?;
        if probe_alive(&mut session).await {
            return Some(session);
        }
        // Dead connection: drop it (closing the transport) and try the next.
    }
}

/// Cheap liveness check before committing to reuse: poll the transport once
/// without blocking. `Pending` (no data) presumes it alive; readable bytes are
/// stale frames from the prior stream, buffered for the new stream's reader to
/// skip by id; a clean EOF or error means the server closed the connection.
async fn probe_alive(session: &mut SessionState) -> bool {
    let mut scratch = [0u8; 4096];
    let mut read_buf = ReadBuf::new(&mut scratch);
    let outcome = poll_fn(|cx| match Pin::new(&mut session.inner).poll_read(cx, &mut read_buf) {
        Poll::Ready(result) => Poll::Ready(Some(result)),
        Poll::Pending => Poll::Ready(None),
    })
    .await;
    match outcome {
        None => true,
        Some(Ok(())) => {
            let filled = read_buf.filled();
            if filled.is_empty() {
                false
            } else {
                session.read_raw.extend_from_slice(filled);
                true
            }
        }
        Some(Err(_)) => false,
    }
}

/// Return a cleanly-closed session to its server's idle pool for reuse, unless
/// the pool is at capacity (then the session is dropped, closing the connection).
fn return_to_pool(key: ServerKey, session: SessionState) {
    let mut guard = SESSION_POOL.lock().expect("anytls session pool");
    let map = guard.get_or_insert_with(HashMap::new);
    let list = map.entry(key).or_default();
    if list.len() < SESSION_POOL_MAX {
        list.push((Instant::now(), session));
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
    Ok(Box::new(acquire_stream(config, target).await?))
}

/// Acquire an AnyTLS stream to `target`: reuse a live pooled session for the
/// config's server if one is available (opening a fresh stream on it), otherwise
/// establish a new TLS session (handshake + auth + `cmdSettings`). Either way the
/// returned stream has its opening `cmdSYN` + `cmdPSH`(target) flushed and is
/// ready to relay.
async fn acquire_stream(config: &AnyTlsOutboundConfig, target: &TargetAddr) -> Result<AnyTlsStream> {
    let key = ServerKey {
        server: config.server.clone(),
        port: config.port,
    };

    // Reuse path: a pooled connection just needs a new `cmdSYN` + `cmdPSH`(target)
    // for its next stream id; the session keeps its own shaper and scheme.
    if let Some(session) = take_pooled(&key).await {
        let sid = session.stream_id;
        let mut anytls = AnyTlsStream::from_pool(session, key);
        anytls.enqueue_session_unit(build_stream_open(sid, target));
        anytls.flush().await.context("anytls: open stream on pooled session")?;
        return Ok(anytls);
    }

    // New-session path: TLS handshake, auth (packet 0), then the padded packet-1
    // flush of `cmdSettings` + `cmdSYN` + `cmdPSH`(target), as anytls-go does
    // after `OpenStream` clears buffering.
    let scheme = current_scheme(&key);
    let mut transport = transport::establish(&config.server, config.port, &config.security, &config.transport).await?;
    transport
        .write_all(&build_auth_header(&config.password_sha256, &scheme))
        .await
        .context("anytls: send auth header")?;
    let init = build_session_init(&scheme, target);
    let mut anytls = AnyTlsStream::new(transport, (*scheme).clone(), key, STREAM_ID);
    anytls.enqueue_session_unit(init);
    anytls.flush().await.context("anytls: send settings + open stream")?;
    Ok(anytls)
}

/// Open an AnyTLS outbound for UDP datagrams to `target` via udp-over-tcp v2
/// (sing `common/uot`). The session stream is opened to the UoT magic address;
/// the first application bytes are the UoT *connect* request (`IsConnect=1` +
/// SOCKS5 destination), after which every datagram is framed as `len(u16 BE) |
/// payload` in both directions (connect mode carries no per-packet address).
/// One stream is opened per destination, matching the relay's per-target model.
pub async fn connect_udp(config: &AnyTlsOutboundConfig, target: &TargetAddr) -> Result<BoxedStream> {
    // Open the session stream to the UoT magic address (new or reused), then send
    // the UoT connect request as its first application bytes.
    let magic = TargetAddr::Domain(UOT_MAGIC_ADDRESS.to_string(), 0);
    let mut anytls = acquire_stream(config, &magic).await?;
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

/// Build the bytes opening a stream on an already-established (pooled) session:
/// `cmdSYN` with the next stream id, then the `cmdPSH` carrying the SOCKS5-encoded
/// proxy target. No `cmdSettings` — that is sent once at session creation. The
/// caller feeds the blob through the session's shaper as one `writeConn` unit.
fn build_stream_open(stream_id: u32, target: &TargetAddr) -> Vec<u8> {
    let mut buf = Vec::with_capacity(FRAME_HEADER_LEN * 2 + 64);
    push_frame(&mut buf, CMD_SYN, stream_id, &[]);
    let mut addr = Vec::with_capacity(1 + 256 + 2);
    socks5::encode_address(&mut addr, target);
    push_frame(&mut buf, CMD_PSH, stream_id, &addr);
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

/// The live IO + shaping state of an AnyTLS session that can outlive a single
/// logical stream: the TLS transport, the padding shaper (whose `writeConn`
/// counter is per-connection), the outgoing record queue, the unparsed read
/// buffer, and the current stream id. It is moved out of a cleanly-closed
/// [`AnyTlsStream`] back into [`SESSION_POOL`] so the next connection to the same
/// server reuses the TLS connection (a fresh `cmdSYN` with the next stream id)
/// instead of handshaking again.
struct SessionState {
    inner: BoxedStream,
    /// Outgoing records pending write to the inner transport. Each entry is one
    /// intended inner write (one TLS record) produced by the padding shaper, so
    /// record sizes follow the scheme rather than leaking frame boundaries.
    out: VecDeque<Vec<u8>>,
    /// Bytes already written from `out.front()` (partial-write resume point).
    out_pos: usize,
    /// Padding-scheme state shaping the outgoing record stream.
    shaper: PaddingShaper,
    /// Raw bytes read from the inner transport not yet parsed into frames.
    read_raw: Vec<u8>,
    /// The id of the stream currently open on this connection. Monotonic across
    /// reuses, so frames left over from a previous (closed) stream carry an
    /// older id and are skipped.
    stream_id: u32,
}

impl SessionState {
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

    /// Shape one `writeConn` unit of complete frame bytes into the outgoing queue.
    fn enqueue(&mut self, frame_bytes: Vec<u8>) {
        self.shaper.shape(&mut self.out, frame_bytes);
    }
}

/// Session-layer stream over the TLS transport: relay writes become `cmdPSH`
/// frames; reads strip the framing and surface only this stream's `cmdPSH`
/// payload, handling the control frames (`cmdSYNACK`/`cmdFIN`/`cmdAlert`/
/// `cmdHeartRequest`/padding) transparently. On a clean close (our `cmdFIN` sent
/// and the server's received, transport intact) the [`SessionState`] is returned
/// to the per-server pool for reuse; otherwise it is dropped, closing the
/// connection.
struct AnyTlsStream {
    /// `Some` while live; taken on `Drop` to either pool the session (clean
    /// close) or drop the transport (closing the connection).
    state: Option<SessionState>,
    /// Server endpoint: routes a received `cmdUpdatePaddingScheme` to the right
    /// server's scheme store, and keys the session pool on close.
    server_key: ServerKey,
    /// Decoded `cmdPSH` payload pending delivery to the reader.
    plain: Vec<u8>,
    plain_pos: usize,
    /// Reads are exhausted (the server closed this stream, or the transport).
    eof: bool,
    /// The server closed this stream with `cmdFIN` (as opposed to the transport
    /// dying) — a precondition for returning the connection to the pool.
    stream_finished: bool,
    /// The transport hit EOF/error, or the server rejected the stream / sent
    /// `cmdAlert` — the connection is unusable and must not be pooled.
    broken: bool,
    /// We have sent our `cmdFIN` for this stream.
    fin_sent: bool,
}

impl AnyTlsStream {
    fn new(inner: BoxedStream, scheme: PaddingScheme, server_key: ServerKey, stream_id: u32) -> Self {
        Self::with_state(
            SessionState {
                inner,
                out: VecDeque::new(),
                out_pos: 0,
                shaper: PaddingShaper::new(scheme),
                read_raw: Vec::new(),
                stream_id,
            },
            server_key,
        )
    }

    /// Build a stream over a session taken from the pool, opening a fresh stream
    /// on the already-authenticated connection.
    fn from_pool(session: SessionState, server_key: ServerKey) -> Self {
        Self::with_state(session, server_key)
    }

    fn with_state(state: SessionState, server_key: ServerKey) -> Self {
        Self {
            state: Some(state),
            server_key,
            plain: Vec::new(),
            plain_pos: 0,
            eof: false,
            stream_finished: false,
            broken: false,
            fin_sent: false,
        }
    }

    /// The current stream id (the id this logical stream's frames carry).
    fn stream_id(&self) -> u32 {
        self.state.as_ref().map_or(0, |s| s.stream_id)
    }

    /// Enqueue a multi-frame `writeConn` unit (the packet-1 settings + SYN + PSH
    /// blob, or a reused session's SYN + PSH) through the padding shaper.
    fn enqueue_session_unit(&mut self, frame_bytes: Vec<u8>) {
        if let Some(state) = self.state.as_mut() {
            state.enqueue(frame_bytes);
        }
    }

    /// Whether the connection can be returned to the pool: our `cmdFIN` was sent,
    /// the server closed its half with `cmdFIN`, and nothing broke the transport.
    fn reusable(&self) -> bool {
        self.fin_sent && self.stream_finished && !self.broken && self.state.is_some()
    }
}

impl Drop for AnyTlsStream {
    fn drop(&mut self) {
        if !self.reusable() {
            return;
        }
        if let Some(mut session) = self.state.take() {
            // The next stream on this connection gets the following id; any
            // leftover frames from the just-closed stream carry the old id and
            // are skipped by the reusing reader.
            session.stream_id = session.stream_id.wrapping_add(1);
            return_to_pool(self.server_key.clone(), session);
        }
    }
}

impl AsyncRead for AnyTlsStream {
    fn poll_read(self: Pin<&mut Self>, cx: &mut TaskContext<'_>, buf: &mut ReadBuf<'_>) -> Poll<io::Result<()>> {
        let this = self.get_mut();
        let state = this.state.as_mut().expect("anytls stream state present");
        // Best-effort flush of any queued control replies (e.g. heart responses
        // produced while parsing). Errors/pending here do not block the read.
        let _ = state.poll_drain(cx);

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
            let need = if state.read_raw.len() < FRAME_HEADER_LEN {
                FRAME_HEADER_LEN
            } else {
                FRAME_HEADER_LEN + u16::from_be_bytes([state.read_raw[5], state.read_raw[6]]) as usize
            };
            if state.read_raw.len() < need {
                let mut scratch = [0u8; 4096];
                let mut read_buf = ReadBuf::new(&mut scratch);
                ready!(Pin::new(&mut state.inner).poll_read(cx, &mut read_buf))?;
                let filled = read_buf.filled();
                if filled.is_empty() {
                    // Peer closed the transport; the stream ends but the
                    // connection is broken and must not be pooled.
                    this.eof = true;
                    this.broken = true;
                    return Poll::Ready(Ok(()));
                }
                state.read_raw.extend_from_slice(filled);
                continue;
            }

            let cmd = state.read_raw[0];
            let stream_id = u32::from_be_bytes([
                state.read_raw[1],
                state.read_raw[2],
                state.read_raw[3],
                state.read_raw[4],
            ]);
            let len = u16::from_be_bytes([state.read_raw[5], state.read_raw[6]]) as usize;
            let data: Vec<u8> = state.read_raw[FRAME_HEADER_LEN..FRAME_HEADER_LEN + len].to_vec();
            state.read_raw.drain(..FRAME_HEADER_LEN + len);

            match cmd {
                CMD_PSH if stream_id == state.stream_id => {
                    this.plain = data;
                    this.plain_pos = 0;
                }
                CMD_FIN if stream_id == state.stream_id => {
                    this.eof = true;
                    this.stream_finished = true;
                    return Poll::Ready(Ok(()));
                }
                CMD_SYNACK if stream_id == state.stream_id && !data.is_empty() => {
                    this.broken = true;
                    return Poll::Ready(Err(io::Error::new(
                        io::ErrorKind::ConnectionRefused,
                        format!("anytls: stream rejected: {}", String::from_utf8_lossy(&data)),
                    )));
                }
                CMD_ALERT => {
                    this.broken = true;
                    return Poll::Ready(Err(io::Error::other(format!(
                        "anytls: server alert: {}",
                        String::from_utf8_lossy(&data)
                    ))));
                }
                CMD_HEART_REQUEST => {
                    let mut frame = Vec::with_capacity(FRAME_HEADER_LEN);
                    push_frame(&mut frame, CMD_HEART_RESPONSE, stream_id, &[]);
                    state.enqueue(frame);
                    let _ = state.poll_drain(cx);
                }
                // Store a server-pushed scheme for this server's future
                // connections; the current session keeps its own scheme.
                CMD_UPDATE_PADDING_SCHEME => apply_scheme_update(&this.server_key, &data),
                // Padding, settings, heart responses, the stream's own
                // SYN/SYNACK(ok), and frames left over from a previous stream (an
                // older id) carry nothing this stream needs: read past them.
                CMD_WASTE | CMD_SETTINGS | CMD_SERVER_SETTINGS | CMD_HEART_RESPONSE | CMD_SYN | CMD_SYNACK
                | CMD_PSH | CMD_FIN => {}
                _ => {}
            }
        }
    }
}

impl AsyncWrite for AnyTlsStream {
    fn poll_write(self: Pin<&mut Self>, cx: &mut TaskContext<'_>, buf: &[u8]) -> Poll<io::Result<usize>> {
        let this = self.get_mut();
        let sid = this.stream_id();
        let state = this.state.as_mut().expect("anytls stream state present");
        ready!(state.poll_drain(cx))?;
        if buf.is_empty() {
            return Poll::Ready(Ok(0));
        }
        let take = buf.len().min(MAX_PSH_CHUNK);
        let mut frame = Vec::with_capacity(FRAME_HEADER_LEN + take);
        push_frame(&mut frame, CMD_PSH, sid, &buf[..take]);
        state.enqueue(frame);
        if let Poll::Ready(Err(e)) = state.poll_drain(cx) {
            return Poll::Ready(Err(e));
        }
        Poll::Ready(Ok(take))
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut TaskContext<'_>) -> Poll<io::Result<()>> {
        let this = self.get_mut();
        let state = this.state.as_mut().expect("anytls stream state present");
        ready!(state.poll_drain(cx))?;
        Pin::new(&mut state.inner).poll_flush(cx)
    }

    fn poll_shutdown(self: Pin<&mut Self>, cx: &mut TaskContext<'_>) -> Poll<io::Result<()>> {
        let this = self.get_mut();
        let sid = this.stream_id();
        let state = this.state.as_mut().expect("anytls stream state present");
        ready!(state.poll_drain(cx))?;
        if !this.fin_sent {
            let mut frame = Vec::with_capacity(FRAME_HEADER_LEN);
            push_frame(&mut frame, CMD_FIN, sid, &[]);
            state.enqueue(frame);
            this.fin_sent = true;
        }
        ready!(state.poll_drain(cx))?;
        // Do not shut down the inner transport: `cmdFIN` closes only this stream,
        // leaving the TLS connection healthy for reuse. The transport is closed
        // by dropping the session when it is not returned to the pool.
        Pin::new(&mut state.inner).poll_flush(cx)
    }
}

#[cfg(test)]
mod tests {
    use std::net::{Ipv4Addr, SocketAddr};
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::time::Duration;

    use super::*;
    use crate::config::outbound_opts::ProxyEntry;
    use crate::transport::tls::ClientFingerprint;
    use tokio::io::AsyncReadExt;
    use tokio::net::{TcpListener, TcpStream};

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

    // ---- Session-pool reuse (PR B) ----------------------------------------

    /// Read one session frame from a server-side socket, or `None` at EOF.
    async fn read_frame_opt(stream: &mut TcpStream) -> Option<(u8, u32, Vec<u8>)> {
        let mut header = [0u8; FRAME_HEADER_LEN];
        stream.read_exact(&mut header).await.ok()?;
        let cmd = header[0];
        let sid = u32::from_be_bytes([header[1], header[2], header[3], header[4]]);
        let len = u16::from_be_bytes([header[5], header[6]]) as usize;
        let mut data = vec![0u8; len];
        stream.read_exact(&mut data).await.ok()?;
        Some((cmd, sid, data))
    }

    /// Write one session frame to a server-side socket.
    async fn server_write(stream: &mut TcpStream, cmd: u8, sid: u32, data: &[u8]) {
        let mut frame = Vec::new();
        push_frame(&mut frame, cmd, sid, data);
        stream.write_all(&frame).await.unwrap();
    }

    /// A minimal anytls server that handles multiple *sequential* streams on one
    /// connection: it records each `cmdSYN`'s stream id, acks it, treats the
    /// first `cmdPSH` of a stream as the target address and echoes the rest, and
    /// answers `cmdFIN` with `cmdFIN` (closing only that stream). If
    /// `close_after_first_stream` is set it drops the connection after the first
    /// stream's FIN, simulating a server that reaped an idle connection.
    async fn pool_test_serve(mut stream: TcpStream, sids: Arc<Mutex<Vec<u32>>>, close_after_first_stream: bool) {
        // Note: callers set `close_after_first_stream` only for the connection
        // that is expected to be pooled, so the replacement connection still
        // closes cleanly.
        let mut hash = [0u8; 32];
        if stream.read_exact(&mut hash).await.is_err() {
            return;
        }
        let mut padding_len = [0u8; 2];
        stream.read_exact(&mut padding_len).await.unwrap();
        let padding_len = u16::from_be_bytes(padding_len) as usize;
        if padding_len > 0 {
            let mut padding = vec![0u8; padding_len];
            stream.read_exact(&mut padding).await.unwrap();
        }

        let mut awaiting_addr = false;
        let mut streams_done = 0u32;
        while let Some((cmd, sid, data)) = read_frame_opt(&mut stream).await {
            match cmd {
                CMD_WASTE => {}
                CMD_SETTINGS => server_write(&mut stream, CMD_SERVER_SETTINGS, 0, b"v=2").await,
                CMD_SYN => {
                    sids.lock().unwrap().push(sid);
                    server_write(&mut stream, CMD_SYNACK, sid, &[]).await;
                    awaiting_addr = true;
                }
                CMD_PSH => {
                    if awaiting_addr {
                        awaiting_addr = false; // target address; not echoed
                    } else {
                        server_write(&mut stream, CMD_PSH, sid, &data).await;
                    }
                }
                CMD_FIN => {
                    server_write(&mut stream, CMD_FIN, sid, &[]).await;
                    streams_done += 1;
                    if close_after_first_stream && streams_done == 1 {
                        // Reap the connection so the pooled session goes dead, but
                        // do it gracefully: half-close (a clean FIN the client's
                        // liveness probe reads as EOF), then drain until the client
                        // closes, so Windows never turns the drop into an RST.
                        let _ = stream.shutdown().await;
                        let mut sink = [0u8; 64];
                        while let Ok(n) = stream.read(&mut sink).await {
                            if n == 0 {
                                break;
                            }
                        }
                        return;
                    }
                }
                _ => {}
            }
        }
    }

    /// Spawn `pool_test_serve` accepting on a fresh port; returns the address, the
    /// recorded stream ids, and the count of accepted TCP connections.
    async fn spawn_pool_server(close_first_connection: bool) -> (SocketAddr, Arc<Mutex<Vec<u32>>>, Arc<AtomicUsize>) {
        let sids = Arc::new(Mutex::new(Vec::<u32>::new()));
        let conns = Arc::new(AtomicUsize::new(0));
        let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).await.unwrap();
        let addr = listener.local_addr().unwrap();
        let (sids_task, conns_task) = (sids.clone(), conns.clone());
        tokio::spawn(async move {
            while let Ok((stream, _)) = listener.accept().await {
                let index = conns_task.fetch_add(1, Ordering::SeqCst);
                // Only the first connection (the one that gets pooled) is reaped.
                let close = close_first_connection && index == 0;
                tokio::spawn(pool_test_serve(stream, sids_task.clone(), close));
            }
        });
        (addr, sids, conns)
    }

    fn pool_test_config(addr: SocketAddr) -> AnyTlsOutboundConfig {
        AnyTlsOutboundConfig {
            server: addr.ip().to_string(),
            port: addr.port(),
            password_sha256: Sha256::digest(b"password").into(),
            security: Security::None,
            transport: Transport::Tcp,
        }
    }

    fn pool_len(key: &ServerKey) -> usize {
        SESSION_POOL
            .lock()
            .unwrap()
            .as_ref()
            .and_then(|map| map.get(key))
            .map_or(0, |list| list.len())
    }

    /// Drive a relay-style round trip on `stream`, then close it cleanly (send
    /// our `cmdFIN`, read the server's `cmdFIN` to EOF) so the session qualifies
    /// for pooling once the stream is dropped.
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
    async fn pool_reuses_session_and_increments_stream_id() {
        let (addr, sids, conns) = spawn_pool_server(false).await;
        let config = pool_test_config(addr);
        let target = TargetAddr::Domain("example.com".to_string(), 443);
        let key = ServerKey {
            server: config.server.clone(),
            port: config.port,
        };

        // Stream 1 over a fresh session; a clean close pools the connection.
        {
            let mut s1 = connect(&config, &target).await.unwrap();
            round_trip_and_close(&mut s1, b"first").await;
        }
        assert_eq!(pool_len(&key), 1, "clean close returns the session to the pool");

        // Stream 2 must reuse it: no new TCP connection, next stream id.
        {
            let mut s2 = connect(&config, &target).await.unwrap();
            round_trip_and_close(&mut s2, b"second").await;
        }

        assert_eq!(
            conns.load(Ordering::SeqCst),
            1,
            "second stream reused one TCP connection"
        );
        assert_eq!(
            *sids.lock().unwrap(),
            vec![1, 2],
            "sequential stream ids on the reused connection"
        );
    }

    #[tokio::test]
    async fn pool_discards_dead_session_on_reuse() {
        // The server drops the connection after the first stream, so the pooled
        // session is dead by the time it is reused.
        let (addr, sids, conns) = spawn_pool_server(true).await;
        let config = pool_test_config(addr);
        let target = TargetAddr::Domain("example.com".to_string(), 443);
        let key = ServerKey {
            server: config.server.clone(),
            port: config.port,
        };

        {
            let mut s1 = connect(&config, &target).await.unwrap();
            round_trip_and_close(&mut s1, b"first").await;
        }
        // The session is pooled (we received the stream FIN before the server
        // closed the transport, which we have not observed yet).
        assert_eq!(pool_len(&key), 1, "session is pooled before the probe runs");

        // Let the server's transport close propagate so the liveness probe sees
        // it deterministically (loopback FIN delivery; mirrors relay.rs sleeps).
        tokio::time::sleep(Duration::from_millis(100)).await;

        // Reuse must detect the dead connection, discard it, and dial a new one.
        {
            let mut s2 = connect(&config, &target).await.unwrap();
            round_trip_and_close(&mut s2, b"second").await;
        }

        assert_eq!(
            conns.load(Ordering::SeqCst),
            2,
            "dead session discarded; a new connection dialled"
        );
        assert_eq!(
            *sids.lock().unwrap(),
            vec![1, 1],
            "the replacement session starts a fresh stream id"
        );
    }
}
