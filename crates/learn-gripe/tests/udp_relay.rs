//! End-to-end tests for the SOCKS5 `UDP ASSOCIATE` relay.
//!
//! A client performs the SOCKS5 handshake + UDP ASSOCIATE over TCP, then sends
//! a SOCKS5-wrapped datagram to the returned relay socket. The kernel forwards
//! it (Direct egress) to a real UDP echo server and relays the reply back,
//! re-wrapped. We also assert that a proxy-only outbound refuses the
//! association, since proxy-tunnelled UDP is not implemented yet.

use std::collections::HashMap;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};

use learn_gripe::{DIRECT, GripeConfig, GripeKernel, OutboundMode, Router};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpStream, UdpSocket};

/// UDP echo server: returns every datagram to its sender.
async fn spawn_udp_echo() -> SocketAddr {
    let socket = UdpSocket::bind((Ipv4Addr::LOCALHOST, 0)).await.unwrap();
    let addr = socket.local_addr().unwrap();
    tokio::spawn(async move {
        let mut buf = [0u8; 2048];
        loop {
            match socket.recv_from(&mut buf).await {
                Ok((n, from)) => {
                    let _ = socket.send_to(&buf[..n], from).await;
                }
                Err(_) => return,
            }
        }
    });
    addr
}

/// SOCKS5 no-auth greeting; returns the negotiated stream.
async fn socks5_greet(proxy: SocketAddr) -> TcpStream {
    let mut stream = TcpStream::connect(proxy).await.unwrap();
    stream.write_all(&[0x05, 0x01, 0x00]).await.unwrap();
    let mut selection = [0u8; 2];
    stream.read_exact(&mut selection).await.unwrap();
    assert_eq!(selection, [0x05, 0x00]);
    stream
}

/// Send a UDP ASSOCIATE (with the conventional 0.0.0.0:0 placeholder) and
/// return the control connection plus the relay address from the reply.
async fn socks5_udp_associate(proxy: SocketAddr) -> (TcpStream, SocketAddr) {
    let mut stream = socks5_greet(proxy).await;
    stream
        .write_all(&[0x05, 0x03, 0x00, 0x01, 0, 0, 0, 0, 0, 0])
        .await
        .unwrap();
    let mut reply = [0u8; 10];
    stream.read_exact(&mut reply).await.unwrap();
    assert_eq!(reply[1], 0x00, "expected ASSOCIATE success reply");
    assert_eq!(reply[3], 0x01, "expected an IPv4 bound address");
    let ip = Ipv4Addr::new(reply[4], reply[5], reply[6], reply[7]);
    let port = u16::from_be_bytes([reply[8], reply[9]]);
    (stream, SocketAddr::from((ip, port)))
}

/// Build a SOCKS5 UDP datagram (FRAG=0, IPv4 destination) carrying `payload`.
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

#[tokio::test]
async fn udp_associate_relays_direct_to_echo() {
    let echo = spawn_udp_echo().await;

    let handle = GripeKernel::start(GripeConfig {
        socks_listen: SocketAddr::from((Ipv4Addr::LOCALHOST, 0)),
        outbound: OutboundMode::Direct,
    })
    .await
    .unwrap();

    // The control connection must stay open for the association lifetime.
    let (_control, relay) = socks5_udp_associate(handle.local_addr()).await;

    let client = UdpSocket::bind((Ipv4Addr::LOCALHOST, 0)).await.unwrap();
    client.send_to(&udp_datagram_ipv4(echo, b"ping"), relay).await.unwrap();

    let mut buf = [0u8; 2048];
    let (n, from) = client.recv_from(&mut buf).await.unwrap();
    assert_eq!(from, relay, "reply must come from the relay socket");
    // RSV RSV FRAG ATYP=ipv4 + 4 addr + 2 port = 10-byte header.
    assert_eq!(buf[3], 0x01);
    assert_eq!(&buf[10..n], b"ping", "payload must be echoed verbatim");

    handle.shutdown().await;
}

#[tokio::test]
async fn udp_associate_routed_direct_fallback() {
    let echo = spawn_udp_echo().await;

    // Empty rule list -> everything takes the DIRECT fallback, proving the
    // Routed path resolves UDP egress per datagram.
    let router = Router::new(HashMap::new(), vec![], DIRECT).unwrap();
    let handle = GripeKernel::start(GripeConfig {
        socks_listen: SocketAddr::from((Ipv4Addr::LOCALHOST, 0)),
        outbound: OutboundMode::Routed(Box::new(router)),
    })
    .await
    .unwrap();

    let (_control, relay) = socks5_udp_associate(handle.local_addr()).await;
    let client = UdpSocket::bind((Ipv4Addr::LOCALHOST, 0)).await.unwrap();
    client.send_to(&udp_datagram_ipv4(echo, b"abc"), relay).await.unwrap();

    let mut buf = [0u8; 2048];
    let (n, _from) = client.recv_from(&mut buf).await.unwrap();
    assert_eq!(&buf[10..n], b"abc");

    handle.shutdown().await;
}

#[tokio::test]
async fn udp_associate_refused_for_proxy_outbound() {
    // A proxy-only outbound cannot carry UDP yet, so the associate is refused
    // up front. The upstream address is never dialled.
    let handle = GripeKernel::start(GripeConfig {
        socks_listen: SocketAddr::from((Ipv4Addr::LOCALHOST, 0)),
        outbound: OutboundMode::Socks5Upstream {
            addr: SocketAddr::from((Ipv4Addr::LOCALHOST, 1)),
        },
    })
    .await
    .unwrap();

    let mut stream = socks5_greet(handle.local_addr()).await;
    stream
        .write_all(&[0x05, 0x03, 0x00, 0x01, 0, 0, 0, 0, 0, 0])
        .await
        .unwrap();
    let mut reply = [0u8; 10];
    stream.read_exact(&mut reply).await.unwrap();
    assert_eq!(reply[0], 0x05);
    assert_ne!(reply[1], 0x00, "proxy outbound must refuse UDP ASSOCIATE");

    handle.shutdown().await;
}
