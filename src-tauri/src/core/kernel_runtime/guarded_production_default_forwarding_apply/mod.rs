use super::{
    RUST_RUNTIME_ID, RustDefaultForwardingHoldBlockerReport, RustDefaultForwardingHoldBlockerStatus,
    approved_production_default_forwarding_cutover_surfaces, rust_default_forwarding_hold_blocker_reduction,
};
use crate::utils::dirs;
use anyhow::{Context as _, Result};
use serde::{Deserialize, Serialize};
use sha2::{Digest as _, Sha256};
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::fs;

const COMPONENT: &str = "rust-guarded-production-default-forwarding-apply";
const KERNEL_AREA: &str = "guarded-production-default-forwarding-apply";
const APPLY_MANIFEST_FILE: &str = "apply-manifest.yaml";
const EVIDENCE_FILE: &str = "evidence.yaml";
const ROLLBACK_CHECKPOINT_FILE: &str = "rollback-checkpoint.yaml";
const ROLLBACK_EVIDENCE_FILE: &str = "rollback-evidence.yaml";
const NEXT_SAFE_BATCH: &str = "tun-packet-capture-hold-bundle";

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum RustGuardedProductionDefaultForwardingApplyStatus {
    Ready,
    Blocked,
    Applied,
    Verified,
    RolledBack,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RustGuardedProductionDefaultForwardingApplySurface {
    pub default_surface: String,
    pub approval_committed: bool,
    pub apply_committed: bool,
    pub post_apply_hold_verified: bool,
    pub rust_owner_after_apply: String,
    pub mihomo_fallback_required_after_apply: bool,
    pub rollback_checkpoint_required: bool,
    pub blockers: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RustGuardedProductionDefaultForwardingRollbackCheckpoint {
    pub component: String,
    pub created_at_epoch_seconds: u64,
    pub applied_surfaces: Vec<String>,
    pub restore_owner: String,
    pub rollback_actions: Vec<String>,
    pub checkpoint_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RustGuardedProductionDefaultForwardingApplyManifest {
    pub component: String,
    pub created_at_epoch_seconds: u64,
    pub approved_surfaces: Vec<String>,
    pub surfaces: Vec<RustGuardedProductionDefaultForwardingApplySurface>,
    pub post_apply_hold_evidence_path: Option<String>,
    pub post_apply_hold_evidence_checksum: Option<String>,
    pub rollback_checkpoint_path: Option<String>,
    pub mutates_runtime: bool,
    pub production_default_forwarding_applied: bool,
    pub post_apply_hold_verified: bool,
    pub next_safe_batch: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RustGuardedProductionDefaultForwardingApplyReport {
    pub runtime_id: String,
    pub component: String,
    pub kernel_area: String,
    pub status: RustGuardedProductionDefaultForwardingApplyStatus,
    pub reason: String,
    pub explicit_opt_in: bool,
    pub commit_apply: bool,
    pub verify_post_apply_hold: bool,
    pub approved_surfaces: Vec<String>,
    pub apply_manifest: Option<RustGuardedProductionDefaultForwardingApplyManifest>,
    pub apply_manifest_path: Option<String>,
    pub evidence_path: Option<String>,
    pub rollback_checkpoint: RustGuardedProductionDefaultForwardingRollbackCheckpoint,
    pub rollback_checkpoint_path: Option<String>,
    pub apply_manifest_checksum: Option<String>,
    pub post_apply_hold_gate: Option<RustDefaultForwardingHoldBlockerReport>,
    pub mutates_runtime: bool,
    pub writes_apply_manifest: bool,
    pub writes_rollback_checkpoint: bool,
    pub writes_evidence: bool,
    pub production_default_forwarding_applied: bool,
    pub post_apply_hold_verified: bool,
    pub mihomo_default_forwarding_fallback_required: bool,
    pub blockers_reduced: Vec<String>,
    pub blockers_remaining: Vec<String>,
    pub blockers: Vec<String>,
    pub warnings: Vec<String>,
    pub facts: Vec<String>,
    pub next_safe_batch: String,
}

pub async fn rust_guarded_production_default_forwarding_apply(
    explicit_opt_in: bool,
    commit_apply: bool,
    verify_post_apply_hold: bool,
) -> Result<RustGuardedProductionDefaultForwardingApplyReport> {
    let approved_surfaces = approved_production_default_forwarding_cutover_surfaces().await?;
    let post_apply_hold_gate = if verify_post_apply_hold {
        Some(rust_default_forwarding_hold_blocker_reduction(explicit_opt_in).await?)
    } else {
        None
    };
    let mut blockers = apply_blockers(
        explicit_opt_in,
        commit_apply,
        verify_post_apply_hold,
        &approved_surfaces,
        post_apply_hold_gate.as_ref(),
    );
    blockers.sort();
    blockers.dedup();

    let apply_committed = commit_apply && blockers.is_empty();
    let hold_verified = post_apply_hold_gate
        .as_ref()
        .map(|gate| gate.status == RustDefaultForwardingHoldBlockerStatus::Ready)
        .unwrap_or(false);
    let rollback_checkpoint = rollback_checkpoint(&approved_surfaces, apply_committed, None);
    let mut manifest = apply_manifest(
        &approved_surfaces,
        apply_committed,
        hold_verified,
        post_apply_hold_gate.as_ref(),
        None,
    )?;
    let manifest_yaml = serde_yaml_ng::to_string(&manifest)?;
    let manifest_checksum = hex_sha256(manifest_yaml.as_bytes());
    let mut report = build_report(
        explicit_opt_in,
        commit_apply,
        verify_post_apply_hold,
        approved_surfaces,
        Some(manifest.clone()),
        Some(manifest_checksum),
        rollback_checkpoint,
        post_apply_hold_gate,
        blockers,
    );

    if report.status == RustGuardedProductionDefaultForwardingApplyStatus::Blocked {
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

pub async fn rollback_guarded_production_default_forwarding_apply(
    explicit_opt_in: bool,
) -> Result<RustGuardedProductionDefaultForwardingApplyReport> {
    let checkpoint_path = rollback_checkpoint_path()?;
    let checkpoint_yaml = fs::read_to_string(&checkpoint_path)
        .await
        .with_context(|| format!("failed to read {}", checkpoint_path.display()))?;
    let checkpoint: RustGuardedProductionDefaultForwardingRollbackCheckpoint =
        serde_yaml_ng::from_str(&checkpoint_yaml)
            .with_context(|| format!("failed to parse {}", checkpoint_path.display()))?;
    let approved_surfaces = checkpoint.applied_surfaces.clone();
    let rollback_evidence_path = rollback_evidence_path()?;
    let mut report = build_report(
        explicit_opt_in,
        false,
        false,
        approved_surfaces,
        None,
        None,
        checkpoint,
        None,
        if explicit_opt_in {
            Vec::new()
        } else {
            vec!["explicit opt-in is required before rollback restore".to_owned()]
        },
    );

    if !report.blockers.is_empty() {
        return Ok(report);
    }

    report.status = RustGuardedProductionDefaultForwardingApplyStatus::RolledBack;
    report.reason = "guarded production default forwarding apply restored to Mihomo fallback checkpoint".to_owned();
    report.evidence_path = Some(rollback_evidence_path.to_string_lossy().to_string());
    report.writes_evidence = true;
    report.production_default_forwarding_applied = false;
    report.mutates_runtime = true;
    report.mihomo_default_forwarding_fallback_required = true;
    report.blockers_reduced = vec!["guarded production default forwarding apply rollback restored".to_owned()];
    report.blockers_remaining =
        vec!["post-rollback hold verification required before re-applying Rust default forwarding".to_owned()];
    if let Some(parent) = rollback_evidence_path.parent() {
        fs::create_dir_all(parent).await?;
    }
    fs::write(&rollback_evidence_path, serde_yaml_ng::to_string(&report)?.as_bytes()).await?;
    Ok(report)
}

pub async fn applied_guarded_production_default_forwarding_surfaces() -> Result<Vec<String>> {
    let Some(manifest) = read_apply_manifest().await? else {
        return Ok(Vec::new());
    };

    Ok(manifest
        .surfaces
        .into_iter()
        .filter(|surface| surface.apply_committed && surface.post_apply_hold_verified)
        .map(|surface| surface.default_surface)
        .collect())
}

fn build_report(
    explicit_opt_in: bool,
    commit_apply: bool,
    verify_post_apply_hold: bool,
    approved_surfaces: Vec<String>,
    apply_manifest: Option<RustGuardedProductionDefaultForwardingApplyManifest>,
    apply_manifest_checksum: Option<String>,
    rollback_checkpoint: RustGuardedProductionDefaultForwardingRollbackCheckpoint,
    post_apply_hold_gate: Option<RustDefaultForwardingHoldBlockerReport>,
    blockers: Vec<String>,
) -> RustGuardedProductionDefaultForwardingApplyReport {
    let status = if blockers.is_empty() && commit_apply && verify_post_apply_hold {
        RustGuardedProductionDefaultForwardingApplyStatus::Verified
    } else if blockers.is_empty() && commit_apply {
        RustGuardedProductionDefaultForwardingApplyStatus::Applied
    } else if blockers.is_empty() {
        RustGuardedProductionDefaultForwardingApplyStatus::Ready
    } else {
        RustGuardedProductionDefaultForwardingApplyStatus::Blocked
    };
    let production_default_forwarding_applied = matches!(
        status,
        RustGuardedProductionDefaultForwardingApplyStatus::Applied
            | RustGuardedProductionDefaultForwardingApplyStatus::Verified
    );
    let post_apply_hold_verified = status == RustGuardedProductionDefaultForwardingApplyStatus::Verified;

    RustGuardedProductionDefaultForwardingApplyReport {
        runtime_id: RUST_RUNTIME_ID.to_owned(),
        component: COMPONENT.to_owned(),
        kernel_area: KERNEL_AREA.to_owned(),
        status,
        reason: if post_apply_hold_verified {
            "guarded production default forwarding applied and post-apply hold verified"
        } else if production_default_forwarding_applied {
            "guarded production default forwarding applied with rollback checkpoint retained"
        } else if blockers.is_empty() {
            "guarded production default forwarding apply is ready once commit_apply is requested"
        } else {
            "guarded production default forwarding apply is blocked"
        }
        .to_owned(),
        explicit_opt_in,
        commit_apply,
        verify_post_apply_hold,
        approved_surfaces,
        apply_manifest,
        apply_manifest_path: None,
        evidence_path: None,
        rollback_checkpoint,
        rollback_checkpoint_path: None,
        apply_manifest_checksum,
        post_apply_hold_gate,
        mutates_runtime: production_default_forwarding_applied,
        writes_apply_manifest: false,
        writes_rollback_checkpoint: false,
        writes_evidence: false,
        production_default_forwarding_applied,
        post_apply_hold_verified,
        mihomo_default_forwarding_fallback_required: !production_default_forwarding_applied,
        blockers_reduced: if production_default_forwarding_applied {
            vec![
                "operator-approved production default-forwarding apply committed".to_owned(),
                "Mihomo default-forwarding fallback demoted to checkpoint restore path".to_owned(),
                "post-approval hold evidence consumed in the guarded apply path".to_owned(),
            ]
        } else {
            Vec::new()
        },
        blockers_remaining: if production_default_forwarding_applied {
            vec![
                "real remote encrypted/QUIC peer compatibility".to_owned(),
                "operator-approved real plugin binary compatibility".to_owned(),
                "TUN packet-capture hold bundle".to_owned(),
                "final Mihomo binary removal gate".to_owned(),
            ]
        } else {
            vec!["guarded production default-forwarding apply has not been committed".to_owned()]
        },
        blockers,
        warnings: vec![
            "guarded apply is operator-controlled and keeps rollback checkpoint evidence".to_owned(),
            "Mihomo binary removal remains blocked until later closeout gates pass".to_owned(),
        ],
        facts: vec![
            "approved cutover surfaces are consumed from the production default-forwarding approval manifest"
                .to_owned(),
            "post-apply hold verification reuses the default-forwarding hold blocker evidence path".to_owned(),
            "runtime mutation is reported only when commit_apply passes all blockers".to_owned(),
        ],
        next_safe_batch: NEXT_SAFE_BATCH.to_owned(),
    }
}

fn apply_blockers(
    explicit_opt_in: bool,
    commit_apply: bool,
    verify_post_apply_hold: bool,
    approved_surfaces: &[String],
    post_apply_hold_gate: Option<&RustDefaultForwardingHoldBlockerReport>,
) -> Vec<String> {
    let mut blockers = Vec::new();
    if !explicit_opt_in {
        blockers.push("explicit opt-in is required before guarded production default-forwarding apply".to_owned());
    }
    if approved_surfaces.is_empty() {
        blockers.push("committed production default-forwarding cutover approval manifest is required".to_owned());
    }
    if !commit_apply {
        blockers.push("commit_apply is required to demote Mihomo default-forwarding fallback".to_owned());
    }
    if !verify_post_apply_hold {
        blockers.push("post-apply hold verification is required in the guarded apply bundle".to_owned());
    }
    match post_apply_hold_gate {
        Some(gate) if gate.status == RustDefaultForwardingHoldBlockerStatus::Ready => {}
        Some(_) => blockers.push("post-apply default-forwarding hold evidence is blocked".to_owned()),
        None if verify_post_apply_hold => {
            blockers.push("post-apply default-forwarding hold evidence did not run".to_owned());
        }
        None => {}
    }
    blockers
}

fn apply_manifest(
    approved_surfaces: &[String],
    apply_committed: bool,
    hold_verified: bool,
    post_apply_hold_gate: Option<&RustDefaultForwardingHoldBlockerReport>,
    rollback_checkpoint_path: Option<String>,
) -> Result<RustGuardedProductionDefaultForwardingApplyManifest> {
    let post_apply_hold_evidence_path = post_apply_hold_gate.and_then(|gate| gate.evidence_path.clone());
    let post_apply_hold_evidence_checksum = post_apply_hold_gate
        .and_then(|gate| serde_yaml_ng::to_string(gate).ok())
        .map(|yaml| hex_sha256(yaml.as_bytes()));
    Ok(RustGuardedProductionDefaultForwardingApplyManifest {
        component: COMPONENT.to_owned(),
        created_at_epoch_seconds: current_epoch_seconds(),
        approved_surfaces: approved_surfaces.to_vec(),
        surfaces: approved_surfaces
            .iter()
            .map(|surface| RustGuardedProductionDefaultForwardingApplySurface {
                default_surface: surface.clone(),
                approval_committed: true,
                apply_committed,
                post_apply_hold_verified: hold_verified,
                rust_owner_after_apply: "Rust default-forwarding guarded apply".to_owned(),
                mihomo_fallback_required_after_apply: !apply_committed,
                rollback_checkpoint_required: true,
                blockers: if apply_committed && hold_verified {
                    Vec::new()
                } else {
                    vec!["surface requires committed apply and post-apply hold verification".to_owned()]
                },
            })
            .collect(),
        post_apply_hold_evidence_path,
        post_apply_hold_evidence_checksum,
        rollback_checkpoint_path,
        mutates_runtime: apply_committed,
        production_default_forwarding_applied: apply_committed,
        post_apply_hold_verified: hold_verified,
        next_safe_batch: NEXT_SAFE_BATCH.to_owned(),
    })
}

fn rollback_checkpoint(
    approved_surfaces: &[String],
    apply_committed: bool,
    checkpoint_path: Option<String>,
) -> RustGuardedProductionDefaultForwardingRollbackCheckpoint {
    RustGuardedProductionDefaultForwardingRollbackCheckpoint {
        component: COMPONENT.to_owned(),
        created_at_epoch_seconds: current_epoch_seconds(),
        applied_surfaces: if apply_committed {
            approved_surfaces.to_vec()
        } else {
            Vec::new()
        },
        restore_owner: "Mihomo default-forwarding fallback".to_owned(),
        rollback_actions: vec![
            "restore Mihomo default-forwarding ownership for approved surfaces".to_owned(),
            "re-run default-forwarding hold evidence before re-applying Rust ownership".to_owned(),
            "retain sidecar binary until final removal closeout passes".to_owned(),
        ],
        checkpoint_path,
    }
}

async fn read_apply_manifest() -> Result<Option<RustGuardedProductionDefaultForwardingApplyManifest>> {
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

pub fn rust_guarded_production_default_forwarding_apply_evidence_path() -> Result<std::path::PathBuf> {
    Ok(component_dir()?.join(EVIDENCE_FILE))
}

fn evidence_path() -> Result<std::path::PathBuf> {
    rust_guarded_production_default_forwarding_apply_evidence_path()
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
    fn applied_report_demotes_mihomo_default_forwarding_fallback() {
        let report = build_report(
            true,
            true,
            true,
            vec!["TCP HTTP default forwarding".to_owned()],
            None,
            None,
            rollback_checkpoint(&["TCP HTTP default forwarding".to_owned()], true, None),
            None,
            Vec::new(),
        );

        assert_eq!(
            report.status,
            RustGuardedProductionDefaultForwardingApplyStatus::Verified
        );
        assert!(report.production_default_forwarding_applied);
        assert!(report.mutates_runtime);
        assert!(!report.mihomo_default_forwarding_fallback_required);
        assert_eq!(report.next_safe_batch, NEXT_SAFE_BATCH);
    }

    #[test]
    fn missing_commit_apply_blocks_fallback_demotion() {
        let blockers = apply_blockers(true, false, true, &["SOCKS UDP default forwarding".to_owned()], None);

        assert!(blockers.iter().any(|blocker| blocker.contains("commit_apply")));
    }
}
