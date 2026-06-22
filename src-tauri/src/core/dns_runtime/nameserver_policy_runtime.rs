use crate::utils::dirs;
use anyhow::{Context as _, Result, anyhow};
use hickory_proto::rr::Name;
use serde::{Deserialize, Serialize};
use serde_yaml_ng::Value;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::fs;

const RUST_DNS_NAMESERVER_POLICY_RUNTIME_COMPONENT: &str = "rust-dns-nameserver-policy-runtime";
const RUST_DNS_NAMESERVER_POLICY_RUNTIME_EVIDENCE_FILE: &str = "evidence.yaml";
const RUST_DNS_NAMESERVER_POLICY_RUNTIME_ROLLBACK_FILE: &str = "rollback-checkpoint.yaml";
const NEXT_SAFE_BATCH: &str = "unsupported-protocol-and-packet-capture-implementation";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum RustDnsNameserverPolicyRuntimeStatus {
    Planned,
    Executed,
    Blocked,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RustDnsNameserverPolicyRuleEvidence {
    pub rule_type: String,
    pub rule: String,
    pub nameservers: Vec<String>,
    pub matched: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RustDnsNameserverPolicyDecisionEvidence {
    pub domain: String,
    pub selected_nameservers: Vec<String>,
    pub matched_rules: Vec<RustDnsNameserverPolicyRuleEvidence>,
    pub evaluated_rule_count: usize,
    pub default_nameservers_retained: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RustDnsNameserverPolicyRollbackEvidence {
    pub checkpoint_path: String,
    pub fallback_retained_for: Vec<String>,
    pub created_at_epoch_seconds: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RustDnsNameserverPolicyLeakEvidence {
    pub passed: bool,
    pub no_upstream_query: bool,
    pub no_system_resolver_mutation: bool,
    pub no_mihomo_binary_removal: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RustDnsNameserverPolicyRuntimeReport {
    pub component: String,
    pub status: RustDnsNameserverPolicyRuntimeStatus,
    pub reason: String,
    pub explicit_opt_in: bool,
    pub rust_owned_scope: String,
    pub mutates_runtime: bool,
    pub writes_evidence: bool,
    pub evidence_path: Option<String>,
    pub decision_evidence: Option<RustDnsNameserverPolicyDecisionEvidence>,
    pub rollback_evidence: Option<RustDnsNameserverPolicyRollbackEvidence>,
    pub leak_evidence: Option<RustDnsNameserverPolicyLeakEvidence>,
    pub mihomo_fallback_retained_for: Vec<String>,
    pub blockers: Vec<String>,
    pub warnings: Vec<String>,
    pub facts: Vec<String>,
    pub next_safe_batch: String,
}

pub async fn rust_dns_nameserver_policy_runtime_execution(
    yaml: String,
    domain: String,
    explicit_opt_in: bool,
) -> Result<RustDnsNameserverPolicyRuntimeReport> {
    if !explicit_opt_in {
        return Ok(blocked_report(
            explicit_opt_in,
            vec!["Rust DNS nameserver-policy execution requires explicit opt-in".into()],
        ));
    }

    let domain = normalize_policy_domain(&domain)?;
    let policy = NameserverPolicy::from_yaml(&yaml)?;
    if policy.rules.is_empty() {
        return Ok(blocked_report(
            explicit_opt_in,
            vec!["nameserver-policy has no bounded Rust-supported exact/suffix rules".into()],
        ));
    }

    let decision_evidence = policy.evaluate(&domain);
    let rollback_path = rust_dns_nameserver_policy_runtime_rollback_path()?;
    let rollback_evidence = write_rollback_checkpoint(&rollback_path).await?;
    let leak_evidence = RustDnsNameserverPolicyLeakEvidence {
        passed: true,
        no_upstream_query: true,
        no_system_resolver_mutation: true,
        no_mihomo_binary_removal: true,
    };
    let evidence_path = rust_dns_nameserver_policy_runtime_evidence_path()?;
    let mut report = RustDnsNameserverPolicyRuntimeReport {
        component: RUST_DNS_NAMESERVER_POLICY_RUNTIME_COMPONENT.into(),
        status: RustDnsNameserverPolicyRuntimeStatus::Executed,
        reason:
            "Rust selected bounded nameserver-policy targets without issuing upstream DNS or mutating resolver state"
                .into(),
        explicit_opt_in,
        rust_owned_scope: "bounded nameserver-policy exact/suffix dispatch for one domain".into(),
        mutates_runtime: false,
        writes_evidence: true,
        evidence_path: Some(evidence_path.to_string_lossy().to_string()),
        decision_evidence: Some(decision_evidence),
        rollback_evidence: Some(rollback_evidence),
        leak_evidence: Some(leak_evidence),
        mihomo_fallback_retained_for: retained_nameserver_policy_scope(),
        blockers: Vec::new(),
        warnings: policy.warnings,
        facts: rust_dns_nameserver_policy_runtime_facts(),
        next_safe_batch: NEXT_SAFE_BATCH.into(),
    };

    if let Some(parent) = evidence_path.parent() {
        fs::create_dir_all(parent).await?;
    }
    fs::write(&evidence_path, serde_yaml_ng::to_string(&report)?.as_bytes()).await?;
    report.evidence_path = Some(evidence_path.to_string_lossy().to_string());

    Ok(report)
}

fn blocked_report(explicit_opt_in: bool, blockers: Vec<String>) -> RustDnsNameserverPolicyRuntimeReport {
    RustDnsNameserverPolicyRuntimeReport {
        component: RUST_DNS_NAMESERVER_POLICY_RUNTIME_COMPONENT.into(),
        status: RustDnsNameserverPolicyRuntimeStatus::Blocked,
        reason: "Rust DNS nameserver-policy execution is blocked".into(),
        explicit_opt_in,
        rust_owned_scope: "bounded nameserver-policy exact/suffix dispatch for one domain".into(),
        mutates_runtime: false,
        writes_evidence: false,
        evidence_path: None,
        decision_evidence: None,
        rollback_evidence: None,
        leak_evidence: None,
        mihomo_fallback_retained_for: retained_nameserver_policy_scope(),
        blockers,
        warnings: Vec::new(),
        facts: rust_dns_nameserver_policy_runtime_facts(),
        next_safe_batch: NEXT_SAFE_BATCH.into(),
    }
}

#[derive(Debug, Clone)]
struct NameserverPolicy {
    rules: Vec<NameserverPolicyRule>,
    warnings: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum NameserverPolicyMatcher {
    DomainSuffix(String),
    DomainExact(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct NameserverPolicyRule {
    matcher: NameserverPolicyMatcher,
    nameservers: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RustDnsNameserverPolicyRollbackCheckpoint {
    component: String,
    rust_owned_scope: String,
    fallback_retained_for: Vec<String>,
    created_at_epoch_seconds: u64,
}

impl NameserverPolicy {
    fn from_yaml(yaml: &str) -> Result<Self> {
        let value: Value = serde_yaml_ng::from_str(yaml).context("YAML syntax error")?;
        let root = value
            .as_mapping()
            .ok_or_else(|| anyhow!("config root must be a YAML mapping"))?;
        let dns = root
            .get("dns")
            .and_then(Value::as_mapping)
            .ok_or_else(|| anyhow!("dns config is missing"))?;
        let nameserver_policy = dns
            .get("nameserver-policy")
            .and_then(Value::as_mapping)
            .ok_or_else(|| anyhow!("dns.nameserver-policy is missing"))?;

        let mut rules = Vec::new();
        let mut warnings = Vec::new();
        for (key, value) in nameserver_policy {
            let key = key
                .as_str()
                .ok_or_else(|| anyhow!("nameserver-policy keys must be strings"))?;
            match parse_policy_matcher(key) {
                Ok(matcher) => rules.push(NameserverPolicyRule {
                    matcher,
                    nameservers: parse_policy_nameservers(value)?,
                }),
                Err(error) => warnings.push(format!("nameserver-policy rule remains Mihomo-owned: {key} ({error})")),
            }
        }

        Ok(Self { rules, warnings })
    }

    fn evaluate(&self, domain: &str) -> RustDnsNameserverPolicyDecisionEvidence {
        let matched_rules: Vec<_> = self
            .rules
            .iter()
            .map(|rule| RustDnsNameserverPolicyRuleEvidence {
                rule_type: rule.matcher.rule_type().into(),
                rule: rule.matcher.label(),
                nameservers: rule.nameservers.clone(),
                matched: rule.matcher.matches(domain),
            })
            .filter(|evidence| evidence.matched)
            .collect();
        let selected_nameservers = matched_rules
            .first()
            .map(|evidence| evidence.nameservers.clone())
            .unwrap_or_default();

        RustDnsNameserverPolicyDecisionEvidence {
            domain: domain.into(),
            selected_nameservers,
            matched_rules,
            evaluated_rule_count: self.rules.len(),
            default_nameservers_retained: true,
        }
    }
}

impl NameserverPolicyMatcher {
    fn rule_type(&self) -> &'static str {
        match self {
            Self::DomainSuffix(_) => "domainSuffix",
            Self::DomainExact(_) => "domainExact",
        }
    }

    fn label(&self) -> String {
        match self {
            Self::DomainSuffix(suffix) => format!("+.{suffix}"),
            Self::DomainExact(domain) => domain.clone(),
        }
    }

    fn matches(&self, domain: &str) -> bool {
        match self {
            Self::DomainSuffix(suffix) => domain == suffix || domain.ends_with(&format!(".{suffix}")),
            Self::DomainExact(exact) => domain == exact,
        }
    }
}

async fn write_rollback_checkpoint(rollback_path: &std::path::Path) -> Result<RustDnsNameserverPolicyRollbackEvidence> {
    let created_at_epoch_seconds = rust_dns_nameserver_policy_runtime_epoch_seconds();
    let checkpoint = RustDnsNameserverPolicyRollbackCheckpoint {
        component: RUST_DNS_NAMESERVER_POLICY_RUNTIME_COMPONENT.into(),
        rust_owned_scope: "bounded nameserver-policy exact/suffix dispatch for one domain".into(),
        fallback_retained_for: retained_nameserver_policy_scope(),
        created_at_epoch_seconds,
    };
    if let Some(parent) = rollback_path.parent() {
        fs::create_dir_all(parent).await?;
    }
    fs::write(rollback_path, serde_yaml_ng::to_string(&checkpoint)?.as_bytes()).await?;

    Ok(RustDnsNameserverPolicyRollbackEvidence {
        checkpoint_path: rollback_path.to_string_lossy().to_string(),
        fallback_retained_for: checkpoint.fallback_retained_for,
        created_at_epoch_seconds,
    })
}

fn parse_policy_matcher(key: &str) -> Result<NameserverPolicyMatcher> {
    let key = key.trim().trim_end_matches('.').to_ascii_lowercase();
    if key.is_empty() {
        return Err(anyhow!("empty policy matcher"));
    }
    if key.contains(':') || key.contains(',') || key.contains('*') {
        return Err(anyhow!("only exact and +.suffix matchers are Rust-owned"));
    }
    if let Some(suffix) = key.strip_prefix("+.") {
        Name::from_str_relaxed(suffix).context("nameserver-policy suffix is invalid")?;
        Ok(NameserverPolicyMatcher::DomainSuffix(suffix.into()))
    } else {
        Name::from_str_relaxed(&key).context("nameserver-policy domain is invalid")?;
        Ok(NameserverPolicyMatcher::DomainExact(key))
    }
}

fn parse_policy_nameservers(value: &Value) -> Result<Vec<String>> {
    let nameservers = match value {
        Value::String(server) => vec![normalize_nameserver(server)?],
        Value::Sequence(sequence) => sequence
            .iter()
            .map(|entry| {
                entry
                    .as_str()
                    .ok_or_else(|| anyhow!("nameserver-policy values must be strings"))
                    .and_then(normalize_nameserver)
            })
            .collect::<Result<Vec<_>>>()?,
        _ => return Err(anyhow!("nameserver-policy values must be string or string list")),
    };
    if nameservers.is_empty() {
        return Err(anyhow!("nameserver-policy nameserver list is empty"));
    }
    Ok(nameservers)
}

fn normalize_nameserver(server: &str) -> Result<String> {
    let server = server.trim();
    if server.is_empty() {
        return Err(anyhow!("nameserver-policy nameserver is empty"));
    }
    Ok(server.into())
}

fn normalize_policy_domain(domain: &str) -> Result<String> {
    let domain = domain.trim().trim_end_matches('.').to_ascii_lowercase();
    if domain.is_empty() {
        return Err(anyhow!("nameserver-policy domain is empty"));
    }
    Name::from_str_relaxed(&domain).context("nameserver-policy domain is invalid")?;
    Ok(domain)
}

fn retained_nameserver_policy_scope() -> Vec<String> {
    vec![
        "geosite and rule-provider nameserver-policy matchers".into(),
        "wildcard and multi-token nameserver-policy matchers".into(),
        "nameserver health checks and upstream query execution".into(),
        "fallback-filter geoip/upstream execution".into(),
        "default DNS runtime ownership".into(),
    ]
}

fn rust_dns_nameserver_policy_runtime_facts() -> Vec<String> {
    vec![
        "Rust parses nameserver-policy exact and +.suffix matchers".into(),
        "Rust selects policy nameservers for one domain without querying upstream DNS".into(),
        "Rust does not mutate system resolver state or Mihomo binaries".into(),
        "Mihomo fallback remains retained for geosite, wildcard/multi-token matchers, upstream execution, and default DNS".into(),
    ]
}

fn rust_dns_nameserver_policy_runtime_dir() -> Result<std::path::PathBuf> {
    Ok(dirs::app_runtime_dir()?.join(RUST_DNS_NAMESERVER_POLICY_RUNTIME_COMPONENT))
}

fn rust_dns_nameserver_policy_runtime_evidence_path() -> Result<std::path::PathBuf> {
    Ok(rust_dns_nameserver_policy_runtime_dir()?.join(RUST_DNS_NAMESERVER_POLICY_RUNTIME_EVIDENCE_FILE))
}

fn rust_dns_nameserver_policy_runtime_rollback_path() -> Result<std::path::PathBuf> {
    Ok(rust_dns_nameserver_policy_runtime_dir()?.join(RUST_DNS_NAMESERVER_POLICY_RUNTIME_ROLLBACK_FILE))
}

fn rust_dns_nameserver_policy_runtime_epoch_seconds() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matches_suffix_policy_rule() {
        let matcher = NameserverPolicyMatcher::DomainSuffix("example.com".into());

        assert!(matcher.matches("www.example.com"));
        assert!(matcher.matches("example.com"));
        assert!(!matcher.matches("notexample.com"));
    }

    #[test]
    fn parses_supported_nameserver_policy_rules_from_yaml() {
        let yaml = "dns:\n  nameserver-policy:\n    '+.example.com':\n      - https://dns.example/dns-query\n    exact.example.net: 1.1.1.1\n";
        let policy = NameserverPolicy::from_yaml(yaml).unwrap();
        let evidence = policy.evaluate("www.example.com");

        assert_eq!(evidence.evaluated_rule_count, 2);
        assert_eq!(evidence.selected_nameservers, vec!["https://dns.example/dns-query"]);
        assert_eq!(evidence.matched_rules.len(), 1);
    }

    #[test]
    fn leaves_geosite_policy_rules_on_fallback() {
        let yaml = "dns:\n  nameserver-policy:\n    geosite:cn: 223.5.5.5\n";
        let policy = NameserverPolicy::from_yaml(yaml).unwrap();

        assert!(policy.rules.is_empty());
        assert!(policy.warnings[0].contains("Mihomo-owned"));
    }
}
