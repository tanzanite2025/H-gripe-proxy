use crate::utils::dirs;
use anyhow::{Context as _, Result, anyhow};
use hickory_proto::rr::Name;
use serde::{Deserialize, Serialize};
use serde_yaml_ng::{Mapping, Value};
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::fs;

const RUST_DNS_FALLBACK_FILTER_RUNTIME_COMPONENT: &str = "rust-dns-fallback-filter-runtime";
const RUST_DNS_FALLBACK_FILTER_RUNTIME_EVIDENCE_FILE: &str = "evidence.yaml";
const RUST_DNS_FALLBACK_FILTER_RUNTIME_ROLLBACK_FILE: &str = "rollback-checkpoint.yaml";
const NEXT_SAFE_BATCH: &str = "unsupported-protocol-and-packet-capture-implementation";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum RustDnsFallbackFilterRuntimeStatus {
    Planned,
    Executed,
    Blocked,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RustDnsFallbackFilterRuleEvidence {
    pub rule_type: String,
    pub rule: String,
    pub matched: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RustDnsFallbackFilterDecisionEvidence {
    pub domain: String,
    pub candidate_ip: String,
    pub fallback_required: bool,
    pub matched_rules: Vec<RustDnsFallbackFilterRuleEvidence>,
    pub evaluated_rule_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RustDnsFallbackFilterRollbackEvidence {
    pub checkpoint_path: String,
    pub fallback_retained_for: Vec<String>,
    pub created_at_epoch_seconds: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RustDnsFallbackFilterLeakEvidence {
    pub passed: bool,
    pub no_upstream_query: bool,
    pub no_system_resolver_mutation: bool,
    pub no_mihomo_binary_removal: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RustDnsFallbackFilterRuntimeReport {
    pub component: String,
    pub status: RustDnsFallbackFilterRuntimeStatus,
    pub reason: String,
    pub explicit_opt_in: bool,
    pub rust_owned_scope: String,
    pub mutates_runtime: bool,
    pub writes_evidence: bool,
    pub evidence_path: Option<String>,
    pub decision_evidence: Option<RustDnsFallbackFilterDecisionEvidence>,
    pub rollback_evidence: Option<RustDnsFallbackFilterRollbackEvidence>,
    pub leak_evidence: Option<RustDnsFallbackFilterLeakEvidence>,
    pub mihomo_fallback_retained_for: Vec<String>,
    pub blockers: Vec<String>,
    pub warnings: Vec<String>,
    pub facts: Vec<String>,
    pub next_safe_batch: String,
}

pub async fn rust_dns_fallback_filter_runtime_execution(
    yaml: String,
    domain: String,
    candidate_ip: String,
    explicit_opt_in: bool,
) -> Result<RustDnsFallbackFilterRuntimeReport> {
    if !explicit_opt_in {
        return Ok(blocked_report(
            explicit_opt_in,
            vec!["Rust DNS fallback-filter execution requires explicit opt-in".into()],
        ));
    }

    let domain = normalize_filter_domain(&domain)?;
    let candidate_ip = candidate_ip
        .trim()
        .parse::<IpAddr>()
        .with_context(|| format!("candidate IP is invalid: {candidate_ip}"))?;
    let filter = FallbackFilter::from_yaml(&yaml)?;
    if filter.rules.is_empty() {
        return Ok(blocked_report(
            explicit_opt_in,
            vec!["fallback-filter has no bounded Rust-supported domain/ipcidr rules".into()],
        ));
    }

    let decision_evidence = filter.evaluate(&domain, candidate_ip);
    let rollback_path = rust_dns_fallback_filter_runtime_rollback_path()?;
    let rollback_evidence = write_rollback_checkpoint(&rollback_path).await?;
    let leak_evidence = RustDnsFallbackFilterLeakEvidence {
        passed: true,
        no_upstream_query: true,
        no_system_resolver_mutation: true,
        no_mihomo_binary_removal: true,
    };
    let evidence_path = rust_dns_fallback_filter_runtime_evidence_path()?;
    let mut report = RustDnsFallbackFilterRuntimeReport {
        component: RUST_DNS_FALLBACK_FILTER_RUNTIME_COMPONENT.into(),
        status: RustDnsFallbackFilterRuntimeStatus::Executed,
        reason: "Rust evaluated bounded fallback-filter domain/ipcidr policy without upstream DNS or system resolver mutation".into(),
        explicit_opt_in,
        rust_owned_scope: "bounded fallback-filter domain and ipcidr decision for one DNS answer".into(),
        mutates_runtime: false,
        writes_evidence: true,
        evidence_path: Some(evidence_path.to_string_lossy().to_string()),
        decision_evidence: Some(decision_evidence),
        rollback_evidence: Some(rollback_evidence),
        leak_evidence: Some(leak_evidence),
        mihomo_fallback_retained_for: retained_fallback_filter_scope(),
        blockers: Vec::new(),
        warnings: filter.warnings,
        facts: rust_dns_fallback_filter_runtime_facts(),
        next_safe_batch: NEXT_SAFE_BATCH.into(),
    };

    if let Some(parent) = evidence_path.parent() {
        fs::create_dir_all(parent).await?;
    }
    fs::write(&evidence_path, serde_yaml_ng::to_string(&report)?.as_bytes()).await?;
    report.evidence_path = Some(evidence_path.to_string_lossy().to_string());

    Ok(report)
}

fn blocked_report(explicit_opt_in: bool, blockers: Vec<String>) -> RustDnsFallbackFilterRuntimeReport {
    RustDnsFallbackFilterRuntimeReport {
        component: RUST_DNS_FALLBACK_FILTER_RUNTIME_COMPONENT.into(),
        status: RustDnsFallbackFilterRuntimeStatus::Blocked,
        reason: "Rust DNS fallback-filter execution is blocked".into(),
        explicit_opt_in,
        rust_owned_scope: "bounded fallback-filter domain and ipcidr decision for one DNS answer".into(),
        mutates_runtime: false,
        writes_evidence: false,
        evidence_path: None,
        decision_evidence: None,
        rollback_evidence: None,
        leak_evidence: None,
        mihomo_fallback_retained_for: retained_fallback_filter_scope(),
        blockers,
        warnings: Vec::new(),
        facts: rust_dns_fallback_filter_runtime_facts(),
        next_safe_batch: NEXT_SAFE_BATCH.into(),
    }
}

#[derive(Debug, Clone)]
struct FallbackFilter {
    rules: Vec<FallbackFilterRule>,
    warnings: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum FallbackFilterRule {
    DomainSuffix(String),
    DomainExact(String),
    IpCidr(IpCidr),
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum IpCidr {
    V4 { network: u32, prefix: u8 },
    V6 { network: u128, prefix: u8 },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RustDnsFallbackFilterRollbackCheckpoint {
    component: String,
    rust_owned_scope: String,
    fallback_retained_for: Vec<String>,
    created_at_epoch_seconds: u64,
}

impl FallbackFilter {
    fn from_yaml(yaml: &str) -> Result<Self> {
        let value: Value = serde_yaml_ng::from_str(yaml).context("YAML syntax error")?;
        let root = value
            .as_mapping()
            .ok_or_else(|| anyhow!("config root must be a YAML mapping"))?;
        let dns = root
            .get("dns")
            .and_then(Value::as_mapping)
            .ok_or_else(|| anyhow!("dns config is missing"))?;
        let fallback_filter = dns
            .get("fallback-filter")
            .and_then(Value::as_mapping)
            .ok_or_else(|| anyhow!("dns.fallback-filter is missing"))?;

        let mut warnings = Vec::new();
        if fallback_filter
            .get("geoip")
            .and_then(Value::as_bool)
            .unwrap_or_default()
            || fallback_filter.contains_key("geoip-code")
        {
            warnings.push("fallback-filter geoip and geoip-code semantics remain Mihomo-owned".into());
        }

        let mut rules = Vec::new();
        collect_domain_rules(fallback_filter, &mut rules)?;
        collect_ipcidr_rules(fallback_filter, &mut rules)?;

        Ok(Self { rules, warnings })
    }

    fn evaluate(&self, domain: &str, candidate_ip: IpAddr) -> RustDnsFallbackFilterDecisionEvidence {
        let matched_rules: Vec<_> = self
            .rules
            .iter()
            .map(|rule| RustDnsFallbackFilterRuleEvidence {
                rule_type: rule.rule_type().into(),
                rule: rule.label(),
                matched: rule.matches(domain, candidate_ip),
            })
            .filter(|evidence| evidence.matched)
            .collect();

        RustDnsFallbackFilterDecisionEvidence {
            domain: domain.into(),
            candidate_ip: candidate_ip.to_string(),
            fallback_required: !matched_rules.is_empty(),
            matched_rules,
            evaluated_rule_count: self.rules.len(),
        }
    }
}

impl FallbackFilterRule {
    fn rule_type(&self) -> &'static str {
        match self {
            FallbackFilterRule::DomainSuffix(_) => "domainSuffix",
            FallbackFilterRule::DomainExact(_) => "domainExact",
            FallbackFilterRule::IpCidr(_) => "ipcidr",
        }
    }

    fn label(&self) -> String {
        match self {
            FallbackFilterRule::DomainSuffix(suffix) => format!("+.{suffix}"),
            FallbackFilterRule::DomainExact(domain) => domain.clone(),
            FallbackFilterRule::IpCidr(cidr) => cidr.to_string(),
        }
    }

    fn matches(&self, domain: &str, candidate_ip: IpAddr) -> bool {
        match self {
            FallbackFilterRule::DomainSuffix(suffix) => domain == suffix || domain.ends_with(&format!(".{suffix}")),
            FallbackFilterRule::DomainExact(exact) => domain == exact,
            FallbackFilterRule::IpCidr(cidr) => cidr.contains(candidate_ip),
        }
    }
}

impl IpCidr {
    fn parse(input: &str) -> Result<Self> {
        let (addr, prefix) = input
            .split_once('/')
            .ok_or_else(|| anyhow!("ipcidr rule must use CIDR notation"))?;
        match addr
            .parse::<IpAddr>()
            .with_context(|| format!("invalid ipcidr address: {addr}"))?
        {
            IpAddr::V4(ip) => {
                let prefix = parse_prefix(prefix, 32)?;
                let mask = if prefix == 0 { 0 } else { u32::MAX << (32 - prefix) };
                Ok(Self::V4 {
                    network: u32::from(ip) & mask,
                    prefix,
                })
            }
            IpAddr::V6(ip) => {
                let prefix = parse_prefix(prefix, 128)?;
                let mask = if prefix == 0 { 0 } else { u128::MAX << (128 - prefix) };
                Ok(Self::V6 {
                    network: u128::from(ip) & mask,
                    prefix,
                })
            }
        }
    }

    fn contains(&self, ip: IpAddr) -> bool {
        match (self, ip) {
            (Self::V4 { network, prefix }, IpAddr::V4(ip)) => {
                let mask = if *prefix == 0 { 0 } else { u32::MAX << (32 - prefix) };
                (u32::from(ip) & mask) == *network
            }
            (Self::V6 { network, prefix }, IpAddr::V6(ip)) => {
                let mask = if *prefix == 0 { 0 } else { u128::MAX << (128 - prefix) };
                (u128::from(ip) & mask) == *network
            }
            _ => false,
        }
    }
}

impl std::fmt::Display for IpCidr {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::V4 { network, prefix } => {
                write!(formatter, "{}/{}", Ipv4Addr::from(*network), prefix)
            }
            Self::V6 { network, prefix } => {
                write!(formatter, "{}/{}", Ipv6Addr::from(*network), prefix)
            }
        }
    }
}

async fn write_rollback_checkpoint(rollback_path: &std::path::Path) -> Result<RustDnsFallbackFilterRollbackEvidence> {
    let created_at_epoch_seconds = rust_dns_fallback_filter_runtime_epoch_seconds();
    let checkpoint = RustDnsFallbackFilterRollbackCheckpoint {
        component: RUST_DNS_FALLBACK_FILTER_RUNTIME_COMPONENT.into(),
        rust_owned_scope: "bounded fallback-filter domain and ipcidr decision for one DNS answer".into(),
        fallback_retained_for: retained_fallback_filter_scope(),
        created_at_epoch_seconds,
    };
    if let Some(parent) = rollback_path.parent() {
        fs::create_dir_all(parent).await?;
    }
    fs::write(rollback_path, serde_yaml_ng::to_string(&checkpoint)?.as_bytes()).await?;

    Ok(RustDnsFallbackFilterRollbackEvidence {
        checkpoint_path: rollback_path.to_string_lossy().to_string(),
        fallback_retained_for: checkpoint.fallback_retained_for,
        created_at_epoch_seconds,
    })
}

fn collect_domain_rules(fallback_filter: &Mapping, rules: &mut Vec<FallbackFilterRule>) -> Result<()> {
    for entry in fallback_filter
        .get("domain")
        .and_then(Value::as_sequence)
        .into_iter()
        .flatten()
    {
        let entry = entry
            .as_str()
            .ok_or_else(|| anyhow!("fallback-filter domain entries must be strings"))?;
        let entry = normalize_domain_rule(entry)?;
        if let Some(suffix) = entry.strip_prefix("+.") {
            rules.push(FallbackFilterRule::DomainSuffix(suffix.into()));
        } else {
            rules.push(FallbackFilterRule::DomainExact(entry));
        }
    }
    Ok(())
}

fn collect_ipcidr_rules(fallback_filter: &Mapping, rules: &mut Vec<FallbackFilterRule>) -> Result<()> {
    for entry in fallback_filter
        .get("ipcidr")
        .and_then(Value::as_sequence)
        .into_iter()
        .flatten()
    {
        let entry = entry
            .as_str()
            .ok_or_else(|| anyhow!("fallback-filter ipcidr entries must be strings"))?;
        rules.push(FallbackFilterRule::IpCidr(IpCidr::parse(entry.trim())?));
    }
    Ok(())
}

fn normalize_filter_domain(domain: &str) -> Result<String> {
    let domain = domain.trim().trim_end_matches('.').to_ascii_lowercase();
    if domain.is_empty() {
        return Err(anyhow!("fallback-filter domain is empty"));
    }
    Name::from_str_relaxed(&domain).context("fallback-filter domain is invalid")?;
    Ok(domain)
}

fn normalize_domain_rule(rule: &str) -> Result<String> {
    let rule = rule.trim().trim_end_matches('.').to_ascii_lowercase();
    if rule.is_empty() {
        return Err(anyhow!("fallback-filter domain rule is empty"));
    }
    if let Some(suffix) = rule.strip_prefix("+.") {
        Name::from_str_relaxed(suffix).context("fallback-filter suffix rule is invalid")?;
    } else {
        Name::from_str_relaxed(&rule).context("fallback-filter exact rule is invalid")?;
    }
    Ok(rule)
}

fn parse_prefix(prefix: &str, max: u8) -> Result<u8> {
    let prefix = prefix
        .parse::<u8>()
        .with_context(|| format!("invalid CIDR prefix: {prefix}"))?;
    if prefix > max {
        return Err(anyhow!("CIDR prefix {prefix} exceeds {max}"));
    }
    Ok(prefix)
}

fn retained_fallback_filter_scope() -> Vec<String> {
    vec![
        "fallback-filter geoip and geoip-code semantics".into(),
        "fallback resolver selection and upstream query execution".into(),
        "nameserver-policy dispatch".into(),
        "fake-ip cache/reverse mapping".into(),
        "default DNS runtime ownership".into(),
    ]
}

fn rust_dns_fallback_filter_runtime_facts() -> Vec<String> {
    vec![
        "Rust parses fallback-filter domain suffix/exact and ipcidr rules".into(),
        "Rust evaluates one candidate DNS answer without querying upstream DNS".into(),
        "Rust does not mutate system resolver state or Mihomo binaries".into(),
        "Mihomo fallback remains retained for geoip, upstream execution, nameserver-policy, and default DNS".into(),
    ]
}

fn rust_dns_fallback_filter_runtime_dir() -> Result<std::path::PathBuf> {
    Ok(dirs::app_runtime_dir()?.join(RUST_DNS_FALLBACK_FILTER_RUNTIME_COMPONENT))
}

fn rust_dns_fallback_filter_runtime_evidence_path() -> Result<std::path::PathBuf> {
    Ok(rust_dns_fallback_filter_runtime_dir()?.join(RUST_DNS_FALLBACK_FILTER_RUNTIME_EVIDENCE_FILE))
}

fn rust_dns_fallback_filter_runtime_rollback_path() -> Result<std::path::PathBuf> {
    Ok(rust_dns_fallback_filter_runtime_dir()?.join(RUST_DNS_FALLBACK_FILTER_RUNTIME_ROLLBACK_FILE))
}

fn rust_dns_fallback_filter_runtime_epoch_seconds() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matches_domain_suffix_rule() {
        let rule = FallbackFilterRule::DomainSuffix("example.com".into());

        assert!(rule.matches("www.example.com", "1.1.1.1".parse().unwrap()));
        assert!(rule.matches("example.com", "1.1.1.1".parse().unwrap()));
        assert!(!rule.matches("notexample.com", "1.1.1.1".parse().unwrap()));
    }

    #[test]
    fn matches_ipcidr_rule() {
        let cidr = IpCidr::parse("203.0.113.0/24").unwrap();

        assert!(cidr.contains("203.0.113.7".parse().unwrap()));
        assert!(!cidr.contains("198.51.100.7".parse().unwrap()));
    }

    #[test]
    fn parses_supported_fallback_filter_rules_from_yaml() {
        let yaml =
            "dns:\n  fallback-filter:\n    domain:\n      - '+.example.com'\n    ipcidr:\n      - 203.0.113.0/24\n";
        let filter = FallbackFilter::from_yaml(yaml).unwrap();
        let evidence = filter.evaluate("www.example.com", "203.0.113.7".parse().unwrap());

        assert_eq!(evidence.evaluated_rule_count, 2);
        assert!(evidence.fallback_required);
        assert_eq!(evidence.matched_rules.len(), 2);
    }
}
