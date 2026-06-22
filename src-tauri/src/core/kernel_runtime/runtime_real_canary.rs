use super::{
    KernelLoopbackDnsSmokeEvidenceReport, RUST_RUNTIME_ID, RustRuntimeRealCanaryEvidenceReport,
    RustRuntimeRealCanaryStatus, RustTunSystemProxyParityPreflightReport, mihomo_kernel_loopback_dns_smoke_evidence,
    rust_fallback_retirement_readiness_manifest, rust_protocol_forwarding_subset_smoke_evidence,
    rust_tun_system_proxy_parity_preflight,
};
use crate::utils::dirs;
use anyhow::Result;
use smartstring::alias::String;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::fs;

const RUST_RUNTIME_REAL_CANARY_COMPONENT: &str = "rust-runtime-real-canary";
const RUST_RUNTIME_REAL_CANARY_KERNEL_AREA: &str = "runtime-canary";
const RUST_RUNTIME_REAL_CANARY_PROFILE: &str = "loopback-dns-forwarding-route-canary";
const RUST_RUNTIME_REAL_CANARY_EVIDENCE_FILE: &str = "evidence.yaml";
const RUST_RUNTIME_REAL_CANARY_NEXT_BATCH: &str = "mihomo-fallback-retirement-execution";

pub async fn rust_runtime_real_canary_evidence(
    canary_profile: Option<String>,
    explicit_opt_in: bool,
) -> Result<RustRuntimeRealCanaryEvidenceReport> {
    let canary_profile = canary_profile
        .as_deref()
        .map(str::trim)
        .filter(|profile| !profile.is_empty())
        .unwrap_or(RUST_RUNTIME_REAL_CANARY_PROFILE)
        .into();
    let started_at_epoch_seconds = rust_runtime_real_canary_epoch_seconds();

    if !explicit_opt_in {
        return Ok(RustRuntimeRealCanaryEvidenceReport {
            runtime_id: RUST_RUNTIME_ID.into(),
            component: RUST_RUNTIME_REAL_CANARY_COMPONENT.into(),
            kernel_area: RUST_RUNTIME_REAL_CANARY_KERNEL_AREA.into(),
            status: RustRuntimeRealCanaryStatus::Blocked,
            reason: "explicit opt-in is required to run the Rust runtime real canary".into(),
            canary_profile,
            started_at_epoch_seconds,
            explicit_opt_in,
            dns_smoke_evidence: None,
            protocol_forwarding_evidence: None,
            tun_system_proxy_preflight: None,
            fallback_readiness_manifest: None,
            evidence_path: None,
            mutates_runtime: false,
            writes_evidence_artifact: false,
            removes_mihomo_fallback: false,
            mihomo_fallback: true,
            blockers: vec!["explicit opt-in is required".into()],
            warnings: Vec::new(),
            facts: rust_runtime_real_canary_facts(),
            next_safe_batch: RUST_RUNTIME_REAL_CANARY_NEXT_BATCH.into(),
        });
    }

    let dns_smoke_evidence = mihomo_kernel_loopback_dns_smoke_evidence(None).await?;
    let protocol_forwarding_evidence = rust_protocol_forwarding_subset_smoke_evidence(None, None).await?;
    let tun_system_proxy_preflight = rust_tun_system_proxy_parity_preflight(Some("off".into())).await?;
    let fallback_readiness_manifest = rust_fallback_retirement_readiness_manifest().await?;
    let blockers = rust_runtime_real_canary_blockers(
        &dns_smoke_evidence,
        &protocol_forwarding_evidence,
        &tun_system_proxy_preflight,
    );
    let status = if blockers.is_empty() {
        RustRuntimeRealCanaryStatus::Passed
    } else {
        RustRuntimeRealCanaryStatus::Failed
    };

    let mut report = RustRuntimeRealCanaryEvidenceReport {
        runtime_id: RUST_RUNTIME_ID.into(),
        component: RUST_RUNTIME_REAL_CANARY_COMPONENT.into(),
        kernel_area: RUST_RUNTIME_REAL_CANARY_KERNEL_AREA.into(),
        status,
        reason: if status == RustRuntimeRealCanaryStatus::Passed {
            "Rust runtime real canary evidence passed for the bounded loopback profile".into()
        } else {
            "Rust runtime real canary evidence failed".into()
        },
        canary_profile,
        started_at_epoch_seconds,
        explicit_opt_in,
        dns_smoke_evidence: Some(dns_smoke_evidence),
        protocol_forwarding_evidence: Some(protocol_forwarding_evidence),
        tun_system_proxy_preflight: Some(tun_system_proxy_preflight),
        fallback_readiness_manifest: Some(fallback_readiness_manifest),
        evidence_path: None,
        mutates_runtime: false,
        writes_evidence_artifact: true,
        removes_mihomo_fallback: false,
        mihomo_fallback: true,
        blockers,
        warnings: vec![
            "canary uses loopback DNS and Rust protocol forwarding; it does not remove Mihomo fallback".into(),
            "fallback retirement execution remains unsupported for SOCKS, remote adapters, and packet capture".into(),
        ],
        facts: rust_runtime_real_canary_facts(),
        next_safe_batch: RUST_RUNTIME_REAL_CANARY_NEXT_BATCH.into(),
    };

    let evidence_path = rust_runtime_real_canary_evidence_path()?;
    if let Some(parent) = evidence_path.parent() {
        fs::create_dir_all(parent).await?;
    }
    report.evidence_path = Some(evidence_path.to_string_lossy().to_string().into());
    fs::write(&evidence_path, serde_yaml_ng::to_string(&report)?.as_bytes()).await?;
    Ok(report)
}

fn rust_runtime_real_canary_blockers(
    dns: &KernelLoopbackDnsSmokeEvidenceReport,
    protocol: &super::RustProtocolForwardingSubsetSmokeEvidenceReport,
    tun: &RustTunSystemProxyParityPreflightReport,
) -> Vec<String> {
    let mut blockers = Vec::new();
    if !dns.passed {
        blockers.push("loopback DNS canary smoke evidence failed".into());
        blockers.extend(dns.blockers.iter().cloned());
    }
    if !protocol.passed {
        blockers.push("Rust protocol forwarding canary smoke evidence failed".into());
        blockers.extend(protocol.blockers.iter().cloned());
    }
    if !tun.blockers.is_empty() {
        blockers.push("TUN/system-proxy route canary preflight is blocked".into());
        blockers.extend(tun.blockers.iter().cloned());
    }
    blockers
}

fn rust_runtime_real_canary_evidence_path() -> Result<std::path::PathBuf> {
    Ok(dirs::app_runtime_dir()?
        .join(RUST_RUNTIME_REAL_CANARY_COMPONENT)
        .join(RUST_RUNTIME_REAL_CANARY_EVIDENCE_FILE))
}

fn rust_runtime_real_canary_facts() -> Vec<String> {
    vec![
        "canary exercises real loopback UDP DNS request/response handling".into(),
        "canary exercises the Rust TCP accept loop and bidirectional byte forwarding".into(),
        "canary verifies route-mode preflight without changing the default route".into(),
        "canary writes a durable evidence artifact for fallback-retirement review".into(),
    ]
}

fn rust_runtime_real_canary_epoch_seconds() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or_default()
}
