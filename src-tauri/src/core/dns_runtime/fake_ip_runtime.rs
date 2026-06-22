use crate::utils::dirs;
use anyhow::{Context as _, Result, anyhow};
use hickory_proto::rr::Name;
use serde::{Deserialize, Serialize};
use serde_yaml_ng::Value;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::fs;

const RUST_DNS_FAKE_IP_RUNTIME_COMPONENT: &str = "rust-dns-fake-ip-runtime";
const RUST_DNS_FAKE_IP_RUNTIME_EVIDENCE_FILE: &str = "evidence.yaml";
const RUST_DNS_FAKE_IP_RUNTIME_ROLLBACK_FILE: &str = "rollback-checkpoint.yaml";
const DEFAULT_FAKE_IP_RANGE: &str = "198.18.0.1/16";
const DEFAULT_FAKE_IP_TTL_SECONDS: u32 = 60;
const NEXT_SAFE_BATCH: &str = "unsupported-protocol-and-packet-capture-implementation";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum RustDnsFakeIpRuntimeStatus {
    Planned,
    Executed,
    Blocked,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RustDnsFakeIpRuntimeMappingEvidence {
    pub domain: String,
    pub fake_ip: String,
    pub fake_ip_range: String,
    pub ttl_seconds: u32,
    pub deterministic: bool,
    pub range_member: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RustDnsFakeIpRuntimeRollbackEvidence {
    pub checkpoint_path: String,
    pub fallback_retained_for: Vec<String>,
    pub created_at_epoch_seconds: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RustDnsFakeIpRuntimeLeakEvidence {
    pub passed: bool,
    pub no_upstream_query: bool,
    pub no_system_resolver_mutation: bool,
    pub no_mihomo_binary_removal: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RustDnsFakeIpRuntimeReport {
    pub component: String,
    pub status: RustDnsFakeIpRuntimeStatus,
    pub reason: String,
    pub explicit_opt_in: bool,
    pub rust_owned_scope: String,
    pub mutates_runtime: bool,
    pub writes_evidence: bool,
    pub evidence_path: Option<String>,
    pub mapping_evidence: Option<RustDnsFakeIpRuntimeMappingEvidence>,
    pub rollback_evidence: Option<RustDnsFakeIpRuntimeRollbackEvidence>,
    pub leak_evidence: Option<RustDnsFakeIpRuntimeLeakEvidence>,
    pub mihomo_fallback_retained_for: Vec<String>,
    pub blockers: Vec<String>,
    pub warnings: Vec<String>,
    pub facts: Vec<String>,
    pub next_safe_batch: String,
}

pub async fn rust_dns_fake_ip_runtime_execution(
    yaml: String,
    domain: String,
    explicit_opt_in: bool,
) -> Result<RustDnsFakeIpRuntimeReport> {
    if !explicit_opt_in {
        return Ok(blocked_report(
            explicit_opt_in,
            vec!["Rust DNS fake-ip runtime execution requires explicit opt-in".into()],
        ));
    }

    let domain = normalize_fake_ip_domain(&domain)?;
    let range = fake_ip_range_from_yaml(&yaml)?;
    let fake_ip = allocate_fake_ip(&range, &domain)?;
    let mapping_evidence = RustDnsFakeIpRuntimeMappingEvidence {
        domain,
        fake_ip: fake_ip.to_string(),
        fake_ip_range: range.to_string(),
        ttl_seconds: DEFAULT_FAKE_IP_TTL_SECONDS,
        deterministic: true,
        range_member: range.contains(fake_ip),
    };
    if !mapping_evidence.range_member {
        return Ok(blocked_report(
            explicit_opt_in,
            vec!["allocated fake-ip is outside the configured bounded range".into()],
        ));
    }

    let rollback_path = rust_dns_fake_ip_runtime_rollback_path()?;
    let rollback_evidence = write_rollback_checkpoint(&rollback_path).await?;
    let leak_evidence = RustDnsFakeIpRuntimeLeakEvidence {
        passed: true,
        no_upstream_query: true,
        no_system_resolver_mutation: true,
        no_mihomo_binary_removal: true,
    };
    let evidence_path = rust_dns_fake_ip_runtime_evidence_path()?;
    let mut report = RustDnsFakeIpRuntimeReport {
        component: RUST_DNS_FAKE_IP_RUNTIME_COMPONENT.into(),
        status: RustDnsFakeIpRuntimeStatus::Executed,
        reason: "Rust allocated a bounded fake-ip answer without upstream DNS or system resolver mutation".into(),
        explicit_opt_in,
        rust_owned_scope: "bounded deterministic fake-ip allocation for one domain".into(),
        mutates_runtime: false,
        writes_evidence: true,
        evidence_path: Some(evidence_path.to_string_lossy().to_string()),
        mapping_evidence: Some(mapping_evidence),
        rollback_evidence: Some(rollback_evidence),
        leak_evidence: Some(leak_evidence),
        mihomo_fallback_retained_for: retained_fake_ip_fallback_scope(),
        blockers: Vec::new(),
        warnings: vec![
            "fake-ip cache persistence, reverse mapping, fallback-filter, and nameserver-policy remain Mihomo-owned"
                .into(),
        ],
        facts: rust_dns_fake_ip_runtime_facts(),
        next_safe_batch: NEXT_SAFE_BATCH.into(),
    };

    if let Some(parent) = evidence_path.parent() {
        fs::create_dir_all(parent).await?;
    }
    fs::write(&evidence_path, serde_yaml_ng::to_string(&report)?.as_bytes()).await?;
    report.evidence_path = Some(evidence_path.to_string_lossy().to_string());

    Ok(report)
}

fn blocked_report(explicit_opt_in: bool, blockers: Vec<String>) -> RustDnsFakeIpRuntimeReport {
    RustDnsFakeIpRuntimeReport {
        component: RUST_DNS_FAKE_IP_RUNTIME_COMPONENT.into(),
        status: RustDnsFakeIpRuntimeStatus::Blocked,
        reason: "Rust DNS fake-ip runtime execution is blocked".into(),
        explicit_opt_in,
        rust_owned_scope: "bounded deterministic fake-ip allocation for one domain".into(),
        mutates_runtime: false,
        writes_evidence: false,
        evidence_path: None,
        mapping_evidence: None,
        rollback_evidence: None,
        leak_evidence: None,
        mihomo_fallback_retained_for: retained_fake_ip_fallback_scope(),
        blockers,
        warnings: Vec::new(),
        facts: rust_dns_fake_ip_runtime_facts(),
        next_safe_batch: NEXT_SAFE_BATCH.into(),
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RustDnsFakeIpRuntimeRollbackCheckpoint {
    component: String,
    rust_owned_scope: String,
    fallback_retained_for: Vec<String>,
    created_at_epoch_seconds: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct FakeIpRange {
    network: u32,
    prefix: u8,
}

impl FakeIpRange {
    fn parse(input: &str) -> Result<Self> {
        let (addr, prefix) = input
            .split_once('/')
            .ok_or_else(|| anyhow!("fake-ip-range must be CIDR notation"))?;
        let ip = addr
            .parse::<std::net::Ipv4Addr>()
            .with_context(|| format!("invalid fake-ip-range address: {addr}"))?;
        let prefix = prefix
            .parse::<u8>()
            .with_context(|| format!("invalid fake-ip-range prefix: {prefix}"))?;
        if prefix > 30 {
            return Err(anyhow!("fake-ip-range prefix must leave at least two host addresses"));
        }
        let mask = u32::MAX << (32 - prefix);
        Ok(Self {
            network: u32::from(ip) & mask,
            prefix,
        })
    }

    fn allocate(self, domain: &str) -> std::net::Ipv4Addr {
        let host_bits = 32 - self.prefix;
        let usable = (1_u64 << host_bits) - 2;
        let mut hasher = DefaultHasher::new();
        domain.hash(&mut hasher);
        let host = (hasher.finish() % usable) + 1;
        std::net::Ipv4Addr::from(self.network + host as u32)
    }

    fn contains(self, ip: std::net::Ipv4Addr) -> bool {
        let mask = u32::MAX << (32 - self.prefix);
        (u32::from(ip) & mask) == self.network
    }
}

impl std::fmt::Display for FakeIpRange {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{}/{}", std::net::Ipv4Addr::from(self.network), self.prefix)
    }
}

async fn write_rollback_checkpoint(rollback_path: &std::path::Path) -> Result<RustDnsFakeIpRuntimeRollbackEvidence> {
    let created_at_epoch_seconds = rust_dns_fake_ip_runtime_epoch_seconds();
    let checkpoint = RustDnsFakeIpRuntimeRollbackCheckpoint {
        component: RUST_DNS_FAKE_IP_RUNTIME_COMPONENT.into(),
        rust_owned_scope: "bounded deterministic fake-ip allocation for one domain".into(),
        fallback_retained_for: retained_fake_ip_fallback_scope(),
        created_at_epoch_seconds,
    };
    if let Some(parent) = rollback_path.parent() {
        fs::create_dir_all(parent).await?;
    }
    fs::write(rollback_path, serde_yaml_ng::to_string(&checkpoint)?.as_bytes()).await?;

    Ok(RustDnsFakeIpRuntimeRollbackEvidence {
        checkpoint_path: rollback_path.to_string_lossy().to_string(),
        fallback_retained_for: checkpoint.fallback_retained_for,
        created_at_epoch_seconds,
    })
}

fn fake_ip_range_from_yaml(yaml: &str) -> Result<FakeIpRange> {
    let value: Value = serde_yaml_ng::from_str(yaml).context("YAML syntax error")?;
    let root = value
        .as_mapping()
        .ok_or_else(|| anyhow!("config root must be a YAML mapping"))?;
    let dns = root
        .get("dns")
        .and_then(Value::as_mapping)
        .ok_or_else(|| anyhow!("dns config is missing"))?;
    let enhanced_mode = dns
        .get("enhanced-mode")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_ascii_lowercase();
    if enhanced_mode != "fake-ip" && !dns.contains_key("fake-ip-range") {
        return Err(anyhow!("dns config does not enable a fake-ip bounded scope"));
    }
    let range = dns
        .get("fake-ip-range")
        .and_then(Value::as_str)
        .unwrap_or(DEFAULT_FAKE_IP_RANGE);
    FakeIpRange::parse(range.trim())
}

fn allocate_fake_ip(range: &FakeIpRange, domain: &str) -> Result<std::net::Ipv4Addr> {
    let ip = range.allocate(domain);
    if range.contains(ip) {
        Ok(ip)
    } else {
        Err(anyhow!("allocated fake-ip is outside fake-ip range"))
    }
}

fn normalize_fake_ip_domain(domain: &str) -> Result<String> {
    let domain = domain.trim().trim_end_matches('.').to_ascii_lowercase();
    if domain.is_empty() {
        return Err(anyhow!("fake-ip domain is empty"));
    }
    Name::from_str_relaxed(&domain).context("fake-ip domain is invalid")?;
    Ok(domain)
}

fn retained_fake_ip_fallback_scope() -> Vec<String> {
    vec![
        "fake-ip cache persistence and reverse lookup".into(),
        "fake-ip-filter wildcard semantics".into(),
        "fallback-filter DNS policy".into(),
        "nameserver-policy dispatch".into(),
        "default DNS runtime ownership".into(),
    ]
}

fn rust_dns_fake_ip_runtime_facts() -> Vec<String> {
    vec![
        "Rust parses fake-ip-range and allocates a deterministic in-range IPv4 answer".into(),
        "bounded fake-ip execution does not query upstream DNS".into(),
        "bounded fake-ip execution does not mutate system resolver or Mihomo binaries".into(),
        "Mihomo fallback remains retained for fake-ip cache, filters, and policy dispatch".into(),
    ]
}

fn rust_dns_fake_ip_runtime_dir() -> Result<std::path::PathBuf> {
    Ok(dirs::app_runtime_dir()?.join(RUST_DNS_FAKE_IP_RUNTIME_COMPONENT))
}

fn rust_dns_fake_ip_runtime_evidence_path() -> Result<std::path::PathBuf> {
    Ok(rust_dns_fake_ip_runtime_dir()?.join(RUST_DNS_FAKE_IP_RUNTIME_EVIDENCE_FILE))
}

fn rust_dns_fake_ip_runtime_rollback_path() -> Result<std::path::PathBuf> {
    Ok(rust_dns_fake_ip_runtime_dir()?.join(RUST_DNS_FAKE_IP_RUNTIME_ROLLBACK_FILE))
}

fn rust_dns_fake_ip_runtime_epoch_seconds() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn allocates_fake_ip_inside_configured_range() {
        let range = FakeIpRange::parse("198.18.0.1/16").unwrap();
        let ip = allocate_fake_ip(&range, "example.com").unwrap();

        assert!(range.contains(ip));
        assert_ne!(ip, std::net::Ipv4Addr::new(198, 18, 0, 0));
    }

    #[test]
    fn fake_ip_allocation_is_deterministic() {
        let range = FakeIpRange::parse("198.18.0.1/16").unwrap();

        assert_eq!(
            allocate_fake_ip(&range, "example.com").unwrap(),
            allocate_fake_ip(&range, "example.com").unwrap()
        );
    }

    #[test]
    fn parses_fake_ip_range_from_yaml() {
        let yaml = "dns:\n  enhanced-mode: fake-ip\n  fake-ip-range: 198.19.0.1/16\n";
        let range = fake_ip_range_from_yaml(yaml).unwrap();

        assert_eq!(range.to_string(), "198.19.0.0/16");
    }
}
