//! GOST relay outbound (go-gost relay protocol, version 1).
//!
//! Forwards through a [go-gost](https://github.com/go-gost) relay server using
//! the relay `CONNECT` handshake: the client sends a single request carrying a
//! feature list (optional user-auth, the target address, and the network), the
//! server dials the target and answers with a status, after which bytes are
//! relayed verbatim. This is the upstream-proxy analogue of the HTTP/SOCKS5
//! outbounds, with a binary framing instead of a text one.
//!
//! Supported clash/mihomo knobs (matching `transport/gost` in mihomo):
//!
//! * `username` / `password` → a relay `UserAuth` feature.
//! * `tls: true` → the relay connection runs over TLS, reusing the shared
//!   [`crate::transport`] TLS layer (`sni` / `skip-cert-verify` /
//!   `fingerprint` / `client-fingerprint`).
//! * `forward: true` → send the request with an *empty* target address so the
//!   relay routes to its own preconfigured upstream instead of `target`.
//!
//! Only TCP is implemented (UDP associations are refused up front, see
//! [`crate::outbound::supports_udp_associate`]); `mux: true` (smux) and mTLS
//! client certificates (`certificate` / `private-key`) are rejected at parse
//! time rather than silently ignored.
//!
//! Request / response wire format (big-endian lengths), per `go-gost/relay`:
//! ```text
//! Request : VER(0x01) | CMD(0x01) | FEALEN(2) | FEATURES
//! Response: VER(0x01) | STATUS    | FEALEN(2) | FEATURES
//! Feature : TYPE(1) | LEN(2) | DATA(LEN)
//!   UserAuth(0x01): ULEN(1) UNAME PLEN(1) PASSWD
//!   Addr(0x02)    : ATYP(1) ADDR PORT(2)   (ATYP 1=IPv4, 3=domain, 4=IPv6)
//!   Network(0x04) : NETWORK(2)             (0=tcp)
//! ```
//! `STATUS == 0x00` means the tunnel is open; any other status is an error.

use std::net::SocketAddr;

use anyhow::{Context, Result, bail};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWriteExt};

use crate::address::TargetAddr;
use crate::config::outbound_opts::ProxyEntry;
use crate::outbound::BoxedStream;
use crate::transport::tls::{ClientFingerprint, TlsClientConfig};
use crate::transport::{self, Security, Transport};

const RELAY_VERSION1: u8 = 0x01;
const RELAY_CMD_CONNECT: u8 = 0x01;
const RELAY_STATUS_OK: u8 = 0x00;

const FEATURE_USER_AUTH: u8 = 0x01;
const FEATURE_ADDR: u8 = 0x02;
const FEATURE_NETWORK: u8 = 0x04;

const ADDR_IPV4: u8 = 0x01;
const ADDR_DOMAIN: u8 = 0x03;
const ADDR_IPV6: u8 = 0x04;

const NETWORK_TCP: u16 = 0x0000;

/// Fully-resolved GOST relay outbound parameters.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GostRelayOutboundConfig {
    pub server: String,
    pub port: u16,
    /// Forward mode (`forward: true`): omit the target address from the request
    /// so the relay routes to its own preconfigured upstream.
    pub forward: bool,
    /// Optional `username`/`password` for the relay `UserAuth` feature. Present
    /// when either is non-empty (the relay protocol allows one to be empty).
    pub auth: Option<(String, String)>,
    /// Security layer wrapping the relay connection: [`Security::None`] for a
    /// plaintext relay, [`Security::Tls`] for `tls: true`.
    pub security: Security,
}

impl GostRelayOutboundConfig {
    /// Build an outbound config from a parsed `gost-relay` proxy entry.
    pub fn from_proxy(entry: &ProxyEntry) -> Result<Self> {
        let opts = &entry.options;
        let server = opts
            .server
            .clone()
            .filter(|s| !s.is_empty())
            .context("gost-relay: missing server")?;
        let port = opts.port.context("gost-relay: missing port")?;

        if opts.mux.unwrap_or(false) {
            bail!("gost-relay: mux (smux) is not supported yet");
        }
        if opts.reality_opts.is_some() {
            bail!("gost-relay: reality-opts not supported on a relay");
        }

        // Credentials are optional; the relay UserAuth feature permits an empty
        // username or password, so a single non-empty field is enough to send
        // it. Both empty means anonymous (no feature).
        let user = opts.username.clone().unwrap_or_default();
        let pass = opts.password.clone().unwrap_or_default();
        if user.len() > 0xFF {
            bail!("gost-relay: username exceeds 255 bytes");
        }
        if pass.len() > 0xFF {
            bail!("gost-relay: password exceeds 255 bytes");
        }
        let auth = if user.is_empty() && pass.is_empty() {
            None
        } else {
            Some((user, pass))
        };

        // A relay is plaintext by default; `tls: true` wraps it in TLS. mTLS
        // client certificates are not wired through the shared TLS layer yet.
        let security = if opts.tls.unwrap_or(false) {
            if opts.certificate.as_deref().is_some_and(|s| !s.is_empty())
                || opts.private_key.as_deref().is_some_and(|s| !s.is_empty())
            {
                bail!("gost-relay: mTLS client certificate (certificate/private-key) is not supported yet");
            }
            let client_fingerprint = match opts.client_fingerprint.as_deref() {
                None | Some("") => None,
                Some(value) => Some(ClientFingerprint::parse(value).map_err(|e| anyhow::anyhow!("gost-relay: {e}"))?),
            };
            Security::Tls(TlsClientConfig {
                server_name: opts
                    .sni
                    .clone()
                    .filter(|s| !s.is_empty())
                    .or_else(|| Some(server.clone())),
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
            forward: opts.forward.unwrap_or(false),
            auth,
            security,
        })
    }
}

/// Connect through the relay to `target` and return a relay-ready stream. Once
/// the server answers the `CONNECT` with `STATUS_OK` the stream is transparent.
pub async fn connect(config: &GostRelayOutboundConfig, target: &TargetAddr) -> Result<BoxedStream> {
    let mut stream = transport::establish(&config.server, config.port, &config.security, &Transport::Tcp)
        .await
        .context("gost-relay: dial relay server")?;
    let request = build_connect_request(target, config.forward, config.auth.as_ref())?;
    stream
        .write_all(&request)
        .await
        .context("gost-relay: send relay request")?;
    read_connect_response(&mut stream)
        .await
        .with_context(|| format!("gost-relay: CONNECT to {target}"))?;
    Ok(stream)
}

/// Encode a `Feature` (TYPE, 2-byte LEN, DATA) onto `buf`.
fn push_feature(buf: &mut Vec<u8>, feature_type: u8, payload: &[u8]) {
    buf.push(feature_type);
    buf.extend_from_slice(&(payload.len() as u16).to_be_bytes());
    buf.extend_from_slice(payload);
}

/// Encode the `Addr` feature payload (ATYP, ADDR, 2-byte PORT) for `target`.
fn encode_addr(target: &TargetAddr) -> Vec<u8> {
    let mut out = Vec::new();
    match target {
        TargetAddr::Ip(SocketAddr::V4(addr)) => {
            out.push(ADDR_IPV4);
            out.extend_from_slice(&addr.ip().octets());
            out.extend_from_slice(&addr.port().to_be_bytes());
        }
        TargetAddr::Ip(SocketAddr::V6(addr)) => {
            out.push(ADDR_IPV6);
            out.extend_from_slice(&addr.ip().octets());
            out.extend_from_slice(&addr.port().to_be_bytes());
        }
        TargetAddr::Domain(host, port) => {
            out.push(ADDR_DOMAIN);
            out.push(host.len() as u8);
            out.extend_from_slice(host.as_bytes());
            out.extend_from_slice(&port.to_be_bytes());
        }
    }
    out
}

/// Build the relay `CONNECT` request: header + feature list (optional
/// `UserAuth`, the `Addr` unless forwarding, and `Network` = TCP).
fn build_connect_request(target: &TargetAddr, forward: bool, auth: Option<&(String, String)>) -> Result<Vec<u8>> {
    if matches!(target, TargetAddr::Domain(host, _) if host.len() > 0xFF) {
        bail!("gost-relay: target host exceeds 255 bytes");
    }

    let mut features = Vec::new();
    if let Some((user, pass)) = auth {
        let mut payload = Vec::with_capacity(2 + user.len() + pass.len());
        payload.push(user.len() as u8);
        payload.extend_from_slice(user.as_bytes());
        payload.push(pass.len() as u8);
        payload.extend_from_slice(pass.as_bytes());
        push_feature(&mut features, FEATURE_USER_AUTH, &payload);
    }
    if !forward {
        push_feature(&mut features, FEATURE_ADDR, &encode_addr(target));
    }
    push_feature(&mut features, FEATURE_NETWORK, &NETWORK_TCP.to_be_bytes());

    if features.len() > 0xFFFF {
        bail!("gost-relay: feature list exceeds 65535 bytes");
    }

    let mut req = Vec::with_capacity(4 + features.len());
    req.push(RELAY_VERSION1);
    req.push(RELAY_CMD_CONNECT);
    req.extend_from_slice(&(features.len() as u16).to_be_bytes());
    req.extend_from_slice(&features);
    Ok(req)
}

/// Read the relay response header, require `version == 1` and `STATUS_OK`, and
/// drain the response feature list so the stream is positioned at tunnel data.
async fn read_connect_response<R>(stream: &mut R) -> Result<()>
where
    R: AsyncRead + Unpin,
{
    let mut header = [0u8; 4];
    stream
        .read_exact(&mut header)
        .await
        .context("gost-relay: read relay response header")?;
    if header[0] != RELAY_VERSION1 {
        bail!("gost-relay: unexpected response version 0x{:02x}", header[0]);
    }
    if header[1] != RELAY_STATUS_OK {
        bail!(
            "gost-relay: connect failed with status 0x{:02x} ({})",
            header[1],
            status_text(header[1])
        );
    }
    let feature_len = u16::from_be_bytes([header[2], header[3]]) as usize;
    if feature_len > 0 {
        let mut features = vec![0u8; feature_len];
        stream
            .read_exact(&mut features)
            .await
            .context("gost-relay: read relay response features")?;
    }
    Ok(())
}

/// Human-readable label for a relay status byte (for error messages).
fn status_text(status: u8) -> &'static str {
    match status {
        0x00 => "OK",
        0x01 => "bad request",
        0x02 => "unauthorized",
        0x03 => "forbidden",
        0x04 => "timeout",
        0x05 => "service unavailable",
        0x06 => "host unreachable",
        0x07 => "network unreachable",
        0x08 => "internal server error",
        _ => "unknown",
    }
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
    fn domain_connect_request_wire_format() {
        let target = TargetAddr::Domain("example.com".to_string(), 443);
        let req = build_connect_request(&target, false, None).unwrap();
        // Header: VER, CMD, FEALEN(2).
        assert_eq!(req[0], RELAY_VERSION1);
        assert_eq!(req[1], RELAY_CMD_CONNECT);
        let fealen = u16::from_be_bytes([req[2], req[3]]) as usize;
        assert_eq!(fealen, req.len() - 4);

        // First feature: Addr (no auth).
        assert_eq!(req[4], FEATURE_ADDR);
        let addr_len = u16::from_be_bytes([req[5], req[6]]) as usize;
        // ATYP(1) + dlen(1) + "example.com"(11) + port(2) = 15.
        assert_eq!(addr_len, 15);
        assert_eq!(req[7], ADDR_DOMAIN);
        assert_eq!(req[8] as usize, "example.com".len());
        assert_eq!(&req[9..9 + 11], b"example.com");
        assert_eq!(&req[20..22], &443u16.to_be_bytes());

        // Trailing feature: Network = TCP.
        let net_type_idx = 4 + 3 + addr_len;
        assert_eq!(req[net_type_idx], FEATURE_NETWORK);
        assert_eq!(&req[net_type_idx + 3..net_type_idx + 5], &NETWORK_TCP.to_be_bytes());
    }

    #[test]
    fn auth_feature_precedes_addr() {
        let target = TargetAddr::Domain("h".to_string(), 80);
        let auth = ("user".to_string(), "pass".to_string());
        let req = build_connect_request(&target, false, Some(&auth)).unwrap();
        assert_eq!(req[4], FEATURE_USER_AUTH);
        let auth_len = u16::from_be_bytes([req[5], req[6]]) as usize;
        // ULEN(1)+user(4)+PLEN(1)+pass(4) = 10.
        assert_eq!(auth_len, 10);
        assert_eq!(req[7] as usize, "user".len());
        assert_eq!(&req[8..12], b"user");
        assert_eq!(req[12] as usize, "pass".len());
        assert_eq!(&req[13..17], b"pass");
    }

    #[test]
    fn forward_mode_omits_addr_feature() {
        let target = TargetAddr::Domain("example.com".to_string(), 443);
        let req = build_connect_request(&target, true, None).unwrap();
        // Only the Network feature remains.
        assert_eq!(req[4], FEATURE_NETWORK);
        let fealen = u16::from_be_bytes([req[2], req[3]]) as usize;
        // Network feature: TYPE(1)+LEN(2)+DATA(2) = 5.
        assert_eq!(fealen, 5);
    }

    #[test]
    fn ipv4_target_addr_feature() {
        let target = TargetAddr::Ip(SocketAddr::from((Ipv4Addr::new(1, 2, 3, 4), 8080)));
        let req = build_connect_request(&target, false, None).unwrap();
        assert_eq!(req[4], FEATURE_ADDR);
        let addr_len = u16::from_be_bytes([req[5], req[6]]) as usize;
        // ATYP(1)+IPv4(4)+port(2) = 7.
        assert_eq!(addr_len, 7);
        assert_eq!(req[7], ADDR_IPV4);
        assert_eq!(&req[8..12], &[1, 2, 3, 4]);
        assert_eq!(&req[12..14], &8080u16.to_be_bytes());
    }

    #[test]
    fn ipv6_target_addr_feature() {
        let target = TargetAddr::Ip("[2001:db8::1]:8443".parse().unwrap());
        let req = build_connect_request(&target, false, None).unwrap();
        assert_eq!(req[7], ADDR_IPV6);
        let addr_len = u16::from_be_bytes([req[5], req[6]]) as usize;
        // ATYP(1)+IPv6(16)+port(2) = 19.
        assert_eq!(addr_len, 19);
    }

    #[tokio::test]
    async fn reads_ok_status_and_stops_at_tunnel_data() {
        // VER, STATUS_OK, FEALEN=0, then tunnel bytes.
        let response = [0x01u8, 0x00, 0x00, 0x00, b'D', b'A', b'T', b'A'];
        let mut reader: &[u8] = &response;
        read_connect_response(&mut reader).await.unwrap();
        let mut rest = Vec::new();
        reader.read_to_end(&mut rest).await.unwrap();
        assert_eq!(rest, b"DATA");
    }

    #[tokio::test]
    async fn drains_response_features_before_tunnel() {
        // FEALEN=3 with a 3-byte feature body, then tunnel data.
        let response = [0x01u8, 0x00, 0x00, 0x03, 0xAA, 0xBB, 0xCC, b'X'];
        let mut reader: &[u8] = &response;
        read_connect_response(&mut reader).await.unwrap();
        let mut rest = Vec::new();
        reader.read_to_end(&mut rest).await.unwrap();
        assert_eq!(rest, b"X");
    }

    #[tokio::test]
    async fn rejects_non_ok_status() {
        let response = [0x01u8, 0x02, 0x00, 0x00];
        let mut reader: &[u8] = &response;
        let err = read_connect_response(&mut reader).await.unwrap_err();
        assert!(err.to_string().contains("unauthorized"), "{err}");
    }

    #[tokio::test]
    async fn rejects_bad_version() {
        let response = [0x09u8, 0x00, 0x00, 0x00];
        let mut reader: &[u8] = &response;
        assert!(read_connect_response(&mut reader).await.is_err());
    }

    #[test]
    fn parses_plain_relay_without_tls() {
        let cfg = GostRelayOutboundConfig::from_proxy(&parse_entry(
            "name: g\ntype: gost-relay\nserver: relay.example\nport: 8443\n",
        ))
        .unwrap();
        assert_eq!(cfg.server, "relay.example");
        assert_eq!(cfg.port, 8443);
        assert!(!cfg.forward);
        assert_eq!(cfg.auth, None);
        assert!(matches!(cfg.security, Security::None));
    }

    #[test]
    fn parses_credentials() {
        let cfg = GostRelayOutboundConfig::from_proxy(&parse_entry(
            "name: g\ntype: gost-relay\nserver: relay.example\nport: 8443\nusername: bob\npassword: secret\n",
        ))
        .unwrap();
        assert_eq!(cfg.auth, Some(("bob".to_string(), "secret".to_string())));
    }

    #[test]
    fn password_only_credentials_are_sent() {
        let cfg = GostRelayOutboundConfig::from_proxy(&parse_entry(
            "name: g\ntype: gost-relay\nserver: relay.example\nport: 8443\npassword: secret\n",
        ))
        .unwrap();
        assert_eq!(cfg.auth, Some((String::new(), "secret".to_string())));
    }

    #[test]
    fn tls_true_yields_tls_with_sni_default() {
        let cfg = GostRelayOutboundConfig::from_proxy(&parse_entry(
            "name: g\ntype: gost-relay\nserver: relay.example\nport: 443\ntls: true\nskip-cert-verify: true\n",
        ))
        .unwrap();
        match cfg.security {
            Security::Tls(tls) => {
                // SNI falls back to the dial host when unset.
                assert_eq!(tls.server_name.as_deref(), Some("relay.example"));
                assert!(tls.skip_cert_verify);
            }
            other => panic!("expected TLS security, got {other:?}"),
        }
    }

    #[test]
    fn explicit_sni_overrides_server() {
        let cfg = GostRelayOutboundConfig::from_proxy(&parse_entry(
            "name: g\ntype: gost-relay\nserver: relay.example\nport: 443\ntls: true\nsni: cdn.example\n",
        ))
        .unwrap();
        match cfg.security {
            Security::Tls(tls) => assert_eq!(tls.server_name.as_deref(), Some("cdn.example")),
            other => panic!("expected TLS security, got {other:?}"),
        }
    }

    #[test]
    fn forward_flag_parsed() {
        let cfg = GostRelayOutboundConfig::from_proxy(&parse_entry(
            "name: g\ntype: gost-relay\nserver: relay.example\nport: 8443\nforward: true\n",
        ))
        .unwrap();
        assert!(cfg.forward);
    }

    #[test]
    fn mux_is_rejected() {
        let err = GostRelayOutboundConfig::from_proxy(&parse_entry(
            "name: g\ntype: gost-relay\nserver: relay.example\nport: 8443\nmux: true\n",
        ))
        .unwrap_err();
        assert!(err.to_string().contains("mux"), "{err}");
    }

    #[test]
    fn mtls_client_cert_is_rejected() {
        let err = GostRelayOutboundConfig::from_proxy(&parse_entry(
            "name: g\ntype: gost-relay\nserver: relay.example\nport: 443\ntls: true\ncertificate: x\nprivate-key: y\n",
        ))
        .unwrap_err();
        assert!(err.to_string().contains("mTLS"), "{err}");
    }

    #[test]
    fn missing_server_is_rejected() {
        let err =
            GostRelayOutboundConfig::from_proxy(&parse_entry("name: g\ntype: gost-relay\nport: 8443\n")).unwrap_err();
        assert!(err.to_string().contains("server"), "{err}");
    }
}
