//! End-to-end proof that UDP rides a Shadowsocks 2022 (SIP022) outbound:
//! SOCKS5 UDP ASSOCIATE -> gripe inbound -> Shadowsocks 2022 UDP -> fake SS-2022
//! server.
//!
//! The fake server is an *independent* server-side implementation of the
//! Shadowsocks 2022 UDP packet format. For the AES methods it decrypts the
//! 16-byte separate header (`session ID | packet ID`) with the PSK via a single
//! AES-ECB block, derives the session subkey with BLAKE3 `derive_key` over
//! `PSK || session ID`, and opens the AES-GCM body with the nonce taken from
//! the plaintext header's last 12 bytes. For the chacha method it opens the
//! body with XChaCha20-Poly1305 keyed by the PSK and a 24-byte nonce, reading
//! the session/packet IDs from the body's main header. It then strips the
//! client main header, echoes the payload back inside a freshly-built
//! server-to-client packet, and the client validates and unwraps it. Driving a
//! real association against a separate implementation proves the separate
//! header, BLAKE3 key schedule, nonce derivation and both header formats
//! interoperate for every 2022 method.

use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::time::{SystemTime, UNIX_EPOCH};

use aes::cipher::{BlockDecrypt, BlockEncrypt, generic_array::GenericArray as BlockArray};
use aes::{Aes128, Aes256};
use aes_gcm::aead::generic_array::GenericArray;
use aes_gcm::aead::{Aead, KeyInit, Payload};
use aes_gcm::{Aes128Gcm, Aes256Gcm};
use chacha20poly1305::{XChaCha20Poly1305, XNonce};
use learn_gripe::{GripeConfig, GripeKernel, OutboundMode, ShadowsocksCipher, ShadowsocksOutboundConfig};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpStream, UdpSocket};

const SUBKEY_CONTEXT: &str = "shadowsocks 2022 session subkey";
const HEADER_TYPE_CLIENT: u8 = 0;
const HEADER_TYPE_SERVER: u8 = 1;
const SESSION_ID_LEN: usize = 8;
const SEPARATE_HEADER_LEN: usize = 16;
const XNONCE_LEN: usize = 24;

/// A fixed 32-byte PSK; the 16-byte methods use the leading half.
const PSK32: [u8; 32] = [
    0x9e, 0x1c, 0x4f, 0x77, 0x20, 0xab, 0x3d, 0x55, 0x01, 0x88, 0xfe, 0x6a, 0x42, 0xc3, 0x10, 0x77, 0xbb, 0x0d, 0x2e,
    0x91, 0x64, 0xa5, 0x37, 0xee, 0x12, 0x49, 0x8c, 0xd0, 0x3f, 0x71, 0x06, 0x5b,
];

// --- minimal independent Shadowsocks 2022 UDP crypto for the fake server ----

fn key_size(cipher: ShadowsocksCipher) -> usize {
    match cipher {
        ShadowsocksCipher::Blake3Aes128Gcm => 16,
        ShadowsocksCipher::Blake3Aes256Gcm | ShadowsocksCipher::Blake3Chacha20Poly1305 => 32,
        other => panic!("this fake server only handles Shadowsocks 2022 ciphers, got {other:?}"),
    }
}

fn psk(cipher: ShadowsocksCipher) -> Vec<u8> {
    PSK32[..key_size(cipher)].to_vec()
}

fn is_chacha(cipher: ShadowsocksCipher) -> bool {
    matches!(cipher, ShadowsocksCipher::Blake3Chacha20Poly1305)
}

fn session_subkey(cipher: ShadowsocksCipher, psk: &[u8], session_id: &[u8]) -> Vec<u8> {
    let material = [psk, session_id].concat();
    let derived = blake3::derive_key(SUBKEY_CONTEXT, &material);
    derived[..key_size(cipher)].to_vec()
}

fn unix_timestamp() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs()
}

enum AeadCipher {
    Aes128(Box<Aes128Gcm>),
    Aes256(Box<Aes256Gcm>),
}

impl AeadCipher {
    fn new(cipher: ShadowsocksCipher, subkey: &[u8]) -> Self {
        match cipher {
            ShadowsocksCipher::Blake3Aes128Gcm => {
                AeadCipher::Aes128(Box::new(Aes128Gcm::new_from_slice(subkey).unwrap()))
            }
            ShadowsocksCipher::Blake3Aes256Gcm => {
                AeadCipher::Aes256(Box::new(Aes256Gcm::new_from_slice(subkey).unwrap()))
            }
            other => panic!("not an AES 2022 cipher: {other:?}"),
        }
    }

    fn seal(&self, nonce: &[u8; 12], pt: &[u8]) -> Vec<u8> {
        let payload = Payload { msg: pt, aad: &[] };
        match self {
            AeadCipher::Aes128(c) => c.encrypt(GenericArray::from_slice(nonce), payload),
            AeadCipher::Aes256(c) => c.encrypt(GenericArray::from_slice(nonce), payload),
        }
        .unwrap()
    }

    fn open(&self, nonce: &[u8; 12], ct: &[u8]) -> Vec<u8> {
        let payload = Payload { msg: ct, aad: &[] };
        match self {
            AeadCipher::Aes128(c) => c.decrypt(GenericArray::from_slice(nonce), payload),
            AeadCipher::Aes256(c) => c.decrypt(GenericArray::from_slice(nonce), payload),
        }
        .unwrap()
    }
}

/// Length of the SOCKS5 address at the head of `buf`.
fn address_len(buf: &[u8]) -> usize {
    match buf[0] {
        0x01 => 7,
        0x04 => 19,
        0x03 => 2 + buf[1] as usize + 2,
        other => panic!("unexpected SOCKS5 atyp {other:#x}"),
    }
}

fn aes_encrypt_block(cipher: ShadowsocksCipher, key: &[u8], block: &mut [u8; SEPARATE_HEADER_LEN]) {
    let ga = BlockArray::from_mut_slice(block);
    match cipher {
        ShadowsocksCipher::Blake3Aes128Gcm => Aes128::new_from_slice(key).unwrap().encrypt_block(ga),
        ShadowsocksCipher::Blake3Aes256Gcm => Aes256::new_from_slice(key).unwrap().encrypt_block(ga),
        other => panic!("not an AES 2022 cipher: {other:?}"),
    }
}

fn aes_decrypt_block(cipher: ShadowsocksCipher, key: &[u8], block: &mut [u8; SEPARATE_HEADER_LEN]) {
    let ga = BlockArray::from_mut_slice(block);
    match cipher {
        ShadowsocksCipher::Blake3Aes128Gcm => Aes128::new_from_slice(key).unwrap().decrypt_block(ga),
        ShadowsocksCipher::Blake3Aes256Gcm => Aes256::new_from_slice(key).unwrap().decrypt_block(ga),
        other => panic!("not an AES 2022 cipher: {other:?}"),
    }
}

/// Decode one client packet, returning `(client_session_id, payload)`.
fn open_client_packet(cipher: ShadowsocksCipher, psk: &[u8], datagram: &[u8]) -> ([u8; SESSION_ID_LEN], Vec<u8>) {
    if is_chacha(cipher) {
        let (nonce, sealed) = datagram.split_at(XNONCE_LEN);
        let body = XChaCha20Poly1305::new_from_slice(psk)
            .unwrap()
            .decrypt(XNonce::from_slice(nonce), Payload { msg: sealed, aad: &[] })
            .unwrap();
        let mut session_id = [0u8; SESSION_ID_LEN];
        session_id.copy_from_slice(&body[..SESSION_ID_LEN]);
        // body: session ID(8) | packet ID(8) | type | timestamp | padding len(2) | padding | socks_addr | payload
        let main = &body[SESSION_ID_LEN + 8..];
        let payload = strip_client_main_header(main);
        (session_id, payload)
    } else {
        let mut separate = [0u8; SEPARATE_HEADER_LEN];
        separate.copy_from_slice(&datagram[..SEPARATE_HEADER_LEN]);
        aes_decrypt_block(cipher, psk, &mut separate);
        let mut session_id = [0u8; SESSION_ID_LEN];
        session_id.copy_from_slice(&separate[..SESSION_ID_LEN]);
        let subkey = session_subkey(cipher, psk, &session_id);
        let mut nonce = [0u8; 12];
        nonce.copy_from_slice(&separate[4..]);
        let body = AeadCipher::new(cipher, &subkey).open(&nonce, &datagram[SEPARATE_HEADER_LEN..]);
        // body: type | timestamp | padding len(2) | padding | socks_addr | payload
        let payload = strip_client_main_header(&body);
        (session_id, payload)
    }
}

/// Strip a client-to-server main header (`type | timestamp | padding len |
/// padding | socks_addr`) and return the trailing payload.
fn strip_client_main_header(main: &[u8]) -> Vec<u8> {
    assert_eq!(main[0], HEADER_TYPE_CLIENT, "client packet header type");
    let pad_len = u16::from_be_bytes([main[9], main[10]]) as usize;
    let addr_at = 11 + pad_len;
    let alen = address_len(&main[addr_at..]);
    main[addr_at + alen..].to_vec()
}

/// Build a server-to-client packet echoing `payload` to `client_session_id`.
fn build_server_packet(
    cipher: ShadowsocksCipher,
    psk: &[u8],
    client_session_id: &[u8; SESSION_ID_LEN],
    server_session_id: &[u8; SESSION_ID_LEN],
    packet_id: u64,
    target: SocketAddr,
    payload: &[u8],
) -> Vec<u8> {
    // Shared main-header tail: type | timestamp | client session ID | padding
    // len(0) | socks_addr | payload.
    let mut tail = Vec::new();
    tail.push(HEADER_TYPE_SERVER);
    tail.extend_from_slice(&unix_timestamp().to_be_bytes());
    tail.extend_from_slice(client_session_id);
    tail.extend_from_slice(&0u16.to_be_bytes());
    encode_socks_addr(&mut tail, target);
    tail.extend_from_slice(payload);

    if is_chacha(cipher) {
        let mut nonce = [0u8; XNONCE_LEN];
        for (i, b) in nonce.iter_mut().enumerate() {
            *b = (i as u8).wrapping_mul(13).wrapping_add(7);
        }
        let mut body = Vec::new();
        body.extend_from_slice(server_session_id);
        body.extend_from_slice(&packet_id.to_be_bytes());
        body.extend_from_slice(&tail);
        let sealed = XChaCha20Poly1305::new_from_slice(psk)
            .unwrap()
            .encrypt(XNonce::from_slice(&nonce), Payload { msg: &body, aad: &[] })
            .unwrap();
        let mut packet = nonce.to_vec();
        packet.extend_from_slice(&sealed);
        packet
    } else {
        let mut separate = [0u8; SEPARATE_HEADER_LEN];
        separate[..SESSION_ID_LEN].copy_from_slice(server_session_id);
        separate[SESSION_ID_LEN..].copy_from_slice(&packet_id.to_be_bytes());
        let subkey = session_subkey(cipher, psk, server_session_id);
        let mut nonce = [0u8; 12];
        nonce.copy_from_slice(&separate[4..]);
        let sealed = AeadCipher::new(cipher, &subkey).seal(&nonce, &tail);
        aes_encrypt_block(cipher, psk, &mut separate);
        let mut packet = separate.to_vec();
        packet.extend_from_slice(&sealed);
        packet
    }
}

fn encode_socks_addr(buf: &mut Vec<u8>, addr: SocketAddr) {
    match addr.ip() {
        IpAddr::V4(v4) => {
            buf.push(0x01);
            buf.extend_from_slice(&v4.octets());
        }
        IpAddr::V6(v6) => {
            buf.push(0x04);
            buf.extend_from_slice(&v6.octets());
        }
    }
    buf.extend_from_slice(&addr.port().to_be_bytes());
}

async fn serve_ss2022_udp(socket: UdpSocket, cipher: ShadowsocksCipher) {
    let psk = psk(cipher);
    let server_session_id = [0xa5u8; SESSION_ID_LEN];
    let mut packet_id: u64 = 0;
    let mut buf = vec![0u8; 64 * 1024];
    loop {
        let Ok((n, from)) = socket.recv_from(&mut buf).await else {
            return;
        };
        let (client_session_id, payload) = open_client_packet(cipher, &psk, &buf[..n]);
        // Echo to an arbitrary target address (the client strips and ignores it).
        let target = SocketAddr::from((Ipv4Addr::new(1, 2, 3, 4), 443));
        let reply = build_server_packet(
            cipher,
            &psk,
            &client_session_id,
            &server_session_id,
            packet_id,
            target,
            &payload,
        );
        packet_id = packet_id.wrapping_add(1);
        if socket.send_to(&reply, from).await.is_err() {
            return;
        }
    }
}

async fn spawn_fake_ss2022_udp_server(cipher: ShadowsocksCipher) -> SocketAddr {
    let socket = UdpSocket::bind((Ipv4Addr::LOCALHOST, 0)).await.unwrap();
    let addr = socket.local_addr().unwrap();
    tokio::spawn(serve_ss2022_udp(socket, cipher));
    addr
}

async fn socks5_udp_associate(proxy: SocketAddr) -> (TcpStream, SocketAddr) {
    let mut stream = TcpStream::connect(proxy).await.unwrap();
    stream.write_all(&[0x05, 0x01, 0x00]).await.unwrap();
    let mut selection = [0u8; 2];
    stream.read_exact(&mut selection).await.unwrap();
    assert_eq!(selection, [0x05, 0x00]);
    stream
        .write_all(&[0x05, 0x03, 0x00, 0x01, 0, 0, 0, 0, 0, 0])
        .await
        .unwrap();
    let mut reply = [0u8; 10];
    stream.read_exact(&mut reply).await.unwrap();
    assert_eq!(reply[1], 0x00, "expected ASSOCIATE success reply");
    let ip = Ipv4Addr::new(reply[4], reply[5], reply[6], reply[7]);
    let port = u16::from_be_bytes([reply[8], reply[9]]);
    (stream, SocketAddr::from((ip, port)))
}

fn udp_datagram_ipv4(dst: SocketAddr, payload: &[u8]) -> Vec<u8> {
    let ip = match dst.ip() {
        IpAddr::V4(v4) => v4.octets(),
        IpAddr::V6(_) => panic!("test uses IPv4"),
    };
    let mut datagram = vec![0x00, 0x00, 0x00, 0x01];
    datagram.extend_from_slice(&ip);
    datagram.extend_from_slice(&dst.port().to_be_bytes());
    datagram.extend_from_slice(payload);
    datagram
}

/// Payload offset in a relayed reply datagram (skips RSV/FRAG and the address).
fn payload_offset(buf: &[u8]) -> usize {
    match buf[3] {
        0x01 => 3 + 1 + 4 + 2,
        0x04 => 3 + 1 + 16 + 2,
        0x03 => 3 + 1 + 1 + buf[4] as usize + 2,
        other => panic!("unexpected reply atyp {other}"),
    }
}

fn config(server: SocketAddr, cipher: ShadowsocksCipher) -> OutboundMode {
    OutboundMode::Shadowsocks(Box::new(ShadowsocksOutboundConfig {
        server: server.ip().to_string(),
        port: server.port(),
        cipher,
        key: psk(cipher),
    }))
}

/// Drive one client datagram through the kernel and assert the echo round-trips.
async fn assert_udp_relays(cipher: ShadowsocksCipher, payload: &[u8]) {
    let server = spawn_fake_ss2022_udp_server(cipher).await;
    let handle = GripeKernel::start(GripeConfig {
        socks_listen: SocketAddr::from((Ipv4Addr::LOCALHOST, 0)),
        outbound: config(server, cipher),
    })
    .await
    .unwrap();

    let (_control, relay) = socks5_udp_associate(handle.local_addr()).await;
    let client = UdpSocket::bind((Ipv4Addr::LOCALHOST, 0)).await.unwrap();
    let dst = SocketAddr::from((Ipv4Addr::new(1, 2, 3, 4), 443));
    client.send_to(&udp_datagram_ipv4(dst, payload), relay).await.unwrap();

    let mut buf = [0u8; 2048];
    let (n, from) = client.recv_from(&mut buf).await.unwrap();
    assert_eq!(from, relay, "reply must come from the relay socket");
    let offset = payload_offset(&buf[..n]);
    assert_eq!(&buf[offset..n], payload, "payload must be echoed verbatim");

    handle.shutdown().await;
}

#[tokio::test]
async fn udp_relays_through_2022_blake3_aes_128_gcm() {
    assert_udp_relays(ShadowsocksCipher::Blake3Aes128Gcm, b"ss 2022 udp aes-128-gcm").await;
}

#[tokio::test]
async fn udp_relays_through_2022_blake3_aes_256_gcm() {
    assert_udp_relays(ShadowsocksCipher::Blake3Aes256Gcm, b"ss 2022 udp aes-256-gcm").await;
}

#[tokio::test]
async fn udp_relays_through_2022_blake3_chacha20_poly1305() {
    assert_udp_relays(
        ShadowsocksCipher::Blake3Chacha20Poly1305,
        b"ss 2022 udp chacha20-poly1305",
    )
    .await;
}
