//! HTTP(S) upstream-proxy outbound.
//!
//! Forwards through an upstream HTTP proxy with the standard `CONNECT` method
//! (RFC 9110 §9.3.6): the proxy opens a TCP tunnel to the requested target and
//! relays bytes verbatim afterwards, so this is the HTTP analogue of the
//! upstream-SOCKS5 outbound. Two clash/mihomo knobs are supported:
//!
//! * `username` / `password` → a `Proxy-Authorization: Basic` header.
//! * `tls: true` → the `CONNECT` exchange itself runs over TLS (an "HTTPS
//!   proxy"), reusing the shared [`crate::transport`] TLS layer (SNI / ALPN /
//!   `skip-cert-verify` / `client-fingerprint`).
//!
//! Unlike the upstream SOCKS5 outbound this keeps the server as an unresolved
//! hostname and lets the dial path resolve it, matching every other protocol
//! module. There is no UDP relay over an HTTP proxy, so UDP associations are
//! refused up front (see [`crate::outbound::supports_udp_associate`]).
//!
//! Request sent to the proxy (CRLF line endings):
//! ```text
//! CONNECT host:port HTTP/1.1
//! Host: host:port
//! [Proxy-Authorization: Basic base64(user:pass)]
//! <blank line>
//! ```
//! A `2xx` status line means the tunnel is open; anything else is an error.

use std::net::SocketAddr;

use anyhow::{Context, Result, bail};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWriteExt};

use crate::address::TargetAddr;
use crate::config::outbound_opts::ProxyEntry;
use crate::outbound::BoxedStream;
use crate::transport::simple_obfs::base64_encode;
use crate::transport::tls::{ClientFingerprint, TlsClientConfig};
use crate::transport::{self, Security, Transport};

/// Cap on the proxy's `CONNECT` response head (status line + headers) so a
/// misbehaving or malicious upstream cannot make us buffer without bound while
/// scanning for the end-of-headers marker.
const MAX_RESPONSE_HEAD: usize = 8 * 1024;

const CRLF_CRLF: &[u8; 4] = b"\r\n\r\n";

/// Fully-resolved HTTP upstream-proxy outbound parameters.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HttpOutboundConfig {
    pub server: String,
    pub port: u16,
    /// Optional `username:password` for `Proxy-Authorization: Basic`.
    pub auth: Option<(String, String)>,
    /// Security layer wrapping the proxy connection: [`Security::None`] for a
    /// plain HTTP proxy, [`Security::Tls`] for an HTTPS proxy (`tls: true`).
    pub security: Security,
}

impl HttpOutboundConfig {
    /// Build an outbound config from a parsed `http` proxy entry.
    pub fn from_proxy(entry: &ProxyEntry) -> Result<Self> {
        let opts = &entry.options;
        let server = opts
            .server
            .clone()
            .filter(|s| !s.is_empty())
            .context("http: missing server")?;
        let port = opts.port.context("http: missing port")?;

        if let Some(flow) = opts.flow.as_deref().filter(|s| !s.is_empty()) {
            bail!("http: flow {flow:?} not supported on an HTTP proxy");
        }
        if opts.reality_opts.is_some() {
            bail!("http: reality-opts not supported on an HTTP proxy");
        }

        // Credentials are optional, but a password without a username (or vice
        // versa) is a malformed entry rather than an anonymous proxy.
        let auth = match (opts.username.as_deref(), opts.password.as_deref()) {
            (Some(u), Some(p)) if !u.is_empty() => Some((u.to_string(), p.to_string())),
            (None, None) | (Some(""), _) | (None, Some("")) => None,
            (Some(_), None) => bail!("http: username set without password"),
            (None, Some(p)) if !p.is_empty() => bail!("http: password set without username"),
            _ => None,
        };

        // An HTTP proxy is plaintext by default; `tls: true` upgrades the
        // CONNECT exchange to an HTTPS proxy. No other transport applies.
        let security = if opts.tls.unwrap_or(false) {
            let client_fingerprint = match opts.client_fingerprint.as_deref() {
                None | Some("") => None,
                Some(value) => Some(ClientFingerprint::parse(value).map_err(|e| anyhow::anyhow!("http: {e}"))?),
            };
            Security::Tls(TlsClientConfig {
                server_name: opts.servername.clone().or_else(|| opts.sni.clone()),
                alpn: opts.alpn.clone().unwrap_or_default(),
                skip_cert_verify: opts.skip_cert_verify.unwrap_or(false),
                client_fingerprint,
                ech: None,
            })
        } else {
            Security::None
        };

        Ok(Self {
            server,
            port,
            auth,
            security,
        })
    }
}

/// Connect through the HTTP proxy to `target` and return a relay-ready stream.
/// Once the proxy answers the `CONNECT` with a `2xx` the tunnel is transparent,
/// so the stream is handed back as-is for relaying.
pub async fn connect(config: &HttpOutboundConfig, target: &TargetAddr) -> Result<BoxedStream> {
    let mut stream = transport::establish(&config.server, config.port, &config.security, &Transport::Tcp)
        .await
        .context("http: dial upstream proxy")?;
    let request = build_connect_request(target, config.auth.as_ref());
    stream.write_all(&request).await.context("http: send CONNECT request")?;
    read_connect_response(&mut stream)
        .await
        .with_context(|| format!("http: CONNECT to {target}"))?;
    Ok(stream)
}

/// The request-target / `Host` authority for `target`: `host:port`, with an
/// IPv6 literal wrapped in brackets per RFC 3986.
fn authority(target: &TargetAddr) -> String {
    match target {
        TargetAddr::Ip(SocketAddr::V6(addr)) => format!("[{}]:{}", addr.ip(), addr.port()),
        other => format!("{}:{}", other.host(), other.port()),
    }
}

/// Encode the `CONNECT` request (request line, `Host`, optional
/// `Proxy-Authorization`, blank line).
fn build_connect_request(target: &TargetAddr, auth: Option<&(String, String)>) -> Vec<u8> {
    let authority = authority(target);
    let mut req = format!("CONNECT {authority} HTTP/1.1\r\nHost: {authority}\r\n");
    if let Some((user, pass)) = auth {
        let token = base64_encode(format!("{user}:{pass}").as_bytes());
        req.push_str(&format!("Proxy-Authorization: Basic {token}\r\n"));
    }
    req.push_str("\r\n");
    req.into_bytes()
}

/// Read the proxy's response head up to the blank line and require a `2xx`
/// status. Bytes after the header terminator belong to the tunnel and must not
/// be consumed, so this reads one byte at a time once it is near the terminator;
/// in practice a compliant proxy sends no body before relaying, so the loop
/// stops exactly at `\r\n\r\n`.
async fn read_connect_response<R>(stream: &mut R) -> Result<()>
where
    R: AsyncRead + Unpin,
{
    let mut head = Vec::with_capacity(128);
    let mut byte = [0u8; 1];
    loop {
        let n = stream.read(&mut byte).await.context("http: read CONNECT response")?;
        if n == 0 {
            bail!("http: upstream closed before completing CONNECT response");
        }
        head.push(byte[0]);
        if head.ends_with(CRLF_CRLF) {
            break;
        }
        if head.len() > MAX_RESPONSE_HEAD {
            bail!("http: CONNECT response head exceeded {MAX_RESPONSE_HEAD} bytes");
        }
    }
    parse_status(&head)
}

/// Validate the status line of a CONNECT response, accepting any `2xx` code.
fn parse_status(head: &[u8]) -> Result<()> {
    let line_end = head
        .windows(2)
        .position(|w| w == b"\r\n")
        .context("http: malformed CONNECT response (no status line)")?;
    let status_line = std::str::from_utf8(&head[..line_end]).context("http: CONNECT status line is not valid UTF-8")?;
    let mut parts = status_line.splitn(3, ' ');
    let version = parts.next().unwrap_or("");
    if !version.starts_with("HTTP/") {
        bail!("http: unexpected CONNECT status line {status_line:?}");
    }
    let code: u16 = parts
        .next()
        .and_then(|c| c.parse().ok())
        .with_context(|| format!("http: missing status code in {status_line:?}"))?;
    if !(200..300).contains(&code) {
        bail!("http: upstream CONNECT failed: {status_line:?}");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::net::Ipv4Addr;

    use super::*;
    use crate::config::outbound_opts::ProxyEntry;

    fn parse_entry(yaml: &str) -> ProxyEntry {
        serde_yaml_ng::from_str(yaml).expect("parse proxy entry")
    }

    #[test]
    fn domain_connect_request_has_no_auth_by_default() {
        let target = TargetAddr::Domain("example.com".to_string(), 443);
        let req = build_connect_request(&target, None);
        let text = String::from_utf8(req).unwrap();
        assert_eq!(
            text,
            "CONNECT example.com:443 HTTP/1.1\r\nHost: example.com:443\r\n\r\n"
        );
    }

    #[test]
    fn auth_adds_basic_proxy_authorization() {
        let target = TargetAddr::Domain("example.com".to_string(), 80);
        let auth = ("aladdin".to_string(), "opensesame".to_string());
        let req = build_connect_request(&target, Some(&auth));
        let text = String::from_utf8(req).unwrap();
        // RFC 7617 worked example: base64("aladdin:opensesame").
        assert!(
            text.contains("Proxy-Authorization: Basic YWxhZGRpbjpvcGVuc2VzYW1l\r\n"),
            "{text}"
        );
    }

    #[test]
    fn ipv6_target_is_bracketed() {
        let target = TargetAddr::Ip("[2001:db8::1]:8443".parse().unwrap());
        let req = build_connect_request(&target, None);
        let text = String::from_utf8(req).unwrap();
        assert!(text.starts_with("CONNECT [2001:db8::1]:8443 HTTP/1.1\r\n"), "{text}");
    }

    #[test]
    fn ipv4_target_request_line() {
        let target = TargetAddr::Ip(SocketAddr::from((Ipv4Addr::new(1, 2, 3, 4), 8080)));
        let req = build_connect_request(&target, None);
        let text = String::from_utf8(req).unwrap();
        assert!(text.starts_with("CONNECT 1.2.3.4:8080 HTTP/1.1\r\n"), "{text}");
    }

    #[tokio::test]
    async fn reads_200_and_stops_at_blank_line() {
        let response = b"HTTP/1.1 200 Connection established\r\nX-Proxy: t\r\n\r\nTUNNELDATA";
        let mut reader: &[u8] = response;
        read_connect_response(&mut reader).await.unwrap();
        // Only the head is consumed; the tunnel bytes remain for relaying.
        let mut rest = Vec::new();
        reader.read_to_end(&mut rest).await.unwrap();
        assert_eq!(rest, b"TUNNELDATA");
    }

    #[tokio::test]
    async fn rejects_non_2xx_status() {
        let response = b"HTTP/1.1 407 Proxy Authentication Required\r\n\r\n";
        let mut reader: &[u8] = response;
        let err = read_connect_response(&mut reader).await.unwrap_err();
        assert!(err.to_string().contains("407"), "{err}");
    }

    #[tokio::test]
    async fn rejects_eof_before_terminator() {
        let response = b"HTTP/1.1 200 OK\r\n";
        let mut reader: &[u8] = response;
        assert!(read_connect_response(&mut reader).await.is_err());
    }

    #[test]
    fn parses_plain_proxy_without_tls() {
        let cfg =
            HttpOutboundConfig::from_proxy(&parse_entry("name: h\ntype: http\nserver: proxy.example\nport: 8080\n"))
                .unwrap();
        assert_eq!(cfg.server, "proxy.example");
        assert_eq!(cfg.port, 8080);
        assert_eq!(cfg.auth, None);
        assert!(matches!(cfg.security, Security::None));
    }

    #[test]
    fn parses_credentials() {
        let cfg = HttpOutboundConfig::from_proxy(&parse_entry(
            "name: h\ntype: http\nserver: proxy.example\nport: 8080\nusername: bob\npassword: secret\n",
        ))
        .unwrap();
        assert_eq!(cfg.auth, Some(("bob".to_string(), "secret".to_string())));
    }

    #[test]
    fn tls_true_yields_tls_security() {
        let cfg = HttpOutboundConfig::from_proxy(&parse_entry(
            "name: h\ntype: http\nserver: proxy.example\nport: 443\ntls: true\nsni: proxy.example\nskip-cert-verify: true\n",
        ))
        .unwrap();
        match cfg.security {
            Security::Tls(tls) => {
                assert_eq!(tls.server_name.as_deref(), Some("proxy.example"));
                assert!(tls.skip_cert_verify);
            }
            other => panic!("expected TLS security, got {other:?}"),
        }
    }

    #[test]
    fn missing_server_is_rejected() {
        let err = HttpOutboundConfig::from_proxy(&parse_entry("name: h\ntype: http\nport: 8080\n")).unwrap_err();
        assert!(err.to_string().contains("server"), "{err}");
    }

    #[test]
    fn username_without_password_is_rejected() {
        let err = HttpOutboundConfig::from_proxy(&parse_entry(
            "name: h\ntype: http\nserver: proxy.example\nport: 8080\nusername: bob\n",
        ))
        .unwrap_err();
        assert!(err.to_string().contains("password"), "{err}");
    }
}
