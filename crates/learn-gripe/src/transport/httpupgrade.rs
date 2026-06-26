//! HTTP Upgrade transport (`network: ws` + `ws-opts.v2ray-http-upgrade: true`).
//!
//! V2Ray/Xray's `httpupgrade` is a leaner cousin of WebSocket: the client sends
//! a single HTTP/1.1 `GET` with `Connection: Upgrade` / `Upgrade: websocket`,
//! the server answers `101 Switching Protocols`, and from then on the socket is
//! a **raw** bidirectional byte stream — there is no per-message WebSocket
//! framing or masking. That makes the adapter trivial: after the handshake the
//! protocol layer above (VLESS today) reads and writes plain bytes, with only a
//! small buffer for any application bytes that arrived alongside the response.
//!
//! Security (TLS/REALITY) lives below in [`crate::transport`], so this module
//! never deals with certificates.

use std::collections::BTreeMap;
use std::io;
use std::pin::Pin;
use std::task::{Context as TaskContext, Poll};

use anyhow::{Context, Result, bail};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt, ReadBuf};

/// Resolved HTTP-Upgrade transport options.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct HttpUpgradeTransportConfig {
    /// Request path (defaults to `/`).
    pub path: String,
    /// `Host` header / authority; falls back to the dial server when unset.
    pub host: Option<String>,
    /// Extra request headers to send during the handshake.
    pub headers: BTreeMap<String, String>,
}

/// Perform the HTTP Upgrade handshake over `stream` and return a byte-stream
/// view that yields the raw application bytes flowing after `101`.
pub async fn connect<S>(
    mut stream: S,
    server: &str,
    cfg: &HttpUpgradeTransportConfig,
) -> Result<HttpUpgradeByteStream<S>>
where
    S: AsyncRead + AsyncWrite + Unpin,
{
    let authority = cfg
        .host
        .clone()
        .filter(|h| !h.is_empty())
        .unwrap_or_else(|| server.to_string());
    let path = normalize_path(&cfg.path);

    let mut request =
        format!("GET {path} HTTP/1.1\r\nHost: {authority}\r\nConnection: Upgrade\r\nUpgrade: websocket\r\n");
    for (key, value) in &cfg.headers {
        // The handshake-defining headers are owned by this layer; never let a
        // pass-through header override them.
        if is_reserved_header(key) {
            continue;
        }
        request.push_str(key);
        request.push_str(": ");
        request.push_str(value);
        request.push_str("\r\n");
    }
    request.push_str("\r\n");

    stream
        .write_all(request.as_bytes())
        .await
        .context("httpupgrade: send request")?;
    stream.flush().await.context("httpupgrade: flush request")?;

    let mut buf = Vec::with_capacity(256);
    let mut tmp = [0u8; 256];
    let header_end = loop {
        let n = stream.read(&mut tmp).await.context("httpupgrade: read response")?;
        if n == 0 {
            bail!("httpupgrade: connection closed before response completed");
        }
        buf.extend_from_slice(&tmp[..n]);
        if let Some(pos) = find_header_end(&buf) {
            break pos;
        }
        if buf.len() > 64 * 1024 {
            bail!("httpupgrade: response headers exceed 64 KiB");
        }
    };

    verify_switching_protocols(&buf[..header_end])?;
    let prefix = buf[header_end..].to_vec();
    Ok(HttpUpgradeByteStream::new(stream, prefix))
}

fn is_reserved_header(key: &str) -> bool {
    let key = key.to_ascii_lowercase();
    matches!(key.as_str(), "host" | "connection" | "upgrade")
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

/// Index just past the `\r\n\r\n` that terminates the response head, if present.
fn find_header_end(buf: &[u8]) -> Option<usize> {
    buf.windows(4).position(|w| w == b"\r\n\r\n").map(|p| p + 4)
}

/// Confirm the response status line is `101 Switching Protocols`.
fn verify_switching_protocols(head: &[u8]) -> Result<()> {
    let text = std::str::from_utf8(head).context("httpupgrade: non-UTF8 response head")?;
    let status_line = text.lines().next().unwrap_or_default();
    // e.g. "HTTP/1.1 101 Switching Protocols"
    let code = status_line.split_whitespace().nth(1);
    if code != Some("101") {
        bail!("httpupgrade: expected 101 Switching Protocols, got {status_line:?}");
    }
    Ok(())
}

/// Raw byte stream over an upgraded HTTP/1.1 connection. Reads first drain any
/// application bytes that arrived with the `101` response, then pass straight
/// through to the inner (optionally TLS-secured) stream; writes always pass
/// straight through.
#[derive(Debug)]
pub struct HttpUpgradeByteStream<S> {
    inner: S,
    prefix: Vec<u8>,
    prefix_pos: usize,
}

impl<S> HttpUpgradeByteStream<S> {
    fn new(inner: S, prefix: Vec<u8>) -> Self {
        Self {
            inner,
            prefix,
            prefix_pos: 0,
        }
    }
}

impl<S: AsyncRead + Unpin> AsyncRead for HttpUpgradeByteStream<S> {
    fn poll_read(self: Pin<&mut Self>, cx: &mut TaskContext<'_>, buf: &mut ReadBuf<'_>) -> Poll<io::Result<()>> {
        let this = self.get_mut();
        if this.prefix_pos < this.prefix.len() {
            let remaining = &this.prefix[this.prefix_pos..];
            let n = remaining.len().min(buf.remaining());
            buf.put_slice(&remaining[..n]);
            this.prefix_pos += n;
            return Poll::Ready(Ok(()));
        }
        Pin::new(&mut this.inner).poll_read(cx, buf)
    }
}

impl<S: AsyncWrite + Unpin> AsyncWrite for HttpUpgradeByteStream<S> {
    fn poll_write(self: Pin<&mut Self>, cx: &mut TaskContext<'_>, buf: &[u8]) -> Poll<io::Result<usize>> {
        Pin::new(&mut self.get_mut().inner).poll_write(cx, buf)
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut TaskContext<'_>) -> Poll<io::Result<()>> {
        Pin::new(&mut self.get_mut().inner).poll_flush(cx)
    }

    fn poll_shutdown(self: Pin<&mut Self>, cx: &mut TaskContext<'_>) -> Poll<io::Result<()>> {
        Pin::new(&mut self.get_mut().inner).poll_shutdown(cx)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_paths() {
        assert_eq!(normalize_path(""), "/");
        assert_eq!(normalize_path("/up"), "/up");
        assert_eq!(normalize_path("up"), "/up");
    }

    #[test]
    fn finds_header_terminator() {
        assert_eq!(find_header_end(b"abc"), None);
        assert_eq!(find_header_end(b"head\r\n\r\nbody"), Some(8));
    }

    #[test]
    fn accepts_switching_protocols() {
        let head = b"HTTP/1.1 101 Switching Protocols\r\nUpgrade: websocket\r\n";
        verify_switching_protocols(head).unwrap();
    }

    #[test]
    fn rejects_non_101() {
        let head = b"HTTP/1.1 200 OK\r\n";
        assert!(verify_switching_protocols(head).is_err());
    }
}
