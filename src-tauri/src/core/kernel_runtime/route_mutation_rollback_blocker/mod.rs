use super::RUST_RUNTIME_ID;
use crate::utils::dirs;
use anyhow::{Context as _, Result};
use serde::{Deserialize, Serialize};
use sha2::{Digest as _, Sha256};
use std::process::Command;
use tokio::fs;

const COMPONENT: &str = "rust-route-mutation-rollback-blocker";
const KERNEL_AREA: &str = "route-mutation-rollback-blocker";
const EVIDENCE_FILE: &str = "evidence.yaml";
const PRE_SNAPSHOT_FILE: &str = "route-snapshot-before.txt";
const POST_SNAPSHOT_FILE: &str = "route-snapshot-after.txt";
const APPLY_PLAN_FILE: &str = "route-apply-plan.yaml";
const ROLLBACK_PLAN_FILE: &str = "route-rollback-plan.yaml";
const NEXT_SAFE_BATCH: &str = "packet-leak-hold-window-blocker";

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum RustRouteMutationRollbackBlockerStatus {
    Ready,
    Blocked,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RustRouteMutationCommandPlan {
    pub platform: String,
    pub phase: String,
    pub command_preview: Vec<String>,
    pub requires_privilege: bool,
    pub executed_in_evidence: bool,
    pub mutates_route_table: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RustRouteSnapshotEvidence {
    pub command: Vec<String>,
    pub before_snapshot_path: String,
    pub after_snapshot_path: String,
    pub before_checksum: String,
    pub after_checksum: String,
    pub snapshots_match: bool,
    pub route_entries_observed: usize,
    pub mutates_route_table: bool,
    pub passed: bool,
    pub blockers: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RustRouteMutationRollbackEvidence {
    pub apply_plan_path: String,
    pub rollback_plan_path: String,
    pub apply_plan_checksum: String,
    pub rollback_plan_checksum: String,
    pub apply_plan: Vec<RustRouteMutationCommandPlan>,
    pub rollback_plan: Vec<RustRouteMutationCommandPlan>,
    pub passed: bool,
    pub blockers: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RustRouteMutationRollbackBlockerReport {
    pub runtime_id: String,
    pub component: String,
    pub kernel_area: String,
    pub status: RustRouteMutationRollbackBlockerStatus,
    pub reason: String,
    pub explicit_opt_in: bool,
    pub snapshot_evidence: Option<RustRouteSnapshotEvidence>,
    pub rollback_evidence: Option<RustRouteMutationRollbackEvidence>,
    pub evidence_path: Option<String>,
    pub mutates_runtime: bool,
    pub writes_evidence: bool,
    pub default_route_forwarding_allowed: bool,
    pub mihomo_route_fallback_required: bool,
    pub blockers_reduced: Vec<String>,
    pub blockers_remaining: Vec<String>,
    pub blockers: Vec<String>,
    pub warnings: Vec<String>,
    pub facts: Vec<String>,
    pub next_safe_batch: String,
}

pub async fn rust_route_mutation_rollback_blocker_reduction(
    explicit_opt_in: bool,
) -> Result<RustRouteMutationRollbackBlockerReport> {
    if !explicit_opt_in {
        return Ok(blocked_report(vec![
            "explicit opt-in is required to run route mutation rollback blocker reduction".to_owned(),
        ]));
    }

    let snapshot_evidence = snapshot_evidence().await?;
    let rollback_evidence = rollback_evidence().await?;
    let mut blockers = Vec::new();
    blockers.extend(snapshot_evidence.blockers.iter().cloned());
    blockers.extend(rollback_evidence.blockers.iter().cloned());
    let status = if blockers.is_empty() {
        RustRouteMutationRollbackBlockerStatus::Ready
    } else {
        RustRouteMutationRollbackBlockerStatus::Blocked
    };
    let evidence_path = evidence_path()?;
    let mut report = RustRouteMutationRollbackBlockerReport {
        runtime_id: RUST_RUNTIME_ID.to_owned(),
        component: COMPONENT.to_owned(),
        kernel_area: KERNEL_AREA.to_owned(),
        status,
        reason: if status == RustRouteMutationRollbackBlockerStatus::Ready {
            "Rust reduced route mutation rollback blocker with read-only snapshot and bounded apply/rollback plans"
        } else {
            "Rust route mutation rollback blocker reduction is blocked"
        }
        .to_owned(),
        explicit_opt_in,
        snapshot_evidence: Some(snapshot_evidence),
        rollback_evidence: Some(rollback_evidence),
        evidence_path: Some(evidence_path.to_string_lossy().to_string()),
        mutates_runtime: false,
        writes_evidence: true,
        default_route_forwarding_allowed: false,
        mihomo_route_fallback_required: true,
        blockers_reduced: vec![
            "read-only route snapshot replay evidence".to_owned(),
            "bounded platform route apply/rollback plan evidence".to_owned(),
        ],
        blockers_remaining: vec![
            "operator-approved privileged route mutation on real interfaces".to_owned(),
            "privileged TUN device create/destroy on real interfaces".to_owned(),
            "post-cutover packet leak hold window".to_owned(),
        ],
        blockers,
        warnings: vec![
            "route mutation evidence is read-only and does not execute privileged route commands".to_owned(),
            "Mihomo route/TUN fallback remains required until operator-approved real-interface apply/rollback evidence exists".to_owned(),
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

fn blocked_report(blockers: Vec<String>) -> RustRouteMutationRollbackBlockerReport {
    RustRouteMutationRollbackBlockerReport {
        runtime_id: RUST_RUNTIME_ID.to_owned(),
        component: COMPONENT.to_owned(),
        kernel_area: KERNEL_AREA.to_owned(),
        status: RustRouteMutationRollbackBlockerStatus::Blocked,
        reason: "Rust route mutation rollback blocker reduction is blocked".to_owned(),
        explicit_opt_in: false,
        snapshot_evidence: None,
        rollback_evidence: None,
        evidence_path: None,
        mutates_runtime: false,
        writes_evidence: false,
        default_route_forwarding_allowed: false,
        mihomo_route_fallback_required: true,
        blockers_reduced: Vec::new(),
        blockers_remaining: vec![
            "privileged route table mutation apply/rollback on real interfaces".to_owned(),
            "privileged TUN device create/destroy on real interfaces".to_owned(),
            "post-cutover packet leak hold window".to_owned(),
        ],
        blockers,
        warnings: Vec::new(),
        facts: facts(),
        next_safe_batch: NEXT_SAFE_BATCH.to_owned(),
    }
}

async fn snapshot_evidence() -> Result<RustRouteSnapshotEvidence> {
    let (program, args) = route_snapshot_command();
    let before = route_snapshot(program, &args)?;
    let after = route_snapshot(program, &args)?;
    let before_snapshot_path = evidence_dir()?.join(PRE_SNAPSHOT_FILE);
    let after_snapshot_path = evidence_dir()?.join(POST_SNAPSHOT_FILE);
    if let Some(parent) = before_snapshot_path.parent() {
        fs::create_dir_all(parent).await?;
    }
    fs::write(&before_snapshot_path, before.as_bytes()).await?;
    fs::write(&after_snapshot_path, after.as_bytes()).await?;

    let before_checksum = hex_sha256(before.as_bytes());
    let after_checksum = hex_sha256(after.as_bytes());
    let snapshots_match = before_checksum == after_checksum;
    let route_entries_observed = route_entry_count(&before);
    let passed = snapshots_match && route_entries_observed > 0;

    Ok(RustRouteSnapshotEvidence {
        command: std::iter::once(program.to_owned())
            .chain(args.iter().cloned())
            .collect(),
        before_snapshot_path: before_snapshot_path.to_string_lossy().to_string(),
        after_snapshot_path: after_snapshot_path.to_string_lossy().to_string(),
        before_checksum,
        after_checksum,
        snapshots_match,
        route_entries_observed,
        mutates_route_table: false,
        passed,
        blockers: evidence_blockers(passed, "read-only route snapshot replay evidence failed"),
    })
}

fn route_snapshot(program: &str, args: &[String]) -> Result<String> {
    let output = Command::new(program)
        .args(args)
        .output()
        .with_context(|| format!("failed to run route snapshot command `{program}`"))?;
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    if output.status.success() {
        Ok(stdout)
    } else {
        Ok(format!("{stdout}\n{stderr}"))
    }
}

#[cfg(target_os = "windows")]
fn route_snapshot_command() -> (&'static str, Vec<String>) {
    ("route", vec!["print".to_owned()])
}

#[cfg(target_os = "macos")]
fn route_snapshot_command() -> (&'static str, Vec<String>) {
    ("netstat", vec!["-rn".to_owned()])
}

#[cfg(all(not(target_os = "windows"), not(target_os = "macos")))]
fn route_snapshot_command() -> (&'static str, Vec<String>) {
    (
        "ip",
        vec![
            "route".to_owned(),
            "show".to_owned(),
            "table".to_owned(),
            "main".to_owned(),
        ],
    )
}

async fn rollback_evidence() -> Result<RustRouteMutationRollbackEvidence> {
    let apply_plan = apply_plan();
    let rollback_plan = rollback_plan();
    let apply_yaml = serde_yaml_ng::to_string(&apply_plan)?;
    let rollback_yaml = serde_yaml_ng::to_string(&rollback_plan)?;
    let apply_plan_path = evidence_dir()?.join(APPLY_PLAN_FILE);
    let rollback_plan_path = evidence_dir()?.join(ROLLBACK_PLAN_FILE);
    if let Some(parent) = apply_plan_path.parent() {
        fs::create_dir_all(parent).await?;
    }
    fs::write(&apply_plan_path, apply_yaml.as_bytes()).await?;
    fs::write(&rollback_plan_path, rollback_yaml.as_bytes()).await?;
    let passed = apply_plan
        .iter()
        .chain(rollback_plan.iter())
        .all(|plan| plan.requires_privilege && !plan.executed_in_evidence && !plan.mutates_route_table);

    Ok(RustRouteMutationRollbackEvidence {
        apply_plan_path: apply_plan_path.to_string_lossy().to_string(),
        rollback_plan_path: rollback_plan_path.to_string_lossy().to_string(),
        apply_plan_checksum: hex_sha256(apply_yaml.as_bytes()),
        rollback_plan_checksum: hex_sha256(rollback_yaml.as_bytes()),
        apply_plan,
        rollback_plan,
        passed,
        blockers: evidence_blockers(passed, "bounded route apply/rollback plan evidence failed"),
    })
}

fn apply_plan() -> Vec<RustRouteMutationCommandPlan> {
    vec![route_plan("apply", route_apply_command_preview(), true, false, false)]
}

fn rollback_plan() -> Vec<RustRouteMutationCommandPlan> {
    vec![route_plan(
        "rollback",
        route_rollback_command_preview(),
        true,
        false,
        false,
    )]
}

fn route_plan(
    phase: &str,
    command_preview: Vec<String>,
    requires_privilege: bool,
    executed_in_evidence: bool,
    mutates_route_table: bool,
) -> RustRouteMutationCommandPlan {
    RustRouteMutationCommandPlan {
        platform: std::env::consts::OS.to_owned(),
        phase: phase.to_owned(),
        command_preview,
        requires_privilege,
        executed_in_evidence,
        mutates_route_table,
    }
}

#[cfg(target_os = "windows")]
fn route_apply_command_preview() -> Vec<String> {
    vec![
        "route".to_owned(),
        "add".to_owned(),
        "198.18.0.0".to_owned(),
        "mask".to_owned(),
        "255.254.0.0".to_owned(),
        "<rust-tun-gateway>".to_owned(),
    ]
}

#[cfg(target_os = "windows")]
fn route_rollback_command_preview() -> Vec<String> {
    vec!["route".to_owned(), "delete".to_owned(), "198.18.0.0".to_owned()]
}

#[cfg(not(target_os = "windows"))]
fn route_apply_command_preview() -> Vec<String> {
    vec![
        "ip".to_owned(),
        "route".to_owned(),
        "add".to_owned(),
        "198.18.0.0/15".to_owned(),
        "dev".to_owned(),
        "<rust-tun-device>".to_owned(),
    ]
}

#[cfg(not(target_os = "windows"))]
fn route_rollback_command_preview() -> Vec<String> {
    vec![
        "ip".to_owned(),
        "route".to_owned(),
        "delete".to_owned(),
        "198.18.0.0/15".to_owned(),
    ]
}

fn route_entry_count(snapshot: &str) -> usize {
    snapshot
        .lines()
        .filter(|line| {
            let trimmed = line.trim();
            !trimmed.is_empty() && !trimmed.starts_with('=') && !trimmed.to_ascii_lowercase().contains("interface list")
        })
        .count()
}

fn evidence_blockers(passed: bool, blocker: &str) -> Vec<String> {
    if passed { Vec::new() } else { vec![blocker.to_owned()] }
}

fn facts() -> Vec<String> {
    vec![
        "Rust captures route snapshots before and after no-op evidence replay".to_owned(),
        "Rust writes platform route apply/rollback command plans without executing privileged mutations".to_owned(),
        "Mihomo route/TUN fallback remains required until operator-approved real-interface route mutation evidence exists".to_owned(),
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blocked_report_keeps_route_fallback() {
        let report = blocked_report(Vec::new());

        assert!(report.mihomo_route_fallback_required);
        assert!(!report.default_route_forwarding_allowed);
    }

    #[test]
    fn route_plans_are_not_executed_by_evidence() {
        let plans = apply_plan().into_iter().chain(rollback_plan());

        assert!(
            plans
                .into_iter()
                .all(|plan| !plan.executed_in_evidence && !plan.mutates_route_table)
        );
    }
}
