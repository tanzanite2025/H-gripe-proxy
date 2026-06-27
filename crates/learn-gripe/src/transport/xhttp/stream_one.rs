//! XHTTP `stream-one` mode: a single full-duplex HTTP/2 `POST` whose request
//! body carries the uplink and whose response body carries the downlink, with
//! **raw** application bytes in both directions (no gRPC/`Hunk` framing). The
//! HTTP/2 byte-stream itself lives in [`crate::transport::h2stream`] and is
//! shared with the `h2` (`PUT`) transport.

use anyhow::{Context, Result};
use http::{Method, Request};
use tokio::io::{AsyncRead, AsyncWrite};

use super::{XhttpTransportConfig, authority_of, normalize_path};
use crate::transport::h2stream::{self, H2ByteStream};

/// Open the XHTTP `stream-one` tunnel over `stream` and return a byte-stream view.
pub(super) async fn connect<S>(
    stream: S,
    server: &str,
    over_tls: bool,
    cfg: &XhttpTransportConfig,
) -> Result<H2ByteStream>
where
    S: AsyncRead + AsyncWrite + Unpin + Send + 'static,
{
    let authority = authority_of(cfg, server);
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
