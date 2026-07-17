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
//! The server also implements independent `tls-auth` (HMAC-SHA1 wrap of every
//! control packet) and `tls-crypt` (AES-256-CTR + HMAC-SHA256) modes keyed from
//! a shared "OpenVPN Static key V1", proving the client's static
//! control-channel protection interoperates in both directions.
//!
//! The server crypto/codec is written independently of the crate under test so
//! the test proves genuine interop rather than that the code agrees with itself.

use std::collections::{HashSet, VecDeque};
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};
use std::pin::Pin;
use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};
use std::task::{Context as TaskContext, Poll};
use std::time::{Duration, Instant};

use aes::Aes256;
use aes_gcm::Aes256Gcm;
use aes_gcm::aead::{Aead, KeyInit, Payload};
use ctr::cipher::{KeyIvInit, StreamCipher};
use hmac::{Hmac, Mac};
use learn_gripe::{OpenVpnOutboundConfig, ProxyEntry, TargetAddr, openvpn};
use md5::Md5;
use sha1::Sha1;
use sha2::Sha256;
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
/// Tunnel IPv6 address the fake server assigns via `PUSH_REPLY ifconfig-ipv6`.
const CLIENT_TUN_IP_V6: Ipv6Addr = Ipv6Addr::new(0xfd00, 8, 0, 0, 0, 0, 0, 2);
/// The server's own tunnel-side IPv6 address (the `ifconfig-ipv6` remote).
const SERVER_TUN_IP_V6: Ipv6Addr = Ipv6Addr::new(0xfd00, 8, 0, 0, 0, 0, 0, 1);
/// Inner IPv6 target the relayed connection dials (accepted via `any_ip`).
const INNER_IP_V6: Ipv6Addr = Ipv6Addr::new(0xfd00, 8, 0, 0, 0, 0, 0, 0x88);
/// Data-channel peer id the server pushes.
const PEER_ID: u32 = 5;
const MTU: usize = 1600;

// --- OpenVPN opcodes / framing (independent copy of the wire format) ----------

const OPCODE_SHIFT: u8 = 3;
const KEY_ID_MASK: u8 = 0x07;
const P_CONTROL_HARD_RESET_CLIENT_V2: u8 = 7;
const P_CONTROL_HARD_RESET_SERVER_V2: u8 = 8;
const P_CONTROL_SOFT_RESET_V1: u8 = 3;
const P_CONTROL_V1: u8 = 4;
const P_ACK_V1: u8 = 5;
const P_DATA_V2: u8 = 9;
const P_CONTROL_HARD_RESET_CLIENT_V3: u8 = 10;

fn opcode_of(b: u8) -> u8 {
    b >> OPCODE_SHIFT
}

// --- server-side tls-auth / tls-crypt (independent copy of the wire format) ----

/// A fixed 2048-bit "OpenVPN Static key V1" shared by client and fake server.
fn static_key_bytes() -> [u8; 256] {
    let mut key = [0u8; 256];
    for (i, b) in key.iter_mut().enumerate() {
        *b = (i as u8).wrapping_mul(31).wrapping_add(7);
    }
    key
}

/// The same key rendered as the inline file format the client config takes.
fn static_key_text() -> String {
    let key = static_key_bytes();
    let mut body = String::new();
    for (i, b) in key.iter().enumerate() {
        body.push_str(&format!("{b:02x}"));
        if i % 16 == 15 {
            body.push('\n');
        }
    }
    format!("-----BEGIN OpenVPN Static key V1-----\n{body}-----END OpenVPN Static key V1-----\n")
}

/// Server-side control-channel protection. The server is `key-direction 0`
/// (normal): it sends with key slot 0 and receives with slot 1, the mirror of
/// the client's `key-direction 1`. tls-crypt likewise uses slot 0 to send and
/// slot 1 to receive on the server.
#[derive(Clone)]
enum ServerWrap {
    None,
    /// HMAC-SHA1 keys (first 20 bytes of each slot's HMAC half).
    TlsAuth {
        send: Vec<u8>,
        recv: Vec<u8>,
    },
    /// AES-256-CTR + HMAC-SHA256 keys (first 32 bytes of each half).
    TlsCrypt {
        send_cipher: Vec<u8>,
        send_hmac: Vec<u8>,
        recv_cipher: Vec<u8>,
        recv_hmac: Vec<u8>,
    },
}

impl ServerWrap {
    fn tls_auth() -> Self {
        let key = static_key_bytes();
        Self::TlsAuth {
            send: key[64..84].to_vec(),   // slot 0 HMAC half
            recv: key[192..212].to_vec(), // slot 1 HMAC half
        }
    }

    fn tls_crypt() -> Self {
        Self::tls_crypt_with(&static_key_bytes())
    }

    fn tls_crypt_with(key: &[u8; 256]) -> Self {
        Self::TlsCrypt {
            send_cipher: key[0..32].to_vec(),
            send_hmac: key[64..96].to_vec(),
            recv_cipher: key[128..160].to_vec(),
            recv_hmac: key[192..224].to_vec(),
        }
    }

    /// tls-crypt-v2: the same tls-crypt wire format keyed from the per-client
    /// key `Kc` (which the server recovers from the WKc on the V3 hard reset).
    fn tls_crypt_v2() -> Self {
        Self::tls_crypt_with(&v2_client_kc())
    }

    /// Wrap one plaintext control packet (`op || session || rest`) for the wire.
    fn wrap(&self, plain: &[u8], packet_id: u32) -> Vec<u8> {
        let header = &plain[..9];
        let rest = &plain[9..];
        let mut replay = [0u8; 8];
        replay[..4].copy_from_slice(&packet_id.to_be_bytes());
        replay[4..].copy_from_slice(
            &(std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs() as u32)
                .to_be_bytes(),
        );
        match self {
            Self::None => plain.to_vec(),
            Self::TlsAuth { send, .. } => {
                let mut mac = <Hmac<Sha1> as Mac>::new_from_slice(send).unwrap();
                mac.update(&replay);
                mac.update(header);
                mac.update(rest);
                let tag = mac.finalize().into_bytes();
                let mut out = Vec::new();
                out.extend_from_slice(header);
                out.extend_from_slice(&tag);
                out.extend_from_slice(&replay);
                out.extend_from_slice(rest);
                out
            }
            Self::TlsCrypt {
                send_cipher, send_hmac, ..
            } => {
                let mut mac = <Hmac<Sha256> as Mac>::new_from_slice(send_hmac).unwrap();
                mac.update(header);
                mac.update(&replay);
                mac.update(rest);
                let tag = mac.finalize().into_bytes();
                let mut ct = rest.to_vec();
                let mut ctr = ctr::Ctr128BE::<Aes256>::new(send_cipher.as_slice().into(), tag[..16].into());
                ctr.apply_keystream(&mut ct);
                let mut out = Vec::new();
                out.extend_from_slice(header);
                out.extend_from_slice(&replay);
                out.extend_from_slice(&tag);
                out.extend_from_slice(&ct);
                out
            }
        }
    }

    /// Verify/decrypt one wire control packet back into `op || session || rest`.
    fn unwrap(&self, wire: &[u8]) -> Vec<u8> {
        match self {
            Self::None => wire.to_vec(),
            Self::TlsAuth { recv, .. } => {
                let header = &wire[..9];
                let tag = &wire[9..29];
                let replay = &wire[29..37];
                let rest = &wire[37..];
                let mut mac = <Hmac<Sha1> as Mac>::new_from_slice(recv).unwrap();
                mac.update(replay);
                mac.update(header);
                mac.update(rest);
                mac.verify_slice(tag).expect("client tls-auth HMAC");
                let mut out = Vec::new();
                out.extend_from_slice(header);
                out.extend_from_slice(rest);
                out
            }
            Self::TlsCrypt {
                recv_cipher, recv_hmac, ..
            } => {
                let header = &wire[..9];
                let replay = &wire[9..17];
                let tag = &wire[17..49];
                let mut plain = wire[49..].to_vec();
                let mut ctr = ctr::Ctr128BE::<Aes256>::new(recv_cipher.as_slice().into(), tag[..16].into());
                ctr.apply_keystream(&mut plain);
                let mut mac = <Hmac<Sha256> as Mac>::new_from_slice(recv_hmac).unwrap();
                mac.update(header);
                mac.update(replay);
                mac.update(&plain);
                mac.verify_slice(tag).expect("client tls-crypt tag");
                let mut out = Vec::new();
                out.extend_from_slice(header);
                out.extend_from_slice(&plain);
                out
            }
        }
    }
}

// --- server-side tls-crypt-v2 (per-client key + server-wrapped WKc) -----------

/// The fixed per-client tls-crypt-v2 key `Kc`.
fn v2_client_kc() -> [u8; 256] {
    let mut key = [0u8; 256];
    for (i, b) in key.iter_mut().enumerate() {
        *b = (i as u8).wrapping_mul(13).wrapping_add(5);
    }
    key
}

/// The server key: `Ke` (AES-256-CTR) = first 32 bytes, `Ka` (HMAC-SHA256) =
/// last 32 bytes.
fn v2_server_key() -> [u8; 64] {
    let mut key = [0u8; 64];
    for (i, b) in key.iter_mut().enumerate() {
        *b = (i as u8).wrapping_mul(7).wrapping_add(1);
    }
    key
}

/// User metadata carried inside the WKc (type byte 0x00 + free-form bytes).
const V2_METADATA: &[u8] = b"\x00client-1";

/// Build the WKc the way `openvpn --genkey tls-crypt-v2-client` does:
/// `T || AES-256-CTR(Ke, IV=T[..16], Kc || metadata) || len`, with
/// `T = HMAC-SHA256(Ka, len || Kc || metadata)`.
fn v2_wkc() -> Vec<u8> {
    let kc = v2_client_kc();
    let server_key = v2_server_key();
    let (ke, ka) = server_key.split_at(32);
    let mut payload = kc.to_vec();
    payload.extend_from_slice(V2_METADATA);
    let len = (32 + payload.len() + 2) as u16;
    let mut mac = <Hmac<Sha256> as Mac>::new_from_slice(ka).unwrap();
    mac.update(&len.to_be_bytes());
    mac.update(&payload);
    let tag = mac.finalize().into_bytes();
    let mut ct = payload;
    let mut ctr = ctr::Ctr128BE::<Aes256>::new(ke.into(), tag[..16].into());
    ctr.apply_keystream(&mut ct);
    let mut wkc = tag.to_vec();
    wkc.extend_from_slice(&ct);
    wkc.extend_from_slice(&len.to_be_bytes());
    wkc
}

/// The inline client key file: base64 of `Kc || WKc` between the v2 markers.
fn v2_client_key_text() -> String {
    use base64::Engine as _;
    let mut raw = v2_client_kc().to_vec();
    raw.extend_from_slice(&v2_wkc());
    let b64 = base64::engine::general_purpose::STANDARD.encode(&raw);
    format!("-----BEGIN OpenVPN tls-crypt-v2 client key-----\n{b64}\n-----END OpenVPN tls-crypt-v2 client key-----\n")
}

/// Server side of the V3 hard reset: split the cleartext WKc off the tail
/// (via its trailing length field), unwrap it with the server key, check the
/// recovered `Kc` + metadata, and return the inner tls-crypt packet.
fn strip_and_verify_wkc(packet: &[u8]) -> Vec<u8> {
    let len = u16::from_be_bytes(packet[packet.len() - 2..].try_into().unwrap()) as usize;
    assert!(len >= 34 && len <= packet.len(), "WKc length field");
    let (inner, wkc) = packet.split_at(packet.len() - len);
    let tag = &wkc[..32];
    let mut payload = wkc[32..len - 2].to_vec();
    let server_key = v2_server_key();
    let (ke, ka) = server_key.split_at(32);
    let mut ctr = ctr::Ctr128BE::<Aes256>::new(ke.into(), tag[..16].into());
    ctr.apply_keystream(&mut payload);
    let mut mac = <Hmac<Sha256> as Mac>::new_from_slice(ka).unwrap();
    mac.update(&(len as u16).to_be_bytes());
    mac.update(&payload);
    mac.verify_slice(tag).expect("WKc authentication");
    assert_eq!(&payload[..256], &v2_client_kc()[..], "recovered Kc");
    assert_eq!(&payload[256..], V2_METADATA, "WKc metadata");
    inner.to_vec()
}

fn opcode_key_id(opcode: u8, key_id: u8) -> u8 {
    (opcode << OPCODE_SHIFT) | (key_id & KEY_ID_MASK)
}

#[derive(Clone)]
struct ControlPacket {
    opcode: u8,
    key_id: u8,
    local_session: [u8; 8],
    ack_ids: Vec<u32>,
    ack_remote_session: [u8; 8],
    message_id: u32,
    payload: Vec<u8>,
}

impl ControlPacket {
    fn decode(packet: &[u8]) -> Self {
        let opcode = opcode_of(packet[0]);
        let key_id = packet[0] & KEY_ID_MASK;
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
            key_id,
            local_session,
            ack_ids,
            ack_remote_session,
            message_id,
            payload,
        }
    }

    fn encode(&self) -> Vec<u8> {
        let mut out = Vec::new();
        out.push(opcode_key_id(self.opcode, self.key_id));
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

fn data_header(peer_id: u32, key_id: u8) -> [u8; 4] {
    [
        opcode_key_id(P_DATA_V2, key_id),
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
    fn seal(&self, key_id: u8, packet_id: u32, plaintext: &[u8]) -> Vec<u8> {
        let header = data_header(PEER_ID, key_id);
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

/// OpenVPN's fixed data-channel keepalive ping payload (independent copy).
const SERVER_PING: [u8; 16] = [
    0x2a, 0x18, 0x7b, 0xf3, 0x64, 0x1e, 0xb4, 0xcb, 0x07, 0xed, 0x2d, 0x0a, 0x98, 0x1f, 0xc7, 0x48,
];

/// Per-connection knobs for the fake server.
#[derive(Clone, Default)]
struct ServerOpts {
    /// Extra options appended verbatim to the `PUSH_REPLY` (leading comma
    /// included), e.g. `",keepalive 1 60"`.
    push_extra: String,
    /// Give the server's inner stack an IPv6 address + default route so it can
    /// terminate inner IPv6 flows (pair with an `ifconfig-ipv6` push_extra).
    ipv6: bool,
    /// Notified once for every decrypted data-channel ping from the client.
    ping_seen: Option<mpsc::UnboundedSender<()>>,
    /// Notified with the key id when the server first decrypts a client data
    /// packet under a renegotiated key (proof the client rotated keys).
    rekey_done: Option<mpsc::UnboundedSender<u8>>,
    /// If set, the server initiates a soft-reset renegotiation itself this long
    /// after the data phase starts (TCP server only).
    initiate_rekey_after: Option<Duration>,
}

/// Everything the TCP data plane needs to renegotiate keys mid-session.
struct DataPlaneRekey {
    /// Client soft resets surfaced by the packet splitter.
    soft_rx: mpsc::UnboundedReceiver<ControlPacket>,
    /// Where the splitter routes `P_CONTROL_V1` payloads (the current key
    /// state's TLS stream); replaced for each renegotiation.
    tls_in: Arc<std::sync::Mutex<Option<mpsc::UnboundedSender<Vec<u8>>>>>,
    wrap: ServerWrap,
    wrap_pid: Arc<AtomicU32>,
    server_session: [u8; 8],
    client_session: [u8; 8],
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
async fn serve_openvpn(tcp: tokio::net::TcpStream, wrap: ServerWrap, opts: ServerOpts) {
    let _ = tcp.set_nodelay(true);
    let (mut read_half, write_half) = tcp.into_split();
    let out = Out::Tcp(Arc::new(Mutex::new(write_half)));

    // Server session id + monotonic control message-id counter + monotonic
    // tls-auth/tls-crypt packet-id counter.
    let mut server_session = [0u8; 8];
    getrandom::fill(&mut server_session).unwrap();
    let send_msg = Arc::new(AtomicU32::new(0));
    let wrap_pid = Arc::new(AtomicU32::new(1));

    // 1. Hard-reset exchange. First framed packet must be the client reset.
    let mut first = read_one_frame(&mut read_half).await.expect("client hard reset");
    if opcode_of(first[0]) == P_CONTROL_HARD_RESET_CLIENT_V3 {
        first = strip_and_verify_wkc(&first);
    }
    let first = wrap.unwrap(&first);
    let reset = ControlPacket::decode(&first);
    assert!(
        matches!(
            reset.opcode,
            P_CONTROL_HARD_RESET_CLIENT_V2 | P_CONTROL_HARD_RESET_CLIENT_V3
        ),
        "first packet is client reset"
    );
    let client_session = reset.local_session;

    let server_reset = ControlPacket {
        opcode: P_CONTROL_HARD_RESET_SERVER_V2,
        key_id: 0,
        local_session: server_session,
        ack_ids: vec![reset.message_id],
        ack_remote_session: client_session,
        message_id: send_msg.fetch_add(1, Ordering::Relaxed),
        payload: Vec::new(),
    };
    out.send(&wrap.wrap(&server_reset.encode(), wrap_pid.fetch_add(1, Ordering::Relaxed)))
        .await;

    // 2. Split the inbound packet stream into a control-payload queue (feeding
    //    the current key state's server-side TLS), a soft-reset queue (rekey
    //    triggers), and a data-packet queue.
    let (ctrl_tx, ctrl_rx) = mpsc::unbounded_channel::<Vec<u8>>();
    let tls_in: Arc<std::sync::Mutex<Option<mpsc::UnboundedSender<Vec<u8>>>>> =
        Arc::new(std::sync::Mutex::new(Some(ctrl_tx)));
    let (soft_tx, soft_rx) = mpsc::unbounded_channel::<ControlPacket>();
    let (data_tx, mut data_rx) = mpsc::unbounded_channel::<Vec<u8>>();
    {
        let wrap = wrap.clone();
        let tls_in = tls_in.clone();
        tokio::spawn(async move {
            loop {
                let Some(packet) = read_one_frame(&mut read_half).await else {
                    return;
                };
                match opcode_of(packet[0]) {
                    P_CONTROL_V1 => {
                        let ctrl = ControlPacket::decode(&wrap.unwrap(&packet));
                        if !ctrl.payload.is_empty()
                            && let Some(tx) = tls_in.lock().unwrap().as_ref()
                        {
                            let _ = tx.send(ctrl.payload);
                        }
                    }
                    P_CONTROL_SOFT_RESET_V1 => {
                        let _ = soft_tx.send(ControlPacket::decode(&wrap.unwrap(&packet)));
                    }
                    P_ACK_V1 => {}
                    P_DATA_V2 => {
                        let _ = data_tx.send(packet);
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
        let wrap = wrap.clone();
        let wrap_pid = wrap_pid.clone();
        tokio::spawn(async move {
            while let Some(bytes) = out_rx.recv().await {
                let packet = ControlPacket {
                    opcode: P_CONTROL_V1,
                    key_id: 0,
                    local_session: server_session,
                    ack_ids: Vec::new(),
                    ack_remote_session: client_session,
                    message_id: send_msg.fetch_add(1, Ordering::Relaxed),
                    payload: bytes,
                };
                out.send(&wrap.wrap(&packet.encode(), wrap_pid.fetch_add(1, Ordering::Relaxed)))
                    .await;
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
    let push_reply = format!(
        "PUSH_REPLY,ifconfig {CLIENT_TUN_IP} 255.255.255.0,peer-id {PEER_ID},route-gateway 10.8.0.1{}\0",
        opts.push_extra
    );
    tls.write_all(push_reply.as_bytes()).await.expect("write push reply");
    tls.flush().await.ok();
    drop(tls);
    *tls_in.lock().unwrap() = None;

    // 6. Data phase: a smoltcp stack that accepts the inner TCP and echoes,
    //    with soft-reset renegotiation support.
    let rekey = DataPlaneRekey {
        soft_rx,
        tls_in,
        wrap,
        wrap_pid,
        server_session,
        client_session,
    };
    run_data_plane(keys, out, &mut data_rx, opts, Some(rekey)).await;
}

/// Serve one fake OpenVPN client over a connected UDP socket. Mirrors
/// [`serve_openvpn`] but over datagrams (no length prefix): it acks client
/// control packets (so the client's retransmit timer stops) and, when
/// `drop_first_reset` is set, ignores the client's first hard reset to force a
/// retransmit.
async fn serve_openvpn_udp(socket: Arc<UdpSocket>, drop_first_reset: bool, wrap: ServerWrap, opts: ServerOpts) {
    let mut buf = vec![0u8; 65535];

    // Learn the client's address from its first datagram, then connect so we
    // only ever talk to that peer.
    let (mut n, peer) = socket.recv_from(&mut buf).await.expect("client hard reset datagram");
    socket.connect(peer).await.expect("connect udp to client");
    if drop_first_reset {
        // Ignore the first hard reset; the client must retransmit it.
        n = socket.recv(&mut buf).await.expect("retransmitted client hard reset");
    }
    let mut first = buf[..n].to_vec();
    if opcode_of(first[0]) == P_CONTROL_HARD_RESET_CLIENT_V3 {
        first = strip_and_verify_wkc(&first);
    }
    let first = wrap.unwrap(&first);
    let reset = ControlPacket::decode(&first);
    assert!(
        matches!(
            reset.opcode,
            P_CONTROL_HARD_RESET_CLIENT_V2 | P_CONTROL_HARD_RESET_CLIENT_V3
        ),
        "first packet is client reset"
    );
    let client_session = reset.local_session;

    let mut server_session = [0u8; 8];
    getrandom::fill(&mut server_session).unwrap();
    let send_msg = Arc::new(AtomicU32::new(0));
    let wrap_pid = Arc::new(AtomicU32::new(1));
    let out = Out::Udp(socket.clone());

    let server_reset = ControlPacket {
        opcode: P_CONTROL_HARD_RESET_SERVER_V2,
        key_id: 0,
        local_session: server_session,
        ack_ids: vec![reset.message_id],
        ack_remote_session: client_session,
        message_id: send_msg.fetch_add(1, Ordering::Relaxed),
        payload: Vec::new(),
    };
    out.send(&wrap.wrap(&server_reset.encode(), wrap_pid.fetch_add(1, Ordering::Relaxed)))
        .await;

    // Inbound datagram reader: dispatch control payloads (feeding server TLS)
    // and data packets, acking each reliable client control packet and
    // de-duplicating retransmits so the TLS stream never sees duplicate bytes.
    let (ctrl_tx, ctrl_rx) = mpsc::unbounded_channel::<Vec<u8>>();
    let (data_tx, mut data_rx) = mpsc::unbounded_channel::<Vec<u8>>();
    {
        let socket = socket.clone();
        let out = out.clone();
        let wrap = wrap.clone();
        let wrap_pid = wrap_pid.clone();
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
                        let ctrl = ControlPacket::decode(&wrap.unwrap(&buf[..n]));
                        let ack = ControlPacket {
                            opcode: P_ACK_V1,
                            key_id: 0,
                            local_session: server_session,
                            ack_ids: vec![ctrl.message_id],
                            ack_remote_session: client_session,
                            message_id: 0,
                            payload: Vec::new(),
                        };
                        out.send(&wrap.wrap(&ack.encode(), wrap_pid.fetch_add(1, Ordering::Relaxed)))
                            .await;
                        if !ctrl.payload.is_empty() && forwarded.insert(ctrl.message_id) {
                            let _ = ctrl_tx.send(ctrl.payload);
                        }
                    }
                    P_CONTROL_HARD_RESET_CLIENT_V2 | P_CONTROL_HARD_RESET_CLIENT_V3 => {
                        // A retransmitted hard reset: re-ack it.
                        let mut raw = buf[..n].to_vec();
                        if opcode_of(raw[0]) == P_CONTROL_HARD_RESET_CLIENT_V3 {
                            raw = strip_and_verify_wkc(&raw);
                        }
                        let ctrl = ControlPacket::decode(&wrap.unwrap(&raw));
                        let ack = ControlPacket {
                            opcode: P_ACK_V1,
                            key_id: 0,
                            local_session: server_session,
                            ack_ids: vec![ctrl.message_id],
                            ack_remote_session: client_session,
                            message_id: 0,
                            payload: Vec::new(),
                        };
                        out.send(&wrap.wrap(&ack.encode(), wrap_pid.fetch_add(1, Ordering::Relaxed)))
                            .await;
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
        let wrap = wrap.clone();
        let wrap_pid = wrap_pid.clone();
        tokio::spawn(async move {
            while let Some(bytes) = out_rx.recv().await {
                let packet = ControlPacket {
                    opcode: P_CONTROL_V1,
                    key_id: 0,
                    local_session: server_session,
                    ack_ids: Vec::new(),
                    ack_remote_session: client_session,
                    message_id: send_msg.fetch_add(1, Ordering::Relaxed),
                    payload: bytes,
                };
                out.send(&wrap.wrap(&packet.encode(), wrap_pid.fetch_add(1, Ordering::Relaxed)))
                    .await;
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
    let push_reply = format!(
        "PUSH_REPLY,ifconfig {CLIENT_TUN_IP} 255.255.255.0,peer-id {PEER_ID},route-gateway 10.8.0.1{}\0",
        opts.push_extra
    );
    tls.write_all(push_reply.as_bytes()).await.expect("write push reply");
    tls.flush().await.ok();
    drop(tls);

    run_data_plane(keys, out, &mut data_rx, opts, None).await;
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

/// One negotiated data-channel key epoch on the server side.
struct Epoch {
    key_id: u8,
    keys: ServerKeys,
    send_pid: u32,
}

/// Open a new server-side key state for `key_id`: install a fresh TLS inbound
/// queue, send the server's soft reset (acking the client's when replying to
/// one), and run the TLS + key-method-2 exchange in a task that delivers the
/// derived keys through `keys_tx`.
async fn start_server_rekey(
    rk: &DataPlaneRekey,
    key_id: u8,
    ack: Option<u32>,
    out: &Out,
    keys_tx: &mpsc::UnboundedSender<(u8, ServerKeys)>,
) {
    let (ctrl_tx, ctrl_rx) = mpsc::unbounded_channel::<Vec<u8>>();
    *rk.tls_in.lock().unwrap() = Some(ctrl_tx);

    // Fresh per-key-state message-id space, as upstream requires.
    let send_msg = Arc::new(AtomicU32::new(0));
    let reply = ControlPacket {
        opcode: P_CONTROL_SOFT_RESET_V1,
        key_id,
        local_session: rk.server_session,
        ack_ids: ack.into_iter().collect(),
        ack_remote_session: rk.client_session,
        message_id: send_msg.fetch_add(1, Ordering::Relaxed),
        payload: Vec::new(),
    };
    out.send(
        &rk.wrap
            .wrap(&reply.encode(), rk.wrap_pid.fetch_add(1, Ordering::Relaxed)),
    )
    .await;

    // Outbound pump: TLS writes become P_CONTROL_V1 messages under `key_id`.
    let (out_tx, mut out_rx) = mpsc::unbounded_channel::<Vec<u8>>();
    {
        let out = out.clone();
        let wrap = rk.wrap.clone();
        let wrap_pid = rk.wrap_pid.clone();
        let send_msg = send_msg.clone();
        let (server_session, client_session) = (rk.server_session, rk.client_session);
        tokio::spawn(async move {
            while let Some(bytes) = out_rx.recv().await {
                let packet = ControlPacket {
                    opcode: P_CONTROL_V1,
                    key_id,
                    local_session: server_session,
                    ack_ids: Vec::new(),
                    ack_remote_session: client_session,
                    message_id: send_msg.fetch_add(1, Ordering::Relaxed),
                    payload: bytes,
                };
                out.send(&wrap.wrap(&packet.encode(), wrap_pid.fetch_add(1, Ordering::Relaxed)))
                    .await;
            }
        });
    }

    let io = ControlIo {
        inbound_rx: ctrl_rx,
        leftover: Vec::new(),
        leftover_pos: 0,
        outbound_tx: out_tx,
    };
    let (server_session, client_session) = (rk.server_session, rk.client_session);
    let keys_tx = keys_tx.clone();
    tokio::spawn(async move {
        let mut tls = tls_acceptor().accept(io).await.expect("rekey TLS handshake");
        let (client_pre, client_r1, client_r2) = read_client_key_method(&mut tls).await;
        let mut server_r1 = [0u8; 32];
        let mut server_r2 = [0u8; 32];
        getrandom::fill(&mut server_r1).unwrap();
        getrandom::fill(&mut server_r2).unwrap();
        tls.write_all(&server_key_method_record(&server_r1, &server_r2))
            .await
            .expect("write rekey server key method");
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
        let _ = keys_tx.send((key_id, keys));
    });
}

/// Await the next client soft reset, or pend forever when the connection has
/// no rekey support (the UDP server).
async fn next_soft_reset(rx: Option<&mut mpsc::UnboundedReceiver<ControlPacket>>) -> Option<ControlPacket> {
    match rx {
        Some(rx) => rx.recv().await,
        None => std::future::pending().await,
    }
}

async fn run_data_plane(
    keys: ServerKeys,
    out: Out,
    data_rx: &mut mpsc::UnboundedReceiver<Vec<u8>>,
    opts: ServerOpts,
    mut rekey: Option<DataPlaneRekey>,
) {
    let ping_seen = opts.ping_seen;
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
            if opts.ipv6 {
                let _ = a.push(IpCidr::new(IpAddress::Ipv6(SERVER_TUN_IP_V6), 0));
            }
        });
        let _ = iface
            .routes_mut()
            .add_default_ipv4_route(Ipv4Address::from(Ipv4Addr::new(10, 8, 0, 1)));
        if opts.ipv6 {
            let _ = iface.routes_mut().add_default_ipv6_route(SERVER_TUN_IP_V6);
        }
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

    // Key epochs, oldest first. The server keeps sending under the epoch of
    // the newest key it has *received* client data under, so it never sends
    // with keys the client may not have installed yet.
    let mut epochs: Vec<Epoch> = vec![Epoch {
        key_id: 0,
        keys,
        send_pid: 0,
    }];
    let mut send_idx: usize = 0;
    let (keys_tx, mut keys_rx) = mpsc::unbounded_channel::<(u8, ServerKeys)>();
    // Key ids whose renegotiation is underway or complete.
    let mut known_key_ids: HashSet<u8> = HashSet::from([0]);
    let mut initiate_at = opts
        .initiate_rekey_after
        .map(|after| tokio::time::Instant::now() + after);

    loop {
        if let (Some(at), Some(rk)) = (initiate_at, rekey.as_ref())
            && tokio::time::Instant::now() >= at
        {
            initiate_at = None;
            known_key_ids.insert(1);
            start_server_rekey(rk, 1, None, &out, &keys_tx).await;
        }

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
            let epoch = &mut epochs[send_idx];
            epoch.send_pid = epoch.send_pid.wrapping_add(1);
            let sealed = epoch.keys.seal(epoch.key_id, epoch.send_pid, &pkt);
            out.send(&sealed).await;
        }

        let delay = iface
            .poll_delay(now_since(start), &sockets)
            .map(|d| Duration::from_micros(d.total_micros()))
            .map_or(Duration::from_millis(20), |d| d.min(Duration::from_millis(20)));

        tokio::select! {
            pkt = data_rx.recv() => match pkt {
                Some(pkt) => {
                    let key_id = pkt[0] & KEY_ID_MASK;
                    if let Some(idx) = epochs.iter().position(|e| e.key_id == key_id)
                        && let Some(plain) = epochs[idx].keys.open(&pkt)
                    {
                        if idx > send_idx {
                            // First client data under the renegotiated key:
                            // switch our send epoch to it.
                            send_idx = idx;
                            if let Some(tx) = &opts.rekey_done {
                                let _ = tx.send(key_id);
                            }
                        }
                        if plain == SERVER_PING {
                            if let Some(tx) = &ping_seen {
                                let _ = tx.send(());
                            }
                        } else if !plain.is_empty() {
                            phy.rx.push_back(plain);
                        }
                    }
                }
                None => return,
            },
            soft = next_soft_reset(rekey.as_mut().map(|r| &mut r.soft_rx)) => match soft {
                Some(soft) => {
                    // A client-initiated soft reset -- unless it belongs to a
                    // renegotiation already underway (our own initiation, or a
                    // retransmit of a reset we already answered).
                    if known_key_ids.insert(soft.key_id) {
                        let rk = rekey.as_ref().unwrap();
                        start_server_rekey(rk, soft.key_id, Some(soft.message_id), &out, &keys_tx).await;
                    }
                }
                None => rekey = None,
            },
            newly = keys_rx.recv() => {
                if let Some((key_id, keys)) = newly {
                    epochs.push(Epoch { key_id, keys, send_pid: 0 });
                }
            }
            _ = tokio::time::sleep(delay) => {}
        }
    }
}

// --- test harness -------------------------------------------------------------

/// Stand up a fake server on an ephemeral port and return a parsed client config
/// pointing at it (username/password auth, AES-256-GCM, inline CA). `wrap`
/// selects the server's control-channel protection; `extra_yaml` appends the
/// matching client config lines.
async fn start_server_with(wrap: ServerWrap, extra_yaml: &str) -> OpenVpnOutboundConfig {
    start_server_opts(wrap, extra_yaml, ServerOpts::default()).await
}

async fn start_server_opts(wrap: ServerWrap, extra_yaml: &str, opts: ServerOpts) -> OpenVpnOutboundConfig {
    let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        while let Ok((tcp, _)) = listener.accept().await {
            tokio::spawn(serve_openvpn(tcp, wrap.clone(), opts.clone()));
        }
    });

    let ca = TEST_CA.replace('\n', "\\n");
    let yaml = format!(
        "name: o\ntype: openvpn\nserver: 127.0.0.1\nport: {}\nproto: tcp\nusername: u\npassword: p\ncipher: AES-256-GCM\nca: \"{}\"\n{}",
        addr.port(),
        ca,
        extra_yaml,
    );
    let entry: ProxyEntry = serde_yaml_ng::from_str(&yaml).unwrap();
    OpenVpnOutboundConfig::from_proxy(&entry).unwrap()
}

async fn start_server() -> OpenVpnOutboundConfig {
    start_server_with(ServerWrap::None, "").await
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
    start_udp_server_with(drop_first_reset, ServerWrap::None, "").await
}

async fn start_udp_server_with(drop_first_reset: bool, wrap: ServerWrap, extra_yaml: &str) -> OpenVpnOutboundConfig {
    let socket = UdpSocket::bind((Ipv4Addr::LOCALHOST, 0)).await.unwrap();
    let addr = socket.local_addr().unwrap();
    let socket = Arc::new(socket);
    tokio::spawn(serve_openvpn_udp(socket, drop_first_reset, wrap, ServerOpts::default()));

    let ca = TEST_CA.replace('\n', "\\n");
    let yaml = format!(
        "name: o\ntype: openvpn\nserver: 127.0.0.1\nport: {}\nproto: udp\nusername: u\npassword: p\ncipher: AES-256-GCM\nca: \"{}\"\n{}",
        addr.port(),
        ca,
        extra_yaml,
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

/// A server whose `PUSH_REPLY` includes `ifconfig-ipv6` and whose inner stack
/// terminates IPv6 flows.
async fn start_ipv6_server() -> OpenVpnOutboundConfig {
    start_server_opts(
        ServerWrap::None,
        "",
        ServerOpts {
            push_extra: format!(",ifconfig-ipv6 {CLIENT_TUN_IP_V6}/64 {SERVER_TUN_IP_V6}"),
            ipv6: true,
            ..ServerOpts::default()
        },
    )
    .await
}

#[tokio::test]
async fn openvpn_tcp_relays_an_inner_ipv6_target() {
    let config = start_ipv6_server().await;
    let target = TargetAddr::Ip(SocketAddr::new(IpAddr::V6(INNER_IP_V6), INNER_PORT));

    let mut stream = tokio::time::timeout(Duration::from_secs(20), openvpn::connect(&config, &target))
        .await
        .expect("connect did not time out")
        .expect("openvpn connect");

    let payload = b"hello openvpn ipv6 tunnel";
    stream.write_all(payload).await.unwrap();

    let mut got = vec![0u8; payload.len()];
    stream.read_exact(&mut got).await.unwrap();
    assert_eq!(&got, payload);
}

#[tokio::test]
async fn openvpn_relays_inner_udp_datagrams_to_an_ipv6_target() {
    let config = start_ipv6_server().await;
    let target = TargetAddr::Ip(SocketAddr::new(IpAddr::V6(INNER_IP_V6), INNER_PORT));

    let assoc = tokio::time::timeout(Duration::from_secs(20), openvpn::connect_udp(&config, &target))
        .await
        .expect("connect did not time out")
        .expect("openvpn connect_udp");

    for payload in [&b"first inner ipv6 datagram"[..], &b"second one"[..]] {
        assoc.send(payload).await.unwrap();
        let got = tokio::time::timeout(Duration::from_secs(10), assoc.recv())
            .await
            .expect("recv did not time out")
            .expect("udp recv");
        assert_eq!(got, payload);
    }
}

#[tokio::test]
async fn openvpn_rejects_ipv6_target_without_ifconfig_ipv6() {
    // The server pushes only an IPv4 `ifconfig`, so an inner IPv6 destination
    // must be rejected with a clear error rather than hanging.
    let config = start_server().await;
    let target = TargetAddr::Ip(SocketAddr::new(IpAddr::V6(INNER_IP_V6), INNER_PORT));

    let result = tokio::time::timeout(Duration::from_secs(20), openvpn::connect(&config, &target))
        .await
        .expect("connect did not time out");
    let err = match result {
        Ok(_) => panic!("IPv6 target must be rejected"),
        Err(err) => err,
    };
    assert!(err.to_string().contains("no IPv6 address"), "{err}");
}

#[tokio::test]
async fn openvpn_tcp_round_trips_with_tls_auth() {
    // Client `tls-auth` + `key-direction 1` against a server that verifies and
    // HMAC-wraps every control packet with the mirrored key direction.
    let key = static_key_text().replace('\n', "\\n");
    let extra = format!("tls-auth: \"{key}\"\nkey-direction: 1\n");
    let config = start_server_with(ServerWrap::tls_auth(), &extra).await;
    let target = TargetAddr::Ip(SocketAddr::new(IpAddr::V4(INNER_IP), INNER_PORT));

    let mut stream = tokio::time::timeout(Duration::from_secs(20), openvpn::connect(&config, &target))
        .await
        .expect("connect did not time out")
        .expect("openvpn connect");

    let payload = b"hello tls-auth tunnel";
    stream.write_all(payload).await.unwrap();

    let mut got = vec![0u8; payload.len()];
    stream.read_exact(&mut got).await.unwrap();
    assert_eq!(&got, payload);
}

#[tokio::test]
async fn openvpn_udp_round_trips_with_tls_crypt() {
    // Client `tls-crypt` over the UDP transport against a server that encrypts
    // + authenticates every control packet with the mirrored key slots.
    let key = static_key_text().replace('\n', "\\n");
    let extra = format!("tls-crypt: \"{key}\"\n");
    let config = start_udp_server_with(false, ServerWrap::tls_crypt(), &extra).await;
    let target = TargetAddr::Ip(SocketAddr::new(IpAddr::V4(INNER_IP), INNER_PORT));

    let mut stream = tokio::time::timeout(Duration::from_secs(20), openvpn::connect(&config, &target))
        .await
        .expect("connect did not time out")
        .expect("openvpn connect");

    let payload = b"hello tls-crypt tunnel";
    stream.write_all(payload).await.unwrap();

    let mut got = vec![0u8; payload.len()];
    stream.read_exact(&mut got).await.unwrap();
    assert_eq!(&got, payload);
}

#[tokio::test]
async fn openvpn_tcp_round_trips_with_tls_crypt() {
    let key = static_key_text().replace('\n', "\\n");
    let extra = format!("tls-crypt: \"{key}\"\n");
    let config = start_server_with(ServerWrap::tls_crypt(), &extra).await;
    let target = TargetAddr::Ip(SocketAddr::new(IpAddr::V4(INNER_IP), INNER_PORT));

    let mut stream = tokio::time::timeout(Duration::from_secs(20), openvpn::connect(&config, &target))
        .await
        .expect("connect did not time out")
        .expect("openvpn connect");

    let payload = b"tls-crypt over tcp";
    stream.write_all(payload).await.unwrap();

    let mut got = vec![0u8; payload.len()];
    stream.read_exact(&mut got).await.unwrap();
    assert_eq!(&got, payload);
}

#[tokio::test]
async fn openvpn_tcp_round_trips_with_tls_crypt_v2() {
    // Client `tls-crypt-v2`: the V3 hard reset carries the server-wrapped WKc,
    // which the fake server unwraps (verifying Kc + metadata) before speaking
    // tls-crypt with the recovered per-client key.
    let key = v2_client_key_text().replace('\n', "\\n");
    let extra = format!("tls-crypt-v2: \"{key}\"\n");
    let config = start_server_with(ServerWrap::tls_crypt_v2(), &extra).await;
    let target = TargetAddr::Ip(SocketAddr::new(IpAddr::V4(INNER_IP), INNER_PORT));

    let mut stream = tokio::time::timeout(Duration::from_secs(20), openvpn::connect(&config, &target))
        .await
        .expect("connect did not time out")
        .expect("openvpn connect");

    let payload = b"tls-crypt-v2 over tcp";
    stream.write_all(payload).await.unwrap();

    let mut got = vec![0u8; payload.len()];
    stream.read_exact(&mut got).await.unwrap();
    assert_eq!(&got, payload);
}

#[tokio::test]
async fn openvpn_udp_round_trips_with_tls_crypt_v2() {
    // Same over UDP, with the first hard reset dropped so the retransmitted V3
    // reset (carrying the WKc again) completes the handshake.
    let key = v2_client_key_text().replace('\n', "\\n");
    let extra = format!("tls-crypt-v2: \"{key}\"\n");
    let config = start_udp_server_with(true, ServerWrap::tls_crypt_v2(), &extra).await;
    let target = TargetAddr::Ip(SocketAddr::new(IpAddr::V4(INNER_IP), INNER_PORT));

    let mut stream = tokio::time::timeout(Duration::from_secs(20), openvpn::connect(&config, &target))
        .await
        .expect("connect did not time out")
        .expect("openvpn connect");

    let payload = b"tls-crypt-v2 over udp";
    stream.write_all(payload).await.unwrap();

    let mut got = vec![0u8; payload.len()];
    stream.read_exact(&mut got).await.unwrap();
    assert_eq!(&got, payload);
}

#[tokio::test]
async fn openvpn_udp_round_trips_with_tls_auth() {
    let key = static_key_text().replace('\n', "\\n");
    let extra = format!("tls-auth: \"{key}\"\nkey-direction: 1\n");
    let config = start_udp_server_with(false, ServerWrap::tls_auth(), &extra).await;
    let target = TargetAddr::Ip(SocketAddr::new(IpAddr::V4(INNER_IP), INNER_PORT));

    let mut stream = tokio::time::timeout(Duration::from_secs(20), openvpn::connect(&config, &target))
        .await
        .expect("connect did not time out")
        .expect("openvpn connect");

    let payload = b"tls-auth over udp";
    stream.write_all(payload).await.unwrap();

    let mut got = vec![0u8; payload.len()];
    stream.read_exact(&mut got).await.unwrap();
    assert_eq!(&got, payload);
}

#[tokio::test]
async fn openvpn_sends_keepalive_pings_when_idle() {
    // The server pushes `ping 1`; an idle client must emit the fixed 16-byte
    // data-channel ping over the AEAD data channel within a couple of seconds.
    let (ping_tx, mut ping_rx) = mpsc::unbounded_channel::<()>();
    let opts = ServerOpts {
        push_extra: ",ping 1".into(),
        ping_seen: Some(ping_tx),
        ..Default::default()
    };
    let config = start_server_opts(ServerWrap::None, "", opts).await;
    let target = TargetAddr::Ip(SocketAddr::new(IpAddr::V4(INNER_IP), INNER_PORT));

    let _stream = tokio::time::timeout(Duration::from_secs(20), openvpn::connect(&config, &target))
        .await
        .expect("connect did not time out")
        .expect("openvpn connect");

    tokio::time::timeout(Duration::from_secs(10), ping_rx.recv())
        .await
        .expect("client never sent a keepalive ping")
        .expect("ping channel closed");
}

#[tokio::test]
async fn openvpn_tears_down_after_ping_restart_expires() {
    // The server pushes `ping-restart 1` and then goes silent; the client must
    // tear the tunnel down (upstream's SIGUSR1 restart), surfacing as EOF /
    // error on the relayed stream instead of hanging forever.
    let opts = ServerOpts {
        push_extra: ",ping-restart 1".into(),
        ..Default::default()
    };
    let config = start_server_opts(ServerWrap::None, "", opts).await;
    let target = TargetAddr::Ip(SocketAddr::new(IpAddr::V4(INNER_IP), INNER_PORT));

    let mut stream = tokio::time::timeout(Duration::from_secs(20), openvpn::connect(&config, &target))
        .await
        .expect("connect did not time out")
        .expect("openvpn connect");

    // The tunnel works while traffic flows...
    let payload = b"before restart";
    stream.write_all(payload).await.unwrap();
    let mut got = vec![0u8; payload.len()];
    stream.read_exact(&mut got).await.unwrap();
    assert_eq!(&got, payload);

    // ...then, once the server goes silent past ping-restart, the device loop
    // exits and the relayed stream ends rather than blocking.
    let mut buf = [0u8; 16];
    let ended = tokio::time::timeout(Duration::from_secs(10), stream.read(&mut buf))
        .await
        .expect("stream did not end after ping-restart");
    assert!(matches!(ended, Ok(0) | Err(_)), "expected EOF or error, got {ended:?}");
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

#[tokio::test]
async fn openvpn_client_initiated_rekey_rotates_data_keys() {
    // With `reneg-sec 2`, the client must soft-reset on its own, run a fresh
    // TLS + key-method-2 exchange under key id 1, and keep the relayed stream
    // alive across the rotation. The server reports the first data packet it
    // decrypts under the new key.
    let (rekey_tx, mut rekey_rx) = mpsc::unbounded_channel::<u8>();
    let opts = ServerOpts {
        rekey_done: Some(rekey_tx),
        ..Default::default()
    };
    let config = start_server_opts(ServerWrap::None, "reneg-sec: 2\n", opts).await;
    let target = TargetAddr::Ip(SocketAddr::new(IpAddr::V4(INNER_IP), INNER_PORT));

    let mut stream = tokio::time::timeout(Duration::from_secs(20), openvpn::connect(&config, &target))
        .await
        .expect("connect did not time out")
        .expect("openvpn connect");

    // Traffic works under the initial key id 0...
    let payload = b"before rekey";
    stream.write_all(payload).await.unwrap();
    let mut got = vec![0u8; payload.len()];
    stream.read_exact(&mut got).await.unwrap();
    assert_eq!(&got, payload);

    // ...the client renegotiates on the reneg-sec timer...
    let deadline = Instant::now() + Duration::from_secs(15);
    loop {
        assert!(Instant::now() < deadline, "client never rotated data keys");
        // Keep traffic flowing so the server sees data under the new key.
        stream.write_all(b"tick").await.unwrap();
        let mut tick = [0u8; 4];
        stream.read_exact(&mut tick).await.unwrap();
        match tokio::time::timeout(Duration::from_millis(200), rekey_rx.recv()).await {
            Ok(Some(key_id)) => {
                assert_eq!(key_id, 1, "first renegotiated key id");
                break;
            }
            Ok(None) => panic!("rekey channel closed"),
            Err(_) => {}
        }
    }

    // ...and the same relayed stream still round-trips afterwards.
    let payload = b"after rekey";
    stream.write_all(payload).await.unwrap();
    let mut got = vec![0u8; payload.len()];
    stream.read_exact(&mut got).await.unwrap();
    assert_eq!(&got, payload);
}

#[tokio::test]
async fn openvpn_answers_a_server_initiated_soft_reset() {
    // The server sends `P_CONTROL_SOFT_RESET_V1` under key id 1 shortly after
    // the data phase starts; the client must answer with its own soft reset,
    // renegotiate, and move its data channel to the new key without dropping
    // the relayed stream. `reneg-sec 0` disables the client's own timer so the
    // rotation can only come from answering the server.
    let (rekey_tx, mut rekey_rx) = mpsc::unbounded_channel::<u8>();
    let opts = ServerOpts {
        rekey_done: Some(rekey_tx),
        initiate_rekey_after: Some(Duration::from_secs(1)),
        ..Default::default()
    };
    let config = start_server_opts(ServerWrap::None, "reneg-sec: 0\n", opts).await;
    let target = TargetAddr::Ip(SocketAddr::new(IpAddr::V4(INNER_IP), INNER_PORT));

    let mut stream = tokio::time::timeout(Duration::from_secs(20), openvpn::connect(&config, &target))
        .await
        .expect("connect did not time out")
        .expect("openvpn connect");

    let payload = b"before server rekey";
    stream.write_all(payload).await.unwrap();
    let mut got = vec![0u8; payload.len()];
    stream.read_exact(&mut got).await.unwrap();
    assert_eq!(&got, payload);

    let deadline = Instant::now() + Duration::from_secs(15);
    loop {
        assert!(Instant::now() < deadline, "client never answered the soft reset");
        stream.write_all(b"tick").await.unwrap();
        let mut tick = [0u8; 4];
        stream.read_exact(&mut tick).await.unwrap();
        match tokio::time::timeout(Duration::from_millis(200), rekey_rx.recv()).await {
            Ok(Some(key_id)) => {
                assert_eq!(key_id, 1, "server-chosen key id");
                break;
            }
            Ok(None) => panic!("rekey channel closed"),
            Err(_) => {}
        }
    }

    let payload = b"after server rekey";
    stream.write_all(payload).await.unwrap();
    let mut got = vec![0u8; payload.len()];
    stream.read_exact(&mut got).await.unwrap();
    assert_eq!(&got, payload);
}
