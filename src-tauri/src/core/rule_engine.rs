use anyhow::{Context, Result, bail};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::net::IpAddr;

// ---------------------------------------------------------------------------
// Connection metadata — the "packet header" we match rules against
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ConnectionMeta {
    /// Destination domain (lowercase). Empty when connecting by raw IP.
    #[serde(default)]
    pub host: String,
    /// Destination IP address (may be absent for pure-domain connections).
    #[serde(default, deserialize_with = "deser_opt_ip")]
    pub dst_ip: Option<IpAddr>,
    /// Source IP address (may be absent).
    #[serde(default, deserialize_with = "deser_opt_ip")]
    pub src_ip: Option<IpAddr>,
    #[serde(default)]
    pub dst_port: u16,
    #[serde(default)]
    pub src_port: u16,
    #[serde(default)]
    pub in_port: u16,
    #[serde(default)]
    pub network: NetworkType,
}

fn deser_opt_ip<'de, D: serde::Deserializer<'de>>(d: D) -> Result<Option<IpAddr>, D::Error> {
    let s: Option<String> = Option::deserialize(d)?;
    match s {
        None => Ok(None),
        Some(s) if s.is_empty() => Ok(None),
        Some(s) => s.parse::<IpAddr>().map(Some).map_err(serde::de::Error::custom),
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum NetworkType {
    #[default]
    Tcp,
    Udp,
}

// ---------------------------------------------------------------------------
// Match result
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize)]
pub struct RuleMatchResult {
    pub matched: bool,
    pub rule_index: Option<usize>,
    pub rule_raw: Option<String>,
    pub target: Option<String>,
    pub rule_type: Option<String>,
}

// ---------------------------------------------------------------------------
// Parsed rule representation
// ---------------------------------------------------------------------------

enum ParsedRule {
    Domain {
        domain: String,
        target: String,
    },
    DomainSuffix {
        suffix: String,
        target: String,
    },
    DomainKeyword {
        keyword: String,
        target: String,
    },
    DomainRegex {
        regex: Regex,
        target: String,
    },
    DomainWildcard {
        pattern: String,
        target: String,
    },
    IpCidr {
        addr: IpAddr,
        prefix_len: u8,
        is_src: bool,
        target: String,
    },
    IpSuffix {
        addr_bytes: Vec<u8>,
        bits: u8,
        is_src: bool,
        target: String,
    },
    Port {
        ranges: Vec<(u16, u16)>,
        port_kind: PortKind,
        target: String,
    },
    Network {
        network: NetworkType,
        target: String,
    },
    Match {
        target: String,
    },
    // Rule types that require external data — we can parse but not match locally.
    ExternalData {
        rule_type: String,
        payload: String,
        target: String,
    },
}

#[derive(Debug, Clone, Copy)]
enum PortKind {
    Src,
    Dst,
    In,
}

// ---------------------------------------------------------------------------
// Rule engine
// ---------------------------------------------------------------------------

pub struct RuleEngine {
    rules: Vec<(ParsedRule, String)>, // (parsed, raw_string)
}

impl RuleEngine {
    pub fn from_rules(raw_rules: &[&str]) -> Result<Self> {
        let mut rules = Vec::with_capacity(raw_rules.len());
        for &raw in raw_rules {
            let parsed = parse_rule(raw)?;
            rules.push((parsed, raw.to_owned()));
        }
        Ok(Self { rules })
    }

    pub fn match_connection(&self, meta: &ConnectionMeta) -> RuleMatchResult {
        let host = meta.host.to_ascii_lowercase();
        for (i, (rule, raw)) in self.rules.iter().enumerate() {
            if let Some(target) = rule_matches(rule, meta, &host) {
                return RuleMatchResult {
                    matched: true,
                    rule_index: Some(i),
                    rule_raw: Some(raw.clone()),
                    target: Some(target.to_owned()),
                    rule_type: Some(rule_type_name(rule).to_owned()),
                };
            }
        }
        RuleMatchResult {
            matched: false,
            rule_index: None,
            rule_raw: None,
            target: None,
            rule_type: None,
        }
    }
}

// ---------------------------------------------------------------------------
// Rule validation (for config validator integration)
// ---------------------------------------------------------------------------

#[derive(Debug)]
pub struct RuleValidation {
    pub valid: bool,
    pub error: Option<String>,
}

/// Validate a single rule string — returns Ok(()) if the rule parses, Err
/// with a human-readable message otherwise.
pub fn validate_rule(raw: &str) -> RuleValidation {
    match parse_rule(raw) {
        Ok(_) => RuleValidation {
            valid: true,
            error: None,
        },
        Err(e) => RuleValidation {
            valid: false,
            error: Some(e.to_string()),
        },
    }
}

/// Validate a rule spec (TYPE, PAYLOAD, TARGET) before sending to the runtime.
/// This is the Rust gatekeeper for any rule that will be created via the
/// mihomo runtime API — ensures the Rust rule engine accepts the format
/// before delegating to Go.
pub fn validate_rule_spec(rule_type: &str, payload: &str, target: &str) -> RuleValidation {
    if rule_type.eq_ignore_ascii_case("MATCH") {
        return validate_rule(&format!("MATCH,{target}"));
    }
    validate_rule(&format!("{rule_type},{payload},{target}"))
}

// ---------------------------------------------------------------------------
// Parsing
// ---------------------------------------------------------------------------

fn parse_rule(raw: &str) -> Result<ParsedRule> {
    let parts = parse_rule_payload(raw)?;
    let rule_type_upper = parts.rule_type.to_ascii_uppercase();

    if rule_type_upper == "MATCH" {
        if parts.target.is_empty() {
            bail!("MATCH rule requires a target policy");
        }
        return Ok(ParsedRule::Match { target: parts.target });
    }

    if parts.payload.is_empty() || parts.target.is_empty() {
        bail!("rule must have at least TYPE,PAYLOAD,TARGET");
    }

    let payload = parts.payload.as_str();
    let target = parts.target;
    let param_refs: Vec<&str> = parts.params.iter().map(|s| s.as_str()).collect();
    let (is_src, _no_resolve) = parse_params(&param_refs);

    match rule_type_upper.as_str() {
        "DOMAIN" => Ok(ParsedRule::Domain {
            domain: payload.to_ascii_lowercase(),
            target,
        }),
        "DOMAIN-SUFFIX" => Ok(ParsedRule::DomainSuffix {
            suffix: payload.to_ascii_lowercase(),
            target,
        }),
        "DOMAIN-KEYWORD" => Ok(ParsedRule::DomainKeyword {
            keyword: payload.to_ascii_lowercase(),
            target,
        }),
        "DOMAIN-REGEX" => {
            let regex = Regex::new(&format!("(?i){payload}")).context("invalid regex in DOMAIN-REGEX")?;
            Ok(ParsedRule::DomainRegex { regex, target })
        }
        "DOMAIN-WILDCARD" => Ok(ParsedRule::DomainWildcard {
            pattern: payload.to_ascii_lowercase(),
            target,
        }),
        "IP-CIDR" | "IP-CIDR6" => {
            let (addr, prefix_len) = parse_cidr(payload)?;
            Ok(ParsedRule::IpCidr {
                addr,
                prefix_len,
                is_src,
                target,
            })
        }
        "SRC-IP-CIDR" => {
            let (addr, prefix_len) = parse_cidr(payload)?;
            Ok(ParsedRule::IpCidr {
                addr,
                prefix_len,
                is_src: true,
                target,
            })
        }
        "IP-SUFFIX" => {
            let (addr_bytes, bits) = parse_ip_suffix(payload)?;
            Ok(ParsedRule::IpSuffix {
                addr_bytes,
                bits,
                is_src,
                target,
            })
        }
        "SRC-IP-SUFFIX" => {
            let (addr_bytes, bits) = parse_ip_suffix(payload)?;
            Ok(ParsedRule::IpSuffix {
                addr_bytes,
                bits,
                is_src: true,
                target,
            })
        }
        "SRC-PORT" => {
            let ranges = parse_port_ranges(payload)?;
            Ok(ParsedRule::Port {
                ranges,
                port_kind: PortKind::Src,
                target,
            })
        }
        "DST-PORT" => {
            let ranges = parse_port_ranges(payload)?;
            Ok(ParsedRule::Port {
                ranges,
                port_kind: PortKind::Dst,
                target,
            })
        }
        "IN-PORT" => {
            let ranges = parse_port_ranges(payload)?;
            Ok(ParsedRule::Port {
                ranges,
                port_kind: PortKind::In,
                target,
            })
        }
        "NETWORK" => {
            let network = match payload.to_ascii_uppercase().as_str() {
                "TCP" => NetworkType::Tcp,
                "UDP" => NetworkType::Udp,
                _ => bail!("NETWORK: unsupported network type \"{payload}\", expected TCP or UDP"),
            };
            Ok(ParsedRule::Network { network, target })
        }
        // Types that require external data — validate format but don't match locally
        "GEOIP"
        | "SRC-GEOIP"
        | "GEOSITE"
        | "IP-ASN"
        | "SRC-IP-ASN"
        | "RULE-SET"
        | "IN-TYPE"
        | "IN-USER"
        | "IN-NAME"
        | "DSCP"
        | "UID"
        | "PROCESS-NAME"
        | "PROCESS-PATH"
        | "PROCESS-NAME-REGEX"
        | "PROCESS-PATH-REGEX"
        | "PROCESS-NAME-WILDCARD"
        | "PROCESS-PATH-WILDCARD"
        | "AND"
        | "OR"
        | "NOT"
        | "SUB-RULE" => Ok(ParsedRule::ExternalData {
            rule_type: rule_type.to_ascii_uppercase(),
            payload: payload.to_owned(),
            target,
        }),
        _ => bail!("unsupported rule type: {}", parts.rule_type),
    }
}

struct RuleParts {
    rule_type: String,
    payload: String,
    target: String,
    params: Vec<String>,
}

fn parse_rule_payload(raw: &str) -> Result<RuleParts> {
    let items: Vec<String> = raw.split(',').map(|s| s.trim().to_owned()).collect();
    if items.is_empty() || items[0].is_empty() {
        bail!("empty rule");
    }

    let rule_type = items[0].to_ascii_uppercase();
    let mut payload = String::new();
    let mut target = String::new();
    let mut params = Vec::new();

    if items.len() > 1 {
        match rule_type.as_str() {
            "MATCH" => {
                target = items[1].clone();
            }
            "NOT" | "OR" | "AND" | "SUB-RULE" | "DOMAIN-REGEX" | "PROCESS-NAME-REGEX" | "PROCESS-PATH-REGEX" => {
                if let Some(last) = items.last() {
                    target = last.clone();
                }
                if items.len() > 2 {
                    payload = items[1..items.len() - 1].join(",");
                }
            }
            _ => {
                payload = items[1].clone();
                if items.len() > 2 {
                    target = items[2].clone();
                }
                if items.len() > 3 {
                    params = items[3..].to_vec();
                }
            }
        }
    }

    Ok(RuleParts {
        rule_type,
        payload,
        target,
        params,
    })
}

fn parse_params(params: &[&str]) -> (bool, bool) {
    let has_src = params.iter().any(|p| p.eq_ignore_ascii_case("src"));
    let no_resolve = has_src || params.iter().any(|p| p.eq_ignore_ascii_case("no-resolve"));
    (has_src, no_resolve)
}

fn parse_cidr(s: &str) -> Result<(IpAddr, u8)> {
    let (addr_s, len_s) = s
        .rsplit_once('/')
        .context("IP-CIDR payload must be in CIDR notation (e.g. 10.0.0.0/8)")?;
    let addr: IpAddr = addr_s.parse().context("IP-CIDR: invalid IP address")?;
    let prefix_len: u8 = len_s.parse().context("IP-CIDR: invalid prefix length")?;
    let max_bits = if addr.is_ipv4() { 32 } else { 128 };
    if prefix_len > max_bits {
        bail!(
            "IP-CIDR: prefix length {prefix_len} exceeds maximum {max_bits} for {}",
            if addr.is_ipv4() { "IPv4" } else { "IPv6" }
        );
    }
    Ok((addr, prefix_len))
}

fn parse_ip_suffix(s: &str) -> Result<(Vec<u8>, u8)> {
    let (addr_s, bits_s) = s
        .rsplit_once('/')
        .context("IP-SUFFIX payload must be in addr/bits notation")?;
    let addr: IpAddr = addr_s.parse().context("IP-SUFFIX: invalid IP address")?;
    let bits: u8 = bits_s.parse().context("IP-SUFFIX: invalid suffix bit count")?;
    let addr_bytes = match addr {
        IpAddr::V4(v4) => v4.octets().to_vec(),
        IpAddr::V6(v6) => v6.octets().to_vec(),
    };
    let max_bits = (addr_bytes.len() * 8) as u8;
    if bits > max_bits {
        bail!("IP-SUFFIX: bit count {bits} exceeds maximum {max_bits}");
    }
    Ok((addr_bytes, bits))
}

fn parse_port_ranges(s: &str) -> Result<Vec<(u16, u16)>> {
    let mut ranges = Vec::new();
    for part in s.split('/') {
        let part = part.trim();
        if let Some((lo_s, hi_s)) = part.split_once('-') {
            let lo: u16 = lo_s.trim().parse().context("invalid port number")?;
            let hi: u16 = hi_s.trim().parse().context("invalid port number")?;
            if lo > hi {
                bail!("port range {lo}-{hi}: start > end");
            }
            ranges.push((lo, hi));
        } else {
            let port: u16 = part.parse().context("invalid port number")?;
            ranges.push((port, port));
        }
    }
    if ranges.is_empty() {
        bail!("empty port specification");
    }
    Ok(ranges)
}

// ---------------------------------------------------------------------------
// Matching
// ---------------------------------------------------------------------------

fn rule_matches<'a>(rule: &'a ParsedRule, meta: &ConnectionMeta, host_lower: &str) -> Option<&'a str> {
    match rule {
        ParsedRule::Domain { domain, target } => {
            if host_lower == domain {
                Some(target)
            } else {
                None
            }
        }
        ParsedRule::DomainSuffix { suffix, target } => {
            if host_lower.ends_with(&format!(".{suffix}")) || host_lower == suffix.as_str() {
                Some(target)
            } else {
                None
            }
        }
        ParsedRule::DomainKeyword { keyword, target } => {
            if host_lower.contains(keyword.as_str()) {
                Some(target)
            } else {
                None
            }
        }
        ParsedRule::DomainRegex { regex, target } => {
            if regex.is_match(host_lower) {
                Some(target)
            } else {
                None
            }
        }
        ParsedRule::DomainWildcard { pattern, target } => {
            if wildcard_match(pattern, host_lower) {
                Some(target)
            } else {
                None
            }
        }
        ParsedRule::IpCidr {
            addr,
            prefix_len,
            is_src,
            target,
        } => {
            let ip = if *is_src { meta.src_ip } else { meta.dst_ip };
            ip.and_then(|ip| {
                if cidr_contains(*addr, *prefix_len, ip) {
                    Some(target.as_str())
                } else {
                    None
                }
            })
        }
        ParsedRule::IpSuffix {
            addr_bytes,
            bits,
            is_src,
            target,
        } => {
            let ip = if *is_src { meta.src_ip } else { meta.dst_ip };
            ip.and_then(|ip| {
                if ip_suffix_match(addr_bytes, *bits, ip) {
                    Some(target.as_str())
                } else {
                    None
                }
            })
        }
        ParsedRule::Port {
            ranges,
            port_kind,
            target,
        } => {
            let port = match port_kind {
                PortKind::Src => meta.src_port,
                PortKind::Dst => meta.dst_port,
                PortKind::In => meta.in_port,
            };
            if ranges.iter().any(|&(lo, hi)| port >= lo && port <= hi) {
                Some(target)
            } else {
                None
            }
        }
        ParsedRule::Network { network, target } => {
            if meta.network == *network {
                Some(target)
            } else {
                None
            }
        }
        ParsedRule::Match { target } => Some(target),
        // External data rules cannot be matched locally
        ParsedRule::ExternalData { .. } => None,
    }
}

fn rule_type_name(rule: &ParsedRule) -> &str {
    match rule {
        ParsedRule::Domain { .. } => "DOMAIN",
        ParsedRule::DomainSuffix { .. } => "DOMAIN-SUFFIX",
        ParsedRule::DomainKeyword { .. } => "DOMAIN-KEYWORD",
        ParsedRule::DomainRegex { .. } => "DOMAIN-REGEX",
        ParsedRule::DomainWildcard { .. } => "DOMAIN-WILDCARD",
        ParsedRule::IpCidr { is_src: true, .. } => "SRC-IP-CIDR",
        ParsedRule::IpCidr { .. } => "IP-CIDR",
        ParsedRule::IpSuffix { is_src: true, .. } => "SRC-IP-SUFFIX",
        ParsedRule::IpSuffix { .. } => "IP-SUFFIX",
        ParsedRule::Port {
            port_kind: PortKind::Src,
            ..
        } => "SRC-PORT",
        ParsedRule::Port {
            port_kind: PortKind::Dst,
            ..
        } => "DST-PORT",
        ParsedRule::Port {
            port_kind: PortKind::In,
            ..
        } => "IN-PORT",
        ParsedRule::Network { .. } => "NETWORK",
        ParsedRule::Match { .. } => "MATCH",
        ParsedRule::ExternalData { rule_type, .. } => rule_type,
    }
}

// ---------------------------------------------------------------------------
// IP helpers
// ---------------------------------------------------------------------------

fn cidr_contains(network: IpAddr, prefix_len: u8, candidate: IpAddr) -> bool {
    match (network, candidate) {
        (IpAddr::V4(net), IpAddr::V4(cand)) => {
            if prefix_len == 0 {
                return true;
            }
            if prefix_len > 32 {
                return false;
            }
            let mask = u32::MAX.checked_shl(32 - prefix_len as u32).unwrap_or(0);
            (u32::from(net) & mask) == (u32::from(cand) & mask)
        }
        (IpAddr::V6(net), IpAddr::V6(cand)) => {
            if prefix_len == 0 {
                return true;
            }
            if prefix_len > 128 {
                return false;
            }
            let mask = u128::MAX.checked_shl(128 - prefix_len as u32).unwrap_or(0);
            (u128::from(net) & mask) == (u128::from(cand) & mask)
        }
        _ => false, // v4 vs v6 mismatch
    }
}

fn ip_suffix_match(rule_bytes: &[u8], bits: u8, candidate: IpAddr) -> bool {
    let cand_bytes: Vec<u8> = match candidate {
        IpAddr::V4(v4) => v4.octets().to_vec(),
        IpAddr::V6(v6) => v6.octets().to_vec(),
    };
    if rule_bytes.len() != cand_bytes.len() {
        return false;
    }
    let size = cand_bytes.len();
    let full_bytes = (bits / 8) as usize;
    let remainder_bits = bits % 8;

    // Compare full trailing bytes
    for i in 1..=full_bytes {
        if i > size {
            return false;
        }
        if rule_bytes[size - i] != cand_bytes[size - i] {
            return false;
        }
    }

    // Compare partial byte
    if remainder_bits > 0 && full_bytes < size {
        let idx = size - full_bytes - 1;
        let shift = 8 - remainder_bits;
        if (rule_bytes[idx] << shift) != (cand_bytes[idx] << shift) {
            return false;
        }
    }

    true
}

// ---------------------------------------------------------------------------
// Wildcard matching (compatible with mihomo's wildcard.Match)
// ---------------------------------------------------------------------------

fn wildcard_match(pattern: &str, text: &str) -> bool {
    let pat = pattern.as_bytes();
    let txt = text.as_bytes();
    let (mut pi, mut ti) = (0usize, 0usize);
    let (mut star_pi, mut star_ti) = (usize::MAX, 0usize);

    while ti < txt.len() {
        if pi < pat.len() && (pat[pi] == b'?' || pat[pi] == txt[ti]) {
            pi += 1;
            ti += 1;
        } else if pi < pat.len() && pat[pi] == b'*' {
            star_pi = pi;
            star_ti = ti;
            pi += 1;
        } else if star_pi != usize::MAX {
            pi = star_pi + 1;
            star_ti += 1;
            ti = star_ti;
        } else {
            return false;
        }
    }

    while pi < pat.len() && pat[pi] == b'*' {
        pi += 1;
    }

    pi == pat.len()
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn meta_domain(host: &str) -> ConnectionMeta {
        ConnectionMeta {
            host: host.to_owned(),
            ..Default::default()
        }
    }

    fn meta_ip(dst: &str, port: u16) -> ConnectionMeta {
        ConnectionMeta {
            dst_ip: Some(dst.parse().unwrap()),
            dst_port: port,
            ..Default::default()
        }
    }

    #[test]
    fn domain_exact_match() {
        let engine = RuleEngine::from_rules(&["DOMAIN,google.com,Proxy", "MATCH,DIRECT"]).unwrap();
        let r = engine.match_connection(&meta_domain("google.com"));
        assert!(r.matched);
        assert_eq!(r.target.as_deref(), Some("Proxy"));
        assert_eq!(r.rule_type.as_deref(), Some("DOMAIN"));
    }

    #[test]
    fn domain_suffix_match() {
        let engine = RuleEngine::from_rules(&["DOMAIN-SUFFIX,google.com,Proxy", "MATCH,DIRECT"]).unwrap();
        assert!(engine.match_connection(&meta_domain("www.google.com")).matched);
        assert!(engine.match_connection(&meta_domain("google.com")).matched);
        let r = engine.match_connection(&meta_domain("notgoogle.com"));
        assert_eq!(r.target.as_deref(), Some("DIRECT"));
    }

    #[test]
    fn domain_keyword_match() {
        let engine = RuleEngine::from_rules(&["DOMAIN-KEYWORD,goog,Proxy", "MATCH,DIRECT"]).unwrap();
        assert!(engine.match_connection(&meta_domain("www.google.com")).matched);
        let r = engine.match_connection(&meta_domain("www.google.com"));
        assert_eq!(r.target.as_deref(), Some("Proxy"));
    }

    #[test]
    fn domain_regex_match() {
        let engine = RuleEngine::from_rules(&["DOMAIN-REGEX,^(www\\.)?google\\.com$,Proxy", "MATCH,DIRECT"]).unwrap();
        assert!(engine.match_connection(&meta_domain("google.com")).matched);
        assert!(engine.match_connection(&meta_domain("www.google.com")).matched);
        let r = engine.match_connection(&meta_domain("mail.google.com"));
        assert_eq!(r.target.as_deref(), Some("DIRECT"));
    }

    #[test]
    fn ip_cidr_match() {
        let engine = RuleEngine::from_rules(&["IP-CIDR,10.0.0.0/8,Direct", "MATCH,Proxy"]).unwrap();
        let r = engine.match_connection(&meta_ip("10.1.2.3", 80));
        assert_eq!(r.target.as_deref(), Some("Direct"));

        let r = engine.match_connection(&meta_ip("192.168.1.1", 80));
        assert_eq!(r.target.as_deref(), Some("Proxy"));
    }

    #[test]
    fn port_range_match() {
        let engine = RuleEngine::from_rules(&["DST-PORT,80/443/8000-9000,Web", "MATCH,DIRECT"]).unwrap();
        assert_eq!(
            engine.match_connection(&meta_ip("1.2.3.4", 443)).target.as_deref(),
            Some("Web")
        );
        assert_eq!(
            engine.match_connection(&meta_ip("1.2.3.4", 8500)).target.as_deref(),
            Some("Web")
        );
        assert_eq!(
            engine.match_connection(&meta_ip("1.2.3.4", 22)).target.as_deref(),
            Some("DIRECT")
        );
    }

    #[test]
    fn network_match() {
        let engine = RuleEngine::from_rules(&["NETWORK,udp,UdpProxy", "MATCH,DIRECT"]).unwrap();
        let mut m = meta_domain("example.com");
        m.network = NetworkType::Udp;
        assert_eq!(engine.match_connection(&m).target.as_deref(), Some("UdpProxy"));
    }

    #[test]
    fn wildcard_match_cases() {
        assert!(wildcard_match("*.google.com", "www.google.com"));
        assert!(wildcard_match("*.google.com", "mail.google.com"));
        assert!(!wildcard_match("*.google.com", "google.com"));
        assert!(wildcard_match("google.*", "google.com"));
        assert!(wildcard_match("g?ogle.com", "google.com"));
    }

    #[test]
    fn validate_rule_catches_errors() {
        assert!(validate_rule("DOMAIN,google.com,Proxy").valid);
        assert!(validate_rule("IP-CIDR,10.0.0.0/8,Direct").valid);
        assert!(validate_rule("MATCH,DIRECT").valid);

        assert!(!validate_rule("DOMAIN").valid);
        assert!(!validate_rule("IP-CIDR,not-a-cidr,Direct").valid);
        assert!(!validate_rule("IP-SUFFIX,1.2.3.4/40,Direct").valid);
        assert!(!validate_rule("DST-PORT,notaport,Direct").valid);
        assert!(!validate_rule("UNKNOWN-TYPE,foo,bar").valid);
    }

    #[test]
    fn geoip_parses_as_external_data() {
        let v = validate_rule("GEOIP,CN,DIRECT,no-resolve");
        assert!(v.valid);
        // external data rules parse but don't match
        let engine = RuleEngine::from_rules(&["GEOIP,CN,DIRECT,no-resolve", "MATCH,Proxy"]).unwrap();
        let r = engine.match_connection(&meta_ip("1.2.3.4", 80));
        // GEOIP can't match locally → falls through to MATCH
        assert_eq!(r.target.as_deref(), Some("Proxy"));
    }

    #[test]
    fn cidr_v6() {
        let engine = RuleEngine::from_rules(&["IP-CIDR6,fd00::/8,Local", "MATCH,Proxy"]).unwrap();
        let mut m = ConnectionMeta::default();
        m.dst_ip = Some("fd12:3456::1".parse().unwrap());
        assert_eq!(engine.match_connection(&m).target.as_deref(), Some("Local"));
    }

    #[test]
    fn domain_regex_payload_with_comma() {
        let engine = RuleEngine::from_rules(&["DOMAIN-REGEX,^(foo|bar).example\\.com$,Proxy", "MATCH,DIRECT"]).unwrap();
        assert!(engine.match_connection(&meta_domain("foo.example.com")).matched);
    }
}
