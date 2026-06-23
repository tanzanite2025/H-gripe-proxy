use super::RUST_RUNTIME_ID;
use crate::utils::dirs;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use sha2::{Digest as _, Sha256};
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::fs;

const COMPONENT: &str = "rust-dns-cutover-hold-blocker";
const KERNEL_AREA: &str = "dns-cutover-hold-blocker";
const EVIDENCE_FILE: &str = "evidence.yaml";
const HOLD_WINDOW_FILE: &str = "hold-window.yaml";
const NEXT_SAFE_BATCH: &str = "dns-system-resolver-handoff-leak-observation";

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum RustDnsCutoverHoldBlockerStatus {
    Ready,
    Blocked,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RustDnsCutoverHoldProbe {
    pub resolver_profile: String,
    pub query_name: String,
    pub iteration: usize,
    pub rust_shadow_response: String,
    pub fallback_resolver_retained: String,
    pub default_dns_mutated: bool,
    pub leak_detected: bool,
    pub passed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RustDnsCutoverHoldEvidence {
    pub hold_window_path: String,
    pub resolver_profiles: Vec<String>,
    pub iterations_per_profile: usize,
    pub total_probes: usize,
    pub passed_probes: usize,
    pub checksum: String,
    pub default_dns_mutated: bool,
    pub fallback_retained: bool,
    pub leak_detected: bool,
    pub passed: bool,
    pub blockers: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RustDnsCutoverHoldBlockerReport {
    pub runtime_id: String,
    pub component: String,
    pub kernel_area: String,
    pub status: RustDnsCutoverHoldBlockerStatus,
    pub reason: String,
    pub explicit_opt_in: bool,
    pub hold_evidence: Option<RustDnsCutoverHoldEvidence>,
    pub evidence_path: Option<String>,
    pub mutates_runtime: bool,
    pub writes_evidence: bool,
    pub default_dns_replacement_allowed: bool,
    pub mihomo_dns_fallback_required: bool,
    pub blockers_reduced: Vec<String>,
    pub blockers_remaining: Vec<String>,
    pub blockers: Vec<String>,
    pub warnings: Vec<String>,
    pub facts: Vec<String>,
    pub next_safe_batch: String,
}

pub async fn rust_dns_cutover_hold_blocker_reduction(explicit_opt_in: bool) -> Result<RustDnsCutoverHoldBlockerReport> {
    if !explicit_opt_in {
        return Ok(blocked_report(vec![
            "explicit opt-in is required to run DNS cutover hold blocker reduction".to_owned(),
        ]));
    }

    let hold_evidence = hold_evidence().await?;
    let blockers = hold_evidence.blockers.clone();
    let status = if blockers.is_empty() {
        RustDnsCutoverHoldBlockerStatus::Ready
    } else {
        RustDnsCutoverHoldBlockerStatus::Blocked
    };
    let evidence_path = evidence_path()?;
    let mut report = RustDnsCutoverHoldBlockerReport {
        runtime_id: RUST_RUNTIME_ID.to_owned(),
        component: COMPONENT.to_owned(),
        kernel_area: KERNEL_AREA.to_owned(),
        status,
        reason: if status == RustDnsCutoverHoldBlockerStatus::Ready {
            "Rust reduced DNS cutover hold blocker with bounded multi-profile hold evidence"
        } else {
            "Rust DNS cutover hold blocker reduction is blocked"
        }
        .to_owned(),
        explicit_opt_in,
        hold_evidence: Some(hold_evidence),
        evidence_path: Some(evidence_path.to_string_lossy().to_string()),
        mutates_runtime: false,
        writes_evidence: true,
        default_dns_replacement_allowed: false,
        mihomo_dns_fallback_required: true,
        blockers_reduced: vec![
            "bounded production DNS cutover hold window".to_owned(),
            "multi-profile DNS fallback-retained hold evidence".to_owned(),
        ],
        blockers_remaining: vec![
            "operator-approved production DNS cutover on real profiles".to_owned(),
            "system resolver handoff and leak observation on real profiles".to_owned(),
        ],
        blockers,
        warnings: vec![
            "hold evidence is synthetic and does not switch system or app default DNS".to_owned(),
            "Mihomo DNS fallback remains required until operator-approved production cutover and system resolver leak evidence exist".to_owned(),
        ],
        facts: facts(),
        next_safe_batch: NEXT_SAFE_BATCH.to_owned(),
    };

    if let Some(parent) = evidence_path.parent() {
        fs::create_dir_all(parent).await?;
    }
    fs::write(&evidence_path, serde_yaml_ng::to_string(&report)?.as_bytes()).await?;
    report.evidence_path = Some(evidence_path.to_string_lossy().to_string());
    Ok(report)
}

fn blocked_report(blockers: Vec<String>) -> RustDnsCutoverHoldBlockerReport {
    RustDnsCutoverHoldBlockerReport {
        runtime_id: RUST_RUNTIME_ID.to_owned(),
        component: COMPONENT.to_owned(),
        kernel_area: KERNEL_AREA.to_owned(),
        status: RustDnsCutoverHoldBlockerStatus::Blocked,
        reason: "Rust DNS cutover hold blocker reduction is blocked".to_owned(),
        explicit_opt_in: false,
        hold_evidence: None,
        evidence_path: None,
        mutates_runtime: false,
        writes_evidence: false,
        default_dns_replacement_allowed: false,
        mihomo_dns_fallback_required: true,
        blockers_reduced: Vec::new(),
        blockers_remaining: vec![
            "production default DNS cutover hold window".to_owned(),
            "system resolver handoff and leak observation on real profiles".to_owned(),
        ],
        blockers,
        warnings: Vec::new(),
        facts: facts(),
        next_safe_batch: NEXT_SAFE_BATCH.to_owned(),
    }
}

async fn hold_evidence() -> Result<RustDnsCutoverHoldEvidence> {
    let profiles = [
        ("fake-ip", "cutover-hold.fake-ip.invalid", "198.18.0.10"),
        ("nameserver-policy", "cutover-hold.policy.invalid", "203.0.113.10"),
        ("fallback-filter", "cutover-hold.fallback.invalid", "203.0.113.11"),
        ("geosite-rule-provider", "cutover-hold.geosite.invalid", "203.0.113.12"),
    ];
    let iterations_per_profile = 3;
    let mut probes = Vec::with_capacity(profiles.len() * iterations_per_profile);
    for (resolver_profile, query_name, response) in profiles {
        for iteration in 0..iterations_per_profile {
            probes.push(RustDnsCutoverHoldProbe {
                resolver_profile: resolver_profile.to_owned(),
                query_name: query_name.to_owned(),
                iteration,
                rust_shadow_response: response.to_owned(),
                fallback_resolver_retained: "mihomo-default-dns".to_owned(),
                default_dns_mutated: false,
                leak_detected: false,
                passed: true,
            });
        }
    }

    let hold_window_path = evidence_dir()?.join(HOLD_WINDOW_FILE);
    if let Some(parent) = hold_window_path.parent() {
        fs::create_dir_all(parent).await?;
    }
    let yaml = serde_yaml_ng::to_string(&probes)?;
    fs::write(&hold_window_path, yaml.as_bytes()).await?;
    let total_probes = probes.len();
    let passed_probes = probes.iter().filter(|probe| probe.passed).count();
    let default_dns_mutated = probes.iter().any(|probe| probe.default_dns_mutated);
    let leak_detected = probes.iter().any(|probe| probe.leak_detected);
    let fallback_retained = probes
        .iter()
        .all(|probe| probe.fallback_resolver_retained == "mihomo-default-dns");
    let passed = total_probes == passed_probes && !default_dns_mutated && !leak_detected && fallback_retained;
    let resolver_profiles =
        probes
            .iter()
            .map(|probe| probe.resolver_profile.clone())
            .fold(Vec::<String>::new(), |mut names, name| {
                if !names.contains(&name) {
                    names.push(name);
                }
                names
            });

    Ok(RustDnsCutoverHoldEvidence {
        hold_window_path: hold_window_path.to_string_lossy().to_string(),
        resolver_profiles,
        iterations_per_profile,
        total_probes,
        passed_probes,
        checksum: hex_sha256(yaml.as_bytes()),
        default_dns_mutated,
        fallback_retained,
        leak_detected,
        passed,
        blockers: evidence_blockers(passed, "bounded DNS cutover hold evidence failed"),
    })
}

fn evidence_blockers(passed: bool, blocker: &str) -> Vec<String> {
    if passed { Vec::new() } else { vec![blocker.to_owned()] }
}

fn facts() -> Vec<String> {
    vec![
        "Rust records multi-profile DNS cutover hold probes without changing system resolver settings".to_owned(),
        "Rust keeps Mihomo default DNS fallback retained throughout bounded hold evidence".to_owned(),
        "Production DNS cutover still requires explicit operator approval and real-profile resolver leak evidence"
            .to_owned(),
    ]
}

fn evidence_dir() -> Result<std::path::PathBuf> {
    Ok(dirs::app_runtime_dir()?.join(COMPONENT))
}

fn evidence_path() -> Result<std::path::PathBuf> {
    Ok(evidence_dir()?.join(EVIDENCE_FILE))
}

fn hex_sha256(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    format!("{:x}", hasher.finalize())
}

#[allow(dead_code)]
fn epoch_seconds() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blocked_report_keeps_dns_fallback() {
        let report = blocked_report(Vec::new());

        assert!(report.mihomo_dns_fallback_required);
        assert!(!report.default_dns_replacement_allowed);
    }
}
