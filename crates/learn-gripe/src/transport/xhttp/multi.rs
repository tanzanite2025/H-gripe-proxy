//! XHTTP multi-request modes: `stream-up` and `packet-up`.
//!
//! Both decouple uplink from downlink across **separate** HTTP/2 requests on one
//! connection, correlated by a random session id embedded in the request path:
//!
//! ```text
//!   downlink (both modes):  GET  <base>/<session>            -> response body
//!   stream-up uplink:       POST <base>/<session>            (streamed body)
//!   packet-up uplink:       POST <base>/<session>/<seq>      (one body / packet)
//! ```
//!
//! The downlink `GET` response body is the read half. The write half hands bytes
//! to a background task over a bounded channel (back-pressuring the relay); that
//! task drives the uplink `POST`(s). Splitting the uplink across requests is what
//! lets XHTTP traverse CDNs that buffer whole request bodies (`packet-up`) or
//! reject full-duplex bodies (`stream-up`).

use std::future::{Future, poll_fn};
use std::io;
use std::pin::Pin;
use std::task::{Context as TaskContext, Poll, ready};

use anyhow::{Context, Result};
use bytes::Bytes;
use h2::client::{self, SendRequest};
use h2::{RecvStream, SendStream};
use http::{Method, Request};
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};
use tokio::sync::mpsc;

use super::XhttpTransportConfig;

/// Max body bytes per `POST` in `packet-up` mode (Xray's `scMaxEachPostBytes`
/// default). Larger writes are split across sequential packets.
const MAX_POST_BYTES: usize = 1024 * 1024;

/// Bound on the uplink hand-off channel; back-pressures the relay when the
/// network is slower than the local reader.
const UPLINK_BACKLOG: usize = 16;

/// Uplink shape for a multi-request session.
#[derive(Clone, Copy)]
pub(super) enum Uplink {
    /// One streaming `POST` carries the whole uplink.
    Stream,
    /// One `POST` per packet, paths suffixed with a monotonic sequence number.
    Packet,
}

/// URL parts shared by the downlink `GET` and the uplink `POST`(s).
struct Endpoint {
    scheme: &'static str,
    authority: String,
    base: String,
    session: String,
}

impl Endpoint {
    fn session_uri(&self) -> String {
        format!(
            "{}://{}{}{}",
            self.scheme,
            self.authority,
            super::session_path(&self.base, &self.session),
            super::padding_query()
        )
    }

    fn packet_uri(&self, seq: u64) -> String {
        format!(
            "{}://{}{}{}",
            self.scheme,
            self.authority,
            super::packet_path(&self.base, &self.session, seq),
            super::padding_query()
        )
    }
}

/// Open a multi-request XHTTP session and return its byte-stream view.
pub(super) async fn connect<S>(
    stream: S,
    server: &str,
    over_tls: bool,
    cfg: &XhttpTransportConfig,
    uplink: Uplink,
) -> Result<MultiStream>
where
    S: AsyncRead + AsyncWrite + Unpin + Send + 'static,
{
    let (send_request, connection) = client::Builder::new()
        .handshake::<S, Bytes>(stream)
        .await
        .context("xhttp: http/2 handshake")?;
    tokio::spawn(async move {
        let _ = connection.await;
    });

    let endpoint = Endpoint {
        scheme: if over_tls { "https" } else { "http" },
        authority: super::authority_of(cfg, server),
        base: super::base_path(&cfg.path),
        session: super::session_id(),
    };

    // Downlink first: a GET whose response body streams bytes back. Opening it
    // before any uplink POST lets the server register the session id first.
    let down_uri = endpoint.session_uri();
    let get = Request::builder()
        .method(Method::GET)
        .uri(&down_uri)
        .body(())
        .with_context(|| format!("xhttp: build downlink request for {down_uri}"))?;
    let mut send_request = send_request.ready().await.context("xhttp: connection not ready")?;
    let (response, _) = send_request
        .send_request(get, true)
        .context("xhttp: send downlink request")?;
    let recv = response.await.context("xhttp: await downlink response")?.into_body();

    // Uplink: a background task issues POST(s) for bytes received on `tx`.
    let (tx, rx) = mpsc::channel::<Bytes>(UPLINK_BACKLOG);
    tokio::spawn(async move {
        let _ = match uplink {
            Uplink::Stream => drive_stream_up(send_request, endpoint, rx).await,
            Uplink::Packet => drive_packet_up(send_request, endpoint, rx).await,
        };
    });

    Ok(MultiStream::new(recv, tx))
}

/// `stream-up`: a single `POST` whose body streams every uplink byte.
async fn drive_stream_up(
    mut send_request: SendRequest<Bytes>,
    endpoint: Endpoint,
    mut rx: mpsc::Receiver<Bytes>,
) -> Result<()> {
    let uri = endpoint.session_uri();
    let request = post_request(&uri)?;
    send_request = send_request.ready().await.context("xhttp: uplink not ready")?;
    let (response, mut body) = send_request
        .send_request(request, false)
        .context("xhttp: send uplink request")?;
    while let Some(chunk) = rx.recv().await {
        send_body(&mut body, chunk).await?;
    }
    let _ = body.send_data(Bytes::new(), true);
    let _ = response.await;
    Ok(())
}

/// `packet-up`: one `POST` per write (split at `MAX_POST_BYTES`), paths carrying
/// a monotonic sequence number so the server can reorder.
async fn drive_packet_up(
    mut send_request: SendRequest<Bytes>,
    endpoint: Endpoint,
    mut rx: mpsc::Receiver<Bytes>,
) -> Result<()> {
    let mut seq = 0u64;
    while let Some(mut chunk) = rx.recv().await {
        while !chunk.is_empty() {
            let take = chunk.len().min(MAX_POST_BYTES);
            let part = chunk.split_to(take);
            let uri = endpoint.packet_uri(seq);
            let request = post_request(&uri)?;
            send_request = send_request.ready().await.context("xhttp: uplink not ready")?;
            let (response, mut body) = send_request
                .send_request(request, false)
                .context("xhttp: send uplink packet")?;
            send_body(&mut body, part).await?;
            let _ = body.send_data(Bytes::new(), true);
            let _ = response.await;
            seq += 1;
        }
    }
    Ok(())
}

fn post_request(uri: &str) -> Result<Request<()>> {
    Request::builder()
        .method(Method::POST)
        .uri(uri)
        .header(http::header::CONTENT_TYPE, "application/octet-stream")
        .body(())
        .with_context(|| format!("xhttp: build uplink request for {uri}"))
}

/// Stream `data` into an HTTP/2 request body, respecting flow-control capacity.
async fn send_body(body: &mut SendStream<Bytes>, mut data: Bytes) -> Result<()> {
    while !data.is_empty() {
        body.reserve_capacity(data.len());
        match poll_fn(|cx| body.poll_capacity(cx)).await {
            Some(Ok(cap)) => {
                let n = cap.min(data.len());
                if n > 0 {
                    let chunk = data.split_to(n);
                    body.send_data(chunk, false).context("xhttp: send uplink data")?;
                }
            }
            Some(Err(e)) => return Err(anyhow::Error::new(e).context("xhttp: uplink capacity")),
            None => anyhow::bail!("xhttp: uplink stream closed"),
        }
    }
    Ok(())
}

type ReserveFut = Pin<Box<dyn Future<Output = Result<mpsc::OwnedPermit<Bytes>, mpsc::error::SendError<()>>> + Send>>;

/// Byte-stream view over a multi-request XHTTP session: reads drain the downlink
/// `GET` response body; writes are handed to the uplink driver task.
pub struct MultiStream {
    recv: RecvStream,
    read_buf: Bytes,
    recv_eof: bool,
    /// `None` once the write half is shut down (drops the last sender, ending
    /// the uplink driver).
    tx: Option<mpsc::Sender<Bytes>>,
    /// In-flight channel reservation for the current `poll_write`.
    reserve: Option<ReserveFut>,
}

impl MultiStream {
    fn new(recv: RecvStream, tx: mpsc::Sender<Bytes>) -> Self {
        Self {
            recv,
            read_buf: Bytes::new(),
            recv_eof: false,
            tx: Some(tx),
            reserve: None,
        }
    }
}

impl AsyncRead for MultiStream {
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
                    let _ = this.recv.flow_control().release_capacity(data.len());
                    this.read_buf = data;
                }
                Some(Err(e)) => return Poll::Ready(Err(io::Error::other(e.to_string()))),
                None => this.recv_eof = true,
            }
        }
    }
}

impl AsyncWrite for MultiStream {
    fn poll_write(self: Pin<&mut Self>, cx: &mut TaskContext<'_>, buf: &[u8]) -> Poll<io::Result<usize>> {
        let this = self.get_mut();
        let tx = match &this.tx {
            Some(tx) => tx.clone(),
            None => return Poll::Ready(Err(io::Error::new(io::ErrorKind::BrokenPipe, "xhttp: uplink closed"))),
        };
        if this.reserve.is_none() {
            this.reserve = Some(Box::pin(tx.reserve_owned()));
        }
        let fut = this.reserve.as_mut().expect("reserve future set above");
        match fut.as_mut().poll(cx) {
            Poll::Pending => Poll::Pending,
            Poll::Ready(Ok(permit)) => {
                this.reserve = None;
                permit.send(Bytes::copy_from_slice(buf));
                Poll::Ready(Ok(buf.len()))
            }
            Poll::Ready(Err(_)) => {
                this.reserve = None;
                Poll::Ready(Err(io::Error::new(io::ErrorKind::BrokenPipe, "xhttp: uplink closed")))
            }
        }
    }

    fn poll_flush(self: Pin<&mut Self>, _cx: &mut TaskContext<'_>) -> Poll<io::Result<()>> {
        Poll::Ready(Ok(()))
    }

    fn poll_shutdown(self: Pin<&mut Self>, _cx: &mut TaskContext<'_>) -> Poll<io::Result<()>> {
        let this = self.get_mut();
        // Drop the reservation and the sender: the uplink driver sees the channel
        // close and finalizes its POST(s).
        this.reserve = None;
        this.tx = None;
        Poll::Ready(Ok(()))
    }
}
