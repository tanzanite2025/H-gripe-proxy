//! End-to-end test for the TUN inbound, without a real OS TUN device.
//!
//! A second, independent smoltcp stack plays the role of the OS networking
//! stack ("the client"): it performs a real TCP handshake to a destination,
//! sends bytes, and reads the reply — all as raw IP frames exchanged with
//! `learn_gripe::serve_tun` over in-memory channels. The kernel's outbound is
//! `Direct`, so the flow is dialed to a real tokio echo server. Real bytes
//! traverse two real TCP state machines plus the kernel relay.

use std::net::SocketAddr;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context as TaskContext, Poll};
use std::time::{Duration, Instant};

use learn_gripe::{DEFAULT_MTU, DnsMode, FakeIpConfig, OutboundMode, serve_tun, serve_tun_device};
use smoltcp::iface::{Config, Interface, SocketSet};
use smoltcp::phy::{ChecksumCapabilities, Device, DeviceCapabilities, Medium, RxToken, TxToken};
use smoltcp::socket::tcp;
use smoltcp::time::Instant as SmolInstant;
use smoltcp::wire::{
    HardwareAddress, IpAddress, IpCidr, IpEndpoint, IpProtocol, Ipv4Address, Ipv4Packet, Ipv4Repr, UdpPacket, UdpRepr,
};
use std::collections::VecDeque;
use std::net::Ipv4Addr;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt, ReadBuf};
use tokio::net::{TcpListener, UdpSocket};
use tokio::sync::{Notify, mpsc};

#[tokio::test]
async fn tun_relays_tcp_flow_to_direct_outbound() {
    // Echo server reached via the kernel's Direct outbound.
    let echo = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let echo_addr = echo.local_addr().unwrap();
    tokio::spawn(async move {
        let (mut sock, _) = echo.accept().await.unwrap();
        let mut buf = [0u8; 1024];
        loop {
            match sock.read(&mut buf).await {
                Ok(0) | Err(_) => break,
                Ok(n) => {
                    if sock.write_all(&buf[..n]).await.is_err() {
                        break;
                    }
                }
            }
        }
    });

    // In-memory TUN: client <-> kernel frame channels.
    let (to_kernel_tx, to_kernel_rx) = mpsc::channel::<Vec<u8>>(256);
    let (to_client_tx, to_client_rx) = mpsc::channel::<Vec<u8>>(256);

    let shutdown = Arc::new(Notify::new());
    let kernel = tokio::spawn(serve_tun(
        to_kernel_rx,
        to_client_tx,
        OutboundMode::Direct,
        None,
        shutdown.clone(),
        DEFAULT_MTU,
    ));

    let payload = b"hello tun relay".to_vec();
    let echoed = tokio::time::timeout(
        Duration::from_secs(10),
        run_client(echo_addr, payload.clone(), to_kernel_tx, to_client_rx),
    )
    .await
    .expect("client timed out");

    assert_eq!(echoed, payload, "bytes should round-trip through the TUN relay");

    shutdown.notify_waiters();
    kernel.abort();
}

#[tokio::test]
async fn tun_relays_multi_segment_payload() {
    let echo = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let echo_addr = echo.local_addr().unwrap();
    tokio::spawn(async move {
        let (mut sock, _) = echo.accept().await.unwrap();
        let mut buf = vec![0u8; 16 * 1024];
        loop {
            match sock.read(&mut buf).await {
                Ok(0) | Err(_) => break,
                Ok(n) => {
                    if sock.write_all(&buf[..n]).await.is_err() {
                        break;
                    }
                }
            }
        }
    });

    let (to_kernel_tx, to_kernel_rx) = mpsc::channel::<Vec<u8>>(1024);
    let (to_client_tx, to_client_rx) = mpsc::channel::<Vec<u8>>(1024);
    let shutdown = Arc::new(Notify::new());
    let kernel = tokio::spawn(serve_tun(
        to_kernel_rx,
        to_client_tx,
        OutboundMode::Direct,
        None,
        shutdown.clone(),
        DEFAULT_MTU,
    ));

    // Much larger than the MTU and the socket buffers, so it spans many
    // segments and exercises back-pressure in both directions.
    let payload: Vec<u8> = (0..256 * 1024).map(|i| (i % 251) as u8).collect();
    let echoed = tokio::time::timeout(
        Duration::from_secs(20),
        run_client(echo_addr, payload.clone(), to_kernel_tx, to_client_rx),
    )
    .await
    .expect("client timed out");

    assert_eq!(echoed.len(), payload.len(), "all bytes should round-trip");
    assert_eq!(echoed, payload, "multi-segment payload should be intact");

    shutdown.notify_waiters();
    kernel.abort();
}

/// Same round-trip, but driven through [`serve_tun_device`] over a mock device
/// that delivers/accepts one IP packet per read/write — the contract a real
/// `tun`-crate device exposes. This exercises the device<->stack frame pump the
/// OS adapter will rely on, still without any OS device.
#[tokio::test]
async fn tun_device_pump_relays_tcp_flow() {
    let echo = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let echo_addr = echo.local_addr().unwrap();
    tokio::spawn(async move {
        let (mut sock, _) = echo.accept().await.unwrap();
        let mut buf = [0u8; 1024];
        loop {
            match sock.read(&mut buf).await {
                Ok(0) | Err(_) => break,
                Ok(n) => {
                    if sock.write_all(&buf[..n]).await.is_err() {
                        break;
                    }
                }
            }
        }
    });

    // Mock device: client_to_kernel feeds the device's read side; the device's
    // write side feeds kernel_to_client. serve_tun_device pumps between the
    // device and its internal serve_tun channels.
    let (client_to_kernel_tx, client_to_kernel_rx) = mpsc::channel::<Vec<u8>>(256);
    let (kernel_to_client_tx, kernel_to_client_rx) = mpsc::channel::<Vec<u8>>(256);
    let device = MockTunDevice::new(client_to_kernel_rx, kernel_to_client_tx);
    let (dev_reader, dev_writer) = tokio::io::split(device);

    let shutdown = Arc::new(Notify::new());
    let kernel = tokio::spawn(serve_tun_device(
        dev_reader,
        dev_writer,
        OutboundMode::Direct,
        None,
        shutdown.clone(),
        DEFAULT_MTU,
    ));

    let payload = b"hello tun device pump".to_vec();
    let echoed = tokio::time::timeout(
        Duration::from_secs(10),
        run_client(echo_addr, payload.clone(), client_to_kernel_tx, kernel_to_client_rx),
    )
    .await
    .expect("client timed out");

    assert_eq!(echoed, payload, "bytes should round-trip through the device pump");

    shutdown.notify_waiters();
    kernel.abort();
}

/// Drive a client smoltcp stack: connect to `remote`, send `payload`, read the
/// echo back, then close. Returns the bytes received.
async fn run_client(
    remote: SocketAddr,
    payload: Vec<u8>,
    to_kernel: mpsc::Sender<Vec<u8>>,
    mut from_kernel: mpsc::Receiver<Vec<u8>>,
) -> Vec<u8> {
    let start = Instant::now();
    let mut phy = ClientPhy::new(DEFAULT_MTU);
    let mut iface = Interface::new(Config::new(HardwareAddress::Ip), &mut phy, now(start));
    iface.update_ip_addrs(|addrs| {
        let _ = addrs.push(IpCidr::new(IpAddress::Ipv4(Ipv4Address::new(10, 0, 0, 1)), 24));
    });
    let _ = iface.routes_mut().add_default_ipv4_route(Ipv4Address::new(10, 0, 0, 1));

    let mut sockets = SocketSet::new(Vec::new());
    let handle = sockets.add(tcp::Socket::new(
        tcp::SocketBuffer::new(vec![0u8; 64 * 1024]),
        tcp::SocketBuffer::new(vec![0u8; 64 * 1024]),
    ));

    let remote_ep = IpEndpoint::new(
        IpAddress::Ipv4(Ipv4Address::from(match remote.ip() {
            std::net::IpAddr::V4(v4) => v4.octets(),
            std::net::IpAddr::V6(_) => panic!("test uses ipv4"),
        })),
        remote.port(),
    );
    {
        let sock = sockets.get_mut::<tcp::Socket>(handle);
        sock.connect(iface.context(), remote_ep, 40000u16).unwrap();
    }

    let mut sent_off = 0usize;
    let mut received: Vec<u8> = Vec::new();

    loop {
        iface.poll(now(start), &mut phy, &mut sockets);
        {
            let sock = sockets.get_mut::<tcp::Socket>(handle);
            while sent_off < payload.len() && sock.can_send() {
                match sock.send_slice(&payload[sent_off..]) {
                    Ok(0) | Err(_) => break,
                    Ok(n) => sent_off += n,
                }
            }
            while sock.can_recv() {
                let chunk = sock.recv(|b| (b.len(), b.to_vec())).unwrap_or_default();
                if chunk.is_empty() {
                    break;
                }
                received.extend_from_slice(&chunk);
            }
            // The payload has round-tripped: initiate close, flush the FIN, and
            // return without blocking on the full TIME-WAIT teardown.
            if sent_off >= payload.len() && received.len() >= payload.len() {
                sock.close();
                drain_tx(&mut phy, &to_kernel).await;
                return received;
            }
        }

        drain_tx(&mut phy, &to_kernel).await;

        let delay = iface
            .poll_delay(now(start), &sockets)
            .map(|d| Duration::from_micros(d.total_micros()))
            .map_or(Duration::from_millis(5), |d| d.min(Duration::from_millis(5)));

        tokio::select! {
            frame = from_kernel.recv() => {
                match frame {
                    Some(frame) => phy.rx.push_back(frame),
                    None => return received,
                }
            }
            _ = tokio::time::sleep(delay) => {}
        }
    }
}

async fn drain_tx(phy: &mut ClientPhy, to_kernel: &mpsc::Sender<Vec<u8>>) {
    while let Some(frame) = phy.tx.pop_front() {
        if to_kernel.send(frame).await.is_err() {
            return;
        }
    }
}

fn now(start: Instant) -> SmolInstant {
    SmolInstant::from_micros(start.elapsed().as_micros() as i64)
}

struct ClientPhy {
    rx: VecDeque<Vec<u8>>,
    tx: VecDeque<Vec<u8>>,
    mtu: usize,
}

impl ClientPhy {
    fn new(mtu: usize) -> Self {
        Self {
            rx: VecDeque::new(),
            tx: VecDeque::new(),
            mtu,
        }
    }
}

struct ClientRxToken {
    buf: Vec<u8>,
}

struct ClientTxToken<'a> {
    tx: &'a mut VecDeque<Vec<u8>>,
}

impl Device for ClientPhy {
    type RxToken<'a> = ClientRxToken;
    type TxToken<'a> = ClientTxToken<'a>;

    fn receive(&mut self, _timestamp: SmolInstant) -> Option<(Self::RxToken<'_>, Self::TxToken<'_>)> {
        let buf = self.rx.pop_front()?;
        Some((ClientRxToken { buf }, ClientTxToken { tx: &mut self.tx }))
    }

    fn transmit(&mut self, _timestamp: SmolInstant) -> Option<Self::TxToken<'_>> {
        Some(ClientTxToken { tx: &mut self.tx })
    }

    fn capabilities(&self) -> DeviceCapabilities {
        let mut caps = DeviceCapabilities::default();
        caps.medium = Medium::Ip;
        caps.max_transmission_unit = self.mtu;
        caps
    }
}

impl RxToken for ClientRxToken {
    fn consume<R, F: FnOnce(&[u8]) -> R>(self, f: F) -> R {
        f(&self.buf)
    }
}

impl TxToken for ClientTxToken<'_> {
    fn consume<R, F: FnOnce(&mut [u8]) -> R>(self, len: usize, f: F) -> R {
        let mut buf = vec![0u8; len];
        let result = f(&mut buf);
        self.tx.push_back(buf);
        result
    }
}

/// A mock TUN device with the same I/O contract as the `tun` crate's async
/// device: each `read` yields exactly one queued IP packet, and each `write`
/// is delivered as exactly one packet. Backed by mpsc channels so a test can
/// feed/observe frames.
struct MockTunDevice {
    rx: mpsc::Receiver<Vec<u8>>,
    tx: mpsc::Sender<Vec<u8>>,
}

impl MockTunDevice {
    fn new(rx: mpsc::Receiver<Vec<u8>>, tx: mpsc::Sender<Vec<u8>>) -> Self {
        Self { rx, tx }
    }
}

impl AsyncRead for MockTunDevice {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut TaskContext<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<std::io::Result<()>> {
        match self.rx.poll_recv(cx) {
            Poll::Ready(Some(frame)) => {
                let n = frame.len().min(buf.remaining());
                buf.put_slice(&frame[..n]);
                Poll::Ready(Ok(()))
            }
            // Channel closed -> EOF.
            Poll::Ready(None) => Poll::Ready(Ok(())),
            Poll::Pending => Poll::Pending,
        }
    }
}

impl AsyncWrite for MockTunDevice {
    fn poll_write(self: Pin<&mut Self>, _cx: &mut TaskContext<'_>, buf: &[u8]) -> Poll<std::io::Result<usize>> {
        // serve_tun_device calls write_all with one whole frame; deliver it as
        // a single packet.
        let _ = self.tx.try_send(buf.to_vec());
        Poll::Ready(Ok(buf.len()))
    }

    fn poll_flush(self: Pin<&mut Self>, _cx: &mut TaskContext<'_>) -> Poll<std::io::Result<()>> {
        Poll::Ready(Ok(()))
    }

    fn poll_shutdown(self: Pin<&mut Self>, _cx: &mut TaskContext<'_>) -> Poll<std::io::Result<()>> {
        Poll::Ready(Ok(()))
    }
}

/// DNS over TUN: a UDP query to port 53 fed into the stack is answered in-place
/// from the fake-IP pool — no OS device, no upstream resolver. This is the piece
/// that lets a global default-route capture work without black-holing DNS: the
/// client resolves names to fake IPs, then opens TCP to those (already handled).
#[tokio::test]
async fn tun_answers_dns_query_from_fake_ip_pool() {
    use hickory_proto::op::{Message, MessageType, OpCode, Query};
    use hickory_proto::rr::{Name, RData, RecordType};

    let (mode, _pool) = DnsMode::fake_ip(FakeIpConfig::default());

    let (to_kernel_tx, to_kernel_rx) = mpsc::channel::<Vec<u8>>(16);
    let (from_kernel_tx, mut from_kernel_rx) = mpsc::channel::<Vec<u8>>(16);
    let shutdown = Arc::new(Notify::new());
    let kernel = tokio::spawn(serve_tun(
        to_kernel_rx,
        from_kernel_tx,
        OutboundMode::Direct,
        Some(mode),
        shutdown.clone(),
        DEFAULT_MTU,
    ));

    // Build an A query for example.com and wrap it in IPv4/UDP, client -> resolver.
    let mut request = Message::new();
    request.set_id(0x1234);
    request.set_message_type(MessageType::Query);
    request.set_op_code(OpCode::Query);
    request.set_recursion_desired(true);
    let mut query = Query::new();
    query.set_name(Name::from_ascii("example.com.").unwrap());
    query.set_query_type(RecordType::A);
    request.add_query(query);
    let dns_query = request.to_vec().unwrap();

    let client = Ipv4Address::new(10, 0, 0, 2);
    let resolver = Ipv4Address::new(10, 0, 0, 1);
    let query_frame = build_udp4_frame(client, 5300, resolver, 53, &dns_query);
    to_kernel_tx.send(query_frame).await.unwrap();

    let reply_frame = tokio::time::timeout(Duration::from_secs(5), from_kernel_rx.recv())
        .await
        .expect("dns reply timed out")
        .expect("kernel closed without replying");

    let (src, src_port, dst, dst_port, payload) = parse_udp4_frame(&reply_frame).expect("reply is a udp4 datagram");
    // The reply comes back from the resolver to the client's source port.
    assert_eq!((src, src_port), (resolver, 53));
    assert_eq!((dst, dst_port), (client, 5300));

    let response = Message::from_vec(&payload).unwrap();
    assert_eq!(response.id(), 0x1234);
    let answers = response.answers();
    assert_eq!(answers.len(), 1, "exactly one A answer expected");
    let Some(RData::A(addr)) = answers[0].data() else {
        panic!("expected an A record");
    };
    // First usable address of the default 198.18.0.0/15 fake-IP pool.
    assert_eq!(Ipv4Addr::from(addr.0), Ipv4Addr::new(198, 18, 0, 1));

    shutdown.notify_waiters();
    kernel.abort();
}

/// A non-DNS UDP datagram is relayed through the `Direct` outbound: it leaves
/// over a real OS UDP socket to a tokio echo server, and the reply is rewritten
/// back into an IP frame addressed to the original client endpoint.
#[tokio::test]
async fn tun_relays_udp_datagram_through_direct_outbound() {
    // UDP echo server reached via the kernel's Direct outbound.
    let echo = UdpSocket::bind("127.0.0.1:0").await.unwrap();
    let echo_addr = echo.local_addr().unwrap();
    let SocketAddr::V4(echo_v4) = echo_addr else {
        panic!("expected an IPv4 echo address");
    };
    tokio::spawn(async move {
        let mut buf = vec![0u8; 2048];
        loop {
            let Ok((n, from)) = echo.recv_from(&mut buf).await else {
                break;
            };
            if echo.send_to(&buf[..n], from).await.is_err() {
                break;
            }
        }
    });

    let (to_kernel_tx, to_kernel_rx) = mpsc::channel::<Vec<u8>>(16);
    let (from_kernel_tx, mut from_kernel_rx) = mpsc::channel::<Vec<u8>>(16);
    let shutdown = Arc::new(Notify::new());
    let kernel = tokio::spawn(serve_tun(
        to_kernel_rx,
        from_kernel_tx,
        OutboundMode::Direct,
        None,
        shutdown.clone(),
        DEFAULT_MTU,
    ));

    let client = Ipv4Address::new(10, 0, 0, 2);
    let o = echo_v4.ip().octets();
    let server = Ipv4Address::new(o[0], o[1], o[2], o[3]);
    let payload = b"hello udp over tun";
    let frame = build_udp4_frame(client, 41000, server, echo_v4.port(), payload);
    to_kernel_tx.send(frame).await.unwrap();

    let reply_frame = tokio::time::timeout(Duration::from_secs(5), from_kernel_rx.recv())
        .await
        .expect("udp reply timed out")
        .expect("kernel closed without replying");

    let (src, src_port, dst, dst_port, body) = parse_udp4_frame(&reply_frame).expect("reply is a udp4 datagram");
    // The reply comes back from the server endpoint to the client's source port.
    assert_eq!((src, src_port), (server, echo_v4.port()));
    assert_eq!((dst, dst_port), (client, 41000));
    assert_eq!(body, payload);

    shutdown.notify_waiters();
    kernel.abort();
}

/// Wrap `payload` in an IPv4 + UDP datagram (smoltcp computes the checksums).
fn build_udp4_frame(src: Ipv4Address, src_port: u16, dst: Ipv4Address, dst_port: u16, payload: &[u8]) -> Vec<u8> {
    let caps = ChecksumCapabilities::default();
    let udp_repr = UdpRepr { src_port, dst_port };
    let ip_repr = Ipv4Repr {
        src_addr: src,
        dst_addr: dst,
        next_header: IpProtocol::Udp,
        payload_len: udp_repr.header_len() + payload.len(),
        hop_limit: 64,
    };
    let mut frame = vec![0u8; ip_repr.buffer_len() + ip_repr.payload_len];
    let mut packet = Ipv4Packet::new_unchecked(&mut frame);
    ip_repr.emit(&mut packet, &caps);
    let mut udp = UdpPacket::new_unchecked(packet.payload_mut());
    udp_repr.emit(
        &mut udp,
        &IpAddress::Ipv4(src),
        &IpAddress::Ipv4(dst),
        payload.len(),
        |buf| buf.copy_from_slice(payload),
        &caps,
    );
    frame
}

/// Inverse of [`build_udp4_frame`]: pull the endpoints and payload back out.
fn parse_udp4_frame(frame: &[u8]) -> Option<(Ipv4Address, u16, Ipv4Address, u16, Vec<u8>)> {
    let ip = Ipv4Packet::new_checked(frame).ok()?;
    if ip.next_header() != IpProtocol::Udp {
        return None;
    }
    let udp = UdpPacket::new_checked(ip.payload()).ok()?;
    Some((
        ip.src_addr(),
        udp.src_port(),
        ip.dst_addr(),
        udp.dst_port(),
        udp.payload().to_vec(),
    ))
}
