//! End-to-end proof that traffic flows through a Hysteria2 (QUIC) outbound:
//! a SOCKS5 client -> gripe inbound -> Hysteria2 outbound -> fake Hysteria2 server.
//!
//! The fake server runs on a real QUIC endpoint (quinn, the same vendored
//! rustls fork) speaking ALPN `h3`. It authenticates the client over HTTP/3
//! (validating `POST /auth` with the `Hysteria-Auth` password header, then
//! replying `233`), then accepts the *raw* QUIC proxy stream, parses the
//! `TCPRequest` (frame id `0x401` + `host:port` address), answers with a
//! success `TCPResponse`, and echoes the relayed payload. This exercises the
//! full client path: QUIC handshake, HTTP/3 auth, TCPRequest framing, and
//! bidirectional relay through the kernel's SOCKS5 inbound.

use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::sync::Arc;

use bytes::Bytes;
use learn_gripe::{Congestion, GripeConfig, GripeKernel, Hysteria2OutboundConfig, OutboundMode};
use quinn::Endpoint;
use quinn::crypto::rustls::QuicServerConfig;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::sync::oneshot;

const TEST_CERT: &str = include_str!("data/vless_tls_cert.pem");
const TEST_KEY: &str = include_str!("data/vless_tls_key.pem");

const PASSWORD: &str = "correct horse battery staple";
const MESSAGE: &[u8] = b"the quick brown fox jumps over the lazy dog";

/// Build a quinn server config from the baked test cert/key, offering the "h3"
/// ALPN the client defaults to.
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

/// Run the fake Hysteria2 server: authenticate over HTTP/3, parse the
/// `TCPRequest` target, answer OK, and echo the relayed payload. Reports the
/// parsed target address string.
async fn run_server(endpoint: Endpoint, target_tx: oneshot::Sender<String>) {
    let conn = endpoint.accept().await.unwrap().await.unwrap();
    // A cheap clone for the raw proxy stream; the HTTP/3 connection takes the
    // other handle. The proxy stream is opened by the client only after auth, so
    // accepting it after the HTTP/3 exchange avoids racing the h3 accept loop.
    let proxy_conn = conn.clone();

    // --- HTTP/3 authentication (POST /auth -> 233) ---
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
    assert!(
        request.headers().contains_key("hysteria-padding"),
        "padding header present"
    );
    let response = http::Response::builder().status(233).body(()).unwrap();
    stream.send_response(response).await.unwrap();
    stream.finish().await.unwrap();

    // --- Raw QUIC proxy stream: TCPRequest -> TCPResponse + echo ---
    let (mut send, mut recv) = proxy_conn.accept_bi().await.unwrap();
    assert_eq!(read_varint(&mut recv).await, 0x401, "TCPRequest frame id");
    let address = String::from_utf8(read_varint_bytes(&mut recv).await).unwrap();
    target_tx.send(address).unwrap();
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
async fn relays_through_hysteria2_outbound() {
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
        })),
    })
    .await
    .unwrap();

    let dummy_target = SocketAddr::from((Ipv4Addr::new(93, 184, 216, 34), 443));
    let mut conn = socks5_connect(handle.local_addr(), dummy_target).await;

    conn.write_all(MESSAGE).await.unwrap();
    conn.flush().await.unwrap();

    let mut echo = vec![0u8; MESSAGE.len()];
    conn.read_exact(&mut echo).await.unwrap();
    assert_eq!(echo, MESSAGE, "payload relayed and echoed verbatim through Hysteria2");

    let parsed = target_rx.await.unwrap();
    assert_eq!(
        parsed,
        dummy_target.to_string(),
        "server parsed the TCPRequest target address"
    );

    drop(conn);
    server.await.unwrap();
}
