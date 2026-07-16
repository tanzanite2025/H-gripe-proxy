//! TrustTunnel outbound (HTTP/2 `CONNECT` tunnel).
//!
//! TrustTunnel multiplexes proxied streams over a single HTTP/2 connection to
//! the proxy server (ALPN `h2`, always over TLS). Each proxied target is one
//! HTTP/2 `CONNECT` request whose `:authority` names the *target* (not the
//! proxy): the request body carries the uplink and the response body carries
//! the downlink, exactly like the shared [`crate::transport::h2stream`] adapter.
//! A `200` response means the tunnel is open.
//!
//! Two knobs shape the request:
//! * `username` / `password` → a `Proxy-Authorization: Basic base64(user:pass)`
//!   header (always sent, matching the upstream client, so an anonymous proxy
//!   sees `Basic base64(":")`).
//! * TLS: SNI / ALPN / `skip-cert-verify` / `client-fingerprint` / ECH reuse the
//!   shared [`crate::transport`] TLS layer. ALPN always offers `h2`.
//!
//! UDP rides a second HTTP/2 `CONNECT` whose `:authority` is the magic
//! `_udp2` token; datagrams are then carried with a length-prefixed binary
//! framing that names each packet's peer address (see [`encode_client_packet`]
//! / [`decode_server_packet`]). Because a datagram must name an IP peer, a
//! domain target is resolved to an address before the association opens.
//!
//! The `quic` (HTTP/3) carrier and client-certificate auth are not implemented
//! and are rejected at config time rather than silently mis-dialed.

use std::net::{IpAddr, SocketAddr};

use anyhow::{Context, Result, anyhow, bail};
use http::header::{PROXY_AUTHORIZATION, USER_AGENT};
use http::uri::{Authority, Parts, Uri};
use http::{Method, Request, StatusCode};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWriteExt, ReadHalf, WriteHalf};
use tokio::sync::Mutex;

use crate::address::TargetAddr;
use crate::config::outbound_opts::ProxyEntry;
use crate::outbound::BoxedStream;
use crate::transport::h2stream::{self, H2ByteStream};
use crate::transport::simple_obfs::base64_encode;
use crate::transport::tls::{ClientFingerprint, TlsClientConfig};
use crate::transport::{self, Security, Transport, build_ech_config};

/// `:authority` naming the UDP multiplexing tunnel (mihomo `UDPMagicAddress`).
const UDP_MAGIC_AUTHORITY: &str = "_udp2";

/// `User-Agent` for the TCP `CONNECT` (`<platform> <app>/<version>`), matching
/// the upstream format. It is cosmetic — the server routes on `:authority`.
const TCP_USER_AGENT: &str = "windows learn-gripe/1";

/// `User-Agent` for the UDP `CONNECT` (`<platform> _udp2`); also cosmetic.
const UDP_USER_AGENT: &str = "windows _udp2";

/// The app-name tag written into each client UDP packet header. The server
/// reads its length and skips it, so an empty tag is valid and keeps the header
/// minimal.
const UDP_APP_NAME: &[u8] = b"";

/// Fixed per-packet UDP header overhead *after* the 4-byte length prefix on the
/// server → client path: source address (16) + port (2) + a zeroed local
/// address (16) + port (2).
const UDP_SERVER_OVERHEAD: usize = 16 + 2 + 16 + 2;

/// Fully-resolved TrustTunnel outbound parameters.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrustTunnelOutboundConfig {
    pub server: String,
    pub port: u16,
    /// `Proxy-Authorization: Basic` credentials, always sent (empty strings for
    /// an anonymous proxy, matching the upstream client).
    pub username: String,
    pub password: String,
    /// TLS layer wrapping the HTTP/2 connection (ALPN always offers `h2`).
    pub security: Security,
    /// Whether the proxy advertises UDP relay (`udp: true`).
    pub udp: bool,
}

impl TrustTunnelOutboundConfig {
    /// Build an outbound config from a parsed `trusttunnel` proxy entry.
    pub fn from_proxy(entry: &ProxyEntry) -> Result<Self> {
        let opts = &entry.options;
        let server = opts
            .server
            .clone()
            .filter(|s| !s.is_empty())
            .context("trusttunnel: missing server")?;
        let port = opts.port.context("trusttunnel: missing port")?;

        if opts.quic.unwrap_or(false) {
            bail!("trusttunnel: quic (HTTP/3) carrier not implemented yet; only the h2 tunnel is supported");
        }
        if opts.reality_opts.is_some() {
            bail!("trusttunnel: reality-opts not supported");
        }
        if let Some(flow) = opts.flow.as_deref().filter(|s| !s.is_empty()) {
            bail!("trusttunnel: flow {flow:?} not supported");
        }
        if opts.certificate.as_deref().is_some_and(|s| !s.is_empty())
            || opts.private_key.as_deref().is_some_and(|s| !s.is_empty())
        {
            bail!("trusttunnel: client-certificate auth (certificate/private-key) not implemented yet");
        }

        // The tunnel runs HTTP/2, so ALPN must offer `h2`; default to it when
        // unset, and reject an explicit list that omits it rather than dialing a
        // connection the server will not accept.
        let mut alpn: Vec<String> = opts
            .alpn
            .clone()
            .unwrap_or_default()
            .into_iter()
            .filter(|p| !p.is_empty())
            .collect();
        if alpn.is_empty() {
            alpn.push("h2".to_string());
        } else if !alpn.iter().any(|p| p == "h2") {
            bail!("trusttunnel: alpn must include \"h2\"");
        }

        let client_fingerprint = match opts.client_fingerprint.as_deref() {
            None | Some("") => None,
            Some(value) => Some(ClientFingerprint::parse(value).map_err(|e| anyhow!("trusttunnel: {e}"))?),
        };

        let security = Security::Tls(TlsClientConfig {
            server_name: opts.servername.clone().or_else(|| opts.sni.clone()),
            alpn,
            skip_cert_verify: opts.skip_cert_verify.unwrap_or(false),
            client_fingerprint,
            ech: build_ech_config(opts.ech_opts.as_ref(), "trusttunnel")?,
        });

        Ok(Self {
            server,
            port,
            username: opts.username.clone().unwrap_or_default(),
            password: opts.password.clone().unwrap_or_default(),
            security,
            udp: opts.udp.unwrap_or(false),
        })
    }
}

/// Connect through the proxy to `target` and return a relay-ready stream.
pub async fn connect(config: &TrustTunnelOutboundConfig, target: &TargetAddr) -> Result<BoxedStream> {
    let authority = authority(target);
    let stream = open_tunnel(config, &authority, TCP_USER_AGENT)
        .await
        .with_context(|| format!("trusttunnel: CONNECT to {target}"))?;
    Ok(Box::new(stream))
}

/// The `:authority` for `target`: `host:port`, with an IPv6 literal bracketed
/// per RFC 3986.
fn authority(target: &TargetAddr) -> String {
    match target {
        TargetAddr::Ip(SocketAddr::V6(addr)) => format!("[{}]:{}", addr.ip(), addr.port()),
        other => format!("{}:{}", other.host(), other.port()),
    }
}

/// `Proxy-Authorization: Basic` value for `user:pass`.
fn basic_auth(user: &str, pass: &str) -> String {
    format!("Basic {}", base64_encode(format!("{user}:{pass}").as_bytes()))
}

/// Build an authority-only request URI (`:authority` with no `:scheme` /
/// `:path`), which RFC 9113 §8.5 requires for an HTTP/2 `CONNECT`.
fn connect_uri(authority: &str) -> Result<Uri> {
    let authority: Authority = authority
        .parse()
        .with_context(|| format!("trusttunnel: invalid CONNECT authority {authority:?}"))?;
    let mut parts = Parts::default();
    parts.authority = Some(authority);
    Uri::from_parts(parts).context("trusttunnel: build CONNECT uri")
}

/// Assemble the `CONNECT` request for `authority` (`Proxy-Authorization` +
/// `User-Agent`).
fn connect_request(authority: &str, user_agent: &str, auth: &str) -> Result<Request<()>> {
    Request::builder()
        .method(Method::CONNECT)
        .uri(connect_uri(authority)?)
        .header(USER_AGENT, user_agent)
        .header(PROXY_AUTHORIZATION, auth)
        .body(())
        .context("trusttunnel: build CONNECT request")
}

/// Dial the proxy, run the TLS + HTTP/2 handshake and open a `CONNECT` tunnel to
/// `authority`. A `200` response means the tunnel is open; the returned stream
/// carries raw bytes (uplink on the request body, downlink on the response).
async fn open_tunnel(config: &TrustTunnelOutboundConfig, authority: &str, user_agent: &str) -> Result<H2ByteStream> {
    let transport_stream = transport::establish(&config.server, config.port, &config.security, &Transport::Tcp)
        .await
        .context("trusttunnel: dial proxy")?;
    let auth = basic_auth(&config.username, &config.password);
    let request = connect_request(authority, user_agent, &auth)?;
    let (stream, status) = h2stream::open_with_status(transport_stream, request)
        .await
        .context("trusttunnel: CONNECT handshake")?;
    if status != StatusCode::OK {
        bail!("trusttunnel: unexpected CONNECT status {status}");
    }
    Ok(stream)
}

/// A TrustTunnel UDP association: one `_udp2` `CONNECT` carrying datagrams to a
/// single resolved peer, matching the other UDP egresses' `connect` / `send` /
/// `recv` shape. The stream is split so `send` and `recv` can run concurrently
/// in the egress `select!`.
pub struct TrustTunnelUdpAssoc {
    /// The resolved peer named in every datagram sent on this association.
    dest: SocketAddr,
    write: Mutex<WriteHalf<H2ByteStream>>,
    read: Mutex<ReadHalf<H2ByteStream>>,
}

impl TrustTunnelUdpAssoc {
    /// Open a `_udp2` association for datagrams destined to `target`. The peer
    /// must be an IP; a domain target is resolved first (the wire framing names
    /// a literal address).
    pub async fn connect(config: &TrustTunnelOutboundConfig, target: &TargetAddr) -> Result<Self> {
        let dest = resolve_udp_target(target).await?;
        let stream = open_tunnel(config, UDP_MAGIC_AUTHORITY, UDP_USER_AGENT)
            .await
            .context("trusttunnel: open _udp2 tunnel")?;
        let (reader, writer) = tokio::io::split(stream);
        Ok(Self {
            dest,
            write: Mutex::new(writer),
            read: Mutex::new(reader),
        })
    }

    /// Frame `payload` as one client UDP packet to `self.dest` and write it.
    pub async fn send(&self, payload: &[u8]) -> Result<()> {
        let frame = encode_client_packet(self.dest, payload);
        let mut w = self.write.lock().await;
        w.write_all(&frame).await.context("trusttunnel udp: write datagram")?;
        w.flush().await.context("trusttunnel udp: flush datagram")?;
        Ok(())
    }

    /// Read one reply UDP packet, discard its peer/local address header, and
    /// return the application payload.
    pub async fn recv(&self) -> Result<Vec<u8>> {
        let mut r = self.read.lock().await;
        decode_server_packet(&mut *r).await
    }
}

/// Resolve a UDP target to a literal address (TrustTunnel UDP frames name an IP
/// peer, so a domain must be resolved before the association opens).
async fn resolve_udp_target(target: &TargetAddr) -> Result<SocketAddr> {
    match target {
        TargetAddr::Ip(addr) => Ok(*addr),
        TargetAddr::Domain(host, port) => tokio::net::lookup_host((host.as_str(), *port))
            .await
            .with_context(|| format!("trusttunnel udp: resolve {host}:{port}"))?
            .next()
            .with_context(|| format!("trusttunnel udp: no address for {host}:{port}")),
    }
}

/// Encode a destination IP as the 16-byte padded address used on the wire: an
/// IPv4 address is right-aligned in a zeroed 16-byte field, an IPv6 address
/// fills it.
fn padding_ip(ip: IpAddr) -> [u8; 16] {
    match ip {
        IpAddr::V4(v4) => {
            let mut buf = [0u8; 16];
            buf[12..16].copy_from_slice(&v4.octets());
            buf
        }
        IpAddr::V6(v6) => v6.octets(),
    }
}

/// Build one client → server UDP packet for `dest`:
///
/// ```text
/// length(u32 BE) | src addr(16, zero) | src port(2, zero) |
/// dst addr(16) | dst port(2) | app_name_len(1) | app_name | payload
/// ```
///
/// `length` counts every byte after the 4-byte prefix.
fn encode_client_packet(dest: SocketAddr, payload: &[u8]) -> Vec<u8> {
    let app_name = UDP_APP_NAME;
    let length = 16 + 2 + 16 + 2 + 1 + app_name.len() + payload.len();
    let mut out = Vec::with_capacity(4 + length);
    out.extend_from_slice(&(length as u32).to_be_bytes());
    // Source address (16) + port (2): unknown on the client side, sent zeroed.
    out.extend_from_slice(&[0u8; 18]);
    out.extend_from_slice(&padding_ip(dest.ip()));
    out.extend_from_slice(&dest.port().to_be_bytes());
    out.push(app_name.len() as u8);
    out.extend_from_slice(app_name);
    out.extend_from_slice(payload);
    out
}

/// Read one server → client UDP packet and return its payload:
///
/// ```text
/// length(u32 BE) | src addr(16) | src port(2) | local addr(16) | local port(2) | payload
/// ```
///
/// The per-packet source/local address header is read and discarded (the
/// association already knows its peer); `payload_len = length - overhead`.
async fn decode_server_packet<R>(reader: &mut R) -> Result<Vec<u8>>
where
    R: AsyncRead + Unpin,
{
    let mut header = [0u8; 4 + UDP_SERVER_OVERHEAD];
    reader
        .read_exact(&mut header)
        .await
        .context("trusttunnel udp: read header")?;
    let length = u32::from_be_bytes([header[0], header[1], header[2], header[3]]) as usize;
    let payload_len = length
        .checked_sub(UDP_SERVER_OVERHEAD)
        .with_context(|| format!("trusttunnel udp: invalid packet length {length}"))?;
    let mut payload = vec![0u8; payload_len];
    reader
        .read_exact(&mut payload)
        .await
        .context("trusttunnel udp: read payload")?;
    Ok(payload)
}

#[cfg(test)]
mod tests {
    use std::net::{Ipv4Addr, Ipv6Addr};

    use super::*;

    fn parse_entry(yaml: &str) -> ProxyEntry {
        serde_yaml_ng::from_str(yaml).expect("parse proxy entry")
    }

    #[test]
    fn parses_minimal_entry_with_default_h2_alpn() {
        let cfg = TrustTunnelOutboundConfig::from_proxy(&parse_entry(
            "name: t\ntype: trusttunnel\nserver: proxy.example\nport: 443\n",
        ))
        .unwrap();
        assert_eq!(cfg.server, "proxy.example");
        assert_eq!(cfg.port, 443);
        assert_eq!(cfg.username, "");
        assert_eq!(cfg.password, "");
        assert!(!cfg.udp);
        match cfg.security {
            Security::Tls(tls) => {
                assert_eq!(tls.alpn, vec!["h2".to_string()]);
                assert_eq!(tls.server_name, None);
            }
            other => panic!("expected TLS security, got {other:?}"),
        }
    }

    #[test]
    fn honours_credentials_udp_and_sni() {
        let cfg = TrustTunnelOutboundConfig::from_proxy(&parse_entry(
            "name: t\ntype: trusttunnel\nserver: proxy.example\nport: 443\nusername: bob\npassword: secret\nudp: true\nsni: edge.example\nskip-cert-verify: true\n",
        ))
        .unwrap();
        assert_eq!(cfg.username, "bob");
        assert_eq!(cfg.password, "secret");
        assert!(cfg.udp);
        match cfg.security {
            Security::Tls(tls) => {
                assert_eq!(tls.server_name.as_deref(), Some("edge.example"));
                assert!(tls.skip_cert_verify);
            }
            other => panic!("expected TLS security, got {other:?}"),
        }
    }

    #[test]
    fn quic_carrier_is_rejected() {
        let err = TrustTunnelOutboundConfig::from_proxy(&parse_entry(
            "name: t\ntype: trusttunnel\nserver: proxy.example\nport: 443\nquic: true\n",
        ))
        .unwrap_err();
        assert!(err.to_string().contains("quic"), "{err}");
    }

    #[test]
    fn alpn_without_h2_is_rejected() {
        let err = TrustTunnelOutboundConfig::from_proxy(&parse_entry(
            "name: t\ntype: trusttunnel\nserver: proxy.example\nport: 443\nalpn:\n  - http/1.1\n",
        ))
        .unwrap_err();
        assert!(err.to_string().contains("h2"), "{err}");
    }

    #[test]
    fn explicit_alpn_with_h2_is_kept() {
        let cfg = TrustTunnelOutboundConfig::from_proxy(&parse_entry(
            "name: t\ntype: trusttunnel\nserver: proxy.example\nport: 443\nalpn:\n  - h2\n  - http/1.1\n",
        ))
        .unwrap();
        match cfg.security {
            Security::Tls(tls) => assert_eq!(tls.alpn, vec!["h2".to_string(), "http/1.1".to_string()]),
            other => panic!("expected TLS security, got {other:?}"),
        }
    }

    #[test]
    fn missing_server_is_rejected() {
        let err =
            TrustTunnelOutboundConfig::from_proxy(&parse_entry("name: t\ntype: trusttunnel\nport: 443\n")).unwrap_err();
        assert!(err.to_string().contains("server"), "{err}");
    }

    #[test]
    fn basic_auth_matches_rfc_vector() {
        // RFC 7617 worked example: base64("aladdin:opensesame").
        assert_eq!(basic_auth("aladdin", "opensesame"), "Basic YWxhZGRpbjpvcGVuc2VzYW1l");
        // Anonymous proxy still sends `Basic base64(":")`.
        assert_eq!(basic_auth("", ""), "Basic Og==");
    }

    #[test]
    fn tcp_authority_brackets_ipv6() {
        let v4 = TargetAddr::Ip(SocketAddr::from((Ipv4Addr::new(1, 2, 3, 4), 80)));
        assert_eq!(authority(&v4), "1.2.3.4:80");
        let domain = TargetAddr::Domain("example.com".to_string(), 443);
        assert_eq!(authority(&domain), "example.com:443");
        let v6 = TargetAddr::Ip("[2001:db8::1]:8443".parse().unwrap());
        assert_eq!(authority(&v6), "[2001:db8::1]:8443");
    }

    #[test]
    fn connect_uri_is_authority_only() {
        let uri = connect_uri("example.com:443").unwrap();
        assert_eq!(uri.authority().map(|a| a.as_str()), Some("example.com:443"));
        assert_eq!(uri.scheme(), None);
        assert_eq!(uri.path(), "");
        // The magic UDP authority (a leading underscore) is a valid reg-name.
        let udp = connect_uri(UDP_MAGIC_AUTHORITY).unwrap();
        assert_eq!(udp.authority().map(|a| a.as_str()), Some("_udp2"));
    }

    #[test]
    fn padding_ip_places_ipv4_in_low_bytes() {
        assert_eq!(
            padding_ip(IpAddr::V4(Ipv4Addr::new(1, 2, 3, 4))),
            [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 2, 3, 4]
        );
        let v6 = Ipv6Addr::new(0x2001, 0xdb8, 0, 0, 0, 0, 0, 1);
        assert_eq!(padding_ip(IpAddr::V6(v6)), v6.octets());
    }

    #[tokio::test]
    async fn client_packet_roundtrips_through_server_framing() {
        // Encode a client packet, then re-frame it as a server reply and confirm
        // the decoder recovers the payload (the two headers differ only in which
        // address field is zeroed).
        let dest = SocketAddr::from((Ipv4Addr::new(9, 9, 9, 9), 53));
        let payload = b"hello trusttunnel udp";
        let client = encode_client_packet(dest, payload);

        // Client header: 4 + 18 (zero src) + 16 (dst) + 2 (port) + 1 (app len 0).
        let length = u32::from_be_bytes([client[0], client[1], client[2], client[3]]) as usize;
        assert_eq!(length, 16 + 2 + 16 + 2 + 1 + payload.len());
        assert_eq!(&client[client.len() - payload.len()..], payload);

        // Build a server reply frame for the same payload and decode it.
        let mut server = Vec::new();
        let reply_len = (UDP_SERVER_OVERHEAD + payload.len()) as u32;
        server.extend_from_slice(&reply_len.to_be_bytes());
        server.extend_from_slice(&padding_ip(dest.ip()));
        server.extend_from_slice(&dest.port().to_be_bytes());
        server.extend_from_slice(&[0u8; 18]); // zeroed local address/port
        server.extend_from_slice(payload);

        let mut reader: &[u8] = &server;
        let decoded = decode_server_packet(&mut reader).await.unwrap();
        assert_eq!(decoded, payload);
    }
}
