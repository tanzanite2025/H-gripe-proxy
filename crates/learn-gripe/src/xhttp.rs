//! XHTTP transport (`network: xhttp`), `stream-one` mode.
//!
//! XHTTP (formerly SplitHTTP) multiplexes a proxy stream over HTTP. It defines
//! several modes; this slice implements `stream-one`: a single full-duplex
//! HTTP/2 `POST` whose request body carries the uplink and whose response body
//! carries the downlink, with **raw** application bytes in both directions (no
//! gRPC/`Hunk` framing). It is, in effect, the un-framed sibling of the gRPC
//! "gun" tunnel and reuses the same HTTP/2 byte-stream shape.
//!
//! The multi-request `stream-up`/`packet-up` modes (separate uplink POSTs and a
//! downlink GET, correlated by a session id) need request-sequencing machinery
//! and are rejected at config-build time in `vless` rather than mis-encoded.
//!
//! HTTP/2 framing/flow-control is delegated to the `h2` crate (the same "do not
//! hand-roll" boundary as TLS); only the byte-stream adapter is local. Security
//! (TLS/REALITY) lives below in [`crate::transport`], so this module never
//! deals with certificates.

use std::io;
use std::pin::Pin;
use std::task::{Context as TaskContext, Poll, ready};

use anyhow::{Context, Result};
use bytes::Bytes;
use h2::client;
use h2::{RecvStream, SendStream};
use http::{Method, Request};
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};

/// XHTTP transmission mode. Only `stream-one` is implemented in this slice.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum XhttpMode {
    /// Single full-duplex HTTP/2 stream (request body up, response body down).
    #[default]
    StreamOne,
}

/// Resolved XHTTP transport options.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct XhttpTransportConfig {
    /// Request path (defaults to `/`).
    pub path: String,
    /// `:authority` used for the request; falls back to the dial server.
    pub host: Option<String>,
    /// Transmission mode.
    pub mode: XhttpMode,
}

/// Open the XHTTP `stream-one` tunnel over `stream` and return a byte-stream view.
pub async fn connect<S>(stream: S, server: &str, over_tls: bool, cfg: &XhttpTransportConfig) -> Result<XhttpByteStream>
where
    S: AsyncRead + AsyncWrite + Unpin + Send + 'static,
{
    let XhttpMode::StreamOne = cfg.mode;

    let (send_request, connection) = client::Builder::new()
        .handshake::<S, Bytes>(stream)
        .await
        .context("xhttp: http/2 handshake")?;

    // Drive the HTTP/2 connection in the background; it completes once both the
    // send and receive halves are closed.
    tokio::spawn(async move {
        let _ = connection.await;
    });

    let authority = cfg
        .host
        .clone()
        .filter(|h| !h.is_empty())
        .unwrap_or_else(|| server.to_string());
    let scheme = if over_tls { "https" } else { "http" };
    let path = normalize_path(&cfg.path);
    let uri = format!("{scheme}://{authority}{path}");

    let request = Request::builder()
        .method(Method::POST)
        .uri(&uri)
        .header(http::header::CONTENT_TYPE, "application/octet-stream")
        .body(())
        .with_context(|| format!("xhttp: build request for {uri}"))?;

    let mut send_request = send_request.ready().await.context("xhttp: connection not ready")?;
    let (response, send_stream) = send_request
        .send_request(request, false)
        .context("xhttp: send request")?;
    let response = response.await.context("xhttp: await response headers")?;
    let recv_stream = response.into_body();

    Ok(XhttpByteStream::new(send_stream, recv_stream))
}

fn normalize_path(path: &str) -> String {
    if path.is_empty() {
        "/".to_string()
    } else if path.starts_with('/') {
        path.to_string()
    } else {
        format!("/{path}")
    }
}

fn to_io_err<E: std::fmt::Display>(e: E) -> io::Error {
    io::Error::other(e.to_string())
}

/// Adapts a single HTTP/2 stream into a contiguous byte stream by passing the
/// application bytes through verbatim in both directions.
pub struct XhttpByteStream {
    send: SendStream<Bytes>,
    recv: RecvStream,
    /// Outbound bytes not yet handed to HTTP/2 flow control.
    write_buf: Bytes,
    /// Inbound bytes not yet copied to the caller.
    read_buf: Bytes,
    recv_eof: bool,
}

impl XhttpByteStream {
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
                    return Poll::Ready(Err(io::Error::new(
                        io::ErrorKind::BrokenPipe,
                        "xhttp: send stream closed",
                    )));
                }
            }
        }
        Poll::Ready(Ok(()))
    }
}

impl AsyncRead for XhttpByteStream {
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

impl AsyncWrite for XhttpByteStream {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_paths() {
        assert_eq!(normalize_path(""), "/");
        assert_eq!(normalize_path("/x"), "/x");
        assert_eq!(normalize_path("x"), "/x");
    }
}
