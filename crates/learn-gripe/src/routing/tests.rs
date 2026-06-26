//! Behavioural tests for the routing module (matchers + Router selection).
use std::sync::Arc;

use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};

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
                uid: Some(1000),
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
fn uid_range_parses_single_and_range() {
    let one = UidRange::parse("1000").unwrap();
    assert!(one.contains(1000));
    assert!(!one.contains(999) && !one.contains(1001));
    let range = UidRange::parse("1000-2000").unwrap();
    assert!(range.contains(1000) && range.contains(1500) && range.contains(2000));
    assert!(!range.contains(999) && !range.contains(2001));
    assert_eq!(one.to_string(), "1000");
    assert_eq!(range.to_string(), "1000-2000");
    // Whitespace is tolerated; invalid/inverted bounds are rejected.
    assert!(UidRange::parse(" 1000 - 2000 ").is_ok());
    assert!(UidRange::parse("").is_err());
    assert!(UidRange::parse("abc").is_err());
    assert!(UidRange::parse("2000-1000").is_err());
}

#[test]
fn uid_matches_owning_process_uid() {
    // FakeProcess resolves the curl socket to uid 1000.
    let provider: Arc<dyn ProcessLookup> = Arc::new(FakeProcess);
    let m = RuleMatcher::Uid {
        range: UidRange::parse("1000").unwrap(),
        provider,
    };
    // The match is independent of the (here domain) target.
    assert!(m.matches_conn(&domain("example.com"), ConnNetwork::Tcp, src_port(50000)));
    assert!(m.matches_conn(&ipv4(8, 8, 8, 8), ConnNetwork::Tcp, src_port(50000)));
    // A different uid misses.
    let provider: Arc<dyn ProcessLookup> = Arc::new(FakeProcess);
    let other = RuleMatcher::Uid {
        range: UidRange::parse("0").unwrap(),
        provider,
    };
    assert!(!other.matches_conn(&domain("example.com"), ConnNetwork::Tcp, src_port(50000)));
}

#[test]
fn uid_rule_never_matches_without_a_resolvable_source() {
    let provider: Arc<dyn ProcessLookup> = Arc::new(FakeProcess);
    let m = RuleMatcher::Uid {
        range: UidRange::parse("1000").unwrap(),
        provider,
    };
    // No source at all -> the `matches`/`matches_network` wrappers miss.
    assert!(!m.matches_conn(&domain("example.com"), ConnNetwork::Tcp, None));
    assert!(!m.matches_network(&domain("example.com"), ConnNetwork::Tcp));
    assert!(!m.matches(&domain("example.com")));
    // A source the provider cannot resolve (wrong port) misses.
    assert!(!m.matches_conn(&domain("example.com"), ConnNetwork::Tcp, src_port(40000)));
    // The provider only knows the TCP socket, so the same source over UDP
    // resolves to no process and misses.
    assert!(!m.matches_conn(&domain("example.com"), ConnNetwork::Udp, src_port(50000)));
}

#[test]
fn router_routes_uid_rule() {
    let provider: Arc<dyn ProcessLookup> = Arc::new(FakeProcess);
    let mut outbounds = HashMap::new();
    outbounds.insert(
        "proxy".to_string(),
        OutboundMode::Socks5Upstream {
            addr: SocketAddr::from((Ipv4Addr::LOCALHOST, 1080)),
        },
    );
    let rules = vec![Rule::new(
        RuleMatcher::Uid {
            range: UidRange::parse("500-1500").unwrap(),
            provider,
        },
        DIRECT,
    )];
    let router = Router::new(outbounds, rules, "proxy").unwrap();

    // A connection owned by uid 1000 (inside 500-1500) -> DIRECT.
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
fn uid_as_logical_sub_rule_respects_source() {
    // AND,((DOMAIN-SUFFIX,example.com),(UID,1000)) matches only a connection
    // to that domain from a uid-1000 process; the source threads into the
    // sub-rule just like PROCESS-NAME.
    let provider: Arc<dyn ProcessLookup> = Arc::new(FakeProcess);
    let m = RuleMatcher::Logical {
        op: LogicalOp::And,
        subs: vec![
            RuleMatcher::DomainSuffix("example.com".to_string()),
            RuleMatcher::Uid {
                range: UidRange::parse("1000").unwrap(),
                provider,
            },
        ],
    };
    assert!(m.matches_conn(&domain("www.example.com"), ConnNetwork::Tcp, src_port(50000)));
    // Wrong domain misses despite the right uid.
    assert!(!m.matches_conn(&domain("other.net"), ConnNetwork::Tcp, src_port(50000)));
    // Right domain but unresolvable process misses.
    assert!(!m.matches_conn(&domain("www.example.com"), ConnNetwork::Tcp, src_port(40000)));
    // Without a source the UID sub-rule cannot match.
    assert!(!m.matches_conn(&domain("www.example.com"), ConnNetwork::Tcp, None));
}

#[test]
fn uid_equality_and_metadata() {
    let provider: Arc<dyn ProcessLookup> = Arc::new(FakeProcess);
    let other: Arc<dyn ProcessLookup> = Arc::new(FakeProcess);
    let a = RuleMatcher::Uid {
        range: UidRange::parse("1000").unwrap(),
        provider: Arc::clone(&provider),
    };
    let same = RuleMatcher::Uid {
        range: UidRange::parse("1000").unwrap(),
        provider: Arc::clone(&provider),
    };
    let different_provider = RuleMatcher::Uid {
        range: UidRange::parse("1000").unwrap(),
        provider: other,
    };
    let different_range = RuleMatcher::Uid {
        range: UidRange::parse("1000-2000").unwrap(),
        provider: Arc::clone(&provider),
    };
    assert_eq!(a, same);
    assert_ne!(a, different_provider);
    assert_ne!(a, different_range);
    assert_eq!(a.kind_str(), "Uid");
    assert_eq!(a.payload(), "1000");
    assert_eq!(different_range.payload(), "1000-2000");
    // A UID rule never equals a ProcessName rule.
    let name = RuleMatcher::ProcessName {
        name: "curl".to_string(),
        provider,
    };
    assert_ne!(a, name);
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
fn src_ip_cidr_matches_source_ip_independent_of_target() {
    let m = RuleMatcher::SrcIpCidr(IpCidr::parse("192.168.1.0/24").unwrap());
    let inside = SocketAddr::new(Ipv4Addr::new(192, 168, 1, 5).into(), 50000);
    let outside = SocketAddr::new(Ipv4Addr::new(10, 0, 0, 1).into(), 50000);
    // The source IP is matched regardless of the destination host kind.
    assert!(m.matches_conn(&domain("example.com"), ConnNetwork::Tcp, Some(inside)));
    assert!(m.matches_conn(&ipv4(8, 8, 8, 8), ConnNetwork::Udp, Some(inside)));
    // A source outside the block misses.
    assert!(!m.matches_conn(&domain("example.com"), ConnNetwork::Tcp, Some(outside)));
}

#[test]
fn src_ip_cidr_never_matches_without_a_known_source() {
    // With no source the rule cannot match, so the `matches` /
    // `matches_network` wrappers (which pass `None`) always miss.
    let m = RuleMatcher::SrcIpCidr(IpCidr::parse("0.0.0.0/0").unwrap());
    assert!(!m.matches_conn(&domain("example.com"), ConnNetwork::Tcp, None));
    assert!(!m.matches_network(&domain("example.com"), ConnNetwork::Tcp));
    assert!(!m.matches(&domain("example.com")));
}

#[test]
fn router_routes_src_ip_cidr_rule() {
    let mut outbounds = HashMap::new();
    outbounds.insert(
        "proxy".to_string(),
        OutboundMode::Socks5Upstream {
            addr: SocketAddr::from((Ipv4Addr::LOCALHOST, 1080)),
        },
    );
    let rules = vec![Rule::new(
        RuleMatcher::SrcIpCidr(IpCidr::parse("192.168.0.0/16").unwrap()),
        DIRECT,
    )];
    let router = Router::new(outbounds, rules, "proxy").unwrap();

    let inside = SocketAddr::new(Ipv4Addr::new(192, 168, 1, 5).into(), 40000);
    let outside = SocketAddr::new(Ipv4Addr::new(8, 8, 4, 4).into(), 40000);
    // Source inside the block -> DIRECT.
    assert!(matches!(
        router.select_conn(&domain("example.com"), ConnNetwork::Tcp, Some(inside)),
        OutboundMode::Direct
    ));
    // Source outside the block -> fallback proxy.
    assert!(matches!(
        router.select_conn(&domain("example.com"), ConnNetwork::Tcp, Some(outside)),
        OutboundMode::Socks5Upstream { .. }
    ));
    // No source (the `select`/`select_network` wrappers) -> never matches.
    assert!(matches!(
        router.select_network(&domain("example.com"), ConnNetwork::Tcp),
        OutboundMode::Socks5Upstream { .. }
    ));
}

#[test]
fn src_ip_cidr_as_logical_sub_rule_respects_source() {
    // AND,((DOMAIN-SUFFIX,example.com),(SRC-IP-CIDR,192.168.1.0/24)) only
    // matches a connection to that domain from a source in the block; the
    // source threads into sub-rules.
    let m = RuleMatcher::Logical {
        op: LogicalOp::And,
        subs: vec![
            RuleMatcher::DomainSuffix("example.com".to_string()),
            RuleMatcher::SrcIpCidr(IpCidr::parse("192.168.1.0/24").unwrap()),
        ],
    };
    let inside = SocketAddr::new(Ipv4Addr::new(192, 168, 1, 5).into(), 50000);
    let outside = SocketAddr::new(Ipv4Addr::new(10, 0, 0, 1).into(), 50000);
    assert!(m.matches_conn(&domain("www.example.com"), ConnNetwork::Tcp, Some(inside)));
    assert!(!m.matches_conn(&domain("www.example.com"), ConnNetwork::Tcp, Some(outside)));
    assert!(!m.matches_conn(&domain("other.net"), ConnNetwork::Tcp, Some(inside)));
    // Without a known source the SRC-IP-CIDR sub-rule cannot match.
    assert!(!m.matches_conn(&domain("www.example.com"), ConnNetwork::Tcp, None));
}

#[test]
fn src_ip_cidr_equality_and_metadata() {
    let a = RuleMatcher::SrcIpCidr(IpCidr::parse("192.168.1.0/24").unwrap());
    let b = RuleMatcher::SrcIpCidr(IpCidr::parse("192.168.1.0/24").unwrap());
    let c = RuleMatcher::SrcIpCidr(IpCidr::parse("10.0.0.0/8").unwrap());
    assert_eq!(a, b);
    assert_ne!(a, c);
    // A same-CIDR destination matcher is a different variant and never equal.
    assert_ne!(a, RuleMatcher::IpCidr(IpCidr::parse("192.168.1.0/24").unwrap()));
    assert_eq!(a.kind_str(), "SrcIpCidr");
    assert_eq!(a.payload(), "192.168.1.0/24");
}

#[test]
fn src_ip_asn_matches_source_ip_independent_of_target() {
    // FakeGeo announces AS13335 for the 1.0.0.0/8 block.
    let db: Arc<dyn GeoLookup> = Arc::new(FakeGeo);
    let m = RuleMatcher::SrcIpAsn { asn: 13335, db };
    let inside = SocketAddr::new(Ipv4Addr::new(1, 2, 3, 4).into(), 50000);
    let outside = SocketAddr::new(Ipv4Addr::new(8, 8, 8, 8).into(), 50000);
    // The source IP's ASN is matched regardless of the destination kind.
    assert!(m.matches_conn(&domain("example.com"), ConnNetwork::Tcp, Some(inside)));
    assert!(m.matches_conn(&ipv4(8, 8, 8, 8), ConnNetwork::Udp, Some(inside)));
    // A source outside AS13335 misses.
    assert!(!m.matches_conn(&domain("example.com"), ConnNetwork::Tcp, Some(outside)));
}

#[test]
fn src_ip_asn_never_matches_without_a_known_source() {
    // With no source the rule cannot match, so the `matches` /
    // `matches_network` wrappers (which pass `None`) always miss, even for
    // an ASN that covers the destination IP.
    let db: Arc<dyn GeoLookup> = Arc::new(FakeGeo);
    let m = RuleMatcher::SrcIpAsn { asn: 13335, db };
    assert!(!m.matches_conn(&ipv4(1, 1, 1, 1), ConnNetwork::Tcp, None));
    assert!(!m.matches_network(&ipv4(1, 1, 1, 1), ConnNetwork::Tcp));
    assert!(!m.matches(&ipv4(1, 1, 1, 1)));
}

#[test]
fn router_routes_src_ip_asn_rule() {
    let db: Arc<dyn GeoLookup> = Arc::new(FakeGeo);
    let mut outbounds = HashMap::new();
    outbounds.insert(
        "proxy".to_string(),
        OutboundMode::Socks5Upstream {
            addr: SocketAddr::from((Ipv4Addr::LOCALHOST, 1080)),
        },
    );
    let rules = vec![Rule::new(RuleMatcher::SrcIpAsn { asn: 13335, db }, DIRECT)];
    let router = Router::new(outbounds, rules, "proxy").unwrap();

    let inside = SocketAddr::new(Ipv4Addr::new(1, 2, 3, 4).into(), 40000);
    let outside = SocketAddr::new(Ipv4Addr::new(8, 8, 4, 4).into(), 40000);
    // Source in AS13335 -> DIRECT, regardless of destination.
    assert!(matches!(
        router.select_conn(&domain("example.com"), ConnNetwork::Tcp, Some(inside)),
        OutboundMode::Direct
    ));
    // Source outside the ASN -> fallback proxy.
    assert!(matches!(
        router.select_conn(&domain("example.com"), ConnNetwork::Tcp, Some(outside)),
        OutboundMode::Socks5Upstream { .. }
    ));
    // No source (the `select`/`select_network` wrappers) -> never matches.
    assert!(matches!(
        router.select_network(&ipv4(1, 1, 1, 1), ConnNetwork::Tcp),
        OutboundMode::Socks5Upstream { .. }
    ));
}

#[test]
fn src_ip_asn_as_logical_sub_rule_respects_source() {
    // AND,((DOMAIN-SUFFIX,example.com),(SRC-IP-ASN,13335)) only matches a
    // connection to that domain from a source in AS13335; the source
    // threads into the sub-rule.
    let db: Arc<dyn GeoLookup> = Arc::new(FakeGeo);
    let m = RuleMatcher::Logical {
        op: LogicalOp::And,
        subs: vec![
            RuleMatcher::DomainSuffix("example.com".to_string()),
            RuleMatcher::SrcIpAsn { asn: 13335, db },
        ],
    };
    let inside = SocketAddr::new(Ipv4Addr::new(1, 2, 3, 4).into(), 50000);
    let outside = SocketAddr::new(Ipv4Addr::new(10, 0, 0, 1).into(), 50000);
    assert!(m.matches_conn(&domain("www.example.com"), ConnNetwork::Tcp, Some(inside)));
    assert!(!m.matches_conn(&domain("www.example.com"), ConnNetwork::Tcp, Some(outside)));
    assert!(!m.matches_conn(&domain("other.net"), ConnNetwork::Tcp, Some(inside)));
    // Without a known source the SRC-IP-ASN sub-rule cannot match.
    assert!(!m.matches_conn(&domain("www.example.com"), ConnNetwork::Tcp, None));
}

#[test]
fn src_ip_asn_equality_and_metadata() {
    let db: Arc<dyn GeoLookup> = Arc::new(FakeGeo);
    let other: Arc<dyn GeoLookup> = Arc::new(FakeGeo);
    let a = RuleMatcher::SrcIpAsn {
        asn: 13335,
        db: Arc::clone(&db),
    };
    let same = RuleMatcher::SrcIpAsn {
        asn: 13335,
        db: Arc::clone(&db),
    };
    let different_db = RuleMatcher::SrcIpAsn { asn: 13335, db: other };
    let different_asn = RuleMatcher::SrcIpAsn {
        asn: 15169,
        db: Arc::clone(&db),
    };
    assert_eq!(a, same);
    assert_ne!(a, different_db);
    assert_ne!(a, different_asn);
    // A same-ASN destination matcher is a different variant, never equal.
    assert_ne!(a, RuleMatcher::Asn { asn: 13335, db });
    assert_eq!(a.kind_str(), "SrcIPASN");
    assert_eq!(a.payload(), "13335");
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
