use super::{
    MihomoFallbackRetirementExecutionReport, MihomoFallbackRetirementExecutionScope,
    MihomoFallbackRetirementExecutionStatus, RUST_RUNTIME_ID, RustDefaultDataPlaneCloseoutEvidenceOwnership,
    RustDefaultDataPlaneCloseoutReport, RustDefaultDataPlaneCloseoutStatus, RustDefaultDataPlaneUnsupportedBlocker,
};
use crate::utils::dirs;
use anyhow::{Context, Result};
use smartstring::alias::String;
use tokio::fs;

const RUST_DEFAULT_DATA_PLANE_CLOSEOUT_COMPONENT: &str = "rust-default-data-plane-closeout";
const RUST_DEFAULT_DATA_PLANE_CLOSEOUT_KERNEL_AREA: &str = "default-data-plane-closeout";
const RUST_DEFAULT_DATA_PLANE_CLOSEOUT_MANIFEST_FILE: &str = "closeout.yaml";
const MIHOMO_FALLBACK_RETIREMENT_COMPONENT: &str = "mihomo-fallback-retirement-execution";
const MIHOMO_FALLBACK_RETIREMENT_MANIFEST_FILE: &str = "execution.yaml";
const NEXT_SAFE_BATCH: &str = "unsupported-protocol-and-packet-capture-implementation";

pub async fn rust_default_data_plane_closeout_plan() -> Result<RustDefaultDataPlaneCloseoutReport> {
    build_rust_default_data_plane_closeout_report(false, false).await
}

pub async fn closeout_rust_default_data_plane(explicit_opt_in: bool) -> Result<RustDefaultDataPlaneCloseoutReport> {
    let mut report = build_rust_default_data_plane_closeout_report(explicit_opt_in, true).await?;
    if report.status == RustDefaultDataPlaneCloseoutStatus::Blocked {
        return Ok(report);
    }

    let closeout_manifest_path = rust_default_data_plane_closeout_manifest_path()?;
    if let Some(parent) = closeout_manifest_path.parent() {
        fs::create_dir_all(parent).await?;
    }

    report.status = RustDefaultDataPlaneCloseoutStatus::ClosedOut;
    report.reason = "bounded Rust data-plane ownership reconciled against wider fallback retirement evidence".into();
    report.closeout_manifest_path = Some(closeout_manifest_path.to_string_lossy().to_string().into());
    report.writes_closeout_manifest = true;
    fs::write(&closeout_manifest_path, serde_yaml_ng::to_string(&report)?.as_bytes()).await?;
    Ok(report)
}

async fn build_rust_default_data_plane_closeout_report(
    explicit_opt_in: bool,
    write_requested: bool,
) -> Result<RustDefaultDataPlaneCloseoutReport> {
    let fallback_manifest_path = mihomo_fallback_retirement_manifest_path()?;
    let fallback_manifest = read_fallback_retirement_manifest(&fallback_manifest_path).await?;
    let mut blockers = closeout_blockers_from_report(fallback_manifest.as_ref());
    if write_requested && !explicit_opt_in {
        blockers.push("explicit closeout opt-in is required before writing the closeout manifest".into());
    }

    let evidence_ownership = fallback_manifest
        .as_ref()
        .map(|manifest| closeout_evidence_ownership_from_scope(&manifest.supported_scope))
        .unwrap_or_default();
    let ownership_reconciled = !evidence_ownership.is_empty()
        && evidence_ownership
            .iter()
            .all(|ownership| ownership.default_eligible && !ownership.evidence.is_empty());
    if fallback_manifest.is_some() && !ownership_reconciled {
        blockers
            .push("bounded evidence ownership could not be reconciled to supported fallback-retirement scopes".into());
    }

    let status = if blockers.is_empty() {
        RustDefaultDataPlaneCloseoutStatus::Ready
    } else {
        RustDefaultDataPlaneCloseoutStatus::Blocked
    };
    let unsupported_blockers = rust_default_data_plane_unsupported_blockers();

    Ok(RustDefaultDataPlaneCloseoutReport {
        runtime_id: RUST_RUNTIME_ID.into(),
        component: RUST_DEFAULT_DATA_PLANE_CLOSEOUT_COMPONENT.into(),
        kernel_area: RUST_DEFAULT_DATA_PLANE_CLOSEOUT_KERNEL_AREA.into(),
        status,
        reason: if status == RustDefaultDataPlaneCloseoutStatus::Ready {
            "bounded Rust data-plane closeout is ready to archive".into()
        } else {
            "bounded Rust data-plane closeout is blocked".into()
        },
        explicit_opt_in,
        mutates_runtime: false,
        writes_closeout_manifest: false,
        closeout_manifest_path: Some(
            rust_default_data_plane_closeout_manifest_path()?
                .to_string_lossy()
                .to_string()
                .into(),
        ),
        fallback_retirement_manifest_path: Some(fallback_manifest_path.to_string_lossy().to_string().into()),
        evidence_ownership,
        unsupported_blockers,
        ownership_reconciled,
        default_scope_locked_to_passed_evidence: ownership_reconciled,
        unsupported_mihomo_fallback_retained: fallback_manifest
            .as_ref()
            .map(|manifest| manifest.unsupported_mihomo_fallback_retained)
            .unwrap_or(false),
        removes_mihomo_fallback_binary: fallback_manifest
            .as_ref()
            .map(|manifest| manifest.removes_mihomo_fallback_binary)
            .unwrap_or(false),
        blockers,
        warnings: vec![
            "closeout does not broaden default ownership beyond passed bounded evidence".into(),
            "Mihomo fallback remains required for unsupported protocols, UDP, route install, and packet capture".into(),
        ],
        facts: vec![
            "closeout consumes the wider fallback retirement execution manifest".into(),
            "supported Rust-owned scope is converted into explicit evidence ownership records".into(),
            "remaining Mihomo-owned blockers are preserved as first-class closeout output".into(),
        ],
        next_safe_batch: NEXT_SAFE_BATCH.into(),
    })
}

async fn read_fallback_retirement_manifest(
    fallback_manifest_path: &std::path::Path,
) -> Result<Option<MihomoFallbackRetirementExecutionReport>> {
    let manifest_yaml = match fs::read_to_string(fallback_manifest_path).await {
        Ok(manifest_yaml) => manifest_yaml,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => {
            return Err(error).with_context(|| format!("failed to read {}", fallback_manifest_path.display()));
        }
    };
    serde_yaml_ng::from_str(&manifest_yaml)
        .with_context(|| format!("failed to parse {}", fallback_manifest_path.display()))
        .map(Some)
}

fn closeout_blockers_from_report(fallback_manifest: Option<&MihomoFallbackRetirementExecutionReport>) -> Vec<String> {
    let Some(manifest) = fallback_manifest else {
        return vec![
            "wider fallback retirement execution manifest is missing; run mihomo-fallback-retirement-execution first"
                .into(),
        ];
    };

    let mut blockers = Vec::new();
    if manifest.status != MihomoFallbackRetirementExecutionStatus::Executed {
        blockers.push("wider fallback retirement execution manifest has not reached executed status".into());
    }
    if !manifest.retires_supported_fallback {
        blockers.push("supported fallback retirement was not recorded in the execution manifest".into());
    }
    if !manifest.unsupported_mihomo_fallback_retained {
        blockers.push("unsupported Mihomo fallback retention is not recorded in the execution manifest".into());
    }
    if manifest.removes_mihomo_fallback_binary {
        blockers.push("fallback retirement manifest unexpectedly removes the Mihomo fallback binary".into());
    }
    if !manifest.blockers.is_empty() {
        blockers.push("fallback retirement manifest still contains blockers".into());
    }
    if manifest.supported_scope.is_empty() {
        blockers.push("fallback retirement manifest has no supported Rust-owned scopes".into());
    }
    blockers
}

fn closeout_evidence_ownership_from_scope(
    supported_scope: &[MihomoFallbackRetirementExecutionScope],
) -> Vec<RustDefaultDataPlaneCloseoutEvidenceOwnership> {
    supported_scope
        .iter()
        .map(|scope| RustDefaultDataPlaneCloseoutEvidenceOwnership {
            scope: scope.scope.clone(),
            rust_owned_path: scope.rust_owned_path.clone(),
            evidence: scope.evidence.clone(),
            mihomo_fallback_retained_for: scope.mihomo_fallback_retained_for.clone(),
            default_eligible: scope.fallback_retired_for_scope
                && !scope.evidence.is_empty()
                && !scope.mihomo_fallback_retained_for.is_empty(),
        })
        .collect()
}

fn rust_default_data_plane_unsupported_blockers() -> Vec<RustDefaultDataPlaneUnsupportedBlocker> {
    vec![
        RustDefaultDataPlaneUnsupportedBlocker {
            blocker: "VMess, VLESS, and Trojan encrypted session implementations".into(),
            mihomo_owner: "Mihomo protocol stack".into(),
            retirement_requirement:
                "separate Rust implementations with canary, rollback, hold, and byte-accounting evidence".into(),
        },
        RustDefaultDataPlaneUnsupportedBlocker {
            blocker: "SOCKS non-loopback UDP plus fragment queues/timeouts and Shadowsocks UDP/plugin transports".into(),
            mihomo_owner: "Mihomo adapter runtime".into(),
            retirement_requirement: "bounded UDP/plugin execution, non-loopback UDP evidence, and unsupported fallback preservation".into(),
        },
        RustDefaultDataPlaneUnsupportedBlocker {
            blocker: "system-wide TUN packet capture and transparent routing defaults".into(),
            mihomo_owner: "Mihomo service/TUN runtime".into(),
            retirement_requirement:
                "platform route install, packet capture, rollback, and leak evidence across Windows, macOS, and Linux"
                    .into(),
        },
        RustDefaultDataPlaneUnsupportedBlocker {
            blocker: "DNS default runtime ownership, live resolver replacement, full GeoIP database loading, production persistent cache storage, and geodata refresh".into(),
            mihomo_owner: "Mihomo DNS runtime".into(),
            retirement_requirement: "default DNS runtime parity, live resolver replacement, full GeoIP database loading, production persistent cache storage, and geodata refresh evidence while Rust adapters are active"
                .into(),
        },
        RustDefaultDataPlaneUnsupportedBlocker {
            blocker: "connectivity-preserving fallback for every unsupported path without app restart".into(),
            mihomo_owner: "Mihomo fallback bridge".into(),
            retirement_requirement: "fallback trigger, health telemetry, rollback, and post-canary hold evidence"
                .into(),
        },
        RustDefaultDataPlaneUnsupportedBlocker {
            blocker: "full Mihomo fallback binary removal".into(),
            mihomo_owner: "in-repo Mihomo sidecar packaging".into(),
            retirement_requirement: "all unsupported protocol, DNS, adapter, TUN, and packet-capture blockers retired"
                .into(),
        },
    ]
}

fn mihomo_fallback_retirement_manifest_path() -> Result<std::path::PathBuf> {
    Ok(dirs::app_runtime_dir()?
        .join(MIHOMO_FALLBACK_RETIREMENT_COMPONENT)
        .join(MIHOMO_FALLBACK_RETIREMENT_MANIFEST_FILE))
}

fn rust_default_data_plane_closeout_manifest_path() -> Result<std::path::PathBuf> {
    Ok(dirs::app_runtime_dir()?
        .join(RUST_DEFAULT_DATA_PLANE_CLOSEOUT_COMPONENT)
        .join(RUST_DEFAULT_DATA_PLANE_CLOSEOUT_MANIFEST_FILE))
}

#[cfg(test)]
mod tests {
    use super::super::MihomoFallbackRetirementEmergencyCheckpoint;
    use super::*;

    #[test]
    fn closeout_blockers_require_executed_fallback_manifest() {
        let manifest = MihomoFallbackRetirementExecutionReport {
            runtime_id: RUST_RUNTIME_ID.into(),
            component: MIHOMO_FALLBACK_RETIREMENT_COMPONENT.into(),
            kernel_area: "fallback-retirement-execution".into(),
            status: MihomoFallbackRetirementExecutionStatus::Planned,
            reason: "planned".into(),
            explicit_opt_in: true,
            supported_scope: vec![],
            emergency_checkpoint: MihomoFallbackRetirementEmergencyCheckpoint {
                checkpoint_path: None,
                canary_evidence_path: None,
                previous_execution_manifest_path: None,
                retained_fallback_scope: vec![],
                created_at_epoch_seconds: 0,
            },
            execution_manifest_path: None,
            mutates_runtime: false,
            writes_execution_manifest: false,
            retires_supported_fallback: false,
            removes_mihomo_fallback_binary: false,
            unsupported_mihomo_fallback_retained: false,
            blockers: vec![],
            warnings: vec![],
            facts: vec![],
            next_safe_batch: RUST_DEFAULT_DATA_PLANE_CLOSEOUT_COMPONENT.into(),
        };

        let blockers = closeout_blockers_from_report(Some(&manifest));

        assert!(blockers.iter().any(|blocker| blocker.contains("executed status")));
        assert!(blockers.iter().any(|blocker| blocker.contains("supported fallback")));
        assert!(
            blockers
                .iter()
                .any(|blocker| blocker.contains("unsupported Mihomo fallback"))
        );
    }

    #[test]
    fn closeout_scope_maps_to_default_eligible_evidence_ownership() {
        let scope = MihomoFallbackRetirementExecutionScope {
            scope: "http-connect-proxy-adapter".into(),
            rust_owned_path: "Rust HTTP CONNECT adapter tunnel".into(),
            fallback_retired_for_scope: true,
            mihomo_fallback_retained_for: vec!["VMess".into()],
            evidence: vec!["rust-http-connect-proxy-adapter/evidence.yaml".into()],
        };

        let ownership = closeout_evidence_ownership_from_scope(&[scope]);

        assert_eq!(ownership.len(), 1);
        assert_eq!(ownership[0].scope, "http-connect-proxy-adapter");
        assert!(ownership[0].default_eligible);
    }

    #[test]
    fn unsupported_blockers_preserve_mihomo_owned_runtime_boundaries() {
        let blockers = rust_default_data_plane_unsupported_blockers();

        assert!(blockers.iter().any(|blocker| blocker.blocker.contains("VMess")));
        assert!(
            blockers
                .iter()
                .any(|blocker| blocker.blocker.contains("packet capture"))
        );
        assert!(
            blockers
                .iter()
                .any(|blocker| blocker.blocker.contains("binary removal"))
        );
    }
}
