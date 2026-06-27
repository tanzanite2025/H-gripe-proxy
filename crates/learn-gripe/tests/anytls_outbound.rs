//! End-to-end proof that traffic flows through an AnyTLS outbound:
//! a SOCKS5 client -> gripe inbound -> AnyTLS outbound -> fake AnyTLS server.
//!
//! The fake server validates the AnyTLS handshake — the 32-byte `SHA256`
//! password authenticator and zero-padding, the mandatory `cmdSettings`, the
//! `cmdSYN` opening the stream, and the `cmdPSH` carrying the SOCKS5 target
//! address — replies with the v2 control frames (`cmdServerSettings`,
//! `cmdSYNACK`) plus a `cmdWaste`/`cmdHeartRequest` to exercise the client's
//! control-frame handling, then echoes the application `cmdPSH` payload. Because
//! security and transport are orthogonal layers shared with Trojan/VLESS (see
//! `crate::transport`), these tests focus on the AnyTLS session framing and its
//! composition with the `none` / `tls` security layers.

use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::sync::Arc;

use learn_gripe::{AnyTlsOutboundConfig, GripeConfig, GripeKernel, OutboundMode, Security, TlsClientConfig, Transport};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio_rustls::TlsAcceptor;

const TEST_CERT: &str = include_str!("data/vless_tls_cert.pem");
const TEST_KEY: &str = include_str!("data/vless_tls_key.pem");

/// `SHA256("password")` — the on-wire AnyTLS authenticator for the password used
/// by these tests. Hardcoded so the test crate need not depend on a hashing
/// crate; matches the `Sha256::digest` the outbound computes from `password`.
const TEST_PASSWORD_SHA256: [u8; 32] = [
    0x5e, 0x88, 0x48, 0x98, 0xda, 0x28, 0x04, 0x71, 0x51, 0xd0, 0xe5, 0x6f, 0x8d, 0xc6, 0x29, 0x27, 0x73, 0x60, 0x3d,
    0x0d, 0x6a, 0xab, 0xbd, 0xd6, 0x2a, 0x11, 0xef, 0x72, 0x1d, 0x15, 0x42, 0xd8,
];

// Session-layer commands (see anytls protocol spec).
const CMD_WASTE: u8 = 0;
const CMD_SYN: u8 = 1;
const CMD_PSH: u8 = 2;
const CMD_FIN: u8 = 3;
const CMD_SETTINGS: u8 = 4;
const CMD_SYNACK: u8 = 7;
const CMD_HEART_REQUEST: u8 = 8;
const CMD_SERVER_SETTINGS: u8 = 10;

const STREAM_ID: u32 = 1;

/// Read one AnyTLS session frame: `cmd(1) | streamId(u32 BE) | len(u16 BE) | data`.
async fn read_frame<S>(stream: &mut S) -> (u8, u32, Vec<u8>)
where
    S: AsyncRead + Unpin,
{
    let mut header = [0u8; 7];
    stream.read_exact(&mut header).await.unwrap();
    let cmd = header[0];
    let sid = u32::from_be_bytes([header[1], header[2], header[3], header[4]]);
    let len = u16::from_be_bytes([header[5], header[6]]) as usize;
    let mut data = vec![0u8; len];
    stream.read_exact(&mut data).await.unwrap();
    (cmd, sid, data)
}

/// Write one AnyTLS session frame to `stream`.
async fn write_frame<S>(stream: &mut S, cmd: u8, sid: u32, data: &[u8])
where
    S: AsyncWrite + Unpin,
{
    let mut frame = Vec::with_capacity(7 + data.len());
    frame.push(cmd);
    frame.extend_from_slice(&sid.to_be_bytes());
    frame.extend_from_slice(&(data.len() as u16).to_be_bytes());
    frame.extend_from_slice(data);
    stream.write_all(&frame).await.unwrap();
}

/// Validate an AnyTLS handshake, then echo application `cmdPSH` payload back.
async fn serve_anytls<S>(mut stream: S)
where
    S: AsyncRead + AsyncWrite + Unpin,
{
    // Authentication: SHA256(password) + padding0 length + padding0.
    let mut hash = [0u8; 32];
    stream.read_exact(&mut hash).await.unwrap();
    assert_eq!(hash, TEST_PASSWORD_SHA256, "anytls password hash");
    let mut padding_len = [0u8; 2];
    stream.read_exact(&mut padding_len).await.unwrap();
    let padding_len = u16::from_be_bytes(padding_len) as usize;
    if padding_len > 0 {
        let mut padding = vec![0u8; padding_len];
        stream.read_exact(&mut padding).await.unwrap();
    }

    // Mandatory cmdSettings first; a v2 server answers with cmdServerSettings.
    let (cmd, sid, _data) = read_frame(&mut stream).await;
    assert_eq!(cmd, CMD_SETTINGS, "first frame must be cmdSettings");
    assert_eq!(sid, 0, "cmdSettings rides stream 0");
    write_frame(&mut stream, CMD_SERVER_SETTINGS, 0, b"v=2").await;
    // A stray cmdWaste must be silently dropped by the client.
    write_frame(&mut stream, CMD_WASTE, 0, b"junk-padding").await;

    // cmdSYN opens the stream; a v2 server acknowledges with an empty cmdSYNACK.
    let (cmd, sid, _data) = read_frame(&mut stream).await;
    assert_eq!(cmd, CMD_SYN, "expected cmdSYN");
    assert_eq!(sid, STREAM_ID, "stream id");
    write_frame(&mut stream, CMD_SYNACK, STREAM_ID, &[]).await;

    // First cmdPSH carries the SOCKS5-encoded proxy target.
    let (cmd, sid, addr) = read_frame(&mut stream).await;
    assert_eq!(cmd, CMD_PSH, "expected cmdPSH with the target address");
    assert_eq!(sid, STREAM_ID, "stream id");
    assert!(!addr.is_empty(), "target address present");

    // Probe the client's keepalive handling: it must reply with cmdHeartResponse
    // (which this echo loop simply ignores as a non-PSH frame).
    write_frame(&mut stream, CMD_HEART_REQUEST, 0, &[]).await;

    // Echo application data: each cmdPSH(stream) is reflected back verbatim.
    loop {
        let (cmd, sid, data) = read_frame(&mut stream).await;
        match cmd {
            CMD_PSH if sid == STREAM_ID => {
                write_frame(&mut stream, CMD_PSH, STREAM_ID, &data).await;
            }
            CMD_FIN => return,
            _ => {}
        }
    }
}

async fn spawn_fake_anytls_server() -> SocketAddr {
    let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        while let Ok((stream, _)) = listener.accept().await {
            tokio::spawn(serve_anytls(stream));
        }
    });
    addr
}

async fn spawn_fake_anytls_tls_server() -> SocketAddr {
    let acceptor = tls_acceptor();
    let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        while let Ok((tcp, _)) = listener.accept().await {
            let acceptor = acceptor.clone();
            tokio::spawn(async move {
                if let Ok(tls) = acceptor.accept(tcp).await {
                    serve_anytls(tls).await;
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
    let provider = Arc::new(rustls::crypto::ring::default_provider());
    let config = rustls::ServerConfig::builder_with_provider(provider)
        .with_safe_default_protocol_versions()
        .unwrap()
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

/// Drive a SOCKS5 round trip through the kernel built from `outbound` and assert
/// the payload is echoed back unchanged.
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

#[tokio::test]
async fn relays_through_plaintext_anytls_outbound() {
    let server = spawn_fake_anytls_server().await;
    assert_relays(
        OutboundMode::AnyTls(Box::new(AnyTlsOutboundConfig {
            server: server.ip().to_string(),
            port: server.port(),
            password_sha256: TEST_PASSWORD_SHA256,
            security: Security::None,
            transport: Transport::Tcp,
        })),
        b"hello anytls over plaintext",
    )
    .await;
}

#[tokio::test]
async fn relays_through_tls_anytls_outbound() {
    let server = spawn_fake_anytls_tls_server().await;
    assert_relays(
        OutboundMode::AnyTls(Box::new(AnyTlsOutboundConfig {
            server: server.ip().to_string(),
            port: server.port(),
            password_sha256: TEST_PASSWORD_SHA256,
            security: Security::Tls(TlsClientConfig {
                server_name: Some("localhost".to_string()),
                alpn: Vec::new(),
                skip_cert_verify: true,
                client_fingerprint: None,
                ech: None,
            }),
            transport: Transport::Tcp,
        })),
        b"hello anytls over tls",
    )
    .await;
}

#[tokio::test]
async fn relays_larger_payload_spanning_multiple_frames() {
    let server = spawn_fake_anytls_server().await;
    // Larger than one MAX_PSH_CHUNK (8 KiB) so the relay must split it across
    // several cmdPSH frames and reassemble the echo.
    let payload: Vec<u8> = (0..20_000u32).map(|i| (i % 251) as u8).collect();
    assert_relays(
        OutboundMode::AnyTls(Box::new(AnyTlsOutboundConfig {
            server: server.ip().to_string(),
            port: server.port(),
            password_sha256: TEST_PASSWORD_SHA256,
            security: Security::None,
            transport: Transport::Tcp,
        })),
        &payload,
    )
    .await;
}
