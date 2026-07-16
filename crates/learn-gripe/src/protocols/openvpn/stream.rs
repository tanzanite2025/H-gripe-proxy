//! Caller-facing relayed TCP stream over the OpenVPN tunnel: channel-backed
//! `AsyncRead`/`AsyncWrite` bridged to a smoltcp socket inside the device loop.

use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context as TaskContext, Poll};

use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};
use tokio::sync::{Notify, mpsc};

use super::device_loop::{FLOW_BUFFER, WriterWakers};

/// A relayed TCP stream: bytes flow through channels to/from a smoltcp socket
/// whose IP packets are sealed into `P_DATA_V2` by the device loop.
pub struct OvpnTcpStream {
    /// Caller -> loop bytes; dropped on shutdown to half-close the socket.
    pub(super) write_tx: Option<mpsc::Sender<Vec<u8>>>,
    /// Loop -> caller bytes; closed (EOF) on peer FIN or device shutdown.
    pub(super) read_rx: mpsc::Receiver<Vec<u8>>,
    pub(super) wake: Arc<Notify>,
    pub(super) writer_wakers: WriterWakers,
    pub(super) leftover: Vec<u8>,
    pub(super) leftover_pos: usize,
}

impl AsyncRead for OvpnTcpStream {
    fn poll_read(self: Pin<&mut Self>, cx: &mut TaskContext<'_>, buf: &mut ReadBuf<'_>) -> Poll<std::io::Result<()>> {
        let this = self.get_mut();
        loop {
            if this.leftover_pos < this.leftover.len() {
                let n = buf.remaining().min(this.leftover.len() - this.leftover_pos);
                buf.put_slice(&this.leftover[this.leftover_pos..this.leftover_pos + n]);
                this.leftover_pos += n;
                return Poll::Ready(Ok(()));
            }
            match this.read_rx.poll_recv(cx) {
                Poll::Ready(Some(data)) => {
                    this.leftover = data;
                    this.leftover_pos = 0;
                }
                Poll::Ready(None) => return Poll::Ready(Ok(())), // EOF
                Poll::Pending => return Poll::Pending,
            }
        }
    }
}

impl AsyncWrite for OvpnTcpStream {
    fn poll_write(self: Pin<&mut Self>, cx: &mut TaskContext<'_>, buf: &[u8]) -> Poll<std::io::Result<usize>> {
        let this = self.get_mut();
        let Some(tx) = &this.write_tx else {
            return Poll::Ready(Err(std::io::ErrorKind::BrokenPipe.into()));
        };
        if buf.is_empty() {
            return Poll::Ready(Ok(0));
        }
        let take = buf.len().min(FLOW_BUFFER);
        match tx.try_send(buf[..take].to_vec()) {
            Ok(()) => {
                this.wake.notify_one();
                Poll::Ready(Ok(take))
            }
            Err(mpsc::error::TrySendError::Full(_)) => {
                this.writer_wakers
                    .lock()
                    .expect("openvpn writer wakers")
                    .push(cx.waker().clone());
                this.wake.notify_one();
                Poll::Pending
            }
            Err(mpsc::error::TrySendError::Closed(_)) => Poll::Ready(Err(std::io::ErrorKind::BrokenPipe.into())),
        }
    }

    fn poll_flush(self: Pin<&mut Self>, _cx: &mut TaskContext<'_>) -> Poll<std::io::Result<()>> {
        Poll::Ready(Ok(()))
    }

    fn poll_shutdown(self: Pin<&mut Self>, _cx: &mut TaskContext<'_>) -> Poll<std::io::Result<()>> {
        let this = self.get_mut();
        if this.write_tx.take().is_some() {
            this.wake.notify_one();
        }
        Poll::Ready(Ok(()))
    }
}
