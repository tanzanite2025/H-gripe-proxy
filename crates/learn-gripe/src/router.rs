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
//! `MATCH`, plus `GEOIP` / `GEOSITE`. The geo matchers carry a shared
//! [`GeoLookup`] handle to a locally-maintained geo database (mmdb / geosite
//! `.dat`); the kernel never fetches geo data itself — the embedder loads the
//! local files and supplies the lookup, keeping data sourcing out of the data
//! plane.

use std::collections::HashMap;
use std::fmt;
use std::net::IpAddr;
use std::sync::Arc;

use anyhow::{Result, bail};

use crate::address::TargetAddr;
use crate::config::OutboundMode;

/// Lookup into a locally-maintained geo database, used by the `GEOIP` /
/// `GEOSITE` matchers. The kernel does not own or fetch this data: the embedder
/// loads the local mmdb / geosite files it maintains and provides an
/// implementation, so the routing data plane only ever *queries* the database.
pub trait GeoLookup: Send + Sync {
    /// Whether `ip` belongs to the geo country `code` (e.g. `"cn"`).
    fn geoip_matches(&self, code: &str, ip: IpAddr) -> bool;
    /// Whether `host` belongs to the geosite category `code` (e.g. `"google"`).
    fn geosite_matches(&self, code: &str, host: &str) -> bool;
}

/// Built-in outbound name that connects straight to the target.
pub const DIRECT: &str = "DIRECT";
/// Built-in outbound name that refuses the connection.
pub const REJECT: &str = "REJECT";

/// A single routing predicate.
///
/// `Clone` is derived; `Debug`/`PartialEq`/`Eq` are hand-written because the
/// `GeoIp`/`GeoSite` variants carry an `Arc<dyn GeoLookup>` trait object that
/// cannot derive them. Two geo matchers are equal when they name the same code
/// and share the same underlying database (compared by pointer).
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
    /// Catch-all: matches every target.
    Match,
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
            (RuleMatcher::Match, RuleMatcher::Match) => true,
            _ => false,
        }
    }
}

impl Eq for RuleMatcher {}

impl RuleMatcher {
    /// Whether this matcher applies to `target`. Domain matchers never match a
    /// raw-IP target and IP matchers never match an (unresolved) domain target,
    /// matching Clash semantics where `IP-CIDR` only applies once an address is
    /// known.
    pub fn matches(&self, target: &TargetAddr) -> bool {
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

    /// Select the outbound for `target`: the first matching rule's outbound, or
    /// the fallback. The name is guaranteed to resolve (checked in
    /// [`Router::new`]).
    pub fn select(&self, target: &TargetAddr) -> &OutboundMode {
        self.select_detailed(target).outbound
    }

    /// Like [`select`](Router::select) but also reports the chosen outbound's
    /// name and the rule that matched (if any), for connection bookkeeping.
    pub fn select_detailed<'a>(&'a self, target: &TargetAddr) -> Selection<'a> {
        let matched = self.rules.iter().find(|rule| rule.matcher.matches(target));
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
    /// covers any host containing `ad`; the `cdn` category covers `cdn.test`.
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
