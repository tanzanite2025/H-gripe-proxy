//! End-to-end proof that TUIC's 0-RTT mode (`reduce-rtt`) relays correctly:
//! a SOCKS5 client -> gripe inbound -> TUIC outbound (reduce-rtt) -> fake TUIC
//! server that issues TLS 1.3 session tickets and accepts 0-RTT early data.
//!
//! The first dial has no cached ticket and completes a full handshake (the
//! `None` branch of the client's connect path); later dials resume the session
//! and send the `Connect` request as 0-RTT early data, authenticating once the
//! handshake completes (the `Some` branch). The fake server re-derives the
//! RFC 5705 auth token and echoes the payload, so every dial — 1-RTT or
//! 0-RTT — must relay the message verbatim.

use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};
use std::sync::Arc;

use learn_gripe::{Congestion, GripeConfig, GripeKernel, OutboundMode, TuicOutboundConfig};
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

/// Read a TUIC `Address` and advance past it (value unused here).
async fn read_address(recv: &mut quinn::RecvStream) -> SocketAddr {
    let mut atyp = [0u8; 1];
    recv.read_exact(&mut atyp).await.unwrap();
    match atyp[0] {
        ATYP_IPV4 => {
            let mut addr = [0u8; 4];
            recv.read_exact(&mut addr).await.unwrap();
            let mut port = [0u8; 2];
            recv.read_exact(&mut port).await.unwrap();
            SocketAddr::from((Ipv4Addr::from(addr), u16::from_be_bytes(port)))
        }
        ATYP_IPV6 => {
            let mut addr = [0u8; 16];
            recv.read_exact(&mut addr).await.unwrap();
            let mut port = [0u8; 2];
            recv.read_exact(&mut port).await.unwrap();
            SocketAddr::from((Ipv6Addr::from(addr), u16::from_be_bytes(port)))
        }
        ATYP_DOMAIN => panic!("test uses an IPv4 target, got a domain address"),
        other => panic!("unexpected TUIC address type {other:#x}"),
    }
}

/// Handle one TUIC connection: validate auth, parse Connect, echo the payload.
/// The client may send the Connect stream as 0-RTT before the Authenticate
/// stream; quinn buffers both, so reading auth then connect works either way.
async fn serve_connection(conn: quinn::Connection) {
    let mut auth_stream = conn.accept_uni().await.unwrap();
    let auth = auth_stream.read_to_end(2 + 16 + 32).await.unwrap();
    assert_eq!(auth[0], VERSION, "auth version");
    assert_eq!(auth[1], CMD_AUTHENTICATE, "auth command");
    assert_eq!(&auth[2..18], &TEST_UUID, "auth uuid");
    let mut expected_token = [0u8; 32];
    conn.export_keying_material(&mut expected_token, &TEST_UUID, PASSWORD.as_bytes())
        .unwrap();
    assert_eq!(&auth[18..50], &expected_token, "auth token (server-side re-derivation)");

    let (mut send, mut recv) = conn.accept_bi().await.unwrap();
    let mut header = [0u8; 2];
    recv.read_exact(&mut header).await.unwrap();
    assert_eq!(header, [VERSION, CMD_CONNECT], "connect header");
    let _target = read_address(&mut recv).await;

    let mut payload = vec![0u8; MESSAGE.len()];
    recv.read_exact(&mut payload).await.unwrap();
    send.write_all(&payload).await.unwrap();
    send.finish().unwrap();
    conn.closed().await;
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
        outbound: OutboundMode::Tuic(Box::new(TuicOutboundConfig {
            server: "127.0.0.1".to_string(),
            port: server_addr.port(),
            uuid: TEST_UUID,
            password: PASSWORD.to_string(),
            server_name: "tuic-0rtt.example".to_string(),
            alpn: vec!["h3".to_string()],
            skip_cert_verify: true,
            congestion: Congestion::Bbr,
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
