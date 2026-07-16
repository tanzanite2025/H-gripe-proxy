//! OpenVPN packet framing, the reliability/ack control channel, and the
//! adapter that tunnels the control-channel TLS handshake over `P_CONTROL_V1`
//! messages.
//!
//! The underlying transport is either a single TCP connection (each OpenVPN
//! packet length-prefixed with a `u16` big-endian length) or a connected UDP
//! socket (each datagram carries exactly one OpenVPN packet, no length prefix).
//! A background mux task reads packets and splits them into a control queue and
//! a data queue. Control packets ride the reliability layer here (message ids +
//! acks + retransmission); data packets go straight to the device loop.
//!
//! TCP is an ordered, lossless stream, so control retransmission is a no-op
//! there. UDP is lossy/unordered, so the handshake drives
//! [`ControlChannel::retransmit_pending`] on a timer until it completes;
//! unacked reliable control packets are retained until the peer acks them.

use std::collections::HashMap;
use std::pin::Pin;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::task::{Context as TaskContext, Poll};

use anyhow::{Result, anyhow};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt, ReadBuf};
use tokio::net::UdpSocket;
use tokio::net::tcp::{OwnedReadHalf, OwnedWriteHalf};
use tokio::sync::{Mutex as AsyncMutex, mpsc};

use super::packet::{
    ControlPacket, P_ACK_V1, P_CONTROL_HARD_RESET_CLIENT_V2, P_CONTROL_V1, SessionId, has_message_id,
    parse_opcode_key_id,
};

/// Max OpenVPN packet size on the wire (the `u16` length ceiling).
const MAX_PACKET: usize = 0xffff;

/// Serialized writer over the tunnel transport; every control ack/message and
/// every encrypted data packet is written through this so the two producers
/// (control channel + device loop) never interleave bytes on the TCP stream.
pub(super) enum PacketWriter {
    /// A TCP write half; packets are length-prefixed (`u16` big-endian).
    Tcp(AsyncMutex<OwnedWriteHalf>),
    /// A connected UDP socket; each packet is sent as one datagram.
    Udp(Arc<UdpSocket>),
}

impl PacketWriter {
    pub(super) fn tcp(write_half: OwnedWriteHalf) -> Self {
        Self::Tcp(AsyncMutex::new(write_half))
    }

    pub(super) fn udp(socket: Arc<UdpSocket>) -> Self {
        Self::Udp(socket)
    }

    /// Write one OpenVPN packet to the transport (length-prefixed on TCP, one
    /// datagram on UDP).
    pub(super) async fn write_packet(&self, packet: &[u8]) -> Result<()> {
        if packet.len() > MAX_PACKET {
            return Err(anyhow!("openvpn: packet too large: {}", packet.len()));
        }
        match self {
            Self::Tcp(inner) => {
                let mut frame = Vec::with_capacity(2 + packet.len());
                frame.extend_from_slice(&(packet.len() as u16).to_be_bytes());
                frame.extend_from_slice(packet);
                let mut guard = inner.lock().await;
                guard.write_all(&frame).await?;
            }
            Self::Udp(socket) => {
                socket.send(packet).await?;
            }
        }
        Ok(())
    }
}

/// Read length-prefixed packets from `read_half`, dispatching control packets to
/// `control_tx` and data packets to `data_tx`. Exits (dropping both senders) on
/// EOF or error, which tears down the dependent channels/loops.
pub(super) async fn run_mux(
    mut read_half: OwnedReadHalf,
    control_tx: mpsc::UnboundedSender<Vec<u8>>,
    data_tx: mpsc::UnboundedSender<Vec<u8>>,
) {
    loop {
        let mut len_buf = [0u8; 2];
        if read_half.read_exact(&mut len_buf).await.is_err() {
            return;
        }
        let size = u16::from_be_bytes(len_buf) as usize;
        if size == 0 {
            // A zero-length frame is a framing violation (every OpenVPN packet
            // has at least an opcode byte); tear the transport down.
            return;
        }
        let mut packet = vec![0u8; size];
        if read_half.read_exact(&mut packet).await.is_err() {
            return;
        }
        let (opcode, _) = parse_opcode_key_id(packet[0]);
        let sender = if super::packet::is_control(opcode) {
            &control_tx
        } else {
            &data_tx
        };
        if sender.send(packet).is_err() {
            return;
        }
    }
}

/// Read datagrams from a connected UDP `socket`, dispatching each (one OpenVPN
/// packet per datagram, no length prefix) to `control_tx` or `data_tx`. Exits
/// (dropping both senders) on socket error, tearing down dependent channels.
pub(super) async fn run_mux_udp(
    socket: Arc<UdpSocket>,
    control_tx: mpsc::UnboundedSender<Vec<u8>>,
    data_tx: mpsc::UnboundedSender<Vec<u8>>,
) {
    let mut buf = vec![0u8; MAX_PACKET];
    loop {
        let n = match socket.recv(&mut buf).await {
            Ok(n) => n,
            Err(_) => return,
        };
        if n == 0 {
            continue;
        }
        let packet = buf[..n].to_vec();
        let (opcode, _) = parse_opcode_key_id(packet[0]);
        let sender = if super::packet::is_control(opcode) {
            &control_tx
        } else {
            &data_tx
        };
        if sender.send(packet).is_err() {
            return;
        }
    }
}

struct ControlState {
    send_message: u32,
    recv_message: u32,
    ack_pending: Vec<u32>,
    recv_pending: HashMap<u32, ControlPacket>,
    /// Reliable control packets we sent but have not yet seen acked, kept for
    /// retransmission over a lossy (UDP) transport. Removed when the peer acks
    /// their message id.
    unacked: HashMap<u32, ControlPacket>,
    remote: SessionId,
}

/// The OpenVPN control channel: a reliable, acked, ordered message stream layered
/// over the raw control-packet queue.
pub(super) struct ControlChannel {
    writer: Arc<PacketWriter>,
    local: SessionId,
    key_id: u8,
    state: std::sync::Mutex<ControlState>,
    rx: AsyncMutex<mpsc::UnboundedReceiver<Vec<u8>>>,
}

impl ControlChannel {
    pub(super) fn new(writer: Arc<PacketWriter>, local: SessionId, rx: mpsc::UnboundedReceiver<Vec<u8>>) -> Self {
        Self {
            writer,
            local,
            key_id: 0,
            state: std::sync::Mutex::new(ControlState {
                send_message: 0,
                recv_message: 0,
                ack_pending: Vec::new(),
                recv_pending: HashMap::new(),
                unacked: HashMap::new(),
                remote: [0u8; 8],
            }),
            rx: AsyncMutex::new(rx),
        }
    }

    pub(super) fn local_session(&self) -> SessionId {
        self.local
    }

    pub(super) fn remote_session(&self) -> SessionId {
        self.state.lock().expect("openvpn control state").remote
    }

    /// Send the initial client hard reset.
    pub(super) async fn send_reset(&self) -> Result<()> {
        self.send(P_CONTROL_HARD_RESET_CLIENT_V2, &[]).await.map(|_| ())
    }

    /// Send a reliable control message, folding any pending acks into it.
    pub(super) async fn send(&self, opcode: u8, payload: &[u8]) -> Result<u32> {
        let (message_id, packet) = {
            let mut st = self.state.lock().expect("openvpn control state");
            let message_id = st.send_message;
            st.send_message += 1;
            let ack_ids = std::mem::take(&mut st.ack_pending);
            let remote = st.remote;
            let packet = ControlPacket {
                opcode,
                key_id: self.key_id,
                local_session: self.local,
                ack_ids,
                ack_remote_session: remote,
                message_id,
                payload: payload.to_vec(),
            };
            st.unacked.insert(message_id, packet.clone());
            (message_id, packet)
        };
        self.writer.write_packet(&packet.encode()?).await?;
        Ok(message_id)
    }

    /// Resend every reliable control packet the peer has not yet acked, folding
    /// in any acks we currently owe. Called on a timer during the handshake on
    /// UDP transports; a no-op once everything has been acked.
    pub(super) async fn retransmit_pending(&self) -> Result<()> {
        let packets = {
            let mut st = self.state.lock().expect("openvpn control state");
            if st.unacked.is_empty() {
                return Ok(());
            }
            let ack_ids = std::mem::take(&mut st.ack_pending);
            let remote = st.remote;
            let mut packets: Vec<ControlPacket> = st.unacked.values().cloned().collect();
            for packet in &mut packets {
                packet.ack_ids = ack_ids.clone();
                packet.ack_remote_session = remote;
            }
            packets
        };
        for packet in &packets {
            self.writer.write_packet(&packet.encode()?).await?;
        }
        Ok(())
    }

    /// Flush any pending acks as a bare `P_ACK_V1` packet.
    pub(super) async fn send_ack(&self) -> Result<()> {
        let packet = {
            let mut st = self.state.lock().expect("openvpn control state");
            if st.ack_pending.is_empty() {
                return Ok(());
            }
            let ack_ids = std::mem::take(&mut st.ack_pending);
            ControlPacket {
                opcode: P_ACK_V1,
                key_id: self.key_id,
                local_session: self.local,
                ack_ids,
                ack_remote_session: st.remote,
                message_id: 0,
                payload: Vec::new(),
            }
        };
        self.writer.write_packet(&packet.encode()?).await
    }

    /// Read the next in-order reliable control message, buffering/acking
    /// out-of-order and duplicate packets. Bare acks are consumed silently.
    pub(super) async fn read(&self) -> Result<ControlPacket> {
        loop {
            {
                let mut st = self.state.lock().expect("openvpn control state");
                let next = st.recv_message;
                if let Some(packet) = st.recv_pending.remove(&next) {
                    st.recv_message += 1;
                    return Ok(packet);
                }
            }

            let raw = {
                let mut rx = self.rx.lock().await;
                rx.recv()
                    .await
                    .ok_or_else(|| anyhow!("openvpn: control channel closed"))?
            };
            let packet = ControlPacket::decode(&raw)?;

            let mut deliver: Option<ControlPacket> = None;
            let mut send_ack = false;
            {
                let mut st = self.state.lock().expect("openvpn control state");
                if st.remote == [0u8; 8] && packet.local_session != self.local {
                    st.remote = packet.local_session;
                }
                for ack in &packet.ack_ids {
                    st.unacked.remove(ack);
                }
                if has_message_id(packet.opcode) {
                    let id = packet.message_id;
                    if !st.ack_pending.contains(&id) {
                        st.ack_pending.push(id);
                    }
                }

                if packet.opcode == P_ACK_V1 {
                    // consumed silently
                } else if !has_message_id(packet.opcode) {
                    deliver = Some(packet);
                } else if packet.message_id < st.recv_message {
                    send_ack = true;
                } else if packet.message_id == st.recv_message {
                    st.recv_message += 1;
                    deliver = Some(packet);
                } else {
                    st.recv_pending.entry(packet.message_id).or_insert(packet);
                    send_ack = true;
                }
            }

            if let Some(packet) = deliver {
                return Ok(packet);
            }
            if send_ack {
                self.send_ack().await?;
            }
        }
    }
}

/// A rustls transport that tunnels the TLS byte stream over the OpenVPN control
/// channel: writes become `P_CONTROL_V1` payloads, reads deliver payloads
/// received from the peer.
pub(super) struct ControlTlsIo {
    inbound_rx: mpsc::UnboundedReceiver<Vec<u8>>,
    leftover: Vec<u8>,
    leftover_pos: usize,
    outbound_tx: Option<mpsc::UnboundedSender<Vec<u8>>>,
}

impl AsyncRead for ControlTlsIo {
    fn poll_read(self: Pin<&mut Self>, cx: &mut TaskContext<'_>, buf: &mut ReadBuf<'_>) -> Poll<std::io::Result<()>> {
        let this = self.get_mut();
        loop {
            if this.leftover_pos < this.leftover.len() {
                let n = buf.remaining().min(this.leftover.len() - this.leftover_pos);
                buf.put_slice(&this.leftover[this.leftover_pos..this.leftover_pos + n]);
                this.leftover_pos += n;
                return Poll::Ready(Ok(()));
            }
            match this.inbound_rx.poll_recv(cx) {
                Poll::Ready(Some(data)) => {
                    this.leftover = data;
                    this.leftover_pos = 0;
                }
                Poll::Ready(None) => return Poll::Ready(Ok(())),
                Poll::Pending => return Poll::Pending,
            }
        }
    }
}

impl AsyncWrite for ControlTlsIo {
    fn poll_write(self: Pin<&mut Self>, _cx: &mut TaskContext<'_>, buf: &[u8]) -> Poll<std::io::Result<usize>> {
        let this = self.get_mut();
        let Some(tx) = &this.outbound_tx else {
            return Poll::Ready(Err(std::io::ErrorKind::BrokenPipe.into()));
        };
        match tx.send(buf.to_vec()) {
            Ok(()) => Poll::Ready(Ok(buf.len())),
            Err(_) => Poll::Ready(Err(std::io::ErrorKind::BrokenPipe.into())),
        }
    }

    fn poll_flush(self: Pin<&mut Self>, _cx: &mut TaskContext<'_>) -> Poll<std::io::Result<()>> {
        Poll::Ready(Ok(()))
    }

    fn poll_shutdown(self: Pin<&mut Self>, _cx: &mut TaskContext<'_>) -> Poll<std::io::Result<()>> {
        self.get_mut().outbound_tx = None;
        Poll::Ready(Ok(()))
    }
}

/// Spawn the two bridge tasks connecting `control` to a [`ControlTlsIo`]:
/// outbound TLS bytes become control messages, inbound control payloads become
/// TLS bytes. Once `handshake_done` is set the inbound task stops forwarding
/// payloads but keeps reading + acking control packets so the mux never stalls
/// (there is no control-channel renegotiation in this slice).
pub(super) fn spawn_tls_bridge(control: Arc<ControlChannel>, handshake_done: Arc<AtomicBool>) -> ControlTlsIo {
    let (in_tx, in_rx) = mpsc::unbounded_channel::<Vec<u8>>();
    let (out_tx, mut out_rx) = mpsc::unbounded_channel::<Vec<u8>>();

    let outbound = control.clone();
    tokio::spawn(async move {
        while let Some(bytes) = out_rx.recv().await {
            if outbound.send(P_CONTROL_V1, &bytes).await.is_err() {
                break;
            }
        }
    });

    tokio::spawn(async move {
        loop {
            let packet = match control.read().await {
                Ok(packet) => packet,
                Err(_) => break,
            };
            if control.send_ack().await.is_err() {
                break;
            }
            if packet.opcode == P_CONTROL_V1 && !handshake_done.load(Ordering::Relaxed) && !packet.payload.is_empty() {
                let _ = in_tx.send(packet.payload);
            }
        }
    });

    ControlTlsIo {
        inbound_rx: in_rx,
        leftover: Vec::new(),
        leftover_pos: 0,
        outbound_tx: Some(out_tx),
    }
}
