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
//! Scope of this first slice: IPv4/IPv6 **TCP**. UDP-over-TUN (and thus DNS over
//! TUN) is not handled here; non-TCP packets are ignored.

use crate::address::TargetAddr;
use crate::config::OutboundMode;
use crate::dns::{FakeIpPool, unmap_fake_ip};
use crate::outbound;

use std::collections::{HashMap, VecDeque};
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};
use std::sync::{Arc, Mutex};
use std::time::{Duration as StdDuration, Instant as StdInstant};

use smoltcp::iface::{Config as IfaceConfig, Interface, SocketHandle, SocketSet};
use smoltcp::phy::{Device, DeviceCapabilities, Medium, RxToken, TxToken};
use smoltcp::socket::tcp;
use smoltcp::time::Instant as SmolInstant;
use smoltcp::wire::{
    HardwareAddress, IpAddress, IpCidr, IpEndpoint, IpProtocol, Ipv4Address, Ipv4Packet, Ipv6Address, Ipv6Packet,
    TcpPacket,
};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
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
    fake_ip: FakeIp,
    shutdown: Arc<Notify>,
    mtu: usize,
) {
    let mode = Arc::new(mode);
    let start = StdInstant::now();
    let mut phy = TunPhy::new(mtu);
    let mut iface = build_interface(&mut phy, smol_now(start));
    let mut sockets = SocketSet::new(Vec::new());
    let mut flows: HashMap<FlowKey, Flow> = HashMap::new();
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
                        new_flow_for_syn(&frame, &mut sockets, &mut flows, &mode, &fake_ip, &wake);
                        phy.rx.push_back(frame);
                    }
                    None => return,
                }
            }
        }
    }
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

    let (to_out_tx, to_out_rx) = mpsc::channel::<Vec<u8>>(CHANNEL_DEPTH);
    let (from_out_tx, from_out_rx) = mpsc::channel::<Vec<u8>>(CHANNEL_DEPTH);
    tokio::spawn(run_flow(target, mode.clone(), to_out_rx, from_out_tx, wake.clone()));

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
    mode: Arc<OutboundMode>,
    mut to_out_rx: mpsc::Receiver<Vec<u8>>,
    from_out_tx: mpsc::Sender<Vec<u8>>,
    wake: Arc<Notify>,
) {
    let stream = match outbound::connect(mode.as_ref(), &target).await {
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
