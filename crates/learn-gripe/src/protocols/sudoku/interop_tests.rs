//! Full-stack interop test: a fake Sudoku server built from the same internal
//! primitives the client uses, exercised end-to-end through [`super::connect`].
//!
//! The fake server runs the mirror of the client stack — obfuscation (decode
//! the client uplink / encode the client downlink), the AEAD record layer with
//! swapped directional bases, the KIP `ClientHello`/`ServerHello` X25519
//! handshake, then reads the `OpenTCP` request and echoes the relayed bytes.
//! A passing round-trip proves every layer lines up byte-for-byte.

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use x25519_dalek::{EphemeralSecret, PublicKey};

use crate::address::TargetAddr;

use super::kip::{
    self, KIP_TYPE_CLIENT_HELLO, KIP_TYPE_OPEN_TCP, KIP_TYPE_SERVER_HELLO, derive_psk_bases, derive_session_bases,
    read_message, write_message,
};
use super::obfs::ObfsStream;
use super::record::{AeadMethod, RecordStream};
use super::{SudokuOutboundConfig, connect, table};

const KEY: &str = "interop-test-key";
const TABLE_TYPE: &str = "prefer_entropy";

fn config(port: u16, method: AeadMethod) -> SudokuOutboundConfig {
    SudokuOutboundConfig {
        server: "127.0.0.1".to_string(),
        port,
        key: KEY.to_string(),
        aead_method: method,
        table_type: TABLE_TYPE.to_string(),
        custom_pattern: String::new(),
        padding_min: 0,
        padding_max: 0,
    }
}

/// Accept one connection and mirror the client stack, asserting the handshake
/// fields and `OpenTCP` address, then echo all relayed bytes.
async fn run_fake_server(listener: TcpListener, method: AeadMethod, expected_addr: Vec<u8>) {
    let (sock, _) = listener.accept().await.expect("accept");

    // Server obfuscation: decode the client's uplink, encode its downlink.
    let tables = table::new_directional_table(KEY, TABLE_TYPE, "").expect("server table");
    let obfs = ObfsStream::new(sock, tables.downlink, tables.uplink, 0, 0);

    // Record layer with swapped directional bases (server send = s2c).
    let (psk_c2s, psk_s2c) = derive_psk_bases(KEY);
    let mut rec = RecordStream::new(obfs, method, &psk_s2c, &psk_c2s).expect("server record");

    // --- KIP ClientHello ---
    let (typ, payload) = read_message(&mut rec).await.expect("read ClientHello");
    assert_eq!(typ, KIP_TYPE_CLIENT_HELLO);
    // ts(8) | user_hash(8) | nonce(16) | client_pub(32) | features(4)
    assert_eq!(payload.len(), 8 + 8 + 16 + 32 + 4);
    let mut nonce = [0u8; 16];
    nonce.copy_from_slice(&payload[16..32]);
    let mut client_pub = [0u8; 32];
    client_pub.copy_from_slice(&payload[32..64]);

    // --- ServerHello + ECDH ---
    let server_secret = EphemeralSecret::random();
    let server_pub = PublicKey::from(&server_secret);
    let shared = server_secret.diffie_hellman(&PublicKey::from(client_pub));
    let (session_c2s, session_s2c) = derive_session_bases(KEY, shared.as_bytes(), &nonce);

    let mut hello = Vec::with_capacity(16 + 32 + 4);
    hello.extend_from_slice(&nonce);
    hello.extend_from_slice(server_pub.as_bytes());
    hello.extend_from_slice(&1u32.to_be_bytes()); // selected features (OpenTCP)
    write_message(&mut rec, KIP_TYPE_SERVER_HELLO, &hello)
        .await
        .expect("write ServerHello");
    rec.flush().await.expect("flush ServerHello");

    rec.rekey(&session_s2c, &session_c2s).expect("server rekey");

    // --- OpenTCP request ---
    let (typ, addr) = read_message(&mut rec).await.expect("read OpenTCP");
    assert_eq!(typ, KIP_TYPE_OPEN_TCP);
    assert_eq!(addr, expected_addr, "OpenTCP address mismatch");

    // --- echo relayed bytes ---
    let mut buf = vec![0u8; 16 * 1024];
    loop {
        match rec.read(&mut buf).await {
            Ok(0) | Err(_) => break,
            Ok(n) => {
                if rec.write_all(&buf[..n]).await.is_err() {
                    break;
                }
                let _ = rec.flush().await;
            }
        }
    }
}

async fn round_trip(method: AeadMethod, payload: Vec<u8>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
    let port = listener.local_addr().expect("addr").port();

    let target = TargetAddr::Domain("example.com".to_string(), 443);
    let expected_addr = kip::encode_address(&target).expect("encode addr");

    let server = tokio::spawn(run_fake_server(listener, method, expected_addr));

    let cfg = config(port, method);
    let mut stream = connect(&cfg, &target).await.expect("client connect");

    stream.write_all(&payload).await.expect("client write");
    stream.flush().await.expect("client flush");

    let mut got = vec![0u8; payload.len()];
    stream.read_exact(&mut got).await.expect("client read echo");
    assert_eq!(got, payload);

    drop(stream);
    let _ = server.await;
}

#[tokio::test]
async fn chacha_full_stack_round_trip_small() {
    round_trip(AeadMethod::ChaCha20Poly1305, b"hello sudoku interop".to_vec()).await;
}

#[tokio::test]
async fn chacha_full_stack_round_trip_near_mtu() {
    round_trip(
        AeadMethod::ChaCha20Poly1305,
        (0..1400u32).map(|i| (i * 7) as u8).collect(),
    )
    .await;
}

#[tokio::test]
async fn aes_gcm_full_stack_round_trip() {
    round_trip(AeadMethod::Aes128Gcm, (0..3000u32).map(|i| i as u8).collect()).await;
}
