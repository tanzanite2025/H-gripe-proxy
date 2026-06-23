use super::{
    RUST_RUNTIME_ID, approved_manual_default_path_removal_fallback_scopes,
    approved_manual_default_path_removal_surfaces,
};
use crate::utils::dirs;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::fs;

const COMPONENT: &str = "rust-operator-default-path-cutover";
const KERNEL_AREA: &str = "operator-default-path-cutover";
const CUTOVER_FILE: &str = "cutover.yaml";
const EVIDENCE_FILE: &str = "evidence.yaml";
const NEXT_SAFE_BATCH: &str = "mihomo-sidecar-binary-removal";

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum RustOperatorDefaultPathCutoverStatus {
    Ready,
    Blocked,
    Committed,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RustOperatorDefaultPathCutoverSurface {
    pub default_surface: String,
    pub fallback_scopes_removed: Vec<String>,
    pub manual_review_approved: bool,
    pub operator_approved: bool,
    pub cutover_committed: bool,
    pub mihomo_fallback_required_after_cutover: bool,
    pub rollback_required_before_binary_removal: bool,
    pub blockers: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RustOperatorDefaultPathCutoverManifest {
    pub component: String,
    pub created_at_epoch_seconds: u64,
    pub surfaces: Vec<RustOperatorDefaultPathCutoverSurface>,
    pub fallback_scopes_removed: Vec<String>,
    pub mutates_runtime: bool,
    pub mihomo_binary_removal_allowed_after_cutover: bool,
    pub next_safe_batch: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RustOperatorDefaultPathCutoverReport {
    pub runtime_id: String,
    pub component: String,
    pub kernel_area: String,
    pub status: RustOperatorDefaultPathCutoverStatus,
    pub reason: String,
    pub explicit_opt_in: bool,
    pub operator_approved: bool,
    pub commit_cutover: bool,
    pub cutover_manifest: Option<RustOperatorDefaultPathCutoverManifest>,
    pub cutover_manifest_path: Option<String>,
    pub evidence_path: Option<String>,
    pub mutates_runtime: bool,
    pub writes_cutover_manifest: bool,
    pub writes_evidence: bool,
    pub default_path_cutover_committed: bool,
    pub mihomo_binary_removal_allowed_after_cutover: bool,
    pub fallback_scopes_removed: Vec<String>,
    pub blockers: Vec<String>,
    pub warnings: Vec<String>,
    pub facts: Vec<String>,
    pub next_safe_batch: String,
}

pub async fn rust_operator_default_path_cutover(
    explicit_opt_in: bool,
    operator_approved: bool,
    commit_cutover: bool,
) -> Result<RustOperatorDefaultPathCutoverReport> {
    let surfaces = cutover_surfaces(operator_approved, false).await?;
    let mut blockers = surfaces
        .iter()
        .flat_map(|surface| surface.blockers.iter().cloned())
        .collect::<Vec<_>>();

    if !explicit_opt_in {
        blockers.push("explicit opt-in is required before committing default-path cutover".to_owned());
    }
    if !operator_approved {
        blockers.push("operator approval is required before committing default-path cutover".to_owned());
    }
    if surfaces.is_empty() {
        blockers.push("manual default-path removal review has no approved surfaces to cut over".to_owned());
    }
    blockers.sort();
    blockers.dedup();

    let fallback_scopes_removed = approved_manual_default_path_removal_fallback_scopes().await?;
    let manifest = RustOperatorDefaultPathCutoverManifest {
        component: COMPONENT.to_owned(),
        created_at_epoch_seconds: current_epoch_seconds(),
        surfaces: cutover_surfaces(operator_approved, commit_cutover && blockers.is_empty()).await?,
        fallback_scopes_removed: fallback_scopes_removed.clone(),
        mutates_runtime: false,
        mihomo_binary_removal_allowed_after_cutover: surfaces
            .iter()
            .any(|surface| surface.default_surface == "Mihomo sidecar binary removal"),
        next_safe_batch: NEXT_SAFE_BATCH.to_owned(),
    };

    let mut report = build_report(
        explicit_opt_in,
        operator_approved,
        commit_cutover,
        blockers,
        Some(manifest),
        fallback_scopes_removed,
    )?;

    if report.status == RustOperatorDefaultPathCutoverStatus::Blocked {
        return Ok(report);
    }

    if commit_cutover {
        let cutover_path = cutover_manifest_path()?;
        let evidence_path = evidence_path()?;
        if let Some(parent) = cutover_path.parent() {
            fs::create_dir_all(parent).await?;
        }
        report.cutover_manifest_path = Some(cutover_path.to_string_lossy().to_string());
        report.evidence_path = Some(evidence_path.to_string_lossy().to_string());
        report.writes_cutover_manifest = true;
        report.writes_evidence = true;
        if let Some(manifest) = report.cutover_manifest.as_ref() {
            fs::write(&cutover_path, serde_yaml_ng::to_string(manifest)?.as_bytes()).await?;
        }
        fs::write(&evidence_path, serde_yaml_ng::to_string(&report)?.as_bytes()).await?;
    }

    Ok(report)
}

pub async fn approved_operator_default_path_cutover_surfaces() -> Result<Vec<String>> {
    let Some(manifest) = read_cutover_manifest().await? else {
        return Ok(Vec::new());
    };

    Ok(manifest
        .surfaces
        .into_iter()
        .filter(|surface| surface.operator_approved && surface.cutover_committed)
        .map(|surface| surface.default_surface)
        .collect())
}

pub async fn approved_operator_default_path_cutover_fallback_scopes() -> Result<Vec<String>> {
    let Some(manifest) = read_cutover_manifest().await? else {
        return Ok(Vec::new());
    };

    Ok(committed_operator_default_path_cutover_fallback_scopes(&manifest))
}

fn committed_operator_default_path_cutover_fallback_scopes(
    manifest: &RustOperatorDefaultPathCutoverManifest,
) -> Vec<String> {
    let mut scopes = manifest
        .surfaces
        .iter()
        .filter(|surface| surface.operator_approved && surface.cutover_committed)
        .flat_map(|surface| surface.fallback_scopes_removed.iter().cloned())
        .collect::<Vec<_>>();
    scopes.sort();
    scopes.dedup();
    scopes
}

async fn cutover_surfaces(
    operator_approved: bool,
    cutover_committed: bool,
) -> Result<Vec<RustOperatorDefaultPathCutoverSurface>> {
    let approved_surfaces = approved_manual_default_path_removal_surfaces().await?;
    let fallback_scopes = approved_manual_default_path_removal_fallback_scopes().await?;
    Ok(approved_surfaces
        .into_iter()
        .map(|surface| {
            let scopes = fallback_scopes_for_surface(&surface, &fallback_scopes);
            RustOperatorDefaultPathCutoverSurface {
                default_surface: surface,
                fallback_scopes_removed: scopes,
                manual_review_approved: true,
                operator_approved,
                cutover_committed,
                mihomo_fallback_required_after_cutover: !cutover_committed,
                rollback_required_before_binary_removal: true,
                blockers: if operator_approved {
                    Vec::new()
                } else {
                    vec!["operator approval is required for this default-path cutover surface".to_owned()]
                },
            }
        })
        .collect())
}

fn build_report(
    explicit_opt_in: bool,
    operator_approved: bool,
    commit_cutover: bool,
    blockers: Vec<String>,
    cutover_manifest: Option<RustOperatorDefaultPathCutoverManifest>,
    fallback_scopes_removed: Vec<String>,
) -> Result<RustOperatorDefaultPathCutoverReport> {
    let status = if blockers.is_empty() && commit_cutover {
        RustOperatorDefaultPathCutoverStatus::Committed
    } else if blockers.is_empty() {
        RustOperatorDefaultPathCutoverStatus::Ready
    } else {
        RustOperatorDefaultPathCutoverStatus::Blocked
    };
    let default_path_cutover_committed = status == RustOperatorDefaultPathCutoverStatus::Committed;
    let mihomo_binary_removal_allowed_after_cutover = cutover_manifest
        .as_ref()
        .map(|manifest| manifest.mihomo_binary_removal_allowed_after_cutover && default_path_cutover_committed)
        .unwrap_or(false);

    Ok(RustOperatorDefaultPathCutoverReport {
        runtime_id: RUST_RUNTIME_ID.to_owned(),
        component: COMPONENT.to_owned(),
        kernel_area: KERNEL_AREA.to_owned(),
        status,
        reason: match status {
            RustOperatorDefaultPathCutoverStatus::Ready => {
                "operator-approved default-path cutover is ready to commit".to_owned()
            }
            RustOperatorDefaultPathCutoverStatus::Committed => {
                "operator-approved default-path cutover manifest committed".to_owned()
            }
            RustOperatorDefaultPathCutoverStatus::Blocked => {
                "operator-approved default-path cutover is blocked".to_owned()
            }
        },
        explicit_opt_in,
        operator_approved,
        commit_cutover,
        cutover_manifest,
        cutover_manifest_path: Some(cutover_manifest_path()?.to_string_lossy().to_string()),
        evidence_path: Some(evidence_path()?.to_string_lossy().to_string()),
        mutates_runtime: false,
        writes_cutover_manifest: false,
        writes_evidence: false,
        default_path_cutover_committed,
        mihomo_binary_removal_allowed_after_cutover,
        fallback_scopes_removed,
        blockers,
        warnings: vec![
            "cutover manifest does not mutate DNS, routes, TUN, proxy forwarding, plugin processes, or sidecar files"
                .to_owned(),
            "Mihomo binary removal still requires committed cutover plus rollback coverage".to_owned(),
        ],
        facts: vec![
            "committed cutover surfaces are consumed by migration final review".to_owned(),
            "manual default-path removal review remains the approval source".to_owned(),
        ],
        next_safe_batch: NEXT_SAFE_BATCH.to_owned(),
    })
}

fn fallback_scopes_for_surface(default_surface: &str, fallback_scopes: &[String]) -> Vec<String> {
    fallback_scopes
        .iter()
        .filter(|scope| match default_surface {
            "default DNS resolver replacement" => scope.contains("DNS"),
            "non-loopback proxy protocol forwarding defaults" => {
                scope.contains("QUIC") || scope.contains("UDP") || scope.contains("plugin")
            }
            "system-wide packet capture and route install" => {
                scope.contains("packet capture") || scope.contains("route") || scope.contains("transparent proxy")
            }
            "Mihomo sidecar binary removal" => scope.contains("Mihomo sidecar"),
            _ => false,
        })
        .cloned()
        .collect()
}

async fn read_cutover_manifest() -> Result<Option<RustOperatorDefaultPathCutoverManifest>> {
    let path = cutover_manifest_path()?;
    let Some(yaml) = fs::read_to_string(path).await.ok() else {
        return Ok(None);
    };

    Ok(serde_yaml_ng::from_str::<RustOperatorDefaultPathCutoverManifest>(&yaml).ok())
}

fn cutover_manifest_path() -> Result<std::path::PathBuf> {
    Ok(dirs::app_home_dir()?
        .join("kernel-runtime")
        .join(KERNEL_AREA)
        .join(CUTOVER_FILE))
}

fn evidence_path() -> Result<std::path::PathBuf> {
    Ok(dirs::app_home_dir()?
        .join("kernel-runtime")
        .join(KERNEL_AREA)
        .join(EVIDENCE_FILE))
}

fn current_epoch_seconds() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cutover_surface(
        default_surface: &str,
        fallback_scopes_removed: Vec<&str>,
        operator_approved: bool,
        cutover_committed: bool,
    ) -> RustOperatorDefaultPathCutoverSurface {
        RustOperatorDefaultPathCutoverSurface {
            default_surface: default_surface.to_owned(),
            fallback_scopes_removed: fallback_scopes_removed.into_iter().map(str::to_owned).collect(),
            manual_review_approved: true,
            operator_approved,
            cutover_committed,
            mihomo_fallback_required_after_cutover: !cutover_committed,
            rollback_required_before_binary_removal: true,
            blockers: vec![],
        }
    }

    #[test]
    fn fallback_scopes_only_include_committed_operator_approved_surfaces() {
        let manifest = RustOperatorDefaultPathCutoverManifest {
            component: COMPONENT.to_owned(),
            created_at_epoch_seconds: 0,
            surfaces: vec![
                cutover_surface("Mihomo sidecar binary removal", vec!["dns", "adapter"], true, true),
                cutover_surface("uncommitted", vec!["packet-capture"], true, false),
                cutover_surface("unapproved", vec!["route"], false, true),
                cutover_surface("duplicate", vec!["adapter"], true, true),
            ],
            fallback_scopes_removed: vec![
                "dns".to_owned(),
                "adapter".to_owned(),
                "packet-capture".to_owned(),
                "route".to_owned(),
            ],
            mutates_runtime: false,
            mihomo_binary_removal_allowed_after_cutover: true,
            next_safe_batch: NEXT_SAFE_BATCH.to_owned(),
        };

        assert_eq!(
            committed_operator_default_path_cutover_fallback_scopes(&manifest),
            vec!["adapter".to_owned(), "dns".to_owned()]
        );
    }
}
