//! Generic byte-counting relay primitives.
//!
//! These are the IO half of the connection surface: a [`Counted`] stream
//! adapter and the [`relay_tracked`] bidirectional relay. They are independent
//! of how connections are tracked — they only borrow a [`TrackedConn`]'s byte
//! counters and close signal — so they live next to, but separate from, the
//! connection registry in [`super`].
//!
//! Counting is done by wrapping the *inbound* stream in [`Counted`], so a read
//! off the inbound is an upload (client → target) and a write to the inbound is
//! a download (target → client). That lets a single `copy_bidirectional`
//! account both directions.

use std::io;
use std::pin::Pin;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::task::{Context, Poll};

use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};

use super::TrackedConn;

/// A stream wrapper that counts bytes read into `read_bytes` and bytes written
/// into `write_bytes`. Wrapping the *inbound* stream lets a single
/// `copy_bidirectional` account both directions: reads are uploads (toward the
/// target) and writes are downloads (toward the client).
pub struct Counted<S> {
    inner: S,
    read_bytes: Arc<AtomicU64>,
    write_bytes: Arc<AtomicU64>,
}

impl<S> Counted<S> {
    pub fn new(inner: S, read_bytes: Arc<AtomicU64>, write_bytes: Arc<AtomicU64>) -> Self {
        Self {
            inner,
            read_bytes,
            write_bytes,
        }
    }
}

impl<S: AsyncRead + Unpin> AsyncRead for Counted<S> {
    fn poll_read(mut self: Pin<&mut Self>, cx: &mut Context<'_>, buf: &mut ReadBuf<'_>) -> Poll<io::Result<()>> {
        let before = buf.filled().len();
        let result = Pin::new(&mut self.inner).poll_read(cx, buf);
        if matches!(&result, Poll::Ready(Ok(()))) {
            let read = buf.filled().len().saturating_sub(before);
            self.read_bytes.fetch_add(read as u64, Ordering::Relaxed);
        }
        result
    }
}

impl<S: AsyncWrite + Unpin> AsyncWrite for Counted<S> {
    fn poll_write(mut self: Pin<&mut Self>, cx: &mut Context<'_>, buf: &[u8]) -> Poll<io::Result<usize>> {
        let result = Pin::new(&mut self.inner).poll_write(cx, buf);
        if let Poll::Ready(Ok(n)) = &result {
            self.write_bytes.fetch_add(*n as u64, Ordering::Relaxed);
        }
        result
    }

    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Pin::new(&mut self.inner).poll_flush(cx)
    }

    fn poll_shutdown(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Pin::new(&mut self.inner).poll_shutdown(cx)
    }
}

/// Relay between `inbound` and `outbound`, counting traffic into `conn`'s
/// counters and tearing down when `conn` is signalled to close. Returns when
/// either side closes, on relay error, or on a close signal.
pub async fn relay_tracked<A, B>(inbound: A, mut outbound: B, conn: &TrackedConn) -> io::Result<()>
where
    A: AsyncRead + AsyncWrite + Unpin,
    B: AsyncRead + AsyncWrite + Unpin,
{
    let mut counted = Counted::new(inbound, conn.upload().clone(), conn.download().clone());
    let close = conn.close_signal().clone();
    let closed = close.notified();
    tokio::pin!(closed);
    tokio::select! {
        result = tokio::io::copy_bidirectional(&mut counted, &mut outbound) => result.map(|_| ()),
        _ = &mut closed => Ok(()),
    }
}

#[cfg(test)]
mod tests {
    use std::net::{Ipv4Addr, SocketAddr};
    use std::sync::Arc;
    use std::sync::atomic::Ordering;

    use super::relay_tracked;
    use crate::conntrack::{ConnMeta, ConnNetwork, ConnRegistry};

    fn meta(host: &str) -> ConnMeta {
        ConnMeta {
            network: ConnNetwork::Tcp,
            source: Some(SocketAddr::from((Ipv4Addr::LOCALHOST, 12345))),
            inbound_local: Some(SocketAddr::from((Ipv4Addr::LOCALHOST, 7890))),
            host: host.to_string(),
            destination_ip: None,
            destination_port: 443,
            chains: vec!["DIRECT".to_string()],
            rule: String::new(),
            rule_payload: String::new(),
        }
    }

    #[tokio::test]
    async fn relay_counts_both_directions() {
        let registry = Arc::new(ConnRegistry::default());
        let conn = registry.register(meta("example.com"));

        let (client, inbound) = tokio::io::duplex(1024);
        let (outbound, mut target) = tokio::io::duplex(1024);

        let relay = tokio::spawn(async move { relay_tracked(inbound, outbound, &conn).await });

        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        let mut client = client;
        client.write_all(b"hello target").await.unwrap();
        client.flush().await.unwrap();

        let mut buf = vec![0u8; 12];
        target.read_exact(&mut buf).await.unwrap();
        assert_eq!(&buf, b"hello target");

        target.write_all(b"hi back").await.unwrap();
        target.flush().await.unwrap();
        let mut buf = vec![0u8; 7];
        client.read_exact(&mut buf).await.unwrap();
        assert_eq!(&buf, b"hi back");

        // Close both ends so the relay finishes and its guard drops.
        drop(client);
        drop(target);
        relay.await.unwrap().unwrap();

        let snap = registry.snapshot();
        assert_eq!(snap.upload_total, "hello target".len() as u64);
        assert_eq!(snap.download_total, "hi back".len() as u64);
    }

    #[tokio::test]
    async fn close_signal_tears_down_relay() {
        let registry = Arc::new(ConnRegistry::default());
        let conn = registry.register(meta("example.com"));
        let id = conn.id();

        let (_client, inbound) = tokio::io::duplex(1024);
        let (outbound, _target) = tokio::io::duplex(1024);
        let relay = tokio::spawn(async move { relay_tracked(inbound, outbound, &conn).await });

        // Wait until the relay is actually awaiting before signalling close, so
        // `notify_waiters` is observed.
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        assert!(registry.close(id));

        // The relay returns promptly on the close signal.
        let result = tokio::time::timeout(std::time::Duration::from_secs(1), relay).await;
        assert!(result.is_ok(), "relay did not stop after close signal");
        result.unwrap().unwrap().unwrap();
    }
}
