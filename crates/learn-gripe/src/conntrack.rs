//! In-process connection registry and tracked relay.
//!
//! This is the kernel half of the controller "connection table + close" surface
//! that the app used to obtain from the external Mihomo controller. The kernel
//! owns the data plane, so it can track live connections directly instead of
//! polling a separate process:
//!
//! - every relayed TCP connection [`register`](ConnRegistry::register)s itself,
//!   yielding a [`TrackedConn`] whose live byte counters the relay increments;
//! - [`snapshot`](ConnRegistry::snapshot) returns the current table plus
//!   cumulative up/down totals (closed connections keep contributing to the
//!   totals, matching the Mihomo `/connections` shape);
//! - [`close`](ConnRegistry::close) / [`close_all`](ConnRegistry::close_all)
//!   signal a connection (or all of them) to tear down.
//!
//! The relay itself is [`relay_tracked`]: a counted, closable
//! `copy_bidirectional`. Counting is done by wrapping the *inbound* stream in
//! [`Counted`], so a read off the inbound is an upload (client → target) and a
//! write to the inbound is a download (target → client).

use std::collections::HashMap;
use std::io;
use std::net::{IpAddr, SocketAddr};
use std::pin::Pin;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll};
use std::time::{SystemTime, UNIX_EPOCH};

use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};
use tokio::sync::Notify;

use crate::address::TargetAddr;
use crate::config::OutboundMode;

/// Transport-layer protocol of a tracked connection.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ConnNetwork {
    Tcp,
    Udp,
}

impl ConnNetwork {
    /// Lowercase wire name (`"tcp"` / `"udp"`), matching the controller JSON.
    pub fn as_str(self) -> &'static str {
        match self {
            ConnNetwork::Tcp => "tcp",
            ConnNetwork::Udp => "udp",
        }
    }
}

/// Descriptive metadata captured when a connection is registered. Immutable for
/// the connection's lifetime; the mutable part (byte counts) lives in atomics.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ConnMeta {
    pub network: ConnNetwork,
    /// The client that opened the inbound connection.
    pub source: Option<SocketAddr>,
    /// The local inbound address the client reached us on.
    pub inbound_local: Option<SocketAddr>,
    /// Requested destination host (domain, or IP literal as text).
    pub host: String,
    /// Resolved destination IP when the target was already an IP literal.
    pub destination_ip: Option<IpAddr>,
    pub destination_port: u16,
    /// Outbound chain that carried the connection, outermost-first. For the
    /// rule router this is the matched outbound's name; otherwise the outbound
    /// type label.
    pub chains: Vec<String>,
    /// Matched rule type (e.g. `"DomainSuffix"`, `"GeoIP"`, `"Match"`), empty
    /// when no rule router was involved.
    pub rule: String,
    /// Matched rule payload (e.g. the suffix or CIDR), empty for `Match` or when
    /// no rule router was involved.
    pub rule_payload: String,
}

impl ConnMeta {
    /// Build metadata for a connection to `target` over `outbound`, resolving
    /// the chain (and matched rule, for the rule router) the connection takes.
    pub fn for_target(
        network: ConnNetwork,
        source: Option<SocketAddr>,
        inbound_local: Option<SocketAddr>,
        outbound: &OutboundMode,
        target: &TargetAddr,
    ) -> Self {
        let (chains, rule, rule_payload) = match outbound {
            OutboundMode::Routed(router) => {
                let selection = router.select_detailed(target);
                let (rule, payload) = match selection.rule {
                    Some(rule) => (rule.matcher.kind_str().to_string(), rule.matcher.payload()),
                    None => (String::new(), String::new()),
                };
                (vec![selection.outbound_name.to_string()], rule, payload)
            }
            other => (vec![other.type_label().to_string()], String::new(), String::new()),
        };
        Self {
            network,
            source,
            inbound_local,
            host: target.host(),
            destination_ip: target.ip(),
            destination_port: target.port(),
            chains,
            rule,
            rule_payload,
        }
    }
}

/// A point-in-time view of a single live connection.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ConnSnapshot {
    pub id: u64,
    pub meta: ConnMeta,
    pub upload: u64,
    pub download: u64,
    /// Wall-clock start time, milliseconds since the Unix epoch.
    pub start_unix_ms: u64,
}

/// The whole connection table plus cumulative byte totals.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ConnTableSnapshot {
    pub connections: Vec<ConnSnapshot>,
    pub upload_total: u64,
    pub download_total: u64,
}

#[derive(Debug)]
struct ConnEntry {
    meta: ConnMeta,
    upload: Arc<AtomicU64>,
    download: Arc<AtomicU64>,
    start_unix_ms: u64,
    close: Arc<Notify>,
}

#[derive(Debug, Default)]
struct RegistryInner {
    next_id: u64,
    entries: HashMap<u64, ConnEntry>,
    /// Cumulative bytes of connections that have already closed; live
    /// connections' current counts are added on top in [`snapshot`].
    closed_upload: u64,
    closed_download: u64,
}

/// Registry of live connections owned by a running kernel.
#[derive(Debug, Default)]
pub struct ConnRegistry {
    inner: Mutex<RegistryInner>,
}

impl ConnRegistry {
    /// Register a new connection and return its [`TrackedConn`] handle. The
    /// relay increments the handle's counters and races its close signal; when
    /// the handle is dropped the connection is removed from the table and its
    /// bytes roll into the cumulative totals.
    pub fn register(self: &Arc<Self>, meta: ConnMeta) -> TrackedConn {
        let upload = Arc::new(AtomicU64::new(0));
        let download = Arc::new(AtomicU64::new(0));
        let close = Arc::new(Notify::new());
        let start_unix_ms = now_unix_ms();

        let id = {
            let mut inner = self.inner.lock().unwrap();
            let id = inner.next_id;
            inner.next_id += 1;
            inner.entries.insert(
                id,
                ConnEntry {
                    meta,
                    upload: upload.clone(),
                    download: download.clone(),
                    start_unix_ms,
                    close: close.clone(),
                },
            );
            id
        };

        TrackedConn {
            registry: self.clone(),
            id,
            upload,
            download,
            close,
        }
    }

    /// Snapshot the current connection table plus cumulative totals.
    pub fn snapshot(&self) -> ConnTableSnapshot {
        let inner = self.inner.lock().unwrap();
        let mut connections = Vec::with_capacity(inner.entries.len());
        let mut live_upload = 0u64;
        let mut live_download = 0u64;
        for (&id, entry) in inner.entries.iter() {
            let upload = entry.upload.load(Ordering::Relaxed);
            let download = entry.download.load(Ordering::Relaxed);
            live_upload = live_upload.saturating_add(upload);
            live_download = live_download.saturating_add(download);
            connections.push(ConnSnapshot {
                id,
                meta: entry.meta.clone(),
                upload,
                download,
                start_unix_ms: entry.start_unix_ms,
            });
        }
        connections.sort_by_key(|c| c.id);
        ConnTableSnapshot {
            connections,
            upload_total: inner.closed_upload.saturating_add(live_upload),
            download_total: inner.closed_download.saturating_add(live_download),
        }
    }

    /// Number of live connections.
    pub fn len(&self) -> usize {
        self.inner.lock().unwrap().entries.len()
    }

    /// Whether there are no live connections.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Signal the connection with `id` to close. Returns `true` if it existed.
    pub fn close(&self, id: u64) -> bool {
        let inner = self.inner.lock().unwrap();
        match inner.entries.get(&id) {
            Some(entry) => {
                entry.close.notify_waiters();
                true
            }
            None => false,
        }
    }

    /// Signal every live connection to close. Returns the number signalled.
    pub fn close_all(&self) -> usize {
        let inner = self.inner.lock().unwrap();
        for entry in inner.entries.values() {
            entry.close.notify_waiters();
        }
        inner.entries.len()
    }

    /// Remove a connection, rolling its final byte counts into the totals.
    fn deregister(&self, id: u64) {
        let mut inner = self.inner.lock().unwrap();
        if let Some(entry) = inner.entries.remove(&id) {
            let upload = entry.upload.load(Ordering::Relaxed);
            let download = entry.download.load(Ordering::Relaxed);
            inner.closed_upload = inner.closed_upload.saturating_add(upload);
            inner.closed_download = inner.closed_download.saturating_add(download);
        }
    }
}

/// Handle to a registered connection. The relay holds it for the connection's
/// lifetime; dropping it deregisters the connection from the [`ConnRegistry`].
pub struct TrackedConn {
    registry: Arc<ConnRegistry>,
    id: u64,
    upload: Arc<AtomicU64>,
    download: Arc<AtomicU64>,
    close: Arc<Notify>,
}

impl TrackedConn {
    /// The connection's id within the registry (also its close key).
    pub fn id(&self) -> u64 {
        self.id
    }

    /// Upload (client → target) byte counter the relay increments.
    pub fn upload(&self) -> &Arc<AtomicU64> {
        &self.upload
    }

    /// Download (target → client) byte counter the relay increments.
    pub fn download(&self) -> &Arc<AtomicU64> {
        &self.download
    }

    /// The close signal for this connection (notified by [`ConnRegistry::close`]).
    pub fn close_signal(&self) -> &Arc<Notify> {
        &self.close
    }
}

impl Drop for TrackedConn {
    fn drop(&mut self) {
        self.registry.deregister(self.id);
    }
}

fn now_unix_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

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
    use super::*;
    use std::net::Ipv4Addr;

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

    #[test]
    fn register_then_drop_updates_table_and_totals() {
        let registry = Arc::new(ConnRegistry::default());
        assert!(registry.is_empty());

        let conn = registry.register(meta("example.com"));
        conn.upload().fetch_add(100, Ordering::Relaxed);
        conn.download().fetch_add(250, Ordering::Relaxed);

        let snap = registry.snapshot();
        assert_eq!(snap.connections.len(), 1);
        assert_eq!(snap.connections[0].meta.host, "example.com");
        assert_eq!(snap.connections[0].upload, 100);
        assert_eq!(snap.connections[0].download, 250);
        // Live bytes already contribute to the running totals.
        assert_eq!(snap.upload_total, 100);
        assert_eq!(snap.download_total, 250);

        let id = conn.id();
        drop(conn);

        // Removed from the table, but its bytes persist in the totals.
        let snap = registry.snapshot();
        assert!(snap.connections.is_empty());
        assert_eq!(snap.upload_total, 100);
        assert_eq!(snap.download_total, 250);
        assert!(!registry.close(id));
    }

    #[test]
    fn ids_are_monotonic_and_unique() {
        let registry = Arc::new(ConnRegistry::default());
        let a = registry.register(meta("a"));
        let b = registry.register(meta("b"));
        assert_ne!(a.id(), b.id());
        assert!(b.id() > a.id());
        assert_eq!(registry.len(), 2);
    }

    #[test]
    fn close_targets_only_the_requested_connection() {
        let registry = Arc::new(ConnRegistry::default());
        let a = registry.register(meta("a"));
        let _b = registry.register(meta("b"));
        assert!(registry.close(a.id()));
        assert!(!registry.close(9999));
        // Signalling does not remove the entry; the relay's guard does on drop.
        assert_eq!(registry.len(), 2);
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
