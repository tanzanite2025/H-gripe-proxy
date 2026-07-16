//! The per-tunnel poll loop: bridges caller TCP flows through a smoltcp netstack
//! whose inner IP packets are sealed into (and recovered from) the OpenVPN AEAD
//! data channel.

use std::net::Ipv4Addr;
use std::sync::{Arc, Mutex};
use std::task::Waker;
use std::time::{Duration, Instant};

use smoltcp::iface::{Interface, SocketHandle, SocketSet};
use smoltcp::socket::tcp;
use smoltcp::wire::IpEndpoint;
use tokio::sync::{Notify, mpsc, oneshot};

use super::data::{DataChannel, is_ping};
use super::device::Command;
use super::netstack::{OvPhy, build_interface, ip_address, is_dead, smol_now};
use super::stream::OvpnTcpStream;

use crate::protocols::openvpn::control::PacketWriter;

/// Per-flow bridge channel depth (in chunks).
pub(super) const CHANNEL_DEPTH: usize = 64;
/// Per-flow smoltcp socket buffer size (each direction).
pub(super) const FLOW_BUFFER: usize = 64 * 1024;
/// How long to wait for a relayed TCP connection to reach `Established`.
const CONNECT_TIMEOUT: Duration = Duration::from_secs(10);
/// Upper bound on how long the device poll loop sleeps between wakeups.
const MAX_POLL_SLEEP: Duration = Duration::from_millis(250);

/// Wakers parked by streams whose write channel filled, woken once the loop has
/// drained their bytes into the smoltcp sockets.
pub(super) type WriterWakers = Arc<Mutex<Vec<Waker>>>;

/// State owned by the per-tunnel poll loop.
pub(super) struct DeviceLoop {
    data: DataChannel,
    writer: Arc<PacketWriter>,
    data_rx: mpsc::UnboundedReceiver<Vec<u8>>,
    commands: mpsc::Receiver<Command>,
    mtu: usize,
    local_v4: Ipv4Addr,
    flows: Vec<OvpnFlow>,
    next_port: u16,
    wake: Arc<Notify>,
    writer_wakers: WriterWakers,
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

impl DeviceLoop {
    pub(super) fn new(
        data: DataChannel,
        writer: Arc<PacketWriter>,
        data_rx: mpsc::UnboundedReceiver<Vec<u8>>,
        commands: mpsc::Receiver<Command>,
        mtu: usize,
        local_v4: Ipv4Addr,
    ) -> Self {
        Self {
            data,
            writer,
            data_rx,
            commands,
            mtu,
            local_v4,
            flows: Vec::new(),
            next_port: 1024,
            wake: Arc::new(Notify::new()),
            writer_wakers: Arc::new(Mutex::new(Vec::new())),
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
            self.wake_writers();
            self.encapsulate_tx(&mut phy).await;

            let delay = iface
                .poll_delay(smol_now(start), &sockets)
                .map(|d| Duration::from_micros(d.total_micros()))
                .map_or(MAX_POLL_SLEEP, |d| d.min(MAX_POLL_SLEEP));

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
                _ = tokio::time::sleep(delay) => {}
            }
        }
    }

    /// Open a smoltcp client socket to `dst`, wire its bridge channels, and stash
    /// the caller's stream to hand over once it connects.
    fn handle_command(&mut self, cmd: Command, sockets: &mut SocketSet, iface: &mut Interface) {
        let Command::OpenTcp { dst, reply } = cmd;
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

    fn alloc_port(&mut self) -> u16 {
        let port = self.next_port;
        self.next_port = self.next_port.checked_add(1).unwrap_or(1024);
        port
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
        while let Some(pkt) = phy.tx.pop_front() {
            if let Ok(sealed) = self.data.encrypt(&pkt) {
                let _ = self.writer.write_packet(&sealed).await;
            }
        }
    }

    /// Decrypt one received data packet and hand the inner IP packet to smoltcp,
    /// dropping keepalive pings and anything that fails to decrypt / replays.
    fn decapsulate(&mut self, packet: Vec<u8>, phy: &mut OvPhy) {
        if let Ok(plain) = self.data.decrypt(&packet) {
            if !plain.is_empty() && !is_ping(&plain) {
                phy.rx.push_back(plain);
                self.wake.notify_one();
            }
        }
    }
}
