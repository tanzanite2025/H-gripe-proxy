//! End-to-end proof that traffic flows through a Shadowsocks outbound wrapped
//! in a SIP003 plugin transport: a SOCKS5 client -> gripe inbound -> Shadowsocks
//! outbound (over the plugin) -> fake plugin server -> fake SS server.
//!
//! Covers the simple-obfs **http** mode and v2ray-plugin **websocket** (with and
//! without TLS). The fake servers here strip the plugin framing, then hand the
//! bare byte stream to the shared independent Shadowsocks AEAD server in
//! [`common`]. The fake-TLS (`obfs=tls`) mode lives in its own test file
//! (`shadowsocks_obfs_tls.rs`) because its TLS-record framing is substantial.

mod common;

use std::io;
use std::net::{Ipv4Addr, SocketAddr};
use std::pin::Pin;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::task::{Context as TaskContext, Poll, ready};

use common::{assert_relays, serve_shadowsocks, socks5_connect, ss_plugin_config};
use futures_util::{Sink, Stream};
use learn_gripe::{GripeConfig, GripeKernel};
use quinn::Endpoint;
use quinn::crypto::rustls::QuicServerConfig;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt, ReadBuf};
use tokio::net::{TcpListener, TcpStream};
use tokio_rustls::TlsAcceptor;
use tokio_tungstenite::WebSocketStream;
use tokio_tungstenite::tungstenite::Message;

const TEST_CERT: &str = include_str!("data/vless_tls_cert.pem");
const TEST_KEY: &str = include_str!("data/vless_tls_key.pem");

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

// --- v2ray-plugin mux.cool fake server ------------------------------------

/// Bridge a transport stream that carries mux.cool framing to a plaintext
/// Shadowsocks server. A decode task strips inbound frames (New + Keep/Data)
/// into application bytes; an encode task wraps the server's replies back into
/// `Keep + Data` frames. This mirrors what a mux-enabled v2ray-plugin server
/// does, proving the client speaks the wire format.
async fn serve_v2ray_mux<S>(inner: S)
where
    S: AsyncRead + AsyncWrite + Unpin + Send + 'static,
{
    let (plain, framed_side) = tokio::io::duplex(64 * 1024);
    let (mut inner_rd, mut inner_wr) = tokio::io::split(inner);
    let (mut plain_rd, mut plain_wr) = tokio::io::split(framed_side);

    // inbound frames -> plaintext
    let decode = async move {
        loop {
            let mut metalen_buf = [0u8; 2];
            if inner_rd.read_exact(&mut metalen_buf).await.is_err() {
                break;
            }
            let metalen = u16::from_be_bytes(metalen_buf) as usize;
            if !(4..=512).contains(&metalen) {
                break;
            }
            let mut meta = vec![0u8; metalen];
            if inner_rd.read_exact(&mut meta).await.is_err() {
                break;
            }
            let status = meta[2];
            let option = meta[3];
            if option == 0x01 {
                // OptionData: a `datalen`-prefixed payload follows.
                let mut dl = [0u8; 2];
                if inner_rd.read_exact(&mut dl).await.is_err() {
                    break;
                }
                let mut data = vec![0u8; u16::from_be_bytes(dl) as usize];
                if inner_rd.read_exact(&mut data).await.is_err() {
                    break;
                }
                if plain_wr.write_all(&data).await.is_err() {
                    break;
                }
            }
            if status == 0x03 {
                break; // End
            }
        }
    };

    // plaintext replies -> Keep + Data frames
    let encode = async move {
        let mut buf = [0u8; 16384];
        loop {
            let n = match plain_rd.read(&mut buf).await {
                Ok(0) | Err(_) => break,
                Ok(n) => n,
            };
            let len = (n as u16).to_be_bytes();
            let mut frame = vec![0x00, 0x04, 0x00, 0x00, 0x02, 0x01, len[0], len[1]];
            frame.extend_from_slice(&buf[..n]);
            if inner_wr.write_all(&frame).await.is_err() {
                break;
            }
            if inner_wr.flush().await.is_err() {
                break;
            }
        }
    };

    tokio::join!(serve_shadowsocks(plain), decode, encode);
}

// --- v2ray-http-upgrade fake server ----------------------------------------

/// Complete the HTTP-Upgrade handshake (`GET` + `Connection: Upgrade` ->
/// `101 Switching Protocols`), then run the Shadowsocks server over the raw
/// stream. Bytes that arrived after the header terminator begin the SS stream
/// and are prepended back.
async fn serve_v2ray_http_upgrade(mut tcp: TcpStream) {
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
    assert!(buf.starts_with(b"GET "), "http-upgrade request should be a GET");
    assert!(
        buf.windows(9).any(|w| w.eq_ignore_ascii_case(b"Upgrade: ")),
        "http-upgrade request should carry an Upgrade header"
    );

    let response = "HTTP/1.1 101 Switching Protocols\r\nConnection: Upgrade\r\nUpgrade: websocket\r\n\r\n";
    if tcp.write_all(response.as_bytes()).await.is_err() {
        return;
    }

    let (rd, wr) = tcp.into_split();
    let reader = std::io::Cursor::new(leftover).chain(rd);
    serve_shadowsocks(tokio::io::join(reader, wr)).await;
}

// --- v2ray-plugin quic fake server -----------------------------------------

/// quinn server config offering the `["h2", "http/1.1"]` ALPN the v2ray-plugin
/// quic client presents.
fn quic_server_config() -> quinn::ServerConfig {
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
    crypto.alpn_protocols = vec![b"h2".to_vec(), b"http/1.1".to_vec()];
    let quic = QuicServerConfig::try_from(crypto).unwrap();
    quinn::ServerConfig::with_crypto(Arc::new(quic))
}

/// Run a v2ray-plugin quic server: every bidirectional stream carries one
/// independent Shadowsocks connection. `conn_count` records distinct QUIC
/// connections so a test can assert the client pools them.
async fn run_quic_server(endpoint: Endpoint, conn_count: Arc<AtomicUsize>) {
    while let Some(incoming) = endpoint.accept().await {
        let conn_count = Arc::clone(&conn_count);
        tokio::spawn(async move {
            let conn = match incoming.await {
                Ok(conn) => conn,
                Err(_) => return,
            };
            conn_count.fetch_add(1, Ordering::SeqCst);
            while let Ok((send, recv)) = conn.accept_bi().await {
                tokio::spawn(serve_shadowsocks(tokio::io::join(recv, send)));
            }
        });
    }
}

// --- fake-server spawners (mux / http-upgrade / quic) ----------------------

async fn spawn_v2ray_mux_server() -> SocketAddr {
    let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        while let Ok((tcp, _)) = listener.accept().await {
            tokio::spawn(async move {
                if let Ok(ws) = tokio_tungstenite::accept_async(tcp).await {
                    Box::pin(serve_v2ray_mux(WsServerStream::new(ws))).await;
                }
            });
        }
    });
    addr
}

async fn spawn_v2ray_http_upgrade_server() -> SocketAddr {
    let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        while let Ok((tcp, _)) = listener.accept().await {
            tokio::spawn(serve_v2ray_http_upgrade(tcp));
        }
    });
    addr
}

/// Spawn the quic fake server and return its UDP address plus the connection
/// counter that [`run_quic_server`] increments.
async fn spawn_v2ray_quic_server() -> (SocketAddr, Arc<AtomicUsize>) {
    let endpoint = Endpoint::server(quic_server_config(), (Ipv4Addr::LOCALHOST, 0).into()).unwrap();
    let addr = endpoint.local_addr().unwrap();
    let conn_count = Arc::new(AtomicUsize::new(0));
    let counter = Arc::clone(&conn_count);
    tokio::spawn(run_quic_server(endpoint, counter));
    (addr, conn_count)
}

// --- tests -----------------------------------------------------------------

#[tokio::test]
async fn relays_through_obfs_http() {
    let server = spawn_obfs_http_server().await;
    let plugin = "plugin: obfs\nplugin-opts:\n  mode: http\n  host: www.bing.com\n";
    assert_relays(
        ss_plugin_config(server, plugin),
        b"hello shadowsocks over simple-obfs http",
    )
    .await;
}

#[tokio::test]
async fn relays_through_v2ray_plugin_websocket() {
    let server = spawn_v2ray_ws_server().await;
    let plugin = "plugin: v2ray-plugin\nplugin-opts:\n  mode: websocket\n  host: cdn.example.com\n  path: /ray\n";
    assert_relays(
        ss_plugin_config(server, plugin),
        b"hello shadowsocks over v2ray-plugin ws",
    )
    .await;
}

#[tokio::test]
async fn relays_through_v2ray_plugin_websocket_tls() {
    let server = spawn_v2ray_ws_tls_server().await;
    let plugin = "plugin: v2ray-plugin\nplugin-opts:\n  mode: websocket\n  tls: true\n  host: example.com\n  skip-cert-verify: true\n  path: /ray\n";
    assert_relays(
        ss_plugin_config(server, plugin),
        b"hello shadowsocks over v2ray-plugin ws+tls",
    )
    .await;
}

#[tokio::test]
async fn relays_large_payload_over_obfs_http() {
    // Larger than one 0x3FFF-byte chunk to exercise the chunk loop over the
    // plugin transport.
    let server = spawn_obfs_http_server().await;
    let plugin = "plugin: obfs\nplugin-opts:\n  mode: http\n  host: www.bing.com\n";
    let payload: Vec<u8> = (0..40_000u32).map(|i| (i % 251) as u8).collect();
    assert_relays(ss_plugin_config(server, plugin), &payload).await;
}

#[tokio::test]
async fn relays_through_v2ray_plugin_mux() {
    let server = spawn_v2ray_mux_server().await;
    let plugin =
        "plugin: v2ray-plugin\nplugin-opts:\n  mode: websocket\n  mux: true\n  host: cdn.example.com\n  path: /ray\n";
    assert_relays(
        ss_plugin_config(server, plugin),
        b"hello shadowsocks over v2ray-plugin mux.cool",
    )
    .await;
}

#[tokio::test]
async fn relays_large_payload_over_v2ray_plugin_mux() {
    // Spans many mux Data frames and multiple 0x3FFF-byte SS chunks, exercising
    // both frame splitting on write and reassembly on read.
    let server = spawn_v2ray_mux_server().await;
    let plugin =
        "plugin: v2ray-plugin\nplugin-opts:\n  mode: websocket\n  mux: true\n  host: cdn.example.com\n  path: /ray\n";
    let payload: Vec<u8> = (0..200_000u32).map(|i| (i % 251) as u8).collect();
    assert_relays(ss_plugin_config(server, plugin), &payload).await;
}

#[tokio::test]
async fn relays_through_v2ray_plugin_http_upgrade() {
    let server = spawn_v2ray_http_upgrade_server().await;
    let plugin = "plugin: v2ray-plugin\nplugin-opts:\n  mode: websocket\n  v2ray-http-upgrade: true\n  host: cdn.example.com\n  path: /up\n";
    assert_relays(
        ss_plugin_config(server, plugin),
        b"hello shadowsocks over v2ray-http-upgrade",
    )
    .await;
}

#[tokio::test]
async fn relays_through_v2ray_plugin_quic() {
    let (server, _count) = spawn_v2ray_quic_server().await;
    let plugin = "plugin: v2ray-plugin\nplugin-opts:\n  mode: quic\n  host: example.com\n  skip-cert-verify: true\n";
    assert_relays(
        ss_plugin_config(server, plugin),
        b"hello shadowsocks over v2ray-plugin quic",
    )
    .await;
}

#[tokio::test]
async fn pools_v2ray_plugin_quic_connections() {
    // Two relays that are alive at the same time must share one QUIC handshake:
    // the second dial reuses the first's pooled connection and opens its own
    // bidirectional stream.
    let (server, conn_count) = spawn_v2ray_quic_server().await;
    let plugin = "plugin: v2ray-plugin\nplugin-opts:\n  mode: quic\n  host: example.com\n  skip-cert-verify: true\n";
    let handle = GripeKernel::start(GripeConfig {
        socks_listen: SocketAddr::from((Ipv4Addr::LOCALHOST, 0)),
        outbound: ss_plugin_config(server, plugin),
    })
    .await
    .unwrap();
    let proxy = handle.local_addr();
    let target = SocketAddr::from((Ipv4Addr::new(1, 2, 3, 4), 443));

    // Open the first relay and keep it open while the second is established.
    let mut first = socks5_connect(proxy, target).await;
    first.write_all(b"first").await.unwrap();
    first.flush().await.unwrap();
    let mut got = [0u8; 5];
    first.read_exact(&mut got).await.unwrap();
    assert_eq!(&got, b"first");

    let mut second = socks5_connect(proxy, target).await;
    second.write_all(b"second").await.unwrap();
    second.flush().await.unwrap();
    let mut got2 = [0u8; 6];
    second.read_exact(&mut got2).await.unwrap();
    assert_eq!(&got2, b"second");

    assert_eq!(
        conn_count.load(Ordering::SeqCst),
        1,
        "both relays should share a single pooled QUIC connection"
    );

    handle.shutdown().await;
}
