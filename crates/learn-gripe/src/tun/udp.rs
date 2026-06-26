//! UDP over TUN: terminate each datagram in-stack. DNS queries go to [`dns`],
//! everything else is relayed through the outbound pipeline via a per-5-tuple
//! NAT session whose replies are rewritten back into IP frames.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration as StdDuration;

use smoltcp::wire::{IpAddress, IpEndpoint};
use tokio::sync::mpsc;

use crate::address::TargetAddr;
use crate::config::OutboundMode;
use crate::dns::{DnsMode, unmap_fake_ip};
use crate::outbound::{self, UdpEgress};
use crate::udp;

use super::dns::answer_dns;
use super::wire::{build_udp_reply_frame, endpoint_socketaddr, parse_udp_datagram};
use super::{FakeIp, FlowKey};

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
pub(super) struct UdpFlow {
    pub(super) src_addr: IpAddress,
    pub(super) dst_addr: IpAddress,
    pub(super) src_port: u16,
    pub(super) dst_port: u16,
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
pub(super) struct UdpDatagram {
    pub(super) flow: UdpFlow,
    pub(super) payload: Vec<u8>,
}

/// Terminate a UDP datagram in-stack: DNS queries (port [`DNS_PORT`]) with a
/// configured [`DnsMode`] are answered from the kernel DNS logic; every other
/// UDP datagram is relayed through the outbound pipeline via a NAT session.
/// Returns `true` when `frame` was UDP (consumed here, never fed to the TCP
/// stack); `false` for non-UDP frames, which the caller handles normally.
pub(super) fn handle_udp(
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
/// The relay loop, transports, and framing are shared with the SOCKS5 inbound
/// via [`udp::run_egress`]; the TUN side only differs in how replies are
/// delivered ([`TunSink`]) and that idle sessions are reaped (UDP has no close).
async fn run_udp_session(
    egress: UdpEgress,
    target: TargetAddr,
    reply: UdpFlow,
    rx: mpsc::Receiver<Vec<u8>>,
    frames_out: mpsc::Sender<Vec<u8>>,
) {
    let sink = TunSink { reply, frames_out };
    if let Err(err) = udp::run_egress(egress, target, rx, sink, Some(UDP_IDLE_TIMEOUT)).await {
        log::debug!("learn-gripe tun udp: session ended: {err:#}");
    }
}

/// [`udp::ReplySink`] for the TUN stack: rewrite each reply into an IP+UDP frame
/// (sourced from the host the client targeted) and push it onto the device's
/// outbound frame queue.
struct TunSink {
    reply: UdpFlow,
    frames_out: mpsc::Sender<Vec<u8>>,
}

impl udp::ReplySink for TunSink {
    async fn deliver(&self, payload: &[u8]) -> bool {
        match build_udp_reply_frame(&self.reply, payload) {
            Some(frame) => self.frames_out.send(frame).await.is_ok(),
            // Unbuildable frame (e.g. mixed address families): drop it but keep
            // the session alive, matching the original per-egress relays.
            None => true,
        }
    }
}
