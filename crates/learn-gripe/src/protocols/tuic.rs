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
//! UDP relay uses the `Packet` command in `native` mode: each datagram is sent
//! as a QUIC datagram frame over the authenticated connection, fragmenting a
//! payload too large for one frame across fragments that share a packet id (the
//! target address rides only on fragment 0):
//! ```text
//! Packet: VER(0x05) TYPE(0x02) ASSOC_ID(2) PKT_ID(2) FRAG_TOTAL(1) FRAG_ID(1)
//!         SIZE(2) [ADDR if FRAG_ID==0] PAYLOAD
//! ```
//! Reassembly lives in [`crate::protocols::quic_udp`].
//!
//! Scope: TCP relay plus UDP relay (`native` datagram mode), with optional 0-RTT
//! (`reduce-rtt`): once a TLS session ticket is cached, the TCP `Connect`
//! request is sent as 0-RTT early data while authentication waits for the
//! handshake (the RFC 5705 token needs the finished exporter). The `quic`
//! (uni-stream) UDP mode is not yet implemented, and a fresh authenticated QUIC
//! connection is established per dial (connection pooling is a follow-up).
//! `congestion-controller` is honored as a local send-rate choice.
//!
//! [RFC 5705]: https://www.rfc-editor.org/rfc/rfc5705

use std::io;
use std::net::SocketAddr;
use std::pin::Pin;
use std::sync::Mutex;
use std::sync::atomic::{AtomicU16, Ordering};
use std::task::{Context as TaskContext, Poll};

use anyhow::{Context, Result, anyhow};
use bytes::Bytes;
use quinn::{Connection, Endpoint, RecvStream, SendStream};
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};

use crate::address::TargetAddr;
use crate::config::outbound_opts::{ProxyEntry, parse_uuid};
use crate::outbound::BoxedStream;
use crate::protocols::quic_udp::{self, Reassembler};
use crate::transport::quic::{self, Congestion, QuicClientParams};

const VERSION: u8 = 0x05;
const CMD_AUTHENTICATE: u8 = 0x00;
const CMD_CONNECT: u8 = 0x01;
const CMD_PACKET: u8 = 0x02;

const ATYP_DOMAIN: u8 = 0x00;
const ATYP_IPV4: u8 = 0x01;
const ATYP_IPV6: u8 = 0x02;
const ATYP_NONE: u8 = 0xff;

/// Fixed-size header of a `Packet` datagram before the optional address:
/// VER(1) TYPE(1) ASSOC_ID(2) PKT_ID(2) FRAG_TOTAL(1) FRAG_ID(1) SIZE(2).
const PACKET_HEADER_PREFIX: usize = 10;

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
    /// Attempt a 0-RTT handshake (`reduce-rtt`): send the `Connect` request as
    /// early data on a resumed connection, authenticating concurrently once the
    /// handshake completes (the RFC 5705 token needs the finished exporter).
    pub reduce_rtt: bool,
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
            reduce_rtt: opts.reduce_rtt.unwrap_or(false),
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
            // TUIC has no packet obfuscation or port hopping.
            obfs: None,
            port_hop: None,
            zero_rtt: self.reduce_rtt,
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
    let connection = quic.connection.clone();
    let header = encode_connect_header(target);

    let (send, recv) = match quic.zero_rtt {
        // 0-RTT (`reduce-rtt`): the `Connect` request carries no secret, so send
        // it as early data right away; authentication needs the finished TLS
        // exporter, so it waits for the handshake. If the server rejects 0-RTT
        // the early stream is dead, so re-send the request on a 1-RTT stream.
        Some(accepted) => {
            let early = open_connect_stream(&connection, &header).await?;
            let accepted = accepted.await;
            authenticate(&connection, &config.uuid, &config.password)
                .await
                .context("tuic: authenticate")?;
            if accepted {
                early
            } else {
                open_connect_stream(&connection, &header).await?
            }
        }
        None => {
            authenticate(&connection, &config.uuid, &config.password)
                .await
                .context("tuic: authenticate")?;
            open_connect_stream(&connection, &header).await?
        }
    };

    Ok(Box::new(TuicStream {
        _endpoint: quic.endpoint,
        _connection: connection,
        send,
        recv,
    }))
}

/// Open a `Connect` bidirectional stream and write the request header.
async fn open_connect_stream(connection: &Connection, header: &[u8]) -> Result<(SendStream, RecvStream)> {
    let (mut send, recv) = connection.open_bi().await.context("tuic: open Connect stream")?;
    send.write_all(header).await.context("tuic: send Connect header")?;
    Ok((send, recv))
}

/// Connect a TUIC UDP relay session for `target` in `native` (QUIC datagram)
/// mode. Authenticates the connection, then carries each datagram as one or more
/// `Packet` QUIC datagram frames under a fresh association id.
pub async fn connect_udp(config: &TuicOutboundConfig, target: &TargetAddr) -> Result<TuicUdp> {
    let quic = quic::connect(&config.quic_params())
        .await
        .context("tuic: QUIC connect")?;
    let connection = quic.connection.clone();

    // UDP relay rides QUIC datagrams that need the established 1-RTT keys, and
    // the auth token needs the finished exporter, so wait out any 0-RTT handshake
    // before authenticating.
    if let Some(accepted) = quic.zero_rtt {
        accepted.await;
    }
    authenticate(&quic.connection, &config.uuid, &config.password)
        .await
        .context("tuic: authenticate")?;

    let mut address = Vec::new();
    encode_address(&mut address, target);

    Ok(TuicUdp {
        _endpoint: quic.endpoint,
        connection,
        address,
        assoc_id: random_u16(),
        next_packet_id: AtomicU16::new(0),
        reassembler: Mutex::new(Reassembler::new()),
    })
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

/// Advance `*pos` past a TUIC `Address` in `data`. Returns `None` if truncated.
fn skip_address(data: &[u8], pos: &mut usize) -> Option<()> {
    let atyp = *data.get(*pos)?;
    let len = match atyp {
        ATYP_NONE => 1,
        ATYP_IPV4 => 1 + 4 + 2,
        ATYP_IPV6 => 1 + 16 + 2,
        ATYP_DOMAIN => {
            let host_len = *data.get(*pos + 1)? as usize;
            1 + 1 + host_len + 2
        }
        _ => return None,
    };
    let end = pos.checked_add(len).filter(|p| *p <= data.len())?;
    *pos = end;
    Some(())
}

/// A random 16-bit value for an association / heartbeat id.
fn random_u16() -> u16 {
    let mut bytes = [0u8; 2];
    getrandom::fill(&mut bytes).expect("os rng");
    u16::from_be_bytes(bytes)
}

/// Encode a UDP payload into one or more TUIC `Packet` datagram fragments, each
/// sized to fit within `max_datagram`. The target address rides on fragment 0
/// only; the payload is split across the `FRAG_TOTAL` fragments.
fn encode_packet_datagrams(
    assoc_id: u16,
    packet_id: u16,
    address: &[u8],
    payload: &[u8],
    max_datagram: usize,
) -> Result<Vec<Bytes>> {
    // Every fragment is bounded by the fragment-0 overhead (which includes the
    // address), so non-zero fragments simply leave a little headroom — cheaper
    // than recomputing per fragment and always within the datagram limit.
    let overhead = PACKET_HEADER_PREFIX + address.len();
    let chunk_size = max_datagram
        .checked_sub(overhead)
        .filter(|n| *n > 0)
        .ok_or_else(|| anyhow!("tuic: QUIC datagram too small for Packet header ({max_datagram} bytes)"))?;

    let chunks = quic_udp::fragments(payload, chunk_size);
    let frag_total = u8::try_from(chunks.len()).map_err(|_| anyhow!("tuic: UDP payload needs too many fragments"))?;

    let mut datagrams = Vec::with_capacity(chunks.len());
    for (frag_id, chunk) in chunks.into_iter().enumerate() {
        let mut buf = Vec::with_capacity(overhead + chunk.len());
        buf.push(VERSION);
        buf.push(CMD_PACKET);
        buf.extend_from_slice(&assoc_id.to_be_bytes());
        buf.extend_from_slice(&packet_id.to_be_bytes());
        buf.push(frag_total);
        buf.push(frag_id as u8);
        buf.extend_from_slice(&(chunk.len() as u16).to_be_bytes());
        if frag_id == 0 {
            buf.extend_from_slice(address);
        }
        buf.extend_from_slice(chunk);
        datagrams.push(Bytes::from(buf));
    }
    Ok(datagrams)
}

/// Parse an inbound TUIC `Packet` datagram into `(packet_id, frag_id,
/// frag_total, payload)`. The association id and (fragment-0) address are
/// skipped — a session has one fixed target here. Returns `None` for a malformed
/// datagram or a non-`Packet` command.
fn parse_packet_datagram(data: &[u8]) -> Option<(u16, u8, u8, Vec<u8>)> {
    if data.len() < PACKET_HEADER_PREFIX || data[0] != VERSION || data[1] != CMD_PACKET {
        return None;
    }
    let packet_id = u16::from_be_bytes([data[4], data[5]]);
    let frag_total = data[6];
    let frag_id = data[7];
    let size = u16::from_be_bytes([data[8], data[9]]) as usize;
    let mut pos = PACKET_HEADER_PREFIX;
    if frag_id == 0 {
        skip_address(data, &mut pos)?;
    }
    let end = pos.checked_add(size).filter(|p| *p <= data.len())?;
    Some((packet_id, frag_id, frag_total, data[pos..end].to_vec()))
}

/// A TUIC UDP relay session bound to a single target. Datagrams to/from the
/// target are carried as `Packet` QUIC datagram frames over the authenticated
/// connection. The owning [`Endpoint`] and [`Connection`] are held so the
/// connection stays alive for the session's lifetime.
pub struct TuicUdp {
    _endpoint: Endpoint,
    connection: Connection,
    /// The pre-encoded TUIC address all datagrams in this session carry.
    address: Vec<u8>,
    assoc_id: u16,
    next_packet_id: AtomicU16,
    reassembler: Mutex<Reassembler>,
}

impl TuicUdp {
    /// Send one UDP datagram to the session target, fragmenting if it does not
    /// fit a single QUIC datagram.
    pub async fn send(&self, payload: &[u8]) -> Result<()> {
        let max = self
            .connection
            .max_datagram_size()
            .ok_or_else(|| anyhow!("tuic: peer does not allow QUIC datagrams (UDP relay unavailable)"))?;
        let packet_id = self.next_packet_id.fetch_add(1, Ordering::Relaxed);
        for datagram in encode_packet_datagrams(self.assoc_id, packet_id, &self.address, payload, max)? {
            self.connection
                .send_datagram(datagram)
                .map_err(|e| anyhow!("tuic: send Packet datagram: {e}"))?;
        }
        Ok(())
    }

    /// Receive the next fully reassembled UDP datagram from the target.
    pub async fn recv(&self) -> Result<Vec<u8>> {
        loop {
            let datagram = self
                .connection
                .read_datagram()
                .await
                .context("tuic: read Packet datagram")?;
            let Some((packet_id, frag_id, frag_total, payload)) = parse_packet_datagram(&datagram) else {
                continue;
            };
            if let Some(full) = self
                .reassembler
                .lock()
                .expect("reassembler mutex poisoned")
                .accept(packet_id, frag_id, frag_total, payload)
            {
                return Ok(full);
            }
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

    #[test]
    fn reduce_rtt_defaults_off_and_parses() {
        let base = format!("name: t\ntype: tuic\nserver: example.com\nport: 443\nuuid: {UUID}\npassword: pw\n");
        assert!(!TuicOutboundConfig::from_proxy(&parse_entry(&base)).unwrap().reduce_rtt);
        let yaml = format!("{base}reduce-rtt: true\n");
        assert!(TuicOutboundConfig::from_proxy(&parse_entry(&yaml)).unwrap().reduce_rtt);
    }

    fn encoded_address(target: &TargetAddr) -> Vec<u8> {
        let mut buf = Vec::new();
        encode_address(&mut buf, target);
        buf
    }

    #[test]
    fn packet_datagram_round_trips_single_fragment() {
        let target = TargetAddr::Domain("example.com".to_string(), 443);
        let address = encoded_address(&target);
        let datagrams = encode_packet_datagrams(0x1234, 7, &address, b"hello", 1200).unwrap();
        assert_eq!(datagrams.len(), 1);
        let d = &datagrams[0];
        assert_eq!(d[0], VERSION);
        assert_eq!(d[1], CMD_PACKET);
        assert_eq!(&d[2..4], &0x1234u16.to_be_bytes());
        assert_eq!(&d[4..6], &7u16.to_be_bytes());
        assert_eq!(d[6], 1); // frag total
        assert_eq!(d[7], 0); // frag id
        assert_eq!(&d[8..10], &5u16.to_be_bytes()); // size
        let (packet_id, frag_id, frag_total, payload) = parse_packet_datagram(d).unwrap();
        assert_eq!((packet_id, frag_id, frag_total), (7, 0, 1));
        assert_eq!(payload, b"hello");
    }

    #[test]
    fn packet_datagram_fragments_address_on_first_only() {
        let target = TargetAddr::Ip(SocketAddr::from((Ipv4Addr::new(1, 2, 3, 4), 53)));
        let address = encoded_address(&target);
        let payload: Vec<u8> = (0..1000u32).map(|i| i as u8).collect();
        let datagrams = encode_packet_datagrams(9, 42, &address, &payload, 64).unwrap();
        assert!(datagrams.len() > 1, "expected fragmentation");

        let mut reassembler = Reassembler::new();
        let mut recovered = None;
        for (i, d) in datagrams.iter().enumerate() {
            // Only fragment 0 carries the address: its header is longer.
            let frag_id = d[7];
            assert_eq!(frag_id as usize, i);
            let (packet_id, frag_id, frag_total, frag_payload) = parse_packet_datagram(d).unwrap();
            assert_eq!(packet_id, 42);
            assert_eq!(frag_total as usize, datagrams.len());
            if let Some(full) = reassembler.accept(packet_id, frag_id, frag_total, frag_payload) {
                recovered = Some(full);
            }
        }
        assert_eq!(recovered, Some(payload));
    }

    #[test]
    fn packet_datagram_ignores_non_packet_command() {
        // A heartbeat-style datagram (wrong command byte) is not a Packet.
        let mut bytes = vec![VERSION, CMD_CONNECT];
        bytes.extend_from_slice(&[0u8; 8]);
        assert!(parse_packet_datagram(&bytes).is_none());
    }
}
