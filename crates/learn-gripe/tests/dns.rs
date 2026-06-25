//! End-to-end tests for the kernel DNS server.
//!
//! A UDP DNS client sends real wire-format queries to the kernel's DNS socket.
//! In fake-IP mode we assert the kernel synthesizes an `A` answer from the pool
//! and records the reverse mapping; in forward mode we stand up an independent
//! fake upstream resolver and assert the kernel relays its answer verbatim.

use std::net::{Ipv4Addr, SocketAddr};
use std::str::FromStr;
use std::sync::Arc;

use hickory_proto::op::{Message, MessageType, OpCode, Query, ResponseCode};
use hickory_proto::rr::rdata::A;
use hickory_proto::rr::{Name, RData, Record, RecordType};
use learn_gripe::{DnsConfig, DnsMode, DnsServer, FakeIpConfig};
use tokio::net::UdpSocket;

/// Build an `A` query for `domain` with id `id`.
fn a_query(id: u16, domain: &str) -> Vec<u8> {
    let mut message = Message::new();
    message.set_id(id);
    message.set_message_type(MessageType::Query);
    message.set_op_code(OpCode::Query);
    message.set_recursion_desired(true);
    message.add_query(Query::query(Name::from_str(domain).unwrap(), RecordType::A));
    message.to_vec().unwrap()
}

/// Send `query` to `server` and return the parsed response.
async fn ask(server: SocketAddr, query: &[u8]) -> Message {
    let client = UdpSocket::bind((Ipv4Addr::LOCALHOST, 0)).await.unwrap();
    client.send_to(query, server).await.unwrap();
    let mut buf = [0u8; 4096];
    let (n, _) = client.recv_from(&mut buf).await.unwrap();
    Message::from_vec(&buf[..n]).unwrap()
}

#[tokio::test]
async fn fake_ip_mode_synthesizes_and_reverses() {
    let (mode, pool) = DnsMode::fake_ip(FakeIpConfig::default());
    let handle = DnsServer::start(DnsConfig {
        listen: SocketAddr::from((Ipv4Addr::LOCALHOST, 0)),
        mode,
    })
    .await
    .unwrap();

    let response = ask(handle.local_addr(), &a_query(0xabcd, "example.com.")).await;
    assert_eq!(response.id(), 0xabcd);
    assert_eq!(response.answers().len(), 1);
    let ip = match response.answers()[0].data() {
        Some(RData::A(A(ip))) => *ip,
        other => panic!("expected A answer, got {other:?}"),
    };
    assert_eq!(ip, Ipv4Addr::new(198, 18, 0, 1));

    // The kernel recorded the reverse mapping for the routing path to consume.
    assert_eq!(pool.lock().unwrap().domain_for(ip), Some("example.com"));

    // The same domain resolves to the same fake IP across queries.
    let again = ask(handle.local_addr(), &a_query(0x0001, "example.com.")).await;
    assert_eq!(
        again.answers()[0].data().and_then(|d| match d {
            RData::A(A(ip)) => Some(*ip),
            _ => None,
        }),
        Some(ip)
    );

    handle.shutdown().await;
}

/// Spawn a fake upstream resolver that answers every `A` query with `answer`.
async fn spawn_fake_upstream(answer: Ipv4Addr) -> SocketAddr {
    let socket = UdpSocket::bind((Ipv4Addr::LOCALHOST, 0)).await.unwrap();
    let addr = socket.local_addr().unwrap();
    let socket = Arc::new(socket);
    tokio::spawn(async move {
        let mut buf = [0u8; 4096];
        loop {
            let (n, peer) = match socket.recv_from(&mut buf).await {
                Ok(v) => v,
                Err(_) => return,
            };
            let request = match Message::from_vec(&buf[..n]) {
                Ok(m) => m,
                Err(_) => continue,
            };
            let mut response = Message::new();
            response.set_id(request.id());
            response.set_message_type(MessageType::Response);
            response.set_op_code(OpCode::Query);
            response.set_response_code(ResponseCode::NoError);
            for query in request.queries() {
                response.add_query(query.clone());
                if query.query_type() == RecordType::A {
                    response.add_answer(Record::from_rdata(query.name().clone(), 60, RData::A(A(answer))));
                }
            }
            let _ = socket.send_to(&response.to_vec().unwrap(), peer).await;
        }
    });
    addr
}

#[tokio::test]
async fn forward_mode_relays_upstream_answer() {
    let upstream = spawn_fake_upstream(Ipv4Addr::new(93, 184, 216, 34)).await;
    let handle = DnsServer::start(DnsConfig {
        listen: SocketAddr::from((Ipv4Addr::LOCALHOST, 0)),
        mode: DnsMode::Forward { upstream },
    })
    .await
    .unwrap();

    let response = ask(handle.local_addr(), &a_query(0x1111, "example.org.")).await;
    assert_eq!(response.id(), 0x1111);
    assert_eq!(response.answers().len(), 1);
    match response.answers()[0].data() {
        Some(RData::A(A(ip))) => assert_eq!(*ip, Ipv4Addr::new(93, 184, 216, 34)),
        other => panic!("expected forwarded A answer, got {other:?}"),
    }

    handle.shutdown().await;
}
