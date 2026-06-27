//! Shared helpers for the Shadowsocks SIP003 plugin interop tests.
//!
//! Provides an *independent* server-side implementation of the Shadowsocks AEAD
//! stream (aes-256-gcm) that runs over any already-unwrapped byte stream, plus
//! the SOCKS5 client + kernel harness used to drive a round trip. The plugin
//! framing (simple-obfs http/tls, v2ray-plugin ws/tls) is stripped by the
//! per-test fake servers before they hand the bare stream to [`serve_shadowsocks`].

#![allow(dead_code)]

use std::net::{IpAddr, Ipv4Addr, SocketAddr};

use aes_gcm::Aes256Gcm;
use aes_gcm::aead::generic_array::GenericArray;
use aes_gcm::aead::{Aead, KeyInit, Payload};
use learn_gripe::{GripeConfig, GripeKernel, OutboundMode, ProxyEntry, ShadowsocksOutboundConfig};
use md5::Md5;
use sha1::{Digest, Sha1};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use tokio::net::TcpStream;

pub const PASSWORD: &str = "correct horse battery staple";
pub const SS_SUBKEY_INFO: &[u8] = b"ss-subkey";
pub const TAG_LEN: usize = 16;
pub const KEY_SIZE: usize = 32; // aes-256-gcm

// --- minimal independent Shadowsocks crypto for the fake server -----------

pub fn evp_bytes_to_key(password: &[u8], key_len: usize) -> Vec<u8> {
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

fn seal(cipher: &Aes256Gcm, nonce: &[u8; 12], pt: &[u8]) -> Vec<u8> {
    cipher
        .encrypt(GenericArray::from_slice(nonce), Payload { msg: pt, aad: &[] })
        .unwrap()
}

fn open(cipher: &Aes256Gcm, nonce: &[u8; 12], ct: &[u8]) -> Vec<u8> {
    cipher
        .decrypt(GenericArray::from_slice(nonce), Payload { msg: ct, aad: &[] })
        .unwrap()
}

/// Read and decrypt one AEAD chunk; returns its plaintext, or `None` at EOF.
async fn read_chunk<S>(stream: &mut S, cipher: &Aes256Gcm, nonce: &mut [u8; 12]) -> Option<Vec<u8>>
where
    S: AsyncRead + Unpin,
{
    let mut sealed_len = [0u8; 2 + TAG_LEN];
    stream.read_exact(&mut sealed_len).await.ok()?;
    let len_pt = open(cipher, nonce, &sealed_len);
    increment_nonce(nonce);
    let clen = u16::from_be_bytes([len_pt[0], len_pt[1]]) as usize;
    let mut sealed = vec![0u8; clen + TAG_LEN];
    stream.read_exact(&mut sealed).await.ok()?;
    let pt = open(cipher, nonce, &sealed);
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

/// Run the Shadowsocks AEAD server (aes-256-gcm) over an already-unwrapped byte
/// stream: read the client salt + address, then echo application bytes back as a
/// salted, AEAD-chunked Shadowsocks response.
pub async fn serve_shadowsocks<S>(mut stream: S)
where
    S: AsyncRead + AsyncWrite + Unpin,
{
    let master = evp_bytes_to_key(PASSWORD.as_bytes(), KEY_SIZE);

    let mut salt = vec![0u8; KEY_SIZE];
    if stream.read_exact(&mut salt).await.is_err() {
        return;
    }
    let read_subkey = hkdf_sha1(&master, &salt, SS_SUBKEY_INFO, KEY_SIZE);
    let read_cipher = Aes256Gcm::new_from_slice(&read_subkey).unwrap();
    let mut read_nonce = [0u8; 12];

    let mut resp_salt = vec![0u8; KEY_SIZE];
    for (i, b) in resp_salt.iter_mut().enumerate() {
        *b = (i as u8).wrapping_mul(7).wrapping_add(1);
    }
    if stream.write_all(&resp_salt).await.is_err() {
        return;
    }
    let write_subkey = hkdf_sha1(&master, &resp_salt, SS_SUBKEY_INFO, KEY_SIZE);
    let write_cipher = Aes256Gcm::new_from_slice(&write_subkey).unwrap();
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
        let sealed_len = seal(&write_cipher, &write_nonce, &(data.len() as u16).to_be_bytes());
        increment_nonce(&mut write_nonce);
        let sealed_payload = seal(&write_cipher, &write_nonce, &data);
        increment_nonce(&mut write_nonce);
        if stream.write_all(&sealed_len).await.is_err()
            || stream.write_all(&sealed_payload).await.is_err()
            || stream.flush().await.is_err()
        {
            return;
        }
    }
}

// --- SOCKS5 client + kernel harness ---------------------------------------

pub async fn socks5_connect(proxy: SocketAddr, target: SocketAddr) -> TcpStream {
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

/// Build an `ss` outbound (aes-256-gcm) with the given `plugin` config block via
/// the real `from_proxy` parser, so the plugin parsing path is exercised too.
pub fn ss_plugin_config(server: SocketAddr, plugin_yaml: &str) -> OutboundMode {
    let yaml = format!(
        "name: s\ntype: ss\nserver: {}\nport: {}\ncipher: aes-256-gcm\npassword: {PASSWORD}\n{plugin_yaml}",
        server.ip(),
        server.port(),
    );
    let entry: ProxyEntry = serde_yaml_ng::from_str(&yaml).expect("parse proxy entry");
    let cfg = ShadowsocksOutboundConfig::from_proxy(&entry).expect("build ss config");
    OutboundMode::Shadowsocks(Box::new(cfg))
}

/// Drive a SOCKS5 round trip through the kernel built from `outbound`, sending
/// the payload in two writes to exercise multiple body chunks, and assert it is
/// echoed back unchanged.
pub async fn assert_relays(outbound: OutboundMode, payload: &[u8]) {
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
