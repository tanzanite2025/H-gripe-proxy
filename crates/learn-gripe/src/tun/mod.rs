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
//!
//! The module is split by concern: this file owns the poll loop and the device
//! adapter; [`device`] is the in-memory smoltcp [`Device`](smoltcp::phy::Device);
//! [`tcp`] bridges accepted TCP flows; [`udp`] terminates UDP (NAT relay);
//! [`dns`] answers in-stack DNS; [`wire`] is the IP/UDP frame codec.

mod device;
mod dns;
mod tcp;
mod udp;
mod wire;

use crate::config::OutboundMode;
use crate::dns::{DnsMode, FakeIpPool};

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration as StdDuration, Instant as StdInstant};

use smoltcp::iface::SocketSet;
use smoltcp::wire::IpEndpoint;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use tokio::sync::{Notify, mpsc};

use device::{TunPhy, build_interface, smol_now};
use tcp::{Flow, drain_tx, new_flow_for_syn, service_flows};
use udp::handle_udp;

/// Default MTU for the userspace stack. The OS TUN device should be configured
/// with the same value.
pub const DEFAULT_MTU: usize = 1500;

/// Upper bound on how long the poll loop sleeps between wakeups.
const MAX_POLL_SLEEP: StdDuration = StdDuration::from_millis(50);

/// Number of frames buffered between the device pump and the userspace stack.
const DEVICE_QUEUE_DEPTH: usize = 256;

/// Identifies a flow / NAT session by its (source, destination) endpoints.
type FlowKey = (IpEndpoint, IpEndpoint);

/// The fake-IP pool shared with the DNS server: TCP/UDP destinations that are
/// synthesized IPs get unmapped back to the domain DNS just handed out.
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
