//! Rule-based outbound selection.
//!
//! A [`Router`] holds a set of named outbounds plus an ordered list of
//! [`Rule`]s. For each connection the rules are evaluated top-to-bottom and the
//! first matching rule decides which named outbound the connection takes; if no
//! rule matches, the router's `fallback` outbound is used (the idiomatic Clash
//! `MATCH` catch-all can be expressed either as a trailing `MATCH` rule or via
//! `fallback`).
//!
//! Two outbound names are always available without being declared:
//! `DIRECT` (connect straight to the target) and `REJECT` (refuse the
//! connection). This mirrors Clash's built-in policies.
//!
//! Scope: `DOMAIN`, `DOMAIN-SUFFIX`, `DOMAIN-KEYWORD`, `IP-CIDR` (v4 and v6),
//! `DST-PORT`, `SRC-PORT`, `NETWORK`, `PROCESS-NAME`, `PROCESS-PATH`, `MATCH`,
//! plus `GEOIP` / `GEOSITE` / `IP-ASN` and `RULE-SET`. The geo matchers
//! carry a shared [`GeoLookup`] handle to a locally-maintained geo database
//! (mmdb / geosite `.dat`), `RULE-SET` carries a shared [`RuleSetLookup`]
//! handle to locally-loaded rule providers, and the process matchers carry a
//! shared [`ProcessLookup`] handle that maps a connection's source socket to
//! the owning local process; the kernel never fetches that data itself — the
//! embedder loads the local files / performs the OS lookup and supplies the
//! handle, keeping data sourcing out of the data plane.

use std::collections::HashMap;
use std::fmt;
use std::net::{IpAddr, SocketAddr};
use std::sync::Arc;

use anyhow::{Result, bail};

use crate::address::TargetAddr;
use crate::config::OutboundMode;
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

/// Built-in outbound name that connects straight to the target.
pub const DIRECT: &str = "DIRECT";
/// Built-in outbound name that refuses the connection.
pub const REJECT: &str = "REJECT";

/// A single routing predicate.
///
/// `Clone` is derived; `Debug`/`PartialEq`/`Eq` are hand-written because the
/// `GeoIp`/`GeoSite`/`Asn` variants carry an `Arc<dyn GeoLookup>`, `RuleSet` /
/// `SrcRuleSet` an `Arc<dyn RuleSetLookup>`, and `ProcessName` / `ProcessPath`
/// an `Arc<dyn ProcessLookup>` trait object that cannot derive them. Two such
/// matchers are equal when they name the same code/ASN/set/pattern and share
/// the same underlying database/provider (compared by pointer).
#[derive(Clone)]
pub enum RuleMatcher {
    /// Exact (case-insensitive) domain match.
    Domain(String),
    /// Matches the domain itself or any subdomain of it.
    DomainSuffix(String),
    /// Matches when the domain contains the keyword (case-insensitive).
    DomainKeyword(String),
    /// Matches when the target is an IP inside the CIDR block.
    IpCidr(IpCidr),
    /// Matches when the target IP belongs to the geo country `code`. Like
    /// `IpCidr`, it only applies to a resolved IP target, never a domain.
    GeoIp { code: String, db: Arc<dyn GeoLookup> },
    /// Matches when the target domain belongs to the geosite category `code`.
    /// Like the domain matchers, it never applies to a raw-IP target.
    GeoSite { code: String, db: Arc<dyn GeoLookup> },
    /// Matches when the target IP is announced by the autonomous system number
    /// `asn`. Like `IpCidr` / `GeoIp`, it only applies to a resolved IP target,
    /// never a domain.
    Asn { asn: u32, db: Arc<dyn GeoLookup> },
    /// Matches when the target is contained in the locally-loaded rule-set
    /// ("rule provider") named `name`. The set decides whether it matches the
    /// domain or the IP, so unlike the geo matchers it applies to either kind
    /// of target.
    RuleSet {
        name: String,
        provider: Arc<dyn RuleSetLookup>,
    },
    /// Matches when the connection's *source* IP is contained in the
    /// locally-loaded rule-set ("rule provider") named `name`. Unlike
    /// [`RuleSet`](RuleMatcher::RuleSet), which queries the target, this feeds
    /// the source address (supplied by the embedder at selection time) to the
    /// set as an IP target, so an ipcidr/classical set matches the source IP
    /// and a domain set never matches. When the source is unknown the rule
    /// never matches, mirroring `SRC-IP-CIDR`.
    SrcRuleSet {
        name: String,
        provider: Arc<dyn RuleSetLookup>,
    },
    /// Matches when the target's destination port falls inside the (inclusive)
    /// range. A single-port rule parses as a one-wide range. Applies to both IP
    /// and domain targets, since the port is known either way.
    DstPort(PortRange),
    /// Matches when the connection's *source* port falls inside the (inclusive)
    /// range. A single-port rule parses as a one-wide range. The source address
    /// is supplied by the embedder at selection time (the inbound's peer); when
    /// it is unknown the rule never matches, mirroring how `SRC-IP-CIDR` only
    /// applies once the source is known.
    SrcPort(PortRange),
    /// Matches when the connection's transport protocol equals this network
    /// (`tcp` / `udp`). The protocol is supplied by the embedder at selection
    /// time and does not depend on the target, so it applies to IP and domain
    /// targets alike.
    Network(ConnNetwork),
    /// Matches when the local process that owns the connection has this
    /// executable base name (e.g. `curl`, `chrome.exe`), compared
    /// case-insensitively. The owning process is resolved by the embedder from
    /// the connection's source socket at selection time; when the source is
    /// unknown or no process can be resolved the rule never matches, mirroring
    /// how `SRC-IP-CIDR` only applies once the source is known.
    ProcessName {
        name: String,
        provider: Arc<dyn ProcessLookup>,
    },
    /// Matches when the local process that owns the connection has this
    /// executable full path (e.g. `/usr/bin/curl`), compared case-insensitively.
    /// Like [`ProcessName`](RuleMatcher::ProcessName) the process is resolved
    /// from the source socket and the rule never matches without a resolvable
    /// source.
    ProcessPath {
        path: String,
        provider: Arc<dyn ProcessLookup>,
    },
    /// Combines its sub-matchers with a boolean operator, letting one rule mix
    /// the other predicates (`AND((DOMAIN-SUFFIX,example.com),(IP-CIDR,10.0.0.0/8))`).
    /// `And` matches when every sub-matcher matches, `Or` when any does, and
    /// `Not` when its single sub-matcher does not. Sub-matchers may themselves
    /// be logical, so the tree nests arbitrarily.
    Logical { op: LogicalOp, subs: Vec<RuleMatcher> },
    /// Catch-all: matches every target.
    Match,
}

/// Boolean operator for a [`RuleMatcher::Logical`] matcher.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogicalOp {
    /// Every sub-matcher must match.
    And,
    /// At least one sub-matcher must match.
    Or,
    /// The single sub-matcher must not match.
    Not,
}

impl fmt::Debug for RuleMatcher {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RuleMatcher::Domain(d) => f.debug_tuple("Domain").field(d).finish(),
            RuleMatcher::DomainSuffix(s) => f.debug_tuple("DomainSuffix").field(s).finish(),
            RuleMatcher::DomainKeyword(k) => f.debug_tuple("DomainKeyword").field(k).finish(),
            RuleMatcher::IpCidr(c) => f.debug_tuple("IpCidr").field(c).finish(),
            RuleMatcher::GeoIp { code, .. } => f.debug_struct("GeoIp").field("code", code).finish_non_exhaustive(),
            RuleMatcher::GeoSite { code, .. } => f.debug_struct("GeoSite").field("code", code).finish_non_exhaustive(),
            RuleMatcher::Asn { asn, .. } => f.debug_struct("Asn").field("asn", asn).finish_non_exhaustive(),
            RuleMatcher::RuleSet { name, .. } => f.debug_struct("RuleSet").field("name", name).finish_non_exhaustive(),
            RuleMatcher::SrcRuleSet { name, .. } => {
                f.debug_struct("SrcRuleSet").field("name", name).finish_non_exhaustive()
            }
            RuleMatcher::DstPort(r) => f.debug_tuple("DstPort").field(r).finish(),
            RuleMatcher::SrcPort(r) => f.debug_tuple("SrcPort").field(r).finish(),
            RuleMatcher::Network(n) => f.debug_tuple("Network").field(n).finish(),
            RuleMatcher::ProcessName { name, .. } => f
                .debug_struct("ProcessName")
                .field("name", name)
                .finish_non_exhaustive(),
            RuleMatcher::ProcessPath { path, .. } => f
                .debug_struct("ProcessPath")
                .field("path", path)
                .finish_non_exhaustive(),
            RuleMatcher::Logical { op, subs } => f.debug_struct("Logical").field("op", op).field("subs", subs).finish(),
            RuleMatcher::Match => f.write_str("Match"),
        }
    }
}

impl PartialEq for RuleMatcher {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (RuleMatcher::Domain(a), RuleMatcher::Domain(b)) => a == b,
            (RuleMatcher::DomainSuffix(a), RuleMatcher::DomainSuffix(b)) => a == b,
            (RuleMatcher::DomainKeyword(a), RuleMatcher::DomainKeyword(b)) => a == b,
            (RuleMatcher::IpCidr(a), RuleMatcher::IpCidr(b)) => a == b,
            (RuleMatcher::GeoIp { code: a, db: da }, RuleMatcher::GeoIp { code: b, db: db2 }) => {
                a == b && Arc::ptr_eq(da, db2)
            }
            (RuleMatcher::GeoSite { code: a, db: da }, RuleMatcher::GeoSite { code: b, db: db2 }) => {
                a == b && Arc::ptr_eq(da, db2)
            }
            (RuleMatcher::Asn { asn: a, db: da }, RuleMatcher::Asn { asn: b, db: db2 }) => {
                a == b && Arc::ptr_eq(da, db2)
            }
            (RuleMatcher::RuleSet { name: a, provider: pa }, RuleMatcher::RuleSet { name: b, provider: pb }) => {
                a == b && Arc::ptr_eq(pa, pb)
            }
            (RuleMatcher::SrcRuleSet { name: a, provider: pa }, RuleMatcher::SrcRuleSet { name: b, provider: pb }) => {
                a == b && Arc::ptr_eq(pa, pb)
            }
            (RuleMatcher::DstPort(a), RuleMatcher::DstPort(b)) => a == b,
            (RuleMatcher::SrcPort(a), RuleMatcher::SrcPort(b)) => a == b,
            (RuleMatcher::Network(a), RuleMatcher::Network(b)) => a == b,
            (
                RuleMatcher::ProcessName { name: a, provider: pa },
                RuleMatcher::ProcessName { name: b, provider: pb },
            ) => a == b && Arc::ptr_eq(pa, pb),
            (
                RuleMatcher::ProcessPath { path: a, provider: pa },
                RuleMatcher::ProcessPath { path: b, provider: pb },
            ) => a == b && Arc::ptr_eq(pa, pb),
            (RuleMatcher::Logical { op: oa, subs: sa }, RuleMatcher::Logical { op: ob, subs: sb }) => {
                oa == ob && sa == sb
            }
            (RuleMatcher::Match, RuleMatcher::Match) => true,
            _ => false,
        }
    }
}

impl Eq for RuleMatcher {}

impl RuleMatcher {
    /// Whether this matcher applies to `target` on a TCP connection. Domain
    /// matchers never match a raw-IP target and IP matchers never match an
    /// (unresolved) domain target, matching Clash semantics where `IP-CIDR`
    /// only applies once an address is known.
    ///
    /// This is a convenience wrapper over [`RuleMatcher::matches_network`] that
    /// assumes [`ConnNetwork::Tcp`]; use `matches_network` when the connection's
    /// protocol is known so that `NETWORK` rules evaluate correctly.
    pub fn matches(&self, target: &TargetAddr) -> bool {
        self.matches_network(target, ConnNetwork::Tcp)
    }

    /// Whether this matcher applies to `target` on a connection of the given
    /// `network` (transport protocol). Convenience wrapper over
    /// [`matches_conn`](RuleMatcher::matches_conn) with an unknown source, so
    /// `SRC-PORT` rules never match; use `matches_conn` when the source is
    /// known.
    pub fn matches_network(&self, target: &TargetAddr, network: ConnNetwork) -> bool {
        self.matches_conn(target, network, None)
    }

    /// Whether this matcher applies to a connection to `target` over `network`
    /// from source `src`. Only `NETWORK` rules depend on `network`; `SRC-PORT`,
    /// `SRC-IP-RULE-SET`, `PROCESS-NAME` and `PROCESS-PATH` rules depend on
    /// `src`; every other matcher ignores them. `src` is `None` when the
    /// embedder cannot supply the source (in which case those source-dependent
    /// rules never match). Logical sub-rules inherit the same `network` and
    /// `src`.
    pub fn matches_conn(&self, target: &TargetAddr, network: ConnNetwork, src: Option<SocketAddr>) -> bool {
        match self {
            RuleMatcher::Domain(d) => match target {
                TargetAddr::Domain(host, _) => host.eq_ignore_ascii_case(d),
                TargetAddr::Ip(_) => false,
            },
            RuleMatcher::DomainSuffix(suffix) => match target {
                TargetAddr::Domain(host, _) => domain_has_suffix(host, suffix),
                TargetAddr::Ip(_) => false,
            },
            RuleMatcher::DomainKeyword(keyword) => match target {
                TargetAddr::Domain(host, _) => host.to_ascii_lowercase().contains(&keyword.to_ascii_lowercase()),
                TargetAddr::Ip(_) => false,
            },
            RuleMatcher::IpCidr(cidr) => match target {
                TargetAddr::Ip(addr) => cidr.contains(addr.ip()),
                TargetAddr::Domain(_, _) => false,
            },
            RuleMatcher::GeoIp { code, db } => match target {
                TargetAddr::Ip(addr) => db.geoip_matches(code, addr.ip()),
                TargetAddr::Domain(_, _) => false,
            },
            RuleMatcher::GeoSite { code, db } => match target {
                TargetAddr::Domain(host, _) => db.geosite_matches(code, host),
                TargetAddr::Ip(_) => false,
            },
            RuleMatcher::Asn { asn, db } => match target {
                TargetAddr::Ip(addr) => db.asn_matches(*asn, addr.ip()),
                TargetAddr::Domain(_, _) => false,
            },
            RuleMatcher::RuleSet { name, provider } => provider.rule_set_matches(name, target),
            RuleMatcher::SrcRuleSet { name, provider } => {
                src.is_some_and(|addr| provider.rule_set_matches(name, &TargetAddr::Ip(addr)))
            }
            RuleMatcher::DstPort(range) => range.contains(target.port()),
            RuleMatcher::SrcPort(range) => src.is_some_and(|addr| range.contains(addr.port())),
            RuleMatcher::Network(want) => network == *want,
            RuleMatcher::ProcessName { name, provider } => src.is_some_and(|addr| {
                provider
                    .lookup(network, addr)
                    .is_some_and(|info| info.name.eq_ignore_ascii_case(name))
            }),
            RuleMatcher::ProcessPath { path, provider } => src.is_some_and(|addr| {
                provider
                    .lookup(network, addr)
                    .is_some_and(|info| info.path.eq_ignore_ascii_case(path))
            }),
            RuleMatcher::Logical { op, subs } => match op {
                LogicalOp::And => subs.iter().all(|m| m.matches_conn(target, network, src)),
                LogicalOp::Or => subs.iter().any(|m| m.matches_conn(target, network, src)),
                // Built only with exactly one sub-matcher (see the parser), so
                // negating "any matches" negates that single sub-matcher.
                LogicalOp::Not => !subs.iter().any(|m| m.matches_conn(target, network, src)),
            },
            RuleMatcher::Match => true,
        }
    }
}

impl RuleMatcher {
    /// Short, human-readable matcher type, matching the controller rule names
    /// the UI shows (e.g. `"DomainSuffix"`, `"GeoIP"`, `"Match"`).
    pub fn kind_str(&self) -> &'static str {
        match self {
            RuleMatcher::Domain(_) => "Domain",
            RuleMatcher::DomainSuffix(_) => "DomainSuffix",
            RuleMatcher::DomainKeyword(_) => "DomainKeyword",
            RuleMatcher::IpCidr(_) => "IpCidr",
            RuleMatcher::GeoIp { .. } => "GeoIP",
            RuleMatcher::GeoSite { .. } => "GeoSite",
            RuleMatcher::Asn { .. } => "IPASN",
            RuleMatcher::RuleSet { .. } => "RuleSet",
            RuleMatcher::SrcRuleSet { .. } => "SrcRuleSet",
            RuleMatcher::DstPort(_) => "DstPort",
            RuleMatcher::SrcPort(_) => "SrcPort",
            RuleMatcher::Network(_) => "Network",
            RuleMatcher::ProcessName { .. } => "Process",
            RuleMatcher::ProcessPath { .. } => "ProcessPath",
            RuleMatcher::Logical { op, .. } => match op {
                LogicalOp::And => "AND",
                LogicalOp::Or => "OR",
                LogicalOp::Not => "NOT",
            },
            RuleMatcher::Match => "Match",
        }
    }

    /// The matcher's payload as text (the domain, keyword, CIDR, or geo code);
    /// empty for the catch-all `Match`.
    pub fn payload(&self) -> String {
        match self {
            RuleMatcher::Domain(d) => d.clone(),
            RuleMatcher::DomainSuffix(s) => s.clone(),
            RuleMatcher::DomainKeyword(k) => k.clone(),
            RuleMatcher::IpCidr(c) => c.to_string(),
            RuleMatcher::GeoIp { code, .. } => code.clone(),
            RuleMatcher::GeoSite { code, .. } => code.clone(),
            RuleMatcher::Asn { asn, .. } => asn.to_string(),
            RuleMatcher::RuleSet { name, .. } => name.clone(),
            RuleMatcher::SrcRuleSet { name, .. } => name.clone(),
            RuleMatcher::DstPort(range) => range.to_string(),
            RuleMatcher::SrcPort(range) => range.to_string(),
            RuleMatcher::Network(n) => n.as_str().to_string(),
            RuleMatcher::ProcessName { name, .. } => name.clone(),
            RuleMatcher::ProcessPath { path, .. } => path.clone(),
            // The sub-rule expression is parsed away into the matcher tree and
            // not retained verbatim, so a logical matcher has no flat payload.
            RuleMatcher::Logical { .. } => String::new(),
            RuleMatcher::Match => String::new(),
        }
    }
}

/// `DOMAIN-SUFFIX` semantics: `example.com` matches `example.com` itself and
/// any `*.example.com`, but not `notexample.com`.
fn domain_has_suffix(host: &str, suffix: &str) -> bool {
    let host = host.to_ascii_lowercase();
    let suffix = suffix.to_ascii_lowercase();
    if host == suffix {
        return true;
    }
    host.len() > suffix.len() && host.ends_with(&suffix) && host.as_bytes()[host.len() - suffix.len() - 1] == b'.'
}

/// A CIDR block (`addr/prefix`) supporting both IPv4 and IPv6.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IpCidr {
    network: IpAddr,
    prefix: u8,
}

impl IpCidr {
    /// Parse a CIDR such as `10.0.0.0/8` or `2001:db8::/32`. A bare address
    /// (no `/prefix`) is treated as a host route (`/32` or `/128`).
    pub fn parse(s: &str) -> Result<Self> {
        let (addr_str, prefix) = match s.split_once('/') {
            Some((addr, prefix)) => {
                let prefix: u8 = prefix
                    .parse()
                    .map_err(|_| anyhow::anyhow!("invalid CIDR prefix in {s:?}"))?;
                (addr, Some(prefix))
            }
            None => (s, None),
        };
        let addr: IpAddr = addr_str
            .parse()
            .map_err(|_| anyhow::anyhow!("invalid CIDR address in {s:?}"))?;
        let max = match addr {
            IpAddr::V4(_) => 32,
            IpAddr::V6(_) => 128,
        };
        let prefix = prefix.unwrap_or(max);
        if prefix > max {
            bail!("CIDR prefix /{prefix} out of range for {addr}");
        }
        Ok(Self {
            network: masked(addr, prefix),
            prefix,
        })
    }

    /// Whether `ip` falls inside this block.
    pub fn contains(&self, ip: IpAddr) -> bool {
        match (self.network, ip) {
            (IpAddr::V4(_), IpAddr::V4(_)) | (IpAddr::V6(_), IpAddr::V6(_)) => masked(ip, self.prefix) == self.network,
            _ => false,
        }
    }
}

impl fmt::Display for IpCidr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}/{}", self.network, self.prefix)
    }
}

/// An inclusive destination-port range used by the `DST-PORT` matcher. A single
/// port (`443`) parses as a one-wide range (`start == end`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PortRange {
    start: u16,
    end: u16,
}

impl PortRange {
    /// Parse a single port (`443`) or an inclusive range (`8000-9000`). The
    /// bounds must be valid `u16`s and `start` must not exceed `end`.
    pub fn parse(s: &str) -> Result<Self> {
        let s = s.trim();
        let (start, end) = match s.split_once('-') {
            Some((a, b)) => {
                let start = a
                    .trim()
                    .parse()
                    .map_err(|_| anyhow::anyhow!("invalid port range start in {s:?}"))?;
                let end = b
                    .trim()
                    .parse()
                    .map_err(|_| anyhow::anyhow!("invalid port range end in {s:?}"))?;
                (start, end)
            }
            None => {
                let port = s.parse().map_err(|_| anyhow::anyhow!("invalid port in {s:?}"))?;
                (port, port)
            }
        };
        if start > end {
            bail!("port range start {start} exceeds end {end}");
        }
        Ok(Self { start, end })
    }

    /// Whether `port` falls inside this inclusive range.
    pub fn contains(&self, port: u16) -> bool {
        self.start <= port && port <= self.end
    }
}

impl fmt::Display for PortRange {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.start == self.end {
            write!(f, "{}", self.start)
        } else {
            write!(f, "{}-{}", self.start, self.end)
        }
    }
}

/// Zero out the host bits of `addr` below `prefix`.
fn masked(addr: IpAddr, prefix: u8) -> IpAddr {
    match addr {
        IpAddr::V4(v4) => {
            let bits = u32::from(v4);
            let mask = if prefix == 0 { 0 } else { u32::MAX << (32 - prefix) };
            IpAddr::V4((bits & mask).into())
        }
        IpAddr::V6(v6) => {
            let bits = u128::from(v6);
            let mask = if prefix == 0 { 0 } else { u128::MAX << (128 - prefix) };
            IpAddr::V6((bits & mask).into())
        }
    }
}

/// A rule: a predicate and the name of the outbound to use when it matches.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rule {
    pub matcher: RuleMatcher,
    pub outbound: String,
}

impl Rule {
    pub fn new(matcher: RuleMatcher, outbound: impl Into<String>) -> Self {
        Self {
            matcher,
            outbound: outbound.into(),
        }
    }
}

/// Rule-based outbound selector. Build with [`Router::new`], which validates
/// that every referenced outbound name resolves.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Router {
    outbounds: HashMap<String, OutboundMode>,
    rules: Vec<Rule>,
    fallback: String,
}

impl Router {
    /// Build a router from named `outbounds`, ordered `rules`, and a `fallback`
    /// outbound name used when no rule matches. The built-in `DIRECT` and
    /// `REJECT` names are always resolvable and may be referenced without being
    /// present in `outbounds`. Returns an error if any rule target or the
    /// fallback names an outbound that does not resolve.
    pub fn new(
        outbounds: HashMap<String, OutboundMode>,
        rules: Vec<Rule>,
        fallback: impl Into<String>,
    ) -> Result<Self> {
        let fallback = fallback.into();
        let router = Self {
            outbounds,
            rules,
            fallback,
        };
        for rule in &router.rules {
            if router.lookup(&rule.outbound).is_none() {
                bail!("router: rule references unknown outbound {:?}", rule.outbound);
            }
        }
        if router.lookup(&router.fallback).is_none() {
            bail!("router: fallback references unknown outbound {:?}", router.fallback);
        }
        Ok(router)
    }

    /// Resolve an outbound name, honouring the built-in `DIRECT` / `REJECT`
    /// policies. Built-ins are shadowed if explicitly declared in `outbounds`.
    fn lookup<'a>(&'a self, name: &str) -> Option<&'a OutboundMode> {
        if let Some(mode) = self.outbounds.get(name) {
            return Some(mode);
        }
        match name {
            DIRECT => Some(&OutboundMode::Direct),
            REJECT => Some(&OutboundMode::Reject),
            _ => None,
        }
    }

    /// The distinct named outbounds this router can select. The built-in
    /// `DIRECT`/`REJECT` policies carry no server and are not included.
    pub fn outbound_modes(&self) -> impl Iterator<Item = &OutboundMode> {
        self.outbounds.values()
    }

    /// Select the outbound for `target` on a TCP connection: the first matching
    /// rule's outbound, or the fallback. The name is guaranteed to resolve
    /// (checked in [`Router::new`]).
    ///
    /// Convenience wrapper over [`select_network`](Router::select_network) that
    /// assumes [`ConnNetwork::Tcp`]; use `select_network` when the connection's
    /// protocol is known so that `NETWORK` rules evaluate correctly.
    pub fn select(&self, target: &TargetAddr) -> &OutboundMode {
        self.select_network(target, ConnNetwork::Tcp)
    }

    /// Select the outbound for `target` on a connection of the given `network`
    /// (transport protocol). Behaves like [`select`](Router::select) but lets
    /// `NETWORK` rules match the protocol. Convenience wrapper over
    /// [`select_conn`](Router::select_conn) with an unknown source.
    pub fn select_network(&self, target: &TargetAddr, network: ConnNetwork) -> &OutboundMode {
        self.select_conn(target, network, None)
    }

    /// Select the outbound for a connection to `target` over `network` from
    /// source `src`. Behaves like [`select_network`](Router::select_network)
    /// but also lets `SRC-PORT` rules match the source port. `src` is `None`
    /// when the embedder cannot supply the source.
    pub fn select_conn(&self, target: &TargetAddr, network: ConnNetwork, src: Option<SocketAddr>) -> &OutboundMode {
        self.select_detailed_conn(target, network, src).outbound
    }

    /// Like [`select`](Router::select) but also reports the chosen outbound's
    /// name and the rule that matched (if any), for connection bookkeeping.
    pub fn select_detailed<'a>(&'a self, target: &TargetAddr) -> Selection<'a> {
        self.select_detailed_network(target, ConnNetwork::Tcp)
    }

    /// Like [`select_detailed`](Router::select_detailed) but for a connection of
    /// the given `network`, so `NETWORK` rules evaluate against the protocol.
    /// Convenience wrapper over [`select_detailed_conn`](Router::select_detailed_conn)
    /// with an unknown source.
    pub fn select_detailed_network<'a>(&'a self, target: &TargetAddr, network: ConnNetwork) -> Selection<'a> {
        self.select_detailed_conn(target, network, None)
    }

    /// Like [`select_detailed_network`](Router::select_detailed_network) but
    /// also carries the connection's source `src`, so `SRC-PORT` rules evaluate
    /// against the source port. `src` is `None` when the embedder cannot supply
    /// the source.
    pub fn select_detailed_conn<'a>(
        &'a self,
        target: &TargetAddr,
        network: ConnNetwork,
        src: Option<SocketAddr>,
    ) -> Selection<'a> {
        let matched = self
            .rules
            .iter()
            .find(|rule| rule.matcher.matches_conn(target, network, src));
        let name = matched.map(|rule| rule.outbound.as_str()).unwrap_or(&self.fallback);
        Selection {
            outbound_name: name,
            outbound: self.lookup(name).unwrap_or(&OutboundMode::Reject),
            rule: matched,
        }
    }
}

/// The outcome of resolving a [`Router`] for one target: which outbound was
/// chosen, its name, and the rule that selected it (`None` when the fallback
/// was used).
#[derive(Debug)]
pub struct Selection<'a> {
    pub outbound_name: &'a str,
    pub outbound: &'a OutboundMode,
    pub rule: Option<&'a Rule>,
}

#[cfg(test)]
mod tests {
    use std::net::{Ipv4Addr, Ipv6Addr, SocketAddr};

    use super::*;

    fn domain(host: &str) -> TargetAddr {
        TargetAddr::Domain(host.to_string(), 443)
    }

    fn ipv4(a: u8, b: u8, c: u8, d: u8) -> TargetAddr {
        TargetAddr::Ip(SocketAddr::new(Ipv4Addr::new(a, b, c, d).into(), 80))
    }

    #[test]
    fn domain_exact_is_case_insensitive_and_strict() {
        let m = RuleMatcher::Domain("Example.com".to_string());
        assert!(m.matches(&domain("example.com")));
        assert!(m.matches(&domain("EXAMPLE.COM")));
        assert!(!m.matches(&domain("www.example.com")));
        assert!(!m.matches(&ipv4(1, 2, 3, 4)));
    }

    #[test]
    fn domain_suffix_matches_self_and_subdomains_only() {
        let m = RuleMatcher::DomainSuffix("example.com".to_string());
        assert!(m.matches(&domain("example.com")));
        assert!(m.matches(&domain("www.example.com")));
        assert!(m.matches(&domain("a.b.example.com")));
        assert!(!m.matches(&domain("notexample.com")));
        assert!(!m.matches(&domain("example.com.evil.net")));
    }

    #[test]
    fn domain_keyword_substring() {
        let m = RuleMatcher::DomainKeyword("goog".to_string());
        assert!(m.matches(&domain("www.GOOGLE.com")));
        assert!(!m.matches(&domain("example.com")));
    }

    #[test]
    fn ipv4_cidr_contains() {
        let cidr = IpCidr::parse("10.0.0.0/8").unwrap();
        assert!(cidr.contains(Ipv4Addr::new(10, 1, 2, 3).into()));
        assert!(!cidr.contains(Ipv4Addr::new(11, 0, 0, 1).into()));
        // v4 CIDR never matches a v6 address.
        assert!(!cidr.contains(Ipv6Addr::LOCALHOST.into()));
    }

    #[test]
    fn ipv6_cidr_contains() {
        let cidr = IpCidr::parse("2001:db8::/32").unwrap();
        assert!(cidr.contains("2001:db8::1".parse::<Ipv6Addr>().unwrap().into()));
        assert!(!cidr.contains("2001:dead::1".parse::<Ipv6Addr>().unwrap().into()));
    }

    #[test]
    fn bare_address_is_host_route() {
        let cidr = IpCidr::parse("192.168.1.5").unwrap();
        assert!(cidr.contains(Ipv4Addr::new(192, 168, 1, 5).into()));
        assert!(!cidr.contains(Ipv4Addr::new(192, 168, 1, 6).into()));
    }

    #[test]
    fn rejects_bad_cidr() {
        assert!(IpCidr::parse("10.0.0.0/33").is_err());
        assert!(IpCidr::parse("not-an-ip/8").is_err());
        assert!(IpCidr::parse("::/129").is_err());
    }

    #[test]
    fn ip_cidr_matcher_ignores_domain_targets() {
        let m = RuleMatcher::IpCidr(IpCidr::parse("0.0.0.0/0").unwrap());
        assert!(m.matches(&ipv4(8, 8, 8, 8)));
        assert!(!m.matches(&domain("example.com")));
    }

    /// Test geo database: `cn` covers the 1.0.0.0/8 block; the `ads` category
    /// covers any host containing `ad`; the `cdn` category covers `cdn.test`;
    /// AS13335 covers the 1.0.0.0/8 block.
    #[derive(Debug)]
    struct FakeGeo;

    impl GeoLookup for FakeGeo {
        fn geoip_matches(&self, code: &str, ip: IpAddr) -> bool {
            code == "cn" && matches!(ip, IpAddr::V4(v4) if v4.octets()[0] == 1)
        }

        fn geosite_matches(&self, code: &str, host: &str) -> bool {
            match code {
                "ads" => host.contains("ad"),
                "cdn" => host == "cdn.test",
                _ => false,
            }
        }

        fn asn_matches(&self, asn: u32, ip: IpAddr) -> bool {
            asn == 13335 && matches!(ip, IpAddr::V4(v4) if v4.octets()[0] == 1)
        }
    }

    /// Test rule-set provider: the `ads` set matches any host containing `ad`
    /// and the IP `1.0.0.1` (a mixed/classical set); every other set name
    /// matches nothing.
    #[derive(Debug)]
    struct FakeRuleSet;

    impl RuleSetLookup for FakeRuleSet {
        fn rule_set_matches(&self, name: &str, target: &TargetAddr) -> bool {
            if name != "ads" {
                return false;
            }
            match target {
                TargetAddr::Domain(host, _) => host.contains("ad"),
                TargetAddr::Ip(addr) => addr.ip() == IpAddr::V4(Ipv4Addr::new(1, 0, 0, 1)),
            }
        }
    }

    #[test]
    fn rule_set_matcher_delegates_to_provider_for_domain_and_ip() {
        let provider: Arc<dyn RuleSetLookup> = Arc::new(FakeRuleSet);
        let m = RuleMatcher::RuleSet {
            name: "ads".to_string(),
            provider,
        };
        // The provider decides per target kind, so both a matching domain and
        // a matching IP hit the same set.
        assert!(m.matches(&domain("ads.example")));
        assert!(m.matches(&TargetAddr::Ip(SocketAddr::new(Ipv4Addr::new(1, 0, 0, 1).into(), 80))));
        // Non-members of either kind miss.
        assert!(!m.matches(&domain("good.example")));
        assert!(!m.matches(&ipv4(8, 8, 8, 8)));
    }

    #[test]
    fn rule_set_matcher_with_unknown_set_never_matches() {
        let provider: Arc<dyn RuleSetLookup> = Arc::new(FakeRuleSet);
        let m = RuleMatcher::RuleSet {
            name: "missing".to_string(),
            provider,
        };
        assert!(!m.matches(&domain("ads.example")));
        assert!(!m.matches(&ipv4(1, 0, 0, 1)));
    }

    #[test]
    fn rule_set_matcher_equality_compares_name_and_shared_provider() {
        let provider: Arc<dyn RuleSetLookup> = Arc::new(FakeRuleSet);
        let other: Arc<dyn RuleSetLookup> = Arc::new(FakeRuleSet);
        let a = RuleMatcher::RuleSet {
            name: "ads".to_string(),
            provider: Arc::clone(&provider),
        };
        let same = RuleMatcher::RuleSet {
            name: "ads".to_string(),
            provider: Arc::clone(&provider),
        };
        let different_provider = RuleMatcher::RuleSet {
            name: "ads".to_string(),
            provider: other,
        };
        let different_name = RuleMatcher::RuleSet {
            name: "cdn".to_string(),
            provider,
        };
        assert_eq!(a, same);
        assert_ne!(a, different_provider);
        assert_ne!(a, different_name);
    }

    #[test]
    fn src_rule_set_matches_source_ip_via_provider() {
        let provider: Arc<dyn RuleSetLookup> = Arc::new(FakeRuleSet);
        let m = RuleMatcher::SrcRuleSet {
            name: "ads".to_string(),
            provider,
        };
        // The source IP is fed to the set as an IP target, so the `ads` set's
        // member 1.0.0.1 matches regardless of the destination (here a domain).
        assert!(m.matches_conn(
            &domain("good.example"),
            ConnNetwork::Tcp,
            Some(SocketAddr::new(Ipv4Addr::new(1, 0, 0, 1).into(), 50000)),
        ));
        // A non-member source IP misses even though the destination is fine.
        assert!(!m.matches_conn(
            &domain("good.example"),
            ConnNetwork::Tcp,
            Some(SocketAddr::new(Ipv4Addr::new(8, 8, 8, 8).into(), 50000)),
        ));
    }

    #[test]
    fn src_rule_set_with_unknown_set_never_matches() {
        let provider: Arc<dyn RuleSetLookup> = Arc::new(FakeRuleSet);
        let m = RuleMatcher::SrcRuleSet {
            name: "missing".to_string(),
            provider,
        };
        assert!(!m.matches_conn(
            &domain("good.example"),
            ConnNetwork::Tcp,
            Some(SocketAddr::new(Ipv4Addr::new(1, 0, 0, 1).into(), 50000)),
        ));
    }

    #[test]
    fn src_rule_set_never_matches_without_a_known_source() {
        // Like `SRC-IP-CIDR`, the rule needs the source to apply; the
        // `matches`/`matches_network` wrappers (which pass `None`) always miss
        // even when the target IP would belong to the set.
        let provider: Arc<dyn RuleSetLookup> = Arc::new(FakeRuleSet);
        let m = RuleMatcher::SrcRuleSet {
            name: "ads".to_string(),
            provider,
        };
        assert!(!m.matches_conn(&domain("good.example"), ConnNetwork::Tcp, None));
        assert!(!m.matches_network(&ipv4(1, 0, 0, 1), ConnNetwork::Tcp));
        assert!(!m.matches(&ipv4(1, 0, 0, 1)));
    }

    #[test]
    fn router_routes_src_rule_set_rule() {
        let provider: Arc<dyn RuleSetLookup> = Arc::new(FakeRuleSet);
        let mut outbounds = HashMap::new();
        outbounds.insert(
            "proxy".to_string(),
            OutboundMode::Socks5Upstream {
                addr: SocketAddr::from((Ipv4Addr::LOCALHOST, 1080)),
            },
        );
        let rules = vec![Rule::new(
            RuleMatcher::SrcRuleSet {
                name: "ads".to_string(),
                provider,
            },
            DIRECT,
        )];
        let router = Router::new(outbounds, rules, "proxy").unwrap();

        // Source IP in the `ads` set -> DIRECT.
        assert!(matches!(
            router.select_conn(
                &domain("example.com"),
                ConnNetwork::Tcp,
                Some(SocketAddr::new(Ipv4Addr::new(1, 0, 0, 1).into(), 50000)),
            ),
            OutboundMode::Direct
        ));
        // Source IP not in the set -> fallback proxy.
        assert!(matches!(
            router.select_conn(
                &domain("example.com"),
                ConnNetwork::Tcp,
                Some(SocketAddr::new(Ipv4Addr::new(8, 8, 8, 8).into(), 50000)),
            ),
            OutboundMode::Socks5Upstream { .. }
        ));
        // No source (the `select`/`select_network` wrappers) -> never matches.
        assert!(matches!(
            router.select_network(&domain("example.com"), ConnNetwork::Tcp),
            OutboundMode::Socks5Upstream { .. }
        ));
    }

    #[test]
    fn src_rule_set_as_logical_sub_rule_respects_source() {
        // AND,((DOMAIN-SUFFIX,example.com),(SRC-IP-RULE-SET,ads)) matches only a
        // connection to that domain from a source IP in the `ads` set; the
        // source threads into sub-rules.
        let provider: Arc<dyn RuleSetLookup> = Arc::new(FakeRuleSet);
        let m = RuleMatcher::Logical {
            op: LogicalOp::And,
            subs: vec![
                RuleMatcher::DomainSuffix("example.com".to_string()),
                RuleMatcher::SrcRuleSet {
                    name: "ads".to_string(),
                    provider,
                },
            ],
        };
        let member = SocketAddr::new(Ipv4Addr::new(1, 0, 0, 1).into(), 50000);
        let other = SocketAddr::new(Ipv4Addr::new(8, 8, 8, 8).into(), 50000);
        assert!(m.matches_conn(&domain("www.example.com"), ConnNetwork::Tcp, Some(member)));
        assert!(!m.matches_conn(&domain("www.example.com"), ConnNetwork::Tcp, Some(other)));
        assert!(!m.matches_conn(&domain("other.net"), ConnNetwork::Tcp, Some(member)));
        // Without a known source the SRC-IP-RULE-SET sub-rule cannot match.
        assert!(!m.matches_conn(&domain("www.example.com"), ConnNetwork::Tcp, None));
    }

    #[test]
    fn src_rule_set_equality_and_metadata() {
        let provider: Arc<dyn RuleSetLookup> = Arc::new(FakeRuleSet);
        let other: Arc<dyn RuleSetLookup> = Arc::new(FakeRuleSet);
        let a = RuleMatcher::SrcRuleSet {
            name: "ads".to_string(),
            provider: Arc::clone(&provider),
        };
        let same = RuleMatcher::SrcRuleSet {
            name: "ads".to_string(),
            provider: Arc::clone(&provider),
        };
        let different_provider = RuleMatcher::SrcRuleSet {
            name: "ads".to_string(),
            provider: other,
        };
        let different_name = RuleMatcher::SrcRuleSet {
            name: "cdn".to_string(),
            provider,
        };
        assert_eq!(a, same);
        assert_ne!(a, different_provider);
        assert_ne!(a, different_name);
        assert_eq!(a.kind_str(), "SrcRuleSet");
        assert_eq!(a.payload(), "ads");
        // A SrcRuleSet never equals a target-side RuleSet, even with the same
        // name and provider.
        let provider2: Arc<dyn RuleSetLookup> = Arc::new(FakeRuleSet);
        let dst = RuleMatcher::RuleSet {
            name: "ads".to_string(),
            provider: Arc::clone(&provider2),
        };
        let src = RuleMatcher::SrcRuleSet {
            name: "ads".to_string(),
            provider: provider2,
        };
        assert_ne!(dst, src);
    }

    /// Resolves a single well-known source socket to `curl` at `/usr/bin/curl`,
    /// but only for TCP; everything else (other ports, UDP, unknown sockets)
    /// resolves to no process, exercising the network + source dependence.
    #[derive(Debug)]
    struct FakeProcess;

    impl ProcessLookup for FakeProcess {
        fn lookup(&self, network: ConnNetwork, src: SocketAddr) -> Option<ProcessInfo> {
            if network == ConnNetwork::Tcp && src.port() == 50000 {
                Some(ProcessInfo {
                    name: "curl".to_string(),
                    path: "/usr/bin/curl".to_string(),
                })
            } else {
                None
            }
        }
    }

    fn src_port(port: u16) -> Option<SocketAddr> {
        Some(SocketAddr::new(Ipv4Addr::LOCALHOST.into(), port))
    }

    #[test]
    fn process_name_matches_owning_process_case_insensitively() {
        let provider: Arc<dyn ProcessLookup> = Arc::new(FakeProcess);
        // Pattern casing is ignored, mirroring Mihomo's case-insensitive match.
        let m = RuleMatcher::ProcessName {
            name: "CURL".to_string(),
            provider,
        };
        assert!(m.matches_conn(&domain("example.com"), ConnNetwork::Tcp, src_port(50000)));
        // A different process name misses.
        let provider: Arc<dyn ProcessLookup> = Arc::new(FakeProcess);
        let other = RuleMatcher::ProcessName {
            name: "wget".to_string(),
            provider,
        };
        assert!(!other.matches_conn(&domain("example.com"), ConnNetwork::Tcp, src_port(50000)));
    }

    #[test]
    fn process_path_matches_full_executable_path() {
        let provider: Arc<dyn ProcessLookup> = Arc::new(FakeProcess);
        let m = RuleMatcher::ProcessPath {
            path: "/usr/bin/curl".to_string(),
            provider,
        };
        assert!(m.matches_conn(&ipv4(8, 8, 8, 8), ConnNetwork::Tcp, src_port(50000)));
        // The base name alone is not the full path, so a name fed to a PATH
        // rule misses.
        let provider: Arc<dyn ProcessLookup> = Arc::new(FakeProcess);
        let name_only = RuleMatcher::ProcessPath {
            path: "curl".to_string(),
            provider,
        };
        assert!(!name_only.matches_conn(&ipv4(8, 8, 8, 8), ConnNetwork::Tcp, src_port(50000)));
    }

    #[test]
    fn process_rules_never_match_without_a_resolvable_source() {
        let provider: Arc<dyn ProcessLookup> = Arc::new(FakeProcess);
        let m = RuleMatcher::ProcessName {
            name: "curl".to_string(),
            provider,
        };
        // No source at all -> the `matches`/`matches_network` wrappers miss.
        assert!(!m.matches_conn(&domain("example.com"), ConnNetwork::Tcp, None));
        assert!(!m.matches_network(&domain("example.com"), ConnNetwork::Tcp));
        assert!(!m.matches(&domain("example.com")));
        // A source the provider cannot resolve (wrong port) misses too.
        assert!(!m.matches_conn(&domain("example.com"), ConnNetwork::Tcp, src_port(40000)));
        // The provider only knows the TCP socket, so the same source over UDP
        // resolves to no process and misses.
        assert!(!m.matches_conn(&domain("example.com"), ConnNetwork::Udp, src_port(50000)));
    }

    #[test]
    fn router_routes_process_name_rule() {
        let provider: Arc<dyn ProcessLookup> = Arc::new(FakeProcess);
        let mut outbounds = HashMap::new();
        outbounds.insert(
            "proxy".to_string(),
            OutboundMode::Socks5Upstream {
                addr: SocketAddr::from((Ipv4Addr::LOCALHOST, 1080)),
            },
        );
        let rules = vec![Rule::new(
            RuleMatcher::ProcessName {
                name: "curl".to_string(),
                provider,
            },
            DIRECT,
        )];
        let router = Router::new(outbounds, rules, "proxy").unwrap();

        // A connection owned by `curl` -> DIRECT.
        assert!(matches!(
            router.select_conn(&domain("example.com"), ConnNetwork::Tcp, src_port(50000)),
            OutboundMode::Direct
        ));
        // An unresolvable source -> fallback proxy.
        assert!(matches!(
            router.select_conn(&domain("example.com"), ConnNetwork::Tcp, src_port(40000)),
            OutboundMode::Socks5Upstream { .. }
        ));
        // No source (the `select`/`select_network` wrappers) -> never matches.
        assert!(matches!(
            router.select_network(&domain("example.com"), ConnNetwork::Tcp),
            OutboundMode::Socks5Upstream { .. }
        ));
    }

    #[test]
    fn process_name_as_logical_sub_rule_respects_source() {
        // AND,((DOMAIN-SUFFIX,example.com),(PROCESS-NAME,curl)) matches only a
        // connection to that domain from the `curl` process; both the target
        // and the source thread into the sub-rules.
        let provider: Arc<dyn ProcessLookup> = Arc::new(FakeProcess);
        let m = RuleMatcher::Logical {
            op: LogicalOp::And,
            subs: vec![
                RuleMatcher::DomainSuffix("example.com".to_string()),
                RuleMatcher::ProcessName {
                    name: "curl".to_string(),
                    provider,
                },
            ],
        };
        assert!(m.matches_conn(&domain("www.example.com"), ConnNetwork::Tcp, src_port(50000)));
        // Wrong domain misses despite the right process.
        assert!(!m.matches_conn(&domain("other.net"), ConnNetwork::Tcp, src_port(50000)));
        // Right domain but unresolvable process misses.
        assert!(!m.matches_conn(&domain("www.example.com"), ConnNetwork::Tcp, src_port(40000)));
        // Without a source the PROCESS-NAME sub-rule cannot match.
        assert!(!m.matches_conn(&domain("www.example.com"), ConnNetwork::Tcp, None));
    }

    #[test]
    fn process_equality_and_metadata() {
        let provider: Arc<dyn ProcessLookup> = Arc::new(FakeProcess);
        let other: Arc<dyn ProcessLookup> = Arc::new(FakeProcess);
        let a = RuleMatcher::ProcessName {
            name: "curl".to_string(),
            provider: Arc::clone(&provider),
        };
        let same = RuleMatcher::ProcessName {
            name: "curl".to_string(),
            provider: Arc::clone(&provider),
        };
        let different_provider = RuleMatcher::ProcessName {
            name: "curl".to_string(),
            provider: other,
        };
        let different_name = RuleMatcher::ProcessName {
            name: "wget".to_string(),
            provider: Arc::clone(&provider),
        };
        assert_eq!(a, same);
        assert_ne!(a, different_provider);
        assert_ne!(a, different_name);
        assert_eq!(a.kind_str(), "Process");
        assert_eq!(a.payload(), "curl");

        let path = RuleMatcher::ProcessPath {
            path: "/usr/bin/curl".to_string(),
            provider,
        };
        assert_eq!(path.kind_str(), "ProcessPath");
        assert_eq!(path.payload(), "/usr/bin/curl");
        // A ProcessName never equals a ProcessPath, even with related payloads.
        assert_ne!(a, path);
    }

    #[test]
    fn geoip_matcher_uses_lookup_and_ignores_domains() {
        let db: Arc<dyn GeoLookup> = Arc::new(FakeGeo);
        let m = RuleMatcher::GeoIp {
            code: "cn".to_string(),
            db,
        };
        assert!(m.matches(&ipv4(1, 2, 3, 4)));
        assert!(!m.matches(&ipv4(8, 8, 8, 8)));
        // A domain target is never matched by GEOIP (no resolved IP).
        assert!(!m.matches(&domain("example.com")));
    }

    #[test]
    fn geosite_matcher_uses_lookup_and_ignores_ips() {
        let db: Arc<dyn GeoLookup> = Arc::new(FakeGeo);
        let m = RuleMatcher::GeoSite {
            code: "cdn".to_string(),
            db,
        };
        assert!(m.matches(&domain("cdn.test")));
        assert!(!m.matches(&domain("www.test")));
        assert!(!m.matches(&ipv4(1, 2, 3, 4)));
    }

    #[test]
    fn asn_matcher_uses_lookup_and_ignores_domains() {
        let db: Arc<dyn GeoLookup> = Arc::new(FakeGeo);
        let m = RuleMatcher::Asn { asn: 13335, db };
        assert!(m.matches(&ipv4(1, 2, 3, 4)));
        // Wrong ASN and a non-AS13335 address both miss.
        assert!(!m.matches(&ipv4(8, 8, 8, 8)));
        // A domain target is never matched by IP-ASN (no resolved IP).
        assert!(!m.matches(&domain("example.com")));
    }

    #[test]
    fn asn_matcher_equality_compares_asn_and_shared_db() {
        let db: Arc<dyn GeoLookup> = Arc::new(FakeGeo);
        let other: Arc<dyn GeoLookup> = Arc::new(FakeGeo);
        let a = RuleMatcher::Asn {
            asn: 13335,
            db: Arc::clone(&db),
        };
        let same = RuleMatcher::Asn {
            asn: 13335,
            db: Arc::clone(&db),
        };
        let different_db = RuleMatcher::Asn { asn: 13335, db: other };
        let different_asn = RuleMatcher::Asn { asn: 15169, db };
        assert_eq!(a, same);
        assert_ne!(a, different_db);
        assert_ne!(a, different_asn);
    }

    #[test]
    fn geo_matcher_equality_compares_code_and_shared_db() {
        let db: Arc<dyn GeoLookup> = Arc::new(FakeGeo);
        let other: Arc<dyn GeoLookup> = Arc::new(FakeGeo);
        let a = RuleMatcher::GeoIp {
            code: "cn".to_string(),
            db: Arc::clone(&db),
        };
        let same = RuleMatcher::GeoIp {
            code: "cn".to_string(),
            db: Arc::clone(&db),
        };
        let different_db = RuleMatcher::GeoIp {
            code: "cn".to_string(),
            db: other,
        };
        let different_code = RuleMatcher::GeoIp {
            code: "us".to_string(),
            db,
        };
        assert_eq!(a, same);
        assert_ne!(a, different_db);
        assert_ne!(a, different_code);
    }

    #[test]
    fn router_routes_geo_rules() {
        let db: Arc<dyn GeoLookup> = Arc::new(FakeGeo);
        let mut outbounds = HashMap::new();
        outbounds.insert(
            "proxy".to_string(),
            OutboundMode::Socks5Upstream {
                addr: SocketAddr::from((Ipv4Addr::LOCALHOST, 1080)),
            },
        );
        let rules = vec![
            Rule::new(
                RuleMatcher::GeoSite {
                    code: "ads".to_string(),
                    db: Arc::clone(&db),
                },
                REJECT,
            ),
            Rule::new(
                RuleMatcher::GeoSite {
                    code: "cdn".to_string(),
                    db: Arc::clone(&db),
                },
                "proxy",
            ),
            Rule::new(
                RuleMatcher::GeoIp {
                    code: "cn".to_string(),
                    db,
                },
                DIRECT,
            ),
        ];
        let router = Router::new(outbounds, rules, "proxy").unwrap();

        assert!(matches!(router.select(&domain("ads.example")), OutboundMode::Reject));
        assert!(matches!(
            router.select(&domain("cdn.test")),
            OutboundMode::Socks5Upstream { .. }
        ));
        assert!(matches!(router.select(&ipv4(1, 1, 1, 1)), OutboundMode::Direct));
        // No geo rule matches a foreign IP -> fallback `proxy`.
        assert!(matches!(
            router.select(&ipv4(8, 8, 8, 8)),
            OutboundMode::Socks5Upstream { .. }
        ));
    }

    #[test]
    fn router_routes_asn_rule() {
        let db: Arc<dyn GeoLookup> = Arc::new(FakeGeo);
        let mut outbounds = HashMap::new();
        outbounds.insert(
            "proxy".to_string(),
            OutboundMode::Socks5Upstream {
                addr: SocketAddr::from((Ipv4Addr::LOCALHOST, 1080)),
            },
        );
        let rules = vec![Rule::new(RuleMatcher::Asn { asn: 13335, db }, DIRECT)];
        let router = Router::new(outbounds, rules, "proxy").unwrap();

        // An AS13335 IP routes DIRECT; everything else falls back to `proxy`.
        assert!(matches!(router.select(&ipv4(1, 1, 1, 1)), OutboundMode::Direct));
        assert!(matches!(
            router.select(&ipv4(8, 8, 8, 8)),
            OutboundMode::Socks5Upstream { .. }
        ));
        // A domain target never matches an IP-ASN rule.
        assert!(matches!(
            router.select(&domain("example.com")),
            OutboundMode::Socks5Upstream { .. }
        ));
    }

    #[test]
    fn router_routes_rule_set_rule() {
        let provider: Arc<dyn RuleSetLookup> = Arc::new(FakeRuleSet);
        let mut outbounds = HashMap::new();
        outbounds.insert(
            "proxy".to_string(),
            OutboundMode::Socks5Upstream {
                addr: SocketAddr::from((Ipv4Addr::LOCALHOST, 1080)),
            },
        );
        let rules = vec![Rule::new(
            RuleMatcher::RuleSet {
                name: "ads".to_string(),
                provider,
            },
            REJECT,
        )];
        let router = Router::new(outbounds, rules, "proxy").unwrap();

        // Both a domain and an IP member of the set are rejected; non-members
        // fall back to `proxy`.
        assert!(matches!(router.select(&domain("ads.example")), OutboundMode::Reject));
        assert!(matches!(router.select(&ipv4(1, 0, 0, 1)), OutboundMode::Reject));
        assert!(matches!(
            router.select(&domain("good.example")),
            OutboundMode::Socks5Upstream { .. }
        ));
    }

    #[test]
    fn logical_and_requires_every_sub_matcher() {
        let m = RuleMatcher::Logical {
            op: LogicalOp::And,
            subs: vec![
                RuleMatcher::DomainSuffix("example.com".to_string()),
                RuleMatcher::DomainKeyword("ads".to_string()),
            ],
        };
        // Both sub-matchers hit.
        assert!(m.matches(&domain("ads.example.com")));
        // Only the suffix hits.
        assert!(!m.matches(&domain("www.example.com")));
        // Only the keyword hits (different apex).
        assert!(!m.matches(&domain("ads.other.net")));
    }

    #[test]
    fn logical_or_needs_any_sub_matcher() {
        let m = RuleMatcher::Logical {
            op: LogicalOp::Or,
            subs: vec![
                RuleMatcher::DomainSuffix("example.com".to_string()),
                RuleMatcher::IpCidr(IpCidr::parse("10.0.0.0/8").unwrap()),
            ],
        };
        // The domain branch matches a domain target...
        assert!(m.matches(&domain("www.example.com")));
        // ...and the CIDR branch matches an IP target.
        assert!(m.matches(&ipv4(10, 1, 2, 3)));
        // Neither branch matches.
        assert!(!m.matches(&domain("other.net")));
        assert!(!m.matches(&ipv4(8, 8, 8, 8)));
    }

    #[test]
    fn logical_not_inverts_its_single_sub_matcher() {
        let m = RuleMatcher::Logical {
            op: LogicalOp::Not,
            subs: vec![RuleMatcher::DomainSuffix("example.com".to_string())],
        };
        assert!(!m.matches(&domain("www.example.com")));
        assert!(m.matches(&domain("other.net")));
        // A domain matcher never matches an IP target, so NOT of it does.
        assert!(m.matches(&ipv4(8, 8, 8, 8)));
    }

    #[test]
    fn logical_matchers_nest() {
        // OR( AND(suffix example.com, keyword ads), NOT(suffix example.com) )
        let m = RuleMatcher::Logical {
            op: LogicalOp::Or,
            subs: vec![
                RuleMatcher::Logical {
                    op: LogicalOp::And,
                    subs: vec![
                        RuleMatcher::DomainSuffix("example.com".to_string()),
                        RuleMatcher::DomainKeyword("ads".to_string()),
                    ],
                },
                RuleMatcher::Logical {
                    op: LogicalOp::Not,
                    subs: vec![RuleMatcher::DomainSuffix("example.com".to_string())],
                },
            ],
        };
        // Inner AND hits.
        assert!(m.matches(&domain("ads.example.com")));
        // Inner AND misses but NOT(example.com) hits.
        assert!(m.matches(&domain("foo.net")));
        // Under example.com without the keyword: AND misses and NOT misses.
        assert!(!m.matches(&domain("www.example.com")));
    }

    #[test]
    fn logical_equality_compares_op_and_subs() {
        let a = RuleMatcher::Logical {
            op: LogicalOp::And,
            subs: vec![RuleMatcher::DomainSuffix("example.com".to_string())],
        };
        let same = RuleMatcher::Logical {
            op: LogicalOp::And,
            subs: vec![RuleMatcher::DomainSuffix("example.com".to_string())],
        };
        let different_op = RuleMatcher::Logical {
            op: LogicalOp::Or,
            subs: vec![RuleMatcher::DomainSuffix("example.com".to_string())],
        };
        let different_subs = RuleMatcher::Logical {
            op: LogicalOp::And,
            subs: vec![RuleMatcher::DomainSuffix("other.net".to_string())],
        };
        assert_eq!(a, same);
        assert_ne!(a, different_op);
        assert_ne!(a, different_subs);
    }

    #[test]
    fn router_routes_logical_rule() {
        let mut outbounds = HashMap::new();
        outbounds.insert(
            "proxy".to_string(),
            OutboundMode::Socks5Upstream {
                addr: SocketAddr::from((Ipv4Addr::LOCALHOST, 1080)),
            },
        );
        // AND(suffix example.com, keyword ads) -> REJECT, else fallback proxy.
        let rules = vec![Rule::new(
            RuleMatcher::Logical {
                op: LogicalOp::And,
                subs: vec![
                    RuleMatcher::DomainSuffix("example.com".to_string()),
                    RuleMatcher::DomainKeyword("ads".to_string()),
                ],
            },
            REJECT,
        )];
        let router = Router::new(outbounds, rules, "proxy").unwrap();

        assert!(matches!(
            router.select(&domain("ads.example.com")),
            OutboundMode::Reject
        ));
        assert!(matches!(
            router.select(&domain("www.example.com")),
            OutboundMode::Socks5Upstream { .. }
        ));
    }

    #[test]
    fn router_picks_first_matching_rule_then_fallback() {
        let mut outbounds = HashMap::new();
        outbounds.insert(
            "proxy".to_string(),
            OutboundMode::Socks5Upstream {
                addr: SocketAddr::from((Ipv4Addr::LOCALHOST, 1080)),
            },
        );
        let rules = vec![
            Rule::new(RuleMatcher::DomainSuffix("ads.com".to_string()), REJECT),
            Rule::new(RuleMatcher::DomainSuffix("example.com".to_string()), "proxy"),
            Rule::new(RuleMatcher::IpCidr(IpCidr::parse("10.0.0.0/8").unwrap()), DIRECT),
        ];
        let router = Router::new(outbounds, rules, DIRECT).unwrap();

        assert!(matches!(router.select(&domain("x.ads.com")), OutboundMode::Reject));
        assert!(matches!(
            router.select(&domain("www.example.com")),
            OutboundMode::Socks5Upstream { .. }
        ));
        assert!(matches!(router.select(&ipv4(10, 1, 1, 1)), OutboundMode::Direct));
        // No rule matches a public IP -> fallback DIRECT.
        assert!(matches!(router.select(&ipv4(1, 1, 1, 1)), OutboundMode::Direct));
    }

    #[test]
    fn port_range_parses_single_and_range() {
        let single = PortRange::parse("443").unwrap();
        assert!(single.contains(443));
        assert!(!single.contains(442));
        assert_eq!(single.to_string(), "443");

        let range = PortRange::parse("8000-9000").unwrap();
        assert!(range.contains(8000) && range.contains(8500) && range.contains(9000));
        assert!(!range.contains(7999) && !range.contains(9001));
        assert_eq!(range.to_string(), "8000-9000");
    }

    #[test]
    fn port_range_rejects_bad_input() {
        assert!(PortRange::parse("").is_err());
        assert!(PortRange::parse("70000").is_err()); // out of u16 range
        assert!(PortRange::parse("80-").is_err());
        assert!(PortRange::parse("abc").is_err());
        // Inverted bounds are rejected.
        assert!(PortRange::parse("9000-8000").is_err());
    }

    #[test]
    fn dst_port_matcher_applies_to_ip_and_domain_targets() {
        let m = RuleMatcher::DstPort(PortRange::parse("443").unwrap());
        // The destination port is matched regardless of host kind.
        assert!(m.matches(&domain("example.com"))); // helper uses port 443
        assert!(m.matches(&TargetAddr::Ip(SocketAddr::new(Ipv4Addr::new(8, 8, 8, 8).into(), 443))));
        // A different port misses.
        assert!(!m.matches(&ipv4(8, 8, 8, 8))); // helper uses port 80
        assert!(!m.matches(&TargetAddr::Domain("example.com".to_string(), 80)));
    }

    #[test]
    fn dst_port_range_matches_inclusive_bounds() {
        let m = RuleMatcher::DstPort(PortRange::parse("8000-9000").unwrap());
        for port in [8000u16, 8443, 9000] {
            assert!(m.matches(&TargetAddr::Domain("h".to_string(), port)));
        }
        for port in [7999u16, 9001] {
            assert!(!m.matches(&TargetAddr::Domain("h".to_string(), port)));
        }
    }

    #[test]
    fn router_routes_dst_port_rule() {
        let mut outbounds = HashMap::new();
        outbounds.insert(
            "proxy".to_string(),
            OutboundMode::Socks5Upstream {
                addr: SocketAddr::from((Ipv4Addr::LOCALHOST, 1080)),
            },
        );
        let rules = vec![Rule::new(RuleMatcher::DstPort(PortRange::parse("80").unwrap()), DIRECT)];
        let router = Router::new(outbounds, rules, "proxy").unwrap();

        // Port 80 -> DIRECT (the ipv4 helper uses port 80).
        assert!(matches!(router.select(&ipv4(8, 8, 8, 8)), OutboundMode::Direct));
        // Port 443 (domain helper) -> no rule, fallback proxy.
        assert!(matches!(
            router.select(&domain("example.com")),
            OutboundMode::Socks5Upstream { .. }
        ));
    }

    #[test]
    fn dst_port_equality_compares_range() {
        let a = RuleMatcher::DstPort(PortRange::parse("80").unwrap());
        let b = RuleMatcher::DstPort(PortRange::parse("80").unwrap());
        let c = RuleMatcher::DstPort(PortRange::parse("80-90").unwrap());
        assert_eq!(a, b);
        assert_ne!(a, c);
    }

    fn src(port: u16) -> SocketAddr {
        SocketAddr::new(Ipv4Addr::LOCALHOST.into(), port)
    }

    #[test]
    fn src_port_matches_source_port_independent_of_target() {
        let m = RuleMatcher::SrcPort(PortRange::parse("12345").unwrap());
        // The source port is matched regardless of the destination host kind.
        assert!(m.matches_conn(&domain("example.com"), ConnNetwork::Tcp, Some(src(12345))));
        assert!(m.matches_conn(&ipv4(8, 8, 8, 8), ConnNetwork::Udp, Some(src(12345))));
        // A different source port misses.
        assert!(!m.matches_conn(&domain("example.com"), ConnNetwork::Tcp, Some(src(12346))));
    }

    #[test]
    fn src_port_never_matches_without_a_known_source() {
        // Mirrors how `SRC-IP-CIDR` only applies once the source is known: with
        // no source the rule cannot match, and the `matches`/`matches_network`
        // wrappers (which pass `None`) therefore always miss.
        let m = RuleMatcher::SrcPort(PortRange::parse("443").unwrap());
        assert!(!m.matches_conn(&domain("example.com"), ConnNetwork::Tcp, None));
        assert!(!m.matches_network(&domain("example.com"), ConnNetwork::Tcp));
        assert!(!m.matches(&domain("example.com")));
    }

    #[test]
    fn src_port_range_matches_inclusive_bounds() {
        let m = RuleMatcher::SrcPort(PortRange::parse("8000-9000").unwrap());
        for port in [8000u16, 8443, 9000] {
            assert!(m.matches_conn(&domain("h"), ConnNetwork::Tcp, Some(src(port))));
        }
        for port in [7999u16, 9001] {
            assert!(!m.matches_conn(&domain("h"), ConnNetwork::Tcp, Some(src(port))));
        }
    }

    #[test]
    fn router_routes_src_port_rule() {
        let mut outbounds = HashMap::new();
        outbounds.insert(
            "proxy".to_string(),
            OutboundMode::Socks5Upstream {
                addr: SocketAddr::from((Ipv4Addr::LOCALHOST, 1080)),
            },
        );
        let rules = vec![Rule::new(
            RuleMatcher::SrcPort(PortRange::parse("50000").unwrap()),
            DIRECT,
        )];
        let router = Router::new(outbounds, rules, "proxy").unwrap();

        // Source port 50000 -> DIRECT.
        assert!(matches!(
            router.select_conn(&domain("example.com"), ConnNetwork::Tcp, Some(src(50000))),
            OutboundMode::Direct
        ));
        // A different source port misses and uses the fallback proxy.
        assert!(matches!(
            router.select_conn(&domain("example.com"), ConnNetwork::Tcp, Some(src(40000))),
            OutboundMode::Socks5Upstream { .. }
        ));
        // No source (the `select`/`select_network` wrappers) -> never matches.
        assert!(matches!(
            router.select_network(&domain("example.com"), ConnNetwork::Tcp),
            OutboundMode::Socks5Upstream { .. }
        ));
    }

    #[test]
    fn src_port_as_logical_sub_rule_respects_source() {
        // AND,((DOMAIN-SUFFIX,example.com),(SRC-PORT,50000)) only matches a
        // connection to that domain from source port 50000; the source threads
        // into sub-rules.
        let m = RuleMatcher::Logical {
            op: LogicalOp::And,
            subs: vec![
                RuleMatcher::DomainSuffix("example.com".to_string()),
                RuleMatcher::SrcPort(PortRange::parse("50000").unwrap()),
            ],
        };
        assert!(m.matches_conn(&domain("www.example.com"), ConnNetwork::Tcp, Some(src(50000))));
        assert!(!m.matches_conn(&domain("www.example.com"), ConnNetwork::Tcp, Some(src(40000))));
        assert!(!m.matches_conn(&domain("other.net"), ConnNetwork::Tcp, Some(src(50000))));
        // Without a known source the SRC-PORT sub-rule cannot match, so the AND fails.
        assert!(!m.matches_conn(&domain("www.example.com"), ConnNetwork::Tcp, None));
    }

    #[test]
    fn src_port_equality_and_metadata() {
        let a = RuleMatcher::SrcPort(PortRange::parse("80").unwrap());
        let b = RuleMatcher::SrcPort(PortRange::parse("80").unwrap());
        let c = RuleMatcher::SrcPort(PortRange::parse("80-90").unwrap());
        assert_eq!(a, b);
        assert_ne!(a, c);
        assert_eq!(a.kind_str(), "SrcPort");
        assert_eq!(a.payload(), "80");
        assert_eq!(c.payload(), "80-90");
    }

    #[test]
    fn conn_network_parse_accepts_tcp_udp_case_insensitively() {
        assert_eq!(ConnNetwork::parse("tcp"), Some(ConnNetwork::Tcp));
        assert_eq!(ConnNetwork::parse("UDP"), Some(ConnNetwork::Udp));
        assert_eq!(ConnNetwork::parse(" Tcp "), Some(ConnNetwork::Tcp));
        assert_eq!(ConnNetwork::parse("sctp"), None);
        assert_eq!(ConnNetwork::parse(""), None);
    }

    #[test]
    fn network_matcher_compares_protocol_independent_of_target() {
        let tcp = RuleMatcher::Network(ConnNetwork::Tcp);
        // The protocol comes from the connection, not the target, so the same
        // matcher answers per `network` for both IP and domain targets.
        assert!(tcp.matches_network(&ipv4(8, 8, 8, 8), ConnNetwork::Tcp));
        assert!(tcp.matches_network(&domain("example.com"), ConnNetwork::Tcp));
        assert!(!tcp.matches_network(&ipv4(8, 8, 8, 8), ConnNetwork::Udp));
        assert!(!tcp.matches_network(&domain("example.com"), ConnNetwork::Udp));

        let udp = RuleMatcher::Network(ConnNetwork::Udp);
        assert!(udp.matches_network(&domain("example.com"), ConnNetwork::Udp));
        assert!(!udp.matches_network(&domain("example.com"), ConnNetwork::Tcp));
        // The bare `matches` wrapper assumes TCP.
        assert!(tcp.matches(&domain("example.com")));
        assert!(!udp.matches(&domain("example.com")));
    }

    #[test]
    fn router_routes_network_rule() {
        let mut outbounds = HashMap::new();
        outbounds.insert(
            "proxy".to_string(),
            OutboundMode::Socks5Upstream {
                addr: SocketAddr::from((Ipv4Addr::LOCALHOST, 1080)),
            },
        );
        // UDP -> REJECT, everything else falls back to proxy.
        let rules = vec![Rule::new(RuleMatcher::Network(ConnNetwork::Udp), REJECT)];
        let router = Router::new(outbounds, rules, "proxy").unwrap();

        // A UDP datagram to any target hits the rule.
        assert!(matches!(
            router.select_network(&domain("example.com"), ConnNetwork::Udp),
            OutboundMode::Reject
        ));
        // The same target over TCP misses and uses the fallback.
        assert!(matches!(
            router.select_network(&domain("example.com"), ConnNetwork::Tcp),
            OutboundMode::Socks5Upstream { .. }
        ));
        // The TCP-default `select` wrapper agrees.
        assert!(matches!(
            router.select(&domain("example.com")),
            OutboundMode::Socks5Upstream { .. }
        ));
    }

    #[test]
    fn network_rule_as_logical_sub_rule_respects_protocol() {
        // AND,((DOMAIN-SUFFIX,example.com),(NETWORK,UDP)) only matches a UDP
        // connection to that domain; the protocol threads into sub-rules.
        let m = RuleMatcher::Logical {
            op: LogicalOp::And,
            subs: vec![
                RuleMatcher::DomainSuffix("example.com".to_string()),
                RuleMatcher::Network(ConnNetwork::Udp),
            ],
        };
        assert!(m.matches_network(&domain("www.example.com"), ConnNetwork::Udp));
        assert!(!m.matches_network(&domain("www.example.com"), ConnNetwork::Tcp));
        assert!(!m.matches_network(&domain("other.net"), ConnNetwork::Udp));
    }

    #[test]
    fn network_equality_and_metadata() {
        let a = RuleMatcher::Network(ConnNetwork::Tcp);
        let b = RuleMatcher::Network(ConnNetwork::Tcp);
        let c = RuleMatcher::Network(ConnNetwork::Udp);
        assert_eq!(a, b);
        assert_ne!(a, c);
        assert_eq!(a.kind_str(), "Network");
        assert_eq!(a.payload(), "tcp");
        assert_eq!(c.payload(), "udp");
    }

    #[test]
    fn router_rejects_unknown_outbound_reference() {
        let rules = vec![Rule::new(RuleMatcher::Match, "ghost")];
        let err = Router::new(HashMap::new(), rules, DIRECT).unwrap_err();
        assert!(err.to_string().contains("unknown outbound"), "got: {err}");
    }

    #[test]
    fn router_rejects_unknown_fallback() {
        let err = Router::new(HashMap::new(), Vec::new(), "ghost").unwrap_err();
        assert!(err.to_string().contains("fallback"), "got: {err}");
    }
}
