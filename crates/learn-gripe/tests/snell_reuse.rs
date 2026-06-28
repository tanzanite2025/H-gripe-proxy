//! End-to-end proof that Snell v2 reuses one TCP connection for several
//! sequential streams: a SOCKS5 client -> gripe inbound -> Snell v2 outbound ->
//! fake reuse-capable Snell server.
//!
//! The fake server is an independent re-implementation of the Snell v2 wire
//! format *with* session reuse (CommandConnectV2 + half-close): after the one
//! handshake it loops, serving one logical stream at a time on the same
//! shadowaead chunk stream (continuous Argon2id subkey + counter nonces). Each
//! stream is echoed until the client's zero-length chunk (its half-close); the
//! server then sends its own zero-length chunk (a clean logical EOF) and reads
//! the next request header on the same connection. We assert several SOCKS5
//! round trips share a single accepted TCP connection and each used
//! CommandConnectV2.

use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use aes_gcm::Aes128Gcm;
use aes_gcm::aead::generic_array::GenericArray;
use aes_gcm::aead::{Aead, KeyInit, Payload};
use argon2::{Algorithm, Argon2, Params, Version};
use learn_gripe::{GripeConfig, GripeKernel, OutboundMode, SnellOutboundConfig};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

const TEST_PSK: &[u8] = b"snell-reuse-e2e-psk";
const SALT_LEN: usize = 16;
const TAG_LEN: usize = 16;
const RESP_TUNNEL: u8 = 0;
const COMMAND_CONNECT_V2: u8 = 5;

/// Snell's session-subkey KDF (independent of the kernel's copy).
fn snell_kdf(psk: &[u8], salt: &[u8]) -> Vec<u8> {
    let params = Params::new(8, 3, 1, Some(32)).unwrap();
    let argon2 = Argon2::new(Algorithm::Argon2id, Version::V0x13, params);
    let mut out = [0u8; 32];
    argon2.hash_password_into(psk, salt, &mut out).unwrap();
    // v2 uses AES-128-GCM (16-byte key).
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

enum Chunk {
    Data(Vec<u8>),
    /// A zero-length chunk: the client's half-close.
    Zero,
    /// The transport closed.
    Eof,
}

async fn read_chunk(stream: &mut TcpStream, cipher: &Cipher, nonce: &mut [u8; 12]) -> Chunk {
    let mut sealed_len = [0u8; 2 + TAG_LEN];
    if stream.read_exact(&mut sealed_len).await.is_err() {
        return Chunk::Eof;
    }
    let len_plain = cipher.open(nonce, &sealed_len);
    increment_nonce(nonce);
    let clen = u16::from_be_bytes([len_plain[0], len_plain[1]]) as usize;
    if clen == 0 {
        return Chunk::Zero;
    }
    let mut sealed = vec![0u8; clen + TAG_LEN];
    stream.read_exact(&mut sealed).await.expect("read chunk body");
    let plain = cipher.open(nonce, &sealed);
    increment_nonce(nonce);
    Chunk::Data(plain)
}

async fn write_chunk(stream: &mut TcpStream, cipher: &Cipher, nonce: &mut [u8; 12], plaintext: &[u8]) {
    let len = (plaintext.len() as u16).to_be_bytes();
    let sealed_len = cipher.seal(nonce, &len);
    increment_nonce(nonce);
    let sealed = cipher.seal(nonce, plaintext);
    increment_nonce(nonce);
    stream.write_all(&sealed_len).await.unwrap();
    stream.write_all(&sealed).await.unwrap();
    stream.flush().await.unwrap();
}

/// The half-close: a single sealed zero-length field, no payload block (matching
/// mihomo's shadowaead empty write / `writeZeroChunk`).
async fn write_zero_chunk(stream: &mut TcpStream, cipher: &Cipher, nonce: &mut [u8; 12]) {
    let sealed_len = cipher.seal(nonce, &[0u8, 0u8]);
    increment_nonce(nonce);
    stream.write_all(&sealed_len).await.unwrap();
    stream.flush().await.unwrap();
}

/// Serve sequential reused streams on one connection, recording each request's
/// command byte.
async fn serve_reuse(mut stream: TcpStream, commands: Arc<std::sync::Mutex<Vec<u8>>>) {
    let mut salt = [0u8; SALT_LEN];
    if stream.read_exact(&mut salt).await.is_err() {
        return;
    }
    let read_cipher = Cipher::new(&snell_kdf(TEST_PSK, &salt));
    let mut read_nonce = [0u8; 12];

    let mut salt_w = [0u8; SALT_LEN];
    for (i, b) in salt_w.iter_mut().enumerate() {
        *b = (i as u8).wrapping_mul(13).wrapping_add(5);
    }
    stream.write_all(&salt_w).await.unwrap();
    let write_cipher = Cipher::new(&snell_kdf(TEST_PSK, &salt_w));
    let mut write_nonce = [0u8; 12];

    loop {
        let header = match read_chunk(&mut stream, &read_cipher, &mut read_nonce).await {
            Chunk::Data(h) => h,
            Chunk::Zero | Chunk::Eof => return,
        };
        assert_eq!(header[0], 1, "snell proto byte");
        commands.lock().unwrap().push(header[1]);
        write_chunk(&mut stream, &write_cipher, &mut write_nonce, &[RESP_TUNNEL]).await;

        loop {
            match read_chunk(&mut stream, &read_cipher, &mut read_nonce).await {
                Chunk::Data(d) => write_chunk(&mut stream, &write_cipher, &mut write_nonce, &d).await,
                Chunk::Zero => {
                    write_zero_chunk(&mut stream, &write_cipher, &mut write_nonce).await;
                    break;
                }
                Chunk::Eof => return,
            }
        }
    }
}

async fn spawn_reuse_server() -> (SocketAddr, Arc<AtomicUsize>, Arc<std::sync::Mutex<Vec<u8>>>) {
    let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).await.unwrap();
    let addr = listener.local_addr().unwrap();
    let conns = Arc::new(AtomicUsize::new(0));
    let commands = Arc::new(std::sync::Mutex::new(Vec::new()));
    let conns_task = conns.clone();
    let commands_task = commands.clone();
    tokio::spawn(async move {
        while let Ok((stream, _)) = listener.accept().await {
            conns_task.fetch_add(1, Ordering::SeqCst);
            tokio::spawn(serve_reuse(stream, commands_task.clone()));
        }
    });
    (addr, conns, commands)
}

fn snell_v2(server: SocketAddr) -> Box<SnellOutboundConfig> {
    Box::new(SnellOutboundConfig {
        server: server.ip().to_string(),
        port: server.port(),
        psk: TEST_PSK.to_vec(),
        version: 2,
        obfs: None,
        reuse: false,
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
/// Snell zero-length chunk, ending the logical stream so the connection can be
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
async fn v2_reuses_one_connection_across_sequential_streams() {
    let (server, conns, commands) = spawn_reuse_server().await;
    let handle = GripeKernel::start(GripeConfig {
        socks_listen: SocketAddr::from((Ipv4Addr::LOCALHOST, 0)),
        outbound: OutboundMode::Snell(snell_v2(server)),
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
        "every request used CommandConnectV2",
    );

    handle.shutdown().await;
}

#[tokio::test]
async fn v2_reuse_round_trips_a_large_payload() {
    let (server, conns, _commands) = spawn_reuse_server().await;
    let handle = GripeKernel::start(GripeConfig {
        socks_listen: SocketAddr::from((Ipv4Addr::LOCALHOST, 0)),
        outbound: OutboundMode::Snell(snell_v2(server)),
    })
    .await
    .unwrap();
    let proxy = handle.local_addr();
    let target = SocketAddr::from((Ipv4Addr::new(1, 2, 3, 4), 443));

    // First (small) stream parks the session; the second rides it with a payload
    // spanning many AEAD chunks, exercising continuous nonces across reuse.
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
