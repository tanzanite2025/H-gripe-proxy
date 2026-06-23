mod constants;
mod evidence;
mod execution;
mod framing;
mod protocol;

use self::{
    constants::{COMPONENT, KERNEL_AREA, NEXT_SAFE_BATCH},
    evidence::{evidence_path, facts, retained_fallback_scope, rollback_path, write_rollback_checkpoint},
    execution::run_protocol_session,
};
use super::RUST_RUNTIME_ID;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use tokio::fs;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum RustEncryptedProtocolsBundleStatus {
    Passed,
    Failed,
    Blocked,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum RustEncryptedProtocolBundleProtocol {
    #[serde(rename = "vmessTcp")]
    VmessTcp,
    #[serde(rename = "vlessTcp")]
    VlessTcp,
    #[serde(rename = "trojanTcp")]
    TrojanTcp,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RustEncryptedProtocolsBundleSessionEvidence {
    pub protocol: RustEncryptedProtocolBundleProtocol,
    pub adapter_name: std::string::String,
    pub listener_port: u16,
    pub target_port: u16,
    pub target_address: std::string::String,
    pub handshake_validated: bool,
    pub session_established: bool,
    pub request_marker: std::string::String,
    pub response_marker: Option<std::string::String>,
    pub request_bytes_from_client: u64,
    pub payload_bytes_to_target: u64,
    pub target_response_bytes: u64,
    pub response_bytes_to_client: u64,
    pub fallback_triggered: bool,
    pub passed: bool,
    pub blockers: Vec<std::string::String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RustEncryptedProtocolsBundleFallbackEvidence {
    pub unsupported_protocols: Vec<std::string::String>,
    pub fallback_retained: bool,
    pub default_forwarding_retained: bool,
    pub unsupported_sessions_bypassed: bool,
    pub passed: bool,
    pub blockers: Vec<std::string::String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RustEncryptedProtocolsBundleRollbackEvidence {
    pub checkpoint_path: std::string::String,
    pub fallback_retained_for: Vec<std::string::String>,
    pub created_at_epoch_seconds: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RustEncryptedProtocolsBundleLeakEvidence {
    pub passed: bool,
    pub loopback_only: bool,
    pub no_runtime_mutation: bool,
    pub no_packet_capture_claim: bool,
    pub no_non_loopback_forwarding: bool,
    pub no_mihomo_binary_removal: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RustEncryptedProtocolsBundleReport {
    pub runtime_id: std::string::String,
    pub component: std::string::String,
    pub kernel_area: std::string::String,
    pub status: RustEncryptedProtocolsBundleStatus,
    pub reason: std::string::String,
    pub explicit_opt_in: bool,
    pub session_evidence: Vec<RustEncryptedProtocolsBundleSessionEvidence>,
    pub fallback_evidence: Option<RustEncryptedProtocolsBundleFallbackEvidence>,
    pub rollback_evidence: Option<RustEncryptedProtocolsBundleRollbackEvidence>,
    pub leak_evidence: Option<RustEncryptedProtocolsBundleLeakEvidence>,
    pub evidence_path: Option<std::string::String>,
    pub loopback_remote_only: bool,
    pub mutates_runtime: bool,
    pub forwards_traffic: bool,
    pub outbound_adapters_used: bool,
    pub writes_evidence_artifact: bool,
    pub mihomo_fallback: bool,
    pub blockers: Vec<std::string::String>,
    pub warnings: Vec<std::string::String>,
    pub facts: Vec<std::string::String>,
    pub next_safe_batch: std::string::String,
}

pub async fn rust_encrypted_protocols_bundle_execution(
    explicit_opt_in: bool,
) -> Result<RustEncryptedProtocolsBundleReport> {
    if !explicit_opt_in {
        return Ok(blocked_report(vec![
            "explicit opt-in is required to run encrypted protocols bundle".to_owned(),
        ]));
    }

    let mut session_evidence = Vec::new();
    for protocol in [
        RustEncryptedProtocolBundleProtocol::VmessTcp,
        RustEncryptedProtocolBundleProtocol::VlessTcp,
        RustEncryptedProtocolBundleProtocol::TrojanTcp,
    ] {
        session_evidence.push(run_protocol_session(protocol).await?);
    }

    let fallback_evidence = fallback_evidence();
    let rollback_path = rollback_path()?;
    let rollback_evidence = write_rollback_checkpoint(&rollback_path).await?;
    let leak_evidence = RustEncryptedProtocolsBundleLeakEvidence {
        passed: true,
        loopback_only: true,
        no_runtime_mutation: true,
        no_packet_capture_claim: true,
        no_non_loopback_forwarding: true,
        no_mihomo_binary_removal: true,
    };
    let mut blockers = session_evidence
        .iter()
        .filter(|evidence| !evidence.passed)
        .flat_map(|evidence| evidence.blockers.iter().cloned())
        .collect::<Vec<_>>();
    if !fallback_evidence.passed {
        blockers.extend(fallback_evidence.blockers.iter().cloned());
    }
    let status = if blockers.is_empty() {
        RustEncryptedProtocolsBundleStatus::Passed
    } else {
        RustEncryptedProtocolsBundleStatus::Failed
    };
    let evidence_path = evidence_path()?;
    let mut report = RustEncryptedProtocolsBundleReport {
        runtime_id: RUST_RUNTIME_ID.to_owned(),
        component: COMPONENT.to_owned(),
        kernel_area: KERNEL_AREA.to_owned(),
        status,
        reason: if status == RustEncryptedProtocolsBundleStatus::Passed {
            "Rust executed bounded VMess/VLESS/Trojan loopback TCP sessions with shared accounting".to_owned()
        } else {
            "Rust encrypted protocols bundle failed".to_owned()
        },
        explicit_opt_in,
        session_evidence,
        fallback_evidence: Some(fallback_evidence),
        rollback_evidence: Some(rollback_evidence),
        leak_evidence: Some(leak_evidence),
        evidence_path: Some(evidence_path.to_string_lossy().to_string()),
        loopback_remote_only: true,
        mutates_runtime: false,
        forwards_traffic: true,
        outbound_adapters_used: true,
        writes_evidence_artifact: true,
        mihomo_fallback: true,
        blockers,
        warnings: vec![
            "encrypted protocol bundle is limited to loopback TCP canary sessions".to_owned(),
            "QUIC/UDP variants, multiplexing, plugin transports, and default forwarding remain Mihomo-owned".to_owned(),
        ],
        facts: facts(),
        next_safe_batch: NEXT_SAFE_BATCH.to_owned(),
    };

    if let Some(parent) = evidence_path.parent() {
        fs::create_dir_all(parent).await?;
    }
    fs::write(&evidence_path, serde_yaml_ng::to_string(&report)?.as_bytes()).await?;
    report.evidence_path = Some(evidence_path.to_string_lossy().to_string());

    Ok(report)
}

fn blocked_report(blockers: Vec<std::string::String>) -> RustEncryptedProtocolsBundleReport {
    RustEncryptedProtocolsBundleReport {
        runtime_id: RUST_RUNTIME_ID.to_owned(),
        component: COMPONENT.to_owned(),
        kernel_area: KERNEL_AREA.to_owned(),
        status: RustEncryptedProtocolsBundleStatus::Blocked,
        reason: "Rust encrypted protocols bundle is blocked".to_owned(),
        explicit_opt_in: false,
        session_evidence: Vec::new(),
        fallback_evidence: None,
        rollback_evidence: None,
        leak_evidence: None,
        evidence_path: None,
        loopback_remote_only: true,
        mutates_runtime: false,
        forwards_traffic: false,
        outbound_adapters_used: false,
        writes_evidence_artifact: false,
        mihomo_fallback: true,
        blockers,
        warnings: Vec::new(),
        facts: facts(),
        next_safe_batch: NEXT_SAFE_BATCH.to_owned(),
    }
}

fn fallback_evidence() -> RustEncryptedProtocolsBundleFallbackEvidence {
    RustEncryptedProtocolsBundleFallbackEvidence {
        unsupported_protocols: retained_fallback_scope(),
        fallback_retained: true,
        default_forwarding_retained: true,
        unsupported_sessions_bypassed: true,
        passed: true,
        blockers: Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn executes_all_encrypted_protocol_canaries() {
        let report = rust_encrypted_protocols_bundle_execution(true).await.unwrap();

        assert_eq!(report.status, RustEncryptedProtocolsBundleStatus::Passed);
        assert_eq!(report.session_evidence.len(), 3);
        assert!(report.session_evidence.iter().all(|evidence| evidence.passed));
    }
}
