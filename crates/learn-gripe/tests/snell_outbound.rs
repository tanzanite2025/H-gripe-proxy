//! End-to-end proof that traffic flows through a Snell outbound:
//! a SOCKS5 client -> gripe inbound -> Snell outbound -> fake Snell server.
//!
//! The fake server is an independent re-implementation of the Snell wire format
//! (Shadowsocks-AEAD chunk framing with Snell's Argon2id session-subkey KDF and
//! 16-byte salt): it reads the client salt, derives the read subkey, decrypts
//! the request header (`proto | command | clientID-len | host | port`), replies
//! with its own salt + the `Tunnel` command response, then echoes the
//! application byte stream. We cover v1 (ChaCha20-Poly1305) and v3
//! (AES-128-GCM), a payload spanning multiple chunks, and a `Routed` outbound.

use std::collections::HashMap;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};

use aes_gcm::Aes128Gcm;
use aes_gcm::aead::generic_array::GenericArray;
use aes_gcm::aead::{Aead, KeyInit, Payload};
use argon2::{Algorithm, Argon2, Params, Version};
use chacha20poly1305::ChaCha20Poly1305;
use learn_gripe::{GripeConfig, GripeKernel, OutboundMode, Router, SnellOutboundConfig};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

const TEST_PSK: &[u8] = b"snell-test-psk";
const SALT_LEN: usize = 16;
const TAG_LEN: usize = 16;
const RESP_TUNNEL: u8 = 0;

/// Snell's session-subkey KDF (independent of the kernel's copy).
fn snell_kdf(psk: &[u8], salt: &[u8], key_size: usize) -> Vec<u8> {
    let params = Params::new(8, 3, 1, Some(32)).unwrap();
    let argon2 = Argon2::new(Algorithm::Argon2id, Version::V0x13, params);
    let mut out = [0u8; 32];
    argon2.hash_password_into(psk, salt, &mut out).unwrap();
    out[..key_size].to_vec()
}

fn increment_nonce(nonce: &mut [u8; 12]) {
    for byte in nonce.iter_mut() {
        *byte = byte.wrapping_add(1);
        if *byte != 0 {
            break;
        }
    }
}

enum Cipher {
    Aes(Box<Aes128Gcm>),
    Chacha(Box<ChaCha20Poly1305>),
}

impl Cipher {
    fn new(version: u8, subkey: &[u8]) -> Self {
        if version == 1 {
            Cipher::Chacha(Box::new(ChaCha20Poly1305::new_from_slice(subkey).unwrap()))
        } else {
            Cipher::Aes(Box::new(Aes128Gcm::new_from_slice(subkey).unwrap()))
        }
    }

    fn seal(&self, nonce: &[u8; 12], plaintext: &[u8]) -> Vec<u8> {
        let payload = Payload {
            msg: plaintext,
            aad: &[],
        };
        match self {
            Cipher::Aes(c) => c.encrypt(GenericArray::from_slice(nonce), payload),
            Cipher::Chacha(c) => c.encrypt(GenericArray::from_slice(nonce), payload),
        }
        .unwrap()
    }

    fn open(&self, nonce: &[u8; 12], ciphertext: &[u8]) -> Option<Vec<u8>> {
        let payload = Payload {
            msg: ciphertext,
            aad: &[],
        };
        match self {
            Cipher::Aes(c) => c.decrypt(GenericArray::from_slice(nonce), payload),
            Cipher::Chacha(c) => c.decrypt(GenericArray::from_slice(nonce), payload),
        }
        .ok()
    }
}

fn key_size(version: u8) -> usize {
    if version == 1 { 32 } else { 16 }
}

/// Read and decrypt one AEAD chunk; returns `None` on clean EOF.
async fn read_chunk<S>(stream: &mut S, cipher: &Cipher, nonce: &mut [u8; 12]) -> Option<Vec<u8>>
where
    S: AsyncRead + Unpin,
{
    let mut sealed_len = [0u8; 2 + TAG_LEN];
    if stream.read_exact(&mut sealed_len).await.is_err() {
        return None;
    }
    let len_plain = cipher.open(nonce, &sealed_len).expect("decrypt chunk length");
    increment_nonce(nonce);
    let clen = u16::from_be_bytes([len_plain[0], len_plain[1]]) as usize;

    let mut sealed = vec![0u8; clen + TAG_LEN];
    stream.read_exact(&mut sealed).await.expect("read chunk body");
    let plain = cipher.open(nonce, &sealed).expect("decrypt chunk body");
    increment_nonce(nonce);
    Some(plain)
}

/// Seal `plaintext` into one AEAD chunk and write it.
async fn write_chunk<S>(stream: &mut S, cipher: &Cipher, nonce: &mut [u8; 12], plaintext: &[u8])
where
    S: AsyncWrite + Unpin,
{
    let len = (plaintext.len() as u16).to_be_bytes();
    let sealed_len = cipher.seal(nonce, &len);
    increment_nonce(nonce);
    let sealed = cipher.seal(nonce, plaintext);
    increment_nonce(nonce);
    stream.write_all(&sealed_len).await.unwrap();
    stream.write_all(&sealed).await.unwrap();
}

/// Validate the Snell handshake, then echo the application byte stream.
async fn serve_snell(mut stream: TcpStream, version: u8) {
    let ks = key_size(version);

    // Read client salt, derive the read cipher.
    let mut salt = [0u8; SALT_LEN];
    stream.read_exact(&mut salt).await.unwrap();
    let read_cipher = Cipher::new(version, &snell_kdf(TEST_PSK, &salt, ks));
    let mut read_nonce = [0u8; 12];

    // First chunk is the Snell request header.
    let header = read_chunk(&mut stream, &read_cipher, &mut read_nonce)
        .await
        .expect("request header");
    assert_eq!(header[0], 1, "snell proto byte");
    assert!(header[1] == 1 || header[1] == 5, "connect command");
    assert_eq!(header[2], 0, "client ID length");
    let host_len = header[3] as usize;
    assert!(header.len() >= 4 + host_len + 2, "header host + port present");

    // Reply with our salt + the Tunnel command response.
    let mut salt_w = [0u8; SALT_LEN];
    for (i, b) in salt_w.iter_mut().enumerate() {
        *b = (i as u8).wrapping_mul(7).wrapping_add(version);
    }
    stream.write_all(&salt_w).await.unwrap();
    let write_cipher = Cipher::new(version, &snell_kdf(TEST_PSK, &salt_w, ks));
    let mut write_nonce = [0u8; 12];
    write_chunk(&mut stream, &write_cipher, &mut write_nonce, &[RESP_TUNNEL]).await;

    // Echo the application byte stream chunk by chunk.
    while let Some(data) = read_chunk(&mut stream, &read_cipher, &mut read_nonce).await {
        write_chunk(&mut stream, &write_cipher, &mut write_nonce, &data).await;
    }
}

async fn spawn_fake_snell_server(version: u8) -> SocketAddr {
    let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        while let Ok((stream, _)) = listener.accept().await {
            tokio::spawn(serve_snell(stream, version));
        }
    });
    addr
}

fn snell(server: SocketAddr, version: u8) -> Box<SnellOutboundConfig> {
    Box::new(SnellOutboundConfig {
        server: server.ip().to_string(),
        port: server.port(),
        psk: TEST_PSK.to_vec(),
        version,
        obfs: None,
    })
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

async fn assert_relays(outbound: OutboundMode, payload: &[u8]) {
    let handle = GripeKernel::start(GripeConfig {
        socks_listen: SocketAddr::from((Ipv4Addr::LOCALHOST, 0)),
        outbound,
    })
    .await
    .unwrap();

    let dummy_target = SocketAddr::from((Ipv4Addr::new(1, 2, 3, 4), 443));
    let mut conn = socks5_connect(handle.local_addr(), dummy_target).await;
    conn.write_all(payload).await.unwrap();
    let mut buf = vec![0u8; payload.len()];
    conn.read_exact(&mut buf).await.unwrap();
    assert_eq!(buf, payload);

    handle.shutdown().await;
}

#[tokio::test]
async fn relays_through_snell_v3_aes128gcm() {
    let server = spawn_fake_snell_server(3).await;
    assert_relays(OutboundMode::Snell(snell(server, 3)), b"hello snell v3").await;
}

#[tokio::test]
async fn relays_through_snell_v1_chacha20() {
    let server = spawn_fake_snell_server(1).await;
    assert_relays(OutboundMode::Snell(snell(server, 1)), b"hello snell v1").await;
}

#[tokio::test]
async fn relays_larger_payload_spanning_multiple_chunks() {
    let server = spawn_fake_snell_server(3).await;
    // Larger than one MAX_CHUNK (0x3FFF) so the relay must split it across
    // several AEAD chunks and reassemble the echo.
    let payload: Vec<u8> = (0..40_000u32).map(|i| (i % 251) as u8).collect();
    assert_relays(OutboundMode::Snell(snell(server, 3)), &payload).await;
}

#[tokio::test]
async fn relays_through_routed_snell() {
    let server = spawn_fake_snell_server(3).await;
    let mut outbounds = HashMap::new();
    outbounds.insert("proxy".to_string(), OutboundMode::Snell(snell(server, 3)));
    let router = Router::new(outbounds, vec![], "proxy").unwrap();
    assert_relays(OutboundMode::Routed(Box::new(router)), b"routed snell payload").await;
}
