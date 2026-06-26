//! gRPC transport (`network: grpc`) — the Xray/V2Ray "gun" tunnel.
//!
//! Runs an HTTP/2 stream over an already-established (optionally TLS-secured)
//! connection and adapts it into a plain [`AsyncRead`]/[`AsyncWrite`] byte
//! stream, so the protocol layer above it (VLESS today) is unaware its bytes
//! are tunnelled inside gRPC messages.
//!
//! Wire shape: a single bidirectional stream to `POST /{service}/Tun` with
//! `content-type: application/grpc`. Each chunk of application bytes is wrapped
//! as one gRPC length-prefixed message whose body is a protobuf `Hunk { bytes
//! data = 1; }`:
//!
//! ```text
//! gRPC frame:  0x00 | len(u32 BE) | message
//! message:     0x0a | varint(N)   | data(N)   (protobuf field #1, wire type 2)
//! ```
//!
//! HTTP/2 framing/flow-control is delegated to the `h2` crate (the same
//! "do not hand-roll" boundary as TLS); only the gRPC/protobuf wrapping and the
//! byte-stream adapter are local, which is where this kernel wants control.
//! Security (TLS/REALITY) lives below in [`crate::transport`], so this module
//! never deals with certificates.

use std::io;
use std::pin::Pin;
use std::task::{Context as TaskContext, Poll, ready};

use anyhow::{Context, Result};
use bytes::Bytes;
use h2::client;
use h2::{RecvStream, SendStream};
use http::{Method, Request};
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};

/// Default gRPC service name (matches v2ray-core's "gun" default) used when the
/// proxy leaves `grpc-service-name` empty.
const DEFAULT_SERVICE_NAME: &str = "GunService";

/// Resolved gRPC transport options.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct GrpcTransportConfig {
    /// gRPC service name; the request path is `/{service_name}/Tun`.
    pub service_name: String,
    /// `:authority` used for the request; falls back to the dial server.
    pub host: Option<String>,
}

/// Open the gRPC tunnel over `stream` and return a byte-stream view.
pub async fn connect<S>(stream: S, server: &str, over_tls: bool, cfg: &GrpcTransportConfig) -> Result<GrpcByteStream>
where
    S: AsyncRead + AsyncWrite + Unpin + Send + 'static,
{
    let (send_request, connection) = client::Builder::new()
        .handshake::<S, Bytes>(stream)
        .await
        .context("grpc: http/2 handshake")?;

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
    let path = service_path(&cfg.service_name);
    let uri = format!("{scheme}://{authority}{path}");

    let request = Request::builder()
        .method(Method::POST)
        .uri(&uri)
        .header(http::header::CONTENT_TYPE, "application/grpc")
        .header(http::header::TE, "trailers")
        .header(http::header::USER_AGENT, "grpc-go/1.26.0")
        .body(())
        .with_context(|| format!("grpc: build request for {uri}"))?;

    let mut send_request = send_request.ready().await.context("grpc: connection not ready")?;
    let (response, send_stream) = send_request
        .send_request(request, false)
        .context("grpc: send request")?;
    let response = response.await.context("grpc: await response headers")?;
    let recv_stream = response.into_body();

    Ok(GrpcByteStream::new(send_stream, recv_stream))
}

fn service_path(service_name: &str) -> String {
    let name = service_name.trim_matches('/');
    let name = if name.is_empty() { DEFAULT_SERVICE_NAME } else { name };
    format!("/{name}/Tun")
}

/// Encode `data` as a single gRPC-framed protobuf `Hunk` message.
pub(crate) fn encode_frame(data: &[u8]) -> Bytes {
    let mut hunk = Vec::with_capacity(data.len() + 6);
    hunk.push(0x0a); // field #1, wire type 2 (length-delimited)
    write_varint(&mut hunk, data.len() as u64);
    hunk.extend_from_slice(data);

    let mut frame = Vec::with_capacity(hunk.len() + 5);
    frame.push(0x00); // not compressed
    frame.extend_from_slice(&(hunk.len() as u32).to_be_bytes());
    frame.extend_from_slice(&hunk);
    Bytes::from(frame)
}

fn write_varint(out: &mut Vec<u8>, mut value: u64) {
    loop {
        let mut byte = (value & 0x7f) as u8;
        value >>= 7;
        if value != 0 {
            byte |= 0x80;
        }
        out.push(byte);
        if value == 0 {
            break;
        }
    }
}

fn read_varint(buf: &[u8]) -> Option<(u64, usize)> {
    let mut value: u64 = 0;
    let mut shift = 0;
    for (i, &byte) in buf.iter().enumerate().take(10) {
        value |= u64::from(byte & 0x7f) << shift;
        if byte & 0x80 == 0 {
            return Some((value, i + 1));
        }
        shift += 7;
    }
    None
}

fn invalid<E: std::fmt::Display>(msg: E) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, msg.to_string())
}

/// Extract the `data` field(s) (protobuf field #1) of a `Hunk` message body
/// into `out`, tolerating (skipping) any other fields for forward-compat.
pub(crate) fn decode_hunk(msg: &[u8], out: &mut Vec<u8>) -> io::Result<()> {
    let mut i = 0;
    while i < msg.len() {
        let (tag, n) = read_varint(&msg[i..]).ok_or_else(|| invalid("grpc: truncated protobuf tag"))?;
        i += n;
        let field = tag >> 3;
        let wire = tag & 0x07;
        match wire {
            0 => {
                let (_, n) = read_varint(&msg[i..]).ok_or_else(|| invalid("grpc: truncated varint field"))?;
                i += n;
            }
            1 => i += 8,
            5 => i += 4,
            2 => {
                let (len, n) = read_varint(&msg[i..]).ok_or_else(|| invalid("grpc: truncated length field"))?;
                i += n;
                let len = len as usize;
                if i + len > msg.len() {
                    return Err(invalid("grpc: length-delimited field overruns message"));
                }
                if field == 1 {
                    out.extend_from_slice(&msg[i..i + len]);
                }
                i += len;
            }
            other => return Err(invalid(format!("grpc: unsupported protobuf wire type {other}"))),
        }
    }
    Ok(())
}

fn to_io_err<E: std::fmt::Display>(e: E) -> io::Error {
    io::Error::other(e.to_string())
}

/// Adapts an HTTP/2 stream into a contiguous byte stream: writes become gRPC
/// `Hunk` messages, and inbound messages are unwrapped back into bytes.
pub struct GrpcByteStream {
    send: SendStream<Bytes>,
    recv: RecvStream,
    /// Encoded outbound frame bytes not yet handed to HTTP/2 flow control.
    write_buf: Bytes,
    /// Raw inbound bytes (gRPC framing) not yet decoded.
    raw: Vec<u8>,
    /// Decoded application bytes not yet copied to the caller.
    read_buf: Vec<u8>,
    read_pos: usize,
    recv_eof: bool,
}

impl GrpcByteStream {
    fn new(send: SendStream<Bytes>, recv: RecvStream) -> Self {
        Self {
            send,
            recv,
            write_buf: Bytes::new(),
            raw: Vec::new(),
            read_buf: Vec::new(),
            read_pos: 0,
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
                        "grpc: send stream closed",
                    )));
                }
            }
        }
        Poll::Ready(Ok(()))
    }

    /// Decode one complete gRPC message from `raw` into `read_buf`. Returns
    /// `Ok(true)` when a message was consumed (even if it carried no data).
    fn decode_one(&mut self) -> io::Result<bool> {
        if self.raw.len() < 5 {
            return Ok(false);
        }
        if self.raw[0] != 0 {
            return Err(invalid("grpc: compressed messages are not supported"));
        }
        let msg_len = u32::from_be_bytes([self.raw[1], self.raw[2], self.raw[3], self.raw[4]]) as usize;
        if self.raw.len() < 5 + msg_len {
            return Ok(false);
        }
        let msg = self.raw[5..5 + msg_len].to_vec();
        decode_hunk(&msg, &mut self.read_buf)?;
        self.raw.drain(0..5 + msg_len);
        Ok(true)
    }
}

impl AsyncRead for GrpcByteStream {
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
            this.read_buf.clear();
            this.read_pos = 0;

            if this.decode_one()? {
                continue;
            }
            if this.recv_eof {
                return Poll::Ready(Ok(()));
            }

            match ready!(Pin::new(&mut this.recv).poll_data(cx)) {
                Some(Ok(data)) => {
                    let len = data.len();
                    this.raw.extend_from_slice(&data);
                    // Return the consumed bytes to the HTTP/2 receive window.
                    let _ = this.recv.flow_control().release_capacity(len);
                }
                Some(Err(e)) => return Poll::Ready(Err(to_io_err(e))),
                None => this.recv_eof = true,
            }
        }
    }
}

impl AsyncWrite for GrpcByteStream {
    fn poll_write(self: Pin<&mut Self>, cx: &mut TaskContext<'_>, buf: &[u8]) -> Poll<io::Result<usize>> {
        let this = self.get_mut();
        // Hand off any leftover from a previous frame before accepting more.
        ready!(this.poll_drain(cx))?;
        this.write_buf = encode_frame(buf);
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
    fn frame_roundtrips_through_hunk() {
        let frame = encode_frame(b"hello gripe");
        assert_eq!(frame[0], 0x00);
        let msg_len = u32::from_be_bytes([frame[1], frame[2], frame[3], frame[4]]) as usize;
        assert_eq!(frame.len(), 5 + msg_len);

        let mut out = Vec::new();
        decode_hunk(&frame[5..], &mut out).unwrap();
        assert_eq!(out, b"hello gripe");
    }

    #[test]
    fn service_path_defaults_when_empty() {
        assert_eq!(service_path(""), "/GunService/Tun");
        assert_eq!(service_path("/foo/"), "/foo/Tun");
        assert_eq!(service_path("bar"), "/bar/Tun");
    }

    #[test]
    fn decode_hunk_skips_unknown_fields() {
        // field #2 varint (skipped) + field #1 bytes "ok"
        let msg = [0x10, 0x07, 0x0a, 0x02, b'o', b'k'];
        let mut out = Vec::new();
        decode_hunk(&msg, &mut out).unwrap();
        assert_eq!(out, b"ok");
    }
}
