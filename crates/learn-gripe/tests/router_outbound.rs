//! End-to-end tests for rule-based outbound selection.
//!
//! A SOCKS5 client connects through the gripe kernel configured with an
//! [`OutboundMode::Routed`] router. Each named outbound is a fake SOCKS5
//! upstream that echoes a distinct tag, so the tag observed by the client
//! proves which outbound a given target was routed to. `DIRECT` (a plain echo
//! server reached straight) and `REJECT` (refused connection) are exercised
//! too.

use std::collections::HashMap;
use std::net::{Ipv4Addr, SocketAddr};

use learn_gripe::{DIRECT, GripeConfig, GripeKernel, IpCidr, OutboundMode, REJECT, Router, Rule, RuleMatcher};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

/// Fake SOCKS5 upstream: completes the no-auth CONNECT handshake, then echoes
/// received bytes with `tag` prepended to the first chunk. It does not dial the
/// requested target — the tag is all the test needs to identify the outbound.
async fn serve_tagged_upstream(mut stream: TcpStream, tag: &'static [u8]) {
    let mut greeting = [0u8; 2];
    if stream.read_exact(&mut greeting).await.is_err() {
        return;
    }
    let mut methods = vec![0u8; greeting[1] as usize];
    if stream.read_exact(&mut methods).await.is_err() {
        return;
    }
    if stream.write_all(&[0x05, 0x00]).await.is_err() {
        return;
    }

    let mut head = [0u8; 4];
    if stream.read_exact(&mut head).await.is_err() {
        return;
    }
    match head[3] {
        0x01 => {
            let mut addr = [0u8; 4];
            let _ = stream.read_exact(&mut addr).await;
        }
        0x04 => {
            let mut addr = [0u8; 16];
            let _ = stream.read_exact(&mut addr).await;
        }
        0x03 => {
            let mut len = [0u8; 1];
            let _ = stream.read_exact(&mut len).await;
            let mut host = vec![0u8; len[0] as usize];
            let _ = stream.read_exact(&mut host).await;
        }
        _ => return,
    }
    let mut port = [0u8; 2];
    let _ = stream.read_exact(&mut port).await;
    if stream
        .write_all(&[0x05, 0x00, 0x00, 0x01, 0, 0, 0, 0, 0, 0])
        .await
        .is_err()
    {
        return;
    }

    let mut buf = [0u8; 1024];
    let mut first = true;
    loop {
        match stream.read(&mut buf).await {
            Ok(0) | Err(_) => return,
            Ok(n) => {
                let mut out = Vec::new();
                if first {
                    out.extend_from_slice(tag);
                    first = false;
                }
                out.extend_from_slice(&buf[..n]);
                if stream.write_all(&out).await.is_err() {
                    return;
                }
            }
        }
    }
}

async fn spawn_tagged_upstream(tag: &'static [u8]) -> SocketAddr {
    let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        while let Ok((stream, _)) = listener.accept().await {
            tokio::spawn(serve_tagged_upstream(stream, tag));
        }
    });
    addr
}

/// Plain TCP echo server, used as the real target for the `DIRECT` path.
async fn spawn_echo_target() -> SocketAddr {
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

/// SOCKS5 no-auth greeting; returns the negotiated stream ready for a request.
async fn socks5_greet(proxy: SocketAddr) -> TcpStream {
    let mut stream = TcpStream::connect(proxy).await.unwrap();
    stream.write_all(&[0x05, 0x01, 0x00]).await.unwrap();
    let mut selection = [0u8; 2];
    stream.read_exact(&mut selection).await.unwrap();
    assert_eq!(selection, [0x05, 0x00]);
    stream
}

/// CONNECT to a domain target through the proxy, asserting success.
async fn socks5_connect_domain(proxy: SocketAddr, host: &str, port: u16) -> TcpStream {
    let mut stream = socks5_greet(proxy).await;
    let mut request = vec![0x05, 0x01, 0x00, 0x03, host.len() as u8];
    request.extend_from_slice(host.as_bytes());
    request.extend_from_slice(&port.to_be_bytes());
    stream.write_all(&request).await.unwrap();
    let mut reply = [0u8; 10];
    stream.read_exact(&mut reply).await.unwrap();
    assert_eq!(reply[1], 0x00, "expected SOCKS5 success reply");
    stream
}

/// CONNECT to an IPv4 target through the proxy, asserting success.
async fn socks5_connect_ip(proxy: SocketAddr, target: SocketAddr) -> TcpStream {
    let mut stream = socks5_greet(proxy).await;
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
    assert_eq!(reply[1], 0x00, "expected SOCKS5 success reply");
    stream
}

fn socks5_upstream(addr: SocketAddr) -> OutboundMode {
    OutboundMode::Socks5Upstream { addr }
}

async fn echo_roundtrip(conn: &mut TcpStream, payload: &[u8], expect: &[u8]) {
    conn.write_all(payload).await.unwrap();
    let mut buf = vec![0u8; expect.len()];
    conn.read_exact(&mut buf).await.unwrap();
    assert_eq!(&buf, expect);
}

#[tokio::test]
async fn routes_domains_to_distinct_outbounds() {
    let upstream_a = spawn_tagged_upstream(b"A:").await;
    let upstream_b = spawn_tagged_upstream(b"B:").await;

    let mut outbounds = HashMap::new();
    outbounds.insert("alpha".to_string(), socks5_upstream(upstream_a));
    outbounds.insert("beta".to_string(), socks5_upstream(upstream_b));
    let rules = vec![
        Rule::new(RuleMatcher::DomainSuffix("alpha.test".to_string()), "alpha"),
        Rule::new(RuleMatcher::DomainKeyword("beta".to_string()), "beta"),
    ];
    let router = Router::new(outbounds, rules, "alpha").unwrap();

    let handle = GripeKernel::start(GripeConfig {
        socks_listen: SocketAddr::from((Ipv4Addr::LOCALHOST, 0)),
        outbound: OutboundMode::Routed(Box::new(router)),
    })
    .await
    .unwrap();

    let mut to_a = socks5_connect_domain(handle.local_addr(), "www.alpha.test", 443).await;
    echo_roundtrip(&mut to_a, b"ping", b"A:ping").await;

    let mut to_b = socks5_connect_domain(handle.local_addr(), "cdn.beta.example", 443).await;
    echo_roundtrip(&mut to_b, b"ping", b"B:ping").await;

    handle.shutdown().await;
}

#[tokio::test]
async fn routes_ip_cidr_with_direct_fallback() {
    let upstream = spawn_tagged_upstream(b"VIA:").await;
    let echo = spawn_echo_target().await;

    let mut outbounds = HashMap::new();
    outbounds.insert("proxy".to_string(), socks5_upstream(upstream));
    // Route the loopback /8 through the proxy; everything else falls back to
    // DIRECT (which dials the real echo target).
    let rules = vec![Rule::new(
        RuleMatcher::IpCidr(IpCidr::parse("10.0.0.0/8").unwrap()),
        "proxy",
    )];
    let router = Router::new(outbounds, rules, DIRECT).unwrap();

    let handle = GripeKernel::start(GripeConfig {
        socks_listen: SocketAddr::from((Ipv4Addr::LOCALHOST, 0)),
        outbound: OutboundMode::Routed(Box::new(router)),
    })
    .await
    .unwrap();

    // 10.x.x.x matches the CIDR -> proxy outbound (tagged echo).
    let mut via_proxy =
        socks5_connect_ip(handle.local_addr(), SocketAddr::from((Ipv4Addr::new(10, 1, 2, 3), 80))).await;
    echo_roundtrip(&mut via_proxy, b"ping", b"VIA:ping").await;

    // The real echo target is on 127.0.0.1, which does not match the rule, so
    // it takes the DIRECT fallback and reaches the echo server untagged.
    let mut direct = socks5_connect_ip(handle.local_addr(), echo).await;
    echo_roundtrip(&mut direct, b"hello", b"hello").await;

    handle.shutdown().await;
}

#[tokio::test]
async fn reject_rule_refuses_connection() {
    let rules = vec![Rule::new(RuleMatcher::DomainSuffix("blocked.test".to_string()), REJECT)];
    let router = Router::new(HashMap::new(), rules, DIRECT).unwrap();

    let handle = GripeKernel::start(GripeConfig {
        socks_listen: SocketAddr::from((Ipv4Addr::LOCALHOST, 0)),
        outbound: OutboundMode::Routed(Box::new(router)),
    })
    .await
    .unwrap();

    let mut stream = socks5_greet(handle.local_addr()).await;
    let host = "ads.blocked.test";
    let mut request = vec![0x05, 0x01, 0x00, 0x03, host.len() as u8];
    request.extend_from_slice(host.as_bytes());
    request.extend_from_slice(&443u16.to_be_bytes());
    stream.write_all(&request).await.unwrap();

    let mut reply = [0u8; 10];
    stream.read_exact(&mut reply).await.unwrap();
    assert_eq!(reply[0], 0x05);
    assert_ne!(reply[1], 0x00, "REJECT must not yield a success reply");

    handle.shutdown().await;
}
