//! TUN inbound: terminate L3 IP packets in a userspace TCP/IP stack and relay
//! each TCP flow through the normal [`OutboundMode`] pipeline.
//!
//! This module is **device-agnostic**: it consumes and produces raw IP frames
//! over two channels (`frames_in` from the OS TUN device, `frames_out` back to
//! it). Binding an actual OS TUN device (the `tun` crate, elevated privileges,
//! leak-safe apply/rollback) is a thin adapter that pumps the device into these
//! channels and lives outside the kernel crate.
//!
//! smoltcp is adopted purely as the IP/TCP stack primitive (packet wire codec +
//! per-flow TCP state machine), analogous to adopting rustls for TLS. The
//! orchestration — reading frames, demultiplexing flows, bridging each flow to
//! the outbound pipeline, and the back-pressure/close handling — is ours.
//!
//! Scope: IPv4/IPv6 **TCP**, plus full **UDP over TUN**. UDP datagrams to port
//! 53 are answered in-stack through the kernel's DNS logic (fake-IP allocation
//! or upstream forwarding) when a [`DnsMode`] is configured; every other UDP
//! datagram is relayed through the normal [`OutboundMode`] pipeline via a NAT
//! session table keyed by the UDP 5-tuple, with replies rewritten back as IP
//! frames. Answering DNS in-stack is what lets a global default route capture
//! all traffic without black-holing name resolution.

use crate::address::TargetAddr;
use crate::config::OutboundMode;
use crate::dns::{DnsMode, FakeIpPool, answer_query, unmap_fake_ip};
use crate::outbound::{self, UdpEgress};
use crate::protocols::shadowsocks::{ShadowsocksOutboundConfig, ShadowsocksUdp};
use crate::udp;

use anyhow::Result;
use std::collections::{HashMap, VecDeque};
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};
use std::sync::{Arc, Mutex};
use std::time::{Duration as StdDuration, Instant as StdInstant};

use smoltcp::iface::{Config as IfaceConfig, Interface, SocketHandle, SocketSet};
use smoltcp::phy::ChecksumCapabilities;
use smoltcp::phy::{Device, DeviceCapabilities, Medium, RxToken, TxToken};
use smoltcp::socket::tcp;
use smoltcp::time::Instant as SmolInstant;
use smoltcp::wire::{
    HardwareAddress, IpAddress, IpCidr, IpEndpoint, IpProtocol, Ipv4Address, Ipv4Packet, Ipv4Repr, Ipv6Address,
    Ipv6Packet, Ipv6Repr, TcpPacket, UdpPacket, UdpRepr,
};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use tokio::sync::{Notify, mpsc};

/// Default MTU for the userspace stack. The OS TUN device should be configured
/// with the same value.
pub const DEFAULT_MTU: usize = 1500;

/// Per-flow socket buffer size (each direction).
const FLOW_BUFFER: usize = 64 * 1024;
/// Bounded depth of the per-flow bridge channels (in frames/chunks).
const CHANNEL_DEPTH: usize = 64;
/// Upper bound on how long the poll loop sleeps between wakeups.
const MAX_POLL_SLEEP: StdDuration = StdDuration::from_millis(50);

type FakeIp = Option<Arc<Mutex<FakeIpPool>>>;

/// Run the TUN inbound until `shutdown` is notified or `frames_in` closes.
///
/// * `frames_in` — IP frames read from the TUN device.
/// * `frames_out` — IP frames the stack wants written back to the device. The
///   caller must drain this promptly; a full queue drops frames (TCP recovers).
pub async fn serve_tun(
    mut frames_in: mpsc::Receiver<Vec<u8>>,
    frames_out: mpsc::Sender<Vec<u8>>,
    mode: OutboundMode,
    dns: Option<DnsMode>,
    shutdown: Arc<Notify>,
    mtu: usize,
) {
    let mode = Arc::new(mode);
    let dns = dns.map(Arc::new);
    // The fake-IP pool used to unmap TCP destinations is the *same* pool the DNS
    // server allocates from, so a connection to a synthesized IP routes by the
    // domain that DNS just handed out.
    let fake_ip: FakeIp = dns.as_ref().and_then(|mode| match mode.as_ref() {
        DnsMode::FakeIp { pool, .. } => Some(pool.clone()),
        DnsMode::Forward { .. } => None,
    });
    let start = StdInstant::now();
    let mut phy = TunPhy::new(mtu);
    let mut iface = build_interface(&mut phy, smol_now(start));
    let mut sockets = SocketSet::new(Vec::new());
    let mut flows: HashMap<FlowKey, Flow> = HashMap::new();
    // NAT sessions for relayed (non-DNS) UDP, keyed by the datagram 5-tuple.
    let mut udp_nat: HashMap<FlowKey, mpsc::Sender<Vec<u8>>> = HashMap::new();
    let wake = Arc::new(Notify::new());

    loop {
        iface.poll(smol_now(start), &mut phy, &mut sockets);
        service_flows(&mut sockets, &mut flows);
        drain_tx(&mut phy, &frames_out);

        let delay = iface
            .poll_delay(smol_now(start), &sockets)
            .map(|d| StdDuration::from_micros(d.total_micros()))
            .map_or(MAX_POLL_SLEEP, |d| d.min(MAX_POLL_SLEEP));

        tokio::select! {
            _ = shutdown.notified() => return,
            _ = wake.notified() => {}
            _ = tokio::time::sleep(delay) => {}
            frame = frames_in.recv() => {
                match frame {
                    Some(frame) => {
                        // UDP datagrams are terminated here (answered in-stack
                        // for DNS, otherwise relayed via NAT) and never reach
                        // the TCP stack; everything else is fed to smoltcp.
                        if handle_udp(&frame, &dns, &mode, &fake_ip, &frames_out, &mut udp_nat) {
                            continue;
                        }
                        new_flow_for_syn(&frame, &mut sockets, &mut flows, &mode, &fake_ip, &wake);
                        phy.rx.push_back(frame);
                    }
                    None => return,
                }
            }
        }
    }
}

/// Number of frames buffered between the device pump and the userspace stack.
const DEVICE_QUEUE_DEPTH: usize = 256;

/// Run the TUN inbound against a byte-stream device that delivers and accepts
/// **one IP packet per read/write** — the contract the `tun` crate's async
/// device exposes. This is the thin adapter an OS TUN binding calls: it spawns
/// [`serve_tun`] over internal channels and pumps frames between the device and
/// those channels in both directions, terminating when `shutdown` fires or the
/// device hits EOF/error.
///
/// The device must be configured **without** a packet-information header (Linux
/// `IFF_NO_PI`, Windows wintun has none); platforms whose header cannot be
/// disabled (e.g. macOS utun's 4-byte prefix) need a codec layered on top and
/// are not handled here.
pub async fn serve_tun_device<R, W>(
    mut reader: R,
    mut writer: W,
    mode: OutboundMode,
    dns: Option<DnsMode>,
    shutdown: Arc<Notify>,
    mtu: usize,
) where
    R: AsyncRead + Unpin + Send + 'static,
    W: AsyncWrite + Unpin + Send + 'static,
{
    let (to_kernel_tx, to_kernel_rx) = mpsc::channel::<Vec<u8>>(DEVICE_QUEUE_DEPTH);
    let (to_device_tx, mut to_device_rx) = mpsc::channel::<Vec<u8>>(DEVICE_QUEUE_DEPTH);

    let stack = tokio::spawn(serve_tun(to_kernel_rx, to_device_tx, mode, dns, shutdown.clone(), mtu));

    // Device -> stack: each read yields a single IP packet.
    let read_shutdown = shutdown.clone();
    let reader_task = tokio::spawn(async move {
        let mut buf = vec![0u8; mtu.max(DEFAULT_MTU)];
        loop {
            tokio::select! {
                _ = read_shutdown.notified() => break,
                res = reader.read(&mut buf) => match res {
                    Ok(0) | Err(_) => break,
                    Ok(n) => {
                        if to_kernel_tx.send(buf[..n].to_vec()).await.is_err() {
                            break;
                        }
                    }
                },
            }
        }
    });

    // Stack -> device: write each frame as a single packet.
    let write_shutdown = shutdown.clone();
    let writer_task = tokio::spawn(async move {
        loop {
            tokio::select! {
                _ = write_shutdown.notified() => break,
                frame = to_device_rx.recv() => match frame {
                    Some(frame) => {
                        if writer.write_all(&frame).await.is_err() {
                            break;
                        }
                    }
                    None => break,
                },
            }
        }
    });

    let _ = reader_task.await;
    shutdown.notify_waiters();
    let _ = writer_task.await;
    stack.abort();
}

type FlowKey = (IpEndpoint, IpEndpoint);

/// Bridge state for one accepted TCP flow, owned by the poll loop.
struct Flow {
    handle: SocketHandle,
    /// Client -> outbound. Dropped (set to `None`) when the client half-closes,
    /// which closes the outbound write side.
    tx_to_out: Option<mpsc::Sender<Vec<u8>>>,
    /// Outbound -> client.
    rx_from_out: mpsc::Receiver<Vec<u8>>,
    pending: Vec<u8>,
    pending_off: usize,
    out_done: bool,
    established: bool,
    closing: bool,
}

/// Move data between each flow's smoltcp socket and its outbound bridge,
/// honoring back-pressure, and reap fully-closed flows.
fn service_flows(sockets: &mut SocketSet, flows: &mut HashMap<FlowKey, Flow>) {
    let mut done: Vec<FlowKey> = Vec::new();

    for (key, flow) in flows.iter_mut() {
        let sock = sockets.get_mut::<tcp::Socket>(flow.handle);
        if sock.state() == tcp::State::Established {
            flow.established = true;
        }

        // client -> outbound (only consume what the bridge can accept).
        if let Some(tx) = &flow.tx_to_out {
            while sock.can_recv() {
                match tx.try_reserve() {
                    Ok(permit) => {
                        let data = sock.recv(|buf| (buf.len(), buf.to_vec())).unwrap_or_default();
                        if data.is_empty() {
                            break;
                        }
                        permit.send(data);
                    }
                    Err(_) => break,
                }
            }
        }

        // Client half-closed -> close the outbound write side.
        if flow.established && !sock.may_recv() {
            flow.tx_to_out = None;
        }

        // outbound -> client.
        while sock.can_send() {
            if flow.pending_off >= flow.pending.len() {
                flow.pending.clear();
                flow.pending_off = 0;
                match flow.rx_from_out.try_recv() {
                    Ok(buf) => flow.pending = buf,
                    Err(mpsc::error::TryRecvError::Empty) => break,
                    Err(mpsc::error::TryRecvError::Disconnected) => {
                        flow.out_done = true;
                        break;
                    }
                }
            }
            match sock.send_slice(&flow.pending[flow.pending_off..]) {
                Ok(0) => break,
                Ok(n) => flow.pending_off += n,
                Err(_) => break,
            }
        }

        // Outbound finished and everything flushed -> FIN to the client.
        if flow.out_done && flow.pending_off >= flow.pending.len() && !flow.closing {
            sock.close();
            flow.closing = true;
        }

        if sock.state() == tcp::State::Closed {
            done.push(*key);
        }
    }

    for key in done {
        if let Some(flow) = flows.remove(&key) {
            sockets.remove(flow.handle);
        }
    }
}

/// Drain frames the stack produced back to the TUN device. Uses non-blocking
/// sends so a stalled consumer cannot wedge the poll loop; a dropped frame is
/// recovered by TCP retransmission.
fn drain_tx(phy: &mut TunPhy, frames_out: &mpsc::Sender<Vec<u8>>) {
    while let Some(frame) = phy.tx.pop_front() {
        if frames_out.try_send(frame).is_err() {
            break;
        }
    }
}

/// If `frame` is a TCP SYN for an unseen flow, create a listening socket on its
/// destination and spawn the outbound bridge task.
fn new_flow_for_syn(
    frame: &[u8],
    sockets: &mut SocketSet,
    flows: &mut HashMap<FlowKey, Flow>,
    mode: &Arc<OutboundMode>,
    fake_ip: &FakeIp,
    wake: &Arc<Notify>,
) {
    let Some((src, dst, is_syn)) = parse_tcp_endpoints(frame) else {
        return;
    };
    if !is_syn || flows.contains_key(&(src, dst)) {
        return;
    }

    let mut sock = tcp::Socket::new(
        tcp::SocketBuffer::new(vec![0u8; FLOW_BUFFER]),
        tcp::SocketBuffer::new(vec![0u8; FLOW_BUFFER]),
    );
    if sock.listen(dst).is_err() {
        return;
    }
    let handle = sockets.add(sock);

    let mut target = TargetAddr::Ip(endpoint_socketaddr(dst));
    if let Some(pool) = fake_ip {
        target = unmap_fake_ip(pool, target);
    }

    let source = endpoint_socketaddr(src);
    let (to_out_tx, to_out_rx) = mpsc::channel::<Vec<u8>>(CHANNEL_DEPTH);
    let (from_out_tx, from_out_rx) = mpsc::channel::<Vec<u8>>(CHANNEL_DEPTH);
    tokio::spawn(run_flow(
        target,
        source,
        mode.clone(),
        to_out_rx,
        from_out_tx,
        wake.clone(),
    ));

    flows.insert(
        (src, dst),
        Flow {
            handle,
            tx_to_out: Some(to_out_tx),
            rx_from_out: from_out_rx,
            pending: Vec::new(),
            pending_off: 0,
            out_done: false,
            established: false,
            closing: false,
        },
    );
}

/// Dial the outbound and pump bytes between it and the flow's bridge channels.
async fn run_flow(
    target: TargetAddr,
    source: SocketAddr,
    mode: Arc<OutboundMode>,
    mut to_out_rx: mpsc::Receiver<Vec<u8>>,
    from_out_tx: mpsc::Sender<Vec<u8>>,
    wake: Arc<Notify>,
) {
    let stream = match outbound::connect(mode.as_ref(), &target, Some(source)).await {
        Ok(stream) => stream,
        Err(err) => {
            log::debug!("learn-gripe tun: outbound to {target} failed: {err:#}");
            return;
        }
    };
    let (mut reader, mut writer) = tokio::io::split(stream);

    let to_outbound = async {
        while let Some(buf) = to_out_rx.recv().await {
            if writer.write_all(&buf).await.is_err() {
                break;
            }
            wake.notify_one();
        }
        let _ = writer.shutdown().await;
    };

    let from_outbound = async {
        let mut buf = vec![0u8; 16 * 1024];
        loop {
            match reader.read(&mut buf).await {
                Ok(0) | Err(_) => break,
                Ok(n) => {
                    if from_out_tx.send(buf[..n].to_vec()).await.is_err() {
                        break;
                    }
                    wake.notify_one();
                }
            }
        }
        // Dropping `from_out_tx` here signals EOF to the poll loop.
    };

    tokio::join!(to_outbound, from_outbound);
    wake.notify_one();
}

/// Parse just enough of an IP frame to extract the TCP 5-tuple and SYN flag.
fn parse_tcp_endpoints(frame: &[u8]) -> Option<(IpEndpoint, IpEndpoint, bool)> {
    match frame.first().map(|b| b >> 4) {
        Some(4) => {
            let ip = Ipv4Packet::new_checked(frame).ok()?;
            if ip.next_header() != IpProtocol::Tcp {
                return None;
            }
            let tcp = TcpPacket::new_checked(ip.payload()).ok()?;
            let src = IpEndpoint::new(IpAddress::Ipv4(ip.src_addr()), tcp.src_port());
            let dst = IpEndpoint::new(IpAddress::Ipv4(ip.dst_addr()), tcp.dst_port());
            Some((src, dst, tcp.syn() && !tcp.ack()))
        }
        Some(6) => {
            let ip = Ipv6Packet::new_checked(frame).ok()?;
            if ip.next_header() != IpProtocol::Tcp {
                return None;
            }
            let tcp = TcpPacket::new_checked(ip.payload()).ok()?;
            let src = IpEndpoint::new(IpAddress::Ipv6(ip.src_addr()), tcp.src_port());
            let dst = IpEndpoint::new(IpAddress::Ipv6(ip.dst_addr()), tcp.dst_port());
            Some((src, dst, tcp.syn() && !tcp.ack()))
        }
        _ => None,
    }
}

fn endpoint_socketaddr(endpoint: IpEndpoint) -> SocketAddr {
    let ip = match endpoint.addr {
        IpAddress::Ipv4(addr) => IpAddr::V4(Ipv4Addr::from(addr.octets())),
        IpAddress::Ipv6(addr) => IpAddr::V6(Ipv6Addr::from(addr.octets())),
    };
    SocketAddr::new(ip, endpoint.port)
}

/// UDP port DNS queries are sent to.
const DNS_PORT: u16 = 53;
/// Bounded depth of a NAT session's payload queue. Beyond this the datagram is
/// dropped (UDP is lossy) rather than stalling the poll loop.
const UDP_SESSION_QUEUE: usize = 128;
/// A relayed UDP session with no traffic in either direction for this long is
/// torn down (UDP has no close, so sessions must be reaped on idle).
const UDP_IDLE_TIMEOUT: StdDuration = StdDuration::from_secs(60);

/// The L3/L4 endpoints of a UDP datagram. Retained so a reply can be built by
/// swapping source and destination.
#[derive(Clone, Copy)]
struct UdpFlow {
    src_addr: IpAddress,
    dst_addr: IpAddress,
    src_port: u16,
    dst_port: u16,
}

impl UdpFlow {
    fn src(&self) -> IpEndpoint {
        IpEndpoint::new(self.src_addr, self.src_port)
    }

    fn dst(&self) -> IpEndpoint {
        IpEndpoint::new(self.dst_addr, self.dst_port)
    }
}

/// A parsed UDP datagram with its endpoints and payload.
struct UdpDatagram {
    flow: UdpFlow,
    payload: Vec<u8>,
}

/// Terminate a UDP datagram in-stack: DNS queries (port [`DNS_PORT`]) with a
/// configured [`DnsMode`] are answered from the kernel DNS logic; every other
/// UDP datagram is relayed through the outbound pipeline via a NAT session.
/// Returns `true` when `frame` was UDP (consumed here, never fed to the TCP
/// stack); `false` for non-UDP frames, which the caller handles normally.
fn handle_udp(
    frame: &[u8],
    dns: &Option<Arc<DnsMode>>,
    mode: &Arc<OutboundMode>,
    fake_ip: &FakeIp,
    frames_out: &mpsc::Sender<Vec<u8>>,
    nat: &mut HashMap<FlowKey, mpsc::Sender<Vec<u8>>>,
) -> bool {
    let Some(datagram) = parse_udp_datagram(frame) else {
        return false;
    };

    // DNS to :53 is answered in-stack when a DNS mode is configured; without
    // one it falls through and is relayed to whatever resolver the client used.
    if datagram.flow.dst_port == DNS_PORT
        && let Some(dns) = dns
    {
        answer_dns(datagram, dns, frames_out);
        return true;
    }

    relay_udp(datagram, mode, fake_ip, frames_out, nat);
    true
}

/// Answer a DNS datagram in the background via the kernel DNS logic and emit the
/// reply frame back to the device.
fn answer_dns(datagram: UdpDatagram, dns: &Arc<DnsMode>, frames_out: &mpsc::Sender<Vec<u8>>) {
    let dns = dns.clone();
    let frames_out = frames_out.clone();
    tokio::spawn(async move {
        match answer_query(&datagram.payload, &dns).await {
            Ok(response) => {
                if let Some(frame) = build_udp_reply_frame(&datagram.flow, &response) {
                    let _ = frames_out.send(frame).await;
                }
            }
            Err(err) => log::debug!("learn-gripe tun dns: dropped query: {err:#}"),
        }
    });
}

/// Relay a non-DNS UDP datagram through the outbound pipeline, keeping one NAT
/// session per 5-tuple so replies can be steered back to the right client.
fn relay_udp(
    datagram: UdpDatagram,
    mode: &Arc<OutboundMode>,
    fake_ip: &FakeIp,
    frames_out: &mpsc::Sender<Vec<u8>>,
    nat: &mut HashMap<FlowKey, mpsc::Sender<Vec<u8>>>,
) {
    let flow = datagram.flow;
    let key = (flow.src(), flow.dst());
    let mut payload = datagram.payload;

    if let Some(tx) = nat.get(&key) {
        match tx.try_send(payload) {
            Ok(()) => return,
            // Session busy: drop this datagram (UDP is lossy) but keep the task.
            Err(mpsc::error::TrySendError::Full(_)) => return,
            // Task has exited (idle/error); rebuild it below with the payload.
            Err(mpsc::error::TrySendError::Closed(p)) => {
                nat.remove(&key);
                payload = p;
            }
        }
    }

    // Unmap a fake IP back to its domain so routing sees the real host; the
    // reply frame still sources from the address the client targeted.
    let mut target = TargetAddr::Ip(endpoint_socketaddr(flow.dst()));
    if let Some(pool) = fake_ip {
        target = unmap_fake_ip(pool, target);
    }

    // No UDP egress (Reject, an upstream SOCKS5 proxy): drop, don't leak.
    let source = endpoint_socketaddr(flow.src());
    let Some(egress) = outbound::resolve_udp_egress(mode, &target, Some(source)) else {
        return;
    };

    let (tx, rx) = mpsc::channel(UDP_SESSION_QUEUE);
    tokio::spawn(run_udp_session(egress, target, flow, rx, frames_out.clone()));
    // The fresh channel has capacity; only fails if the task already died.
    let _ = tx.try_send(payload);
    nat.insert(key, tx);
}

/// Drive one UDP NAT session until it goes idle, errors, or its sender drops.
async fn run_udp_session(
    egress: UdpEgress,
    target: TargetAddr,
    reply: UdpFlow,
    rx: mpsc::Receiver<Vec<u8>>,
    frames_out: mpsc::Sender<Vec<u8>>,
) {
    let result = match egress {
        UdpEgress::Direct => run_udp_direct(target, reply, rx, frames_out).await,
        UdpEgress::Shadowsocks(config) => run_udp_ss(config, target, reply, rx, frames_out).await,
        proxy => run_udp_proxy(proxy, target, reply, rx, frames_out).await,
    };
    if let Err(err) = result {
        log::debug!("learn-gripe tun udp: session ended: {err:#}");
    }
}

/// Direct UDP egress over a plain OS socket: forward client payloads to the
/// destination and rewrite each reply back into an IP frame for the client.
async fn run_udp_direct(
    target: TargetAddr,
    reply: UdpFlow,
    mut rx: mpsc::Receiver<Vec<u8>>,
    frames_out: mpsc::Sender<Vec<u8>>,
) -> Result<()> {
    let dest = udp::resolve(&target).await?;
    let socket = udp::bind_egress(dest).await?;
    socket.connect(dest).await?;

    let mut buf = vec![0u8; udp::MAX_DATAGRAM];
    loop {
        tokio::select! {
            maybe = rx.recv() => match maybe {
                Some(payload) => {
                    socket.send(&payload).await?;
                }
                None => return Ok(()),
            },
            res = socket.recv(&mut buf) => {
                let n = res?;
                if let Some(frame) = build_udp_reply_frame(&reply, &buf[..n])
                    && frames_out.send(frame).await.is_err()
                {
                    return Ok(());
                }
            }
            _ = tokio::time::sleep(UDP_IDLE_TIMEOUT) => return Ok(()),
        }
    }
}

/// Shadowsocks UDP egress: relay datagrams over a UDP socket to the Shadowsocks
/// server (per-packet AEAD framing), rewriting replies back into IP frames.
async fn run_udp_ss(
    config: Box<ShadowsocksOutboundConfig>,
    target: TargetAddr,
    reply: UdpFlow,
    mut rx: mpsc::Receiver<Vec<u8>>,
    frames_out: mpsc::Sender<Vec<u8>>,
) -> Result<()> {
    let assoc = ShadowsocksUdp::connect(&config, &target).await?;
    loop {
        tokio::select! {
            maybe = rx.recv() => match maybe {
                Some(payload) => assoc.send(&payload).await?,
                None => return Ok(()),
            },
            res = assoc.recv() => {
                let payload = res?;
                if let Some(frame) = build_udp_reply_frame(&reply, &payload)
                    && frames_out.send(frame).await.is_err()
                {
                    return Ok(());
                }
            }
            _ = tokio::time::sleep(UDP_IDLE_TIMEOUT) => return Ok(()),
        }
    }
}

/// Proxy-tunnel UDP egress: relay datagrams over the protocol's UDP stream,
/// applying its per-packet framing, and rewrite replies back into IP frames.
async fn run_udp_proxy(
    egress: UdpEgress,
    target: TargetAddr,
    reply: UdpFlow,
    mut rx: mpsc::Receiver<Vec<u8>>,
    frames_out: mpsc::Sender<Vec<u8>>,
) -> Result<()> {
    let framing = udp::ProxyFraming::for_egress(&egress);
    let stream = outbound::connect_proxy_udp(&egress, &target).await?;
    let (mut reader, mut writer) = tokio::io::split(stream);

    loop {
        tokio::select! {
            maybe = rx.recv() => match maybe {
                Some(payload) => udp::write_proxy_packet(&mut writer, framing, &target, &payload).await?,
                None => return Ok(()),
            },
            res = udp::read_proxy_packet(&mut reader, framing) => {
                let payload = res?;
                if let Some(frame) = build_udp_reply_frame(&reply, &payload)
                    && frames_out.send(frame).await.is_err()
                {
                    return Ok(());
                }
            }
            _ = tokio::time::sleep(UDP_IDLE_TIMEOUT) => return Ok(()),
        }
    }
}

/// Parse an IP frame as a UDP datagram, extracting the endpoints and payload.
/// Returns `None` for anything that is not a well-formed IPv4/IPv6 UDP packet.
fn parse_udp_datagram(frame: &[u8]) -> Option<UdpDatagram> {
    match frame.first().map(|b| b >> 4) {
        Some(4) => {
            let ip = Ipv4Packet::new_checked(frame).ok()?;
            if ip.next_header() != IpProtocol::Udp {
                return None;
            }
            let udp = UdpPacket::new_checked(ip.payload()).ok()?;
            Some(UdpDatagram {
                flow: UdpFlow {
                    src_addr: IpAddress::Ipv4(ip.src_addr()),
                    dst_addr: IpAddress::Ipv4(ip.dst_addr()),
                    src_port: udp.src_port(),
                    dst_port: udp.dst_port(),
                },
                payload: udp.payload().to_vec(),
            })
        }
        Some(6) => {
            let ip = Ipv6Packet::new_checked(frame).ok()?;
            if ip.next_header() != IpProtocol::Udp {
                return None;
            }
            let udp = UdpPacket::new_checked(ip.payload()).ok()?;
            Some(UdpDatagram {
                flow: UdpFlow {
                    src_addr: IpAddress::Ipv6(ip.src_addr()),
                    dst_addr: IpAddress::Ipv6(ip.dst_addr()),
                    src_port: udp.src_port(),
                    dst_port: udp.dst_port(),
                },
                payload: udp.payload().to_vec(),
            })
        }
        _ => None,
    }
}

/// Build the IP+UDP reply frame for `flow`, carrying `payload`. The reply swaps
/// source/destination (so it appears to come from the host the client targeted)
/// and lets smoltcp compute the checksums.
fn build_udp_reply_frame(flow: &UdpFlow, payload: &[u8]) -> Option<Vec<u8>> {
    let udp_repr = UdpRepr {
        src_port: flow.dst_port,
        dst_port: flow.src_port,
    };
    let caps = ChecksumCapabilities::default();

    match (flow.dst_addr, flow.src_addr) {
        // Reply source = original destination, reply destination = original source.
        (IpAddress::Ipv4(reply_src), IpAddress::Ipv4(reply_dst)) => {
            let ip_repr = Ipv4Repr {
                src_addr: reply_src,
                dst_addr: reply_dst,
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
                &IpAddress::Ipv4(reply_src),
                &IpAddress::Ipv4(reply_dst),
                payload.len(),
                |buf| buf.copy_from_slice(payload),
                &caps,
            );
            Some(frame)
        }
        (IpAddress::Ipv6(reply_src), IpAddress::Ipv6(reply_dst)) => {
            let ip_repr = Ipv6Repr {
                src_addr: reply_src,
                dst_addr: reply_dst,
                next_header: IpProtocol::Udp,
                payload_len: udp_repr.header_len() + payload.len(),
                hop_limit: 64,
            };
            let mut frame = vec![0u8; ip_repr.buffer_len() + ip_repr.payload_len];
            let mut packet = Ipv6Packet::new_unchecked(&mut frame);
            ip_repr.emit(&mut packet);
            let mut udp = UdpPacket::new_unchecked(packet.payload_mut());
            udp_repr.emit(
                &mut udp,
                &IpAddress::Ipv6(reply_src),
                &IpAddress::Ipv6(reply_dst),
                payload.len(),
                |buf| buf.copy_from_slice(payload),
                &caps,
            );
            Some(frame)
        }
        // Mixed address families cannot occur within a single IP packet.
        _ => None,
    }
}

/// Build the interface in transparent mode: `any_ip` lets it accept packets
/// destined to addresses it does not own, and the catch-all assigned addresses
/// plus default routes let it source replies from the destination the client
/// actually targeted.
fn build_interface(phy: &mut TunPhy, now: SmolInstant) -> Interface {
    let config = IfaceConfig::new(HardwareAddress::Ip);
    let mut iface = Interface::new(config, phy, now);
    iface.set_any_ip(true);
    iface.update_ip_addrs(|addrs| {
        let _ = addrs.push(IpCidr::new(IpAddress::Ipv4(Ipv4Address::new(0, 0, 0, 1)), 0));
        let _ = addrs.push(IpCidr::new(
            IpAddress::Ipv6(Ipv6Address::new(0, 0, 0, 0, 0, 0, 0, 1)),
            0,
        ));
    });
    let _ = iface.routes_mut().add_default_ipv4_route(Ipv4Address::new(0, 0, 0, 1));
    let _ = iface
        .routes_mut()
        .add_default_ipv6_route(Ipv6Address::new(0, 0, 0, 0, 0, 0, 0, 1));
    iface
}

fn smol_now(start: StdInstant) -> SmolInstant {
    SmolInstant::from_micros(start.elapsed().as_micros() as i64)
}

/// In-memory smoltcp [`Device`] backed by two frame queues the poll loop fills
/// and drains.
struct TunPhy {
    rx: VecDeque<Vec<u8>>,
    tx: VecDeque<Vec<u8>>,
    mtu: usize,
}

impl TunPhy {
    fn new(mtu: usize) -> Self {
        Self {
            rx: VecDeque::new(),
            tx: VecDeque::new(),
            mtu,
        }
    }
}

struct PhyRxToken {
    buf: Vec<u8>,
}

struct PhyTxToken<'a> {
    tx: &'a mut VecDeque<Vec<u8>>,
}

impl Device for TunPhy {
    type RxToken<'a> = PhyRxToken;
    type TxToken<'a> = PhyTxToken<'a>;

    fn receive(&mut self, _timestamp: SmolInstant) -> Option<(Self::RxToken<'_>, Self::TxToken<'_>)> {
        let buf = self.rx.pop_front()?;
        Some((PhyRxToken { buf }, PhyTxToken { tx: &mut self.tx }))
    }

    fn transmit(&mut self, _timestamp: SmolInstant) -> Option<Self::TxToken<'_>> {
        Some(PhyTxToken { tx: &mut self.tx })
    }

    fn capabilities(&self) -> DeviceCapabilities {
        let mut caps = DeviceCapabilities::default();
        caps.medium = Medium::Ip;
        caps.max_transmission_unit = self.mtu;
        caps
    }
}

impl RxToken for PhyRxToken {
    fn consume<R, F: FnOnce(&[u8]) -> R>(self, f: F) -> R {
        f(&self.buf)
    }
}

impl TxToken for PhyTxToken<'_> {
    fn consume<R, F: FnOnce(&mut [u8]) -> R>(self, len: usize, f: F) -> R {
        let mut buf = vec![0u8; len];
        let result = f(&mut buf);
        self.tx.push_back(buf);
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_ipv4_tcp_syn_endpoints() {
        // Build a minimal IPv4 + TCP SYN with smoltcp's own wire writers.
        use smoltcp::wire::{Ipv4Repr, TcpControl, TcpRepr};

        let src = IpEndpoint::new(IpAddress::Ipv4(Ipv4Address::new(10, 0, 0, 1)), 40000);
        let dst = IpEndpoint::new(IpAddress::Ipv4(Ipv4Address::new(93, 184, 216, 34)), 443);

        let tcp_repr = TcpRepr {
            src_port: src.port,
            dst_port: dst.port,
            control: TcpControl::Syn,
            seq_number: smoltcp::wire::TcpSeqNumber(0),
            ack_number: None,
            window_len: 64240,
            window_scale: None,
            max_seg_size: None,
            sack_permitted: false,
            sack_ranges: [None, None, None],
            timestamp: None,
            payload: &[],
        };
        let ipv4_repr = Ipv4Repr {
            src_addr: Ipv4Address::new(10, 0, 0, 1),
            dst_addr: Ipv4Address::new(93, 184, 216, 34),
            next_header: IpProtocol::Tcp,
            payload_len: tcp_repr.buffer_len(),
            hop_limit: 64,
        };
        let mut frame = vec![0u8; ipv4_repr.buffer_len() + tcp_repr.buffer_len()];
        let mut ipv4_packet = Ipv4Packet::new_unchecked(&mut frame);
        ipv4_repr.emit(&mut ipv4_packet, &smoltcp::phy::ChecksumCapabilities::default());
        let mut tcp_packet = TcpPacket::new_unchecked(ipv4_packet.payload_mut());
        tcp_repr.emit(
            &mut tcp_packet,
            &IpAddress::Ipv4(ipv4_repr.src_addr),
            &IpAddress::Ipv4(ipv4_repr.dst_addr),
            &smoltcp::phy::ChecksumCapabilities::default(),
        );

        let (parsed_src, parsed_dst, is_syn) = parse_tcp_endpoints(&frame).expect("parse syn");
        assert_eq!(parsed_src, src);
        assert_eq!(parsed_dst, dst);
        assert!(is_syn);
        assert_eq!(endpoint_socketaddr(dst), "93.184.216.34:443".parse().unwrap());
    }

    #[test]
    fn ignores_non_tcp_and_garbage() {
        assert!(parse_tcp_endpoints(&[]).is_none());
        assert!(parse_tcp_endpoints(&[0x45]).is_none());
        // IPv4 header with UDP protocol -> ignored.
        let mut frame = vec![0u8; 28];
        frame[0] = 0x45;
        frame[9] = IpProtocol::Udp.into();
        assert!(parse_tcp_endpoints(&frame).is_none());
    }
}
