//! XHTTP transport (`network: xhttp`), `stream-one` mode.
//!
//! XHTTP (formerly SplitHTTP) multiplexes a proxy stream over HTTP. It defines
//! several modes; this slice implements `stream-one`: a single full-duplex
//! HTTP/2 `POST` whose request body carries the uplink and whose response body
//! carries the downlink, with **raw** application bytes in both directions (no
//! gRPC/`Hunk` framing). The HTTP/2 byte-stream itself lives in
//! [`crate::transport::h2stream`] and is shared with the `h2` (`PUT`) transport.
//!
//! The multi-request `stream-up`/`packet-up` modes (separate uplink POSTs and a
//! downlink GET, correlated by a session id) need request-sequencing machinery
//! and are rejected at config-build time in `vless` rather than mis-encoded.
//!
//! Security (TLS/REALITY) lives below in [`crate::transport`], so this module
//! never deals with certificates.

use anyhow::{Context, Result};
use http::{Method, Request};
use tokio::io::{AsyncRead, AsyncWrite};

use crate::transport::h2stream::{self, H2ByteStream};

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
pub async fn connect<S>(stream: S, server: &str, over_tls: bool, cfg: &XhttpTransportConfig) -> Result<H2ByteStream>
where
    S: AsyncRead + AsyncWrite + Unpin + Send + 'static,
{
    let XhttpMode::StreamOne = cfg.mode;

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

    h2stream::open(stream, request).await
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
