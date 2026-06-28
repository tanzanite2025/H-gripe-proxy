//! v2ray-plugin `mux` framing (a mux.cool-compatible client).
//!
//! When a Shadowsocks node sets `mux: true`, the v2ray-plugin server expects the
//! stream wrapped in v2ray's "mux.cool" framing rather than raw bytes, so we
//! must speak it to interoperate. Like mihomo's client (`transport/v2ray-plugin`
//! `mux.go`, which it explicitly calls "not a complete implementation"), we run
//! a single logical sub-connection over the one transport stream — enough to
//! satisfy a mux-enabled server without the full N-way multiplexer (the SS layer
//! already opens one connection per relay).
//!
//! Wire format (all integers big-endian):
//!
//! ```text
//! frame := metalen:u16  metadata[metalen]  [ datalen:u16  data[datalen] ]
//! metadata := id:u16  status:u8  option:u8  [status==New: net/port/addr ...]
//! ```
//!
//! - **New** (`status=0x01`), sent once prefixed to the first write, declares a
//!   TCP sub-connection to a dummy `127.0.0.1:0` (the real target lives inside
//!   the Shadowsocks layer above, so the address is irrelevant).
//! - **Keep + Data** (`status=0x02`, `option=0x01`) carries each application
//!   write: `00 04  id  02 01  datalen  data`.
//! - **End** (`status=0x03`) is sent on shutdown.
//! - Inbound **KeepAlive** (`status=0x04`) and any non-Data frame carry no
//!   payload and are skipped.

use std::io;
use std::pin::Pin;
use std::task::{Context as TaskContext, Poll, ready};

use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};

use crate::outbound::BoxedStream;

const STATUS_NEW: u8 = 0x01;
const STATUS_KEEP: u8 = 0x02;
const STATUS_END: u8 = 0x03;
const STATUS_KEEPALIVE: u8 = 0x04;

const OPTION_NONE: u8 = 0x00;
const OPTION_DATA: u8 = 0x01;

/// Fixed sub-connection id (`0x0000`); a single logical stream is all we run.
const ID: [u8; 2] = [0x00, 0x00];

/// Upper bound on the metadata length, matching mihomo's guard against a peer
/// announcing an absurd header.
const MAX_META: usize = 512;

/// Largest payload we frame in a single Data frame (`datalen` is a `u16`); a
/// larger write is split across frames over successive `poll_write` calls.
const MAX_FRAME: usize = u16::MAX as usize;

/// What the read state machine is currently collecting.
enum RxPhase {
    /// The 6-byte frame prefix: `metalen:u16 id:u16 status:u8 option:u8`.
    Prefix,
    /// The 2-byte `datalen` that follows a `Keep + Data` metadata header.
    DataLen,
}

/// A mux.cool client wrapper over a single v2ray-plugin transport stream.
pub struct V2rayMux {
    inner: BoxedStream,
    /// Encoded bytes queued for the transport (frame headers + payload).
    write_buf: Vec<u8>,
    write_pos: usize,
    /// Whether the one-shot `New` frame has been queued ahead of the first
    /// payload.
    new_frame_sent: bool,
    /// Whether the `End` frame has been queued during shutdown.
    end_queued: bool,
    /// Accumulator for the current read step's fixed-size header.
    acc: Vec<u8>,
    phase: RxPhase,
    /// Remaining payload bytes of the current Data frame still to hand upward.
    remain: usize,
    /// Set once the transport reported a clean EOF on a frame boundary.
    eof: bool,
}

impl V2rayMux {
    /// Wrap `inner` (a websocket / http-upgrade transport stream) in mux.cool
    /// framing.
    pub fn new(inner: BoxedStream) -> Self {
        Self {
            inner,
            write_buf: Vec::new(),
            write_pos: 0,
            new_frame_sent: false,
            end_queued: false,
            acc: Vec::with_capacity(6),
            phase: RxPhase::Prefix,
            remain: 0,
            eof: false,
        }
    }

    /// The one-shot `New` frame: open a TCP sub-connection to `127.0.0.1:0`.
    fn new_frame() -> [u8; 14] {
        [
            0x00,
            0x0C, // metalen = 12
            ID[0],
            ID[1], // sub-connection id
            STATUS_NEW,
            OPTION_NONE, //
            0x01,        // network type: TCP
            0x00,
            0x00, // port = 0
            0x01, // address type: IPv4
            127,
            0,
            0,
            1, // 127.0.0.1
        ]
    }

    /// Append a `Keep + Data` frame carrying `data` (prefixed by the `New` frame
    /// on the first call).
    fn encode_data(&mut self, data: &[u8]) {
        if !self.new_frame_sent {
            self.write_buf.extend_from_slice(&Self::new_frame());
            self.new_frame_sent = true;
        }
        let len = data.len() as u16;
        self.write_buf
            .extend_from_slice(&[0x00, 0x04, ID[0], ID[1], STATUS_KEEP, OPTION_DATA]);
        self.write_buf.extend_from_slice(&len.to_be_bytes());
        self.write_buf.extend_from_slice(data);
    }

    /// Flush `write_buf` to the transport, returning `Ready(Ok(()))` only once it
    /// has fully drained.
    fn poll_drain(&mut self, cx: &mut TaskContext<'_>) -> Poll<io::Result<()>> {
        while self.write_pos < self.write_buf.len() {
            let n = ready!(Pin::new(&mut self.inner).poll_write(cx, &self.write_buf[self.write_pos..]))?;
            if n == 0 {
                return Poll::Ready(Err(io::Error::new(
                    io::ErrorKind::WriteZero,
                    "v2ray-plugin mux: transport accepted no bytes",
                )));
            }
            self.write_pos += n;
        }
        self.write_buf.clear();
        self.write_pos = 0;
        Poll::Ready(Ok(()))
    }

    /// Read from the transport until `acc` holds `need` bytes. `Ok(false)` means
    /// a clean EOF arrived on a frame boundary (`acc` empty); a partial header
    /// followed by EOF is a truncation error.
    fn poll_fill(&mut self, cx: &mut TaskContext<'_>, need: usize) -> Poll<io::Result<bool>> {
        while self.acc.len() < need {
            let mut tmp = [0u8; MAX_META];
            let want = (need - self.acc.len()).min(tmp.len());
            let mut rb = ReadBuf::new(&mut tmp[..want]);
            ready!(Pin::new(&mut self.inner).poll_read(cx, &mut rb))?;
            let filled = rb.filled();
            if filled.is_empty() {
                if self.acc.is_empty() {
                    return Poll::Ready(Ok(false));
                }
                return Poll::Ready(Err(io::Error::new(
                    io::ErrorKind::UnexpectedEof,
                    "v2ray-plugin mux: truncated frame",
                )));
            }
            self.acc.extend_from_slice(filled);
        }
        Poll::Ready(Ok(true))
    }
}

impl AsyncRead for V2rayMux {
    fn poll_read(self: Pin<&mut Self>, cx: &mut TaskContext<'_>, buf: &mut ReadBuf<'_>) -> Poll<io::Result<()>> {
        let this = self.get_mut();
        loop {
            if this.eof {
                return Poll::Ready(Ok(()));
            }

            // Pass through the body of the Data frame currently being decoded.
            if this.remain > 0 {
                let cap = buf.remaining().min(this.remain);
                if cap == 0 {
                    return Poll::Ready(Ok(()));
                }
                let mut tmp = [0u8; 8192];
                let want = cap.min(tmp.len());
                let mut rb = ReadBuf::new(&mut tmp[..want]);
                ready!(Pin::new(&mut this.inner).poll_read(cx, &mut rb))?;
                let filled = rb.filled();
                if filled.is_empty() {
                    return Poll::Ready(Err(io::Error::new(
                        io::ErrorKind::UnexpectedEof,
                        "v2ray-plugin mux: truncated frame body",
                    )));
                }
                buf.put_slice(filled);
                this.remain -= filled.len();
                return Poll::Ready(Ok(()));
            }

            match this.phase {
                RxPhase::Prefix => {
                    if !ready!(this.poll_fill(cx, 6))? {
                        this.eof = true;
                        return Poll::Ready(Ok(()));
                    }
                    let metalen = u16::from_be_bytes([this.acc[0], this.acc[1]]) as usize;
                    let status = this.acc[4];
                    let option = this.acc[5];
                    this.acc.clear();
                    // We run a single sub-connection, so every inbound frame's
                    // metadata is the fixed 4 bytes (id + status + option). A
                    // longer header would desync the stream.
                    if metalen != 4 {
                        if metalen > MAX_META {
                            return Poll::Ready(Err(io::Error::new(
                                io::ErrorKind::InvalidData,
                                "v2ray-plugin mux: metadata length too large",
                            )));
                        }
                        return Poll::Ready(Err(io::Error::new(
                            io::ErrorKind::InvalidData,
                            "v2ray-plugin mux: unexpected metadata length",
                        )));
                    }
                    if status == STATUS_END {
                        this.eof = true;
                        return Poll::Ready(Ok(()));
                    }
                    if status == STATUS_KEEPALIVE || option != OPTION_DATA {
                        // No payload section; read the next frame.
                        continue;
                    }
                    this.phase = RxPhase::DataLen;
                }
                RxPhase::DataLen => {
                    if !ready!(this.poll_fill(cx, 2))? {
                        return Poll::Ready(Err(io::Error::new(
                            io::ErrorKind::UnexpectedEof,
                            "v2ray-plugin mux: truncated data length",
                        )));
                    }
                    this.remain = u16::from_be_bytes([this.acc[0], this.acc[1]]) as usize;
                    this.acc.clear();
                    this.phase = RxPhase::Prefix;
                    // Loop: a zero-length Data frame just advances to the next
                    // header, otherwise the body is handed up above.
                }
            }
        }
    }
}

impl AsyncWrite for V2rayMux {
    fn poll_write(self: Pin<&mut Self>, cx: &mut TaskContext<'_>, buf: &[u8]) -> Poll<io::Result<usize>> {
        let this = self.get_mut();
        // Finish sending any previously framed bytes before accepting more, so a
        // single Data frame is never interleaved on the wire.
        ready!(this.poll_drain(cx))?;
        if buf.is_empty() {
            return Poll::Ready(Ok(0));
        }
        let take = buf.len().min(MAX_FRAME);
        this.encode_data(&buf[..take]);
        // Best-effort flush; whatever remains is drained on the next call.
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
        if !this.end_queued {
            this.write_buf
                .extend_from_slice(&[0x00, 0x04, ID[0], ID[1], STATUS_END, OPTION_NONE]);
            this.end_queued = true;
        }
        ready!(this.poll_drain(cx))?;
        Pin::new(&mut this.inner).poll_shutdown(cx)
    }
}
