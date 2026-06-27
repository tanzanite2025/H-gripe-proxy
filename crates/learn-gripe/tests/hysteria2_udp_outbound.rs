//! End-to-end proof that UDP rides a Hysteria2 (QUIC) outbound:
//! SOCKS5 UDP ASSOCIATE -> gripe inbound -> Hysteria2 UDP tunnel -> fake server.
//!
//! The fake server runs on a real QUIC endpoint (quinn, the same vendored rustls
//! fork) speaking ALPN `h3`. It authenticates the client over HTTP/3 (replying
//! `233` with `Hysteria-UDP: true`), then relays UDP over QUIC datagram frames:
//! it parses each Hysteria2 UDP datagram (`SESSION(4) PACKET(2) FRAG_ID(1)
//! FRAG_COUNT(1) varint(addr_len) addr payload`), reassembles fragments by
//! packet id, and echoes the whole payload back (re-fragmenting to fit the link
//! MTU). We cover a single-datagram payload and a large payload that forces
//! fragmentation in both directions.

use std::collections::HashMap;
use std::net::{Ipv4Addr, SocketAddr};
use std::sync::Arc;

use bytes::Bytes;
use learn_gripe::{Congestion, GripeConfig, GripeKernel, Hysteria2OutboundConfig, OutboundMode};
use quinn::Endpoint;
use quinn::crypto::rustls::QuicServerConfig;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpStream, UdpSocket};
use tokio::sync::oneshot;

const TEST_CERT: &str = include_str!("data/vless_tls_cert.pem");
const TEST_KEY: &str = include_str!("data/vless_tls_key.pem");

const PASSWORD: &str = "correct horse battery staple";
const UDP_HEADER_PREFIX: usize = 8;

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

fn read_varint(data: &[u8], pos: &mut usize) -> Option<u64> {
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

/// Parse a Hysteria2 UDP datagram into `(packet_id, frag_id, frag_count, addr, chunk)`.
fn parse_datagram(data: &[u8]) -> Option<(u16, u8, u8, String, Vec<u8>)> {
    if data.len() < UDP_HEADER_PREFIX {
        return None;
    }
    let packet_id = u16::from_be_bytes([data[4], data[5]]);
    let frag_id = data[6];
    let frag_count = data[7];
    let mut pos = UDP_HEADER_PREFIX;
    let addr_len = read_varint(data, &mut pos)? as usize;
    let addr = String::from_utf8(data[pos..pos + addr_len].to_vec()).ok()?;
    pos += addr_len;
    Some((packet_id, frag_id, frag_count, addr, data[pos..].to_vec()))
}

/// Encode a payload into Hysteria2 UDP datagram fragments (mirrors the client).
fn encode_datagrams(session: u32, packet_id: u16, addr: &str, payload: &[u8], max: usize) -> Vec<Bytes> {
    let overhead = UDP_HEADER_PREFIX + 1 + addr.len(); // addr_len varint is 1 byte for our short addrs
    let chunk_size = max - overhead;
    let chunks: Vec<&[u8]> = if payload.is_empty() {
        vec![&[]]
    } else {
        payload.chunks(chunk_size).collect()
    };
    let frag_count = chunks.len() as u8;
    chunks
        .into_iter()
        .enumerate()
        .map(|(frag_id, chunk)| {
            let mut buf = Vec::new();
            buf.extend_from_slice(&session.to_be_bytes());
            buf.extend_from_slice(&packet_id.to_be_bytes());
            buf.push(frag_id as u8);
            buf.push(frag_count);
            put_varint(&mut buf, addr.len() as u64);
            buf.extend_from_slice(addr.as_bytes());
            buf.extend_from_slice(chunk);
            Bytes::from(buf)
        })
        .collect()
}

/// Run the fake Hysteria2 server: authenticate over HTTP/3, then echo each UDP
/// payload over QUIC datagrams (reassembling and re-fragmenting). Reports the
/// first parsed target address.
async fn run_server(endpoint: Endpoint, target_tx: oneshot::Sender<String>) {
    let conn = endpoint.accept().await.unwrap().await.unwrap();
    let dgram_conn = conn.clone();

    // --- HTTP/3 authentication (POST /auth -> 233 with Hysteria-UDP: true) ---
    let mut h3_conn = h3::server::Connection::<_, Bytes>::new(h3_quinn::Connection::new(conn))
        .await
        .unwrap();
    let resolver = h3_conn.accept().await.unwrap().expect("auth request");
    let (request, mut stream) = resolver.resolve_request().await.unwrap();
    assert_eq!(request.uri().path(), "/auth", "auth path");
    assert_eq!(
        request.headers().get("hysteria-auth").map(|v| v.as_bytes()),
        Some(PASSWORD.as_bytes()),
        "auth password header"
    );
    let response = http::Response::builder()
        .status(233)
        .header("Hysteria-UDP", "true")
        .body(())
        .unwrap();
    stream.send_response(response).await.unwrap();
    stream.finish().await.unwrap();
    // Hold the HTTP/3 connection open so the shared QUIC connection stays alive.
    let _h3_conn = h3_conn;

    // --- UDP datagram echo: reassemble inbound fragments, echo whole payloads ---
    let mut target_tx = Some(target_tx);
    let mut pending: HashMap<u16, HashMap<u8, Vec<u8>>> = HashMap::new();
    loop {
        let datagram = match dgram_conn.read_datagram().await {
            Ok(d) => d,
            Err(_) => break,
        };
        let Some((packet_id, frag_id, frag_count, addr, chunk)) = parse_datagram(&datagram) else {
            continue;
        };
        if let Some(tx) = target_tx.take() {
            tx.send(addr.clone()).unwrap();
        }
        pending.entry(packet_id).or_default().insert(frag_id, chunk);
        if pending[&packet_id].len() as u8 != frag_count {
            continue;
        }
        // Reassemble in fragment order and echo back.
        let parts = pending.remove(&packet_id).unwrap();
        let mut payload = Vec::new();
        for i in 0..frag_count {
            payload.extend_from_slice(&parts[&i]);
        }
        let max = dgram_conn.max_datagram_size().unwrap();
        for d in encode_datagrams(0xABCD_1234, packet_id, &addr, &payload, max) {
            dgram_conn.send_datagram(d).unwrap();
        }
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
        outbound: OutboundMode::Hysteria2(Box::new(Hysteria2OutboundConfig {
            server: "127.0.0.1".to_string(),
            port: server_addr.port(),
            password: PASSWORD.to_string(),
            server_name: "example.com".to_string(),
            alpn: vec!["h3".to_string()],
            skip_cert_verify: true,
            congestion: Congestion::Bbr,
            obfs: None,
            port_hop: None,
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

    let mut buf = vec![0u8; 16 * 1024];
    let (n, from) = client.recv_from(&mut buf).await.unwrap();
    assert_eq!(from, relay, "reply must come from the relay socket");
    // SOCKS5 reply header for an IPv4 address is 3 + 1 + 4 + 2 bytes.
    let offset = 3 + 1 + 4 + 2;
    assert_eq!(
        &buf[offset..n],
        &payload[..],
        "payload echoed verbatim through Hysteria2 UDP"
    );

    let parsed = target_rx.await.unwrap();
    assert_eq!(parsed, dst.to_string(), "server parsed the datagram target address");

    handle.shutdown().await;
    server.abort();
}

#[tokio::test]
async fn udp_relays_single_datagram() {
    run_relay_test(b"hysteria2 udp ping".to_vec()).await;
}

#[tokio::test]
async fn udp_relays_fragmented_datagram() {
    // A payload well above the QUIC datagram MTU forces fragmentation and
    // reassembly in both directions.
    let payload: Vec<u8> = (0..4000u32).map(|i| (i % 251) as u8).collect();
    run_relay_test(payload).await;
}
