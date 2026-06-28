//! End-to-end proof that traffic flows through a Hysteria **v1** (QUIC)
//! outbound: a SOCKS5 client -> gripe inbound -> Hysteria v1 outbound -> fake
//! Hysteria v1 server.
//!
//! The fake server runs on a real QUIC endpoint (quinn, the same vendored
//! rustls fork) speaking ALPN `hysteria`. It drives the v1 control protocol:
//! it reads the 1-byte version + `ClientHello { rate, auth }` on the first
//! bidirectional stream (validating the auth payload) and answers a
//! `ServerHello { ok }`, then accepts the proxy stream, parses the
//! `ClientRequest { udp, host, port }`, answers a `ServerResponse { ok }`, and
//! echoes the relayed payload. This exercises the full client path: QUIC
//! handshake, control-stream auth, request framing, and bidirectional relay.
//!
//! `relays_through_hysteria_xplus_obfs` additionally wraps the server's UDP
//! socket with an **independent** XPlus codec (SHA-256 XOR, not the kernel's),
//! proving byte-for-byte obfuscation compatibility.

use std::io;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::pin::Pin;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::task::{Context, Poll};

use learn_gripe::{GripeConfig, GripeKernel, HysteriaOutboundConfig, OutboundMode, PortHopConfig, XPlus};
use quinn::crypto::rustls::QuicServerConfig;
use quinn::udp::{RecvMeta, Transmit};
use quinn::{AsyncUdpSocket, Endpoint, EndpointConfig, RecvStream, SendStream, ServerConfig, UdpPoller};
use sha2::{Digest, Sha256};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::sync::oneshot;

const TEST_CERT: &str = include_str!("data/vless_tls_cert.pem");
const TEST_KEY: &str = include_str!("data/vless_tls_key.pem");

const AUTH: &str = "correct horse battery staple";
const OBFS_KEY: &str = "xplus-shared-secret";
const MESSAGE: &[u8] = b"the quick brown fox jumps over the lazy dog";
const PROTOCOL_VERSION: u8 = 3;
const XPLUS_SALT_LEN: usize = 16;

// --- Independent XPlus codec (separate from the kernel implementation) ---

/// Derive the per-datagram XOR key: `SHA-256(psk || salt)`.
fn xplus_key(psk: &[u8], salt: &[u8]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(psk);
    hasher.update(salt);
    hasher.finalize().into()
}

/// Obfuscate `payload` into `salt(16) || (payload XOR key)`.
fn xplus_obfuscate(psk: &[u8], payload: &[u8], salt: [u8; XPLUS_SALT_LEN]) -> Vec<u8> {
    let key = xplus_key(psk, &salt);
    let mut out = Vec::with_capacity(XPLUS_SALT_LEN + payload.len());
    out.extend_from_slice(&salt);
    for (i, &b) in payload.iter().enumerate() {
        out.push(b ^ key[i % 32]);
    }
    out
}

/// Recover the payload from `salt(16) || ciphertext`, or `None` if too short.
fn xplus_deobfuscate(psk: &[u8], datagram: &[u8]) -> Option<Vec<u8>> {
    if datagram.len() <= XPLUS_SALT_LEN {
        return None;
    }
    let key = xplus_key(psk, &datagram[..XPLUS_SALT_LEN]);
    Some(
        datagram[XPLUS_SALT_LEN..]
            .iter()
            .enumerate()
            .map(|(i, &b)| b ^ key[i % 32])
            .collect(),
    )
}

/// A server-side [`AsyncUdpSocket`] that de-obfuscates inbound datagrams and
/// obfuscates outbound ones with the independent XPlus codec.
struct ServerXplusSocket {
    inner: Arc<dyn AsyncUdpSocket>,
    psk: Vec<u8>,
    salt_counter: AtomicU64,
}

impl std::fmt::Debug for ServerXplusSocket {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ServerXplusSocket").finish()
    }
}

impl AsyncUdpSocket for ServerXplusSocket {
    fn create_io_poller(self: Arc<Self>) -> Pin<Box<dyn UdpPoller>> {
        self.inner.clone().create_io_poller()
    }

    fn try_send(&self, transmit: &Transmit<'_>) -> io::Result<()> {
        let counter = self.salt_counter.fetch_add(1, Ordering::Relaxed);
        let mut salt = [0u8; XPLUS_SALT_LEN];
        salt[..8].copy_from_slice(&counter.to_be_bytes());
        let obfuscated = xplus_obfuscate(&self.psk, transmit.contents, salt);
        self.inner.try_send(&Transmit {
            destination: transmit.destination,
            ecn: transmit.ecn,
            contents: &obfuscated,
            segment_size: None,
            src_ip: transmit.src_ip,
        })
    }

    fn poll_recv(
        &self,
        cx: &mut Context<'_>,
        bufs: &mut [io::IoSliceMut<'_>],
        meta: &mut [RecvMeta],
    ) -> Poll<io::Result<usize>> {
        let poll = self.inner.poll_recv(cx, bufs, meta);
        if let Poll::Ready(Ok(count)) = &poll {
            for i in 0..*count {
                let len = meta[i].len;
                match xplus_deobfuscate(&self.psk, &bufs[i][..len]) {
                    Some(payload) => {
                        bufs[i][..payload.len()].copy_from_slice(&payload);
                        meta[i].len = payload.len();
                        meta[i].stride = payload.len().max(1);
                    }
                    None => {
                        meta[i].len = 0;
                        meta[i].stride = 1;
                    }
                }
            }
        }
        poll
    }

    fn local_addr(&self) -> io::Result<SocketAddr> {
        self.inner.local_addr()
    }

    fn max_transmit_segments(&self) -> usize {
        1
    }

    fn max_receive_segments(&self) -> usize {
        1
    }

    fn may_fragment(&self) -> bool {
        self.inner.may_fragment()
    }
}

/// Build a quinn server config from the baked test cert/key, offering the
/// `hysteria` ALPN the v1 client defaults to.
fn server_config() -> ServerConfig {
    let certs = rustls_pemfile::certs(&mut TEST_CERT.as_bytes())
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    let key = rustls_pemfile::private_key(&mut TEST_KEY.as_bytes()).unwrap().unwrap();
    let mut crypto = rustls::ServerConfig::builder_with_provider(Arc::new(rustls::crypto::ring::default_provider()))
        .with_protocol_versions(&[&rustls::version::TLS13])
        .unwrap()
        .with_no_client_auth()
        .with_single_cert(certs, key)
        .unwrap();
    crypto.alpn_protocols = vec![b"hysteria".to_vec()];
    let quic = QuicServerConfig::try_from(crypto).unwrap();
    ServerConfig::with_crypto(Arc::new(quic))
}

/// A plain quinn server endpoint on an ephemeral loopback port.
fn plain_server_endpoint() -> Endpoint {
    Endpoint::server(server_config(), (Ipv4Addr::LOCALHOST, 0).into()).unwrap()
}

/// A quinn server endpoint whose UDP socket de-obfuscates/obfuscates with the
/// independent XPlus codec.
fn xplus_server_endpoint(psk: &[u8]) -> Endpoint {
    let runtime = quinn::default_runtime().expect("tokio runtime");
    let socket = std::net::UdpSocket::bind((Ipv4Addr::LOCALHOST, 0)).unwrap();
    let inner = runtime.wrap_udp_socket(socket).unwrap();
    let wrapped = Arc::new(ServerXplusSocket {
        inner,
        psk: psk.to_vec(),
        salt_counter: AtomicU64::new(1),
    });
    Endpoint::new_with_abstract_socket(EndpointConfig::default(), Some(server_config()), wrapped, runtime).unwrap()
}

/// Read a `u16`-length-prefixed byte string from a quinn recv stream.
async fn read_u16_bytes(recv: &mut RecvStream) -> Vec<u8> {
    let mut len = [0u8; 2];
    recv.read_exact(&mut len).await.unwrap();
    let mut buf = vec![0u8; u16::from_be_bytes(len) as usize];
    recv.read_exact(&mut buf).await.unwrap();
    buf
}

/// Write a `ServerHello { ok: true }` with empty rate/message.
async fn write_server_hello(send: &mut SendStream) {
    let mut buf = vec![1u8]; // ok = true
    buf.extend_from_slice(&0u64.to_be_bytes()); // send_bps
    buf.extend_from_slice(&0u64.to_be_bytes()); // recv_bps
    buf.extend_from_slice(&0u16.to_be_bytes()); // message length
    send.write_all(&buf).await.unwrap();
    send.flush().await.unwrap();
}

/// Run the fake Hysteria v1 server: validate the control-stream `ClientHello`
/// auth, accept the proxy stream, parse the `ClientRequest`, answer OK, and
/// echo the relayed payload. Reports the parsed `host:port` target.
async fn run_server(endpoint: Endpoint, target_tx: oneshot::Sender<String>) {
    let conn = endpoint.accept().await.unwrap().await.unwrap();

    // --- Control stream: version + ClientHello -> ServerHello ---
    let (mut ctl_send, mut ctl_recv) = conn.accept_bi().await.unwrap();
    let mut version = [0u8; 1];
    ctl_recv.read_exact(&mut version).await.unwrap();
    assert_eq!(version[0], PROTOCOL_VERSION, "protocol version");
    let mut rate = [0u8; 16]; // send_bps(8) + recv_bps(8)
    ctl_recv.read_exact(&mut rate).await.unwrap();
    let auth = read_u16_bytes(&mut ctl_recv).await;
    assert_eq!(auth, AUTH.as_bytes(), "ClientHello auth payload");
    write_server_hello(&mut ctl_send).await;

    // --- Proxy stream: ClientRequest -> ServerResponse + echo ---
    let (mut send, mut recv) = conn.accept_bi().await.unwrap();
    let mut udp = [0u8; 1];
    recv.read_exact(&mut udp).await.unwrap();
    assert_eq!(udp[0], 0, "TCP request (udp = false)");
    let host = read_u16_bytes(&mut recv).await;
    let mut port = [0u8; 2];
    recv.read_exact(&mut port).await.unwrap();
    let target = format!("{}:{}", String::from_utf8(host).unwrap(), u16::from_be_bytes(port));
    target_tx.send(target).unwrap();

    // ServerResponse: ok = true, session id 0, empty message.
    let mut resp = vec![1u8];
    resp.extend_from_slice(&0u32.to_be_bytes());
    resp.extend_from_slice(&0u16.to_be_bytes());
    send.write_all(&resp).await.unwrap();

    let mut payload = vec![0u8; MESSAGE.len()];
    recv.read_exact(&mut payload).await.unwrap();
    send.write_all(&payload).await.unwrap();
    send.finish().unwrap();

    conn.closed().await;
    drop(ctl_send);
}

/// Drive a minimal SOCKS5 CONNECT to `target` through the kernel inbound.
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

fn base_config(port: u16) -> HysteriaOutboundConfig {
    HysteriaOutboundConfig {
        server: "127.0.0.1".to_string(),
        port,
        auth: AUTH.as_bytes().to_vec(),
        server_name: "example.com".to_string(),
        alpn: vec!["hysteria".to_string()],
        skip_cert_verify: true,
        send_bps: 0,
        recv_bps: 0,
        obfs: None,
        port_hop: None,
    }
}

/// Relay `MESSAGE` through the kernel using `config`; assert it echoes back and
/// the server parsed the expected target.
async fn relay_and_assert(config: HysteriaOutboundConfig, target_rx: oneshot::Receiver<String>) {
    let dummy_target = SocketAddr::from((Ipv4Addr::new(93, 184, 216, 34), 443));
    let handle = GripeKernel::start(GripeConfig {
        socks_listen: SocketAddr::from((Ipv4Addr::LOCALHOST, 0)),
        outbound: OutboundMode::Hysteria(Box::new(config)),
    })
    .await
    .unwrap();

    let mut conn = socks5_connect(handle.local_addr(), dummy_target).await;
    conn.write_all(MESSAGE).await.unwrap();
    conn.flush().await.unwrap();

    let mut echo = vec![0u8; MESSAGE.len()];
    conn.read_exact(&mut echo).await.unwrap();
    assert_eq!(echo, MESSAGE, "payload relayed and echoed verbatim through Hysteria v1");

    assert_eq!(
        target_rx.await.unwrap(),
        dummy_target.to_string(),
        "server parsed the ClientRequest target"
    );
    drop(conn);
}

#[tokio::test]
async fn relays_through_hysteria_with_auth() {
    let endpoint = plain_server_endpoint();
    let server_addr = endpoint.local_addr().unwrap();
    let (target_tx, target_rx) = oneshot::channel();
    let server = tokio::spawn(run_server(endpoint, target_tx));

    relay_and_assert(base_config(server_addr.port()), target_rx).await;
    server.await.unwrap();
}

#[tokio::test]
async fn relays_through_hysteria_xplus_obfs() {
    let endpoint = xplus_server_endpoint(OBFS_KEY.as_bytes());
    let real_port = endpoint.local_addr().unwrap().port();
    // Canonical port differs from the real port; the only hop port is the real
    // one, exercising both the send rewrite and recv source normalization.
    let canonical_port = real_port.wrapping_add(1).max(1);
    let (target_tx, target_rx) = oneshot::channel();
    let server = tokio::spawn(run_server(endpoint, target_tx));

    let mut config = base_config(canonical_port);
    config.obfs = Some(XPlus::new(OBFS_KEY.as_bytes().to_vec()));
    config.port_hop = Some(PortHopConfig::parse(&real_port.to_string(), None).unwrap());
    relay_and_assert(config, target_rx).await;
    server.await.unwrap();
}
