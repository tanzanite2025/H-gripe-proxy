use std::collections::{HashMap, VecDeque};
use std::io;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::{Arc, Mutex};
use std::task::Waker;
use std::time::{Duration, Instant};

use anyhow::{Result, anyhow};
use tokio::io::{AsyncReadExt, AsyncWriteExt, ReadHalf, WriteHalf};
use tokio::sync::{Notify, mpsc, oneshot};

use crate::address::TargetAddr;
use crate::outbound::BoxedStream;

use super::frame::{
    CMD_ALERT, CMD_FIN, CMD_HEART_REQUEST, CMD_HEART_RESPONSE, CMD_PSH, CMD_SERVER_SETTINGS, CMD_SETTINGS, CMD_SYN,
    CMD_SYNACK, CMD_UPDATE_PADDING_SCHEME, CMD_WASTE, FRAME_HEADER_LEN, MAX_PSH_CHUNK, STREAM_ID, build_session_init,
    build_stream_open, push_frame,
};
use super::padding::{PaddingScheme, PaddingShaper, ServerKey, apply_scheme_update};
use super::stream::MuxStream;

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
pub(super) enum Ctrl {
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
pub(super) struct WriteMsg {
    pub(super) sid: u32,
    pub(super) data: Vec<u8>,
}

/// State shared between a session's reader task, writer task and streams: the
/// inbound demux table, liveness flag, writer-backpressure wakers, and the
/// signal that lets the writer tell the reader to stop.
pub(super) struct SessionShared {
    /// Open streams: id → inbound payload sender, used by the reader to demux
    /// `cmdPSH` and registered by the writer when it opens a stream. Dropping a
    /// sender (on `cmdFIN`/close) gives that stream's reader EOF.
    streams: Mutex<HashMap<u32, mpsc::Sender<Vec<u8>>>>,
    /// Set when the connection is unusable (transport died / `cmdAlert` / a
    /// stream rejected): streams fail and the session is never reused.
    broken: AtomicBool,
    /// Wakers of streams whose `poll_write` found the bounded write channel full;
    /// the writer wakes them after it consumes a write (freeing capacity).
    pub(super) write_wakers: Mutex<Vec<Waker>>,
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

    pub(super) fn is_broken(&self) -> bool {
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
pub(super) struct SessionHandle {
    pub(super) ctrl: mpsc::UnboundedSender<Ctrl>,
    pub(super) writes: mpsc::Sender<WriteMsg>,
    pub(super) shared: Arc<SessionShared>,
    /// Currently-open streams, capped by [`MAX_STREAMS_PER_SESSION`].
    active: AtomicU32,
    /// When `active` last reached zero, for idle-TTL eviction; `None` while busy.
    idle_since: Mutex<Option<Instant>>,
}

impl SessionHandle {
    /// Whether this session is still usable for a new stream: not broken and, if
    /// idle, still within the idle TTL.
    pub(super) fn alive(&self) -> bool {
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
    pub(super) fn release_slot(&self) {
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
pub(super) static SESSION_REGISTRY: Mutex<Option<HashMap<ServerKey, Vec<Arc<SessionHandle>>>>> = Mutex::new(None);

/// Find a live registered session for `key` with a free stream slot and reserve
/// it, evicting broken/idle-expired entries. `None` means a new session is
/// needed.
pub(super) fn take_reusable(key: &ServerKey) -> Option<Arc<SessionHandle>> {
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
pub(super) fn register_session(key: ServerKey, handle: Arc<SessionHandle>) {
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
pub(super) fn spawn_session(inner: BoxedStream, scheme: PaddingScheme, key: ServerKey) -> Arc<SessionHandle> {
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
pub(super) async fn open_on(handle: &Arc<SessionHandle>, target: &TargetAddr) -> Result<MuxStream> {
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
