use super::{
    RUST_RUNTIME_ID, RustMihomoFallbackRetirementBundleReport, RustMihomoFallbackRetirementBundleStatus,
    RustSidecarIndependentRollbackReport, RustSidecarIndependentRollbackStatus,
    applied_guarded_production_default_forwarding_surfaces, applied_guarded_tun_packet_capture_surfaces,
    rust_mihomo_fallback_retirement_bundle_execution, rust_sidecar_independent_rollback_archive,
};
use crate::utils::dirs;
use anyhow::{Context as _, Result};
use serde::{Deserialize, Serialize};
use sha2::{Digest as _, Sha256};
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::fs;

const COMPONENT: &str = "rust-fallback-retirement-closeout";
const KERNEL_AREA: &str = "fallback-retirement-closeout";
const CLOSEOUT_MANIFEST_FILE: &str = "closeout-manifest.yaml";
const EVIDENCE_FILE: &str = "evidence.yaml";
const ROLLBACK_CHECKPOINT_FILE: &str = "rollback-checkpoint.yaml";
const ROLLBACK_EVIDENCE_FILE: &str = "rollback-evidence.yaml";
const NEXT_SAFE_BATCH: &str = "final-mihomo-binary-removal-gate";

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum RustFallbackRetirementCloseoutStatus {
    Ready,
    Blocked,
    ClosedOut,
    Verified,
    RolledBack,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RustFallbackRetirementCloseoutSurface {
    pub fallback_surface: String,
    pub guarded_apply_evidence: Vec<String>,
    pub bundle_supported: bool,
    pub sidecar_independent_rollback_ready: bool,
    pub closeout_committed: bool,
    pub mihomo_fallback_retired: bool,
    pub mihomo_binary_removal_allowed: bool,
    pub rollback_checkpoint_required: bool,
    pub blockers: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RustFallbackRetirementRollbackCheckpoint {
    pub component: String,
    pub created_at_epoch_seconds: u64,
    pub closed_out_surfaces: Vec<String>,
    pub restore_owner: String,
    pub rollback_actions: Vec<String>,
    pub checkpoint_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RustFallbackRetirementCloseoutManifest {
    pub component: String,
    pub created_at_epoch_seconds: u64,
    pub surfaces: Vec<RustFallbackRetirementCloseoutSurface>,
    pub fallback_bundle_evidence_path: Option<String>,
    pub sidecar_independent_rollback_path: Option<String>,
    pub production_default_forwarding_surfaces: Vec<String>,
    pub tun_packet_capture_surfaces: Vec<String>,
    pub rollback_checkpoint_path: Option<String>,
    pub evidence_checksum: String,
    pub mutates_runtime: bool,
    pub fallback_retirement_closed_out: bool,
    pub removes_mihomo_fallback_binary: bool,
    pub next_safe_batch: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RustFallbackRetirementCloseoutReport {
    pub runtime_id: String,
    pub component: String,
    pub kernel_area: String,
    pub status: RustFallbackRetirementCloseoutStatus,
    pub reason: String,
    pub explicit_opt_in: bool,
    pub operator_approved: bool,
    pub commit_closeout: bool,
    pub verify_post_closeout: bool,
    pub fallback_bundle_gate: Option<RustMihomoFallbackRetirementBundleReport>,
    pub sidecar_independent_rollback_gate: Option<RustSidecarIndependentRollbackReport>,
    pub production_default_forwarding_surfaces: Vec<String>,
    pub tun_packet_capture_surfaces: Vec<String>,
    pub closeout_manifest: Option<RustFallbackRetirementCloseoutManifest>,
    pub closeout_manifest_path: Option<String>,
    pub evidence_path: Option<String>,
    pub rollback_checkpoint: RustFallbackRetirementRollbackCheckpoint,
    pub rollback_checkpoint_path: Option<String>,
    pub closeout_manifest_checksum: Option<String>,
    pub mutates_runtime: bool,
    pub writes_closeout_manifest: bool,
    pub writes_rollback_checkpoint: bool,
    pub writes_evidence: bool,
    pub fallback_retirement_closed_out: bool,
    pub removes_mihomo_fallback_binary: bool,
    pub mihomo_fallback_restore_required: bool,
    pub blockers_reduced: Vec<String>,
    pub blockers_remaining: Vec<String>,
    pub blockers: Vec<String>,
    pub warnings: Vec<String>,
    pub facts: Vec<String>,
    pub next_safe_batch: String,
}

pub async fn rust_fallback_retirement_closeout(
    explicit_opt_in: bool,
    operator_approved: bool,
    commit_closeout: bool,
    verify_post_closeout: bool,
) -> Result<RustFallbackRetirementCloseoutReport> {
    let fallback_bundle_gate = Some(rust_mihomo_fallback_retirement_bundle_execution(explicit_opt_in).await?);
    let sidecar_independent_rollback_gate = Some(rust_sidecar_independent_rollback_archive(explicit_opt_in).await?);
    let production_default_forwarding_surfaces = applied_guarded_production_default_forwarding_surfaces().await?;
    let tun_packet_capture_surfaces = applied_guarded_tun_packet_capture_surfaces().await?;

    let mut blockers = closeout_blockers(
        explicit_opt_in,
        operator_approved,
        commit_closeout,
        verify_post_closeout,
        fallback_bundle_gate.as_ref(),
        sidecar_independent_rollback_gate.as_ref(),
        &production_default_forwarding_surfaces,
        &tun_packet_capture_surfaces,
    );
    blockers.sort();
    blockers.dedup();

    let closeout_committed = commit_closeout && blockers.is_empty();
    let rollback_checkpoint = rollback_checkpoint(closeout_committed, None);
    let mut manifest = closeout_manifest(
        closeout_committed,
        fallback_bundle_gate.as_ref(),
        sidecar_independent_rollback_gate.as_ref(),
        &production_default_forwarding_surfaces,
        &tun_packet_capture_surfaces,
        None,
    )?;
    let manifest_checksum = hex_sha256(serde_yaml_ng::to_string(&manifest)?.as_bytes());

    let mut report = build_report(
        explicit_opt_in,
        operator_approved,
        commit_closeout,
        verify_post_closeout,
        fallback_bundle_gate,
        sidecar_independent_rollback_gate,
        production_default_forwarding_surfaces,
        tun_packet_capture_surfaces,
        Some(manifest.clone()),
        Some(manifest_checksum),
        rollback_checkpoint,
        blockers,
    );

    if report.status == RustFallbackRetirementCloseoutStatus::Blocked {
        return Ok(report);
    }

    if commit_closeout {
        let closeout_manifest_path = closeout_manifest_path()?;
        let evidence_path = evidence_path()?;
        let rollback_checkpoint_path = rollback_checkpoint_path()?;
        if let Some(parent) = closeout_manifest_path.parent() {
            fs::create_dir_all(parent).await?;
        }

        manifest.rollback_checkpoint_path = Some(rollback_checkpoint_path.to_string_lossy().to_string());
        report.closeout_manifest = Some(manifest.clone());
        report.closeout_manifest_path = Some(closeout_manifest_path.to_string_lossy().to_string());
        report.evidence_path = Some(evidence_path.to_string_lossy().to_string());
        report.rollback_checkpoint_path = Some(rollback_checkpoint_path.to_string_lossy().to_string());
        report.rollback_checkpoint.checkpoint_path = Some(rollback_checkpoint_path.to_string_lossy().to_string());
        report.closeout_manifest_checksum = Some(hex_sha256(serde_yaml_ng::to_string(&manifest)?.as_bytes()));
        report.writes_closeout_manifest = true;
        report.writes_rollback_checkpoint = true;
        report.writes_evidence = true;

        fs::write(
            &rollback_checkpoint_path,
            serde_yaml_ng::to_string(&report.rollback_checkpoint)?.as_bytes(),
        )
        .await?;
        fs::write(&closeout_manifest_path, serde_yaml_ng::to_string(&manifest)?.as_bytes()).await?;
        fs::write(&evidence_path, serde_yaml_ng::to_string(&report)?.as_bytes()).await?;
    }

    Ok(report)
}

pub async fn rollback_fallback_retirement_closeout(
    explicit_opt_in: bool,
) -> Result<RustFallbackRetirementCloseoutReport> {
    let checkpoint_path = rollback_checkpoint_path()?;
    let checkpoint_yaml = fs::read_to_string(&checkpoint_path)
        .await
        .with_context(|| format!("failed to read {}", checkpoint_path.display()))?;
    let checkpoint: RustFallbackRetirementRollbackCheckpoint = serde_yaml_ng::from_str(&checkpoint_yaml)
        .with_context(|| format!("failed to parse {}", checkpoint_path.display()))?;
    let mut report = build_report(
        explicit_opt_in,
        true,
        false,
        false,
        None,
        None,
        Vec::new(),
        Vec::new(),
        None,
        None,
        checkpoint,
        if explicit_opt_in {
            Vec::new()
        } else {
            vec!["explicit opt-in is required before fallback retirement closeout rollback".to_owned()]
        },
    );

    if !report.blockers.is_empty() {
        return Ok(report);
    }

    let rollback_evidence_path = rollback_evidence_path()?;
    report.status = RustFallbackRetirementCloseoutStatus::RolledBack;
    report.reason = "fallback retirement closeout restored Mihomo fallback checkpoint".to_owned();
    report.evidence_path = Some(rollback_evidence_path.to_string_lossy().to_string());
    report.writes_evidence = true;
    report.mutates_runtime = true;
    report.fallback_retirement_closed_out = false;
    report.mihomo_fallback_restore_required = true;
    report.blockers_reduced = vec!["fallback retirement closeout rollback restored".to_owned()];
    report.blockers_remaining = vec![
        "re-run guarded default-forwarding and TUN packet-capture apply evidence before closeout".to_owned(),
        "final Mihomo binary removal remains blocked after rollback".to_owned(),
    ];
    if let Some(parent) = rollback_evidence_path.parent() {
        fs::create_dir_all(parent).await?;
    }
    fs::write(&rollback_evidence_path, serde_yaml_ng::to_string(&report)?.as_bytes()).await?;
    Ok(report)
}

pub async fn fallback_retirement_closed_out_surfaces() -> Result<Vec<String>> {
    let Some(manifest) = read_closeout_manifest().await? else {
        return Ok(Vec::new());
    };

    Ok(manifest
        .surfaces
        .into_iter()
        .filter(|surface| surface.closeout_committed && surface.mihomo_fallback_retired)
        .map(|surface| surface.fallback_surface)
        .collect())
}

#[allow(clippy::too_many_arguments)]
fn build_report(
    explicit_opt_in: bool,
    operator_approved: bool,
    commit_closeout: bool,
    verify_post_closeout: bool,
    fallback_bundle_gate: Option<RustMihomoFallbackRetirementBundleReport>,
    sidecar_independent_rollback_gate: Option<RustSidecarIndependentRollbackReport>,
    production_default_forwarding_surfaces: Vec<String>,
    tun_packet_capture_surfaces: Vec<String>,
    closeout_manifest: Option<RustFallbackRetirementCloseoutManifest>,
    closeout_manifest_checksum: Option<String>,
    rollback_checkpoint: RustFallbackRetirementRollbackCheckpoint,
    blockers: Vec<String>,
) -> RustFallbackRetirementCloseoutReport {
    let status = if blockers.is_empty() && commit_closeout && verify_post_closeout {
        RustFallbackRetirementCloseoutStatus::Verified
    } else if blockers.is_empty() && commit_closeout {
        RustFallbackRetirementCloseoutStatus::ClosedOut
    } else if blockers.is_empty() {
        RustFallbackRetirementCloseoutStatus::Ready
    } else {
        RustFallbackRetirementCloseoutStatus::Blocked
    };
    let closed_out = matches!(
        status,
        RustFallbackRetirementCloseoutStatus::ClosedOut | RustFallbackRetirementCloseoutStatus::Verified
    );

    RustFallbackRetirementCloseoutReport {
        runtime_id: RUST_RUNTIME_ID.to_owned(),
        component: COMPONENT.to_owned(),
        kernel_area: KERNEL_AREA.to_owned(),
        status,
        reason: if status == RustFallbackRetirementCloseoutStatus::Verified {
            "fallback retirement closeout committed and post-closeout evidence verified"
        } else if closed_out {
            "fallback retirement closeout committed with rollback checkpoint retained"
        } else if blockers.is_empty() {
            "fallback retirement closeout is ready once commit_closeout is requested"
        } else {
            "fallback retirement closeout is blocked"
        }
        .to_owned(),
        explicit_opt_in,
        operator_approved,
        commit_closeout,
        verify_post_closeout,
        fallback_bundle_gate,
        sidecar_independent_rollback_gate,
        production_default_forwarding_surfaces,
        tun_packet_capture_surfaces,
        closeout_manifest,
        closeout_manifest_path: None,
        evidence_path: None,
        rollback_checkpoint,
        rollback_checkpoint_path: None,
        closeout_manifest_checksum,
        mutates_runtime: closed_out,
        writes_closeout_manifest: false,
        writes_rollback_checkpoint: false,
        writes_evidence: false,
        fallback_retirement_closed_out: closed_out,
        removes_mihomo_fallback_binary: false,
        mihomo_fallback_restore_required: !closed_out,
        blockers_reduced: if closed_out {
            vec![
                "guarded default-forwarding fallback surfaces closed out".to_owned(),
                "guarded TUN/packet-capture fallback surfaces closed out".to_owned(),
                "fallback retirement bundle promoted to closeout manifest".to_owned(),
                "Mihomo fallback binary removal advanced to final gate only".to_owned(),
            ]
        } else {
            Vec::new()
        },
        blockers_remaining: if closed_out {
            vec![
                "real remote encrypted/QUIC peer compatibility".to_owned(),
                "operator-approved real plugin binary compatibility".to_owned(),
                "final Mihomo binary removal gate".to_owned(),
            ]
        } else {
            vec!["fallback retirement closeout has not been committed".to_owned()]
        },
        blockers,
        warnings: vec![
            "closeout retires fallback surfaces but does not remove the Mihomo binary".to_owned(),
            "rollback checkpoint remains mandatory until the final binary-removal gate passes".to_owned(),
        ],
        facts: vec![
            "closeout consumes fallback bundle, sidecar-independent rollback, default-forwarding apply, and TUN packet-capture apply evidence".to_owned(),
            "Mihomo binary removal remains blocked behind the final gate".to_owned(),
            "rollback evidence is written separately from the closeout manifest".to_owned(),
        ],
        next_safe_batch: NEXT_SAFE_BATCH.to_owned(),
    }
}

fn closeout_blockers(
    explicit_opt_in: bool,
    operator_approved: bool,
    commit_closeout: bool,
    verify_post_closeout: bool,
    fallback_bundle_gate: Option<&RustMihomoFallbackRetirementBundleReport>,
    sidecar_independent_rollback_gate: Option<&RustSidecarIndependentRollbackReport>,
    production_default_forwarding_surfaces: &[String],
    tun_packet_capture_surfaces: &[String],
) -> Vec<String> {
    let mut blockers = Vec::new();
    if !explicit_opt_in {
        blockers.push("explicit opt-in is required before fallback retirement closeout".to_owned());
    }
    if !operator_approved {
        blockers.push("operator approval is required before fallback retirement closeout".to_owned());
    }
    if !commit_closeout {
        blockers.push("commit_closeout is required to retire Mihomo fallback surfaces".to_owned());
    }
    if !verify_post_closeout {
        blockers.push("post-closeout verification is required in the fallback retirement bundle".to_owned());
    }
    if !matches!(
        fallback_bundle_gate.map(|gate| gate.status),
        Some(RustMihomoFallbackRetirementBundleStatus::Passed)
    ) {
        blockers.push("fallback retirement bundle evidence must pass".to_owned());
    }
    if !matches!(
        sidecar_independent_rollback_gate.map(|gate| gate.status),
        Some(RustSidecarIndependentRollbackStatus::Ready)
    ) {
        blockers.push("sidecar-independent rollback archive must be ready".to_owned());
    }
    if production_default_forwarding_surfaces.is_empty() {
        blockers.push("guarded production default-forwarding apply surfaces are required".to_owned());
    }
    if tun_packet_capture_surfaces.is_empty() {
        blockers.push("guarded TUN/packet-capture apply surfaces are required".to_owned());
    }
    blockers
}

fn closeout_manifest(
    closeout_committed: bool,
    fallback_bundle_gate: Option<&RustMihomoFallbackRetirementBundleReport>,
    sidecar_independent_rollback_gate: Option<&RustSidecarIndependentRollbackReport>,
    production_default_forwarding_surfaces: &[String],
    tun_packet_capture_surfaces: &[String],
    rollback_checkpoint_path: Option<String>,
) -> Result<RustFallbackRetirementCloseoutManifest> {
    let fallback_bundle_ready = matches!(
        fallback_bundle_gate.map(|gate| gate.status),
        Some(RustMihomoFallbackRetirementBundleStatus::Passed)
    );
    let sidecar_independent_ready = matches!(
        sidecar_independent_rollback_gate.map(|gate| gate.status),
        Some(RustSidecarIndependentRollbackStatus::Ready)
    );
    let fallback_bundle_evidence_path = fallback_bundle_gate.and_then(|gate| gate.evidence_path.clone());
    let sidecar_independent_rollback_path =
        sidecar_independent_rollback_gate.and_then(|gate| gate.rollback_plan_path.clone());
    let evidence_checksum = hex_sha256(
        [
            fallback_bundle_evidence_path.clone(),
            sidecar_independent_rollback_path.clone(),
        ]
        .into_iter()
        .flatten()
        .collect::<Vec<_>>()
        .join("\n")
        .as_bytes(),
    );

    Ok(RustFallbackRetirementCloseoutManifest {
        component: COMPONENT.to_owned(),
        created_at_epoch_seconds: current_epoch_seconds(),
        surfaces: vec![
            closeout_surface(
                "production default-forwarding fallback",
                production_default_forwarding_surfaces,
                fallback_bundle_ready,
                sidecar_independent_ready,
                closeout_committed,
            ),
            closeout_surface(
                "TUN packet-capture fallback",
                tun_packet_capture_surfaces,
                fallback_bundle_ready,
                sidecar_independent_ready,
                closeout_committed,
            ),
            closeout_surface(
                "supported fallback-retirement inventory",
                &["bounded DNS/adapter/protocol/UDP/plugin/TUN inventory".to_owned()],
                fallback_bundle_ready,
                sidecar_independent_ready,
                closeout_committed,
            ),
        ],
        fallback_bundle_evidence_path,
        sidecar_independent_rollback_path,
        production_default_forwarding_surfaces: production_default_forwarding_surfaces.to_vec(),
        tun_packet_capture_surfaces: tun_packet_capture_surfaces.to_vec(),
        rollback_checkpoint_path,
        evidence_checksum,
        mutates_runtime: closeout_committed,
        fallback_retirement_closed_out: closeout_committed,
        removes_mihomo_fallback_binary: false,
        next_safe_batch: NEXT_SAFE_BATCH.to_owned(),
    })
}

fn closeout_surface(
    fallback_surface: &str,
    guarded_apply_evidence: &[String],
    bundle_supported: bool,
    sidecar_independent_rollback_ready: bool,
    closeout_committed: bool,
) -> RustFallbackRetirementCloseoutSurface {
    let mut blockers = Vec::new();
    if guarded_apply_evidence.is_empty() {
        blockers.push("guarded apply evidence is required".to_owned());
    }
    if !bundle_supported {
        blockers.push("fallback retirement bundle evidence is required".to_owned());
    }
    if !sidecar_independent_rollback_ready {
        blockers.push("sidecar-independent rollback evidence is required".to_owned());
    }
    if !closeout_committed {
        blockers.push("closeout has not been committed".to_owned());
    }

    RustFallbackRetirementCloseoutSurface {
        fallback_surface: fallback_surface.to_owned(),
        guarded_apply_evidence: guarded_apply_evidence.to_vec(),
        bundle_supported,
        sidecar_independent_rollback_ready,
        closeout_committed,
        mihomo_fallback_retired: closeout_committed,
        mihomo_binary_removal_allowed: false,
        rollback_checkpoint_required: true,
        blockers,
    }
}

fn rollback_checkpoint(
    closeout_committed: bool,
    checkpoint_path: Option<String>,
) -> RustFallbackRetirementRollbackCheckpoint {
    RustFallbackRetirementRollbackCheckpoint {
        component: COMPONENT.to_owned(),
        created_at_epoch_seconds: current_epoch_seconds(),
        closed_out_surfaces: if closeout_committed {
            vec![
                "production default-forwarding fallback".to_owned(),
                "TUN packet-capture fallback".to_owned(),
                "supported fallback-retirement inventory".to_owned(),
            ]
        } else {
            Vec::new()
        },
        restore_owner: "Mihomo fallback restore path".to_owned(),
        rollback_actions: vec![
            "restore Mihomo fallback ownership for closed-out runtime surfaces".to_owned(),
            "re-run guarded default-forwarding apply evidence before closeout".to_owned(),
            "re-run guarded TUN packet-capture apply evidence before closeout".to_owned(),
            "keep Mihomo sidecar binary until final removal gate passes".to_owned(),
        ],
        checkpoint_path,
    }
}

async fn read_closeout_manifest() -> Result<Option<RustFallbackRetirementCloseoutManifest>> {
    let path = closeout_manifest_path()?;
    if fs::metadata(&path).await.is_err() {
        return Ok(None);
    }
    let yaml = fs::read_to_string(&path)
        .await
        .with_context(|| format!("failed to read {}", path.display()))?;
    let manifest = serde_yaml_ng::from_str(&yaml).with_context(|| format!("failed to parse {}", path.display()))?;
    Ok(Some(manifest))
}

fn closeout_manifest_path() -> Result<std::path::PathBuf> {
    Ok(component_dir()?.join(CLOSEOUT_MANIFEST_FILE))
}

pub fn rust_fallback_retirement_closeout_evidence_path() -> Result<std::path::PathBuf> {
    Ok(component_dir()?.join(EVIDENCE_FILE))
}

fn evidence_path() -> Result<std::path::PathBuf> {
    rust_fallback_retirement_closeout_evidence_path()
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
    fn verified_closeout_keeps_binary_removal_blocked() {
        let report = build_report(
            true,
            true,
            true,
            true,
            None,
            None,
            vec!["TCP HTTP default forwarding".to_owned()],
            vec!["TUN device lifecycle".to_owned()],
            None,
            None,
            rollback_checkpoint(true, None),
            Vec::new(),
        );

        assert_eq!(report.status, RustFallbackRetirementCloseoutStatus::Verified);
        assert!(report.fallback_retirement_closed_out);
        assert!(report.mutates_runtime);
        assert!(!report.removes_mihomo_fallback_binary);
        assert_eq!(report.next_safe_batch, NEXT_SAFE_BATCH);
    }

    #[test]
    fn missing_guarded_apply_surfaces_block_closeout() {
        let blockers = closeout_blockers(true, true, true, true, None, None, &[], &[]);

        assert!(blockers.iter().any(|blocker| blocker.contains("default-forwarding")));
        assert!(blockers.iter().any(|blocker| blocker.contains("TUN/packet-capture")));
    }
}
