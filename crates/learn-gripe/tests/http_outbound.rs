//! End-to-end proof for the HTTP upstream-proxy outbound: a SOCKS5 client ->
//! gripe inbound -> HTTP-proxy outbound -> fake CONNECT proxy -> echo server.
//!
//! The fake proxy is an independent implementation of the `CONNECT` method
//! (optionally requiring `Proxy-Authorization: Basic`) so the test exercises the
//! real on-wire exchange rather than gripe talking to itself.

use std::net::{Ipv4Addr, SocketAddr};

use learn_gripe::{GripeConfig, GripeKernel, HttpOutboundConfig, OutboundMode, Security};
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

/// A minimal HTTP `CONNECT` proxy. When `expect_auth` is set, requests missing
/// the matching `Proxy-Authorization: Basic` header are answered `407`.
async fn spawn_http_proxy(expect_auth: Option<&'static str>) -> SocketAddr {
    let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        while let Ok((mut client, _)) = listener.accept().await {
            tokio::spawn(async move {
                // Read the request head up to the blank line.
                let mut head = Vec::new();
                let mut byte = [0u8; 1];
                loop {
                    match client.read(&mut byte).await {
                        Ok(0) | Err(_) => return,
                        Ok(_) => head.push(byte[0]),
                    }
                    if head.ends_with(b"\r\n\r\n") {
                        break;
                    }
                }
                let request = String::from_utf8_lossy(&head);
                let mut lines = request.split("\r\n");
                let request_line = lines.next().unwrap_or("");
                let mut parts = request_line.split(' ');
                assert_eq!(parts.next(), Some("CONNECT"), "expected CONNECT, got {request_line:?}");
                let authority = parts.next().unwrap_or("").to_string();

                if let Some(expected) = expect_auth {
                    let header = format!("Proxy-Authorization: Basic {expected}");
                    if !lines.any(|l| l.eq_ignore_ascii_case(&header)) {
                        let _ = client
                            .write_all(b"HTTP/1.1 407 Proxy Authentication Required\r\n\r\n")
                            .await;
                        return;
                    }
                }

                let Ok(upstream) = TcpStream::connect(&authority).await else {
                    let _ = client.write_all(b"HTTP/1.1 502 Bad Gateway\r\n\r\n").await;
                    return;
                };
                client
                    .write_all(b"HTTP/1.1 200 Connection established\r\n\r\n")
                    .await
                    .unwrap();

                let (mut cr, mut cw) = client.into_split();
                let (mut ur, mut uw) = upstream.into_split();
                let c2u = tokio::io::copy(&mut cr, &mut uw);
                let u2c = tokio::io::copy(&mut ur, &mut cw);
                let _ = tokio::join!(c2u, u2c);
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

fn http_mode(proxy: SocketAddr, auth: Option<(&str, &str)>) -> OutboundMode {
    OutboundMode::Http(Box::new(HttpOutboundConfig {
        server: proxy.ip().to_string(),
        port: proxy.port(),
        auth: auth.map(|(u, p)| (u.to_string(), p.to_string())),
        security: Security::None,
    }))
}

#[tokio::test]
async fn relays_through_http_proxy() {
    let echo = spawn_echo_server().await;
    let proxy = spawn_http_proxy(None).await;

    let edge = GripeKernel::start(GripeConfig {
        socks_listen: SocketAddr::from((Ipv4Addr::LOCALHOST, 0)),
        outbound: http_mode(proxy, None),
    })
    .await
    .unwrap();

    let mut conn = socks5_connect(edge.local_addr(), echo).await;
    conn.write_all(b"hello http proxy").await.unwrap();
    let mut buf = [0u8; 16];
    conn.read_exact(&mut buf).await.unwrap();
    assert_eq!(&buf, b"hello http proxy");

    edge.shutdown().await;
}

#[tokio::test]
async fn relays_through_authenticated_http_proxy() {
    let echo = spawn_echo_server().await;
    // base64("bob:secret").
    let proxy = spawn_http_proxy(Some("Ym9iOnNlY3JldA==")).await;

    let edge = GripeKernel::start(GripeConfig {
        socks_listen: SocketAddr::from((Ipv4Addr::LOCALHOST, 0)),
        outbound: http_mode(proxy, Some(("bob", "secret"))),
    })
    .await
    .unwrap();

    let mut conn = socks5_connect(edge.local_addr(), echo).await;
    conn.write_all(b"authed").await.unwrap();
    let mut buf = [0u8; 6];
    conn.read_exact(&mut buf).await.unwrap();
    assert_eq!(&buf, b"authed");

    edge.shutdown().await;
}

#[tokio::test]
async fn rejected_when_auth_missing() {
    let echo = spawn_echo_server().await;
    let proxy = spawn_http_proxy(Some("Ym9iOnNlY3JldA==")).await;

    // Edge offers no credentials, so the proxy answers 407 and the CONNECT fails.
    let edge = GripeKernel::start(GripeConfig {
        socks_listen: SocketAddr::from((Ipv4Addr::LOCALHOST, 0)),
        outbound: http_mode(proxy, None),
    })
    .await
    .unwrap();

    // The outbound CONNECT fails, so the inbound answers the SOCKS5 request with
    // a non-success reply code rather than opening a relay.
    let mut stream = TcpStream::connect(edge.local_addr()).await.unwrap();
    stream.write_all(&[0x05, 0x01, 0x00]).await.unwrap();
    let mut selection = [0u8; 2];
    stream.read_exact(&mut selection).await.unwrap();
    assert_eq!(selection, [0x05, 0x00]);

    let ip = match echo.ip() {
        std::net::IpAddr::V4(v4) => v4.octets(),
        std::net::IpAddr::V6(_) => panic!("test uses IPv4"),
    };
    let mut request = vec![0x05, 0x01, 0x00, 0x01];
    request.extend_from_slice(&ip);
    request.extend_from_slice(&echo.port().to_be_bytes());
    stream.write_all(&request).await.unwrap();

    let mut reply = [0u8; 10];
    stream.read_exact(&mut reply).await.unwrap();
    assert_eq!(reply[0], 0x05);
    assert_ne!(reply[1], 0x00, "rejected CONNECT must not report success");

    edge.shutdown().await;
}
