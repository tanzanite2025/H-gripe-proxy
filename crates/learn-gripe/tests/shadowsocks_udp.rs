//! End-to-end proof that UDP rides a Shadowsocks (AEAD) outbound:
//! SOCKS5 UDP ASSOCIATE -> gripe inbound -> Shadowsocks UDP -> fake SS server.
//!
//! The fake server is an *independent* server-side implementation of the
//! Shadowsocks AEAD UDP packet format: for each datagram it splits off the
//! per-packet salt, derives the subkey via HKDF-SHA1, opens the AEAD body with
//! an all-zero nonce, strips the SOCKS5 destination address, then echoes the
//! payload back as its own freshly-salted, AEAD-sealed packet. Driving a real
//! association against a separate implementation proves the per-packet key
//! schedule, the zero nonce and the address framing all interoperate — for
//! every supported cipher — rather than just round-tripping with ourselves.

use std::net::{IpAddr, Ipv4Addr, SocketAddr};

use aes_gcm::aead::generic_array::GenericArray;
use aes_gcm::aead::{Aead, KeyInit, Payload};
use aes_gcm::{Aes128Gcm, Aes256Gcm};
use chacha20poly1305::ChaCha20Poly1305;
use learn_gripe::{GripeConfig, GripeKernel, OutboundMode, ShadowsocksCipher, ShadowsocksOutboundConfig};
use md5::Md5;
use sha1::{Digest, Sha1};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpStream, UdpSocket};

const PASSWORD: &str = "correct horse battery staple";
const SS_SUBKEY_INFO: &[u8] = b"ss-subkey";
const TAG_LEN: usize = 16;

// --- minimal independent Shadowsocks crypto for the fake server -----------

fn key_size(cipher: ShadowsocksCipher) -> usize {
    match cipher {
        ShadowsocksCipher::Aes128Gcm => 16,
        ShadowsocksCipher::Aes256Gcm | ShadowsocksCipher::Chacha20IetfPoly1305 => 32,
        ShadowsocksCipher::Blake3Aes128Gcm
        | ShadowsocksCipher::Blake3Aes256Gcm
        | ShadowsocksCipher::Blake3Chacha20Poly1305 => {
            panic!("2017 fake udp server does not handle Shadowsocks 2022 ciphers")
        }
    }
}

fn evp_bytes_to_key(password: &[u8], key_len: usize) -> Vec<u8> {
    let mut key = Vec::with_capacity(key_len);
    let mut prev: Vec<u8> = Vec::new();
    while key.len() < key_len {
        let mut hasher = Md5::new();
        hasher.update(&prev);
        hasher.update(password);
        let digest: [u8; 16] = hasher.finalize().into();
        key.extend_from_slice(&digest);
        prev = digest.to_vec();
    }
    key.truncate(key_len);
    key
}

fn sha1(parts: &[&[u8]]) -> [u8; 20] {
    let mut hasher = Sha1::new();
    for part in parts {
        hasher.update(part);
    }
    hasher.finalize().into()
}

fn hmac_sha1(key: &[u8], msg: &[u8]) -> [u8; 20] {
    const BLOCK: usize = 64;
    let mut block = [0u8; BLOCK];
    if key.len() > BLOCK {
        block[..20].copy_from_slice(&sha1(&[key]));
    } else {
        block[..key.len()].copy_from_slice(key);
    }
    let mut ipad = [0u8; BLOCK];
    let mut opad = [0u8; BLOCK];
    for i in 0..BLOCK {
        ipad[i] = block[i] ^ 0x36;
        opad[i] = block[i] ^ 0x5c;
    }
    let inner = sha1(&[&ipad, msg]);
    sha1(&[&opad, &inner])
}

fn hkdf_sha1(ikm: &[u8], salt: &[u8], info: &[u8], length: usize) -> Vec<u8> {
    let prk = hmac_sha1(salt, ikm);
    let mut okm = Vec::with_capacity(length);
    let mut prev: Vec<u8> = Vec::new();
    let mut counter: u8 = 1;
    while okm.len() < length {
        let mut input = Vec::new();
        input.extend_from_slice(&prev);
        input.extend_from_slice(info);
        input.push(counter);
        let block = hmac_sha1(&prk, &input);
        okm.extend_from_slice(&block);
        prev = block.to_vec();
        counter += 1;
    }
    okm.truncate(length);
    okm
}

enum AeadCipher {
    Aes128(Box<Aes128Gcm>),
    Aes256(Box<Aes256Gcm>),
    Chacha(Box<ChaCha20Poly1305>),
}

impl AeadCipher {
    fn new(cipher: ShadowsocksCipher, subkey: &[u8]) -> Self {
        match cipher {
            ShadowsocksCipher::Aes128Gcm => AeadCipher::Aes128(Box::new(Aes128Gcm::new_from_slice(subkey).unwrap())),
            ShadowsocksCipher::Aes256Gcm => AeadCipher::Aes256(Box::new(Aes256Gcm::new_from_slice(subkey).unwrap())),
            ShadowsocksCipher::Chacha20IetfPoly1305 => {
                AeadCipher::Chacha(Box::new(ChaCha20Poly1305::new_from_slice(subkey).unwrap()))
            }
            ShadowsocksCipher::Blake3Aes128Gcm
            | ShadowsocksCipher::Blake3Aes256Gcm
            | ShadowsocksCipher::Blake3Chacha20Poly1305 => {
                panic!("2017 fake udp server does not handle Shadowsocks 2022 ciphers")
            }
        }
    }

    fn seal(&self, nonce: &[u8; 12], pt: &[u8]) -> Vec<u8> {
        let payload = Payload { msg: pt, aad: &[] };
        match self {
            AeadCipher::Aes128(c) => c.encrypt(GenericArray::from_slice(nonce), payload),
            AeadCipher::Aes256(c) => c.encrypt(GenericArray::from_slice(nonce), payload),
            AeadCipher::Chacha(c) => c.encrypt(GenericArray::from_slice(nonce), payload),
        }
        .unwrap()
    }

    fn open(&self, nonce: &[u8; 12], ct: &[u8]) -> Vec<u8> {
        let payload = Payload { msg: ct, aad: &[] };
        match self {
            AeadCipher::Aes128(c) => c.decrypt(GenericArray::from_slice(nonce), payload),
            AeadCipher::Aes256(c) => c.decrypt(GenericArray::from_slice(nonce), payload),
            AeadCipher::Chacha(c) => c.decrypt(GenericArray::from_slice(nonce), payload),
        }
        .unwrap()
    }
}

/// Open each Shadowsocks UDP packet and echo its plaintext (`addr | payload`)
/// back as a fresh salted, AEAD-sealed packet; the client strips the address
/// and keeps the payload.
async fn serve_shadowsocks_udp(socket: UdpSocket, cipher: ShadowsocksCipher) {
    let master = evp_bytes_to_key(PASSWORD.as_bytes(), key_size(cipher));
    let salt_len = key_size(cipher);
    let zero = [0u8; 12];
    let mut buf = vec![0u8; 64 * 1024];

    loop {
        let Ok((n, from)) = socket.recv_from(&mut buf).await else {
            return;
        };
        let datagram = &buf[..n];
        let (salt, sealed) = datagram.split_at(salt_len);
        let subkey = hkdf_sha1(&master, salt, SS_SUBKEY_INFO, salt_len);
        let plain = AeadCipher::new(cipher, &subkey).open(&zero, sealed);

        // The reply uses a fresh salt -> fresh subkey, so the zero nonce is safe.
        let mut reply_salt = vec![0u8; salt_len];
        for (i, b) in reply_salt.iter_mut().enumerate() {
            *b = (i as u8).wrapping_mul(7).wrapping_add(1);
        }
        let reply_subkey = hkdf_sha1(&master, &reply_salt, SS_SUBKEY_INFO, salt_len);
        let sealed_reply = AeadCipher::new(cipher, &reply_subkey).seal(&zero, &plain);
        let mut packet = reply_salt;
        packet.extend_from_slice(&sealed_reply);
        if socket.send_to(&packet, from).await.is_err() {
            return;
        }
    }
}

async fn spawn_fake_ss_udp_server(cipher: ShadowsocksCipher) -> SocketAddr {
    let socket = UdpSocket::bind((Ipv4Addr::LOCALHOST, 0)).await.unwrap();
    let addr = socket.local_addr().unwrap();
    tokio::spawn(serve_shadowsocks_udp(socket, cipher));
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
        key: evp_bytes_to_key(PASSWORD.as_bytes(), key_size(cipher)),
        plugin: None,
    }))
}

/// Drive one client datagram through the kernel and assert the echo round-trips.
async fn assert_udp_relays(cipher: ShadowsocksCipher, payload: &[u8]) {
    let server = spawn_fake_ss_udp_server(cipher).await;
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
async fn udp_relays_through_aes_128_gcm() {
    assert_udp_relays(ShadowsocksCipher::Aes128Gcm, b"ss udp aes-128-gcm").await;
}

#[tokio::test]
async fn udp_relays_through_aes_256_gcm() {
    assert_udp_relays(ShadowsocksCipher::Aes256Gcm, b"ss udp aes-256-gcm").await;
}

#[tokio::test]
async fn udp_relays_through_chacha20_ietf_poly1305() {
    assert_udp_relays(
        ShadowsocksCipher::Chacha20IetfPoly1305,
        b"ss udp chacha20-ietf-poly1305",
    )
    .await;
}
