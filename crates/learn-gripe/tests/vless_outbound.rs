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

use futures_util::{Sink, Stream};
use learn_gripe::{
    GripeConfig, GripeKernel, OutboundMode, Security, TlsClientConfig, Transport, VlessOutboundConfig,
    WsTransportConfig,
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

#[test]
fn uuid_str_matches_bytes() {
    // Guards the test fixture against drift between the string and byte forms.
    let hex: String = TEST_UUID_STR.chars().filter(|c| *c != '-').collect();
    let bytes: Vec<u8> = (0..16)
        .map(|i| u8::from_str_radix(&hex[i * 2..i * 2 + 2], 16).unwrap())
        .collect();
    assert_eq!(bytes, TEST_UUID);
}
