//! WireGuard outbound data plane.
//!
//! Unlike the per-target proxy outbounds (Trojan/VLESS/Snell/…), WireGuard is
//! not a stream proxy: it is an L3 encrypted tunnel carrying arbitrary IP
//! packets to one peer. To relay a TCP connection we run a **userspace TCP/IP
//! stack** (smoltcp, already vendored for the TUN inbound) bound to the address
//! the peer assigned us; each relayed connection is a smoltcp socket whose IP
//! packets are sealed by WireGuard and sent to the peer over a real UDP socket,
//! and whose inbound packets come from decrypting the peer's UDP datagrams.
//! This mirrors sing-box / wireguard-go's userspace `netstack`.
//!
//! The Noise_IKpsk2 handshake, transport-data sealing, rekey/cookie/keepalive
//! timers — the error-prone protocol state machine — are delegated to the
//! vetted `boringtun` crate (`noise::Tunn`), which deliberately ships no
//! network or tunnel stack. We own only the orchestration: UDP I/O, the smoltcp
//! netstack, per-connection bridging, and the per-config device registry. This
//! is the same "delegate the wire codec, own the plumbing" split used for
//! rustls / quinn / smoltcp / hickory elsewhere in the kernel.
//!
//! Scope (this module): single peer, **TCP relay** (IPv4/IPv6 inner targets).
//! UDP relay, multi-peer, amnezia-wg obfuscation, and tunnel-side DNS
//! (`remote-dns-resolve`) are deliberately left to follow-ups; a domain target
//! is resolved to an IP by the host resolver before it enters the tunnel.

use std::collections::HashMap;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::task::{Context as TaskContext, Poll, Waker};
use std::time::{Duration, Instant};

use anyhow::{Context, Result, anyhow, bail};
use boringtun::noise::{Tunn, TunnResult};
use boringtun::x25519::{PublicKey, StaticSecret};
use smoltcp::iface::{Config as IfaceConfig, Interface, SocketHandle, SocketSet};
use smoltcp::socket::tcp;
use smoltcp::time::Instant as SmolInstant;
use smoltcp::wire::{HardwareAddress, IpAddress, IpCidr, IpEndpoint};
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};
use tokio::net::UdpSocket;
use tokio::sync::{Notify, mpsc, oneshot};

use crate::address::TargetAddr;
use crate::config::outbound_opts::ProxyEntry;
use crate::outbound::BoxedStream;

/// Default tunnel MTU (max inner IP packet); WireGuard adds a 32-byte overhead
/// on top, so the UDP datagram stays within a typical 1500-byte path.
const DEFAULT_MTU: u32 = 1408;
/// Per-flow bridge channel depth (in chunks).
const CHANNEL_DEPTH: usize = 64;
/// Per-flow smoltcp socket buffer size (each direction).
const FLOW_BUFFER: usize = 64 * 1024;
/// How long to wait for a relayed TCP connection to reach `Established` (covers
/// the WireGuard handshake plus the inner TCP handshake).
const CONNECT_TIMEOUT: Duration = Duration::from_secs(10);
/// Upper bound on how long the device poll loop sleeps between wakeups; also the
/// cadence at which `Tunn::update_timers` is driven (rekey / keepalive).
const MAX_POLL_SLEEP: Duration = Duration::from_millis(250);

/// Parsed WireGuard outbound configuration (single peer).
#[derive(Debug, Clone)]
pub struct WireGuardOutboundConfig {
    pub server: String,
    pub port: u16,
    private_key: [u8; 32],
    public_key: [u8; 32],
    preshared_key: Option<[u8; 32]>,
    local_v4: Option<Ipv4Addr>,
    local_v6: Option<Ipv6Addr>,
    mtu: u32,
    reserved: [u8; 3],
    keepalive: Option<u16>,
}

impl PartialEq for WireGuardOutboundConfig {
    fn eq(&self, other: &Self) -> bool {
        self.server == other.server
            && self.port == other.port
            && self.private_key == other.private_key
            && self.public_key == other.public_key
            && self.preshared_key == other.preshared_key
            && self.local_v4 == other.local_v4
            && self.local_v6 == other.local_v6
            && self.mtu == other.mtu
            && self.reserved == other.reserved
            && self.keepalive == other.keepalive
    }
}

impl Eq for WireGuardOutboundConfig {}

impl WireGuardOutboundConfig {
    pub fn from_proxy(entry: &ProxyEntry) -> Result<Self> {
        let opts = &entry.options;
        let server = opts
            .server
            .clone()
            .ok_or_else(|| anyhow!("wireguard: missing `server`"))?;
        let port = opts.port.ok_or_else(|| anyhow!("wireguard: missing `port`"))?;
        let private_key = parse_key(
            opts.private_key
                .as_deref()
                .ok_or_else(|| anyhow!("wireguard: missing `private-key`"))?,
        )
        .context("wireguard: invalid `private-key`")?;
        let public_key = parse_key(
            opts.public_key
                .as_deref()
                .ok_or_else(|| anyhow!("wireguard: missing `public-key`"))?,
        )
        .context("wireguard: invalid `public-key`")?;
        let preshared_key = match opts.pre_shared_key.as_deref() {
            Some(psk) => Some(parse_key(psk).context("wireguard: invalid `pre-shared-key`")?),
            None => None,
        };

        let local_v4 = match opts.ip.as_deref() {
            Some(ip) => Some(parse_local_v4(ip).with_context(|| format!("wireguard: invalid `ip` {ip:?}"))?),
            None => None,
        };
        let local_v6 = match opts.ipv6.as_deref() {
            Some(ip) => Some(
                ip.trim()
                    .split('/')
                    .next()
                    .unwrap_or("")
                    .parse::<Ipv6Addr>()
                    .with_context(|| format!("wireguard: invalid `ipv6` {ip:?}"))?,
            ),
            None => None,
        };
        if local_v4.is_none() && local_v6.is_none() {
            bail!("wireguard: at least one of `ip` / `ipv6` (the assigned tunnel address) is required");
        }

        let reserved = match &opts.reserved {
            Some(bytes) => {
                if bytes.len() != 3 {
                    bail!("wireguard: `reserved` must be exactly 3 bytes, got {}", bytes.len());
                }
                [bytes[0], bytes[1], bytes[2]]
            }
            None => [0u8; 3],
        };

        let keepalive = opts.persistent_keepalive.and_then(|k| {
            if k == 0 {
                None
            } else {
                Some(k.min(u16::MAX as u32) as u16)
            }
        });

        let mtu = opts.mtu.filter(|m| *m >= 576).unwrap_or(DEFAULT_MTU);

        Ok(Self {
            server,
            port,
            private_key,
            public_key,
            preshared_key,
            local_v4,
            local_v6,
            mtu,
            reserved,
            keepalive,
        })
    }

    fn registry_key(&self) -> WgKey {
        (self.server.clone(), self.port, self.public_key, self.private_key)
    }
}

/// Connect a relayed TCP stream to `target` through the configured WireGuard
/// tunnel, reusing (or lazily building) the per-config device.
pub async fn connect(config: &WireGuardOutboundConfig, target: &TargetAddr) -> Result<BoxedStream> {
    let dst = resolve_target(target).await?;
    let device = WireGuardDevice::get_or_create(config).await?;
    let stream = device.open_tcp(dst).await?;
    Ok(Box::new(stream) as BoxedStream)
}

/// Resolve a relayed target to a literal socket address. Domains are resolved by
/// the host resolver (tunnel-side DNS is a follow-up).
async fn resolve_target(target: &TargetAddr) -> Result<SocketAddr> {
    match target {
        TargetAddr::Ip(addr) => Ok(*addr),
        TargetAddr::Domain(host, port) => tokio::net::lookup_host((host.as_str(), *port))
            .await
            .with_context(|| format!("wireguard: resolve {host}:{port}"))?
            .next()
            .ok_or_else(|| anyhow!("wireguard: no addresses for {host}:{port}")),
    }
}

type WgKey = (String, u16, [u8; 32], [u8; 32]);

/// Per-config registry of live tunnel devices, so concurrent connections to the
/// same peer share one Noise session + netstack (mirrors the AnyTLS session
/// registry). A device whose command channel has closed (its loop exited) is
/// discarded and rebuilt on the next connect.
static DEVICE_REGISTRY: Mutex<Option<HashMap<WgKey, Arc<WireGuardDevice>>>> = Mutex::new(None);

/// Command sent from a `connect` call into the device's poll loop.
enum Command {
    OpenTcp {
        dst: SocketAddr,
        reply: oneshot::Sender<WgTcpStream>,
    },
}

/// Handle to a running WireGuard tunnel device: just the command channel into
/// its poll loop task.
pub struct WireGuardDevice {
    commands: mpsc::Sender<Command>,
}

impl WireGuardDevice {
    async fn get_or_create(config: &WireGuardOutboundConfig) -> Result<Arc<Self>> {
        let key = config.registry_key();
        {
            let mut guard = DEVICE_REGISTRY.lock().expect("wireguard device registry");
            let map = guard.get_or_insert_with(HashMap::new);
            if let Some(device) = map.get(&key) {
                if !device.commands.is_closed() {
                    return Ok(device.clone());
                }
                map.remove(&key);
            }
        }

        let device = Arc::new(Self::spawn(config).await?);
        let mut guard = DEVICE_REGISTRY.lock().expect("wireguard device registry");
        let map = guard.get_or_insert_with(HashMap::new);
        // Another task may have raced us; prefer the existing live device.
        if let Some(existing) = map.get(&key) {
            if !existing.commands.is_closed() {
                return Ok(existing.clone());
            }
        }
        map.insert(key, device.clone());
        Ok(device)
    }

    /// Dial the peer's UDP endpoint, build the Noise tunnel + smoltcp interface,
    /// and spawn the poll loop.
    async fn spawn(config: &WireGuardOutboundConfig) -> Result<Self> {
        let peer = tokio::net::lookup_host((config.server.as_str(), config.port))
            .await
            .with_context(|| format!("wireguard: resolve peer {}:{}", config.server, config.port))?
            .next()
            .ok_or_else(|| anyhow!("wireguard: no addresses for peer {}:{}", config.server, config.port))?;

        let bind: SocketAddr = if peer.is_ipv4() {
            (Ipv4Addr::UNSPECIFIED, 0).into()
        } else {
            (Ipv6Addr::UNSPECIFIED, 0).into()
        };
        let udp = UdpSocket::bind(bind).await.context("wireguard: bind UDP socket")?;
        udp.connect(peer)
            .await
            .with_context(|| format!("wireguard: connect UDP to {peer}"))?;

        let mut index = [0u8; 4];
        getrandom::fill(&mut index).map_err(|_| anyhow!("wireguard: system RNG unavailable"))?;
        let tunn = Tunn::new(
            StaticSecret::from(config.private_key),
            PublicKey::from(config.public_key),
            config.preshared_key,
            config.keepalive,
            u32::from_le_bytes(index),
            None,
        );

        let (commands_tx, commands_rx) = mpsc::channel::<Command>(CHANNEL_DEPTH);
        let loop_state = DeviceLoop::new(tunn, udp, config, commands_rx);
        tokio::spawn(loop_state.run());

        Ok(Self { commands: commands_tx })
    }

    async fn open_tcp(&self, dst: SocketAddr) -> Result<WgTcpStream> {
        let (reply_tx, reply_rx) = oneshot::channel();
        self.commands
            .send(Command::OpenTcp { dst, reply: reply_tx })
            .await
            .map_err(|_| anyhow!("wireguard: device loop is gone"))?;
        reply_rx
            .await
            .map_err(|_| anyhow!("wireguard: connection to {dst} failed (handshake/connect timeout)"))
    }
}

/// Wakers parked by streams whose write channel filled, woken once the loop has
/// drained their bytes into the smoltcp sockets.
type WriterWakers = Arc<Mutex<Vec<Waker>>>;

/// State owned by the per-device poll loop.
struct DeviceLoop {
    tunn: Tunn,
    udp: UdpSocket,
    reserved: [u8; 3],
    mtu: usize,
    local_v4: Option<Ipv4Addr>,
    local_v6: Option<Ipv6Addr>,
    commands: mpsc::Receiver<Command>,
    flows: Vec<WgFlow>,
    next_port: u16,
    wake: Arc<Notify>,
    writer_wakers: WriterWakers,
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

impl DeviceLoop {
    fn new(tunn: Tunn, udp: UdpSocket, config: &WireGuardOutboundConfig, commands: mpsc::Receiver<Command>) -> Self {
        Self {
            tunn,
            udp,
            reserved: config.reserved,
            mtu: config.mtu as usize,
            local_v4: config.local_v4,
            local_v6: config.local_v6,
            commands,
            flows: Vec::new(),
            next_port: 1024,
            wake: Arc::new(Notify::new()),
            writer_wakers: Arc::new(Mutex::new(Vec::new())),
        }
    }

    async fn run(mut self) {
        let start = Instant::now();
        let mut phy = WgPhy::new(self.mtu);
        let mut iface = build_interface(&mut phy, smol_now(start), self.local_v4, self.local_v6);
        let mut sockets = SocketSet::new(Vec::new());
        let mut udp_buf = vec![0u8; 65535];
        let mut scratch = vec![0u8; 65535 + 32];

        // Kick the handshake proactively so the first SYN has a session to ride
        // instead of waiting for smoltcp's first retransmit.
        if let TunnResult::WriteToNetwork(out) = self.tunn.format_handshake_initiation(&mut scratch, false) {
            apply_reserved(out, self.reserved);
            let _ = self.udp.send(out).await;
        }

        loop {
            let now = smol_now(start);
            iface.poll(now, &mut phy, &mut sockets);
            self.service_flows(&mut sockets, &mut iface);
            self.wake_writers();
            self.encapsulate_tx(&mut phy, &mut scratch).await;

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
                res = self.udp.recv(&mut udp_buf) => {
                    if let Ok(n) = res {
                        self.decapsulate_rx(&mut udp_buf, n, &mut phy, &mut scratch).await;
                    }
                }
                _ = tokio::time::sleep(delay) => {
                    self.drive_timers(&mut scratch).await;
                }
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

    fn alloc_port(&mut self) -> u16 {
        let port = self.next_port;
        self.next_port = self.next_port.checked_add(1).unwrap_or(1024);
        port
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

    /// Encapsulate every IP packet smoltcp queued and send it to the peer.
    async fn encapsulate_tx(&mut self, phy: &mut WgPhy, scratch: &mut [u8]) {
        while let Some(pkt) = phy.tx.pop_front() {
            match self.tunn.encapsulate(&pkt, scratch) {
                TunnResult::WriteToNetwork(out) => {
                    apply_reserved(out, self.reserved);
                    let _ = self.udp.send(out).await;
                }
                TunnResult::Err(_) | TunnResult::Done => {}
                // encapsulate only ever yields WriteToNetwork / Done / Err.
                _ => {}
            }
        }
    }

    /// Decapsulate one received UDP datagram, feeding decrypted IP packets to
    /// smoltcp and flushing any handshake/cookie responses back to the peer.
    async fn decapsulate_rx(&mut self, udp_buf: &mut [u8], n: usize, phy: &mut WgPhy, scratch: &mut [u8]) {
        clear_reserved(&mut udp_buf[..n]);
        // First call parses the datagram; subsequent calls with an empty slice
        // flush queued network writes until `Done`.
        let mut first = true;
        loop {
            let datagram: &[u8] = if first { &udp_buf[..n] } else { &[] };
            match self.tunn.decapsulate(None, datagram, scratch) {
                TunnResult::WriteToNetwork(out) => {
                    apply_reserved(out, self.reserved);
                    let _ = self.udp.send(out).await;
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

    /// Drive rekey / keepalive / handshake retransmit timers.
    async fn drive_timers(&mut self, scratch: &mut [u8]) {
        if let TunnResult::WriteToNetwork(out) = self.tunn.update_timers(scratch) {
            apply_reserved(out, self.reserved);
            let _ = self.udp.send(out).await;
        }
    }
}

/// A relayed TCP stream over the tunnel: channel-backed `AsyncRead`/`AsyncWrite`
/// bridged to a smoltcp socket inside the device loop.
pub struct WgTcpStream {
    /// Caller -> loop bytes; dropped on shutdown to half-close the socket.
    write_tx: Option<mpsc::Sender<Vec<u8>>>,
    /// Loop -> caller bytes; closed (EOF) on peer FIN or device shutdown.
    read_rx: mpsc::Receiver<Vec<u8>>,
    wake: Arc<Notify>,
    writer_wakers: WriterWakers,
    leftover: Vec<u8>,
    leftover_pos: usize,
}

impl AsyncRead for WgTcpStream {
    fn poll_read(self: Pin<&mut Self>, cx: &mut TaskContext<'_>, buf: &mut ReadBuf<'_>) -> Poll<std::io::Result<()>> {
        let this = self.get_mut();
        loop {
            if this.leftover_pos < this.leftover.len() {
                let n = buf.remaining().min(this.leftover.len() - this.leftover_pos);
                buf.put_slice(&this.leftover[this.leftover_pos..this.leftover_pos + n]);
                this.leftover_pos += n;
                return Poll::Ready(Ok(()));
            }
            match this.read_rx.poll_recv(cx) {
                Poll::Ready(Some(data)) => {
                    this.leftover = data;
                    this.leftover_pos = 0;
                }
                Poll::Ready(None) => return Poll::Ready(Ok(())), // EOF
                Poll::Pending => return Poll::Pending,
            }
        }
    }
}

impl AsyncWrite for WgTcpStream {
    fn poll_write(self: Pin<&mut Self>, cx: &mut TaskContext<'_>, buf: &[u8]) -> Poll<std::io::Result<usize>> {
        let this = self.get_mut();
        let Some(tx) = &this.write_tx else {
            return Poll::Ready(Err(std::io::ErrorKind::BrokenPipe.into()));
        };
        if buf.is_empty() {
            return Poll::Ready(Ok(0));
        }
        let take = buf.len().min(FLOW_BUFFER);
        match tx.try_send(buf[..take].to_vec()) {
            Ok(()) => {
                this.wake.notify_one();
                Poll::Ready(Ok(take))
            }
            Err(mpsc::error::TrySendError::Full(_)) => {
                this.writer_wakers
                    .lock()
                    .expect("wireguard writer wakers")
                    .push(cx.waker().clone());
                this.wake.notify_one();
                Poll::Pending
            }
            Err(mpsc::error::TrySendError::Closed(_)) => Poll::Ready(Err(std::io::ErrorKind::BrokenPipe.into())),
        }
    }

    fn poll_flush(self: Pin<&mut Self>, _cx: &mut TaskContext<'_>) -> Poll<std::io::Result<()>> {
        Poll::Ready(Ok(()))
    }

    fn poll_shutdown(self: Pin<&mut Self>, _cx: &mut TaskContext<'_>) -> Poll<std::io::Result<()>> {
        let this = self.get_mut();
        // Dropping the write sender signals half-close to the loop, which FINs
        // the socket once buffered bytes are flushed.
        if this.write_tx.take().is_some() {
            this.wake.notify_one();
        }
        Poll::Ready(Ok(()))
    }
}

// --- smoltcp in-memory device --------------------------------------------------

/// In-memory smoltcp [`Device`](smoltcp::phy::Device) backed by two frame
/// queues: `tx` holds IP packets the stack wants encrypted + sent to the peer,
/// `rx` holds decrypted IP packets from the peer waiting to enter the stack.
struct WgPhy {
    rx: std::collections::VecDeque<Vec<u8>>,
    tx: std::collections::VecDeque<Vec<u8>>,
    mtu: usize,
}

impl WgPhy {
    fn new(mtu: usize) -> Self {
        Self {
            rx: std::collections::VecDeque::new(),
            tx: std::collections::VecDeque::new(),
            mtu,
        }
    }
}

struct PhyRxToken {
    buf: Vec<u8>,
}

struct PhyTxToken<'a> {
    tx: &'a mut std::collections::VecDeque<Vec<u8>>,
}

impl smoltcp::phy::Device for WgPhy {
    type RxToken<'a> = PhyRxToken;
    type TxToken<'a> = PhyTxToken<'a>;

    fn receive(&mut self, _t: SmolInstant) -> Option<(Self::RxToken<'_>, Self::TxToken<'_>)> {
        let buf = self.rx.pop_front()?;
        Some((PhyRxToken { buf }, PhyTxToken { tx: &mut self.tx }))
    }

    fn transmit(&mut self, _t: SmolInstant) -> Option<Self::TxToken<'_>> {
        Some(PhyTxToken { tx: &mut self.tx })
    }

    fn capabilities(&self) -> smoltcp::phy::DeviceCapabilities {
        let mut caps = smoltcp::phy::DeviceCapabilities::default();
        caps.medium = smoltcp::phy::Medium::Ip;
        caps.max_transmission_unit = self.mtu;
        caps
    }
}

impl smoltcp::phy::RxToken for PhyRxToken {
    fn consume<R, F: FnOnce(&[u8]) -> R>(self, f: F) -> R {
        f(&self.buf)
    }
}

impl smoltcp::phy::TxToken for PhyTxToken<'_> {
    fn consume<R, F: FnOnce(&mut [u8]) -> R>(self, len: usize, f: F) -> R {
        let mut buf = vec![0u8; len];
        let result = f(&mut buf);
        self.tx.push_back(buf);
        result
    }
}

/// Build the userspace interface, assigning the peer-given tunnel address(es) at
/// prefix 0 so every inner destination is treated as on-link (the tunnel is the
/// only egress) while replies still source from our assigned address.
fn build_interface(
    phy: &mut WgPhy,
    now: SmolInstant,
    local_v4: Option<Ipv4Addr>,
    local_v6: Option<Ipv6Addr>,
) -> Interface {
    let config = IfaceConfig::new(HardwareAddress::Ip);
    let mut iface = Interface::new(config, phy, now);
    iface.set_any_ip(true);
    iface.update_ip_addrs(|addrs| {
        if let Some(v4) = local_v4 {
            let _ = addrs.push(IpCidr::new(IpAddress::Ipv4(v4), 0));
        }
        if let Some(v6) = local_v6 {
            let _ = addrs.push(IpCidr::new(IpAddress::Ipv6(v6), 0));
        }
    });
    if let Some(v4) = local_v4 {
        let _ = iface.routes_mut().add_default_ipv4_route(v4);
    }
    if let Some(v6) = local_v6 {
        let _ = iface.routes_mut().add_default_ipv6_route(v6);
    }
    iface
}

fn smol_now(start: Instant) -> SmolInstant {
    SmolInstant::from_micros(start.elapsed().as_micros() as i64)
}

fn ip_address(ip: IpAddr) -> IpAddress {
    match ip {
        IpAddr::V4(v4) => IpAddress::Ipv4(v4),
        IpAddr::V6(v6) => IpAddress::Ipv6(v6),
    }
}

fn is_dead(state: tcp::State) -> bool {
    matches!(state, tcp::State::Closed | tcp::State::TimeWait | tcp::State::Closing)
}

/// Stamp the 3-byte WireGuard `reserved` field (bytes 1..4 of the message
/// header) on an outgoing datagram. A no-op for the default all-zero value.
fn apply_reserved(datagram: &mut [u8], reserved: [u8; 3]) {
    if reserved != [0u8; 3] && datagram.len() >= 4 {
        datagram[1..4].copy_from_slice(&reserved);
    }
}

/// Zero the `reserved` field before handing a received datagram to boringtun
/// (which validates those bytes are zero).
fn clear_reserved(datagram: &mut [u8]) {
    if datagram.len() >= 4 {
        datagram[1] = 0;
        datagram[2] = 0;
        datagram[3] = 0;
    }
}

/// Parse a base64-encoded 32-byte WireGuard key.
fn parse_key(value: &str) -> Result<[u8; 32]> {
    let bytes = base64_decode(value.trim())?;
    if bytes.len() != 32 {
        bail!("expected a 32-byte key, decoded {} bytes", bytes.len());
    }
    let mut key = [0u8; 32];
    key.copy_from_slice(&bytes);
    Ok(key)
}

/// Parse the assigned IPv4 tunnel address (`ip`), accepting an optional CIDR
/// suffix (`10.0.0.2/32`).
fn parse_local_v4(value: &str) -> Result<Ipv4Addr> {
    value
        .trim()
        .split('/')
        .next()
        .unwrap_or("")
        .parse::<Ipv4Addr>()
        .map_err(|_| anyhow!("not an IPv4 address"))
}

/// Decode standard or URL-safe Base64 (padding / whitespace ignored).
fn base64_decode(input: &str) -> Result<Vec<u8>> {
    fn sextet(c: u8) -> Option<u8> {
        match c {
            b'A'..=b'Z' => Some(c - b'A'),
            b'a'..=b'z' => Some(c - b'a' + 26),
            b'0'..=b'9' => Some(c - b'0' + 52),
            b'+' | b'-' => Some(62),
            b'/' | b'_' => Some(63),
            _ => None,
        }
    }
    let mut out = Vec::with_capacity(input.len() / 4 * 3);
    let mut acc = 0u32;
    let mut bits = 0u32;
    for &c in input.as_bytes() {
        if c == b'=' || c.is_ascii_whitespace() {
            continue;
        }
        let v = sextet(c).ok_or_else(|| anyhow!("invalid base64 character {:?}", c as char))?;
        acc = (acc << 6) | u32::from(v);
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push((acc >> bits) as u8);
        }
    }
    Ok(out)
}
