//! End-to-end proof that traffic flows through a VLESS outbound over XHTTP's
//! multi-request modes (`stream-up` and `packet-up`):
//! SOCKS5 client -> gripe inbound -> VLESS/XHTTP outbound -> fake XHTTP server.
//!
//! The fake server speaks real HTTP/2: it correlates the downlink `GET` and the
//! uplink `POST`(s) by session id within a connection (the `GET` arrives first,
//! opening the downlink response body; each subsequent `POST` body is appended
//! to the uplink byte stream). Over that reconstructed full-duplex stream it
//! runs the same VLESS handshake + echo as the other transport tests, proving
//! the client's session-path / sequential-`POST` framing reassembles correctly.

use std::io;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::pin::Pin;
use std::task::{Context as TaskContext, Poll, ready};

use bytes::Bytes;
use h2::{RecvStream, SendStream};
use http::{Method, Response};
use learn_gripe::{
    GripeConfig, GripeKernel, OutboundMode, Security, Transport, VlessOutboundConfig, XhttpMode, XhttpTransportConfig,
};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt, ReadBuf};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::mpsc;

const TEST_UUID: [u8; 16] = [
    0xb8, 0x31, 0x38, 0x1d, 0x63, 0x24, 0x4d, 0x53, 0xad, 0x4f, 0x8c, 0xda, 0x48, 0xb3, 0x08, 0x11,
];

fn io_err<E: std::fmt::Display>(e: E) -> io::Error {
    io::Error::other(e.to_string())
}

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

/// Server-side view of an XHTTP multi-request session: reads concatenate the
/// uplink `POST` bodies (delivered in order over `up_rx`); writes stream into
/// the downlink `GET` response body.
struct ServerMultiStream {
    send: SendStream<Bytes>,
    up_rx: mpsc::Receiver<RecvStream>,
    current: Option<RecvStream>,
    up_closed: bool,
    read_buf: Bytes,
    write_buf: Bytes,
}

impl ServerMultiStream {
    fn new(send: SendStream<Bytes>, up_rx: mpsc::Receiver<RecvStream>) -> Self {
        Self {
            send,
            up_rx,
            current: None,
            up_closed: false,
            read_buf: Bytes::new(),
            write_buf: Bytes::new(),
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
                    self.send.send_data(chunk, false).map_err(io_err)?;
                }
                Some(Err(e)) => return Poll::Ready(Err(io_err(e))),
                None => return Poll::Ready(Err(io::Error::new(io::ErrorKind::BrokenPipe, "downlink closed"))),
            }
        }
        Poll::Ready(Ok(()))
    }
}

impl AsyncRead for ServerMultiStream {
    fn poll_read(self: Pin<&mut Self>, cx: &mut TaskContext<'_>, buf: &mut ReadBuf<'_>) -> Poll<io::Result<()>> {
        let this = self.get_mut();
        loop {
            if !this.read_buf.is_empty() {
                let n = this.read_buf.len().min(buf.remaining());
                let chunk = this.read_buf.split_to(n);
                buf.put_slice(&chunk);
                return Poll::Ready(Ok(()));
            }
            if this.current.is_none() {
                if this.up_closed {
                    return Poll::Ready(Ok(()));
                }
                match ready!(this.up_rx.poll_recv(cx)) {
                    Some(recv) => this.current = Some(recv),
                    None => {
                        this.up_closed = true;
                        return Poll::Ready(Ok(()));
                    }
                }
            }
            let recv = this.current.as_mut().expect("current set above");
            match ready!(recv.poll_data(cx)) {
                Some(Ok(data)) => {
                    let _ = recv.flow_control().release_capacity(data.len());
                    this.read_buf = data;
                }
                Some(Err(e)) => return Poll::Ready(Err(io_err(e))),
                None => this.current = None,
            }
        }
    }
}

impl AsyncWrite for ServerMultiStream {
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
        this.send.send_data(Bytes::new(), true).map_err(io_err)?;
        Poll::Ready(Ok(()))
    }
}

/// Accept one HTTP/2 connection: the first `GET` opens the downlink and starts a
/// VLESS session; every later `POST` body is forwarded to that session in order.
async fn serve_xhttp_multi_connection<T>(io: T)
where
    T: AsyncRead + AsyncWrite + Unpin + Send + 'static,
{
    let mut connection = match h2::server::handshake(io).await {
        Ok(c) => c,
        Err(_) => return,
    };
    let mut up_tx: Option<mpsc::Sender<RecvStream>> = None;
    while let Some(result) = connection.accept().await {
        let (request, mut respond) = match result {
            Ok(v) => v,
            Err(_) => return,
        };
        if request.method() == Method::GET {
            let response = Response::builder()
                .status(200)
                .header("content-type", "application/octet-stream")
                .body(())
                .unwrap();
            let send = match respond.send_response(response, false) {
                Ok(s) => s,
                Err(_) => return,
            };
            let (tx, rx) = mpsc::channel::<RecvStream>(16);
            up_tx = Some(tx);
            tokio::spawn(serve_vless(ServerMultiStream::new(send, rx)));
        } else {
            let body = request.into_body();
            let response = Response::builder().status(200).body(()).unwrap();
            let _ = respond.send_response(response, true);
            if let Some(tx) = &up_tx {
                let _ = tx.send(body).await;
            }
        }
    }
}

async fn spawn_fake_vless_xhttp_multi_server() -> SocketAddr {
    let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        while let Ok((tcp, _)) = listener.accept().await {
            tokio::spawn(serve_xhttp_multi_connection(tcp));
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

async fn relays_through_xhttp_mode(mode: XhttpMode, payload: &[u8]) {
    let server = spawn_fake_vless_xhttp_multi_server().await;

    let handle = GripeKernel::start(GripeConfig {
        socks_listen: SocketAddr::from((Ipv4Addr::LOCALHOST, 0)),
        outbound: OutboundMode::Vless(Box::new(VlessOutboundConfig {
            server: server.ip().to_string(),
            port: server.port(),
            uuid: TEST_UUID,
            security: Security::None,
            transport: Transport::Xhttp(XhttpTransportConfig {
                path: "/proxy".to_string(),
                host: None,
                mode,
            }),
            vision: false,
        })),
    })
    .await
    .unwrap();

    let dummy_target = SocketAddr::from((Ipv4Addr::new(1, 2, 3, 4), 443));
    let mut conn = socks5_connect(handle.local_addr(), dummy_target).await;
    conn.write_all(payload).await.unwrap();
    let mut buf = vec![0u8; payload.len()];
    conn.read_exact(&mut buf).await.unwrap();
    assert_eq!(buf, payload);

    handle.shutdown().await;
}

#[tokio::test]
async fn relays_through_xhttp_stream_up_vless_outbound() {
    relays_through_xhttp_mode(XhttpMode::StreamUp, b"hello xhttp stream-up vless").await;
}

#[tokio::test]
async fn relays_through_xhttp_packet_up_vless_outbound() {
    relays_through_xhttp_mode(XhttpMode::PacketUp, b"hello xhttp packet-up vless").await;
}

#[tokio::test]
async fn relays_large_payload_through_xhttp_packet_up() {
    // Exercise multi-chunk reassembly: the relay writes well over one socket
    // read's worth, producing several sequential uplink POSTs.
    let payload: Vec<u8> = (0..200_000).map(|i| (i % 251) as u8).collect();
    relays_through_xhttp_mode(XhttpMode::PacketUp, &payload).await;
}
