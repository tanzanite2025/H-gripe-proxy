//! End-to-end proof that traffic flows through a Shadowsocks outbound wrapped
//! in a SIP003 plugin transport: a SOCKS5 client -> gripe inbound -> Shadowsocks
//! outbound (over the plugin) -> fake plugin server -> fake SS server.
//!
//! The fake server is an *independent* implementation of both layers: it first
//! strips the plugin framing (simple-obfs HTTP upgrade, or the v2ray-plugin
//! WebSocket/TLS handshake), then runs an ordinary Shadowsocks AEAD server over
//! the unwrapped byte stream (`EVP_BytesToKey` master key, HKDF-SHA1 subkey,
//! length-prefixed AEAD chunks, SOCKS5 target address at the head, echo back).
//!
//! Because the Shadowsocks layer is identical to the plain-outbound test, the
//! point here is the plugin transport: proving the SS stream composes correctly
//! over simple-obfs and v2ray-plugin exactly as it does over a raw socket.

use std::io;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context as TaskContext, Poll, ready};

use aes_gcm::Aes256Gcm;
use aes_gcm::aead::generic_array::GenericArray;
use aes_gcm::aead::{Aead, KeyInit, Payload};
use futures_util::{Sink, Stream};
use learn_gripe::{GripeConfig, GripeKernel, OutboundMode, ProxyEntry, ShadowsocksOutboundConfig};
use md5::Md5;
use sha1::{Digest, Sha1};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt, ReadBuf};
use tokio::net::{TcpListener, TcpStream};
use tokio_rustls::TlsAcceptor;
use tokio_tungstenite::WebSocketStream;
use tokio_tungstenite::tungstenite::Message;

const PASSWORD: &str = "correct horse battery staple";
const SS_SUBKEY_INFO: &[u8] = b"ss-subkey";
const TAG_LEN: usize = 16;
const KEY_SIZE: usize = 32; // aes-256-gcm

const TEST_CERT: &str = include_str!("data/vless_tls_cert.pem");
const TEST_KEY: &str = include_str!("data/vless_tls_key.pem");

// --- minimal independent Shadowsocks crypto for the fake server -----------

fn evp_bytes_to_key(password: &[u8], key_len: usize) -> Vec<u8> {
    let mut key = Vec::with_capacity(key_len);
    let mut prev: Vec<u8> = Vec::new();
    while key.len() < key_len {
        let mut hasher = Md5::new();
        hasher.update(&prev);
        hasher.update(password);
        let digest: [u8; 16] = hasher.finalize().into();
        key.extend_from_slice(&digest);
        prev = digest.to_vec();
    }
    key.truncate(key_len);
    key
}

fn sha1(parts: &[&[u8]]) -> [u8; 20] {
    let mut hasher = Sha1::new();
    for part in parts {
        hasher.update(part);
    }
    hasher.finalize().into()
}

fn hmac_sha1(key: &[u8], msg: &[u8]) -> [u8; 20] {
    const BLOCK: usize = 64;
    let mut block = [0u8; BLOCK];
    if key.len() > BLOCK {
        block[..20].copy_from_slice(&sha1(&[key]));
    } else {
        block[..key.len()].copy_from_slice(key);
    }
    let mut ipad = [0u8; BLOCK];
    let mut opad = [0u8; BLOCK];
    for i in 0..BLOCK {
        ipad[i] = block[i] ^ 0x36;
        opad[i] = block[i] ^ 0x5c;
    }
    let inner = sha1(&[&ipad, msg]);
    sha1(&[&opad, &inner])
}

fn hkdf_sha1(ikm: &[u8], salt: &[u8], info: &[u8], length: usize) -> Vec<u8> {
    let prk = hmac_sha1(salt, ikm);
    let mut okm = Vec::with_capacity(length);
    let mut prev: Vec<u8> = Vec::new();
    let mut counter: u8 = 1;
    while okm.len() < length {
        let mut input = Vec::new();
        input.extend_from_slice(&prev);
        input.extend_from_slice(info);
        input.push(counter);
        let block = hmac_sha1(&prk, &input);
        okm.extend_from_slice(&block);
        prev = block.to_vec();
        counter += 1;
    }
    okm.truncate(length);
    okm
}

fn increment_nonce(nonce: &mut [u8; 12]) {
    for byte in nonce.iter_mut() {
        *byte = byte.wrapping_add(1);
        if *byte != 0 {
            break;
        }
    }
}

fn seal(cipher: &Aes256Gcm, nonce: &[u8; 12], pt: &[u8]) -> Vec<u8> {
    cipher
        .encrypt(GenericArray::from_slice(nonce), Payload { msg: pt, aad: &[] })
        .unwrap()
}

fn open(cipher: &Aes256Gcm, nonce: &[u8; 12], ct: &[u8]) -> Vec<u8> {
    cipher
        .decrypt(GenericArray::from_slice(nonce), Payload { msg: ct, aad: &[] })
        .unwrap()
}

/// Read and decrypt one AEAD chunk; returns its plaintext, or `None` at EOF.
async fn read_chunk<S>(stream: &mut S, cipher: &Aes256Gcm, nonce: &mut [u8; 12]) -> Option<Vec<u8>>
where
    S: AsyncRead + Unpin,
{
    let mut sealed_len = [0u8; 2 + TAG_LEN];
    stream.read_exact(&mut sealed_len).await.ok()?;
    let len_pt = open(cipher, nonce, &sealed_len);
    increment_nonce(nonce);
    let clen = u16::from_be_bytes([len_pt[0], len_pt[1]]) as usize;
    let mut sealed = vec![0u8; clen + TAG_LEN];
    stream.read_exact(&mut sealed).await.ok()?;
    let pt = open(cipher, nonce, &sealed);
    increment_nonce(nonce);
    Some(pt)
}

/// Length of the SOCKS5 address at the head of `buf`, once enough bytes are
/// present; `None` if more bytes are still needed.
fn address_len(buf: &[u8]) -> Option<usize> {
    match buf.first()? {
        0x01 => (buf.len() >= 7).then_some(7),
        0x04 => (buf.len() >= 19).then_some(19),
        0x03 => {
            let host_len = *buf.get(1)? as usize;
            let total = 2 + host_len + 2;
            (buf.len() >= total).then_some(total)
        }
        other => panic!("unexpected SOCKS5 atyp {other:#x}"),
    }
}

/// Run the Shadowsocks AEAD server (aes-256-gcm) over an already-unwrapped byte
/// stream: read the client salt + address, then echo application bytes back.
async fn serve_shadowsocks<S>(mut stream: S)
where
    S: AsyncRead + AsyncWrite + Unpin,
{
    let master = evp_bytes_to_key(PASSWORD.as_bytes(), KEY_SIZE);

    let mut salt = vec![0u8; KEY_SIZE];
    if stream.read_exact(&mut salt).await.is_err() {
        return;
    }
    let read_subkey = hkdf_sha1(&master, &salt, SS_SUBKEY_INFO, KEY_SIZE);
    let read_cipher = Aes256Gcm::new_from_slice(&read_subkey).unwrap();
    let mut read_nonce = [0u8; 12];

    let mut resp_salt = vec![0u8; KEY_SIZE];
    for (i, b) in resp_salt.iter_mut().enumerate() {
        *b = (i as u8).wrapping_mul(7).wrapping_add(1);
    }
    if stream.write_all(&resp_salt).await.is_err() {
        return;
    }
    let write_subkey = hkdf_sha1(&master, &resp_salt, SS_SUBKEY_INFO, KEY_SIZE);
    let write_cipher = Aes256Gcm::new_from_slice(&write_subkey).unwrap();
    let mut write_nonce = [0u8; 12];

    let mut head: Vec<u8> = Vec::new();
    let mut address_consumed = false;

    while let Some(plain) = read_chunk(&mut stream, &read_cipher, &mut read_nonce).await {
        let mut data = plain;
        if !address_consumed {
            head.extend_from_slice(&data);
            match address_len(&head) {
                Some(n) => {
                    address_consumed = true;
                    data = head.split_off(n);
                }
                None => continue,
            }
        }
        if data.is_empty() {
            continue;
        }
        let sealed_len = seal(&write_cipher, &write_nonce, &(data.len() as u16).to_be_bytes());
        increment_nonce(&mut write_nonce);
        let sealed_payload = seal(&write_cipher, &write_nonce, &data);
        increment_nonce(&mut write_nonce);
        if stream.write_all(&sealed_len).await.is_err()
            || stream.write_all(&sealed_payload).await.is_err()
            || stream.flush().await.is_err()
        {
            return;
        }
    }
}

// --- simple-obfs HTTP fake server -----------------------------------------

/// Strip the simple-obfs HTTP request header, reply with `101 Switching
/// Protocols`, then run the Shadowsocks server over the unwrapped stream. Any
/// bytes that arrived after the `\r\n\r\n` terminator are the start of the SS
/// stream and are prepended back before serving.
async fn serve_obfs_http(mut tcp: TcpStream) {
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
    serve_shadowsocks(tokio::io::join(reader, wr)).await;
}

// --- v2ray-plugin (WebSocket) fake server ---------------------------------

/// Server-side byte-stream view of a `WebSocketStream`, so the SS server can run
/// unchanged over the WebSocket frames.
struct WsServerStream<S> {
    ws: WebSocketStream<S>,
    read_buf: Vec<u8>,
    read_pos: usize,
    flushing: bool,
}

impl<S> WsServerStream<S> {
    fn new(ws: WebSocketStream<S>) -> Self {
        Self {
            ws,
            read_buf: Vec::new(),
            read_pos: 0,
            flushing: false,
        }
    }
}

fn ws_io_err<E: std::fmt::Display>(e: E) -> io::Error {
    io::Error::other(e.to_string())
}

impl<S: AsyncRead + AsyncWrite + Unpin> AsyncRead for WsServerStream<S> {
    fn poll_read(self: Pin<&mut Self>, cx: &mut TaskContext<'_>, buf: &mut ReadBuf<'_>) -> Poll<io::Result<()>> {
        let this = self.get_mut();
        loop {
            if this.read_pos < this.read_buf.len() {
                let remaining = &this.read_buf[this.read_pos..];
                let n = remaining.len().min(buf.remaining());
                buf.put_slice(&remaining[..n]);
                this.read_pos += n;
                return Poll::Ready(Ok(()));
            }
            match ready!(Pin::new(&mut this.ws).poll_next(cx)) {
                Some(Ok(Message::Binary(data))) => {
                    this.read_buf = data.into();
                    this.read_pos = 0;
                }
                Some(Ok(Message::Text(text))) => {
                    this.read_buf = text.as_bytes().to_vec();
                    this.read_pos = 0;
                }
                Some(Ok(Message::Ping(_) | Message::Pong(_) | Message::Frame(_))) => {}
                Some(Ok(Message::Close(_))) | None => return Poll::Ready(Ok(())),
                Some(Err(e)) => return Poll::Ready(Err(ws_io_err(e))),
            }
        }
    }
}

impl<S: AsyncRead + AsyncWrite + Unpin> AsyncWrite for WsServerStream<S> {
    fn poll_write(self: Pin<&mut Self>, cx: &mut TaskContext<'_>, buf: &[u8]) -> Poll<io::Result<usize>> {
        let this = self.get_mut();
        if this.flushing {
            ready!(Pin::new(&mut this.ws).poll_flush(cx)).map_err(ws_io_err)?;
            this.flushing = false;
        }
        ready!(Pin::new(&mut this.ws).poll_ready(cx)).map_err(ws_io_err)?;
        Pin::new(&mut this.ws)
            .start_send(Message::binary(buf.to_vec()))
            .map_err(ws_io_err)?;
        match Pin::new(&mut this.ws).poll_flush(cx) {
            Poll::Ready(Ok(())) => {}
            Poll::Ready(Err(e)) => return Poll::Ready(Err(ws_io_err(e))),
            Poll::Pending => this.flushing = true,
        }
        Poll::Ready(Ok(buf.len()))
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut TaskContext<'_>) -> Poll<io::Result<()>> {
        Pin::new(&mut self.get_mut().ws).poll_flush(cx).map_err(ws_io_err)
    }

    fn poll_shutdown(self: Pin<&mut Self>, cx: &mut TaskContext<'_>) -> Poll<io::Result<()>> {
        Pin::new(&mut self.get_mut().ws).poll_close(cx).map_err(ws_io_err)
    }
}

fn tls_acceptor() -> TlsAcceptor {
    let certs = rustls_pemfile::certs(&mut TEST_CERT.as_bytes())
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    let key = rustls_pemfile::private_key(&mut TEST_KEY.as_bytes()).unwrap().unwrap();
    let server_config = rustls::ServerConfig::builder_with_provider(Arc::new(rustls::crypto::ring::default_provider()))
        .with_safe_default_protocol_versions()
        .unwrap()
        .with_no_client_auth()
        .with_single_cert(certs, key)
        .unwrap();
    TlsAcceptor::from(Arc::new(server_config))
}

// --- fake-server spawners --------------------------------------------------

async fn spawn_obfs_http_server() -> SocketAddr {
    let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        while let Ok((tcp, _)) = listener.accept().await {
            tokio::spawn(serve_obfs_http(tcp));
        }
    });
    addr
}

async fn spawn_v2ray_ws_server() -> SocketAddr {
    let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        while let Ok((tcp, _)) = listener.accept().await {
            tokio::spawn(async move {
                if let Ok(ws) = tokio_tungstenite::accept_async(tcp).await {
                    serve_shadowsocks(WsServerStream::new(ws)).await;
                }
            });
        }
    });
    addr
}

async fn spawn_v2ray_ws_tls_server() -> SocketAddr {
    let acceptor = tls_acceptor();
    let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        while let Ok((tcp, _)) = listener.accept().await {
            let acceptor = acceptor.clone();
            tokio::spawn(async move {
                if let Ok(tls) = acceptor.accept(tcp).await
                    && let Ok(ws) = tokio_tungstenite::accept_async(tls).await
                {
                    serve_shadowsocks(WsServerStream::new(ws)).await;
                }
            });
        }
    });
    addr
}

// --- SOCKS5 client + harness ----------------------------------------------

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

/// Build an `ss` outbound (aes-256-gcm) with the given `plugin` config block via
/// the real `from_proxy` parser, so the plugin parsing path is exercised too.
fn config(server: SocketAddr, plugin_yaml: &str) -> OutboundMode {
    let yaml = format!(
        "name: s\ntype: ss\nserver: {}\nport: {}\ncipher: aes-256-gcm\npassword: {PASSWORD}\n{plugin_yaml}",
        server.ip(),
        server.port(),
    );
    let entry: ProxyEntry = serde_yaml_ng::from_str(&yaml).expect("parse proxy entry");
    let cfg = ShadowsocksOutboundConfig::from_proxy(&entry).expect("build ss config");
    OutboundMode::Shadowsocks(Box::new(cfg))
}

/// Drive a SOCKS5 round trip through the kernel built from `outbound`, sending
/// the payload in two writes, and assert it is echoed back unchanged.
async fn assert_relays(outbound: OutboundMode, payload: &[u8]) {
    let handle = GripeKernel::start(GripeConfig {
        socks_listen: SocketAddr::from((Ipv4Addr::LOCALHOST, 0)),
        outbound,
    })
    .await
    .unwrap();

    let dummy_target = SocketAddr::from((Ipv4Addr::new(1, 2, 3, 4), 443));
    let mut conn = socks5_connect(handle.local_addr(), dummy_target).await;

    let split = payload.len() / 2;
    conn.write_all(&payload[..split]).await.unwrap();
    conn.flush().await.unwrap();
    conn.write_all(&payload[split..]).await.unwrap();

    let mut buf = vec![0u8; payload.len()];
    conn.read_exact(&mut buf).await.unwrap();
    assert_eq!(buf, payload);

    handle.shutdown().await;
}

#[tokio::test]
async fn relays_through_obfs_http() {
    let server = spawn_obfs_http_server().await;
    let plugin = "plugin: obfs\nplugin-opts:\n  mode: http\n  host: www.bing.com\n";
    assert_relays(config(server, plugin), b"hello shadowsocks over simple-obfs http").await;
}

#[tokio::test]
async fn relays_through_v2ray_plugin_websocket() {
    let server = spawn_v2ray_ws_server().await;
    let plugin = "plugin: v2ray-plugin\nplugin-opts:\n  mode: websocket\n  host: cdn.example.com\n  path: /ray\n";
    assert_relays(config(server, plugin), b"hello shadowsocks over v2ray-plugin ws").await;
}

#[tokio::test]
async fn relays_through_v2ray_plugin_websocket_tls() {
    let server = spawn_v2ray_ws_tls_server().await;
    let plugin = "plugin: v2ray-plugin\nplugin-opts:\n  mode: websocket\n  tls: true\n  host: example.com\n  skip-cert-verify: true\n  path: /ray\n";
    assert_relays(config(server, plugin), b"hello shadowsocks over v2ray-plugin ws+tls").await;
}

#[tokio::test]
async fn relays_large_payload_over_obfs_http() {
    // Larger than one 0x3FFF-byte chunk to exercise the chunk loop over the
    // plugin transport.
    let server = spawn_obfs_http_server().await;
    let plugin = "plugin: obfs\nplugin-opts:\n  mode: http\n  host: www.bing.com\n";
    let payload: Vec<u8> = (0..40_000u32).map(|i| (i % 251) as u8).collect();
    assert_relays(config(server, plugin), &payload).await;
}
