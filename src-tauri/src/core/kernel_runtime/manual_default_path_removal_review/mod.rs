use super::{RUST_RUNTIME_ID, go_to_rust_migration_final_review};
use crate::utils::dirs;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_yaml_ng::Value;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::fs;

const COMPONENT: &str = "rust-manual-default-path-removal-review";
const KERNEL_AREA: &str = "manual-default-path-removal-review";
const EVIDENCE_FILE: &str = "evidence.yaml";
const REMOVAL_REVIEW_FILE: &str = "default-path-removal-review.yaml";
const NEXT_SAFE_BATCH: &str = "operator-approved-default-path-cutover";

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum RustManualDefaultPathRemovalReviewStatus {
    Ready,
    Blocked,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RustManualDefaultPathRemovalArtifactEvidence {
    pub component: String,
    pub evidence_path: String,
    pub artifact_present: bool,
    pub status: Option<String>,
    pub blockers_present: bool,
    pub accepted_for_removal_review: bool,
    pub blockers: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RustManualDefaultPathRemovalSurfaceReview {
    pub default_surface: String,
    pub retained_fallback_scope: Vec<String>,
    pub operator_approval_required: bool,
    pub operator_approved: bool,
    pub artifacts: Vec<RustManualDefaultPathRemovalArtifactEvidence>,
    pub rust_default_ownership_allowed: bool,
    pub mihomo_fallback_required: bool,
    pub blockers_reduced: Vec<String>,
    pub blockers_remaining: Vec<String>,
    pub blockers: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RustManualDefaultPathRemovalReviewArtifact {
    pub component: String,
    pub created_at_epoch_seconds: u64,
    pub surfaces: Vec<RustManualDefaultPathRemovalSurfaceReview>,
    pub removal_allowed_surfaces: Vec<String>,
    pub fallback_retained_surfaces: Vec<String>,
    pub sidecar_removal_allowed: bool,
    pub mutates_runtime: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RustManualDefaultPathRemovalReviewReport {
    pub runtime_id: String,
    pub component: String,
    pub kernel_area: String,
    pub status: RustManualDefaultPathRemovalReviewStatus,
    pub reason: String,
    pub explicit_opt_in: bool,
    pub operator_approved: bool,
    pub review_artifact: Option<RustManualDefaultPathRemovalReviewArtifact>,
    pub review_artifact_path: Option<String>,
    pub evidence_path: Option<String>,
    pub mutates_runtime: bool,
    pub writes_evidence: bool,
    pub default_path_removal_allowed: bool,
    pub mihomo_binary_removal_allowed: bool,
    pub blockers_reduced: Vec<String>,
    pub blockers_remaining: Vec<String>,
    pub blockers: Vec<String>,
    pub warnings: Vec<String>,
    pub facts: Vec<String>,
    pub next_safe_batch: String,
}

pub async fn rust_manual_default_path_removal_review(
    explicit_opt_in: bool,
    operator_approved: bool,
) -> Result<RustManualDefaultPathRemovalReviewReport> {
    if !explicit_opt_in {
        return Ok(blocked_report(
            explicit_opt_in,
            operator_approved,
            vec!["explicit opt-in is required to review default-path removal".to_owned()],
        ));
    }

    let final_review = go_to_rust_migration_final_review(true).await?;
    let surfaces = surface_reviews(operator_approved).await?;
    let mut blockers = final_review
        .artifact_evidence
        .iter()
        .flat_map(|artifact| artifact.blockers.iter().cloned())
        .collect::<Vec<_>>();
    blockers.extend(surfaces.iter().flat_map(|surface| surface.blockers.iter().cloned()));
    blockers.sort();
    blockers.dedup();

    let removal_allowed_surfaces = surfaces
        .iter()
        .filter(|surface| surface.rust_default_ownership_allowed)
        .map(|surface| surface.default_surface.clone())
        .collect::<Vec<_>>();
    let fallback_retained_surfaces = surfaces
        .iter()
        .filter(|surface| surface.mihomo_fallback_required)
        .map(|surface| surface.default_surface.clone())
        .collect::<Vec<_>>();
    let sidecar_removal_allowed = removal_allowed_surfaces
        .iter()
        .any(|surface| surface == "Mihomo sidecar binary removal");
    let default_path_removal_allowed = surfaces
        .iter()
        .filter(|surface| surface.default_surface != "Mihomo sidecar binary removal")
        .all(|surface| surface.rust_default_ownership_allowed);
    let status = if blockers.is_empty() {
        RustManualDefaultPathRemovalReviewStatus::Ready
    } else {
        RustManualDefaultPathRemovalReviewStatus::Blocked
    };
    let review_artifact = RustManualDefaultPathRemovalReviewArtifact {
        component: COMPONENT.to_owned(),
        created_at_epoch_seconds: epoch_seconds(),
        surfaces,
        removal_allowed_surfaces,
        fallback_retained_surfaces,
        sidecar_removal_allowed,
        mutates_runtime: false,
    };
    let blockers_reduced = review_artifact
        .surfaces
        .iter()
        .flat_map(|surface| surface.blockers_reduced.iter().cloned())
        .collect::<Vec<_>>();
    let blockers_remaining = review_artifact
        .surfaces
        .iter()
        .flat_map(|surface| surface.blockers_remaining.iter().cloned())
        .collect::<Vec<_>>();
    let evidence_path = evidence_path()?;
    let review_artifact_path = review_artifact_path()?;
    let mut report = RustManualDefaultPathRemovalReviewReport {
        runtime_id: RUST_RUNTIME_ID.to_owned(),
        component: COMPONENT.to_owned(),
        kernel_area: KERNEL_AREA.to_owned(),
        status,
        reason: if status == RustManualDefaultPathRemovalReviewStatus::Ready {
            "Rust default-path removal review is ready for operator-approved cutover"
        } else {
            "Rust default-path removal review keeps Mihomo fallback for incomplete or unapproved surfaces"
        }
        .to_owned(),
        explicit_opt_in,
        operator_approved,
        review_artifact: Some(review_artifact),
        review_artifact_path: Some(review_artifact_path.to_string_lossy().to_string()),
        evidence_path: Some(evidence_path.to_string_lossy().to_string()),
        mutates_runtime: false,
        writes_evidence: true,
        default_path_removal_allowed,
        mihomo_binary_removal_allowed: sidecar_removal_allowed,
        blockers_reduced,
        blockers_remaining,
        blockers,
        warnings: warnings(operator_approved),
        facts: facts(),
        next_safe_batch: NEXT_SAFE_BATCH.to_owned(),
    };

    if let Some(parent) = evidence_path.parent() {
        fs::create_dir_all(parent).await?;
    }
    if let Some(review_artifact) = &report.review_artifact {
        fs::write(
            &review_artifact_path,
            serde_yaml_ng::to_string(review_artifact)?.as_bytes(),
        )
        .await?;
    }
    fs::write(&evidence_path, serde_yaml_ng::to_string(&report)?.as_bytes()).await?;
    report.evidence_path = Some(evidence_path.to_string_lossy().to_string());
    Ok(report)
}

fn blocked_report(
    explicit_opt_in: bool,
    operator_approved: bool,
    blockers: Vec<String>,
) -> RustManualDefaultPathRemovalReviewReport {
    RustManualDefaultPathRemovalReviewReport {
        runtime_id: RUST_RUNTIME_ID.to_owned(),
        component: COMPONENT.to_owned(),
        kernel_area: KERNEL_AREA.to_owned(),
        status: RustManualDefaultPathRemovalReviewStatus::Blocked,
        reason: "Rust default-path removal review is blocked".to_owned(),
        explicit_opt_in,
        operator_approved,
        review_artifact: None,
        review_artifact_path: None,
        evidence_path: None,
        mutates_runtime: false,
        writes_evidence: false,
        default_path_removal_allowed: false,
        mihomo_binary_removal_allowed: false,
        blockers_reduced: Vec::new(),
        blockers_remaining: blockers.clone(),
        blockers,
        warnings: warnings(operator_approved),
        facts: facts(),
        next_safe_batch: NEXT_SAFE_BATCH.to_owned(),
    }
}

async fn surface_reviews(operator_approved: bool) -> Result<Vec<RustManualDefaultPathRemovalSurfaceReview>> {
    let mut reviews = Vec::new();
    let specs = surface_specs();
    for spec in specs {
        let artifacts = artifact_evidence(spec.required_artifacts).await?;
        let artifact_blockers = artifacts
            .iter()
            .flat_map(|artifact| artifact.blockers.iter().cloned())
            .collect::<Vec<_>>();
        let mut blockers = artifact_blockers;
        if !operator_approved {
            blockers.push(format!(
                "operator approval is required before removing {}",
                spec.default_surface
            ));
        }
        let rust_default_ownership_allowed = blockers.is_empty();
        reviews.push(RustManualDefaultPathRemovalSurfaceReview {
            default_surface: spec.default_surface.to_owned(),
            retained_fallback_scope: spec
                .retained_fallback_scope
                .iter()
                .map(|scope| (*scope).to_owned())
                .collect(),
            operator_approval_required: true,
            operator_approved,
            artifacts,
            rust_default_ownership_allowed,
            mihomo_fallback_required: !rust_default_ownership_allowed,
            blockers_reduced: if rust_default_ownership_allowed {
                spec.retained_fallback_scope
                    .iter()
                    .map(|scope| format!("{scope} removal approved for cutover handoff"))
                    .collect()
            } else {
                Vec::new()
            },
            blockers_remaining: blockers.clone(),
            blockers,
        });
    }
    Ok(reviews)
}

async fn artifact_evidence(components: &[&'static str]) -> Result<Vec<RustManualDefaultPathRemovalArtifactEvidence>> {
    let mut artifacts = Vec::new();
    for component in components {
        let evidence_path = dirs::app_runtime_dir()?.join(component).join(EVIDENCE_FILE);
        let yaml = fs::read_to_string(&evidence_path).await.ok();
        let value = yaml
            .as_deref()
            .and_then(|yaml| serde_yaml_ng::from_str::<Value>(yaml).ok());
        let status = value
            .as_ref()
            .and_then(|value| value.get("status"))
            .and_then(Value::as_str)
            .map(ToOwned::to_owned);
        let blockers_present = value
            .as_ref()
            .and_then(|value| value.get("blockers"))
            .and_then(Value::as_sequence)
            .map(|blockers| !blockers.is_empty())
            .unwrap_or_else(|| yaml.is_none());
        let artifact_present = yaml.is_some();
        let accepted_for_removal_review =
            artifact_present && matches!(status.as_deref(), Some("passed") | Some("ready")) && !blockers_present;
        let mut blockers = Vec::new();
        if !artifact_present {
            blockers.push(format!("{component} evidence artifact is missing"));
        }
        if artifact_present && !matches!(status.as_deref(), Some("passed") | Some("ready")) {
            blockers.push(format!("{component} status is not ready or passed"));
        }
        if blockers_present {
            blockers.push(format!("{component} contains blockers"));
        }
        artifacts.push(RustManualDefaultPathRemovalArtifactEvidence {
            component: (*component).to_owned(),
            evidence_path: evidence_path.to_string_lossy().to_string(),
            artifact_present,
            status,
            blockers_present,
            accepted_for_removal_review,
            blockers,
        });
    }
    Ok(artifacts)
}

struct SurfaceSpec {
    default_surface: &'static str,
    retained_fallback_scope: &'static [&'static str],
    required_artifacts: &'static [&'static str],
}

fn surface_specs() -> Vec<SurfaceSpec> {
    vec![
        SurfaceSpec {
            default_surface: "default DNS resolver replacement",
            retained_fallback_scope: &[
                "default DNS live resolver replacement",
                "production geodata refresh/file availability",
            ],
            required_artifacts: &[
                "rust-dns-default-path-blocker",
                "rust-dns-cutover-hold-blocker",
                "rust-dns-system-resolver-leak-blocker",
                "rust-geoip-database-blocker",
            ],
        },
        SurfaceSpec {
            default_surface: "non-loopback proxy protocol forwarding defaults",
            retained_fallback_scope: &[
                "QUIC/UDP variants and multiplexed transports",
                "SOCKS non-loopback UDP and fragment queue defaults",
                "unsupported non-loopback encrypted protocols",
                "external plugin process lifecycle",
                "transparent proxy defaults",
            ],
            required_artifacts: &[
                "rust-protocol-default-path-blocker",
                "rust-plugin-process-supervision-blocker",
                "rust-plugin-binary-compatibility-blocker",
                "rust-quic-udp-profile-blocker",
                "rust-default-forwarding-hold-blocker",
                "rust-socks-udp-default-blocker",
                "rust-encrypted-protocol-default-blocker",
            ],
        },
        SurfaceSpec {
            default_surface: "system-wide packet capture and route install",
            retained_fallback_scope: &[
                "system-wide packet capture and route installation",
                "privileged TUN device lifecycle",
                "privileged route mutation rollback",
                "production packet leak hold",
            ],
            required_artifacts: &[
                "rust-route-packet-capture-blocker",
                "rust-tun-device-lifecycle-blocker",
                "rust-route-mutation-rollback-blocker",
                "rust-packet-leak-hold-blocker",
                "rust-tun-packet-capture-hold-bundle",
            ],
        },
        SurfaceSpec {
            default_surface: "Mihomo sidecar binary removal",
            retained_fallback_scope: &["Mihomo sidecar binary fallback", "unsupported fallback list empty"],
            required_artifacts: &[
                "rust-sidecar-independent-rollback",
                "rust-mihomo-fallback-retirement-bundle",
                "go-to-rust-migration-final-review",
            ],
        },
    ]
}

pub async fn approved_manual_default_path_removal_surfaces() -> Result<Vec<String>> {
    let Some(artifact) = read_review_artifact().await? else {
        return Ok(Vec::new());
    };

    Ok(artifact
        .surfaces
        .into_iter()
        .filter(|surface| surface.operator_approved && surface.rust_default_ownership_allowed)
        .map(|surface| surface.default_surface)
        .collect())
}

pub async fn approved_manual_default_path_removal_fallback_scopes() -> Result<Vec<String>> {
    let Some(artifact) = read_review_artifact().await? else {
        return Ok(Vec::new());
    };

    let mut scopes = artifact
        .surfaces
        .into_iter()
        .filter(|surface| surface.operator_approved && surface.rust_default_ownership_allowed)
        .flat_map(|surface| surface.retained_fallback_scope)
        .collect::<Vec<_>>();
    scopes.sort();
    scopes.dedup();
    Ok(scopes)
}

async fn read_review_artifact() -> Result<Option<RustManualDefaultPathRemovalReviewArtifact>> {
    let path = review_artifact_path()?;
    let Some(yaml) = fs::read_to_string(path).await.ok() else {
        return Ok(None);
    };

    Ok(serde_yaml_ng::from_str::<RustManualDefaultPathRemovalReviewArtifact>(&yaml).ok())
}

fn warnings(operator_approved: bool) -> Vec<String> {
    let mut warnings = vec![
        "review evidence does not mutate DNS, routes, TUN, proxy forwarding, plugin processes, or sidecar files"
            .to_owned(),
        "default-path cutover still requires a separate runtime mutation path after approval".to_owned(),
    ];
    if !operator_approved {
        warnings.push("operator approval was not provided, so all Mihomo fallback remains required".to_owned());
    }
    warnings
}

fn facts() -> Vec<String> {
    vec![
        "this review bundles the remaining default-path blockers instead of adding one blocker PR per surface".to_owned(),
        "accepted artifacts must have status ready/passed and an empty blockers list".to_owned(),
        "sidecar removal is reviewed only after DNS, protocol forwarding, route capture, fallback retirement, and rollback evidence are accepted".to_owned(),
    ]
}

fn evidence_path() -> Result<std::path::PathBuf> {
    Ok(dirs::app_runtime_dir()?.join(COMPONENT).join(EVIDENCE_FILE))
}

fn review_artifact_path() -> Result<std::path::PathBuf> {
    Ok(dirs::app_runtime_dir()?.join(COMPONENT).join(REMOVAL_REVIEW_FILE))
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

    #[tokio::test]
    async fn manual_review_requires_explicit_opt_in() {
        let report = rust_manual_default_path_removal_review(false, false).await.unwrap();

        assert_eq!(report.status, RustManualDefaultPathRemovalReviewStatus::Blocked);
        assert!(!report.writes_evidence);
        assert!(
            report
                .blockers
                .iter()
                .any(|blocker| blocker.contains("explicit opt-in"))
        );
    }

    #[test]
    fn manual_review_bundles_remaining_default_surfaces() {
        let specs = surface_specs();

        assert_eq!(specs.len(), 4);
        assert!(specs.iter().any(|spec| spec.default_surface.contains("DNS")));
        assert!(specs.iter().any(|spec| spec.default_surface.contains("sidecar")));
        assert!(specs.iter().all(|spec| spec.required_artifacts.len() >= 3));
    }
}
