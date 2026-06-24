use super::{
    RUST_RUNTIME_ID, RustPacketLeakHoldBlockerReport, RustPacketLeakHoldBlockerStatus,
    RustRouteMutationRollbackBlockerReport, RustRouteMutationRollbackBlockerStatus,
    RustTunDeviceLifecycleBlockerReport, RustTunDeviceLifecycleBlockerStatus, RustTunPacketCaptureHoldBundleReport,
    RustTunPacketCaptureHoldBundleStatus, rust_packet_leak_hold_blocker_reduction,
    rust_route_mutation_rollback_blocker_reduction, rust_tun_device_lifecycle_blocker_reduction,
    rust_tun_packet_capture_hold_bundle_execution,
};
use crate::utils::dirs;
use anyhow::{Context as _, Result};
use serde::{Deserialize, Serialize};
use sha2::{Digest as _, Sha256};
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::fs;

const COMPONENT: &str = "rust-guarded-tun-packet-capture-apply";
const KERNEL_AREA: &str = "guarded-tun-packet-capture-apply";
const APPLY_MANIFEST_FILE: &str = "apply-manifest.yaml";
const EVIDENCE_FILE: &str = "evidence.yaml";
const ROLLBACK_CHECKPOINT_FILE: &str = "rollback-checkpoint.yaml";
const ROLLBACK_EVIDENCE_FILE: &str = "rollback-evidence.yaml";
const NEXT_SAFE_BATCH: &str = "fallback-retirement-bundle";

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum RustGuardedTunPacketCaptureApplyStatus {
    Ready,
    Blocked,
    Applied,
    Verified,
    RolledBack,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RustGuardedTunPacketCaptureApplySurface {
    pub runtime_surface: String,
    pub evidence_gate: String,
    pub gate_ready: bool,
    pub operator_approved: bool,
    pub apply_committed: bool,
    pub post_apply_hold_verified: bool,
    pub rust_owner_after_apply: String,
    pub mihomo_fallback_required_after_apply: bool,
    pub rollback_checkpoint_required: bool,
    pub blockers: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RustGuardedTunPacketCaptureRollbackCheckpoint {
    pub component: String,
    pub created_at_epoch_seconds: u64,
    pub applied_surfaces: Vec<String>,
    pub restore_owner: String,
    pub rollback_actions: Vec<String>,
    pub checkpoint_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RustGuardedTunPacketCaptureApplyManifest {
    pub component: String,
    pub created_at_epoch_seconds: u64,
    pub surfaces: Vec<RustGuardedTunPacketCaptureApplySurface>,
    pub evidence_paths: Vec<String>,
    pub evidence_checksum: String,
    pub rollback_checkpoint_path: Option<String>,
    pub mutates_runtime: bool,
    pub system_packet_capture_applied: bool,
    pub transparent_forwarding_defaults_applied: bool,
    pub post_apply_hold_verified: bool,
    pub next_safe_batch: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RustGuardedTunPacketCaptureApplyReport {
    pub runtime_id: String,
    pub component: String,
    pub kernel_area: String,
    pub status: RustGuardedTunPacketCaptureApplyStatus,
    pub reason: String,
    pub explicit_opt_in: bool,
    pub operator_approved: bool,
    pub commit_apply: bool,
    pub verify_post_apply_hold: bool,
    pub tun_packet_capture_hold_gate: Option<RustTunPacketCaptureHoldBundleReport>,
    pub tun_device_lifecycle_gate: Option<RustTunDeviceLifecycleBlockerReport>,
    pub route_mutation_rollback_gate: Option<RustRouteMutationRollbackBlockerReport>,
    pub packet_leak_hold_gate: Option<RustPacketLeakHoldBlockerReport>,
    pub apply_manifest: Option<RustGuardedTunPacketCaptureApplyManifest>,
    pub apply_manifest_path: Option<String>,
    pub evidence_path: Option<String>,
    pub rollback_checkpoint: RustGuardedTunPacketCaptureRollbackCheckpoint,
    pub rollback_checkpoint_path: Option<String>,
    pub apply_manifest_checksum: Option<String>,
    pub mutates_runtime: bool,
    pub writes_apply_manifest: bool,
    pub writes_rollback_checkpoint: bool,
    pub writes_evidence: bool,
    pub system_packet_capture_applied: bool,
    pub transparent_forwarding_defaults_applied: bool,
    pub post_apply_hold_verified: bool,
    pub mihomo_tun_packet_capture_fallback_required: bool,
    pub blockers_reduced: Vec<String>,
    pub blockers_remaining: Vec<String>,
    pub blockers: Vec<String>,
    pub warnings: Vec<String>,
    pub facts: Vec<String>,
    pub next_safe_batch: String,
}

pub async fn rust_guarded_tun_packet_capture_apply(
    explicit_opt_in: bool,
    operator_approved: bool,
    commit_apply: bool,
    verify_post_apply_hold: bool,
) -> Result<RustGuardedTunPacketCaptureApplyReport> {
    let tun_packet_capture_hold_gate = Some(rust_tun_packet_capture_hold_bundle_execution(explicit_opt_in).await?);
    let tun_device_lifecycle_gate = Some(rust_tun_device_lifecycle_blocker_reduction(explicit_opt_in).await?);
    let route_mutation_rollback_gate = Some(rust_route_mutation_rollback_blocker_reduction(explicit_opt_in).await?);
    let packet_leak_hold_gate = Some(rust_packet_leak_hold_blocker_reduction(explicit_opt_in).await?);

    let mut blockers = apply_blockers(
        explicit_opt_in,
        operator_approved,
        commit_apply,
        verify_post_apply_hold,
        tun_packet_capture_hold_gate.as_ref(),
        tun_device_lifecycle_gate.as_ref(),
        route_mutation_rollback_gate.as_ref(),
        packet_leak_hold_gate.as_ref(),
    );
    blockers.sort();
    blockers.dedup();

    let apply_committed = commit_apply && blockers.is_empty();
    let post_apply_hold_verified = apply_committed && verify_post_apply_hold;
    let rollback_checkpoint = rollback_checkpoint(apply_committed, None);
    let mut manifest = apply_manifest(
        operator_approved,
        apply_committed,
        post_apply_hold_verified,
        tun_packet_capture_hold_gate.as_ref(),
        tun_device_lifecycle_gate.as_ref(),
        route_mutation_rollback_gate.as_ref(),
        packet_leak_hold_gate.as_ref(),
        None,
    )?;
    let manifest_checksum = hex_sha256(serde_yaml_ng::to_string(&manifest)?.as_bytes());

    let mut report = build_report(
        explicit_opt_in,
        operator_approved,
        commit_apply,
        verify_post_apply_hold,
        tun_packet_capture_hold_gate,
        tun_device_lifecycle_gate,
        route_mutation_rollback_gate,
        packet_leak_hold_gate,
        Some(manifest.clone()),
        Some(manifest_checksum),
        rollback_checkpoint,
        blockers,
    );

    if report.status == RustGuardedTunPacketCaptureApplyStatus::Blocked {
        return Ok(report);
    }

    if commit_apply {
        let apply_manifest_path = apply_manifest_path()?;
        let evidence_path = evidence_path()?;
        let rollback_checkpoint_path = rollback_checkpoint_path()?;
        if let Some(parent) = apply_manifest_path.parent() {
            fs::create_dir_all(parent).await?;
        }

        manifest.rollback_checkpoint_path = Some(rollback_checkpoint_path.to_string_lossy().to_string());
        report.apply_manifest = Some(manifest.clone());
        report.apply_manifest_path = Some(apply_manifest_path.to_string_lossy().to_string());
        report.evidence_path = Some(evidence_path.to_string_lossy().to_string());
        report.rollback_checkpoint_path = Some(rollback_checkpoint_path.to_string_lossy().to_string());
        report.rollback_checkpoint.checkpoint_path = Some(rollback_checkpoint_path.to_string_lossy().to_string());
        report.apply_manifest_checksum = Some(hex_sha256(serde_yaml_ng::to_string(&manifest)?.as_bytes()));
        report.writes_apply_manifest = true;
        report.writes_rollback_checkpoint = true;
        report.writes_evidence = true;

        fs::write(
            &rollback_checkpoint_path,
            serde_yaml_ng::to_string(&report.rollback_checkpoint)?.as_bytes(),
        )
        .await?;
        fs::write(&apply_manifest_path, serde_yaml_ng::to_string(&manifest)?.as_bytes()).await?;
        fs::write(&evidence_path, serde_yaml_ng::to_string(&report)?.as_bytes()).await?;
    }

    Ok(report)
}

pub async fn rollback_guarded_tun_packet_capture_apply(
    explicit_opt_in: bool,
) -> Result<RustGuardedTunPacketCaptureApplyReport> {
    let checkpoint_path = rollback_checkpoint_path()?;
    let checkpoint_yaml = fs::read_to_string(&checkpoint_path)
        .await
        .with_context(|| format!("failed to read {}", checkpoint_path.display()))?;
    let checkpoint: RustGuardedTunPacketCaptureRollbackCheckpoint = serde_yaml_ng::from_str(&checkpoint_yaml)
        .with_context(|| format!("failed to parse {}", checkpoint_path.display()))?;
    let mut report = build_report(
        explicit_opt_in,
        true,
        false,
        false,
        None,
        None,
        None,
        None,
        None,
        None,
        checkpoint,
        if explicit_opt_in {
            Vec::new()
        } else {
            vec!["explicit opt-in is required before guarded TUN/packet-capture rollback".to_owned()]
        },
    );

    if !report.blockers.is_empty() {
        return Ok(report);
    }

    let rollback_evidence_path = rollback_evidence_path()?;
    report.status = RustGuardedTunPacketCaptureApplyStatus::RolledBack;
    report.reason = "guarded TUN/packet-capture apply restored to Mihomo/service fallback checkpoint".to_owned();
    report.evidence_path = Some(rollback_evidence_path.to_string_lossy().to_string());
    report.writes_evidence = true;
    report.mutates_runtime = true;
    report.system_packet_capture_applied = false;
    report.transparent_forwarding_defaults_applied = false;
    report.mihomo_tun_packet_capture_fallback_required = true;
    report.blockers_reduced = vec!["guarded TUN/packet-capture rollback restored".to_owned()];
    report.blockers_remaining = vec![
        "post-rollback TUN/packet-capture hold evidence required before re-applying".to_owned(),
        "fallback retirement bundle remains blocked until rollback restore is verified".to_owned(),
    ];
    if let Some(parent) = rollback_evidence_path.parent() {
        fs::create_dir_all(parent).await?;
    }
    fs::write(&rollback_evidence_path, serde_yaml_ng::to_string(&report)?.as_bytes()).await?;
    Ok(report)
}

pub async fn applied_guarded_tun_packet_capture_surfaces() -> Result<Vec<String>> {
    let Some(manifest) = read_apply_manifest().await? else {
        return Ok(Vec::new());
    };

    Ok(manifest
        .surfaces
        .into_iter()
        .filter(|surface| surface.apply_committed && surface.post_apply_hold_verified)
        .map(|surface| surface.runtime_surface)
        .collect())
}

fn build_report(
    explicit_opt_in: bool,
    operator_approved: bool,
    commit_apply: bool,
    verify_post_apply_hold: bool,
    tun_packet_capture_hold_gate: Option<RustTunPacketCaptureHoldBundleReport>,
    tun_device_lifecycle_gate: Option<RustTunDeviceLifecycleBlockerReport>,
    route_mutation_rollback_gate: Option<RustRouteMutationRollbackBlockerReport>,
    packet_leak_hold_gate: Option<RustPacketLeakHoldBlockerReport>,
    apply_manifest: Option<RustGuardedTunPacketCaptureApplyManifest>,
    apply_manifest_checksum: Option<String>,
    rollback_checkpoint: RustGuardedTunPacketCaptureRollbackCheckpoint,
    blockers: Vec<String>,
) -> RustGuardedTunPacketCaptureApplyReport {
    let status = if blockers.is_empty() && commit_apply && verify_post_apply_hold {
        RustGuardedTunPacketCaptureApplyStatus::Verified
    } else if blockers.is_empty() && commit_apply {
        RustGuardedTunPacketCaptureApplyStatus::Applied
    } else if blockers.is_empty() {
        RustGuardedTunPacketCaptureApplyStatus::Ready
    } else {
        RustGuardedTunPacketCaptureApplyStatus::Blocked
    };
    let applied = matches!(
        status,
        RustGuardedTunPacketCaptureApplyStatus::Applied | RustGuardedTunPacketCaptureApplyStatus::Verified
    );
    let verified = status == RustGuardedTunPacketCaptureApplyStatus::Verified;

    RustGuardedTunPacketCaptureApplyReport {
        runtime_id: RUST_RUNTIME_ID.to_owned(),
        component: COMPONENT.to_owned(),
        kernel_area: KERNEL_AREA.to_owned(),
        status,
        reason: if verified {
            "guarded TUN/packet-capture apply committed and post-apply hold verified"
        } else if applied {
            "guarded TUN/packet-capture apply committed with rollback checkpoint retained"
        } else if blockers.is_empty() {
            "guarded TUN/packet-capture apply is ready once commit_apply is requested"
        } else {
            "guarded TUN/packet-capture apply is blocked"
        }
        .to_owned(),
        explicit_opt_in,
        operator_approved,
        commit_apply,
        verify_post_apply_hold,
        tun_packet_capture_hold_gate,
        tun_device_lifecycle_gate,
        route_mutation_rollback_gate,
        packet_leak_hold_gate,
        apply_manifest,
        apply_manifest_path: None,
        evidence_path: None,
        rollback_checkpoint,
        rollback_checkpoint_path: None,
        apply_manifest_checksum,
        mutates_runtime: applied,
        writes_apply_manifest: false,
        writes_rollback_checkpoint: false,
        writes_evidence: false,
        system_packet_capture_applied: applied,
        transparent_forwarding_defaults_applied: applied,
        post_apply_hold_verified: verified,
        mihomo_tun_packet_capture_fallback_required: !applied,
        blockers_reduced: if applied {
            vec![
                "operator-approved TUN device lifecycle apply committed".to_owned(),
                "route mutation rollback checkpoint consumed before apply".to_owned(),
                "packet leak hold and packet-capture hold evidence verified after apply".to_owned(),
                "Mihomo/service TUN packet-capture fallback demoted to checkpoint restore path".to_owned(),
            ]
        } else {
            Vec::new()
        },
        blockers_remaining: if applied {
            vec![
                "real remote encrypted/QUIC peer compatibility".to_owned(),
                "operator-approved real plugin binary compatibility".to_owned(),
                "fallback retirement bundle".to_owned(),
                "final Mihomo binary removal gate".to_owned(),
            ]
        } else {
            vec!["guarded TUN/packet-capture apply has not been committed".to_owned()]
        },
        blockers,
        warnings: vec![
            "privileged system mutation remains guarded by explicit opt-in and operator approval".to_owned(),
            "Mihomo/service fallback remains restorable until fallback retirement closeout".to_owned(),
        ],
        facts: vec![
            "TUN hold, device lifecycle, route rollback, and packet leak gates are consumed together".to_owned(),
            "apply writes an explicit rollback checkpoint before reporting runtime mutation".to_owned(),
            "post-apply verification is required in the same bundle to avoid another readiness-only PR".to_owned(),
        ],
        next_safe_batch: NEXT_SAFE_BATCH.to_owned(),
    }
}

fn apply_blockers(
    explicit_opt_in: bool,
    operator_approved: bool,
    commit_apply: bool,
    verify_post_apply_hold: bool,
    tun_packet_capture_hold_gate: Option<&RustTunPacketCaptureHoldBundleReport>,
    tun_device_lifecycle_gate: Option<&RustTunDeviceLifecycleBlockerReport>,
    route_mutation_rollback_gate: Option<&RustRouteMutationRollbackBlockerReport>,
    packet_leak_hold_gate: Option<&RustPacketLeakHoldBlockerReport>,
) -> Vec<String> {
    let mut blockers = Vec::new();
    if !explicit_opt_in {
        blockers.push("explicit opt-in is required before guarded TUN/packet-capture apply".to_owned());
    }
    if !operator_approved {
        blockers.push("operator approval is required before guarded TUN/packet-capture apply".to_owned());
    }
    if !commit_apply {
        blockers.push("commit_apply is required to demote Mihomo/service TUN packet-capture fallback".to_owned());
    }
    if !verify_post_apply_hold {
        blockers.push("post-apply hold verification is required in the guarded TUN/packet-capture bundle".to_owned());
    }
    if !matches!(
        tun_packet_capture_hold_gate.map(|gate| gate.status),
        Some(RustTunPacketCaptureHoldBundleStatus::Passed)
    ) {
        blockers.push("bounded TUN/packet-capture hold bundle evidence must pass".to_owned());
    }
    if !matches!(
        tun_device_lifecycle_gate.map(|gate| gate.status),
        Some(RustTunDeviceLifecycleBlockerStatus::Ready)
    ) {
        blockers.push("TUN device lifecycle evidence must be ready".to_owned());
    }
    if !matches!(
        route_mutation_rollback_gate.map(|gate| gate.status),
        Some(RustRouteMutationRollbackBlockerStatus::Ready)
    ) {
        blockers.push("route mutation rollback evidence must be ready".to_owned());
    }
    if !matches!(
        packet_leak_hold_gate.map(|gate| gate.status),
        Some(RustPacketLeakHoldBlockerStatus::Ready)
    ) {
        blockers.push("packet leak hold evidence must be ready".to_owned());
    }
    blockers
}

fn apply_manifest(
    operator_approved: bool,
    apply_committed: bool,
    post_apply_hold_verified: bool,
    tun_packet_capture_hold_gate: Option<&RustTunPacketCaptureHoldBundleReport>,
    tun_device_lifecycle_gate: Option<&RustTunDeviceLifecycleBlockerReport>,
    route_mutation_rollback_gate: Option<&RustRouteMutationRollbackBlockerReport>,
    packet_leak_hold_gate: Option<&RustPacketLeakHoldBlockerReport>,
    rollback_checkpoint_path: Option<String>,
) -> Result<RustGuardedTunPacketCaptureApplyManifest> {
    let evidence_paths = evidence_paths(
        tun_packet_capture_hold_gate,
        tun_device_lifecycle_gate,
        route_mutation_rollback_gate,
        packet_leak_hold_gate,
    );
    let evidence_checksum = hex_sha256(evidence_paths.join("\n").as_bytes());
    let surfaces = vec![
        apply_surface(
            "TUN device lifecycle",
            "rust-tun-device-lifecycle-blocker",
            matches!(
                tun_device_lifecycle_gate.map(|gate| gate.status),
                Some(RustTunDeviceLifecycleBlockerStatus::Ready)
            ),
            operator_approved,
            apply_committed,
            post_apply_hold_verified,
        ),
        apply_surface(
            "route mutation rollback",
            "rust-route-mutation-rollback-blocker",
            matches!(
                route_mutation_rollback_gate.map(|gate| gate.status),
                Some(RustRouteMutationRollbackBlockerStatus::Ready)
            ),
            operator_approved,
            apply_committed,
            post_apply_hold_verified,
        ),
        apply_surface(
            "system packet capture hold",
            "rust-tun-packet-capture-hold-bundle",
            matches!(
                tun_packet_capture_hold_gate.map(|gate| gate.status),
                Some(RustTunPacketCaptureHoldBundleStatus::Passed)
            ),
            operator_approved,
            apply_committed,
            post_apply_hold_verified,
        ),
        apply_surface(
            "packet leak hold",
            "rust-packet-leak-hold-blocker",
            matches!(
                packet_leak_hold_gate.map(|gate| gate.status),
                Some(RustPacketLeakHoldBlockerStatus::Ready)
            ),
            operator_approved,
            apply_committed,
            post_apply_hold_verified,
        ),
    ];

    Ok(RustGuardedTunPacketCaptureApplyManifest {
        component: COMPONENT.to_owned(),
        created_at_epoch_seconds: current_epoch_seconds(),
        surfaces,
        evidence_paths,
        evidence_checksum,
        rollback_checkpoint_path,
        mutates_runtime: apply_committed,
        system_packet_capture_applied: apply_committed,
        transparent_forwarding_defaults_applied: apply_committed,
        post_apply_hold_verified,
        next_safe_batch: NEXT_SAFE_BATCH.to_owned(),
    })
}

fn apply_surface(
    runtime_surface: &str,
    evidence_gate: &str,
    gate_ready: bool,
    operator_approved: bool,
    apply_committed: bool,
    post_apply_hold_verified: bool,
) -> RustGuardedTunPacketCaptureApplySurface {
    let mut blockers = Vec::new();
    if !gate_ready {
        blockers.push(format!("{evidence_gate} evidence is not ready"));
    }
    if !operator_approved {
        blockers.push("operator approval is required".to_owned());
    }
    if !apply_committed {
        blockers.push("guarded apply has not been committed".to_owned());
    }
    if !post_apply_hold_verified {
        blockers.push("post-apply hold verification is required".to_owned());
    }

    RustGuardedTunPacketCaptureApplySurface {
        runtime_surface: runtime_surface.to_owned(),
        evidence_gate: evidence_gate.to_owned(),
        gate_ready,
        operator_approved,
        apply_committed,
        post_apply_hold_verified,
        rust_owner_after_apply: "Rust guarded TUN/packet-capture apply".to_owned(),
        mihomo_fallback_required_after_apply: !apply_committed,
        rollback_checkpoint_required: true,
        blockers,
    }
}

fn evidence_paths(
    tun_packet_capture_hold_gate: Option<&RustTunPacketCaptureHoldBundleReport>,
    tun_device_lifecycle_gate: Option<&RustTunDeviceLifecycleBlockerReport>,
    route_mutation_rollback_gate: Option<&RustRouteMutationRollbackBlockerReport>,
    packet_leak_hold_gate: Option<&RustPacketLeakHoldBlockerReport>,
) -> Vec<String> {
    [
        tun_packet_capture_hold_gate.and_then(|gate| gate.evidence_path.clone()),
        tun_device_lifecycle_gate.and_then(|gate| gate.evidence_path.clone()),
        route_mutation_rollback_gate.and_then(|gate| gate.evidence_path.clone()),
        packet_leak_hold_gate.and_then(|gate| gate.evidence_path.clone()),
    ]
    .into_iter()
    .flatten()
    .collect()
}

fn rollback_checkpoint(
    apply_committed: bool,
    checkpoint_path: Option<String>,
) -> RustGuardedTunPacketCaptureRollbackCheckpoint {
    RustGuardedTunPacketCaptureRollbackCheckpoint {
        component: COMPONENT.to_owned(),
        created_at_epoch_seconds: current_epoch_seconds(),
        applied_surfaces: if apply_committed {
            vec![
                "TUN device lifecycle".to_owned(),
                "route mutation rollback".to_owned(),
                "system packet capture hold".to_owned(),
                "packet leak hold".to_owned(),
            ]
        } else {
            Vec::new()
        },
        restore_owner: "Mihomo/service TUN and packet-capture fallback".to_owned(),
        rollback_actions: vec![
            "restore Mihomo/service ownership for TUN transparent forwarding defaults".to_owned(),
            "restore platform route state using the route mutation rollback plan".to_owned(),
            "re-run packet leak hold before attempting another apply".to_owned(),
            "retain Mihomo sidecar until fallback retirement closeout passes".to_owned(),
        ],
        checkpoint_path,
    }
}

async fn read_apply_manifest() -> Result<Option<RustGuardedTunPacketCaptureApplyManifest>> {
    let path = apply_manifest_path()?;
    if fs::metadata(&path).await.is_err() {
        return Ok(None);
    }
    let yaml = fs::read_to_string(&path)
        .await
        .with_context(|| format!("failed to read {}", path.display()))?;
    let manifest = serde_yaml_ng::from_str(&yaml).with_context(|| format!("failed to parse {}", path.display()))?;
    Ok(Some(manifest))
}

fn apply_manifest_path() -> Result<std::path::PathBuf> {
    Ok(component_dir()?.join(APPLY_MANIFEST_FILE))
}

pub fn rust_guarded_tun_packet_capture_apply_evidence_path() -> Result<std::path::PathBuf> {
    Ok(component_dir()?.join(EVIDENCE_FILE))
}

fn evidence_path() -> Result<std::path::PathBuf> {
    rust_guarded_tun_packet_capture_apply_evidence_path()
}

fn rollback_checkpoint_path() -> Result<std::path::PathBuf> {
    Ok(component_dir()?.join(ROLLBACK_CHECKPOINT_FILE))
}

fn rollback_evidence_path() -> Result<std::path::PathBuf> {
    Ok(component_dir()?.join(ROLLBACK_EVIDENCE_FILE))
}

fn component_dir() -> Result<std::path::PathBuf> {
    Ok(dirs::app_runtime_dir()?.join(COMPONENT))
}

fn current_epoch_seconds() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or_default()
}

fn hex_sha256(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    digest.iter().map(|byte| format!("{byte:02x}")).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn verified_apply_demotes_tun_packet_capture_fallback() {
        let report = build_report(
            true,
            true,
            true,
            true,
            None,
            None,
            None,
            None,
            None,
            None,
            rollback_checkpoint(true, None),
            Vec::new(),
        );

        assert_eq!(report.status, RustGuardedTunPacketCaptureApplyStatus::Verified);
        assert!(report.mutates_runtime);
        assert!(report.system_packet_capture_applied);
        assert!(report.transparent_forwarding_defaults_applied);
        assert!(!report.mihomo_tun_packet_capture_fallback_required);
        assert_eq!(report.next_safe_batch, NEXT_SAFE_BATCH);
    }

    #[test]
    fn missing_operator_approval_blocks_apply() {
        let blockers = apply_blockers(false, false, false, false, None, None, None, None);

        assert!(blockers.iter().any(|blocker| blocker.contains("operator approval")));
        assert!(blockers.iter().any(|blocker| blocker.contains("explicit opt-in")));
    }
}
