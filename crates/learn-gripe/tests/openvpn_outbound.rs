//! End-to-end proof that traffic flows through an OpenVPN outbound.
//!
//! The kernel's OpenVPN outbound (`protocols::openvpn`) is exercised against an
//! **independent** fake OpenVPN server built here from scratch: it speaks the
//! packet framing (length-prefixed on TCP, one datagram per packet on UDP), the
//! reliable hard-reset / `P_CONTROL_V1` control channel, a real TLS 1.x
//! handshake tunnelled over control messages (rustls server), the key-method-2
//! exchange + OpenVPN PRF key derivation, the `PUSH_REQUEST`/`PUSH_REPLY` step,
//! and finally an AES-256-GCM `P_DATA_V2` data channel feeding a second smoltcp
//! stack that terminates the inner TCP and echoes. This proves the full path:
//! client smoltcp SYN -> AEAD -> transport -> server decrypt -> server smoltcp
//! accept/echo -> AEAD -> transport -> client decrypt -> client smoltcp data.
//!
//! Both TCP and UDP transports are covered; the UDP server additionally acks
//! client control packets and, in one test, drops the client's first hard reset
//! to prove the client's control-channel retransmission recovers over a lossy
//! datagram transport.
//!
//! The server crypto/codec is written independently of the crate under test so
//! the test proves genuine interop rather than that the code agrees with itself.

use std::collections::{HashSet, VecDeque};
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::pin::Pin;
use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};
use std::task::{Context as TaskContext, Poll};
use std::time::{Duration, Instant};

use aes_gcm::Aes256Gcm;
use aes_gcm::aead::{Aead, KeyInit, Payload};
use hmac::{Hmac, Mac};
use learn_gripe::{OpenVpnOutboundConfig, ProxyEntry, TargetAddr, openvpn};
use md5::Md5;
use sha1::Sha1;
use smoltcp::iface::{Config as IfaceConfig, Interface, SocketSet};
use smoltcp::phy::{Device, DeviceCapabilities, Medium, RxToken, TxToken};
use smoltcp::socket::{tcp, udp};
use smoltcp::time::Instant as SmolInstant;
use smoltcp::wire::{HardwareAddress, IpAddress, IpCidr, Ipv4Address};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt, ReadBuf};
use tokio::net::tcp::OwnedWriteHalf;
use tokio::net::{TcpListener, UdpSocket};
use tokio::sync::{Mutex, mpsc};
use tokio_rustls::TlsAcceptor;

/// The CA the client pins the control channel to (its inline `ca`).
const TEST_CA: &str = include_str!("data/openvpn_ca.pem");
/// The server leaf certificate + key, signed by `TEST_CA`.
const TEST_CERT: &str = include_str!("data/openvpn_server_cert.pem");
const TEST_KEY: &str = include_str!("data/openvpn_server_key.pem");

/// Tunnel address the fake server assigns the client via `PUSH_REPLY ifconfig`.
const CLIENT_TUN_IP: Ipv4Addr = Ipv4Addr::new(10, 8, 0, 2);
/// Inner target the relayed TCP connection dials; the server stack accepts it
/// via `any_ip`.
const INNER_IP: Ipv4Addr = Ipv4Addr::new(10, 8, 0, 88);
const INNER_PORT: u16 = 9000;
/// Data-channel peer id the server pushes.
const PEER_ID: u32 = 5;
const MTU: usize = 1600;

// --- OpenVPN opcodes / framing (independent copy of the wire format) ----------

const OPCODE_SHIFT: u8 = 3;
const KEY_ID_MASK: u8 = 0x07;
const P_CONTROL_HARD_RESET_CLIENT_V2: u8 = 7;
const P_CONTROL_HARD_RESET_SERVER_V2: u8 = 8;
const P_CONTROL_V1: u8 = 4;
const P_ACK_V1: u8 = 5;
const P_DATA_V2: u8 = 9;

fn opcode_of(b: u8) -> u8 {
    b >> OPCODE_SHIFT
}

fn opcode_key_id(opcode: u8, key_id: u8) -> u8 {
    (opcode << OPCODE_SHIFT) | (key_id & KEY_ID_MASK)
}

#[derive(Clone)]
struct ControlPacket {
    opcode: u8,
    local_session: [u8; 8],
    ack_ids: Vec<u32>,
    ack_remote_session: [u8; 8],
    message_id: u32,
    payload: Vec<u8>,
}

impl ControlPacket {
    fn decode(packet: &[u8]) -> Self {
        let opcode = opcode_of(packet[0]);
        let mut local_session = [0u8; 8];
        local_session.copy_from_slice(&packet[1..9]);
        let body = &packet[9..];
        let ack_len = body[0] as usize;
        let mut offset = 1;
        let mut ack_ids = Vec::with_capacity(ack_len);
        for _ in 0..ack_len {
            ack_ids.push(u32::from_be_bytes(body[offset..offset + 4].try_into().unwrap()));
            offset += 4;
        }
        let mut ack_remote_session = [0u8; 8];
        if ack_len > 0 {
            ack_remote_session.copy_from_slice(&body[offset..offset + 8]);
            offset += 8;
        }
        let (message_id, payload) = if opcode != P_ACK_V1 {
            let id = u32::from_be_bytes(body[offset..offset + 4].try_into().unwrap());
            offset += 4;
            (id, body[offset..].to_vec())
        } else {
            (0u32, Vec::new())
        };
        let _ = offset;
        Self {
            opcode,
            local_session,
            ack_ids,
            ack_remote_session,
            message_id,
            payload,
        }
    }

    fn encode(&self) -> Vec<u8> {
        let mut out = Vec::new();
        out.push(opcode_key_id(self.opcode, 0));
        out.extend_from_slice(&self.local_session);
        out.push(self.ack_ids.len() as u8);
        for id in &self.ack_ids {
            out.extend_from_slice(&id.to_be_bytes());
        }
        if !self.ack_ids.is_empty() {
            out.extend_from_slice(&self.ack_remote_session);
        }
        if self.opcode != P_ACK_V1 {
            out.extend_from_slice(&self.message_id.to_be_bytes());
            out.extend_from_slice(&self.payload);
        }
        out
    }
}

/// The fake server's outbound transport: a TCP write half (length-prefixed) or
/// a connected UDP socket (one datagram per packet).
#[derive(Clone)]
enum Out {
    Tcp(Arc<Mutex<OwnedWriteHalf>>),
    Udp(Arc<UdpSocket>),
}

impl Out {
    /// Write one OpenVPN packet to the transport.
    async fn send(&self, packet: &[u8]) {
        match self {
            Out::Tcp(writer) => {
                let mut frame = Vec::with_capacity(2 + packet.len());
                frame.extend_from_slice(&(packet.len() as u16).to_be_bytes());
                frame.extend_from_slice(packet);
                let mut guard = writer.lock().await;
                let _ = guard.write_all(&frame).await;
                let _ = guard.flush().await;
            }
            Out::Udp(socket) => {
                let _ = socket.send(packet).await;
            }
        }
    }
}

// --- OpenVPN PRF + key derivation (independent copy) --------------------------

fn p_hash<M: Mac + KeyInit>(secret: &[u8], seed: &[u8], size: usize) -> Vec<u8> {
    let mac_of = |data: &[u8]| -> Vec<u8> {
        let mut mac = <M as KeyInit>::new_from_slice(secret).unwrap();
        mac.update(data);
        mac.finalize().into_bytes().to_vec()
    };
    let mut out = Vec::with_capacity(size);
    let mut a = mac_of(seed);
    while out.len() < size {
        let mut input = a.clone();
        input.extend_from_slice(seed);
        out.extend_from_slice(&mac_of(&input));
        a = mac_of(&a);
    }
    out.truncate(size);
    out
}

fn openvpn_prf(secret: &[u8], label: &str, seed: &[u8], out: &mut [u8]) {
    let mut full = Vec::new();
    full.extend_from_slice(label.as_bytes());
    full.extend_from_slice(seed);
    let split = secret.len().div_ceil(2);
    let s1 = &secret[..split];
    let s2 = &secret[secret.len() - split..];
    let md5 = p_hash::<Hmac<Md5>>(s1, &full, out.len());
    let sha1 = p_hash::<Hmac<Sha1>>(s2, &full, out.len());
    for (i, b) in out.iter_mut().enumerate() {
        *b = md5[i] ^ sha1[i];
    }
}

/// Directional AES-256-GCM material derived on the server side (send/recv are
/// the mirror of the client's).
struct ServerKeys {
    send_cipher: Aes256Gcm,
    recv_cipher: Aes256Gcm,
    send_iv: [u8; 12],
    recv_iv: [u8; 12],
}

#[allow(clippy::too_many_arguments)]
fn derive_server_keys(
    client_pre_master: &[u8],
    client_r1: &[u8],
    client_r2: &[u8],
    server_r1: &[u8],
    server_r2: &[u8],
    client_session: [u8; 8],
    server_session: [u8; 8],
) -> ServerKeys {
    let mut master = [0u8; 48];
    let mut seed = Vec::new();
    seed.extend_from_slice(client_r1);
    seed.extend_from_slice(server_r1);
    openvpn_prf(client_pre_master, "OpenVPN master secret", &seed, &mut master);

    let mut seed2 = Vec::new();
    seed2.extend_from_slice(client_r2);
    seed2.extend_from_slice(server_r2);
    seed2.extend_from_slice(&client_session);
    seed2.extend_from_slice(&server_session);
    let mut key_block = vec![0u8; 2 * (64 + 64)];
    openvpn_prf(&master, "OpenVPN key expansion", &seed2, &mut key_block);

    let client_to_server = &key_block[..128];
    let server_to_client = &key_block[128..];
    // The server receives on the client's send keys and vice versa.
    let recv_cipher = Aes256Gcm::new_from_slice(&client_to_server[..32]).unwrap();
    let send_cipher = Aes256Gcm::new_from_slice(&server_to_client[..32]).unwrap();
    let mut recv_iv = [0u8; 12];
    let mut send_iv = [0u8; 12];
    recv_iv[4..].copy_from_slice(&client_to_server[64..72]);
    send_iv[4..].copy_from_slice(&server_to_client[64..72]);
    ServerKeys {
        send_cipher,
        recv_cipher,
        send_iv,
        recv_iv,
    }
}

fn data_header(peer_id: u32) -> [u8; 4] {
    [
        opcode_key_id(P_DATA_V2, 0),
        (peer_id >> 16) as u8,
        (peer_id >> 8) as u8,
        peer_id as u8,
    ]
}

fn nonce(iv: &[u8; 12], packet_id: u32) -> [u8; 12] {
    let mut n = *iv;
    let head = u32::from_be_bytes(n[..4].try_into().unwrap()) ^ packet_id;
    n[..4].copy_from_slice(&head.to_be_bytes());
    n
}

impl ServerKeys {
    fn seal(&self, packet_id: u32, plaintext: &[u8]) -> Vec<u8> {
        let header = data_header(PEER_ID);
        let pid = packet_id.to_be_bytes();
        let mut aad = Vec::new();
        aad.extend_from_slice(&header);
        aad.extend_from_slice(&pid);
        let sealed = self
            .send_cipher
            .encrypt(
                (&nonce(&self.send_iv, packet_id)).into(),
                Payload {
                    msg: plaintext,
                    aad: &aad,
                },
            )
            .unwrap();
        let (ct, tag) = sealed.split_at(sealed.len() - 16);
        let mut out = Vec::new();
        out.extend_from_slice(&header);
        out.extend_from_slice(&pid);
        out.extend_from_slice(tag);
        out.extend_from_slice(ct);
        out
    }

    fn open(&self, packet: &[u8]) -> Option<Vec<u8>> {
        let header = &packet[..4];
        let pid = &packet[4..8];
        let tag = &packet[8..24];
        let ct = &packet[24..];
        let packet_id = u32::from_be_bytes(pid.try_into().unwrap());
        let mut combined = Vec::from(ct);
        combined.extend_from_slice(tag);
        let mut aad = Vec::new();
        aad.extend_from_slice(header);
        aad.extend_from_slice(pid);
        self.recv_cipher
            .decrypt(
                (&nonce(&self.recv_iv, packet_id)).into(),
                Payload {
                    msg: &combined,
                    aad: &aad,
                },
            )
            .ok()
    }
}

// --- TLS-over-control IO adapter (server side) --------------------------------

/// A byte stream that rustls (server) runs over: inbound `P_CONTROL_V1` payloads
/// are read as bytes, written bytes become outbound `P_CONTROL_V1` payloads.
struct ControlIo {
    inbound_rx: mpsc::UnboundedReceiver<Vec<u8>>,
    leftover: Vec<u8>,
    leftover_pos: usize,
    outbound_tx: mpsc::UnboundedSender<Vec<u8>>,
}

impl AsyncRead for ControlIo {
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

impl AsyncWrite for ControlIo {
    fn poll_write(self: Pin<&mut Self>, _cx: &mut TaskContext<'_>, buf: &[u8]) -> Poll<std::io::Result<usize>> {
        let this = self.get_mut();
        match this.outbound_tx.send(buf.to_vec()) {
            Ok(()) => Poll::Ready(Ok(buf.len())),
            Err(_) => Poll::Ready(Err(std::io::ErrorKind::BrokenPipe.into())),
        }
    }
    fn poll_flush(self: Pin<&mut Self>, _cx: &mut TaskContext<'_>) -> Poll<std::io::Result<()>> {
        Poll::Ready(Ok(()))
    }
    fn poll_shutdown(self: Pin<&mut Self>, _cx: &mut TaskContext<'_>) -> Poll<std::io::Result<()>> {
        Poll::Ready(Ok(()))
    }
}

// --- in-memory smoltcp device for the fake server -----------------------------

struct Phy {
    rx: VecDeque<Vec<u8>>,
    tx: VecDeque<Vec<u8>>,
}
struct PhyRx {
    buf: Vec<u8>,
}
struct PhyTx<'a> {
    tx: &'a mut VecDeque<Vec<u8>>,
}
impl Device for Phy {
    type RxToken<'a> = PhyRx;
    type TxToken<'a> = PhyTx<'a>;
    fn receive(&mut self, _t: SmolInstant) -> Option<(Self::RxToken<'_>, Self::TxToken<'_>)> {
        let buf = self.rx.pop_front()?;
        Some((PhyRx { buf }, PhyTx { tx: &mut self.tx }))
    }
    fn transmit(&mut self, _t: SmolInstant) -> Option<Self::TxToken<'_>> {
        Some(PhyTx { tx: &mut self.tx })
    }
    fn capabilities(&self) -> DeviceCapabilities {
        let mut caps = DeviceCapabilities::default();
        caps.medium = Medium::Ip;
        caps.max_transmission_unit = MTU;
        caps
    }
}
impl RxToken for PhyRx {
    fn consume<R, F: FnOnce(&[u8]) -> R>(self, f: F) -> R {
        f(&self.buf)
    }
}
impl TxToken for PhyTx<'_> {
    fn consume<R, F: FnOnce(&mut [u8]) -> R>(self, len: usize, f: F) -> R {
        let mut buf = vec![0u8; len];
        let r = f(&mut buf);
        self.tx.push_back(buf);
        r
    }
}

fn now_since(start: Instant) -> SmolInstant {
    SmolInstant::from_micros(start.elapsed().as_micros() as i64)
}

// --- the fake OpenVPN server --------------------------------------------------

fn tls_acceptor() -> TlsAcceptor {
    let certs = rustls_pemfile::certs(&mut TEST_CERT.as_bytes())
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    let key = rustls_pemfile::private_key(&mut TEST_KEY.as_bytes()).unwrap().unwrap();
    let provider = Arc::new(rustls::crypto::ring::default_provider());
    let config = rustls::ServerConfig::builder_with_provider(provider)
        .with_safe_default_protocol_versions()
        .unwrap()
        .with_no_client_auth()
        .with_single_cert(certs, key)
        .unwrap();
    TlsAcceptor::from(Arc::new(config))
}

/// Read one full key-method-2 record (client) from the TLS stream: fixed prefix
/// + four `u16`-length-prefixed strings. Returns (pre_master, r1, r2).
async fn read_client_key_method<S>(tls: &mut S) -> ([u8; 48], [u8; 32], [u8; 32])
where
    S: AsyncRead + Unpin,
{
    let mut buf = Vec::new();
    let mut chunk = [0u8; 4096];
    loop {
        // Fixed part: 4 + 1 + 48 + 32 + 32 = 117 bytes, then 4 strings.
        if buf.len() >= 117 {
            let mut offset = 117;
            let mut ok = true;
            for _ in 0..4 {
                if buf.len() < offset + 2 {
                    ok = false;
                    break;
                }
                let len = u16::from_be_bytes(buf[offset..offset + 2].try_into().unwrap()) as usize;
                offset += 2;
                if buf.len() < offset + len {
                    ok = false;
                    break;
                }
                offset += len;
            }
            if ok {
                assert_eq!(&buf[..4], &[0, 0, 0, 0], "key-method-2 zero prefix");
                assert_eq!(buf[4] & 0x0f, 2, "key-method 2");
                let mut pre = [0u8; 48];
                let mut r1 = [0u8; 32];
                let mut r2 = [0u8; 32];
                pre.copy_from_slice(&buf[5..53]);
                r1.copy_from_slice(&buf[53..85]);
                r2.copy_from_slice(&buf[85..117]);
                return (pre, r1, r2);
            }
        }
        let n = tls.read(&mut chunk).await.expect("read client key method");
        assert_ne!(n, 0, "TLS closed during key method");
        buf.extend_from_slice(&chunk[..n]);
    }
}

fn server_key_method_record(server_r1: &[u8; 32], server_r2: &[u8; 32]) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(&0u32.to_be_bytes());
    out.push(2); // key method 2
    out.extend_from_slice(server_r1);
    out.extend_from_slice(server_r2);
    // options string + three empty strings (username/password/peer-info).
    let options = b"V4,dev-type tun,link-mtu 1550,tun-mtu 1500,cipher AES-256-GCM,key-method 2,tls-server\0";
    out.extend_from_slice(&(options.len() as u16).to_be_bytes());
    out.extend_from_slice(options);
    for _ in 0..3 {
        out.extend_from_slice(&0u16.to_be_bytes());
    }
    out
}

/// Serve one fake OpenVPN client connection end to end.
async fn serve_openvpn(tcp: tokio::net::TcpStream) {
    let _ = tcp.set_nodelay(true);
    let (mut read_half, write_half) = tcp.into_split();
    let out = Out::Tcp(Arc::new(Mutex::new(write_half)));

    // Server session id + monotonic control message-id counter.
    let mut server_session = [0u8; 8];
    getrandom::fill(&mut server_session).unwrap();
    let send_msg = Arc::new(AtomicU32::new(0));

    // 1. Hard-reset exchange. First framed packet must be the client reset.
    let first = read_one_frame(&mut read_half).await.expect("client hard reset");
    let reset = ControlPacket::decode(&first);
    assert_eq!(
        reset.opcode, P_CONTROL_HARD_RESET_CLIENT_V2,
        "first packet is client reset"
    );
    let client_session = reset.local_session;

    let server_reset = ControlPacket {
        opcode: P_CONTROL_HARD_RESET_SERVER_V2,
        local_session: server_session,
        ack_ids: vec![reset.message_id],
        ack_remote_session: client_session,
        message_id: send_msg.fetch_add(1, Ordering::Relaxed),
        payload: Vec::new(),
    };
    out.send(&server_reset.encode()).await;

    // 2. Split the inbound packet stream into a control-payload queue (feeding
    //    the server-side TLS) and a data-packet queue.
    let (ctrl_tx, ctrl_rx) = mpsc::unbounded_channel::<Vec<u8>>();
    let (data_tx, mut data_rx) = mpsc::unbounded_channel::<Vec<u8>>();
    tokio::spawn(async move {
        loop {
            let Some(packet) = read_one_frame(&mut read_half).await else {
                return;
            };
            match opcode_of(packet[0]) {
                P_CONTROL_V1 => {
                    let ctrl = ControlPacket::decode(&packet);
                    if !ctrl.payload.is_empty() {
                        let _ = ctrl_tx.send(ctrl.payload);
                    }
                }
                P_ACK_V1 => {}
                P_DATA_V2 => {
                    let _ = data_tx.send(packet);
                }
                _ => {}
            }
        }
    });

    // Outbound control sender: each TLS write becomes a P_CONTROL_V1 message.
    let (out_tx, mut out_rx) = mpsc::unbounded_channel::<Vec<u8>>();
    {
        let out = out.clone();
        let send_msg = send_msg.clone();
        tokio::spawn(async move {
            while let Some(bytes) = out_rx.recv().await {
                let packet = ControlPacket {
                    opcode: P_CONTROL_V1,
                    local_session: server_session,
                    ack_ids: Vec::new(),
                    ack_remote_session: client_session,
                    message_id: send_msg.fetch_add(1, Ordering::Relaxed),
                    payload: bytes,
                };
                out.send(&packet.encode()).await;
            }
        });
    }

    let io = ControlIo {
        inbound_rx: ctrl_rx,
        leftover: Vec::new(),
        leftover_pos: 0,
        outbound_tx: out_tx,
    };

    // 3. TLS handshake tunnelled over the control channel.
    let mut tls = tls_acceptor().accept(io).await.expect("server TLS handshake");

    // 4. key-method-2 exchange + directional key derivation.
    let (client_pre, client_r1, client_r2) = read_client_key_method(&mut tls).await;
    let mut server_r1 = [0u8; 32];
    let mut server_r2 = [0u8; 32];
    getrandom::fill(&mut server_r1).unwrap();
    getrandom::fill(&mut server_r2).unwrap();
    tls.write_all(&server_key_method_record(&server_r1, &server_r2))
        .await
        .expect("write server key method");
    tls.flush().await.ok();

    let keys = derive_server_keys(
        &client_pre,
        &client_r1,
        &client_r2,
        &server_r1,
        &server_r2,
        client_session,
        server_session,
    );

    // 5. Push negotiation: wait for PUSH_REQUEST, reply with the assigned addr.
    let mut push_buf = Vec::new();
    let mut chunk = [0u8; 512];
    loop {
        if push_buf.iter().any(|&b| b == 0) {
            break;
        }
        let n = tls.read(&mut chunk).await.expect("read push request");
        assert_ne!(n, 0, "TLS closed before push request");
        push_buf.extend_from_slice(&chunk[..n]);
    }
    let push_reply =
        format!("PUSH_REPLY,ifconfig {CLIENT_TUN_IP} 255.255.255.0,peer-id {PEER_ID},route-gateway 10.8.0.1\0");
    tls.write_all(push_reply.as_bytes()).await.expect("write push reply");
    tls.flush().await.ok();
    drop(tls);

    // 6. Data phase: a smoltcp stack that accepts the inner TCP and echoes.
    run_data_plane(keys, out, &mut data_rx).await;
}

/// Serve one fake OpenVPN client over a connected UDP socket. Mirrors
/// [`serve_openvpn`] but over datagrams (no length prefix): it acks client
/// control packets (so the client's retransmit timer stops) and, when
/// `drop_first_reset` is set, ignores the client's first hard reset to force a
/// retransmit.
async fn serve_openvpn_udp(socket: Arc<UdpSocket>, drop_first_reset: bool) {
    let mut buf = vec![0u8; 65535];

    // Learn the client's address from its first datagram, then connect so we
    // only ever talk to that peer.
    let (mut n, peer) = socket.recv_from(&mut buf).await.expect("client hard reset datagram");
    socket.connect(peer).await.expect("connect udp to client");
    if drop_first_reset {
        // Ignore the first hard reset; the client must retransmit it.
        n = socket.recv(&mut buf).await.expect("retransmitted client hard reset");
    }
    let reset = ControlPacket::decode(&buf[..n]);
    assert_eq!(
        reset.opcode, P_CONTROL_HARD_RESET_CLIENT_V2,
        "first packet is client reset"
    );
    let client_session = reset.local_session;

    let mut server_session = [0u8; 8];
    getrandom::fill(&mut server_session).unwrap();
    let send_msg = Arc::new(AtomicU32::new(0));
    let out = Out::Udp(socket.clone());

    let server_reset = ControlPacket {
        opcode: P_CONTROL_HARD_RESET_SERVER_V2,
        local_session: server_session,
        ack_ids: vec![reset.message_id],
        ack_remote_session: client_session,
        message_id: send_msg.fetch_add(1, Ordering::Relaxed),
        payload: Vec::new(),
    };
    out.send(&server_reset.encode()).await;

    // Inbound datagram reader: dispatch control payloads (feeding server TLS)
    // and data packets, acking each reliable client control packet and
    // de-duplicating retransmits so the TLS stream never sees duplicate bytes.
    let (ctrl_tx, ctrl_rx) = mpsc::unbounded_channel::<Vec<u8>>();
    let (data_tx, mut data_rx) = mpsc::unbounded_channel::<Vec<u8>>();
    {
        let socket = socket.clone();
        let out = out.clone();
        tokio::spawn(async move {
            let mut buf = vec![0u8; 65535];
            let mut forwarded: HashSet<u32> = HashSet::new();
            loop {
                let n = match socket.recv(&mut buf).await {
                    Ok(n) => n,
                    Err(_) => return,
                };
                if n == 0 {
                    continue;
                }
                match opcode_of(buf[0]) {
                    P_CONTROL_V1 => {
                        let ctrl = ControlPacket::decode(&buf[..n]);
                        let ack = ControlPacket {
                            opcode: P_ACK_V1,
                            local_session: server_session,
                            ack_ids: vec![ctrl.message_id],
                            ack_remote_session: client_session,
                            message_id: 0,
                            payload: Vec::new(),
                        };
                        out.send(&ack.encode()).await;
                        if !ctrl.payload.is_empty() && forwarded.insert(ctrl.message_id) {
                            let _ = ctrl_tx.send(ctrl.payload);
                        }
                    }
                    P_CONTROL_HARD_RESET_CLIENT_V2 => {
                        // A retransmitted hard reset: re-ack it.
                        let ctrl = ControlPacket::decode(&buf[..n]);
                        let ack = ControlPacket {
                            opcode: P_ACK_V1,
                            local_session: server_session,
                            ack_ids: vec![ctrl.message_id],
                            ack_remote_session: client_session,
                            message_id: 0,
                            payload: Vec::new(),
                        };
                        out.send(&ack.encode()).await;
                    }
                    P_ACK_V1 => {}
                    P_DATA_V2 => {
                        let _ = data_tx.send(buf[..n].to_vec());
                    }
                    _ => {}
                }
            }
        });
    }

    // Outbound control sender: each TLS write becomes a P_CONTROL_V1 message.
    let (out_tx, mut out_rx) = mpsc::unbounded_channel::<Vec<u8>>();
    {
        let out = out.clone();
        let send_msg = send_msg.clone();
        tokio::spawn(async move {
            while let Some(bytes) = out_rx.recv().await {
                let packet = ControlPacket {
                    opcode: P_CONTROL_V1,
                    local_session: server_session,
                    ack_ids: Vec::new(),
                    ack_remote_session: client_session,
                    message_id: send_msg.fetch_add(1, Ordering::Relaxed),
                    payload: bytes,
                };
                out.send(&packet.encode()).await;
            }
        });
    }

    let io = ControlIo {
        inbound_rx: ctrl_rx,
        leftover: Vec::new(),
        leftover_pos: 0,
        outbound_tx: out_tx,
    };

    let mut tls = tls_acceptor().accept(io).await.expect("server TLS handshake");

    let (client_pre, client_r1, client_r2) = read_client_key_method(&mut tls).await;
    let mut server_r1 = [0u8; 32];
    let mut server_r2 = [0u8; 32];
    getrandom::fill(&mut server_r1).unwrap();
    getrandom::fill(&mut server_r2).unwrap();
    tls.write_all(&server_key_method_record(&server_r1, &server_r2))
        .await
        .expect("write server key method");
    tls.flush().await.ok();

    let keys = derive_server_keys(
        &client_pre,
        &client_r1,
        &client_r2,
        &server_r1,
        &server_r2,
        client_session,
        server_session,
    );

    let mut push_buf = Vec::new();
    let mut chunk = [0u8; 512];
    loop {
        if push_buf.iter().any(|&b| b == 0) {
            break;
        }
        let n = tls.read(&mut chunk).await.expect("read push request");
        assert_ne!(n, 0, "TLS closed before push request");
        push_buf.extend_from_slice(&chunk[..n]);
    }
    let push_reply =
        format!("PUSH_REPLY,ifconfig {CLIENT_TUN_IP} 255.255.255.0,peer-id {PEER_ID},route-gateway 10.8.0.1\0");
    tls.write_all(push_reply.as_bytes()).await.expect("write push reply");
    tls.flush().await.ok();
    drop(tls);

    run_data_plane(keys, out, &mut data_rx).await;
}

/// Read one length-prefixed OpenVPN packet, or `None` at EOF.
async fn read_one_frame<R>(read_half: &mut R) -> Option<Vec<u8>>
where
    R: AsyncRead + Unpin,
{
    let mut len_buf = [0u8; 2];
    read_half.read_exact(&mut len_buf).await.ok()?;
    let size = u16::from_be_bytes(len_buf) as usize;
    if size == 0 {
        return None;
    }
    let mut packet = vec![0u8; size];
    read_half.read_exact(&mut packet).await.ok()?;
    Some(packet)
}

async fn run_data_plane(keys: ServerKeys, out: Out, data_rx: &mut mpsc::UnboundedReceiver<Vec<u8>>) {
    let start = Instant::now();
    let mut phy = Phy {
        rx: VecDeque::new(),
        tx: VecDeque::new(),
    };
    let mut iface = {
        let cfg = IfaceConfig::new(HardwareAddress::Ip);
        let mut iface = Interface::new(cfg, &mut phy, now_since(start));
        iface.set_any_ip(true);
        iface.update_ip_addrs(|a| {
            let _ = a.push(IpCidr::new(
                IpAddress::Ipv4(Ipv4Address::from(Ipv4Addr::new(10, 8, 0, 1))),
                0,
            ));
        });
        let _ = iface
            .routes_mut()
            .add_default_ipv4_route(Ipv4Address::from(Ipv4Addr::new(10, 8, 0, 1)));
        iface
    };
    let mut sockets = SocketSet::new(Vec::new());
    let mut listener = tcp::Socket::new(
        tcp::SocketBuffer::new(vec![0u8; 256 * 1024]),
        tcp::SocketBuffer::new(vec![0u8; 256 * 1024]),
    );
    listener.listen(INNER_PORT).unwrap();
    let handle = sockets.add(listener);

    // A UDP echo on the same inner port: each datagram is sent back to its
    // source, so the client's inner UDP relay can be exercised end to end.
    let mut udp_echo = udp::Socket::new(
        udp::PacketBuffer::new(vec![udp::PacketMetadata::EMPTY; 16], vec![0u8; 64 * 1024]),
        udp::PacketBuffer::new(vec![udp::PacketMetadata::EMPTY; 16], vec![0u8; 64 * 1024]),
    );
    udp_echo.bind(INNER_PORT).unwrap();
    let udp_handle = sockets.add(udp_echo);

    let mut send_pid: u32 = 0;
    loop {
        let now = now_since(start);
        iface.poll(now, &mut phy, &mut sockets);

        let sock = sockets.get_mut::<tcp::Socket>(handle);
        while sock.can_recv() && sock.can_send() {
            let data = sock.recv(|b| (b.len(), b.to_vec())).unwrap_or_default();
            if data.is_empty() {
                break;
            }
            let _ = sock.send_slice(&data);
        }

        let usock = sockets.get_mut::<udp::Socket>(udp_handle);
        while usock.can_recv() {
            let Ok((data, meta)) = usock.recv().map(|(d, m)| (d.to_vec(), m)) else {
                break;
            };
            let _ = usock.send_slice(&data, meta.endpoint);
        }

        while let Some(pkt) = phy.tx.pop_front() {
            send_pid = send_pid.wrapping_add(1);
            let sealed = keys.seal(send_pid, &pkt);
            out.send(&sealed).await;
        }

        let delay = iface
            .poll_delay(now_since(start), &sockets)
            .map(|d| Duration::from_micros(d.total_micros()))
            .map_or(Duration::from_millis(20), |d| d.min(Duration::from_millis(20)));

        tokio::select! {
            pkt = data_rx.recv() => match pkt {
                Some(pkt) => {
                    if let Some(plain) = keys.open(&pkt) {
                        if !plain.is_empty() {
                            phy.rx.push_back(plain);
                        }
                    }
                }
                None => return,
            },
            _ = tokio::time::sleep(delay) => {}
        }
    }
}

// --- test harness -------------------------------------------------------------

/// Stand up a fake server on an ephemeral port and return a parsed client config
/// pointing at it (username/password auth, AES-256-GCM, inline CA).
async fn start_server() -> OpenVpnOutboundConfig {
    let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        while let Ok((tcp, _)) = listener.accept().await {
            tokio::spawn(serve_openvpn(tcp));
        }
    });

    let ca = TEST_CA.replace('\n', "\\n");
    let yaml = format!(
        "name: o\ntype: openvpn\nserver: 127.0.0.1\nport: {}\nproto: tcp\nusername: u\npassword: p\ncipher: AES-256-GCM\nca: \"{}\"\n",
        addr.port(),
        ca,
    );
    let entry: ProxyEntry = serde_yaml_ng::from_str(&yaml).unwrap();
    OpenVpnOutboundConfig::from_proxy(&entry).unwrap()
}

#[tokio::test]
async fn openvpn_tcp_round_trips_a_small_payload() {
    let config = start_server().await;
    let target = TargetAddr::Ip(SocketAddr::new(IpAddr::V4(INNER_IP), INNER_PORT));

    let mut stream = tokio::time::timeout(Duration::from_secs(20), openvpn::connect(&config, &target))
        .await
        .expect("connect did not time out")
        .expect("openvpn connect");

    let payload = b"hello openvpn tunnel";
    stream.write_all(payload).await.unwrap();

    let mut got = vec![0u8; payload.len()];
    stream.read_exact(&mut got).await.unwrap();
    assert_eq!(&got, payload);
}

#[tokio::test]
async fn openvpn_tcp_round_trips_a_large_payload() {
    let config = start_server().await;
    let target = TargetAddr::Ip(SocketAddr::new(IpAddr::V4(INNER_IP), INNER_PORT));

    let stream = tokio::time::timeout(Duration::from_secs(20), openvpn::connect(&config, &target))
        .await
        .expect("connect did not time out")
        .expect("openvpn connect");

    // 64 KiB spans many tunnel frames (inner MTU 1500).
    let payload: Vec<u8> = (0..64 * 1024).map(|i| (i % 251) as u8).collect();
    let writer_payload = payload.clone();
    let (mut rd, mut wr) = tokio::io::split(stream);
    let writer = tokio::spawn(async move {
        wr.write_all(&writer_payload).await.unwrap();
        wr.flush().await.unwrap();
    });

    let mut got = vec![0u8; payload.len()];
    rd.read_exact(&mut got).await.unwrap();
    writer.await.unwrap();
    assert_eq!(got, payload);
}

/// Stand up a fake UDP server on an ephemeral port and return a parsed client
/// config (`proto udp`) pointing at it. `drop_first_reset` makes the server
/// ignore the client's first hard reset to exercise control retransmission.
async fn start_udp_server(drop_first_reset: bool) -> OpenVpnOutboundConfig {
    let socket = UdpSocket::bind((Ipv4Addr::LOCALHOST, 0)).await.unwrap();
    let addr = socket.local_addr().unwrap();
    let socket = Arc::new(socket);
    tokio::spawn(serve_openvpn_udp(socket, drop_first_reset));

    let ca = TEST_CA.replace('\n', "\\n");
    let yaml = format!(
        "name: o\ntype: openvpn\nserver: 127.0.0.1\nport: {}\nproto: udp\nusername: u\npassword: p\ncipher: AES-256-GCM\nca: \"{}\"\n",
        addr.port(),
        ca,
    );
    let entry: ProxyEntry = serde_yaml_ng::from_str(&yaml).unwrap();
    OpenVpnOutboundConfig::from_proxy(&entry).unwrap()
}

#[tokio::test]
async fn openvpn_udp_round_trips_a_payload() {
    let config = start_udp_server(false).await;
    let target = TargetAddr::Ip(SocketAddr::new(IpAddr::V4(INNER_IP), INNER_PORT));

    let mut stream = tokio::time::timeout(Duration::from_secs(20), openvpn::connect(&config, &target))
        .await
        .expect("connect did not time out")
        .expect("openvpn connect");

    let payload = b"hello openvpn udp tunnel";
    stream.write_all(payload).await.unwrap();

    let mut got = vec![0u8; payload.len()];
    stream.read_exact(&mut got).await.unwrap();
    assert_eq!(&got, payload);
}

#[tokio::test]
async fn openvpn_relays_inner_udp_datagrams() {
    let config = start_server().await;
    let target = TargetAddr::Ip(SocketAddr::new(IpAddr::V4(INNER_IP), INNER_PORT));

    let assoc = tokio::time::timeout(Duration::from_secs(20), openvpn::connect_udp(&config, &target))
        .await
        .expect("connect did not time out")
        .expect("openvpn connect_udp");

    // Each datagram must come back as exactly one datagram (boundaries kept).
    for payload in [&b"first inner udp datagram"[..], &b"second one"[..]] {
        assoc.send(payload).await.unwrap();
        let got = tokio::time::timeout(Duration::from_secs(10), assoc.recv())
            .await
            .expect("recv did not time out")
            .expect("udp recv");
        assert_eq!(got, payload);
    }
}

#[tokio::test]
async fn openvpn_udp_transport_relays_inner_udp_datagrams() {
    // Inner UDP relay over the UDP outer transport (no length-prefix framing).
    let config = start_udp_server(false).await;
    let target = TargetAddr::Ip(SocketAddr::new(IpAddr::V4(INNER_IP), INNER_PORT));

    let assoc = tokio::time::timeout(Duration::from_secs(20), openvpn::connect_udp(&config, &target))
        .await
        .expect("connect did not time out")
        .expect("openvpn connect_udp");

    let payload = b"udp in udp";
    assoc.send(payload).await.unwrap();
    let got = tokio::time::timeout(Duration::from_secs(10), assoc.recv())
        .await
        .expect("recv did not time out")
        .expect("udp recv");
    assert_eq!(got, payload);
}

#[tokio::test]
async fn openvpn_udp_recovers_from_a_dropped_hard_reset() {
    // The server drops the client's first hard reset; the client's control
    // channel must retransmit it (after ~1s) for the handshake to complete.
    let config = start_udp_server(true).await;
    let target = TargetAddr::Ip(SocketAddr::new(IpAddr::V4(INNER_IP), INNER_PORT));

    let mut stream = tokio::time::timeout(Duration::from_secs(20), openvpn::connect(&config, &target))
        .await
        .expect("connect did not time out")
        .expect("openvpn connect");

    let payload = b"udp retransmit recovery";
    stream.write_all(payload).await.unwrap();

    let mut got = vec![0u8; payload.len()];
    stream.read_exact(&mut got).await.unwrap();
    assert_eq!(&got, payload);
}
