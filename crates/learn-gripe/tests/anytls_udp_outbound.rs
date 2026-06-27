//! End-to-end proof that UDP rides an AnyTLS outbound via udp-over-tcp v2:
//! SOCKS5 UDP ASSOCIATE -> gripe inbound -> AnyTLS UoT tunnel -> fake server.
//!
//! The fake server validates the AnyTLS handshake (auth + `cmdSettings` +
//! `cmdSYN` + the `cmdPSH` to the UoT magic address), then switches to the
//! application byte stream carried by `cmdPSH` frames: it reads the UoT v2
//! *connect* request (`IsConnect=1` + SOCKS5 destination) and echoes each
//! `len(2 BE) | payload` datagram. We cover `none` / `tls` security, an IPv4
//! and a domain destination, and a `Routed` outbound resolving the datagram to
//! the AnyTLS tunnel.

use std::collections::{HashMap, VecDeque};
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::sync::Arc;

use learn_gripe::{
    AnyTlsOutboundConfig, GripeConfig, GripeKernel, OutboundMode, Router, Security, TlsClientConfig, Transport,
};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream, UdpSocket};
use tokio_rustls::TlsAcceptor;

const TEST_CERT: &str = include_str!("data/vless_tls_cert.pem");
const TEST_KEY: &str = include_str!("data/vless_tls_key.pem");

/// `SHA256("password")` — the AnyTLS authenticator for the test password.
const TEST_PASSWORD_SHA256: [u8; 32] = [
    0x5e, 0x88, 0x48, 0x98, 0xda, 0x28, 0x04, 0x71, 0x51, 0xd0, 0xe5, 0x6f, 0x8d, 0xc6, 0x29, 0x27, 0x73, 0x60, 0x3d,
    0x0d, 0x6a, 0xab, 0xbd, 0xd6, 0x2a, 0x11, 0xef, 0x72, 0x1d, 0x15, 0x42, 0xd8,
];

const UOT_MAGIC_ADDRESS: &str = "sp.v2.udp-over-tcp.arpa";

const CMD_PSH: u8 = 2;
const CMD_FIN: u8 = 3;
const CMD_SETTINGS: u8 = 4;
const CMD_SYN: u8 = 1;
const CMD_SYNACK: u8 = 7;
const CMD_SERVER_SETTINGS: u8 = 10;

const STREAM_ID: u32 = 1;

/// Read one AnyTLS session frame: `cmd(1) | streamId(u32 BE) | len(u16 BE) | data`.
async fn read_frame<S>(stream: &mut S) -> std::io::Result<(u8, u32, Vec<u8>)>
where
    S: AsyncRead + Unpin,
{
    let mut header = [0u8; 7];
    stream.read_exact(&mut header).await?;
    let cmd = header[0];
    let sid = u32::from_be_bytes([header[1], header[2], header[3], header[4]]);
    let len = u16::from_be_bytes([header[5], header[6]]) as usize;
    let mut data = vec![0u8; len];
    stream.read_exact(&mut data).await?;
    Ok((cmd, sid, data))
}

/// Write one AnyTLS session frame.
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

/// Reassembles the application byte stream carried by `cmdPSH` frames so the
/// UoT layer (which spans `cmdPSH` boundaries) can be parsed length-prefixed.
struct AppReader<S> {
    stream: S,
    buf: VecDeque<u8>,
}

impl<S: AsyncRead + AsyncWrite + Unpin> AppReader<S> {
    fn new(stream: S) -> Self {
        Self {
            stream,
            buf: VecDeque::new(),
        }
    }

    /// Read exactly `n` application bytes, pulling further `cmdPSH` frames as
    /// needed and skipping control frames. Returns `None` on `cmdFIN`/EOF.
    async fn read(&mut self, n: usize) -> Option<Vec<u8>> {
        while self.buf.len() < n {
            match read_frame(&mut self.stream).await {
                Ok((CMD_PSH, _sid, data)) => self.buf.extend(data),
                Ok((CMD_FIN, _, _)) | Err(_) => return None,
                Ok(_) => {} // ignore other control frames
            }
        }
        Some(self.buf.drain(..n).collect())
    }

    /// Send `data` as one `cmdPSH` frame on the stream.
    async fn write_psh(&mut self, data: &[u8]) {
        write_frame(&mut self.stream, CMD_PSH, STREAM_ID, data).await;
    }
}

/// Validate the AnyTLS + UoT handshake, then echo length-prefixed datagrams.
async fn serve_anytls_udp<S>(mut stream: S)
where
    S: AsyncRead + AsyncWrite + Unpin,
{
    // AnyTLS authentication: SHA256(password) + padding0.
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

    // cmdSettings -> cmdServerSettings; cmdSYN -> cmdSYNACK.
    let (cmd, _sid, _data) = read_frame(&mut stream).await.unwrap();
    assert_eq!(cmd, CMD_SETTINGS, "first frame must be cmdSettings");
    write_frame(&mut stream, CMD_SERVER_SETTINGS, 0, b"v=2").await;
    let (cmd, sid, _data) = read_frame(&mut stream).await.unwrap();
    assert_eq!(cmd, CMD_SYN, "expected cmdSYN");
    assert_eq!(sid, STREAM_ID, "stream id");
    write_frame(&mut stream, CMD_SYNACK, STREAM_ID, &[]).await;

    // First cmdPSH carries the UoT magic destination (so the server knows the
    // stream is udp-over-tcp rather than a raw TCP relay).
    let (cmd, sid, addr) = read_frame(&mut stream).await.unwrap();
    assert_eq!(cmd, CMD_PSH, "expected cmdPSH with the proxy target");
    assert_eq!(sid, STREAM_ID, "stream id");
    assert_eq!(
        parse_socks_host(&addr),
        UOT_MAGIC_ADDRESS,
        "must dial the UoT magic address"
    );

    // Everything after is the application byte stream: the UoT connect request
    // then length-prefixed datagrams.
    let mut app = AppReader::new(stream);

    // UoT v2 request: IsConnect(1) + SOCKS5 destination.
    let is_connect = app.read(1).await.expect("uot request")[0];
    assert_eq!(is_connect, 1, "kernel opens UoT in connect mode");
    consume_socks_addr(&mut app).await;

    // Connect mode: each datagram is `len(2 BE) | payload`, no per-packet addr.
    loop {
        let Some(len) = app.read(2).await else {
            return;
        };
        let len = u16::from_be_bytes([len[0], len[1]]) as usize;
        let Some(payload) = app.read(len).await else {
            return;
        };
        // Echo: re-frame as a UoT connect-mode datagram inside one cmdPSH.
        let mut out = Vec::with_capacity(2 + payload.len());
        out.extend_from_slice(&(payload.len() as u16).to_be_bytes());
        out.extend_from_slice(&payload);
        app.write_psh(&out).await;
    }
}

/// Return the host of a SOCKS5-encoded address (`atyp | addr | port`) as text.
fn parse_socks_host(addr: &[u8]) -> String {
    match addr[0] {
        0x01 => IpAddr::from([addr[1], addr[2], addr[3], addr[4]]).to_string(),
        0x03 => {
            let len = addr[1] as usize;
            String::from_utf8(addr[2..2 + len].to_vec()).unwrap()
        }
        0x04 => {
            let mut octets = [0u8; 16];
            octets.copy_from_slice(&addr[1..17]);
            IpAddr::from(octets).to_string()
        }
        other => panic!("unexpected atyp {other}"),
    }
}

/// Read and discard a SOCKS5 address (`atyp | addr | port`) from the app stream.
async fn consume_socks_addr<S: AsyncRead + AsyncWrite + Unpin>(app: &mut AppReader<S>) {
    let atyp = app.read(1).await.expect("atyp")[0];
    let addr_len = match atyp {
        0x01 => 4,
        0x04 => 16,
        0x03 => app.read(1).await.expect("domain len")[0] as usize,
        other => panic!("unexpected atyp {other}"),
    };
    app.read(addr_len).await.expect("addr");
    app.read(2).await.expect("port"); // port
}

async fn spawn_plaintext_server() -> SocketAddr {
    let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        while let Ok((stream, _)) = listener.accept().await {
            tokio::spawn(serve_anytls_udp(stream));
        }
    });
    addr
}

async fn spawn_tls_server() -> SocketAddr {
    let acceptor = tls_acceptor();
    let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        while let Ok((tcp, _)) = listener.accept().await {
            let acceptor = acceptor.clone();
            tokio::spawn(async move {
                if let Ok(tls) = acceptor.accept(tcp).await {
                    serve_anytls_udp(tls).await;
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
    let config = rustls::ServerConfig::builder_with_provider(Arc::new(rustls::crypto::ring::default_provider()))
        .with_safe_default_protocol_versions()
        .unwrap()
        .with_no_client_auth()
        .with_single_cert(certs, key)
        .unwrap();
    TlsAcceptor::from(Arc::new(config))
}

fn anytls(server: SocketAddr, security: Security) -> Box<AnyTlsOutboundConfig> {
    Box::new(AnyTlsOutboundConfig {
        server: server.ip().to_string(),
        port: server.port(),
        password_sha256: TEST_PASSWORD_SHA256,
        security,
        transport: Transport::Tcp,
    })
}

fn tls_security() -> Security {
    Security::Tls(TlsClientConfig {
        server_name: Some("localhost".to_string()),
        alpn: Vec::new(),
        skip_cert_verify: true,
        client_fingerprint: None,
        ech: None,
    })
}

async fn socks5_udp_associate(proxy: SocketAddr) -> (TcpStream, SocketAddr) {
    let mut stream = TcpStream::connect(proxy).await.unwrap();
    stream.write_all(&[0x05, 0x01, 0x00]).await.unwrap();
    let mut selection = [0u8; 2];
    stream.read_exact(&mut selection).await.unwrap();
    assert_eq!(selection, [0x05, 0x00]);
    stream
        .write_all(&[0x05, 0x03, 0x00, 0x01, 0, 0, 0, 0, 0, 0])
        .await
        .unwrap();
    let mut reply = [0u8; 10];
    stream.read_exact(&mut reply).await.unwrap();
    assert_eq!(reply[1], 0x00, "expected ASSOCIATE success reply");
    let ip = Ipv4Addr::new(reply[4], reply[5], reply[6], reply[7]);
    let port = u16::from_be_bytes([reply[8], reply[9]]);
    (stream, SocketAddr::from((ip, port)))
}

fn udp_datagram_ipv4(dst: SocketAddr, payload: &[u8]) -> Vec<u8> {
    let ip = match dst.ip() {
        IpAddr::V4(v4) => v4.octets(),
        IpAddr::V6(_) => panic!("test uses IPv4"),
    };
    let mut datagram = vec![0x00, 0x00, 0x00, 0x01];
    datagram.extend_from_slice(&ip);
    datagram.extend_from_slice(&dst.port().to_be_bytes());
    datagram.extend_from_slice(payload);
    datagram
}

fn udp_datagram_domain(host: &str, port: u16, payload: &[u8]) -> Vec<u8> {
    let mut datagram = vec![0x00, 0x00, 0x00, 0x03, host.len() as u8];
    datagram.extend_from_slice(host.as_bytes());
    datagram.extend_from_slice(&port.to_be_bytes());
    datagram.extend_from_slice(payload);
    datagram
}

fn payload_offset(buf: &[u8]) -> usize {
    match buf[3] {
        0x01 => 3 + 1 + 4 + 2,
        0x04 => 3 + 1 + 16 + 2,
        0x03 => 3 + 1 + 1 + buf[4] as usize + 2,
        other => panic!("unexpected reply atyp {other}"),
    }
}

async fn assert_udp_relays(outbound: OutboundMode, datagram: Vec<u8>, payload: &[u8]) {
    let handle = GripeKernel::start(GripeConfig {
        socks_listen: SocketAddr::from((Ipv4Addr::LOCALHOST, 0)),
        outbound,
    })
    .await
    .unwrap();

    let (_control, relay) = socks5_udp_associate(handle.local_addr()).await;
    let client = UdpSocket::bind((Ipv4Addr::LOCALHOST, 0)).await.unwrap();
    client.send_to(&datagram, relay).await.unwrap();

    let mut buf = [0u8; 2048];
    let (n, from) = client.recv_from(&mut buf).await.unwrap();
    assert_eq!(from, relay, "reply must come from the relay socket");
    let offset = payload_offset(&buf[..n]);
    assert_eq!(&buf[offset..n], payload, "payload must be echoed verbatim");

    handle.shutdown().await;
}

#[tokio::test]
async fn udp_relays_through_plaintext_anytls_ipv4() {
    let server = spawn_plaintext_server().await;
    let dst = SocketAddr::from((Ipv4Addr::new(1, 2, 3, 4), 443));
    assert_udp_relays(
        OutboundMode::AnyTls(anytls(server, Security::None)),
        udp_datagram_ipv4(dst, b"anytls udp ping"),
        b"anytls udp ping",
    )
    .await;
}

#[tokio::test]
async fn udp_relays_through_plaintext_anytls_domain() {
    let server = spawn_plaintext_server().await;
    assert_udp_relays(
        OutboundMode::AnyTls(anytls(server, Security::None)),
        udp_datagram_domain("example.com", 53, b"anytls domain query"),
        b"anytls domain query",
    )
    .await;
}

#[tokio::test]
async fn udp_relays_through_tls_anytls_ipv4() {
    let server = spawn_tls_server().await;
    let dst = SocketAddr::from((Ipv4Addr::new(9, 9, 9, 9), 443));
    assert_udp_relays(
        OutboundMode::AnyTls(anytls(server, tls_security())),
        udp_datagram_ipv4(dst, b"tls anytls udp"),
        b"tls anytls udp",
    )
    .await;
}

#[tokio::test]
async fn udp_relays_through_routed_anytls() {
    let server = spawn_plaintext_server().await;
    let mut outbounds = HashMap::new();
    outbounds.insert(
        "proxy".to_string(),
        OutboundMode::AnyTls(anytls(server, Security::None)),
    );
    let router = Router::new(outbounds, vec![], "proxy").unwrap();
    let dst = SocketAddr::from((Ipv4Addr::new(1, 2, 3, 4), 443));
    assert_udp_relays(
        OutboundMode::Routed(Box::new(router)),
        udp_datagram_ipv4(dst, b"routed anytls udp"),
        b"routed anytls udp",
    )
    .await;
}
