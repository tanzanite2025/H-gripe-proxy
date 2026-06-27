//! End-to-end proof that UDP rides a VLESS outbound:
//! SOCKS5 UDP ASSOCIATE -> gripe inbound -> VLESS UDP tunnel -> fake server.
//!
//! The fake server validates the VLESS request header (version, UUID, the UDP
//! command 0x02, the target address), replies with the VLESS response header,
//! then echoes each length-prefixed UDP packet (`len(2 BE) | payload`). We
//! cover `none` / `tls` security, an IPv4 and a domain destination, and a
//! `Routed` outbound resolving the datagram to the VLESS tunnel.

use std::collections::HashMap;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::sync::Arc;

use learn_gripe::{
    GripeConfig, GripeKernel, OutboundMode, Router, Security, TlsClientConfig, Transport, VlessOutboundConfig,
};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream, UdpSocket};
use tokio_rustls::TlsAcceptor;

const TEST_UUID: [u8; 16] = [
    0xb8, 0x31, 0x38, 0x1d, 0x63, 0x24, 0x4d, 0x53, 0xad, 0x4f, 0x8c, 0xda, 0x48, 0xb3, 0x08, 0x11,
];
const TEST_UUID_STR: &str = "b831381d-6324-4d53-ad4f-8cda48b30811";

const TEST_CERT: &str = include_str!("data/vless_tls_cert.pem");
const TEST_KEY: &str = include_str!("data/vless_tls_key.pem");

/// Validate the VLESS UDP request header, then echo length-prefixed packets.
async fn serve_vless_udp<S>(mut stream: S)
where
    S: AsyncRead + AsyncWrite + Unpin,
{
    let mut version = [0u8; 1];
    stream.read_exact(&mut version).await.unwrap();
    assert_eq!(version[0], 0x00, "VLESS version");

    let mut uuid = [0u8; 16];
    stream.read_exact(&mut uuid).await.unwrap();
    assert_eq!(uuid, TEST_UUID, "VLESS uuid");

    let mut addon_len = [0u8; 1];
    stream.read_exact(&mut addon_len).await.unwrap();
    assert_eq!(addon_len[0], 0, "VLESS UDP carries no addon");

    let mut command = [0u8; 1];
    stream.read_exact(&mut command).await.unwrap();
    assert_eq!(command[0], 0x02, "VLESS command should be UDP");

    let mut port = [0u8; 2];
    stream.read_exact(&mut port).await.unwrap();

    let mut atyp = [0u8; 1];
    stream.read_exact(&mut atyp).await.unwrap();
    match atyp[0] {
        0x01 => {
            let mut addr = [0u8; 4];
            stream.read_exact(&mut addr).await.unwrap();
        }
        0x03 => {
            let mut addr = [0u8; 16];
            stream.read_exact(&mut addr).await.unwrap();
        }
        0x02 => {
            let mut len = [0u8; 1];
            stream.read_exact(&mut len).await.unwrap();
            let mut host = vec![0u8; len[0] as usize];
            stream.read_exact(&mut host).await.unwrap();
        }
        other => panic!("unexpected atyp {other}"),
    }

    // VLESS response header: version + zero-length addons.
    stream.write_all(&[0x00, 0x00]).await.unwrap();

    loop {
        let mut len = [0u8; 2];
        if stream.read_exact(&mut len).await.is_err() {
            return;
        }
        let mut payload = vec![0u8; u16::from_be_bytes(len) as usize];
        if stream.read_exact(&mut payload).await.is_err() {
            return;
        }
        let mut out = Vec::with_capacity(2 + payload.len());
        out.extend_from_slice(&len);
        out.extend_from_slice(&payload);
        if stream.write_all(&out).await.is_err() {
            return;
        }
    }
}

async fn spawn_fake_vless_udp_server() -> SocketAddr {
    let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        while let Ok((stream, _)) = listener.accept().await {
            tokio::spawn(serve_vless_udp(stream));
        }
    });
    addr
}

async fn spawn_fake_vless_udp_tls_server() -> SocketAddr {
    let acceptor = tls_acceptor();
    let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        while let Ok((tcp, _)) = listener.accept().await {
            let acceptor = acceptor.clone();
            tokio::spawn(async move {
                if let Ok(tls) = acceptor.accept(tcp).await {
                    serve_vless_udp(tls).await;
                }
            });
        }
    });
    addr
}

fn tls_acceptor() -> TlsAcceptor {
    let certs = rustls_pemfile::certs(&mut TEST_CERT.as_bytes())
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    let key = rustls_pemfile::private_key(&mut TEST_KEY.as_bytes()).unwrap().unwrap();
    let config = rustls::ServerConfig::builder_with_provider(Arc::new(rustls::crypto::ring::default_provider()))
        .with_safe_default_protocol_versions()
        .unwrap()
        .with_no_client_auth()
        .with_single_cert(certs, key)
        .unwrap();
    TlsAcceptor::from(Arc::new(config))
}

fn vless(server: SocketAddr, security: Security) -> Box<VlessOutboundConfig> {
    Box::new(VlessOutboundConfig {
        server: server.ip().to_string(),
        port: server.port(),
        uuid: TEST_UUID,
        security,
        transport: Transport::Tcp,
        vision: false,
    })
}

fn tls_security() -> Security {
    Security::Tls(TlsClientConfig {
        server_name: Some("localhost".to_string()),
        alpn: Vec::new(),
        skip_cert_verify: true,
        client_fingerprint: None,
        ech_config_list: None,
    })
}

async fn socks5_greet(proxy: SocketAddr) -> TcpStream {
    let mut stream = TcpStream::connect(proxy).await.unwrap();
    stream.write_all(&[0x05, 0x01, 0x00]).await.unwrap();
    let mut selection = [0u8; 2];
    stream.read_exact(&mut selection).await.unwrap();
    assert_eq!(selection, [0x05, 0x00]);
    stream
}

async fn socks5_udp_associate(proxy: SocketAddr) -> (TcpStream, SocketAddr) {
    let mut stream = socks5_greet(proxy).await;
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

fn udp_datagram_ipv4(dst: SocketAddr, payload: &[u8]) -> Vec<u8> {
    let ip = match dst.ip() {
        IpAddr::V4(v4) => v4.octets(),
        IpAddr::V6(_) => panic!("test uses IPv4"),
    };
    let mut datagram = vec![0x00, 0x00, 0x00, 0x01];
    datagram.extend_from_slice(&ip);
    datagram.extend_from_slice(&dst.port().to_be_bytes());
    datagram.extend_from_slice(payload);
    datagram
}

fn udp_datagram_domain(host: &str, port: u16, payload: &[u8]) -> Vec<u8> {
    let mut datagram = vec![0x00, 0x00, 0x00, 0x03, host.len() as u8];
    datagram.extend_from_slice(host.as_bytes());
    datagram.extend_from_slice(&port.to_be_bytes());
    datagram.extend_from_slice(payload);
    datagram
}

fn payload_offset(buf: &[u8]) -> usize {
    match buf[3] {
        0x01 => 3 + 1 + 4 + 2,
        0x04 => 3 + 1 + 16 + 2,
        0x03 => 3 + 1 + 1 + buf[4] as usize + 2,
        other => panic!("unexpected reply atyp {other}"),
    }
}

async fn assert_udp_relays(outbound: OutboundMode, datagram: Vec<u8>, payload: &[u8]) {
    let handle = GripeKernel::start(GripeConfig {
        socks_listen: SocketAddr::from((Ipv4Addr::LOCALHOST, 0)),
        outbound,
    })
    .await
    .unwrap();

    let (_control, relay) = socks5_udp_associate(handle.local_addr()).await;
    let client = UdpSocket::bind((Ipv4Addr::LOCALHOST, 0)).await.unwrap();
    client.send_to(&datagram, relay).await.unwrap();

    let mut buf = [0u8; 2048];
    let (n, from) = client.recv_from(&mut buf).await.unwrap();
    assert_eq!(from, relay, "reply must come from the relay socket");
    let offset = payload_offset(&buf[..n]);
    assert_eq!(&buf[offset..n], payload, "payload must be echoed verbatim");

    handle.shutdown().await;
}

#[tokio::test]
async fn udp_relays_through_plaintext_vless_ipv4() {
    let server = spawn_fake_vless_udp_server().await;
    let _ = TEST_UUID_STR;
    let dst = SocketAddr::from((Ipv4Addr::new(1, 2, 3, 4), 443));
    assert_udp_relays(
        OutboundMode::Vless(vless(server, Security::None)),
        udp_datagram_ipv4(dst, b"vless udp ping"),
        b"vless udp ping",
    )
    .await;
}

#[tokio::test]
async fn udp_relays_through_plaintext_vless_domain() {
    let server = spawn_fake_vless_udp_server().await;
    assert_udp_relays(
        OutboundMode::Vless(vless(server, Security::None)),
        udp_datagram_domain("example.com", 53, b"vless domain query"),
        b"vless domain query",
    )
    .await;
}

#[tokio::test]
async fn udp_relays_through_tls_vless_ipv4() {
    let server = spawn_fake_vless_udp_tls_server().await;
    let dst = SocketAddr::from((Ipv4Addr::new(9, 9, 9, 9), 443));
    assert_udp_relays(
        OutboundMode::Vless(vless(server, tls_security())),
        udp_datagram_ipv4(dst, b"tls vless udp"),
        b"tls vless udp",
    )
    .await;
}

#[tokio::test]
async fn udp_relays_through_routed_vless() {
    let server = spawn_fake_vless_udp_server().await;
    let mut outbounds = HashMap::new();
    outbounds.insert("proxy".to_string(), OutboundMode::Vless(vless(server, Security::None)));
    let router = Router::new(outbounds, vec![], "proxy").unwrap();
    let dst = SocketAddr::from((Ipv4Addr::new(1, 2, 3, 4), 443));
    assert_udp_relays(
        OutboundMode::Routed(Box::new(router)),
        udp_datagram_ipv4(dst, b"routed vless udp"),
        b"routed vless udp",
    )
    .await;
}
