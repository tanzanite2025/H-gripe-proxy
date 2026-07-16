use std::collections::{HashSet, VecDeque};
use std::net::{Ipv4Addr, SocketAddr};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use sha2::{Digest, Sha256};

use super::frame::*;
use super::padding::*;
use super::session::*;
use super::*;
use crate::config::outbound_opts::ProxyEntry;
use crate::transport::tls::ClientFingerprint;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

fn parse_entry(yaml: &str) -> ProxyEntry {
    serde_yaml_ng::from_str(yaml).expect("parse proxy entry")
}

#[test]
fn padding_md5_matches_upstream_default_scheme() {
    // Cross-checked against `md5sum` of anytls-go's default padding scheme.
    assert_eq!(DEFAULT_PADDING_MD5, "75cff2ad89aadf5e257059ee571ebe11");
    assert_eq!(PaddingScheme::default_scheme().md5_hex, DEFAULT_PADDING_MD5);
}

#[test]
fn default_scheme_parses_stop_and_packet_tokens() {
    let scheme = PaddingScheme::default_scheme();
    assert_eq!(scheme.stop, 8);
    // Packet 0 is the fixed 30-byte auth padding0.
    assert_eq!(scheme.record_payload_sizes(0), vec![30]);
    // Packet 1 is a single range in [100, 400).
    let one = scheme.record_payload_sizes(1);
    assert_eq!(one.len(), 1);
    assert!((100..400).contains(&one[0]), "{one:?}");
    // Packet 2: 5 ranges interleaved with 4 check marks.
    let two = scheme.record_payload_sizes(2);
    assert_eq!(two.len(), 9);
    assert_eq!(two.iter().filter(|&&s| s == CHECK_MARK).count(), 4, "{two:?}");
    assert!((400..500).contains(&two[0]), "{two:?}");
    // An undefined packet shapes nothing.
    assert!(scheme.record_payload_sizes(99).is_empty());
}

#[test]
fn auth_header_carries_password_and_padding0() {
    let password_sha256: [u8; 32] = Sha256::digest(b"secret").into();
    let scheme = PaddingScheme::default_scheme();
    let auth = build_auth_header(&password_sha256, &scheme);
    // SHA256(password) | padding0_len(=30) | 30 zero bytes.
    assert_eq!(&auth[..32], &password_sha256);
    assert_eq!(&auth[32..34], &30u16.to_be_bytes());
    assert_eq!(auth.len(), 34 + 30);
    assert!(auth[34..].iter().all(|&b| b == 0), "padding0 must be zero");
}

#[test]
fn session_init_carries_settings_syn_and_target() {
    let target = TargetAddr::Domain("example.com".to_string(), 443);
    let scheme = PaddingScheme::default_scheme();
    let init = build_session_init(&scheme, STREAM_ID, &target);

    // cmdSettings frame (sid 0) with v / client / padding-md5.
    let mut pos = 0;
    assert_eq!(init[pos], CMD_SETTINGS);
    assert_eq!(&init[pos + 1..pos + 5], &0u32.to_be_bytes());
    let settings_len = u16::from_be_bytes([init[pos + 5], init[pos + 6]]) as usize;
    let settings = &init[pos + FRAME_HEADER_LEN..pos + FRAME_HEADER_LEN + settings_len];
    let settings = std::str::from_utf8(settings).unwrap();
    assert!(settings.contains("v=2"), "{settings}");
    assert!(
        settings.contains("padding-md5=75cff2ad89aadf5e257059ee571ebe11"),
        "{settings}"
    );
    pos += FRAME_HEADER_LEN + settings_len;

    // cmdSYN frame for the stream, no data.
    assert_eq!(init[pos], CMD_SYN);
    assert_eq!(&init[pos + 1..pos + 5], &STREAM_ID.to_be_bytes());
    assert_eq!(&init[pos + 5..pos + 7], &0u16.to_be_bytes());
    pos += FRAME_HEADER_LEN;

    // cmdPSH frame carrying the SOCKS5-encoded target.
    assert_eq!(init[pos], CMD_PSH);
    assert_eq!(&init[pos + 1..pos + 5], &STREAM_ID.to_be_bytes());
    let addr_len = u16::from_be_bytes([init[pos + 5], init[pos + 6]]) as usize;
    let mut expected = Vec::new();
    socks5::encode_address(&mut expected, &target);
    assert_eq!(
        &init[pos + FRAME_HEADER_LEN..pos + FRAME_HEADER_LEN + addr_len],
        &expected[..]
    );
}

/// Concatenate the shaper's output records back into the byte stream the peer
/// receives.
fn drain(out: &VecDeque<Vec<u8>>) -> Vec<u8> {
    out.iter().flatten().copied().collect()
}

/// Walk a frame stream, returning the `(cmd, payload-len)` of each frame.
/// Panics on a truncated trailer, proving the stream is exactly frame-aligned.
fn frames(bytes: &[u8]) -> Vec<(u8, usize)> {
    let mut out = Vec::new();
    let mut pos = 0;
    while pos < bytes.len() {
        assert!(pos + FRAME_HEADER_LEN <= bytes.len(), "truncated frame header");
        let cmd = bytes[pos];
        let len = u16::from_be_bytes([bytes[pos + 5], bytes[pos + 6]]) as usize;
        assert!(pos + FRAME_HEADER_LEN + len <= bytes.len(), "truncated frame body");
        out.push((cmd, len));
        pos += FRAME_HEADER_LEN + len;
    }
    out
}

/// Parse a scheme with deterministic fixed-size records (min == max).
fn fixed_scheme(raw: &str) -> PaddingScheme {
    PaddingScheme::parse(raw.as_bytes()).expect("scheme parses")
}

#[test]
fn shaper_pads_short_payload_up_to_record_size() {
    // Packet 1 is one fixed 100-byte record, so a small frame is padded with
    // a trailing cmdWaste to reach exactly 100 bytes.
    let mut shaper = PaddingShaper::new(fixed_scheme("stop=8\n1=100-100"));
    let mut out = VecDeque::new();
    let mut frame = Vec::new();
    push_frame(&mut frame, CMD_PSH, STREAM_ID, b"hi"); // 7 + 2 = 9 bytes
    shaper.shape(&mut out, frame);

    let stream = drain(&out);
    assert_eq!(stream.len(), 100, "record padded to scheme size");
    // Real PSH(2) then a cmdWaste filling the rest: 100 - 9 - 7 = 84 bytes.
    assert_eq!(frames(&stream), vec![(CMD_PSH, 2), (CMD_WASTE, 84)]);
}

#[test]
fn shaper_emits_standalone_waste_when_payload_exhausted() {
    // Two ranges but a tiny payload: the first record carries the payload +
    // padding, the second is pure cmdWaste (no check mark stops it).
    let mut shaper = PaddingShaper::new(fixed_scheme("stop=8\n1=50-50,60-60"));
    let mut out = VecDeque::new();
    let mut frame = Vec::new();
    push_frame(&mut frame, CMD_PSH, STREAM_ID, b"x"); // 8 bytes
    shaper.shape(&mut out, frame);

    assert_eq!(out.len(), 2, "two records");
    // The payload+padding record is exactly the scheme size (50). The pure
    // cmdWaste record is `header + size` (= 7 + 60), matching upstream's
    // `make([]byte, headerOverHeadSize+l)` for the all-padding branch.
    assert_eq!(out[0].len(), 50);
    assert_eq!(out[1].len(), FRAME_HEADER_LEN + 60);
    assert_eq!(frames(&out[0]), vec![(CMD_PSH, 1), (CMD_WASTE, 50 - 8 - 7)]);
    assert_eq!(frames(&out[1]), vec![(CMD_WASTE, 60)]);
}

#[test]
fn shaper_check_mark_stops_padding_when_drained() {
    // After the first range consumes the payload, the `c` check mark stops
    // further padding records for this packet.
    let mut shaper = PaddingShaper::new(fixed_scheme("stop=8\n1=50-50,c,500-500"));
    let mut out = VecDeque::new();
    let mut frame = Vec::new();
    push_frame(&mut frame, CMD_PSH, STREAM_ID, b"x");
    shaper.shape(&mut out, frame);

    assert_eq!(out.len(), 1, "check mark halted padding");
    assert_eq!(out[0].len(), 50);
}

#[test]
fn shaper_splits_large_payload_and_keeps_bytes_intact() {
    // A payload larger than the single record size is split: one record of
    // the scheme size, then the remainder. No bytes are added or lost.
    let mut shaper = PaddingShaper::new(fixed_scheme("stop=8\n1=40-40"));
    let mut out = VecDeque::new();
    let mut frame = Vec::new();
    let payload: Vec<u8> = (0..200u32).map(|i| i as u8).collect();
    push_frame(&mut frame, CMD_PSH, STREAM_ID, &payload);
    let expected = frame.clone();
    shaper.shape(&mut out, frame);

    assert_eq!(out[0].len(), 40, "first record is the scheme size");
    assert!(out.len() >= 2, "remainder spilled into more records");
    assert_eq!(drain(&out), expected, "payload bytes unchanged, just rechunked");
}

#[test]
fn shaper_stops_after_scheme_stop_packet() {
    let mut shaper = PaddingShaper::new(fixed_scheme("stop=2\n1=100-100"));
    let mut out = VecDeque::new();
    // Packet 1 is shaped (padded to 100).
    let mut f1 = Vec::new();
    push_frame(&mut f1, CMD_PSH, STREAM_ID, b"a");
    shaper.shape(&mut out, f1);
    assert_eq!(out[0].len(), 100);
    out.clear();
    // Packet 2 reaches `stop`: passed through verbatim, padding disabled.
    let mut f2 = Vec::new();
    push_frame(&mut f2, CMD_PSH, STREAM_ID, b"b");
    let raw = f2.clone();
    shaper.shape(&mut out, f2);
    assert!(!shaper.send_padding);
    assert_eq!(out.len(), 1);
    assert_eq!(out[0], raw);
}

#[test]
fn defaults_to_tls_security() {
    let yaml = "name: a\ntype: anytls\nserver: example.com\nport: 443\npassword: secret\n";
    let cfg = AnyTlsOutboundConfig::from_proxy(&parse_entry(yaml)).unwrap();
    assert!(matches!(cfg.security, Security::Tls(_)));
    assert!(matches!(cfg.transport, Transport::Tcp));
    assert_eq!(cfg.password_sha256, <[u8; 32]>::from(Sha256::digest(b"secret")));
}

#[test]
fn missing_password_is_rejected() {
    let yaml = "name: a\ntype: anytls\nserver: example.com\nport: 443\n";
    let err = AnyTlsOutboundConfig::from_proxy(&parse_entry(yaml)).unwrap_err();
    assert!(err.to_string().contains("password"), "got: {err}");
}

#[test]
fn missing_server_is_rejected() {
    let yaml = "name: a\ntype: anytls\nport: 443\npassword: secret\n";
    let err = AnyTlsOutboundConfig::from_proxy(&parse_entry(yaml)).unwrap_err();
    assert!(err.to_string().contains("server"), "got: {err}");
}

#[test]
fn sni_and_skip_cert_verify_flow_into_tls() {
    let yaml = "name: a\ntype: anytls\nserver: example.com\nport: 443\npassword: secret\n\
         sni: real.example\nskip-cert-verify: true\nclient-fingerprint: chrome\n";
    let cfg = AnyTlsOutboundConfig::from_proxy(&parse_entry(yaml)).unwrap();
    match cfg.security {
        Security::Tls(tls) => {
            assert_eq!(tls.server_name.as_deref(), Some("real.example"));
            assert!(tls.skip_cert_verify);
            assert_eq!(tls.client_fingerprint, Some(ClientFingerprint::Chrome));
        }
        other => panic!("expected TLS security, got {other:?}"),
    }
}

#[test]
fn scheme_update_is_stored_per_server_and_applied_to_new_connections() {
    let key = ServerKey {
        server: "scheme-update-apply.invalid".to_string(),
        port: 443,
    };
    // An unknown server falls back to the built-in default scheme.
    assert_eq!(current_scheme(&key).md5_hex, DEFAULT_PADDING_MD5);

    // A pushed scheme with a different md5 is adopted for future connections.
    let pushed = b"stop=4\n0=20-20\n1=120-120";
    apply_scheme_update(&key, pushed);
    let now = current_scheme(&key);
    assert_eq!(now.md5_hex, md5_hex(pushed));
    assert_eq!(now.stop, 4);
    assert_eq!(now.record_payload_sizes(0), vec![20]);

    // Storage is per server: another endpoint is unaffected.
    let other = ServerKey {
        server: "scheme-update-other.invalid".to_string(),
        port: 443,
    };
    assert_eq!(current_scheme(&other).md5_hex, DEFAULT_PADDING_MD5);
}

#[test]
fn scheme_update_ignores_unchanged_and_invalid_schemes() {
    let key = ServerKey {
        server: "scheme-update-noop.invalid".to_string(),
        port: 1,
    };
    // Re-pushing the default scheme (same md5) is a no-op: still default.
    apply_scheme_update(&key, DEFAULT_PADDING_SCHEME.as_bytes());
    assert_eq!(current_scheme(&key).md5_hex, DEFAULT_PADDING_MD5);

    // A scheme without a `stop` line fails to parse and is ignored.
    apply_scheme_update(&key, b"0=10-10");
    assert_eq!(current_scheme(&key).md5_hex, DEFAULT_PADDING_MD5);
}

#[test]
fn scheme_parse_trims_crlf_and_spaces_like_upstream() {
    // A scheme delimited with `\r\n` (each line keeps a trailing `\r`) and
    // written with spaces around `=` must parse identically to its trimmed
    // form, matching anytls `util.StringMapFromBytes`.
    let raw = b"stop = 3\r\n0 = 20-20\r\n1 = 120-120\r\n2 = 7-7,c,9-9";
    let scheme = PaddingScheme::parse(raw).expect("crlf/spaced scheme parses");
    assert_eq!(scheme.stop, 3);
    assert_eq!(scheme.record_payload_sizes(0), vec![20]);
    assert_eq!(scheme.record_payload_sizes(1), vec![120]);
    // The `c` check mark and both ranges survive (no token dropped to `\r`).
    assert_eq!(scheme.record_payload_sizes(2), vec![7, CHECK_MARK, 9]);
    // md5 is still over the raw bytes (what we advertise), not the trimmed
    // form, so it differs from the equivalent `\n`-delimited scheme.
    assert_eq!(scheme.md5_hex, md5_hex(raw));
}

// ---- Session-pool reuse (PR B) ----------------------------------------

/// Read one session frame from a server-side socket, or `None` at EOF.
async fn read_frame_opt(stream: &mut TcpStream) -> Option<(u8, u32, Vec<u8>)> {
    let mut header = [0u8; FRAME_HEADER_LEN];
    stream.read_exact(&mut header).await.ok()?;
    let cmd = header[0];
    let sid = u32::from_be_bytes([header[1], header[2], header[3], header[4]]);
    let len = u16::from_be_bytes([header[5], header[6]]) as usize;
    let mut data = vec![0u8; len];
    stream.read_exact(&mut data).await.ok()?;
    Some((cmd, sid, data))
}

/// Write one session frame to a server-side socket.
async fn server_write(stream: &mut TcpStream, cmd: u8, sid: u32, data: &[u8]) {
    let mut frame = Vec::new();
    push_frame(&mut frame, cmd, sid, data);
    stream.write_all(&frame).await.unwrap();
}

/// A minimal anytls server multiplexing several streams on one connection: it
/// records each `cmdSYN`'s stream id, acks it, treats each stream's first
/// `cmdPSH` as the (unechoed) target address and echoes the rest back on the
/// same id, and answers `cmdFIN` with `cmdFIN` (closing only that stream). If
/// `alert_after_first_stream` is set it sends a `cmdAlert` on the first
/// stream's FIN, marking the session broken so it must not be reused.
async fn pool_test_serve(mut stream: TcpStream, sids: Arc<Mutex<Vec<u32>>>, alert_after_first_stream: bool) {
    // Note: callers set `alert_after_first_stream` only for the connection
    // expected to be pooled, so the replacement connection stays healthy.
    let mut hash = [0u8; 32];
    if stream.read_exact(&mut hash).await.is_err() {
        return;
    }
    let mut padding_len = [0u8; 2];
    stream.read_exact(&mut padding_len).await.unwrap();
    let padding_len = u16::from_be_bytes(padding_len) as usize;
    if padding_len > 0 {
        let mut padding = vec![0u8; padding_len];
        stream.read_exact(&mut padding).await.unwrap();
    }

    // Per-stream: whether the next `cmdPSH` is the (unechoed) target address.
    let mut awaiting_addr: HashSet<u32> = HashSet::new();
    let mut streams_done = 0u32;
    while let Some((cmd, sid, data)) = read_frame_opt(&mut stream).await {
        match cmd {
            CMD_WASTE => {}
            CMD_SETTINGS => server_write(&mut stream, CMD_SERVER_SETTINGS, 0, b"v=2").await,
            CMD_SYN => {
                sids.lock().unwrap().push(sid);
                server_write(&mut stream, CMD_SYNACK, sid, &[]).await;
                awaiting_addr.insert(sid);
            }
            // A stream's first `cmdPSH` is its (unechoed) target address; the
            // rest are echoed back on the same id.
            CMD_PSH if !awaiting_addr.remove(&sid) => {
                server_write(&mut stream, CMD_PSH, sid, &data).await;
            }
            CMD_FIN => {
                streams_done += 1;
                if alert_after_first_stream && streams_done == 1 {
                    // Mark the session broken (before its FIN) so the client's
                    // driver tears it down deterministically and never reuses
                    // it, then drop the connection.
                    server_write(&mut stream, CMD_ALERT, sid, b"reaped").await;
                    server_write(&mut stream, CMD_FIN, sid, &[]).await;
                    return;
                }
                server_write(&mut stream, CMD_FIN, sid, &[]).await;
            }
            _ => {}
        }
    }
}

/// Spawn `pool_test_serve` accepting on a fresh port; returns the address, the
/// recorded stream ids, and the count of accepted TCP connections.
async fn spawn_pool_server(alert_first_connection: bool) -> (SocketAddr, Arc<Mutex<Vec<u32>>>, Arc<AtomicUsize>) {
    let sids = Arc::new(Mutex::new(Vec::<u32>::new()));
    let conns = Arc::new(AtomicUsize::new(0));
    let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).await.unwrap();
    let addr = listener.local_addr().unwrap();
    let (sids_task, conns_task) = (sids.clone(), conns.clone());
    tokio::spawn(async move {
        while let Ok((stream, _)) = listener.accept().await {
            let index = conns_task.fetch_add(1, Ordering::SeqCst);
            // Only the first connection (the one that gets pooled) is reaped.
            let alert = alert_first_connection && index == 0;
            tokio::spawn(pool_test_serve(stream, sids_task.clone(), alert));
        }
    });
    (addr, sids, conns)
}

fn pool_test_config(addr: SocketAddr) -> AnyTlsOutboundConfig {
    AnyTlsOutboundConfig {
        server: addr.ip().to_string(),
        port: addr.port(),
        password_sha256: Sha256::digest(b"password").into(),
        security: Security::None,
        transport: Transport::Tcp,
    }
}

/// Number of live (reusable) registered sessions for `key`.
fn pool_len(key: &ServerKey) -> usize {
    SESSION_REGISTRY
        .lock()
        .unwrap()
        .as_ref()
        .and_then(|map| map.get(key))
        .map_or(0, |list| list.iter().filter(|h| h.alive()).count())
}

/// Drive a relay-style round trip on `stream`, then close it cleanly (send
/// our `cmdFIN`, read the server's `cmdFIN` to EOF) so the session is left
/// idle (and reusable) once the stream is dropped.
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
async fn pool_reuses_session_and_increments_stream_id() {
    let (addr, sids, conns) = spawn_pool_server(false).await;
    let config = pool_test_config(addr);
    let target = TargetAddr::Domain("example.com".to_string(), 443);
    let key = ServerKey {
        server: config.server.clone(),
        port: config.port,
    };

    // Stream 1 over a fresh session; a clean close leaves it idle in the pool.
    {
        let mut s1 = connect(&config, &target).await.unwrap();
        round_trip_and_close(&mut s1, b"first").await;
    }
    assert_eq!(pool_len(&key), 1, "clean close leaves the session reusable");

    // Stream 2 must reuse it: no new TCP connection, next stream id.
    {
        let mut s2 = connect(&config, &target).await.unwrap();
        round_trip_and_close(&mut s2, b"second").await;
    }

    assert_eq!(
        conns.load(Ordering::SeqCst),
        1,
        "second stream reused one TCP connection"
    );
    assert_eq!(
        *sids.lock().unwrap(),
        vec![1, 2],
        "sequential stream ids on the reused connection"
    );
}

#[tokio::test]
async fn multiplexes_concurrent_streams_on_one_connection() {
    let (addr, sids, conns) = spawn_pool_server(false).await;
    let config = pool_test_config(addr);
    let t1 = TargetAddr::Domain("one.example".to_string(), 443);
    let t2 = TargetAddr::Domain("two.example".to_string(), 443);

    // Two overlapping streams: the second opens on the first's live session.
    let mut s1 = connect(&config, &t1).await.unwrap();
    let mut s2 = connect(&config, &t2).await.unwrap();

    // Interleave writes; demux must route each echo back to its own stream.
    s1.write_all(b"aaa").await.unwrap();
    s1.flush().await.unwrap();
    s2.write_all(b"bbb").await.unwrap();
    s2.flush().await.unwrap();
    let (mut r1, mut r2) = ([0u8; 3], [0u8; 3]);
    s1.read_exact(&mut r1).await.unwrap();
    s2.read_exact(&mut r2).await.unwrap();
    assert_eq!(&r1, b"aaa", "stream 1 received its own payload");
    assert_eq!(&r2, b"bbb", "stream 2 received its own payload");

    assert_eq!(
        conns.load(Ordering::SeqCst),
        1,
        "both concurrent streams shared one TCP connection"
    );
    let mut ids = sids.lock().unwrap().clone();
    ids.sort_unstable();
    assert_eq!(ids, vec![1, 2], "concurrent streams got distinct incrementing ids");

    round_trip_and_close(&mut s1, b"aaa").await;
    round_trip_and_close(&mut s2, b"bbb").await;
}

#[tokio::test]
async fn pool_discards_dead_session_on_reuse() {
    // The server sends a `cmdAlert` on the first stream's FIN, so the pooled
    // session is marked broken and must not be reused.
    let (addr, sids, conns) = spawn_pool_server(true).await;
    let config = pool_test_config(addr);
    let target = TargetAddr::Domain("example.com".to_string(), 443);
    let key = ServerKey {
        server: config.server.clone(),
        port: config.port,
    };

    {
        let mut s1 = connect(&config, &target).await.unwrap();
        round_trip_and_close(&mut s1, b"first").await;
    }
    // The `cmdAlert` tore the session down, so it is not reusable.
    assert_eq!(pool_len(&key), 0, "broken session is not pooled for reuse");

    // Reuse must skip the dead session and dial a new connection.
    {
        let mut s2 = connect(&config, &target).await.unwrap();
        round_trip_and_close(&mut s2, b"second").await;
    }

    assert_eq!(
        conns.load(Ordering::SeqCst),
        2,
        "dead session discarded; a new connection dialled"
    );
    assert_eq!(
        *sids.lock().unwrap(),
        vec![1, 1],
        "the replacement session starts a fresh stream id"
    );
}
