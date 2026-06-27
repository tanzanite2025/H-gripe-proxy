//! simple-obfs **http** mode client.
//!
//! The HTTP mode frames the connection as a fake WebSocket upgrade: the client
//! sends a `GET ... Upgrade: websocket` request, the server replies with a
//! `101 Switching Protocols` response, and the real Shadowsocks bytes flow as
//! the request/response bodies. Neither side parses the other's header beyond
//! locating the `\r\n\r\n` terminator, so the obfs layer is a one-shot header on
//! connect with no per-packet framing thereafter.
//!
//! The request header is written eagerly at [`connect_http`]; the server's
//! response header is stripped lazily on the first read by [`ObfsHttpStream`],
//! after which reads and writes pass straight through to the inner transport.

use std::io;
use std::pin::Pin;
use std::task::{Context as TaskContext, Poll, ready};

use anyhow::{Context, Result};
use tokio::io::{AsyncRead, AsyncWrite, AsyncWriteExt, ReadBuf};

use super::{base64_encode, find_subsequence, random_bytes};

/// Maximum response header we will buffer before giving up, guarding against a
/// peer that never sends the `\r\n\r\n` terminator.
const MAX_RESPONSE_HEADER: usize = 8 * 1024;

const HEADER_TERMINATOR: &[u8] = b"\r\n\r\n";

/// Perform the simple-obfs HTTP "handshake": send the fake WebSocket-upgrade
/// request, then hand back a stream that strips the server's HTTP response
/// header on the first read.
pub async fn connect_http<S>(mut stream: S, host: &str, path: &str) -> Result<ObfsHttpStream<S>>
where
    S: AsyncRead + AsyncWrite + Unpin,
{
    let mut key_bytes = [0u8; 16];
    random_bytes(&mut key_bytes)?;
    let key = base64_encode(&key_bytes);

    let request = format!(
        "GET {path} HTTP/1.1\r\n\
         Host: {host}\r\n\
         User-Agent: curl/7.88.1\r\n\
         Upgrade: websocket\r\n\
         Connection: Upgrade\r\n\
         Sec-WebSocket-Key: {key}\r\n\
         Content-Length: 0\r\n\
         \r\n"
    );
    stream
        .write_all(request.as_bytes())
        .await
        .context("simple-obfs http: send request header")?;

    Ok(ObfsHttpStream {
        inner: stream,
        response_stripped: false,
        header: Vec::new(),
        leftover: Vec::new(),
        leftover_pos: 0,
    })
}

/// A simple-obfs HTTP client stream. Writes pass straight through (the request
/// header was already sent by [`connect_http`]); the first reads strip the
/// server's `101 Switching Protocols` response header before yielding the
/// Shadowsocks bytes that follow it.
pub struct ObfsHttpStream<S> {
    inner: S,
    response_stripped: bool,
    /// Accumulated response-header bytes while still searching for the
    /// terminator.
    header: Vec<u8>,
    /// Body bytes that arrived in the same read as the header terminator and
    /// have not yet been handed to the caller.
    leftover: Vec<u8>,
    leftover_pos: usize,
}

impl<S> AsyncRead for ObfsHttpStream<S>
where
    S: AsyncRead + AsyncWrite + Unpin,
{
    fn poll_read(self: Pin<&mut Self>, cx: &mut TaskContext<'_>, buf: &mut ReadBuf<'_>) -> Poll<io::Result<()>> {
        let this = self.get_mut();

        // Drain any body bytes that were buffered alongside the header.
        if this.leftover_pos < this.leftover.len() {
            let n = std::cmp::min(buf.remaining(), this.leftover.len() - this.leftover_pos);
            buf.put_slice(&this.leftover[this.leftover_pos..this.leftover_pos + n]);
            this.leftover_pos += n;
            if this.leftover_pos == this.leftover.len() {
                this.leftover.clear();
                this.leftover_pos = 0;
            }
            return Poll::Ready(Ok(()));
        }

        if this.response_stripped {
            return Pin::new(&mut this.inner).poll_read(cx, buf);
        }

        // Read raw bytes and search for the end of the response header.
        let mut tmp = [0u8; 2048];
        loop {
            let mut read_buf = ReadBuf::new(&mut tmp);
            ready!(Pin::new(&mut this.inner).poll_read(cx, &mut read_buf))?;
            let chunk = read_buf.filled();
            if chunk.is_empty() {
                return Poll::Ready(Err(io::Error::new(
                    io::ErrorKind::UnexpectedEof,
                    "simple-obfs http: connection closed before response header",
                )));
            }
            this.header.extend_from_slice(chunk);

            if let Some(pos) = find_subsequence(&this.header, HEADER_TERMINATOR) {
                let body_start = pos + HEADER_TERMINATOR.len();
                let body = this.header.split_off(body_start);
                this.header.clear();
                this.response_stripped = true;

                if body.is_empty() {
                    // No body bytes yet; let the caller poll again for data.
                    return Pin::new(&mut this.inner).poll_read(cx, buf);
                }
                let n = std::cmp::min(buf.remaining(), body.len());
                buf.put_slice(&body[..n]);
                if n < body.len() {
                    this.leftover = body[n..].to_vec();
                    this.leftover_pos = 0;
                }
                return Poll::Ready(Ok(()));
            }

            if this.header.len() > MAX_RESPONSE_HEADER {
                return Poll::Ready(Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "simple-obfs http: response header too large",
                )));
            }
        }
    }
}

impl<S> AsyncWrite for ObfsHttpStream<S>
where
    S: AsyncRead + AsyncWrite + Unpin,
{
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
