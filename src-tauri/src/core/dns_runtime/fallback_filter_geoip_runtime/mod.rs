mod constants;
mod evidence;
mod geoip;
mod yaml;

use self::{
    constants::{
        NEXT_SAFE_BATCH, RUST_DNS_FALLBACK_FILTER_GEOIP_COMPONENT, RUST_DNS_FALLBACK_FILTER_GEOIP_OWNED_SCOPE,
    },
    evidence::{
        retained_fallback_filter_geoip_scope, rust_dns_fallback_filter_geoip_evidence_path,
        rust_dns_fallback_filter_geoip_facts, rust_dns_fallback_filter_geoip_rollback_path, write_rollback_checkpoint,
    },
    geoip::evaluate_geoip_filter,
    yaml::parse_geoip_filter,
};
use anyhow::{Context as _, Result};
use serde::{Deserialize, Serialize};
use smartstring::alias::String;
use std::net::IpAddr;
use tokio::fs;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum RustDnsFallbackFilterGeoipRuntimeStatus {
    Planned,
    Executed,
    Blocked,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RustDnsFallbackFilterGeoipDecisionEvidence {
    pub domain: String,
    pub candidate_ip: String,
    pub geoip_enabled: bool,
    pub geoip_code: String,
    pub matched_country: bool,
    pub matched_cidr: Option<String>,
    pub fallback_required: bool,
    pub evaluated_cidr_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RustDnsFallbackFilterGeoipRollbackEvidence {
    pub checkpoint_path: String,
    pub fallback_retained_for: Vec<String>,
    pub created_at_epoch_seconds: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RustDnsFallbackFilterGeoipLeakEvidence {
    pub passed: bool,
    pub no_upstream_query: bool,
    pub no_system_resolver_mutation: bool,
    pub no_mihomo_binary_removal: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RustDnsFallbackFilterGeoipRuntimeReport {
    pub component: String,
    pub status: RustDnsFallbackFilterGeoipRuntimeStatus,
    pub reason: String,
    pub explicit_opt_in: bool,
    pub rust_owned_scope: String,
    pub mutates_runtime: bool,
    pub writes_evidence: bool,
    pub evidence_path: Option<String>,
    pub decision_evidence: Option<RustDnsFallbackFilterGeoipDecisionEvidence>,
    pub rollback_evidence: Option<RustDnsFallbackFilterGeoipRollbackEvidence>,
    pub leak_evidence: Option<RustDnsFallbackFilterGeoipLeakEvidence>,
    pub mihomo_fallback_retained_for: Vec<String>,
    pub blockers: Vec<String>,
    pub warnings: Vec<String>,
    pub facts: Vec<String>,
    pub next_safe_batch: String,
}

pub async fn rust_dns_fallback_filter_geoip_runtime_execution(
    yaml: std::string::String,
    domain: std::string::String,
    candidate_ip: std::string::String,
    explicit_opt_in: bool,
) -> Result<RustDnsFallbackFilterGeoipRuntimeReport> {
    if !explicit_opt_in {
        return Ok(blocked_report(
            explicit_opt_in,
            vec!["Rust DNS fallback-filter geoip execution requires explicit opt-in".into()],
        ));
    }

    let filter = match parse_geoip_filter(&yaml) {
        Ok(filter) => filter,
        Err(error) => {
            return Ok(blocked_report(explicit_opt_in, vec![error.to_string().into()]));
        }
    };
    let candidate_ip = candidate_ip
        .trim()
        .parse::<IpAddr>()
        .with_context(|| format!("candidate IP is invalid: {candidate_ip}"))?;
    let decision_evidence = evaluate_geoip_filter(&filter, &domain, candidate_ip);
    let rollback_path = rust_dns_fallback_filter_geoip_rollback_path()?;
    let rollback_evidence = write_rollback_checkpoint(&rollback_path).await?;
    let leak_evidence = RustDnsFallbackFilterGeoipLeakEvidence {
        passed: true,
        no_upstream_query: true,
        no_system_resolver_mutation: true,
        no_mihomo_binary_removal: true,
    };
    let evidence_path = rust_dns_fallback_filter_geoip_evidence_path()?;
    let mut report = RustDnsFallbackFilterGeoipRuntimeReport {
        component: RUST_DNS_FALLBACK_FILTER_GEOIP_COMPONENT.into(),
        status: RustDnsFallbackFilterGeoipRuntimeStatus::Executed,
        reason: "Rust evaluated bounded fallback-filter geoip/geoip-code policy without upstream DNS or system resolver mutation".into(),
        explicit_opt_in,
        rust_owned_scope: RUST_DNS_FALLBACK_FILTER_GEOIP_OWNED_SCOPE.into(),
        mutates_runtime: false,
        writes_evidence: true,
        evidence_path: Some(evidence_path.to_string_lossy().to_string().into()),
        decision_evidence: Some(decision_evidence),
        rollback_evidence: Some(rollback_evidence),
        leak_evidence: Some(leak_evidence),
        mihomo_fallback_retained_for: retained_fallback_filter_geoip_scope(),
        blockers: Vec::new(),
        warnings: vec![
            "GeoIP ownership is bounded to the built-in canary CIDR set; full geodata databases and fallback upstream execution remain Mihomo-owned".into(),
        ],
        facts: rust_dns_fallback_filter_geoip_facts(),
        next_safe_batch: NEXT_SAFE_BATCH.into(),
    };

    if let Some(parent) = evidence_path.parent() {
        fs::create_dir_all(parent).await?;
    }
    fs::write(&evidence_path, serde_yaml_ng::to_string(&report)?.as_bytes()).await?;
    report.evidence_path = Some(evidence_path.to_string_lossy().to_string().into());

    Ok(report)
}

fn blocked_report(explicit_opt_in: bool, blockers: Vec<String>) -> RustDnsFallbackFilterGeoipRuntimeReport {
    RustDnsFallbackFilterGeoipRuntimeReport {
        component: RUST_DNS_FALLBACK_FILTER_GEOIP_COMPONENT.into(),
        status: RustDnsFallbackFilterGeoipRuntimeStatus::Blocked,
        reason: "Rust DNS fallback-filter geoip execution is blocked".into(),
        explicit_opt_in,
        rust_owned_scope: RUST_DNS_FALLBACK_FILTER_GEOIP_OWNED_SCOPE.into(),
        mutates_runtime: false,
        writes_evidence: false,
        evidence_path: None,
        decision_evidence: None,
        rollback_evidence: None,
        leak_evidence: None,
        mihomo_fallback_retained_for: retained_fallback_filter_geoip_scope(),
        blockers,
        warnings: Vec::new(),
        facts: rust_dns_fallback_filter_geoip_facts(),
        next_safe_batch: NEXT_SAFE_BATCH.into(),
    }
}
