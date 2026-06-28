//! End-to-end proof that traffic flows through a GOST relay outbound:
//! a SOCKS5 client -> gripe inbound -> gost-relay outbound -> fake relay server
//! -> echo server.
//!
//! The fake server speaks the genuine go-gost relay v1 wire format: it reads the
//! `CONNECT` request, parses the feature list (user-auth, target address,
//! network), optionally validates credentials, answers with a status byte, and
//! then bridges to the address the client actually encoded (a real echo server).
//! Dialing the *decoded* target proves the address feature is serialised
//! correctly rather than the relay echoing to itself. Plaintext, TLS, username/
//! password auth, and an unauthorized rejection are all covered.

use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::sync::Arc;

use learn_gripe::{GostRelayOutboundConfig, GripeConfig, GripeKernel, OutboundMode, Security, TlsClientConfig};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt, copy_bidirectional};
use tokio::net::{TcpListener, TcpStream};
use tokio_rustls::TlsAcceptor;

const TEST_CERT: &str = include_str!("data/vless_tls_cert.pem");
const TEST_KEY: &str = include_str!("data/vless_tls_key.pem");

const RELAY_VERSION1: u8 = 0x01;
const RELAY_CMD_CONNECT: u8 = 0x01;
const RELAY_STATUS_OK: u8 = 0x00;
const RELAY_STATUS_UNAUTHORIZED: u8 = 0x02;
const FEATURE_USER_AUTH: u8 = 0x01;
const FEATURE_ADDR: u8 = 0x02;
const FEATURE_NETWORK: u8 = 0x04;

const RELAY_USER: &str = "relay-user";
const RELAY_PASS: &str = "relay-pass";

async fn spawn_echo_server() -> SocketAddr {
    let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        while let Ok((mut stream, _)) = listener.accept().await {
            tokio::spawn(async move {
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
            });
        }
    });
    addr
}

/// Decode the target address out of a relay `Addr` feature payload.
fn decode_addr(data: &[u8]) -> String {
    match data[0] {
        0x01 => {
            let ip = Ipv4Addr::new(data[1], data[2], data[3], data[4]);
            let port = u16::from_be_bytes([data[5], data[6]]);
            format!("{ip}:{port}")
        }
        0x03 => {
            let dlen = data[1] as usize;
            let host = std::str::from_utf8(&data[2..2 + dlen]).unwrap();
            let port = u16::from_be_bytes([data[2 + dlen], data[3 + dlen]]);
            format!("{host}:{port}")
        }
        other => panic!("unexpected relay atyp {other}"),
    }
}

/// Read a relay `CONNECT` request, validate credentials when required, answer
/// with a status, and bridge to the decoded target on success.
async fn serve_relay<S>(mut stream: S, require_auth: bool)
where
    S: AsyncRead + AsyncWrite + Unpin + Send,
{
    let mut header = [0u8; 4];
    if stream.read_exact(&mut header).await.is_err() {
        return;
    }
    assert_eq!(header[0], RELAY_VERSION1, "relay request version");
    assert_eq!(header[1], RELAY_CMD_CONNECT, "relay command should be CONNECT");
    let fealen = u16::from_be_bytes([header[2], header[3]]) as usize;
    let mut features = vec![0u8; fealen];
    stream.read_exact(&mut features).await.unwrap();

    let mut target: Option<String> = None;
    let mut auth: Option<(String, String)> = None;
    let mut saw_network = false;

    let mut i = 0;
    while i + 3 <= features.len() {
        let ftype = features[i];
        let flen = u16::from_be_bytes([features[i + 1], features[i + 2]]) as usize;
        let data = &features[i + 3..i + 3 + flen];
        i += 3 + flen;
        match ftype {
            FEATURE_USER_AUTH => {
                let ulen = data[0] as usize;
                let user = String::from_utf8(data[1..1 + ulen].to_vec()).unwrap();
                let plen = data[1 + ulen] as usize;
                let pass = String::from_utf8(data[2 + ulen..2 + ulen + plen].to_vec()).unwrap();
                auth = Some((user, pass));
            }
            FEATURE_ADDR => target = Some(decode_addr(data)),
            FEATURE_NETWORK => {
                assert_eq!(data, &[0x00, 0x00], "relay network should be TCP");
                saw_network = true;
            }
            _ => {}
        }
    }
    assert!(saw_network, "relay request must carry a network feature");

    if require_auth {
        let ok = matches!(&auth, Some((u, p)) if u == RELAY_USER && p == RELAY_PASS);
        if !ok {
            let _ = stream
                .write_all(&[RELAY_VERSION1, RELAY_STATUS_UNAUTHORIZED, 0x00, 0x00])
                .await;
            return;
        }
    }

    let target = target.expect("relay request must carry a target address");
    stream
        .write_all(&[RELAY_VERSION1, RELAY_STATUS_OK, 0x00, 0x00])
        .await
        .unwrap();

    if let Ok(mut upstream) = TcpStream::connect(target).await {
        let _ = copy_bidirectional(&mut stream, &mut upstream).await;
    }
}

async fn spawn_fake_relay_server(require_auth: bool) -> SocketAddr {
    let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        while let Ok((stream, _)) = listener.accept().await {
            tokio::spawn(serve_relay(stream, require_auth));
        }
    });
    addr
}

async fn spawn_fake_relay_tls_server(require_auth: bool) -> SocketAddr {
    let acceptor = tls_acceptor();
    let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        while let Ok((tcp, _)) = listener.accept().await {
            let acceptor = acceptor.clone();
            tokio::spawn(async move {
                if let Ok(tls) = acceptor.accept(tcp).await {
                    serve_relay(tls, require_auth).await;
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

fn gost_relay_mode(server: SocketAddr, auth: Option<(&str, &str)>, security: Security) -> OutboundMode {
    OutboundMode::GostRelay(Box::new(GostRelayOutboundConfig {
        server: server.ip().to_string(),
        port: server.port(),
        forward: false,
        auth: auth.map(|(u, p)| (u.to_string(), p.to_string())),
        security,
    }))
}

async fn assert_relays(outbound: OutboundMode, echo: SocketAddr, payload: &[u8]) {
    let handle = GripeKernel::start(GripeConfig {
        socks_listen: SocketAddr::from((Ipv4Addr::LOCALHOST, 0)),
        outbound,
    })
    .await
    .unwrap();

    let mut conn = socks5_connect(handle.local_addr(), echo).await;
    conn.write_all(payload).await.unwrap();
    let mut buf = vec![0u8; payload.len()];
    conn.read_exact(&mut buf).await.unwrap();
    assert_eq!(buf, payload);

    handle.shutdown().await;
}

#[tokio::test]
async fn relays_through_plaintext_gost_relay() {
    let echo = spawn_echo_server().await;
    let server = spawn_fake_relay_server(false).await;
    assert_relays(gost_relay_mode(server, None, Security::None), echo, b"hello gost relay").await;
}

#[tokio::test]
async fn relays_through_gost_relay_with_auth() {
    let echo = spawn_echo_server().await;
    let server = spawn_fake_relay_server(true).await;
    assert_relays(
        gost_relay_mode(server, Some((RELAY_USER, RELAY_PASS)), Security::None),
        echo,
        b"authenticated gost relay",
    )
    .await;
}

#[tokio::test]
async fn relays_through_tls_gost_relay() {
    let echo = spawn_echo_server().await;
    let server = spawn_fake_relay_tls_server(true).await;
    assert_relays(
        gost_relay_mode(
            server,
            Some((RELAY_USER, RELAY_PASS)),
            Security::Tls(TlsClientConfig {
                server_name: Some("localhost".to_string()),
                alpn: Vec::new(),
                skip_cert_verify: true,
                client_fingerprint: None,
                ech: None,
            }),
        ),
        echo,
        b"hello tls gost relay",
    )
    .await;
}

#[tokio::test]
async fn rejects_bad_credentials() {
    let echo = spawn_echo_server().await;
    let server = spawn_fake_relay_server(true).await;

    // Send the wrong password: the relay answers StatusUnauthorized, the
    // outbound fails to open, and the inbound reports a SOCKS5 failure.
    let handle = GripeKernel::start(GripeConfig {
        socks_listen: SocketAddr::from((Ipv4Addr::LOCALHOST, 0)),
        outbound: gost_relay_mode(server, Some((RELAY_USER, "wrong")), Security::None),
    })
    .await
    .unwrap();

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

    let mut reply = [0u8; 10];
    stream.read_exact(&mut reply).await.unwrap();
    assert_eq!(reply[0], 0x05);
    assert_ne!(reply[1], 0x00, "unauthorized relay must not report SOCKS5 success");

    handle.shutdown().await;
}
