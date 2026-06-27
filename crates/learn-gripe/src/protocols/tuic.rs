//! TUIC v5 outbound (QUIC data plane).
//!
//! TUIC multiplexes relay tasks over a single TLS-encrypted QUIC connection.
//! This module implements the client side of the v5 protocol's TCP relay:
//!
//! 1. Dial the server over QUIC (TLS 1.3) via [`crate::transport::quic`].
//! 2. Authenticate by opening a unidirectional stream and sending an
//!    `Authenticate` command — the client UUID plus a 32-byte token derived
//!    from the QUIC TLS session with the [RFC 5705] keying-material exporter
//!    (`label = UUID`, `context = password`).
//! 3. For each target, open a bidirectional stream, send a `Connect` command
//!    (header + target address), and relay raw bytes. The server never replies
//!    to the header, so reads pass straight through to the relayed payload.
//!
//! Wire format (all integers big-endian):
//! ```text
//! Command header:  VER(0x05) TYPE
//! Authenticate:    UUID(16)  TOKEN(32)
//! Connect:         ADDR                       ; TUIC address, see below
//! Address:         TYPE(1) ADDR(var) PORT(2)  ; 0x00 domain(len+bytes),
//!                                               0x01 IPv4(4), 0x02 IPv6(16)
//! ```
//!
//! Scope: this is the TCP relay milestone. UDP relay (the `Packet` command, in
//! `native`/`quic` modes) and the 0-RTT (`reduce-rtt`) optimization are not yet
//! implemented; a fresh authenticated QUIC connection is established per dial
//! (connection pooling is a follow-up). `congestion-controller` is honored as a
//! local send-rate choice.
//!
//! [RFC 5705]: https://www.rfc-editor.org/rfc/rfc5705

use std::io;
use std::net::SocketAddr;
use std::pin::Pin;
use std::task::{Context as TaskContext, Poll};

use anyhow::{Context, Result, anyhow};
use quinn::{Connection, Endpoint, RecvStream, SendStream};
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};

use crate::address::TargetAddr;
use crate::config::outbound_opts::{ProxyEntry, parse_uuid};
use crate::outbound::BoxedStream;
use crate::transport::quic::{self, Congestion, QuicClientParams};

const VERSION: u8 = 0x05;
const CMD_AUTHENTICATE: u8 = 0x00;
const CMD_CONNECT: u8 = 0x01;

const ATYP_DOMAIN: u8 = 0x00;
const ATYP_IPV4: u8 = 0x01;
const ATYP_IPV6: u8 = 0x02;

/// Fully-resolved TUIC v5 outbound parameters.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TuicOutboundConfig {
    pub server: String,
    pub port: u16,
    pub uuid: [u8; 16],
    pub password: String,
    /// TLS SNI / certificate name (`sni`, falling back to `servername`/server).
    pub server_name: String,
    pub alpn: Vec<String>,
    pub skip_cert_verify: bool,
    pub congestion: Congestion,
}

impl TuicOutboundConfig {
    /// Build an outbound config from a parsed `tuic` proxy entry.
    pub fn from_proxy(entry: &ProxyEntry) -> Result<Self> {
        let opts = &entry.options;
        let server = opts
            .server
            .clone()
            .filter(|s| !s.is_empty())
            .context("tuic: missing server")?;
        let port = opts.port.context("tuic: missing port")?;
        let uuid = parse_uuid(opts.uuid.as_deref().context("tuic: missing uuid")?)?;
        let password = opts
            .password
            .clone()
            .filter(|s| !s.is_empty())
            .context("tuic: missing password")?;

        // SNI precedence: explicit `sni`, then `servername`, then the dial host.
        let server_name = opts
            .sni
            .clone()
            .or_else(|| opts.servername.clone())
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| server.clone());

        // QUIC always carries ALPN; TUIC deployments default to "h3".
        let alpn = match &opts.alpn {
            Some(list) if !list.is_empty() => list.clone(),
            _ => vec!["h3".to_string()],
        };

        let congestion = opts
            .congestion_controller
            .as_deref()
            .map(Congestion::parse)
            .unwrap_or(Congestion::Bbr);

        Ok(Self {
            server,
            port,
            uuid,
            password,
            server_name,
            alpn,
            skip_cert_verify: opts.skip_cert_verify.unwrap_or(false),
            congestion,
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
        }
    }
}

/// Connect a TUIC outbound to `target` and return a relay-ready stream. The
/// QUIC connection is authenticated and the `Connect` header is already sent,
/// so the caller relays payload bytes directly over the returned stream.
pub async fn connect(config: &TuicOutboundConfig, target: &TargetAddr) -> Result<BoxedStream> {
    let quic = quic::connect(&config.quic_params())
        .await
        .context("tuic: QUIC connect")?;

    authenticate(&quic.connection, &config.uuid, &config.password)
        .await
        .context("tuic: authenticate")?;

    let (mut send, recv) = quic.connection.open_bi().await.context("tuic: open Connect stream")?;
    let header = encode_connect_header(target);
    send.write_all(&header).await.context("tuic: send Connect header")?;

    Ok(Box::new(TuicStream {
        _endpoint: quic.endpoint,
        _connection: quic.connection,
        send,
        recv,
    }))
}

/// Send the `Authenticate` command on a fresh unidirectional stream. The token
/// is exported from the live QUIC TLS session, so it cannot be replayed onto a
/// different connection.
async fn authenticate(conn: &Connection, uuid: &[u8; 16], password: &str) -> Result<()> {
    let mut token = [0u8; 32];
    conn.export_keying_material(&mut token, uuid, password.as_bytes())
        .map_err(|_| anyhow!("export TUIC token from TLS session (unsupported keying-material length)"))?;

    let mut buf = Vec::with_capacity(2 + 16 + 32);
    buf.push(VERSION);
    buf.push(CMD_AUTHENTICATE);
    buf.extend_from_slice(uuid);
    buf.extend_from_slice(&token);

    let mut stream = conn.open_uni().await.context("open Authenticate stream")?;
    stream.write_all(&buf).await.context("send Authenticate")?;
    stream.finish().context("finish Authenticate stream")?;
    Ok(())
}

/// Encode a `Connect` command: the version/type header followed by the TUIC
/// target address.
fn encode_connect_header(target: &TargetAddr) -> Vec<u8> {
    let mut buf = Vec::with_capacity(2 + 1 + 256 + 2);
    buf.push(VERSION);
    buf.push(CMD_CONNECT);
    encode_address(&mut buf, target);
    buf
}

/// Append a TUIC `Address` (type, address, big-endian port) to `buf`.
fn encode_address(buf: &mut Vec<u8>, target: &TargetAddr) {
    match target {
        TargetAddr::Ip(SocketAddr::V4(addr)) => {
            buf.push(ATYP_IPV4);
            buf.extend_from_slice(&addr.ip().octets());
            buf.extend_from_slice(&addr.port().to_be_bytes());
        }
        TargetAddr::Ip(SocketAddr::V6(addr)) => {
            buf.push(ATYP_IPV6);
            buf.extend_from_slice(&addr.ip().octets());
            buf.extend_from_slice(&addr.port().to_be_bytes());
        }
        TargetAddr::Domain(host, port) => {
            buf.push(ATYP_DOMAIN);
            buf.push(host.len() as u8);
            buf.extend_from_slice(host.as_bytes());
            buf.extend_from_slice(&port.to_be_bytes());
        }
    }
}

/// A relay-ready stream over a TUIC `Connect` bidirectional QUIC stream.
///
/// The owning [`Endpoint`] and [`Connection`] are held so their background
/// driver/keep-alive stay alive for the lifetime of the relay; reads and writes
/// delegate to the QUIC stream halves.
struct TuicStream {
    _endpoint: Endpoint,
    _connection: Connection,
    send: SendStream,
    recv: RecvStream,
}

impl AsyncRead for TuicStream {
    fn poll_read(mut self: Pin<&mut Self>, cx: &mut TaskContext<'_>, buf: &mut ReadBuf<'_>) -> Poll<io::Result<()>> {
        AsyncRead::poll_read(Pin::new(&mut self.recv), cx, buf)
    }
}

impl AsyncWrite for TuicStream {
    fn poll_write(mut self: Pin<&mut Self>, cx: &mut TaskContext<'_>, buf: &[u8]) -> Poll<io::Result<usize>> {
        AsyncWrite::poll_write(Pin::new(&mut self.send), cx, buf)
    }

    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut TaskContext<'_>) -> Poll<io::Result<()>> {
        AsyncWrite::poll_flush(Pin::new(&mut self.send), cx)
    }

    fn poll_shutdown(mut self: Pin<&mut Self>, cx: &mut TaskContext<'_>) -> Poll<io::Result<()>> {
        AsyncWrite::poll_shutdown(Pin::new(&mut self.send), cx)
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

    const UUID: &str = "00000000-0000-0000-0000-000000000001";

    #[test]
    fn encodes_domain_connect_header() {
        let target = TargetAddr::Domain("example.com".to_string(), 443);
        let header = encode_connect_header(&target);
        assert_eq!(header[0], VERSION);
        assert_eq!(header[1], CMD_CONNECT);
        assert_eq!(header[2], ATYP_DOMAIN);
        assert_eq!(header[3], "example.com".len() as u8);
        assert_eq!(&header[4..15], b"example.com");
        assert_eq!(&header[15..17], &443u16.to_be_bytes());
    }

    #[test]
    fn encodes_ipv4_connect_header() {
        let target = TargetAddr::Ip(SocketAddr::from((Ipv4Addr::new(1, 2, 3, 4), 8443)));
        let header = encode_connect_header(&target);
        assert_eq!(header[0], VERSION);
        assert_eq!(header[1], CMD_CONNECT);
        assert_eq!(header[2], ATYP_IPV4);
        assert_eq!(&header[3..7], &[1, 2, 3, 4]);
        assert_eq!(&header[7..9], &8443u16.to_be_bytes());
    }

    #[test]
    fn parses_minimal_config_with_defaults() {
        let yaml = format!("name: t\ntype: tuic\nserver: example.com\nport: 443\nuuid: {UUID}\npassword: secret\n");
        let cfg = TuicOutboundConfig::from_proxy(&parse_entry(&yaml)).unwrap();
        assert_eq!(cfg.server, "example.com");
        assert_eq!(cfg.port, 443);
        assert_eq!(cfg.uuid, parse_uuid(UUID).unwrap());
        assert_eq!(cfg.password, "secret");
        // Defaults: SNI = dial host, ALPN = ["h3"], BBR, verify on.
        assert_eq!(cfg.server_name, "example.com");
        assert_eq!(cfg.alpn, vec!["h3".to_string()]);
        assert_eq!(cfg.congestion, Congestion::Bbr);
        assert!(!cfg.skip_cert_verify);
    }

    #[test]
    fn honors_sni_alpn_and_congestion() {
        let yaml = format!(
            "name: t\ntype: tuic\nserver: 1.2.3.4\nport: 443\nuuid: {UUID}\npassword: pw\n\
             sni: hidden.example\nalpn:\n  - h3\n  - h2\ncongestion-controller: cubic\n\
             skip-cert-verify: true\n"
        );
        let cfg = TuicOutboundConfig::from_proxy(&parse_entry(&yaml)).unwrap();
        assert_eq!(cfg.server_name, "hidden.example");
        assert_eq!(cfg.alpn, vec!["h3".to_string(), "h2".to_string()]);
        assert_eq!(cfg.congestion, Congestion::Cubic);
        assert!(cfg.skip_cert_verify);
    }

    #[test]
    fn missing_uuid_is_rejected() {
        let yaml = "name: t\ntype: tuic\nserver: example.com\nport: 443\npassword: secret\n";
        let err = TuicOutboundConfig::from_proxy(&parse_entry(yaml)).unwrap_err();
        assert!(err.to_string().contains("uuid"), "got: {err}");
    }

    #[test]
    fn missing_password_is_rejected() {
        let yaml = format!("name: t\ntype: tuic\nserver: example.com\nport: 443\nuuid: {UUID}\n");
        let err = TuicOutboundConfig::from_proxy(&parse_entry(&yaml)).unwrap_err();
        assert!(err.to_string().contains("password"), "got: {err}");
    }
}
