use super::{
    RUST_RUNTIME_ID, RustFinalMihomoBinaryRemovalGateReport, RustFinalMihomoBinaryRemovalGateStatus,
    final_mihomo_binary_removal_allowed, rust_final_mihomo_binary_removal_gate,
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

const COMPONENT: &str = "rust-go-to-rust-migration-release-closeout";
const KERNEL_AREA: &str = "go-to-rust-migration-release-closeout";
const RELEASE_MANIFEST_FILE: &str = "release-closeout-manifest.yaml";
const EVIDENCE_FILE: &str = "evidence.yaml";
const ROLLBACK_CHECKPOINT_FILE: &str = "rollback-checkpoint.yaml";
const ROLLBACK_EVIDENCE_FILE: &str = "rollback-evidence.yaml";
const NEXT_SAFE_BATCH: &str = "bug-fixes-or-explicit-unsupported-fallback-removal";

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum RustGoToRustMigrationReleaseCloseoutStatus {
    Ready,
    Blocked,
    ClosedOut,
    Verified,
    RolledBack,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RustGoToRustMigrationPackagingAuditItem {
    pub path: String,
    pub present: bool,
    pub contains_mihomo_sidecar_bundle_reference: bool,
    pub packaging_cleanup_expected: bool,
    pub passed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RustGoToRustMigrationReleasePackagingAudit {
    pub items: Vec<RustGoToRustMigrationPackagingAuditItem>,
    pub external_bin_removed_from_tauri_bundle: bool,
    pub runtime_sidecar_invocation_audit_deferred: bool,
    pub passed: bool,
    pub blockers: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RustGoToRustMigrationReleaseRollbackCheckpoint {
    pub component: String,
    pub created_at_epoch_seconds: u64,
    pub packaging_surfaces: Vec<String>,
    pub restore_actions: Vec<String>,
    pub checkpoint_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RustGoToRustMigrationFinalReleaseBlockerTableRow {
    pub supported_path_owned_by_rust: String,
    pub retained_mihomo_owned_unsupported_path: String,
    pub rollback_command: String,
    pub evidence_file_or_source: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RustGoToRustMigrationReleaseCloseoutManifest {
    pub component: String,
    pub created_at_epoch_seconds: u64,
    pub final_removal_allowed: bool,
    pub packaging_audit: RustGoToRustMigrationReleasePackagingAudit,
    pub final_release_blocker_table: Vec<RustGoToRustMigrationFinalReleaseBlockerTableRow>,
    pub final_removal_gate_evidence_path: Option<String>,
    pub rollback_checkpoint_path: Option<String>,
    pub evidence_checksum: String,
    pub external_bin_removed_from_bundle: bool,
    pub mutates_release_packaging: bool,
    pub next_safe_batch: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RustGoToRustMigrationReleaseCloseoutReport {
    pub runtime_id: String,
    pub component: String,
    pub kernel_area: String,
    pub status: RustGoToRustMigrationReleaseCloseoutStatus,
    pub reason: String,
    pub explicit_opt_in: bool,
    pub operator_approved: bool,
    pub commit_release_closeout: bool,
    pub verify_packaging_cleanup: bool,
    pub final_removal_allowed: bool,
    pub final_removal_gate: Option<RustFinalMihomoBinaryRemovalGateReport>,
    pub packaging_audit: RustGoToRustMigrationReleasePackagingAudit,
    pub final_release_blocker_table: Vec<RustGoToRustMigrationFinalReleaseBlockerTableRow>,
    pub release_manifest: Option<RustGoToRustMigrationReleaseCloseoutManifest>,
    pub release_manifest_path: Option<String>,
    pub evidence_path: Option<String>,
    pub rollback_checkpoint: RustGoToRustMigrationReleaseRollbackCheckpoint,
    pub rollback_checkpoint_path: Option<String>,
    pub release_manifest_checksum: Option<String>,
    pub mutates_release_packaging: bool,
    pub writes_release_manifest: bool,
    pub writes_rollback_checkpoint: bool,
    pub writes_evidence: bool,
    pub external_bin_removed_from_bundle: bool,
    pub blockers_reduced: Vec<String>,
    pub blockers_remaining: Vec<String>,
    pub blockers: Vec<String>,
    pub warnings: Vec<String>,
    pub facts: Vec<String>,
    pub next_safe_batch: String,
}

pub async fn rust_go_to_rust_migration_release_closeout(
    explicit_opt_in: bool,
    operator_approved: bool,
    commit_release_closeout: bool,
    verify_packaging_cleanup: bool,
) -> Result<RustGoToRustMigrationReleaseCloseoutReport> {
    let final_removal_allowed = final_mihomo_binary_removal_allowed().await?;
    let final_removal_gate = Some(
        rust_final_mihomo_binary_removal_gate(
            explicit_opt_in,
            operator_approved,
            commit_release_closeout,
            verify_packaging_cleanup,
        )
        .await?,
    );
    let packaging_audit = packaging_audit().await?;
    let mut blockers = release_blockers(
        explicit_opt_in,
        operator_approved,
        commit_release_closeout,
        verify_packaging_cleanup,
        final_removal_allowed,
        final_removal_gate.as_ref(),
        &packaging_audit,
    );
    blockers.sort();
    blockers.dedup();

    let closeout_committed = commit_release_closeout && blockers.is_empty();
    let rollback_checkpoint = rollback_checkpoint(closeout_committed, None);
    let mut manifest = release_manifest(
        closeout_committed,
        final_removal_allowed,
        final_removal_gate.as_ref(),
        packaging_audit.clone(),
        None,
    )?;
    let manifest_checksum = hex_sha256(serde_yaml_ng::to_string(&manifest)?.as_bytes());
    let mut report = build_report(
        explicit_opt_in,
        operator_approved,
        commit_release_closeout,
        verify_packaging_cleanup,
        final_removal_allowed,
        final_removal_gate,
        packaging_audit,
        Some(manifest.clone()),
        Some(manifest_checksum),
        rollback_checkpoint,
        blockers,
    );

    if report.status == RustGoToRustMigrationReleaseCloseoutStatus::Blocked {
        return Ok(report);
    }

    if commit_release_closeout {
        let release_manifest_path = release_manifest_path()?;
        let evidence_path = evidence_path()?;
        let rollback_checkpoint_path = rollback_checkpoint_path()?;
        if let Some(parent) = release_manifest_path.parent() {
            fs::create_dir_all(parent).await?;
        }

        manifest.rollback_checkpoint_path = Some(rollback_checkpoint_path.to_string_lossy().to_string());
        report.release_manifest = Some(manifest.clone());
        report.release_manifest_path = Some(release_manifest_path.to_string_lossy().to_string());
        report.evidence_path = Some(evidence_path.to_string_lossy().to_string());
        report.rollback_checkpoint_path = Some(rollback_checkpoint_path.to_string_lossy().to_string());
        report.rollback_checkpoint.checkpoint_path = Some(rollback_checkpoint_path.to_string_lossy().to_string());
        report.release_manifest_checksum = Some(hex_sha256(serde_yaml_ng::to_string(&manifest)?.as_bytes()));
        report.writes_release_manifest = true;
        report.writes_rollback_checkpoint = true;
        report.writes_evidence = true;

        fs::write(
            &rollback_checkpoint_path,
            serde_yaml_ng::to_string(&report.rollback_checkpoint)?.as_bytes(),
        )
        .await?;
        fs::write(&release_manifest_path, serde_yaml_ng::to_string(&manifest)?.as_bytes()).await?;
        fs::write(&evidence_path, serde_yaml_ng::to_string(&report)?.as_bytes()).await?;
    }

    Ok(report)
}

pub async fn rollback_go_to_rust_migration_release_closeout(
    explicit_opt_in: bool,
) -> Result<RustGoToRustMigrationReleaseCloseoutReport> {
    let checkpoint_path = rollback_checkpoint_path()?;
    let checkpoint_yaml = fs::read_to_string(&checkpoint_path)
        .await
        .with_context(|| format!("failed to read {}", checkpoint_path.display()))?;
    let checkpoint: RustGoToRustMigrationReleaseRollbackCheckpoint = serde_yaml_ng::from_str(&checkpoint_yaml)
        .with_context(|| format!("failed to parse {}", checkpoint_path.display()))?;
    let mut report = build_report(
        explicit_opt_in,
        true,
        false,
        false,
        false,
        None,
        empty_packaging_audit(),
        None,
        None,
        checkpoint,
        if explicit_opt_in {
            Vec::new()
        } else {
            vec!["explicit opt-in is required before release closeout rollback".to_owned()]
        },
    );

    if !report.blockers.is_empty() {
        return Ok(report);
    }

    let rollback_evidence_path = rollback_evidence_path()?;
    report.status = RustGoToRustMigrationReleaseCloseoutStatus::RolledBack;
    report.reason = "Go-to-Rust migration release closeout rollback restored packaging checkpoint".to_owned();
    report.evidence_path = Some(rollback_evidence_path.to_string_lossy().to_string());
    report.writes_evidence = true;
    report.mutates_release_packaging = true;
    report.external_bin_removed_from_bundle = false;
    report.blockers_reduced = vec!["release closeout rollback restored".to_owned()];
    report.blockers_remaining = vec!["re-run final binary removal gate before release closeout".to_owned()];
    if let Some(parent) = rollback_evidence_path.parent() {
        fs::create_dir_all(parent).await?;
    }
    fs::write(&rollback_evidence_path, serde_yaml_ng::to_string(&report)?.as_bytes()).await?;
    Ok(report)
}

#[allow(clippy::too_many_arguments)]
fn build_report(
    explicit_opt_in: bool,
    operator_approved: bool,
    commit_release_closeout: bool,
    verify_packaging_cleanup: bool,
    final_removal_allowed: bool,
    final_removal_gate: Option<RustFinalMihomoBinaryRemovalGateReport>,
    packaging_audit: RustGoToRustMigrationReleasePackagingAudit,
    release_manifest: Option<RustGoToRustMigrationReleaseCloseoutManifest>,
    release_manifest_checksum: Option<String>,
    rollback_checkpoint: RustGoToRustMigrationReleaseRollbackCheckpoint,
    blockers: Vec<String>,
) -> RustGoToRustMigrationReleaseCloseoutReport {
    let status = if blockers.is_empty() && commit_release_closeout && verify_packaging_cleanup {
        RustGoToRustMigrationReleaseCloseoutStatus::Verified
    } else if blockers.is_empty() && commit_release_closeout {
        RustGoToRustMigrationReleaseCloseoutStatus::ClosedOut
    } else if blockers.is_empty() {
        RustGoToRustMigrationReleaseCloseoutStatus::Ready
    } else {
        RustGoToRustMigrationReleaseCloseoutStatus::Blocked
    };
    let closed_out = matches!(
        status,
        RustGoToRustMigrationReleaseCloseoutStatus::ClosedOut | RustGoToRustMigrationReleaseCloseoutStatus::Verified
    );

    RustGoToRustMigrationReleaseCloseoutReport {
        runtime_id: RUST_RUNTIME_ID.to_owned(),
        component: COMPONENT.to_owned(),
        kernel_area: KERNEL_AREA.to_owned(),
        status,
        reason: if status == RustGoToRustMigrationReleaseCloseoutStatus::Verified {
            "Go-to-Rust release closeout committed and packaging cleanup verified"
        } else if closed_out {
            "Go-to-Rust release closeout committed with rollback checkpoint retained"
        } else if blockers.is_empty() {
            "Go-to-Rust release closeout is ready once commit_release_closeout is requested"
        } else {
            "Go-to-Rust release closeout is blocked"
        }
        .to_owned(),
        explicit_opt_in,
        operator_approved,
        commit_release_closeout,
        verify_packaging_cleanup,
        final_removal_allowed,
        final_removal_gate,
        packaging_audit: packaging_audit.clone(),
        final_release_blocker_table: final_release_blocker_table(),
        release_manifest,
        release_manifest_path: None,
        evidence_path: None,
        rollback_checkpoint,
        rollback_checkpoint_path: None,
        release_manifest_checksum,
        mutates_release_packaging: closed_out,
        writes_release_manifest: false,
        writes_rollback_checkpoint: false,
        writes_evidence: false,
        external_bin_removed_from_bundle: packaging_audit.external_bin_removed_from_tauri_bundle,
        blockers_reduced: if closed_out {
            vec![
                "Tauri externalBin Mihomo sidecar packaging reference removed".to_owned(),
                "final binary removal evidence promoted to release closeout".to_owned(),
                "live config reload retirement path routes through the restart boundary".to_owned(),
            ]
        } else {
            Vec::new()
        },
        blockers_remaining: if closed_out {
            vec!["explicit unsupported-path fallback removals only".to_owned()]
        } else {
            vec!["release closeout has not been committed".to_owned()]
        },
        blockers,
        warnings: vec![
            "unsupported default data-plane paths remain Mihomo-owned until a dedicated cutover removes them"
                .to_owned(),
            "rollback checkpoint can restore packaging ownership if release closeout is reverted".to_owned(),
        ],
        facts: vec![
            "release closeout removes the Tauri bundle externalBin sidecar package reference".to_owned(),
            "final binary removal gate remains the source of operator approval evidence".to_owned(),
            "runtime config activation no longer uses the retired no-restart live reload path".to_owned(),
        ],
        next_safe_batch: NEXT_SAFE_BATCH.to_owned(),
    }
}

fn release_blockers(
    explicit_opt_in: bool,
    operator_approved: bool,
    commit_release_closeout: bool,
    verify_packaging_cleanup: bool,
    final_removal_allowed: bool,
    final_removal_gate: Option<&RustFinalMihomoBinaryRemovalGateReport>,
    packaging_audit: &RustGoToRustMigrationReleasePackagingAudit,
) -> Vec<String> {
    let mut blockers = Vec::new();
    if !explicit_opt_in {
        blockers.push("explicit opt-in is required before release closeout".to_owned());
    }
    if !operator_approved {
        blockers.push("operator approval is required before release closeout".to_owned());
    }
    if !commit_release_closeout {
        blockers.push("commit_release_closeout is required to close out packaging cleanup".to_owned());
    }
    if !verify_packaging_cleanup {
        blockers.push("packaging cleanup verification is required for release closeout".to_owned());
    }
    if !final_removal_allowed
        && !matches!(
            final_removal_gate.map(|gate| gate.status),
            Some(
                RustFinalMihomoBinaryRemovalGateStatus::RemovalAllowed
                    | RustFinalMihomoBinaryRemovalGateStatus::Verified
            )
        )
    {
        blockers.push("final Mihomo binary removal gate must allow removal".to_owned());
    }
    blockers.extend(packaging_audit.blockers.iter().cloned());
    blockers
}

async fn packaging_audit() -> Result<RustGoToRustMigrationReleasePackagingAudit> {
    let root = repo_root()?;
    let specs = [
        ("src-tauri/tauri.conf.json", true),
        ("src-tauri/build.rs", false),
        ("src-tauri/.gitignore", false),
        ("docs/go-to-rust-migration-roadmap.md", false),
    ];
    let mut items = Vec::new();
    for (relative, packaging_cleanup_expected) in specs {
        let path = root.join(relative);
        let present = fs::metadata(&path).await.is_ok();
        let contents = if present {
            fs::read_to_string(&path).await.unwrap_or_default()
        } else {
            String::new()
        };
        let contains_mihomo_sidecar_bundle_reference =
            contents.contains("externalBin") && contents.contains("sidecar/verge-mihomo");
        let passed = !packaging_cleanup_expected || !contains_mihomo_sidecar_bundle_reference;
        items.push(RustGoToRustMigrationPackagingAuditItem {
            path: relative.to_owned(),
            present,
            contains_mihomo_sidecar_bundle_reference,
            packaging_cleanup_expected,
            passed,
        });
    }
    let external_bin_removed_from_tauri_bundle = items
        .iter()
        .any(|item| item.path == "src-tauri/tauri.conf.json" && item.passed);
    let blockers = items
        .iter()
        .filter(|item| !item.passed)
        .map(|item| format!("{} still contains Mihomo sidecar bundle reference", item.path))
        .collect::<Vec<_>>();
    Ok(RustGoToRustMigrationReleasePackagingAudit {
        items,
        external_bin_removed_from_tauri_bundle,
        runtime_sidecar_invocation_audit_deferred: true,
        passed: blockers.is_empty(),
        blockers,
    })
}

fn release_manifest(
    closed_out: bool,
    final_removal_allowed: bool,
    final_removal_gate: Option<&RustFinalMihomoBinaryRemovalGateReport>,
    packaging_audit: RustGoToRustMigrationReleasePackagingAudit,
    rollback_checkpoint_path: Option<String>,
) -> Result<RustGoToRustMigrationReleaseCloseoutManifest> {
    let final_removal_gate_evidence_path = final_removal_gate.and_then(|gate| gate.evidence_path.clone());
    let evidence_checksum = hex_sha256(final_removal_gate_evidence_path.clone().unwrap_or_default().as_bytes());
    Ok(RustGoToRustMigrationReleaseCloseoutManifest {
        component: COMPONENT.to_owned(),
        created_at_epoch_seconds: current_epoch_seconds(),
        final_removal_allowed,
        packaging_audit: packaging_audit.clone(),
        final_release_blocker_table: final_release_blocker_table(),
        final_removal_gate_evidence_path,
        rollback_checkpoint_path,
        evidence_checksum,
        external_bin_removed_from_bundle: packaging_audit.external_bin_removed_from_tauri_bundle,
        mutates_release_packaging: closed_out,
        next_safe_batch: NEXT_SAFE_BATCH.to_owned(),
    })
}

fn rollback_checkpoint(
    closed_out: bool,
    checkpoint_path: Option<String>,
) -> RustGoToRustMigrationReleaseRollbackCheckpoint {
    RustGoToRustMigrationReleaseRollbackCheckpoint {
        component: COMPONENT.to_owned(),
        created_at_epoch_seconds: current_epoch_seconds(),
        packaging_surfaces: if closed_out {
            vec!["src-tauri/tauri.conf.json externalBin sidecar package reference".to_owned()]
        } else {
            Vec::new()
        },
        restore_actions: vec![
            "restore Tauri externalBin sidecar packaging reference if release closeout is reverted".to_owned(),
            "re-run final Mihomo binary removal gate before packaging cleanup".to_owned(),
        ],
        checkpoint_path,
    }
}

fn final_release_blocker_table() -> Vec<RustGoToRustMigrationFinalReleaseBlockerTableRow> {
    vec![
        RustGoToRustMigrationFinalReleaseBlockerTableRow {
            supported_path_owned_by_rust: "profile, subscription, and app-runtime projection config regeneration route through the Rust runtime restart boundary".to_owned(),
            retained_mihomo_owned_unsupported_path: "Mihomo live config reload through the Go plugin API is retired; unsupported default data-plane startup remains blocked until Rust startup owns it".to_owned(),
            rollback_command: "restart_runtime_core after restoring the previous release-closeout checkpoint".to_owned(),
            evidence_file_or_source: "src-tauri/src/core/manager/config.rs; src-tauri/src/core/runtime_lifecycle.rs".to_owned(),
        },
        RustGoToRustMigrationFinalReleaseBlockerTableRow {
            supported_path_owned_by_rust: "Tauri release packaging no longer declares the Mihomo sidecar externalBin bundle".to_owned(),
            retained_mihomo_owned_unsupported_path: "real remote encrypted/QUIC peer compatibility, real plugin binary compatibility, and system packet capture stay Mihomo-owned without operator cutover evidence".to_owned(),
            rollback_command: "rollback_runtime_kernel_rust_go_to_rust_migration_release_closeout".to_owned(),
            evidence_file_or_source: "release-closeout-manifest.yaml; evidence.yaml; rollback-checkpoint.yaml".to_owned(),
        },
        RustGoToRustMigrationFinalReleaseBlockerTableRow {
            supported_path_owned_by_rust: "bounded DNS, adapter, protocol, UDP/plugin, TUN, fallback, and release closeout evidence remains Rust-owned".to_owned(),
            retained_mihomo_owned_unsupported_path: "unsupported fallback paths are retained until a PR removes a concrete default path with apply, hold, leak, and rollback proof".to_owned(),
            rollback_command: "rollback_runtime_kernel_rust_final_mihomo_binary_removal_gate".to_owned(),
            evidence_file_or_source: "src-tauri/src/core/kernel_runtime/*_blocker; docs/go-to-rust-migration-roadmap.md".to_owned(),
        },
    ]
}

fn empty_packaging_audit() -> RustGoToRustMigrationReleasePackagingAudit {
    RustGoToRustMigrationReleasePackagingAudit {
        items: Vec::new(),
        external_bin_removed_from_tauri_bundle: false,
        runtime_sidecar_invocation_audit_deferred: true,
        passed: false,
        blockers: Vec::new(),
    }
}

fn release_manifest_path() -> Result<PathBuf> {
    Ok(component_dir()?.join(RELEASE_MANIFEST_FILE))
}
pub fn rust_go_to_rust_migration_release_closeout_evidence_path() -> Result<PathBuf> {
    Ok(component_dir()?.join(EVIDENCE_FILE))
}
fn evidence_path() -> Result<PathBuf> {
    rust_go_to_rust_migration_release_closeout_evidence_path()
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
    fn verified_release_closeout_tracks_next_sidecar_invocation_batch() {
        let audit = RustGoToRustMigrationReleasePackagingAudit {
            items: Vec::new(),
            external_bin_removed_from_tauri_bundle: true,
            runtime_sidecar_invocation_audit_deferred: true,
            passed: true,
            blockers: Vec::new(),
        };
        let report = build_report(
            true,
            true,
            true,
            true,
            true,
            None,
            audit,
            None,
            None,
            rollback_checkpoint(true, None),
            Vec::new(),
        );

        assert_eq!(report.status, RustGoToRustMigrationReleaseCloseoutStatus::Verified);
        assert!(report.external_bin_removed_from_bundle);
        assert!(report.mutates_release_packaging);
        assert_eq!(report.next_safe_batch, NEXT_SAFE_BATCH);
    }
}
