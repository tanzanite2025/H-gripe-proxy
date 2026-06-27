//! End-to-end proof that traffic flows through a Shadowsocks (AEAD) outbound:
//! a SOCKS5 client -> gripe inbound -> Shadowsocks outbound -> fake SS server.
//!
//! The fake server is an *independent* server-side implementation of the
//! Shadowsocks AEAD stream: it derives the master key with `EVP_BytesToKey`,
//! reads the client salt, derives the per-session subkey via HKDF-SHA1,
//! decrypts each length-prefixed AEAD chunk, strips the SOCKS5 target address
//! from the head of the plaintext stream, then echoes the remaining application
//! bytes back as its own salted, AEAD-chunked response. Driving a real session
//! against a separate implementation proves the key schedule, nonce handling
//! and chunk framing all compose correctly for every supported cipher — not
//! just that the client round-trips with itself.

use std::net::{IpAddr, Ipv4Addr, SocketAddr};

use aes_gcm::aead::generic_array::GenericArray;
use aes_gcm::aead::{Aead, KeyInit, Payload};
use aes_gcm::{Aes128Gcm, Aes256Gcm};
use chacha20poly1305::ChaCha20Poly1305;
use learn_gripe::{GripeConfig, GripeKernel, OutboundMode, ShadowsocksCipher, ShadowsocksOutboundConfig};
use md5::Md5;
use sha1::{Digest, Sha1};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

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
            panic!("2017 fake server does not handle Shadowsocks 2022 ciphers")
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

fn increment_nonce(nonce: &mut [u8; 12]) {
    for byte in nonce.iter_mut() {
        *byte = byte.wrapping_add(1);
        if *byte != 0 {
            break;
        }
    }
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
                panic!("2017 fake server does not handle Shadowsocks 2022 ciphers")
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

/// Read and decrypt one AEAD chunk; returns its plaintext, or `None` at EOF.
async fn read_chunk(stream: &mut TcpStream, cipher: &AeadCipher, nonce: &mut [u8; 12]) -> Option<Vec<u8>> {
    let mut sealed_len = [0u8; 2 + TAG_LEN];
    stream.read_exact(&mut sealed_len).await.ok()?;
    let len_pt = cipher.open(nonce, &sealed_len);
    increment_nonce(nonce);
    let clen = u16::from_be_bytes([len_pt[0], len_pt[1]]) as usize;
    let mut sealed = vec![0u8; clen + TAG_LEN];
    stream.read_exact(&mut sealed).await.ok()?;
    let pt = cipher.open(nonce, &sealed);
    increment_nonce(nonce);
    Some(pt)
}

/// Length of the SOCKS5 address at the head of `buf`, once enough bytes are
/// present; `None` if more bytes are still needed.
fn address_len(buf: &[u8]) -> Option<usize> {
    match buf.first()? {
        0x01 => (buf.len() >= 7).then_some(7),
        0x04 => (buf.len() >= 19).then_some(19),
        0x03 => {
            let host_len = *buf.get(1)? as usize;
            let total = 2 + host_len + 2;
            (buf.len() >= total).then_some(total)
        }
        other => panic!("unexpected SOCKS5 atyp {other:#x}"),
    }
}

/// Read the client salt + address, then echo every application byte back to the
/// client as a salted, AEAD-chunked Shadowsocks response.
async fn serve_shadowsocks(mut stream: TcpStream, cipher: ShadowsocksCipher) {
    let master = evp_bytes_to_key(PASSWORD.as_bytes(), key_size(cipher));
    let salt_len = key_size(cipher);

    // Client direction: read salt, derive subkey, decrypt the chunk stream.
    let mut salt = vec![0u8; salt_len];
    stream.read_exact(&mut salt).await.unwrap();
    let read_subkey = hkdf_sha1(&master, &salt, SS_SUBKEY_INFO, salt_len);
    let read_cipher = AeadCipher::new(cipher, &read_subkey);
    let mut read_nonce = [0u8; 12];

    // Server direction: send our own salt, derive the response subkey.
    let mut resp_salt = vec![0u8; salt_len];
    for (i, b) in resp_salt.iter_mut().enumerate() {
        *b = (i as u8).wrapping_mul(7).wrapping_add(1);
    }
    stream.write_all(&resp_salt).await.unwrap();
    let write_subkey = hkdf_sha1(&master, &resp_salt, SS_SUBKEY_INFO, salt_len);
    let write_cipher = AeadCipher::new(cipher, &write_subkey);
    let mut write_nonce = [0u8; 12];

    let mut head: Vec<u8> = Vec::new();
    let mut address_consumed = false;

    while let Some(plain) = read_chunk(&mut stream, &read_cipher, &mut read_nonce).await {
        let mut data = plain;
        if !address_consumed {
            head.extend_from_slice(&data);
            match address_len(&head) {
                Some(n) => {
                    address_consumed = true;
                    data = head.split_off(n);
                }
                None => continue,
            }
        }
        if data.is_empty() {
            continue;
        }
        let sealed_len = write_cipher.seal(&write_nonce, &(data.len() as u16).to_be_bytes());
        increment_nonce(&mut write_nonce);
        let sealed_payload = write_cipher.seal(&write_nonce, &data);
        increment_nonce(&mut write_nonce);
        if stream.write_all(&sealed_len).await.is_err()
            || stream.write_all(&sealed_payload).await.is_err()
            || stream.flush().await.is_err()
        {
            return;
        }
    }
}

async fn spawn_server(cipher: ShadowsocksCipher) -> SocketAddr {
    let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        while let Ok((stream, _)) = listener.accept().await {
            tokio::spawn(serve_shadowsocks(stream, cipher));
        }
    });
    addr
}

async fn socks5_connect(proxy: SocketAddr, target: SocketAddr) -> TcpStream {
    let mut stream = TcpStream::connect(proxy).await.unwrap();
    stream.write_all(&[0x05, 0x01, 0x00]).await.unwrap();
    let mut selection = [0u8; 2];
    stream.read_exact(&mut selection).await.unwrap();
    assert_eq!(selection, [0x05, 0x00]);

    let ip = match target.ip() {
        IpAddr::V4(v4) => v4.octets(),
        IpAddr::V6(_) => panic!("test uses IPv4"),
    };
    let mut request = vec![0x05, 0x01, 0x00, 0x01];
    request.extend_from_slice(&ip);
    request.extend_from_slice(&target.port().to_be_bytes());
    stream.write_all(&request).await.unwrap();

    let mut reply = [0u8; 10];
    stream.read_exact(&mut reply).await.unwrap();
    assert_eq!(reply[0], 0x05);
    assert_eq!(reply[1], 0x00, "SOCKS5 reply should be success");
    stream
}

/// Drive a SOCKS5 round trip through the kernel built from `outbound`, sending
/// the payload in two writes to exercise multiple body chunks, and assert it is
/// echoed back unchanged.
async fn assert_relays(outbound: OutboundMode, payload: &[u8]) {
    let handle = GripeKernel::start(GripeConfig {
        socks_listen: SocketAddr::from((Ipv4Addr::LOCALHOST, 0)),
        outbound,
    })
    .await
    .unwrap();

    let dummy_target = SocketAddr::from((Ipv4Addr::new(1, 2, 3, 4), 443));
    let mut conn = socks5_connect(handle.local_addr(), dummy_target).await;

    let split = payload.len() / 2;
    conn.write_all(&payload[..split]).await.unwrap();
    conn.flush().await.unwrap();
    conn.write_all(&payload[split..]).await.unwrap();

    let mut buf = vec![0u8; payload.len()];
    conn.read_exact(&mut buf).await.unwrap();
    assert_eq!(buf, payload);

    handle.shutdown().await;
}

fn config(server: SocketAddr, cipher: ShadowsocksCipher) -> OutboundMode {
    OutboundMode::Shadowsocks(Box::new(ShadowsocksOutboundConfig {
        server: server.ip().to_string(),
        port: server.port(),
        cipher,
        key: evp_bytes_to_key(PASSWORD.as_bytes(), key_size(cipher)),
    }))
}

#[tokio::test]
async fn relays_through_aes_128_gcm() {
    let server = spawn_server(ShadowsocksCipher::Aes128Gcm).await;
    assert_relays(
        config(server, ShadowsocksCipher::Aes128Gcm),
        b"hello shadowsocks aes-128-gcm",
    )
    .await;
}

#[tokio::test]
async fn relays_through_aes_256_gcm() {
    let server = spawn_server(ShadowsocksCipher::Aes256Gcm).await;
    assert_relays(
        config(server, ShadowsocksCipher::Aes256Gcm),
        b"hello shadowsocks aes-256-gcm",
    )
    .await;
}

#[tokio::test]
async fn relays_through_chacha20_ietf_poly1305() {
    let server = spawn_server(ShadowsocksCipher::Chacha20IetfPoly1305).await;
    assert_relays(
        config(server, ShadowsocksCipher::Chacha20IetfPoly1305),
        b"hello shadowsocks chacha20-ietf-poly1305",
    )
    .await;
}

#[tokio::test]
async fn relays_large_payload_spanning_multiple_chunks() {
    // Larger than one 0x3FFF-byte chunk to exercise the chunk loop on both ends.
    let server = spawn_server(ShadowsocksCipher::Aes256Gcm).await;
    let payload: Vec<u8> = (0..40_000u32).map(|i| (i % 251) as u8).collect();
    assert_relays(config(server, ShadowsocksCipher::Aes256Gcm), &payload).await;
}
