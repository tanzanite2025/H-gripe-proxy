use anyhow::{Context, Result, bail};
use regex::{Regex, RegexBuilder};
use serde::{Deserialize, Serialize};
use std::{
    collections::{HashMap, HashSet},
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

    fn explain(&self, name: &str, meta: &ConnectionMeta) -> Option<RuleMatchResult> {
        self.sets.get(name).map(|matcher| matcher.explain(meta))
    }
}

#[derive(Default)]
pub struct SubRuleData {
    sets: HashMap<String, SubRuleMatcher>,
}

impl SubRuleData {
    pub fn empty() -> Self {
        Self::default()
    }

    pub fn from_sub_rules(sub_rules: HashMap<String, Vec<String>>) -> Result<Self> {
        let mut sets = HashMap::new();
        for (name, raw_rules) in sub_rules {
            if name.is_empty() {
                bail!("sub-rule name cannot be empty");
            }
            let matcher =
                SubRuleMatcher::from_rules(&raw_rules).with_context(|| format!("failed to load sub-rule {name}"))?;
            sets.insert(name, matcher);
        }
        let data = Self { sets };
        data.validate_references()?;
        Ok(data)
    }

    fn match_named<'a>(
        &'a self,
        name: &str,
        meta: &ConnectionMeta,
        host_lower: &str,
        geo_data: &'a RuleGeoData,
        rule_sets: &'a RuleSetData,
        visited: &mut HashSet<String>,
    ) -> Option<&'a str> {
        if !visited.insert(name.to_owned()) {
            return None;
        }
        let result = self
            .sets
            .get(name)
            .and_then(|matcher| matcher.matches(meta, host_lower, geo_data, rule_sets, self, visited));
        visited.remove(name);
        result
    }

    fn explain_match_named(
        &self,
        name: &str,
        meta: &ConnectionMeta,
        host_lower: &str,
        geo_data: &RuleGeoData,
        rule_sets: &RuleSetData,
        visited: &mut HashSet<String>,
    ) -> Option<SubRuleMatchTrace> {
        if !visited.insert(name.to_owned()) {
            return None;
        }
        let result = self
            .sets
            .get(name)
            .and_then(|matcher| matcher.explain_match(meta, host_lower, geo_data, rule_sets, self, visited));
        visited.remove(name);
        result
    }

    fn validate_references(&self) -> Result<()> {
        for name in self.sets.keys() {
            self.validate_sub_rule_references(name, &mut Vec::new())?;
        }
        Ok(())
    }

    fn validate_sub_rule_references(&self, name: &str, stack: &mut Vec<String>) -> Result<()> {
        if stack.iter().any(|existing| existing == name) {
            stack.push(name.to_owned());
            bail!("sub-rule circular reference: {}", stack.join("->"));
        }
        let Some(matcher) = self.sets.get(name) else {
            bail!("sub-rule {name} not found");
        };
        stack.push(name.to_owned());
        for reference in matcher.sub_rule_references() {
            if !self.sets.contains_key(reference) {
                bail!("sub-rule {reference} not found");
            }
            self.validate_sub_rule_references(reference, stack)?;
        }
        stack.pop();
        Ok(())
    }
}

struct SubRuleMatcher {
    rules: Vec<(ParsedRule, String)>,
}

struct SubRuleMatchTrace {
    rule_raw: String,
    rule_type: String,
    target: String,
}

impl SubRuleMatcher {
    fn from_rules(raw_rules: &[String]) -> Result<Self> {
        let mut rules = Vec::with_capacity(raw_rules.len());
        for raw in raw_rules {
            rules.push((parse_rule(raw)?, raw.to_owned()));
        }
        Ok(Self { rules })
    }

    fn matches<'a>(
        &'a self,
        meta: &ConnectionMeta,
        host_lower: &str,
        geo_data: &'a RuleGeoData,
        rule_sets: &'a RuleSetData,
        sub_rules: &'a SubRuleData,
        visited: &mut HashSet<String>,
    ) -> Option<&'a str> {
        self.rules
            .iter()
            .find_map(|(rule, _)| rule_matches_inner(rule, meta, host_lower, geo_data, rule_sets, sub_rules, visited))
    }

    fn explain_match(
        &self,
        meta: &ConnectionMeta,
        host_lower: &str,
        geo_data: &RuleGeoData,
        rule_sets: &RuleSetData,
        sub_rules: &SubRuleData,
        visited: &mut HashSet<String>,
    ) -> Option<SubRuleMatchTrace> {
        self.rules.iter().find_map(|(rule, raw)| {
            rule_matches_inner(rule, meta, host_lower, geo_data, rule_sets, sub_rules, visited).map(|target| {
                SubRuleMatchTrace {
                    rule_raw: raw.clone(),
                    rule_type: rule_type_name(rule).to_owned(),
                    target: target.to_owned(),
                }
            })
        })
    }

    fn sub_rule_references(&self) -> Vec<&str> {
        let mut references = Vec::new();
        for (rule, _) in &self.rules {
            collect_sub_rule_references(rule, &mut references);
        }
        references
    }
}

fn collect_sub_rule_references<'a>(rule: &'a ParsedRule, references: &mut Vec<&'a str>) {
    match rule {
        ParsedRule::SubRule { condition, name } => {
            collect_sub_rule_references(condition, references);
            references.push(name);
        }
        ParsedRule::Logical { rules, .. } => {
            for rule in rules {
                collect_sub_rule_references(rule, references);
            }
        }
        _ => {}
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

    fn explain(&self, meta: &ConnectionMeta) -> RuleMatchResult {
        self.engine.match_connection(meta)
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

    let parts = parse_rule_payload(&rule, true)?;
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
    let parts = parse_rule_payload(item, true).unwrap_or_else(|_| RuleParts {
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

    fn meta_uid(uid: u32) -> ConnectionMeta {
        ConnectionMeta {
            uid,
            ..Default::default()
        }
    }

    fn meta_dscp(dscp: u8) -> ConnectionMeta {
        ConnectionMeta {
            dscp,
            ..Default::default()
        }
    }

    fn meta_in_type(in_type: &str) -> ConnectionMeta {
        ConnectionMeta {
            in_type: in_type.to_owned(),
            ..Default::default()
        }
    }

    fn meta_in_user(in_user: &str) -> ConnectionMeta {
        ConnectionMeta {
            in_user: in_user.to_owned(),
            ..Default::default()
        }
    }

    fn meta_in_name(in_name: &str) -> ConnectionMeta {
        ConnectionMeta {
            in_name: in_name.to_owned(),
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
    fn logical_and_matches_all_child_rules() {
        let engine = RuleEngine::from_rules(&[
            "AND,((DOMAIN,example.com),(NETWORK,TCP),(DST-PORT,443)),Proxy",
            "MATCH,DIRECT",
        ])
        .unwrap();
        let mut meta = meta_domain("example.com");
        meta.network = NetworkType::Tcp;
        meta.dst_port = 443;

        assert_eq!(engine.match_connection(&meta).target.as_deref(), Some("Proxy"));

        meta.dst_port = 80;
        assert_eq!(engine.match_connection(&meta).target.as_deref(), Some("DIRECT"));
    }

    #[test]
    fn logical_or_and_not_match_mihomo_payload_shape() {
        let engine =
            RuleEngine::from_rules(&["OR,((DOMAIN,example.com),(NOT,((NETWORK,UDP)))),Proxy", "MATCH,DIRECT"]).unwrap();
        let mut meta = meta_domain("other.example");
        meta.network = NetworkType::Tcp;

        assert_eq!(engine.match_connection(&meta).target.as_deref(), Some("Proxy"));

        meta.network = NetworkType::Udp;
        assert_eq!(engine.match_connection(&meta).target.as_deref(), Some("DIRECT"));
    }

    #[test]
    fn sub_rule_routes_to_named_rule_list() {
        let sub_rules = SubRuleData::from_sub_rules(HashMap::from([(
            "sub-rule-name1".to_string(),
            vec!["DOMAIN,example.com,Proxy".to_string(), "MATCH,DIRECT".to_string()],
        )]))
        .unwrap();
        let engine = RuleEngine::from_rules_with_default_geo_data_rule_sets_and_sub_rules(
            &[
                "SUB-RULE,(OR,((NETWORK,TCP),(NETWORK,UDP))),sub-rule-name1",
                "MATCH,FALLBACK",
            ],
            RuleSetData::empty(),
            sub_rules,
        )
        .unwrap();
        let mut meta = meta_domain("example.com");
        meta.network = NetworkType::Tcp;

        assert_eq!(engine.match_connection(&meta).target.as_deref(), Some("Proxy"));
    }

    #[test]
    fn sub_rule_rejects_missing_or_circular_references() {
        assert!(
            SubRuleData::from_sub_rules(HashMap::from([(
                "first".to_string(),
                vec!["SUB-RULE,(DOMAIN,example.com),missing".to_string()],
            )]))
            .is_err()
        );
        assert!(
            SubRuleData::from_sub_rules(HashMap::from([
                (
                    "first".to_string(),
                    vec!["SUB-RULE,(DOMAIN,example.com),second".to_string()],
                ),
                (
                    "second".to_string(),
                    vec!["SUB-RULE,(DOMAIN,example.org),first".to_string()],
                ),
            ]))
            .is_err()
        );
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
    fn process_name_regex_matches_case_insensitively() {
        let engine =
            RuleEngine::from_rules(&["PROCESS-NAME-REGEX,^telegram(-desktop)?\\.exe$,Proxy", "MATCH,DIRECT"]).unwrap();
        let result = engine.match_connection(&meta_process_name("Telegram-Desktop.EXE"));

        assert_eq!(result.target.as_deref(), Some("Proxy"));
        assert_eq!(result.rule_type.as_deref(), Some("PROCESS-NAME-REGEX"));
    }

    #[test]
    fn process_name_regex_without_metadata_falls_through() {
        let engine = RuleEngine::from_rules(&["PROCESS-NAME-REGEX,^telegram\\.exe$,Proxy", "MATCH,DIRECT"]).unwrap();

        assert_eq!(
            engine.match_connection(&ConnectionMeta::default()).target.as_deref(),
            Some("DIRECT")
        );
    }

    #[test]
    fn process_path_regex_matches_case_insensitively() {
        let engine = RuleEngine::from_rules(&[
            "PROCESS-PATH-REGEX,^c:\\\\program files\\\\telegram\\\\telegram\\.exe$,Proxy",
            "MATCH,DIRECT",
        ])
        .unwrap();
        let result = engine.match_connection(&meta_process_path("C:\\Program Files\\Telegram\\Telegram.EXE"));

        assert_eq!(result.target.as_deref(), Some("Proxy"));
        assert_eq!(result.rule_type.as_deref(), Some("PROCESS-PATH-REGEX"));
    }

    #[test]
    fn process_path_regex_without_metadata_falls_through() {
        let engine = RuleEngine::from_rules(&["PROCESS-PATH-REGEX,telegram\\.exe$,Proxy", "MATCH,DIRECT"]).unwrap();

        assert_eq!(
            engine.match_connection(&ConnectionMeta::default()).target.as_deref(),
            Some("DIRECT")
        );
    }

    #[test]
    fn process_name_wildcard_matches_case_insensitively() {
        let engine = RuleEngine::from_rules(&["PROCESS-NAME-WILDCARD,*telegram?.exe,Proxy", "MATCH,DIRECT"]).unwrap();

        assert_eq!(
            engine
                .match_connection(&meta_process_name("ORG.Telegram1.EXE"))
                .target
                .as_deref(),
            Some("Proxy")
        );
        assert_eq!(
            engine
                .match_connection(&meta_process_name("firefox.exe"))
                .target
                .as_deref(),
            Some("DIRECT")
        );
    }

    #[test]
    fn process_path_wildcard_matches_case_insensitively() {
        let engine =
            RuleEngine::from_rules(&["PROCESS-PATH-WILDCARD,*\\telegram\\telegram.exe,Proxy", "MATCH,DIRECT"]).unwrap();

        assert_eq!(
            engine
                .match_connection(&meta_process_path("C:\\Users\\Alice\\Telegram\\Telegram.EXE"))
                .target
                .as_deref(),
            Some("Proxy")
        );
        assert_eq!(
            engine
                .match_connection(&meta_process_path("C:\\Program Files\\Firefox\\firefox.exe"))
                .target
                .as_deref(),
            Some("DIRECT")
        );
    }

    #[test]
    fn process_wildcard_without_metadata_falls_through() {
        let engine = RuleEngine::from_rules(&["PROCESS-NAME-WILDCARD,*telegram*,Proxy", "MATCH,DIRECT"]).unwrap();

        assert_eq!(
            engine.match_connection(&ConnectionMeta::default()).target.as_deref(),
            Some("DIRECT")
        );
    }

    #[test]
    fn uid_matches_single_value_and_ranges() {
        let engine = RuleEngine::from_rules(&["UID,1000/2000-2002,Proxy", "MATCH,DIRECT"]).unwrap();

        assert_eq!(
            engine.match_connection(&meta_uid(1000)).target.as_deref(),
            Some("Proxy")
        );
        assert_eq!(
            engine.match_connection(&meta_uid(2001)).target.as_deref(),
            Some("Proxy")
        );
        assert_eq!(
            engine.match_connection(&meta_uid(3000)).target.as_deref(),
            Some("DIRECT")
        );
    }

    #[test]
    fn uid_without_metadata_falls_through() {
        let engine = RuleEngine::from_rules(&["UID,1000,Proxy", "MATCH,DIRECT"]).unwrap();

        assert_eq!(
            engine.match_connection(&ConnectionMeta::default()).target.as_deref(),
            Some("DIRECT")
        );
    }

    #[test]
    fn dscp_matches_single_value_and_ranges() {
        let engine = RuleEngine::from_rules(&["DSCP,10/46-48,Proxy", "MATCH,DIRECT"]).unwrap();

        assert_eq!(engine.match_connection(&meta_dscp(10)).target.as_deref(), Some("Proxy"));
        assert_eq!(engine.match_connection(&meta_dscp(47)).target.as_deref(), Some("Proxy"));
        assert_eq!(
            engine.match_connection(&meta_dscp(20)).target.as_deref(),
            Some("DIRECT")
        );
    }

    #[test]
    fn dscp_wildcard_matches_default_metadata() {
        let engine = RuleEngine::from_rules(&["DSCP,*,Proxy", "MATCH,DIRECT"]).unwrap();

        assert_eq!(
            engine.match_connection(&ConnectionMeta::default()).target.as_deref(),
            Some("Proxy")
        );
    }

    #[test]
    fn in_type_matches_case_insensitively() {
        let engine = RuleEngine::from_rules(&["IN-TYPE,HTTP/TUN,Proxy", "MATCH,DIRECT"]).unwrap();

        assert_eq!(
            engine.match_connection(&meta_in_type("http")).target.as_deref(),
            Some("Proxy")
        );
        assert_eq!(
            engine.match_connection(&meta_in_type("Tun")).target.as_deref(),
            Some("Proxy")
        );
        assert_eq!(
            engine.match_connection(&meta_in_type("HTTPS")).target.as_deref(),
            Some("DIRECT")
        );
    }

    #[test]
    fn in_type_socks_expands_to_socks4_and_socks5() {
        let engine = RuleEngine::from_rules(&["IN-TYPE,SOCKS,Proxy", "MATCH,DIRECT"]).unwrap();

        assert_eq!(
            engine.match_connection(&meta_in_type("socks4")).target.as_deref(),
            Some("Proxy")
        );
        assert_eq!(
            engine.match_connection(&meta_in_type("Socks5")).target.as_deref(),
            Some("Proxy")
        );
    }

    #[test]
    fn in_type_without_metadata_falls_through() {
        let engine = RuleEngine::from_rules(&["IN-TYPE,HTTP,Proxy", "MATCH,DIRECT"]).unwrap();

        assert_eq!(
            engine.match_connection(&ConnectionMeta::default()).target.as_deref(),
            Some("DIRECT")
        );
    }

    #[test]
    fn in_user_matches_exactly() {
        let engine = RuleEngine::from_rules(&["IN-USER,alice/bob,Proxy", "MATCH,DIRECT"]).unwrap();

        assert_eq!(
            engine.match_connection(&meta_in_user("alice")).target.as_deref(),
            Some("Proxy")
        );
        assert_eq!(
            engine.match_connection(&meta_in_user("bob")).target.as_deref(),
            Some("Proxy")
        );
        assert_eq!(
            engine.match_connection(&meta_in_user("Alice")).target.as_deref(),
            Some("DIRECT")
        );
    }

    #[test]
    fn in_user_without_metadata_falls_through() {
        let engine = RuleEngine::from_rules(&["IN-USER,alice,Proxy", "MATCH,DIRECT"]).unwrap();

        assert_eq!(
            engine.match_connection(&ConnectionMeta::default()).target.as_deref(),
            Some("DIRECT")
        );
    }

    #[test]
    fn in_name_matches_exactly() {
        let engine = RuleEngine::from_rules(&["IN-NAME,home/work,Proxy", "MATCH,DIRECT"]).unwrap();

        assert_eq!(
            engine.match_connection(&meta_in_name("home")).target.as_deref(),
            Some("Proxy")
        );
        assert_eq!(
            engine.match_connection(&meta_in_name("work")).target.as_deref(),
            Some("Proxy")
        );
        assert_eq!(
            engine.match_connection(&meta_in_name("Home")).target.as_deref(),
            Some("DIRECT")
        );
    }

    #[test]
    fn in_name_without_metadata_falls_through() {
        let engine = RuleEngine::from_rules(&["IN-NAME,home,Proxy", "MATCH,DIRECT"]).unwrap();

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
        assert!(validate_rule("PROCESS-NAME-REGEX,^telegram(-desktop)?\\.exe$,Proxy").valid);
        assert!(validate_rule("PROCESS-PATH-REGEX,telegram\\.exe$,Proxy").valid);
        assert!(validate_rule("PROCESS-NAME-WILDCARD,*telegram*,Proxy").valid);
        assert!(validate_rule("PROCESS-PATH-WILDCARD,*\\telegram\\telegram.exe,Proxy").valid);
        assert!(validate_rule("UID,1000/2000-2002,Proxy").valid);
        assert!(validate_rule("DSCP,10/46-48,Proxy").valid);
        assert!(validate_rule("DSCP,*,Proxy").valid);
        assert!(validate_rule("IN-TYPE,HTTP/SOCKS/TUN,Proxy").valid);
        assert!(validate_rule("IN-USER,alice/bob,Proxy").valid);
        assert!(validate_rule("IN-NAME,home/work,Proxy").valid);
        assert!(validate_rule("MATCH,DIRECT").valid);

        assert!(!validate_rule("DOMAIN").valid);
        assert!(!validate_rule("PROCESS-NAME-REGEX,[,Proxy").valid);
        assert!(!validate_rule("PROCESS-PATH-REGEX,[,Proxy").valid);
        assert!(!validate_rule("UID,*,Proxy").valid);
        assert!(!validate_rule("UID,not-a-uid,Proxy").valid);
        assert!(!validate_rule("DSCP,64,Proxy").valid);
        assert!(!validate_rule("DSCP,not-a-dscp,Proxy").valid);
        assert!(!validate_rule("IN-TYPE,,Proxy").valid);
        assert!(!validate_rule("IN-TYPE,UNKNOWN,Proxy").valid);
        assert!(!validate_rule("IN-USER,,Proxy").valid);
        assert!(!validate_rule("IN-NAME,,Proxy").valid);
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
    fn explain_connection_records_match_trace() {
        let engine = RuleEngine::from_rules(&["DOMAIN,example.net,Proxy", "DOMAIN,example.com,DIRECT"]).unwrap();
        let result = engine.explain_connection(&meta_domain("example.com"));

        assert!(result.matched);
        assert_eq!(result.outcome, "matched");
        assert_eq!(result.rule_index, Some(1));
        assert_eq!(result.rule_type.as_deref(), Some("DOMAIN"));
        assert_eq!(result.target.as_deref(), Some("DIRECT"));
        assert_eq!(result.trace.len(), 2);
        assert!(!result.trace[0].matched);
        assert_eq!(result.trace[1].rule_raw, "DOMAIN,example.com,DIRECT");
        assert_eq!(result.trace[1].target.as_deref(), Some("DIRECT"));
    }

    #[test]
    fn explain_connection_records_fallthrough_trace() {
        let engine = RuleEngine::from_rules(&["DOMAIN,example.net,Proxy"]).unwrap();
        let result = engine.explain_connection(&meta_domain("example.com"));

        assert!(!result.matched);
        assert_eq!(result.outcome, "fallthrough");
        assert_eq!(result.explanation, "no rules matched; fallthrough without target");
        assert_eq!(result.trace.len(), 1);
        assert_eq!(result.trace[0].rule_type, "DOMAIN");
        assert!(!result.trace[0].matched);
    }

    #[test]
    fn explain_connection_shows_rule_set_inner_match() {
        let provider = RuleProviderConfig {
            provider_type: "inline".to_string(),
            behavior: RuleProviderBehavior::Classical,
            path: None,
            payload: vec!["DOMAIN-SUFFIX,example.com,REJECT".to_string()],
            format: None,
        };
        let rule_sets = RuleSetData::from_rule_providers(HashMap::from([("private".to_string(), provider)])).unwrap();
        let engine =
            RuleEngine::from_rules_with_rule_sets(&["RULE-SET,private,DIRECT", "MATCH,Proxy"], rule_sets).unwrap();
        let result = engine.explain_connection(&meta_domain("www.example.com"));
        let detail = result.trace[0].detail.as_ref().unwrap();

        assert_eq!(result.target.as_deref(), Some("DIRECT"));
        assert_eq!(detail.reference_type, "rule_set");
        assert_eq!(detail.name, "private");
        assert_eq!(
            detail.matched_rule_raw.as_deref(),
            Some("DOMAIN-SUFFIX,example.com,__RULE_SET_MATCH__")
        );
        assert_eq!(detail.matched_rule_type.as_deref(), Some("DOMAIN-SUFFIX"));
    }

    #[test]
    fn explain_connection_shows_sub_rule_inner_match() {
        let sub_rules = SubRuleData::from_sub_rules(HashMap::from([(
            "domain-preview".to_string(),
            vec!["DOMAIN-SUFFIX,example.com,Proxy".to_string()],
        )]))
        .unwrap();
        let engine = RuleEngine::from_rules_with_geo_data_rule_sets_and_sub_rules(
            &["SUB-RULE,DOMAIN-SUFFIX,example.com,domain-preview", "MATCH,DIRECT"],
            RuleGeoData::empty(),
            RuleSetData::empty(),
            sub_rules,
        )
        .unwrap();
        let result = engine.explain_connection(&meta_domain("www.example.com"));
        let detail = result.trace[0].detail.as_ref().unwrap();

        assert_eq!(result.target.as_deref(), Some("Proxy"));
        assert_eq!(detail.reference_type, "sub_rule");
        assert_eq!(detail.name, "domain-preview");
        assert_eq!(detail.condition_matched, Some(true));
        assert_eq!(
            detail.matched_rule_raw.as_deref(),
            Some("DOMAIN-SUFFIX,example.com,Proxy")
        );
        assert_eq!(detail.matched_rule_type.as_deref(), Some("DOMAIN-SUFFIX"));
        assert_eq!(detail.matched_target.as_deref(), Some("Proxy"));
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
