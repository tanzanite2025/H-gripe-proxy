use std::io;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context as TaskContext, Poll};

use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};
use tokio::sync::mpsc;

use super::frame::MAX_PSH_CHUNK;
use super::session::{Ctrl, SessionHandle, SessionShared, WriteMsg};

/// A logical stream multiplexed on an AnyTLS session: a lightweight handle over
/// the shared session tasks. Reads pull this stream's demultiplexed `cmdPSH`
/// payloads from a bounded channel filled by the session reader (which handles
/// all control frames); writes hand `cmdPSH` units to the session writer over a
/// bounded channel (backpressured); shutdown/drop queue this stream's `cmdFIN`
/// and free its session slot, leaving the connection up for its other streams.
pub(super) struct MuxStream {
    /// This stream's id (the id its frames carry); fixed for the stream's life.
    pub(super) sid: u32,
    /// Demultiplexed inbound `cmdPSH` payloads from the driver; closed (EOF) when
    /// the server FINs this stream or the session ends.
    pub(super) data_rx: mpsc::Receiver<Vec<u8>>,
    /// The session's bounded write channel (shared by all its streams).
    pub(super) writes: mpsc::Sender<WriteMsg>,
    /// Keeps the session alive while this stream lives, and carries the control
    /// channel and slot accounting used on close.
    pub(super) handle: Arc<SessionHandle>,
    /// Shared liveness + writer-wakeup state.
    pub(super) shared: Arc<SessionShared>,
    /// Inbound payload being handed to the reader (front consumed first).
    pub(super) leftover: Vec<u8>,
    pub(super) leftover_pos: usize,
    /// Reads are exhausted (server FIN or session ended).
    pub(super) eof: bool,
    /// We have queued this stream's `cmdFIN`.
    pub(super) fin_sent: bool,
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
