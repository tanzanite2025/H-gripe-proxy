use super::{
    MihomoFallbackRetirementExecutionReport, RUST_RUNTIME_ID, execute_mihomo_fallback_retirement,
    rust_tun_packet_capture_hold_bundle_execution, rust_udp_plugin_transport_bundle_execution,
};
use crate::utils::dirs;
use anyhow::{Context as _, Result};
use serde::{Deserialize, Serialize};
use std::{
    fs as std_fs,
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};
use tokio::fs;

const COMPONENT: &str = "rust-mihomo-fallback-retirement-bundle";
const KERNEL_AREA: &str = "fallback-retirement-bundle";
const EVIDENCE_FILE: &str = "evidence.yaml";
const ROLLBACK_FILE: &str = "rollback-checkpoint.yaml";
const RUST_OWNED_SCOPE: &str = "bounded supported fallback retirement with unsupported fallback continuity, emergency rollback, hold telemetry, and sidecar audit evidence";
const NEXT_SAFE_BATCH: &str = "go-to-rust-migration-final-review";

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum RustMihomoFallbackRetirementBundleStatus {
    Passed,
    Failed,
    Blocked,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RustMihomoFallbackRetirementUnsupportedPathEvidence {
    pub path: String,
    pub mihomo_fallback_retained: bool,
    pub rust_retirement_bypassed: bool,
    pub emergency_rollback_available: bool,
    pub passed: bool,
    pub blockers: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RustMihomoFallbackRetirementUnsupportedFallbackEvidence {
    pub unsupported_paths: Vec<RustMihomoFallbackRetirementUnsupportedPathEvidence>,
    pub fallback_continuity_without_app_restart: bool,
    pub unsupported_mihomo_fallback_retained: bool,
    pub passed: bool,
    pub blockers: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RustMihomoFallbackRetirementEmergencyRollbackEvidence {
    pub checkpoint_path: Option<String>,
    pub bundle_checkpoint_path: String,
    pub checkpoint_written: bool,
    pub retained_fallback_scope: Vec<String>,
    pub manifest_restoration_supported: bool,
    pub rollback_without_app_restart: bool,
    pub passed: bool,
    pub blockers: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RustMihomoFallbackRetirementHoldTelemetryEvidence {
    pub udp_plugin_status: String,
    pub tun_packet_capture_status: String,
    pub fallback_retirement_status: String,
    pub udp_plugin_evidence_path: Option<String>,
    pub tun_packet_capture_evidence_path: Option<String>,
    pub fallback_execution_manifest_path: Option<String>,
    pub hold_telemetry_archived: bool,
    pub passed: bool,
    pub blockers: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RustMihomoFallbackRetirementSidecarDependencyAuditEvidence {
    pub mihomo_source_dir: String,
    pub mihomo_source_present: bool,
    pub sidecar_dir: String,
    pub sidecar_binary_count: usize,
    pub build_script_path: String,
    pub build_script_present: bool,
    pub sidecar_binary_removal_requested: bool,
    pub sidecar_binary_removal_allowed: bool,
    pub passed: bool,
    pub blockers: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RustMihomoFallbackRetirementSelectiveScopeEvidence {
    pub scope: String,
    pub fallback_retired_for_scope: bool,
    pub evidence: Vec<String>,
    pub retained_fallback_count: usize,
    pub passed: bool,
    pub blockers: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RustMihomoFallbackRetirementSelectiveDeprecationEvidence {
    pub supported_scopes: Vec<RustMihomoFallbackRetirementSelectiveScopeEvidence>,
    pub retires_supported_fallback: bool,
    pub removes_mihomo_fallback_binary: bool,
    pub unsupported_fallback_retained: bool,
    pub passed: bool,
    pub blockers: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RustMihomoFallbackRetirementBundleReport {
    pub runtime_id: String,
    pub component: String,
    pub kernel_area: String,
    pub status: RustMihomoFallbackRetirementBundleStatus,
    pub reason: String,
    pub explicit_opt_in: bool,
    pub rust_owned_scope: String,
    pub unsupported_fallback_evidence: Option<RustMihomoFallbackRetirementUnsupportedFallbackEvidence>,
    pub emergency_rollback_evidence: Option<RustMihomoFallbackRetirementEmergencyRollbackEvidence>,
    pub hold_telemetry_evidence: Option<RustMihomoFallbackRetirementHoldTelemetryEvidence>,
    pub sidecar_dependency_audit: Option<RustMihomoFallbackRetirementSidecarDependencyAuditEvidence>,
    pub selective_deprecation_evidence: Option<RustMihomoFallbackRetirementSelectiveDeprecationEvidence>,
    pub fallback_execution_report: Option<MihomoFallbackRetirementExecutionReport>,
    pub evidence_path: Option<String>,
    pub mutates_runtime: bool,
    pub writes_evidence: bool,
    pub retires_supported_fallback: bool,
    pub removes_mihomo_fallback_binary: bool,
    pub unsupported_mihomo_fallback_retained: bool,
    pub blockers: Vec<String>,
    pub warnings: Vec<String>,
    pub facts: Vec<String>,
    pub next_safe_batch: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RustMihomoFallbackRetirementBundleCheckpoint {
    component: String,
    rust_owned_scope: String,
    fallback_execution_manifest_path: Option<String>,
    retained_fallback_scope: Vec<String>,
    created_at_epoch_seconds: u64,
}

pub async fn rust_mihomo_fallback_retirement_bundle_execution(
    explicit_opt_in: bool,
) -> Result<RustMihomoFallbackRetirementBundleReport> {
    if !explicit_opt_in {
        return Ok(blocked_report(vec![
            "explicit opt-in is required to run Mihomo fallback retirement bundle".to_owned(),
        ]));
    }

    let udp_plugin_report = rust_udp_plugin_transport_bundle_execution(true).await?;
    let tun_packet_capture_report = rust_tun_packet_capture_hold_bundle_execution(true).await?;
    let fallback_execution_report = execute_mihomo_fallback_retirement(true, true).await?;
    let unsupported_fallback_evidence = unsupported_fallback_evidence(&fallback_execution_report);
    let rollback_path = rollback_path()?;
    let emergency_rollback_evidence = emergency_rollback_evidence(&fallback_execution_report, &rollback_path).await?;
    let hold_telemetry_evidence = hold_telemetry_evidence(
        &udp_plugin_report.status_string(),
        &tun_packet_capture_report.status_string(),
        &fallback_execution_report.status_string(),
        udp_plugin_report.evidence_path.clone(),
        tun_packet_capture_report.evidence_path.clone(),
        fallback_execution_report
            .execution_manifest_path
            .clone()
            .map(|path| path.to_string()),
    );
    let sidecar_dependency_audit = sidecar_dependency_audit(&fallback_execution_report).await?;
    let selective_deprecation_evidence = selective_deprecation_evidence(&fallback_execution_report);

    let mut blockers = Vec::new();
    blockers.extend(unsupported_fallback_evidence.blockers.iter().cloned());
    blockers.extend(emergency_rollback_evidence.blockers.iter().cloned());
    blockers.extend(hold_telemetry_evidence.blockers.iter().cloned());
    blockers.extend(sidecar_dependency_audit.blockers.iter().cloned());
    blockers.extend(selective_deprecation_evidence.blockers.iter().cloned());
    blockers.extend(
        fallback_execution_report
            .blockers
            .iter()
            .map(|blocker| blocker.to_string()),
    );

    let status = if blockers.is_empty() {
        RustMihomoFallbackRetirementBundleStatus::Passed
    } else {
        RustMihomoFallbackRetirementBundleStatus::Failed
    };
    let reason = if status == RustMihomoFallbackRetirementBundleStatus::Passed {
        "Rust completed bounded Mihomo fallback retirement for supported scopes while retaining unsupported fallback"
    } else {
        "Rust Mihomo fallback retirement bundle evidence failed"
    };
    let evidence_path = evidence_path()?;
    let mut report = RustMihomoFallbackRetirementBundleReport {
        runtime_id: RUST_RUNTIME_ID.to_owned(),
        component: COMPONENT.to_owned(),
        kernel_area: KERNEL_AREA.to_owned(),
        status,
        reason: reason.to_owned(),
        explicit_opt_in,
        rust_owned_scope: RUST_OWNED_SCOPE.to_owned(),
        unsupported_fallback_evidence: Some(unsupported_fallback_evidence),
        emergency_rollback_evidence: Some(emergency_rollback_evidence),
        hold_telemetry_evidence: Some(hold_telemetry_evidence),
        sidecar_dependency_audit: Some(sidecar_dependency_audit),
        selective_deprecation_evidence: Some(selective_deprecation_evidence),
        fallback_execution_report: Some(fallback_execution_report.clone()),
        evidence_path: Some(evidence_path.to_string_lossy().to_string()),
        mutates_runtime: fallback_execution_report.mutates_runtime,
        writes_evidence: true,
        retires_supported_fallback: fallback_execution_report.retires_supported_fallback,
        removes_mihomo_fallback_binary: fallback_execution_report.removes_mihomo_fallback_binary,
        unsupported_mihomo_fallback_retained: fallback_execution_report.unsupported_mihomo_fallback_retained,
        blockers,
        warnings: vec![
            "retirement is limited to supported scopes with bounded Rust execution evidence".to_owned(),
            "Mihomo source, sidecar build script, and sidecar binary fallback remain retained for unsupported paths"
                .to_owned(),
        ],
        facts: facts(),
        next_safe_batch: NEXT_SAFE_BATCH.to_owned(),
    };

    if let Some(parent) = evidence_path.parent() {
        fs::create_dir_all(parent).await?;
    }
    report.evidence_path = Some(evidence_path.to_string_lossy().to_string());
    fs::write(&evidence_path, serde_yaml_ng::to_string(&report)?.as_bytes()).await?;

    Ok(report)
}

fn blocked_report(blockers: Vec<String>) -> RustMihomoFallbackRetirementBundleReport {
    RustMihomoFallbackRetirementBundleReport {
        runtime_id: RUST_RUNTIME_ID.to_owned(),
        component: COMPONENT.to_owned(),
        kernel_area: KERNEL_AREA.to_owned(),
        status: RustMihomoFallbackRetirementBundleStatus::Blocked,
        reason: "Rust Mihomo fallback retirement bundle is blocked".to_owned(),
        explicit_opt_in: false,
        rust_owned_scope: RUST_OWNED_SCOPE.to_owned(),
        unsupported_fallback_evidence: None,
        emergency_rollback_evidence: None,
        hold_telemetry_evidence: None,
        sidecar_dependency_audit: None,
        selective_deprecation_evidence: None,
        fallback_execution_report: None,
        evidence_path: None,
        mutates_runtime: false,
        writes_evidence: false,
        retires_supported_fallback: false,
        removes_mihomo_fallback_binary: false,
        unsupported_mihomo_fallback_retained: true,
        blockers,
        warnings: Vec::new(),
        facts: facts(),
        next_safe_batch: NEXT_SAFE_BATCH.to_owned(),
    }
}

fn unsupported_fallback_evidence(
    report: &MihomoFallbackRetirementExecutionReport,
) -> RustMihomoFallbackRetirementUnsupportedFallbackEvidence {
    let unsupported_paths = retained_fallback_scope()
        .into_iter()
        .map(|path| {
            let passed = report.unsupported_mihomo_fallback_retained && !report.removes_mihomo_fallback_binary;
            RustMihomoFallbackRetirementUnsupportedPathEvidence {
                path,
                mihomo_fallback_retained: report.unsupported_mihomo_fallback_retained,
                rust_retirement_bypassed: true,
                emergency_rollback_available: report.emergency_checkpoint.checkpoint_path.is_some(),
                passed,
                blockers: evidence_blockers(passed, "unsupported fallback continuity failed"),
            }
        })
        .collect::<Vec<_>>();
    let fallback_continuity_without_app_restart = unsupported_paths.iter().all(|path| {
        path.mihomo_fallback_retained && path.rust_retirement_bypassed && path.emergency_rollback_available
    });
    let unsupported_mihomo_fallback_retained = report.unsupported_mihomo_fallback_retained;
    let passed = fallback_continuity_without_app_restart && unsupported_mihomo_fallback_retained;
    let mut blockers = unsupported_paths
        .iter()
        .flat_map(|path| path.blockers.iter().cloned())
        .collect::<Vec<_>>();
    blockers.extend(evidence_blockers(
        passed,
        "unsupported Mihomo fallback was not retained through retirement bundle",
    ));

    RustMihomoFallbackRetirementUnsupportedFallbackEvidence {
        unsupported_paths,
        fallback_continuity_without_app_restart,
        unsupported_mihomo_fallback_retained,
        passed: blockers.is_empty(),
        blockers,
    }
}

async fn emergency_rollback_evidence(
    report: &MihomoFallbackRetirementExecutionReport,
    bundle_checkpoint_path: &std::path::Path,
) -> Result<RustMihomoFallbackRetirementEmergencyRollbackEvidence> {
    let checkpoint = RustMihomoFallbackRetirementBundleCheckpoint {
        component: COMPONENT.to_owned(),
        rust_owned_scope: RUST_OWNED_SCOPE.to_owned(),
        fallback_execution_manifest_path: report.execution_manifest_path.clone().map(|path| path.to_string()),
        retained_fallback_scope: report
            .emergency_checkpoint
            .retained_fallback_scope
            .iter()
            .map(|scope| scope.to_string())
            .collect(),
        created_at_epoch_seconds: epoch_seconds(),
    };
    if let Some(parent) = bundle_checkpoint_path.parent() {
        fs::create_dir_all(parent).await?;
    }
    fs::write(
        bundle_checkpoint_path,
        serde_yaml_ng::to_string(&checkpoint)?.as_bytes(),
    )
    .await?;

    let checkpoint_path = report
        .emergency_checkpoint
        .checkpoint_path
        .clone()
        .map(|path| path.to_string());
    let checkpoint_written = checkpoint_path.is_some() && !checkpoint.retained_fallback_scope.is_empty();
    let manifest_restoration_supported = report.execution_manifest_path.is_some();
    let rollback_without_app_restart = checkpoint_written && manifest_restoration_supported;
    let passed = checkpoint_written && manifest_restoration_supported && rollback_without_app_restart;

    Ok(RustMihomoFallbackRetirementEmergencyRollbackEvidence {
        checkpoint_path,
        bundle_checkpoint_path: bundle_checkpoint_path.to_string_lossy().to_string(),
        checkpoint_written,
        retained_fallback_scope: checkpoint.retained_fallback_scope,
        manifest_restoration_supported,
        rollback_without_app_restart,
        passed,
        blockers: evidence_blockers(passed, "emergency rollback checkpoint evidence failed"),
    })
}

fn hold_telemetry_evidence(
    udp_plugin_status: &str,
    tun_packet_capture_status: &str,
    fallback_retirement_status: &str,
    udp_plugin_evidence_path: Option<String>,
    tun_packet_capture_evidence_path: Option<String>,
    fallback_execution_manifest_path: Option<String>,
) -> RustMihomoFallbackRetirementHoldTelemetryEvidence {
    let hold_telemetry_archived = udp_plugin_evidence_path.is_some()
        && tun_packet_capture_evidence_path.is_some()
        && fallback_execution_manifest_path.is_some();
    let passed = udp_plugin_status == "passed"
        && tun_packet_capture_status == "passed"
        && fallback_retirement_status == "executed"
        && hold_telemetry_archived;

    RustMihomoFallbackRetirementHoldTelemetryEvidence {
        udp_plugin_status: udp_plugin_status.to_owned(),
        tun_packet_capture_status: tun_packet_capture_status.to_owned(),
        fallback_retirement_status: fallback_retirement_status.to_owned(),
        udp_plugin_evidence_path,
        tun_packet_capture_evidence_path,
        fallback_execution_manifest_path,
        hold_telemetry_archived,
        passed,
        blockers: evidence_blockers(passed, "fallback retirement hold telemetry evidence failed"),
    }
}

async fn sidecar_dependency_audit(
    report: &MihomoFallbackRetirementExecutionReport,
) -> Result<RustMihomoFallbackRetirementSidecarDependencyAuditEvidence> {
    let repo_root = repo_root()?;
    let mihomo_source_dir = repo_root.join("mihomo");
    let sidecar_dir = repo_root.join("src-tauri").join("sidecar");
    let build_script_path = repo_root.join("scripts").join("build-mihomo-sidecar.mjs");
    let mihomo_source_present = fs::try_exists(&mihomo_source_dir).await?;
    let build_script_present = fs::try_exists(&build_script_path).await?;
    let sidecar_binary_count = sidecar_binary_count(&sidecar_dir);
    let sidecar_binary_removal_requested = report.removes_mihomo_fallback_binary;
    let sidecar_binary_removal_allowed = false;
    let passed = mihomo_source_present
        && build_script_present
        && !sidecar_binary_removal_requested
        && !sidecar_binary_removal_allowed;
    let blockers = evidence_blockers(passed, "sidecar dependency audit failed");

    Ok(RustMihomoFallbackRetirementSidecarDependencyAuditEvidence {
        mihomo_source_dir: mihomo_source_dir.to_string_lossy().to_string(),
        mihomo_source_present,
        sidecar_dir: sidecar_dir.to_string_lossy().to_string(),
        sidecar_binary_count,
        build_script_path: build_script_path.to_string_lossy().to_string(),
        build_script_present,
        sidecar_binary_removal_requested,
        sidecar_binary_removal_allowed,
        passed: blockers.is_empty(),
        blockers,
    })
}

fn selective_deprecation_evidence(
    report: &MihomoFallbackRetirementExecutionReport,
) -> RustMihomoFallbackRetirementSelectiveDeprecationEvidence {
    let supported_scopes = report
        .supported_scope
        .iter()
        .map(|scope| {
            let passed = scope.fallback_retired_for_scope
                && !scope.evidence.is_empty()
                && !scope.mihomo_fallback_retained_for.is_empty();
            RustMihomoFallbackRetirementSelectiveScopeEvidence {
                scope: scope.scope.to_string(),
                fallback_retired_for_scope: scope.fallback_retired_for_scope,
                evidence: scope.evidence.iter().map(|evidence| evidence.to_string()).collect(),
                retained_fallback_count: scope.mihomo_fallback_retained_for.len(),
                passed,
                blockers: evidence_blockers(passed, "selective retirement scope lacked evidence"),
            }
        })
        .collect::<Vec<_>>();
    let retires_supported_fallback = report.retires_supported_fallback;
    let removes_mihomo_fallback_binary = report.removes_mihomo_fallback_binary;
    let unsupported_fallback_retained = report.unsupported_mihomo_fallback_retained;
    let passed = retires_supported_fallback
        && !removes_mihomo_fallback_binary
        && unsupported_fallback_retained
        && supported_scopes.iter().all(|scope| scope.passed);
    let mut blockers = supported_scopes
        .iter()
        .flat_map(|scope| scope.blockers.iter().cloned())
        .collect::<Vec<_>>();
    blockers.extend(evidence_blockers(
        passed,
        "selective deprecation/removal evidence failed",
    ));

    RustMihomoFallbackRetirementSelectiveDeprecationEvidence {
        supported_scopes,
        retires_supported_fallback,
        removes_mihomo_fallback_binary,
        unsupported_fallback_retained,
        passed: blockers.is_empty(),
        blockers,
    }
}

trait StatusString {
    fn status_string(&self) -> String;
}

impl StatusString for super::RustUdpPluginTransportBundleReport {
    fn status_string(&self) -> String {
        match self.status {
            super::RustUdpPluginTransportBundleStatus::Passed => "passed",
            super::RustUdpPluginTransportBundleStatus::Failed => "failed",
            super::RustUdpPluginTransportBundleStatus::Blocked => "blocked",
        }
        .to_owned()
    }
}

impl StatusString for super::RustTunPacketCaptureHoldBundleReport {
    fn status_string(&self) -> String {
        match self.status {
            super::RustTunPacketCaptureHoldBundleStatus::Passed => "passed",
            super::RustTunPacketCaptureHoldBundleStatus::Failed => "failed",
            super::RustTunPacketCaptureHoldBundleStatus::Blocked => "blocked",
        }
        .to_owned()
    }
}

impl StatusString for MihomoFallbackRetirementExecutionReport {
    fn status_string(&self) -> String {
        match self.status {
            super::MihomoFallbackRetirementExecutionStatus::Planned => "planned",
            super::MihomoFallbackRetirementExecutionStatus::Executed => "executed",
            super::MihomoFallbackRetirementExecutionStatus::Restored => "restored",
            super::MihomoFallbackRetirementExecutionStatus::Blocked => "blocked",
        }
        .to_owned()
    }
}

fn retained_fallback_scope() -> Vec<String> {
    vec![
        "SOCKS non-loopback UDP and fragment queue default ownership".to_owned(),
        "unsupported non-loopback encrypted protocols".to_owned(),
        "QUIC/UDP variants and multiplexed transports".to_owned(),
        "external plugin process lifecycle".to_owned(),
        "system-wide packet capture and route installation".to_owned(),
        "full Mihomo sidecar source and binary fallback".to_owned(),
    ]
}

fn facts() -> Vec<String> {
    vec![
        "bundle runs UDP/plugin and TUN/packet-capture hold evidence before fallback retirement".to_owned(),
        "supported scopes can be retired only when execution evidence and emergency rollback are present".to_owned(),
        "unsupported fallback and the sidecar build dependency remain retained".to_owned(),
    ]
}

fn repo_root() -> Result<PathBuf> {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .map(std::path::Path::to_path_buf)
        .context("resolve repository root")
}

fn sidecar_binary_count(sidecar_dir: &std::path::Path) -> usize {
    std_fs::read_dir(sidecar_dir)
        .ok()
        .into_iter()
        .flat_map(|entries| entries.filter_map(|entry| entry.ok()))
        .filter(|entry| entry.file_name().to_string_lossy().starts_with("verge-mihomo"))
        .count()
}

fn evidence_blockers(passed: bool, blocker: &str) -> Vec<String> {
    if passed { Vec::new() } else { vec![blocker.to_owned()] }
}

fn evidence_dir() -> Result<PathBuf> {
    Ok(dirs::app_runtime_dir()?.join(COMPONENT))
}

fn evidence_path() -> Result<PathBuf> {
    Ok(evidence_dir()?.join(EVIDENCE_FILE))
}

fn rollback_path() -> Result<PathBuf> {
    Ok(evidence_dir()?.join(ROLLBACK_FILE))
}

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
    fn unsupported_paths_remain_retained() {
        let paths = retained_fallback_scope();

        assert!(paths.iter().any(|path| path.contains("SOCKS non-loopback UDP")));
        assert!(paths.iter().any(|path| path.contains("sidecar")));
    }

    #[test]
    fn sidecar_audit_resolves_repo_paths() {
        let root = repo_root().unwrap();

        assert!(root.join("src-tauri").exists());
        assert!(root.join("scripts").join("build-mihomo-sidecar.mjs").exists());
    }
}
