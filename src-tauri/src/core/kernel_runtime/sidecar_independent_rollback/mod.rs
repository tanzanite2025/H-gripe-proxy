use super::RUST_RUNTIME_ID;
use crate::utils::dirs;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use sha2::{Digest as _, Sha256};
use std::{
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};
use tokio::fs;

const COMPONENT: &str = "rust-sidecar-independent-rollback";
const KERNEL_AREA: &str = "sidecar-independent-rollback";
const ARCHIVE_DIR: &str = "archive";
const EVIDENCE_FILE: &str = "evidence.yaml";
const ROLLBACK_PLAN_FILE: &str = "rollback-plan.yaml";
const RUST_OWNED_SCOPE: &str =
    "Rust-owned rollback archive and restore plan for migration evidence without invoking the Mihomo sidecar";
const NEXT_SAFE_BATCH: &str = "default-path-blocker-reduction";

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum RustSidecarIndependentRollbackStatus {
    Ready,
    Blocked,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RustSidecarIndependentRollbackArtifactEvidence {
    pub component: String,
    pub source_path: String,
    pub archive_path: String,
    pub source_present: bool,
    pub archived: bool,
    pub checksum: Option<String>,
    pub byte_len: usize,
    pub sidecar_invocation_required: bool,
    pub blockers: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RustSidecarIndependentRollbackRestoreStep {
    pub order: usize,
    pub artifact_component: String,
    pub restore_target_path: String,
    pub checksum: Option<String>,
    pub requires_mihomo_sidecar: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RustSidecarIndependentRollbackPlan {
    pub component: String,
    pub created_at_epoch_seconds: u64,
    pub archive_dir: String,
    pub restore_steps: Vec<RustSidecarIndependentRollbackRestoreStep>,
    pub sidecar_invocation_required: bool,
    pub restores_without_app_restart: bool,
    pub rollback_scope: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RustSidecarIndependentRollbackReport {
    pub runtime_id: String,
    pub component: String,
    pub kernel_area: String,
    pub status: RustSidecarIndependentRollbackStatus,
    pub reason: String,
    pub explicit_opt_in: bool,
    pub rust_owned_scope: String,
    pub artifacts: Vec<RustSidecarIndependentRollbackArtifactEvidence>,
    pub rollback_plan: Option<RustSidecarIndependentRollbackPlan>,
    pub evidence_path: Option<String>,
    pub rollback_plan_path: Option<String>,
    pub mutates_runtime: bool,
    pub writes_evidence: bool,
    pub sidecar_invocation_required: bool,
    pub sidecar_binary_removal_allowed: bool,
    pub sidecar_independent_rollback_ready: bool,
    pub blockers: Vec<String>,
    pub warnings: Vec<String>,
    pub facts: Vec<String>,
    pub next_safe_batch: String,
}

pub async fn rust_sidecar_independent_rollback_archive(
    explicit_opt_in: bool,
) -> Result<RustSidecarIndependentRollbackReport> {
    if !explicit_opt_in {
        return Ok(blocked_report(vec![
            "explicit opt-in is required to archive sidecar-independent rollback evidence".to_owned(),
        ]));
    }

    let archive_dir = evidence_dir()?.join(ARCHIVE_DIR);
    fs::create_dir_all(&archive_dir).await?;
    let mut artifacts = Vec::new();
    for spec in artifact_specs()? {
        artifacts.push(archive_artifact(&archive_dir, spec).await?);
    }
    let blockers = artifacts
        .iter()
        .flat_map(|artifact| artifact.blockers.iter().cloned())
        .collect::<Vec<_>>();
    let restore_steps = artifacts
        .iter()
        .enumerate()
        .filter(|(_, artifact)| artifact.archived)
        .map(|(index, artifact)| RustSidecarIndependentRollbackRestoreStep {
            order: index + 1,
            artifact_component: artifact.component.clone(),
            restore_target_path: artifact.source_path.clone(),
            checksum: artifact.checksum.clone(),
            requires_mihomo_sidecar: false,
        })
        .collect::<Vec<_>>();
    let rollback_plan = RustSidecarIndependentRollbackPlan {
        component: COMPONENT.to_owned(),
        created_at_epoch_seconds: epoch_seconds(),
        archive_dir: archive_dir.to_string_lossy().to_string(),
        restore_steps,
        sidecar_invocation_required: false,
        restores_without_app_restart: true,
        rollback_scope: rollback_scope(),
    };
    let status = if blockers.is_empty() {
        RustSidecarIndependentRollbackStatus::Ready
    } else {
        RustSidecarIndependentRollbackStatus::Blocked
    };
    let evidence_path = evidence_path()?;
    let rollback_plan_path = rollback_plan_path()?;
    let mut report = RustSidecarIndependentRollbackReport {
        runtime_id: RUST_RUNTIME_ID.to_owned(),
        component: COMPONENT.to_owned(),
        kernel_area: KERNEL_AREA.to_owned(),
        status,
        reason: if status == RustSidecarIndependentRollbackStatus::Ready {
            "Rust rollback archive can restore migration evidence without invoking Mihomo sidecar"
        } else {
            "Rust sidecar-independent rollback archive is missing required evidence artifacts"
        }
        .to_owned(),
        explicit_opt_in,
        rust_owned_scope: RUST_OWNED_SCOPE.to_owned(),
        artifacts,
        rollback_plan: Some(rollback_plan.clone()),
        evidence_path: Some(evidence_path.to_string_lossy().to_string()),
        rollback_plan_path: Some(rollback_plan_path.to_string_lossy().to_string()),
        mutates_runtime: false,
        writes_evidence: true,
        sidecar_invocation_required: false,
        sidecar_binary_removal_allowed: false,
        sidecar_independent_rollback_ready: status == RustSidecarIndependentRollbackStatus::Ready,
        blockers,
        warnings: vec![
            "this archive removes sidecar invocation from rollback evidence restore, not from packet forwarding"
                .to_owned(),
            "Mihomo sidecar remains required for unsupported runtime fallback paths".to_owned(),
        ],
        facts: facts(),
        next_safe_batch: NEXT_SAFE_BATCH.to_owned(),
    };

    fs::write(
        &rollback_plan_path,
        serde_yaml_ng::to_string(&rollback_plan)?.as_bytes(),
    )
    .await?;
    report.rollback_plan_path = Some(rollback_plan_path.to_string_lossy().to_string());
    fs::write(&evidence_path, serde_yaml_ng::to_string(&report)?.as_bytes()).await?;
    Ok(report)
}

fn blocked_report(blockers: Vec<String>) -> RustSidecarIndependentRollbackReport {
    RustSidecarIndependentRollbackReport {
        runtime_id: RUST_RUNTIME_ID.to_owned(),
        component: COMPONENT.to_owned(),
        kernel_area: KERNEL_AREA.to_owned(),
        status: RustSidecarIndependentRollbackStatus::Blocked,
        reason: "Rust sidecar-independent rollback archive is blocked".to_owned(),
        explicit_opt_in: false,
        rust_owned_scope: RUST_OWNED_SCOPE.to_owned(),
        artifacts: Vec::new(),
        rollback_plan: None,
        evidence_path: None,
        rollback_plan_path: None,
        mutates_runtime: false,
        writes_evidence: false,
        sidecar_invocation_required: false,
        sidecar_binary_removal_allowed: false,
        sidecar_independent_rollback_ready: false,
        blockers,
        warnings: Vec::new(),
        facts: facts(),
        next_safe_batch: NEXT_SAFE_BATCH.to_owned(),
    }
}

struct ArtifactSpec {
    component: &'static str,
    relative_path: &'static str,
}

async fn archive_artifact(
    archive_dir: &std::path::Path,
    spec: ArtifactSpec,
) -> Result<RustSidecarIndependentRollbackArtifactEvidence> {
    let source_path = dirs::app_runtime_dir()?.join(spec.relative_path);
    let archive_path = archive_dir.join(archive_file_name(spec.relative_path));
    let content = fs::read(&source_path).await.ok();
    let source_present = content.is_some();
    let mut archived = false;
    let mut checksum = None;
    let byte_len = if let Some(content) = content.as_ref() {
        fs::write(&archive_path, content).await?;
        archived = true;
        checksum = Some(hex_sha256(content));
        content.len()
    } else {
        0
    };
    let mut blockers = Vec::new();
    if !source_present {
        blockers.push(format!("{} rollback artifact is missing", spec.component));
    }
    if source_present && !archived {
        blockers.push(format!("{} rollback artifact was not archived", spec.component));
    }

    Ok(RustSidecarIndependentRollbackArtifactEvidence {
        component: spec.component.to_owned(),
        source_path: source_path.to_string_lossy().to_string(),
        archive_path: archive_path.to_string_lossy().to_string(),
        source_present,
        archived,
        checksum,
        byte_len,
        sidecar_invocation_required: false,
        blockers,
    })
}

fn artifact_specs() -> Result<Vec<ArtifactSpec>> {
    Ok(vec![
        ArtifactSpec {
            component: "mihomo-fallback-retirement-execution",
            relative_path: "mihomo-fallback-retirement-execution/emergency-rollback.yaml",
        },
        ArtifactSpec {
            component: "mihomo-fallback-retirement-execution-manifest",
            relative_path: "mihomo-fallback-retirement-execution/execution.yaml",
        },
        ArtifactSpec {
            component: "rust-mihomo-fallback-retirement-bundle",
            relative_path: "rust-mihomo-fallback-retirement-bundle/rollback-checkpoint.yaml",
        },
        ArtifactSpec {
            component: "rust-mihomo-fallback-retirement-bundle-evidence",
            relative_path: "rust-mihomo-fallback-retirement-bundle/evidence.yaml",
        },
        ArtifactSpec {
            component: "go-to-rust-migration-final-review",
            relative_path: "go-to-rust-migration-final-review/evidence.yaml",
        },
    ])
}

fn archive_file_name(relative_path: &str) -> String {
    relative_path.replace(['/', '\\'], "__")
}

fn hex_sha256(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    format!("{:x}", hasher.finalize())
}

fn rollback_scope() -> Vec<String> {
    vec![
        "fallback retirement execution checkpoint".to_owned(),
        "fallback retirement execution manifest".to_owned(),
        "Mihomo fallback retirement bundle checkpoint".to_owned(),
        "Mihomo fallback retirement bundle evidence".to_owned(),
        "Go-to-Rust migration final review evidence".to_owned(),
    ]
}

fn facts() -> Vec<String> {
    vec![
        "rollback archive copies YAML evidence artifacts with checksums into a Rust-owned runtime directory".to_owned(),
        "restore steps target evidence/manifest files and never invoke the Mihomo sidecar process".to_owned(),
        "sidecar binary removal remains blocked for unsupported runtime fallback paths".to_owned(),
    ]
}

fn evidence_dir() -> Result<PathBuf> {
    Ok(dirs::app_runtime_dir()?.join(COMPONENT))
}

fn evidence_path() -> Result<PathBuf> {
    Ok(evidence_dir()?.join(EVIDENCE_FILE))
}

fn rollback_plan_path() -> Result<PathBuf> {
    Ok(evidence_dir()?.join(ROLLBACK_PLAN_FILE))
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
    fn archive_file_names_are_flat() {
        let name = archive_file_name("a/b/c.yaml");

        assert_eq!(name, "a__b__c.yaml");
    }

    #[test]
    fn rollback_scope_keeps_sidecar_removal_blocked() {
        let report = blocked_report(Vec::new());

        assert!(!report.sidecar_binary_removal_allowed);
        assert!(!report.sidecar_invocation_required);
    }
}
