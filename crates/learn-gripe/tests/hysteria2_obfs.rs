//! End-to-end proof that Hysteria2 **Salamander obfuscation** (`obfs:
//! salamander`) and **port hopping** (`ports`) interoperate with a peer that
//! speaks the same wire format.
//!
//! The three tests share the Hysteria2 fake server from `hysteria2_outbound.rs`
//! (HTTP/3 `POST /auth` then a `TCPRequest` echo) but vary the UDP socket below
//! QUIC:
//!
//! - `relays_through_salamander_obfs`: the server runs on a custom
//!   [`quinn::AsyncUdpSocket`] that de-obfuscates every inbound datagram and
//!   obfuscates every outbound one using an **independent** BLAKE2b-256 XOR
//!   implementation (not the kernel's), proving byte-for-byte compatibility.
//! - `relays_through_port_hopping`: a plain server on port `P`, while the client
//!   is configured with a *different* canonical `port` and a `ports` range of
//!   exactly `{P}`. This exercises both the send-side port rewrite and the
//!   recv-side source-address normalization (canonical != real port).
//! - `relays_through_obfs_and_port_hopping`: both at once, the real-world combo.

use std::io;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};

use blake2::Blake2b;
use blake2::digest::Digest;
use blake2::digest::consts::U32;
use bytes::Bytes;
use learn_gripe::{
    Congestion, GripeConfig, GripeKernel, Hysteria2OutboundConfig, OutboundMode, PortHopConfig, Salamander,
};
use quinn::crypto::rustls::QuicServerConfig;
use quinn::udp::{RecvMeta, Transmit};
use quinn::{AsyncUdpSocket, Endpoint, EndpointConfig, ServerConfig, UdpPoller};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::sync::oneshot;

const TEST_CERT: &str = include_str!("data/vless_tls_cert.pem");
const TEST_KEY: &str = include_str!("data/vless_tls_key.pem");

const PASSWORD: &str = "correct horse battery staple";
const OBFS_PASSWORD: &str = "salamander-shared-secret";
const MESSAGE: &[u8] = b"the quick brown fox jumps over the lazy dog";
const SALT_LEN: usize = 8;

// --- Independent Salamander codec (separate from the kernel implementation) ---

/// Derive the per-datagram XOR key: `BLAKE2b-256(psk || salt)`.
fn salamander_key(psk: &[u8], salt: &[u8]) -> [u8; 32] {
    let mut hasher = Blake2b::<U32>::new();
    hasher.update(psk);
    hasher.update(salt);
    let digest = hasher.finalize();
    let mut key = [0u8; 32];
    key.copy_from_slice(&digest);
    key
}

/// Obfuscate `payload` into `salt(8) || (payload XOR key)` with a fixed-but-
/// varying salt (a simple counter is fine for a test server).
fn salamander_obfuscate(psk: &[u8], payload: &[u8], salt: [u8; SALT_LEN]) -> Vec<u8> {
    let key = salamander_key(psk, &salt);
    let mut out = Vec::with_capacity(SALT_LEN + payload.len());
    out.extend_from_slice(&salt);
    for (i, &b) in payload.iter().enumerate() {
        out.push(b ^ key[i % 32]);
    }
    out
}

/// Recover the payload from `salt(8) || ciphertext`, or `None` if too short.
fn salamander_deobfuscate(psk: &[u8], datagram: &[u8]) -> Option<Vec<u8>> {
    if datagram.len() <= SALT_LEN {
        return None;
    }
    let key = salamander_key(psk, &datagram[..SALT_LEN]);
    Some(
        datagram[SALT_LEN..]
            .iter()
            .enumerate()
            .map(|(i, &b)| b ^ key[i % 32])
            .collect(),
    )
}

/// A server-side [`AsyncUdpSocket`] that de-obfuscates inbound datagrams and
/// obfuscates outbound ones, independent of the kernel's `ObfsHopSocket`.
struct ServerObfsSocket {
    inner: Arc<dyn AsyncUdpSocket>,
    psk: Vec<u8>,
    salt_counter: std::sync::atomic::AtomicU64,
}

impl std::fmt::Debug for ServerObfsSocket {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ServerObfsSocket").finish()
    }
}

impl AsyncUdpSocket for ServerObfsSocket {
    fn create_io_poller(self: Arc<Self>) -> Pin<Box<dyn UdpPoller>> {
        self.inner.clone().create_io_poller()
    }

    fn try_send(&self, transmit: &Transmit<'_>) -> io::Result<()> {
        let counter = self.salt_counter.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let mut salt = [0u8; SALT_LEN];
        salt.copy_from_slice(&counter.to_be_bytes());
        let obfuscated = salamander_obfuscate(&self.psk, transmit.contents, salt);
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
                match salamander_deobfuscate(&self.psk, &bufs[i][..len]) {
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

/// Build a quinn server config from the baked test cert/key, offering `h3`.
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
    crypto.alpn_protocols = vec![b"h3".to_vec()];
    let quic = QuicServerConfig::try_from(crypto).unwrap();
    ServerConfig::with_crypto(Arc::new(quic))
}

/// A plain quinn server endpoint on an ephemeral loopback port.
fn plain_server_endpoint() -> Endpoint {
    Endpoint::server(server_config(), (Ipv4Addr::LOCALHOST, 0).into()).unwrap()
}

/// A quinn server endpoint whose UDP socket de-obfuscates/obfuscates with the
/// independent Salamander codec.
fn obfs_server_endpoint(psk: &[u8]) -> Endpoint {
    let runtime = quinn::default_runtime().expect("tokio runtime");
    let socket = std::net::UdpSocket::bind((Ipv4Addr::LOCALHOST, 0)).unwrap();
    let inner = runtime.wrap_udp_socket(socket).unwrap();
    let wrapped = Arc::new(ServerObfsSocket {
        inner,
        psk: psk.to_vec(),
        salt_counter: std::sync::atomic::AtomicU64::new(1),
    });
    Endpoint::new_with_abstract_socket(EndpointConfig::default(), Some(server_config()), wrapped, runtime).unwrap()
}

/// Read a QUIC variable-length integer from a quinn recv stream.
async fn read_varint(recv: &mut quinn::RecvStream) -> u64 {
    let mut first = [0u8; 1];
    recv.read_exact(&mut first).await.unwrap();
    let len = 1usize << (first[0] >> 6);
    let mut value = (first[0] & 0x3f) as u64;
    let mut rest = [0u8; 7];
    recv.read_exact(&mut rest[..len - 1]).await.unwrap();
    for &b in &rest[..len - 1] {
        value = (value << 8) | b as u64;
    }
    value
}

/// Read a varint-length-prefixed byte string from a quinn recv stream.
async fn read_varint_bytes(recv: &mut quinn::RecvStream) -> Vec<u8> {
    let len = read_varint(recv).await as usize;
    let mut buf = vec![0u8; len];
    recv.read_exact(&mut buf).await.unwrap();
    buf
}

/// Run the fake Hysteria2 server: authenticate over HTTP/3, parse the
/// `TCPRequest` target, answer OK, and echo the relayed payload.
async fn run_server(endpoint: Endpoint, target_tx: oneshot::Sender<String>) {
    let conn = endpoint.accept().await.unwrap().await.unwrap();
    let proxy_conn = conn.clone();

    let mut h3_conn = h3::server::Connection::<_, Bytes>::new(h3_quinn::Connection::new(conn))
        .await
        .unwrap();
    let resolver = h3_conn.accept().await.unwrap().expect("auth request");
    let (request, mut stream) = resolver.resolve_request().await.unwrap();
    assert_eq!(request.uri().path(), "/auth", "auth path");
    assert_eq!(
        request.headers().get("hysteria-auth").map(|v| v.as_bytes()),
        Some(PASSWORD.as_bytes()),
        "auth password header"
    );
    let response = http::Response::builder().status(233).body(()).unwrap();
    stream.send_response(response).await.unwrap();
    stream.finish().await.unwrap();

    let (mut send, mut recv) = proxy_conn.accept_bi().await.unwrap();
    assert_eq!(read_varint(&mut recv).await, 0x401, "TCPRequest frame id");
    let address = String::from_utf8(read_varint_bytes(&mut recv).await).unwrap();
    target_tx.send(address).unwrap();
    let _padding = read_varint_bytes(&mut recv).await;

    send.write_all(&[0x00, 0x00, 0x00]).await.unwrap();

    let mut payload = vec![0u8; MESSAGE.len()];
    recv.read_exact(&mut payload).await.unwrap();
    send.write_all(&payload).await.unwrap();
    send.finish().unwrap();

    proxy_conn.closed().await;
    drop(h3_conn);
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

/// Relay `MESSAGE` through the kernel using `config` and assert it echoes back
/// and the server parsed the expected target.
async fn relay_and_assert(config: Hysteria2OutboundConfig) {
    let dummy_target = SocketAddr::from((Ipv4Addr::new(93, 184, 216, 34), 443));
    let handle = GripeKernel::start(GripeConfig {
        socks_listen: SocketAddr::from((Ipv4Addr::LOCALHOST, 0)),
        outbound: OutboundMode::Hysteria2(Box::new(config)),
    })
    .await
    .unwrap();

    let mut conn = socks5_connect(handle.local_addr(), dummy_target).await;
    conn.write_all(MESSAGE).await.unwrap();
    conn.flush().await.unwrap();

    let mut echo = vec![0u8; MESSAGE.len()];
    conn.read_exact(&mut echo).await.unwrap();
    assert_eq!(echo, MESSAGE, "payload relayed and echoed verbatim");
    drop(conn);
}

fn base_config(port: u16) -> Hysteria2OutboundConfig {
    Hysteria2OutboundConfig {
        server: "127.0.0.1".to_string(),
        port,
        password: PASSWORD.to_string(),
        server_name: "example.com".to_string(),
        alpn: vec!["h3".to_string()],
        skip_cert_verify: true,
        congestion: Congestion::Bbr,
        obfs: None,
        port_hop: None,
        reduce_rtt: false,
    }
}

#[tokio::test]
async fn relays_through_salamander_obfs() {
    let endpoint = obfs_server_endpoint(OBFS_PASSWORD.as_bytes());
    let server_addr = endpoint.local_addr().unwrap();
    let (target_tx, target_rx) = oneshot::channel();
    let server = tokio::spawn(run_server(endpoint, target_tx));

    let mut config = base_config(server_addr.port());
    config.obfs = Some(Salamander::new(OBFS_PASSWORD.as_bytes().to_vec()));
    relay_and_assert(config).await;

    assert_eq!(target_rx.await.unwrap(), "93.184.216.34:443");
    server.await.unwrap();
}

#[tokio::test]
async fn relays_through_port_hopping() {
    // Plain server on port P; the client's canonical port is a *different* value
    // and its only hop port is P, so both the send rewrite (Q->P) and the recv
    // normalization (P->Q) must work for the handshake to complete.
    let endpoint = plain_server_endpoint();
    let real_port = endpoint.local_addr().unwrap().port();
    let canonical_port = real_port.wrapping_add(1).max(1);
    let (target_tx, target_rx) = oneshot::channel();
    let server = tokio::spawn(run_server(endpoint, target_tx));

    let mut config = base_config(canonical_port);
    config.port_hop = Some(PortHopConfig::parse(&real_port.to_string(), None).unwrap());
    relay_and_assert(config).await;

    assert_eq!(target_rx.await.unwrap(), "93.184.216.34:443");
    server.await.unwrap();
}

#[tokio::test]
async fn relays_through_obfs_and_port_hopping() {
    let endpoint = obfs_server_endpoint(OBFS_PASSWORD.as_bytes());
    let real_port = endpoint.local_addr().unwrap().port();
    let canonical_port = real_port.wrapping_add(1).max(1);
    let (target_tx, target_rx) = oneshot::channel();
    let server = tokio::spawn(run_server(endpoint, target_tx));

    let mut config = base_config(canonical_port);
    config.obfs = Some(Salamander::new(OBFS_PASSWORD.as_bytes().to_vec()));
    config.port_hop = Some(PortHopConfig::parse(&real_port.to_string(), None).unwrap());
    relay_and_assert(config).await;

    assert_eq!(target_rx.await.unwrap(), "93.184.216.34:443");
    server.await.unwrap();
}
