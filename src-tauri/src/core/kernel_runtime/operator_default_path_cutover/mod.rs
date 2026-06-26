use crate::utils::dirs;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use tokio::fs;

const KERNEL_AREA: &str = "operator-default-path-cutover";
const CUTOVER_FILE: &str = "cutover.yaml";

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
            component: "rust-operator-default-path-cutover".to_owned(),
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
            next_safe_batch: "mihomo-sidecar-binary-removal".to_owned(),
        };

        assert_eq!(
            committed_operator_default_path_cutover_fallback_scopes(&manifest),
            vec!["adapter".to_owned(), "dns".to_owned()]
        );
    }
}
