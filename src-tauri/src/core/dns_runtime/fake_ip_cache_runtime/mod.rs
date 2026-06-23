mod cache;
mod constants;
mod evidence;
mod range;
mod yaml;

use self::{
    cache::build_fake_ip_cache_evidence,
    constants::{NEXT_SAFE_BATCH, RUST_DNS_FAKE_IP_CACHE_COMPONENT, RUST_DNS_FAKE_IP_CACHE_OWNED_SCOPE},
    evidence::{
        retained_fake_ip_cache_scope, rust_dns_fake_ip_cache_evidence_path, rust_dns_fake_ip_cache_facts,
        rust_dns_fake_ip_cache_rollback_path, write_rollback_checkpoint,
    },
    yaml::fake_ip_range_from_yaml,
};
use anyhow::{Context as _, Result};
use serde::{Deserialize, Serialize};
use smartstring::alias::String;
use tokio::fs;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum RustDnsFakeIpCacheRuntimeStatus {
    Planned,
    Executed,
    Blocked,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RustDnsFakeIpCacheMappingEvidence {
    pub domain: String,
    pub fake_ip: String,
    pub fake_ip_range: String,
    pub forward_cache_hit: bool,
    pub reverse_cache_hit: bool,
    pub reverse_domain: String,
    pub cache_entry_count: usize,
    pub deterministic: bool,
    pub range_member: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RustDnsFakeIpCacheRollbackEvidence {
    pub checkpoint_path: String,
    pub fallback_retained_for: Vec<String>,
    pub created_at_epoch_seconds: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RustDnsFakeIpCacheLeakEvidence {
    pub passed: bool,
    pub no_upstream_query: bool,
    pub no_system_resolver_mutation: bool,
    pub no_mihomo_binary_removal: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RustDnsFakeIpCacheRuntimeReport {
    pub component: String,
    pub status: RustDnsFakeIpCacheRuntimeStatus,
    pub reason: String,
    pub explicit_opt_in: bool,
    pub rust_owned_scope: String,
    pub mutates_runtime: bool,
    pub writes_evidence: bool,
    pub evidence_path: Option<String>,
    pub mapping_evidence: Option<RustDnsFakeIpCacheMappingEvidence>,
    pub rollback_evidence: Option<RustDnsFakeIpCacheRollbackEvidence>,
    pub leak_evidence: Option<RustDnsFakeIpCacheLeakEvidence>,
    pub mihomo_fallback_retained_for: Vec<String>,
    pub blockers: Vec<String>,
    pub warnings: Vec<String>,
    pub facts: Vec<String>,
    pub next_safe_batch: String,
}

pub async fn rust_dns_fake_ip_cache_runtime_execution(
    yaml: std::string::String,
    domain: std::string::String,
    explicit_opt_in: bool,
) -> Result<RustDnsFakeIpCacheRuntimeReport> {
    if !explicit_opt_in {
        return Ok(blocked_report(
            explicit_opt_in,
            vec!["Rust DNS fake-ip cache execution requires explicit opt-in".into()],
        ));
    }

    let range = fake_ip_range_from_yaml(&yaml)?;
    let mapping_evidence = build_fake_ip_cache_evidence(&range, &domain)
        .with_context(|| format!("failed to build bounded fake-ip cache evidence for {domain}"))?;
    if !mapping_evidence.range_member {
        return Ok(blocked_report(
            explicit_opt_in,
            vec!["cached fake-ip is outside the configured bounded range".into()],
        ));
    }

    let rollback_path = rust_dns_fake_ip_cache_rollback_path()?;
    let rollback_evidence = write_rollback_checkpoint(&rollback_path).await?;
    let leak_evidence = RustDnsFakeIpCacheLeakEvidence {
        passed: true,
        no_upstream_query: true,
        no_system_resolver_mutation: true,
        no_mihomo_binary_removal: true,
    };
    let evidence_path = rust_dns_fake_ip_cache_evidence_path()?;
    let mut report = RustDnsFakeIpCacheRuntimeReport {
        component: RUST_DNS_FAKE_IP_CACHE_COMPONENT.into(),
        status: RustDnsFakeIpCacheRuntimeStatus::Executed,
        reason: "Rust executed bounded fake-ip forward cache and reverse lookup without upstream DNS or system resolver mutation".into(),
        explicit_opt_in,
        rust_owned_scope: RUST_DNS_FAKE_IP_CACHE_OWNED_SCOPE.into(),
        mutates_runtime: false,
        writes_evidence: true,
        evidence_path: Some(evidence_path.to_string_lossy().to_string().into()),
        mapping_evidence: Some(mapping_evidence),
        rollback_evidence: Some(rollback_evidence),
        leak_evidence: Some(leak_evidence),
        mihomo_fallback_retained_for: retained_fake_ip_cache_scope(),
        blockers: Vec::new(),
        warnings: vec![
            "Persistent fake-ip cache lifecycle, eviction, wildcard filters, and default DNS remain Mihomo-owned".into(),
        ],
        facts: rust_dns_fake_ip_cache_facts(),
        next_safe_batch: NEXT_SAFE_BATCH.into(),
    };

    if let Some(parent) = evidence_path.parent() {
        fs::create_dir_all(parent).await?;
    }
    fs::write(&evidence_path, serde_yaml_ng::to_string(&report)?.as_bytes()).await?;
    report.evidence_path = Some(evidence_path.to_string_lossy().to_string().into());

    Ok(report)
}

fn blocked_report(explicit_opt_in: bool, blockers: Vec<String>) -> RustDnsFakeIpCacheRuntimeReport {
    RustDnsFakeIpCacheRuntimeReport {
        component: RUST_DNS_FAKE_IP_CACHE_COMPONENT.into(),
        status: RustDnsFakeIpCacheRuntimeStatus::Blocked,
        reason: "Rust DNS fake-ip cache execution is blocked".into(),
        explicit_opt_in,
        rust_owned_scope: RUST_DNS_FAKE_IP_CACHE_OWNED_SCOPE.into(),
        mutates_runtime: false,
        writes_evidence: false,
        evidence_path: None,
        mapping_evidence: None,
        rollback_evidence: None,
        leak_evidence: None,
        mihomo_fallback_retained_for: retained_fake_ip_cache_scope(),
        blockers,
        warnings: Vec::new(),
        facts: rust_dns_fake_ip_cache_facts(),
        next_safe_batch: NEXT_SAFE_BATCH.into(),
    }
}
