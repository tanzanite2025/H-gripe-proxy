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
//! The kernel multiplexes streams over per-server sessions: each session is one
//! live TLS connection driven by a background task that owns the transport,
//! demultiplexing inbound `cmdPSH`/`cmdFIN` to each logical stream and
//! serialising every stream's writes through the per-session padding shaper. A
//! new outbound connection opens another stream on an existing session to the
//! same server (a fresh `cmdSYN` with the next id) — running concurrently with
//! that session's other streams — and only does a new TLS handshake + auth when
//! no session has a free slot (`MAX_STREAMS_PER_SESSION`). A session also stays
//! registered while idle (no open streams) so a later connection reuses it
//! instead of handshaking, expiring on an idle TTL; once broken or idle-expired
//! it is evicted and its connection closed. Because the shared reader stalls all
//! of a session's streams while one stream's bounded inbound buffer is full,
//! per-session fan-out is capped (matching anytls-go's bounded per-stream pipe).
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
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::{Arc, Mutex};
use std::task::{Context as TaskContext, Poll, Waker};
use std::time::{Duration, Instant};

use anyhow::{Context, Result, anyhow};
use md5::Md5;
use sha2::{Digest, Sha256};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt, ReadBuf, ReadHalf, WriteHalf};
use tokio::sync::{Notify, mpsc, oneshot};

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

/// Maximum logical streams multiplexed concurrently on one TLS session before a
/// fresh connection is opened. Because the session's single reader stalls all of
/// its streams while one stream's bounded inbound buffer is full, this caps the
/// fan-out (and head-of-line coupling) a slow consumer can impose.
const MAX_STREAMS_PER_SESSION: u32 = 8;
/// Bounded depth (frames) of each per-stream inbound channel: caps buffering and
/// backpressures the shared reader (anytls-go's bounded per-stream pipe).
const STREAM_RECV_CAP: usize = 16;
/// Bounded depth of the shared outbound write channel feeding the session
/// writer; together with the writer awaiting each record it bounds write memory.
const SESSION_WRITE_CAP: usize = 64;
/// How long a session with no open streams stays registered for reuse before it
/// is evicted on the next access and its connection closed.
const SESSION_IDLE_TTL: Duration = Duration::from_secs(30);
/// Cap on live sessions tracked per server, bounding memory and fd use.
const SESSION_POOL_MAX: usize = 8;

/// Control message from a logical stream to its session driver. Unbounded (low
/// volume) so `cmdFIN`/close can be queued synchronously from `Drop`.
enum Ctrl {
    /// Open a new stream to `target`: the driver allocates the next id, writes
    /// `cmdSYN` + `cmdPSH`(target) (prefixed with `cmdSettings` on the session's
    /// first stream), registers `data` for inbound payloads, and replies the id.
    Open {
        target: TargetAddr,
        data: mpsc::Sender<Vec<u8>>,
        reply: oneshot::Sender<io::Result<u32>>,
    },
    /// Send this stream's `cmdFIN` (our half-close); keep routing inbound to it.
    Fin { sid: u32 },
    /// The stream was dropped: stop routing inbound to it and free its slot.
    Close { sid: u32 },
}

/// One application write on a stream: a `writeConn` unit the driver frames as
/// `cmdPSH`(s) and shapes. Bounded ([`SESSION_WRITE_CAP`]) for backpressure.
struct WriteMsg {
    sid: u32,
    data: Vec<u8>,
}

/// State shared between a session's reader task, writer task and streams: the
/// inbound demux table, liveness flag, writer-backpressure wakers, and the
/// signal that lets the writer tell the reader to stop.
struct SessionShared {
    /// Open streams: id → inbound payload sender, used by the reader to demux
    /// `cmdPSH` and registered by the writer when it opens a stream. Dropping a
    /// sender (on `cmdFIN`/close) gives that stream's reader EOF.
    streams: Mutex<HashMap<u32, mpsc::Sender<Vec<u8>>>>,
    /// Set when the connection is unusable (transport died / `cmdAlert` / a
    /// stream rejected): streams fail and the session is never reused.
    broken: AtomicBool,
    /// Wakers of streams whose `poll_write` found the bounded write channel full;
    /// the writer wakes them after it consumes a write (freeing capacity).
    write_wakers: Mutex<Vec<Waker>>,
    /// Notified when the writer exits (all handles dropped) so the reader stops
    /// and the read half is closed, fully tearing down the connection.
    closing: Notify,
}

impl SessionShared {
    fn wake_writers(&self) {
        for waker in self.write_wakers.lock().expect("anytls write wakers").drain(..) {
            waker.wake();
        }
    }

    fn mark_broken(&self) {
        self.broken.store(true, Ordering::Release);
    }

    fn is_broken(&self) -> bool {
        self.broken.load(Ordering::Acquire)
    }

    /// Mark broken and drop every stream sender so all readers see EOF.
    fn tear_down_streams(&self) {
        self.mark_broken();
        self.streams.lock().expect("anytls streams").clear();
        self.wake_writers();
    }
}

/// A handle to a live multiplexed AnyTLS session: the channels into its driver,
/// shared liveness state, the count of open streams (for capacity + idle
/// detection) and when it last went idle. Held by every stream on the session
/// and (for reuse) by the per-server registry; the driver shuts down and the
/// connection closes once the last handle is dropped.
struct SessionHandle {
    ctrl: mpsc::UnboundedSender<Ctrl>,
    writes: mpsc::Sender<WriteMsg>,
    shared: Arc<SessionShared>,
    /// Currently-open streams, capped by [`MAX_STREAMS_PER_SESSION`].
    active: AtomicU32,
    /// When `active` last reached zero, for idle-TTL eviction; `None` while busy.
    idle_since: Mutex<Option<Instant>>,
}

impl SessionHandle {
    /// Whether this session is still usable for a new stream: not broken and, if
    /// idle, still within the idle TTL.
    fn alive(&self) -> bool {
        if self.shared.is_broken() {
            return false;
        }
        if self.active.load(Ordering::Acquire) == 0
            && let Some(since) = *self.idle_since.lock().expect("anytls idle_since")
        {
            return since.elapsed() <= SESSION_IDLE_TTL;
        }
        true
    }

    /// Try to reserve a stream slot (`active < MAX` and not broken). Clears the
    /// idle marker on the idle→busy transition.
    fn reserve_slot(&self) -> bool {
        let mut cur = self.active.load(Ordering::Acquire);
        loop {
            if cur >= MAX_STREAMS_PER_SESSION || self.shared.is_broken() {
                return false;
            }
            match self
                .active
                .compare_exchange_weak(cur, cur + 1, Ordering::AcqRel, Ordering::Acquire)
            {
                Ok(_) => {
                    *self.idle_since.lock().expect("anytls idle_since") = None;
                    return true;
                }
                Err(actual) => cur = actual,
            }
        }
    }

    /// Release a stream slot; record the idle instant when the last one closes.
    fn release_slot(&self) {
        if self.active.fetch_sub(1, Ordering::AcqRel) == 1 {
            *self.idle_since.lock().expect("anytls idle_since") = Some(Instant::now());
        }
    }
}

/// Per-server registry of live multiplexed sessions: a new connection first
/// tries to open another stream on an existing session (concurrent
/// multiplexing / idle reuse) before a fresh TLS handshake + auth. Broken and
/// idle-expired sessions are evicted on access. Process-wide, like
/// [`SCHEME_STORE`], with the same lazily-initialised `Mutex<Option<HashMap>>`.
static SESSION_REGISTRY: Mutex<Option<HashMap<ServerKey, Vec<Arc<SessionHandle>>>>> = Mutex::new(None);

/// Find a live registered session for `key` with a free stream slot and reserve
/// it, evicting broken/idle-expired entries. `None` means a new session is
/// needed.
fn take_reusable(key: &ServerKey) -> Option<Arc<SessionHandle>> {
    let mut guard = SESSION_REGISTRY.lock().expect("anytls session registry");
    let map = guard.as_mut()?;
    let list = map.get_mut(key)?;
    list.retain(|handle| handle.alive());
    let mut chosen = None;
    for handle in list.iter() {
        if handle.reserve_slot() {
            chosen = Some(handle.clone());
            break;
        }
    }
    if list.is_empty() {
        map.remove(key);
    }
    chosen
}

/// Register a freshly-created session for reuse, bounded by [`SESSION_POOL_MAX`].
fn register_session(key: ServerKey, handle: Arc<SessionHandle>) {
    let mut guard = SESSION_REGISTRY.lock().expect("anytls session registry");
    let map = guard.get_or_insert_with(HashMap::new);
    let list = map.entry(key).or_default();
    list.retain(|handle| handle.alive());
    if list.len() < SESSION_POOL_MAX {
        list.push(handle);
    }
}

/// Spawn the two background tasks driving a new session over `inner` (auth
/// header already sent) and return its handle with one stream slot pre-reserved
/// for the opener. The transport is split so a writer task and a reader task run
/// independently (a blocked transport write never stalls reads, and vice versa):
/// the writer owns the write half, the padding shaper and the stream-id counter,
/// applying control/data commands as shaped records; the reader owns the read
/// half and demultiplexes inbound `cmdPSH`/`cmdFIN` to each stream's channel.
fn spawn_session(inner: BoxedStream, scheme: PaddingScheme, key: ServerKey) -> Arc<SessionHandle> {
    let (ctrl_tx, ctrl_rx) = mpsc::unbounded_channel();
    let (write_tx, write_rx) = mpsc::channel(SESSION_WRITE_CAP);
    let (heart_tx, heart_rx) = mpsc::unbounded_channel();
    let shared = Arc::new(SessionShared {
        streams: Mutex::new(HashMap::new()),
        broken: AtomicBool::new(false),
        write_wakers: Mutex::new(Vec::new()),
        closing: Notify::new(),
    });
    let handle = Arc::new(SessionHandle {
        ctrl: ctrl_tx,
        writes: write_tx,
        shared: shared.clone(),
        active: AtomicU32::new(1),
        idle_since: Mutex::new(None),
    });
    let (rd, wr) = tokio::io::split(inner);
    let writer = SessionWriter {
        wr,
        shaper: PaddingShaper::new(scheme),
        out: VecDeque::new(),
        next_id: STREAM_ID,
        settings_sent: false,
        ctrl_rx,
        write_rx,
        heart_rx,
        shared: shared.clone(),
    };
    let reader = SessionReader {
        rd,
        read_raw: Vec::new(),
        server_key: key,
        heart_tx,
        shared,
    };
    tokio::spawn(writer.run());
    tokio::spawn(reader.run());
    handle
}

/// Open a new logical stream to `target` on `handle` (new or reused session).
async fn open_on(handle: &Arc<SessionHandle>, target: &TargetAddr) -> Result<MuxStream> {
    if handle.shared.is_broken() {
        anyhow::bail!("anytls: session broken");
    }
    let (data_tx, data_rx) = mpsc::channel(STREAM_RECV_CAP);
    let (reply_tx, reply_rx) = oneshot::channel();
    handle
        .ctrl
        .send(Ctrl::Open {
            target: target.clone(),
            data: data_tx,
            reply: reply_tx,
        })
        .map_err(|_| anyhow!("anytls: session closed"))?;
    let sid = reply_rx.await.map_err(|_| anyhow!("anytls: session closed"))??;
    Ok(MuxStream {
        sid,
        data_rx,
        writes: handle.writes.clone(),
        handle: handle.clone(),
        shared: handle.shared.clone(),
        leftover: Vec::new(),
        leftover_pos: 0,
        eof: false,
        fin_sent: false,
    })
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

/// Acquire an AnyTLS stream to `target`: open another stream on a live
/// registered session for the config's server if one has a free slot (concurrent
/// multiplexing or idle reuse), otherwise establish a new TLS session (handshake
/// then auth, after which the driver writes `cmdSettings` on its first stream).
/// Either way the returned stream has its `cmdSYN` + `cmdPSH`(target) queued and
/// is ready to relay.
async fn acquire_stream(config: &AnyTlsOutboundConfig, target: &TargetAddr) -> Result<MuxStream> {
    let key = ServerKey {
        server: config.server.clone(),
        port: config.port,
    };

    // Reuse path: open another stream on an existing session (its driver assigns
    // the next id and writes `cmdSYN` + `cmdPSH`). On failure the session just
    // broke; release the reserved slot and fall through to a fresh connection.
    if let Some(handle) = take_reusable(&key) {
        match open_on(&handle, target).await {
            Ok(stream) => return Ok(stream),
            Err(_) => handle.release_slot(),
        }
    }

    // New-session path: TLS handshake + auth (packet 0). The spawned driver then
    // writes the padded packet-1 flush of `cmdSettings` + `cmdSYN` +
    // `cmdPSH`(target) when this first stream is opened, as anytls-go does after
    // `OpenStream` clears buffering.
    let scheme = current_scheme(&key);
    let mut transport = transport::establish(&config.server, config.port, &config.security, &config.transport).await?;
    transport
        .write_all(&build_auth_header(&config.password_sha256, &scheme))
        .await
        .context("anytls: send auth header")?;
    let handle = spawn_session(transport, (*scheme).clone(), key.clone());
    register_session(key, handle.clone());
    open_on(&handle, target).await.context("anytls: open first stream")
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

/// Build the packet-1 session bytes for the session's first stream:
/// `cmdSettings` (advertising the scheme md5), `cmdSYN` opening stream `sid`, and
/// the `cmdPSH` carrying the SOCKS5-encoded proxy target. The caller feeds the
/// whole blob through the padding shaper as a single `writeConn` unit.
fn build_session_init(scheme: &PaddingScheme, sid: u32, target: &TargetAddr) -> Vec<u8> {
    let mut buf = Vec::with_capacity(64 + FRAME_HEADER_LEN * 2 + 64);
    let settings = format!(
        "v={PROTOCOL_VERSION}\nclient={CLIENT_NAME}\npadding-md5={}",
        scheme.md5_hex
    );
    push_frame(&mut buf, CMD_SETTINGS, 0, settings.as_bytes());
    push_frame(&mut buf, CMD_SYN, sid, &[]);
    let mut addr = Vec::with_capacity(1 + 256 + 2);
    socks5::encode_address(&mut addr, target);
    push_frame(&mut buf, CMD_PSH, sid, &addr);
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

/// The writer task of one AnyTLS session: it owns the transport's write half,
/// the per-connection padding shaper and the stream-id counter. It serialises
/// every stream's writes, control frames (`cmdSYN`/`cmdPSH`/`cmdFIN`) and the
/// reader's heartbeat responses into shaped records on the transport. It runs
/// until the transport write dies or the last [`SessionHandle`] is dropped
/// (closing both command channels), then shuts the write half down, tears the
/// streams down (readers see EOF) and signals the reader to stop.
struct SessionWriter {
    wr: WriteHalf<BoxedStream>,
    /// Padding-scheme state shaping the outgoing record stream (per session, so
    /// its `writeConn` counter spans all of this connection's streams).
    shaper: PaddingShaper,
    /// Shaped records pending write to the transport (each becomes a TLS record).
    out: VecDeque<Vec<u8>>,
    /// Next stream id to assign (monotonic across this session's streams).
    next_id: u32,
    /// Whether `cmdSettings` has been written (once, on the first stream).
    settings_sent: bool,
    ctrl_rx: mpsc::UnboundedReceiver<Ctrl>,
    write_rx: mpsc::Receiver<WriteMsg>,
    /// Heartbeat-response requests forwarded by the reader (carries the sid).
    heart_rx: mpsc::UnboundedReceiver<u32>,
    shared: Arc<SessionShared>,
}

impl SessionWriter {
    /// The select loop: flush pending records, then service the next of {control
    /// message, heartbeat-response request, application write}. Exits on a
    /// transport write error or once both command channels are closed.
    async fn run(mut self) {
        let mut ctrl_open = true;
        let mut write_open = true;
        let mut heart_open = true;
        loop {
            if self.flush_out().await.is_err() {
                break;
            }
            if !ctrl_open && !write_open {
                break;
            }
            tokio::select! {
                biased;
                ctrl = self.ctrl_rx.recv(), if ctrl_open => match ctrl {
                    Some(c) => self.handle_ctrl(c),
                    None => ctrl_open = false,
                },
                sid = self.heart_rx.recv(), if heart_open => match sid {
                    Some(sid) => {
                        let mut frame = Vec::with_capacity(FRAME_HEADER_LEN);
                        push_frame(&mut frame, CMD_HEART_RESPONSE, sid, &[]);
                        self.shaper.shape(&mut self.out, frame);
                    }
                    None => heart_open = false,
                },
                msg = self.write_rx.recv(), if write_open => {
                    self.shared.wake_writers();
                    match msg {
                        Some(m) => self.handle_write(m),
                        None => write_open = false,
                    }
                }
            }
        }
        let _ = self.wr.shutdown().await;
        self.shared.tear_down_streams();
        self.shared.closing.notify_waiters();
    }

    /// Write all queued shaped records to the transport, one `write_all` each so
    /// each becomes its own TLS record, then flush. Marks the session broken on
    /// error.
    async fn flush_out(&mut self) -> io::Result<()> {
        if self.out.is_empty() {
            return Ok(());
        }
        while let Some(record) = self.out.pop_front() {
            if let Err(e) = self.wr.write_all(&record).await {
                self.shared.mark_broken();
                return Err(e);
            }
        }
        self.wr.flush().await.inspect_err(|_| self.shared.mark_broken())
    }

    /// Apply a control message: open a stream (assign id, write `cmdSYN`+`cmdPSH`,
    /// register its channel), send a stream's `cmdFIN`, or drop a closed stream.
    fn handle_ctrl(&mut self, ctrl: Ctrl) {
        match ctrl {
            Ctrl::Open { target, data, reply } => {
                let sid = self.next_id;
                self.next_id = self.next_id.wrapping_add(1);
                self.shared.streams.lock().expect("anytls streams").insert(sid, data);
                let unit = if self.settings_sent {
                    build_stream_open(sid, &target)
                } else {
                    self.settings_sent = true;
                    build_session_init(&self.shaper.scheme, sid, &target)
                };
                self.shaper.shape(&mut self.out, unit);
                let _ = reply.send(Ok(sid));
            }
            Ctrl::Fin { sid } => {
                if self.shared.streams.lock().expect("anytls streams").contains_key(&sid) {
                    let mut frame = Vec::with_capacity(FRAME_HEADER_LEN);
                    push_frame(&mut frame, CMD_FIN, sid, &[]);
                    self.shaper.shape(&mut self.out, frame);
                }
            }
            Ctrl::Close { sid } => {
                self.shared.streams.lock().expect("anytls streams").remove(&sid);
            }
        }
    }

    /// Frame and shape one application write as `cmdPSH`(s) for its stream,
    /// dropping it if the stream has since closed.
    fn handle_write(&mut self, msg: WriteMsg) {
        if !self
            .shared
            .streams
            .lock()
            .expect("anytls streams")
            .contains_key(&msg.sid)
        {
            return;
        }
        let mut pos = 0;
        while pos < msg.data.len() {
            let take = (msg.data.len() - pos).min(MAX_PSH_CHUNK);
            let mut frame = Vec::with_capacity(FRAME_HEADER_LEN + take);
            push_frame(&mut frame, CMD_PSH, msg.sid, &msg.data[pos..pos + take]);
            self.shaper.shape(&mut self.out, frame);
            pos += take;
        }
    }
}

/// The reader task of one AnyTLS session: it owns the transport's read half and
/// demultiplexes inbound frames, routing each `cmdPSH` payload to its stream's
/// bounded channel (awaiting capacity, head-of-line, without blocking the writer
/// task) and dropping a stream's sender on `cmdFIN`/reject so its reader sees
/// EOF. It exits on transport EOF/error or when the writer signals closing, then
/// tears down all streams.
struct SessionReader {
    rd: ReadHalf<BoxedStream>,
    /// Raw bytes read from the transport not yet parsed into frames.
    read_raw: Vec<u8>,
    /// Server endpoint, to route a received `cmdUpdatePaddingScheme`.
    server_key: ServerKey,
    /// Forwards heartbeat-response requests to the writer (only it may write).
    heart_tx: mpsc::UnboundedSender<u32>,
    shared: Arc<SessionShared>,
}

impl SessionReader {
    async fn run(mut self) {
        let mut buf = vec![0u8; 16 * 1024];
        loop {
            if !self.parse().await {
                break;
            }
            tokio::select! {
                biased;
                _ = self.shared.closing.notified() => break,
                res = self.rd.read(&mut buf) => match res {
                    Ok(0) | Err(_) => break,
                    Ok(n) => self.read_raw.extend_from_slice(&buf[..n]),
                },
            }
        }
        self.shared.tear_down_streams();
    }

    /// Look up a stream's sender (cloned, without holding the lock across the
    /// await) and deliver `data`, awaiting capacity. Drops the stream if its
    /// receiver is gone. Takes `shared` (not `&self`) so the future stays `Send`.
    async fn deliver(shared: &SessionShared, sid: u32, data: Vec<u8>) {
        let tx = shared.streams.lock().expect("anytls streams").get(&sid).cloned();
        let Some(tx) = tx else {
            return; // stream gone: drop the payload
        };
        if tx.reserve().await.map(|p| p.send(data)).is_err() {
            shared.streams.lock().expect("anytls streams").remove(&sid);
        }
    }

    /// Parse complete frames from `read_raw`, routing payloads to streams. Awaits
    /// channel capacity per `cmdPSH` (head-of-line). Returns `false` if the
    /// session must shut down (`cmdAlert`).
    async fn parse(&mut self) -> bool {
        loop {
            if self.read_raw.len() < FRAME_HEADER_LEN {
                return true;
            }
            let len = u16::from_be_bytes([self.read_raw[5], self.read_raw[6]]) as usize;
            let need = FRAME_HEADER_LEN + len;
            if self.read_raw.len() < need {
                return true;
            }
            let cmd = self.read_raw[0];
            let sid = u32::from_be_bytes([self.read_raw[1], self.read_raw[2], self.read_raw[3], self.read_raw[4]]);
            let data: Vec<u8> = self.read_raw[FRAME_HEADER_LEN..need].to_vec();
            self.read_raw.drain(..need);

            match cmd {
                CMD_PSH => Self::deliver(&self.shared, sid, data).await,
                // The server closed this stream (reader sees EOF) — the session
                // stays up for its other streams.
                CMD_FIN => {
                    self.shared.streams.lock().expect("anytls streams").remove(&sid);
                }
                // A non-empty `cmdSYNACK` rejects the stream; its reader sees EOF.
                CMD_SYNACK if !data.is_empty() => {
                    self.shared.streams.lock().expect("anytls streams").remove(&sid);
                }
                // The connection is unusable: stop and mark broken.
                CMD_ALERT => {
                    self.shared.mark_broken();
                    return false;
                }
                CMD_HEART_REQUEST => {
                    let _ = self.heart_tx.send(sid);
                }
                // Store a server-pushed scheme for this server's future sessions.
                CMD_UPDATE_PADDING_SCHEME => apply_scheme_update(&self.server_key, &data),
                // Padding, server settings, heart responses and our own
                // SYN/SYNACK(ok) carry nothing to deliver.
                CMD_WASTE | CMD_SETTINGS | CMD_SERVER_SETTINGS | CMD_HEART_RESPONSE | CMD_SYN => {}
                _ => {}
            }
        }
    }
}

/// A logical stream multiplexed on an AnyTLS session: a lightweight handle over
/// the shared session tasks. Reads pull this stream's demultiplexed `cmdPSH`
/// payloads from a bounded channel filled by the session reader (which handles
/// all control frames); writes hand `cmdPSH` units to the session writer over a
/// bounded channel (backpressured); shutdown/drop queue this stream's `cmdFIN`
/// and free its session slot, leaving the connection up for its other streams.
struct MuxStream {
    /// This stream's id (the id its frames carry); fixed for the stream's life.
    sid: u32,
    /// Demultiplexed inbound `cmdPSH` payloads from the driver; closed (EOF) when
    /// the server FINs this stream or the session ends.
    data_rx: mpsc::Receiver<Vec<u8>>,
    /// The session's bounded write channel (shared by all its streams).
    writes: mpsc::Sender<WriteMsg>,
    /// Keeps the session alive while this stream lives, and carries the control
    /// channel and slot accounting used on close.
    handle: Arc<SessionHandle>,
    /// Shared liveness + writer-wakeup state.
    shared: Arc<SessionShared>,
    /// Inbound payload being handed to the reader (front consumed first).
    leftover: Vec<u8>,
    leftover_pos: usize,
    /// Reads are exhausted (server FIN or session ended).
    eof: bool,
    /// We have queued this stream's `cmdFIN`.
    fin_sent: bool,
}

impl Drop for MuxStream {
    fn drop(&mut self) {
        if !self.fin_sent {
            let _ = self.handle.ctrl.send(Ctrl::Fin { sid: self.sid });
            self.fin_sent = true;
        }
        let _ = self.handle.ctrl.send(Ctrl::Close { sid: self.sid });
        self.handle.release_slot();
    }
}

impl AsyncRead for MuxStream {
    fn poll_read(self: Pin<&mut Self>, cx: &mut TaskContext<'_>, buf: &mut ReadBuf<'_>) -> Poll<io::Result<()>> {
        let this = self.get_mut();
        loop {
            if this.leftover_pos < this.leftover.len() {
                let n = buf.remaining().min(this.leftover.len() - this.leftover_pos);
                buf.put_slice(&this.leftover[this.leftover_pos..this.leftover_pos + n]);
                this.leftover_pos += n;
                return Poll::Ready(Ok(()));
            }
            if this.eof {
                return Poll::Ready(Ok(()));
            }
            match this.data_rx.poll_recv(cx) {
                // Skip empty payloads rather than spin on a zero-length read.
                Poll::Ready(Some(data)) => {
                    this.leftover = data;
                    this.leftover_pos = 0;
                }
                Poll::Ready(None) => {
                    this.eof = true;
                    return Poll::Ready(Ok(()));
                }
                Poll::Pending => return Poll::Pending,
            }
        }
    }
}

impl AsyncWrite for MuxStream {
    fn poll_write(self: Pin<&mut Self>, cx: &mut TaskContext<'_>, buf: &[u8]) -> Poll<io::Result<usize>> {
        let this = self.get_mut();
        if this.shared.is_broken() {
            return Poll::Ready(Err(io::ErrorKind::BrokenPipe.into()));
        }
        if buf.is_empty() {
            return Poll::Ready(Ok(0));
        }
        let take = buf.len().min(MAX_PSH_CHUNK);
        match this.writes.try_send(WriteMsg {
            sid: this.sid,
            data: buf[..take].to_vec(),
        }) {
            Ok(()) => Poll::Ready(Ok(take)),
            // The session writer wakes parked writers after it consumes a write.
            Err(mpsc::error::TrySendError::Full(_)) => {
                this.shared
                    .write_wakers
                    .lock()
                    .expect("anytls write wakers")
                    .push(cx.waker().clone());
                Poll::Pending
            }
            Err(mpsc::error::TrySendError::Closed(_)) => Poll::Ready(Err(io::ErrorKind::BrokenPipe.into())),
        }
    }

    fn poll_flush(self: Pin<&mut Self>, _cx: &mut TaskContext<'_>) -> Poll<io::Result<()>> {
        // Accepted writes are owned by the session writer, which flushes the
        // transport each loop iteration; the stream itself has nothing to flush.
        Poll::Ready(Ok(()))
    }

    fn poll_shutdown(self: Pin<&mut Self>, _cx: &mut TaskContext<'_>) -> Poll<io::Result<()>> {
        let this = self.get_mut();
        if !this.fin_sent {
            this.fin_sent = true;
            // `cmdFIN` half-closes only this stream; the session stays up. Drop
            // frees the slot and stops inbound routing.
            let _ = this.handle.ctrl.send(Ctrl::Fin { sid: this.sid });
        }
        Poll::Ready(Ok(()))
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;
    use std::net::{Ipv4Addr, SocketAddr};
    use std::sync::atomic::{AtomicUsize, Ordering};

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
        let init = build_session_init(&scheme, STREAM_ID, &target);

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

    /// A minimal anytls server multiplexing several streams on one connection: it
    /// records each `cmdSYN`'s stream id, acks it, treats each stream's first
    /// `cmdPSH` as the (unechoed) target address and echoes the rest back on the
    /// same id, and answers `cmdFIN` with `cmdFIN` (closing only that stream). If
    /// `alert_after_first_stream` is set it sends a `cmdAlert` on the first
    /// stream's FIN, marking the session broken so it must not be reused.
    async fn pool_test_serve(mut stream: TcpStream, sids: Arc<Mutex<Vec<u32>>>, alert_after_first_stream: bool) {
        // Note: callers set `alert_after_first_stream` only for the connection
        // expected to be pooled, so the replacement connection stays healthy.
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

        // Per-stream: whether the next `cmdPSH` is the (unechoed) target address.
        let mut awaiting_addr: HashSet<u32> = HashSet::new();
        let mut streams_done = 0u32;
        while let Some((cmd, sid, data)) = read_frame_opt(&mut stream).await {
            match cmd {
                CMD_WASTE => {}
                CMD_SETTINGS => server_write(&mut stream, CMD_SERVER_SETTINGS, 0, b"v=2").await,
                CMD_SYN => {
                    sids.lock().unwrap().push(sid);
                    server_write(&mut stream, CMD_SYNACK, sid, &[]).await;
                    awaiting_addr.insert(sid);
                }
                // A stream's first `cmdPSH` is its (unechoed) target address; the
                // rest are echoed back on the same id.
                CMD_PSH if !awaiting_addr.remove(&sid) => {
                    server_write(&mut stream, CMD_PSH, sid, &data).await;
                }
                CMD_FIN => {
                    streams_done += 1;
                    if alert_after_first_stream && streams_done == 1 {
                        // Mark the session broken (before its FIN) so the client's
                        // driver tears it down deterministically and never reuses
                        // it, then drop the connection.
                        server_write(&mut stream, CMD_ALERT, sid, b"reaped").await;
                        server_write(&mut stream, CMD_FIN, sid, &[]).await;
                        return;
                    }
                    server_write(&mut stream, CMD_FIN, sid, &[]).await;
                }
                _ => {}
            }
        }
    }

    /// Spawn `pool_test_serve` accepting on a fresh port; returns the address, the
    /// recorded stream ids, and the count of accepted TCP connections.
    async fn spawn_pool_server(alert_first_connection: bool) -> (SocketAddr, Arc<Mutex<Vec<u32>>>, Arc<AtomicUsize>) {
        let sids = Arc::new(Mutex::new(Vec::<u32>::new()));
        let conns = Arc::new(AtomicUsize::new(0));
        let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).await.unwrap();
        let addr = listener.local_addr().unwrap();
        let (sids_task, conns_task) = (sids.clone(), conns.clone());
        tokio::spawn(async move {
            while let Ok((stream, _)) = listener.accept().await {
                let index = conns_task.fetch_add(1, Ordering::SeqCst);
                // Only the first connection (the one that gets pooled) is reaped.
                let alert = alert_first_connection && index == 0;
                tokio::spawn(pool_test_serve(stream, sids_task.clone(), alert));
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

    /// Number of live (reusable) registered sessions for `key`.
    fn pool_len(key: &ServerKey) -> usize {
        SESSION_REGISTRY
            .lock()
            .unwrap()
            .as_ref()
            .and_then(|map| map.get(key))
            .map_or(0, |list| list.iter().filter(|h| h.alive()).count())
    }

    /// Drive a relay-style round trip on `stream`, then close it cleanly (send
    /// our `cmdFIN`, read the server's `cmdFIN` to EOF) so the session is left
    /// idle (and reusable) once the stream is dropped.
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

        // Stream 1 over a fresh session; a clean close leaves it idle in the pool.
        {
            let mut s1 = connect(&config, &target).await.unwrap();
            round_trip_and_close(&mut s1, b"first").await;
        }
        assert_eq!(pool_len(&key), 1, "clean close leaves the session reusable");

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
    async fn multiplexes_concurrent_streams_on_one_connection() {
        let (addr, sids, conns) = spawn_pool_server(false).await;
        let config = pool_test_config(addr);
        let t1 = TargetAddr::Domain("one.example".to_string(), 443);
        let t2 = TargetAddr::Domain("two.example".to_string(), 443);

        // Two overlapping streams: the second opens on the first's live session.
        let mut s1 = connect(&config, &t1).await.unwrap();
        let mut s2 = connect(&config, &t2).await.unwrap();

        // Interleave writes; demux must route each echo back to its own stream.
        s1.write_all(b"aaa").await.unwrap();
        s1.flush().await.unwrap();
        s2.write_all(b"bbb").await.unwrap();
        s2.flush().await.unwrap();
        let (mut r1, mut r2) = ([0u8; 3], [0u8; 3]);
        s1.read_exact(&mut r1).await.unwrap();
        s2.read_exact(&mut r2).await.unwrap();
        assert_eq!(&r1, b"aaa", "stream 1 received its own payload");
        assert_eq!(&r2, b"bbb", "stream 2 received its own payload");

        assert_eq!(
            conns.load(Ordering::SeqCst),
            1,
            "both concurrent streams shared one TCP connection"
        );
        let mut ids = sids.lock().unwrap().clone();
        ids.sort_unstable();
        assert_eq!(ids, vec![1, 2], "concurrent streams got distinct incrementing ids");

        round_trip_and_close(&mut s1, b"aaa").await;
        round_trip_and_close(&mut s2, b"bbb").await;
    }

    #[tokio::test]
    async fn pool_discards_dead_session_on_reuse() {
        // The server sends a `cmdAlert` on the first stream's FIN, so the pooled
        // session is marked broken and must not be reused.
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
        // The `cmdAlert` tore the session down, so it is not reusable.
        assert_eq!(pool_len(&key), 0, "broken session is not pooled for reuse");

        // Reuse must skip the dead session and dial a new connection.
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
