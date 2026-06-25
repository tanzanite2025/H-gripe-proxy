//! Minimal HTTP/HTTPS proxy inbound, served on the same listener as SOCKS5 so
//! the app's system-proxy integration (which speaks HTTP) keeps working.
//!
//! Two request shapes are handled, both per RFC 7230 forward-proxy semantics:
//!
//! - `CONNECT host:port` — reply `200 Connection established` then tunnel the
//!   raw bytes, identical to a SOCKS5 `CONNECT` (this is how HTTPS rides an HTTP
//!   proxy).
//! - A plain request with an absolute-form target (`GET http://host/path ...`)
//!   — dial the origin, rewrite the request line to origin-form (`GET /path`),
//!   drop hop-by-hop proxy headers, forward the head and relay the rest.
//!
//! Only the head is parsed in-crate; the bodies/responses flow through
//! `tokio::io::copy_bidirectional`. One forward target per connection: after the
//! first request the connection is bridged to that origin, which covers the
//! common keep-alive-to-one-host case browsers use.

use crate::address::TargetAddr;
use crate::config::GripeConfig;
use crate::conntrack::{ConnMeta, ConnNetwork, ConnRegistry, relay_tracked};
use crate::dns::{FakeIpPool, unmap_fake_ip};
use crate::outbound;
use anyhow::{Result, anyhow, bail};
use std::net::IpAddr;
use std::sync::atomic::Ordering;
use std::sync::{Arc, Mutex};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

/// Cap on the request head we buffer before dialing the origin. Headers larger
/// than this are almost certainly hostile or malformed.
const MAX_HEAD_LEN: usize = 64 * 1024;

const RESP_CONNECT_OK: &[u8] = b"HTTP/1.1 200 Connection established\r\n\r\n";
const RESP_BAD_GATEWAY: &[u8] = b"HTTP/1.1 502 Bad Gateway\r\n\r\n";

/// A parsed forward-proxy request, ready to act on once the target is resolved.
#[derive(Debug, PartialEq, Eq)]
enum Request {
    /// `CONNECT host:port` — tunnel raw bytes after the success reply.
    Connect { host: String, port: u16 },
    /// A plain absolute-form request; `head` is already rewritten to the
    /// origin-form bytes to send upstream.
    Plain { host: String, port: u16, head: Vec<u8> },
}

/// Handle an HTTP proxy connection on `inbound`. The first byte has already been
/// peeked by the dispatcher and is known not to be the SOCKS5 version, so the
/// stream still starts at the request line.
pub(crate) async fn handle(
    mut inbound: TcpStream,
    config: &GripeConfig,
    fake_ip: Option<&Arc<Mutex<FakeIpPool>>>,
    registry: &Arc<ConnRegistry>,
) -> Result<()> {
    let (head, rest) = read_head(&mut inbound).await?;
    let request = parse_request(&head)?;

    let (host, port, upstream_head) = match request {
        Request::Connect { host, port } => (host, port, None),
        Request::Plain { host, port, head } => (host, port, Some(head)),
    };

    let mut target = make_target(&host, port);
    if let Some(pool) = fake_ip {
        target = unmap_fake_ip(pool, target);
    }

    let mut outbound = match outbound::connect(&config.outbound, &target).await {
        Ok(stream) => stream,
        Err(err) => {
            let _ = inbound.write_all(RESP_BAD_GATEWAY).await;
            return Err(err);
        }
    };

    match upstream_head {
        // CONNECT: acknowledge to the client, then bridge raw bytes.
        None => inbound.write_all(RESP_CONNECT_OK).await?,
        // Plain proxy: replay the rewritten head to the origin first.
        Some(upstream_head) => outbound.write_all(&upstream_head).await?,
    }

    // Anything already read past the head (early request body or pipelined
    // bytes) belongs to the origin.
    if !rest.is_empty() {
        outbound.write_all(&rest).await?;
    }

    let meta = ConnMeta::for_target(
        ConnNetwork::Tcp,
        inbound.peer_addr().ok(),
        inbound.local_addr().ok(),
        &config.outbound,
        &target,
    );
    let conn = registry.register(meta);
    // Bytes consumed past the head were already forwarded to the origin; count
    // them as upload so the table reflects the early request body.
    if !rest.is_empty() {
        conn.upload().fetch_add(rest.len() as u64, Ordering::Relaxed);
    }

    relay_tracked(inbound, outbound, &conn)
        .await
        .map_err(|err| anyhow!("relay to {target}: {err}"))?;
    Ok(())
}

/// Read the request head up to and including the `\r\n\r\n` terminator.
/// Returns `(head, rest)` where `rest` is any bytes read past the terminator.
async fn read_head<S>(stream: &mut S) -> Result<(Vec<u8>, Vec<u8>)>
where
    S: AsyncRead + Unpin,
{
    let mut buf = Vec::with_capacity(1024);
    let mut chunk = [0u8; 1024];
    let mut scanned = 0usize;
    loop {
        // Resume scanning a few bytes back so a terminator split across reads is
        // still found.
        if let Some(pos) = find_crlf_crlf(&buf, scanned.saturating_sub(3)) {
            let head = buf[..pos + 4].to_vec();
            let rest = buf[pos + 4..].to_vec();
            return Ok((head, rest));
        }
        scanned = buf.len();
        if buf.len() > MAX_HEAD_LEN {
            bail!("HTTP request head exceeds {MAX_HEAD_LEN} bytes");
        }
        let n = stream.read(&mut chunk).await?;
        if n == 0 {
            bail!("connection closed before the HTTP request head completed");
        }
        buf.extend_from_slice(&chunk[..n]);
    }
}

fn find_crlf_crlf(buf: &[u8], from: usize) -> Option<usize> {
    if buf.len() < 4 {
        return None;
    }
    (from..=buf.len() - 4).find(|&i| &buf[i..i + 4] == b"\r\n\r\n")
}

/// Parse a forward-proxy request head into a [`Request`]. Rewrites plain
/// absolute-form requests to origin-form upstream bytes.
fn parse_request(head: &[u8]) -> Result<Request> {
    let text = std::str::from_utf8(head).map_err(|_| anyhow!("HTTP request head is not valid UTF-8"))?;
    let mut lines = text.split("\r\n");
    let request_line = lines.next().ok_or_else(|| anyhow!("empty HTTP request"))?;

    let mut parts = request_line.split(' ');
    let method = parts.next().ok_or_else(|| anyhow!("missing HTTP method"))?;
    let target = parts.next().ok_or_else(|| anyhow!("missing HTTP request target"))?;
    let version = parts.next().ok_or_else(|| anyhow!("missing HTTP version"))?;

    if method.eq_ignore_ascii_case("CONNECT") {
        let (host, port) = split_host_port(target, 443)?;
        return Ok(Request::Connect { host, port });
    }

    // Plain proxy request: the target is an absolute URI we dial directly.
    let authority_and_path = target
        .strip_prefix("http://")
        .ok_or_else(|| anyhow!("unsupported HTTP proxy target (expected absolute http:// URI): {target}"))?;
    let (authority, path) = match authority_and_path.find('/') {
        Some(idx) => (&authority_and_path[..idx], &authority_and_path[idx..]),
        None => (authority_and_path, "/"),
    };
    let (host, port) = split_host_port(authority, 80)?;

    let head = rewrite_origin_form(method, path, version, lines);
    Ok(Request::Plain { host, port, head })
}

/// Rebuild the request head in origin-form for the upstream origin: rewrite the
/// request line to a relative path and drop the hop-by-hop `Proxy-Connection`
/// header. Remaining headers are forwarded verbatim.
fn rewrite_origin_form<'a>(method: &str, path: &str, version: &str, headers: impl Iterator<Item = &'a str>) -> Vec<u8> {
    let mut out = format!("{method} {path} {version}\r\n");
    for header in headers {
        if header.is_empty() {
            break;
        }
        let name = header.split(':').next().unwrap_or("");
        if name.eq_ignore_ascii_case("proxy-connection") {
            continue;
        }
        out.push_str(header);
        out.push_str("\r\n");
    }
    out.push_str("\r\n");
    out.into_bytes()
}

/// Split an `authority` (`host`, `host:port`, `[v6]`, or `[v6]:port`) into host
/// and port, falling back to `default_port` when no port is present.
fn split_host_port(authority: &str, default_port: u16) -> Result<(String, u16)> {
    if authority.is_empty() {
        bail!("empty authority");
    }
    if let Some(after_bracket) = authority.strip_prefix('[') {
        let end = after_bracket
            .find(']')
            .ok_or_else(|| anyhow!("unterminated IPv6 literal: {authority}"))?;
        let host = &after_bracket[..end];
        let tail = &after_bracket[end + 1..];
        let port = match tail.strip_prefix(':') {
            Some(p) => p.parse().map_err(|_| anyhow!("invalid port: {p}"))?,
            None => default_port,
        };
        return Ok((host.to_string(), port));
    }
    match authority.rsplit_once(':') {
        Some((host, port)) if !port.is_empty() && port.bytes().all(|b| b.is_ascii_digit()) => Ok((
            host.to_string(),
            port.parse().map_err(|_| anyhow!("invalid port: {port}"))?,
        )),
        _ => Ok((authority.to_string(), default_port)),
    }
}

/// Build a [`TargetAddr`] from a host string, keeping a domain unresolved so the
/// outbound decides how to resolve it.
fn make_target(host: &str, port: u16) -> TargetAddr {
    match host.parse::<IpAddr>() {
        Ok(ip) => TargetAddr::Ip(std::net::SocketAddr::new(ip, port)),
        Err(_) => TargetAddr::Domain(host.to_string(), port),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::{Ipv4Addr, SocketAddr};

    #[test]
    fn parses_connect_with_explicit_port() {
        let req = parse_request(b"CONNECT example.com:8443 HTTP/1.1\r\nHost: example.com:8443\r\n\r\n").unwrap();
        assert_eq!(
            req,
            Request::Connect {
                host: "example.com".to_string(),
                port: 8443,
            }
        );
    }

    #[test]
    fn connect_defaults_to_https_port() {
        let req = parse_request(b"CONNECT example.com HTTP/1.1\r\n\r\n").unwrap();
        assert_eq!(
            req,
            Request::Connect {
                host: "example.com".to_string(),
                port: 443,
            }
        );
    }

    #[test]
    fn connect_method_is_case_insensitive() {
        let req = parse_request(b"connect example.com:443 HTTP/1.1\r\n\r\n").unwrap();
        assert!(matches!(req, Request::Connect { .. }));
    }

    #[test]
    fn parses_connect_ipv6_literal() {
        let req = parse_request(b"CONNECT [2001:db8::1]:443 HTTP/1.1\r\n\r\n").unwrap();
        assert_eq!(
            req,
            Request::Connect {
                host: "2001:db8::1".to_string(),
                port: 443,
            }
        );
    }

    #[test]
    fn rewrites_plain_request_to_origin_form() {
        let raw = b"GET http://example.com/path?q=1 HTTP/1.1\r\nHost: example.com\r\nProxy-Connection: keep-alive\r\nUser-Agent: t\r\n\r\n";
        let req = parse_request(raw).unwrap();
        let Request::Plain { host, port, head } = req else {
            panic!("expected a plain request");
        };
        assert_eq!(host, "example.com");
        assert_eq!(port, 80);
        let head = String::from_utf8(head).unwrap();
        // Origin-form request line, proxy header stripped, others preserved.
        assert_eq!(
            head,
            "GET /path?q=1 HTTP/1.1\r\nHost: example.com\r\nUser-Agent: t\r\n\r\n"
        );
    }

    #[test]
    fn plain_request_without_path_uses_root() {
        let req = parse_request(b"GET http://example.com HTTP/1.1\r\nHost: example.com\r\n\r\n").unwrap();
        let Request::Plain { head, .. } = req else {
            panic!("expected a plain request");
        };
        assert!(String::from_utf8(head).unwrap().starts_with("GET / HTTP/1.1\r\n"));
    }

    #[test]
    fn plain_request_with_explicit_port() {
        let req = parse_request(b"GET http://example.com:8080/x HTTP/1.1\r\nHost: example.com:8080\r\n\r\n").unwrap();
        let Request::Plain { host, port, .. } = req else {
            panic!("expected a plain request");
        };
        assert_eq!((host.as_str(), port), ("example.com", 8080));
    }

    #[test]
    fn rejects_non_absolute_plain_target() {
        assert!(parse_request(b"GET /relative HTTP/1.1\r\nHost: example.com\r\n\r\n").is_err());
    }

    #[test]
    fn make_target_distinguishes_ip_and_domain() {
        assert_eq!(
            make_target("127.0.0.1", 80),
            TargetAddr::Ip(SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 80))
        );
        assert_eq!(
            make_target("example.com", 443),
            TargetAddr::Domain("example.com".to_string(), 443)
        );
    }

    #[tokio::test]
    async fn read_head_splits_terminator_across_reads() {
        use tokio::io::AsyncWriteExt;
        let (mut client, server) = tokio::io::duplex(64);
        tokio::spawn(async move {
            client.write_all(b"GET / HTTP/1.1\r\n").await.unwrap();
            client.write_all(b"Host: x\r\n\r").await.unwrap();
            client.write_all(b"\nBODYBYTES").await.unwrap();
        });
        let mut server = server;
        let (head, rest) = read_head(&mut server).await.unwrap();
        assert_eq!(head, b"GET / HTTP/1.1\r\nHost: x\r\n\r\n");
        assert_eq!(rest, b"BODYBYTES");
    }
}
