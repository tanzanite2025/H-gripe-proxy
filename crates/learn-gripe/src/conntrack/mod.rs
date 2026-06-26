//! In-process connection registry.
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
//! The relay itself lives in [`relay`]: [`relay_tracked`] is a counted, closable
//! `copy_bidirectional` that borrows a [`TrackedConn`]'s counters and close
//! signal but is otherwise independent of this registry.

use std::collections::HashMap;
use std::net::{IpAddr, SocketAddr};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

use tokio::sync::{Notify, watch};

use crate::address::TargetAddr;
use crate::config::OutboundMode;

pub mod relay;

pub use relay::relay_tracked;

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

    /// Parse a transport name (`"tcp"` / `"udp"`, case-insensitive). Used by the
    /// router's `NETWORK` rule parsing; returns `None` for anything else.
    pub fn parse(s: &str) -> Option<Self> {
        match s.trim().to_ascii_lowercase().as_str() {
            "tcp" => Some(ConnNetwork::Tcp),
            "udp" => Some(ConnNetwork::Udp),
            _ => None,
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
#[derive(Debug)]
pub struct ConnRegistry {
    inner: Mutex<RegistryInner>,
    /// Monotonic generation counter, bumped whenever the table's membership
    /// changes (a connection registered or removed). Drives [`subscribe`] so the
    /// live-connections stream can refresh on structural changes without
    /// busy-polling. Byte-count-only changes are not signalled here (consumers
    /// poll on an interval for those).
    change: watch::Sender<u64>,
}

impl Default for ConnRegistry {
    fn default() -> Self {
        let (change, _) = watch::channel(0);
        Self {
            inner: Mutex::new(RegistryInner::default()),
            change,
        }
    }
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
        self.bump_generation();

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

    /// Subscribe to structural changes of the table (a connection registered or
    /// removed). The watched value is a monotonically increasing generation
    /// counter; consumers await `changed()` to learn membership changed, then
    /// re-[`snapshot`](Self::snapshot). The receiver's `changed()` resolves with
    /// an error once the registry (and the kernel owning it) is dropped, which
    /// the stream uses to detect a stopped kernel.
    pub fn subscribe(&self) -> watch::Receiver<u64> {
        self.change.subscribe()
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
        let removed = {
            let mut inner = self.inner.lock().unwrap();
            match inner.entries.remove(&id) {
                Some(entry) => {
                    let upload = entry.upload.load(Ordering::Relaxed);
                    let download = entry.download.load(Ordering::Relaxed);
                    inner.closed_upload = inner.closed_upload.saturating_add(upload);
                    inner.closed_download = inner.closed_download.saturating_add(download);
                    true
                }
                None => false,
            }
        };
        if removed {
            self.bump_generation();
        }
    }

    /// Signal subscribers that the table's membership changed.
    fn bump_generation(&self) {
        self.change
            .send_modify(|generation| *generation = generation.wrapping_add(1));
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

    #[test]
    fn subscribe_signals_register_and_deregister() {
        let registry = Arc::new(ConnRegistry::default());
        let mut rx = registry.subscribe();
        assert!(!rx.has_changed().unwrap());

        let conn = registry.register(meta("example.com"));
        // Registering bumped the generation.
        assert!(rx.has_changed().unwrap());
        let after_register = *rx.borrow_and_update();
        assert_ne!(after_register, 0);

        // Byte-count changes do not bump the generation.
        conn.upload().fetch_add(10, Ordering::Relaxed);
        assert!(!rx.has_changed().unwrap());

        drop(conn);
        // Dropping (deregister) bumped it again.
        assert!(rx.has_changed().unwrap());
        assert_ne!(*rx.borrow(), after_register);
    }
}
