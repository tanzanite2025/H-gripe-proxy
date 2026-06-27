//! End-to-end proof that traffic flows through a Shadowsocks 2022 (SIP022)
//! outbound: a SOCKS5 client -> gripe inbound -> Shadowsocks 2022 outbound ->
//! fake SS-2022 server.
//!
//! The fake server is an *independent* server-side implementation of the
//! Shadowsocks 2022 stream: it derives the per-session subkey with BLAKE3
//! `derive_key` over `PSK || salt`, reads the client salt, decrypts the
//! fixed-length request header (`type | timestamp | variable-header length`)
//! and the variable-length header (`socks_addr | padding`), then echoes the
//! application bytes back as a salted response whose header echoes the request
//! salt and carries the first payload chunk. Driving a real session against a
//! separate implementation proves the BLAKE3 key schedule, nonce handling and
//! both header formats compose correctly for every 2022 method — not just that
//! the client round-trips with itself.

use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::time::{SystemTime, UNIX_EPOCH};

use aes_gcm::aead::generic_array::GenericArray;
use aes_gcm::aead::{Aead, KeyInit, Payload};
use aes_gcm::{Aes128Gcm, Aes256Gcm};
use chacha20poly1305::ChaCha20Poly1305;
use learn_gripe::{GripeConfig, GripeKernel, OutboundMode, ShadowsocksCipher, ShadowsocksOutboundConfig};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

const SUBKEY_CONTEXT: &str = "shadowsocks 2022 session subkey";
const TAG_LEN: usize = 16;
const HEADER_TYPE_REQUEST: u8 = 0;
const HEADER_TYPE_RESPONSE: u8 = 1;

/// A fixed 32-byte PSK; the 16-byte methods use the leading half.
const PSK32: [u8; 32] = [
    0x9e, 0x1c, 0x4f, 0x77, 0x20, 0xab, 0x3d, 0x55, 0x01, 0x88, 0xfe, 0x6a, 0x42, 0xc3, 0x10, 0x77, 0xbb, 0x0d, 0x2e,
    0x91, 0x64, 0xa5, 0x37, 0xee, 0x12, 0x49, 0x8c, 0xd0, 0x3f, 0x71, 0x06, 0x5b,
];

// --- minimal independent Shadowsocks 2022 crypto for the fake server -------

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

/// BLAKE3 session subkey derivation, computed independently of the kernel.
fn session_subkey(cipher: ShadowsocksCipher, psk: &[u8], salt: &[u8]) -> Vec<u8> {
    let material = [psk, salt].concat();
    let derived = blake3::derive_key(SUBKEY_CONTEXT, &material);
    derived[..key_size(cipher)].to_vec()
}

fn unix_timestamp() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs()
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
            ShadowsocksCipher::Blake3Aes128Gcm => {
                AeadCipher::Aes128(Box::new(Aes128Gcm::new_from_slice(subkey).unwrap()))
            }
            ShadowsocksCipher::Blake3Aes256Gcm => {
                AeadCipher::Aes256(Box::new(Aes256Gcm::new_from_slice(subkey).unwrap()))
            }
            ShadowsocksCipher::Blake3Chacha20Poly1305 => {
                AeadCipher::Chacha(Box::new(ChaCha20Poly1305::new_from_slice(subkey).unwrap()))
            }
            other => panic!("this fake server only handles Shadowsocks 2022 ciphers, got {other:?}"),
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

/// Read and decrypt one fixed-size sealed blob of `plain_len` plaintext bytes.
async fn read_sealed(stream: &mut TcpStream, cipher: &AeadCipher, nonce: &mut [u8; 12], plain_len: usize) -> Vec<u8> {
    let mut sealed = vec![0u8; plain_len + TAG_LEN];
    stream.read_exact(&mut sealed).await.unwrap();
    let pt = cipher.open(nonce, &sealed);
    increment_nonce(nonce);
    pt
}

/// Read and decrypt one length-prefixed body chunk; `None` at EOF.
async fn read_body_chunk(stream: &mut TcpStream, cipher: &AeadCipher, nonce: &mut [u8; 12]) -> Option<Vec<u8>> {
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

/// Length of the SOCKS5 address at the head of `buf`.
fn address_len(buf: &[u8]) -> usize {
    match buf[0] {
        0x01 => 7,
        0x04 => 19,
        0x03 => 2 + buf[1] as usize + 2,
        other => panic!("unexpected SOCKS5 atyp {other:#x}"),
    }
}

/// Response side: lazily writes the salt + response header on the first piece of
/// echoed data (with that piece as the first payload chunk), then standard
/// body chunks for everything after.
struct Responder {
    cipher: ShadowsocksCipher,
    psk: Vec<u8>,
    request_salt: Vec<u8>,
    started: Option<(AeadCipher, [u8; 12])>,
}

impl Responder {
    async fn echo(&mut self, stream: &mut TcpStream, data: &[u8]) -> std::io::Result<()> {
        if data.is_empty() {
            return Ok(());
        }
        match &mut self.started {
            None => {
                // Deterministic response salt (distinct from the request salt).
                let salt_len = self.psk.len();
                let mut resp_salt = vec![0u8; salt_len];
                for (i, b) in resp_salt.iter_mut().enumerate() {
                    *b = (i as u8).wrapping_mul(11).wrapping_add(5);
                }
                let subkey = session_subkey(self.cipher, &self.psk, &resp_salt);
                let cipher = AeadCipher::new(self.cipher, &subkey);
                let mut nonce = [0u8; 12];

                let mut fixed = Vec::new();
                fixed.push(HEADER_TYPE_RESPONSE);
                fixed.extend_from_slice(&unix_timestamp().to_be_bytes());
                fixed.extend_from_slice(&self.request_salt);
                fixed.extend_from_slice(&(data.len() as u16).to_be_bytes());

                let sealed_fixed = cipher.seal(&nonce, &fixed);
                increment_nonce(&mut nonce);
                let sealed_first = cipher.seal(&nonce, data);
                increment_nonce(&mut nonce);

                stream.write_all(&resp_salt).await?;
                stream.write_all(&sealed_fixed).await?;
                stream.write_all(&sealed_first).await?;
                stream.flush().await?;
                self.started = Some((cipher, nonce));
            }
            Some((cipher, nonce)) => {
                let sealed_len = cipher.seal(nonce, &(data.len() as u16).to_be_bytes());
                increment_nonce(nonce);
                let sealed_payload = cipher.seal(nonce, data);
                increment_nonce(nonce);
                stream.write_all(&sealed_len).await?;
                stream.write_all(&sealed_payload).await?;
                stream.flush().await?;
            }
        }
        Ok(())
    }
}

async fn serve_ss2022(mut stream: TcpStream, cipher: ShadowsocksCipher) {
    let psk = psk(cipher);
    let salt_len = key_size(cipher);

    // Client direction: read salt, derive subkey via BLAKE3.
    let mut salt = vec![0u8; salt_len];
    stream.read_exact(&mut salt).await.unwrap();
    let read_subkey = session_subkey(cipher, &psk, &salt);
    let read_cipher = AeadCipher::new(cipher, &read_subkey);
    let mut read_nonce = [0u8; 12];

    // Fixed-length request header: type | timestamp | variable-header length.
    let fixed = read_sealed(&mut stream, &read_cipher, &mut read_nonce, 1 + 8 + 2).await;
    assert_eq!(fixed[0], HEADER_TYPE_REQUEST, "request header type");
    let var_len = u16::from_be_bytes([fixed[9], fixed[10]]) as usize;

    // Variable-length request header: socks_addr | padding length | padding |
    // initial payload (empty for our client).
    let var = read_sealed(&mut stream, &read_cipher, &mut read_nonce, var_len).await;
    let alen = address_len(&var);
    let pad_len = u16::from_be_bytes([var[alen], var[alen + 1]]) as usize;
    let initial_payload = var[alen + 2 + pad_len..].to_vec();

    let mut responder = Responder {
        cipher,
        psk: psk.clone(),
        request_salt: salt.clone(),
        started: None,
    };

    if responder.echo(&mut stream, &initial_payload).await.is_err() {
        return;
    }

    while let Some(data) = read_body_chunk(&mut stream, &read_cipher, &mut read_nonce).await {
        if responder.echo(&mut stream, &data).await.is_err() {
            return;
        }
    }
}

async fn spawn_server(cipher: ShadowsocksCipher) -> SocketAddr {
    let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        while let Ok((stream, _)) = listener.accept().await {
            tokio::spawn(serve_ss2022(stream, cipher));
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
        key: psk(cipher),
        plugin: None,
    }))
}

#[tokio::test]
async fn relays_through_2022_blake3_aes_128_gcm() {
    let server = spawn_server(ShadowsocksCipher::Blake3Aes128Gcm).await;
    assert_relays(
        config(server, ShadowsocksCipher::Blake3Aes128Gcm),
        b"hello shadowsocks 2022-blake3-aes-128-gcm",
    )
    .await;
}

#[tokio::test]
async fn relays_through_2022_blake3_aes_256_gcm() {
    let server = spawn_server(ShadowsocksCipher::Blake3Aes256Gcm).await;
    assert_relays(
        config(server, ShadowsocksCipher::Blake3Aes256Gcm),
        b"hello shadowsocks 2022-blake3-aes-256-gcm",
    )
    .await;
}

#[tokio::test]
async fn relays_through_2022_blake3_chacha20_poly1305() {
    let server = spawn_server(ShadowsocksCipher::Blake3Chacha20Poly1305).await;
    assert_relays(
        config(server, ShadowsocksCipher::Blake3Chacha20Poly1305),
        b"hello shadowsocks 2022-blake3-chacha20-poly1305",
    )
    .await;
}

#[tokio::test]
async fn relays_large_payload_spanning_multiple_chunks() {
    // Larger than one 0x3FFF-byte chunk to exercise the chunk loop and the
    // response header's first-chunk handling on both ends.
    let server = spawn_server(ShadowsocksCipher::Blake3Aes256Gcm).await;
    let payload: Vec<u8> = (0..40_000u32).map(|i| (i % 251) as u8).collect();
    assert_relays(config(server, ShadowsocksCipher::Blake3Aes256Gcm), &payload).await;
}
