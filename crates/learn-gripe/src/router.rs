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
//! Scope: `DOMAIN`, `DOMAIN-SUFFIX`, `DOMAIN-KEYWORD`, `IP-CIDR` (v4 and v6)
//! and `MATCH` are supported. `GEOIP` / `GEOSITE` need external mmdb / geosite
//! data and are intentionally left for a follow-up.

use std::collections::HashMap;
use std::net::IpAddr;

use anyhow::{Result, bail};

use crate::address::TargetAddr;
use crate::config::OutboundMode;

/// Built-in outbound name that connects straight to the target.
pub const DIRECT: &str = "DIRECT";
/// Built-in outbound name that refuses the connection.
pub const REJECT: &str = "REJECT";

/// A single routing predicate.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RuleMatcher {
    /// Exact (case-insensitive) domain match.
    Domain(String),
    /// Matches the domain itself or any subdomain of it.
    DomainSuffix(String),
    /// Matches when the domain contains the keyword (case-insensitive).
    DomainKeyword(String),
    /// Matches when the target is an IP inside the CIDR block.
    IpCidr(IpCidr),
    /// Catch-all: matches every target.
    Match,
}

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
            RuleMatcher::Match => true,
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
        let name = self
            .rules
            .iter()
            .find(|rule| rule.matcher.matches(target))
            .map(|rule| rule.outbound.as_str())
            .unwrap_or(&self.fallback);
        self.lookup(name).unwrap_or(&OutboundMode::Reject)
    }
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
