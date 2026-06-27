//! End-to-end proof that UDP rides a ShadowsocksR outbound:
//! SOCKS5 UDP ASSOCIATE -> gripe inbound -> SSR UDP -> fake SSR UDP server.
//!
//! The fake server is an *independent* server-side implementation of the SSR
//! UDP packet format (upstream shadowsocksr's `encrypt_all` + the protocol
//! layer's `*_udp_*` framing). Each datagram is encrypted on its own: a random
//! IV plus a one-shot stream cipher, with the protocol layer wrapping the
//! `socks5_addr | payload` plaintext (obfs does not apply to UDP). For each
//! datagram the server strips the IV, runs the stream cipher, removes/verifies
//! the protocol framing, recovers the SOCKS5 destination and payload, then
//! echoes `socks5_addr | payload` back through the same stack in reverse.
//!
//! Driving a real association against a separate implementation proves the
//! per-packet key schedule, the protocol auth tags (including auth_chain_a's
//! RC4 + xorshift padding) and the address framing all interoperate — for every
//! supported cipher × protocol — rather than just round-tripping with ourselves.

use std::net::{IpAddr, Ipv4Addr, SocketAddr};

use aes::Aes128;
use aes::cipher::{BlockEncrypt, KeyInit as AesKeyInit};
use aes_gcm::aead::generic_array::GenericArray;
use hmac::{Hmac, Mac};
use md5::Md5;
use sha1::Sha1;

use learn_gripe::{GripeConfig, GripeKernel, OutboundMode, SsrCipher, SsrObfs, SsrOutboundConfig, SsrProtocol};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpStream, UdpSocket};

const PASSWORD: &str = "ssr-udp-test-password";

// ---------------------------------------------------------------------------
// Independent SSR crypto primitives for the fake server
// ---------------------------------------------------------------------------

fn evp_bytes_to_key(password: &[u8], key_len: usize) -> Vec<u8> {
    use md5::Digest;
    let mut key = Vec::with_capacity(key_len);
    let mut prev = Vec::new();
    while key.len() < key_len {
        let mut hasher = Md5::new();
        hasher.update(&prev);
        hasher.update(password);
        let hash: [u8; 16] = hasher.finalize().into();
        key.extend_from_slice(&hash);
        prev = hash.to_vec();
    }
    key.truncate(key_len);
    key
}

fn hmac_sha1(key: &[u8], msg: &[u8]) -> [u8; 20] {
    let mut mac = <Hmac<Sha1> as Mac>::new_from_slice(key).unwrap();
    mac.update(msg);
    mac.finalize().into_bytes().into()
}

fn hmac_md5(key: &[u8], msg: &[u8]) -> [u8; 16] {
    let mut mac = <Hmac<Md5> as Mac>::new_from_slice(key).unwrap();
    mac.update(msg);
    mac.finalize().into_bytes().into()
}

fn hmac_tag(use_sha1: bool, key: &[u8], msg: &[u8]) -> Vec<u8> {
    if use_sha1 {
        hmac_sha1(key, msg).to_vec()
    } else {
        hmac_md5(key, msg).to_vec()
    }
}

/// Plain RC4 (key used directly), matching auth_chain_a's UDP keystream.
fn rc4_apply(key: &[u8], data: &mut [u8]) {
    let mut s = [0u8; 256];
    for (i, b) in s.iter_mut().enumerate() {
        *b = i as u8;
    }
    let mut j: u8 = 0;
    for i in 0..256 {
        j = j.wrapping_add(s[i]).wrapping_add(key[i % key.len()]);
        s.swap(i, j as usize);
    }
    let (mut i, mut j) = (0u8, 0u8);
    for byte in data.iter_mut() {
        i = i.wrapping_add(1);
        j = j.wrapping_add(s[i as usize]);
        s.swap(i as usize, j as usize);
        let k = s[s[i as usize].wrapping_add(s[j as usize]) as usize];
        *byte ^= k;
    }
}

fn base64_encode(data: &[u8]) -> Vec<u8> {
    const TABLE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = Vec::new();
    for chunk in data.chunks(3) {
        let b0 = chunk[0] as u32;
        let b1 = *chunk.get(1).unwrap_or(&0) as u32;
        let b2 = *chunk.get(2).unwrap_or(&0) as u32;
        let n = (b0 << 16) | (b1 << 8) | b2;
        out.push(TABLE[((n >> 18) & 0x3f) as usize]);
        out.push(TABLE[((n >> 12) & 0x3f) as usize]);
        out.push(if chunk.len() > 1 {
            TABLE[((n >> 6) & 0x3f) as usize]
        } else {
            b'='
        });
        out.push(if chunk.len() > 2 {
            TABLE[(n & 0x3f) as usize]
        } else {
            b'='
        });
    }
    out
}

/// Upstream shadowsocksr's xorshift128plus variant.
struct ShiftRng {
    v0: u64,
    v1: u64,
}

impl ShiftRng {
    const MOV_MASK: u64 = (1u64 << (64 - 23)) - 1;

    fn from_bin(bin: &[u8]) -> Self {
        let mut b = [0u8; 16];
        let n = bin.len().min(16);
        b[..n].copy_from_slice(&bin[..n]);
        Self {
            v0: u64::from_le_bytes(b[..8].try_into().unwrap()),
            v1: u64::from_le_bytes(b[8..16].try_into().unwrap()),
        }
    }

    fn next(&mut self) -> u64 {
        let mut x = self.v0;
        let y = self.v1;
        self.v0 = y;
        x ^= (x & Self::MOV_MASK) << 23;
        x ^= y ^ (x >> 17) ^ (y >> 26);
        self.v1 = x;
        x.wrapping_add(y)
    }
}

fn udp_rnd_data_len(last_hash: &[u8]) -> usize {
    (ShiftRng::from_bin(last_hash).next() % 127) as usize
}

/// Independent one-shot stream cipher for the fake server.
enum FakeCipher {
    Aes128Cfb {
        cipher: Aes128,
        feedback: [u8; 16],
        keystream: [u8; 16],
        pos: usize,
        encrypting: bool,
    },
    Aes256Cfb {
        cipher: aes::Aes256,
        feedback: [u8; 16],
        keystream: [u8; 16],
        pos: usize,
        encrypting: bool,
    },
    Chacha20 {
        key: [u8; 32],
        nonce: [u8; 12],
    },
    Rc4 {
        s: Box<[u8; 256]>,
        i: u8,
        j: u8,
    },
    None,
}

impl FakeCipher {
    fn new(kind: SsrCipher, key: &[u8], iv: &[u8], encrypting: bool) -> Self {
        match kind {
            SsrCipher::Aes128Cfb => FakeCipher::Aes128Cfb {
                cipher: Aes128::new(GenericArray::from_slice(&key[..16])),
                feedback: iv[..16].try_into().unwrap(),
                keystream: [0u8; 16],
                pos: 16,
                encrypting,
            },
            SsrCipher::Aes256Cfb => FakeCipher::Aes256Cfb {
                cipher: aes::Aes256::new(GenericArray::from_slice(&key[..32])),
                feedback: iv[..16].try_into().unwrap(),
                keystream: [0u8; 16],
                pos: 16,
                encrypting,
            },
            SsrCipher::Chacha20Ietf => FakeCipher::Chacha20 {
                key: key[..32].try_into().unwrap(),
                nonce: iv[..12].try_into().unwrap(),
            },
            SsrCipher::Rc4Md5 => {
                use md5::Digest;
                let mut hasher = Md5::new();
                hasher.update(key);
                hasher.update(iv);
                let derived: [u8; 16] = hasher.finalize().into();
                let mut s = Box::new([0u8; 256]);
                for (i, byte) in s.iter_mut().enumerate() {
                    *byte = i as u8;
                }
                let mut j: u8 = 0;
                for i in 0..256 {
                    j = j.wrapping_add(s[i]).wrapping_add(derived[i % derived.len()]);
                    s.swap(i, j as usize);
                }
                FakeCipher::Rc4 { s, i: 0, j: 0 }
            }
            SsrCipher::None => FakeCipher::None,
        }
    }

    fn update(&mut self, data: &mut [u8]) {
        match self {
            FakeCipher::Aes128Cfb {
                cipher,
                feedback,
                keystream,
                pos,
                encrypting,
            } => cfb_update(cipher, feedback, keystream, pos, *encrypting, data),
            FakeCipher::Aes256Cfb {
                cipher,
                feedback,
                keystream,
                pos,
                encrypting,
            } => cfb_update(cipher, feedback, keystream, pos, *encrypting, data),
            FakeCipher::Chacha20 { key, nonce } => {
                use chacha20::ChaCha20;
                use chacha20::cipher::{KeyIvInit, StreamCipher};
                let mut c = ChaCha20::new(GenericArray::from_slice(key), GenericArray::from_slice(nonce));
                c.apply_keystream(data);
            }
            FakeCipher::Rc4 { s, i, j } => {
                for byte in data.iter_mut() {
                    *i = i.wrapping_add(1);
                    *j = j.wrapping_add(s[*i as usize]);
                    s.swap(*i as usize, *j as usize);
                    let k = s[s[*i as usize].wrapping_add(s[*j as usize]) as usize];
                    *byte ^= k;
                }
            }
            FakeCipher::None => {}
        }
    }
}

fn cfb_update<C: BlockEncrypt>(
    cipher: &C,
    feedback: &mut [u8; 16],
    keystream: &mut [u8; 16],
    pos: &mut usize,
    encrypting: bool,
    data: &mut [u8],
) {
    for byte in data.iter_mut() {
        if *pos >= 16 {
            let mut block = GenericArray::clone_from_slice(&*feedback);
            cipher.encrypt_block(&mut block);
            keystream.copy_from_slice(block.as_slice());
            *pos = 0;
        }
        if encrypting {
            *byte ^= keystream[*pos];
            feedback[*pos] = *byte;
        } else {
            let ct = *byte;
            *byte ^= keystream[*pos];
            feedback[*pos] = ct;
        }
        *pos += 1;
    }
}

// ---------------------------------------------------------------------------
// Fake SSR UDP server
// ---------------------------------------------------------------------------

/// Strip the server-side protocol framing from a decrypted client packet,
/// returning the inner `socks5_addr | payload`.
fn server_protocol_post(protocol: SsrProtocol, key: &[u8], framed: &[u8]) -> Vec<u8> {
    match protocol {
        SsrProtocol::Origin => framed.to_vec(),
        SsrProtocol::AuthAes128Sha1 | SsrProtocol::AuthAes128Md5 => {
            let use_sha1 = matches!(protocol, SsrProtocol::AuthAes128Sha1);
            let tag = hmac_tag(use_sha1, key, &framed[..framed.len() - 4]);
            assert_eq!(tag[..4], framed[framed.len() - 4..], "client auth_aes128 udp HMAC");
            // Strip uid(4) + HMAC(4).
            framed[..framed.len() - 8].to_vec()
        }
        SsrProtocol::AuthChainA => {
            let tag = hmac_md5(key, &framed[..framed.len() - 1]);
            assert_eq!(tag[0], framed[framed.len() - 1], "client auth_chain_a udp HMAC");
            let authdata = &framed[framed.len() - 8..framed.len() - 5]; // 3 bytes
            let md5data = hmac_md5(key, authdata);
            let rand_len = udp_rnd_data_len(&md5data);
            let rc4_key = [base64_encode(key), base64_encode(&md5data)].concat();
            let end = framed.len() - 8 - rand_len;
            let mut out = framed[..end].to_vec();
            rc4_apply(&rc4_key, &mut out);
            out
        }
    }
}

/// Apply the server-side protocol framing to a reply's inner `socks5_addr |
/// payload`. `seq` makes the per-packet authdata deterministic for the test.
fn server_protocol_pre(protocol: SsrProtocol, key: &[u8], inner: &[u8], seq: u8) -> Vec<u8> {
    match protocol {
        SsrProtocol::Origin => inner.to_vec(),
        SsrProtocol::AuthAes128Sha1 | SsrProtocol::AuthAes128Md5 => {
            let use_sha1 = matches!(protocol, SsrProtocol::AuthAes128Sha1);
            let mut out = inner.to_vec();
            let tag = hmac_tag(use_sha1, key, inner);
            out.extend_from_slice(&tag[..4]);
            out
        }
        SsrProtocol::AuthChainA => {
            let authdata = [seq; 7];
            let md5data = hmac_md5(key, &authdata);
            let rand_len = udp_rnd_data_len(&md5data);
            let rc4_key = [base64_encode(key), base64_encode(&md5data)].concat();
            let mut out = inner.to_vec();
            rc4_apply(&rc4_key, &mut out);
            out.extend(std::iter::repeat_n(0xAB, rand_len));
            out.extend_from_slice(&authdata);
            let tag = hmac_md5(key, &out);
            out.push(tag[0]);
            out
        }
    }
}

/// Decrypt one client datagram to its inner `socks5_addr | payload`.
fn server_open(cipher: SsrCipher, protocol: SsrProtocol, key: &[u8], datagram: &[u8]) -> Vec<u8> {
    let iv_len = cipher.iv_size();
    let (iv, body) = datagram.split_at(iv_len);
    let mut framed = body.to_vec();
    FakeCipher::new(cipher, key, iv, false).update(&mut framed);
    server_protocol_post(protocol, key, &framed)
}

/// Seal one reply datagram from its inner `socks5_addr | payload`.
fn server_seal(cipher: SsrCipher, protocol: SsrProtocol, key: &[u8], inner: &[u8], seq: u8) -> Vec<u8> {
    let mut framed = server_protocol_pre(protocol, key, inner, seq);
    let iv_len = cipher.iv_size();
    let mut iv = vec![0u8; iv_len];
    for (i, b) in iv.iter_mut().enumerate() {
        *b = (i as u8).wrapping_mul(37).wrapping_add(seq);
    }
    FakeCipher::new(cipher, key, &iv, true).update(&mut framed);
    let mut packet = iv;
    packet.extend_from_slice(&framed);
    packet
}

fn socks5_addr_len(buf: &[u8]) -> usize {
    match buf[0] {
        0x01 => 1 + 4 + 2,
        0x03 => 1 + 1 + buf[1] as usize + 2,
        0x04 => 1 + 16 + 2,
        other => panic!("unknown SOCKS5 address type 0x{other:02x}"),
    }
}

async fn serve_ssr_udp(socket: UdpSocket, cipher: SsrCipher, protocol: SsrProtocol) {
    let key = evp_bytes_to_key(PASSWORD.as_bytes(), cipher.key_size());
    let mut buf = vec![0u8; 64 * 1024];
    let mut seq: u8 = 1;
    loop {
        let Ok((n, from)) = socket.recv_from(&mut buf).await else {
            return;
        };
        let inner = server_open(cipher, protocol, &key, &buf[..n]);
        // Validate the address parses, then echo `socks5_addr | payload` back.
        let _ = socks5_addr_len(&inner);
        let reply = server_seal(cipher, protocol, &key, &inner, seq);
        seq = seq.wrapping_add(1);
        if socket.send_to(&reply, from).await.is_err() {
            return;
        }
    }
}

async fn spawn_fake_ssr_udp_server(cipher: SsrCipher, protocol: SsrProtocol) -> SocketAddr {
    let socket = UdpSocket::bind((Ipv4Addr::LOCALHOST, 0)).await.unwrap();
    let addr = socket.local_addr().unwrap();
    tokio::spawn(serve_ssr_udp(socket, cipher, protocol));
    addr
}

// ---------------------------------------------------------------------------
// SOCKS5 UDP ASSOCIATE harness
// ---------------------------------------------------------------------------

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

fn payload_offset(buf: &[u8]) -> usize {
    match buf[3] {
        0x01 => 3 + 1 + 4 + 2,
        0x04 => 3 + 1 + 16 + 2,
        0x03 => 3 + 1 + 1 + buf[4] as usize + 2,
        other => panic!("unexpected reply atyp {other}"),
    }
}

fn config(server: SocketAddr, cipher: SsrCipher, protocol: SsrProtocol) -> OutboundMode {
    OutboundMode::Ssr(Box::new(SsrOutboundConfig {
        server: server.ip().to_string(),
        port: server.port(),
        cipher,
        key: evp_bytes_to_key(PASSWORD.as_bytes(), cipher.key_size()),
        protocol,
        protocol_param: String::new(),
        obfs: SsrObfs::Plain,
        obfs_param: String::new(),
    }))
}

/// Drive one client datagram through the kernel and assert the echo round-trips.
async fn assert_udp_relays(cipher: SsrCipher, protocol: SsrProtocol, payload: &[u8]) {
    let server = spawn_fake_ssr_udp_server(cipher, protocol).await;
    let handle = GripeKernel::start(GripeConfig {
        socks_listen: SocketAddr::from((Ipv4Addr::LOCALHOST, 0)),
        outbound: config(server, cipher, protocol),
    })
    .await
    .unwrap();

    let (_control, relay) = socks5_udp_associate(handle.local_addr()).await;
    let client = UdpSocket::bind((Ipv4Addr::LOCALHOST, 0)).await.unwrap();
    let dst = SocketAddr::from((Ipv4Addr::new(1, 2, 3, 4), 443));
    client.send_to(&udp_datagram_ipv4(dst, payload), relay).await.unwrap();

    let mut buf = [0u8; 4096];
    let (n, from) = client.recv_from(&mut buf).await.unwrap();
    assert_eq!(from, relay, "reply must come from the relay socket");
    let offset = payload_offset(&buf[..n]);
    assert_eq!(&buf[offset..n], payload, "payload must be echoed verbatim");

    handle.shutdown().await;
}

// ---------------------------------------------------------------------------
// Cipher coverage (origin protocol)
// ---------------------------------------------------------------------------

#[tokio::test]
async fn udp_origin_aes_128_cfb() {
    assert_udp_relays(SsrCipher::Aes128Cfb, SsrProtocol::Origin, b"ssr udp aes-128-cfb").await;
}

#[tokio::test]
async fn udp_origin_aes_256_cfb() {
    assert_udp_relays(SsrCipher::Aes256Cfb, SsrProtocol::Origin, b"ssr udp aes-256-cfb").await;
}

#[tokio::test]
async fn udp_origin_chacha20_ietf() {
    assert_udp_relays(SsrCipher::Chacha20Ietf, SsrProtocol::Origin, b"ssr udp chacha20-ietf").await;
}

#[tokio::test]
async fn udp_origin_rc4_md5() {
    assert_udp_relays(SsrCipher::Rc4Md5, SsrProtocol::Origin, b"ssr udp rc4-md5").await;
}

#[tokio::test]
async fn udp_origin_none() {
    assert_udp_relays(SsrCipher::None, SsrProtocol::Origin, b"ssr udp none").await;
}

// ---------------------------------------------------------------------------
// Protocol coverage
// ---------------------------------------------------------------------------

#[tokio::test]
async fn udp_auth_aes128_sha1() {
    assert_udp_relays(
        SsrCipher::Aes128Cfb,
        SsrProtocol::AuthAes128Sha1,
        b"ssr udp auth_aes128_sha1",
    )
    .await;
}

#[tokio::test]
async fn udp_auth_aes128_md5() {
    assert_udp_relays(
        SsrCipher::Aes256Cfb,
        SsrProtocol::AuthAes128Md5,
        b"ssr udp auth_aes128_md5",
    )
    .await;
}

#[tokio::test]
async fn udp_auth_chain_a() {
    assert_udp_relays(
        SsrCipher::Chacha20Ietf,
        SsrProtocol::AuthChainA,
        b"ssr udp auth_chain_a",
    )
    .await;
}

#[tokio::test]
async fn udp_auth_chain_a_with_rc4_md5() {
    assert_udp_relays(
        SsrCipher::Rc4Md5,
        SsrProtocol::AuthChainA,
        b"ssr udp auth_chain_a rc4-md5",
    )
    .await;
}

#[tokio::test]
async fn udp_relays_large_payload() {
    let payload = vec![0x5au8; 1400];
    assert_udp_relays(SsrCipher::Aes128Cfb, SsrProtocol::AuthChainA, &payload).await;
}
