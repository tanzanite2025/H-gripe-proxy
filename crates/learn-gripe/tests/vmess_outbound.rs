//! End-to-end proof that traffic flows through a VMess (AEAD) outbound:
//! a SOCKS5 client -> gripe inbound -> VMess outbound -> fake VMess server.
//!
//! The fake server is an *independent* server-side implementation of the VMess
//! AEAD handshake: it decrypts the AEAD request header (auth id + length + body
//! command), verifies the FNV-1a command checksum, derives the request/response
//! body keys, then decrypts each request body chunk and echoes it back as a
//! response body chunk (after sending the sealed AEAD response header). Driving
//! a real handshake against a separate implementation proves the framing,
//! key-derivation and chunked body stream all compose correctly — not just that
//! the client round-trips with itself.
//!
//! Because security and transport are orthogonal layers shared with VLESS and
//! Trojan (see `learn_gripe::transport`), these tests cover the VMess framing
//! and its composition with the `none` / `tls` / `reality` security layers (for
//! both `aes-128-gcm` and `chacha20-poly1305` body ciphers); transport
//! composition (ws/grpc/h2/xhttp) rides the same `transport::establish` path
//! proven by the VLESS relay tests.

use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::sync::Arc;

use aes_gcm::Aes128Gcm;
use aes_gcm::aead::generic_array::GenericArray;
use aes_gcm::aead::{Aead, KeyInit, Payload};
use chacha20poly1305::ChaCha20Poly1305;
use learn_gripe::{
    ClientFingerprint, GripeConfig, GripeKernel, OutboundMode, RealityClientConfig, Security, TlsClientConfig,
    Transport, VmessCipher, VmessOutboundConfig,
};
use md5::Md5;
use sha2::{Digest, Sha256};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio_rustls::TlsAcceptor;

const TEST_CERT: &str = include_str!("data/vless_tls_cert.pem");
const TEST_KEY: &str = include_str!("data/vless_tls_key.pem");

/// `b831381d-6324-4d53-ad4f-8cda48b30811` as raw bytes — the VMess user id used
/// by these tests.
const TEST_UUID: [u8; 16] = [
    0xb8, 0x31, 0x38, 0x1d, 0x63, 0x24, 0x4d, 0x53, 0xad, 0x4f, 0x8c, 0xda, 0x48, 0xb3, 0x08, 0x11,
];

const TEST_REALITY_PUBLIC_KEY: [u8; 32] = [
    0x9c, 0x6f, 0x1a, 0x2b, 0x3c, 0x4d, 0x5e, 0x6f, 0x70, 0x81, 0x92, 0xa3, 0xb4, 0xc5, 0xd6, 0xe7, 0xf8, 0x09, 0x1a,
    0x2b, 0x3c, 0x4d, 0x5e, 0x6f, 0x70, 0x81, 0x92, 0xa3, 0xb4, 0xc5, 0xd6, 0x12,
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

/// An AEAD body cipher mirroring the client's request/response directions.
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

/// Read exactly one length-prefixed body chunk; returns its plaintext, or `None`
/// at EOF or on the empty terminating chunk.
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

/// Decode the VMess AEAD request, send the AEAD response header, then echo every
/// request body chunk back to the client as a response body chunk.
async fn serve_vmess<S>(mut stream: S, expected_cipher: VmessCipher)
where
    S: AsyncRead + AsyncWrite + Unpin,
{
    let cmd_key = command_key(&TEST_UUID);

    // authID(16) | sealed length(2+16) | nonce(8)
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

    // Validate the command block and its trailing FNV-1a checksum.
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
    assert_eq!(command[37], 0x01, "vmess command should be TCP");
    let expected_byte = match expected_cipher {
        VmessCipher::Aes128Gcm => 0x03,
        VmessCipher::Chacha20Poly1305 => 0x04,
    };
    assert_eq!(security, expected_byte, "negotiated body cipher");

    // Send the AEAD response header: [V, opt=0, cmd=0, cmdlen=0].
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
            tokio::spawn(serve_vmess(stream, cipher));
        }
    });
    addr
}

async fn spawn_tls_server(cipher: VmessCipher, tls13_only: bool) -> SocketAddr {
    let acceptor = tls_acceptor(tls13_only);
    let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        while let Ok((tcp, _)) = listener.accept().await {
            let acceptor = acceptor.clone();
            tokio::spawn(async move {
                if let Ok(tls) = acceptor.accept(tcp).await {
                    serve_vmess(tls, cipher).await;
                }
            });
        }
    });
    addr
}

fn tls_acceptor(tls13_only: bool) -> TlsAcceptor {
    let certs = rustls_pemfile::certs(&mut TEST_CERT.as_bytes())
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    let key = rustls_pemfile::private_key(&mut TEST_KEY.as_bytes()).unwrap().unwrap();
    let provider = Arc::new(rustls::crypto::ring::default_provider());
    let builder = rustls::ServerConfig::builder_with_provider(provider);
    let config = if tls13_only {
        builder.with_protocol_versions(&[&rustls::version::TLS13]).unwrap()
    } else {
        builder.with_safe_default_protocol_versions().unwrap()
    }
    .with_no_client_auth()
    .with_single_cert(certs, key)
    .unwrap();
    TlsAcceptor::from(Arc::new(config))
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
    conn.write_all(payload).await.unwrap();
    let mut buf = vec![0u8; payload.len()];
    conn.read_exact(&mut buf).await.unwrap();
    assert_eq!(buf, payload);

    handle.shutdown().await;
}

fn config(server: SocketAddr, cipher: VmessCipher, security: Security) -> OutboundMode {
    OutboundMode::Vmess(Box::new(VmessOutboundConfig {
        server: server.ip().to_string(),
        port: server.port(),
        uuid: TEST_UUID,
        cipher,
        security,
        transport: Transport::Tcp,
    }))
}

#[tokio::test]
async fn relays_through_plaintext_aes_gcm_vmess() {
    let server = spawn_plaintext_server(VmessCipher::Aes128Gcm).await;
    assert_relays(
        config(server, VmessCipher::Aes128Gcm, Security::None),
        b"hello vmess aes-gcm",
    )
    .await;
}

#[tokio::test]
async fn relays_through_plaintext_chacha_vmess() {
    let server = spawn_plaintext_server(VmessCipher::Chacha20Poly1305).await;
    assert_relays(
        config(server, VmessCipher::Chacha20Poly1305, Security::None),
        b"hello vmess chacha20",
    )
    .await;
}

#[tokio::test]
async fn relays_through_tls_vmess() {
    let server = spawn_tls_server(VmessCipher::Aes128Gcm, false).await;
    assert_relays(
        config(
            server,
            VmessCipher::Aes128Gcm,
            Security::Tls(TlsClientConfig {
                server_name: Some("localhost".to_string()),
                alpn: Vec::new(),
                skip_cert_verify: true,
                client_fingerprint: None,
                ech_config_list: None,
            }),
        ),
        b"hello vmess over tls",
    )
    .await;
}

#[tokio::test]
async fn relays_through_reality_vmess() {
    let server = spawn_tls_server(VmessCipher::Chacha20Poly1305, true).await;
    assert_relays(
        config(
            server,
            VmessCipher::Chacha20Poly1305,
            Security::Reality(RealityClientConfig {
                server_name: "localhost".to_string(),
                public_key: TEST_REALITY_PUBLIC_KEY,
                short_id: vec![0x01, 0x23, 0x45, 0x67],
                alpn: Vec::new(),
                skip_cert_verify: true,
                client_fingerprint: Some(ClientFingerprint::Chrome),
            }),
        ),
        b"hello vmess over reality",
    )
    .await;
}
