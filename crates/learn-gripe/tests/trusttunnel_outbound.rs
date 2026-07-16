//! End-to-end proof that traffic flows through a TrustTunnel outbound:
//! a SOCKS5 client -> gripe inbound -> TrustTunnel outbound -> fake server.
//!
//! TrustTunnel is an HTTP/2 `CONNECT` tunnel: each proxied target is one
//! HTTP/2 `CONNECT` request whose `:authority` names the target, carrying the
//! uplink on the request body and the downlink on the response body. The fake
//! server therefore speaks HTTP/2 (`h2` crate), validates the request
//! (`CONNECT` method, `:authority`, `Proxy-Authorization`) and echoes the body.
//!
//! Coverage:
//! * plaintext h2 (prior-knowledge) — isolates the CONNECT framing from TLS;
//! * h2 over TLS with ALPN `h2` — the real transport;
//! * a wrong `Proxy-Authorization` yields a non-200 response, so the relay
//!   fails rather than silently carrying unauthenticated traffic;
//! * UDP over the `_udp2` tunnel: a datagram is length-prefixed, addressed to
//!   its peer and echoed byte-for-byte.

use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::sync::Arc;

use bytes::Bytes;
use h2::server::SendResponse;
use h2::{RecvStream, SendStream};
use http::{Method, Response, StatusCode};
use learn_gripe::{GripeConfig, GripeKernel, OutboundMode, Security, TlsClientConfig, TrustTunnelOutboundConfig};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream, UdpSocket};
use tokio_rustls::TlsAcceptor;

const TEST_CERT: &str = include_str!("data/vless_tls_cert.pem");
const TEST_KEY: &str = include_str!("data/vless_tls_key.pem");

/// `Proxy-Authorization: Basic base64("user:pass")` — the value the client is
/// expected to send for the credentials used by these tests.
const EXPECTED_AUTH: &str = "Basic dXNlcjpwYXNz";

/// Fixed server-side UDP header overhead after the 4-byte length prefix:
/// source address (16) + port (2) + zeroed local address (16) + port (2).
const UDP_SERVER_OVERHEAD: usize = 16 + 2 + 16 + 2;

/// Write `data` on an HTTP/2 send stream, respecting flow-control capacity.
async fn send_all(send: &mut SendStream<Bytes>, mut data: Bytes) {
    while !data.is_empty() {
        send.reserve_capacity(data.len());
        let cap = std::future::poll_fn(|cx| send.poll_capacity(cx))
            .await
            .expect("stream not closed")
            .expect("capacity");
        let take = cap.min(data.len());
        let chunk = data.split_to(take);
        send.send_data(chunk, false).expect("send data");
    }
}

/// Validate the `CONNECT` request common to every tunnel and return whether the
/// `Proxy-Authorization` matched. When it does not, the server answers `403` so
/// the client's `200`-only check fails the relay.
fn check_connect(request: &http::Request<RecvStream>, expect_authority: &str) -> bool {
    assert_eq!(request.method(), Method::CONNECT, "method must be CONNECT");
    assert_eq!(
        request.uri().authority().map(|a| a.as_str()),
        Some(expect_authority),
        "CONNECT :authority",
    );
    request
        .headers()
        .get(http::header::PROXY_AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        == Some(EXPECTED_AUTH)
}

/// Serve one HTTP/2 connection carrying a single TCP `CONNECT` tunnel to
/// `expect_authority`, echoing the request body onto the response body.
async fn serve_tcp<S>(io: S, expect_authority: String)
where
    S: AsyncRead + AsyncWrite + Unpin,
{
    let mut conn = h2::server::handshake(io).await.expect("h2 server handshake");
    while let Some(accepted) = conn.accept().await {
        let (request, respond) = accepted.expect("accept request");
        let authed = check_connect(&request, &expect_authority);
        tokio::spawn(echo_tcp_stream(request.into_body(), respond, authed));
    }
}

async fn echo_tcp_stream(mut body: RecvStream, mut respond: SendResponse<Bytes>, authed: bool) {
    if !authed {
        let response = Response::builder().status(StatusCode::FORBIDDEN).body(()).unwrap();
        let _ = respond.send_response(response, true);
        return;
    }
    let response = Response::builder().status(StatusCode::OK).body(()).unwrap();
    let mut send = respond.send_response(response, false).expect("send 200");
    while let Some(chunk) = body.data().await {
        let data = chunk.expect("recv body chunk");
        let _ = body.flow_control().release_capacity(data.len());
        send_all(&mut send, data).await;
    }
    let _ = send.send_data(Bytes::new(), true);
}

/// Serve one HTTP/2 connection carrying a single `_udp2` tunnel, decoding each
/// client datagram frame and echoing its payload back as a server frame
/// addressed to the same peer.
async fn serve_udp<S>(io: S)
where
    S: AsyncRead + AsyncWrite + Unpin,
{
    let mut conn = h2::server::handshake(io).await.expect("h2 server handshake");
    while let Some(accepted) = conn.accept().await {
        let (request, respond) = accepted.expect("accept request");
        let authed = check_connect(&request, "_udp2");
        tokio::spawn(echo_udp_stream(request.into_body(), respond, authed));
    }
}

async fn echo_udp_stream(mut body: RecvStream, mut respond: SendResponse<Bytes>, authed: bool) {
    assert!(authed, "udp tunnel must authenticate");
    let response = Response::builder().status(StatusCode::OK).body(()).unwrap();
    let mut send = respond.send_response(response, false).expect("send 200");
    let mut buf: Vec<u8> = Vec::new();
    while let Some(chunk) = body.data().await {
        let data = chunk.expect("recv body chunk");
        body.flow_control().release_capacity(data.len()).ok();
        buf.extend_from_slice(&data);
        while let Some((peer_addr, peer_port, payload, consumed)) = parse_client_packet(&buf) {
            buf.drain(..consumed);
            send_all(&mut send, encode_server_packet(peer_addr, peer_port, &payload)).await;
        }
    }
    let _ = send.send_data(Bytes::new(), true);
}

/// Parse one client UDP frame from `buf`, returning the destination address /
/// port bytes, the payload and the total bytes consumed, or `None` if `buf`
/// holds an incomplete frame.
///
/// Client frame: `len(u32) | src(16, zero) | src port(2) | dst(16) |
/// dst port(2) | app_name_len(1) | app_name | payload`, `len` counting every
/// byte after the prefix.
fn parse_client_packet(buf: &[u8]) -> Option<([u8; 16], [u8; 2], Vec<u8>, usize)> {
    if buf.len() < 4 {
        return None;
    }
    let len = u32::from_be_bytes([buf[0], buf[1], buf[2], buf[3]]) as usize;
    let total = 4 + len;
    if buf.len() < total {
        return None;
    }
    let frame = &buf[4..total];
    let dst: [u8; 16] = frame[18..34].try_into().unwrap();
    let dst_port: [u8; 2] = frame[34..36].try_into().unwrap();
    let app_len = frame[36] as usize;
    let payload = frame[37 + app_len..].to_vec();
    Some((dst, dst_port, payload, total))
}

/// Server frame: `len(u32) | src(16) | src port(2) | local(16, zero) |
/// local port(2) | payload`.
fn encode_server_packet(src: [u8; 16], src_port: [u8; 2], payload: &[u8]) -> Bytes {
    let len = (UDP_SERVER_OVERHEAD + payload.len()) as u32;
    let mut out = Vec::with_capacity(4 + len as usize);
    out.extend_from_slice(&len.to_be_bytes());
    out.extend_from_slice(&src);
    out.extend_from_slice(&src_port);
    out.extend_from_slice(&[0u8; 18]); // zeroed local address + port
    out.extend_from_slice(payload);
    Bytes::from(out)
}

fn tls_acceptor() -> TlsAcceptor {
    let certs = rustls_pemfile::certs(&mut TEST_CERT.as_bytes())
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    let key = rustls_pemfile::private_key(&mut TEST_KEY.as_bytes()).unwrap().unwrap();
    let provider = Arc::new(rustls::crypto::ring::default_provider());
    let mut config = rustls::ServerConfig::builder_with_provider(provider)
        .with_safe_default_protocol_versions()
        .unwrap()
        .with_no_client_auth()
        .with_single_cert(certs, key)
        .unwrap();
    config.alpn_protocols = vec![b"h2".to_vec()];
    TlsAcceptor::from(Arc::new(config))
}

async fn spawn_plaintext_tcp_server(expect_authority: &'static str) -> SocketAddr {
    let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        while let Ok((tcp, _)) = listener.accept().await {
            tokio::spawn(serve_tcp(tcp, expect_authority.to_string()));
        }
    });
    addr
}

async fn spawn_tls_tcp_server(expect_authority: &'static str) -> SocketAddr {
    let acceptor = tls_acceptor();
    let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        while let Ok((tcp, _)) = listener.accept().await {
            let acceptor = acceptor.clone();
            tokio::spawn(async move {
                if let Ok(tls) = acceptor.accept(tcp).await {
                    serve_tcp(tls, expect_authority.to_string()).await;
                }
            });
        }
    });
    addr
}

async fn spawn_plaintext_udp_server() -> SocketAddr {
    let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        while let Ok((tcp, _)) = listener.accept().await {
            tokio::spawn(serve_udp(tcp));
        }
    });
    addr
}

fn trusttunnel(server: SocketAddr, security: Security, udp: bool) -> Box<TrustTunnelOutboundConfig> {
    Box::new(TrustTunnelOutboundConfig {
        server: server.ip().to_string(),
        port: server.port(),
        username: "user".to_string(),
        password: "pass".to_string(),
        security,
        udp,
    })
}

fn plaintext_security() -> Security {
    Security::None
}

fn tls_security() -> Security {
    Security::Tls(TlsClientConfig {
        server_name: Some("localhost".to_string()),
        alpn: vec!["h2".to_string()],
        skip_cert_verify: true,
        client_fingerprint: None,
        ech: None,
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

/// Start the kernel with `outbound`, run one SOCKS5 CONNECT to `target` and
/// assert the payload echoes back unchanged.
async fn assert_relays(outbound: OutboundMode, target: SocketAddr, payload: &[u8]) {
    let handle = GripeKernel::start(GripeConfig {
        socks_listen: SocketAddr::from((Ipv4Addr::LOCALHOST, 0)),
        outbound,
    })
    .await
    .unwrap();

    let mut conn = socks5_connect(handle.local_addr(), target).await;
    conn.write_all(payload).await.unwrap();
    let mut buf = vec![0u8; payload.len()];
    conn.read_exact(&mut buf).await.unwrap();
    assert_eq!(buf, payload);

    handle.shutdown().await;
}

/// Start the kernel and assert a SOCKS5 CONNECT through it fails (the relay
/// cannot open because the server did not answer `200`).
async fn assert_connect_fails(outbound: OutboundMode, target: SocketAddr) {
    let handle = GripeKernel::start(GripeConfig {
        socks_listen: SocketAddr::from((Ipv4Addr::LOCALHOST, 0)),
        outbound,
    })
    .await
    .unwrap();

    let mut stream = TcpStream::connect(handle.local_addr()).await.unwrap();
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
    // The upstream CONNECT fails, so the inbound reports a non-success reply or
    // closes the stream — either way the read does not yield a success reply.
    if stream.read_exact(&mut reply).await.is_ok() {
        assert_ne!(reply[1], 0x00, "expected a non-success SOCKS5 reply");
    }

    handle.shutdown().await;
}

#[tokio::test]
async fn relays_through_plaintext_trusttunnel() {
    let server = spawn_plaintext_tcp_server("1.2.3.4:443").await;
    assert_relays(
        OutboundMode::TrustTunnel(trusttunnel(server, plaintext_security(), false)),
        SocketAddr::from((Ipv4Addr::new(1, 2, 3, 4), 443)),
        b"hello trusttunnel",
    )
    .await;
}

#[tokio::test]
async fn relays_through_tls_trusttunnel() {
    let server = spawn_tls_tcp_server("5.6.7.8:8443").await;
    assert_relays(
        OutboundMode::TrustTunnel(trusttunnel(server, tls_security(), false)),
        SocketAddr::from((Ipv4Addr::new(5, 6, 7, 8), 8443)),
        b"hello tls trusttunnel",
    )
    .await;
}

#[tokio::test]
async fn wrong_proxy_authorization_fails_the_relay() {
    let server = spawn_plaintext_tcp_server("1.2.3.4:443").await;
    let mut cfg = trusttunnel(server, plaintext_security(), false);
    cfg.password = "wrong".to_string();
    assert_connect_fails(
        OutboundMode::TrustTunnel(cfg),
        SocketAddr::from((Ipv4Addr::new(1, 2, 3, 4), 443)),
    )
    .await;
}

#[tokio::test]
async fn udp_relays_through_trusttunnel() {
    let server = spawn_plaintext_udp_server().await;
    let handle = GripeKernel::start(GripeConfig {
        socks_listen: SocketAddr::from((Ipv4Addr::LOCALHOST, 0)),
        outbound: OutboundMode::TrustTunnel(trusttunnel(server, plaintext_security(), true)),
    })
    .await
    .unwrap();

    // SOCKS5 UDP ASSOCIATE.
    let mut control = TcpStream::connect(handle.local_addr()).await.unwrap();
    control.write_all(&[0x05, 0x01, 0x00]).await.unwrap();
    let mut selection = [0u8; 2];
    control.read_exact(&mut selection).await.unwrap();
    assert_eq!(selection, [0x05, 0x00]);
    control
        .write_all(&[0x05, 0x03, 0x00, 0x01, 0, 0, 0, 0, 0, 0])
        .await
        .unwrap();
    let mut reply = [0u8; 10];
    control.read_exact(&mut reply).await.unwrap();
    assert_eq!(reply[1], 0x00, "expected ASSOCIATE success reply");
    let relay = SocketAddr::from((
        Ipv4Addr::new(reply[4], reply[5], reply[6], reply[7]),
        u16::from_be_bytes([reply[8], reply[9]]),
    ));

    let dst = SocketAddr::from((Ipv4Addr::new(9, 9, 9, 9), 53));
    let mut datagram = vec![0x00, 0x00, 0x00, 0x01];
    datagram.extend_from_slice(&[9, 9, 9, 9]);
    datagram.extend_from_slice(&dst.port().to_be_bytes());
    datagram.extend_from_slice(b"trusttunnel udp ping");

    let client = UdpSocket::bind((Ipv4Addr::LOCALHOST, 0)).await.unwrap();
    client.send_to(&datagram, relay).await.unwrap();

    let mut buf = [0u8; 2048];
    let (n, from) = client.recv_from(&mut buf).await.unwrap();
    assert_eq!(from, relay, "reply must come from the relay socket");
    // Skip the SOCKS5 UDP reply header (RSV/FRAG + IPv4 address + port).
    let offset = 3 + 1 + 4 + 2;
    assert_eq!(&buf[offset..n], b"trusttunnel udp ping", "payload echoed verbatim");

    handle.shutdown().await;
}
