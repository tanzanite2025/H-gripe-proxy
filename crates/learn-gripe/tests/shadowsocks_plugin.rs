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
use std::task::{Context as TaskContext, Poll, ready};

use common::{assert_relays, serve_shadowsocks, ss_plugin_config};
use futures_util::{Sink, Stream};
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
