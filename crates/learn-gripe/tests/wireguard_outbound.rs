//! WireGuard outbound interop tests.
//!
//! The kernel's WireGuard outbound (`protocols::wireguard`) is exercised against
//! an **independent** fake WireGuard peer built here from a second `boringtun`
//! `Tunn` (the responder) plus its own smoltcp TCP/IP stack that terminates the
//! inner connection and echoes. This proves the full path end to end over a real
//! Noise_IKpsk2 handshake: client smoltcp SYN -> encrypt -> UDP -> server
//! decrypt -> server smoltcp accept/echo -> encrypt -> UDP -> client decrypt ->
//! client smoltcp data.

use std::collections::VecDeque;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

use boringtun::noise::{Tunn, TunnResult};
use boringtun::x25519::{PublicKey, StaticSecret};
use hickory_proto::op::{Message, MessageType, OpCode, ResponseCode};
use hickory_proto::rr::rdata::A;
use hickory_proto::rr::{RData, Record, RecordType};
use learn_gripe::{ProxyEntry, TargetAddr, WireGuardOutboundConfig, wireguard};
use smoltcp::iface::{Config as IfaceConfig, Interface, SocketSet};
use smoltcp::phy::{Device, DeviceCapabilities, Medium, RxToken, TxToken};
use smoltcp::socket::{tcp, udp};
use smoltcp::time::Instant as SmolInstant;
use smoltcp::wire::{HardwareAddress, IpAddress, IpCidr, Ipv4Address};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::UdpSocket;

const MTU: usize = 1408;
/// Inner destination the relayed TCP connection targets (an address inside the
/// tunnel that the fake server's stack accepts via `any_ip`).
const INNER_IP: Ipv4Addr = Ipv4Addr::new(10, 0, 0, 1);
const INNER_PORT: u16 = 9000;
/// Inner resolver address the fake server answers `A` queries on (UDP/53), used
/// by the tunnel-side DNS test. `any_ip` lets the server accept this dest.
const RESOLVER_IP: Ipv4Addr = Ipv4Addr::new(10, 0, 0, 53);

// --- key helpers --------------------------------------------------------------

fn keypair() -> ([u8; 32], [u8; 32], StaticSecret, PublicKey) {
    let mut raw = [0u8; 32];
    getrandom::fill(&mut raw).unwrap();
    let secret = StaticSecret::from(raw);
    let public = PublicKey::from(&secret);
    (raw, *public.as_bytes(), secret, public)
}

fn b64(bytes: &[u8]) -> String {
    const T: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::new();
    for chunk in bytes.chunks(3) {
        let b = [chunk[0], *chunk.get(1).unwrap_or(&0), *chunk.get(2).unwrap_or(&0)];
        let n = (u32::from(b[0]) << 16) | (u32::from(b[1]) << 8) | u32::from(b[2]);
        out.push(T[((n >> 18) & 63) as usize] as char);
        out.push(T[((n >> 12) & 63) as usize] as char);
        out.push(if chunk.len() > 1 {
            T[((n >> 6) & 63) as usize] as char
        } else {
            '='
        });
        out.push(if chunk.len() > 2 {
            T[(n & 63) as usize] as char
        } else {
            '='
        });
    }
    out
}

// --- in-memory smoltcp device for the fake server -----------------------------

struct Phy {
    rx: VecDeque<Vec<u8>>,
    tx: VecDeque<Vec<u8>>,
}

struct PhyRx {
    buf: Vec<u8>,
}
struct PhyTx<'a> {
    tx: &'a mut VecDeque<Vec<u8>>,
}

impl Device for Phy {
    type RxToken<'a> = PhyRx;
    type TxToken<'a> = PhyTx<'a>;
    fn receive(&mut self, _t: SmolInstant) -> Option<(Self::RxToken<'_>, Self::TxToken<'_>)> {
        let buf = self.rx.pop_front()?;
        Some((PhyRx { buf }, PhyTx { tx: &mut self.tx }))
    }
    fn transmit(&mut self, _t: SmolInstant) -> Option<Self::TxToken<'_>> {
        Some(PhyTx { tx: &mut self.tx })
    }
    fn capabilities(&self) -> DeviceCapabilities {
        let mut caps = DeviceCapabilities::default();
        caps.medium = Medium::Ip;
        caps.max_transmission_unit = MTU;
        caps
    }
}
impl RxToken for PhyRx {
    fn consume<R, F: FnOnce(&[u8]) -> R>(self, f: F) -> R {
        f(&self.buf)
    }
}
impl TxToken for PhyTx<'_> {
    fn consume<R, F: FnOnce(&mut [u8]) -> R>(self, len: usize, f: F) -> R {
        let mut buf = vec![0u8; len];
        let r = f(&mut buf);
        self.tx.push_back(buf);
        r
    }
}

fn now_since(start: Instant) -> SmolInstant {
    SmolInstant::from_micros(start.elapsed().as_micros() as i64)
}

/// Run the fake WireGuard server: decrypt UDP from the client into a smoltcp
/// stack that echoes the inner TCP, and encrypt the stack's output back.
/// `rx_count` is bumped for every datagram received from the client (including
/// keepalives), so tests can observe that the client's timers keep firing.
async fn run_fake_server(udp: UdpSocket, mut tunn: Tunn, rx_count: Arc<AtomicU64>, tag: u8) {
    let start = Instant::now();
    let mut phy = Phy {
        rx: VecDeque::new(),
        tx: VecDeque::new(),
    };
    let mut iface = {
        let cfg = IfaceConfig::new(HardwareAddress::Ip);
        let mut iface = Interface::new(cfg, &mut phy, now_since(start));
        iface.set_any_ip(true);
        iface.update_ip_addrs(|a| {
            let _ = a.push(IpCidr::new(IpAddress::Ipv4(Ipv4Address::from(INNER_IP)), 0));
        });
        let _ = iface.routes_mut().add_default_ipv4_route(Ipv4Address::from(INNER_IP));
        iface
    };
    let mut sockets = SocketSet::new(Vec::new());
    let mut listener = tcp::Socket::new(
        tcp::SocketBuffer::new(vec![0u8; 256 * 1024]),
        tcp::SocketBuffer::new(vec![0u8; 256 * 1024]),
    );
    listener.listen(INNER_PORT).unwrap();
    let handle = sockets.add(listener);

    // A UDP echo socket on the same inner port, so UDP-relay tests round-trip.
    let mut udp_echo = udp::Socket::new(
        udp::PacketBuffer::new(vec![udp::PacketMetadata::EMPTY; 32], vec![0u8; 256 * 1024]),
        udp::PacketBuffer::new(vec![udp::PacketMetadata::EMPTY; 32], vec![0u8; 256 * 1024]),
    );
    udp_echo.bind(INNER_PORT).unwrap();
    let udp_handle = sockets.add(udp_echo);

    // A DNS responder on UDP/53 that answers every `A` query with INNER_IP, so
    // tunnel-side DNS resolution lands on the echo sockets above.
    let mut dns_sock = udp::Socket::new(
        udp::PacketBuffer::new(vec![udp::PacketMetadata::EMPTY; 32], vec![0u8; 64 * 1024]),
        udp::PacketBuffer::new(vec![udp::PacketMetadata::EMPTY; 32], vec![0u8; 64 * 1024]),
    );
    dns_sock.bind(53).unwrap();
    let dns_handle = sockets.add(dns_sock);

    let mut udp_buf = vec![0u8; 65535];
    let mut scratch = vec![0u8; 65535 + 32];

    loop {
        let now = now_since(start);
        iface.poll(now, &mut phy, &mut sockets);

        // Echo any received bytes back to the client.
        let sock = sockets.get_mut::<tcp::Socket>(handle);
        while sock.can_recv() && sock.can_send() {
            let data = sock.recv(|b| (b.len(), b.to_vec())).unwrap_or_default();
            if data.is_empty() {
                break;
            }
            // A non-zero `tag` prefixes each echo so multi-peer tests can tell
            // which server (hence which peer) actually handled the flow.
            if tag != 0 {
                let _ = sock.send_slice(&[tag]);
            }
            let _ = sock.send_slice(&data);
        }

        // Echo any received UDP datagram back to its sender.
        let usock = sockets.get_mut::<udp::Socket>(udp_handle);
        while usock.can_recv() {
            let (data, endpoint) = match usock.recv() {
                Ok((d, meta)) => (d.to_vec(), meta.endpoint),
                Err(_) => break,
            };
            let _ = usock.send_slice(&data, endpoint);
        }

        // Answer DNS `A` queries with INNER_IP.
        let dsock = sockets.get_mut::<udp::Socket>(dns_handle);
        while dsock.can_recv() {
            let (query, endpoint) = match dsock.recv() {
                Ok((d, meta)) => (d.to_vec(), meta.endpoint),
                Err(_) => break,
            };
            if let Some(reply) = dns_a_reply(&query, INNER_IP) {
                let _ = dsock.send_slice(&reply, endpoint);
            }
        }

        while let Some(pkt) = phy.tx.pop_front() {
            if let TunnResult::WriteToNetwork(out) = tunn.encapsulate(&pkt, &mut scratch) {
                let _ = udp.send(out).await;
            }
        }

        let delay = iface
            .poll_delay(now_since(start), &sockets)
            .map(|d| Duration::from_micros(d.total_micros()))
            .map_or(Duration::from_millis(50), |d| d.min(Duration::from_millis(50)));

        tokio::select! {
            res = udp.recv(&mut udp_buf) => {
                if let Ok(n) = res {
                    rx_count.fetch_add(1, Ordering::Relaxed);
                    let mut first = true;
                    loop {
                        let datagram: &[u8] = if first { &udp_buf[..n] } else { &[] };
                        match tunn.decapsulate(None, datagram, &mut scratch) {
                            TunnResult::WriteToNetwork(out) => {
                                let _ = udp.send(out).await;
                                first = false;
                            }
                            TunnResult::WriteToTunnelV4(p, _) | TunnResult::WriteToTunnelV6(p, _) => {
                                phy.rx.push_back(p.to_vec());
                                break;
                            }
                            _ => break,
                        }
                    }
                }
            }
            _ = tokio::time::sleep(delay) => {
                if let TunnResult::WriteToNetwork(out) = tunn.update_timers(&mut scratch) {
                    let _ = udp.send(out).await;
                }
            }
        }
    }
}

/// Build a DNS response answering each `A` query in `request` with `ip`. Other
/// query types get an empty NOERROR reply. Returns `None` for a non-query.
fn dns_a_reply(request: &[u8], ip: Ipv4Addr) -> Option<Vec<u8>> {
    let request = Message::from_vec(request).ok()?;
    let mut response = Message::new();
    response.set_id(request.id());
    response.set_message_type(MessageType::Response);
    response.set_op_code(OpCode::Query);
    response.set_recursion_desired(request.recursion_desired());
    response.set_recursion_available(true);
    response.set_response_code(ResponseCode::NoError);
    for query in request.queries() {
        response.add_query(query.clone());
        if query.query_type() == RecordType::A {
            let record = Record::from_rdata(query.name().clone(), 60, RData::A(A(ip)));
            response.add_answer(record);
        }
    }
    response.to_vec().ok()
}

/// Stand up a fake server bound to an ephemeral UDP port and return a parsed
/// client config pointing at it.
async fn start_server() -> WireGuardOutboundConfig {
    start_server_with("").await
}

/// Like [`start_server`], but splices `extra_opts` (a leading-comma YAML
/// fragment) into the client config map.
async fn start_server_with(extra_opts: &str) -> WireGuardOutboundConfig {
    start_server_counting(extra_opts).await.0
}

/// Like [`start_server_with`], but also returns a counter of datagrams the
/// server has received from the client (for observing keepalive / timer ticks).
async fn start_server_counting(extra_opts: &str) -> (WireGuardOutboundConfig, Arc<AtomicU64>) {
    let rx_count = Arc::new(AtomicU64::new(0));
    let rx_count_srv = rx_count.clone();
    let (client_priv, client_pub, _cs, _cp) = keypair();
    let (server_priv_raw, server_pub, _ss, _sp) = keypair();
    let server_secret = StaticSecret::from(server_priv_raw);

    let server_udp = UdpSocket::bind((Ipv4Addr::LOCALHOST, 0)).await.unwrap();
    let server_addr = server_udp.local_addr().unwrap();

    // Server tunnel: server's own secret + the client's public key.
    let server_tunn = Tunn::new(server_secret, PublicKey::from(client_pub), None, None, 1, None);

    // The server must learn the client's source address from the first packet;
    // connect it after the first recv. boringtun's `decapsulate(None, …)` works
    // without a connected socket, but we need to send replies, so peek first.
    tokio::spawn(async move {
        // Wait for the first datagram to learn the client's addr, then connect.
        let mut buf = vec![0u8; 65535];
        let (n, peer) = server_udp.recv_from(&mut buf).await.unwrap();
        server_udp.connect(peer).await.unwrap();
        // Re-inject the first datagram by handling it before entering the loop.
        let mut tunn = server_tunn;
        let mut scratch = vec![0u8; 65535 + 32];
        let mut first = true;
        loop {
            let datagram: &[u8] = if first { &buf[..n] } else { &[] };
            match tunn.decapsulate(None, datagram, &mut scratch) {
                TunnResult::WriteToNetwork(out) => {
                    let _ = server_udp.send(out).await;
                    first = false;
                }
                _ => break,
            }
        }
        Box::pin(run_fake_server(server_udp, tunn, rx_count_srv, 0)).await;
    });

    let yaml = format!(
        "{{ name: wg, type: wireguard, server: 127.0.0.1, port: {}, \
         private-key: {}, public-key: {}, ip: 10.0.0.2{} }}",
        server_addr.port(),
        b64(&client_priv),
        b64(&server_pub),
        extra_opts,
    );
    let entry: ProxyEntry = serde_yaml_ng::from_str(&yaml).unwrap();
    (WireGuardOutboundConfig::from_proxy(&entry).unwrap(), rx_count)
}

/// Stand up a fake server keyed to `client_pub` (the shared interface key) on an
/// ephemeral port. Its TCP echoes are prefixed with `tag` so a test can tell
/// which peer carried a flow. Returns the listening port and the server's public
/// key (the peer's `public-key`).
async fn start_peer_for(client_pub: [u8; 32], tag: u8) -> (u16, [u8; 32]) {
    let (_, server_pub, server_secret, _) = keypair();
    let server_udp = UdpSocket::bind((Ipv4Addr::LOCALHOST, 0)).await.unwrap();
    let port = server_udp.local_addr().unwrap().port();
    let server_tunn = Tunn::new(server_secret, PublicKey::from(client_pub), None, None, 1, None);
    tokio::spawn(async move {
        let mut buf = vec![0u8; 65535];
        let (n, peer) = server_udp.recv_from(&mut buf).await.unwrap();
        server_udp.connect(peer).await.unwrap();
        let mut tunn = server_tunn;
        let mut scratch = vec![0u8; 65535 + 32];
        let mut first = true;
        loop {
            let datagram: &[u8] = if first { &buf[..n] } else { &[] };
            match tunn.decapsulate(None, datagram, &mut scratch) {
                TunnResult::WriteToNetwork(out) => {
                    let _ = server_udp.send(out).await;
                    first = false;
                }
                _ => break,
            }
        }
        Box::pin(run_fake_server(server_udp, tunn, Arc::new(AtomicU64::new(0)), tag)).await;
    });
    (port, server_pub)
}

#[tokio::test]
async fn wireguard_routes_to_the_peer_matching_allowed_ips() {
    // Two peers behind one interface key, each owning a distinct inner /24. The
    // device must route an inner packet to the peer whose `allowed-ips` matches;
    // each server tags its echo so we can confirm the right one handled it.
    let (client_priv, client_pub, _, _) = keypair();
    let (port_a, pub_a) = start_peer_for(client_pub, 0xA1).await;
    let (port_b, pub_b) = start_peer_for(client_pub, 0xB2).await;

    let yaml = format!(
        "{{ name: wg, type: wireguard, server: 127.0.0.1, port: {port_a}, \
         private-key: {priv_k}, public-key: {pub_a_k}, ip: 10.0.0.2, \
         allowed-ips: [10.0.1.0/24], \
         peers: [ {{ server: 127.0.0.1, port: {port_b}, public-key: {pub_b_k}, \
         allowed-ips: [10.0.2.0/24] }} ] }}",
        priv_k = b64(&client_priv),
        pub_a_k = b64(&pub_a),
        pub_b_k = b64(&pub_b),
    );
    let entry: ProxyEntry = serde_yaml_ng::from_str(&yaml).unwrap();
    let config = WireGuardOutboundConfig::from_proxy(&entry).unwrap();

    // 10.0.1.1 is inside peer A's /24 -> tag 0xA1.
    let target_a = TargetAddr::Ip(SocketAddr::new(IpAddr::V4(Ipv4Addr::new(10, 0, 1, 1)), INNER_PORT));
    let mut stream_a = tokio::time::timeout(Duration::from_secs(15), wireguard::connect(&config, &target_a))
        .await
        .expect("connect to peer A did not time out")
        .expect("wireguard connect peer A");
    stream_a.write_all(b"alpha").await.unwrap();
    let mut got_a = vec![0u8; 6];
    stream_a.read_exact(&mut got_a).await.unwrap();
    assert_eq!(
        got_a,
        [0xA1, b'a', b'l', b'p', b'h', b'a'],
        "flow to 10.0.1.1 must ride peer A"
    );

    // 10.0.2.1 is inside peer B's /24 -> tag 0xB2.
    let target_b = TargetAddr::Ip(SocketAddr::new(IpAddr::V4(Ipv4Addr::new(10, 0, 2, 1)), INNER_PORT));
    let mut stream_b = tokio::time::timeout(Duration::from_secs(15), wireguard::connect(&config, &target_b))
        .await
        .expect("connect to peer B did not time out")
        .expect("wireguard connect peer B");
    stream_b.write_all(b"bravo").await.unwrap();
    let mut got_b = vec![0u8; 6];
    stream_b.read_exact(&mut got_b).await.unwrap();
    assert_eq!(
        got_b,
        [0xB2, b'b', b'r', b'a', b'v', b'o'],
        "flow to 10.0.2.1 must ride peer B"
    );
}

#[tokio::test]
async fn wireguard_tcp_round_trips_a_small_payload() {
    let config = start_server().await;
    let target = TargetAddr::Ip(SocketAddr::new(IpAddr::V4(INNER_IP), INNER_PORT));

    let mut stream = tokio::time::timeout(Duration::from_secs(15), wireguard::connect(&config, &target))
        .await
        .expect("connect did not time out")
        .expect("wireguard connect");

    let payload = b"hello wireguard tunnel";
    stream.write_all(payload).await.unwrap();

    let mut got = vec![0u8; payload.len()];
    stream.read_exact(&mut got).await.unwrap();
    assert_eq!(&got, payload);
}

#[tokio::test]
async fn wireguard_tcp_round_trips_a_large_payload() {
    let config = start_server().await;
    let target = TargetAddr::Ip(SocketAddr::new(IpAddr::V4(INNER_IP), INNER_PORT));

    let stream = tokio::time::timeout(Duration::from_secs(15), wireguard::connect(&config, &target))
        .await
        .expect("connect did not time out")
        .expect("wireguard connect");

    // 64 KiB spans many tunnel frames (MTU 1408).
    let payload: Vec<u8> = (0..64 * 1024).map(|i| (i % 251) as u8).collect();
    let writer_payload = payload.clone();
    let (mut rd, mut wr) = tokio::io::split(stream);
    let writer = tokio::spawn(async move {
        wr.write_all(&writer_payload).await.unwrap();
        wr.flush().await.unwrap();
    });

    let mut got = vec![0u8; payload.len()];
    rd.read_exact(&mut got).await.unwrap();
    writer.await.unwrap();
    assert_eq!(got, payload);
}

#[tokio::test]
async fn wireguard_udp_round_trips_datagrams() {
    let config = start_server().await;
    let target = TargetAddr::Ip(SocketAddr::new(IpAddr::V4(INNER_IP), INNER_PORT));

    let assoc = tokio::time::timeout(Duration::from_secs(15), wireguard::connect_udp(&config, &target))
        .await
        .expect("connect_udp did not time out")
        .expect("wireguard connect_udp");

    // UDP has no retransmit, so datagrams sent before the Noise handshake
    // completes are lost. Retransmit a probe (as a real UDP client would) until
    // the tunnel is up and we see its echo, then drain any duplicate echoes.
    let probe = b"wg-udp-probe";
    let mut warmed = false;
    for _ in 0..50 {
        assoc.send(probe).await.unwrap();
        if let Ok(Ok(got)) = tokio::time::timeout(Duration::from_millis(300), assoc.recv()).await {
            assert_eq!(got, probe);
            warmed = true;
            break;
        }
    }
    assert!(warmed, "wireguard udp tunnel did not warm up");
    while tokio::time::timeout(Duration::from_millis(100), assoc.recv())
        .await
        .is_ok()
    {}

    // With the tunnel established, distinct datagrams (including a multi-hundred
    // byte one) round-trip 1:1 over localhost.
    for i in 0..5u8 {
        let payload: Vec<u8> = (0..(64 + usize::from(i) * 200)).map(|b| (b as u8) ^ i).collect();
        assoc.send(&payload).await.unwrap();
        let got = tokio::time::timeout(Duration::from_secs(5), assoc.recv())
            .await
            .expect("udp echo did not time out")
            .expect("udp echo");
        assert_eq!(got, payload);
    }
}

#[tokio::test]
async fn wireguard_keepalive_keeps_a_long_idle_session_alive() {
    // `persistent-keepalive: 1` makes the client emit a keepalive every second
    // while idle. The device loop must keep driving `Tunn::update_timers` for
    // these to flow; we observe them via the server's receive counter and then
    // confirm the long-lived connection still relays after the idle gap.
    let (config, rx_count) = start_server_counting(", persistent-keepalive: 1").await;
    let target = TargetAddr::Ip(SocketAddr::new(IpAddr::V4(INNER_IP), INNER_PORT));

    let mut stream = tokio::time::timeout(Duration::from_secs(15), wireguard::connect(&config, &target))
        .await
        .expect("connect did not time out")
        .expect("wireguard connect");
    stream.write_all(b"warmup").await.unwrap();
    let mut got = [0u8; 6];
    tokio::time::timeout(Duration::from_secs(5), stream.read_exact(&mut got))
        .await
        .expect("warmup echo did not time out")
        .unwrap();
    assert_eq!(&got, b"warmup");

    // Stay idle past several keepalive intervals; the client must keep sending.
    let before = rx_count.load(Ordering::Relaxed);
    tokio::time::sleep(Duration::from_millis(3200)).await;
    let after = rx_count.load(Ordering::Relaxed);
    assert!(
        after >= before + 2,
        "expected keepalive datagrams during idle (before={before}, after={after})"
    );

    // The connection survived the idle window and still round-trips.
    stream.write_all(b"after-idle").await.unwrap();
    let mut got = [0u8; 10];
    tokio::time::timeout(Duration::from_secs(5), stream.read_exact(&mut got))
        .await
        .expect("post-idle echo did not time out")
        .unwrap();
    assert_eq!(&got, b"after-idle");
}

#[tokio::test]
async fn wireguard_resolves_domain_targets_over_the_tunnel() {
    // `remote-dns-resolve` makes a domain target resolve by querying the `dns`
    // resolver *through* the tunnel; the fake server answers A=INNER_IP, so the
    // relayed TCP connection lands on the inner echo listener.
    let config = start_server_with(&format!(", remote-dns-resolve: true, dns: [\"{RESOLVER_IP}\"]")).await;
    let target = TargetAddr::Domain("echo.internal".to_string(), INNER_PORT);

    let mut stream = tokio::time::timeout(Duration::from_secs(15), wireguard::connect(&config, &target))
        .await
        .expect("connect did not time out")
        .expect("wireguard connect via tunnel DNS");

    let payload = b"resolved-over-the-tunnel";
    stream.write_all(payload).await.unwrap();
    let mut got = vec![0u8; payload.len()];
    tokio::time::timeout(Duration::from_secs(5), stream.read_exact(&mut got))
        .await
        .expect("echo did not time out")
        .unwrap();
    assert_eq!(&got, payload);
}

// --- AmneziaWG obfuscation interop -------------------------------------------

/// Standard WireGuard message sizes (boringtun emits these), used to recover an
/// obfuscated message's type on the server's RX path.
const WG_INIT: usize = 148;
const WG_RESP: usize = 92;
const WG_COOKIE: usize = 64;
const WG_TRANSPORT_MIN: usize = 32;

/// AmneziaWG obfuscation parameters mirrored on the test's fake server so the
/// client's obfuscated handshake/transport packets round-trip. `s1`/`s2` are the
/// random prefix paddings; `h1`-`h4` rewrite the 4-byte message-type header of
/// the init / response / cookie / transport messages.
#[derive(Clone, Copy)]
struct Obf {
    s1: usize,
    s2: usize,
    h1: u32,
    h2: u32,
    h3: u32,
    h4: u32,
}

/// Apply obfuscation to a boringtun message (mirrors the kernel TX path): prepend
/// `S1`/`S2` random padding and write the `H1`-`H4` header over the type field.
fn obfuscate(o: &Obf, out: &[u8]) -> Vec<u8> {
    if out.len() < 4 {
        return out.to_vec();
    }
    let (header, pad) = match out[0] {
        1 => (o.h1, o.s1),
        2 => (o.h2, o.s2),
        3 => (o.h3, 0),
        4 => (o.h4, 0),
        _ => return out.to_vec(),
    };
    let mut buf = vec![0u8; pad + out.len()];
    if pad > 0 {
        getrandom::fill(&mut buf[..pad]).unwrap();
    }
    buf[pad..].copy_from_slice(out);
    buf[pad..pad + 4].copy_from_slice(&header.to_le_bytes());
    buf
}

/// Reverse obfuscation in place (mirrors the kernel RX path): identify the
/// message by `(padding + size, header)`, strip the padding, and restore the
/// standard type byte. Returns the restored length, or `None` to drop (junk).
fn deobfuscate(o: &Obf, buf: &mut [u8]) -> Option<usize> {
    let size = buf.len();
    let hdr = |off: usize| {
        buf.get(off..off + 4)
            .map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
    };
    let (pad, ty) = if size == o.s1 + WG_INIT && hdr(o.s1) == Some(o.h1) {
        (o.s1, 1u8)
    } else if size == o.s2 + WG_RESP && hdr(o.s2) == Some(o.h2) {
        (o.s2, 2)
    } else if size == WG_COOKIE && hdr(0) == Some(o.h3) {
        (0, 3)
    } else if size >= WG_TRANSPORT_MIN && hdr(0) == Some(o.h4) {
        (0, 4)
    } else {
        return None;
    };
    if pad > 0 {
        buf.copy_within(pad.., 0);
    }
    let n = size - pad;
    buf[0] = ty;
    buf[1] = 0;
    buf[2] = 0;
    buf[3] = 0;
    Some(n)
}

/// A fake WireGuard server that applies the same AmneziaWG obfuscation as the
/// client: it deobfuscates incoming datagrams (dropping junk packets) before
/// decapsulation and obfuscates everything it sends. Echoes inner TCP.
async fn run_fake_server_obf(server_udp: UdpSocket, mut tunn: Tunn, obf: Obf) {
    let start = Instant::now();
    let mut phy = Phy {
        rx: VecDeque::new(),
        tx: VecDeque::new(),
    };
    let mut iface = {
        let cfg = IfaceConfig::new(HardwareAddress::Ip);
        let mut iface = Interface::new(cfg, &mut phy, now_since(start));
        iface.set_any_ip(true);
        iface.update_ip_addrs(|a| {
            let _ = a.push(IpCidr::new(IpAddress::Ipv4(Ipv4Address::from(INNER_IP)), 0));
        });
        let _ = iface.routes_mut().add_default_ipv4_route(Ipv4Address::from(INNER_IP));
        iface
    };
    let mut sockets = SocketSet::new(Vec::new());
    let mut listener = tcp::Socket::new(
        tcp::SocketBuffer::new(vec![0u8; 256 * 1024]),
        tcp::SocketBuffer::new(vec![0u8; 256 * 1024]),
    );
    listener.listen(INNER_PORT).unwrap();
    let handle = sockets.add(listener);

    let mut udp_buf = vec![0u8; 65535];
    let mut scratch = vec![0u8; 65535 + 32];
    let mut connected = false;

    loop {
        let now = now_since(start);
        iface.poll(now, &mut phy, &mut sockets);

        let sock = sockets.get_mut::<tcp::Socket>(handle);
        while sock.can_recv() && sock.can_send() {
            let data = sock.recv(|b| (b.len(), b.to_vec())).unwrap_or_default();
            if data.is_empty() {
                break;
            }
            let _ = sock.send_slice(&data);
        }

        while let Some(pkt) = phy.tx.pop_front() {
            if let TunnResult::WriteToNetwork(out) = tunn.encapsulate(&pkt, &mut scratch) {
                let _ = server_udp.send(&obfuscate(&obf, out)).await;
            }
        }

        let delay = iface
            .poll_delay(now_since(start), &sockets)
            .map(|d| Duration::from_micros(d.total_micros()))
            .map_or(Duration::from_millis(50), |d| d.min(Duration::from_millis(50)));

        tokio::select! {
            res = server_udp.recv_from(&mut udp_buf) => {
                if let Ok((n, peer)) = res {
                    if !connected {
                        server_udp.connect(peer).await.unwrap();
                        connected = true;
                    }
                    // Junk packets (and anything unrecognised) deobfuscate to None.
                    let m = match deobfuscate(&obf, &mut udp_buf[..n]) {
                        Some(m) => m,
                        None => continue,
                    };
                    let mut first = true;
                    loop {
                        let datagram: &[u8] = if first { &udp_buf[..m] } else { &[] };
                        match tunn.decapsulate(None, datagram, &mut scratch) {
                            TunnResult::WriteToNetwork(out) => {
                                let _ = server_udp.send(&obfuscate(&obf, out)).await;
                                first = false;
                            }
                            TunnResult::WriteToTunnelV4(p, _) | TunnResult::WriteToTunnelV6(p, _) => {
                                phy.rx.push_back(p.to_vec());
                                break;
                            }
                            _ => break,
                        }
                    }
                }
            }
            _ = tokio::time::sleep(delay) => {
                if let TunnResult::WriteToNetwork(out) = tunn.update_timers(&mut scratch) {
                    let _ = server_udp.send(&obfuscate(&obf, out)).await;
                }
            }
        }
    }
}

/// Stand up an obfuscating fake server and return a client config carrying the
/// matching `amnezia-wg-option` block.
async fn start_amnezia_server(obf: Obf, amnezia_yaml: &str) -> WireGuardOutboundConfig {
    let (client_priv, client_pub, _, _) = keypair();
    let (server_priv_raw, server_pub, _, _) = keypair();
    let server_secret = StaticSecret::from(server_priv_raw);

    let server_udp = UdpSocket::bind((Ipv4Addr::LOCALHOST, 0)).await.unwrap();
    let server_addr = server_udp.local_addr().unwrap();
    let server_tunn = Tunn::new(server_secret, PublicKey::from(client_pub), None, None, 1, None);

    tokio::spawn(async move {
        Box::pin(run_fake_server_obf(server_udp, server_tunn, obf)).await;
    });

    let yaml = format!(
        "{{ name: wg, type: wireguard, server: 127.0.0.1, port: {}, \
         private-key: {}, public-key: {}, ip: 10.0.0.2, {} }}",
        server_addr.port(),
        b64(&client_priv),
        b64(&server_pub),
        amnezia_yaml,
    );
    let entry: ProxyEntry = serde_yaml_ng::from_str(&yaml).unwrap();
    WireGuardOutboundConfig::from_proxy(&entry).unwrap()
}

#[tokio::test]
async fn wireguard_amnezia_obfuscation_round_trips() {
    // Client and server both apply junk packets, S1/S2 padding, and H1-H4 header
    // rewrites; the relayed TCP connection must still round-trip end to end.
    let obf = Obf {
        s1: 24,
        s2: 16,
        h1: 0x1122_3344,
        h2: 0x5566_7788,
        h3: 0x99aa_bbcc,
        h4: 0xddee_ff00,
    };
    let amnezia_yaml = "amnezia-wg-option: { jc: 3, jmin: 40, jmax: 70, s1: 24, s2: 16, \
         h1: 287454020, h2: 1432778632, h3: 2578103244, h4: 3723427584 }";
    let config = start_amnezia_server(obf, amnezia_yaml).await;
    let target = TargetAddr::Ip(SocketAddr::new(IpAddr::V4(INNER_IP), INNER_PORT));

    let mut stream = tokio::time::timeout(Duration::from_secs(15), wireguard::connect(&config, &target))
        .await
        .expect("connect did not time out")
        .expect("wireguard connect with amnezia obfuscation");

    let payload = b"amnezia-obfuscated-tunnel";
    stream.write_all(payload).await.unwrap();
    let mut got = vec![0u8; payload.len()];
    tokio::time::timeout(Duration::from_secs(5), stream.read_exact(&mut got))
        .await
        .expect("echo did not time out")
        .unwrap();
    assert_eq!(&got, payload);
}

#[test]
fn from_proxy_rejects_invalid_amnezia_headers() {
    let key = b64(&[3u8; 32]);
    // h1 == h2 (headers must be distinct so RX can recover the type).
    let yaml = format!(
        "{{ name: wg, type: wireguard, server: 1.2.3.4, port: 51820, \
         private-key: {key}, public-key: {key}, ip: 10.0.0.2, \
         amnezia-wg-option: {{ h1: 10, h2: 10, h3: 11, h4: 12 }} }}"
    );
    let entry: ProxyEntry = serde_yaml_ng::from_str(&yaml).unwrap();
    assert!(WireGuardOutboundConfig::from_proxy(&entry).is_err());

    // A header colliding with a standard message type (<= 4) is rejected.
    let yaml = format!(
        "{{ name: wg, type: wireguard, server: 1.2.3.4, port: 51820, \
         private-key: {key}, public-key: {key}, ip: 10.0.0.2, \
         amnezia-wg-option: {{ h1: 4, h2: 10, h3: 11, h4: 12 }} }}"
    );
    let entry: ProxyEntry = serde_yaml_ng::from_str(&yaml).unwrap();
    assert!(WireGuardOutboundConfig::from_proxy(&entry).is_err());
}

#[test]
fn from_proxy_rejects_remote_dns_resolve_without_dns_servers() {
    // Valid 32-byte keys so parsing reaches the remote-dns validation.
    let key = b64(&[0u8; 32]);
    let yaml = format!(
        "{{ name: wg, type: wireguard, server: 1.2.3.4, port: 51820, \
         private-key: {key}, public-key: {key}, ip: 10.0.0.2, remote-dns-resolve: true }}"
    );
    let entry: ProxyEntry = serde_yaml_ng::from_str(&yaml).unwrap();
    assert!(WireGuardOutboundConfig::from_proxy(&entry).is_err());
}

#[test]
fn from_proxy_requires_keys_and_an_assigned_address() {
    // Missing public-key.
    let yaml = "{ name: wg, type: wireguard, server: 1.2.3.4, port: 51820, private-key: QUJD }";
    let entry: ProxyEntry = serde_yaml_ng::from_str(yaml).unwrap();
    assert!(WireGuardOutboundConfig::from_proxy(&entry).is_err());

    // Valid 32-byte keys but no ip/ipv6.
    let key = b64(&[7u8; 32]);
    let yaml =
        format!("{{ name: wg, type: wireguard, server: 1.2.3.4, port: 51820, private-key: {key}, public-key: {key} }}");
    let entry: ProxyEntry = serde_yaml_ng::from_str(&yaml).unwrap();
    assert!(WireGuardOutboundConfig::from_proxy(&entry).is_err());

    // Complete config parses.
    let yaml = format!(
        "{{ name: wg, type: wireguard, server: 1.2.3.4, port: 51820, \
         private-key: {key}, public-key: {key}, ip: 10.0.0.2/32 }}"
    );
    let entry: ProxyEntry = serde_yaml_ng::from_str(&yaml).unwrap();
    let config = WireGuardOutboundConfig::from_proxy(&entry).unwrap();
    assert_eq!(config.server, "1.2.3.4");
    assert_eq!(config.port, 51820);
}
