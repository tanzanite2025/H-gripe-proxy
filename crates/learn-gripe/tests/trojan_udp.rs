//! End-to-end proof that UDP rides a Trojan outbound:
//! SOCKS5 UDP ASSOCIATE -> gripe inbound -> Trojan UDP tunnel -> fake server.
//!
//! The fake server validates the Trojan request header (56-byte hex SHA224
//! identifier, CRLF delimiters, the `UDP ASSOCIATE` command 0x03, the SOCKS5
//! target address) and then echoes each Trojan UDP packet
//! (`SOCKS5-addr | len(2) | CRLF | payload`) verbatim. We cover the `none` /
//! `tls` security layers, an IPv4 and a domain destination, and a `Routed`
//! outbound resolving the datagram to the Trojan tunnel.

use std::collections::HashMap;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::sync::Arc;

use learn_gripe::{
    GripeConfig, GripeKernel, OutboundMode, Router, Security, TlsClientConfig, Transport, TrojanOutboundConfig,
};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream, UdpSocket};
use tokio_rustls::TlsAcceptor;

const TEST_CERT: &str = include_str!("data/vless_tls_cert.pem");
const TEST_KEY: &str = include_str!("data/vless_tls_key.pem");

/// `SHA224("password")` in lowercase hex — the on-wire Trojan identifier.
const TEST_PASSWORD_HASH: &[u8; 56] = b"d63dc919e201d7bc4c825630d2cf25fdc93d4b2f0d46706d29038d01";

const CRLF: [u8; 2] = [0x0d, 0x0a];

/// Read one SOCKS5 address (`atyp + addr + port`) and return its raw bytes so
/// the echo can write them back unchanged. Returns `None` on EOF.
async fn read_addr_bytes<S>(stream: &mut S) -> Option<Vec<u8>>
where
    S: AsyncRead + Unpin,
{
    let mut atyp = [0u8; 1];
    if stream.read_exact(&mut atyp).await.is_err() {
        return None;
    }
    let mut out = vec![atyp[0]];
    match atyp[0] {
        0x01 => {
            let mut a = [0u8; 4];
            stream.read_exact(&mut a).await.ok()?;
            out.extend_from_slice(&a);
        }
        0x04 => {
            let mut a = [0u8; 16];
            stream.read_exact(&mut a).await.ok()?;
            out.extend_from_slice(&a);
        }
        0x03 => {
            let mut len = [0u8; 1];
            stream.read_exact(&mut len).await.ok()?;
            out.push(len[0]);
            let mut host = vec![0u8; len[0] as usize];
            stream.read_exact(&mut host).await.ok()?;
            out.extend_from_slice(&host);
        }
        other => panic!("unexpected atyp {other}"),
    }
    let mut port = [0u8; 2];
    stream.read_exact(&mut port).await.ok()?;
    out.extend_from_slice(&port);
    Some(out)
}

/// Validate the Trojan UDP request header, then echo every UDP packet back.
async fn serve_trojan_udp<S>(mut stream: S)
where
    S: AsyncRead + AsyncWrite + Unpin,
{
    let mut hash = [0u8; 56];
    stream.read_exact(&mut hash).await.unwrap();
    assert_eq!(&hash, TEST_PASSWORD_HASH, "trojan password hash");

    let mut delim = [0u8; 2];
    stream.read_exact(&mut delim).await.unwrap();
    assert_eq!(delim, CRLF, "trojan header CRLF");

    let mut command = [0u8; 1];
    stream.read_exact(&mut command).await.unwrap();
    assert_eq!(command[0], 0x03, "trojan command should be UDP ASSOCIATE");

    read_addr_bytes(&mut stream).await.expect("header target address");
    let mut trailing = [0u8; 2];
    stream.read_exact(&mut trailing).await.unwrap();
    assert_eq!(trailing, CRLF, "trojan request CRLF");

    loop {
        let addr = match read_addr_bytes(&mut stream).await {
            Some(a) => a,
            None => return,
        };
        let mut len = [0u8; 2];
        if stream.read_exact(&mut len).await.is_err() {
            return;
        }
        let mut crlf = [0u8; 2];
        stream.read_exact(&mut crlf).await.unwrap();
        assert_eq!(crlf, CRLF, "trojan udp packet CRLF");
        let mut payload = vec![0u8; u16::from_be_bytes(len) as usize];
        stream.read_exact(&mut payload).await.unwrap();

        let mut out = addr;
        out.extend_from_slice(&len);
        out.extend_from_slice(&CRLF);
        out.extend_from_slice(&payload);
        if stream.write_all(&out).await.is_err() {
            return;
        }
    }
}

async fn spawn_fake_trojan_udp_server() -> SocketAddr {
    let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        while let Ok((stream, _)) = listener.accept().await {
            tokio::spawn(serve_trojan_udp(stream));
        }
    });
    addr
}

async fn spawn_fake_trojan_udp_tls_server() -> SocketAddr {
    let acceptor = tls_acceptor();
    let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        while let Ok((tcp, _)) = listener.accept().await {
            let acceptor = acceptor.clone();
            tokio::spawn(async move {
                if let Ok(tls) = acceptor.accept(tcp).await {
                    serve_trojan_udp(tls).await;
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
    let provider = Arc::new(rustls::crypto::ring::default_provider());
    let config = rustls::ServerConfig::builder_with_provider(provider)
        .with_safe_default_protocol_versions()
        .unwrap()
        .with_no_client_auth()
        .with_single_cert(certs, key)
        .unwrap();
    TlsAcceptor::from(Arc::new(config))
}

fn trojan(server: SocketAddr, security: Security) -> Box<TrojanOutboundConfig> {
    Box::new(TrojanOutboundConfig {
        server: server.ip().to_string(),
        port: server.port(),
        password_hash: *TEST_PASSWORD_HASH,
        security,
        transport: Transport::Tcp,
    })
}

fn tls_security() -> Security {
    Security::Tls(TlsClientConfig {
        server_name: Some("localhost".to_string()),
        alpn: Vec::new(),
        skip_cert_verify: true,
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

/// Payload offset in a relayed reply datagram (skips RSV/FRAG and the address).
fn payload_offset(buf: &[u8]) -> usize {
    match buf[3] {
        0x01 => 3 + 1 + 4 + 2,
        0x04 => 3 + 1 + 16 + 2,
        0x03 => 3 + 1 + 1 + buf[4] as usize + 2,
        other => panic!("unexpected reply atyp {other}"),
    }
}

/// Drive one client datagram through the kernel and assert the echo round-trips.
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
async fn udp_relays_through_plaintext_trojan_ipv4() {
    let server = spawn_fake_trojan_udp_server().await;
    let dst = SocketAddr::from((Ipv4Addr::new(1, 2, 3, 4), 443));
    assert_udp_relays(
        OutboundMode::Trojan(trojan(server, Security::None)),
        udp_datagram_ipv4(dst, b"trojan udp ping"),
        b"trojan udp ping",
    )
    .await;
}

#[tokio::test]
async fn udp_relays_through_plaintext_trojan_domain() {
    let server = spawn_fake_trojan_udp_server().await;
    assert_udp_relays(
        OutboundMode::Trojan(trojan(server, Security::None)),
        udp_datagram_domain("example.com", 53, b"domain query"),
        b"domain query",
    )
    .await;
}

#[tokio::test]
async fn udp_relays_through_tls_trojan_ipv4() {
    let server = spawn_fake_trojan_udp_tls_server().await;
    let dst = SocketAddr::from((Ipv4Addr::new(9, 9, 9, 9), 443));
    assert_udp_relays(
        OutboundMode::Trojan(trojan(server, tls_security())),
        udp_datagram_ipv4(dst, b"tls trojan udp"),
        b"tls trojan udp",
    )
    .await;
}

#[tokio::test]
async fn udp_relays_through_routed_trojan() {
    let server = spawn_fake_trojan_udp_server().await;
    let mut outbounds = HashMap::new();
    outbounds.insert(
        "proxy".to_string(),
        OutboundMode::Trojan(trojan(server, Security::None)),
    );
    // No rules -> the DIRECT-like fallback is the named Trojan outbound, proving
    // the Routed path resolves a datagram to the proxy tunnel per destination.
    let router = Router::new(outbounds, vec![], "proxy").unwrap();
    let dst = SocketAddr::from((Ipv4Addr::new(1, 2, 3, 4), 443));
    assert_udp_relays(
        OutboundMode::Routed(Box::new(router)),
        udp_datagram_ipv4(dst, b"routed trojan udp"),
        b"routed trojan udp",
    )
    .await;
}
