//! End-to-end proof that Hysteria2's 0-RTT mode (`reduce-rtt`) relays correctly:
//! a SOCKS5 client -> gripe inbound -> Hysteria2 outbound (reduce-rtt) -> fake
//! Hysteria2 server that issues TLS 1.3 session tickets and accepts 0-RTT early
//! data.
//!
//! The first dial has no cached ticket and completes a full handshake (the
//! `None`/`tcp_handshake` branch of the client's connect path); later dials
//! resume the session and send the HTTP/3 `/auth` POST and the `TCPRequest` as
//! 0-RTT early data, confirming acceptance before reading the responses. The
//! fake server validates auth, parses the `TCPRequest`, and echoes the payload,
//! so every dial — 1-RTT or 0-RTT — must relay the message verbatim.

use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::sync::Arc;

use bytes::Bytes;
use learn_gripe::{Congestion, GripeConfig, GripeKernel, Hysteria2OutboundConfig, OutboundMode};
use quinn::Endpoint;
use quinn::crypto::rustls::QuicServerConfig;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

const TEST_CERT: &str = include_str!("data/vless_tls_cert.pem");
const TEST_KEY: &str = include_str!("data/vless_tls_key.pem");

const PASSWORD: &str = "correct horse battery staple";
const MESSAGE: &[u8] = b"the quick brown fox jumps over the lazy dog";
const DIALS: usize = 4;

/// quinn server config that offers "h3", issues TLS 1.3 tickets, and accepts
/// 0-RTT early data (`max_early_data_size = 0xffff_ffff`, the only non-zero
/// value QUIC allows, paired with the default stateful session cache).
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
    crypto.max_early_data_size = u32::MAX;
    let quic = QuicServerConfig::try_from(crypto).unwrap();
    quinn::ServerConfig::with_crypto(Arc::new(quic))
}

/// Read a QUIC variable-length integer from a quinn recv stream.
async fn read_varint(recv: &mut quinn::RecvStream) -> u64 {
    let mut first = [0u8; 1];
    recv.read_exact(&mut first).await.unwrap();
    let len = 1usize << (first[0] >> 6);
    let mut value = (first[0] & 0x3f) as u64;
    let mut rest = [0u8; 7];
    recv.read_exact(&mut rest[..len - 1]).await.unwrap();
    for &b in &rest[..len - 1] {
        value = (value << 8) | b as u64;
    }
    value
}

/// Read a varint-length-prefixed byte string from a quinn recv stream.
async fn read_varint_bytes(recv: &mut quinn::RecvStream) -> Vec<u8> {
    let len = read_varint(recv).await as usize;
    let mut buf = vec![0u8; len];
    recv.read_exact(&mut buf).await.unwrap();
    buf
}

/// Handle one Hysteria2 connection: authenticate over HTTP/3, parse the
/// `TCPRequest`, answer OK, and echo the relayed payload. The client may send
/// auth and the proxy stream as 0-RTT early data; quinn buffers both, so the
/// usual accept order works regardless of 1-RTT vs 0-RTT.
async fn serve_connection(conn: quinn::Connection) {
    let proxy_conn = conn.clone();

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
    let response = http::Response::builder().status(233).body(()).unwrap();
    stream.send_response(response).await.unwrap();
    stream.finish().await.unwrap();

    let (mut send, mut recv) = proxy_conn.accept_bi().await.unwrap();
    assert_eq!(read_varint(&mut recv).await, 0x401, "TCPRequest frame id");
    let _address = read_varint_bytes(&mut recv).await;
    let _padding = read_varint_bytes(&mut recv).await;

    // TCPResponse: status OK (0x00), empty message, empty padding.
    send.write_all(&[0x00, 0x00, 0x00]).await.unwrap();

    let mut payload = vec![0u8; MESSAGE.len()];
    recv.read_exact(&mut payload).await.unwrap();
    send.write_all(&payload).await.unwrap();
    send.finish().unwrap();

    proxy_conn.closed().await;
    drop(h3_conn);
}

/// Accept connections forever, serving each on its own task.
async fn run_server(endpoint: Endpoint) {
    while let Some(incoming) = endpoint.accept().await {
        tokio::spawn(async move {
            if let Ok(conn) = incoming.await {
                serve_connection(conn).await;
            }
        });
    }
}

/// Drive a minimal SOCKS5 CONNECT to `target` through the kernel inbound.
async fn socks5_connect(proxy: SocketAddr, target: SocketAddr) -> TcpStream {
    let mut stream = TcpStream::connect(proxy).await.unwrap();
    stream.write_all(&[0x05, 0x01, 0x00]).await.unwrap();
    let mut selection = [0u8; 2];
    stream.read_exact(&mut selection).await.unwrap();
    assert_eq!(selection, [0x05, 0x00]);

    let ip = match target.ip() {
        IpAddr::V4(v4) => v4.octets(),
        IpAddr::V6(_) => panic!("test uses IPv4"),
    };
    let mut request = vec![0x05, 0x01, 0x00, 0x01];
    request.extend_from_slice(&ip);
    request.extend_from_slice(&target.port().to_be_bytes());
    stream.write_all(&request).await.unwrap();

    let mut reply = [0u8; 10];
    stream.read_exact(&mut reply).await.unwrap();
    assert_eq!(reply[0], 0x05);
    assert_eq!(reply[1], 0x00, "SOCKS5 reply should be success");
    stream
}

#[tokio::test]
async fn reduce_rtt_relays_across_resumed_dials() {
    let endpoint = Endpoint::server(server_config(), (Ipv4Addr::LOCALHOST, 0).into()).unwrap();
    let server_addr = endpoint.local_addr().unwrap();
    let server = tokio::spawn(run_server(endpoint));

    let handle = GripeKernel::start(GripeConfig {
        socks_listen: SocketAddr::from((Ipv4Addr::LOCALHOST, 0)),
        outbound: OutboundMode::Hysteria2(Box::new(Hysteria2OutboundConfig {
            server: "127.0.0.1".to_string(),
            port: server_addr.port(),
            password: PASSWORD.to_string(),
            server_name: "hysteria2-0rtt.example".to_string(),
            alpn: vec!["h3".to_string()],
            skip_cert_verify: true,
            congestion: Congestion::Bbr,
            obfs: None,
            port_hop: None,
            reduce_rtt: true,
        })),
    })
    .await
    .unwrap();

    let dummy_target = SocketAddr::from((Ipv4Addr::new(93, 184, 216, 34), 443));

    // The first dial completes a full handshake; subsequent dials resume the
    // cached session and exercise the 0-RTT early-data path. Every dial must
    // relay the message verbatim.
    for dial in 0..DIALS {
        let mut conn = socks5_connect(handle.local_addr(), dummy_target).await;
        conn.write_all(MESSAGE).await.unwrap();
        conn.flush().await.unwrap();

        let mut echo = vec![0u8; MESSAGE.len()];
        conn.read_exact(&mut echo).await.unwrap();
        assert_eq!(echo, MESSAGE, "payload relayed and echoed on dial {dial}");
        drop(conn);

        // Give the just-issued NewSessionTicket time to reach the client cache
        // so the next dial can resume.
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    }

    server.abort();
}
