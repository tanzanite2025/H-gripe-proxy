use super::RUST_RUNTIME_ID;
use crate::utils::dirs;
use anyhow::{Context as _, Result};
use serde::{Deserialize, Serialize};
use serde_yaml_ng::Value;
use std::path::PathBuf;
use tokio::fs;

const COMPONENT: &str = "go-to-rust-migration-final-review";
const KERNEL_AREA: &str = "migration-final-review";
const EVIDENCE_FILE: &str = "evidence.yaml";
const RUST_OWNED_SCOPE: &str =
    "final reconciliation of bounded Rust execution evidence, retained Mihomo fallback, and sidecar-removal gates";
const NEXT_SAFE_BATCH: &str = "manual-default-path-removal-review";
const REQUIRED_BUNDLES: [(&str, &str); 3] = [
    ("rust-udp-and-plugin-transport-bundle", "UDP/plugin transport evidence"),
    (
        "rust-tun-packet-capture-hold-bundle",
        "TUN packet-capture hold evidence",
    ),
    (
        "rust-mihomo-fallback-retirement-bundle",
        "Mihomo fallback retirement evidence",
    ),
];

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum GoToRustMigrationFinalReviewStatus {
    Passed,
    Blocked,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct GoToRustMigrationFinalReviewArtifactEvidence {
    pub component: String,
    pub label: String,
    pub evidence_path: String,
    pub artifact_present: bool,
    pub status: Option<String>,
    pub blockers_present: bool,
    pub accepted_for_final_review: bool,
    pub blockers: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct GoToRustMigrationFinalReviewRetainedFallbackEvidence {
    pub retained_scope: String,
    pub owner_after_review: String,
    pub reason: String,
    pub removal_allowed: bool,
    pub blockers: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct GoToRustMigrationFinalReviewDefaultRemovalDecision {
    pub default_surface: String,
    pub rust_default_ownership_allowed: bool,
    pub mihomo_fallback_required: bool,
    pub required_evidence: Vec<String>,
    pub blockers: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct GoToRustMigrationFinalReviewSidecarAuditEvidence {
    pub mihomo_source_dir: String,
    pub mihomo_source_present: bool,
    pub sidecar_dir: String,
    pub sidecar_dir_present: bool,
    pub build_script_path: String,
    pub build_script_present: bool,
    pub sidecar_removal_allowed: bool,
    pub passed: bool,
    pub blockers: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GoToRustMigrationFinalReviewReport {
    pub runtime_id: String,
    pub component: String,
    pub kernel_area: String,
    pub status: GoToRustMigrationFinalReviewStatus,
    pub reason: String,
    pub explicit_opt_in: bool,
    pub rust_owned_scope: String,
    pub artifact_evidence: Vec<GoToRustMigrationFinalReviewArtifactEvidence>,
    pub retained_fallback_evidence: Vec<GoToRustMigrationFinalReviewRetainedFallbackEvidence>,
    pub default_removal_decisions: Vec<GoToRustMigrationFinalReviewDefaultRemovalDecision>,
    pub sidecar_audit: GoToRustMigrationFinalReviewSidecarAuditEvidence,
    pub evidence_path: Option<String>,
    pub mutates_runtime: bool,
    pub writes_evidence: bool,
    pub default_path_removal_allowed: bool,
    pub mihomo_binary_removal_allowed: bool,
    pub unsupported_mihomo_fallback_retained: bool,
    pub blockers: Vec<String>,
    pub warnings: Vec<String>,
    pub facts: Vec<String>,
    pub next_safe_batch: String,
}

pub async fn go_to_rust_migration_final_review(explicit_opt_in: bool) -> Result<GoToRustMigrationFinalReviewReport> {
    if !explicit_opt_in {
        return Ok(build_report(
            false,
            Vec::new(),
            retained_fallback_evidence(),
            default_removal_decisions().await?,
            sidecar_audit().await?,
            vec!["explicit opt-in is required to archive Go-to-Rust migration final review".to_owned()],
            None,
        ));
    }

    let artifact_evidence = artifact_evidence().await?;
    let retained_fallback_evidence = retained_fallback_evidence();
    let default_removal_decisions = default_removal_decisions().await?;
    let sidecar_audit = sidecar_audit().await?;
    let mut blockers = Vec::new();
    blockers.extend(
        artifact_evidence
            .iter()
            .flat_map(|artifact| artifact.blockers.iter().cloned()),
    );
    blockers.extend(
        retained_fallback_evidence
            .iter()
            .flat_map(|fallback| fallback.blockers.iter().cloned()),
    );
    blockers.extend(
        default_removal_decisions
            .iter()
            .flat_map(|decision| decision.blockers.iter().cloned()),
    );
    blockers.extend(sidecar_audit.blockers.iter().cloned());

    let evidence_path = evidence_path()?;
    let mut report = build_report(
        true,
        artifact_evidence,
        retained_fallback_evidence,
        default_removal_decisions,
        sidecar_audit,
        blockers,
        Some(evidence_path.to_string_lossy().to_string()),
    );

    if let Some(parent) = evidence_path.parent() {
        fs::create_dir_all(parent).await?;
    }
    report.evidence_path = Some(evidence_path.to_string_lossy().to_string());
    fs::write(&evidence_path, serde_yaml_ng::to_string(&report)?.as_bytes()).await?;
    Ok(report)
}

fn build_report(
    explicit_opt_in: bool,
    artifact_evidence: Vec<GoToRustMigrationFinalReviewArtifactEvidence>,
    retained_fallback_evidence: Vec<GoToRustMigrationFinalReviewRetainedFallbackEvidence>,
    default_removal_decisions: Vec<GoToRustMigrationFinalReviewDefaultRemovalDecision>,
    sidecar_audit: GoToRustMigrationFinalReviewSidecarAuditEvidence,
    blockers: Vec<String>,
    evidence_path: Option<String>,
) -> GoToRustMigrationFinalReviewReport {
    let status = if blockers.is_empty() {
        GoToRustMigrationFinalReviewStatus::Passed
    } else {
        GoToRustMigrationFinalReviewStatus::Blocked
    };
    GoToRustMigrationFinalReviewReport {
        runtime_id: RUST_RUNTIME_ID.to_owned(),
        component: COMPONENT.to_owned(),
        kernel_area: KERNEL_AREA.to_owned(),
        status,
        reason: if status == GoToRustMigrationFinalReviewStatus::Passed {
            "bounded Go-to-Rust migration evidence is reconciled; unsupported Mihomo fallback remains retained"
        } else {
            "Go-to-Rust migration final review is blocked from default-path or Mihomo binary removal"
        }
        .to_owned(),
        explicit_opt_in,
        rust_owned_scope: RUST_OWNED_SCOPE.to_owned(),
        artifact_evidence,
        retained_fallback_evidence,
        default_removal_decisions,
        sidecar_audit,
        evidence_path,
        mutates_runtime: false,
        writes_evidence: explicit_opt_in,
        default_path_removal_allowed: false,
        mihomo_binary_removal_allowed: false,
        unsupported_mihomo_fallback_retained: true,
        blockers,
        warnings: vec![
            "final review is intentionally conservative and does not authorize broad default-path replacement"
                .to_owned(),
            "Mihomo source and sidecar binary fallback remain required until default-path evidence is expanded"
                .to_owned(),
        ],
        facts: facts(),
        next_safe_batch: NEXT_SAFE_BATCH.to_owned(),
    }
}

async fn artifact_evidence() -> Result<Vec<GoToRustMigrationFinalReviewArtifactEvidence>> {
    let mut artifacts = Vec::with_capacity(REQUIRED_BUNDLES.len());
    for (component, label) in REQUIRED_BUNDLES {
        let path = dirs::app_runtime_dir()?.join(component).join("evidence.yaml");
        let yaml = fs::read_to_string(&path).await.ok();
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
        let accepted_for_final_review = artifact_present && status.as_deref() == Some("passed") && !blockers_present;
        let mut blockers = Vec::new();
        if !artifact_present {
            blockers.push(format!("{label} artifact is missing"));
        }
        if artifact_present && status.as_deref() != Some("passed") {
            blockers.push(format!("{label} status is not passed"));
        }
        if blockers_present {
            blockers.push(format!("{label} contains blockers"));
        }
        artifacts.push(GoToRustMigrationFinalReviewArtifactEvidence {
            component: component.to_owned(),
            label: label.to_owned(),
            evidence_path: path.to_string_lossy().to_string(),
            artifact_present,
            status,
            blockers_present,
            accepted_for_final_review,
            blockers,
        });
    }
    Ok(artifacts)
}

fn retained_fallback_evidence() -> Vec<GoToRustMigrationFinalReviewRetainedFallbackEvidence> {
    retained_fallback_scope()
        .into_iter()
        .map(
            |(retained_scope, reason)| GoToRustMigrationFinalReviewRetainedFallbackEvidence {
                retained_scope: retained_scope.to_owned(),
                owner_after_review: "Mihomo/service fallback".to_owned(),
                reason: reason.to_owned(),
                removal_allowed: false,
                blockers: vec![format!("{retained_scope} remains Mihomo-owned after final review")],
            },
        )
        .collect()
}

async fn default_removal_decisions() -> Result<Vec<GoToRustMigrationFinalReviewDefaultRemovalDecision>> {
    let dns_blockers = if dns_default_path_blocker_ready().await? {
        vec![
            "production default DNS cutover hold window".to_owned(),
            "system resolver handoff and leak observation on real profiles".to_owned(),
        ]
    } else {
        vec![
            "live resolver replacement evidence".to_owned(),
            "production persistent DNS cache migration".to_owned(),
            "geodata refresh ownership".to_owned(),
        ]
    };
    let mut sidecar_required_evidence = vec![
        "all default-path owners moved to Rust".to_owned(),
        "unsupported fallback list empty".to_owned(),
    ];
    if !sidecar_independent_rollback_ready().await? {
        sidecar_required_evidence.push("emergency rollback no longer depends on sidecar".to_owned());
    }

    Ok(vec![
        default_removal_decision("default DNS resolver replacement", dns_blockers),
        default_removal_decision(
            "system-wide packet capture and route install",
            vec![
                "real TUN device lifecycle ownership".to_owned(),
                "host route table mutation and rollback on all platforms".to_owned(),
                "post-cutover packet leak hold window".to_owned(),
            ],
        ),
        default_removal_decision(
            "non-loopback proxy protocol forwarding defaults",
            vec![
                "non-loopback Shadowsocks/Vmess/VLESS/Trojan/QUIC evidence".to_owned(),
                "multiplexed transport coverage".to_owned(),
                "external plugin lifecycle replacement".to_owned(),
            ],
        ),
        default_removal_decision("Mihomo sidecar binary removal", sidecar_required_evidence),
    ])
}

async fn dns_default_path_blocker_ready() -> Result<bool> {
    let evidence_path = dirs::app_runtime_dir()?
        .join("rust-dns-default-path-blocker")
        .join("evidence.yaml");
    let yaml = fs::read_to_string(evidence_path).await.ok();
    let value = yaml
        .as_deref()
        .and_then(|yaml| serde_yaml_ng::from_str::<Value>(yaml).ok());
    let status_ready = value
        .as_ref()
        .and_then(|value| value.get("status"))
        .and_then(Value::as_str)
        == Some("ready");
    let blockers_empty = value
        .as_ref()
        .and_then(|value| value.get("blockers"))
        .and_then(Value::as_sequence)
        .map(|blockers| blockers.is_empty())
        .unwrap_or(false);
    Ok(status_ready && blockers_empty)
}

async fn sidecar_independent_rollback_ready() -> Result<bool> {
    let evidence_path = dirs::app_runtime_dir()?
        .join("rust-sidecar-independent-rollback")
        .join("evidence.yaml");
    let yaml = fs::read_to_string(evidence_path).await.ok();
    let value = yaml
        .as_deref()
        .and_then(|yaml| serde_yaml_ng::from_str::<Value>(yaml).ok());
    let status_ready = value
        .as_ref()
        .and_then(|value| value.get("status"))
        .and_then(Value::as_str)
        == Some("ready");
    let blockers_empty = value
        .as_ref()
        .and_then(|value| value.get("blockers"))
        .and_then(Value::as_sequence)
        .map(|blockers| blockers.is_empty())
        .unwrap_or(false);
    Ok(status_ready && blockers_empty)
}

fn default_removal_decision(
    default_surface: &str,
    required_evidence: Vec<String>,
) -> GoToRustMigrationFinalReviewDefaultRemovalDecision {
    GoToRustMigrationFinalReviewDefaultRemovalDecision {
        default_surface: default_surface.to_owned(),
        rust_default_ownership_allowed: false,
        mihomo_fallback_required: true,
        blockers: required_evidence
            .iter()
            .map(|evidence| format!("missing default-path evidence: {evidence}"))
            .collect(),
        required_evidence,
    }
}

async fn sidecar_audit() -> Result<GoToRustMigrationFinalReviewSidecarAuditEvidence> {
    let repo_root = repo_root()?;
    let mihomo_source_dir = repo_root.join("mihomo");
    let sidecar_dir = repo_root.join("src-tauri").join("sidecar");
    let build_script_path = repo_root.join("scripts").join("build-mihomo-sidecar.mjs");
    let mihomo_source_present = fs::try_exists(&mihomo_source_dir).await?;
    let sidecar_dir_present = fs::try_exists(&sidecar_dir).await?;
    let build_script_present = fs::try_exists(&build_script_path).await?;
    let sidecar_removal_allowed = false;
    let passed = mihomo_source_present && sidecar_dir_present && build_script_present && !sidecar_removal_allowed;

    Ok(GoToRustMigrationFinalReviewSidecarAuditEvidence {
        mihomo_source_dir: mihomo_source_dir.to_string_lossy().to_string(),
        mihomo_source_present,
        sidecar_dir: sidecar_dir.to_string_lossy().to_string(),
        sidecar_dir_present,
        build_script_path: build_script_path.to_string_lossy().to_string(),
        build_script_present,
        sidecar_removal_allowed,
        passed,
        blockers: if passed {
            Vec::new()
        } else {
            vec!["sidecar dependency audit could not verify retained Mihomo fallback files".to_owned()]
        },
    })
}

fn retained_fallback_scope() -> Vec<(&'static str, &'static str)> {
    vec![
        (
            "SOCKS non-loopback UDP and fragment queue defaults",
            "bounded loopback UDP evidence does not replace broad default UDP routing",
        ),
        (
            "default DNS live resolver replacement",
            "bounded DNS evidence does not own production resolver replacement, persistent cache, or geodata refresh",
        ),
        (
            "unsupported non-loopback encrypted protocols",
            "bounded loopback protocol canaries do not cover all real remote protocol paths",
        ),
        (
            "QUIC/UDP variants and multiplexed transports",
            "transport coverage remains incomplete outside bounded canaries",
        ),
        (
            "external plugin process lifecycle",
            "plugin shim evidence does not replace production plugin process management",
        ),
        (
            "system-wide packet capture and route installation",
            "synthetic packet-capture evidence does not mutate host routes or own TUN devices",
        ),
        (
            "transparent proxy defaults",
            "bounded transparent routing evidence does not replace default forwarding",
        ),
        (
            "Mihomo sidecar binary fallback",
            "emergency rollback and unsupported paths still require the sidecar",
        ),
    ]
}

fn facts() -> Vec<String> {
    vec![
        "final review reconciles the three accelerated bundle artifacts before any default-path claim".to_owned(),
        "default-path and Mihomo binary removal remain blocked unless retained fallback scope becomes empty".to_owned(),
        "sidecar source, sidecar directory, and build script are audited as retained dependencies".to_owned(),
    ]
}

fn repo_root() -> Result<PathBuf> {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .map(std::path::Path::to_path_buf)
        .context("resolve repository root")
}

fn evidence_path() -> Result<PathBuf> {
    Ok(dirs::app_runtime_dir()?.join(COMPONENT).join(EVIDENCE_FILE))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn final_review_keeps_sidecar_removal_blocked() {
        let decisions = default_removal_decisions().await.unwrap();

        assert!(
            decisions
                .iter()
                .any(|decision| decision.default_surface.contains("sidecar"))
        );
        assert!(
            decisions
                .iter()
                .all(|decision| !decision.rust_default_ownership_allowed)
        );
    }

    #[test]
    fn retained_fallback_scope_is_not_empty() {
        let retained = retained_fallback_scope();

        assert!(retained.iter().any(|(scope, _)| scope.contains("packet capture")));
        assert!(retained.iter().any(|(scope, _)| scope.contains("sidecar")));
    }
}
