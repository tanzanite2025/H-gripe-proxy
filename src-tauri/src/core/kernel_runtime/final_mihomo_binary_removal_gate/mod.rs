use super::{
    GoToRustMigrationFinalReviewReport, GoToRustMigrationFinalReviewStatus, RUST_RUNTIME_ID,
    RustSidecarIndependentRollbackReport, RustSidecarIndependentRollbackStatus,
    approved_operator_default_path_cutover_surfaces, fallback_retirement_closed_out_surfaces,
    go_to_rust_migration_final_review, rust_sidecar_independent_rollback_archive,
};
use crate::utils::dirs;
use anyhow::{Context as _, Result};
use serde::{Deserialize, Serialize};
use sha2::{Digest as _, Sha256};
use std::{
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};
use tokio::fs;

const COMPONENT: &str = "rust-final-mihomo-binary-removal-gate";
const KERNEL_AREA: &str = "final-mihomo-binary-removal-gate";
const REMOVAL_MANIFEST_FILE: &str = "removal-manifest.yaml";
const EVIDENCE_FILE: &str = "evidence.yaml";
const ROLLBACK_CHECKPOINT_FILE: &str = "rollback-checkpoint.yaml";
const ROLLBACK_EVIDENCE_FILE: &str = "rollback-evidence.yaml";
const NEXT_SAFE_BATCH: &str = "go-to-rust-migration-release-closeout";

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum RustFinalMihomoBinaryRemovalGateStatus {
    Ready,
    Blocked,
    RemovalAllowed,
    Verified,
    RolledBack,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RustFinalMihomoBinaryRemovalAuditFile {
    pub path: String,
    pub expected_to_remain: bool,
    pub present: bool,
    pub removal_blocker: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RustFinalMihomoBinaryRemovalAudit {
    pub checked_files: Vec<RustFinalMihomoBinaryRemovalAuditFile>,
    pub fallback_closeout_surfaces: Vec<String>,
    pub operator_cutover_surfaces: Vec<String>,
    pub final_review_passed: bool,
    pub sidecar_independent_rollback_ready: bool,
    pub release_blockers: Vec<String>,
    pub passed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RustFinalMihomoBinaryRemovalRollbackCheckpoint {
    pub component: String,
    pub created_at_epoch_seconds: u64,
    pub removal_surfaces: Vec<String>,
    pub restore_owner: String,
    pub restore_actions: Vec<String>,
    pub checkpoint_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RustFinalMihomoBinaryRemovalManifest {
    pub component: String,
    pub created_at_epoch_seconds: u64,
    pub fallback_closeout_surfaces: Vec<String>,
    pub operator_cutover_surfaces: Vec<String>,
    pub removal_audit: RustFinalMihomoBinaryRemovalAudit,
    pub final_review_evidence_path: Option<String>,
    pub sidecar_independent_rollback_path: Option<String>,
    pub rollback_checkpoint_path: Option<String>,
    pub evidence_checksum: String,
    pub mutates_runtime: bool,
    pub mihomo_binary_removal_allowed: bool,
    pub removes_mihomo_fallback_binary: bool,
    pub next_safe_batch: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RustFinalMihomoBinaryRemovalGateReport {
    pub runtime_id: String,
    pub component: String,
    pub kernel_area: String,
    pub status: RustFinalMihomoBinaryRemovalGateStatus,
    pub reason: String,
    pub explicit_opt_in: bool,
    pub operator_approved: bool,
    pub commit_removal_gate: bool,
    pub verify_release_audit: bool,
    pub fallback_closeout_surfaces: Vec<String>,
    pub operator_cutover_surfaces: Vec<String>,
    pub final_review_gate: Option<GoToRustMigrationFinalReviewReport>,
    pub sidecar_independent_rollback_gate: Option<RustSidecarIndependentRollbackReport>,
    pub release_audit: RustFinalMihomoBinaryRemovalAudit,
    pub removal_manifest: Option<RustFinalMihomoBinaryRemovalManifest>,
    pub removal_manifest_path: Option<String>,
    pub evidence_path: Option<String>,
    pub rollback_checkpoint: RustFinalMihomoBinaryRemovalRollbackCheckpoint,
    pub rollback_checkpoint_path: Option<String>,
    pub removal_manifest_checksum: Option<String>,
    pub mutates_runtime: bool,
    pub writes_removal_manifest: bool,
    pub writes_rollback_checkpoint: bool,
    pub writes_evidence: bool,
    pub mihomo_binary_removal_allowed: bool,
    pub removes_mihomo_fallback_binary: bool,
    pub rollback_restore_required: bool,
    pub blockers_reduced: Vec<String>,
    pub blockers_remaining: Vec<String>,
    pub blockers: Vec<String>,
    pub warnings: Vec<String>,
    pub facts: Vec<String>,
    pub next_safe_batch: String,
}

pub async fn rust_final_mihomo_binary_removal_gate(
    explicit_opt_in: bool,
    operator_approved: bool,
    commit_removal_gate: bool,
    verify_release_audit: bool,
) -> Result<RustFinalMihomoBinaryRemovalGateReport> {
    let fallback_closeout_surfaces = fallback_retirement_closed_out_surfaces().await?;
    let operator_cutover_surfaces = approved_operator_default_path_cutover_surfaces().await?;
    let final_review_gate = Some(go_to_rust_migration_final_review(explicit_opt_in).await?);
    let sidecar_independent_rollback_gate = Some(rust_sidecar_independent_rollback_archive(explicit_opt_in).await?);
    let release_audit = release_audit(
        final_review_gate.as_ref(),
        sidecar_independent_rollback_gate.as_ref(),
        &fallback_closeout_surfaces,
        &operator_cutover_surfaces,
    )
    .await?;

    let mut blockers = gate_blockers(
        explicit_opt_in,
        operator_approved,
        commit_removal_gate,
        verify_release_audit,
        final_review_gate.as_ref(),
        sidecar_independent_rollback_gate.as_ref(),
        &fallback_closeout_surfaces,
        &operator_cutover_surfaces,
        &release_audit,
    );
    blockers.sort();
    blockers.dedup();

    let removal_allowed = commit_removal_gate && blockers.is_empty();
    let rollback_checkpoint = rollback_checkpoint(removal_allowed, None);
    let mut manifest = removal_manifest(
        removal_allowed,
        final_review_gate.as_ref(),
        sidecar_independent_rollback_gate.as_ref(),
        fallback_closeout_surfaces.clone(),
        operator_cutover_surfaces.clone(),
        release_audit.clone(),
        None,
    )?;
    let manifest_checksum = hex_sha256(serde_yaml_ng::to_string(&manifest)?.as_bytes());

    let mut report = build_report(
        explicit_opt_in,
        operator_approved,
        commit_removal_gate,
        verify_release_audit,
        fallback_closeout_surfaces,
        operator_cutover_surfaces,
        final_review_gate,
        sidecar_independent_rollback_gate,
        release_audit,
        Some(manifest.clone()),
        Some(manifest_checksum),
        rollback_checkpoint,
        blockers,
    );

    if report.status == RustFinalMihomoBinaryRemovalGateStatus::Blocked {
        return Ok(report);
    }

    if commit_removal_gate {
        let removal_manifest_path = removal_manifest_path()?;
        let evidence_path = evidence_path()?;
        let rollback_checkpoint_path = rollback_checkpoint_path()?;
        if let Some(parent) = removal_manifest_path.parent() {
            fs::create_dir_all(parent).await?;
        }

        manifest.rollback_checkpoint_path = Some(rollback_checkpoint_path.to_string_lossy().to_string());
        report.removal_manifest = Some(manifest.clone());
        report.removal_manifest_path = Some(removal_manifest_path.to_string_lossy().to_string());
        report.evidence_path = Some(evidence_path.to_string_lossy().to_string());
        report.rollback_checkpoint_path = Some(rollback_checkpoint_path.to_string_lossy().to_string());
        report.rollback_checkpoint.checkpoint_path = Some(rollback_checkpoint_path.to_string_lossy().to_string());
        report.removal_manifest_checksum = Some(hex_sha256(serde_yaml_ng::to_string(&manifest)?.as_bytes()));
        report.writes_removal_manifest = true;
        report.writes_rollback_checkpoint = true;
        report.writes_evidence = true;

        fs::write(
            &rollback_checkpoint_path,
            serde_yaml_ng::to_string(&report.rollback_checkpoint)?.as_bytes(),
        )
        .await?;
        fs::write(&removal_manifest_path, serde_yaml_ng::to_string(&manifest)?.as_bytes()).await?;
        fs::write(&evidence_path, serde_yaml_ng::to_string(&report)?.as_bytes()).await?;
    }

    Ok(report)
}

pub async fn rollback_final_mihomo_binary_removal_gate(
    explicit_opt_in: bool,
) -> Result<RustFinalMihomoBinaryRemovalGateReport> {
    let checkpoint_path = rollback_checkpoint_path()?;
    let checkpoint_yaml = fs::read_to_string(&checkpoint_path)
        .await
        .with_context(|| format!("failed to read {}", checkpoint_path.display()))?;
    let checkpoint: RustFinalMihomoBinaryRemovalRollbackCheckpoint = serde_yaml_ng::from_str(&checkpoint_yaml)
        .with_context(|| format!("failed to parse {}", checkpoint_path.display()))?;
    let mut report = build_report(
        explicit_opt_in,
        true,
        false,
        false,
        Vec::new(),
        Vec::new(),
        None,
        None,
        empty_release_audit(),
        None,
        None,
        checkpoint,
        if explicit_opt_in {
            Vec::new()
        } else {
            vec!["explicit opt-in is required before final binary removal rollback".to_owned()]
        },
    );

    if !report.blockers.is_empty() {
        return Ok(report);
    }

    let rollback_evidence_path = rollback_evidence_path()?;
    report.status = RustFinalMihomoBinaryRemovalGateStatus::RolledBack;
    report.reason = "final Mihomo binary removal gate restored rollback checkpoint".to_owned();
    report.evidence_path = Some(rollback_evidence_path.to_string_lossy().to_string());
    report.writes_evidence = true;
    report.mutates_runtime = true;
    report.mihomo_binary_removal_allowed = false;
    report.removes_mihomo_fallback_binary = false;
    report.rollback_restore_required = true;
    report.blockers_reduced = vec!["final Mihomo binary removal gate rollback restored".to_owned()];
    report.blockers_remaining = vec![
        "re-run fallback retirement closeout before allowing binary removal".to_owned(),
        "release closeout remains blocked after rollback".to_owned(),
    ];
    if let Some(parent) = rollback_evidence_path.parent() {
        fs::create_dir_all(parent).await?;
    }
    fs::write(&rollback_evidence_path, serde_yaml_ng::to_string(&report)?.as_bytes()).await?;
    Ok(report)
}

pub async fn final_mihomo_binary_removal_allowed() -> Result<bool> {
    let Some(manifest) = read_removal_manifest().await? else {
        return Ok(false);
    };

    Ok(manifest.mihomo_binary_removal_allowed && manifest.removes_mihomo_fallback_binary)
}

#[allow(clippy::too_many_arguments)]
fn build_report(
    explicit_opt_in: bool,
    operator_approved: bool,
    commit_removal_gate: bool,
    verify_release_audit: bool,
    fallback_closeout_surfaces: Vec<String>,
    operator_cutover_surfaces: Vec<String>,
    final_review_gate: Option<GoToRustMigrationFinalReviewReport>,
    sidecar_independent_rollback_gate: Option<RustSidecarIndependentRollbackReport>,
    release_audit: RustFinalMihomoBinaryRemovalAudit,
    removal_manifest: Option<RustFinalMihomoBinaryRemovalManifest>,
    removal_manifest_checksum: Option<String>,
    rollback_checkpoint: RustFinalMihomoBinaryRemovalRollbackCheckpoint,
    blockers: Vec<String>,
) -> RustFinalMihomoBinaryRemovalGateReport {
    let status = if blockers.is_empty() && commit_removal_gate && verify_release_audit {
        RustFinalMihomoBinaryRemovalGateStatus::Verified
    } else if blockers.is_empty() && commit_removal_gate {
        RustFinalMihomoBinaryRemovalGateStatus::RemovalAllowed
    } else if blockers.is_empty() {
        RustFinalMihomoBinaryRemovalGateStatus::Ready
    } else {
        RustFinalMihomoBinaryRemovalGateStatus::Blocked
    };
    let removal_allowed = matches!(
        status,
        RustFinalMihomoBinaryRemovalGateStatus::RemovalAllowed | RustFinalMihomoBinaryRemovalGateStatus::Verified
    );

    RustFinalMihomoBinaryRemovalGateReport {
        runtime_id: RUST_RUNTIME_ID.to_owned(),
        component: COMPONENT.to_owned(),
        kernel_area: KERNEL_AREA.to_owned(),
        status,
        reason: if status == RustFinalMihomoBinaryRemovalGateStatus::Verified {
            "final Mihomo binary removal gate committed and release audit verified"
        } else if removal_allowed {
            "final Mihomo binary removal gate committed with rollback checkpoint retained"
        } else if blockers.is_empty() {
            "final Mihomo binary removal gate is ready once commit_removal_gate is requested"
        } else {
            "final Mihomo binary removal gate is blocked"
        }
        .to_owned(),
        explicit_opt_in,
        operator_approved,
        commit_removal_gate,
        verify_release_audit,
        fallback_closeout_surfaces,
        operator_cutover_surfaces,
        final_review_gate,
        sidecar_independent_rollback_gate,
        release_audit,
        removal_manifest,
        removal_manifest_path: None,
        evidence_path: None,
        rollback_checkpoint,
        rollback_checkpoint_path: None,
        removal_manifest_checksum,
        mutates_runtime: removal_allowed,
        writes_removal_manifest: false,
        writes_rollback_checkpoint: false,
        writes_evidence: false,
        mihomo_binary_removal_allowed: removal_allowed,
        removes_mihomo_fallback_binary: removal_allowed,
        rollback_restore_required: !removal_allowed,
        blockers_reduced: if removal_allowed {
            vec![
                "fallback-retirement closeout consumed for final binary removal".to_owned(),
                "sidecar-independent rollback consumed for final binary removal".to_owned(),
                "operator-approved sidecar binary removal promoted to removal manifest".to_owned(),
                "Mihomo binary removal advanced to release closeout".to_owned(),
            ]
        } else {
            Vec::new()
        },
        blockers_remaining: if removal_allowed {
            vec![
                "release closeout artifact cleanup".to_owned(),
                "distribution packaging audit".to_owned(),
            ]
        } else {
            vec!["final Mihomo binary removal gate has not been committed".to_owned()]
        },
        blockers,
        warnings: vec![
            "this gate authorizes final binary removal but preserves rollback evidence".to_owned(),
            "distribution cleanup remains separate from runtime evidence generation".to_owned(),
        ],
        facts: vec![
            "final removal consumes fallback closeout and final review evidence".to_owned(),
            "operator default-path cutover must include Mihomo sidecar binary removal".to_owned(),
            "rollback checkpoint is written before the removal manifest is committed".to_owned(),
        ],
        next_safe_batch: NEXT_SAFE_BATCH.to_owned(),
    }
}

#[allow(clippy::too_many_arguments)]
fn gate_blockers(
    explicit_opt_in: bool,
    operator_approved: bool,
    commit_removal_gate: bool,
    verify_release_audit: bool,
    final_review_gate: Option<&GoToRustMigrationFinalReviewReport>,
    sidecar_independent_rollback_gate: Option<&RustSidecarIndependentRollbackReport>,
    fallback_closeout_surfaces: &[String],
    operator_cutover_surfaces: &[String],
    release_audit: &RustFinalMihomoBinaryRemovalAudit,
) -> Vec<String> {
    let mut blockers = Vec::new();
    if !explicit_opt_in {
        blockers.push("explicit opt-in is required before final Mihomo binary removal gate".to_owned());
    }
    if !operator_approved {
        blockers.push("operator approval is required before final Mihomo binary removal gate".to_owned());
    }
    if !commit_removal_gate {
        blockers.push("commit_removal_gate is required to allow Mihomo binary removal".to_owned());
    }
    if !verify_release_audit {
        blockers.push("release audit verification is required in the final removal gate".to_owned());
    }
    if fallback_closeout_surfaces.is_empty() {
        blockers.push("fallback retirement closeout surfaces are required".to_owned());
    }
    if !operator_cutover_surfaces
        .iter()
        .any(|surface| surface == "Mihomo sidecar binary removal")
    {
        blockers.push("operator default-path cutover must approve Mihomo sidecar binary removal".to_owned());
    }
    if !matches!(
        final_review_gate.map(|gate| gate.status),
        Some(GoToRustMigrationFinalReviewStatus::Passed)
    ) {
        blockers.push("Go-to-Rust final review must pass before binary removal".to_owned());
    }
    if !matches!(
        sidecar_independent_rollback_gate.map(|gate| gate.status),
        Some(RustSidecarIndependentRollbackStatus::Ready)
    ) {
        blockers.push("sidecar-independent rollback archive must be ready".to_owned());
    }
    blockers.extend(release_audit.release_blockers.iter().cloned());
    blockers
}

async fn release_audit(
    final_review_gate: Option<&GoToRustMigrationFinalReviewReport>,
    sidecar_independent_rollback_gate: Option<&RustSidecarIndependentRollbackReport>,
    fallback_closeout_surfaces: &[String],
    operator_cutover_surfaces: &[String],
) -> Result<RustFinalMihomoBinaryRemovalAudit> {
    let checked_files = release_audit_files().await?;
    let mut release_blockers = Vec::new();
    if checked_files.iter().any(|file| file.removal_blocker) {
        release_blockers.push("release audit still contains Mihomo sidecar runtime references".to_owned());
    }
    if fallback_closeout_surfaces.is_empty() {
        release_blockers.push("fallback closeout manifest has no closed-out surfaces".to_owned());
    }
    if !operator_cutover_surfaces
        .iter()
        .any(|surface| surface == "Mihomo sidecar binary removal")
    {
        release_blockers.push("operator cutover did not include sidecar binary removal".to_owned());
    }
    let final_review_passed = matches!(
        final_review_gate.map(|gate| gate.status),
        Some(GoToRustMigrationFinalReviewStatus::Passed)
    );
    if !final_review_passed {
        release_blockers.push("final review has not passed".to_owned());
    }
    let sidecar_independent_rollback_ready = matches!(
        sidecar_independent_rollback_gate.map(|gate| gate.status),
        Some(RustSidecarIndependentRollbackStatus::Ready)
    );
    if !sidecar_independent_rollback_ready {
        release_blockers.push("sidecar-independent rollback is not ready".to_owned());
    }

    Ok(RustFinalMihomoBinaryRemovalAudit {
        checked_files,
        fallback_closeout_surfaces: fallback_closeout_surfaces.to_vec(),
        operator_cutover_surfaces: operator_cutover_surfaces.to_vec(),
        final_review_passed,
        sidecar_independent_rollback_ready,
        passed: release_blockers.is_empty(),
        release_blockers,
    })
}

async fn release_audit_files() -> Result<Vec<RustFinalMihomoBinaryRemovalAuditFile>> {
    let root = repo_root()?;
    let specs = [
        ("src-tauri/tauri.conf.json", true),
        ("src-tauri/build.rs", true),
        ("README.md", true),
        ("docs/go-to-rust-migration-roadmap.md", true),
    ];
    let mut files = Vec::new();
    for (relative, expected_to_remain) in specs {
        let path = root.join(relative);
        let present = fs::metadata(&path).await.is_ok();
        let contents = if present {
            fs::read_to_string(&path).await.unwrap_or_default()
        } else {
            String::new()
        };
        let removal_blocker =
            present && !expected_to_remain && (contents.contains("verge-mihomo") || contents.contains("sidecar"));
        files.push(RustFinalMihomoBinaryRemovalAuditFile {
            path: relative.to_owned(),
            expected_to_remain,
            present,
            removal_blocker,
        });
    }
    Ok(files)
}

fn removal_manifest(
    removal_allowed: bool,
    final_review_gate: Option<&GoToRustMigrationFinalReviewReport>,
    sidecar_independent_rollback_gate: Option<&RustSidecarIndependentRollbackReport>,
    fallback_closeout_surfaces: Vec<String>,
    operator_cutover_surfaces: Vec<String>,
    removal_audit: RustFinalMihomoBinaryRemovalAudit,
    rollback_checkpoint_path: Option<String>,
) -> Result<RustFinalMihomoBinaryRemovalManifest> {
    let final_review_evidence_path = final_review_gate.and_then(|gate| gate.evidence_path.clone());
    let sidecar_independent_rollback_path =
        sidecar_independent_rollback_gate.and_then(|gate| gate.rollback_plan_path.clone());
    let evidence_checksum = hex_sha256(
        [
            final_review_evidence_path.clone(),
            sidecar_independent_rollback_path.clone(),
        ]
        .into_iter()
        .flatten()
        .collect::<Vec<_>>()
        .join("\n")
        .as_bytes(),
    );

    Ok(RustFinalMihomoBinaryRemovalManifest {
        component: COMPONENT.to_owned(),
        created_at_epoch_seconds: current_epoch_seconds(),
        fallback_closeout_surfaces,
        operator_cutover_surfaces,
        removal_audit,
        final_review_evidence_path,
        sidecar_independent_rollback_path,
        rollback_checkpoint_path,
        evidence_checksum,
        mutates_runtime: removal_allowed,
        mihomo_binary_removal_allowed: removal_allowed,
        removes_mihomo_fallback_binary: removal_allowed,
        next_safe_batch: NEXT_SAFE_BATCH.to_owned(),
    })
}

fn rollback_checkpoint(
    removal_allowed: bool,
    checkpoint_path: Option<String>,
) -> RustFinalMihomoBinaryRemovalRollbackCheckpoint {
    RustFinalMihomoBinaryRemovalRollbackCheckpoint {
        component: COMPONENT.to_owned(),
        created_at_epoch_seconds: current_epoch_seconds(),
        removal_surfaces: if removal_allowed {
            vec!["Mihomo sidecar binary removal".to_owned()]
        } else {
            Vec::new()
        },
        restore_owner: "sidecar-independent rollback archive".to_owned(),
        restore_actions: vec![
            "restore archived sidecar-independent rollback artifacts".to_owned(),
            "re-run fallback retirement closeout before allowing removal again".to_owned(),
            "block release closeout until packaging audit is refreshed".to_owned(),
        ],
        checkpoint_path,
    }
}

async fn read_removal_manifest() -> Result<Option<RustFinalMihomoBinaryRemovalManifest>> {
    let path = removal_manifest_path()?;
    if fs::metadata(&path).await.is_err() {
        return Ok(None);
    }
    let yaml = fs::read_to_string(&path)
        .await
        .with_context(|| format!("failed to read {}", path.display()))?;
    let manifest = serde_yaml_ng::from_str(&yaml).with_context(|| format!("failed to parse {}", path.display()))?;
    Ok(Some(manifest))
}

fn removal_manifest_path() -> Result<PathBuf> {
    Ok(component_dir()?.join(REMOVAL_MANIFEST_FILE))
}

pub fn rust_final_mihomo_binary_removal_gate_evidence_path() -> Result<PathBuf> {
    Ok(component_dir()?.join(EVIDENCE_FILE))
}

fn evidence_path() -> Result<PathBuf> {
    rust_final_mihomo_binary_removal_gate_evidence_path()
}

fn rollback_checkpoint_path() -> Result<PathBuf> {
    Ok(component_dir()?.join(ROLLBACK_CHECKPOINT_FILE))
}

fn rollback_evidence_path() -> Result<PathBuf> {
    Ok(component_dir()?.join(ROLLBACK_EVIDENCE_FILE))
}

fn component_dir() -> Result<PathBuf> {
    Ok(dirs::app_runtime_dir()?.join(COMPONENT))
}

fn repo_root() -> Result<PathBuf> {
    let mut dir = std::env::current_dir()?;
    loop {
        if dir.join("src-tauri").is_dir() && dir.join("README.md").is_file() {
            return Ok(dir);
        }
        if !dir.pop() {
            return std::env::current_dir().context("failed to resolve repository root");
        }
    }
}

fn empty_release_audit() -> RustFinalMihomoBinaryRemovalAudit {
    RustFinalMihomoBinaryRemovalAudit {
        checked_files: Vec::new(),
        fallback_closeout_surfaces: Vec::new(),
        operator_cutover_surfaces: Vec::new(),
        final_review_passed: false,
        sidecar_independent_rollback_ready: false,
        release_blockers: Vec::new(),
        passed: false,
    }
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
    fn verified_gate_allows_removal_without_release_cleanup() {
        let report = build_report(
            true,
            true,
            true,
            true,
            vec!["production default-forwarding fallback".to_owned()],
            vec!["Mihomo sidecar binary removal".to_owned()],
            None,
            None,
            empty_release_audit(),
            None,
            None,
            rollback_checkpoint(true, None),
            Vec::new(),
        );

        assert_eq!(report.status, RustFinalMihomoBinaryRemovalGateStatus::Verified);
        assert!(report.mihomo_binary_removal_allowed);
        assert!(report.removes_mihomo_fallback_binary);
        assert!(report.mutates_runtime);
        assert_eq!(report.next_safe_batch, NEXT_SAFE_BATCH);
    }

    #[test]
    fn missing_operator_cutover_blocks_binary_removal() {
        let blockers = gate_blockers(
            true,
            true,
            true,
            true,
            None,
            None,
            &["surface".to_owned()],
            &[],
            &empty_release_audit(),
        );

        assert!(
            blockers
                .iter()
                .any(|blocker| blocker.contains("sidecar binary removal"))
        );
    }
}
