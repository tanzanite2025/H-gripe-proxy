//! VLESS outbound.
//!
//! Implements the VLESS request/response framing only; the transport (tcp/ws)
//! and security (none/tls/reality) layers it runs over are provided by
//! [`crate::transport`], so this module is purely the protocol layer. `tcp`,
//! `ws`, `grpc`, `xhttp` (stream-one), `httpupgrade` and `h2` transports over
//! `none` / `tls` / `reality` security are supported today (`h2` is
//! TLS-mandatory; REALITY counts as TLS here). Because security and transport
//! are orthogonal, VLESS-REALITY works under every transport automatically. The
//! multi-request xhttp modes and the `flow: xtls-rprx-vision` layer land in
//! follow-up work and are rejected by [`VlessOutboundConfig::from_proxy`] rather
//! than silently mis-encoded.
//!
//! Wire format (client → server request header):
//! ```text
//! +---------+----------+-------------+----------+---------+------+---------+---------+
//! | version | uuid(16) | addon_len=N | addon(N) | command | port | atyp(1) | address |
//! +---------+----------+-------------+----------+---------+------+---------+---------+
//! ```
//! `command` is 0x01 (TCP). `atyp` is 0x01 IPv4 / 0x02 domain / 0x03 IPv6.
//! Server → client response header is `version(1) | addon_len(1) | addon(N)`
//! and is stripped from the read side before application data is surfaced.

use std::net::SocketAddr;
use std::pin::Pin;
use std::task::{Context as TaskContext, Poll, ready};

use anyhow::{Context, Result};
use tokio::io::{AsyncRead, AsyncWrite, AsyncWriteExt, ReadBuf};

use crate::address::TargetAddr;
use crate::outbound::BoxedStream;
use crate::proxy::{ProxyEntry, parse_uuid};
use crate::transport::{self, Security, Transport};

const VERSION: u8 = 0x00;
const CMD_TCP: u8 = 0x01;
const ATYP_IPV4: u8 = 0x01;
const ATYP_DOMAIN: u8 = 0x02;
const ATYP_IPV6: u8 = 0x03;

/// Fully-resolved VLESS outbound parameters.
///
/// `security` and `transport` are orthogonal layers (see [`crate::transport`]):
/// e.g. `VLESS-WS-TLS` is `Security::Tls` + `Transport::Ws`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VlessOutboundConfig {
    pub server: String,
    pub port: u16,
    pub uuid: [u8; 16],
    pub security: Security,
    pub transport: Transport,
}

impl VlessOutboundConfig {
    /// Build an outbound config from a parsed `vless` proxy entry, rejecting
    /// sub-features that are not implemented yet so traffic is never mis-framed.
    pub fn from_proxy(entry: &ProxyEntry) -> Result<Self> {
        let opts = &entry.options;
        let server = opts
            .server
            .clone()
            .filter(|s| !s.is_empty())
            .context("vless: missing server")?;
        let port = opts.port.context("vless: missing port")?;
        let uuid = parse_uuid(opts.uuid.as_deref().context("vless: missing uuid")?)?;

        // Security and transport are orthogonal to the protocol framing and are
        // built by the shared layer helper; VLESS security is plaintext unless
        // `tls` / `reality-opts` opt in.
        let (security, transport) = transport::build_layers(opts, "vless", false)?;

        Ok(Self {
            server,
            port,
            uuid,
            security,
            transport,
        })
    }
}

/// Connect a VLESS outbound to `target` and return a relay-ready stream with
/// the request header already sent and the response header stripped.
pub async fn connect(config: &VlessOutboundConfig, target: &TargetAddr) -> Result<BoxedStream> {
    let mut stream = transport::establish(&config.server, config.port, &config.security, &config.transport).await?;
    let header = encode_request_header(&config.uuid, CMD_TCP, target);
    stream.write_all(&header).await.context("vless: send request header")?;
    Ok(Box::new(VlessStream::new(stream)))
}

/// Encode the VLESS request header for a TCP CONNECT to `target` with no addons.
fn encode_request_header(uuid: &[u8; 16], command: u8, target: &TargetAddr) -> Vec<u8> {
    let mut buf = Vec::with_capacity(24);
    buf.push(VERSION);
    buf.extend_from_slice(uuid);
    buf.push(0); // addon length: no flow / addons in this slice
    buf.push(command);
    buf.extend_from_slice(&target.port().to_be_bytes());
    match target {
        TargetAddr::Ip(SocketAddr::V4(addr)) => {
            buf.push(ATYP_IPV4);
            buf.extend_from_slice(&addr.ip().octets());
        }
        TargetAddr::Ip(SocketAddr::V6(addr)) => {
            buf.push(ATYP_IPV6);
            buf.extend_from_slice(&addr.ip().octets());
        }
        TargetAddr::Domain(host, _) => {
            buf.push(ATYP_DOMAIN);
            buf.push(host.len() as u8);
            buf.extend_from_slice(host.as_bytes());
        }
    }
    buf
}

/// Read-side state while the VLESS response header is being consumed.
#[derive(Debug)]
enum HeadState {
    NeedVersion,
    NeedAddonLen,
    SkipAddons(u8),
    Done,
}

/// Wraps a transport stream, stripping the VLESS response header from the read
/// side on first reads. Writes pass straight through (the request header was
/// already sent at connect time).
#[derive(Debug)]
struct VlessStream<S> {
    inner: S,
    head: HeadState,
}

impl<S> VlessStream<S> {
    fn new(inner: S) -> Self {
        Self {
            inner,
            head: HeadState::NeedVersion,
        }
    }
}

impl<S: AsyncRead + Unpin> AsyncRead for VlessStream<S> {
    fn poll_read(self: Pin<&mut Self>, cx: &mut TaskContext<'_>, buf: &mut ReadBuf<'_>) -> Poll<std::io::Result<()>> {
        let this = self.get_mut();
        let mut scratch = [0u8; 256];
        while !matches!(this.head, HeadState::Done) {
            let want = match this.head {
                HeadState::NeedVersion | HeadState::NeedAddonLen => 1,
                HeadState::SkipAddons(n) => n as usize,
                HeadState::Done => unreachable!(),
            };
            let mut head_buf = ReadBuf::new(&mut scratch[..want]);
            ready!(Pin::new(&mut this.inner).poll_read(cx, &mut head_buf))?;
            let filled = head_buf.filled().len();
            if filled == 0 {
                // EOF before the header completed: surface as clean EOF.
                return Poll::Ready(Ok(()));
            }
            this.head = match this.head {
                HeadState::NeedVersion => HeadState::NeedAddonLen,
                HeadState::NeedAddonLen => {
                    let len = head_buf.filled()[0];
                    if len == 0 {
                        HeadState::Done
                    } else {
                        HeadState::SkipAddons(len)
                    }
                }
                HeadState::SkipAddons(remaining) => {
                    let left = remaining - filled as u8;
                    if left == 0 {
                        HeadState::Done
                    } else {
                        HeadState::SkipAddons(left)
                    }
                }
                HeadState::Done => unreachable!(),
            };
        }
        Pin::new(&mut this.inner).poll_read(cx, buf)
    }
}

impl<S: AsyncWrite + Unpin> AsyncWrite for VlessStream<S> {
    fn poll_write(self: Pin<&mut Self>, cx: &mut TaskContext<'_>, buf: &[u8]) -> Poll<std::io::Result<usize>> {
        Pin::new(&mut self.get_mut().inner).poll_write(cx, buf)
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut TaskContext<'_>) -> Poll<std::io::Result<()>> {
        Pin::new(&mut self.get_mut().inner).poll_flush(cx)
    }

    fn poll_shutdown(self: Pin<&mut Self>, cx: &mut TaskContext<'_>) -> Poll<std::io::Result<()>> {
        Pin::new(&mut self.get_mut().inner).poll_shutdown(cx)
    }
}

#[cfg(test)]
mod tests {
    use std::net::{Ipv4Addr, Ipv6Addr, SocketAddr};

    use super::*;
    use crate::tls::ClientFingerprint;

    #[test]
    fn parses_canonical_uuid() {
        let uuid = parse_uuid("b831381d-6324-4d53-ad4f-8cda48b30811").unwrap();
        assert_eq!(uuid[0], 0xb8);
        assert_eq!(uuid[15], 0x11);
    }

    #[test]
    fn rejects_malformed_uuid() {
        assert!(parse_uuid("not-a-uuid").is_err());
        assert!(parse_uuid("").is_err());
    }

    #[test]
    fn encodes_domain_target_header() {
        let uuid = [0xABu8; 16];
        let target = TargetAddr::Domain("example.com".to_string(), 443);
        let header = encode_request_header(&uuid, CMD_TCP, &target);

        assert_eq!(header[0], VERSION);
        assert_eq!(&header[1..17], &uuid);
        assert_eq!(header[17], 0); // addon length
        assert_eq!(header[18], CMD_TCP);
        assert_eq!(&header[19..21], &443u16.to_be_bytes());
        assert_eq!(header[21], ATYP_DOMAIN);
        assert_eq!(header[22], "example.com".len() as u8);
        assert_eq!(&header[23..], b"example.com");
    }

    #[test]
    fn encodes_ipv4_target_header() {
        let uuid = [0u8; 16];
        let target = TargetAddr::Ip(SocketAddr::new(Ipv4Addr::new(1, 2, 3, 4).into(), 8080));
        let header = encode_request_header(&uuid, CMD_TCP, &target);
        assert_eq!(&header[19..21], &8080u16.to_be_bytes());
        assert_eq!(header[21], ATYP_IPV4);
        assert_eq!(&header[22..26], &[1, 2, 3, 4]);
    }

    #[test]
    fn encodes_ipv6_target_header() {
        let uuid = [0u8; 16];
        let ip = Ipv6Addr::new(0x2001, 0xdb8, 0, 0, 0, 0, 0, 1);
        let target = TargetAddr::Ip(SocketAddr::new(ip.into(), 53));
        let header = encode_request_header(&uuid, CMD_TCP, &target);
        assert_eq!(header[21], ATYP_IPV6);
        assert_eq!(&header[22..38], &ip.octets());
    }

    fn parse_entry(yaml: &str) -> ProxyEntry {
        serde_yaml_ng::from_str(yaml).expect("proxy entry should parse")
    }

    #[test]
    fn h2_without_tls_is_rejected() {
        let entry = parse_entry(
            "name: h2-plain\ntype: vless\nserver: example.com\nport: 443\n\
             uuid: b831381d-6324-4d53-ad4f-8cda48b30811\nnetwork: h2\n",
        );
        let err = VlessOutboundConfig::from_proxy(&entry).unwrap_err();
        assert!(err.to_string().contains("h2 transport requires TLS"), "got: {err}");
    }

    #[test]
    fn h2_with_tls_maps_to_h2_transport_and_forces_alpn() {
        let entry = parse_entry(
            "name: h2-tls\ntype: vless\nserver: example.com\nport: 443\n\
             uuid: b831381d-6324-4d53-ad4f-8cda48b30811\nnetwork: h2\ntls: true\n\
             servername: edge.example.com\nh2-opts:\n  path: /tunnel\n  host: cdn.example.com\n",
        );
        let cfg = VlessOutboundConfig::from_proxy(&entry).unwrap();
        match &cfg.transport {
            Transport::H2(h2) => {
                assert_eq!(h2.path, "/tunnel");
                assert_eq!(h2.host.as_deref(), Some("cdn.example.com"));
            }
            other => panic!("expected H2 transport, got {other:?}"),
        }
        match &cfg.security {
            Security::Tls(tls) => assert_eq!(tls.alpn, vec!["h2".to_string()]),
            other => panic!("expected TLS security, got {other:?}"),
        }
    }

    /// A 43-char base64 string (no padding) that decodes to 32 zero bytes,
    /// usable as a syntactically valid REALITY public-key in fixtures.
    fn zero_public_key_b64() -> String {
        "A".repeat(43)
    }

    #[test]
    fn reality_opts_map_to_reality_security() {
        let yaml = format!(
            "name: r\ntype: vless\nserver: example.com\nport: 443\n\
             uuid: b831381d-6324-4d53-ad4f-8cda48b30811\ntls: true\n\
             servername: www.cloudflare.com\nclient-fingerprint: chrome\n\
             network: grpc\ngrpc-opts:\n  grpc-service-name: GunService\n\
             reality-opts:\n  public-key: \"{}\"\n  short-id: 0123abcd\n",
            zero_public_key_b64()
        );
        let entry = parse_entry(&yaml);
        let cfg = VlessOutboundConfig::from_proxy(&entry).unwrap();
        match &cfg.security {
            Security::Reality(r) => {
                assert_eq!(r.server_name, "www.cloudflare.com");
                assert_eq!(r.public_key, [0u8; 32]);
                assert_eq!(r.short_id, vec![0x01, 0x23, 0xab, 0xcd]);
                assert_eq!(r.client_fingerprint, Some(ClientFingerprint::Chrome));
                // grpc forces the h2 ALPN on the REALITY config too.
                assert_eq!(r.alpn, vec!["h2".to_string()]);
            }
            other => panic!("expected REALITY security, got {other:?}"),
        }
    }

    #[test]
    fn reality_without_servername_is_rejected() {
        let yaml = format!(
            "name: r\ntype: vless\nserver: example.com\nport: 443\n\
             uuid: b831381d-6324-4d53-ad4f-8cda48b30811\ntls: true\n\
             reality-opts:\n  public-key: \"{}\"\n",
            zero_public_key_b64()
        );
        let err = VlessOutboundConfig::from_proxy(&parse_entry(&yaml)).unwrap_err();
        assert!(err.to_string().contains("servername"), "got: {err}");
    }

    #[test]
    fn reality_without_public_key_is_rejected() {
        let yaml = "name: r\ntype: vless\nserver: example.com\nport: 443\n\
             uuid: b831381d-6324-4d53-ad4f-8cda48b30811\ntls: true\n\
             servername: www.cloudflare.com\nreality-opts:\n  short-id: ab\n";
        let err = VlessOutboundConfig::from_proxy(&parse_entry(yaml)).unwrap_err();
        assert!(err.to_string().contains("public-key"), "got: {err}");
    }

    #[test]
    fn reality_short_public_key_is_rejected() {
        let yaml = "name: r\ntype: vless\nserver: example.com\nport: 443\n\
             uuid: b831381d-6324-4d53-ad4f-8cda48b30811\ntls: true\n\
             servername: www.cloudflare.com\nreality-opts:\n  public-key: AAAA\n";
        let err = VlessOutboundConfig::from_proxy(&parse_entry(yaml)).unwrap_err();
        assert!(err.to_string().contains("32 bytes"), "got: {err}");
    }

    #[test]
    fn unknown_client_fingerprint_is_rejected() {
        let yaml = "name: r\ntype: vless\nserver: example.com\nport: 443\n\
             uuid: b831381d-6324-4d53-ad4f-8cda48b30811\ntls: true\n\
             servername: www.cloudflare.com\nclient-fingerprint: netscape\n";
        let err = VlessOutboundConfig::from_proxy(&parse_entry(yaml)).unwrap_err();
        assert!(err.to_string().contains("client-fingerprint"), "got: {err}");
    }

    #[test]
    fn flow_is_still_rejected() {
        let yaml = "name: r\ntype: vless\nserver: example.com\nport: 443\n\
             uuid: b831381d-6324-4d53-ad4f-8cda48b30811\ntls: true\n\
             servername: www.cloudflare.com\nflow: xtls-rprx-vision\n";
        let err = VlessOutboundConfig::from_proxy(&parse_entry(yaml)).unwrap_err();
        assert!(err.to_string().contains("flow"), "got: {err}");
    }
}
