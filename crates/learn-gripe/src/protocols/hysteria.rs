//! Hysteria v1 outbound (QUIC data plane).
//!
//! Hysteria v1 (the original protocol, distinct from the rewritten Hysteria2)
//! runs a proxy over a single TLS-encrypted QUIC connection (default ALPN
//! `hysteria`). After the QUIC handshake the client drives two stages:
//!
//! 1. **Control stream**: open one bidirectional stream, write the 1-byte
//!    protocol version (`3`), then a `ClientHello { rate, auth }`; read the
//!    `ServerHello { ok, rate, message }`. A rejected hello means auth failed.
//!    The control stream is kept open for the connection's lifetime (closing it
//!    tears the session down server-side).
//! 2. **Proxy stream(s)**: for each target, open a new bidirectional stream,
//!    write a `ClientRequest { udp: false, host, port }`, read the
//!    `ServerResponse { ok, session_id, message }`, then relay raw bytes.
//!
//! Structs are serialized with the same fixed big-endian layout the reference
//! `apernet/hysteria` uses (the Go `struc` tags), with `u16` length prefixes
//! before each variable-length field:
//! ```text
//! ClientHello:     send_bps(8) recv_bps(8) auth_len(2)  auth
//! ServerHello:     ok(1) send_bps(8) recv_bps(8) msg_len(2) message
//! ClientRequest:   udp(1) host_len(2) host  port(2)
//! ServerResponse:  ok(1) session_id(4) msg_len(2) message
//! ```
//!
//! Scope: TCP relay (the `direct` proxy stream). Authentication is `auth-str`
//! (literal) or `auth` (base64); the `up`/`down` bandwidth (or `up-speed`/
//! `down-speed` in Mbps) is sent in the hello as advisory rate — the server's
//! Brutal congestion control honors `recv_bps`, while the local sender uses
//! quinn's BBR (a local-only choice that does not affect interop). **XPlus
//! packet obfuscation** (`obfs: <key>`) and **port hopping** (`ports`) run below
//! QUIC in [`crate::transport::quic_obfs`]. UDP relay and the `faketcp` /
//! `wechat-video` packet underlays are not implemented (rejected at config
//! time).

use std::io;
use std::pin::Pin;
use std::task::{Context as TaskContext, Poll};

use anyhow::{Context, Result, bail};
use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64;
use quinn::{Connection, Endpoint, RecvStream, SendStream};
use tokio::io::{AsyncRead, AsyncWrite, AsyncWriteExt, ReadBuf};

use crate::address::TargetAddr;
use crate::config::outbound_opts::ProxyEntry;
use crate::outbound::BoxedStream;
use crate::protocols::xplus::XPlus;
use crate::transport::quic::{self, Congestion, QuicClientParams};
use crate::transport::quic_obfs::{PacketObfs, PortHopConfig};

/// Hysteria v1 wire protocol version, written as the first control-stream byte.
const PROTOCOL_VERSION: u8 = 3;
/// Default ALPN when `alpn` is unset (the reference server's `DefaultALPN`).
const DEFAULT_ALPN: &str = "hysteria";
/// Bytes per second represented by one Mbps (`up-speed`/`down-speed` unit),
/// matching the reference client's `mbpsToBps = 125000`.
const MBPS_TO_BPS: u64 = 125_000;
/// Defensive cap on a server-sent `ServerHello`/`ServerResponse` message length.
const MAX_MESSAGE_LEN: u64 = 4096;

/// Fully-resolved Hysteria v1 outbound parameters.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HysteriaOutboundConfig {
    pub server: String,
    pub port: u16,
    /// Authentication payload sent in the `ClientHello` (`auth-str` bytes, or
    /// the base64-decoded `auth`; empty when neither is set).
    pub auth: Vec<u8>,
    /// TLS SNI / certificate name (`sni`, falling back to `servername`/server).
    pub server_name: String,
    pub alpn: Vec<String>,
    pub skip_cert_verify: bool,
    /// Advertised upstream rate (bytes/sec) sent in the hello; 0 = unset.
    pub send_bps: u64,
    /// Advertised downstream rate (bytes/sec) sent in the hello; 0 = unset.
    pub recv_bps: u64,
    /// XPlus packet obfuscation derived from `obfs: <key>`, or `None`.
    pub obfs: Option<XPlus>,
    /// Port hopping derived from `ports` (+ `hop-interval`), or `None`.
    pub port_hop: Option<PortHopConfig>,
}

impl HysteriaOutboundConfig {
    /// Build an outbound config from a parsed `hysteria` proxy entry.
    pub fn from_proxy(entry: &ProxyEntry) -> Result<Self> {
        let opts = &entry.options;
        let server = opts
            .server
            .clone()
            .filter(|s| !s.is_empty())
            .context("hysteria: missing server")?;
        let port = opts.port.context("hysteria: missing port")?;

        // `protocol` selects the packet underlay; only plain QUIC over UDP
        // (empty / "udp") is implemented. faketcp / wechat-video are rejected
        // rather than silently dialed as plain QUIC (which would not interop).
        if let Some(proto) = opts.protocol.as_deref().filter(|s| !s.is_empty()) {
            if !proto.eq_ignore_ascii_case("udp") {
                bail!("hysteria: protocol {proto:?} not supported (only \"udp\")");
            }
        }

        // Auth precedence mirrors the reference client: the literal `auth-str`
        // wins over base64 `auth`; an absent/empty value means no auth.
        let auth = if let Some(s) = opts.auth_str.as_deref().filter(|s| !s.is_empty()) {
            s.as_bytes().to_vec()
        } else if let Some(s) = opts.auth.as_deref().filter(|s| !s.is_empty()) {
            BASE64
                .decode(s)
                .context("hysteria: auth is not valid base64 (use auth-str for a literal string)")?
        } else {
            Vec::new()
        };

        // XPlus obfuscation XOR-masks every QUIC datagram (see
        // `transport::quic_obfs`); v1 uses the `obfs` value directly as the key
        // (there is no separate mode selector like Hysteria2's salamander).
        let obfs = opts
            .obfs
            .as_deref()
            .filter(|s| !s.is_empty())
            .map(|key| XPlus::new(key.as_bytes().to_vec()));

        let port_hop = match opts.ports.as_deref().filter(|s| !s.is_empty()) {
            None => None,
            Some(spec) => Some(PortHopConfig::parse(spec, opts.hop_interval)?),
        };

        // SNI precedence: explicit `sni`, then `servername`, then the dial host.
        let server_name = opts
            .sni
            .clone()
            .or_else(|| opts.servername.clone())
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| server.clone());

        let alpn = match &opts.alpn {
            Some(list) if !list.is_empty() => list.clone(),
            _ => vec![DEFAULT_ALPN.to_string()],
        };

        let send_bps = rate_bps(opts.up.as_deref(), opts.up_speed);
        let recv_bps = rate_bps(opts.down.as_deref(), opts.down_speed);

        Ok(Self {
            server,
            port,
            auth,
            server_name,
            alpn,
            skip_cert_verify: opts.skip_cert_verify.unwrap_or(false),
            send_bps,
            recv_bps,
            obfs,
            port_hop,
        })
    }

    fn quic_params(&self) -> QuicClientParams {
        QuicClientParams {
            server: self.server.clone(),
            port: self.port,
            server_name: self.server_name.clone(),
            alpn: self.alpn.clone(),
            skip_cert_verify: self.skip_cert_verify,
            // Hysteria v1's server picks Brutal from the advertised rate; the
            // local send-side controller is a non-interop choice, so use BBR.
            congestion: Congestion::Bbr,
            obfs: self.obfs.clone().map(PacketObfs::XPlus),
            port_hop: self.port_hop.clone(),
            // v1 has no 0-RTT auth path; always complete the 1-RTT handshake.
            zero_rtt: false,
        }
    }
}

/// Resolve a bandwidth setting to bytes/sec: prefer the `up`/`down` string (e.g.
/// `"100 Mbps"`), falling back to the numeric `up-speed`/`down-speed` (Mbps).
/// Returns 0 when neither is usable (the server then applies its own default).
fn rate_bps(text: Option<&str>, mbps: Option<u64>) -> u64 {
    if let Some(s) = text.filter(|s| !s.is_empty()) {
        if let Some(bps) = string_to_bps(s) {
            return bps;
        }
    }
    mbps.map(|m| m.saturating_mul(MBPS_TO_BPS)).unwrap_or(0)
}

/// Parse a Hysteria bandwidth string `^(\d+)\s*([KMGT]?)([Bb])ps$` into
/// bytes/sec, matching the reference `stringToBps` (lowercase `b` = bits, so the
/// value is divided by 8; `K/M/G/T` are binary multipliers). Returns `None` for
/// a malformed string.
fn string_to_bps(s: &str) -> Option<u64> {
    let s = s.trim();
    let digits_end = s.find(|c: char| !c.is_ascii_digit())?;
    if digits_end == 0 {
        return None;
    }
    let value: u64 = s[..digits_end].parse().ok()?;
    let unit = s[digits_end..].trim_start();
    let (multiplier, rest) = match unit.as_bytes().first()? {
        b'K' => (1u64 << 10, &unit[1..]),
        b'M' => (1u64 << 20, &unit[1..]),
        b'G' => (1u64 << 30, &unit[1..]),
        b'T' => (1u64 << 40, &unit[1..]),
        _ => (1u64, unit),
    };
    let bits = match rest.as_bytes().first()? {
        b'b' => true,
        b'B' => false,
        _ => return None,
    };
    if &rest[1..] != "ps" {
        return None;
    }
    let mut n = value.checked_mul(multiplier)?;
    if bits {
        n >>= 3;
    }
    Some(n)
}

/// Connect a Hysteria v1 outbound to `target` and return a relay-ready stream.
/// The QUIC connection is authenticated over the control stream and the proxy
/// stream's `ClientRequest`/`ServerResponse` handshake is complete, so the
/// caller relays payload bytes directly.
pub async fn connect(config: &HysteriaOutboundConfig, target: &TargetAddr) -> Result<BoxedStream> {
    let quic = quic::connect(&config.quic_params())
        .await
        .context("hysteria: QUIC connect")?;
    let connection = quic.connection.clone();

    // --- Control stream: version + ClientHello -> ServerHello ---
    let (mut ctl_send, mut ctl_recv) = connection.open_bi().await.context("hysteria: open control stream")?;
    let mut hello = vec![PROTOCOL_VERSION];
    encode_client_hello(&mut hello, config.send_bps, config.recv_bps, &config.auth);
    ctl_send.write_all(&hello).await.context("hysteria: send ClientHello")?;
    ctl_send.flush().await.context("hysteria: flush ClientHello")?;
    let (ok, message) = read_server_hello(&mut ctl_recv)
        .await
        .context("hysteria: read ServerHello")?;
    if !ok {
        bail!("hysteria: authentication rejected: {message}");
    }

    // --- Proxy stream: ClientRequest -> ServerResponse ---
    let (mut send, mut recv) = connection.open_bi().await.context("hysteria: open proxy stream")?;
    send.write_all(&encode_client_request(target))
        .await
        .context("hysteria: send ClientRequest")?;
    send.flush().await.context("hysteria: flush ClientRequest")?;
    let (ok, message) = read_server_response(&mut recv)
        .await
        .context("hysteria: read ServerResponse")?;
    if !ok {
        bail!("hysteria: connection rejected by server: {message}");
    }

    Ok(Box::new(HysteriaStream {
        _endpoint: quic.endpoint,
        _connection: connection,
        _control: (ctl_send, ctl_recv),
        send,
        recv,
    }))
}

/// Encode `ClientHello { rate { send_bps, recv_bps }, auth }`.
fn encode_client_hello(buf: &mut Vec<u8>, send_bps: u64, recv_bps: u64, auth: &[u8]) {
    buf.extend_from_slice(&send_bps.to_be_bytes());
    buf.extend_from_slice(&recv_bps.to_be_bytes());
    buf.extend_from_slice(&(auth.len() as u16).to_be_bytes());
    buf.extend_from_slice(auth);
}

/// Encode `ClientRequest { udp: false, host, port }` for a TCP target.
fn encode_client_request(target: &TargetAddr) -> Vec<u8> {
    let (host, port) = host_port(target);
    let mut buf = Vec::with_capacity(1 + 2 + host.len() + 2);
    buf.push(0); // udp = false
    buf.extend_from_slice(&(host.len() as u16).to_be_bytes());
    buf.extend_from_slice(host.as_bytes());
    buf.extend_from_slice(&port.to_be_bytes());
    buf
}

/// Read a `ServerHello { ok, rate, message }`, returning `(ok, message)`.
async fn read_server_hello(recv: &mut RecvStream) -> Result<(bool, String)> {
    let mut head = [0u8; 17]; // ok(1) + send_bps(8) + recv_bps(8)
    recv.read_exact(&mut head).await.context("read header")?;
    let ok = head[0] != 0;
    let message = read_u16_string(recv).await.context("read message")?;
    Ok((ok, message))
}

/// Read a `ServerResponse { ok, session_id, message }`, returning `(ok, message)`.
async fn read_server_response(recv: &mut RecvStream) -> Result<(bool, String)> {
    let mut head = [0u8; 5]; // ok(1) + session_id(4)
    recv.read_exact(&mut head).await.context("read header")?;
    let ok = head[0] != 0;
    let message = read_u16_string(recv).await.context("read message")?;
    Ok((ok, message))
}

/// Read a `u16`-length-prefixed UTF-8 (lossy) string.
async fn read_u16_string(recv: &mut RecvStream) -> Result<String> {
    let mut len = [0u8; 2];
    recv.read_exact(&mut len).await.context("read length")?;
    let len = u16::from_be_bytes(len) as u64;
    if len > MAX_MESSAGE_LEN {
        bail!("hysteria: server message too long ({len} bytes)");
    }
    let mut buf = vec![0u8; len as usize];
    recv.read_exact(&mut buf).await.context("read body")?;
    Ok(String::from_utf8_lossy(&buf).into_owned())
}

/// Split a target into `(host, port)` for the `ClientRequest` (IPv6 is sent
/// unbracketed; the reference server resolves either form).
fn host_port(target: &TargetAddr) -> (String, u16) {
    match target {
        TargetAddr::Ip(addr) => (addr.ip().to_string(), addr.port()),
        TargetAddr::Domain(host, port) => (host.clone(), *port),
    }
}

/// A relay-ready stream over a Hysteria v1 proxy QUIC stream.
///
/// The owning [`Endpoint`], the QUIC [`Connection`], and the control stream
/// halves are held so the connection / session stays alive for the relay's
/// lifetime (the reference server tears the session down if the control stream
/// closes); reads and writes delegate to the proxy QUIC stream halves.
struct HysteriaStream {
    _endpoint: Endpoint,
    _connection: Connection,
    _control: (SendStream, RecvStream),
    send: SendStream,
    recv: RecvStream,
}

impl AsyncRead for HysteriaStream {
    fn poll_read(mut self: Pin<&mut Self>, cx: &mut TaskContext<'_>, buf: &mut ReadBuf<'_>) -> Poll<io::Result<()>> {
        AsyncRead::poll_read(Pin::new(&mut self.recv), cx, buf)
    }
}

impl AsyncWrite for HysteriaStream {
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
    use std::net::SocketAddr;

    use super::*;
    use crate::config::outbound_opts::ProxyEntry;

    fn parse_entry(yaml: &str) -> ProxyEntry {
        serde_yaml_ng::from_str(yaml).expect("parse proxy entry")
    }

    #[test]
    fn string_to_bps_matches_reference_vectors() {
        // Vectors from apernet/hysteria `Test_stringToBps`.
        assert_eq!(string_to_bps("8 bps"), Some(1));
        assert_eq!(string_to_bps("9991Bps"), Some(9991));
        assert_eq!(string_to_bps("10 KBps"), Some(10240));
        assert_eq!(string_to_bps("10 Kbps"), Some(1280));
        assert_eq!(string_to_bps("10 MBps"), Some(10485760));
        assert_eq!(string_to_bps("10 Mbps"), Some(1310720));
        assert_eq!(string_to_bps("10 Gbps"), Some(1342177280));
        // Malformed strings yield None.
        assert_eq!(string_to_bps("Mbps"), None);
        assert_eq!(string_to_bps("400 Bsp"), None);
        assert_eq!(string_to_bps("9 GBbps"), None);
        assert_eq!(string_to_bps("6699E Kbps"), None);
    }

    #[test]
    fn parses_minimal_config_with_defaults() {
        let yaml = "name: h\ntype: hysteria\nserver: example.com\nport: 443\nauth-str: secret\n";
        let cfg = HysteriaOutboundConfig::from_proxy(&parse_entry(yaml)).unwrap();
        assert_eq!(cfg.server, "example.com");
        assert_eq!(cfg.port, 443);
        assert_eq!(cfg.auth, b"secret");
        assert_eq!(cfg.server_name, "example.com");
        assert_eq!(cfg.alpn, vec!["hysteria".to_string()]);
        assert!(cfg.obfs.is_none());
        assert!(cfg.port_hop.is_none());
        assert_eq!(cfg.send_bps, 0);
        assert_eq!(cfg.recv_bps, 0);
    }

    #[test]
    fn base64_auth_is_decoded_and_authstr_wins() {
        // base64("hi") = "aGk=".
        let cfg = HysteriaOutboundConfig::from_proxy(&parse_entry(
            "name: h\ntype: hysteria\nserver: a\nport: 1\nauth: aGk=\n",
        ))
        .unwrap();
        assert_eq!(cfg.auth, b"hi");

        // auth-str takes precedence over auth when both are present.
        let cfg = HysteriaOutboundConfig::from_proxy(&parse_entry(
            "name: h\ntype: hysteria\nserver: a\nport: 1\nauth: aGk=\nauth-str: literal\n",
        ))
        .unwrap();
        assert_eq!(cfg.auth, b"literal");
    }

    #[test]
    fn invalid_base64_auth_is_rejected() {
        let err = HysteriaOutboundConfig::from_proxy(&parse_entry(
            "name: h\ntype: hysteria\nserver: a\nport: 1\nauth: not base64!!\n",
        ))
        .unwrap_err();
        assert!(err.to_string().contains("base64"), "got: {err}");
    }

    #[test]
    fn honors_sni_alpn_skip_verify_and_rates() {
        let yaml = "name: h\ntype: hysteria\nserver: 1.2.3.4\nport: 8443\nauth-str: pw\n\
                    sni: hidden.example\nalpn:\n  - hy\nskip-cert-verify: true\nup: 100 Mbps\ndown-speed: 200\n";
        let cfg = HysteriaOutboundConfig::from_proxy(&parse_entry(yaml)).unwrap();
        assert_eq!(cfg.server_name, "hidden.example");
        assert_eq!(cfg.alpn, vec!["hy".to_string()]);
        assert!(cfg.skip_cert_verify);
        // "100 Mbps" is megabits/sec -> bytes/sec = 100 * 2^20 / 8.
        assert_eq!(cfg.send_bps, 100 * (1 << 20) / 8);
        assert_eq!(cfg.recv_bps, 200 * MBPS_TO_BPS);
    }

    #[test]
    fn obfs_key_enables_xplus() {
        let yaml = "name: h\ntype: hysteria\nserver: a\nport: 1\nauth-str: pw\nobfs: mysecret\n";
        let cfg = HysteriaOutboundConfig::from_proxy(&parse_entry(yaml)).unwrap();
        assert_eq!(cfg.obfs, Some(XPlus::new(b"mysecret".to_vec())));
    }

    #[test]
    fn faketcp_protocol_is_rejected() {
        let err = HysteriaOutboundConfig::from_proxy(&parse_entry(
            "name: h\ntype: hysteria\nserver: a\nport: 1\nauth-str: pw\nprotocol: faketcp\n",
        ))
        .unwrap_err();
        assert!(err.to_string().contains("not supported"), "got: {err}");
    }

    #[test]
    fn ports_enable_port_hopping() {
        let yaml = "name: h\ntype: hysteria\nserver: a\nport: 443\nauth-str: pw\nports: 20000-30000\n";
        let cfg = HysteriaOutboundConfig::from_proxy(&parse_entry(yaml)).unwrap();
        assert!(cfg.port_hop.is_some());
    }

    #[test]
    fn encodes_client_request_for_ip_and_domain() {
        let req = encode_client_request(&TargetAddr::Domain("example.com".to_string(), 443));
        assert_eq!(req[0], 0); // udp = false
        assert_eq!(&req[1..3], &11u16.to_be_bytes()); // host length
        assert_eq!(&req[3..14], b"example.com");
        assert_eq!(&req[14..16], &443u16.to_be_bytes());

        let req = encode_client_request(&TargetAddr::Ip("93.184.216.34:80".parse::<SocketAddr>().unwrap()));
        let host_len = u16::from_be_bytes([req[1], req[2]]) as usize;
        assert_eq!(&req[3..3 + host_len], b"93.184.216.34");
        assert_eq!(&req[3 + host_len..3 + host_len + 2], &80u16.to_be_bytes());
    }

    #[test]
    fn encodes_client_hello_layout() {
        let mut buf = vec![PROTOCOL_VERSION];
        encode_client_hello(&mut buf, 0x0102, 0x0304, b"abc");
        assert_eq!(buf[0], PROTOCOL_VERSION);
        assert_eq!(&buf[1..9], &0x0102u64.to_be_bytes());
        assert_eq!(&buf[9..17], &0x0304u64.to_be_bytes());
        assert_eq!(&buf[17..19], &3u16.to_be_bytes());
        assert_eq!(&buf[19..22], b"abc");
    }
}
