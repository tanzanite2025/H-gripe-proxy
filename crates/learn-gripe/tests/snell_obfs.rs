//! End-to-end proof that a Snell outbound carries both TCP and UDP traffic when
//! the shadowaead stream is wrapped in simple-obfs (`obfs-opts`):
//! SOCKS5 client -> gripe inbound -> Snell (over obfs) -> fake obfs server ->
//! fake Snell server.
//!
//! simple-obfs sits *beneath* the Snell AEAD layer, so the same one-shot obfs
//! framing applies to TCP and to UDP-over-TCP. The fake servers below are an
//! *independent* re-implementation of both the simple-obfs framings (http: a
//! fake WebSocket upgrade; tls: a fake TLS 1.2 handshake + application-data
//! records) and the Snell wire format (16-byte salt + Argon2id subkey +
//! `AEAD(len)|AEAD(payload)` chunks). Each fake server strips the obfs framing,
//! then hands the bare byte stream to the Snell server (v3 / AES-128-GCM).
//!
//! We cover the http and tls obfs modes over both a TCP relay and a UDP
//! association, plus a payload larger than one obfs record / AEAD chunk.

use std::io;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::pin::Pin;
use std::task::{Context as TaskContext, Poll, ready};

use aes_gcm::Aes128Gcm;
use aes_gcm::aead::generic_array::GenericArray;
use aes_gcm::aead::{Aead, KeyInit, Payload};
use argon2::{Algorithm, Argon2, Params, Version};
use learn_gripe::{GripeConfig, GripeKernel, OutboundMode, SnellObfs, SnellOutboundConfig};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt, ReadBuf};
use tokio::net::{TcpListener, TcpStream, UdpSocket};

const TEST_PSK: &[u8] = b"snell-obfs-test-psk";
const SALT_LEN: usize = 16;
const TAG_LEN: usize = 16;
const KEY_SIZE: usize = 16; // v3 => AES-128-GCM
const COMMAND_UDP: u8 = 6;
const UDP_FORWARD: u8 = 1;
const RESP_TUNNEL: u8 = 0;

// ---------------------------------------------------------------------------
// Independent Snell crypto (v3 / AES-128-GCM)
// ---------------------------------------------------------------------------

fn snell_kdf(psk: &[u8], salt: &[u8]) -> Vec<u8> {
    let params = Params::new(8, 3, 1, Some(32)).unwrap();
    let argon2 = Argon2::new(Algorithm::Argon2id, Version::V0x13, params);
    let mut out = [0u8; 32];
    argon2.hash_password_into(psk, salt, &mut out).unwrap();
    out[..KEY_SIZE].to_vec()
}

fn increment_nonce(nonce: &mut [u8; 12]) {
    for byte in nonce.iter_mut() {
        *byte = byte.wrapping_add(1);
        if *byte != 0 {
            break;
        }
    }
}

fn cipher(subkey: &[u8]) -> Aes128Gcm {
    Aes128Gcm::new_from_slice(subkey).unwrap()
}

async fn read_chunk<S>(stream: &mut S, cipher: &Aes128Gcm, nonce: &mut [u8; 12]) -> Option<Vec<u8>>
where
    S: AsyncRead + Unpin,
{
    let mut sealed_len = [0u8; 2 + TAG_LEN];
    if stream.read_exact(&mut sealed_len).await.is_err() {
        return None;
    }
    let len_plain = cipher
        .decrypt(
            GenericArray::from_slice(nonce),
            Payload {
                msg: &sealed_len,
                aad: &[],
            },
        )
        .expect("decrypt chunk length");
    increment_nonce(nonce);
    let clen = u16::from_be_bytes([len_plain[0], len_plain[1]]) as usize;

    let mut sealed = vec![0u8; clen + TAG_LEN];
    stream.read_exact(&mut sealed).await.expect("read chunk body");
    let plain = cipher
        .decrypt(GenericArray::from_slice(nonce), Payload { msg: &sealed, aad: &[] })
        .expect("decrypt chunk body");
    increment_nonce(nonce);
    Some(plain)
}

async fn write_chunk<S>(stream: &mut S, cipher: &Aes128Gcm, nonce: &mut [u8; 12], plaintext: &[u8])
where
    S: AsyncWrite + Unpin,
{
    let len = (plaintext.len() as u16).to_be_bytes();
    let sealed_len = cipher
        .encrypt(GenericArray::from_slice(nonce), Payload { msg: &len, aad: &[] })
        .unwrap();
    increment_nonce(nonce);
    let sealed = cipher
        .encrypt(
            GenericArray::from_slice(nonce),
            Payload {
                msg: plaintext,
                aad: &[],
            },
        )
        .unwrap();
    increment_nonce(nonce);
    stream.write_all(&sealed_len).await.unwrap();
    stream.write_all(&sealed).await.unwrap();
}

fn parse_client_packet(plain: &[u8]) -> Vec<u8> {
    assert_eq!(plain[0], UDP_FORWARD, "client UDP forward command");
    let rest = &plain[1..];
    let addr_len = match rest[0] {
        0x00 => match rest[1] {
            4 => 2 + 4 + 2,
            6 => 2 + 16 + 2,
            other => panic!("unknown snell IP family {other}"),
        },
        host_len => 1 + host_len as usize + 2,
    };
    rest[addr_len..].to_vec()
}

fn reply_packet(payload: &[u8]) -> Vec<u8> {
    let mut out = vec![4u8];
    out.extend_from_slice(&Ipv4Addr::new(1, 2, 3, 4).octets());
    out.extend_from_slice(&443u16.to_be_bytes());
    out.extend_from_slice(payload);
    out
}

/// Send our salt and derive the write cipher used for replies.
async fn send_server_salt<S>(stream: &mut S, mix: u8) -> (Aes128Gcm, [u8; 12])
where
    S: AsyncWrite + Unpin,
{
    let mut salt_w = [0u8; SALT_LEN];
    for (i, b) in salt_w.iter_mut().enumerate() {
        *b = (i as u8).wrapping_mul(11).wrapping_add(mix);
    }
    stream.write_all(&salt_w).await.unwrap();
    (cipher(&snell_kdf(TEST_PSK, &salt_w)), [0u8; 12])
}

/// Whether the fake Snell server serves a TCP relay (echo) or a UDP association.
#[derive(Clone, Copy)]
enum Role {
    Tcp,
    Udp,
}

/// Read the client salt and the first handshake chunk, then echo the stream
/// according to `role`. Runs over any obfs-unwrapped byte stream.
async fn serve_snell<S>(mut stream: S, role: Role)
where
    S: AsyncRead + AsyncWrite + Unpin,
{
    let mut salt = [0u8; SALT_LEN];
    stream.read_exact(&mut salt).await.unwrap();
    let read_cipher = cipher(&snell_kdf(TEST_PSK, &salt));
    let mut read_nonce = [0u8; 12];

    let header = read_chunk(&mut stream, &read_cipher, &mut read_nonce)
        .await
        .expect("handshake header");
    assert_eq!(header[0], 1, "snell proto byte");

    match role {
        Role::Tcp => {
            assert!(header[1] == 1 || header[1] == 5, "connect command");
            let (write_cipher, mut write_nonce) = send_server_salt(&mut stream, 3).await;
            write_chunk(&mut stream, &write_cipher, &mut write_nonce, &[RESP_TUNNEL]).await;
            while let Some(data) = read_chunk(&mut stream, &read_cipher, &mut read_nonce).await {
                write_chunk(&mut stream, &write_cipher, &mut write_nonce, &data).await;
            }
        }
        Role::Udp => {
            assert_eq!(header, [1, COMMAND_UDP, 0], "snell udp handshake header");
            let (write_cipher, mut write_nonce) = send_server_salt(&mut stream, 3).await;
            while let Some(packet) = read_chunk(&mut stream, &read_cipher, &mut read_nonce).await {
                let payload = parse_client_packet(&packet);
                write_chunk(&mut stream, &write_cipher, &mut write_nonce, &reply_packet(&payload)).await;
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Independent simple-obfs HTTP fake server (fake WebSocket upgrade)
// ---------------------------------------------------------------------------

async fn serve_obfs_http(mut tcp: TcpStream, role: Role) {
    let mut buf: Vec<u8> = Vec::new();
    let mut tmp = [0u8; 1024];
    let leftover = loop {
        let n = match tcp.read(&mut tmp).await {
            Ok(0) | Err(_) => return,
            Ok(n) => n,
        };
        buf.extend_from_slice(&tmp[..n]);
        if let Some(pos) = buf.windows(4).position(|w| w == b"\r\n\r\n") {
            break buf.split_off(pos + 4);
        }
        if buf.len() > 8192 {
            return;
        }
    };
    assert!(buf.starts_with(b"GET "), "obfs http request should be a GET");
    assert!(
        buf.windows(7).any(|w| w.eq_ignore_ascii_case(b"\r\nHost:")),
        "obfs http request should carry a Host header"
    );

    let response = "HTTP/1.1 101 Switching Protocols\r\n\
         Server: nginx/1.24.0\r\n\
         Upgrade: websocket\r\n\
         Connection: Upgrade\r\n\
         Sec-WebSocket-Accept: s3pPLMBiTxaQ9kYGzzhZRbK+xOo=\r\n\
         \r\n";
    if tcp.write_all(response.as_bytes()).await.is_err() {
        return;
    }

    let (rd, wr) = tcp.into_split();
    let reader = std::io::Cursor::new(leftover).chain(rd);
    serve_snell(tokio::io::join(reader, wr), role).await;
}

// ---------------------------------------------------------------------------
// Independent simple-obfs TLS fake server (fake TLS 1.2)
// ---------------------------------------------------------------------------

const CHUNK_SIZE: usize = 1 << 14;
const APP_DATA_HEADER: [u8; 3] = [0x17, 0x03, 0x03];
const EXTENSIONS_LEN_OFFSET: usize = 108;

async fn read_client_hello(tcp: &mut TcpStream) -> Option<(Vec<u8>, Vec<u8>)> {
    let mut buf = Vec::new();
    let mut tmp = [0u8; 2048];

    while buf.len() < EXTENSIONS_LEN_OFFSET + 2 {
        let n = tcp.read(&mut tmp).await.ok()?;
        if n == 0 {
            return None;
        }
        buf.extend_from_slice(&tmp[..n]);
    }
    let ext_len = u16::from_be_bytes([buf[EXTENSIONS_LEN_OFFSET], buf[EXTENSIONS_LEN_OFFSET + 1]]) as usize;
    let ch_end = EXTENSIONS_LEN_OFFSET + 2 + ext_len;

    while buf.len() < ch_end {
        let n = tcp.read(&mut tmp).await.ok()?;
        if n == 0 {
            return None;
        }
        buf.extend_from_slice(&tmp[..n]);
    }

    let mut embedded = None;
    let mut i = EXTENSIONS_LEN_OFFSET + 2;
    while i + 4 <= ch_end {
        let etype = u16::from_be_bytes([buf[i], buf[i + 1]]);
        let elen = u16::from_be_bytes([buf[i + 2], buf[i + 3]]) as usize;
        let data_start = i + 4;
        if data_start + elen > ch_end {
            break;
        }
        if etype == 0x0023 {
            embedded = Some(buf[data_start..data_start + elen].to_vec());
        }
        i = data_start + elen;
    }

    let leftover = buf[ch_end..].to_vec();
    Some((embedded?, leftover))
}

async fn serve_obfs_tls(mut tcp: TcpStream, role: Role) {
    let Some((embedded, leftover)) = read_client_hello(&mut tcp).await else {
        return;
    };
    serve_snell(ObfsTlsServerStream::new(tcp, embedded, leftover), role).await;
}

/// Server-side simple-obfs TLS adapter presenting the de-framed byte stream:
/// reads strip TLS application-data record headers (seeded with the
/// `ClientHello` payload); the first write emits the fixed fake handshake.
struct ObfsTlsServerStream {
    inner: TcpStream,
    first_response: bool,
    write_buf: Vec<u8>,
    write_off: usize,
    raw: Vec<u8>,
    record_remaining: usize,
    out: Vec<u8>,
    out_pos: usize,
    saw_eof: bool,
}

impl ObfsTlsServerStream {
    fn new(inner: TcpStream, embedded: Vec<u8>, leftover: Vec<u8>) -> Self {
        Self {
            inner,
            first_response: true,
            write_buf: Vec::new(),
            write_off: 0,
            raw: leftover,
            record_remaining: 0,
            out: embedded,
            out_pos: 0,
            saw_eof: false,
        }
    }

    fn decode_step(&mut self) -> bool {
        if self.record_remaining > 0 {
            if self.raw.is_empty() {
                return false;
            }
            let take = self.record_remaining.min(self.raw.len());
            self.out.extend_from_slice(&self.raw[..take]);
            self.raw.drain(..take);
            self.record_remaining -= take;
            return true;
        }
        if self.raw.len() < 5 {
            return false;
        }
        let len = u16::from_be_bytes([self.raw[3], self.raw[4]]) as usize;
        self.raw.drain(..5);
        self.record_remaining = len;
        true
    }

    fn poll_flush_write_buf(&mut self, cx: &mut TaskContext<'_>) -> Poll<io::Result<()>> {
        while self.write_off < self.write_buf.len() {
            let n = ready!(Pin::new(&mut self.inner).poll_write(cx, &self.write_buf[self.write_off..]))?;
            if n == 0 {
                return Poll::Ready(Err(io::Error::new(io::ErrorKind::WriteZero, "wrote zero")));
            }
            self.write_off += n;
        }
        self.write_buf.clear();
        self.write_off = 0;
        Poll::Ready(Ok(()))
    }
}

/// The fixed 105-byte fake handshake the client skips wholesale.
fn server_handshake() -> Vec<u8> {
    let mut out = Vec::with_capacity(105);
    out.extend_from_slice(&[0x16, 0x03, 0x03, 0x00, 0x4a]);
    out.extend_from_slice(&[0x02, 0x00, 0x00, 0x46, 0x03, 0x03]);
    out.extend_from_slice(&[0u8; 32]);
    out.push(0x20);
    out.extend_from_slice(&[0u8; 32]);
    out.extend_from_slice(&[0xc0, 0x2f]);
    out.push(0x00);
    out.extend_from_slice(&[0x14, 0x03, 0x03, 0x00, 0x01, 0x01]);
    out.extend_from_slice(&[0x16, 0x03, 0x03, 0x00, 0x0f]);
    out.extend_from_slice(&[0u8; 15]);
    debug_assert_eq!(out.len(), 105);
    out
}

fn encode_records(data: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(data.len() + (data.len() / CHUNK_SIZE + 1) * 5);
    for chunk in data.chunks(CHUNK_SIZE) {
        out.extend_from_slice(&APP_DATA_HEADER);
        out.extend_from_slice(&(chunk.len() as u16).to_be_bytes());
        out.extend_from_slice(chunk);
    }
    out
}

impl AsyncRead for ObfsTlsServerStream {
    fn poll_read(self: Pin<&mut Self>, cx: &mut TaskContext<'_>, buf: &mut ReadBuf<'_>) -> Poll<io::Result<()>> {
        let this = self.get_mut();
        loop {
            if this.out_pos < this.out.len() {
                let n = buf.remaining().min(this.out.len() - this.out_pos);
                buf.put_slice(&this.out[this.out_pos..this.out_pos + n]);
                this.out_pos += n;
                if this.out_pos == this.out.len() {
                    this.out.clear();
                    this.out_pos = 0;
                }
                return Poll::Ready(Ok(()));
            }
            this.out.clear();
            this.out_pos = 0;

            if this.decode_step() {
                continue;
            }
            if this.saw_eof {
                return Poll::Ready(Ok(()));
            }

            let mut tmp = [0u8; 8192];
            let mut rb = ReadBuf::new(&mut tmp);
            ready!(Pin::new(&mut this.inner).poll_read(cx, &mut rb))?;
            let filled = rb.filled();
            if filled.is_empty() {
                this.saw_eof = true;
            } else {
                this.raw.extend_from_slice(filled);
            }
        }
    }
}

impl AsyncWrite for ObfsTlsServerStream {
    fn poll_write(self: Pin<&mut Self>, cx: &mut TaskContext<'_>, buf: &[u8]) -> Poll<io::Result<usize>> {
        let this = self.get_mut();
        ready!(this.poll_flush_write_buf(cx))?;
        if buf.is_empty() {
            return Poll::Ready(Ok(0));
        }

        let mut encoded = Vec::new();
        if this.first_response {
            encoded.extend_from_slice(&server_handshake());
            this.first_response = false;
        }
        encoded.extend_from_slice(&encode_records(buf));
        this.write_buf = encoded;
        this.write_off = 0;

        match this.poll_flush_write_buf(cx) {
            Poll::Ready(Err(e)) => Poll::Ready(Err(e)),
            Poll::Ready(Ok(())) | Poll::Pending => Poll::Ready(Ok(buf.len())),
        }
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut TaskContext<'_>) -> Poll<io::Result<()>> {
        let this = self.get_mut();
        ready!(this.poll_flush_write_buf(cx))?;
        Pin::new(&mut this.inner).poll_flush(cx)
    }

    fn poll_shutdown(self: Pin<&mut Self>, cx: &mut TaskContext<'_>) -> Poll<io::Result<()>> {
        let this = self.get_mut();
        ready!(this.poll_flush_write_buf(cx))?;
        Pin::new(&mut this.inner).poll_shutdown(cx)
    }
}

// ---------------------------------------------------------------------------
// Spawners + config
// ---------------------------------------------------------------------------

/// Which obfs framing the fake server (and the kernel config) should use.
#[derive(Clone, Copy)]
enum ObfsMode {
    Http,
    Tls,
}

async fn spawn_obfs_snell(mode: ObfsMode, role: Role) -> SocketAddr {
    let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        while let Ok((tcp, _)) = listener.accept().await {
            match mode {
                ObfsMode::Http => tokio::spawn(serve_obfs_http(tcp, role)),
                ObfsMode::Tls => tokio::spawn(serve_obfs_tls(tcp, role)),
            };
        }
    });
    addr
}

fn snell(server: SocketAddr, mode: ObfsMode) -> Box<SnellOutboundConfig> {
    let obfs = match mode {
        ObfsMode::Http => SnellObfs::Http {
            host: "www.bing.com".to_string(),
            path: "/".to_string(),
        },
        ObfsMode::Tls => SnellObfs::Tls {
            host: "www.bing.com".to_string(),
        },
    };
    Box::new(SnellOutboundConfig {
        server: server.ip().to_string(),
        port: server.port(),
        psk: TEST_PSK.to_vec(),
        version: 3,
        obfs: Some(obfs),
        reuse: false,
    })
}

// ---------------------------------------------------------------------------
// TCP harness
// ---------------------------------------------------------------------------

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
    assert_eq!(reply[1], 0x00, "SOCKS5 reply should be success");
    stream
}

async fn assert_tcp_relays(mode: ObfsMode, payload: &[u8]) {
    let server = spawn_obfs_snell(mode, Role::Tcp).await;
    let handle = GripeKernel::start(GripeConfig {
        socks_listen: SocketAddr::from((Ipv4Addr::LOCALHOST, 0)),
        outbound: OutboundMode::Snell(snell(server, mode)),
    })
    .await
    .unwrap();

    let target = SocketAddr::from((Ipv4Addr::new(1, 2, 3, 4), 443));
    let mut conn = socks5_connect(handle.local_addr(), target).await;
    conn.write_all(payload).await.unwrap();
    let mut buf = vec![0u8; payload.len()];
    conn.read_exact(&mut buf).await.unwrap();
    assert_eq!(buf, payload);

    handle.shutdown().await;
}

// ---------------------------------------------------------------------------
// UDP harness
// ---------------------------------------------------------------------------

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
        IpAddr::V6(_) => panic!("ipv4 helper"),
    };
    let mut datagram = vec![0x00, 0x00, 0x00, 0x01];
    datagram.extend_from_slice(&ip);
    datagram.extend_from_slice(&dst.port().to_be_bytes());
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

async fn assert_udp_relays(mode: ObfsMode, payload: &[u8]) {
    let server = spawn_obfs_snell(mode, Role::Udp).await;
    let handle = GripeKernel::start(GripeConfig {
        socks_listen: SocketAddr::from((Ipv4Addr::LOCALHOST, 0)),
        outbound: OutboundMode::Snell(snell(server, mode)),
    })
    .await
    .unwrap();

    let (_control, relay) = socks5_udp_associate(handle.local_addr()).await;
    let client = UdpSocket::bind((Ipv4Addr::LOCALHOST, 0)).await.unwrap();
    let dst = SocketAddr::from((Ipv4Addr::new(1, 2, 3, 4), 443));
    client.send_to(&udp_datagram_ipv4(dst, payload), relay).await.unwrap();

    let mut buf = vec![0u8; payload.len() + 64];
    let (n, from) = client.recv_from(&mut buf).await.unwrap();
    assert_eq!(from, relay, "reply must come from the relay socket");
    let offset = payload_offset(&buf[..n]);
    assert_eq!(&buf[offset..n], payload, "payload must be echoed verbatim");

    handle.shutdown().await;
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[tokio::test]
async fn tcp_relays_over_obfs_http() {
    assert_tcp_relays(ObfsMode::Http, b"hello snell over obfs http").await;
}

#[tokio::test]
async fn tcp_relays_over_obfs_tls() {
    assert_tcp_relays(ObfsMode::Tls, b"hello snell over obfs tls").await;
}

#[tokio::test]
async fn tcp_relays_large_payload_over_obfs_tls() {
    // Larger than one 16 KiB TLS record / 0x3FFF AEAD chunk to exercise both
    // the record loop and the chunk loop over the obfs transport.
    let payload: Vec<u8> = (0..50_000u32).map(|i| (i % 251) as u8).collect();
    assert_tcp_relays(ObfsMode::Tls, &payload).await;
}

#[tokio::test]
async fn udp_relays_over_obfs_http() {
    assert_udp_relays(ObfsMode::Http, b"snell udp over obfs http").await;
}

#[tokio::test]
async fn udp_relays_over_obfs_tls() {
    assert_udp_relays(ObfsMode::Tls, b"snell udp over obfs tls").await;
}
