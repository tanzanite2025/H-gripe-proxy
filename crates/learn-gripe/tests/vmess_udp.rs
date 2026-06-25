//! End-to-end proof that UDP rides a VMess (AEAD) outbound:
//! SOCKS5 UDP ASSOCIATE -> gripe inbound -> VMess UDP tunnel -> fake server.
//!
//! The fake server is an *independent* server-side VMess AEAD implementation
//! (the same one proven for TCP in `vmess_outbound.rs`): it decrypts the AEAD
//! request header, asserts the UDP command (0x02), sends the AEAD response
//! header, then echoes every body chunk. Each client UDP datagram is carried as
//! exactly one AEAD body chunk, so the chunk echo doubles as a UDP echo. We
//! cover both body ciphers, `none` / `tls` security, an IPv4 and a domain
//! destination, and a `Routed` outbound resolving the datagram to the tunnel.

use std::collections::HashMap;
use std::net::{Ipv4Addr, SocketAddr};
use std::sync::Arc;

use aes_gcm::Aes128Gcm;
use aes_gcm::aead::generic_array::GenericArray;
use aes_gcm::aead::{Aead, KeyInit, Payload};
use chacha20poly1305::ChaCha20Poly1305;
use learn_gripe::{
    GripeConfig, GripeKernel, OutboundMode, Router, Security, TlsClientConfig, Transport, VmessCipher,
    VmessOutboundConfig,
};
use md5::Md5;
use sha2::{Digest, Sha256};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream, UdpSocket};
use tokio_rustls::TlsAcceptor;

const TEST_CERT: &str = include_str!("data/vless_tls_cert.pem");
const TEST_KEY: &str = include_str!("data/vless_tls_key.pem");

const TEST_UUID: [u8; 16] = [
    0xb8, 0x31, 0x38, 0x1d, 0x63, 0x24, 0x4d, 0x53, 0xad, 0x4f, 0x8c, 0xda, 0x48, 0xb3, 0x08, 0x11,
];

const CMD_KEY_SUFFIX: &[u8] = b"c48619fe-8f02-49e0-b9e9-edf763e17e21";
const KDF_ROOT_KEY: &[u8] = b"VMess AEAD KDF";
const KDF_SALT_REQ_LEN_KEY: &[u8] = b"VMess Header AEAD Key_Length";
const KDF_SALT_REQ_LEN_IV: &[u8] = b"VMess Header AEAD Nonce_Length";
const KDF_SALT_REQ_HDR_KEY: &[u8] = b"VMess Header AEAD Key";
const KDF_SALT_REQ_HDR_IV: &[u8] = b"VMess Header AEAD Nonce";
const KDF_SALT_RESP_LEN_KEY: &[u8] = b"AEAD Resp Header Len Key";
const KDF_SALT_RESP_LEN_IV: &[u8] = b"AEAD Resp Header Len IV";
const KDF_SALT_RESP_HDR_KEY: &[u8] = b"AEAD Resp Header Key";
const KDF_SALT_RESP_HDR_IV: &[u8] = b"AEAD Resp Header IV";

// --- minimal independent VMess crypto for the fake server -----------------

fn sha256(parts: &[&[u8]]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    for part in parts {
        hasher.update(part);
    }
    hasher.finalize().into()
}

fn sha256_16(input: &[u8]) -> [u8; 16] {
    let mut out = [0u8; 16];
    out.copy_from_slice(&sha256(&[input])[..16]);
    out
}

fn command_key(uuid: &[u8; 16]) -> [u8; 16] {
    let mut hasher = Md5::new();
    hasher.update(uuid);
    hasher.update(CMD_KEY_SUFFIX);
    hasher.finalize().into()
}

fn chacha_key(key: &[u8; 16]) -> [u8; 32] {
    let mut h1 = Md5::new();
    h1.update(key);
    let first: [u8; 16] = h1.finalize().into();
    let mut h2 = Md5::new();
    h2.update(first);
    let second: [u8; 16] = h2.finalize().into();
    let mut out = [0u8; 32];
    out[..16].copy_from_slice(&first);
    out[16..].copy_from_slice(&second);
    out
}

const SHA256_BLOCK: usize = 64;

fn hmac_pads(key: &[u8]) -> ([u8; SHA256_BLOCK], [u8; SHA256_BLOCK]) {
    let mut block = [0u8; SHA256_BLOCK];
    if key.len() > SHA256_BLOCK {
        block[..32].copy_from_slice(&sha256(&[key]));
    } else {
        block[..key.len()].copy_from_slice(key);
    }
    let mut ipad = [0u8; SHA256_BLOCK];
    let mut opad = [0u8; SHA256_BLOCK];
    for i in 0..SHA256_BLOCK {
        ipad[i] = block[i] ^ 0x36;
        opad[i] = block[i] ^ 0x5c;
    }
    (ipad, opad)
}

fn hmac_sha256(key: &[u8], msg: &[u8]) -> [u8; 32] {
    let (ipad, opad) = hmac_pads(key);
    let inner = sha256(&[&ipad, msg]);
    sha256(&[&opad, &inner])
}

fn kdf_rec(level: usize, paths: &[&[u8]], msg: &[u8]) -> [u8; 32] {
    if level == 0 {
        return hmac_sha256(KDF_ROOT_KEY, msg);
    }
    let (ipad, opad) = hmac_pads(paths[level - 1]);
    let mut inner_in = Vec::new();
    inner_in.extend_from_slice(&ipad);
    inner_in.extend_from_slice(msg);
    let inner = kdf_rec(level - 1, paths, &inner_in);
    let mut outer_in = Vec::new();
    outer_in.extend_from_slice(&opad);
    outer_in.extend_from_slice(&inner);
    kdf_rec(level - 1, paths, &outer_in)
}

fn kdf16(key: &[u8], paths: &[&[u8]]) -> [u8; 16] {
    let mut out = [0u8; 16];
    out.copy_from_slice(&kdf_rec(paths.len(), paths, key)[..16]);
    out
}

fn kdf12(key: &[u8], paths: &[&[u8]]) -> [u8; 12] {
    let mut out = [0u8; 12];
    out.copy_from_slice(&kdf_rec(paths.len(), paths, key)[..12]);
    out
}

fn fnv1a32(data: &[u8]) -> u32 {
    let mut hash = 0x811c_9dc5u32;
    for byte in data {
        hash ^= u32::from(*byte);
        hash = hash.wrapping_mul(0x0100_0193);
    }
    hash
}

fn aes_gcm_seal(key: &[u8; 16], nonce: &[u8; 12], aad: &[u8], pt: &[u8]) -> Vec<u8> {
    let cipher = Aes128Gcm::new_from_slice(key).unwrap();
    cipher
        .encrypt(GenericArray::from_slice(nonce), Payload { msg: pt, aad })
        .unwrap()
}

fn aes_gcm_open(key: &[u8; 16], nonce: &[u8; 12], aad: &[u8], ct: &[u8]) -> Vec<u8> {
    let cipher = Aes128Gcm::new_from_slice(key).unwrap();
    cipher
        .decrypt(GenericArray::from_slice(nonce), Payload { msg: ct, aad })
        .unwrap()
}

fn chunk_nonce(iv: &[u8; 16], count: u16) -> [u8; 12] {
    let mut nonce = [0u8; 12];
    nonce[..2].copy_from_slice(&count.to_be_bytes());
    nonce[2..].copy_from_slice(&iv[2..12]);
    nonce
}

enum BodyCipher {
    Aes(Box<Aes128Gcm>),
    Chacha(Box<ChaCha20Poly1305>),
}

impl BodyCipher {
    fn new(security: u8, base_key: &[u8; 16]) -> Self {
        match security {
            0x03 => BodyCipher::Aes(Box::new(Aes128Gcm::new_from_slice(base_key).unwrap())),
            0x04 => {
                let key = chacha_key(base_key);
                BodyCipher::Chacha(Box::new(ChaCha20Poly1305::new_from_slice(&key).unwrap()))
            }
            other => panic!("unexpected vmess security byte {other:#x}"),
        }
    }

    fn seal(&self, nonce: &[u8; 12], pt: &[u8]) -> Vec<u8> {
        let payload = Payload { msg: pt, aad: &[] };
        match self {
            BodyCipher::Aes(c) => c.encrypt(GenericArray::from_slice(nonce), payload),
            BodyCipher::Chacha(c) => c.encrypt(GenericArray::from_slice(nonce), payload),
        }
        .unwrap()
    }

    fn open(&self, nonce: &[u8; 12], ct: &[u8]) -> Vec<u8> {
        let payload = Payload { msg: ct, aad: &[] };
        match self {
            BodyCipher::Aes(c) => c.decrypt(GenericArray::from_slice(nonce), payload),
            BodyCipher::Chacha(c) => c.decrypt(GenericArray::from_slice(nonce), payload),
        }
        .unwrap()
    }
}

async fn read_body_chunk<S>(stream: &mut S, cipher: &BodyCipher, iv: &[u8; 16], count: &mut u16) -> Option<Vec<u8>>
where
    S: AsyncRead + Unpin,
{
    let mut len = [0u8; 2];
    stream.read_exact(&mut len).await.ok()?;
    let clen = u16::from_be_bytes(len) as usize;
    assert!(clen >= 16, "body chunk shorter than the AEAD tag");
    let mut ct = vec![0u8; clen];
    stream.read_exact(&mut ct).await.ok()?;
    let nonce = chunk_nonce(iv, *count);
    *count = count.wrapping_add(1);
    let pt = cipher.open(&nonce, &ct);
    if pt.is_empty() { None } else { Some(pt) }
}

/// Decode the VMess AEAD request (asserting the UDP command), send the AEAD
/// response header, then echo every body chunk — one chunk per UDP datagram.
async fn serve_vmess_udp<S>(mut stream: S, expected_cipher: VmessCipher)
where
    S: AsyncRead + AsyncWrite + Unpin,
{
    let cmd_key = command_key(&TEST_UUID);

    let mut prefix = [0u8; 42];
    stream.read_exact(&mut prefix).await.unwrap();
    let mut auth_id = [0u8; 16];
    auth_id.copy_from_slice(&prefix[..16]);
    let sealed_len = &prefix[16..34];
    let mut nonce = [0u8; 8];
    nonce.copy_from_slice(&prefix[34..42]);

    let len_key = kdf16(&cmd_key, &[KDF_SALT_REQ_LEN_KEY, &auth_id, &nonce]);
    let len_iv = kdf12(&cmd_key, &[KDF_SALT_REQ_LEN_IV, &auth_id, &nonce]);
    let len_pt = aes_gcm_open(&len_key, &len_iv, &auth_id, sealed_len);
    let header_len = u16::from_be_bytes([len_pt[0], len_pt[1]]) as usize;

    let mut sealed_header = vec![0u8; header_len + 16];
    stream.read_exact(&mut sealed_header).await.unwrap();
    let hdr_key = kdf16(&cmd_key, &[KDF_SALT_REQ_HDR_KEY, &auth_id, &nonce]);
    let hdr_iv = kdf12(&cmd_key, &[KDF_SALT_REQ_HDR_IV, &auth_id, &nonce]);
    let command = aes_gcm_open(&hdr_key, &hdr_iv, &auth_id, &sealed_header);

    assert_eq!(command[0], 0x01, "vmess version");
    let checksum_at = command.len() - 4;
    let expected = u32::from_be_bytes([
        command[checksum_at],
        command[checksum_at + 1],
        command[checksum_at + 2],
        command[checksum_at + 3],
    ]);
    assert_eq!(fnv1a32(&command[..checksum_at]), expected, "vmess command fnv checksum");

    let mut body_iv = [0u8; 16];
    body_iv.copy_from_slice(&command[1..17]);
    let mut body_key = [0u8; 16];
    body_key.copy_from_slice(&command[17..33]);
    let response_verifier = command[33];
    let security = command[35] & 0x0f;
    assert_eq!(command[37], 0x02, "vmess command should be UDP");
    let expected_byte = match expected_cipher {
        VmessCipher::Aes128Gcm => 0x03,
        VmessCipher::Chacha20Poly1305 => 0x04,
    };
    assert_eq!(security, expected_byte, "negotiated body cipher");

    let response_body_key = sha256_16(&body_key);
    let response_body_iv = sha256_16(&body_iv);
    let resp_len_key = kdf16(&response_body_key, &[KDF_SALT_RESP_LEN_KEY]);
    let resp_len_iv = kdf12(&response_body_iv, &[KDF_SALT_RESP_LEN_IV]);
    let resp_hdr_key = kdf16(&response_body_key, &[KDF_SALT_RESP_HDR_KEY]);
    let resp_hdr_iv = kdf12(&response_body_iv, &[KDF_SALT_RESP_HDR_IV]);
    let resp_header = [response_verifier, 0u8, 0u8, 0u8];
    let sealed_resp_hdr = aes_gcm_seal(&resp_hdr_key, &resp_hdr_iv, &[], &resp_header);
    let sealed_resp_len = aes_gcm_seal(
        &resp_len_key,
        &resp_len_iv,
        &[],
        &(resp_header.len() as u16).to_be_bytes(),
    );
    stream.write_all(&sealed_resp_len).await.unwrap();
    stream.write_all(&sealed_resp_hdr).await.unwrap();
    stream.flush().await.unwrap();

    let request_cipher = BodyCipher::new(security, &body_key);
    let response_cipher = BodyCipher::new(security, &response_body_key);
    let mut request_count = 0u16;
    let mut response_count = 0u16;
    while let Some(plain) = read_body_chunk(&mut stream, &request_cipher, &body_iv, &mut request_count).await {
        let nonce = chunk_nonce(&response_body_iv, response_count);
        response_count = response_count.wrapping_add(1);
        let chunk = response_cipher.seal(&nonce, &plain);
        let len = chunk.len() as u16;
        if stream.write_all(&len.to_be_bytes()).await.is_err() || stream.write_all(&chunk).await.is_err() {
            return;
        }
        let _ = stream.flush().await;
    }
}

async fn spawn_plaintext_server(cipher: VmessCipher) -> SocketAddr {
    let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        while let Ok((stream, _)) = listener.accept().await {
            tokio::spawn(serve_vmess_udp(stream, cipher));
        }
    });
    addr
}

async fn spawn_tls_server(cipher: VmessCipher) -> SocketAddr {
    let acceptor = tls_acceptor();
    let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        while let Ok((tcp, _)) = listener.accept().await {
            let acceptor = acceptor.clone();
            tokio::spawn(async move {
                if let Ok(tls) = acceptor.accept(tcp).await {
                    serve_vmess_udp(tls, cipher).await;
                }
            });
        }
    });
    addr
}

fn tls_acceptor() -> TlsAcceptor {
    let certs = rustls_pemfile::certs(&mut TEST_CERT.as_bytes())
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    let key = rustls_pemfile::private_key(&mut TEST_KEY.as_bytes()).unwrap().unwrap();
    let config = rustls::ServerConfig::builder_with_provider(Arc::new(rustls::crypto::ring::default_provider()))
        .with_safe_default_protocol_versions()
        .unwrap()
        .with_no_client_auth()
        .with_single_cert(certs, key)
        .unwrap();
    TlsAcceptor::from(Arc::new(config))
}

fn vmess(server: SocketAddr, cipher: VmessCipher, security: Security) -> Box<VmessOutboundConfig> {
    Box::new(VmessOutboundConfig {
        server: server.ip().to_string(),
        port: server.port(),
        uuid: TEST_UUID,
        cipher,
        security,
        transport: Transport::Tcp,
    })
}

fn tls_security() -> Security {
    Security::Tls(TlsClientConfig {
        server_name: Some("localhost".to_string()),
        alpn: Vec::new(),
        skip_cert_verify: true,
    })
}

async fn socks5_greet(proxy: SocketAddr) -> TcpStream {
    let mut stream = TcpStream::connect(proxy).await.unwrap();
    stream.write_all(&[0x05, 0x01, 0x00]).await.unwrap();
    let mut selection = [0u8; 2];
    stream.read_exact(&mut selection).await.unwrap();
    assert_eq!(selection, [0x05, 0x00]);
    stream
}

async fn socks5_udp_associate(proxy: SocketAddr) -> (TcpStream, SocketAddr) {
    let mut stream = socks5_greet(proxy).await;
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
        std::net::IpAddr::V4(v4) => v4.octets(),
        std::net::IpAddr::V6(_) => panic!("test uses IPv4"),
    };
    let mut datagram = vec![0x00, 0x00, 0x00, 0x01];
    datagram.extend_from_slice(&ip);
    datagram.extend_from_slice(&dst.port().to_be_bytes());
    datagram.extend_from_slice(payload);
    datagram
}

fn udp_datagram_domain(host: &str, port: u16, payload: &[u8]) -> Vec<u8> {
    let mut datagram = vec![0x00, 0x00, 0x00, 0x03, host.len() as u8];
    datagram.extend_from_slice(host.as_bytes());
    datagram.extend_from_slice(&port.to_be_bytes());
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

async fn assert_udp_relays(outbound: OutboundMode, datagram: Vec<u8>, payload: &[u8]) {
    let handle = GripeKernel::start(GripeConfig {
        socks_listen: SocketAddr::from((Ipv4Addr::LOCALHOST, 0)),
        outbound,
    })
    .await
    .unwrap();

    let (_control, relay) = socks5_udp_associate(handle.local_addr()).await;
    let client = UdpSocket::bind((Ipv4Addr::LOCALHOST, 0)).await.unwrap();
    client.send_to(&datagram, relay).await.unwrap();

    let mut buf = [0u8; 2048];
    let (n, from) = client.recv_from(&mut buf).await.unwrap();
    assert_eq!(from, relay, "reply must come from the relay socket");
    let offset = payload_offset(&buf[..n]);
    assert_eq!(&buf[offset..n], payload, "payload must be echoed verbatim");

    handle.shutdown().await;
}

#[tokio::test]
async fn udp_relays_through_plaintext_aes_gcm_vmess() {
    let server = spawn_plaintext_server(VmessCipher::Aes128Gcm).await;
    let dst = SocketAddr::from((Ipv4Addr::new(1, 2, 3, 4), 443));
    assert_udp_relays(
        OutboundMode::Vmess(vmess(server, VmessCipher::Aes128Gcm, Security::None)),
        udp_datagram_ipv4(dst, b"vmess udp aes"),
        b"vmess udp aes",
    )
    .await;
}

#[tokio::test]
async fn udp_relays_through_plaintext_chacha_vmess_domain() {
    let server = spawn_plaintext_server(VmessCipher::Chacha20Poly1305).await;
    assert_udp_relays(
        OutboundMode::Vmess(vmess(server, VmessCipher::Chacha20Poly1305, Security::None)),
        udp_datagram_domain("example.com", 53, b"vmess udp chacha"),
        b"vmess udp chacha",
    )
    .await;
}

#[tokio::test]
async fn udp_relays_through_tls_vmess() {
    let server = spawn_tls_server(VmessCipher::Aes128Gcm).await;
    let dst = SocketAddr::from((Ipv4Addr::new(9, 9, 9, 9), 443));
    assert_udp_relays(
        OutboundMode::Vmess(vmess(server, VmessCipher::Aes128Gcm, tls_security())),
        udp_datagram_ipv4(dst, b"tls vmess udp"),
        b"tls vmess udp",
    )
    .await;
}

#[tokio::test]
async fn udp_relays_through_routed_vmess() {
    let server = spawn_plaintext_server(VmessCipher::Aes128Gcm).await;
    let mut outbounds = HashMap::new();
    outbounds.insert(
        "proxy".to_string(),
        OutboundMode::Vmess(vmess(server, VmessCipher::Aes128Gcm, Security::None)),
    );
    let router = Router::new(outbounds, vec![], "proxy").unwrap();
    let dst = SocketAddr::from((Ipv4Addr::new(1, 2, 3, 4), 443));
    assert_udp_relays(
        OutboundMode::Routed(Box::new(router)),
        udp_datagram_ipv4(dst, b"routed vmess udp"),
        b"routed vmess udp",
    )
    .await;
}
