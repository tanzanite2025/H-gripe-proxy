//! XHTTP transport (`network: xhttp`).
//!
//! XHTTP (formerly SplitHTTP) multiplexes a proxy stream over HTTP. It defines
//! several transmission modes; this module implements three:
//!
//! * `stream-one` ([`stream_one`]) — a single full-duplex HTTP/2 `POST`
//!   (request body up, response body down). The simplest mode and the default.
//! * `stream-up` ([`multi`]) — uplink is one streaming `POST`, downlink is a
//!   **separate** `GET`, the two correlated by a random session id in the path
//!   (`<path>/<session-id>`). Useful where a CDN refuses full-duplex bodies.
//! * `packet-up` ([`multi`]) — uplink is a sequence of `POST`s, one per packet
//!   (`<path>/<session-id>/<seq>`, `seq` from 0), downlink is the same `GET`.
//!   Useful where the edge buffers whole request bodies (no streamed upload).
//!
//! The wire layout (path scheme, `x_padding` query, sequential `seq`) follows
//! Xray-core's `splithttp` so the kernel interops with real XHTTP servers.
//!
//! `auto` (and an empty/absent mode) maps to `stream-one`: the conservative
//! default that preserves the previously shipped behaviour.
//!
//! Security (TLS/REALITY) lives below in [`crate::transport`], so this module
//! never deals with certificates.

mod multi;
mod stream_one;

use std::io;
use std::pin::Pin;
use std::task::{Context as TaskContext, Poll};

use anyhow::Result;
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};

use crate::transport::h2stream::H2ByteStream;
use multi::MultiStream;

/// XHTTP transmission mode.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum XhttpMode {
    /// Single full-duplex HTTP/2 stream (request body up, response body down).
    #[default]
    StreamOne,
    /// Single streaming `POST` up, separate `GET` down (session-id correlated).
    StreamUp,
    /// One `POST` per packet up, separate `GET` down (session-id + seq).
    PacketUp,
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

/// The byte stream produced by [`connect`], unifying the single-request
/// (`stream-one`) and multi-request (`stream-up` / `packet-up`) shapes behind
/// one `AsyncRead + AsyncWrite` so `build_layers` treats every mode alike.
pub enum XhttpStream {
    One(H2ByteStream),
    Multi(MultiStream),
}

/// Open the XHTTP tunnel over `stream` per the configured mode.
pub async fn connect<S>(stream: S, server: &str, over_tls: bool, cfg: &XhttpTransportConfig) -> Result<XhttpStream>
where
    S: AsyncRead + AsyncWrite + Unpin + Send + 'static,
{
    match cfg.mode {
        XhttpMode::StreamOne => Ok(XhttpStream::One(
            stream_one::connect(stream, server, over_tls, cfg).await?,
        )),
        XhttpMode::StreamUp => Ok(XhttpStream::Multi(
            multi::connect(stream, server, over_tls, cfg, multi::Uplink::Stream).await?,
        )),
        XhttpMode::PacketUp => Ok(XhttpStream::Multi(
            multi::connect(stream, server, over_tls, cfg, multi::Uplink::Packet).await?,
        )),
    }
}

impl AsyncRead for XhttpStream {
    fn poll_read(self: Pin<&mut Self>, cx: &mut TaskContext<'_>, buf: &mut ReadBuf<'_>) -> Poll<io::Result<()>> {
        match self.get_mut() {
            XhttpStream::One(s) => Pin::new(s).poll_read(cx, buf),
            XhttpStream::Multi(s) => Pin::new(s).poll_read(cx, buf),
        }
    }
}

impl AsyncWrite for XhttpStream {
    fn poll_write(self: Pin<&mut Self>, cx: &mut TaskContext<'_>, buf: &[u8]) -> Poll<io::Result<usize>> {
        match self.get_mut() {
            XhttpStream::One(s) => Pin::new(s).poll_write(cx, buf),
            XhttpStream::Multi(s) => Pin::new(s).poll_write(cx, buf),
        }
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut TaskContext<'_>) -> Poll<io::Result<()>> {
        match self.get_mut() {
            XhttpStream::One(s) => Pin::new(s).poll_flush(cx),
            XhttpStream::Multi(s) => Pin::new(s).poll_flush(cx),
        }
    }

    fn poll_shutdown(self: Pin<&mut Self>, cx: &mut TaskContext<'_>) -> Poll<io::Result<()>> {
        match self.get_mut() {
            XhttpStream::One(s) => Pin::new(s).poll_shutdown(cx),
            XhttpStream::Multi(s) => Pin::new(s).poll_shutdown(cx),
        }
    }
}

/// `:authority` for the request: the configured `host`, else the dial server.
fn authority_of(cfg: &XhttpTransportConfig, server: &str) -> String {
    cfg.host
        .clone()
        .filter(|h| !h.is_empty())
        .unwrap_or_else(|| server.to_string())
}

/// Normalize a `stream-one` request path to a leading-slash absolute path.
fn normalize_path(path: &str) -> String {
    if path.is_empty() {
        "/".to_string()
    } else if path.starts_with('/') {
        path.to_string()
    } else {
        format!("/{path}")
    }
}

/// Base path for the multi-request modes: a leading slash and no trailing slash
/// (so `/` becomes `""` and `<base>/<session>` stays single-slashed).
fn base_path(path: &str) -> String {
    let p = normalize_path(path);
    let trimmed = p.trim_end_matches('/');
    trimmed.to_string()
}

/// Downlink (`GET`) / streaming-uplink (`POST`) path: `<base>/<session-id>`.
fn session_path(base: &str, session: &str) -> String {
    format!("{base}/{session}")
}

/// Per-packet uplink (`POST`) path: `<base>/<session-id>/<seq>`.
fn packet_path(base: &str, session: &str, seq: u64) -> String {
    format!("{base}/{session}/{seq}")
}

/// A random `x_padding` query string (Xray pads request URLs to blur sizes; the
/// server ignores the value). Length is random within `100..=900` `0` bytes.
fn padding_query() -> String {
    let mut b = [0u8; 2];
    getrandom::fill(&mut b).expect("os rng");
    let len = 100 + (u16::from_le_bytes(b) % 800) as usize;
    format!("?x_padding={}", "0".repeat(len))
}

/// A random session id formatted as a canonical UUID (matches Xray clients).
fn session_id() -> String {
    let mut b = [0u8; 16];
    getrandom::fill(&mut b).expect("os rng");
    // Tag as RFC 4122 v4 so the value looks like the UUIDs Xray emits.
    b[6] = (b[6] & 0x0f) | 0x40;
    b[8] = (b[8] & 0x3f) | 0x80;
    let h = |s: &[u8]| s.iter().map(|x| format!("{x:02x}")).collect::<String>();
    format!(
        "{}-{}-{}-{}-{}",
        h(&b[0..4]),
        h(&b[4..6]),
        h(&b[6..8]),
        h(&b[8..10]),
        h(&b[10..16])
    )
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

    #[test]
    fn base_path_drops_trailing_slash() {
        assert_eq!(base_path("/"), "");
        assert_eq!(base_path(""), "");
        assert_eq!(base_path("/abc/"), "/abc");
        assert_eq!(base_path("abc"), "/abc");
    }

    #[test]
    fn session_and_packet_paths_are_single_slashed() {
        assert_eq!(session_path("", "sid"), "/sid");
        assert_eq!(session_path("/abc", "sid"), "/abc/sid");
        assert_eq!(packet_path("/abc", "sid", 0), "/abc/sid/0");
        assert_eq!(packet_path("", "sid", 7), "/sid/7");
    }

    #[test]
    fn session_id_is_uuid_shaped() {
        let id = session_id();
        assert_eq!(id.len(), 36);
        assert_eq!(id.as_bytes()[14], b'4'); // version nibble
        let dashes: Vec<usize> = id.match_indices('-').map(|(i, _)| i).collect();
        assert_eq!(dashes, vec![8, 13, 18, 23]);
    }

    #[test]
    fn padding_query_is_in_range() {
        for _ in 0..32 {
            let q = padding_query();
            let pad = q.strip_prefix("?x_padding=").unwrap();
            assert!((100..=899).contains(&pad.len()));
            assert!(pad.bytes().all(|b| b == b'0'));
        }
    }
}
