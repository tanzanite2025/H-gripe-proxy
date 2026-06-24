//! End-to-end proof that traffic flows through a VLESS outbound:
//! a SOCKS5 client -> gripe inbound -> VLESS outbound -> fake VLESS server.
//!
//! The fake server validates the VLESS request header (version, UUID, command,
//! target address), replies with a VLESS response header, then echoes the
//! application payload. This exercises request framing, response-header
//! stripping, the boxed-stream outbound refactor, and — in the TLS case — the
//! rustls client handshake including `skip-cert-verify`.

use std::io;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context as TaskContext, Poll, ready};

use bytes::Bytes;
use futures_util::{Sink, Stream};
use h2::{RecvStream, SendStream};
use http::Response;
use learn_gripe::{
    GripeConfig, GripeKernel, GrpcTransportConfig, HttpUpgradeTransportConfig, OutboundMode, Security, TlsClientConfig,
    Transport, VlessOutboundConfig, WsTransportConfig, XhttpMode, XhttpTransportConfig,
};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt, ReadBuf};
use tokio::net::{TcpListener, TcpStream};
use tokio_rustls::TlsAcceptor;
use tokio_tungstenite::WebSocketStream;
use tokio_tungstenite::tungstenite::Message;

const TEST_UUID: [u8; 16] = [
    0xb8, 0x31, 0x38, 0x1d, 0x63, 0x24, 0x4d, 0x53, 0xad, 0x4f, 0x8c, 0xda, 0x48, 0xb3, 0x08, 0x11,
];
const TEST_UUID_STR: &str = "b831381d-6324-4d53-ad4f-8cda48b30811";

const TEST_CERT: &str = include_str!("data/vless_tls_cert.pem");
const TEST_KEY: &str = include_str!("data/vless_tls_key.pem");

/// Read and validate a VLESS request header, reply with a response header, then
/// echo application bytes back to the client.
async fn serve_vless<S>(mut stream: S)
where
    S: AsyncRead + AsyncWrite + Unpin,
{
    let mut version = [0u8; 1];
    stream.read_exact(&mut version).await.unwrap();
    assert_eq!(version[0], 0x00, "VLESS version");

    let mut uuid = [0u8; 16];
    stream.read_exact(&mut uuid).await.unwrap();
    assert_eq!(uuid, TEST_UUID, "VLESS uuid");

    let mut addon_len = [0u8; 1];
    stream.read_exact(&mut addon_len).await.unwrap();
    if addon_len[0] > 0 {
        let mut addons = vec![0u8; addon_len[0] as usize];
        stream.read_exact(&mut addons).await.unwrap();
    }

    let mut command = [0u8; 1];
    stream.read_exact(&mut command).await.unwrap();
    assert_eq!(command[0], 0x01, "VLESS command should be TCP");

    let mut port = [0u8; 2];
    stream.read_exact(&mut port).await.unwrap();

    let mut atyp = [0u8; 1];
    stream.read_exact(&mut atyp).await.unwrap();
    match atyp[0] {
        0x01 => {
            let mut addr = [0u8; 4];
            stream.read_exact(&mut addr).await.unwrap();
        }
        0x03 => {
            let mut addr = [0u8; 16];
            stream.read_exact(&mut addr).await.unwrap();
        }
        0x02 => {
            let mut len = [0u8; 1];
            stream.read_exact(&mut len).await.unwrap();
            let mut host = vec![0u8; len[0] as usize];
            stream.read_exact(&mut host).await.unwrap();
        }
        other => panic!("unexpected atyp {other}"),
    }

    // VLESS response header: version + zero-length addons.
    stream.write_all(&[0x00, 0x00]).await.unwrap();

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

async fn spawn_fake_vless_server() -> SocketAddr {
    let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        while let Ok((stream, _)) = listener.accept().await {
            tokio::spawn(serve_vless(stream));
        }
    });
    addr
}

async fn spawn_fake_vless_tls_server() -> SocketAddr {
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
    let acceptor = TlsAcceptor::from(Arc::new(server_config));

    let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        while let Ok((tcp, _)) = listener.accept().await {
            let acceptor = acceptor.clone();
            tokio::spawn(async move {
                if let Ok(tls) = acceptor.accept(tcp).await {
                    serve_vless(tls).await;
                }
            });
        }
    });
    addr
}

/// Server-side byte-stream view of a `WebSocketStream`, mirroring the client
/// adapter so the fake server can run `serve_vless` unchanged over `ws`.
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

async fn spawn_fake_vless_ws_server() -> SocketAddr {
    let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        while let Ok((tcp, _)) = listener.accept().await {
            tokio::spawn(async move {
                if let Ok(ws) = tokio_tungstenite::accept_async(tcp).await {
                    serve_vless(WsServerStream::new(ws)).await;
                }
            });
        }
    });
    addr
}

async fn spawn_fake_vless_ws_tls_server() -> SocketAddr {
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
    let acceptor = TlsAcceptor::from(Arc::new(server_config));

    let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        while let Ok((tcp, _)) = listener.accept().await {
            let acceptor = acceptor.clone();
            tokio::spawn(async move {
                if let Ok(tls) = acceptor.accept(tcp).await
                    && let Ok(ws) = tokio_tungstenite::accept_async(tls).await
                {
                    serve_vless(WsServerStream::new(ws)).await;
                }
            });
        }
    });
    addr
}

// ---- gRPC ("gun") transport test helpers ----
//
// Mirror the client-side framing/adapter so the fake server can run the shared
// `serve_vless` over a gRPC tunnel unchanged. Each application chunk is one
// gRPC-framed protobuf `Hunk { bytes data = 1 }` message.

fn grpc_encode_frame(data: &[u8]) -> Bytes {
    let mut hunk = Vec::with_capacity(data.len() + 6);
    hunk.push(0x0a);
    grpc_write_varint(&mut hunk, data.len() as u64);
    hunk.extend_from_slice(data);
    let mut frame = Vec::with_capacity(hunk.len() + 5);
    frame.push(0x00);
    frame.extend_from_slice(&(hunk.len() as u32).to_be_bytes());
    frame.extend_from_slice(&hunk);
    Bytes::from(frame)
}

fn grpc_write_varint(out: &mut Vec<u8>, mut value: u64) {
    loop {
        let mut byte = (value & 0x7f) as u8;
        value >>= 7;
        if value != 0 {
            byte |= 0x80;
        }
        out.push(byte);
        if value == 0 {
            break;
        }
    }
}

fn grpc_read_varint(buf: &[u8]) -> Option<(u64, usize)> {
    let mut value = 0u64;
    let mut shift = 0;
    for (i, &byte) in buf.iter().enumerate().take(10) {
        value |= u64::from(byte & 0x7f) << shift;
        if byte & 0x80 == 0 {
            return Some((value, i + 1));
        }
        shift += 7;
    }
    None
}

fn grpc_decode_hunk(msg: &[u8], out: &mut Vec<u8>) {
    let mut i = 0;
    while i < msg.len() {
        let (tag, n) = grpc_read_varint(&msg[i..]).unwrap();
        i += n;
        let field = tag >> 3;
        let wire = tag & 0x07;
        match wire {
            0 => {
                let (_, n) = grpc_read_varint(&msg[i..]).unwrap();
                i += n;
            }
            1 => i += 8,
            5 => i += 4,
            2 => {
                let (len, n) = grpc_read_varint(&msg[i..]).unwrap();
                i += n;
                let len = len as usize;
                if field == 1 {
                    out.extend_from_slice(&msg[i..i + len]);
                }
                i += len;
            }
            other => panic!("unexpected protobuf wire type {other}"),
        }
    }
}

fn grpc_io_err<E: std::fmt::Display>(e: E) -> io::Error {
    io::Error::other(e.to_string())
}

/// Server-side byte-stream view of an HTTP/2 stream, mirroring the client
/// adapter so the fake server runs `serve_vless` unchanged over `grpc`.
struct GrpcServerStream {
    send: SendStream<Bytes>,
    recv: RecvStream,
    write_buf: Bytes,
    raw: Vec<u8>,
    read_buf: Vec<u8>,
    read_pos: usize,
    recv_eof: bool,
}

impl GrpcServerStream {
    fn new(send: SendStream<Bytes>, recv: RecvStream) -> Self {
        Self {
            send,
            recv,
            write_buf: Bytes::new(),
            raw: Vec::new(),
            read_buf: Vec::new(),
            read_pos: 0,
            recv_eof: false,
        }
    }

    fn poll_drain(&mut self, cx: &mut TaskContext<'_>) -> Poll<io::Result<()>> {
        while !self.write_buf.is_empty() {
            self.send.reserve_capacity(self.write_buf.len());
            match ready!(self.send.poll_capacity(cx)) {
                Some(Ok(cap)) => {
                    let n = cap.min(self.write_buf.len());
                    if n == 0 {
                        return Poll::Pending;
                    }
                    let chunk = self.write_buf.split_to(n);
                    self.send.send_data(chunk, false).map_err(grpc_io_err)?;
                }
                Some(Err(e)) => return Poll::Ready(Err(grpc_io_err(e))),
                None => return Poll::Ready(Err(io::Error::new(io::ErrorKind::BrokenPipe, "grpc send closed"))),
            }
        }
        Poll::Ready(Ok(()))
    }

    fn decode_one(&mut self) -> bool {
        if self.raw.len() < 5 {
            return false;
        }
        let msg_len = u32::from_be_bytes([self.raw[1], self.raw[2], self.raw[3], self.raw[4]]) as usize;
        if self.raw.len() < 5 + msg_len {
            return false;
        }
        let msg = self.raw[5..5 + msg_len].to_vec();
        grpc_decode_hunk(&msg, &mut self.read_buf);
        self.raw.drain(0..5 + msg_len);
        true
    }
}

impl AsyncRead for GrpcServerStream {
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
            this.read_buf.clear();
            this.read_pos = 0;

            if this.decode_one() {
                continue;
            }
            if this.recv_eof {
                return Poll::Ready(Ok(()));
            }

            match ready!(Pin::new(&mut this.recv).poll_data(cx)) {
                Some(Ok(data)) => {
                    let len = data.len();
                    this.raw.extend_from_slice(&data);
                    let _ = this.recv.flow_control().release_capacity(len);
                }
                Some(Err(e)) => return Poll::Ready(Err(grpc_io_err(e))),
                None => this.recv_eof = true,
            }
        }
    }
}

impl AsyncWrite for GrpcServerStream {
    fn poll_write(self: Pin<&mut Self>, cx: &mut TaskContext<'_>, buf: &[u8]) -> Poll<io::Result<usize>> {
        let this = self.get_mut();
        ready!(this.poll_drain(cx))?;
        this.write_buf = grpc_encode_frame(buf);
        match this.poll_drain(cx) {
            Poll::Ready(Err(e)) => return Poll::Ready(Err(e)),
            Poll::Ready(Ok(())) | Poll::Pending => {}
        }
        Poll::Ready(Ok(buf.len()))
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut TaskContext<'_>) -> Poll<io::Result<()>> {
        self.get_mut().poll_drain(cx)
    }

    fn poll_shutdown(self: Pin<&mut Self>, cx: &mut TaskContext<'_>) -> Poll<io::Result<()>> {
        let this = self.get_mut();
        ready!(this.poll_drain(cx))?;
        this.send.send_data(Bytes::new(), true).map_err(grpc_io_err)?;
        Poll::Ready(Ok(()))
    }
}

/// Accept one HTTP/2 connection and serve every inbound stream as a VLESS
/// session. Driving `accept()` in a loop keeps the shared connection polled so
/// already-accepted streams make IO progress.
async fn serve_grpc_connection<T>(io: T)
where
    T: AsyncRead + AsyncWrite + Unpin + Send + 'static,
{
    let mut connection = match h2::server::handshake(io).await {
        Ok(c) => c,
        Err(_) => return,
    };
    while let Some(result) = connection.accept().await {
        let (request, mut respond) = match result {
            Ok(v) => v,
            Err(_) => return,
        };
        let recv = request.into_body();
        let response = Response::builder()
            .status(200)
            .header("content-type", "application/grpc")
            .body(())
            .unwrap();
        let send = match respond.send_response(response, false) {
            Ok(s) => s,
            Err(_) => return,
        };
        tokio::spawn(serve_vless(GrpcServerStream::new(send, recv)));
    }
}

async fn spawn_fake_vless_grpc_server() -> SocketAddr {
    let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        while let Ok((tcp, _)) = listener.accept().await {
            tokio::spawn(serve_grpc_connection(tcp));
        }
    });
    addr
}

async fn spawn_fake_vless_grpc_tls_server() -> SocketAddr {
    let certs = rustls_pemfile::certs(&mut TEST_CERT.as_bytes())
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    let key = rustls_pemfile::private_key(&mut TEST_KEY.as_bytes()).unwrap().unwrap();
    let mut server_config =
        rustls::ServerConfig::builder_with_provider(Arc::new(rustls::crypto::ring::default_provider()))
            .with_safe_default_protocol_versions()
            .unwrap()
            .with_no_client_auth()
            .with_single_cert(certs, key)
            .unwrap();
    server_config.alpn_protocols = vec![b"h2".to_vec()];
    let acceptor = TlsAcceptor::from(Arc::new(server_config));

    let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        while let Ok((tcp, _)) = listener.accept().await {
            let acceptor = acceptor.clone();
            tokio::spawn(async move {
                if let Ok(tls) = acceptor.accept(tcp).await {
                    serve_grpc_connection(tls).await;
                }
            });
        }
    });
    addr
}

// ---- HTTP Upgrade transport test helpers ----
//
// After the `101 Switching Protocols` handshake the connection is a raw byte
// stream (no WebSocket framing), so the server replays `serve_vless` directly
// over a thin wrapper that first surfaces any bytes pipelined behind the
// request head.

/// Wraps a stream, surfacing `prefix` bytes (read past the request head) before
/// passing reads/writes straight through.
struct ServerPrefixStream<S> {
    inner: S,
    prefix: Vec<u8>,
    prefix_pos: usize,
}

impl<S> ServerPrefixStream<S> {
    fn new(inner: S, prefix: Vec<u8>) -> Self {
        Self {
            inner,
            prefix,
            prefix_pos: 0,
        }
    }
}

impl<S: AsyncRead + Unpin> AsyncRead for ServerPrefixStream<S> {
    fn poll_read(self: Pin<&mut Self>, cx: &mut TaskContext<'_>, buf: &mut ReadBuf<'_>) -> Poll<io::Result<()>> {
        let this = self.get_mut();
        if this.prefix_pos < this.prefix.len() {
            let remaining = &this.prefix[this.prefix_pos..];
            let n = remaining.len().min(buf.remaining());
            buf.put_slice(&remaining[..n]);
            this.prefix_pos += n;
            return Poll::Ready(Ok(()));
        }
        Pin::new(&mut this.inner).poll_read(cx, buf)
    }
}

impl<S: AsyncWrite + Unpin> AsyncWrite for ServerPrefixStream<S> {
    fn poll_write(self: Pin<&mut Self>, cx: &mut TaskContext<'_>, buf: &[u8]) -> Poll<io::Result<usize>> {
        Pin::new(&mut self.get_mut().inner).poll_write(cx, buf)
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut TaskContext<'_>) -> Poll<io::Result<()>> {
        Pin::new(&mut self.get_mut().inner).poll_flush(cx)
    }

    fn poll_shutdown(self: Pin<&mut Self>, cx: &mut TaskContext<'_>) -> Poll<io::Result<()>> {
        Pin::new(&mut self.get_mut().inner).poll_shutdown(cx)
    }
}

async fn serve_httpupgrade<T>(mut io: T)
where
    T: AsyncRead + AsyncWrite + Unpin + Send + 'static,
{
    let mut buf = Vec::new();
    let mut tmp = [0u8; 256];
    let header_end = loop {
        let n = match io.read(&mut tmp).await {
            Ok(0) | Err(_) => return,
            Ok(n) => n,
        };
        buf.extend_from_slice(&tmp[..n]);
        if let Some(pos) = buf.windows(4).position(|w| w == b"\r\n\r\n") {
            break pos + 4;
        }
    };
    let response = b"HTTP/1.1 101 Switching Protocols\r\nConnection: Upgrade\r\nUpgrade: websocket\r\n\r\n";
    if io.write_all(response).await.is_err() {
        return;
    }
    let prefix = buf[header_end..].to_vec();
    serve_vless(ServerPrefixStream::new(io, prefix)).await;
}

async fn spawn_fake_vless_httpupgrade_server() -> SocketAddr {
    let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        while let Ok((tcp, _)) = listener.accept().await {
            tokio::spawn(serve_httpupgrade(tcp));
        }
    });
    addr
}

async fn spawn_fake_vless_httpupgrade_tls_server() -> SocketAddr {
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
    let acceptor = TlsAcceptor::from(Arc::new(server_config));

    let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        while let Ok((tcp, _)) = listener.accept().await {
            let acceptor = acceptor.clone();
            tokio::spawn(async move {
                if let Ok(tls) = acceptor.accept(tcp).await {
                    serve_httpupgrade(tls).await;
                }
            });
        }
    });
    addr
}

// ---- XHTTP (stream-one) transport test helpers ----
//
// stream-one is a single full-duplex HTTP/2 POST carrying raw bytes both ways,
// so the server-side adapter is the gRPC one minus the `Hunk` framing.

struct XhttpServerStream {
    send: SendStream<Bytes>,
    recv: RecvStream,
    write_buf: Bytes,
    read_buf: Bytes,
    recv_eof: bool,
}

impl XhttpServerStream {
    fn new(send: SendStream<Bytes>, recv: RecvStream) -> Self {
        Self {
            send,
            recv,
            write_buf: Bytes::new(),
            read_buf: Bytes::new(),
            recv_eof: false,
        }
    }

    fn poll_drain(&mut self, cx: &mut TaskContext<'_>) -> Poll<io::Result<()>> {
        while !self.write_buf.is_empty() {
            self.send.reserve_capacity(self.write_buf.len());
            match ready!(self.send.poll_capacity(cx)) {
                Some(Ok(cap)) => {
                    let n = cap.min(self.write_buf.len());
                    if n == 0 {
                        return Poll::Pending;
                    }
                    let chunk = self.write_buf.split_to(n);
                    self.send.send_data(chunk, false).map_err(grpc_io_err)?;
                }
                Some(Err(e)) => return Poll::Ready(Err(grpc_io_err(e))),
                None => return Poll::Ready(Err(io::Error::new(io::ErrorKind::BrokenPipe, "xhttp send closed"))),
            }
        }
        Poll::Ready(Ok(()))
    }
}

impl AsyncRead for XhttpServerStream {
    fn poll_read(self: Pin<&mut Self>, cx: &mut TaskContext<'_>, buf: &mut ReadBuf<'_>) -> Poll<io::Result<()>> {
        let this = self.get_mut();
        loop {
            if !this.read_buf.is_empty() {
                let n = this.read_buf.len().min(buf.remaining());
                let chunk = this.read_buf.split_to(n);
                buf.put_slice(&chunk);
                return Poll::Ready(Ok(()));
            }
            if this.recv_eof {
                return Poll::Ready(Ok(()));
            }
            match ready!(Pin::new(&mut this.recv).poll_data(cx)) {
                Some(Ok(data)) => {
                    let len = data.len();
                    let _ = this.recv.flow_control().release_capacity(len);
                    this.read_buf = data;
                }
                Some(Err(e)) => return Poll::Ready(Err(grpc_io_err(e))),
                None => this.recv_eof = true,
            }
        }
    }
}

impl AsyncWrite for XhttpServerStream {
    fn poll_write(self: Pin<&mut Self>, cx: &mut TaskContext<'_>, buf: &[u8]) -> Poll<io::Result<usize>> {
        let this = self.get_mut();
        ready!(this.poll_drain(cx))?;
        this.write_buf = Bytes::copy_from_slice(buf);
        match this.poll_drain(cx) {
            Poll::Ready(Err(e)) => return Poll::Ready(Err(e)),
            Poll::Ready(Ok(())) | Poll::Pending => {}
        }
        Poll::Ready(Ok(buf.len()))
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut TaskContext<'_>) -> Poll<io::Result<()>> {
        self.get_mut().poll_drain(cx)
    }

    fn poll_shutdown(self: Pin<&mut Self>, cx: &mut TaskContext<'_>) -> Poll<io::Result<()>> {
        let this = self.get_mut();
        ready!(this.poll_drain(cx))?;
        this.send.send_data(Bytes::new(), true).map_err(grpc_io_err)?;
        Poll::Ready(Ok(()))
    }
}

async fn serve_xhttp_connection<T>(io: T)
where
    T: AsyncRead + AsyncWrite + Unpin + Send + 'static,
{
    let mut connection = match h2::server::handshake(io).await {
        Ok(c) => c,
        Err(_) => return,
    };
    while let Some(result) = connection.accept().await {
        let (request, mut respond) = match result {
            Ok(v) => v,
            Err(_) => return,
        };
        let recv = request.into_body();
        let response = Response::builder()
            .status(200)
            .header("content-type", "application/octet-stream")
            .body(())
            .unwrap();
        let send = match respond.send_response(response, false) {
            Ok(s) => s,
            Err(_) => return,
        };
        tokio::spawn(serve_vless(XhttpServerStream::new(send, recv)));
    }
}

async fn spawn_fake_vless_xhttp_server() -> SocketAddr {
    let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        while let Ok((tcp, _)) = listener.accept().await {
            tokio::spawn(serve_xhttp_connection(tcp));
        }
    });
    addr
}

async fn spawn_fake_vless_xhttp_tls_server() -> SocketAddr {
    let certs = rustls_pemfile::certs(&mut TEST_CERT.as_bytes())
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    let key = rustls_pemfile::private_key(&mut TEST_KEY.as_bytes()).unwrap().unwrap();
    let mut server_config =
        rustls::ServerConfig::builder_with_provider(Arc::new(rustls::crypto::ring::default_provider()))
            .with_safe_default_protocol_versions()
            .unwrap()
            .with_no_client_auth()
            .with_single_cert(certs, key)
            .unwrap();
    server_config.alpn_protocols = vec![b"h2".to_vec()];
    let acceptor = TlsAcceptor::from(Arc::new(server_config));

    let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        while let Ok((tcp, _)) = listener.accept().await {
            let acceptor = acceptor.clone();
            tokio::spawn(async move {
                if let Ok(tls) = acceptor.accept(tcp).await {
                    serve_xhttp_connection(tls).await;
                }
            });
        }
    });
    addr
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

#[tokio::test]
async fn relays_through_plaintext_vless_outbound() {
    let server = spawn_fake_vless_server().await;

    let handle = GripeKernel::start(GripeConfig {
        socks_listen: SocketAddr::from((Ipv4Addr::LOCALHOST, 0)),
        outbound: OutboundMode::Vless(Box::new(VlessOutboundConfig {
            server: server.ip().to_string(),
            port: server.port(),
            uuid: TEST_UUID,
            security: Security::None,
            transport: Transport::Tcp,
        })),
    })
    .await
    .unwrap();

    let dummy_target = SocketAddr::from((Ipv4Addr::new(1, 2, 3, 4), 443));
    let mut conn = socks5_connect(handle.local_addr(), dummy_target).await;
    conn.write_all(b"hello vless").await.unwrap();
    let mut buf = [0u8; 11];
    conn.read_exact(&mut buf).await.unwrap();
    assert_eq!(&buf, b"hello vless");

    handle.shutdown().await;
}

#[tokio::test]
async fn relays_through_tls_vless_outbound() {
    let server = spawn_fake_vless_tls_server().await;

    let handle = GripeKernel::start(GripeConfig {
        socks_listen: SocketAddr::from((Ipv4Addr::LOCALHOST, 0)),
        outbound: OutboundMode::Vless(Box::new(VlessOutboundConfig {
            server: server.ip().to_string(),
            port: server.port(),
            uuid: TEST_UUID,
            security: Security::Tls(TlsClientConfig {
                server_name: Some("localhost".to_string()),
                alpn: Vec::new(),
                skip_cert_verify: true,
            }),
            transport: Transport::Tcp,
        })),
    })
    .await
    .unwrap();

    let dummy_target = SocketAddr::from((Ipv4Addr::new(1, 2, 3, 4), 443));
    let mut conn = socks5_connect(handle.local_addr(), dummy_target).await;
    conn.write_all(b"hello tls vless").await.unwrap();
    let mut buf = [0u8; 15];
    conn.read_exact(&mut buf).await.unwrap();
    assert_eq!(&buf, b"hello tls vless");

    handle.shutdown().await;
}

#[tokio::test]
async fn relays_through_ws_vless_outbound() {
    let server = spawn_fake_vless_ws_server().await;

    let handle = GripeKernel::start(GripeConfig {
        socks_listen: SocketAddr::from((Ipv4Addr::LOCALHOST, 0)),
        outbound: OutboundMode::Vless(Box::new(VlessOutboundConfig {
            server: server.ip().to_string(),
            port: server.port(),
            uuid: TEST_UUID,
            security: Security::None,
            transport: Transport::Ws(WsTransportConfig {
                path: "/ws".to_string(),
                host: None,
                headers: Default::default(),
            }),
        })),
    })
    .await
    .unwrap();

    let dummy_target = SocketAddr::from((Ipv4Addr::new(1, 2, 3, 4), 443));
    let mut conn = socks5_connect(handle.local_addr(), dummy_target).await;
    conn.write_all(b"hello ws vless").await.unwrap();
    let mut buf = [0u8; 14];
    conn.read_exact(&mut buf).await.unwrap();
    assert_eq!(&buf, b"hello ws vless");

    handle.shutdown().await;
}

#[tokio::test]
async fn relays_through_ws_tls_vless_outbound() {
    let server = spawn_fake_vless_ws_tls_server().await;

    let handle = GripeKernel::start(GripeConfig {
        socks_listen: SocketAddr::from((Ipv4Addr::LOCALHOST, 0)),
        outbound: OutboundMode::Vless(Box::new(VlessOutboundConfig {
            server: server.ip().to_string(),
            port: server.port(),
            uuid: TEST_UUID,
            security: Security::Tls(TlsClientConfig {
                server_name: Some("localhost".to_string()),
                alpn: Vec::new(),
                skip_cert_verify: true,
            }),
            transport: Transport::Ws(WsTransportConfig {
                path: "/ws".to_string(),
                host: Some("localhost".to_string()),
                headers: Default::default(),
            }),
        })),
    })
    .await
    .unwrap();

    let dummy_target = SocketAddr::from((Ipv4Addr::new(1, 2, 3, 4), 443));
    let mut conn = socks5_connect(handle.local_addr(), dummy_target).await;
    conn.write_all(b"hello ws tls vless").await.unwrap();
    let mut buf = [0u8; 18];
    conn.read_exact(&mut buf).await.unwrap();
    assert_eq!(&buf, b"hello ws tls vless");

    handle.shutdown().await;
}

#[tokio::test]
async fn relays_through_grpc_vless_outbound() {
    let server = spawn_fake_vless_grpc_server().await;

    let handle = GripeKernel::start(GripeConfig {
        socks_listen: SocketAddr::from((Ipv4Addr::LOCALHOST, 0)),
        outbound: OutboundMode::Vless(Box::new(VlessOutboundConfig {
            server: server.ip().to_string(),
            port: server.port(),
            uuid: TEST_UUID,
            security: Security::None,
            transport: Transport::Grpc(GrpcTransportConfig {
                service_name: "GunService".to_string(),
                host: None,
            }),
        })),
    })
    .await
    .unwrap();

    let dummy_target = SocketAddr::from((Ipv4Addr::new(1, 2, 3, 4), 443));
    let mut conn = socks5_connect(handle.local_addr(), dummy_target).await;
    conn.write_all(b"hello grpc vless").await.unwrap();
    let mut buf = [0u8; 16];
    conn.read_exact(&mut buf).await.unwrap();
    assert_eq!(&buf, b"hello grpc vless");

    handle.shutdown().await;
}

#[tokio::test]
async fn relays_through_grpc_tls_vless_outbound() {
    let server = spawn_fake_vless_grpc_tls_server().await;

    let handle = GripeKernel::start(GripeConfig {
        socks_listen: SocketAddr::from((Ipv4Addr::LOCALHOST, 0)),
        outbound: OutboundMode::Vless(Box::new(VlessOutboundConfig {
            server: server.ip().to_string(),
            port: server.port(),
            uuid: TEST_UUID,
            security: Security::Tls(TlsClientConfig {
                server_name: Some("localhost".to_string()),
                alpn: vec!["h2".to_string()],
                skip_cert_verify: true,
            }),
            transport: Transport::Grpc(GrpcTransportConfig {
                service_name: "GunService".to_string(),
                host: Some("localhost".to_string()),
            }),
        })),
    })
    .await
    .unwrap();

    let dummy_target = SocketAddr::from((Ipv4Addr::new(1, 2, 3, 4), 443));
    let mut conn = socks5_connect(handle.local_addr(), dummy_target).await;
    conn.write_all(b"hello grpc tls vless").await.unwrap();
    let mut buf = [0u8; 20];
    conn.read_exact(&mut buf).await.unwrap();
    assert_eq!(&buf, b"hello grpc tls vless");

    handle.shutdown().await;
}

#[tokio::test]
async fn relays_through_httpupgrade_vless_outbound() {
    let server = spawn_fake_vless_httpupgrade_server().await;

    let handle = GripeKernel::start(GripeConfig {
        socks_listen: SocketAddr::from((Ipv4Addr::LOCALHOST, 0)),
        outbound: OutboundMode::Vless(Box::new(VlessOutboundConfig {
            server: server.ip().to_string(),
            port: server.port(),
            uuid: TEST_UUID,
            security: Security::None,
            transport: Transport::HttpUpgrade(HttpUpgradeTransportConfig {
                path: "/up".to_string(),
                host: None,
                headers: Default::default(),
            }),
        })),
    })
    .await
    .unwrap();

    let dummy_target = SocketAddr::from((Ipv4Addr::new(1, 2, 3, 4), 443));
    let mut conn = socks5_connect(handle.local_addr(), dummy_target).await;
    conn.write_all(b"hello httpupgrade vless").await.unwrap();
    let mut buf = [0u8; 23];
    conn.read_exact(&mut buf).await.unwrap();
    assert_eq!(&buf, b"hello httpupgrade vless");

    handle.shutdown().await;
}

#[tokio::test]
async fn relays_through_httpupgrade_tls_vless_outbound() {
    let server = spawn_fake_vless_httpupgrade_tls_server().await;

    let handle = GripeKernel::start(GripeConfig {
        socks_listen: SocketAddr::from((Ipv4Addr::LOCALHOST, 0)),
        outbound: OutboundMode::Vless(Box::new(VlessOutboundConfig {
            server: server.ip().to_string(),
            port: server.port(),
            uuid: TEST_UUID,
            security: Security::Tls(TlsClientConfig {
                server_name: Some("localhost".to_string()),
                alpn: Vec::new(),
                skip_cert_verify: true,
            }),
            transport: Transport::HttpUpgrade(HttpUpgradeTransportConfig {
                path: "/up".to_string(),
                host: Some("localhost".to_string()),
                headers: Default::default(),
            }),
        })),
    })
    .await
    .unwrap();

    let dummy_target = SocketAddr::from((Ipv4Addr::new(1, 2, 3, 4), 443));
    let mut conn = socks5_connect(handle.local_addr(), dummy_target).await;
    conn.write_all(b"hello httpupgrade tls vless").await.unwrap();
    let mut buf = [0u8; 27];
    conn.read_exact(&mut buf).await.unwrap();
    assert_eq!(&buf, b"hello httpupgrade tls vless");

    handle.shutdown().await;
}

#[tokio::test]
async fn relays_through_xhttp_vless_outbound() {
    let server = spawn_fake_vless_xhttp_server().await;

    let handle = GripeKernel::start(GripeConfig {
        socks_listen: SocketAddr::from((Ipv4Addr::LOCALHOST, 0)),
        outbound: OutboundMode::Vless(Box::new(VlessOutboundConfig {
            server: server.ip().to_string(),
            port: server.port(),
            uuid: TEST_UUID,
            security: Security::None,
            transport: Transport::Xhttp(XhttpTransportConfig {
                path: "/".to_string(),
                host: None,
                mode: XhttpMode::StreamOne,
            }),
        })),
    })
    .await
    .unwrap();

    let dummy_target = SocketAddr::from((Ipv4Addr::new(1, 2, 3, 4), 443));
    let mut conn = socks5_connect(handle.local_addr(), dummy_target).await;
    conn.write_all(b"hello xhttp vless").await.unwrap();
    let mut buf = [0u8; 17];
    conn.read_exact(&mut buf).await.unwrap();
    assert_eq!(&buf, b"hello xhttp vless");

    handle.shutdown().await;
}

#[tokio::test]
async fn relays_through_xhttp_tls_vless_outbound() {
    let server = spawn_fake_vless_xhttp_tls_server().await;

    let handle = GripeKernel::start(GripeConfig {
        socks_listen: SocketAddr::from((Ipv4Addr::LOCALHOST, 0)),
        outbound: OutboundMode::Vless(Box::new(VlessOutboundConfig {
            server: server.ip().to_string(),
            port: server.port(),
            uuid: TEST_UUID,
            security: Security::Tls(TlsClientConfig {
                server_name: Some("localhost".to_string()),
                alpn: vec!["h2".to_string()],
                skip_cert_verify: true,
            }),
            transport: Transport::Xhttp(XhttpTransportConfig {
                path: "/".to_string(),
                host: Some("localhost".to_string()),
                mode: XhttpMode::StreamOne,
            }),
        })),
    })
    .await
    .unwrap();

    let dummy_target = SocketAddr::from((Ipv4Addr::new(1, 2, 3, 4), 443));
    let mut conn = socks5_connect(handle.local_addr(), dummy_target).await;
    conn.write_all(b"hello xhttp tls vless").await.unwrap();
    let mut buf = [0u8; 21];
    conn.read_exact(&mut buf).await.unwrap();
    assert_eq!(&buf, b"hello xhttp tls vless");

    handle.shutdown().await;
}

#[test]
fn uuid_str_matches_bytes() {
    // Guards the test fixture against drift between the string and byte forms.
    let hex: String = TEST_UUID_STR.chars().filter(|c| *c != '-').collect();
    let bytes: Vec<u8> = (0..16)
        .map(|i| u8::from_str_radix(&hex[i * 2..i * 2 + 2], 16).unwrap())
        .collect();
    assert_eq!(bytes, TEST_UUID);
}
