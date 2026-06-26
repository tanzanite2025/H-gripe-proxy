//! Routing predicates: the [`RuleMatcher`] tree and its evaluation.

use std::fmt;
use std::net::SocketAddr;
use std::sync::Arc;

use crate::address::TargetAddr;
use crate::conntrack::ConnNetwork;

use super::lookup::{GeoLookup, ProcessLookup, RuleSetLookup};
use super::types::{IpCidr, PortRange, UidRange};

/// A single routing predicate.
///
/// `Clone` is derived; `Debug`/`PartialEq`/`Eq` are hand-written because the
/// `GeoIp`/`GeoSite`/`Asn`/`SrcIpAsn` variants carry an `Arc<dyn GeoLookup>`, `RuleSet` /
/// `SrcRuleSet` an `Arc<dyn RuleSetLookup>`, and `ProcessName` / `ProcessPath` /
/// `Uid` an `Arc<dyn ProcessLookup>` trait object that cannot derive them. Two
/// such matchers are equal when they name the same code/ASN/set/pattern/range
/// and share the same underlying database/provider (compared by pointer).
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
    /// Matches when the connection's *source* IP is inside the CIDR block. The
    /// source address is supplied by the embedder at selection time (the
    /// inbound's peer); when it is unknown the rule never matches, mirroring
    /// `SRC-PORT` / `SRC-IP-RULE-SET`. Unlike [`IpCidr`](RuleMatcher::IpCidr),
    /// which tests the destination, this never depends on the (possibly
    /// unresolved) target, so it applies to IP and domain targets alike.
    SrcIpCidr(IpCidr),
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
    /// Matches when the connection's *source* IP is announced by the autonomous
    /// system number `asn`. Unlike [`Asn`](RuleMatcher::Asn), which queries the
    /// destination, this feeds the source address (supplied by the embedder at
    /// selection time) to the ASN database, so it never depends on the
    /// (possibly unresolved) target and applies to IP and domain targets alike.
    /// When the source is unknown the rule never matches, mirroring
    /// `SRC-IP-CIDR`.
    SrcIpAsn { asn: u32, db: Arc<dyn GeoLookup> },
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
    /// Matches when the local process that owns the connection runs under a
    /// user id inside the (inclusive) `range`. A single-uid rule parses as a
    /// one-wide range. The owning process is resolved by the embedder from the
    /// connection's source socket at selection time (like
    /// [`ProcessName`](RuleMatcher::ProcessName)); when the source is unknown,
    /// no process resolves, or the platform reports no uid (e.g. Windows), the
    /// rule never matches, mirroring how `SRC-IP-CIDR` only applies once the
    /// source is known.
    Uid {
        range: UidRange,
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
            RuleMatcher::SrcIpCidr(c) => f.debug_tuple("SrcIpCidr").field(c).finish(),
            RuleMatcher::GeoIp { code, .. } => f.debug_struct("GeoIp").field("code", code).finish_non_exhaustive(),
            RuleMatcher::GeoSite { code, .. } => f.debug_struct("GeoSite").field("code", code).finish_non_exhaustive(),
            RuleMatcher::Asn { asn, .. } => f.debug_struct("Asn").field("asn", asn).finish_non_exhaustive(),
            RuleMatcher::SrcIpAsn { asn, .. } => f.debug_struct("SrcIpAsn").field("asn", asn).finish_non_exhaustive(),
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
            RuleMatcher::Uid { range, .. } => f.debug_struct("Uid").field("range", range).finish_non_exhaustive(),
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
            (RuleMatcher::SrcIpCidr(a), RuleMatcher::SrcIpCidr(b)) => a == b,
            (RuleMatcher::GeoIp { code: a, db: da }, RuleMatcher::GeoIp { code: b, db: db2 }) => {
                a == b && Arc::ptr_eq(da, db2)
            }
            (RuleMatcher::GeoSite { code: a, db: da }, RuleMatcher::GeoSite { code: b, db: db2 }) => {
                a == b && Arc::ptr_eq(da, db2)
            }
            (RuleMatcher::Asn { asn: a, db: da }, RuleMatcher::Asn { asn: b, db: db2 }) => {
                a == b && Arc::ptr_eq(da, db2)
            }
            (RuleMatcher::SrcIpAsn { asn: a, db: da }, RuleMatcher::SrcIpAsn { asn: b, db: db2 }) => {
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
            (RuleMatcher::Uid { range: a, provider: pa }, RuleMatcher::Uid { range: b, provider: pb }) => {
                a == b && Arc::ptr_eq(pa, pb)
            }
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
    /// from source `src`. Only `NETWORK` rules depend on `network`; `SRC-IP-CIDR`,
    /// `SRC-PORT`, `SRC-IP-RULE-SET`, `SRC-IP-ASN`, `PROCESS-NAME` and
    /// `PROCESS-PATH` rules depend on `src`; every other matcher ignores them. `src` is `None` when the
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
            RuleMatcher::SrcIpCidr(cidr) => src.is_some_and(|addr| cidr.contains(addr.ip())),
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
            RuleMatcher::SrcIpAsn { asn, db } => src.is_some_and(|addr| db.asn_matches(*asn, addr.ip())),
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
            RuleMatcher::Uid { range, provider } => src.is_some_and(|addr| {
                provider
                    .lookup(network, addr)
                    .and_then(|info| info.uid)
                    .is_some_and(|uid| range.contains(uid))
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
            RuleMatcher::SrcIpCidr(_) => "SrcIpCidr",
            RuleMatcher::GeoIp { .. } => "GeoIP",
            RuleMatcher::GeoSite { .. } => "GeoSite",
            RuleMatcher::Asn { .. } => "IPASN",
            RuleMatcher::SrcIpAsn { .. } => "SrcIPASN",
            RuleMatcher::RuleSet { .. } => "RuleSet",
            RuleMatcher::SrcRuleSet { .. } => "SrcRuleSet",
            RuleMatcher::DstPort(_) => "DstPort",
            RuleMatcher::SrcPort(_) => "SrcPort",
            RuleMatcher::Network(_) => "Network",
            RuleMatcher::ProcessName { .. } => "Process",
            RuleMatcher::ProcessPath { .. } => "ProcessPath",
            RuleMatcher::Uid { .. } => "Uid",
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
            RuleMatcher::SrcIpCidr(c) => c.to_string(),
            RuleMatcher::GeoIp { code, .. } => code.clone(),
            RuleMatcher::GeoSite { code, .. } => code.clone(),
            RuleMatcher::Asn { asn, .. } => asn.to_string(),
            RuleMatcher::SrcIpAsn { asn, .. } => asn.to_string(),
            RuleMatcher::RuleSet { name, .. } => name.clone(),
            RuleMatcher::SrcRuleSet { name, .. } => name.clone(),
            RuleMatcher::DstPort(range) => range.to_string(),
            RuleMatcher::SrcPort(range) => range.to_string(),
            RuleMatcher::Network(n) => n.as_str().to_string(),
            RuleMatcher::ProcessName { name, .. } => name.clone(),
            RuleMatcher::ProcessPath { path, .. } => path.clone(),
            RuleMatcher::Uid { range, .. } => range.to_string(),
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
