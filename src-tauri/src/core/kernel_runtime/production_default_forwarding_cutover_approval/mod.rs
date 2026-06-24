use super::{
    RUST_RUNTIME_ID, RustDefaultForwardingHoldBlockerReport, RustDefaultForwardingHoldBlockerStatus,
    rust_default_forwarding_hold_blocker_evidence_path,
};
use crate::utils::dirs;
use anyhow::{Context as _, Result};
use serde::{Deserialize, Serialize};
use sha2::{Digest as _, Sha256};
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::fs;

const COMPONENT: &str = "rust-production-default-forwarding-cutover-approval";
const KERNEL_AREA: &str = "production-default-forwarding-cutover-approval";
const APPROVAL_MANIFEST_FILE: &str = "approval-manifest.yaml";
const EVIDENCE_FILE: &str = "evidence.yaml";
const NEXT_SAFE_BATCH: &str = "guarded-production-default-forwarding-apply";

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum RustProductionDefaultForwardingCutoverApprovalStatus {
    Ready,
    Blocked,
    Approved,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RustProductionDefaultForwardingCutoverApprovalSurface {
    pub default_surface: String,
    pub required_hold_profiles: Vec<String>,
    pub rollback_scope: String,
    pub rust_owner_after_approval: String,
    pub hold_evidence_ready: bool,
    pub operator_approved: bool,
    pub approval_committed: bool,
    pub production_runtime_mutated: bool,
    pub mihomo_fallback_required_after_approval: bool,
    pub blockers: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RustProductionDefaultForwardingCutoverApprovalManifest {
    pub component: String,
    pub created_at_epoch_seconds: u64,
    pub hold_evidence_path: Option<String>,
    pub hold_evidence_checksum: Option<String>,
    pub surfaces: Vec<RustProductionDefaultForwardingCutoverApprovalSurface>,
    pub approval_committed: bool,
    pub mutates_runtime: bool,
    pub production_default_forwarding_mutated: bool,
    pub mihomo_fallback_required_until_guarded_apply: bool,
    pub rollback_chain: Vec<String>,
    pub next_safe_batch: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RustProductionDefaultForwardingCutoverApprovalReport {
    pub runtime_id: String,
    pub component: String,
    pub kernel_area: String,
    pub status: RustProductionDefaultForwardingCutoverApprovalStatus,
    pub reason: String,
    pub explicit_opt_in: bool,
    pub operator_approved: bool,
    pub commit_approval: bool,
    pub default_forwarding_hold_gate: Option<RustDefaultForwardingHoldBlockerReport>,
    pub approval_manifest: Option<RustProductionDefaultForwardingCutoverApprovalManifest>,
    pub approval_manifest_path: Option<String>,
    pub evidence_path: Option<String>,
    pub approval_manifest_checksum: Option<String>,
    pub mutates_runtime: bool,
    pub writes_approval_manifest: bool,
    pub writes_evidence: bool,
    pub production_default_forwarding_approved: bool,
    pub production_default_forwarding_mutated: bool,
    pub mihomo_default_forwarding_fallback_required: bool,
    pub blockers_reduced: Vec<String>,
    pub blockers_remaining: Vec<String>,
    pub blockers: Vec<String>,
    pub warnings: Vec<String>,
    pub facts: Vec<String>,
    pub next_safe_batch: String,
}

pub async fn rust_production_default_forwarding_cutover_approval(
    explicit_opt_in: bool,
    operator_approved: bool,
    commit_approval: bool,
) -> Result<RustProductionDefaultForwardingCutoverApprovalReport> {
    let hold_gate = default_forwarding_hold_gate().await?;
    let mut blockers = approval_blockers(hold_gate.as_ref(), explicit_opt_in, operator_approved);
    if commit_approval && !operator_approved {
        blockers.push("approval commit requires operator-approved production default forwarding".to_owned());
    }
    blockers.sort();
    blockers.dedup();

    let approval_committed = commit_approval && blockers.is_empty();
    let manifest = approval_manifest(hold_gate.as_ref(), operator_approved, approval_committed).await?;
    let manifest_yaml = serde_yaml_ng::to_string(&manifest)?;
    let approval_manifest_checksum = hex_sha256(manifest_yaml.as_bytes());
    let mut report = build_report(
        explicit_opt_in,
        operator_approved,
        commit_approval,
        hold_gate,
        Some(manifest),
        Some(approval_manifest_checksum),
        blockers,
    );

    if report.status == RustProductionDefaultForwardingCutoverApprovalStatus::Blocked {
        return Ok(report);
    }

    if commit_approval {
        let approval_manifest_path = approval_manifest_path()?;
        let evidence_path = rust_production_default_forwarding_cutover_approval_evidence_path()?;
        if let Some(parent) = approval_manifest_path.parent() {
            fs::create_dir_all(parent).await?;
        }
        report.approval_manifest_path = Some(approval_manifest_path.to_string_lossy().to_string());
        report.evidence_path = Some(evidence_path.to_string_lossy().to_string());
        report.writes_approval_manifest = true;
        report.writes_evidence = true;
        if let Some(manifest) = report.approval_manifest.as_ref() {
            fs::write(&approval_manifest_path, serde_yaml_ng::to_string(manifest)?.as_bytes()).await?;
        }
        fs::write(&evidence_path, serde_yaml_ng::to_string(&report)?.as_bytes()).await?;
    }

    Ok(report)
}

pub async fn approved_production_default_forwarding_cutover_surfaces() -> Result<Vec<String>> {
    let Some(manifest) = read_approval_manifest().await? else {
        return Ok(Vec::new());
    };

    Ok(manifest
        .surfaces
        .into_iter()
        .filter(|surface| surface.operator_approved && surface.approval_committed)
        .map(|surface| surface.default_surface)
        .collect())
}

fn build_report(
    explicit_opt_in: bool,
    operator_approved: bool,
    commit_approval: bool,
    default_forwarding_hold_gate: Option<RustDefaultForwardingHoldBlockerReport>,
    approval_manifest: Option<RustProductionDefaultForwardingCutoverApprovalManifest>,
    approval_manifest_checksum: Option<String>,
    blockers: Vec<String>,
) -> RustProductionDefaultForwardingCutoverApprovalReport {
    let status = if blockers.is_empty() && commit_approval {
        RustProductionDefaultForwardingCutoverApprovalStatus::Approved
    } else if blockers.is_empty() {
        RustProductionDefaultForwardingCutoverApprovalStatus::Ready
    } else {
        RustProductionDefaultForwardingCutoverApprovalStatus::Blocked
    };
    let production_default_forwarding_approved =
        status == RustProductionDefaultForwardingCutoverApprovalStatus::Approved;

    RustProductionDefaultForwardingCutoverApprovalReport {
        runtime_id: RUST_RUNTIME_ID.to_owned(),
        component: COMPONENT.to_owned(),
        kernel_area: KERNEL_AREA.to_owned(),
        status,
        reason: if production_default_forwarding_approved {
            "operator approved guarded production default forwarding cutover manifest"
        } else if status == RustProductionDefaultForwardingCutoverApprovalStatus::Ready {
            "production default forwarding cutover approval is ready to commit"
        } else {
            "production default forwarding cutover approval is blocked"
        }
        .to_owned(),
        explicit_opt_in,
        operator_approved,
        commit_approval,
        default_forwarding_hold_gate,
        approval_manifest,
        approval_manifest_path: None,
        evidence_path: None,
        approval_manifest_checksum,
        mutates_runtime: false,
        writes_approval_manifest: false,
        writes_evidence: false,
        production_default_forwarding_approved,
        production_default_forwarding_mutated: false,
        mihomo_default_forwarding_fallback_required: true,
        blockers_reduced: if production_default_forwarding_approved {
            vec![
                "operator-approved production default forwarding cutover manifest".to_owned(),
                "bounded hold evidence consumed across protocol, UDP, QUIC, and plugin surfaces".to_owned(),
                "single rollback chain retained before guarded apply".to_owned(),
            ]
        } else {
            Vec::new()
        },
        blockers_remaining: if production_default_forwarding_approved {
            vec![
                "guarded production default forwarding apply".to_owned(),
                "post-approval real-profile hold verification".to_owned(),
                "Mihomo fallback retirement after guarded apply".to_owned(),
            ]
        } else {
            vec!["operator-approved production default forwarding cutover manifest".to_owned()]
        },
        blockers,
        warnings: vec![
            "approval evidence does not mutate production forwarding by itself".to_owned(),
            "Mihomo default forwarding fallback remains retained until guarded apply and rollback verification pass"
                .to_owned(),
        ],
        facts: facts(),
        next_safe_batch: NEXT_SAFE_BATCH.to_owned(),
    }
}

async fn approval_manifest(
    hold_gate: Option<&RustDefaultForwardingHoldBlockerReport>,
    operator_approved: bool,
    approval_committed: bool,
) -> Result<RustProductionDefaultForwardingCutoverApprovalManifest> {
    let required_hold_profiles = hold_profiles(hold_gate);
    let hold_evidence_path = hold_gate.as_ref().and_then(|gate| gate.evidence_path.as_ref()).cloned();
    let hold_evidence_checksum = hold_gate_checksum().await?;
    let hold_evidence_ready = hold_ready(hold_gate);
    Ok(RustProductionDefaultForwardingCutoverApprovalManifest {
        component: COMPONENT.to_owned(),
        created_at_epoch_seconds: epoch_seconds(),
        hold_evidence_path,
        hold_evidence_checksum,
        surfaces: production_default_forwarding_surfaces()
            .into_iter()
            .map(|(default_surface, rollback_scope)| {
                let blockers = surface_blockers(hold_evidence_ready, operator_approved);
                RustProductionDefaultForwardingCutoverApprovalSurface {
                    default_surface: default_surface.to_owned(),
                    required_hold_profiles: required_hold_profiles.clone(),
                    rollback_scope: rollback_scope.to_owned(),
                    rust_owner_after_approval: "rust-kernel-runtime guarded apply bridge".to_owned(),
                    hold_evidence_ready,
                    operator_approved,
                    approval_committed,
                    production_runtime_mutated: false,
                    mihomo_fallback_required_after_approval: true,
                    blockers,
                }
            })
            .collect(),
        approval_committed,
        mutates_runtime: false,
        production_default_forwarding_mutated: false,
        mihomo_fallback_required_until_guarded_apply: true,
        rollback_chain: vec![
            "do not alter current Mihomo default forwarding before guarded apply".to_owned(),
            "apply production default forwarding only from committed approval manifest".to_owned(),
            "restore Mihomo default forwarding on failed post-approval hold".to_owned(),
        ],
        next_safe_batch: NEXT_SAFE_BATCH.to_owned(),
    })
}

fn production_default_forwarding_surfaces() -> Vec<(&'static str, &'static str)> {
    vec![
        ("TCP protocol default forwarding", "mihomo-tcp-default-forwarding"),
        ("SOCKS UDP default forwarding", "mihomo-socks-udp-default-forwarding"),
        (
            "encrypted protocol default forwarding",
            "mihomo-encrypted-protocol-default-forwarding",
        ),
        (
            "QUIC/UDP profile default forwarding",
            "mihomo-quic-udp-default-forwarding",
        ),
        (
            "plugin transport default forwarding",
            "mihomo-plugin-default-forwarding",
        ),
    ]
}

fn approval_blockers(
    hold_gate: Option<&RustDefaultForwardingHoldBlockerReport>,
    explicit_opt_in: bool,
    operator_approved: bool,
) -> Vec<String> {
    let mut blockers = Vec::new();
    if !explicit_opt_in {
        blockers.push("explicit opt-in is required before production default forwarding approval".to_owned());
    }
    if !operator_approved {
        blockers.push("operator approval is required before production default forwarding approval".to_owned());
    }
    blockers.extend(hold_gate_blockers(hold_gate));
    blockers
}

fn hold_gate_blockers(hold_gate: Option<&RustDefaultForwardingHoldBlockerReport>) -> Vec<String> {
    let Some(gate) = hold_gate else {
        return vec!["default forwarding hold evidence is missing".to_owned()];
    };

    let mut blockers = Vec::new();
    if gate.status != RustDefaultForwardingHoldBlockerStatus::Ready {
        blockers.push(format!("default forwarding hold gate status is {:?}", gate.status));
    }
    if !gate.blockers.is_empty() {
        blockers.push("default forwarding hold gate contains blockers".to_owned());
    }
    match gate.hold_evidence.as_ref() {
        Some(evidence) => {
            if !evidence.passed {
                blockers.push("default forwarding hold evidence did not pass".to_owned());
            }
            if evidence.default_forwarding_mutated {
                blockers.push("default forwarding hold evidence already mutated runtime".to_owned());
            }
            if !evidence.fallback_retained {
                blockers.push("default forwarding hold evidence did not retain Mihomo fallback".to_owned());
            }
        }
        None => blockers.push("default forwarding hold report has no hold evidence".to_owned()),
    }
    if !gate.mihomo_default_forwarding_fallback_required {
        blockers.push("default forwarding hold gate does not retain Mihomo fallback".to_owned());
    }
    blockers.sort();
    blockers.dedup();
    blockers
}

fn surface_blockers(hold_evidence_ready: bool, operator_approved: bool) -> Vec<String> {
    let mut blockers = Vec::new();
    if !hold_evidence_ready {
        blockers.push("ready default forwarding hold evidence is required".to_owned());
    }
    if !operator_approved {
        blockers.push("operator approval is required for this default forwarding surface".to_owned());
    }
    blockers
}

fn hold_ready(hold_gate: Option<&RustDefaultForwardingHoldBlockerReport>) -> bool {
    hold_gate_blockers(hold_gate).is_empty()
}

fn hold_profiles(hold_gate: Option<&RustDefaultForwardingHoldBlockerReport>) -> Vec<String> {
    hold_gate
        .and_then(|gate| gate.hold_evidence.as_ref())
        .map(|evidence| evidence.profiles.clone())
        .unwrap_or_default()
}

async fn default_forwarding_hold_gate() -> Result<Option<RustDefaultForwardingHoldBlockerReport>> {
    let path = rust_default_forwarding_hold_blocker_evidence_path()?;
    match fs::read_to_string(&path).await {
        Ok(yaml) => serde_yaml_ng::from_str(&yaml)
            .with_context(|| format!("failed to parse {}", path.display()))
            .map(Some),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error).with_context(|| format!("failed to read {}", path.display())),
    }
}

async fn hold_gate_checksum() -> Result<Option<String>> {
    let path = rust_default_forwarding_hold_blocker_evidence_path()?;
    match fs::read(&path).await {
        Ok(bytes) => Ok(Some(hex_sha256(&bytes))),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error).with_context(|| format!("failed to read {}", path.display())),
    }
}

async fn read_approval_manifest() -> Result<Option<RustProductionDefaultForwardingCutoverApprovalManifest>> {
    let path = approval_manifest_path()?;
    match fs::read_to_string(&path).await {
        Ok(yaml) => serde_yaml_ng::from_str(&yaml)
            .with_context(|| format!("failed to parse {}", path.display()))
            .map(Some),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error).with_context(|| format!("failed to read {}", path.display())),
    }
}

fn facts() -> Vec<String> {
    vec![
        "Rust consumes bounded default forwarding hold evidence before accepting production approval".to_owned(),
        "One approval manifest covers TCP, SOCKS UDP, encrypted, QUIC/UDP, and plugin default forwarding".to_owned(),
        "The approval step preserves Mihomo fallback until a guarded apply performs real mutation".to_owned(),
    ]
}

fn evidence_dir() -> Result<std::path::PathBuf> {
    Ok(dirs::app_runtime_dir()?.join(COMPONENT))
}

fn approval_manifest_path() -> Result<std::path::PathBuf> {
    Ok(evidence_dir()?.join(APPROVAL_MANIFEST_FILE))
}

pub fn rust_production_default_forwarding_cutover_approval_evidence_path() -> Result<std::path::PathBuf> {
    Ok(evidence_dir()?.join(EVIDENCE_FILE))
}

fn hex_sha256(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    format!("{:x}", hasher.finalize())
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
    fn missing_hold_gate_blocks_approval() {
        let blockers = approval_blockers(None, true, true);

        assert!(blockers.contains(&"default forwarding hold evidence is missing".to_owned()));
    }

    #[test]
    fn ready_report_retains_fallback_until_guarded_apply() {
        let report = build_report(true, true, true, None, None, None, Vec::new());

        assert_eq!(
            report.status,
            RustProductionDefaultForwardingCutoverApprovalStatus::Approved
        );
        assert!(report.mihomo_default_forwarding_fallback_required);
        assert!(!report.production_default_forwarding_mutated);
    }
}
