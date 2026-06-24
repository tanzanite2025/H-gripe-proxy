//! End-to-end proof that traffic flows through the learn-gripe data plane:
//! a SOCKS5 client -> gripe inbound -> direct outbound -> echo server.

use learn_gripe::{GripeConfig, GripeKernel, OutboundMode};
use std::net::{Ipv4Addr, SocketAddr};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

async fn spawn_echo_server() -> SocketAddr {
    let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        while let Ok((mut stream, _)) = listener.accept().await {
            tokio::spawn(async move {
                let mut buf = [0u8; 1024];
                loop {
                    match stream.read(&mut buf).await {
                        Ok(0) | Err(_) => return,
                        Ok(n) => {
                            if stream.write_all(&buf[..n]).await.is_err() {
                                return;
                            }
                        }
                    }
                }
            });
        }
    });
    addr
}

async fn socks5_connect(proxy: SocketAddr, target: SocketAddr) -> TcpStream {
    let mut stream = TcpStream::connect(proxy).await.unwrap();
    // Greeting: VER, NMETHODS=1, no-auth.
    stream.write_all(&[0x05, 0x01, 0x00]).await.unwrap();
    let mut selection = [0u8; 2];
    stream.read_exact(&mut selection).await.unwrap();
    assert_eq!(selection, [0x05, 0x00]);

    // CONNECT to an IPv4 target.
    let ip = match target.ip() {
        std::net::IpAddr::V4(v4) => v4.octets(),
        std::net::IpAddr::V6(_) => panic!("test uses IPv4"),
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
async fn relays_through_direct_outbound() {
    let echo = spawn_echo_server().await;

    let config = GripeConfig {
        socks_listen: SocketAddr::from((Ipv4Addr::LOCALHOST, 0)),
        outbound: OutboundMode::Direct,
    };
    let handle = GripeKernel::start(config).await.unwrap();
    let proxy = handle.local_addr();

    let mut conn = socks5_connect(proxy, echo).await;
    conn.write_all(b"hello gripe").await.unwrap();
    let mut buf = [0u8; 11];
    conn.read_exact(&mut buf).await.unwrap();
    assert_eq!(&buf, b"hello gripe");

    handle.shutdown().await;
}

#[tokio::test]
async fn relays_through_upstream_socks5() {
    let echo = spawn_echo_server().await;

    // First kernel acts as the upstream SOCKS5 proxy (direct outbound).
    let upstream = GripeKernel::start(GripeConfig {
        socks_listen: SocketAddr::from((Ipv4Addr::LOCALHOST, 0)),
        outbound: OutboundMode::Direct,
    })
    .await
    .unwrap();

    // Second kernel forwards through the upstream.
    let edge = GripeKernel::start(GripeConfig {
        socks_listen: SocketAddr::from((Ipv4Addr::LOCALHOST, 0)),
        outbound: OutboundMode::Socks5Upstream {
            addr: upstream.local_addr(),
        },
    })
    .await
    .unwrap();

    let mut conn = socks5_connect(edge.local_addr(), echo).await;
    conn.write_all(b"chain").await.unwrap();
    let mut buf = [0u8; 5];
    conn.read_exact(&mut buf).await.unwrap();
    assert_eq!(&buf, b"chain");

    edge.shutdown().await;
    upstream.shutdown().await;
}
