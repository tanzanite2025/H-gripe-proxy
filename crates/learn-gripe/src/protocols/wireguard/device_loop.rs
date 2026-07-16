use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};
use std::sync::{Arc, Mutex};
use std::task::{Poll, Waker};
use std::time::{Duration, Instant};

use boringtun::noise::{Tunn, TunnResult};
use smoltcp::iface::{Interface, SocketHandle, SocketSet};
use smoltcp::socket::{tcp, udp};
use smoltcp::wire::IpEndpoint;
use tokio::io::ReadBuf;
use tokio::net::UdpSocket;
use tokio::sync::{Notify, mpsc, oneshot};

use super::amnezia::{Amnezia, clear_reserved, deobfuscate, send_obfuscated};
use super::device::Command;
use super::netstack::{WgPhy, build_interface, ip_address, is_dead, smol_now};
use super::stream::{WgTcpStream, WgUdpAssoc};
use super::{AllowedIp, WireGuardOutboundConfig};

/// Per-flow bridge channel depth (in chunks).
pub(super) const CHANNEL_DEPTH: usize = 64;
/// Per-flow smoltcp socket buffer size (each direction).
pub(super) const FLOW_BUFFER: usize = 64 * 1024;
/// Number of in-flight datagram slots per direction for a UDP flow's smoltcp
/// packet buffer (each datagram needs one metadata slot).
const UDP_META_SLOTS: usize = 64;
/// How long to wait for a relayed TCP connection to reach `Established` (covers
/// the WireGuard handshake plus the inner TCP handshake).
const CONNECT_TIMEOUT: Duration = Duration::from_secs(10);
/// Upper bound on how long the device poll loop sleeps between wakeups.
const MAX_POLL_SLEEP: Duration = Duration::from_millis(250);
/// Wall-clock cadence at which `Tunn::update_timers` is driven (rekey / keepalive
/// / handshake retransmit). boringtun expects this every ~100-250ms; we tick it
/// from the loop top rather than only the timeout arm so steady relay traffic
/// cannot starve it.
const TIMER_TICK: Duration = Duration::from_millis(120);

/// Wakers parked by streams whose write channel filled, woken once the loop has
/// drained their bytes into the smoltcp sockets.
pub(super) type WriterWakers = Arc<Mutex<Vec<Waker>>>;

/// One peer's tunnel state owned by the poll loop: its Noise session, dedicated
/// UDP endpoint, transport `reserved` tag, and the inner prefixes routed to it.
pub(super) struct PeerTunn {
    pub(super) tunn: Tunn,
    pub(super) udp: UdpSocket,
    pub(super) reserved: [u8; 3],
    pub(super) allowed_ips: Vec<AllowedIp>,
}

/// State owned by the per-device poll loop.
pub(super) struct DeviceLoop {
    /// One entry per configured peer; index 0 is the top-level peer.
    peers: Vec<PeerTunn>,
    mtu: usize,
    local_v4: Option<Ipv4Addr>,
    local_v6: Option<Ipv6Addr>,
    commands: mpsc::Receiver<Command>,
    flows: Vec<WgFlow>,
    udp_flows: Vec<WgUdpFlow>,
    next_port: u16,
    wake: Arc<Notify>,
    writer_wakers: WriterWakers,
    /// AmneziaWG obfuscation applied to every peer's UDP I/O when set.
    amnezia: Option<Amnezia>,
}

/// Bridge state for one relayed TCP flow, owned by the poll loop.
struct WgFlow {
    handle: SocketHandle,
    /// Caller -> socket bytes.
    write_rx: mpsc::Receiver<Vec<u8>>,
    /// Socket -> caller bytes; dropped to signal EOF to the caller.
    read_tx: Option<mpsc::Sender<Vec<u8>>>,
    /// Caller bytes not yet accepted by the socket send buffer.
    pending: Vec<u8>,
    pending_off: usize,
    /// We have closed the socket's write side (caller half-closed).
    write_closed: bool,
    /// Pending connect result; resolved once the socket reaches `Established`.
    connect_reply: Option<oneshot::Sender<WgTcpStream>>,
    /// The stream handed to the caller once connected.
    stream_slot: Option<WgTcpStream>,
    deadline: Instant,
}

/// Bridge state for one relayed UDP association, owned by the poll loop. Unlike
/// TCP there is no connection state: datagrams flow to a fixed `remote` and one
/// datagram maps to one inner UDP packet.
struct WgUdpFlow {
    handle: SocketHandle,
    /// Fixed inner destination for this association.
    remote: IpEndpoint,
    /// Caller -> socket datagrams.
    write_rx: mpsc::Receiver<Vec<u8>>,
    /// Socket -> caller datagrams.
    read_tx: mpsc::Sender<Vec<u8>>,
    /// A datagram accepted from the caller but not yet handed to the send buffer.
    pending: Option<Vec<u8>>,
}

impl DeviceLoop {
    pub(super) fn new(
        peers: Vec<PeerTunn>,
        config: &WireGuardOutboundConfig,
        commands: mpsc::Receiver<Command>,
    ) -> Self {
        Self {
            peers,
            mtu: config.mtu as usize,
            local_v4: config.local_v4,
            local_v6: config.local_v6,
            commands,
            flows: Vec::new(),
            udp_flows: Vec::new(),
            next_port: 1024,
            wake: Arc::new(Notify::new()),
            writer_wakers: Arc::new(Mutex::new(Vec::new())),
            amnezia: config.amnezia,
        }
    }

    pub(super) async fn run(mut self) {
        let start = Instant::now();
        let mut phy = WgPhy::new(self.mtu);
        let mut iface = build_interface(&mut phy, smol_now(start), self.local_v4, self.local_v6);
        let mut sockets = SocketSet::new(Vec::new());
        // One receive buffer per peer (each peer has its own UDP socket).
        let mut udp_bufs: Vec<Vec<u8>> = (0..self.peers.len()).map(|_| vec![0u8; 65535]).collect();
        let mut scratch = vec![0u8; 65535 + 32];

        // Kick each peer's handshake proactively so the first SYN has a session
        // to ride instead of waiting for smoltcp's first retransmit.
        for idx in 0..self.peers.len() {
            let reserved = self.peers[idx].reserved;
            if let TunnResult::WriteToNetwork(out) =
                self.peers[idx].tunn.format_handshake_initiation(&mut scratch, false)
            {
                send_obfuscated(&self.amnezia, &self.peers[idx].udp, reserved, out).await;
            }
        }

        // Next wall-clock instant at which the WireGuard timers must be driven.
        let mut next_timer = Instant::now() + TIMER_TICK;

        loop {
            let now = smol_now(start);
            iface.poll(now, &mut phy, &mut sockets);
            self.service_flows(&mut sockets, &mut iface);
            self.service_udp_flows(&mut sockets);
            self.wake_writers();
            self.encapsulate_tx(&mut phy, &mut scratch).await;

            // Drive rekey / keepalive / handshake-retransmit on a steady cadence
            // regardless of `select!` readiness. Folding this into the timeout
            // arm alone lets a busy tunnel (the `udp.recv`/`wake` arms always
            // ready) starve the timers, so a long-lived but bursty session could
            // miss its rekey and die; this gate fires on schedule under load.
            if Instant::now() >= next_timer {
                self.drive_timers(&mut scratch).await;
                next_timer = Instant::now() + TIMER_TICK;
            }

            // Wake by `next_timer` at the latest so the gate above runs on time.
            let timer_wait = next_timer.saturating_duration_since(Instant::now());
            let delay = iface
                .poll_delay(smol_now(start), &sockets)
                .map(|d| Duration::from_micros(d.total_micros()))
                .map_or(MAX_POLL_SLEEP, |d| d.min(MAX_POLL_SLEEP))
                .min(timer_wait);

            tokio::select! {
                _ = self.wake.notified() => {}
                cmd = self.commands.recv() => match cmd {
                    Some(cmd) => self.handle_command(cmd, &mut sockets, &mut iface),
                    None => return,
                },
                (idx, res) = recv_any(&self.peers, &mut udp_bufs) => {
                    if let Ok(n) = res {
                        self.decapsulate_rx(idx, n, &mut udp_bufs, &mut phy, &mut scratch).await;
                    }
                }
                _ = tokio::time::sleep(delay) => {}
            }
        }
    }

    /// Open a smoltcp client socket to `dst`, wire its bridge channels, and stash
    /// the caller's stream to hand over once it connects.
    fn handle_command(&mut self, cmd: Command, sockets: &mut SocketSet, iface: &mut Interface) {
        let (dst, reply) = match cmd {
            Command::OpenTcp { dst, reply } => (dst, reply),
            Command::OpenUdp { dst, reply } => return self.handle_open_udp(dst, reply, sockets),
        };
        let remote = IpEndpoint::new(ip_address(dst.ip()), dst.port());
        let mut sock = tcp::Socket::new(
            tcp::SocketBuffer::new(vec![0u8; FLOW_BUFFER]),
            tcp::SocketBuffer::new(vec![0u8; FLOW_BUFFER]),
        );
        let local_port = self.alloc_port();
        if sock.connect(iface.context(), remote, local_port).is_err() {
            return; // dropping `reply` reports the failure to the caller
        }
        let handle = sockets.add(sock);

        let (write_tx, write_rx) = mpsc::channel::<Vec<u8>>(CHANNEL_DEPTH);
        let (read_tx, read_rx) = mpsc::channel::<Vec<u8>>(CHANNEL_DEPTH);
        let stream = WgTcpStream {
            write_tx: Some(write_tx),
            read_rx,
            wake: self.wake.clone(),
            writer_wakers: self.writer_wakers.clone(),
            leftover: Vec::new(),
            leftover_pos: 0,
        };

        self.flows.push(WgFlow {
            handle,
            write_rx,
            read_tx: Some(read_tx),
            pending: Vec::new(),
            pending_off: 0,
            write_closed: false,
            connect_reply: Some(reply),
            stream_slot: Some(stream),
            deadline: Instant::now() + CONNECT_TIMEOUT,
        });
    }

    /// Open a smoltcp UDP socket bound to a local port for datagrams destined to
    /// `dst`, wire its bridge channels, and hand the association back. Unlike
    /// TCP there is no connect handshake, so the association is returned
    /// immediately; datagrams sent before the Noise handshake completes are
    /// dropped (UDP is lossy).
    fn handle_open_udp(&mut self, dst: SocketAddr, reply: oneshot::Sender<WgUdpAssoc>, sockets: &mut SocketSet) {
        let remote = IpEndpoint::new(ip_address(dst.ip()), dst.port());
        let mut sock = udp::Socket::new(
            udp::PacketBuffer::new(vec![udp::PacketMetadata::EMPTY; UDP_META_SLOTS], vec![0u8; FLOW_BUFFER]),
            udp::PacketBuffer::new(vec![udp::PacketMetadata::EMPTY; UDP_META_SLOTS], vec![0u8; FLOW_BUFFER]),
        );
        let local_port = self.alloc_port();
        if sock.bind(local_port).is_err() {
            return; // dropping `reply` reports the failure to the caller
        }
        let handle = sockets.add(sock);

        let (write_tx, write_rx) = mpsc::channel::<Vec<u8>>(CHANNEL_DEPTH);
        let (read_tx, read_rx) = mpsc::channel::<Vec<u8>>(CHANNEL_DEPTH);
        let assoc = WgUdpAssoc {
            write_tx,
            read_rx: tokio::sync::Mutex::new(read_rx),
            wake: self.wake.clone(),
        };
        self.udp_flows.push(WgUdpFlow {
            handle,
            remote,
            write_rx,
            read_tx,
            pending: None,
        });
        let _ = reply.send(assoc);
    }

    fn alloc_port(&mut self) -> u16 {
        let port = self.next_port;
        self.next_port = self.next_port.checked_add(1).unwrap_or(1024);
        port
    }

    /// Move datagrams between each UDP flow's smoltcp socket and its bridge
    /// channels, dropping (rather than stalling) when a buffer is full, and reap
    /// flows whose caller association has been dropped.
    fn service_udp_flows(&mut self, sockets: &mut SocketSet) {
        let mut done: Vec<usize> = Vec::new();
        for (idx, flow) in self.udp_flows.iter_mut().enumerate() {
            let sock = sockets.get_mut::<udp::Socket>(flow.handle);
            let mut reap = false;

            // caller -> socket
            loop {
                if flow.pending.is_none() {
                    match flow.write_rx.try_recv() {
                        Ok(buf) => flow.pending = Some(buf),
                        Err(mpsc::error::TryRecvError::Empty) => break,
                        Err(mpsc::error::TryRecvError::Disconnected) => {
                            reap = true;
                            break;
                        }
                    }
                }
                if !sock.can_send() {
                    break;
                }
                let Some(buf) = flow.pending.take() else { break };
                match sock.send_slice(&buf, flow.remote) {
                    Ok(()) => {}
                    // Send buffer is full: retry this datagram next turn.
                    Err(udp::SendError::BufferFull) => {
                        flow.pending = Some(buf);
                        break;
                    }
                    // No route to the destination: drop the datagram.
                    Err(udp::SendError::Unaddressable) => {}
                }
            }

            // socket -> caller
            while sock.can_recv() {
                let payload = match sock.recv() {
                    Ok((data, _meta)) => data.to_vec(),
                    Err(_) => break,
                };
                match flow.read_tx.try_send(payload) {
                    Ok(()) => {}
                    // Caller is draining slowly: drop this reply (UDP is lossy).
                    Err(mpsc::error::TrySendError::Full(_)) => break,
                    // Caller association dropped: reap the flow.
                    Err(mpsc::error::TrySendError::Closed(_)) => {
                        reap = true;
                        break;
                    }
                }
            }

            if reap {
                done.push(idx);
            }
        }

        for idx in done.into_iter().rev() {
            let flow = self.udp_flows.swap_remove(idx);
            sockets.remove(flow.handle);
        }
    }

    /// Move bytes between each flow's smoltcp socket and its bridge channels,
    /// resolve pending connects, and reap finished flows.
    fn service_flows(&mut self, sockets: &mut SocketSet, _iface: &mut Interface) {
        let mut done: Vec<usize> = Vec::new();
        for (idx, flow) in self.flows.iter_mut().enumerate() {
            let sock = sockets.get_mut::<tcp::Socket>(flow.handle);

            // Resolve the pending connect once established (or fail it).
            if flow.connect_reply.is_some() {
                if sock.state() == tcp::State::Established {
                    if let (Some(reply), Some(stream)) = (flow.connect_reply.take(), flow.stream_slot.take()) {
                        let _ = reply.send(stream);
                    }
                } else if Instant::now() >= flow.deadline || is_dead(sock.state()) {
                    flow.connect_reply = None; // dropping the sender fails the connect
                    flow.stream_slot = None;
                    done.push(idx);
                    continue;
                } else {
                    continue; // still connecting; no data bridging yet
                }
            }

            // caller -> socket
            loop {
                if flow.pending_off >= flow.pending.len() {
                    flow.pending.clear();
                    flow.pending_off = 0;
                    if flow.write_closed {
                        break;
                    }
                    match flow.write_rx.try_recv() {
                        Ok(buf) => flow.pending = buf,
                        Err(mpsc::error::TryRecvError::Empty) => break,
                        Err(mpsc::error::TryRecvError::Disconnected) => {
                            // Caller dropped/half-closed: FIN the socket once flushed.
                            if !flow.write_closed {
                                sock.close();
                                flow.write_closed = true;
                            }
                            break;
                        }
                    }
                }
                if !sock.can_send() {
                    break;
                }
                match sock.send_slice(&flow.pending[flow.pending_off..]) {
                    Ok(0) => break,
                    Ok(n) => flow.pending_off += n,
                    Err(_) => break,
                }
            }

            // socket -> caller
            if let Some(tx) = &flow.read_tx {
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

            // Peer FIN and everything drained -> signal EOF to the caller.
            if !sock.may_recv() && !sock.can_recv() {
                flow.read_tx = None;
            }

            if sock.state() == tcp::State::Closed {
                done.push(idx);
            }
        }

        for idx in done.into_iter().rev() {
            let flow = self.flows.swap_remove(idx);
            sockets.remove(flow.handle);
        }
    }

    fn wake_writers(&self) {
        let mut wakers = self.writer_wakers.lock().expect("wireguard writer wakers");
        for waker in wakers.drain(..) {
            waker.wake();
        }
    }

    /// Pick the peer an inner packet destined for `dst` should ride: the one
    /// whose `allowed-ips` has the longest prefix matching `dst`. Ties keep the
    /// earlier (lower-index) peer. Returns `None` when no peer claims `dst`.
    fn route(&self, dst: IpAddr) -> Option<usize> {
        let mut best: Option<(usize, u8)> = None;
        for (i, peer) in self.peers.iter().enumerate() {
            for allowed in &peer.allowed_ips {
                if allowed.contains(dst) {
                    let prefix = allowed.prefix();
                    if best.is_none_or(|(_, b)| prefix > b) {
                        best = Some((i, prefix));
                    }
                }
            }
        }
        best.map(|(i, _)| i)
    }

    /// Encapsulate every IP packet smoltcp queued and send it to the peer that
    /// claims the packet's destination (longest `allowed-ips` match). Packets
    /// claimed by no peer are dropped.
    async fn encapsulate_tx(&mut self, phy: &mut WgPhy, scratch: &mut [u8]) {
        while let Some(pkt) = phy.tx.pop_front() {
            let idx = match packet_dst_ip(&pkt).and_then(|dst| self.route(dst)) {
                Some(idx) => idx,
                None => continue,
            };
            let reserved = self.peers[idx].reserved;
            match self.peers[idx].tunn.encapsulate(&pkt, scratch) {
                TunnResult::WriteToNetwork(out) => {
                    send_obfuscated(&self.amnezia, &self.peers[idx].udp, reserved, out).await;
                }
                TunnResult::Err(_) | TunnResult::Done => {}
                // encapsulate only ever yields WriteToNetwork / Done / Err.
                _ => {}
            }
        }
    }

    /// Decapsulate one datagram received on peer `idx`'s socket, feeding
    /// decrypted IP packets to smoltcp and flushing any handshake/cookie
    /// responses back to that peer.
    async fn decapsulate_rx(
        &mut self,
        idx: usize,
        n: usize,
        udp_bufs: &mut [Vec<u8>],
        phy: &mut WgPhy,
        scratch: &mut [u8],
    ) {
        let reserved = self.peers[idx].reserved;
        // Reverse AmneziaWG obfuscation (or just clear `reserved`) before the
        // datagram reaches boringtun. A junk / unrecognised packet yields `None`
        // and is dropped.
        let n = match &self.amnezia {
            Some(am) => match deobfuscate(am, &mut udp_bufs[idx][..n]) {
                Some(m) => m,
                None => return,
            },
            None => {
                clear_reserved(&mut udp_bufs[idx][..n]);
                n
            }
        };
        // First call parses the datagram; subsequent calls with an empty slice
        // flush queued network writes until `Done`.
        let mut first = true;
        loop {
            let datagram: &[u8] = if first { &udp_bufs[idx][..n] } else { &[] };
            match self.peers[idx].tunn.decapsulate(None, datagram, scratch) {
                TunnResult::WriteToNetwork(out) => {
                    send_obfuscated(&self.amnezia, &self.peers[idx].udp, reserved, out).await;
                    first = false;
                }
                TunnResult::WriteToTunnelV4(pkt, _) | TunnResult::WriteToTunnelV6(pkt, _) => {
                    phy.rx.push_back(pkt.to_vec());
                    self.wake.notify_one();
                    break;
                }
                TunnResult::Done | TunnResult::Err(_) => break,
            }
        }
    }

    /// Drive rekey / keepalive / handshake retransmit timers for every peer,
    /// flushing each packet `update_timers` wants to emit this tick. boringtun
    /// yields at most one packet per call, so the bounded drain just covers the
    /// case where more than one timer is simultaneously due; a repeat call at
    /// the same instant returns `Done` and ends the loop.
    async fn drive_timers(&mut self, scratch: &mut [u8]) {
        for idx in 0..self.peers.len() {
            let reserved = self.peers[idx].reserved;
            for _ in 0..4 {
                match self.peers[idx].tunn.update_timers(scratch) {
                    TunnResult::WriteToNetwork(out) => {
                        send_obfuscated(&self.amnezia, &self.peers[idx].udp, reserved, out).await;
                    }
                    _ => break,
                }
            }
        }
    }
}

/// Recv on whichever peer socket is ready first, returning `(peer_index,
/// result)`. Each socket recvs into its own buffer; pending sockets register
/// their waker so the loop is rescheduled when any becomes readable.
async fn recv_any(peers: &[PeerTunn], bufs: &mut [Vec<u8>]) -> (usize, std::io::Result<usize>) {
    std::future::poll_fn(|cx| {
        for (i, (peer, buf)) in peers.iter().zip(bufs.iter_mut()).enumerate() {
            let mut rb = ReadBuf::new(&mut buf[..]);
            match peer.udp.poll_recv(cx, &mut rb) {
                Poll::Ready(Ok(())) => return Poll::Ready((i, Ok(rb.filled().len()))),
                Poll::Ready(Err(e)) => return Poll::Ready((i, Err(e))),
                Poll::Pending => {}
            }
        }
        Poll::Pending
    })
    .await
}

/// Read the destination address of an inner IP packet (IPv4 or IPv6) for
/// `allowed-ips` routing. Returns `None` for a truncated/unknown packet.
fn packet_dst_ip(pkt: &[u8]) -> Option<IpAddr> {
    match pkt.first()? >> 4 {
        4 if pkt.len() >= 20 => {
            let octets: [u8; 4] = pkt[16..20].try_into().ok()?;
            Some(IpAddr::V4(Ipv4Addr::from(octets)))
        }
        6 if pkt.len() >= 40 => {
            let octets: [u8; 16] = pkt[24..40].try_into().ok()?;
            Some(IpAddr::V6(Ipv6Addr::from(octets)))
        }
        _ => None,
    }
}
