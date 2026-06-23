mod constants;
mod evidence;
mod execution;
mod protocol;
mod reassembly;

use self::{
    constants::{
        NEXT_SAFE_BATCH, RUST_SOCKS_UDP_FRAGMENTS_COMPONENT, RUST_SOCKS_UDP_FRAGMENTS_KERNEL_AREA,
        RUST_SOCKS_UDP_FRAGMENTS_OWNED_SCOPE,
    },
    evidence::{
        retained_socks_udp_fragments_fallback_scope, rust_socks_udp_fragments_evidence_path,
        rust_socks_udp_fragments_facts, rust_socks_udp_fragments_rollback_path, write_rollback_checkpoint,
    },
    execution::run_bounded_socks_udp_fragment_reassembly,
};
use super::{
    RUST_RUNTIME_ID, RustDefaultDataPlaneCloseoutGateEvidence, RustSocksUdpFragmentsExecutionReport,
    RustSocksUdpFragmentsExecutionStatus, RustSocksUdpFragmentsLeakEvidence,
    rust_default_data_plane_closeout_gate_evidence,
};
use anyhow::Result;
use smartstring::alias::String;
use tokio::fs;

pub async fn rust_socks_udp_fragments_execution(explicit_opt_in: bool) -> Result<RustSocksUdpFragmentsExecutionReport> {
    let default_data_plane_closeout_gate = rust_default_data_plane_closeout_gate_evidence().await?;

    if !explicit_opt_in {
        let mut blockers = vec!["SOCKS UDP fragment execution requires explicit opt-in".into()];
        blockers.extend(default_data_plane_closeout_gate.blockers.clone());
        return Ok(blocked_report(
            explicit_opt_in,
            default_data_plane_closeout_gate,
            blockers,
        ));
    }
    if !default_data_plane_closeout_gate.blockers.is_empty() {
        return Ok(blocked_report(
            explicit_opt_in,
            default_data_plane_closeout_gate.clone(),
            default_data_plane_closeout_gate.blockers.clone(),
        ));
    }

    let rollback_path = rust_socks_udp_fragments_rollback_path()?;
    let rollback_evidence = write_rollback_checkpoint(&rollback_path).await?;
    let packet_evidence = match run_bounded_socks_udp_fragment_reassembly() {
        Ok(evidence) => evidence,
        Err(error) => {
            return Ok(blocked_report(
                explicit_opt_in,
                default_data_plane_closeout_gate,
                vec![format!("bounded SOCKS UDP fragment execution failed: {error}").into()],
            ));
        }
    };
    let leak_evidence = RustSocksUdpFragmentsLeakEvidence {
        passed: packet_evidence.loopback_only
            && packet_evidence.fragments_reassembled
            && packet_evidence.datagram_round_trip,
        no_system_packet_capture: true,
        no_non_loopback_target: packet_evidence.loopback_only,
        no_mihomo_binary_removal: true,
    };
    let evidence_path = rust_socks_udp_fragments_evidence_path()?;
    let mut report = RustSocksUdpFragmentsExecutionReport {
        runtime_id: RUST_RUNTIME_ID.into(),
        component: RUST_SOCKS_UDP_FRAGMENTS_COMPONENT.into(),
        kernel_area: RUST_SOCKS_UDP_FRAGMENTS_KERNEL_AREA.into(),
        status: RustSocksUdpFragmentsExecutionStatus::Executed,
        reason: "Rust executed bounded SOCKS5 UDP fragment reassembly over loopback".into(),
        explicit_opt_in,
        rust_owned_scope: RUST_SOCKS_UDP_FRAGMENTS_OWNED_SCOPE.into(),
        default_data_plane_closeout_gate,
        mutates_runtime: false,
        writes_evidence: true,
        evidence_path: Some(evidence_path.to_string_lossy().to_string().into()),
        packet_evidence: Some(packet_evidence),
        rollback_evidence: Some(rollback_evidence),
        leak_evidence: Some(leak_evidence),
        mihomo_fallback_retained_for: retained_socks_udp_fragments_fallback_scope(),
        blockers: Vec::new(),
        warnings: vec![
            "SOCKS UDP non-loopback forwarding, fragment windows/timeouts, encrypted protocols, and packet capture remain Mihomo-owned".into(),
        ],
        facts: rust_socks_udp_fragments_facts(),
        next_safe_batch: NEXT_SAFE_BATCH.into(),
    };

    if let Some(parent) = evidence_path.parent() {
        fs::create_dir_all(parent).await?;
    }
    fs::write(&evidence_path, serde_yaml_ng::to_string(&report)?.as_bytes()).await?;
    report.evidence_path = Some(evidence_path.to_string_lossy().to_string().into());

    Ok(report)
}

fn blocked_report(
    explicit_opt_in: bool,
    default_data_plane_closeout_gate: RustDefaultDataPlaneCloseoutGateEvidence,
    blockers: Vec<String>,
) -> RustSocksUdpFragmentsExecutionReport {
    RustSocksUdpFragmentsExecutionReport {
        runtime_id: RUST_RUNTIME_ID.into(),
        component: RUST_SOCKS_UDP_FRAGMENTS_COMPONENT.into(),
        kernel_area: RUST_SOCKS_UDP_FRAGMENTS_KERNEL_AREA.into(),
        status: RustSocksUdpFragmentsExecutionStatus::Blocked,
        reason: "Rust SOCKS UDP fragment execution is blocked".into(),
        explicit_opt_in,
        rust_owned_scope: RUST_SOCKS_UDP_FRAGMENTS_OWNED_SCOPE.into(),
        default_data_plane_closeout_gate,
        mutates_runtime: false,
        writes_evidence: false,
        evidence_path: None,
        packet_evidence: None,
        rollback_evidence: None,
        leak_evidence: None,
        mihomo_fallback_retained_for: retained_socks_udp_fragments_fallback_scope(),
        blockers,
        warnings: Vec::new(),
        facts: rust_socks_udp_fragments_facts(),
        next_safe_batch: NEXT_SAFE_BATCH.into(),
    }
}
