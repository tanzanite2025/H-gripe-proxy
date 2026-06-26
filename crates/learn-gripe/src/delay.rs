//! In-process proxy delay (RTT) measurement.
//!
//! Replaces the Mihomo controller `/proxies/{name}/delay` and
//! `/group/{name}/delay` calls: instead of asking an external Go process to
//! probe a node, the kernel dials the test URL *through the outbound itself*
//! and times how long the full path (proxy handshake + request + first
//! response byte) takes. The embedder builds one [`OutboundMode`] per node it
//! wants to measure and calls [`measure_delay`]; group fan-out (which nodes,
//! concurrency) stays in the control plane, which already knows how to turn a
//! group into its member outbounds.

use std::time::{Duration, Instant};

use anyhow::{Context, Result, anyhow, bail};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

use crate::address::TargetAddr;
use crate::config::OutboundMode;
use crate::outbound::{self, BoxedStream};
use crate::tls::{self, TlsClientConfig};

/// Measure the round-trip delay of dialing `test_url` through `mode`, capped at
/// `timeout`.
///
/// The clock starts before the outbound is dialed, so the result includes the
/// proxy's own handshake (TLS / REALITY / protocol framing) exactly as a real
/// connection would pay it — the same end-to-end latency clash/mihomo report.
/// The probe issues a minimal HTTP/1.1 `GET` and stops at the first response
/// byte (the status line), matching an HTTP client that returns once response
/// headers arrive.
///
/// Returns the delay in milliseconds. A timeout, a refused/failed dial, or a
/// non-HTTP response is an `Err`; the control plane maps that to the UI's
/// "timeout" sentinel rather than surfacing it as a hard failure.
pub async fn measure_delay(mode: &OutboundMode, test_url: &str, timeout: Duration) -> Result<u32> {
    let probe = ProbeTarget::parse(test_url)?;
    let target = TargetAddr::Domain(probe.host.clone(), probe.port);

    let elapsed = tokio::time::timeout(timeout, async {
        let start = Instant::now();
        let stream = outbound::connect(mode, &target, None)
            .await
            .with_context(|| format!("dial {test_url} through outbound"))?;
        run_http_probe(stream, &probe).await?;
        Ok::<Duration, anyhow::Error>(start.elapsed())
    })
    .await
    .map_err(|_| anyhow!("delay probe to {test_url} timed out after {}ms", timeout.as_millis()))??;

    // Clamp into the u32 millisecond range the clash delay API uses. A probe
    // capped by `timeout` can never realistically overflow, but saturate rather
    // than wrap on the off chance.
    Ok(u32::try_from(elapsed.as_millis()).unwrap_or(u32::MAX))
}

/// Run the HTTP(S) probe over an established outbound stream, wrapping it in a
/// TLS client handshake first for `https` URLs.
async fn run_http_probe(stream: BoxedStream, probe: &ProbeTarget) -> Result<()> {
    if probe.tls {
        let tls_config = TlsClientConfig {
            server_name: Some(probe.host.clone()),
            alpn: vec!["http/1.1".to_string()],
            skip_cert_verify: false,
            client_fingerprint: None,
        };
        let tls_stream = tls::connect(&tls_config, &probe.host, stream)
            .await
            .with_context(|| format!("TLS handshake with {}", probe.host))?;
        http_request(tls_stream, probe).await
    } else {
        http_request(stream, probe).await
    }
}

/// Send a minimal `GET` and confirm the peer answers with an HTTP status line.
async fn http_request<S>(mut stream: S, probe: &ProbeTarget) -> Result<()>
where
    S: AsyncRead + AsyncWrite + Unpin,
{
    let request = format!(
        "GET {} HTTP/1.1\r\nHost: {}\r\nUser-Agent: learn-gripe/delay\r\nAccept: */*\r\nConnection: close\r\n\r\n",
        probe.path,
        probe.host_header(),
    );
    stream
        .write_all(request.as_bytes())
        .await
        .context("write delay probe request")?;
    stream.flush().await.context("flush delay probe request")?;

    let mut buf = [0u8; 64];
    let n = stream.read(&mut buf).await.context("read delay probe response")?;
    if n == 0 {
        bail!("delay probe peer closed without responding");
    }
    if !buf[..n].starts_with(b"HTTP/") {
        bail!("delay probe got a non-HTTP response");
    }
    Ok(())
}

/// The pieces of a delay test URL the probe needs: scheme, host, port and the
/// request-target. Kept minimal (no query/userinfo handling beyond stripping)
/// so the kernel does not pull in a full URL parser for this one job.
#[derive(Debug, PartialEq, Eq)]
struct ProbeTarget {
    tls: bool,
    host: String,
    port: u16,
    /// Request-target (path plus any query), always starting with `/`.
    path: String,
}

impl ProbeTarget {
    fn parse(url: &str) -> Result<Self> {
        let (scheme, rest) = url.split_once("://").context("delay test url missing scheme")?;
        let tls = match scheme.to_ascii_lowercase().as_str() {
            "http" => false,
            "https" => true,
            other => bail!("unsupported delay test url scheme {other:?}"),
        };

        let (authority, path) = match rest.find('/') {
            Some(idx) => (&rest[..idx], &rest[idx..]),
            None => (rest, "/"),
        };
        // Drop optional `userinfo@` so only the host[:port] is parsed.
        let authority = authority.rsplit_once('@').map_or(authority, |(_, host)| host);
        if authority.is_empty() {
            bail!("delay test url missing host");
        }
        let (host, port) = split_host_port(authority, tls)?;
        let path = if path.is_empty() {
            "/".to_string()
        } else {
            path.to_string()
        };
        Ok(Self { tls, host, port, path })
    }

    /// `Host` header value: the bare host when the port is the scheme default,
    /// otherwise `host:port`.
    fn host_header(&self) -> String {
        let default = if self.tls { 443 } else { 80 };
        if self.port == default {
            self.host.clone()
        } else {
            format!("{}:{}", self.host, self.port)
        }
    }
}

/// Split an authority into `(host, port)`, defaulting the port to the scheme's
/// (80/443). Handles bracketed IPv6 literals (`[::1]:8080`).
fn split_host_port(authority: &str, tls: bool) -> Result<(String, u16)> {
    let default_port = if tls { 443 } else { 80 };
    if let Some(rest) = authority.strip_prefix('[') {
        let (host, after) = rest
            .split_once(']')
            .context("delay test url has an unterminated IPv6 host")?;
        let port = match after.strip_prefix(':') {
            Some(p) => p.parse().context("invalid port in delay test url")?,
            None if after.is_empty() => default_port,
            None => bail!("invalid IPv6 authority in delay test url"),
        };
        return Ok((host.to_string(), port));
    }
    match authority.rsplit_once(':') {
        Some((host, port)) if !host.is_empty() => Ok((
            host.to_string(),
            port.parse().context("invalid port in delay test url")?,
        )),
        _ => Ok((authority.to_string(), default_port)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_plain_http_url() {
        let p = ProbeTarget::parse("http://www.gstatic.com/generate_204").unwrap();
        assert_eq!(
            p,
            ProbeTarget {
                tls: false,
                host: "www.gstatic.com".to_string(),
                port: 80,
                path: "/generate_204".to_string(),
            }
        );
        assert_eq!(p.host_header(), "www.gstatic.com");
    }

    #[test]
    fn parses_https_url_with_default_port() {
        let p = ProbeTarget::parse("https://www.google.com/").unwrap();
        assert!(p.tls);
        assert_eq!(p.port, 443);
        assert_eq!(p.path, "/");
        assert_eq!(p.host_header(), "www.google.com");
    }

    #[test]
    fn defaults_path_when_url_has_none() {
        let p = ProbeTarget::parse("http://example.com").unwrap();
        assert_eq!(p.path, "/");
        assert_eq!(p.port, 80);
    }

    #[test]
    fn explicit_port_is_kept_and_reflected_in_host_header() {
        let p = ProbeTarget::parse("http://example.com:8080/ok").unwrap();
        assert_eq!(p.port, 8080);
        assert_eq!(p.host_header(), "example.com:8080");
    }

    #[test]
    fn keeps_query_in_request_target() {
        let p = ProbeTarget::parse("https://example.com/path?a=1&b=2").unwrap();
        assert_eq!(p.path, "/path?a=1&b=2");
    }

    #[test]
    fn parses_bracketed_ipv6_authority() {
        let p = ProbeTarget::parse("http://[::1]:8080/x").unwrap();
        assert_eq!(p.host, "::1");
        assert_eq!(p.port, 8080);

        let p = ProbeTarget::parse("https://[2606:4700::1111]/").unwrap();
        assert_eq!(p.host, "2606:4700::1111");
        assert_eq!(p.port, 443);
    }

    #[test]
    fn strips_userinfo_from_authority() {
        let p = ProbeTarget::parse("http://user:pass@example.com:81/p").unwrap();
        assert_eq!(p.host, "example.com");
        assert_eq!(p.port, 81);
    }

    #[test]
    fn rejects_missing_or_unknown_scheme() {
        assert!(ProbeTarget::parse("www.gstatic.com/generate_204").is_err());
        assert!(ProbeTarget::parse("ftp://example.com/").is_err());
    }
}
