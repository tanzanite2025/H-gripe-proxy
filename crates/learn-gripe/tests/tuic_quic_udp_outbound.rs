//! End-to-end proof that UDP rides a TUIC v5 outbound in `quic` (uni-stream)
//! mode: SOCKS5 UDP ASSOCIATE -> gripe inbound -> TUIC UDP tunnel -> fake server.
//!
//! Unlike `native` mode (QUIC datagram frames, see `tuic_udp_outbound.rs`),
//! `quic` mode (`udp-relay-mode: quic`) carries each datagram on its own
//! unidirectional QUIC stream. The fake server validates the `Authenticate`
//! token, then for every inbound uni stream parses the `Packet` command
//! (`VER(0x05) TYPE(0x02) ASSOC(2) PKT(2) FRAG_TOTAL(1) FRAG_ID(1) SIZE(2) ADDR
//! payload`) and echoes the payload back on a fresh uni stream. A reliable
//! stream has no datagram-MTU ceiling, so we also cover a payload far larger
//! than a QUIC datagram to prove it rides a single stream without fragmentation.

use std::net::{Ipv4Addr, SocketAddr};
use std::sync::Arc;

use learn_gripe::{Congestion, GripeConfig, GripeKernel, OutboundMode, TuicOutboundConfig, UdpRelayMode};
use quinn::Endpoint;
use quinn::crypto::rustls::QuicServerConfig;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpStream, UdpSocket};
use tokio::sync::oneshot;

const TEST_CERT: &str = include_str!("data/vless_tls_cert.pem");
const TEST_KEY: &str = include_str!("data/vless_tls_key.pem");

const TEST_UUID: [u8; 16] = [
    0x12, 0x34, 0x56, 0x78, 0x12, 0x34, 0x12, 0x34, 0x12, 0x34, 0x12, 0x34, 0x56, 0x78, 0x90, 0xab,
];
const PASSWORD: &str = "correct horse battery staple";

const VERSION: u8 = 0x05;
const CMD_AUTHENTICATE: u8 = 0x00;
const CMD_PACKET: u8 = 0x02;
const ATYP_IPV4: u8 = 0x01;
const ATYP_IPV6: u8 = 0x02;
const ATYP_DOMAIN: u8 = 0x00;
const PACKET_HEADER_PREFIX: usize = 10;
/// 10-byte prefix + maximal domain address + max u16 payload.
const MAX_PACKET_STREAM: usize = PACKET_HEADER_PREFIX + 259 + u16::MAX as usize;

fn server_config() -> quinn::ServerConfig {
    let certs = rustls_pemfile::certs(&mut TEST_CERT.as_bytes())
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    let key = rustls_pemfile::private_key(&mut TEST_KEY.as_bytes()).unwrap().unwrap();
    let mut crypto = rustls::ServerConfig::builder_with_provider(Arc::new(rustls::crypto::ring::default_provider()))
        .with_protocol_versions(&[&rustls::version::TLS13])
        .unwrap()
        .with_no_client_auth()
        .with_single_cert(certs, key)
        .unwrap();
    crypto.alpn_protocols = vec![b"h3".to_vec()];
    let quic = QuicServerConfig::try_from(crypto).unwrap();
    quinn::ServerConfig::with_crypto(Arc::new(quic))
}

/// Byte length of the TUIC `Address` starting at `data[pos]`.
fn address_len(data: &[u8], pos: usize) -> usize {
    match data[pos] {
        ATYP_IPV4 => 1 + 4 + 2,
        ATYP_IPV6 => 1 + 16 + 2,
        ATYP_DOMAIN => 1 + 1 + data[pos + 1] as usize + 2,
        other => panic!("unexpected TUIC address type {other:#x}"),
    }
}

/// Decode a TUIC IPv4 `Address` to a `SocketAddr` (for reporting).
fn decode_ipv4(addr: &[u8]) -> SocketAddr {
    assert_eq!(addr[0], ATYP_IPV4, "test target is IPv4");
    let ip = Ipv4Addr::new(addr[1], addr[2], addr[3], addr[4]);
    let port = u16::from_be_bytes([addr[5], addr[6]]);
    SocketAddr::from((ip, port))
}

/// Parse a uni-stream `Packet` into `(addr_bytes, payload)`. `quic` mode always
/// sends a whole datagram in one packet (FRAG_TOTAL=1), so there is no
/// cross-stream reassembly to do here.
fn parse_packet(data: &[u8]) -> (Vec<u8>, Vec<u8>) {
    assert!(data.len() >= PACKET_HEADER_PREFIX, "short packet");
    assert_eq!(data[0], VERSION, "packet version");
    assert_eq!(data[1], CMD_PACKET, "packet command");
    assert_eq!(data[6], 1, "quic mode carries one fragment per stream");
    assert_eq!(data[7], 0, "fragment id");
    let size = u16::from_be_bytes([data[8], data[9]]) as usize;
    let addr_len = address_len(data, PACKET_HEADER_PREFIX);
    let addr = data[PACKET_HEADER_PREFIX..PACKET_HEADER_PREFIX + addr_len].to_vec();
    let start = PACKET_HEADER_PREFIX + addr_len;
    (addr, data[start..start + size].to_vec())
}

/// Encode a single uni-stream `Packet` (FRAG_TOTAL=1) carrying `payload`.
fn encode_packet(assoc: u16, packet_id: u16, addr: &[u8], payload: &[u8]) -> Vec<u8> {
    let mut buf = Vec::with_capacity(PACKET_HEADER_PREFIX + addr.len() + payload.len());
    buf.push(VERSION);
    buf.push(CMD_PACKET);
    buf.extend_from_slice(&assoc.to_be_bytes());
    buf.extend_from_slice(&packet_id.to_be_bytes());
    buf.push(1);
    buf.push(0);
    buf.extend_from_slice(&(payload.len() as u16).to_be_bytes());
    buf.extend_from_slice(addr);
    buf.extend_from_slice(payload);
    buf
}

/// Run the fake server: verify auth, then echo each uni-stream `Packet` back on
/// a fresh uni stream. Reports the first parsed target address.
async fn run_server(endpoint: Endpoint, target_tx: oneshot::Sender<SocketAddr>) {
    let conn = endpoint.accept().await.unwrap().await.unwrap();

    let mut auth_stream = conn.accept_uni().await.unwrap();
    let auth = auth_stream.read_to_end(2 + 16 + 32).await.unwrap();
    assert_eq!(auth[0], VERSION, "auth version");
    assert_eq!(auth[1], CMD_AUTHENTICATE, "auth command");
    assert_eq!(&auth[2..18], &TEST_UUID, "auth uuid");
    let mut expected_token = [0u8; 32];
    conn.export_keying_material(&mut expected_token, &TEST_UUID, PASSWORD.as_bytes())
        .unwrap();
    assert_eq!(&auth[18..50], &expected_token, "auth token (server-side re-derivation)");

    let mut target_tx = Some(target_tx);
    let mut packet_id = 0u16;
    loop {
        let mut stream = match conn.accept_uni().await {
            Ok(s) => s,
            Err(_) => break,
        };
        let data = stream.read_to_end(MAX_PACKET_STREAM).await.unwrap();
        let (addr, payload) = parse_packet(&data);
        if let Some(tx) = target_tx.take() {
            tx.send(decode_ipv4(&addr)).unwrap();
        }
        let echo = encode_packet(0x9999, packet_id, &addr, &payload);
        packet_id = packet_id.wrapping_add(1);
        let mut out = conn.open_uni().await.unwrap();
        out.write_all(&echo).await.unwrap();
        out.finish().unwrap();
    }
}

async fn socks5_udp_associate(proxy: SocketAddr) -> (TcpStream, SocketAddr) {
    let mut stream = TcpStream::connect(proxy).await.unwrap();
    stream.write_all(&[0x05, 0x01, 0x00]).await.unwrap();
    let mut selection = [0u8; 2];
    stream.read_exact(&mut selection).await.unwrap();
    assert_eq!(selection, [0x05, 0x00]);
    stream
        .write_all(&[0x05, 0x03, 0x00, 0x01, 0, 0, 0, 0, 0, 0])
        .await
        .unwrap();
    let mut reply = [0u8; 10];
    stream.read_exact(&mut reply).await.unwrap();
    assert_eq!(reply[1], 0x00, "expected ASSOCIATE success reply");
    let ip = Ipv4Addr::new(reply[4], reply[5], reply[6], reply[7]);
    let port = u16::from_be_bytes([reply[8], reply[9]]);
    (stream, SocketAddr::from((ip, port)))
}

fn socks5_udp_datagram(dst: SocketAddr, payload: &[u8]) -> Vec<u8> {
    let ip = match dst.ip() {
        std::net::IpAddr::V4(v4) => v4.octets(),
        std::net::IpAddr::V6(_) => panic!("test uses IPv4"),
    };
    let mut datagram = vec![0x00, 0x00, 0x00, 0x01];
    datagram.extend_from_slice(&ip);
    datagram.extend_from_slice(&dst.port().to_be_bytes());
    datagram.extend_from_slice(payload);
    datagram
}

async fn run_relay_test(payload: Vec<u8>) {
    let endpoint = Endpoint::server(server_config(), (Ipv4Addr::LOCALHOST, 0).into()).unwrap();
    let server_addr = endpoint.local_addr().unwrap();

    let (target_tx, target_rx) = oneshot::channel();
    let server = tokio::spawn(run_server(endpoint, target_tx));

    let handle = GripeKernel::start(GripeConfig {
        socks_listen: SocketAddr::from((Ipv4Addr::LOCALHOST, 0)),
        outbound: OutboundMode::Tuic(Box::new(TuicOutboundConfig {
            server: "127.0.0.1".to_string(),
            port: server_addr.port(),
            uuid: TEST_UUID,
            password: PASSWORD.to_string(),
            server_name: "example.com".to_string(),
            alpn: vec!["h3".to_string()],
            skip_cert_verify: true,
            congestion: Congestion::Bbr,
            reduce_rtt: false,
            udp_relay_mode: UdpRelayMode::Quic,
        })),
    })
    .await
    .unwrap();

    let (_control, relay) = socks5_udp_associate(handle.local_addr()).await;
    let client = UdpSocket::bind((Ipv4Addr::LOCALHOST, 0)).await.unwrap();
    let dst = SocketAddr::from((Ipv4Addr::new(93, 184, 216, 34), 443));
    client
        .send_to(&socks5_udp_datagram(dst, &payload), relay)
        .await
        .unwrap();

    let mut buf = vec![0u8; 128 * 1024];
    let (n, from) = client.recv_from(&mut buf).await.unwrap();
    assert_eq!(from, relay, "reply must come from the relay socket");
    let offset = 3 + 1 + 4 + 2;
    assert_eq!(
        &buf[offset..n],
        &payload[..],
        "payload echoed verbatim through TUIC quic-mode UDP"
    );

    let parsed = target_rx.await.unwrap();
    assert_eq!(parsed, dst, "server parsed the Packet target address");

    handle.shutdown().await;
    server.abort();
}

#[tokio::test]
async fn quic_mode_relays_single_datagram() {
    run_relay_test(b"tuic quic-mode udp ping".to_vec()).await;
}

#[tokio::test]
async fn quic_mode_relays_large_datagram() {
    // Far larger than a QUIC datagram MTU: in `native` mode this fragments, but
    // `quic` mode carries it on a single reliable stream as one Packet.
    let payload: Vec<u8> = (0..5000u32).map(|i| (i % 251) as u8).collect();
    run_relay_test(payload).await;
}
