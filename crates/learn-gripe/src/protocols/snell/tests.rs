use super::crypto::{AeadCipher, SnellCipher, increment_nonce, snell_kdf};
use super::pool::{PooledSession, PooledSnell, SESSION_IDLE_TTL, SESSION_POOL, SnellServerKey, pool_put, pool_take};
use super::udp::{decode_udp_reply, encode_udp_addr};
use super::*;
use crate::config::outbound_opts::ProxyEntry;
use std::time::{Duration, Instant};

fn parse_entry(yaml: &str) -> ProxyEntry {
    serde_yaml_ng::from_str(yaml).expect("parse proxy entry")
}

#[test]
fn parses_snell_entry_defaults_to_v1() {
    let entry = parse_entry("name: s\ntype: snell\nserver: example.com\nport: 443\npsk: secret\n");
    let config = SnellOutboundConfig::from_proxy(&entry).unwrap();
    assert_eq!(config.server, "example.com");
    assert_eq!(config.port, 443);
    assert_eq!(config.psk, b"secret");
    assert_eq!(config.version, 1);
    assert_eq!(config.cipher(), SnellCipher::Chacha20Poly1305);
    assert_eq!(config.command(), COMMAND_CONNECT);
}

#[test]
fn version_selects_cipher_and_command() {
    let v2 = parse_entry("name: s\ntype: snell\nserver: h\nport: 1\npsk: p\nversion: 2\n");
    let v2 = SnellOutboundConfig::from_proxy(&v2).unwrap();
    assert_eq!(v2.cipher(), SnellCipher::Aes128Gcm);
    assert_eq!(v2.command(), COMMAND_CONNECT_V2);

    let v3 = parse_entry("name: s\ntype: snell\nserver: h\nport: 1\npsk: p\nversion: 3\n");
    let v3 = SnellOutboundConfig::from_proxy(&v3).unwrap();
    assert_eq!(v3.cipher(), SnellCipher::Aes128Gcm);
    assert_eq!(v3.command(), COMMAND_CONNECT);
}

#[test]
fn v4_and_v5_select_frame_path_and_v5_normalises_to_v4() {
    for version in [4, 5] {
        let entry = parse_entry(&format!(
            "name: s\ntype: snell\nserver: h\nport: 1\npsk: p\nversion: {version}\n"
        ));
        let config = SnellOutboundConfig::from_proxy(&entry).unwrap();
        // v5 dials as v4 (identical on the wire).
        assert_eq!(config.version, 4);
        assert!(config.uses_v4_framing());
        assert_eq!(config.cipher(), SnellCipher::Aes128Gcm);
        assert_eq!(config.command(), COMMAND_CONNECT);
        // v4/v5 carry UDP over the v4 frame stream.
        assert!(config.supports_udp());
    }
}

#[test]
fn rejects_missing_psk_and_bad_version() {
    let no_psk = parse_entry("name: s\ntype: snell\nserver: h\nport: 1\n");
    assert!(SnellOutboundConfig::from_proxy(&no_psk).is_err());
    let bad_version = parse_entry("name: s\ntype: snell\nserver: h\nport: 1\npsk: p\nversion: 6\n");
    assert!(SnellOutboundConfig::from_proxy(&bad_version).is_err());
}

#[test]
fn request_header_encodes_host_and_port() {
    let target = TargetAddr::Domain("example.com".to_string(), 443);
    let header = build_request_header(COMMAND_CONNECT, &target).unwrap();
    let mut expected = vec![SNELL_PROTO_BYTE, COMMAND_CONNECT, 0, 11];
    expected.extend_from_slice(b"example.com");
    expected.extend_from_slice(&443u16.to_be_bytes());
    assert_eq!(header, expected);
}

#[test]
fn supports_udp_on_v3_and_v4() {
    let cfg = |v: u8| SnellOutboundConfig {
        server: "h".into(),
        port: 1,
        psk: b"p".to_vec(),
        version: v,
        obfs: None,
        reuse: false,
    };
    assert!(!cfg(1).supports_udp());
    assert!(!cfg(2).supports_udp());
    assert!(cfg(3).supports_udp());
    // v4 (and v5, normalised to 4) carry UDP over the v4 frame stream.
    assert!(cfg(4).supports_udp());
}

#[test]
fn encodes_udp_addr_per_family() {
    let mut domain = Vec::new();
    encode_udp_addr(&mut domain, &TargetAddr::Domain("ex.com".into(), 443)).unwrap();
    let mut expected = vec![6u8];
    expected.extend_from_slice(b"ex.com");
    expected.extend_from_slice(&443u16.to_be_bytes());
    assert_eq!(domain, expected);

    let mut v4 = Vec::new();
    encode_udp_addr(&mut v4, &TargetAddr::Ip("1.2.3.4:443".parse().unwrap())).unwrap();
    assert_eq!(v4, vec![0, 4, 1, 2, 3, 4, 0x01, 0xbb]);

    let mut v6 = Vec::new();
    encode_udp_addr(&mut v6, &TargetAddr::Ip("[::1]:53".parse().unwrap())).unwrap();
    let mut expected_v6 = vec![0u8, 6];
    expected_v6.extend_from_slice(&std::net::Ipv6Addr::LOCALHOST.octets());
    expected_v6.extend_from_slice(&53u16.to_be_bytes());
    assert_eq!(v6, expected_v6);
}

#[test]
fn decodes_udp_reply_strips_source_address() {
    let mut v4 = vec![UDP_ADDR_IPV4, 9, 9, 9, 9, 0x00, 0x35];
    v4.extend_from_slice(b"payload");
    assert_eq!(decode_udp_reply(&v4).unwrap(), b"payload");

    let mut v6 = vec![UDP_ADDR_IPV6];
    v6.extend_from_slice(&std::net::Ipv6Addr::LOCALHOST.octets());
    v6.extend_from_slice(&53u16.to_be_bytes());
    v6.extend_from_slice(b"reply6");
    assert_eq!(decode_udp_reply(&v6).unwrap(), b"reply6");

    assert!(decode_udp_reply(&[0x03, 1, 2, 3]).is_err());
    assert!(decode_udp_reply(&[]).is_err());
}

#[test]
fn snell_kdf_truncates_argon2_output() {
    let psk = b"password";
    let salt = [0x11u8; SALT_LEN];
    let k16 = snell_kdf(psk, &salt, 16);
    let k32 = snell_kdf(psk, &salt, 32);
    assert_eq!(k16.len(), 16);
    assert_eq!(k32.len(), 32);
    // Truncation: the 16-byte key is the prefix of the 32-byte derivation.
    assert_eq!(&k32[..16], &k16[..]);
}

#[test]
fn obfs_parses_modes_and_rejects_unknown() {
    assert_eq!(SnellObfs::parse(None).unwrap(), None);
    // Empty `obfs-opts` (no mode) means no obfs.
    assert_eq!(SnellObfs::parse(Some(&ObfsOpts::default())).unwrap(), None);

    let http = SnellObfs::parse(Some(&ObfsOpts {
        mode: Some("http".into()),
        host: Some("a.example".into()),
    }))
    .unwrap();
    assert_eq!(
        http,
        Some(SnellObfs::Http {
            host: "a.example".into(),
            path: "/".into(),
        })
    );

    // Unset host defaults to a plausible value.
    let tls = SnellObfs::parse(Some(&ObfsOpts {
        mode: Some("tls".into()),
        host: None,
    }))
    .unwrap();
    assert_eq!(
        tls,
        Some(SnellObfs::Tls {
            host: "bing.com".into()
        })
    );

    let err = SnellObfs::parse(Some(&ObfsOpts {
        mode: Some("quic".into()),
        host: None,
    }))
    .unwrap_err();
    assert!(err.to_string().contains("unknown obfs mode"), "got: {err}");
}

// ---- v2 session reuse (CommandConnectV2 + half-close) -----------------

use std::net::{Ipv4Addr, SocketAddr};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

const REUSE_PSK: &[u8] = b"snell-reuse-psk";

/// One framing event read off the fake server's socket.
enum SrvChunk {
    Data(Vec<u8>),
    /// A zero-length chunk: the client's half-close.
    Zero,
    /// The transport closed.
    Eof,
}

async fn srv_read_chunk(stream: &mut TcpStream, cipher: &AeadCipher, nonce: &mut [u8; 12]) -> SrvChunk {
    let mut sealed_len = [0u8; 2 + TAG_LEN];
    if stream.read_exact(&mut sealed_len).await.is_err() {
        return SrvChunk::Eof;
    }
    let len_plain = cipher.open(nonce, &sealed_len).expect("server: decrypt chunk length");
    increment_nonce(nonce);
    let clen = u16::from_be_bytes([len_plain[0], len_plain[1]]) as usize;
    if clen == 0 {
        return SrvChunk::Zero;
    }
    let mut sealed = vec![0u8; clen + TAG_LEN];
    stream.read_exact(&mut sealed).await.expect("server: read chunk body");
    let plain = cipher.open(nonce, &sealed).expect("server: decrypt chunk body");
    increment_nonce(nonce);
    SrvChunk::Data(plain)
}

async fn srv_write_chunk(stream: &mut TcpStream, cipher: &AeadCipher, nonce: &mut [u8; 12], plaintext: &[u8]) {
    let len = (plaintext.len() as u16).to_be_bytes();
    let sealed_len = cipher.seal(nonce, &len).unwrap();
    increment_nonce(nonce);
    let sealed = cipher.seal(nonce, plaintext).unwrap();
    increment_nonce(nonce);
    stream.write_all(&sealed_len).await.unwrap();
    stream.write_all(&sealed).await.unwrap();
    stream.flush().await.unwrap();
}

/// The server's half-close: a single sealed zero-length field, no payload.
async fn srv_write_zero(stream: &mut TcpStream, cipher: &AeadCipher, nonce: &mut [u8; 12]) {
    let sealed_len = cipher.seal(nonce, &[0u8, 0u8]).unwrap();
    increment_nonce(nonce);
    stream.write_all(&sealed_len).await.unwrap();
    stream.flush().await.unwrap();
}

/// A reuse-capable fake Snell v2 server: it handshakes once, then loops
/// serving sequential logical streams on the *same* connection, echoing each
/// until the client's zero-length half-close, replying with its own zero
/// chunk so the client sees a clean logical EOF. It records each request's
/// command byte so tests can assert CommandConnectV2 was sent.
async fn serve_reuse(mut stream: TcpStream, commands: Arc<std::sync::Mutex<Vec<u8>>>) {
    let cipher = SnellCipher::Aes128Gcm; // v2
    let ks = cipher.key_size();

    let mut salt = [0u8; SALT_LEN];
    if stream.read_exact(&mut salt).await.is_err() {
        return;
    }
    let read_cipher = AeadCipher::new(cipher, &snell_kdf(REUSE_PSK, &salt, ks)).unwrap();
    let mut read_nonce = [0u8; 12];

    let mut salt_w = [0u8; SALT_LEN];
    for (i, b) in salt_w.iter_mut().enumerate() {
        *b = (i as u8).wrapping_mul(11).wrapping_add(3);
    }
    stream.write_all(&salt_w).await.unwrap();
    let write_cipher = AeadCipher::new(cipher, &snell_kdf(REUSE_PSK, &salt_w, ks)).unwrap();
    let mut write_nonce = [0u8; 12];

    loop {
        // Each logical stream starts with a request header chunk.
        let header = match srv_read_chunk(&mut stream, &read_cipher, &mut read_nonce).await {
            SrvChunk::Data(h) => h,
            SrvChunk::Zero | SrvChunk::Eof => return,
        };
        commands.lock().unwrap().push(header[1]);
        srv_write_chunk(&mut stream, &write_cipher, &mut write_nonce, &[RESP_TUNNEL]).await;

        // Echo until the client half-closes this logical stream.
        loop {
            match srv_read_chunk(&mut stream, &read_cipher, &mut read_nonce).await {
                SrvChunk::Data(d) => srv_write_chunk(&mut stream, &write_cipher, &mut write_nonce, &d).await,
                SrvChunk::Zero => {
                    srv_write_zero(&mut stream, &write_cipher, &mut write_nonce).await;
                    break;
                }
                SrvChunk::Eof => return,
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

fn reuse_config(addr: SocketAddr) -> SnellOutboundConfig {
    SnellOutboundConfig {
        server: addr.ip().to_string(),
        port: addr.port(),
        psk: REUSE_PSK.to_vec(),
        version: 2,
        obfs: None,
        reuse: false,
    }
}

fn pool_len(key: &SnellServerKey) -> usize {
    SESSION_POOL
        .lock()
        .expect("snell session pool")
        .as_ref()
        .and_then(|m| m.get(key))
        .map_or(0, |v| v.len())
}

/// Relay-style round trip then a clean half-close (our zero chunk, then read
/// the server's zero chunk to EOF), leaving the session reusable once
/// dropped — the shape `copy_bidirectional` produces in the real relay.
async fn round_trip_and_close(stream: &mut BoxedStream, payload: &[u8]) {
    stream.write_all(payload).await.unwrap();
    stream.flush().await.unwrap();
    let mut buf = vec![0u8; payload.len()];
    stream.read_exact(&mut buf).await.unwrap();
    assert_eq!(buf, payload);
    stream.shutdown().await.unwrap();
    let mut tail = Vec::new();
    stream.read_to_end(&mut tail).await.unwrap();
    assert!(tail.is_empty(), "no application bytes after the echo");
}

#[tokio::test]
async fn v2_reuses_pooled_session_sequentially() {
    let (addr, conns, commands) = spawn_reuse_server().await;
    let config = reuse_config(addr);
    let key = SnellServerKey::from_config(&config);
    let target = TargetAddr::Domain("example.com".to_string(), 443);

    // First stream over a fresh connection; a clean half-close parks it.
    {
        let mut s = connect(&config, &target).await.unwrap();
        round_trip_and_close(&mut s, b"first").await;
    }
    assert_eq!(pool_len(&key), 1, "clean half-close parks the session for reuse");

    // Second and third streams must ride the same connection.
    {
        let mut s = connect(&config, &target).await.unwrap();
        round_trip_and_close(&mut s, b"second").await;
    }
    {
        let mut s = connect(&config, &target).await.unwrap();
        round_trip_and_close(&mut s, b"third").await;
    }

    assert_eq!(pool_len(&key), 1, "the session stays parked between reuses");
    assert_eq!(
        conns.load(Ordering::SeqCst),
        1,
        "all three streams shared one TCP connection"
    );
    assert_eq!(
        *commands.lock().unwrap(),
        vec![COMMAND_CONNECT_V2, COMMAND_CONNECT_V2, COMMAND_CONNECT_V2],
        "every reuse request used CommandConnectV2",
    );
}

#[tokio::test]
async fn pool_take_evicts_idle_expired_sessions() {
    let key = SnellServerKey {
        server: "ttl-test.invalid".to_string(),
        port: 1,
        version: 2,
        psk: b"p".to_vec(),
        obfs: None,
    };
    let (dummy, _peer) = tokio::io::duplex(64);
    let expired = PooledSnell {
        inner: Box::new(dummy),
        cipher: SnellCipher::Aes128Gcm,
        psk: b"p".to_vec(),
        write_cipher: AeadCipher::new(SnellCipher::Aes128Gcm, &[0u8; 16]).unwrap(),
        write_nonce: [0u8; 12],
        read_cipher: AeadCipher::new(SnellCipher::Aes128Gcm, &[0u8; 16]).unwrap(),
        read_nonce: [0u8; 12],
        idle_since: Instant::now() - SESSION_IDLE_TTL - Duration::from_secs(1),
    };
    pool_put(key.clone(), PooledSession::Shadowaead(expired));
    // The parked session has outlived the idle TTL: it is evicted on access
    // and no session is handed back for reuse.
    assert!(pool_take(&key).is_none(), "an idle-expired session is not reused");
    assert_eq!(pool_len(&key), 0, "the expired entry is evicted");
}
