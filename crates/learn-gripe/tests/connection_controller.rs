//! End-to-end proof of the in-process connection controller API: a SOCKS5
//! client -> gripe inbound -> direct outbound -> echo server, observed and torn
//! down through [`GripeHandle::connections`] / [`GripeHandle::close_connection`]
//! (the replacements for the Mihomo controller `/connections` + close calls).

use learn_gripe::{GripeConfig, GripeKernel, OutboundMode};
use std::net::{Ipv4Addr, SocketAddr};
use std::time::Duration;
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
    stream.write_all(&[0x05, 0x01, 0x00]).await.unwrap();
    let mut selection = [0u8; 2];
    stream.read_exact(&mut selection).await.unwrap();
    assert_eq!(selection, [0x05, 0x00]);

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

/// Poll `f` until it returns `true` or the deadline elapses, so we don't race
/// the relay's asynchronous register/deregister.
async fn wait_until<F: Fn() -> bool>(f: F) -> bool {
    for _ in 0..100 {
        if f() {
            return true;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    f()
}

#[tokio::test]
async fn tracks_then_closes_a_live_connection() {
    let echo = spawn_echo_server().await;

    let handle = GripeKernel::start(GripeConfig {
        socks_listen: SocketAddr::from((Ipv4Addr::LOCALHOST, 0)),
        outbound: OutboundMode::Direct,
    })
    .await
    .unwrap();
    let proxy = handle.local_addr();

    // No traffic yet: the table is empty.
    assert!(handle.connections().connections.is_empty());

    let mut conn = socks5_connect(proxy, echo).await;
    conn.write_all(b"hello gripe").await.unwrap();
    let mut buf = [0u8; 11];
    conn.read_exact(&mut buf).await.unwrap();
    assert_eq!(&buf, b"hello gripe");

    // The connection shows up in the controller snapshot with the right target
    // and accounted upload/download.
    assert!(wait_until(|| handle.connections().connections.len() == 1).await);
    let snap = handle.connections();
    let tracked = &snap.connections[0];
    assert_eq!(tracked.meta.host, echo.ip().to_string());
    assert_eq!(tracked.meta.destination_port, echo.port());
    assert_eq!(tracked.meta.network.as_str(), "tcp");
    assert!(wait_until(|| handle.connections().connections[0].upload >= 11).await);
    let snap = handle.connections();
    assert!(snap.connections[0].download >= 11);
    assert!(snap.upload_total >= 11);
    assert!(snap.download_total >= 11);

    // Closing an unknown id is a no-op; closing the live id tears it down.
    let id = snap.connections[0].id;
    assert!(!handle.close_connection(id + 1_000_000));
    assert!(handle.close_connection(id));

    // The relay stops, its guard drops, and the connection leaves the table,
    // but its bytes persist in the cumulative totals.
    assert!(wait_until(|| handle.connections().connections.is_empty()).await);
    let snap = handle.connections();
    assert!(snap.connections.is_empty());
    assert!(snap.upload_total >= 11);
    assert!(snap.download_total >= 11);

    handle.shutdown().await;
}

#[tokio::test]
async fn close_all_tears_down_every_connection() {
    let echo = spawn_echo_server().await;

    let handle = GripeKernel::start(GripeConfig {
        socks_listen: SocketAddr::from((Ipv4Addr::LOCALHOST, 0)),
        outbound: OutboundMode::Direct,
    })
    .await
    .unwrap();
    let proxy = handle.local_addr();

    // Open three concurrent connections and keep them alive.
    let mut conns = Vec::new();
    for _ in 0..3 {
        let mut conn = socks5_connect(proxy, echo).await;
        conn.write_all(b"x").await.unwrap();
        let mut buf = [0u8; 1];
        conn.read_exact(&mut buf).await.unwrap();
        conns.push(conn);
    }

    assert!(wait_until(|| handle.connections().connections.len() == 3).await);
    assert_eq!(handle.close_all_connections(), 3);
    assert!(wait_until(|| handle.connections().connections.is_empty()).await);

    // Each client observes EOF once its relay is torn down.
    for mut conn in conns {
        let mut buf = [0u8; 1];
        let n = conn.read(&mut buf).await.unwrap_or(0);
        assert_eq!(n, 0, "connection should be closed after close_all");
    }

    handle.shutdown().await;
}
