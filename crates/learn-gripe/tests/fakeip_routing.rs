//! End-to-end test for fake-IP routing.
//!
//! The full loop: a DNS client asks the kernel's fake-IP DNS server to resolve
//! two domains, getting back synthetic IPs from the same `198.18.0.0/16` pool.
//! A SOCKS5 client then connects to those fake IPs. The kernel (started with
//! the shared pool) rewrites each fake IP back to its original domain and routes
//! by the domain rules — so two addresses in the same CIDR reach *different*
//! tagged outbounds purely because of the hostname they were minted for. That
//! is what proves the fake IP drove the routing decision.

use std::collections::HashMap;
use std::net::{Ipv4Addr, SocketAddr};
use std::str::FromStr;

use hickory_proto::op::{Message, MessageType, OpCode, Query};
use hickory_proto::rr::rdata::A;
use hickory_proto::rr::{Name, RData, RecordType};
use learn_gripe::{
    DnsConfig, DnsMode, DnsServer, FakeIpConfig, GripeConfig, GripeKernel, OutboundMode, Router, Rule, RuleMatcher,
};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream, UdpSocket};

/// Fake SOCKS5 upstream that completes a no-auth CONNECT and echoes received
/// bytes with `tag` prepended to the first chunk. The tag identifies which
/// outbound a connection was routed to; it does not dial the real target.
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

/// SOCKS5 no-auth greeting; returns the negotiated stream.
async fn socks5_greet(proxy: SocketAddr) -> TcpStream {
    let mut stream = TcpStream::connect(proxy).await.unwrap();
    stream.write_all(&[0x05, 0x01, 0x00]).await.unwrap();
    let mut selection = [0u8; 2];
    stream.read_exact(&mut selection).await.unwrap();
    assert_eq!(selection, [0x05, 0x00]);
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

async fn echo_roundtrip(conn: &mut TcpStream, payload: &[u8], expect: &[u8]) {
    conn.write_all(payload).await.unwrap();
    let mut buf = vec![0u8; expect.len()];
    conn.read_exact(&mut buf).await.unwrap();
    assert_eq!(&buf, expect);
}

/// Resolve `domain` via the kernel's fake-IP DNS server and return the fake IP.
async fn resolve_fake_ip(dns: SocketAddr, domain: &str) -> Ipv4Addr {
    let mut query = Message::new();
    query.set_id(1);
    query.set_message_type(MessageType::Query);
    query.set_op_code(OpCode::Query);
    query.set_recursion_desired(true);
    query.add_query(Query::query(Name::from_str(domain).unwrap(), RecordType::A));

    let client = UdpSocket::bind((Ipv4Addr::LOCALHOST, 0)).await.unwrap();
    client.send_to(&query.to_vec().unwrap(), dns).await.unwrap();
    let mut buf = [0u8; 4096];
    let (n, _) = client.recv_from(&mut buf).await.unwrap();
    let response = Message::from_vec(&buf[..n]).unwrap();
    match response.answers()[0].data() {
        Some(RData::A(A(ip))) => *ip,
        other => panic!("expected A answer, got {other:?}"),
    }
}

#[tokio::test]
async fn fake_ip_drives_domain_routing() {
    let upstream_a = spawn_tagged_upstream(b"A:").await;
    let upstream_b = spawn_tagged_upstream(b"B:").await;

    // Fake-IP DNS and the kernel share one pool.
    let (mode, pool) = DnsMode::fake_ip(FakeIpConfig::default());
    let dns = DnsServer::start(DnsConfig {
        listen: SocketAddr::from((Ipv4Addr::LOCALHOST, 0)),
        mode,
    })
    .await
    .unwrap();

    let mut outbounds = HashMap::new();
    outbounds.insert("alpha".to_string(), OutboundMode::Socks5Upstream { addr: upstream_a });
    outbounds.insert("beta".to_string(), OutboundMode::Socks5Upstream { addr: upstream_b });
    // Only example.com routes to alpha; everything else falls back to beta.
    let rules = vec![Rule::new(RuleMatcher::DomainSuffix("example.com".to_string()), "alpha")];
    let router = Router::new(outbounds, rules, "beta").unwrap();

    let kernel = GripeKernel::start_with_fake_ip(
        GripeConfig {
            socks_listen: SocketAddr::from((Ipv4Addr::LOCALHOST, 0)),
            outbound: OutboundMode::Routed(Box::new(router)),
        },
        pool,
    )
    .await
    .unwrap();

    // Mint two fake IPs from the same /16 for two different domains.
    let ip_example = resolve_fake_ip(dns.local_addr(), "example.com.").await;
    let ip_other = resolve_fake_ip(dns.local_addr(), "other.org.").await;
    assert_ne!(ip_example, ip_other);

    // Connecting to example.com's fake IP unmaps to example.com -> alpha (A).
    let mut to_a = socks5_connect_ip(kernel.local_addr(), SocketAddr::from((ip_example, 443))).await;
    echo_roundtrip(&mut to_a, b"ping", b"A:ping").await;

    // Connecting to other.org's fake IP (same CIDR) unmaps to other.org, which
    // misses the example.com rule, so it takes the beta fallback (B).
    let mut to_b = socks5_connect_ip(kernel.local_addr(), SocketAddr::from((ip_other, 443))).await;
    echo_roundtrip(&mut to_b, b"ping", b"B:ping").await;

    kernel.shutdown().await;
    dns.shutdown().await;
}
