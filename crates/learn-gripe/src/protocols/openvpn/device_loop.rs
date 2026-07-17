//! The per-tunnel poll loop: bridges caller TCP flows and UDP associations
//! through a smoltcp netstack whose inner IP packets are sealed into (and
//! recovered from) the OpenVPN AEAD data channel.

use std::net::Ipv4Addr;
use std::sync::{Arc, Mutex};
use std::task::Waker;
use std::time::{Duration, Instant};

use smoltcp::iface::{Interface, SocketHandle, SocketSet};
use smoltcp::socket::{tcp, udp};
use smoltcp::wire::IpEndpoint;
use tokio::sync::{Notify, mpsc, oneshot};

use super::data::{DataChannel, PING_PACKET, is_ping};
use super::device::Command;
use super::netstack::{OvPhy, build_interface, ip_address, is_dead, smol_now};
use super::packet::parse_opcode_key_id;
use super::stream::{OvpnTcpStream, OvpnUdpAssoc};

use crate::protocols::openvpn::control::PacketWriter;

/// Per-flow bridge channel depth (in chunks).
pub(super) const CHANNEL_DEPTH: usize = 64;
/// Per-flow smoltcp socket buffer size (each direction).
pub(super) const FLOW_BUFFER: usize = 64 * 1024;
/// Number of in-flight datagram slots per direction for a UDP flow's smoltcp
/// packet buffer (each datagram needs one metadata slot).
const UDP_META_SLOTS: usize = 64;
/// How long to wait for a relayed TCP connection to reach `Established`.
const CONNECT_TIMEOUT: Duration = Duration::from_secs(10);
/// Upper bound on how long the device poll loop sleeps between wakeups.
const MAX_POLL_SLEEP: Duration = Duration::from_millis(250);

/// Keepalive timers negotiated during the handshake (from the pushed
/// `keepalive` / `ping` / `ping-restart` options).
#[derive(Default, Clone, Copy)]
pub(super) struct Keepalive {
    /// Send a data-channel ping after this much send-side idle time.
    pub(super) ping_interval: Option<Duration>,
    /// Exit the loop (tearing the tunnel down) after this much receive-side
    /// silence, mirroring upstream's `ping-restart` SIGUSR1.
    pub(super) ping_restart: Option<Duration>,
}

/// Wakers parked by streams whose write channel filled, woken once the loop has
/// drained their bytes into the smoltcp sockets.
pub(super) type WriterWakers = Arc<Mutex<Vec<Waker>>>;

/// State owned by the per-tunnel poll loop.
pub(super) struct DeviceLoop {
    data: DataChannel,
    /// The previous key epoch, kept alive so in-flight packets sealed under the
    /// old key still decrypt while the peer transitions after a renegotiation.
    prev_data: Option<DataChannel>,
    writer: Arc<PacketWriter>,
    data_rx: mpsc::UnboundedReceiver<Vec<u8>>,
    /// Replacement data channels from the renegotiation service.
    rekey_rx: mpsc::UnboundedReceiver<DataChannel>,
    rekey_closed: bool,
    commands: mpsc::Receiver<Command>,
    mtu: usize,
    local_v4: Ipv4Addr,
    flows: Vec<OvpnFlow>,
    udp_flows: Vec<OvpnUdpFlow>,
    next_port: u16,
    wake: Arc<Notify>,
    writer_wakers: WriterWakers,
    keepalive: Keepalive,
    last_send: Instant,
    last_recv: Instant,
}

/// Bridge state for one relayed TCP flow, owned by the poll loop.
struct OvpnFlow {
    handle: SocketHandle,
    write_rx: mpsc::Receiver<Vec<u8>>,
    read_tx: Option<mpsc::Sender<Vec<u8>>>,
    pending: Vec<u8>,
    pending_off: usize,
    write_closed: bool,
    connect_reply: Option<oneshot::Sender<OvpnTcpStream>>,
    stream_slot: Option<OvpnTcpStream>,
    deadline: Instant,
}

/// Bridge state for one relayed UDP association, owned by the poll loop. Unlike
/// TCP there is no connection state: datagrams flow to a fixed `remote` and one
/// datagram maps to one inner UDP packet.
struct OvpnUdpFlow {
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
        data: DataChannel,
        writer: Arc<PacketWriter>,
        data_rx: mpsc::UnboundedReceiver<Vec<u8>>,
        commands: mpsc::Receiver<Command>,
        mtu: usize,
        local_v4: Ipv4Addr,
        keepalive: Keepalive,
        rekey_rx: mpsc::UnboundedReceiver<DataChannel>,
    ) -> Self {
        let now = Instant::now();
        Self {
            data,
            prev_data: None,
            writer,
            data_rx,
            rekey_rx,
            rekey_closed: false,
            commands,
            mtu,
            local_v4,
            flows: Vec::new(),
            udp_flows: Vec::new(),
            next_port: 1024,
            wake: Arc::new(Notify::new()),
            writer_wakers: Arc::new(Mutex::new(Vec::new())),
            keepalive,
            last_send: now,
            last_recv: now,
        }
    }

    pub(super) async fn run(mut self) {
        let start = Instant::now();
        let mut phy = OvPhy::new(self.mtu);
        let mut iface = build_interface(&mut phy, smol_now(start), self.local_v4);
        let mut sockets = SocketSet::new(Vec::new());

        loop {
            let now = smol_now(start);
            iface.poll(now, &mut phy, &mut sockets);
            self.service_flows(&mut sockets);
            self.service_udp_flows(&mut sockets);
            self.wake_writers();
            self.encapsulate_tx(&mut phy).await;
            if !self.service_keepalive().await {
                return; // ping-restart expired: tear the tunnel down
            }

            let delay = iface
                .poll_delay(smol_now(start), &sockets)
                .map(|d| Duration::from_micros(d.total_micros()))
                .map_or(MAX_POLL_SLEEP, |d| d.min(MAX_POLL_SLEEP));
            let delay = self.keepalive_deadline().map_or(delay, |d| d.min(delay));

            tokio::select! {
                _ = self.wake.notified() => {}
                cmd = self.commands.recv() => match cmd {
                    Some(cmd) => self.handle_command(cmd, &mut sockets, &mut iface),
                    None => return,
                },
                pkt = self.data_rx.recv() => match pkt {
                    Some(pkt) => self.decapsulate(pkt, &mut phy),
                    None => return,
                },
                next = self.rekey_rx.recv(), if !self.rekey_closed => match next {
                    Some(next) => self.prev_data = Some(std::mem::replace(&mut self.data, next)),
                    None => self.rekey_closed = true,
                },
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
        let stream = OvpnTcpStream {
            write_tx: Some(write_tx),
            read_rx,
            wake: self.wake.clone(),
            writer_wakers: self.writer_wakers.clone(),
            leftover: Vec::new(),
            leftover_pos: 0,
        };

        self.flows.push(OvpnFlow {
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
    /// immediately.
    fn handle_open_udp(
        &mut self,
        dst: std::net::SocketAddr,
        reply: oneshot::Sender<OvpnUdpAssoc>,
        sockets: &mut SocketSet,
    ) {
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
        let assoc = OvpnUdpAssoc {
            write_tx,
            read_rx: tokio::sync::Mutex::new(read_rx),
            wake: self.wake.clone(),
        };
        self.udp_flows.push(OvpnUdpFlow {
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
    fn service_flows(&mut self, sockets: &mut SocketSet) {
        let mut done: Vec<usize> = Vec::new();
        for (idx, flow) in self.flows.iter_mut().enumerate() {
            let sock = sockets.get_mut::<tcp::Socket>(flow.handle);

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
        let mut wakers = self.writer_wakers.lock().expect("openvpn writer wakers");
        for waker in wakers.drain(..) {
            waker.wake();
        }
    }

    /// Encrypt every IP packet smoltcp queued into a `P_DATA_V2` packet and write
    /// it to the server. Failures drop the packet (the inner TCP retransmits).
    async fn encapsulate_tx(&mut self, phy: &mut OvPhy) {
        let mut sent = false;
        while let Some(pkt) = phy.tx.pop_front() {
            if let Ok(sealed) = self.data.encrypt(&pkt) {
                let _ = self.writer.write_packet(&sealed).await;
                sent = true;
            }
        }
        if sent {
            self.last_send = Instant::now();
        }
    }

    /// Enforce the negotiated keepalive timers: send a data-channel ping once
    /// the send side has been idle for `ping_interval`, and return `false`
    /// (tearing the tunnel down) once nothing has been received for
    /// `ping_restart`.
    async fn service_keepalive(&mut self) -> bool {
        let now = Instant::now();
        if let Some(restart) = self.keepalive.ping_restart
            && now.duration_since(self.last_recv) >= restart
        {
            return false;
        }
        if let Some(interval) = self.keepalive.ping_interval
            && now.duration_since(self.last_send) >= interval
        {
            if let Ok(sealed) = self.data.encrypt(&PING_PACKET) {
                let _ = self.writer.write_packet(&sealed).await;
            }
            self.last_send = now;
        }
        true
    }

    /// Time until the next keepalive timer fires, so the poll loop wakes up in
    /// time to service it.
    fn keepalive_deadline(&self) -> Option<Duration> {
        let now = Instant::now();
        let until = |last: Instant, period: Duration| period.saturating_sub(now.duration_since(last));
        let ping = self.keepalive.ping_interval.map(|p| until(self.last_send, p));
        let restart = self.keepalive.ping_restart.map(|p| until(self.last_recv, p));
        match (ping, restart) {
            (Some(a), Some(b)) => Some(a.min(b)),
            (a, b) => a.or(b),
        }
    }

    /// Decrypt one received data packet and hand the inner IP packet to smoltcp,
    /// dropping keepalive pings and anything that fails to decrypt / replays.
    fn decapsulate(&mut self, packet: Vec<u8>, phy: &mut OvPhy) {
        let Some(&first) = packet.first() else { return };
        let (_, key_id) = parse_opcode_key_id(first);
        let channel = if key_id == self.data.key_id() {
            &mut self.data
        } else if let Some(prev) = self.prev_data.as_mut().filter(|p| p.key_id() == key_id) {
            prev
        } else {
            return; // unknown key epoch
        };
        if let Ok(plain) = channel.decrypt(&packet) {
            self.last_recv = Instant::now();
            if !plain.is_empty() && !is_ping(&plain) {
                phy.rx.push_back(plain);
                self.wake.notify_one();
            }
        }
    }
}
