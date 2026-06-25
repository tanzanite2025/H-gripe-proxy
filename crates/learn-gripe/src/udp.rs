//! SOCKS5 `UDP ASSOCIATE` relay.
//!
//! The inbound answers an associate request with a bound UDP socket; the client
//! then sends SOCKS5-wrapped datagrams to that socket. Each datagram names its
//! own destination, so a single association can talk to many remote hosts. We
//! keep one **egress task** per destination, fed by a bounded channel, and that
//! task owns the destination's egress and a reverse path that wraps replies
//! back to the client.
//!
//! The egress is resolved per destination by the router:
//! - `Direct` egresses over a plain OS UDP socket;
//! - Trojan / VLESS / VMess tunnel each datagram through their protocol UDP
//!   framing over the (TCP/TLS/REALITY) outbound stream;
//! - destinations that resolve to a non-UDP-capable outbound (`Reject`, an
//!   upstream SOCKS5 proxy) are dropped rather than leaked.
//!
//! Per RFC 1928 the association lives exactly as long as the TCP control
//! connection that created it; when that connection closes we drop the egress
//! senders, which terminates every egress task.

use std::collections::HashMap;
use std::net::{Ipv4Addr, Ipv6Addr, SocketAddr};
use std::sync::{Arc, Mutex};

use anyhow::{Context, Result, anyhow, bail};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use tokio::net::UdpSocket;
use tokio::sync::mpsc;

use crate::address::TargetAddr;
use crate::config::OutboundMode;
use crate::dns::{FakeIpPool, unmap_fake_ip};
use crate::outbound::{self, UdpEgress};
use crate::socks5;

/// Upper bound on a single UDP datagram (IPv4 total length limit).
pub(crate) const MAX_DATAGRAM: usize = 65_535;

/// Per-destination egress queue depth. Datagrams beyond this while the egress
/// is busy are dropped, matching UDP's lossy semantics rather than stalling the
/// whole association.
const EGRESS_QUEUE: usize = 128;

/// Relay datagrams between the client and remote hosts until `control` (the TCP
/// connection that requested the association) closes.
pub async fn run_associate<C>(
    mut control: C,
    relay: UdpSocket,
    mode: Arc<OutboundMode>,
    fake_ip: Option<Arc<Mutex<FakeIpPool>>>,
) -> Result<()>
where
    C: AsyncRead + Unpin,
{
    let relay = Arc::new(relay);
    let mut targets: HashMap<String, mpsc::Sender<Vec<u8>>> = HashMap::new();
    let mut buf = vec![0u8; MAX_DATAGRAM];
    let mut control_buf = [0u8; 256];

    loop {
        tokio::select! {
            res = control.read(&mut control_buf) => {
                // EOF or error on the control connection ends the association.
                // Clients don't normally send payload here; ignore any bytes.
                match res {
                    Ok(0) | Err(_) => return Ok(()),
                    Ok(_) => {}
                }
            }
            res = relay.recv_from(&mut buf) => {
                let (n, client_addr) = match res {
                    Ok(v) => v,
                    Err(_) => continue,
                };
                if let Err(err) = forward_client_datagram(&buf[..n], client_addr, &relay, &mode, &fake_ip, &mut targets) {
                    log::debug!("learn-gripe udp: dropped client datagram: {err:#}");
                }
            }
        }
    }
}

/// Parse one client datagram and hand its payload to the per-destination egress
/// task, spawning that task (Direct or proxy-tunnel) on first use.
fn forward_client_datagram(
    datagram: &[u8],
    client_addr: SocketAddr,
    relay: &Arc<UdpSocket>,
    mode: &Arc<OutboundMode>,
    fake_ip: &Option<Arc<Mutex<FakeIpPool>>>,
    targets: &mut HashMap<String, mpsc::Sender<Vec<u8>>>,
) -> Result<()> {
    let (target, offset) = socks5::parse_udp_datagram(datagram)?;
    // Resolve a fake IP back to its domain so routing sees the real host.
    let target = match fake_ip {
        Some(pool) => unmap_fake_ip(pool, target),
        None => target,
    };
    let mut payload = datagram[offset..].to_vec();
    let key = target.to_string();

    if let Some(tx) = targets.get(&key) {
        match tx.try_send(payload) {
            Ok(()) => return Ok(()),
            // Egress busy: drop this datagram (UDP is lossy) but keep the task.
            Err(mpsc::error::TrySendError::Full(_)) => return Ok(()),
            // Task has exited; rebuild it below with the same payload.
            Err(mpsc::error::TrySendError::Closed(p)) => {
                targets.remove(&key);
                payload = p;
            }
        }
    }

    let egress = outbound::resolve_udp_egress(mode, &target).ok_or_else(|| anyhow!("no UDP egress for {target}"))?;
    let (tx, rx) = mpsc::channel(EGRESS_QUEUE);
    spawn_egress(egress, target, rx, relay.clone(), client_addr);
    // The freshly built channel has capacity, so this only fails if the task
    // already died (e.g. dial failure); dropping the datagram is acceptable.
    let _ = tx.try_send(payload);
    targets.insert(key, tx);
    Ok(())
}

/// Spawn the egress task appropriate for `egress`.
fn spawn_egress(
    egress: UdpEgress,
    target: TargetAddr,
    rx: mpsc::Receiver<Vec<u8>>,
    relay: Arc<UdpSocket>,
    client_addr: SocketAddr,
) {
    match egress {
        UdpEgress::Direct => {
            tokio::spawn(async move {
                if let Err(err) = run_direct_egress(target, rx, relay, client_addr).await {
                    log::debug!("learn-gripe udp: direct egress ended: {err:#}");
                }
            });
        }
        UdpEgress::Shadowsocks(config) => {
            tokio::spawn(async move {
                if let Err(err) = run_ss_egress(config, target, rx, relay, client_addr).await {
                    log::debug!("learn-gripe udp: shadowsocks egress ended: {err:#}");
                }
            });
        }
        proxy => {
            tokio::spawn(async move {
                if let Err(err) = run_proxy_egress(proxy, target, rx, relay, client_addr).await {
                    log::debug!("learn-gripe udp: proxy egress ended: {err:#}");
                }
            });
        }
    }
}

/// Direct UDP egress: bind a socket to the destination and relay both ways
/// until the association closes (sender dropped) or a socket error occurs.
async fn run_direct_egress(
    target: TargetAddr,
    mut rx: mpsc::Receiver<Vec<u8>>,
    relay: Arc<UdpSocket>,
    client_addr: SocketAddr,
) -> Result<()> {
    let dest = resolve(&target).await?;
    let socket = bind_egress(dest).await?;
    socket
        .connect(dest)
        .await
        .with_context(|| format!("udp connect {dest}"))?;

    let mut buf = vec![0u8; MAX_DATAGRAM];
    loop {
        tokio::select! {
            maybe = rx.recv() => match maybe {
                Some(payload) => {
                    socket.send(&payload).await.with_context(|| format!("udp send to {dest}"))?;
                }
                None => return Ok(()),
            },
            res = socket.recv(&mut buf) => {
                let n = res.with_context(|| format!("udp recv from {dest}"))?;
                let packet = socks5::encode_udp_datagram(&target, &buf[..n]);
                if relay.send_to(&packet, client_addr).await.is_err() {
                    return Ok(());
                }
            }
        }
    }
}

/// Shadowsocks UDP egress: relay datagrams over a UDP socket to the Shadowsocks
/// server, sealing/opening each packet with per-packet AEAD framing.
async fn run_ss_egress(
    config: Box<crate::shadowsocks::ShadowsocksOutboundConfig>,
    target: TargetAddr,
    mut rx: mpsc::Receiver<Vec<u8>>,
    relay: Arc<UdpSocket>,
    client_addr: SocketAddr,
) -> Result<()> {
    let assoc = crate::shadowsocks::ShadowsocksUdp::connect(&config, &target).await?;
    loop {
        tokio::select! {
            maybe = rx.recv() => match maybe {
                Some(payload) => assoc.send(&payload).await?,
                None => return Ok(()),
            },
            res = assoc.recv() => {
                let payload = res?;
                let packet = socks5::encode_udp_datagram(&target, &payload);
                if relay.send_to(&packet, client_addr).await.is_err() {
                    return Ok(());
                }
            }
        }
    }
}

/// Proxy-tunnel UDP egress: open the protocol's UDP stream and relay datagrams
/// in both directions, applying the protocol's per-packet framing.
async fn run_proxy_egress(
    egress: UdpEgress,
    target: TargetAddr,
    mut rx: mpsc::Receiver<Vec<u8>>,
    relay: Arc<UdpSocket>,
    client_addr: SocketAddr,
) -> Result<()> {
    let framing = ProxyFraming::for_egress(&egress);
    let stream = outbound::connect_proxy_udp(&egress, &target).await?;
    let (mut reader, mut writer) = tokio::io::split(stream);

    loop {
        tokio::select! {
            maybe = rx.recv() => match maybe {
                Some(payload) => write_proxy_packet(&mut writer, framing, &target, &payload).await?,
                None => return Ok(()),
            },
            res = read_proxy_packet(&mut reader, framing) => {
                let payload = res?;
                let packet = socks5::encode_udp_datagram(&target, &payload);
                if relay.send_to(&packet, client_addr).await.is_err() {
                    return Ok(());
                }
            }
        }
    }
}

/// Per-packet framing applied on a proxy-tunnel UDP stream.
#[derive(Clone, Copy)]
pub(crate) enum ProxyFraming {
    /// Trojan: `SOCKS5-addr | len(2) | CRLF | payload` per packet.
    Trojan,
    /// VLESS UDP: `len(2 BE) | payload` per packet.
    LengthPrefixed,
    /// VMess UDP: one AEAD body chunk per packet (the wrapper preserves the
    /// boundary, so each read yields exactly one datagram).
    Chunked,
}

impl ProxyFraming {
    pub(crate) fn for_egress(egress: &UdpEgress) -> Self {
        match egress {
            UdpEgress::Trojan(_) => ProxyFraming::Trojan,
            UdpEgress::Vless(_) => ProxyFraming::LengthPrefixed,
            // Direct and Shadowsocks never reach a proxy egress (they relay over
            // a UDP socket); treat them as chunked defensively.
            UdpEgress::Vmess(_) | UdpEgress::Direct | UdpEgress::Shadowsocks(_) => ProxyFraming::Chunked,
        }
    }
}

pub(crate) async fn write_proxy_packet<W>(
    writer: &mut W,
    framing: ProxyFraming,
    target: &TargetAddr,
    payload: &[u8],
) -> Result<()>
where
    W: AsyncWrite + Unpin,
{
    match framing {
        ProxyFraming::Trojan => {
            writer
                .write_all(&crate::trojan::encode_udp_packet(target, payload))
                .await?;
        }
        ProxyFraming::LengthPrefixed => {
            let len = u16::try_from(payload.len()).map_err(|_| anyhow!("udp payload too large for vless framing"))?;
            let mut buf = Vec::with_capacity(2 + payload.len());
            buf.extend_from_slice(&len.to_be_bytes());
            buf.extend_from_slice(payload);
            writer.write_all(&buf).await?;
        }
        ProxyFraming::Chunked => {
            writer.write_all(payload).await?;
        }
    }
    writer.flush().await?;
    Ok(())
}

pub(crate) async fn read_proxy_packet<R>(reader: &mut R, framing: ProxyFraming) -> Result<Vec<u8>>
where
    R: AsyncRead + Unpin,
{
    match framing {
        ProxyFraming::Trojan => {
            let (_source, payload) = crate::trojan::read_udp_packet(reader).await?;
            Ok(payload)
        }
        ProxyFraming::LengthPrefixed => {
            let mut len = [0u8; 2];
            reader.read_exact(&mut len).await?;
            let mut payload = vec![0u8; u16::from_be_bytes(len) as usize];
            reader.read_exact(&mut payload).await?;
            Ok(payload)
        }
        ProxyFraming::Chunked => {
            let mut buf = vec![0u8; MAX_DATAGRAM];
            let n = reader.read(&mut buf).await?;
            if n == 0 {
                bail!("proxy udp: stream closed");
            }
            buf.truncate(n);
            Ok(buf)
        }
    }
}

/// Bind an egress socket on the unspecified address of the destination family.
pub(crate) async fn bind_egress(dest: SocketAddr) -> Result<UdpSocket> {
    let bind: SocketAddr = match dest {
        SocketAddr::V4(_) => (Ipv4Addr::UNSPECIFIED, 0).into(),
        SocketAddr::V6(_) => (Ipv6Addr::UNSPECIFIED, 0).into(),
    };
    UdpSocket::bind(bind)
        .await
        .with_context(|| format!("bind udp egress for {dest}"))
}

/// Resolve a target to a concrete socket address (first DNS answer for a
/// domain).
pub(crate) async fn resolve(target: &TargetAddr) -> Result<SocketAddr> {
    match target {
        TargetAddr::Ip(addr) => Ok(*addr),
        TargetAddr::Domain(host, port) => tokio::net::lookup_host((host.as_str(), *port))
            .await
            .with_context(|| format!("resolve udp {host}:{port}"))?
            .next()
            .ok_or_else(|| anyhow!("no address for {host}:{port}")),
    }
}
