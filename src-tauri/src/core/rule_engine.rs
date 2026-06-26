use anyhow::{Context, Result, bail};
use regex::{Regex, RegexBuilder};
use serde::{Deserialize, Serialize};
use std::{collections::HashSet, net::IpAddr};

use super::rule_geodata::RuleGeoData;

mod provider;

pub use provider::{RuleProviderConfig, RuleSetData, SubRuleData};

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
    #[serde(default)]
    pub uid: u32,
    #[serde(default)]
    pub dscp: u8,
    #[serde(
        default,
        alias = "type",
        alias = "inType",
        alias = "inbound_type",
        alias = "inboundType"
    )]
    pub in_type: String,
    #[serde(default, alias = "inUser", alias = "inbound_user", alias = "inboundUser")]
    pub in_user: String,
    #[serde(default, alias = "inName", alias = "inbound_name", alias = "inboundName")]
    pub in_name: String,
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
// Kernel bridge — let the learn-gripe router query loaded rule-set providers
// ---------------------------------------------------------------------------

/// Bridge the app's locally-loaded rule-set providers into the kernel router's
/// `RULE-SET` matcher. The kernel only ever queries a set by name through this
/// trait; it never reads or fetches provider payloads itself — the app loads
/// them in [`RuleSetData::from_rule_providers`].
impl learn_gripe::RuleSetLookup for RuleSetData {
    fn rule_set_matches(&self, name: &str, target: &learn_gripe::TargetAddr) -> bool {
        self.matches(name, &connection_meta_from_target(target))
    }
}

/// Build the [`ConnectionMeta`] the rule-set engine matches against from the
/// kernel's connection target. A domain target only sets `host`, a resolved-IP
/// target only sets `dst_ip`, so domain-behaviour sets match hostnames and
/// ipcidr-behaviour sets match addresses, mirroring how the router feeds the
/// geo matchers.
fn connection_meta_from_target(target: &learn_gripe::TargetAddr) -> ConnectionMeta {
    match target {
        learn_gripe::TargetAddr::Domain(host, port) => ConnectionMeta {
            host: host.to_ascii_lowercase(),
            dst_port: *port,
            ..ConnectionMeta::default()
        },
        learn_gripe::TargetAddr::Ip(addr) => ConnectionMeta {
            dst_ip: Some(addr.ip()),
            dst_port: addr.port(),
            ..ConnectionMeta::default()
        },
    }
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
    pub outcome: String,
    pub explanation: String,
    pub trace: Vec<RuleTraceStep>,
}

#[derive(Debug, Clone, Serialize)]
pub struct RuleTraceStep {
    pub rule_index: usize,
    pub rule_raw: String,
    pub rule_type: String,
    pub matched: bool,
    pub target: Option<String>,
    pub detail: Option<RuleTraceDetail>,
}

#[derive(Debug, Clone, Serialize)]
pub struct RuleTraceDetail {
    pub reference_type: String,
    pub name: String,
    pub condition_matched: Option<bool>,
    pub matched_rule_raw: Option<String>,
    pub matched_rule_type: Option<String>,
    pub matched_target: Option<String>,
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
    ProcessNameRegex {
        regex: Regex,
        target: String,
    },
    ProcessPathRegex {
        regex: Regex,
        target: String,
    },
    ProcessNameWildcard {
        pattern: String,
        target: String,
    },
    ProcessPathWildcard {
        pattern: String,
        target: String,
    },
    Uid {
        ranges: Vec<(u32, u32)>,
        target: String,
    },
    Dscp {
        ranges: Vec<(u8, u8)>,
        target: String,
    },
    InType {
        types: Vec<String>,
        target: String,
    },
    InUser {
        users: Vec<String>,
        target: String,
    },
    InName {
        names: Vec<String>,
        target: String,
    },
    Logical {
        op: LogicalOp,
        rules: Vec<ParsedRule>,
        target: String,
    },
    SubRule {
        condition: Box<ParsedRule>,
        name: String,
    },
}

#[derive(Debug, Clone, Copy)]
enum PortKind {
    Src,
    Dst,
    In,
}

#[derive(Debug, Clone, Copy)]
enum LogicalOp {
    And,
    Or,
    Not,
}

// ---------------------------------------------------------------------------
// Rule engine
// ---------------------------------------------------------------------------

pub struct RuleEngine {
    rules: Vec<(ParsedRule, String)>, // (parsed, raw_string)
    geo_data: RuleGeoData,
    rule_sets: RuleSetData,
    sub_rules: SubRuleData,
}

impl RuleEngine {
    pub fn from_rules(raw_rules: &[&str]) -> Result<Self> {
        Self::from_rules_with_geo_data_rule_sets_and_sub_rules(
            raw_rules,
            RuleGeoData::empty(),
            RuleSetData::empty(),
            SubRuleData::empty(),
        )
    }

    pub fn from_rules_with_default_geo_data(raw_rules: &[&str]) -> Result<Self> {
        Self::from_rules_with_geo_data_rule_sets_and_sub_rules(
            raw_rules,
            RuleGeoData::load_default(),
            RuleSetData::empty(),
            SubRuleData::empty(),
        )
    }

    pub fn from_rules_with_default_geo_data_and_rule_sets(raw_rules: &[&str], rule_sets: RuleSetData) -> Result<Self> {
        Self::from_rules_with_default_geo_data_rule_sets_and_sub_rules(raw_rules, rule_sets, SubRuleData::empty())
    }

    pub fn from_rules_with_default_geo_data_rule_sets_and_sub_rules(
        raw_rules: &[&str],
        rule_sets: RuleSetData,
        sub_rules: SubRuleData,
    ) -> Result<Self> {
        Self::from_rules_with_geo_data_rule_sets_and_sub_rules(
            raw_rules,
            RuleGeoData::load_default(),
            rule_sets,
            sub_rules,
        )
    }

    pub fn from_rules_with_geo_data(raw_rules: &[&str], geo_data: RuleGeoData) -> Result<Self> {
        Self::from_rules_with_geo_data_rule_sets_and_sub_rules(
            raw_rules,
            geo_data,
            RuleSetData::empty(),
            SubRuleData::empty(),
        )
    }

    pub fn from_rules_with_rule_sets(raw_rules: &[&str], rule_sets: RuleSetData) -> Result<Self> {
        Self::from_rules_with_geo_data_rule_sets_and_sub_rules(
            raw_rules,
            RuleGeoData::empty(),
            rule_sets,
            SubRuleData::empty(),
        )
    }

    fn from_rules_with_geo_data_rule_sets_and_sub_rules(
        raw_rules: &[&str],
        geo_data: RuleGeoData,
        rule_sets: RuleSetData,
        sub_rules: SubRuleData,
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
            sub_rules,
        })
    }

    pub fn match_connection(&self, meta: &ConnectionMeta) -> RuleMatchResult {
        self.match_connection_impl(meta, false)
    }

    pub fn explain_connection(&self, meta: &ConnectionMeta) -> RuleMatchResult {
        self.match_connection_impl(meta, true)
    }

    fn match_connection_impl(&self, meta: &ConnectionMeta, include_trace: bool) -> RuleMatchResult {
        let host = meta.host.to_ascii_lowercase();
        let mut trace = include_trace.then(|| Vec::with_capacity(self.rules.len()));
        for (i, (rule, raw)) in self.rules.iter().enumerate() {
            let target = rule_matches(rule, meta, &host, &self.geo_data, &self.rule_sets, &self.sub_rules);
            if let Some(trace) = trace.as_mut() {
                trace.push(rule_trace_step(
                    i,
                    rule,
                    raw,
                    meta,
                    &host,
                    target,
                    &self.geo_data,
                    &self.rule_sets,
                    &self.sub_rules,
                ));
            }
            if let Some(target) = target {
                let rule_type = rule_type_name(rule);
                return RuleMatchResult {
                    matched: true,
                    rule_index: Some(i),
                    rule_raw: Some(raw.clone()),
                    target: Some(target.to_owned()),
                    rule_type: Some(rule_type.to_owned()),
                    outcome: "matched".to_owned(),
                    explanation: format!("matched rule #{i} ({rule_type}) -> {target}"),
                    trace: trace.unwrap_or_default(),
                };
            }
        }
        RuleMatchResult {
            matched: false,
            rule_index: None,
            rule_raw: None,
            target: None,
            rule_type: None,
            outcome: "fallthrough".to_owned(),
            explanation: "no rules matched; fallthrough without target".to_owned(),
            trace: trace.unwrap_or_default(),
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
    parse_rule_with_target(raw, true)
}

fn parse_logic_child_rule(raw: &str) -> Result<ParsedRule> {
    let rule = parse_rule_with_target(raw, false)?;
    match rule {
        ParsedRule::Match { .. } | ParsedRule::SubRule { .. } => {
            bail!("unsupported rule type [{raw}] on logic rule");
        }
        _ => Ok(rule),
    }
}

fn parse_rule_with_target(raw: &str, need_target: bool) -> Result<ParsedRule> {
    let parts = parse_rule_payload(raw, need_target)?;
    let rule_type_upper = parts.rule_type.to_ascii_uppercase();

    if rule_type_upper == "MATCH" {
        if !need_target {
            bail!("MATCH is not supported inside logic rules");
        }
        if parts.target.is_empty() {
            bail!("MATCH rule requires a target policy");
        }
        return Ok(ParsedRule::Match { target: parts.target });
    }

    if parts.payload.is_empty() || (need_target && parts.target.is_empty()) {
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
        "PROCESS-NAME-REGEX" => Ok(ParsedRule::ProcessNameRegex {
            regex: RegexBuilder::new(payload)
                .case_insensitive(true)
                .build()
                .context("invalid PROCESS-NAME-REGEX pattern")?,
            target,
        }),
        "PROCESS-PATH-REGEX" => Ok(ParsedRule::ProcessPathRegex {
            regex: RegexBuilder::new(payload)
                .case_insensitive(true)
                .build()
                .context("invalid PROCESS-PATH-REGEX pattern")?,
            target,
        }),
        "PROCESS-NAME-WILDCARD" => Ok(ParsedRule::ProcessNameWildcard {
            pattern: payload.to_ascii_lowercase(),
            target,
        }),
        "PROCESS-PATH-WILDCARD" => Ok(ParsedRule::ProcessPathWildcard {
            pattern: payload.to_ascii_lowercase(),
            target,
        }),
        "UID" => Ok(ParsedRule::Uid {
            ranges: parse_uid_ranges(payload)?,
            target,
        }),
        "DSCP" => Ok(ParsedRule::Dscp {
            ranges: parse_dscp_ranges(payload)?,
            target,
        }),
        "IN-TYPE" => Ok(ParsedRule::InType {
            types: parse_in_types(payload)?,
            target,
        }),
        "IN-USER" => Ok(ParsedRule::InUser {
            users: parse_slash_list(payload, "IN-USER")?,
            target,
        }),
        "IN-NAME" => Ok(ParsedRule::InName {
            names: parse_slash_list(payload, "IN-NAME")?,
            target,
        }),
        "AND" => Ok(ParsedRule::Logical {
            op: LogicalOp::And,
            rules: parse_logical_rules(payload, LogicalOp::And)?,
            target,
        }),
        "OR" => Ok(ParsedRule::Logical {
            op: LogicalOp::Or,
            rules: parse_logical_rules(payload, LogicalOp::Or)?,
            target,
        }),
        "NOT" => Ok(ParsedRule::Logical {
            op: LogicalOp::Not,
            rules: parse_logical_rules(payload, LogicalOp::Not)?,
            target,
        }),
        "SUB-RULE" => Ok(ParsedRule::SubRule {
            condition: Box::new(parse_sub_rule_condition(payload)?),
            name: target,
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

fn parse_rule_payload(raw: &str, need_target: bool) -> Result<RuleParts> {
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
                if need_target {
                    if let Some(last) = items.last() {
                        target = last.clone();
                    }
                    if items.len() > 2 {
                        payload = items[1..items.len() - 1].join(",");
                    }
                } else {
                    payload = items[1..items.len() - 1].join(",");
                    if let Some(last) = items.last() {
                        payload.push_str(if payload.is_empty() { "" } else { "," });
                        payload.push_str(last);
                    }
                }
            }
            _ => {
                payload = items[1].clone();
                if need_target && items.len() > 2 {
                    target = items[2].clone();
                }
                let param_start = if need_target { 3 } else { 2 };
                if items.len() > param_start {
                    params = items[param_start..].to_vec();
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

fn parse_logical_rules(payload: &str, op: LogicalOp) -> Result<Vec<ParsedRule>> {
    let rules = logical_rule_segments(payload)?
        .into_iter()
        .map(|segment| parse_logic_child_rule(segment))
        .collect::<Result<Vec<_>>>()?;
    if rules.is_empty() {
        bail!("logic rule payload must contain at least one rule");
    }
    if matches!(op, LogicalOp::Not) && rules.len() != 1 {
        bail!("NOT rule must contain one rule");
    }
    Ok(rules)
}

fn parse_sub_rule_condition(payload: &str) -> Result<ParsedRule> {
    let payload = payload.trim();
    if payload.starts_with('(')
        && payload.ends_with(')')
        && let Some(inner) = unwrap_single_parenthesized_rule(payload)?
    {
        parse_logic_child_rule(inner)
    } else {
        parse_logic_child_rule(payload)
    }
}

fn logical_rule_segments(payload: &str) -> Result<Vec<&str>> {
    let payload = payload.trim();
    if payload.is_empty() {
        bail!("logic rule payload cannot be empty");
    }
    if let Some(inner) = unwrap_single_parenthesized_rule(payload)? {
        let inner_segments = direct_parenthesized_segments(inner)?;
        if !inner_segments.is_empty() {
            return Ok(inner_segments);
        }
    }
    direct_parenthesized_segments(payload)
}

fn unwrap_single_parenthesized_rule(payload: &str) -> Result<Option<&str>> {
    let ranges = parenthesized_ranges(payload)?;
    Ok(if ranges.len() == 1 && ranges[0] == (0, payload.len() - 1) {
        Some(&payload[1..payload.len() - 1])
    } else {
        None
    })
}

fn direct_parenthesized_segments(payload: &str) -> Result<Vec<&str>> {
    parenthesized_ranges(payload)?
        .into_iter()
        .map(|(start, end)| Ok(&payload[start + 1..end]))
        .collect()
}

fn parenthesized_ranges(payload: &str) -> Result<Vec<(usize, usize)>> {
    let mut ranges = Vec::new();
    let mut depth = 0usize;
    let mut start = None;
    for (idx, byte) in payload.bytes().enumerate() {
        match byte {
            b'(' => {
                if depth == 0 {
                    start = Some(idx);
                }
                depth += 1;
            }
            b')' => {
                if depth == 0 {
                    bail!("logic rule payload has unmatched ')'");
                }
                depth -= 1;
                if depth == 0 {
                    let Some(range_start) = start.take() else {
                        bail!("logic rule payload has invalid parentheses");
                    };
                    ranges.push((range_start, idx));
                }
            }
            b',' | b' ' | b'\t' | b'\r' | b'\n' if depth == 0 => {}
            _ if depth == 0 => {
                bail!("logic rule payload must wrap each child rule in parentheses");
            }
            _ => {}
        }
    }
    if depth != 0 {
        bail!("logic rule payload has unmatched '('");
    }
    Ok(ranges)
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

fn parse_uid_ranges(s: &str) -> Result<Vec<(u32, u32)>> {
    let s = s.trim();
    if s.is_empty() || s == "*" {
        bail!("UID payload must contain at least one numeric UID or UID range");
    }

    let parts = s.replace(',', "/");
    let parts = parts
        .split('/')
        .filter(|part| !part.trim().is_empty())
        .collect::<Vec<_>>();
    if parts.is_empty() {
        bail!("UID payload must contain at least one numeric UID or UID range");
    }
    if parts.len() > 28 {
        bail!("UID supports at most 28 ranges");
    }

    parts
        .into_iter()
        .map(|part| {
            let part = part.trim();
            if let Some((start_s, end_s)) = part.split_once('-') {
                let start = parse_uid_value(start_s)?;
                let end = parse_uid_value(end_s)?;
                Ok(if start <= end { (start, end) } else { (end, start) })
            } else {
                let uid = parse_uid_value(part)?;
                Ok((uid, uid))
            }
        })
        .collect()
}

fn parse_uid_value(s: &str) -> Result<u32> {
    s.trim_matches([' ', '[', ']'])
        .parse()
        .context("UID payload must be numeric")
}

fn parse_dscp_ranges(s: &str) -> Result<Vec<(u8, u8)>> {
    let s = s.trim();
    if s.is_empty() || s == "*" {
        return Ok(Vec::new());
    }

    let parts = s.replace(',', "/");
    let parts = parts
        .split('/')
        .filter(|part| !part.trim().is_empty())
        .collect::<Vec<_>>();
    if parts.len() > 28 {
        bail!("DSCP supports at most 28 ranges");
    }

    parts
        .into_iter()
        .map(|part| {
            let part = part.trim();
            if let Some((start_s, end_s)) = part.split_once('-') {
                let start = parse_dscp_value(start_s)?;
                let end = parse_dscp_value(end_s)?;
                Ok(if start <= end { (start, end) } else { (end, start) })
            } else {
                let dscp = parse_dscp_value(part)?;
                Ok((dscp, dscp))
            }
        })
        .collect()
}

fn parse_dscp_value(s: &str) -> Result<u8> {
    let dscp: u8 = s
        .trim_matches([' ', '[', ']'])
        .parse()
        .context("DSCP payload must be numeric")?;
    if dscp > 63 {
        bail!("DSCP cannot exceed 63");
    }
    Ok(dscp)
}

fn parse_in_types(s: &str) -> Result<Vec<String>> {
    let mut types = Vec::new();
    for part in s.split('/') {
        let in_type = part.trim();
        if in_type.is_empty() {
            bail!("IN-TYPE payload cannot contain empty types");
        }
        let upper = in_type.to_ascii_uppercase();
        if upper == "SOCKS" {
            types.push("SOCKS4".to_owned());
            types.push("SOCKS5".to_owned());
        } else if is_supported_in_type(&upper) {
            types.push(upper);
        } else {
            bail!("unknown IN-TYPE: {in_type}");
        }
    }
    if types.is_empty() {
        bail!("IN-TYPE payload cannot be empty");
    }
    Ok(types)
}

fn is_supported_in_type(in_type: &str) -> bool {
    matches!(
        in_type,
        "HTTP"
            | "HTTPS"
            | "SOCKS4"
            | "SOCKS5"
            | "SHADOWSOCKS"
            | "SNELL"
            | "VMESS"
            | "VLESS"
            | "REDIR"
            | "TPROXY"
            | "TROJAN"
            | "TUNNEL"
            | "TUN"
            | "TUIC"
            | "HYSTERIA2"
            | "ANYTLS"
            | "MIERU"
            | "SUDOKU"
            | "TRUSTTUNNEL"
            | "INNER"
    )
}

fn parse_slash_list(s: &str, rule_type: &str) -> Result<Vec<String>> {
    let mut values = Vec::new();
    for part in s.split('/') {
        let value = part.trim();
        if value.is_empty() {
            bail!("{rule_type} payload cannot contain empty values");
        }
        values.push(value.to_owned());
    }
    if values.is_empty() {
        bail!("{rule_type} payload cannot be empty");
    }
    Ok(values)
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
    geo_data: &'a RuleGeoData,
    rule_sets: &'a RuleSetData,
    sub_rules: &'a SubRuleData,
) -> Option<&'a str> {
    rule_matches_inner(
        rule,
        meta,
        host_lower,
        geo_data,
        rule_sets,
        sub_rules,
        &mut HashSet::new(),
    )
}

fn rule_matches_inner<'a>(
    rule: &'a ParsedRule,
    meta: &ConnectionMeta,
    host_lower: &str,
    geo_data: &'a RuleGeoData,
    rule_sets: &'a RuleSetData,
    sub_rules: &'a SubRuleData,
    visited_sub_rules: &mut HashSet<String>,
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
        ParsedRule::ProcessNameRegex { regex, target } => {
            if !meta.process_name.is_empty() && regex.is_match(&meta.process_name) {
                Some(target.as_str())
            } else {
                None
            }
        }
        ParsedRule::ProcessPathRegex { regex, target } => {
            if !meta.process_path.is_empty() && regex.is_match(&meta.process_path) {
                Some(target.as_str())
            } else {
                None
            }
        }
        ParsedRule::ProcessNameWildcard { pattern, target } => {
            if !meta.process_name.is_empty() && wildcard_match(pattern, &meta.process_name.to_ascii_lowercase()) {
                Some(target.as_str())
            } else {
                None
            }
        }
        ParsedRule::ProcessPathWildcard { pattern, target } => {
            if !meta.process_path.is_empty() && wildcard_match(pattern, &meta.process_path.to_ascii_lowercase()) {
                Some(target.as_str())
            } else {
                None
            }
        }
        ParsedRule::Uid { ranges, target } => {
            if meta.uid != 0 && ranges.iter().any(|(start, end)| meta.uid >= *start && meta.uid <= *end) {
                Some(target.as_str())
            } else {
                None
            }
        }
        ParsedRule::Dscp { ranges, target } => {
            if ranges.is_empty()
                || ranges
                    .iter()
                    .any(|(start, end)| meta.dscp >= *start && meta.dscp <= *end)
            {
                Some(target.as_str())
            } else {
                None
            }
        }
        ParsedRule::InType { types, target } => {
            let in_type = meta.in_type.to_ascii_uppercase();
            if !in_type.is_empty() && types.iter().any(|rule_type| rule_type == &in_type) {
                Some(target.as_str())
            } else {
                None
            }
        }
        ParsedRule::InUser { users, target } => {
            if !meta.in_user.is_empty() && users.iter().any(|user| user == &meta.in_user) {
                Some(target.as_str())
            } else {
                None
            }
        }
        ParsedRule::InName { names, target } => {
            if !meta.in_name.is_empty() && names.iter().any(|name| name == &meta.in_name) {
                Some(target.as_str())
            } else {
                None
            }
        }
        ParsedRule::Logical { op, rules, target } => {
            let matched = match op {
                LogicalOp::And => rules.iter().all(|rule| {
                    rule_matches_inner(
                        rule,
                        meta,
                        host_lower,
                        geo_data,
                        rule_sets,
                        sub_rules,
                        visited_sub_rules,
                    )
                    .is_some()
                }),
                LogicalOp::Or => rules.iter().any(|rule| {
                    rule_matches_inner(
                        rule,
                        meta,
                        host_lower,
                        geo_data,
                        rule_sets,
                        sub_rules,
                        visited_sub_rules,
                    )
                    .is_some()
                }),
                LogicalOp::Not => rules.first().is_some_and(|rule| {
                    rule_matches_inner(
                        rule,
                        meta,
                        host_lower,
                        geo_data,
                        rule_sets,
                        sub_rules,
                        visited_sub_rules,
                    )
                    .is_none()
                }),
            };
            matched.then_some(target.as_str())
        }
        ParsedRule::SubRule { condition, name } => rule_matches_inner(
            condition,
            meta,
            host_lower,
            geo_data,
            rule_sets,
            sub_rules,
            visited_sub_rules,
        )
        .and_then(|_| sub_rules.match_named(name, meta, host_lower, geo_data, rule_sets, visited_sub_rules)),
    }
}

fn rule_trace_step(
    rule_index: usize,
    rule: &ParsedRule,
    raw: &str,
    meta: &ConnectionMeta,
    host_lower: &str,
    target: Option<&str>,
    geo_data: &RuleGeoData,
    rule_sets: &RuleSetData,
    sub_rules: &SubRuleData,
) -> RuleTraceStep {
    RuleTraceStep {
        rule_index,
        rule_raw: raw.to_owned(),
        rule_type: rule_type_name(rule).to_owned(),
        matched: target.is_some(),
        target: target.map(str::to_owned),
        detail: rule_trace_detail(rule, meta, host_lower, geo_data, rule_sets, sub_rules),
    }
}

fn rule_trace_detail(
    rule: &ParsedRule,
    meta: &ConnectionMeta,
    host_lower: &str,
    geo_data: &RuleGeoData,
    rule_sets: &RuleSetData,
    sub_rules: &SubRuleData,
) -> Option<RuleTraceDetail> {
    match rule {
        ParsedRule::RuleSet { name, .. } => {
            let inner = rule_sets.explain(name, meta);
            Some(RuleTraceDetail {
                reference_type: "rule_set".to_owned(),
                name: name.clone(),
                condition_matched: None,
                matched_rule_raw: inner.as_ref().and_then(|result| result.rule_raw.clone()),
                matched_rule_type: inner.as_ref().and_then(|result| result.rule_type.clone()),
                matched_target: inner.as_ref().and_then(|result| result.target.clone()),
            })
        }
        ParsedRule::SubRule { condition, name } => {
            let mut condition_visited = HashSet::new();
            let condition_matched = rule_matches_inner(
                condition,
                meta,
                host_lower,
                geo_data,
                rule_sets,
                sub_rules,
                &mut condition_visited,
            )
            .is_some();
            let sub_match = if condition_matched {
                let mut sub_rule_visited = HashSet::new();
                sub_rules.explain_match_named(name, meta, host_lower, geo_data, rule_sets, &mut sub_rule_visited)
            } else {
                None
            };
            Some(RuleTraceDetail {
                reference_type: "sub_rule".to_owned(),
                name: name.clone(),
                condition_matched: Some(condition_matched),
                matched_rule_raw: sub_match.as_ref().map(|result| result.rule_raw.clone()),
                matched_rule_type: sub_match.as_ref().map(|result| result.rule_type.clone()),
                matched_target: sub_match.as_ref().map(|result| result.target.clone()),
            })
        }
        _ => None,
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
        ParsedRule::ProcessNameRegex { .. } => "PROCESS-NAME-REGEX",
        ParsedRule::ProcessPathRegex { .. } => "PROCESS-PATH-REGEX",
        ParsedRule::ProcessNameWildcard { .. } => "PROCESS-NAME-WILDCARD",
        ParsedRule::ProcessPathWildcard { .. } => "PROCESS-PATH-WILDCARD",
        ParsedRule::Uid { .. } => "UID",
        ParsedRule::Dscp { .. } => "DSCP",
        ParsedRule::InType { .. } => "IN-TYPE",
        ParsedRule::InUser { .. } => "IN-USER",
        ParsedRule::InName { .. } => "IN-NAME",
        ParsedRule::Logical { op: LogicalOp::And, .. } => "AND",
        ParsedRule::Logical { op: LogicalOp::Or, .. } => "OR",
        ParsedRule::Logical { op: LogicalOp::Not, .. } => "NOT",
        ParsedRule::SubRule { .. } => "SUB-RULE",
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
mod tests;
