//! External lookup abstractions the routing data plane queries but never owns.
//!
//! The geo / rule-set / process matchers carry shared handles to data the
//! embedder maintains: the kernel never fetches or stores this data itself, it
//! only ever *queries* through these traits, keeping data sourcing out of the
//! data plane.

use std::net::{IpAddr, SocketAddr};

use crate::address::TargetAddr;
use crate::conntrack::ConnNetwork;

/// Lookup into a locally-maintained geo database, used by the `GEOIP` /
/// `GEOSITE` matchers. The kernel does not own or fetch this data: the embedder
/// loads the local mmdb / geosite files it maintains and provides an
/// implementation, so the routing data plane only ever *queries* the database.
pub trait GeoLookup: Send + Sync {
    /// Whether `ip` belongs to the geo country `code` (e.g. `"cn"`).
    fn geoip_matches(&self, code: &str, ip: IpAddr) -> bool;
    /// Whether `host` belongs to the geosite category `code` (e.g. `"google"`).
    fn geosite_matches(&self, code: &str, host: &str) -> bool;
    /// Whether `ip` is announced by the autonomous system number `asn` (e.g.
    /// `13335`).
    fn asn_matches(&self, asn: u32, ip: IpAddr) -> bool;
}

/// Lookup into a locally-loaded named rule-set ("rule provider"), used by the
/// `RULE-SET` matcher. Like [`GeoLookup`], the kernel neither owns nor fetches
/// this data: the embedder loads the rule-provider payloads it maintains
/// (inline lists, cached files, …) and supplies an implementation, so the
/// routing data plane only ever *queries* a set by name.
pub trait RuleSetLookup: Send + Sync {
    /// Whether `target` is contained in the rule-set named `name`. The set's
    /// behaviour (domain / ipcidr / classical) is decided by the provider, so
    /// unlike the geo matchers it is handed the whole target and matches a
    /// domain or an IP as appropriate; an unknown `name` matches nothing.
    fn rule_set_matches(&self, name: &str, target: &TargetAddr) -> bool;
}

/// The executable that originated a connection, resolved by a
/// [`ProcessLookup`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProcessInfo {
    /// The executable's base name, e.g. `curl` or `chrome.exe`.
    pub name: String,
    /// The executable's full path, e.g. `/usr/bin/curl`.
    pub path: String,
    /// The owning user's numeric id (Unix `uid`), or `None` when the platform
    /// has no such concept (Windows) or it could not be resolved. Used by the
    /// `UID` matcher.
    pub uid: Option<u32>,
}

/// Lookup from a connection's source socket to the local process that owns it,
/// used by the `PROCESS-NAME` / `PROCESS-PATH` matchers. Like [`GeoLookup`] and
/// [`RuleSetLookup`], the kernel neither owns the OS tables nor performs the
/// (platform-specific) socket→PID→executable resolution itself: the embedder
/// supplies an implementation, so the routing data plane only ever *queries*
/// the owning process for a connection.
pub trait ProcessLookup: Send + Sync {
    /// Resolve the process that owns the local socket identified by `src` on
    /// `network` (the inbound peer the connection arrived from), or `None` when
    /// no owning process can be found — e.g. the socket has already closed, the
    /// caller is on another host, or the OS lookup is unsupported.
    fn lookup(&self, network: ConnNetwork, src: SocketAddr) -> Option<ProcessInfo>;
}
