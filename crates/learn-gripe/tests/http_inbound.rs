//! End-to-end proof that the HTTP proxy inbound moves real bytes through the
//! learn-gripe data plane, on the same mixed listener as SOCKS5:
//!
//! - `CONNECT` tunnels raw bytes to an echo origin (the HTTPS-over-HTTP path).
//! - A plain absolute-form `GET` is rewritten to origin-form and forwarded to
//!   an HTTP origin, which observes the relative path.
//! - A `CONNECT` to a rejected outbound is answered with `502 Bad Gateway`.

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

/// A minimal HTTP origin that echoes the request line back in the body, so the
/// test can assert the proxy forwarded an origin-form (relative) path.
async fn spawn_http_origin() -> SocketAddr {
    let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        while let Ok((mut stream, _)) = listener.accept().await {
            tokio::spawn(async move {
                let mut buf = vec![0u8; 4096];
                let n = stream.read(&mut buf).await.unwrap_or(0);
                if n == 0 {
                    return;
                }
                let text = String::from_utf8_lossy(&buf[..n]);
                let request_line = text.lines().next().unwrap_or("").to_string();
                let body = format!("seen:{request_line}");
                let response = format!(
                    "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                    body.len(),
                    body
                );
                let _ = stream.write_all(response.as_bytes()).await;
            });
        }
    });
    addr
}

async fn start_gripe(outbound: OutboundMode) -> learn_gripe::GripeHandle {
    GripeKernel::start(GripeConfig {
        socks_listen: SocketAddr::from((Ipv4Addr::LOCALHOST, 0)),
        outbound,
    })
    .await
    .unwrap()
}

#[tokio::test]
async fn connect_tunnels_raw_bytes() {
    let echo = spawn_echo_server().await;
    let handle = start_gripe(OutboundMode::Direct).await;

    let mut conn = TcpStream::connect(handle.local_addr()).await.unwrap();
    let request = format!("CONNECT {echo} HTTP/1.1\r\nHost: {echo}\r\n\r\n");
    conn.write_all(request.as_bytes()).await.unwrap();

    // The proxy acknowledges the tunnel.
    let mut reply = [0u8; 12];
    conn.read_exact(&mut reply).await.unwrap();
    assert_eq!(&reply, b"HTTP/1.1 200");
    // Consume the rest of the status line + blank line.
    read_until_double_crlf(&mut conn).await;

    // Now the connection is a raw tunnel to the echo origin.
    conn.write_all(b"hello tunnel").await.unwrap();
    let mut buf = [0u8; 12];
    conn.read_exact(&mut buf).await.unwrap();
    assert_eq!(&buf, b"hello tunnel");

    handle.shutdown().await;
}

#[tokio::test]
async fn plain_request_is_rewritten_to_origin_form() {
    let origin = spawn_http_origin().await;
    let handle = start_gripe(OutboundMode::Direct).await;

    let mut conn = TcpStream::connect(handle.local_addr()).await.unwrap();
    let request =
        format!("GET http://{origin}/hello?q=1 HTTP/1.1\r\nHost: {origin}\r\nProxy-Connection: keep-alive\r\n\r\n");
    conn.write_all(request.as_bytes()).await.unwrap();

    let mut response = String::new();
    conn.read_to_string(&mut response).await.unwrap();

    // The origin saw the relative path, proving the absolute-form line was
    // rewritten before forwarding.
    assert!(response.contains("200 OK"), "response was: {response}");
    assert!(
        response.contains("seen:GET /hello?q=1 HTTP/1.1"),
        "origin did not receive an origin-form request line; response was: {response}"
    );

    handle.shutdown().await;
}

#[tokio::test]
async fn connect_to_rejected_outbound_returns_502() {
    let handle = start_gripe(OutboundMode::Reject).await;

    let mut conn = TcpStream::connect(handle.local_addr()).await.unwrap();
    conn.write_all(b"CONNECT example.com:443 HTTP/1.1\r\nHost: example.com:443\r\n\r\n")
        .await
        .unwrap();

    let mut response = String::new();
    conn.read_to_string(&mut response).await.unwrap();
    assert!(response.starts_with("HTTP/1.1 502"), "response was: {response}");

    handle.shutdown().await;
}

async fn read_until_double_crlf(stream: &mut TcpStream) {
    let mut window = [0u8; 4];
    let mut byte = [0u8; 1];
    loop {
        stream.read_exact(&mut byte).await.unwrap();
        window = [window[1], window[2], window[3], byte[0]];
        if &window == b"\r\n\r\n" {
            return;
        }
    }
}
