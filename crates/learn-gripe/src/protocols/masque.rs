//! MASQUE CONNECT-UDP outbound (QUIC / HTTP/3 data plane).
//!
//! MASQUE proxies UDP over an HTTP/3 connection using the **CONNECT-UDP**
//! method (RFC 9298) layered on HTTP Datagrams (RFC 9297):
//!
//! 1. Dial the proxy over QUIC (TLS 1.3, ALPN `h3`) via
//!    [`crate::transport::quic`], advertising HTTP/3 datagram support in the
//!    client SETTINGS.
//! 2. Open one **extended CONNECT** request (RFC 8441) carrying the
//!    `:protocol = connect-udp` pseudo-header and a target-bearing path,
//!    `/.well-known/masque/udp/{target_host}/{target_port}/` (RFC 9298 §3). A
//!    `2xx` response opens the UDP tunnel; the request stream stays open for
//!    the tunnel's lifetime.
//! 3. Relay UDP payloads as **HTTP Datagrams** carried in QUIC datagram frames.
//!    Each datagram is `varint(quarter_stream_id) varint(context_id) payload`
//!    (RFC 9297 §2.1 + RFC 9298 §5); context id `0` carries raw UDP payloads.
//!
//! The HTTP/3 wire codec (QPACK, framing, the extended-CONNECT request) is
//! delegated to the vetted [`h3`]/[`h3_quinn`] crates; the datagrams ride
//! [`quinn`]'s native QUIC datagram frames directly, exactly as the Hysteria2
//! and TUIC UDP relays do.
//!
//! ## Quarter Stream ID
//!
//! An HTTP Datagram is bound to its request stream by the *Quarter Stream ID*
//! (`stream_id / 4`). Each MASQUE UDP session here uses its **own** QUIC
//! connection carrying exactly one request, so that request is the first
//! client-initiated bidirectional stream — stream id `0`, quarter stream id
//! `0` — by RFC 9000 stream numbering. Inbound datagrams for any other quarter
//! stream id or a non-zero context id are not part of this UDP flow and are
//! ignored.
//!
//! Scope: UDP relay (`connect-udp`) over a single QUIC connection per session.
//! CONNECT-UDP is UDP-only by design, so there is no TCP relay (the `connect-ip`
//! and `connect-tcp` drafts are out of scope). `username`/`password` are sent as
//! HTTP `Proxy-Authorization: Basic` credentials when configured. 0-RTT
//! (`reduce-rtt`) is not used: HTTP Datagrams need the established 1-RTT keys.

use anyhow::{Context, Result, bail};
use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64;
use bytes::Bytes;
use h3::client::{RequestStream, SendRequest};
use h3::ext::Protocol;
use quinn::{Connection, Endpoint};

use crate::address::TargetAddr;
use crate::config::outbound_opts::ProxyEntry;
use crate::transport::quic::{self, Congestion, QuicClientParams};

/// Context id `0` carries raw UDP payloads in a CONNECT-UDP flow (RFC 9298 §5).
const CONTEXT_ID_UDP: u64 = 0;
/// The single request per MASQUE connection is stream id `0`, so its quarter
/// stream id (`stream_id / 4`) is `0` (see module docs).
const QUARTER_STREAM_ID: u64 = 0;

/// Fully-resolved MASQUE CONNECT-UDP outbound parameters.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MasqueOutboundConfig {
    pub server: String,
    pub port: u16,
    /// TLS SNI / certificate name and HTTP/3 `:authority` (`sni`, falling back
    /// to `servername`/server).
    pub server_name: String,
    pub alpn: Vec<String>,
    pub skip_cert_verify: bool,
    pub congestion: Congestion,
    /// Optional `Proxy-Authorization: Basic` credentials (`username`/`password`).
    pub username: Option<String>,
    pub password: Option<String>,
}

impl MasqueOutboundConfig {
    /// Build an outbound config from a parsed `masque` proxy entry.
    pub fn from_proxy(entry: &ProxyEntry) -> Result<Self> {
        let opts = &entry.options;
        let server = opts
            .server
            .clone()
            .filter(|s| !s.is_empty())
            .context("masque: missing server")?;
        let port = opts.port.context("masque: missing port")?;

        // SNI precedence: explicit `sni`, then `servername`, then the dial host.
        let server_name = opts
            .sni
            .clone()
            .or_else(|| opts.servername.clone())
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| server.clone());

        // QUIC always carries ALPN; HTTP/3 (and thus CONNECT-UDP) uses "h3".
        let alpn = match &opts.alpn {
            Some(list) if !list.is_empty() => list.clone(),
            _ => vec!["h3".to_string()],
        };

        let congestion = opts
            .congestion_controller
            .as_deref()
            .map(Congestion::parse)
            .unwrap_or(Congestion::Bbr);

        let username = opts.username.clone().filter(|s| !s.is_empty());
        let password = opts.password.clone().filter(|s| !s.is_empty());

        Ok(Self {
            server,
            port,
            server_name,
            alpn,
            skip_cert_verify: opts.skip_cert_verify.unwrap_or(false),
            congestion,
            username,
            password,
        })
    }

    fn quic_params(&self) -> QuicClientParams {
        QuicClientParams {
            server: self.server.clone(),
            port: self.port,
            server_name: self.server_name.clone(),
            alpn: self.alpn.clone(),
            skip_cert_verify: self.skip_cert_verify,
            congestion: self.congestion,
            // MASQUE does not define a packet obfuscation or port-hopping layer,
            // and HTTP Datagrams require 1-RTT keys, so 0-RTT is never attempted.
            obfs: None,
            port_hop: None,
            zero_rtt: false,
        }
    }

    /// `Proxy-Authorization: Basic` header value, when credentials are set. A
    /// password without a username is sent with an empty username (`":pw"`).
    fn proxy_authorization(&self) -> Option<String> {
        if self.username.is_none() && self.password.is_none() {
            return None;
        }
        let user = self.username.as_deref().unwrap_or("");
        let pass = self.password.as_deref().unwrap_or("");
        Some(format!("Basic {}", BASE64.encode(format!("{user}:{pass}"))))
    }
}

/// The `:path` of a CONNECT-UDP request for `target`: the RFC 9298 default
/// template `/.well-known/masque/udp/{target_host}/{target_port}/` with the host
/// and port as percent-encoded path segments (IPv6 hosts are unbracketed).
fn connect_udp_path(target: &TargetAddr) -> String {
    let (host, port) = match target {
        TargetAddr::Ip(addr) => (addr.ip().to_string(), addr.port()),
        TargetAddr::Domain(host, port) => (host.clone(), *port),
    };
    format!("/.well-known/masque/udp/{}/{}/", percent_encode(&host), port)
}

/// Percent-encode a path segment, escaping everything outside the RFC 3986
/// unreserved set so `/`, `:`, and other delimiters cannot break out of the
/// segment.
fn percent_encode(segment: &str) -> String {
    let mut out = String::with_capacity(segment.len());
    for &b in segment.as_bytes() {
        if b.is_ascii_alphanumeric() || matches!(b, b'-' | b'.' | b'_' | b'~') {
            out.push(b as char);
        } else {
            out.push_str(&format!("%{b:02X}"));
        }
    }
    out
}

/// Connect a MASQUE CONNECT-UDP relay session for `target`. Opens an extended
/// CONNECT request over HTTP/3, requiring a `2xx` response, then carries each
/// UDP datagram as an HTTP Datagram in a QUIC datagram frame.
pub async fn connect_udp(config: &MasqueOutboundConfig, target: &TargetAddr) -> Result<MasqueUdp> {
    let quic = quic::connect(&config.quic_params())
        .await
        .context("masque: QUIC connect")?;
    let connection = quic.connection.clone();

    // Advertise HTTP/3 datagram support (RFC 9297) in the client SETTINGS so the
    // proxy accepts the datagrams that carry UDP payloads.
    let h3_conn = h3_quinn::Connection::new(quic.connection);
    let (mut driver, mut send_request) = h3::client::builder()
        .enable_datagram(true)
        .build::<_, _, Bytes>(h3_conn)
        .await
        .context("masque: HTTP/3 handshake")?;
    // The HTTP/3 connection must be driven for the response to arrive; the
    // driver resolves (and the task ends) once the connection closes.
    tokio::spawn(async move {
        let _ = std::future::poll_fn(|cx| driver.poll_close(cx)).await;
    });

    let stream = open_connect_udp(&mut send_request, config, target)
        .await
        .context("masque: CONNECT-UDP")?;

    Ok(MasqueUdp {
        _endpoint: quic.endpoint,
        connection,
        _h3_send: send_request,
        _stream: stream,
    })
}

/// Send the extended CONNECT (`:protocol = connect-udp`) request for `target`
/// and require a `2xx` response. The returned request stream must be kept alive
/// for the tunnel's lifetime.
async fn open_connect_udp(
    send_request: &mut SendRequest<h3_quinn::OpenStreams, Bytes>,
    config: &MasqueOutboundConfig,
    target: &TargetAddr,
) -> Result<RequestStream<h3_quinn::BidiStream<Bytes>, Bytes>> {
    let uri = http::Uri::builder()
        .scheme("https")
        .authority(config.server_name.as_str())
        .path_and_query(connect_udp_path(target))
        .build()
        .context("masque: build request URI")?;

    let mut builder = http::Request::builder()
        .method(http::Method::CONNECT)
        .uri(uri)
        // RFC 9298 §3: a CONNECT-UDP client indicates capsule support.
        .header("capsule-protocol", "?1");
    if let Some(auth) = config.proxy_authorization() {
        builder = builder.header(http::header::PROXY_AUTHORIZATION, auth);
    }
    let mut request = builder.body(()).context("masque: build CONNECT-UDP request")?;
    // The `:protocol` pseudo-header (RFC 8441) selects the connect-udp upgrade.
    request.extensions_mut().insert(Protocol::CONNECT_UDP);

    let mut stream = send_request
        .send_request(request)
        .await
        .context("masque: send CONNECT-UDP request")?;
    let response = stream
        .recv_response()
        .await
        .context("masque: recv CONNECT-UDP response")?;
    let status = response.status();
    if !status.is_success() {
        bail!("masque: proxy refused CONNECT-UDP (HTTP status {})", status.as_u16());
    }
    Ok(stream)
}

/// A MASQUE CONNECT-UDP relay session bound to a single target. UDP payloads
/// to/from the target ride HTTP Datagrams (QUIC datagram frames) on the
/// connection's single request stream.
///
/// The owning [`Endpoint`], [`Connection`], the HTTP/3 [`SendRequest`], and the
/// CONNECT request stream are held so the connection and the UDP tunnel stay
/// alive for the session's lifetime.
pub struct MasqueUdp {
    _endpoint: Endpoint,
    connection: Connection,
    _h3_send: SendRequest<h3_quinn::OpenStreams, Bytes>,
    _stream: RequestStream<h3_quinn::BidiStream<Bytes>, Bytes>,
}

impl MasqueUdp {
    /// Send one UDP datagram to the session target as an HTTP Datagram. Returns
    /// an error if the payload does not fit a single QUIC datagram (HTTP
    /// Datagrams are not fragmented at this layer).
    pub async fn send(&self, payload: &[u8]) -> Result<()> {
        let datagram = encode_http_datagram(payload);
        let max = self
            .connection
            .max_datagram_size()
            .context("masque: peer does not allow QUIC datagrams (UDP relay unavailable)")?;
        if datagram.len() > max {
            bail!(
                "masque: UDP payload too large for one QUIC datagram ({} > {max} bytes)",
                datagram.len()
            );
        }
        self.connection
            .send_datagram(datagram)
            .map_err(|e| anyhow::anyhow!("masque: send UDP datagram: {e}"))
    }

    /// Receive the next UDP datagram from the target, skipping datagrams that do
    /// not belong to this UDP flow (a different quarter stream id or a non-zero
    /// context id, e.g. capsule control traffic).
    pub async fn recv(&self) -> Result<Vec<u8>> {
        loop {
            let datagram = self
                .connection
                .read_datagram()
                .await
                .context("masque: read UDP datagram")?;
            if let Some(payload) = decode_http_datagram(&datagram) {
                return Ok(payload);
            }
        }
    }
}

/// Encode a UDP payload as an HTTP Datagram for this session's request stream:
/// `varint(quarter_stream_id) varint(context_id=0) payload`.
fn encode_http_datagram(payload: &[u8]) -> Bytes {
    let mut buf = Vec::with_capacity(2 + payload.len());
    put_varint(&mut buf, QUARTER_STREAM_ID);
    put_varint(&mut buf, CONTEXT_ID_UDP);
    buf.extend_from_slice(payload);
    Bytes::from(buf)
}

/// Decode an inbound HTTP Datagram, returning the UDP payload when it targets
/// this session's quarter stream id and the UDP context id (`0`). Returns `None`
/// for any other datagram (wrong stream, capsule context, or malformed).
fn decode_http_datagram(data: &[u8]) -> Option<Vec<u8>> {
    let mut pos = 0;
    let quarter = read_varint_slice(data, &mut pos)?;
    if quarter != QUARTER_STREAM_ID {
        return None;
    }
    let context = read_varint_slice(data, &mut pos)?;
    if context != CONTEXT_ID_UDP {
        return None;
    }
    Some(data[pos..].to_vec())
}

/// Append a QUIC variable-length integer (RFC 9000 §16) to `buf`.
fn put_varint(buf: &mut Vec<u8>, value: u64) {
    if value < (1 << 6) {
        buf.push(value as u8);
    } else if value < (1 << 14) {
        buf.extend_from_slice(&((value as u16) | 0x4000).to_be_bytes());
    } else if value < (1 << 30) {
        buf.extend_from_slice(&((value as u32) | 0x8000_0000).to_be_bytes());
    } else {
        buf.extend_from_slice(&(value | 0xC000_0000_0000_0000).to_be_bytes());
    }
}

/// Read a QUIC variable-length integer from `data` at `*pos`, advancing `*pos`.
/// Returns `None` if the buffer is truncated.
fn read_varint_slice(data: &[u8], pos: &mut usize) -> Option<u64> {
    let first = *data.get(*pos)?;
    let len = 1usize << (first >> 6);
    if *pos + len > data.len() {
        return None;
    }
    let mut value = (first & 0x3f) as u64;
    for &b in &data[*pos + 1..*pos + len] {
        value = (value << 8) | b as u64;
    }
    *pos += len;
    Some(value)
}

#[cfg(test)]
mod tests {
    use std::net::SocketAddr;

    use super::*;
    use crate::config::outbound_opts::ProxyEntry;

    fn parse_entry(yaml: &str) -> ProxyEntry {
        serde_yaml_ng::from_str(yaml).expect("parse proxy entry")
    }

    #[test]
    fn parses_minimal_config_with_defaults() {
        let yaml = "name: m\ntype: masque\nserver: proxy.example\nport: 443\n";
        let cfg = MasqueOutboundConfig::from_proxy(&parse_entry(yaml)).unwrap();
        assert_eq!(cfg.server, "proxy.example");
        assert_eq!(cfg.port, 443);
        assert_eq!(cfg.server_name, "proxy.example");
        assert_eq!(cfg.alpn, vec!["h3".to_string()]);
        assert_eq!(cfg.congestion, Congestion::Bbr);
        assert!(!cfg.skip_cert_verify);
        assert!(cfg.proxy_authorization().is_none());
    }

    #[test]
    fn honors_sni_alpn_and_skip_verify() {
        let yaml = "name: m\ntype: masque\nserver: 1.2.3.4\nport: 8443\n\
                    sni: hidden.example\nalpn:\n  - h3\nskip-cert-verify: true\n";
        let cfg = MasqueOutboundConfig::from_proxy(&parse_entry(yaml)).unwrap();
        assert_eq!(cfg.server_name, "hidden.example");
        assert_eq!(cfg.alpn, vec!["h3".to_string()]);
        assert!(cfg.skip_cert_verify);
    }

    #[test]
    fn missing_server_is_rejected() {
        let yaml = "name: m\ntype: masque\nport: 443\n";
        let err = MasqueOutboundConfig::from_proxy(&parse_entry(yaml)).unwrap_err();
        assert!(err.to_string().contains("server"), "got: {err}");
    }

    #[test]
    fn missing_port_is_rejected() {
        let yaml = "name: m\ntype: masque\nserver: proxy.example\n";
        let err = MasqueOutboundConfig::from_proxy(&parse_entry(yaml)).unwrap_err();
        assert!(err.to_string().contains("port"), "got: {err}");
    }

    #[test]
    fn basic_proxy_authorization_is_built_from_credentials() {
        let yaml = "name: m\ntype: masque\nserver: proxy.example\nport: 443\n\
                    username: alice\npassword: s3cret\n";
        let cfg = MasqueOutboundConfig::from_proxy(&parse_entry(yaml)).unwrap();
        // base64("alice:s3cret")
        assert_eq!(cfg.proxy_authorization().as_deref(), Some("Basic YWxpY2U6czNjcmV0"));
    }

    #[test]
    fn connect_udp_path_uses_well_known_template() {
        let target = TargetAddr::Domain("example.com".to_string(), 443);
        assert_eq!(connect_udp_path(&target), "/.well-known/masque/udp/example.com/443/");

        let ip = TargetAddr::Ip("93.184.216.34:53".parse::<SocketAddr>().unwrap());
        assert_eq!(connect_udp_path(&ip), "/.well-known/masque/udp/93.184.216.34/53/");
    }

    #[test]
    fn percent_encoding_escapes_delimiters() {
        // A `/` in a (hypothetical) host segment must be escaped so it cannot
        // break out of the path segment.
        assert_eq!(percent_encode("a/b:c"), "a%2Fb%3Ac");
        assert_eq!(percent_encode("plain-host.example"), "plain-host.example");
    }

    #[test]
    fn http_datagram_round_trips_udp_payload() {
        let datagram = encode_http_datagram(b"udp ping");
        // quarter stream id (0) + context id (0) + payload.
        assert_eq!(&datagram[..2], &[0x00, 0x00]);
        assert_eq!(decode_http_datagram(&datagram).as_deref(), Some(&b"udp ping"[..]));
    }

    #[test]
    fn decode_skips_other_streams_and_contexts() {
        // Wrong quarter stream id (1) -> not our flow.
        assert_eq!(decode_http_datagram(&[0x01, 0x00, 0xAA]), None);
        // Non-zero context id (1) -> capsule/other context, not raw UDP.
        assert_eq!(decode_http_datagram(&[0x00, 0x01, 0xAA]), None);
        // Truncated (missing context id).
        assert_eq!(decode_http_datagram(&[0x00]), None);
    }
}
