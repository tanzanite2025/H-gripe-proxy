//! End-to-end proof that traffic flows through a Trojan outbound:
//! a SOCKS5 client -> gripe inbound -> Trojan outbound -> fake Trojan server.
//!
//! The fake server validates the Trojan request (the 56-byte hex SHA224 password
//! identifier, the CRLF delimiters, the CONNECT command and the SOCKS5 target
//! address) and then echoes the application payload. Because security and
//! transport are orthogonal layers shared with VLESS (see `crate::transport`),
//! these tests focus on the Trojan framing and its composition with the
//! `none` / `tls` / `reality` security layers; transport composition (ws/grpc/
//! h2/xhttp) rides the same `transport::establish` path proven by the VLESS
//! relay tests.

use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::sync::Arc;

use learn_gripe::{
    ClientFingerprint, GripeConfig, GripeKernel, OutboundMode, RealityClientConfig, Security, TlsClientConfig,
    Transport, TrojanOutboundConfig,
};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio_rustls::TlsAcceptor;

const TEST_CERT: &str = include_str!("data/vless_tls_cert.pem");
const TEST_KEY: &str = include_str!("data/vless_tls_key.pem");

/// `SHA224("password")` in lowercase hex — the on-wire Trojan identifier for the
/// password used by these tests. Hardcoded so the test crate need not depend on
/// a hashing crate; matches the `hash_password` unit-test vector.
const TEST_PASSWORD_HASH: &[u8; 56] = b"d63dc919e201d7bc4c825630d2cf25fdc93d4b2f0d46706d29038d01";

const CRLF: [u8; 2] = [0x0d, 0x0a];

/// A fixed 32-byte x25519 public key. The fake server is plain TLS and ignores
/// the encrypted `session_id`, so any 32-byte key drives a complete handshake;
/// this only needs to be a valid REALITY public-key length.
const TEST_REALITY_PUBLIC_KEY: [u8; 32] = [
    0x9c, 0x6f, 0x1a, 0x2b, 0x3c, 0x4d, 0x5e, 0x6f, 0x70, 0x81, 0x92, 0xa3, 0xb4, 0xc5, 0xd6, 0xe7, 0xf8, 0x09, 0x1a,
    0x2b, 0x3c, 0x4d, 0x5e, 0x6f, 0x70, 0x81, 0x92, 0xa3, 0xb4, 0xc5, 0xd6, 0x12,
];

/// Read and validate a Trojan request, then echo application bytes back.
async fn serve_trojan<S>(mut stream: S)
where
    S: AsyncRead + AsyncWrite + Unpin,
{
    let mut hash = [0u8; 56];
    stream.read_exact(&mut hash).await.unwrap();
    assert_eq!(&hash, TEST_PASSWORD_HASH, "trojan password hash");

    let mut delim = [0u8; 2];
    stream.read_exact(&mut delim).await.unwrap();
    assert_eq!(delim, CRLF, "trojan header CRLF");

    let mut command = [0u8; 1];
    stream.read_exact(&mut command).await.unwrap();
    assert_eq!(command[0], 0x01, "trojan command should be CONNECT");

    let mut atyp = [0u8; 1];
    stream.read_exact(&mut atyp).await.unwrap();
    match atyp[0] {
        0x01 => {
            let mut addr = [0u8; 4];
            stream.read_exact(&mut addr).await.unwrap();
        }
        0x04 => {
            let mut addr = [0u8; 16];
            stream.read_exact(&mut addr).await.unwrap();
        }
        0x03 => {
            let mut len = [0u8; 1];
            stream.read_exact(&mut len).await.unwrap();
            let mut host = vec![0u8; len[0] as usize];
            stream.read_exact(&mut host).await.unwrap();
        }
        other => panic!("unexpected atyp {other}"),
    }

    let mut port = [0u8; 2];
    stream.read_exact(&mut port).await.unwrap();

    let mut trailing = [0u8; 2];
    stream.read_exact(&mut trailing).await.unwrap();
    assert_eq!(trailing, CRLF, "trojan request CRLF");

    let mut buf = [0u8; 1024];
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
}

async fn spawn_fake_trojan_server() -> SocketAddr {
    let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        while let Ok((stream, _)) = listener.accept().await {
            tokio::spawn(serve_trojan(stream));
        }
    });
    addr
}

async fn spawn_fake_trojan_tls_server() -> SocketAddr {
    let acceptor = tls_acceptor(false);
    let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        while let Ok((tcp, _)) = listener.accept().await {
            let acceptor = acceptor.clone();
            tokio::spawn(async move {
                if let Ok(tls) = acceptor.accept(tcp).await {
                    serve_trojan(tls).await;
                }
            });
        }
    });
    addr
}

/// A fake Trojan server behind a plain, TLS-1.3-only rustls listener. A REALITY
/// client completes a normal TLS 1.3 handshake here and its REALITY cert
/// verifier falls back to the (skip-verify) inner verifier, proving the REALITY
/// ClientHello is well-formed and that Trojan bytes flow end-to-end over it.
/// TLS 1.3 is forced because REALITY's key exchange is defined only for 1.3.
async fn spawn_fake_trojan_reality_server() -> SocketAddr {
    let acceptor = tls_acceptor(true);
    let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        while let Ok((tcp, _)) = listener.accept().await {
            let acceptor = acceptor.clone();
            tokio::spawn(async move {
                if let Ok(tls) = acceptor.accept(tcp).await {
                    serve_trojan(tls).await;
                }
            });
        }
    });
    addr
}

/// Build a rustls server acceptor from the test cert. `tls13_only` forces TLS
/// 1.3 (required by the REALITY fixture).
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
async fn relays_through_plaintext_trojan_outbound() {
    let server = spawn_fake_trojan_server().await;
    assert_relays(
        OutboundMode::Trojan(Box::new(TrojanOutboundConfig {
            server: server.ip().to_string(),
            port: server.port(),
            password_hash: *TEST_PASSWORD_HASH,
            security: Security::None,
            transport: Transport::Tcp,
        })),
        b"hello trojan",
    )
    .await;
}

#[tokio::test]
async fn relays_through_tls_trojan_outbound() {
    let server = spawn_fake_trojan_tls_server().await;
    assert_relays(
        OutboundMode::Trojan(Box::new(TrojanOutboundConfig {
            server: server.ip().to_string(),
            port: server.port(),
            password_hash: *TEST_PASSWORD_HASH,
            security: Security::Tls(TlsClientConfig {
                server_name: Some("localhost".to_string()),
                alpn: Vec::new(),
                skip_cert_verify: true,
            }),
            transport: Transport::Tcp,
        })),
        b"hello tls trojan",
    )
    .await;
}

#[tokio::test]
async fn relays_through_reality_trojan_outbound() {
    let server = spawn_fake_trojan_reality_server().await;
    assert_relays(
        OutboundMode::Trojan(Box::new(TrojanOutboundConfig {
            server: server.ip().to_string(),
            port: server.port(),
            password_hash: *TEST_PASSWORD_HASH,
            security: Security::Reality(RealityClientConfig {
                server_name: "localhost".to_string(),
                public_key: TEST_REALITY_PUBLIC_KEY,
                short_id: vec![0x01, 0x23, 0x45, 0x67],
                alpn: Vec::new(),
                skip_cert_verify: true,
                client_fingerprint: Some(ClientFingerprint::Chrome),
            }),
            transport: Transport::Tcp,
        })),
        b"hello reality trojan",
    )
    .await;
}
