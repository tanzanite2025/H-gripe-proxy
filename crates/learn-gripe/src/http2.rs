//! HTTP/2 transport (`network: h2`).
//!
//! V2Ray/Xray/mihomo's `h2` transport tunnels the proxy stream over a single
//! full-duplex HTTP/2 `PUT`: the request body carries the uplink and the
//! response body carries the downlink, with **raw** application bytes in both
//! directions. It is byte-stream-identical to XHTTP `stream-one` (which uses
//! `POST`), so both share [`crate::h2stream`]; only the request line differs.
//!
//! `h2` is always run over TLS (the handshake negotiates ALPN `h2`); that
//! requirement is enforced in [`crate::protocols::vless`] at config-build time. Security
//! lives below in [`crate::transport`], so this module never deals with
//! certificates and always speaks the `https` scheme.

use anyhow::{Context, Result};
use http::{Method, Request};
use tokio::io::{AsyncRead, AsyncWrite};

use crate::h2stream::{self, H2ByteStream};

/// Resolved HTTP/2 transport options.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct H2TransportConfig {
    /// Request path (defaults to `/`).
    pub path: String,
    /// `:authority` used for the request; falls back to the dial server.
    pub host: Option<String>,
}

/// Open the `h2` tunnel over `stream` and return a byte-stream view.
pub async fn connect<S>(stream: S, server: &str, cfg: &H2TransportConfig) -> Result<H2ByteStream>
where
    S: AsyncRead + AsyncWrite + Unpin + Send + 'static,
{
    let authority = cfg
        .host
        .clone()
        .filter(|h| !h.is_empty())
        .unwrap_or_else(|| server.to_string());
    let path = normalize_path(&cfg.path);
    let uri = format!("https://{authority}{path}");

    let request = Request::builder()
        .method(Method::PUT)
        .uri(&uri)
        .body(())
        .with_context(|| format!("h2: build request for {uri}"))?;

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
        assert_eq!(normalize_path("/h2"), "/h2");
        assert_eq!(normalize_path("h2"), "/h2");
    }
}
