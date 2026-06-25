//! Trojan outbound.
//!
//! Implements the Trojan request framing only; the transport (tcp/ws/grpc/
//! xhttp/httpupgrade/h2) and security (none/tls/reality) layers it runs over
//! are provided by [`crate::transport`] via the shared
//! [`crate::transport::build_layers`], so this module is purely the protocol
//! layer. Because security and transport are orthogonal, Trojan works over
//! every supported transport and (thanks to PR-6a) over REALITY automatically.
//!
//! Trojan normally rides TLS — that is the whole point of the protocol — so
//! security defaults to TLS unless `tls: false` is set explicitly (REALITY via
//! `reality-opts` takes precedence). The `flow: xtls-rprx-vision` layer is not
//! implemented and is rejected by [`crate::transport::build_layers`] rather than
//! silently mis-encoded.
//!
//! Wire format (client → server request, sent before any payload):
//! ```text
//! +-----------------------------+-------+----------------------------+-------+
//! | hex(SHA224(password)) (56)  | CRLF  | cmd(1) atyp(1) addr port   | CRLF  |
//! +-----------------------------+-------+----------------------------+-------+
//! ```
//! `cmd` is 0x01 (CONNECT). The address block is the SOCKS5 layout: `atyp`
//! (0x01 IPv4 / 0x03 domain / 0x04 IPv6), the address, then a big-endian port.
//! There is no Trojan response header: after the request the server relays the
//! stream verbatim, so reads pass straight through.

use std::net::SocketAddr;

use anyhow::{Context, Result};
use sha2::{Digest, Sha224};
use tokio::io::AsyncWriteExt;

use crate::address::TargetAddr;
use crate::outbound::BoxedStream;
use crate::proxy::ProxyEntry;
use crate::transport::{self, Security, Transport};

const CMD_CONNECT: u8 = 0x01;
const ATYP_IPV4: u8 = 0x01;
const ATYP_DOMAIN: u8 = 0x03;
const ATYP_IPV6: u8 = 0x04;

const CRLF: [u8; 2] = [0x0d, 0x0a];

/// Fully-resolved Trojan outbound parameters.
///
/// `security` and `transport` are orthogonal layers (see [`crate::transport`]):
/// e.g. `Trojan-gRPC-TLS` is `Security::Tls` + `Transport::Grpc`. The password
/// is pre-hashed into its 56-byte lowercase-hex SHA224 form, which is exactly
/// the on-wire identifier so the dial path never touches the raw secret again.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrojanOutboundConfig {
    pub server: String,
    pub port: u16,
    pub password_hash: [u8; 56],
    pub security: Security,
    pub transport: Transport,
}

impl TrojanOutboundConfig {
    /// Build an outbound config from a parsed `trojan` proxy entry.
    pub fn from_proxy(entry: &ProxyEntry) -> Result<Self> {
        let opts = &entry.options;
        let server = opts
            .server
            .clone()
            .filter(|s| !s.is_empty())
            .context("trojan: missing server")?;
        let port = opts.port.context("trojan: missing port")?;
        let password = opts
            .password
            .as_deref()
            .filter(|s| !s.is_empty())
            .context("trojan: missing password")?;
        let password_hash = hash_password(password);

        // Trojan is TLS-by-default; security and transport are orthogonal to the
        // framing and are built by the shared layer helper.
        let (security, transport) = transport::build_layers(opts, "trojan", true)?;

        Ok(Self {
            server,
            port,
            password_hash,
            security,
            transport,
        })
    }
}

/// Connect a Trojan outbound to `target` and return a relay-ready stream with
/// the request header already sent. Trojan has no response header, so the
/// transport stream is handed back as-is for relaying.
pub async fn connect(config: &TrojanOutboundConfig, target: &TargetAddr) -> Result<BoxedStream> {
    let mut stream = transport::establish(&config.server, config.port, &config.security, &config.transport).await?;
    let header = encode_request_header(&config.password_hash, CMD_CONNECT, target);
    stream.write_all(&header).await.context("trojan: send request header")?;
    Ok(stream)
}

/// Compute the on-wire Trojan password identifier: the lowercase hex of
/// `SHA224(password)` (28 bytes → 56 ASCII hex bytes). SHA224 is delegated to
/// the vetted `sha2` crate; only the hex rendering is done here.
fn hash_password(password: &str) -> [u8; 56] {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let digest = Sha224::digest(password.as_bytes());
    let mut out = [0u8; 56];
    for (i, byte) in digest.iter().enumerate() {
        out[i * 2] = HEX[(byte >> 4) as usize];
        out[i * 2 + 1] = HEX[(byte & 0x0f) as usize];
    }
    out
}

/// Encode the Trojan request header for a CONNECT to `target`.
fn encode_request_header(password_hash: &[u8; 56], command: u8, target: &TargetAddr) -> Vec<u8> {
    let mut buf = Vec::with_capacity(56 + 2 + 2 + 256 + 2);
    buf.extend_from_slice(password_hash);
    buf.extend_from_slice(&CRLF);
    buf.push(command);
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
    buf.extend_from_slice(&target.port().to_be_bytes());
    buf.extend_from_slice(&CRLF);
    buf
}

#[cfg(test)]
mod tests {
    use std::net::{Ipv4Addr, SocketAddr};

    use super::*;
    use crate::proxy::ProxyEntry;
    use crate::tls::ClientFingerprint;

    fn parse_entry(yaml: &str) -> ProxyEntry {
        serde_yaml_ng::from_str(yaml).expect("parse proxy entry")
    }

    fn zero_public_key_b64() -> String {
        // 32 zero bytes, standard base64 (`AAAA...=`), as a REALITY public-key.
        "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=".to_string()
    }

    #[test]
    fn hashes_password_to_lowercase_sha224_hex() {
        // Known vector: SHA224("password").
        let hash = hash_password("password");
        let expected = b"d63dc919e201d7bc4c825630d2cf25fdc93d4b2f0d46706d29038d01";
        assert_eq!(&hash, expected);
        assert_eq!(hash.len(), 56);
    }

    #[test]
    fn encodes_domain_target_header() {
        let hash = hash_password("hunter2");
        let target = TargetAddr::Domain("example.com".to_string(), 443);
        let header = encode_request_header(&hash, CMD_CONNECT, &target);

        // hex(SHA224) + CRLF
        assert_eq!(&header[..56], &hash);
        assert_eq!(&header[56..58], &CRLF);
        // cmd + atyp(domain) + len + host + port + CRLF
        assert_eq!(header[58], CMD_CONNECT);
        assert_eq!(header[59], ATYP_DOMAIN);
        assert_eq!(header[60], "example.com".len() as u8);
        assert_eq!(&header[61..72], b"example.com");
        assert_eq!(&header[72..74], &443u16.to_be_bytes());
        assert_eq!(&header[74..76], &CRLF);
    }

    #[test]
    fn encodes_ipv4_target_header() {
        let hash = hash_password("pw");
        let target = TargetAddr::Ip(SocketAddr::from((Ipv4Addr::new(1, 2, 3, 4), 8443)));
        let header = encode_request_header(&hash, CMD_CONNECT, &target);

        assert_eq!(header[58], CMD_CONNECT);
        assert_eq!(header[59], ATYP_IPV4);
        assert_eq!(&header[60..64], &[1, 2, 3, 4]);
        assert_eq!(&header[64..66], &8443u16.to_be_bytes());
        assert_eq!(&header[66..68], &CRLF);
    }

    #[test]
    fn defaults_to_tls_security() {
        let yaml = "name: t\ntype: trojan\nserver: example.com\nport: 443\npassword: secret\n";
        let cfg = TrojanOutboundConfig::from_proxy(&parse_entry(yaml)).unwrap();
        assert!(matches!(cfg.security, Security::Tls(_)));
        assert!(matches!(cfg.transport, Transport::Tcp));
        assert_eq!(cfg.password_hash, hash_password("secret"));
    }

    #[test]
    fn tls_false_yields_plaintext_security() {
        let yaml = "name: t\ntype: trojan\nserver: example.com\nport: 80\npassword: secret\ntls: false\n";
        let cfg = TrojanOutboundConfig::from_proxy(&parse_entry(yaml)).unwrap();
        assert!(matches!(cfg.security, Security::None));
    }

    #[test]
    fn missing_password_is_rejected() {
        let yaml = "name: t\ntype: trojan\nserver: example.com\nport: 443\n";
        let err = TrojanOutboundConfig::from_proxy(&parse_entry(yaml)).unwrap_err();
        assert!(err.to_string().contains("password"), "got: {err}");
    }

    #[test]
    fn missing_server_is_rejected() {
        let yaml = "name: t\ntype: trojan\nport: 443\npassword: secret\n";
        let err = TrojanOutboundConfig::from_proxy(&parse_entry(yaml)).unwrap_err();
        assert!(err.to_string().contains("server"), "got: {err}");
    }

    #[test]
    fn reality_opts_map_to_reality_security() {
        let yaml = format!(
            "name: t\ntype: trojan\nserver: example.com\nport: 443\npassword: secret\n\
             servername: www.cloudflare.com\nclient-fingerprint: chrome\n\
             reality-opts:\n  public-key: {}\n  short-id: 0123abcd\n",
            zero_public_key_b64()
        );
        let cfg = TrojanOutboundConfig::from_proxy(&parse_entry(&yaml)).unwrap();
        match cfg.security {
            Security::Reality(r) => {
                assert_eq!(r.server_name, "www.cloudflare.com");
                assert_eq!(r.public_key, [0u8; 32]);
                assert_eq!(r.short_id, vec![0x01, 0x23, 0xab, 0xcd]);
                assert_eq!(r.client_fingerprint, Some(ClientFingerprint::Chrome));
            }
            other => panic!("expected REALITY security, got {other:?}"),
        }
    }

    #[test]
    fn grpc_forces_h2_alpn() {
        let yaml = "name: t\ntype: trojan\nserver: example.com\nport: 443\npassword: secret\n\
             network: grpc\ngrpc-opts:\n  grpc-service-name: TunService\n";
        let cfg = TrojanOutboundConfig::from_proxy(&parse_entry(yaml)).unwrap();
        assert!(matches!(cfg.transport, Transport::Grpc(_)));
        match cfg.security {
            Security::Tls(tls) => assert_eq!(tls.alpn, vec!["h2".to_string()]),
            other => panic!("expected TLS security, got {other:?}"),
        }
    }

    #[test]
    fn h2_without_tls_is_rejected() {
        let yaml = "name: t\ntype: trojan\nserver: example.com\nport: 443\npassword: secret\n\
             network: h2\ntls: false\n";
        let err = TrojanOutboundConfig::from_proxy(&parse_entry(yaml)).unwrap_err();
        assert!(err.to_string().contains("h2 transport requires TLS"), "got: {err}");
    }

    #[test]
    fn flow_is_rejected() {
        let yaml = "name: t\ntype: trojan\nserver: example.com\nport: 443\npassword: secret\n\
             flow: xtls-rprx-vision\n";
        let err = TrojanOutboundConfig::from_proxy(&parse_entry(yaml)).unwrap_err();
        assert!(err.to_string().contains("flow"), "got: {err}");
    }
}
