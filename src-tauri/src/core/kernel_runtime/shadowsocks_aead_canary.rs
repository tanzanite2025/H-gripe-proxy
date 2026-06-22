use super::{
    RUST_RUNTIME_ID, RustShadowsocksAeadAdapterCanaryFallbackEvidence, RustShadowsocksAeadAdapterCanaryHealthEvidence,
    RustShadowsocksAeadAdapterCanaryReport, RustShadowsocksAeadAdapterCanaryRollbackEvidence,
    RustShadowsocksAeadAdapterCanaryStatus, RustShadowsocksAeadAdapterExecutionReport,
    RustShadowsocksAeadAdapterExecutionStatus, rust_shadowsocks_aead_adapter_execution,
};
use crate::utils::dirs;
use anyhow::Result;
use serde::Deserialize;
use smartstring::alias::String;
use tokio::fs;

const RUST_SHADOWSOCKS_AEAD_ADAPTER_CANARY_COMPONENT: &str = "rust-shadowsocks-aead-adapter-canary";
const RUST_SHADOWSOCKS_AEAD_ADAPTER_CANARY_KERNEL_AREA: &str = "shadowsocks-aead-adapter-canary";
const RUST_SHADOWSOCKS_AEAD_ADAPTER_CANARY_EVIDENCE_FILE: &str = "evidence.yaml";
const NEXT_SAFE_BATCH: &str = "rust-encrypted-proxy-session-expansion";

pub async fn rust_shadowsocks_aead_adapter_canary(
    explicit_opt_in: bool,
) -> Result<RustShadowsocksAeadAdapterCanaryReport> {
    if !explicit_opt_in {
        return Ok(RustShadowsocksAeadAdapterCanaryReport {
            runtime_id: RUST_RUNTIME_ID.into(),
            component: RUST_SHADOWSOCKS_AEAD_ADAPTER_CANARY_COMPONENT.into(),
            kernel_area: RUST_SHADOWSOCKS_AEAD_ADAPTER_CANARY_KERNEL_AREA.into(),
            status: RustShadowsocksAeadAdapterCanaryStatus::Blocked,
            reason: "explicit opt-in is required to run Shadowsocks AEAD adapter canary".into(),
            explicit_opt_in,
            execution_report: None,
            fallback_trigger_evidence: None,
            rollback_checkpoint_evidence: None,
            health_evidence: None,
            evidence_path: None,
            loopback_remote_only: true,
            mutates_runtime: false,
            forwards_traffic: false,
            outbound_adapters_used: false,
            writes_evidence_artifact: false,
            mihomo_fallback: true,
            blockers: vec!["explicit opt-in is required".into()],
            warnings: Vec::new(),
            facts: rust_shadowsocks_aead_adapter_canary_facts(),
            next_safe_batch: NEXT_SAFE_BATCH.into(),
        });
    }

    let execution_report = rust_shadowsocks_aead_adapter_execution(true).await?;
    let fallback_trigger_evidence = canary_fallback_trigger_evidence(&execution_report);
    let rollback_checkpoint_evidence = canary_rollback_checkpoint_evidence(&execution_report).await;
    let health_evidence = canary_health_evidence(&execution_report);
    let mut blockers = Vec::new();
    if execution_report.status != RustShadowsocksAeadAdapterExecutionStatus::Passed {
        blockers.push("Shadowsocks AEAD adapter execution did not pass during canary".into());
        blockers.extend(execution_report.blockers.iter().cloned());
    }
    if !fallback_trigger_evidence.passed {
        blockers.push("Shadowsocks AEAD canary fallback trigger failed".into());
        blockers.extend(fallback_trigger_evidence.blockers.iter().cloned());
    }
    if !rollback_checkpoint_evidence.passed {
        blockers.push("Shadowsocks AEAD canary rollback checkpoint readback failed".into());
        blockers.extend(rollback_checkpoint_evidence.blockers.iter().cloned());
    }
    if !health_evidence.passed {
        blockers.push("Shadowsocks AEAD canary health evidence failed".into());
        blockers.extend(health_evidence.blockers.iter().cloned());
    }
    let status = if blockers.is_empty() {
        RustShadowsocksAeadAdapterCanaryStatus::Passed
    } else {
        RustShadowsocksAeadAdapterCanaryStatus::Failed
    };
    let mut report = RustShadowsocksAeadAdapterCanaryReport {
        runtime_id: RUST_RUNTIME_ID.into(),
        component: RUST_SHADOWSOCKS_AEAD_ADAPTER_CANARY_COMPONENT.into(),
        kernel_area: RUST_SHADOWSOCKS_AEAD_ADAPTER_CANARY_KERNEL_AREA.into(),
        status,
        reason: if status == RustShadowsocksAeadAdapterCanaryStatus::Passed {
            "Rust Shadowsocks AEAD adapter canary passed execution, fallback, rollback, and health evidence".into()
        } else {
            "Rust Shadowsocks AEAD adapter canary failed".into()
        },
        explicit_opt_in,
        execution_report: Some(execution_report),
        fallback_trigger_evidence: Some(fallback_trigger_evidence),
        rollback_checkpoint_evidence: Some(rollback_checkpoint_evidence),
        health_evidence: Some(health_evidence),
        evidence_path: None,
        loopback_remote_only: true,
        mutates_runtime: false,
        forwards_traffic: true,
        outbound_adapters_used: true,
        writes_evidence_artifact: true,
        mihomo_fallback: true,
        blockers,
        warnings: vec![
            "canary is capped to scoped loopback Shadowsocks AEAD TCP execution".into(),
            "Mihomo remains fallback for UDP associate, plugin transports, VMess/VLESS/Trojan, and packet capture"
                .into(),
        ],
        facts: rust_shadowsocks_aead_adapter_canary_facts(),
        next_safe_batch: NEXT_SAFE_BATCH.into(),
    };

    let evidence_path = rust_shadowsocks_aead_adapter_canary_evidence_path()?;
    if let Some(parent) = evidence_path.parent() {
        fs::create_dir_all(parent).await?;
    }
    report.evidence_path = Some(evidence_path.to_string_lossy().to_string().into());
    fs::write(&evidence_path, serde_yaml_ng::to_string(&report)?.as_bytes()).await?;
    Ok(report)
}

fn canary_fallback_trigger_evidence(
    execution_report: &RustShadowsocksAeadAdapterExecutionReport,
) -> RustShadowsocksAeadAdapterCanaryFallbackEvidence {
    let unsupported_protocol = "Shadowsocks UDP associate";
    let mihomo_fallback_retained = execution_report
        .unsupported_protocols
        .iter()
        .any(|protocol| protocol == unsupported_protocol);
    let rust_adapter_bypassed = mihomo_fallback_retained && execution_report.mihomo_fallback;
    let fallback_triggered = rust_adapter_bypassed;
    let mut blockers = Vec::new();
    if !fallback_triggered {
        blockers.push("unsupported Shadowsocks UDP associate did not trigger fallback".into());
    }
    if !rust_adapter_bypassed {
        blockers.push("unsupported protocol was not kept out of scoped Rust AEAD adapter".into());
    }
    if !mihomo_fallback_retained {
        blockers.push("Mihomo fallback was not retained for unsupported Shadowsocks UDP".into());
    }

    RustShadowsocksAeadAdapterCanaryFallbackEvidence {
        trigger_name: "unsupported-shadowsocks-udp-associate".into(),
        unsupported_protocol: unsupported_protocol.into(),
        fallback_triggered,
        rust_adapter_bypassed,
        mihomo_fallback_retained,
        passed: blockers.is_empty(),
        blockers,
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ShadowsocksAeadRollbackCheckpoint {
    component: std::string::String,
    adapter_name: std::string::String,
    fallback_retained_for_unsupported: bool,
    rollback_action: std::string::String,
}

async fn canary_rollback_checkpoint_evidence(
    execution_report: &RustShadowsocksAeadAdapterExecutionReport,
) -> RustShadowsocksAeadAdapterCanaryRollbackEvidence {
    let Some(checkpoint_path) = execution_report.rollback_checkpoint_path.as_ref() else {
        return RustShadowsocksAeadAdapterCanaryRollbackEvidence {
            checkpoint_path: None,
            checkpoint_readable: false,
            component: None,
            adapter_name: None,
            fallback_retained_for_unsupported: false,
            rollback_action: None,
            passed: false,
            blockers: vec!["rollback checkpoint path is missing".into()],
        };
    };
    let checkpoint = fs::read_to_string(std::path::Path::new(checkpoint_path.as_str()))
        .await
        .ok()
        .and_then(|contents| serde_yaml_ng::from_str::<ShadowsocksAeadRollbackCheckpoint>(&contents).ok());
    let checkpoint_readable = checkpoint.is_some();
    let component: Option<String> = checkpoint
        .as_ref()
        .map(|checkpoint| checkpoint.component.clone().into());
    let adapter_name: Option<String> = checkpoint
        .as_ref()
        .map(|checkpoint| checkpoint.adapter_name.clone().into());
    let fallback_retained_for_unsupported = checkpoint
        .as_ref()
        .map(|checkpoint| checkpoint.fallback_retained_for_unsupported)
        .unwrap_or(false);
    let rollback_action: Option<String> = checkpoint
        .as_ref()
        .map(|checkpoint| checkpoint.rollback_action.clone().into());
    let mut blockers = Vec::new();
    if !checkpoint_readable {
        blockers.push("rollback checkpoint could not be read or parsed".into());
    }
    if component.as_deref() != Some("rust-shadowsocks-aead-adapter-execution") {
        blockers.push("rollback checkpoint component did not match AEAD adapter execution".into());
    }
    if adapter_name.as_deref() != Some("rust-shadowsocks-aead-loopback-execution") {
        blockers.push("rollback checkpoint adapter name did not match scoped AEAD adapter".into());
    }
    if !fallback_retained_for_unsupported {
        blockers.push("rollback checkpoint did not retain fallback for unsupported protocols".into());
    }
    if rollback_action
        .as_ref()
        .map(|action| action.as_str().contains("route encrypted protocols back to Mihomo"))
        != Some(true)
    {
        blockers.push("rollback checkpoint action does not route unsupported protocols back to Mihomo".into());
    }

    RustShadowsocksAeadAdapterCanaryRollbackEvidence {
        checkpoint_path: Some(checkpoint_path.clone()),
        checkpoint_readable,
        component,
        adapter_name,
        fallback_retained_for_unsupported,
        rollback_action,
        passed: blockers.is_empty(),
        blockers,
    }
}

fn canary_health_evidence(
    execution_report: &RustShadowsocksAeadAdapterExecutionReport,
) -> RustShadowsocksAeadAdapterCanaryHealthEvidence {
    let execution = execution_report.execution_evidence.as_ref();
    let target_received = execution.map(|execution| execution.target_received).unwrap_or(false);
    let response_status = execution.and_then(|execution| execution.response_status.clone());
    let byte_accounting_passed = execution
        .map(|execution| {
            execution.encrypted_request_bytes > execution.decrypted_request_bytes
                && execution.encrypted_response_bytes > execution.decrypted_response_bytes
                && execution.accepted_connections == 1
        })
        .unwrap_or(false);
    let no_runtime_mutation = !execution_report.mutates_runtime;
    let execution_passed = execution_report.status == RustShadowsocksAeadAdapterExecutionStatus::Passed;
    let mut blockers = Vec::new();
    if !execution_passed {
        blockers.push("AEAD adapter execution report did not pass".into());
    }
    if !execution_report.loopback_remote_only {
        blockers.push("AEAD adapter canary escaped the loopback-only boundary".into());
    }
    if !target_received {
        blockers.push("AEAD adapter canary target did not receive decrypted traffic".into());
    }
    if response_status.as_deref() != Some("HTTP/1.1 204 No Content") {
        blockers.push("AEAD adapter canary did not return target HTTP 204".into());
    }
    if !byte_accounting_passed {
        blockers.push("AEAD adapter canary byte accounting did not pass".into());
    }
    if !no_runtime_mutation {
        blockers.push("AEAD adapter canary mutated runtime state".into());
    }

    RustShadowsocksAeadAdapterCanaryHealthEvidence {
        execution_evidence_path: execution_report.evidence_path.clone(),
        execution_passed,
        loopback_remote_only: execution_report.loopback_remote_only,
        target_received,
        response_status,
        byte_accounting_passed,
        no_runtime_mutation,
        passed: blockers.is_empty(),
        blockers,
    }
}

fn rust_shadowsocks_aead_adapter_canary_evidence_path() -> Result<std::path::PathBuf> {
    Ok(dirs::app_runtime_dir()?
        .join(RUST_SHADOWSOCKS_AEAD_ADAPTER_CANARY_COMPONENT)
        .join(RUST_SHADOWSOCKS_AEAD_ADAPTER_CANARY_EVIDENCE_FILE))
}

fn rust_shadowsocks_aead_adapter_canary_facts() -> Vec<String> {
    vec![
        "Rust reruns the scoped Shadowsocks AEAD adapter execution during canary".into(),
        "the canary reads back the rollback checkpoint written by the execution path".into(),
        "unsupported Shadowsocks UDP stays bypassed to Mihomo fallback".into(),
        "health evidence checks target receipt, HTTP 204 response, loopback boundary, and AEAD byte accounting".into(),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn shadowsocks_aead_adapter_canary_blocks_without_opt_in() {
        let report = rust_shadowsocks_aead_adapter_canary(false).await.unwrap();

        assert_eq!(report.status, RustShadowsocksAeadAdapterCanaryStatus::Blocked);
        assert!(report.mihomo_fallback);
        assert!(!report.forwards_traffic);
    }

    #[test]
    fn canary_fallback_trigger_requires_unsupported_protocol_retention() {
        let report = RustShadowsocksAeadAdapterExecutionReport {
            runtime_id: RUST_RUNTIME_ID.into(),
            component: "rust-shadowsocks-aead-adapter-execution".into(),
            kernel_area: "shadowsocks-aead-adapter".into(),
            status: RustShadowsocksAeadAdapterExecutionStatus::Passed,
            reason: "test".into(),
            explicit_opt_in: true,
            execution_evidence: None,
            unsupported_protocols: vec!["Shadowsocks UDP associate".into()],
            evidence_path: None,
            rollback_checkpoint_path: None,
            loopback_remote_only: true,
            mutates_runtime: false,
            forwards_traffic: true,
            outbound_adapters_used: true,
            writes_evidence_artifact: true,
            mihomo_fallback: true,
            blockers: Vec::new(),
            warnings: Vec::new(),
            facts: Vec::new(),
            next_safe_batch: "test".into(),
        };

        let evidence = canary_fallback_trigger_evidence(&report);

        assert!(evidence.passed);
        assert!(evidence.fallback_triggered);
        assert!(evidence.rust_adapter_bypassed);
    }
}
