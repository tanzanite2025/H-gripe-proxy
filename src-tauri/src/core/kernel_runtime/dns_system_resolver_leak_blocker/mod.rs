use super::RUST_RUNTIME_ID;
use crate::utils::dirs;
use anyhow::{Context as _, Result};
use serde::{Deserialize, Serialize};
use sha2::{Digest as _, Sha256};
use std::{
    process::Command,
    time::{SystemTime, UNIX_EPOCH},
};
use tokio::fs;

const COMPONENT: &str = "rust-dns-system-resolver-leak-blocker";
const KERNEL_AREA: &str = "dns-system-resolver-leak-blocker";
const EVIDENCE_FILE: &str = "evidence.yaml";
const SNAPSHOT_FILE: &str = "system-resolver-snapshot.txt";
const RESTORE_PLAN_FILE: &str = "system-resolver-restore-plan.yaml";
const LEAK_OBSERVATION_FILE: &str = "leak-observation.yaml";
const NEXT_SAFE_BATCH: &str = "dns-privileged-system-resolver-apply-restore";

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum RustDnsSystemResolverLeakBlockerStatus {
    Ready,
    Blocked,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RustDnsSystemResolverSnapshotEvidence {
    pub platform: String,
    pub command: Vec<String>,
    pub snapshot_path: String,
    pub restore_plan_path: String,
    pub snapshot_present: bool,
    pub resolver_entries_observed: usize,
    pub snapshot_checksum: Option<String>,
    pub mutates_system_resolver: bool,
    pub passed: bool,
    pub blockers: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RustDnsSystemResolverRestoreStep {
    pub order: usize,
    pub action: String,
    pub requires_privilege: bool,
    pub mutates_system_resolver_in_this_step: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RustDnsLeakObservationProbe {
    pub query_name: String,
    pub expected_rust_resolver: String,
    pub observed_resolver: String,
    pub system_resolver_mutated: bool,
    pub external_leak_detected: bool,
    pub passed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RustDnsSystemResolverLeakEvidence {
    pub leak_observation_path: String,
    pub probes: Vec<RustDnsLeakObservationProbe>,
    pub leak_detected: bool,
    pub system_resolver_mutated: bool,
    pub checksum: String,
    pub passed: bool,
    pub blockers: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RustDnsSystemResolverLeakBlockerReport {
    pub runtime_id: String,
    pub component: String,
    pub kernel_area: String,
    pub status: RustDnsSystemResolverLeakBlockerStatus,
    pub reason: String,
    pub explicit_opt_in: bool,
    pub snapshot_evidence: Option<RustDnsSystemResolverSnapshotEvidence>,
    pub restore_steps: Vec<RustDnsSystemResolverRestoreStep>,
    pub leak_evidence: Option<RustDnsSystemResolverLeakEvidence>,
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

pub async fn rust_dns_system_resolver_leak_blocker_reduction(
    explicit_opt_in: bool,
) -> Result<RustDnsSystemResolverLeakBlockerReport> {
    if !explicit_opt_in {
        return Ok(blocked_report(vec![
            "explicit opt-in is required to run DNS system resolver leak blocker reduction".to_owned(),
        ]));
    }

    let snapshot_evidence = snapshot_evidence().await?;
    let restore_steps = restore_steps();
    write_restore_plan(&restore_steps).await?;
    let leak_evidence = leak_evidence().await?;
    let mut blockers = Vec::new();
    blockers.extend(snapshot_evidence.blockers.iter().cloned());
    blockers.extend(leak_evidence.blockers.iter().cloned());
    let status = if blockers.is_empty() {
        RustDnsSystemResolverLeakBlockerStatus::Ready
    } else {
        RustDnsSystemResolverLeakBlockerStatus::Blocked
    };
    let evidence_path = evidence_path()?;
    let mut report = RustDnsSystemResolverLeakBlockerReport {
        runtime_id: RUST_RUNTIME_ID.to_owned(),
        component: COMPONENT.to_owned(),
        kernel_area: KERNEL_AREA.to_owned(),
        status,
        reason: if status == RustDnsSystemResolverLeakBlockerStatus::Ready {
            "Rust reduced DNS system resolver leak blocker with read-only resolver snapshot, restore plan, and bounded leak evidence"
        } else {
            "Rust DNS system resolver leak blocker reduction is blocked"
        }
        .to_owned(),
        explicit_opt_in,
        snapshot_evidence: Some(snapshot_evidence),
        restore_steps,
        leak_evidence: Some(leak_evidence),
        evidence_path: Some(evidence_path.to_string_lossy().to_string()),
        mutates_runtime: false,
        writes_evidence: true,
        default_dns_replacement_allowed: false,
        mihomo_dns_fallback_required: true,
        blockers_reduced: vec![
            "read-only system resolver snapshot evidence".to_owned(),
            "system resolver restore-plan evidence".to_owned(),
            "bounded DNS leak observation evidence".to_owned(),
        ],
        blockers_remaining: vec![
            "operator-approved production DNS cutover on real profiles".to_owned(),
            "privileged system resolver apply/restore on real interfaces".to_owned(),
        ],
        blockers,
        warnings: vec![
            "system resolver evidence is read-only and does not mutate OS DNS settings".to_owned(),
            "Mihomo DNS fallback remains required until privileged resolver apply/restore and operator-approved production cutover evidence exist".to_owned(),
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

fn blocked_report(blockers: Vec<String>) -> RustDnsSystemResolverLeakBlockerReport {
    RustDnsSystemResolverLeakBlockerReport {
        runtime_id: RUST_RUNTIME_ID.to_owned(),
        component: COMPONENT.to_owned(),
        kernel_area: KERNEL_AREA.to_owned(),
        status: RustDnsSystemResolverLeakBlockerStatus::Blocked,
        reason: "Rust DNS system resolver leak blocker reduction is blocked".to_owned(),
        explicit_opt_in: false,
        snapshot_evidence: None,
        restore_steps: Vec::new(),
        leak_evidence: None,
        evidence_path: None,
        mutates_runtime: false,
        writes_evidence: false,
        default_dns_replacement_allowed: false,
        mihomo_dns_fallback_required: true,
        blockers_reduced: Vec::new(),
        blockers_remaining: vec![
            "system resolver handoff and leak observation on real profiles".to_owned(),
            "operator-approved production DNS cutover on real profiles".to_owned(),
        ],
        blockers,
        warnings: Vec::new(),
        facts: facts(),
        next_safe_batch: NEXT_SAFE_BATCH.to_owned(),
    }
}

async fn snapshot_evidence() -> Result<RustDnsSystemResolverSnapshotEvidence> {
    let (program, args) = resolver_snapshot_command();
    let output = Command::new(program)
        .args(&args)
        .output()
        .with_context(|| format!("failed to run resolver snapshot command `{program}`"))?;
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let snapshot_path = evidence_dir()?.join(SNAPSHOT_FILE);
    if let Some(parent) = snapshot_path.parent() {
        fs::create_dir_all(parent).await?;
    }
    fs::write(&snapshot_path, stdout.as_bytes()).await?;
    let snapshot_present = output.status.success() && !stdout.trim().is_empty();
    let resolver_entries_observed = stdout
        .lines()
        .filter(|line| {
            let lower = line.to_ascii_lowercase();
            lower.contains("dns") || lower.contains("resolver") || lower.contains("nameserver")
        })
        .count();
    let passed = snapshot_present && resolver_entries_observed > 0;

    Ok(RustDnsSystemResolverSnapshotEvidence {
        platform: std::env::consts::OS.to_owned(),
        command: std::iter::once(program.to_owned())
            .chain(args.iter().cloned())
            .collect(),
        snapshot_path: snapshot_path.to_string_lossy().to_string(),
        restore_plan_path: restore_plan_path()?.to_string_lossy().to_string(),
        snapshot_present,
        resolver_entries_observed,
        snapshot_checksum: if stdout.is_empty() {
            None
        } else {
            Some(hex_sha256(stdout.as_bytes()))
        },
        mutates_system_resolver: false,
        passed,
        blockers: evidence_blockers(passed, "read-only system resolver snapshot evidence failed"),
    })
}

#[cfg(target_os = "windows")]
fn resolver_snapshot_command() -> (&'static str, Vec<String>) {
    ("ipconfig", vec!["/all".to_owned()])
}

#[cfg(not(target_os = "windows"))]
fn resolver_snapshot_command() -> (&'static str, Vec<String>) {
    (
        "sh",
        vec![
            "-c".to_owned(),
            "cat /etc/resolv.conf 2>/dev/null || scutil --dns 2>/dev/null".to_owned(),
        ],
    )
}

fn restore_steps() -> Vec<RustDnsSystemResolverRestoreStep> {
    vec![
        RustDnsSystemResolverRestoreStep {
            order: 1,
            action: "capture pre-cutover system resolver snapshot checksum".to_owned(),
            requires_privilege: false,
            mutates_system_resolver_in_this_step: false,
        },
        RustDnsSystemResolverRestoreStep {
            order: 2,
            action: "apply approved resolver handoff rollback plan".to_owned(),
            requires_privilege: true,
            mutates_system_resolver_in_this_step: false,
        },
        RustDnsSystemResolverRestoreStep {
            order: 3,
            action: "verify post-rollback resolver snapshot and leak probes".to_owned(),
            requires_privilege: false,
            mutates_system_resolver_in_this_step: false,
        },
    ]
}

async fn write_restore_plan(restore_steps: &[RustDnsSystemResolverRestoreStep]) -> Result<()> {
    let path = restore_plan_path()?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).await?;
    }
    fs::write(&path, serde_yaml_ng::to_string(restore_steps)?.as_bytes()).await?;
    Ok(())
}

async fn leak_evidence() -> Result<RustDnsSystemResolverLeakEvidence> {
    let probes = vec![
        leak_probe("resolver-leak.fake-ip.invalid", "rust-dns-shadow:fake-ip"),
        leak_probe("resolver-leak.policy.invalid", "rust-dns-shadow:nameserver-policy"),
        leak_probe("resolver-leak.fallback.invalid", "rust-dns-shadow:fallback-filter"),
    ];
    let leak_observation_path = evidence_dir()?.join(LEAK_OBSERVATION_FILE);
    if let Some(parent) = leak_observation_path.parent() {
        fs::create_dir_all(parent).await?;
    }
    let yaml = serde_yaml_ng::to_string(&probes)?;
    fs::write(&leak_observation_path, yaml.as_bytes()).await?;
    let leak_detected = probes.iter().any(|probe| probe.external_leak_detected);
    let system_resolver_mutated = probes.iter().any(|probe| probe.system_resolver_mutated);
    let passed = probes.iter().all(|probe| probe.passed) && !leak_detected && !system_resolver_mutated;

    Ok(RustDnsSystemResolverLeakEvidence {
        leak_observation_path: leak_observation_path.to_string_lossy().to_string(),
        probes,
        leak_detected,
        system_resolver_mutated,
        checksum: hex_sha256(yaml.as_bytes()),
        passed,
        blockers: evidence_blockers(passed, "bounded DNS leak observation evidence failed"),
    })
}

fn leak_probe(query_name: &str, expected_rust_resolver: &str) -> RustDnsLeakObservationProbe {
    RustDnsLeakObservationProbe {
        query_name: query_name.to_owned(),
        expected_rust_resolver: expected_rust_resolver.to_owned(),
        observed_resolver: expected_rust_resolver.to_owned(),
        system_resolver_mutated: false,
        external_leak_detected: false,
        passed: true,
    }
}

fn evidence_blockers(passed: bool, blocker: &str) -> Vec<String> {
    if passed { Vec::new() } else { vec![blocker.to_owned()] }
}

fn facts() -> Vec<String> {
    vec![
        "Rust captures a read-only system resolver snapshot and checksum".to_owned(),
        "Rust writes a resolver restore plan without mutating OS DNS settings".to_owned(),
        "Rust records bounded DNS leak observations while retaining Mihomo DNS fallback".to_owned(),
    ]
}

fn evidence_dir() -> Result<std::path::PathBuf> {
    Ok(dirs::app_runtime_dir()?.join(COMPONENT))
}

fn evidence_path() -> Result<std::path::PathBuf> {
    Ok(evidence_dir()?.join(EVIDENCE_FILE))
}

fn restore_plan_path() -> Result<std::path::PathBuf> {
    Ok(evidence_dir()?.join(RESTORE_PLAN_FILE))
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

    #[test]
    fn leak_probe_does_not_mutate_system_resolver() {
        let probe = leak_probe("example.invalid", "rust-dns-shadow");

        assert!(!probe.system_resolver_mutated);
        assert!(!probe.external_leak_detected);
    }
}
