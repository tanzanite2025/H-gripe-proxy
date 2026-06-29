//! End-to-end proof that UDP rides a MASQUE CONNECT-UDP (QUIC/HTTP3) outbound:
//! SOCKS5 UDP ASSOCIATE -> gripe inbound -> MASQUE CONNECT-UDP tunnel -> fake
//! proxy.
//!
//! The fake proxy runs on a real QUIC endpoint (quinn, the same vendored rustls
//! fork) speaking ALPN `h3`. It accepts the client's **extended CONNECT**
//! request (RFC 8441), asserting `:method = CONNECT`, `:protocol = connect-udp`,
//! and the RFC 9298 path `/.well-known/masque/udp/{host}/{port}/`, then replies
//! `200`. It relays UDP as **HTTP Datagrams** (RFC 9297) carried in QUIC
//! datagram frames: each is `varint(quarter_stream_id) varint(context_id)
//! payload`, with quarter stream id `0` (the single request stream) and context
//! id `0` (raw UDP). The proxy echoes each UDP payload back verbatim, framed the
//! same way. We cover a small payload and a payload near the link MTU.

use std::net::{Ipv4Addr, SocketAddr};
use std::sync::Arc;

use bytes::Bytes;
use h3::ext::Protocol;
use learn_gripe::{Congestion, GripeConfig, GripeKernel, MasqueOutboundConfig, OutboundMode};
use quinn::Endpoint;
use quinn::crypto::rustls::QuicServerConfig;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpStream, UdpSocket};
use tokio::sync::oneshot;

const TEST_CERT: &str = include_str!("data/vless_tls_cert.pem");
const TEST_KEY: &str = include_str!("data/vless_tls_key.pem");

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

/// Run the fake MASQUE proxy: accept the extended CONNECT-UDP request (replying
/// `200`), then echo each HTTP Datagram's UDP payload back verbatim. Reports the
/// target parsed from the request path.
async fn run_server(endpoint: Endpoint, target_tx: oneshot::Sender<String>) {
    let conn = endpoint.accept().await.unwrap().await.unwrap();
    let dgram_conn = conn.clone();

    let mut h3_conn = h3::server::builder()
        .enable_datagram(true)
        .enable_extended_connect(true)
        .build::<_, Bytes>(h3_quinn::Connection::new(conn))
        .await
        .unwrap();

    let resolver = h3_conn.accept().await.unwrap().expect("connect-udp request");
    let (request, mut stream) = resolver.resolve_request().await.unwrap();
    assert_eq!(request.method(), http::Method::CONNECT, "extended CONNECT method");
    assert_eq!(
        request.extensions().get::<Protocol>().copied(),
        Some(Protocol::CONNECT_UDP),
        ":protocol = connect-udp"
    );
    // Path: /.well-known/masque/udp/{host}/{port}/
    let path = request.uri().path().to_string();
    let segments: Vec<&str> = path.trim_matches('/').split('/').collect();
    assert_eq!(
        &segments[..3],
        &[".well-known", "masque", "udp"],
        "masque well-known prefix"
    );
    let target = format!("{}:{}", segments[3], segments[4]);
    target_tx.send(target).unwrap();

    let response = http::Response::builder().status(200).body(()).unwrap();
    stream.send_response(response).await.unwrap();
    // Hold the request stream and HTTP/3 connection open so the tunnel and the
    // shared QUIC connection stay alive.
    let _stream = stream;
    let _h3_conn = h3_conn;

    // --- HTTP Datagram echo: strip quarter-stream-id + context-id, echo back ---
    loop {
        let datagram = match dgram_conn.read_datagram().await {
            Ok(d) => d,
            Err(_) => break,
        };
        let mut pos = 0;
        let Some(quarter) = read_varint(&datagram, &mut pos) else {
            continue;
        };
        let Some(context) = read_varint(&datagram, &mut pos) else {
            continue;
        };
        assert_eq!(quarter, 0, "single request stream -> quarter stream id 0");
        assert_eq!(context, 0, "raw UDP rides context id 0");
        let payload = &datagram[pos..];

        let mut echo = Vec::with_capacity(2 + payload.len());
        put_varint(&mut echo, quarter);
        put_varint(&mut echo, context);
        echo.extend_from_slice(payload);
        dgram_conn.send_datagram(Bytes::from(echo)).unwrap();
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
        outbound: OutboundMode::Masque(Box::new(MasqueOutboundConfig {
            server: "127.0.0.1".to_string(),
            port: server_addr.port(),
            server_name: "example.com".to_string(),
            alpn: vec!["h3".to_string()],
            skip_cert_verify: true,
            congestion: Congestion::Bbr,
            username: None,
            password: None,
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
        "payload echoed verbatim through MASQUE CONNECT-UDP"
    );

    let parsed = target_rx.await.unwrap();
    assert_eq!(parsed, dst.to_string(), "proxy parsed the connect-udp target");

    handle.shutdown().await;
    server.abort();
}

#[tokio::test]
async fn udp_relays_single_datagram() {
    run_relay_test(b"masque udp ping".to_vec()).await;
}

#[tokio::test]
async fn udp_relays_near_mtu_datagram() {
    // A payload close to (but within) the QUIC datagram MTU exercises a full
    // single-datagram HTTP Datagram in both directions.
    let payload: Vec<u8> = (0..1100u32).map(|i| (i % 251) as u8).collect();
    run_relay_test(payload).await;
}
