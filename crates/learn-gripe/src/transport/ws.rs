//! WebSocket transport (`network: ws`).
//!
//! Performs the WebSocket client handshake over an already-established
//! (optionally TLS-secured) stream, then adapts the message-framed
//! `WebSocketStream` into a plain [`AsyncRead`]/[`AsyncWrite`] byte stream so
//! the protocol layer above it (VLESS today, VMess/Trojan later) is unaware it
//! is tunnelled inside WebSocket binary frames.
//!
//! The handshake itself is delegated to `tokio-tungstenite`; only the
//! byte-stream framing adapter is local, which is where this kernel wants full
//! control. Security (TLS/REALITY) lives below in [`crate::transport`], so this
//! module never deals with certificates.

use std::collections::BTreeMap;
use std::io;
use std::pin::Pin;
use std::task::{Context as TaskContext, Poll, ready};

use anyhow::{Context, Result};
use futures_util::{Sink, Stream};
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};
use tokio_tungstenite::WebSocketStream;
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::tungstenite::client::IntoClientRequest;
use tokio_tungstenite::tungstenite::http::{HeaderName, HeaderValue};

/// Resolved WebSocket transport options.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct WsTransportConfig {
    /// Request path (defaults to `/`).
    pub path: String,
    /// `Host` header / handshake authority used for domain fronting; falls back
    /// to the dial server when unset.
    pub host: Option<String>,
    /// Extra request headers to send during the handshake.
    pub headers: BTreeMap<String, String>,
}

/// Perform the WebSocket handshake over `stream` and return a byte-stream view.
pub async fn connect<S>(stream: S, server: &str, cfg: &WsTransportConfig) -> Result<WsByteStream<S>>
where
    S: AsyncRead + AsyncWrite + Unpin,
{
    let authority = cfg
        .host
        .clone()
        .filter(|h| !h.is_empty())
        .unwrap_or_else(|| server.to_string());
    let path = normalize_path(&cfg.path);
    let uri = format!("ws://{authority}{path}");

    let mut request = uri
        .as_str()
        .into_client_request()
        .with_context(|| format!("ws: build handshake request for {uri}"))?;
    {
        let headers = request.headers_mut();
        for (key, value) in &cfg.headers {
            let name =
                HeaderName::from_bytes(key.as_bytes()).with_context(|| format!("ws: invalid header name {key:?}"))?;
            let value =
                HeaderValue::from_str(value).with_context(|| format!("ws: invalid header value for {key:?}"))?;
            headers.insert(name, value);
        }
    }

    let (ws, _response) = tokio_tungstenite::client_async(request, stream)
        .await
        .with_context(|| format!("ws: handshake with {authority}{path}"))?;
    Ok(WsByteStream::new(ws))
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

/// Adapts a [`WebSocketStream`] into a contiguous byte stream: writes become
/// binary frames, and inbound binary/text frames are surfaced as bytes. Control
/// frames (ping/pong) are handled by the underlying library and skipped here.
#[derive(Debug)]
pub struct WsByteStream<S> {
    ws: WebSocketStream<S>,
    /// Bytes from a received frame not yet copied to the caller's buffer.
    read_buf: Vec<u8>,
    read_pos: usize,
    /// A queued frame from a prior `poll_write` still needs flushing.
    flushing: bool,
}

impl<S> WsByteStream<S> {
    fn new(ws: WebSocketStream<S>) -> Self {
        Self {
            ws,
            read_buf: Vec::new(),
            read_pos: 0,
            flushing: false,
        }
    }
}

fn to_io_err<E: std::fmt::Display>(e: E) -> io::Error {
    io::Error::other(e.to_string())
}

impl<S: AsyncRead + AsyncWrite + Unpin> AsyncRead for WsByteStream<S> {
    fn poll_read(self: Pin<&mut Self>, cx: &mut TaskContext<'_>, buf: &mut ReadBuf<'_>) -> Poll<io::Result<()>> {
        let this = self.get_mut();
        loop {
            if this.read_pos < this.read_buf.len() {
                let remaining = &this.read_buf[this.read_pos..];
                let n = remaining.len().min(buf.remaining());
                buf.put_slice(&remaining[..n]);
                this.read_pos += n;
                return Poll::Ready(Ok(()));
            }

            match ready!(Pin::new(&mut this.ws).poll_next(cx)) {
                Some(Ok(Message::Binary(data))) => {
                    this.read_buf = data.into();
                    this.read_pos = 0;
                }
                Some(Ok(Message::Text(text))) => {
                    this.read_buf = text.as_bytes().to_vec();
                    this.read_pos = 0;
                }
                // Control frames carry no application data: keep polling.
                Some(Ok(Message::Ping(_) | Message::Pong(_) | Message::Frame(_))) => {}
                // Clean close or end of stream -> EOF (zero bytes filled).
                Some(Ok(Message::Close(_))) | None => return Poll::Ready(Ok(())),
                Some(Err(e)) => return Poll::Ready(Err(to_io_err(e))),
            }
        }
    }
}

impl<S: AsyncRead + AsyncWrite + Unpin> AsyncWrite for WsByteStream<S> {
    fn poll_write(self: Pin<&mut Self>, cx: &mut TaskContext<'_>, buf: &[u8]) -> Poll<io::Result<usize>> {
        let this = self.get_mut();
        // Finish flushing a previously queued frame before accepting more, so a
        // single buffered request header (with no follow-up write) still reaches
        // the peer instead of stalling a request/response exchange.
        if this.flushing {
            ready!(Pin::new(&mut this.ws).poll_flush(cx)).map_err(to_io_err)?;
            this.flushing = false;
        }
        ready!(Pin::new(&mut this.ws).poll_ready(cx)).map_err(to_io_err)?;
        Pin::new(&mut this.ws)
            .start_send(Message::binary(buf.to_vec()))
            .map_err(to_io_err)?;
        match Pin::new(&mut this.ws).poll_flush(cx) {
            Poll::Ready(Ok(())) => {}
            Poll::Ready(Err(e)) => return Poll::Ready(Err(to_io_err(e))),
            Poll::Pending => this.flushing = true,
        }
        Poll::Ready(Ok(buf.len()))
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut TaskContext<'_>) -> Poll<io::Result<()>> {
        Pin::new(&mut self.get_mut().ws).poll_flush(cx).map_err(to_io_err)
    }

    fn poll_shutdown(self: Pin<&mut Self>, cx: &mut TaskContext<'_>) -> Poll<io::Result<()>> {
        Pin::new(&mut self.get_mut().ws).poll_close(cx).map_err(to_io_err)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_paths() {
        assert_eq!(normalize_path(""), "/");
        assert_eq!(normalize_path("/ws"), "/ws");
        assert_eq!(normalize_path("ws"), "/ws");
    }
}
