use super::{
    MihomoFallbackRetirementEmergencyCheckpoint, MihomoFallbackRetirementExecutionReport,
    MihomoFallbackRetirementExecutionScope, MihomoFallbackRetirementExecutionStatus, RUST_RUNTIME_ID,
    rust_runtime_real_canary_evidence,
};
use crate::utils::dirs;
use anyhow::{Context, Result};
use serde_yaml_ng::Value;
use smartstring::alias::String;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::fs;

const MIHOMO_FALLBACK_RETIREMENT_COMPONENT: &str = "mihomo-fallback-retirement-execution";
const MIHOMO_FALLBACK_RETIREMENT_KERNEL_AREA: &str = "fallback-retirement-execution";
const MIHOMO_FALLBACK_RETIREMENT_MANIFEST_FILE: &str = "execution.yaml";
const MIHOMO_FALLBACK_RETIREMENT_CHECKPOINT_FILE: &str = "emergency-rollback.yaml";
const RUST_RUNTIME_REAL_CANARY_COMPONENT: &str = "rust-runtime-real-canary";
const RUST_RUNTIME_REAL_CANARY_EVIDENCE_FILE: &str = "evidence.yaml";
const NEXT_SAFE_BATCH: &str = "rust-protocol-adapter-forwarding-expansion";

pub async fn mihomo_fallback_retirement_execution_plan() -> Result<MihomoFallbackRetirementExecutionReport> {
    build_mihomo_fallback_retirement_execution_report(false, false).await
}

pub async fn execute_mihomo_fallback_retirement(
    explicit_opt_in: bool,
    run_canary: bool,
) -> Result<MihomoFallbackRetirementExecutionReport> {
    if run_canary {
        let _ = rust_runtime_real_canary_evidence(None, explicit_opt_in).await?;
    }
    let mut report = build_mihomo_fallback_retirement_execution_report(explicit_opt_in, true).await?;
    if report.status == MihomoFallbackRetirementExecutionStatus::Blocked {
        return Ok(report);
    }

    let checkpoint_path = mihomo_fallback_retirement_checkpoint_path()?;
    let manifest_path = mihomo_fallback_retirement_manifest_path()?;
    if let Some(parent) = checkpoint_path.parent() {
        fs::create_dir_all(parent).await?;
    }

    report.status = MihomoFallbackRetirementExecutionStatus::Executed;
    report.reason = "Mihomo fallback retired for the bounded Rust canary scope; unsupported fallback retained".into();
    report.execution_manifest_path = Some(manifest_path.to_string_lossy().to_string().into());
    report.emergency_checkpoint.checkpoint_path = Some(checkpoint_path.to_string_lossy().to_string().into());
    report.mutates_runtime = true;
    report.writes_execution_manifest = true;
    report.retires_supported_fallback = true;

    fs::write(
        &checkpoint_path,
        serde_yaml_ng::to_string(&report.emergency_checkpoint)?.as_bytes(),
    )
    .await?;
    fs::write(&manifest_path, serde_yaml_ng::to_string(&report)?.as_bytes()).await?;
    Ok(report)
}

pub async fn rollback_mihomo_fallback_retirement_execution() -> Result<MihomoFallbackRetirementExecutionReport> {
    let checkpoint_path = mihomo_fallback_retirement_checkpoint_path()?;
    let checkpoint_yaml = fs::read_to_string(&checkpoint_path)
        .await
        .with_context(|| format!("failed to read {}", checkpoint_path.display()))?;
    let checkpoint: MihomoFallbackRetirementEmergencyCheckpoint = serde_yaml_ng::from_str(&checkpoint_yaml)
        .with_context(|| format!("failed to parse {}", checkpoint_path.display()))?;
    let manifest_path = mihomo_fallback_retirement_manifest_path()?;
    let mut report = build_mihomo_fallback_retirement_execution_report(true, false).await?;

    report.status = MihomoFallbackRetirementExecutionStatus::Restored;
    report.reason = "Mihomo fallback retirement execution restored to emergency fallback-retained state".into();
    report.emergency_checkpoint = checkpoint;
    report.execution_manifest_path = Some(manifest_path.to_string_lossy().to_string().into());
    report.mutates_runtime = true;
    report.writes_execution_manifest = true;
    report.retires_supported_fallback = false;
    report.unsupported_mihomo_fallback_retained = true;
    report.blockers = Vec::new();
    report.warnings = vec!["rollback keeps Mihomo fallback for all unsupported scopes".into()];
    fs::write(&manifest_path, serde_yaml_ng::to_string(&report)?.as_bytes()).await?;
    Ok(report)
}

async fn build_mihomo_fallback_retirement_execution_report(
    explicit_opt_in: bool,
    execution_requested: bool,
) -> Result<MihomoFallbackRetirementExecutionReport> {
    let evidence_path = rust_runtime_real_canary_evidence_path()?;
    let evidence = fs::read_to_string(&evidence_path).await.ok();
    let evidence_yaml = evidence
        .as_deref()
        .and_then(|yaml| serde_yaml_ng::from_str::<Value>(yaml).ok());
    let mut blockers = Vec::new();
    if execution_requested && !explicit_opt_in {
        blockers.push("explicit opt-in is required for scoped fallback retirement execution".into());
    }
    if evidence_yaml.is_none() {
        blockers.push("Rust runtime real canary evidence artifact is missing".into());
    }
    if let Some(value) = evidence_yaml.as_ref() {
        let status = value.get("status").and_then(Value::as_str).unwrap_or_default();
        if status != "passed" {
            blockers.push(format!("Rust runtime real canary status is {status}").into());
        }
        if value
            .get("removesMihomoFallback")
            .and_then(Value::as_bool)
            .unwrap_or(false)
        {
            blockers.push("canary evidence unexpectedly removed Mihomo fallback".into());
        }
        if value
            .get("blockers")
            .and_then(Value::as_sequence)
            .map(|blockers| !blockers.is_empty())
            .unwrap_or(false)
        {
            blockers.push("canary evidence contains blockers".into());
        }
    }

    let status = if blockers.is_empty() {
        MihomoFallbackRetirementExecutionStatus::Planned
    } else {
        MihomoFallbackRetirementExecutionStatus::Blocked
    };
    let checkpoint = MihomoFallbackRetirementEmergencyCheckpoint {
        checkpoint_path: None,
        canary_evidence_path: Some(evidence_path.to_string_lossy().to_string().into()),
        previous_execution_manifest_path: fs::try_exists(mihomo_fallback_retirement_manifest_path()?)
            .await?
            .then(|| mihomo_fallback_retirement_manifest_path().map(|path| path.to_string_lossy().to_string().into()))
            .transpose()?,
        retained_fallback_scope: retained_fallback_scope(),
        created_at_epoch_seconds: mihomo_fallback_retirement_epoch_seconds(),
    };

    Ok(MihomoFallbackRetirementExecutionReport {
        runtime_id: RUST_RUNTIME_ID.into(),
        component: MIHOMO_FALLBACK_RETIREMENT_COMPONENT.into(),
        kernel_area: MIHOMO_FALLBACK_RETIREMENT_KERNEL_AREA.into(),
        status,
        reason: if status == MihomoFallbackRetirementExecutionStatus::Planned {
            "scoped Mihomo fallback retirement execution is ready".into()
        } else {
            "scoped Mihomo fallback retirement execution is blocked".into()
        },
        explicit_opt_in,
        supported_scope: supported_execution_scope(),
        emergency_checkpoint: checkpoint,
        execution_manifest_path: Some(
            mihomo_fallback_retirement_manifest_path()?
                .to_string_lossy()
                .to_string()
                .into(),
        ),
        mutates_runtime: false,
        writes_execution_manifest: false,
        retires_supported_fallback: false,
        removes_mihomo_fallback_binary: false,
        unsupported_mihomo_fallback_retained: true,
        blockers,
        warnings: vec![
            "execution scope is limited to loopback DNS, loopback TCP/HTTP forwarding, and route preflight".into(),
            "Mihomo remains fallback for SOCKS, remote adapters, and packet capture".into(),
        ],
        facts: vec![
            "execution writes a durable manifest and emergency rollback checkpoint".into(),
            "execution does not remove the Mihomo binary or unsupported fallback paths".into(),
            "rollback restores the manifest to fallback-retained state".into(),
        ],
        next_safe_batch: NEXT_SAFE_BATCH.into(),
    })
}

fn supported_execution_scope() -> Vec<MihomoFallbackRetirementExecutionScope> {
    vec![
        MihomoFallbackRetirementExecutionScope {
            scope: "loopback-dns-runtime".into(),
            rust_owned_path: "loopback UDP DNS smoke response and DNS runtime parity evidence".into(),
            fallback_retired_for_scope: true,
            mihomo_fallback_retained_for: retained_fallback_scope(),
            evidence: vec!["rust-runtime-real-canary/evidence.yaml#dnsSmokeEvidence".into()],
        },
        MihomoFallbackRetirementExecutionScope {
            scope: "loopback-tcp-http-forwarding".into(),
            rust_owned_path: "Rust TCP accept loop with bidirectional byte forwarding".into(),
            fallback_retired_for_scope: true,
            mihomo_fallback_retained_for: retained_fallback_scope(),
            evidence: vec!["rust-runtime-real-canary/evidence.yaml#protocolForwardingEvidence".into()],
        },
        MihomoFallbackRetirementExecutionScope {
            scope: "route-mode-off-preflight".into(),
            rust_owned_path: "Rust TUN/system-proxy route-mode safety preflight".into(),
            fallback_retired_for_scope: true,
            mihomo_fallback_retained_for: retained_fallback_scope(),
            evidence: vec!["rust-runtime-real-canary/evidence.yaml#tunSystemProxyPreflight".into()],
        },
    ]
}

fn retained_fallback_scope() -> Vec<String> {
    vec![
        "SOCKS protocol handling".into(),
        "remote adapter protocol dialing".into(),
        "system-wide TUN packet capture".into(),
        "transparent proxy routing".into(),
        "unsupported DNS features: fake-ip, fallback-filter, nameserver-policy".into(),
    ]
}

fn rust_runtime_real_canary_evidence_path() -> Result<std::path::PathBuf> {
    Ok(dirs::app_runtime_dir()?
        .join(RUST_RUNTIME_REAL_CANARY_COMPONENT)
        .join(RUST_RUNTIME_REAL_CANARY_EVIDENCE_FILE))
}

fn mihomo_fallback_retirement_manifest_path() -> Result<std::path::PathBuf> {
    Ok(dirs::app_runtime_dir()?
        .join(MIHOMO_FALLBACK_RETIREMENT_COMPONENT)
        .join(MIHOMO_FALLBACK_RETIREMENT_MANIFEST_FILE))
}

fn mihomo_fallback_retirement_checkpoint_path() -> Result<std::path::PathBuf> {
    Ok(dirs::app_runtime_dir()?
        .join(MIHOMO_FALLBACK_RETIREMENT_COMPONENT)
        .join(MIHOMO_FALLBACK_RETIREMENT_CHECKPOINT_FILE))
}

fn mihomo_fallback_retirement_epoch_seconds() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or_default()
}
