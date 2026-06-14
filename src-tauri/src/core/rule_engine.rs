use anyhow::{Context, Result, bail};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    fs,
    net::IpAddr,
    path::{Path, PathBuf},
};

use super::rule_geodata::RuleGeoData;
use crate::utils::dirs;

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
    #[serde(default, alias = "process", alias = "processName")]
    pub process_name: String,
    #[serde(default, alias = "processPath", alias = "exe_path", alias = "exePath")]
    pub process_path: String,
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
    GeoIp {
        code: String,
        is_src: bool,
        target: String,
    },
    GeoSite {
        code: String,
        target: String,
    },
    IpAsn {
        asn: u32,
        is_src: bool,
        target: String,
    },
    RuleSet {
        name: String,
        target: String,
    },
    ProcessName {
        name: String,
        target: String,
    },
    ProcessPath {
        path: String,
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
    geo_data: RuleGeoData,
    rule_sets: RuleSetData,
}

impl RuleEngine {
    pub fn from_rules(raw_rules: &[&str]) -> Result<Self> {
        Self::from_rules_with_geo_data_and_rule_sets(raw_rules, RuleGeoData::empty(), RuleSetData::empty())
    }

    pub fn from_rules_with_default_geo_data(raw_rules: &[&str]) -> Result<Self> {
        Self::from_rules_with_geo_data_and_rule_sets(raw_rules, RuleGeoData::load_default(), RuleSetData::empty())
    }

    pub fn from_rules_with_default_geo_data_and_rule_sets(raw_rules: &[&str], rule_sets: RuleSetData) -> Result<Self> {
        Self::from_rules_with_geo_data_and_rule_sets(raw_rules, RuleGeoData::load_default(), rule_sets)
    }

    pub fn from_rules_with_geo_data(raw_rules: &[&str], geo_data: RuleGeoData) -> Result<Self> {
        Self::from_rules_with_geo_data_and_rule_sets(raw_rules, geo_data, RuleSetData::empty())
    }

    pub fn from_rules_with_rule_sets(raw_rules: &[&str], rule_sets: RuleSetData) -> Result<Self> {
        Self::from_rules_with_geo_data_and_rule_sets(raw_rules, RuleGeoData::empty(), rule_sets)
    }

    fn from_rules_with_geo_data_and_rule_sets(
        raw_rules: &[&str],
        geo_data: RuleGeoData,
        rule_sets: RuleSetData,
    ) -> Result<Self> {
        let mut rules = Vec::with_capacity(raw_rules.len());
        for &raw in raw_rules {
            let parsed = parse_rule(raw)?;
            rules.push((parsed, raw.to_owned()));
        }
        Ok(Self {
            rules,
            geo_data,
            rule_sets,
        })
    }

    pub fn match_connection(&self, meta: &ConnectionMeta) -> RuleMatchResult {
        let host = meta.host.to_ascii_lowercase();
        for (i, (rule, raw)) in self.rules.iter().enumerate() {
            if let Some(target) = rule_matches(rule, meta, &host, &self.geo_data, &self.rule_sets) {
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
        "GEOIP" => Ok(ParsedRule::GeoIp {
            code: payload.to_ascii_lowercase(),
            is_src,
            target,
        }),
        "SRC-GEOIP" => Ok(ParsedRule::GeoIp {
            code: payload.to_ascii_lowercase(),
            is_src: true,
            target,
        }),
        "GEOSITE" => Ok(ParsedRule::GeoSite {
            code: payload.to_ascii_lowercase(),
            target,
        }),
        "IP-ASN" => Ok(ParsedRule::IpAsn {
            asn: parse_asn(payload)?,
            is_src,
            target,
        }),
        "SRC-IP-ASN" => Ok(ParsedRule::IpAsn {
            asn: parse_asn(payload)?,
            is_src: true,
            target,
        }),
        "RULE-SET" => Ok(ParsedRule::RuleSet {
            name: payload.to_owned(),
            target,
        }),
        "PROCESS-NAME" => Ok(ParsedRule::ProcessName {
            name: payload.to_owned(),
            target,
        }),
        "PROCESS-PATH" => Ok(ParsedRule::ProcessPath {
            path: payload.to_owned(),
            target,
        }),
        // Types that require external data — validate format but don't match locally
        "IN-TYPE"
        | "IN-USER"
        | "IN-NAME"
        | "DSCP"
        | "UID"
        | "PROCESS-NAME-REGEX"
        | "PROCESS-PATH-REGEX"
        | "PROCESS-NAME-WILDCARD"
        | "PROCESS-PATH-WILDCARD"
        | "AND"
        | "OR"
        | "NOT"
        | "SUB-RULE" => Ok(ParsedRule::ExternalData {
            rule_type: rule_type_upper,
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

fn parse_asn(s: &str) -> Result<u32> {
    s.parse().context("IP-ASN payload must be a numeric ASN")
}

const RULE_SET_INTERNAL_TARGET: &str = "__RULE_SET_MATCH__";

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum RuleProviderBehavior {
    Domain,
    Ipcidr,
    Classical,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RuleProviderConfig {
    #[serde(default, rename = "type")]
    pub provider_type: String,
    pub behavior: RuleProviderBehavior,
    #[serde(default)]
    pub path: Option<PathBuf>,
    #[serde(default)]
    pub payload: Vec<String>,
    #[serde(default)]
    pub format: Option<String>,
}

#[derive(Default)]
pub struct RuleSetData {
    sets: HashMap<String, RuleSetMatcher>,
}

impl RuleSetData {
    pub fn empty() -> Self {
        Self::default()
    }

    pub fn from_rule_providers(providers: HashMap<String, RuleProviderConfig>) -> Result<Self> {
        let mut sets = HashMap::new();
        for (name, provider) in providers {
            if let Some(matcher) = RuleSetMatcher::from_provider(&provider)
                .with_context(|| format!("failed to load rule provider {name}"))?
            {
                sets.insert(name, matcher);
            }
        }
        Ok(Self { sets })
    }

    fn matches(&self, name: &str, meta: &ConnectionMeta) -> bool {
        self.sets.get(name).is_some_and(|matcher| matcher.matches(meta))
    }
}

struct RuleSetMatcher {
    engine: RuleEngine,
}

impl RuleSetMatcher {
    fn from_provider(provider: &RuleProviderConfig) -> Result<Option<Self>> {
        let items = load_rule_provider_items(provider)?;
        let rules = items
            .iter()
            .filter_map(|item| match normalize_rule_set_item(provider.behavior, item) {
                Ok(Some(rule)) => Some(Ok(rule)),
                Ok(None) => None,
                Err(err) => Some(Err(err)),
            })
            .collect::<Result<Vec<_>>>()?;
        if rules.is_empty() {
            return Ok(None);
        }
        let rule_refs = rules.iter().map(String::as_str).collect::<Vec<_>>();
        let engine = RuleEngine::from_rules(&rule_refs)?;
        Ok(Some(Self { engine }))
    }

    fn matches(&self, meta: &ConnectionMeta) -> bool {
        self.engine.match_connection(meta).matched
    }
}

fn load_rule_provider_items(provider: &RuleProviderConfig) -> Result<Vec<String>> {
    if !provider.payload.is_empty() {
        return Ok(provider.payload.clone());
    }
    let provider_type = provider.provider_type.to_ascii_lowercase();
    if provider_type == "inline" {
        return Ok(Vec::new());
    }
    if !provider_type.is_empty() && provider_type != "file" && provider_type != "http" {
        return Ok(Vec::new());
    }
    let Some(path) = provider.path.as_deref().and_then(resolve_provider_path) else {
        return Ok(Vec::new());
    };
    let content =
        fs::read_to_string(&path).with_context(|| format!("failed to read rule provider file {}", path.display()))?;
    if provider
        .format
        .as_deref()
        .is_some_and(|format| format.eq_ignore_ascii_case("text"))
    {
        return Ok(parse_rule_provider_text(&content));
    }
    parse_rule_provider_file(&content)
}

fn resolve_provider_path(path: &Path) -> Option<PathBuf> {
    if path.is_absolute() {
        return path.is_file().then(|| path.to_path_buf());
    }

    let mut roots = Vec::new();
    if let Ok(current_dir) = std::env::current_dir() {
        roots.push(current_dir);
    }
    if let Ok(app_home) = dirs::app_home_dir() {
        roots.push(app_home);
    }
    if let Ok(resources_dir) = dirs::app_resources_dir() {
        roots.push(resources_dir);
    }

    roots
        .into_iter()
        .map(|root| root.join(path))
        .find(|candidate| candidate.is_file())
}

#[derive(Deserialize)]
struct RuleProviderFile {
    payload: Vec<String>,
}

fn parse_rule_provider_file(content: &str) -> Result<Vec<String>> {
    if let Ok(file) = serde_yaml_ng::from_str::<RuleProviderFile>(content) {
        return Ok(file.payload);
    }
    if let Ok(payload) = serde_yaml_ng::from_str::<Vec<String>>(content) {
        return Ok(payload);
    }
    Ok(parse_rule_provider_text(content))
}

fn parse_rule_provider_text(content: &str) -> Vec<String> {
    content.lines().map(str::to_owned).collect()
}

fn normalize_rule_set_item(behavior: RuleProviderBehavior, item: &str) -> Result<Option<String>> {
    let item = item.trim();
    if item.is_empty() || item.starts_with('#') {
        return Ok(None);
    }

    let rule = match behavior {
        RuleProviderBehavior::Domain => normalize_domain_provider_item(item),
        RuleProviderBehavior::Ipcidr => normalize_ipcidr_provider_item(item),
        RuleProviderBehavior::Classical => normalize_classical_provider_item(item)?,
    };

    let parts = parse_rule_payload(&rule)?;
    if parts.rule_type.eq_ignore_ascii_case("RULE-SET") {
        bail!("nested RULE-SET providers are not supported");
    }
    parse_rule(&rule)?;
    Ok(Some(rule))
}

fn normalize_domain_provider_item(item: &str) -> String {
    if item.contains(',') {
        append_or_replace_rule_target(item)
    } else {
        format!("DOMAIN-SUFFIX,{item},{RULE_SET_INTERNAL_TARGET}")
    }
}

fn normalize_ipcidr_provider_item(item: &str) -> String {
    if item.contains(',') {
        append_or_replace_rule_target(item)
    } else {
        format!("IP-CIDR,{item},{RULE_SET_INTERNAL_TARGET}")
    }
}

fn normalize_classical_provider_item(item: &str) -> Result<String> {
    if !item.contains(',') {
        bail!("classical RULE-SET item must include a rule type and payload");
    }
    Ok(append_or_replace_rule_target(item))
}

fn append_or_replace_rule_target(item: &str) -> String {
    let parts = parse_rule_payload(item).unwrap_or_else(|_| RuleParts {
        rule_type: item.to_owned(),
        payload: String::new(),
        target: String::new(),
        params: Vec::new(),
    });
    if parts.rule_type == "MATCH" {
        return format!("MATCH,{RULE_SET_INTERNAL_TARGET}");
    }
    let mut rule = format!("{},{},{}", parts.rule_type, parts.payload, RULE_SET_INTERNAL_TARGET);
    if !parts.params.is_empty() {
        rule.push(',');
        rule.push_str(&parts.params.join(","));
    }
    rule
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

fn rule_matches<'a>(
    rule: &'a ParsedRule,
    meta: &ConnectionMeta,
    host_lower: &str,
    geo_data: &RuleGeoData,
    rule_sets: &RuleSetData,
) -> Option<&'a str> {
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
        ParsedRule::GeoIp { code, is_src, target } => {
            let ip = if *is_src { meta.src_ip } else { meta.dst_ip };
            ip.and_then(|ip| {
                if (code == "lan" && is_lan_ip(ip)) || geo_data.geoip_matches(code, ip) {
                    Some(target.as_str())
                } else {
                    None
                }
            })
        }
        ParsedRule::GeoSite { code, target } => {
            if geo_data.geosite_matches(code, host_lower) {
                Some(target)
            } else {
                None
            }
        }
        ParsedRule::IpAsn { asn, is_src, target } => {
            let ip = if *is_src { meta.src_ip } else { meta.dst_ip };
            ip.and_then(|ip| {
                if geo_data.asn_matches(*asn, ip) {
                    Some(target.as_str())
                } else {
                    None
                }
            })
        }
        ParsedRule::RuleSet { name, target } => {
            if rule_sets.matches(name, meta) {
                Some(target.as_str())
            } else {
                None
            }
        }
        ParsedRule::ProcessName { name, target } => {
            if !meta.process_name.is_empty() && meta.process_name.eq_ignore_ascii_case(name) {
                Some(target.as_str())
            } else {
                None
            }
        }
        ParsedRule::ProcessPath { path, target } => {
            if !meta.process_path.is_empty() && meta.process_path.eq_ignore_ascii_case(path) {
                Some(target.as_str())
            } else {
                None
            }
        }
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
        ParsedRule::GeoIp { is_src: true, .. } => "SRC-GEOIP",
        ParsedRule::GeoIp { .. } => "GEOIP",
        ParsedRule::GeoSite { .. } => "GEOSITE",
        ParsedRule::IpAsn { is_src: true, .. } => "SRC-IP-ASN",
        ParsedRule::IpAsn { .. } => "IP-ASN",
        ParsedRule::RuleSet { .. } => "RULE-SET",
        ParsedRule::ProcessName { .. } => "PROCESS-NAME",
        ParsedRule::ProcessPath { .. } => "PROCESS-PATH",
        ParsedRule::ExternalData { rule_type, .. } => rule_type,
    }
}

// ---------------------------------------------------------------------------
// IP helpers
// ---------------------------------------------------------------------------

fn is_lan_ip(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(ip) => {
            ip.is_private()
                || ip.is_unspecified()
                || ip.is_loopback()
                || ip.is_multicast()
                || ip.is_link_local()
                || ip.is_broadcast()
        }
        IpAddr::V6(ip) => {
            ip.is_unspecified()
                || ip.is_loopback()
                || ip.is_multicast()
                || ip.is_unicast_link_local()
                || ip.is_unique_local()
        }
    }
}

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
    use crate::core::rule_geodata::{AsnData, GeoIpData, GeoSiteData, GeoSiteDomainType, RuleGeoData};
    use std::{collections::HashMap, fs, path::PathBuf};

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

    fn meta_process_name(process_name: &str) -> ConnectionMeta {
        ConnectionMeta {
            process_name: process_name.to_owned(),
            ..Default::default()
        }
    }

    fn meta_process_path(process_path: &str) -> ConnectionMeta {
        ConnectionMeta {
            process_path: process_path.to_owned(),
            ..Default::default()
        }
    }

    fn file_provider(path: PathBuf, behavior: RuleProviderBehavior) -> RuleProviderConfig {
        RuleProviderConfig {
            provider_type: "file".to_string(),
            behavior,
            path: Some(path),
            payload: Vec::new(),
            format: None,
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
    fn process_name_matches_case_insensitively() {
        let engine = RuleEngine::from_rules(&["PROCESS-NAME,Telegram.exe,Proxy", "MATCH,DIRECT"]).unwrap();
        let result = engine.match_connection(&meta_process_name("telegram.EXE"));

        assert_eq!(result.target.as_deref(), Some("Proxy"));
        assert_eq!(result.rule_type.as_deref(), Some("PROCESS-NAME"));
    }

    #[test]
    fn process_name_without_metadata_falls_through() {
        let engine = RuleEngine::from_rules(&["PROCESS-NAME,Telegram.exe,Proxy", "MATCH,DIRECT"]).unwrap();

        assert_eq!(
            engine.match_connection(&ConnectionMeta::default()).target.as_deref(),
            Some("DIRECT")
        );
    }

    #[test]
    fn process_path_matches_case_insensitively() {
        let engine = RuleEngine::from_rules(&[
            "PROCESS-PATH,C:\\Program Files\\Telegram\\Telegram.exe,Proxy",
            "MATCH,DIRECT",
        ])
        .unwrap();
        let result = engine.match_connection(&meta_process_path("c:\\program files\\telegram\\telegram.EXE"));

        assert_eq!(result.target.as_deref(), Some("Proxy"));
        assert_eq!(result.rule_type.as_deref(), Some("PROCESS-PATH"));
    }

    #[test]
    fn process_path_without_metadata_falls_through() {
        let engine = RuleEngine::from_rules(&[
            "PROCESS-PATH,C:\\Program Files\\Telegram\\Telegram.exe,Proxy",
            "MATCH,DIRECT",
        ])
        .unwrap();

        assert_eq!(
            engine.match_connection(&ConnectionMeta::default()).target.as_deref(),
            Some("DIRECT")
        );
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
        assert!(validate_rule("PROCESS-NAME,Telegram.exe,Proxy").valid);
        assert!(validate_rule("PROCESS-PATH,C:\\Program Files\\Telegram\\Telegram.exe,Proxy").valid);
        assert!(validate_rule("MATCH,DIRECT").valid);

        assert!(!validate_rule("DOMAIN").valid);
        assert!(!validate_rule("IP-CIDR,not-a-cidr,Direct").valid);
        assert!(!validate_rule("IP-SUFFIX,1.2.3.4/40,Direct").valid);
        assert!(!validate_rule("IP-ASN,not-a-number,Direct").valid);
        assert!(!validate_rule("DST-PORT,notaport,Direct").valid);
        assert!(!validate_rule("UNKNOWN-TYPE,foo,bar").valid);
    }

    #[test]
    fn geoip_falls_through_without_geodata() {
        let v = validate_rule("GEOIP,CN,DIRECT,no-resolve");
        assert!(v.valid);
        let engine = RuleEngine::from_rules(&["GEOIP,CN,DIRECT,no-resolve", "MATCH,Proxy"]).unwrap();
        let r = engine.match_connection(&meta_ip("1.2.3.4", 80));
        assert_eq!(r.target.as_deref(), Some("Proxy"));
    }

    #[test]
    fn geoip_matches_with_geodata() {
        let geoip = GeoIpData::from_cidr_map(HashMap::from([(
            "cn".to_string(),
            vec![("203.0.113.0".parse().unwrap(), 24)],
        )]));
        let geo_data = RuleGeoData::from_parts(Some(geoip), None, None);
        let engine =
            RuleEngine::from_rules_with_geo_data(&["GEOIP,CN,DIRECT,no-resolve", "MATCH,Proxy"], geo_data).unwrap();

        assert_eq!(
            engine.match_connection(&meta_ip("203.0.113.10", 80)).target.as_deref(),
            Some("DIRECT")
        );
        assert_eq!(
            engine.match_connection(&meta_ip("198.51.100.10", 80)).target.as_deref(),
            Some("Proxy")
        );
    }

    #[test]
    fn geosite_matches_with_geodata() {
        let geosite = GeoSiteData::from_site_map(HashMap::from([(
            "cn".to_string(),
            vec![(GeoSiteDomainType::Domain, "example.cn".to_string())],
        )]))
        .unwrap();
        let geo_data = RuleGeoData::from_parts(None, Some(geosite), None);
        let engine = RuleEngine::from_rules_with_geo_data(&["GEOSITE,CN,DIRECT", "MATCH,Proxy"], geo_data).unwrap();

        assert_eq!(
            engine
                .match_connection(&meta_domain("www.example.cn"))
                .target
                .as_deref(),
            Some("DIRECT")
        );
        assert_eq!(
            engine
                .match_connection(&meta_domain("www.example.com"))
                .target
                .as_deref(),
            Some("Proxy")
        );
    }

    #[test]
    fn geoip_lan_matches_without_external_data() {
        let engine = RuleEngine::from_rules(&["GEOIP,LAN,DIRECT", "MATCH,Proxy"]).unwrap();
        assert_eq!(
            engine.match_connection(&meta_ip("192.168.1.1", 80)).target.as_deref(),
            Some("DIRECT")
        );
    }

    #[test]
    fn ip_asn_falls_through_without_geodata() {
        let v = validate_rule("IP-ASN,13335,DIRECT");
        assert!(v.valid);
        let engine = RuleEngine::from_rules(&["IP-ASN,13335,DIRECT", "MATCH,Proxy"]).unwrap();
        assert_eq!(
            engine.match_connection(&meta_ip("1.1.1.1", 443)).target.as_deref(),
            Some("Proxy")
        );
    }

    #[test]
    fn ip_asn_matches_destination_with_geodata() {
        let asn_data = AsnData::from_asn_map(HashMap::from([("1.1.1.1".parse().unwrap(), 13335)]));
        let geo_data = RuleGeoData::from_parts(None, None, Some(asn_data));
        let engine = RuleEngine::from_rules_with_geo_data(&["IP-ASN,13335,DIRECT", "MATCH,Proxy"], geo_data).unwrap();

        assert_eq!(
            engine.match_connection(&meta_ip("1.1.1.1", 443)).target.as_deref(),
            Some("DIRECT")
        );
        assert_eq!(
            engine.match_connection(&meta_ip("8.8.8.8", 443)).target.as_deref(),
            Some("Proxy")
        );
    }

    #[test]
    fn src_ip_asn_matches_source_with_geodata() {
        let asn_data = AsnData::from_asn_map(HashMap::from([("8.8.8.8".parse().unwrap(), 15169)]));
        let geo_data = RuleGeoData::from_parts(None, None, Some(asn_data));
        let engine =
            RuleEngine::from_rules_with_geo_data(&["SRC-IP-ASN,15169,Proxy", "MATCH,DIRECT"], geo_data).unwrap();
        let mut meta = meta_ip("1.1.1.1", 443);
        meta.src_ip = Some("8.8.8.8".parse().unwrap());

        assert_eq!(engine.match_connection(&meta).target.as_deref(), Some("Proxy"));
    }

    #[test]
    fn ip_asn_accepts_ipv6_query_path() {
        let asn_data = AsnData::from_asn_map(HashMap::from([("2606:4700:4700::1111".parse().unwrap(), 13335)]));
        let geo_data = RuleGeoData::from_parts(None, None, Some(asn_data));
        let engine = RuleEngine::from_rules_with_geo_data(&["IP-ASN,13335,DIRECT", "MATCH,Proxy"], geo_data).unwrap();
        let mut meta = ConnectionMeta::default();
        meta.dst_ip = Some("2606:4700:4700::1111".parse().unwrap());

        assert_eq!(engine.match_connection(&meta).target.as_deref(), Some("DIRECT"));
    }

    #[test]
    fn rule_set_yaml_domain_provider_matches_outer_target() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("private.yaml");
        fs::write(&path, "payload:\n  - DOMAIN-SUFFIX,example.com\n").unwrap();
        let rule_sets = RuleSetData::from_rule_providers(HashMap::from([(
            "private".to_string(),
            file_provider(path, RuleProviderBehavior::Classical),
        )]))
        .unwrap();
        let engine =
            RuleEngine::from_rules_with_rule_sets(&["RULE-SET,private,DIRECT", "MATCH,Proxy"], rule_sets).unwrap();

        assert_eq!(
            engine
                .match_connection(&meta_domain("www.example.com"))
                .target
                .as_deref(),
            Some("DIRECT")
        );
        assert_eq!(
            engine
                .match_connection(&meta_domain("www.example.net"))
                .target
                .as_deref(),
            Some("Proxy")
        );
    }

    #[test]
    fn rule_set_yaml_ipcidr_provider_matches_ip() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("private-ip.yaml");
        fs::write(&path, "payload:\n  - 10.0.0.0/8\n").unwrap();
        let rule_sets = RuleSetData::from_rule_providers(HashMap::from([(
            "private-ip".to_string(),
            file_provider(path, RuleProviderBehavior::Ipcidr),
        )]))
        .unwrap();
        let engine =
            RuleEngine::from_rules_with_rule_sets(&["RULE-SET,private-ip,DIRECT", "MATCH,Proxy"], rule_sets).unwrap();

        assert_eq!(
            engine.match_connection(&meta_ip("10.1.2.3", 443)).target.as_deref(),
            Some("DIRECT")
        );
        assert_eq!(
            engine
                .match_connection(&meta_ip("198.51.100.10", 443))
                .target
                .as_deref(),
            Some("Proxy")
        );
    }

    #[test]
    fn rule_set_text_domain_provider_matches_domain_suffix() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("domain.txt");
        fs::write(&path, "example.org\n# ignored\n\n").unwrap();
        let rule_sets = RuleSetData::from_rule_providers(HashMap::from([(
            "domain".to_string(),
            file_provider(path, RuleProviderBehavior::Domain),
        )]))
        .unwrap();
        let engine =
            RuleEngine::from_rules_with_rule_sets(&["RULE-SET,domain,Proxy", "MATCH,DIRECT"], rule_sets).unwrap();

        assert_eq!(
            engine
                .match_connection(&meta_domain("api.example.org"))
                .target
                .as_deref(),
            Some("Proxy")
        );
    }

    #[test]
    fn rule_set_outer_target_overrides_provider_rule_target() {
        let provider = RuleProviderConfig {
            provider_type: "inline".to_string(),
            behavior: RuleProviderBehavior::Classical,
            path: None,
            payload: vec!["DOMAIN-SUFFIX,example.com,REJECT".to_string()],
            format: None,
        };
        let rule_sets = RuleSetData::from_rule_providers(HashMap::from([("reject".to_string(), provider)])).unwrap();
        let engine =
            RuleEngine::from_rules_with_rule_sets(&["RULE-SET,reject,DIRECT", "MATCH,Proxy"], rule_sets).unwrap();

        assert_eq!(
            engine
                .match_connection(&meta_domain("www.example.com"))
                .target
                .as_deref(),
            Some("DIRECT")
        );
    }

    #[test]
    fn rule_set_missing_provider_falls_through() {
        let engine =
            RuleEngine::from_rules_with_rule_sets(&["RULE-SET,missing,DIRECT", "MATCH,Proxy"], RuleSetData::empty())
                .unwrap();

        assert_eq!(
            engine
                .match_connection(&meta_domain("www.example.com"))
                .target
                .as_deref(),
            Some("Proxy")
        );
    }

    #[test]
    fn rule_set_nested_provider_is_rejected() {
        let provider = RuleProviderConfig {
            provider_type: "inline".to_string(),
            behavior: RuleProviderBehavior::Classical,
            path: None,
            payload: vec!["RULE-SET,other".to_string()],
            format: None,
        };

        assert!(RuleSetData::from_rule_providers(HashMap::from([("nested".to_string(), provider)])).is_err());
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
