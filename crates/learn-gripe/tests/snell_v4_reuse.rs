//! End-to-end proof that Snell v4/v5 reuses one TCP connection for several
//! sequential streams when `reuse` is enabled: a SOCKS5 client -> gripe inbound
//! -> Snell v4 outbound (`reuse: true`) -> fake reuse-capable v4 server.
//!
//! The fake server is an independent re-implementation of the Snell v4 frame
//! wire format (distinct from the shadowaead chunk framing) *with* session
//! reuse (CommandConnectV2 + zero-payload-frame half-close): after the one salt
//! exchange it loops, serving one logical stream at a time on the same frame
//! stream (continuous Argon2id subkey + counter nonces). Each stream is echoed
//! until the client's zero-payload frame (its half-close); the server then sends
//! its own zero-payload frame (a clean logical EOF) and reads the next request
//! header on the same connection. We assert several SOCKS5 round trips share a
//! single accepted TCP connection, each used CommandConnectV2, and that with
//! `reuse: false` every stream dials a fresh connection instead.

use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use aes_gcm::Aes128Gcm;
use aes_gcm::aead::generic_array::GenericArray;
use aes_gcm::aead::{Aead, KeyInit, Payload};
use argon2::{Algorithm, Argon2, Params, Version};
use learn_gripe::{GripeConfig, GripeKernel, OutboundMode, SnellOutboundConfig};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

const TEST_PSK: &[u8] = b"snell-v4-reuse-e2e-psk";
const SALT_LEN: usize = 16;
const TAG_LEN: usize = 16;
const HEADER_PLAIN: usize = 7;
const HEADER_CIPHER: usize = HEADER_PLAIN + TAG_LEN;
const FRAME_BYTE: u8 = 4;
const RESP_TUNNEL: u8 = 0;
const COMMAND_CONNECT: u8 = 1;
const COMMAND_CONNECT_V2: u8 = 5;
/// The fake server's own first-frame padding length (any value the client must
/// tolerate); kept in a plausible range to mirror a real peer.
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
    /// A decrypted payload.
    Data(Vec<u8>),
    /// A zero-payload frame: the client's half-close (logical EOF).
    Zero,
    /// The transport closed.
    Eof,
}

/// Read one v4 frame from the client, advancing the read nonce. A zero-payload
/// frame is reported as `Zero` (half-close) and a transport close as `Eof`.
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
        return Frame::Zero;
    }
    let mut frame = vec![0u8; padding + payload + TAG_LEN];
    stream.read_exact(&mut frame).await.expect("read frame body");
    if padding > 0 {
        let (pad_part, pay_part) = frame.split_at_mut(padding);
        swap_padding(pad_part, pay_part);
    }
    let plain = cipher.open(nonce, &frame[padding..]);
    increment_nonce(nonce);
    Frame::Data(plain)
}

/// Write one v4 frame to the client, prefixing the salt + initial padding on the
/// first frame of the whole connection.
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

/// The v4 half-close: a single zero-payload frame (sealed header with
/// `payLen == 0`, no padding, one nonce step). Never carries the salt — by the
/// time the server half-closes it has already emitted its salt on the reply.
async fn write_zero_frame(stream: &mut TcpStream, cipher: &Cipher, nonce: &mut [u8; 12]) {
    let mut header = [0u8; HEADER_PLAIN];
    header[0] = FRAME_BYTE;
    // padding-len and payload-len both 0.
    let sealed_header = cipher.seal(nonce, &header);
    increment_nonce(nonce);
    stream.write_all(&sealed_header).await.unwrap();
    stream.flush().await.unwrap();
}

/// Serve sequential reused streams on one v4 connection, recording each
/// request's command byte.
async fn serve_v4_reuse(mut stream: TcpStream, commands: Arc<Mutex<Vec<u8>>>) {
    let mut salt = [0u8; SALT_LEN];
    if stream.read_exact(&mut salt).await.is_err() {
        return;
    }
    let read_cipher = Cipher::new(&snell_kdf(TEST_PSK, &salt));
    let mut read_nonce = [0u8; 12];

    // The server's own salt for the reply direction, sent once on its first
    // frame and reused (continuous nonces) across every logical stream.
    let mut salt_w = [0u8; SALT_LEN];
    for (i, b) in salt_w.iter_mut().enumerate() {
        *b = (i as u8).wrapping_mul(17).wrapping_add(3);
    }
    let write_cipher = Cipher::new(&snell_kdf(TEST_PSK, &salt_w));
    let mut write_nonce = [0u8; 12];
    let mut write_salt_sent = false;

    loop {
        // The first frame of each logical stream is its request header.
        let header = match read_frame(&mut stream, &read_cipher, &mut read_nonce).await {
            Frame::Data(h) => h,
            Frame::Zero | Frame::Eof => return,
        };
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
                Frame::Data(d) => {
                    write_frame(
                        &mut stream,
                        &write_cipher,
                        &mut write_nonce,
                        &salt_w,
                        &mut write_salt_sent,
                        &d,
                    )
                    .await;
                }
                Frame::Zero => {
                    // Client half-closed; reply with our own zero frame and
                    // serve the next logical stream on this same connection.
                    write_zero_frame(&mut stream, &write_cipher, &mut write_nonce).await;
                    break;
                }
                Frame::Eof => return,
            }
        }
    }
}

async fn spawn_v4_reuse_server() -> (SocketAddr, Arc<AtomicUsize>, Arc<Mutex<Vec<u8>>>) {
    let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).await.unwrap();
    let addr = listener.local_addr().unwrap();
    let conns = Arc::new(AtomicUsize::new(0));
    let commands = Arc::new(Mutex::new(Vec::new()));
    let conns_task = conns.clone();
    let commands_task = commands.clone();
    tokio::spawn(async move {
        while let Ok((stream, _)) = listener.accept().await {
            conns_task.fetch_add(1, Ordering::SeqCst);
            tokio::spawn(serve_v4_reuse(stream, commands_task.clone()));
        }
    });
    (addr, conns, commands)
}

fn snell_v4(server: SocketAddr, version: u8, reuse: bool) -> Box<SnellOutboundConfig> {
    Box::new(SnellOutboundConfig {
        server: server.ip().to_string(),
        port: server.port(),
        psk: TEST_PSK.to_vec(),
        version,
        obfs: None,
        reuse,
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

/// One SOCKS5 round trip: send `payload`, half-close the upload, read the echo
/// back to EOF. Half-closing the client's write drives the kernel to send the
/// Snell zero-payload frame, ending the logical stream so the connection can be
/// pooled for reuse.
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
async fn v4_reuses_one_connection_across_sequential_streams() {
    let (server, conns, commands) = spawn_v4_reuse_server().await;
    let handle = GripeKernel::start(GripeConfig {
        socks_listen: SocketAddr::from((Ipv4Addr::LOCALHOST, 0)),
        outbound: OutboundMode::Snell(snell_v4(server, 4, true)),
    })
    .await
    .unwrap();
    let proxy = handle.local_addr();
    let target = SocketAddr::from((Ipv4Addr::new(1, 2, 3, 4), 443));

    let payloads: [&[u8]; 3] = [b"first stream", b"second stream", b"third stream"];
    for payload in payloads {
        socks5_round_trip(proxy, target, payload).await;
        // Let the relay's stream drop (which parks the session) land before the
        // next connect tries to reuse it.
        tokio::time::sleep(Duration::from_millis(150)).await;
    }

    assert_eq!(
        conns.load(Ordering::SeqCst),
        1,
        "all three sequential streams shared a single TCP connection",
    );
    assert_eq!(
        *commands.lock().unwrap(),
        vec![COMMAND_CONNECT_V2, COMMAND_CONNECT_V2, COMMAND_CONNECT_V2],
        "every reused request used CommandConnectV2",
    );

    handle.shutdown().await;
}

#[tokio::test]
async fn v4_reuse_round_trips_a_large_payload() {
    let (server, conns, _commands) = spawn_v4_reuse_server().await;
    let handle = GripeKernel::start(GripeConfig {
        socks_listen: SocketAddr::from((Ipv4Addr::LOCALHOST, 0)),
        outbound: OutboundMode::Snell(snell_v4(server, 4, true)),
    })
    .await
    .unwrap();
    let proxy = handle.local_addr();
    let target = SocketAddr::from((Ipv4Addr::new(1, 2, 3, 4), 443));

    // First (small) stream parks the session; the second rides it with a payload
    // spanning many frames (each capped at 0x3FFF), exercising continuous nonces
    // across reuse.
    socks5_round_trip(proxy, target, b"warmup").await;
    tokio::time::sleep(Duration::from_millis(150)).await;
    let big: Vec<u8> = (0..50_000u32).map(|i| (i % 251) as u8).collect();
    socks5_round_trip(proxy, target, &big).await;
    tokio::time::sleep(Duration::from_millis(150)).await;

    assert_eq!(
        conns.load(Ordering::SeqCst),
        1,
        "the large second stream reused the first connection",
    );

    handle.shutdown().await;
}

#[tokio::test]
async fn v5_config_with_reuse_reuses_one_connection() {
    let (server, conns, commands) = spawn_v4_reuse_server().await;
    let handle = GripeKernel::start(GripeConfig {
        socks_listen: SocketAddr::from((Ipv4Addr::LOCALHOST, 0)),
        // v5 dials as v4 on the wire (version >= 4); with `reuse` it pools just
        // like v4.
        outbound: OutboundMode::Snell(snell_v4(server, 5, true)),
    })
    .await
    .unwrap();
    let proxy = handle.local_addr();
    let target = SocketAddr::from((Ipv4Addr::new(1, 2, 3, 4), 443));

    socks5_round_trip(proxy, target, b"first").await;
    tokio::time::sleep(Duration::from_millis(150)).await;
    socks5_round_trip(proxy, target, b"second").await;
    tokio::time::sleep(Duration::from_millis(150)).await;

    assert_eq!(
        conns.load(Ordering::SeqCst),
        1,
        "v5 + reuse shares a single TCP connection",
    );
    assert_eq!(*commands.lock().unwrap(), vec![COMMAND_CONNECT_V2, COMMAND_CONNECT_V2],);

    handle.shutdown().await;
}

#[tokio::test]
async fn v4_without_reuse_dials_a_fresh_connection_per_stream() {
    let (server, conns, commands) = spawn_v4_reuse_server().await;
    let handle = GripeKernel::start(GripeConfig {
        socks_listen: SocketAddr::from((Ipv4Addr::LOCALHOST, 0)),
        outbound: OutboundMode::Snell(snell_v4(server, 4, false)),
    })
    .await
    .unwrap();
    let proxy = handle.local_addr();
    let target = SocketAddr::from((Ipv4Addr::new(1, 2, 3, 4), 443));

    for payload in [b"alpha".as_slice(), b"bravo".as_slice(), b"charlie".as_slice()] {
        socks5_round_trip(proxy, target, payload).await;
        tokio::time::sleep(Duration::from_millis(150)).await;
    }

    assert_eq!(
        conns.load(Ordering::SeqCst),
        3,
        "without `reuse` each stream dials its own connection",
    );
    assert_eq!(
        *commands.lock().unwrap(),
        vec![COMMAND_CONNECT, COMMAND_CONNECT, COMMAND_CONNECT],
        "one-shot v4 uses the plain connect command",
    );

    handle.shutdown().await;
}
