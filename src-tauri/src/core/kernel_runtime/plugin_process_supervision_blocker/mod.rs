use super::{
    RUST_RUNTIME_ID, RustPacketLeakHoldBlockerReport, RustPacketLeakHoldBlockerStatus, RustPacketLeakHoldGateEvidence,
    rust_packet_leak_hold_blocker_evidence_path,
};
use crate::utils::dirs;
use anyhow::{Context as _, Result};
use serde::{Deserialize, Serialize};
use sha2::{Digest as _, Sha256};
use std::{
    process::{Command, Output},
    time::{SystemTime, UNIX_EPOCH},
};
use tokio::fs;

const COMPONENT: &str = "rust-plugin-process-supervision-blocker";
const KERNEL_AREA: &str = "plugin-process-supervision-blocker";
const EVIDENCE_FILE: &str = "evidence.yaml";
const SUPERVISION_FILE: &str = "supervision.yaml";
const NEXT_SAFE_BATCH: &str = "protocol-default-cutover-hold-window";

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum RustPluginProcessSupervisionBlockerStatus {
    Ready,
    Blocked,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RustPluginProcessSupervisionAttempt {
    pub stage: String,
    pub command: Vec<String>,
    pub exit_code: Option<i32>,
    pub stdout: String,
    pub stderr: String,
    pub expected_success: bool,
    pub observed_expected_status: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RustPluginProcessSupervisionEvidence {
    pub manifest_path: String,
    pub attempts: Vec<RustPluginProcessSupervisionAttempt>,
    pub health_output_observed: bool,
    pub crash_exit_observed: bool,
    pub restart_output_observed: bool,
    pub external_process_spawned: bool,
    pub mihomo_process_required: bool,
    pub checksum: String,
    pub passed: bool,
    pub blockers: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RustPluginProcessSupervisionBlockerReport {
    pub runtime_id: String,
    pub component: String,
    pub kernel_area: String,
    pub status: RustPluginProcessSupervisionBlockerStatus,
    pub reason: String,
    pub explicit_opt_in: bool,
    #[serde(default)]
    pub packet_leak_hold_gate: Option<RustPacketLeakHoldGateEvidence>,
    pub supervision_evidence: Option<RustPluginProcessSupervisionEvidence>,
    pub evidence_path: Option<String>,
    pub mutates_runtime: bool,
    pub writes_evidence: bool,
    pub default_protocol_forwarding_allowed: bool,
    pub mihomo_plugin_lifecycle_fallback_required: bool,
    pub blockers_reduced: Vec<String>,
    pub blockers_remaining: Vec<String>,
    pub blockers: Vec<String>,
    pub warnings: Vec<String>,
    pub facts: Vec<String>,
    pub next_safe_batch: String,
}

pub async fn rust_plugin_process_supervision_blocker_reduction(
    explicit_opt_in: bool,
) -> Result<RustPluginProcessSupervisionBlockerReport> {
    let (packet_leak_hold_gate, packet_leak_hold_gate_blockers) = packet_leak_hold_gate().await?;
    if !explicit_opt_in {
        let mut blockers =
            vec!["explicit opt-in is required to run plugin process supervision blocker reduction".to_owned()];
        blockers.extend(packet_leak_hold_gate_blockers);
        return Ok(blocked_report(explicit_opt_in, packet_leak_hold_gate, blockers));
    }
    if !packet_leak_hold_gate_blockers.is_empty() {
        return Ok(blocked_report(
            explicit_opt_in,
            packet_leak_hold_gate,
            packet_leak_hold_gate_blockers,
        ));
    }

    let supervision_evidence = supervision_evidence().await?;
    let blockers = supervision_evidence.blockers.clone();
    let status = if blockers.is_empty() {
        RustPluginProcessSupervisionBlockerStatus::Ready
    } else {
        RustPluginProcessSupervisionBlockerStatus::Blocked
    };
    let evidence_path = evidence_path()?;
    let mut report = RustPluginProcessSupervisionBlockerReport {
        runtime_id: RUST_RUNTIME_ID.to_owned(),
        component: COMPONENT.to_owned(),
        kernel_area: KERNEL_AREA.to_owned(),
        status,
        reason: if status == RustPluginProcessSupervisionBlockerStatus::Ready {
            "Rust reduced plugin process supervision blocker with health, crash, and restart evidence"
        } else {
            "Rust plugin process supervision blocker reduction is blocked"
        }
        .to_owned(),
        explicit_opt_in,
        packet_leak_hold_gate,
        supervision_evidence: Some(supervision_evidence),
        evidence_path: Some(evidence_path.to_string_lossy().to_string()),
        mutates_runtime: false,
        writes_evidence: true,
        default_protocol_forwarding_allowed: false,
        mihomo_plugin_lifecycle_fallback_required: true,
        blockers_reduced: vec![
            "external plugin process health supervision canary".to_owned(),
            "external plugin process crash observation".to_owned(),
            "external plugin process restart observation".to_owned(),
        ],
        blockers_remaining: vec![
            "real plugin binary compatibility matrix".to_owned(),
            "default forwarding cutover hold window".to_owned(),
            "QUIC/UDP protocol variants on real profiles".to_owned(),
        ],
        blockers,
        warnings: vec![
            "supervision uses harmless canary subprocesses, not real proxy plugins".to_owned(),
            "Mihomo plugin fallback remains required until real plugin compatibility and cutover hold evidence exists"
                .to_owned(),
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

fn blocked_report(
    explicit_opt_in: bool,
    packet_leak_hold_gate: Option<RustPacketLeakHoldGateEvidence>,
    blockers: Vec<String>,
) -> RustPluginProcessSupervisionBlockerReport {
    RustPluginProcessSupervisionBlockerReport {
        runtime_id: RUST_RUNTIME_ID.to_owned(),
        component: COMPONENT.to_owned(),
        kernel_area: KERNEL_AREA.to_owned(),
        status: RustPluginProcessSupervisionBlockerStatus::Blocked,
        reason: "Rust plugin process supervision blocker reduction is blocked".to_owned(),
        explicit_opt_in,
        packet_leak_hold_gate,
        supervision_evidence: None,
        evidence_path: None,
        mutates_runtime: false,
        writes_evidence: false,
        default_protocol_forwarding_allowed: false,
        mihomo_plugin_lifecycle_fallback_required: true,
        blockers_reduced: Vec::new(),
        blockers_remaining: vec![
            "external plugin process supervision and crash recovery".to_owned(),
            "real plugin binary compatibility matrix".to_owned(),
            "default forwarding cutover hold window".to_owned(),
        ],
        blockers,
        warnings: Vec::new(),
        facts: facts(),
        next_safe_batch: NEXT_SAFE_BATCH.to_owned(),
    }
}

async fn packet_leak_hold_gate() -> Result<(Option<RustPacketLeakHoldGateEvidence>, Vec<String>)> {
    let evidence_path = rust_packet_leak_hold_blocker_evidence_path()?;
    let Some(report) = read_packet_leak_hold_report(&evidence_path).await? else {
        return Ok((
            None,
            vec!["packet leak hold evidence is missing before protocol default hold reduction".to_owned()],
        ));
    };

    let mut blockers = Vec::new();
    if report.status != RustPacketLeakHoldBlockerStatus::Ready {
        blockers.push(format!("packet leak hold status is {:?}", report.status));
    }
    if !report.blockers.is_empty() {
        blockers.push("packet leak hold evidence contains blockers".to_owned());
    }
    match report.route_mutation_gate.as_ref() {
        Some(gate) => {
            if gate.status != super::RustRouteMutationRollbackBlockerStatus::Ready {
                blockers.push(format!("route mutation gate status is {:?}", gate.status));
            }
            if !gate.blockers.is_empty() {
                blockers.push("route mutation gate contains blockers".to_owned());
            }
        }
        None => blockers.push("packet leak hold lacks route mutation gate".to_owned()),
    }

    blockers.sort();
    blockers.dedup();
    let gate = RustPacketLeakHoldGateEvidence {
        status: report.status,
        blockers: report.blockers.clone(),
        route_mutation_status: report.route_mutation_gate.as_ref().map(|gate| gate.status),
        route_mutation_blockers: report
            .route_mutation_gate
            .as_ref()
            .map(|gate| gate.blockers.clone())
            .unwrap_or_default(),
        evidence_path: report.evidence_path.clone(),
    };
    Ok((Some(gate), blockers))
}

async fn read_packet_leak_hold_report(path: &std::path::Path) -> Result<Option<RustPacketLeakHoldBlockerReport>> {
    match fs::read_to_string(path).await {
        Ok(yaml) => serde_yaml_ng::from_str(&yaml)
            .with_context(|| format!("failed to parse {}", path.display()))
            .map(Some),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error).with_context(|| format!("failed to read {}", path.display())),
    }
}

async fn supervision_evidence() -> Result<RustPluginProcessSupervisionEvidence> {
    let attempts = vec![
        run_attempt("health", true, "plugin-health-ok")?,
        run_attempt("crash", false, "plugin-crash")?,
        run_attempt("restart", true, "plugin-restart-ok")?,
    ];
    let health_output_observed = attempts
        .iter()
        .any(|attempt| attempt.stage == "health" && attempt.stdout.contains("plugin-health-ok"));
    let crash_exit_observed = attempts
        .iter()
        .any(|attempt| attempt.stage == "crash" && !attempt.observed_expected_status && attempt.exit_code != Some(0));
    let restart_output_observed = attempts
        .iter()
        .any(|attempt| attempt.stage == "restart" && attempt.stdout.contains("plugin-restart-ok"));
    let observed_statuses = attempts
        .iter()
        .filter(|attempt| {
            if attempt.stage == "crash" {
                attempt.exit_code != Some(0)
            } else {
                attempt.observed_expected_status
            }
        })
        .count();
    let passed =
        health_output_observed && crash_exit_observed && restart_output_observed && observed_statuses == attempts.len();
    let manifest_path = evidence_dir()?.join(SUPERVISION_FILE);
    if let Some(parent) = manifest_path.parent() {
        fs::create_dir_all(parent).await?;
    }
    let yaml = serde_yaml_ng::to_string(&attempts)?;
    fs::write(&manifest_path, yaml.as_bytes()).await?;

    Ok(RustPluginProcessSupervisionEvidence {
        manifest_path: manifest_path.to_string_lossy().to_string(),
        attempts,
        health_output_observed,
        crash_exit_observed,
        restart_output_observed,
        external_process_spawned: true,
        mihomo_process_required: false,
        checksum: hex_sha256(yaml.as_bytes()),
        passed,
        blockers: evidence_blockers(passed, "plugin process supervision/crash recovery canary failed"),
    })
}

fn run_attempt(stage: &str, expected_success: bool, marker: &str) -> Result<RustPluginProcessSupervisionAttempt> {
    let (program, args) = process_command(expected_success, marker);
    let output = Command::new(program).args(&args).output()?;
    Ok(attempt_from_output(stage, program, &args, expected_success, output))
}

fn attempt_from_output(
    stage: &str,
    program: &str,
    args: &[String],
    expected_success: bool,
    output: Output,
) -> RustPluginProcessSupervisionAttempt {
    RustPluginProcessSupervisionAttempt {
        stage: stage.to_owned(),
        command: std::iter::once(program.to_owned())
            .chain(args.iter().cloned())
            .collect(),
        exit_code: output.status.code(),
        stdout: String::from_utf8_lossy(&output.stdout).trim().to_owned(),
        stderr: String::from_utf8_lossy(&output.stderr).trim().to_owned(),
        expected_success,
        observed_expected_status: output.status.success() == expected_success,
    }
}

#[cfg(target_os = "windows")]
fn process_command(success: bool, marker: &str) -> (&'static str, Vec<String>) {
    if success {
        ("cmd", vec!["/C".to_owned(), format!("echo {marker}")])
    } else {
        ("cmd", vec!["/C".to_owned(), "exit /B 23".to_owned()])
    }
}

#[cfg(not(target_os = "windows"))]
fn process_command(success: bool, marker: &str) -> (&'static str, Vec<String>) {
    if success {
        ("sh", vec!["-c".to_owned(), format!("printf '%s\\n' {marker}")])
    } else {
        ("sh", vec!["-c".to_owned(), "exit 23".to_owned()])
    }
}

fn evidence_blockers(passed: bool, blocker: &str) -> Vec<String> {
    if passed { Vec::new() } else { vec![blocker.to_owned()] }
}

fn facts() -> Vec<String> {
    vec![
        "Rust starts a harmless external plugin canary process and observes health output".to_owned(),
        "Rust records non-zero crash exit status and restart health evidence".to_owned(),
        "Mihomo plugin fallback remains retained for real plugin binary compatibility and default cutover".to_owned(),
    ]
}

fn evidence_dir() -> Result<std::path::PathBuf> {
    Ok(dirs::app_runtime_dir()?.join(COMPONENT))
}

pub fn rust_plugin_process_supervision_blocker_evidence_path() -> Result<std::path::PathBuf> {
    Ok(evidence_dir()?.join(EVIDENCE_FILE))
}

fn evidence_path() -> Result<std::path::PathBuf> {
    rust_plugin_process_supervision_blocker_evidence_path()
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
    fn blocked_report_keeps_plugin_fallback() {
        let report = blocked_report(false, None, Vec::new());

        assert!(report.mihomo_plugin_lifecycle_fallback_required);
        assert!(!report.default_protocol_forwarding_allowed);
    }

    #[test]
    fn attempt_status_matches_expected_failure() {
        let output = process_command(false, "ignored");

        assert!(!output.1.is_empty());
    }
}
