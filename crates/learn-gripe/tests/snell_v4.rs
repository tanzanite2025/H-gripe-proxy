//! End-to-end proof that the Snell v4/v5 frame stream interoperates: a SOCKS5
//! client -> gripe inbound -> Snell v4 outbound -> fake v4 server.
//!
//! The fake server is an independent re-implementation of the Snell v4 wire
//! format (distinct from the shadowaead chunk framing used by v1/v2/v3): the
//! same Argon2id subkey + AES-128-GCM + counter nonce, but each frame is
//! `AEAD(7-byte header) | [padding] | AEAD(payload)` and the first frame is
//! prefixed with the salt and an initial random padding block byte-interleaved
//! ("swapped") with the payload ciphertext. It decodes the client's request
//! header, replies `Tunnel`, then echoes payload frames until the client
//! closes. We assert a SOCKS5 round trip (small and large) works and that the
//! client's first frame carried the salt + an in-range initial padding block.

use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};

use aes_gcm::Aes128Gcm;
use aes_gcm::aead::generic_array::GenericArray;
use aes_gcm::aead::{Aead, KeyInit, Payload};
use argon2::{Algorithm, Argon2, Params, Version};
use learn_gripe::{GripeConfig, GripeKernel, OutboundMode, SnellOutboundConfig};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

const TEST_PSK: &[u8] = b"snell-v4-e2e-psk";
const SALT_LEN: usize = 16;
const TAG_LEN: usize = 16;
const HEADER_PLAIN: usize = 7;
const HEADER_CIPHER: usize = HEADER_PLAIN + TAG_LEN;
const FRAME_BYTE: u8 = 4;
const RESP_TUNNEL: u8 = 0;
const COMMAND_CONNECT: u8 = 1;
const INITIAL_PADDING_MIN: usize = 0x100;
const INITIAL_PADDING_SPAN: usize = 0x100;
/// The fake server's own first-frame padding length (any value the client must
/// tolerate); kept in range to mirror a real peer.
const SERVER_INITIAL_PADDING: usize = 0x180;

/// Snell's session-subkey KDF (independent of the kernel's copy); v4 is always
/// AES-128-GCM (16-byte key).
fn snell_kdf(psk: &[u8], salt: &[u8]) -> Vec<u8> {
    let params = Params::new(8, 3, 1, Some(32)).unwrap();
    let argon2 = Argon2::new(Algorithm::Argon2id, Version::V0x13, params);
    let mut out = [0u8; 32];
    argon2.hash_password_into(psk, salt, &mut out).unwrap();
    out[..16].to_vec()
}

fn increment_nonce(nonce: &mut [u8; 12]) {
    for byte in nonce.iter_mut() {
        *byte = byte.wrapping_add(1);
        if *byte != 0 {
            break;
        }
    }
}

/// Its own inverse: swap every even byte between padding and payload ciphertext.
fn swap_padding(padding: &mut [u8], payload_cipher: &mut [u8]) {
    let limit = padding.len().min(payload_cipher.len());
    let mut i = 0;
    while i < limit {
        std::mem::swap(&mut padding[i], &mut payload_cipher[i]);
        i += 2;
    }
}

struct Cipher(Box<Aes128Gcm>);

impl Cipher {
    fn new(subkey: &[u8]) -> Self {
        Cipher(Box::new(Aes128Gcm::new_from_slice(subkey).unwrap()))
    }

    fn seal(&self, nonce: &[u8; 12], plaintext: &[u8]) -> Vec<u8> {
        self.0
            .encrypt(
                GenericArray::from_slice(nonce),
                Payload {
                    msg: plaintext,
                    aad: &[],
                },
            )
            .unwrap()
    }

    fn open(&self, nonce: &[u8; 12], ciphertext: &[u8]) -> Vec<u8> {
        self.0
            .decrypt(
                GenericArray::from_slice(nonce),
                Payload {
                    msg: ciphertext,
                    aad: &[],
                },
            )
            .unwrap()
    }
}

enum Frame {
    /// A decrypted payload plus the frame's padding length.
    Data { plain: Vec<u8>, padding: usize },
    /// A zero-payload frame (logical EOF) or a transport close.
    Eof,
}

/// Read one v4 frame from the client. The first call must read the leading salt
/// (the caller passes `None` for `cipher` until then).
async fn read_frame(stream: &mut TcpStream, cipher: &Cipher, nonce: &mut [u8; 12]) -> Frame {
    let mut header_cipher = [0u8; HEADER_CIPHER];
    if stream.read_exact(&mut header_cipher).await.is_err() {
        return Frame::Eof;
    }
    let header = cipher.open(nonce, &header_cipher);
    increment_nonce(nonce);
    assert_eq!(header.len(), HEADER_PLAIN);
    assert_eq!(header[0], FRAME_BYTE, "v4 frame marker");
    let padding = u16::from_be_bytes([header[3], header[4]]) as usize;
    let payload = u16::from_be_bytes([header[5], header[6]]) as usize;
    if payload == 0 {
        return Frame::Eof;
    }
    let mut frame = vec![0u8; padding + payload + TAG_LEN];
    stream.read_exact(&mut frame).await.expect("read frame body");
    if padding > 0 {
        let (pad_part, pay_part) = frame.split_at_mut(padding);
        swap_padding(pad_part, pay_part);
    }
    let plain = cipher.open(nonce, &frame[padding..]);
    increment_nonce(nonce);
    Frame::Data { plain, padding }
}

/// Write one v4 frame to the client, prefixing the salt + initial padding on the
/// first frame.
async fn write_frame(
    stream: &mut TcpStream,
    cipher: &Cipher,
    nonce: &mut [u8; 12],
    salt: &[u8; SALT_LEN],
    salt_sent: &mut bool,
    payload: &[u8],
) {
    let first = !*salt_sent;
    let padding_len = if first && !payload.is_empty() {
        SERVER_INITIAL_PADDING
    } else {
        0
    };

    let mut header = [0u8; HEADER_PLAIN];
    header[0] = FRAME_BYTE;
    header[3..5].copy_from_slice(&(padding_len as u16).to_be_bytes());
    header[5..7].copy_from_slice(&(payload.len() as u16).to_be_bytes());
    let sealed_header = cipher.seal(nonce, &header);
    increment_nonce(nonce);
    let mut payload_cipher = if payload.is_empty() {
        Vec::new()
    } else {
        let pc = cipher.seal(nonce, payload);
        increment_nonce(nonce);
        pc
    };

    let mut out = Vec::new();
    if first {
        out.extend_from_slice(salt);
        *salt_sent = true;
    }
    out.extend_from_slice(&sealed_header);
    if padding_len > 0 {
        let mut padding = vec![0u8; padding_len];
        for (i, b) in padding.iter_mut().enumerate() {
            *b = (i as u8).wrapping_mul(31).wrapping_add(7);
        }
        swap_padding(&mut padding, &mut payload_cipher);
        out.extend_from_slice(&padding);
    }
    out.extend_from_slice(&payload_cipher);
    stream.write_all(&out).await.unwrap();
    stream.flush().await.unwrap();
}

/// Serve one v4 connection: read the salt + request header, reply `Tunnel`,
/// then echo payload frames until the client closes.
async fn serve_v4(mut stream: TcpStream, first_padding: Arc<Mutex<Option<usize>>>, commands: Arc<Mutex<Vec<u8>>>) {
    let mut salt = [0u8; SALT_LEN];
    if stream.read_exact(&mut salt).await.is_err() {
        return;
    }
    let read_cipher = Cipher::new(&snell_kdf(TEST_PSK, &salt));
    let mut read_nonce = [0u8; 12];

    // The server's own salt for the reply direction.
    let mut salt_w = [0u8; SALT_LEN];
    for (i, b) in salt_w.iter_mut().enumerate() {
        *b = (i as u8).wrapping_mul(17).wrapping_add(3);
    }
    let write_cipher = Cipher::new(&snell_kdf(TEST_PSK, &salt_w));
    let mut write_nonce = [0u8; 12];
    let mut write_salt_sent = false;

    // First frame is the request header (carrying the client's initial padding).
    let (header, padding) = match read_frame(&mut stream, &read_cipher, &mut read_nonce).await {
        Frame::Data { plain, padding } => (plain, padding),
        Frame::Eof => return,
    };
    *first_padding.lock().unwrap() = Some(padding);
    assert_eq!(header[0], 1, "snell proto byte");
    commands.lock().unwrap().push(header[1]);

    write_frame(
        &mut stream,
        &write_cipher,
        &mut write_nonce,
        &salt_w,
        &mut write_salt_sent,
        &[RESP_TUNNEL],
    )
    .await;

    loop {
        match read_frame(&mut stream, &read_cipher, &mut read_nonce).await {
            Frame::Data { plain, .. } => {
                write_frame(
                    &mut stream,
                    &write_cipher,
                    &mut write_nonce,
                    &salt_w,
                    &mut write_salt_sent,
                    &plain,
                )
                .await;
            }
            Frame::Eof => return,
        }
    }
}

async fn spawn_v4_server() -> (
    SocketAddr,
    Arc<AtomicUsize>,
    Arc<Mutex<Option<usize>>>,
    Arc<Mutex<Vec<u8>>>,
) {
    let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).await.unwrap();
    let addr = listener.local_addr().unwrap();
    let conns = Arc::new(AtomicUsize::new(0));
    let first_padding = Arc::new(Mutex::new(None));
    let commands = Arc::new(Mutex::new(Vec::new()));
    let conns_task = conns.clone();
    let padding_task = first_padding.clone();
    let commands_task = commands.clone();
    tokio::spawn(async move {
        while let Ok((stream, _)) = listener.accept().await {
            conns_task.fetch_add(1, Ordering::SeqCst);
            tokio::spawn(serve_v4(stream, padding_task.clone(), commands_task.clone()));
        }
    });
    (addr, conns, first_padding, commands)
}

fn snell_v4(server: SocketAddr, version: u8) -> Box<SnellOutboundConfig> {
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

async fn socks5_round_trip(proxy: SocketAddr, target: SocketAddr, payload: &[u8]) {
    let mut conn = socks5_connect(proxy, target).await;
    let (mut reader, mut writer) = conn.split();
    writer.write_all(payload).await.unwrap();
    writer.shutdown().await.unwrap();
    let mut echoed = Vec::new();
    reader.read_to_end(&mut echoed).await.unwrap();
    assert_eq!(echoed, payload, "relayed echo round trips");
}

#[tokio::test]
async fn v4_round_trips_through_the_frame_stream() {
    let (server, _conns, first_padding, commands) = spawn_v4_server().await;
    let handle = GripeKernel::start(GripeConfig {
        socks_listen: SocketAddr::from((Ipv4Addr::LOCALHOST, 0)),
        outbound: OutboundMode::Snell(snell_v4(server, 4)),
    })
    .await
    .unwrap();
    let proxy = handle.local_addr();
    let target = SocketAddr::from((Ipv4Addr::new(1, 2, 3, 4), 443));

    socks5_round_trip(proxy, target, b"hello snell v4").await;

    assert_eq!(
        *commands.lock().unwrap(),
        vec![COMMAND_CONNECT],
        "v4 uses the plain connect command"
    );
    let padding = first_padding
        .lock()
        .unwrap()
        .expect("server saw the client's first frame");
    assert!(
        (INITIAL_PADDING_MIN..INITIAL_PADDING_MIN + INITIAL_PADDING_SPAN).contains(&padding),
        "first frame carried an in-range initial padding block (got {padding})",
    );

    handle.shutdown().await;
}

#[tokio::test]
async fn v4_round_trips_a_large_multi_frame_payload() {
    let (server, _conns, _first_padding, _commands) = spawn_v4_server().await;
    let handle = GripeKernel::start(GripeConfig {
        socks_listen: SocketAddr::from((Ipv4Addr::LOCALHOST, 0)),
        outbound: OutboundMode::Snell(snell_v4(server, 4)),
    })
    .await
    .unwrap();
    let proxy = handle.local_addr();
    let target = SocketAddr::from((Ipv4Addr::new(1, 2, 3, 4), 443));

    // Spans many frames (each capped at 0x3FFF), exercising continuous nonces.
    let big: Vec<u8> = (0..50_000u32).map(|i| (i % 251) as u8).collect();
    socks5_round_trip(proxy, target, &big).await;

    handle.shutdown().await;
}

#[tokio::test]
async fn v5_config_dials_as_v4() {
    let (server, _conns, _first_padding, commands) = spawn_v4_server().await;
    let handle = GripeKernel::start(GripeConfig {
        socks_listen: SocketAddr::from((Ipv4Addr::LOCALHOST, 0)),
        // v5 is normalised to v4 by `from_proxy`; constructing the config with
        // version 5 directly still exercises the v4 frame path (version >= 4).
        outbound: OutboundMode::Snell(snell_v4(server, 5)),
    })
    .await
    .unwrap();
    let proxy = handle.local_addr();
    let target = SocketAddr::from((Ipv4Addr::new(1, 2, 3, 4), 443));

    socks5_round_trip(proxy, target, b"hello snell v5").await;
    assert_eq!(*commands.lock().unwrap(), vec![COMMAND_CONNECT]);

    handle.shutdown().await;
}
