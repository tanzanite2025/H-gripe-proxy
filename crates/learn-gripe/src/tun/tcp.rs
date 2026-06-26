//! TCP flow handling: accept SYNs into smoltcp listening sockets, bridge each
//! flow to the outbound pipeline, and reap closed flows.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;

use smoltcp::iface::{SocketHandle, SocketSet};
use smoltcp::socket::tcp;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::sync::{Notify, mpsc};

use crate::address::TargetAddr;
use crate::config::OutboundMode;
use crate::dns::unmap_fake_ip;
use crate::outbound;

use super::device::TunPhy;
use super::wire::{endpoint_socketaddr, parse_tcp_endpoints};
use super::{FakeIp, FlowKey};

/// Per-flow socket buffer size (each direction).
const FLOW_BUFFER: usize = 64 * 1024;
/// Bounded depth of the per-flow bridge channels (in frames/chunks).
const CHANNEL_DEPTH: usize = 64;

/// Bridge state for one accepted TCP flow, owned by the poll loop.
pub(super) struct Flow {
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
pub(super) fn service_flows(sockets: &mut SocketSet, flows: &mut HashMap<FlowKey, Flow>) {
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
pub(super) fn drain_tx(phy: &mut TunPhy, frames_out: &mpsc::Sender<Vec<u8>>) {
    while let Some(frame) = phy.tx.pop_front() {
        if frames_out.try_send(frame).is_err() {
            break;
        }
    }
}

/// If `frame` is a TCP SYN for an unseen flow, create a listening socket on its
/// destination and spawn the outbound bridge task.
pub(super) fn new_flow_for_syn(
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
