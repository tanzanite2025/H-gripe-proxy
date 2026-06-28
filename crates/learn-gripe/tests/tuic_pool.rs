//! Proof that TUIC pools and reuses one authenticated QUIC connection across
//! relays: several concurrent SOCKS5 CONNECTs through one gripe inbound all land
//! on a single server-accepted QUIC connection (one handshake, one auth), each
//! relay opening its own bidirectional stream.
//!
//! The fake server counts accepted connections and authenticates once per
//! connection, then serves every `Connect` bidirectional stream by echoing the
//! relayed payload. With pooling enabled (no `reduce-rtt`), N relays that are
//! open at the same time must share exactly one accepted connection.

use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use learn_gripe::{Congestion, GripeConfig, GripeKernel, OutboundMode, TuicOutboundConfig, UdpRelayMode};
use quinn::Endpoint;
use quinn::crypto::rustls::QuicServerConfig;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

const TEST_CERT: &str = include_str!("data/vless_tls_cert.pem");
const TEST_KEY: &str = include_str!("data/vless_tls_key.pem");

const TEST_UUID: [u8; 16] = [
    0x12, 0x34, 0x56, 0x78, 0x12, 0x34, 0x12, 0x34, 0x12, 0x34, 0x12, 0x34, 0x56, 0x78, 0x90, 0xab,
];
const PASSWORD: &str = "correct horse battery staple";
const MESSAGE: &[u8] = b"the quick brown fox jumps over the lazy dog";
const DIALS: usize = 4;

const VERSION: u8 = 0x05;
const CMD_AUTHENTICATE: u8 = 0x00;
const CMD_CONNECT: u8 = 0x01;
const ATYP_IPV4: u8 = 0x01;
const ATYP_IPV6: u8 = 0x02;
const ATYP_DOMAIN: u8 = 0x00;

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

/// Read and discard a TUIC `Address` (type, address, big-endian port).
async fn skip_address(recv: &mut quinn::RecvStream) {
    let mut atyp = [0u8; 1];
    recv.read_exact(&mut atyp).await.unwrap();
    let body = match atyp[0] {
        ATYP_IPV4 => 4 + 2,
        ATYP_IPV6 => 16 + 2,
        ATYP_DOMAIN => {
            let mut len = [0u8; 1];
            recv.read_exact(&mut len).await.unwrap();
            len[0] as usize + 2
        }
        other => panic!("unexpected TUIC address type {other:#x}"),
    };
    let mut rest = vec![0u8; body];
    recv.read_exact(&mut rest).await.unwrap();
}

/// Serve one accepted connection: authenticate once, then echo the payload on
/// every `Connect` bidirectional stream until the connection closes.
async fn serve_connection(conn: quinn::Connection) {
    let mut auth_stream = conn.accept_uni().await.unwrap();
    let auth = auth_stream.read_to_end(2 + 16 + 32).await.unwrap();
    assert_eq!(auth[0], VERSION, "auth version");
    assert_eq!(auth[1], CMD_AUTHENTICATE, "auth command");
    let mut expected_token = [0u8; 32];
    conn.export_keying_material(&mut expected_token, &TEST_UUID, PASSWORD.as_bytes())
        .unwrap();
    assert_eq!(&auth[18..50], &expected_token, "auth token (server-side re-derivation)");

    while let Ok((mut send, mut recv)) = conn.accept_bi().await {
        tokio::spawn(async move {
            let mut header = [0u8; 2];
            if recv.read_exact(&mut header).await.is_err() {
                return;
            }
            assert_eq!(header, [VERSION, CMD_CONNECT], "connect header");
            skip_address(&mut recv).await;
            let mut payload = vec![0u8; MESSAGE.len()];
            recv.read_exact(&mut payload).await.unwrap();
            send.write_all(&payload).await.unwrap();
            send.finish().unwrap();
        });
    }
}

/// Accept connections, counting each, and serve them concurrently.
async fn run_server(endpoint: Endpoint, conn_count: Arc<AtomicUsize>) {
    while let Some(incoming) = endpoint.accept().await {
        let conn_count = conn_count.clone();
        tokio::spawn(async move {
            let Ok(conn) = incoming.await else { return };
            conn_count.fetch_add(1, Ordering::SeqCst);
            serve_connection(conn).await;
        });
    }
}

/// Open a SOCKS5 CONNECT to `target` and return the established stream (before
/// any payload). Leaving it open keeps the kernel's outbound relay alive.
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
    assert_eq!(reply[1], 0x00, "SOCKS5 reply should be success");
    stream
}

/// Relay `MESSAGE` over an established stream and assert the echo.
async fn relay_and_check(stream: &mut TcpStream) {
    stream.write_all(MESSAGE).await.unwrap();
    stream.flush().await.unwrap();
    let mut echo = vec![0u8; MESSAGE.len()];
    stream.read_exact(&mut echo).await.unwrap();
    assert_eq!(echo, MESSAGE, "payload relayed and echoed verbatim through TUIC");
}

#[tokio::test]
async fn reuses_one_pooled_connection_across_dials() {
    let endpoint = Endpoint::server(server_config(), (Ipv4Addr::LOCALHOST, 0).into()).unwrap();
    let server_addr = endpoint.local_addr().unwrap();

    let conn_count = Arc::new(AtomicUsize::new(0));
    let server = tokio::spawn(run_server(endpoint, conn_count.clone()));

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
            udp_relay_mode: UdpRelayMode::Native,
        })),
    })
    .await
    .unwrap();

    let target = SocketAddr::from((Ipv4Addr::new(93, 184, 216, 34), 443));

    // Open every relay first and keep them all alive, so they overlap and share
    // the pooled connection rather than each re-dialling after the prior closed.
    let mut streams = Vec::with_capacity(DIALS);
    for _ in 0..DIALS {
        streams.push(socks5_connect(handle.local_addr(), target).await);
    }
    for stream in &mut streams {
        relay_and_check(stream).await;
    }

    assert_eq!(
        conn_count.load(Ordering::SeqCst),
        1,
        "all {DIALS} concurrent relays must share a single pooled QUIC connection"
    );

    drop(streams);
    handle.shutdown().await;
    server.abort();
}
