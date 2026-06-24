//! Shared HTTP/2 full-duplex byte-stream adapter.
//!
//! Several transports tunnel raw application bytes over a single HTTP/2
//! request: the request body carries the uplink and the response body carries
//! the downlink, with no additional framing. The V2Ray/Xray `h2` transport
//! (HTTP `PUT`) and XHTTP `stream-one` (HTTP `POST`) differ only in the request
//! line, so they share this adapter and the connection-driving handshake here.
//!
//! HTTP/2 framing/flow-control is delegated to the `h2` crate; only the
//! byte-stream view is local. The byte stream is identical in shape to the
//! gRPC tunnel minus the protobuf `Hunk` framing.

use std::io;
use std::pin::Pin;
use std::task::{Context as TaskContext, Poll, ready};

use anyhow::{Context, Result};
use bytes::Bytes;
use h2::client;
use h2::{RecvStream, SendStream};
use http::Request;
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};

pub(crate) fn to_io_err<E: std::fmt::Display>(e: E) -> io::Error {
    io::Error::other(e.to_string())
}

/// Perform the HTTP/2 client handshake over `stream`, send `request` (opening a
/// full-duplex body), await the response headers and return a byte-stream view
/// over the request/response bodies.
pub(crate) async fn open<S>(stream: S, request: Request<()>) -> Result<H2ByteStream>
where
    S: AsyncRead + AsyncWrite + Unpin + Send + 'static,
{
    let (send_request, connection) = client::Builder::new()
        .handshake::<S, Bytes>(stream)
        .await
        .context("h2: http/2 handshake")?;

    // Drive the HTTP/2 connection in the background; it completes once both the
    // send and receive halves are closed.
    tokio::spawn(async move {
        let _ = connection.await;
    });

    let mut send_request = send_request.ready().await.context("h2: connection not ready")?;
    let (response, send_stream) = send_request.send_request(request, false).context("h2: send request")?;
    let response = response.await.context("h2: await response headers")?;
    let recv_stream = response.into_body();

    Ok(H2ByteStream::new(send_stream, recv_stream))
}

/// Adapts a single HTTP/2 stream into a contiguous byte stream by passing the
/// application bytes through verbatim in both directions.
pub struct H2ByteStream {
    send: SendStream<Bytes>,
    recv: RecvStream,
    /// Outbound bytes not yet handed to HTTP/2 flow control.
    write_buf: Bytes,
    /// Inbound bytes not yet copied to the caller.
    read_buf: Bytes,
    recv_eof: bool,
}

impl H2ByteStream {
    fn new(send: SendStream<Bytes>, recv: RecvStream) -> Self {
        Self {
            send,
            recv,
            write_buf: Bytes::new(),
            read_buf: Bytes::new(),
            recv_eof: false,
        }
    }

    /// Drain `write_buf` into the HTTP/2 send stream, respecting flow control.
    fn poll_drain(&mut self, cx: &mut TaskContext<'_>) -> Poll<io::Result<()>> {
        while !self.write_buf.is_empty() {
            self.send.reserve_capacity(self.write_buf.len());
            match ready!(self.send.poll_capacity(cx)) {
                Some(Ok(cap)) => {
                    let n = cap.min(self.write_buf.len());
                    if n == 0 {
                        return Poll::Pending;
                    }
                    let chunk = self.write_buf.split_to(n);
                    self.send.send_data(chunk, false).map_err(to_io_err)?;
                }
                Some(Err(e)) => return Poll::Ready(Err(to_io_err(e))),
                None => {
                    return Poll::Ready(Err(io::Error::new(io::ErrorKind::BrokenPipe, "h2: send stream closed")));
                }
            }
        }
        Poll::Ready(Ok(()))
    }
}

impl AsyncRead for H2ByteStream {
    fn poll_read(self: Pin<&mut Self>, cx: &mut TaskContext<'_>, buf: &mut ReadBuf<'_>) -> Poll<io::Result<()>> {
        let this = self.get_mut();
        loop {
            if !this.read_buf.is_empty() {
                let n = this.read_buf.len().min(buf.remaining());
                let chunk = this.read_buf.split_to(n);
                buf.put_slice(&chunk);
                return Poll::Ready(Ok(()));
            }
            if this.recv_eof {
                return Poll::Ready(Ok(()));
            }
            match ready!(Pin::new(&mut this.recv).poll_data(cx)) {
                Some(Ok(data)) => {
                    let len = data.len();
                    // Return the consumed bytes to the HTTP/2 receive window.
                    let _ = this.recv.flow_control().release_capacity(len);
                    this.read_buf = data;
                }
                Some(Err(e)) => return Poll::Ready(Err(to_io_err(e))),
                None => this.recv_eof = true,
            }
        }
    }
}

impl AsyncWrite for H2ByteStream {
    fn poll_write(self: Pin<&mut Self>, cx: &mut TaskContext<'_>, buf: &[u8]) -> Poll<io::Result<usize>> {
        let this = self.get_mut();
        // Hand off any leftover from a previous write before accepting more.
        ready!(this.poll_drain(cx))?;
        this.write_buf = Bytes::copy_from_slice(buf);
        // Best-effort flush; any remainder is finished by poll_flush.
        match this.poll_drain(cx) {
            Poll::Ready(Err(e)) => return Poll::Ready(Err(e)),
            Poll::Ready(Ok(())) | Poll::Pending => {}
        }
        Poll::Ready(Ok(buf.len()))
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut TaskContext<'_>) -> Poll<io::Result<()>> {
        self.get_mut().poll_drain(cx)
    }

    fn poll_shutdown(self: Pin<&mut Self>, cx: &mut TaskContext<'_>) -> Poll<io::Result<()>> {
        let this = self.get_mut();
        ready!(this.poll_drain(cx))?;
        this.send.send_data(Bytes::new(), true).map_err(to_io_err)?;
        Poll::Ready(Ok(()))
    }
}
