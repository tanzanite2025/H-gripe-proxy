//! End-to-end proof that traffic flows through a mieru outbound:
//! a SOCKS5 client -> gripe inbound -> mieru outbound -> fake mieru server
//! -> echo server.
//!
//! The fake server is an independent implementation of the mieru TCP-underlay
//! wire format (it shares no code with the kernel): it derives the session key
//! the same way (`PBKDF2(HashPassword(password, username), salt, 64, 32)`,
//! trying the three time-window salts), decrypts the implicit-nonce
//! XChaCha20-Poly1305 segment stream, reads the `openSessionRequest` carrying
//! the inner SOCKS5 `CONNECT`, dials the address the client actually encoded
//! (a real echo server), answers with an `openSessionResponse` + SOCKS5 reply,
//! and then bridges the data segments. Dialing the *decoded* target proves the
//! inner SOCKS5 request is framed correctly rather than the server echoing to
//! itself. A wrong password (a different derived key) is also covered: the
//! server cannot open the first segment, so the outbound fails.

use std::net::{IpAddr, Ipv4Addr, SocketAddr};

use chacha20poly1305::aead::{Aead, KeyInit};
use chacha20poly1305::{XChaCha20Poly1305, XNonce};
use hmac::{Hmac, Mac};
use learn_gripe::{GripeConfig, GripeHandle, GripeKernel, MieruOutboundConfig, OutboundMode};
use sha2::{Digest, Sha256};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

const METADATA_LEN: usize = 32;
const NONCE_SIZE: usize = 24;
const OVERHEAD: usize = 16;

const PROTO_OPEN_SESSION_REQUEST: u8 = 2;
const PROTO_OPEN_SESSION_RESPONSE: u8 = 3;
const PROTO_DATA_SERVER_TO_CLIENT: u8 = 7;

const MIERU_USER: &str = "mieru-user";
const MIERU_PASS: &str = "mieru-pass";

// ---------------------------------------------------------------------------
// Independent mieru crypto (server side).
// ---------------------------------------------------------------------------

fn pbkdf2_hmac_sha256(password: &[u8], salt: &[u8], iterations: u32, out_len: usize) -> Vec<u8> {
    type HmacSha256 = Hmac<Sha256>;
    let mut out = Vec::with_capacity(out_len);
    let mut block_index: u32 = 1;
    while out.len() < out_len {
        let mut mac = <HmacSha256 as Mac>::new_from_slice(password).unwrap();
        mac.update(salt);
        mac.update(&block_index.to_be_bytes());
        let mut u = mac.finalize().into_bytes();
        let mut t = u;
        for _ in 1..iterations {
            let mut mac = <HmacSha256 as Mac>::new_from_slice(password).unwrap();
            mac.update(&u);
            u = mac.finalize().into_bytes();
            for (ti, ui) in t.iter_mut().zip(u.iter()) {
                *ti ^= *ui;
            }
        }
        out.extend_from_slice(&t);
        block_index += 1;
    }
    out.truncate(out_len);
    out
}

fn hash_password(password: &[u8], username: &[u8]) -> [u8; 32] {
    let mut h = Sha256::new();
    h.update(password);
    h.update([0x00]);
    h.update(username);
    h.finalize().into()
}

fn salt_for(rounded: u64) -> [u8; 32] {
    let mut h = Sha256::new();
    h.update(rounded.to_be_bytes());
    h.finalize().into()
}

/// The three candidate keys (previous / current / next 2-minute window) a real
/// mieru server would accept.
fn candidate_keys(password: &[u8], username: &[u8]) -> Vec<[u8; 32]> {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let rounded = ((now + 60) / 120) * 120;
    let hashed = hash_password(password, username);
    [rounded.wrapping_sub(120), rounded, rounded + 120]
        .into_iter()
        .map(|r| {
            let mut key = [0u8; 32];
            key.copy_from_slice(&pbkdf2_hmac_sha256(&hashed, &salt_for(r), 64, 32));
            key
        })
        .collect()
}

fn increment_nonce(nonce: &mut [u8; NONCE_SIZE]) {
    for byte in nonce.iter_mut().rev() {
        *byte = byte.wrapping_add(1);
        if *byte != 0 {
            break;
        }
    }
}

fn random_nonce() -> [u8; NONCE_SIZE] {
    let mut n = [0u8; NONCE_SIZE];
    getrandom::fill(&mut n).unwrap();
    n
}

/// Stateful implicit-nonce XChaCha20-Poly1305, matching the kernel's cipher.
struct Cipher {
    aead: XChaCha20Poly1305,
    nonce: Option<[u8; NONCE_SIZE]>,
}

impl Cipher {
    fn new(key: &[u8; 32]) -> Self {
        Self {
            aead: XChaCha20Poly1305::new_from_slice(key).unwrap(),
            nonce: None,
        }
    }

    fn encrypt(&mut self, plaintext: &[u8]) -> Vec<u8> {
        match self.nonce {
            None => {
                let nonce = random_nonce();
                self.nonce = Some(nonce);
                let ct = self.aead.encrypt(XNonce::from_slice(&nonce), plaintext).unwrap();
                let mut out = nonce.to_vec();
                out.extend_from_slice(&ct);
                out
            }
            Some(mut nonce) => {
                increment_nonce(&mut nonce);
                self.nonce = Some(nonce);
                self.aead.encrypt(XNonce::from_slice(&nonce), plaintext).unwrap()
            }
        }
    }

    fn decrypt(&mut self, data: &[u8]) -> Result<Vec<u8>, ()> {
        match self.nonce {
            Some(mut nonce) => {
                increment_nonce(&mut nonce);
                self.nonce = Some(nonce);
                self.aead.decrypt(XNonce::from_slice(&nonce), data).map_err(|_| ())
            }
            None => {
                if data.len() < NONCE_SIZE {
                    return Err(());
                }
                let mut nonce = [0u8; NONCE_SIZE];
                nonce.copy_from_slice(&data[..NONCE_SIZE]);
                self.nonce = Some(nonce);
                self.aead
                    .decrypt(XNonce::from_slice(&nonce), &data[NONCE_SIZE..])
                    .map_err(|_| ())
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Segment framing (server side).
// ---------------------------------------------------------------------------

struct SegLens {
    prefix: usize,
    payload: usize,
    suffix: usize,
}

fn segment_lens(meta: &[u8]) -> SegLens {
    match meta[0] {
        2 | 3 | 4 | 5 => SegLens {
            prefix: 0,
            payload: u16::from_be_bytes([meta[15], meta[16]]) as usize,
            suffix: meta[17] as usize,
        },
        _ => SegLens {
            prefix: meta[21] as usize,
            payload: u16::from_be_bytes([meta[22], meta[23]]) as usize,
            suffix: meta[24] as usize,
        },
    }
}

fn marshal_session_meta(protocol: u8, payload_len: u16) -> [u8; METADATA_LEN] {
    let mut meta = [0u8; METADATA_LEN];
    meta[0] = protocol;
    meta[15..17].copy_from_slice(&payload_len.to_be_bytes());
    meta
}

fn marshal_data_meta(protocol: u8, seq: u32, payload_len: u16) -> [u8; METADATA_LEN] {
    let mut meta = [0u8; METADATA_LEN];
    meta[0] = protocol;
    meta[10..14].copy_from_slice(&seq.to_be_bytes());
    meta[18..20].copy_from_slice(&1024u16.to_be_bytes());
    meta[22..24].copy_from_slice(&payload_len.to_be_bytes());
    meta
}

/// Read one segment, returning `(protocol, payload)` or `None` at clean EOF.
async fn recv_segment<R>(cipher: &mut Cipher, reader: &mut R) -> Option<(u8, Vec<u8>)>
where
    R: AsyncRead + Unpin,
{
    let meta_total = if cipher.nonce.is_none() {
        NONCE_SIZE + METADATA_LEN + OVERHEAD
    } else {
        METADATA_LEN + OVERHEAD
    };
    let mut raw_meta = vec![0u8; meta_total];
    if reader.read_exact(&mut raw_meta).await.is_err() {
        return None;
    }
    let meta = cipher.decrypt(&raw_meta).expect("server decrypt metadata");
    let lens = segment_lens(&meta);
    let body_total = lens.prefix + if lens.payload > 0 { lens.payload + OVERHEAD } else { 0 } + lens.suffix;
    let mut body = vec![0u8; body_total];
    if body_total > 0 && reader.read_exact(&mut body).await.is_err() {
        return None;
    }
    let payload = if lens.payload > 0 {
        let start = lens.prefix;
        let end = start + lens.payload + OVERHEAD;
        cipher.decrypt(&body[start..end]).expect("server decrypt payload")
    } else {
        Vec::new()
    };
    Some((meta[0], payload))
}

async fn send_session_segment<W>(cipher: &mut Cipher, writer: &mut W, protocol: u8, payload: &[u8])
where
    W: AsyncWrite + Unpin,
{
    let meta = marshal_session_meta(protocol, payload.len() as u16);
    let mut out = cipher.encrypt(&meta);
    if !payload.is_empty() {
        out.extend_from_slice(&cipher.encrypt(payload));
    }
    writer.write_all(&out).await.unwrap();
}

async fn send_data_segment<W>(cipher: &mut Cipher, writer: &mut W, seq: u32, payload: &[u8])
where
    W: AsyncWrite + Unpin,
{
    let meta = marshal_data_meta(PROTO_DATA_SERVER_TO_CLIENT, seq, payload.len() as u16);
    let mut out = cipher.encrypt(&meta);
    out.extend_from_slice(&cipher.encrypt(payload));
    writer.write_all(&out).await.unwrap();
}

/// Decode the inner SOCKS5 `CONNECT` request target (`VER CMD RSV ATYP ...`).
fn parse_socks5_connect(req: &[u8]) -> String {
    assert_eq!(req[0], 0x05, "inner SOCKS5 version");
    assert_eq!(req[1], 0x01, "inner SOCKS5 command should be CONNECT");
    match req[3] {
        0x01 => {
            let ip = Ipv4Addr::new(req[4], req[5], req[6], req[7]);
            let port = u16::from_be_bytes([req[8], req[9]]);
            format!("{ip}:{port}")
        }
        0x03 => {
            let dlen = req[4] as usize;
            let host = std::str::from_utf8(&req[5..5 + dlen]).unwrap();
            let port = u16::from_be_bytes([req[5 + dlen], req[6 + dlen]]);
            format!("{host}:{port}")
        }
        other => panic!("unexpected inner SOCKS5 atyp {other}"),
    }
}

fn socks5_success_reply() -> Vec<u8> {
    // VER REP RSV ATYP=ipv4 BND.ADDR=0.0.0.0 BND.PORT=0
    vec![0x05, 0x00, 0x00, 0x01, 0, 0, 0, 0, 0, 0]
}

// ---------------------------------------------------------------------------
// Fake servers.
// ---------------------------------------------------------------------------

async fn spawn_echo_server() -> SocketAddr {
    let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        while let Ok((mut stream, _)) = listener.accept().await {
            tokio::spawn(async move {
                let mut buf = [0u8; 4096];
                loop {
                    match stream.read(&mut buf).await {
                        Ok(0) | Err(_) => return,
                        Ok(n) => {
                            if stream.write_all(&buf[..n]).await.is_err() {
                                return;
                            }
                        }
                    }
                }
            });
        }
    });
    addr
}

async fn serve_mieru(mut conn: TcpStream, keys: Vec<[u8; 32]>) {
    // Peek the first segment to choose the matching candidate key, mirroring a
    // real multi-window server.
    let first_len = NONCE_SIZE + METADATA_LEN + OVERHEAD;
    let mut raw_meta = vec![0u8; first_len];
    if conn.read_exact(&mut raw_meta).await.is_err() {
        return;
    }
    let mut recv = None;
    for key in &keys {
        let mut cipher = Cipher::new(key);
        if let Ok(meta) = cipher.decrypt(&raw_meta) {
            if meta[0] == PROTO_OPEN_SESSION_REQUEST {
                // Re-seat the cipher's nonce from the segment we consumed.
                let mut nonce = [0u8; NONCE_SIZE];
                nonce.copy_from_slice(&raw_meta[..NONCE_SIZE]);
                let mut c = Cipher::new(key);
                c.nonce = Some(nonce);
                recv = Some((c, meta, *key));
                break;
            }
        }
    }
    // Wrong key (e.g. bad password): nothing decrypts to an open-session.
    let (mut recv, meta, key) = match recv {
        Some(v) => v,
        None => return,
    };

    // Read the open-session payload (the inner SOCKS5 CONNECT request).
    let lens = segment_lens(&meta);
    assert!(lens.payload > 0, "openSession must carry the inner request");
    let mut body = vec![0u8; lens.payload + OVERHEAD];
    if conn.read_exact(&mut body).await.is_err() {
        return;
    }
    let request = recv.decrypt(&body).expect("decrypt inner SOCKS5 request");
    let target = parse_socks5_connect(&request);

    let (mut reader, mut writer) = conn.into_split();
    let mut send = Cipher::new(&key);

    // openSessionResponse (empty) then the inner SOCKS5 success reply as data.
    send_session_segment(&mut send, &mut writer, PROTO_OPEN_SESSION_RESPONSE, &[]).await;
    let upstream = match TcpStream::connect(&target).await {
        Ok(s) => s,
        Err(_) => return,
    };
    send_data_segment(&mut send, &mut writer, 0, &socks5_success_reply()).await;

    let (mut up_read, mut up_write) = upstream.into_split();

    // Client -> server: decrypt data segments, forward plaintext upstream.
    let c2s = tokio::spawn(async move {
        while let Some((_proto, payload)) = recv_segment(&mut recv, &mut reader).await {
            if payload.is_empty() {
                continue;
            }
            if up_write.write_all(&payload).await.is_err() {
                return;
            }
        }
    });
    // Server -> client: read upstream, frame as data segments.
    let s2c = tokio::spawn(async move {
        let mut buf = [0u8; 4096];
        let mut seq = 1u32;
        loop {
            match up_read.read(&mut buf).await {
                Ok(0) | Err(_) => return,
                Ok(n) => {
                    send_data_segment(&mut send, &mut writer, seq, &buf[..n]).await;
                    seq = seq.wrapping_add(1);
                }
            }
        }
    });
    let _ = tokio::join!(c2s, s2c);
}

async fn spawn_fake_mieru_server(password: &str) -> SocketAddr {
    let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).await.unwrap();
    let addr = listener.local_addr().unwrap();
    let keys = candidate_keys(password.as_bytes(), MIERU_USER.as_bytes());
    tokio::spawn(async move {
        while let Ok((conn, _)) = listener.accept().await {
            let keys = keys.clone();
            tokio::spawn(serve_mieru(conn, keys));
        }
    });
    addr
}

// ---------------------------------------------------------------------------
// SOCKS5 client + harness.
// ---------------------------------------------------------------------------

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

fn mieru_mode(server: SocketAddr, password: &str) -> OutboundMode {
    OutboundMode::Mieru(Box::new(MieruOutboundConfig {
        server: server.ip().to_string(),
        port: server.port(),
        username: MIERU_USER.to_string(),
        password: password.to_string(),
    }))
}

async fn start_kernel(outbound: OutboundMode) -> GripeHandle {
    GripeKernel::start(GripeConfig {
        socks_listen: SocketAddr::from((Ipv4Addr::LOCALHOST, 0)),
        outbound,
    })
    .await
    .unwrap()
}

#[tokio::test]
async fn relays_through_mieru() {
    let echo = spawn_echo_server().await;
    let server = spawn_fake_mieru_server(MIERU_PASS).await;
    let handle = start_kernel(mieru_mode(server, MIERU_PASS)).await;

    let mut conn = socks5_connect(handle.local_addr(), echo).await;
    let payload = b"hello mieru tcp underlay";
    conn.write_all(payload).await.unwrap();
    let mut buf = vec![0u8; payload.len()];
    conn.read_exact(&mut buf).await.unwrap();
    assert_eq!(&buf, payload);

    handle.shutdown().await;
}

#[tokio::test]
async fn relays_large_payload_across_many_segments() {
    // 256 KiB exceeds the 32 KiB single-segment cap in both directions, forcing
    // a long run of implicit-nonce-incremented segments to round-trip intact.
    let echo = spawn_echo_server().await;
    let server = spawn_fake_mieru_server(MIERU_PASS).await;
    let handle = start_kernel(mieru_mode(server, MIERU_PASS)).await;

    let conn = socks5_connect(handle.local_addr(), echo).await;
    let payload: Vec<u8> = (0..256 * 1024).map(|i| (i % 251) as u8).collect();

    let to_send = payload.clone();
    let writer = tokio::spawn(async move {
        let (mut r, mut w) = conn.into_split();
        let write = tokio::spawn(async move {
            w.write_all(&to_send).await.unwrap();
            w.shutdown().await.unwrap();
        });
        let mut got = Vec::new();
        let mut buf = [0u8; 8192];
        loop {
            match r.read(&mut buf).await {
                Ok(0) | Err(_) => break,
                Ok(n) => got.extend_from_slice(&buf[..n]),
            }
        }
        write.await.unwrap();
        got
    });

    let got = writer.await.unwrap();
    assert_eq!(got.len(), payload.len());
    assert_eq!(got, payload);

    handle.shutdown().await;
}

#[tokio::test]
async fn wrong_password_fails_to_open_session() {
    let echo = spawn_echo_server().await;
    // Server expects MIERU_PASS; client offers a different password, so the
    // derived key differs and the server cannot open the first segment.
    let server = spawn_fake_mieru_server(MIERU_PASS).await;
    let handle = start_kernel(mieru_mode(server, "wrong-password")).await;

    let mut stream = TcpStream::connect(handle.local_addr()).await.unwrap();
    stream.write_all(&[0x05, 0x01, 0x00]).await.unwrap();
    let mut selection = [0u8; 2];
    stream.read_exact(&mut selection).await.unwrap();
    assert_eq!(selection, [0x05, 0x00]);

    let ip = match echo.ip() {
        IpAddr::V4(v4) => v4.octets(),
        IpAddr::V6(_) => panic!("test uses IPv4"),
    };
    let mut request = vec![0x05, 0x01, 0x00, 0x01];
    request.extend_from_slice(&ip);
    request.extend_from_slice(&echo.port().to_be_bytes());
    stream.write_all(&request).await.unwrap();

    // The server drops the connection (cannot decrypt), so the inbound reports a
    // SOCKS5 failure rather than success.
    let mut reply = [0u8; 10];
    match stream.read_exact(&mut reply).await {
        Ok(_) => assert_ne!(reply[1], 0x00, "mismatched key must not report SOCKS5 success"),
        Err(_) => {} // connection closed before a reply is also acceptable
    }

    handle.shutdown().await;
}
