use super::{
    MihomoFallbackRetirementEmergencyCheckpoint, MihomoFallbackRetirementExecutionReport,
    MihomoFallbackRetirementExecutionScope, MihomoFallbackRetirementExecutionStatus, RUST_RUNTIME_ID,
    rust_encrypted_proxy_protocol_preflight_evidence, rust_encrypted_proxy_session_expansion,
    rust_http_connect_proxy_adapter_evidence, rust_protocol_adapter_forwarding_expansion_evidence,
    rust_remote_adapter_transport_expansion_evidence, rust_runtime_real_canary_evidence,
    rust_shadowsocks_aead_adapter_canary, rust_tun_transparent_routing_execution,
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
const NEXT_SAFE_BATCH: &str = "rust-default-data-plane-closeout";

pub async fn mihomo_fallback_retirement_execution_plan() -> Result<MihomoFallbackRetirementExecutionReport> {
    build_mihomo_fallback_retirement_execution_report(false, false).await
}

pub async fn execute_mihomo_fallback_retirement(
    explicit_opt_in: bool,
    run_canary: bool,
) -> Result<MihomoFallbackRetirementExecutionReport> {
    if run_canary {
        run_wider_fallback_retirement_evidence(explicit_opt_in).await?;
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
    report.reason =
        "Mihomo fallback retired for the wider bounded Rust evidence scope; unsupported fallback retained".into();
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

async fn run_wider_fallback_retirement_evidence(explicit_opt_in: bool) -> Result<()> {
    let _ = rust_runtime_real_canary_evidence(None, explicit_opt_in).await?;
    let _ = rust_protocol_adapter_forwarding_expansion_evidence(explicit_opt_in).await?;
    let _ = rust_remote_adapter_transport_expansion_evidence(explicit_opt_in).await?;
    let _ = rust_http_connect_proxy_adapter_evidence(explicit_opt_in).await?;
    let _ = rust_encrypted_proxy_protocol_preflight_evidence(explicit_opt_in).await?;
    let _ = rust_shadowsocks_aead_adapter_canary(explicit_opt_in).await?;
    let _ = rust_encrypted_proxy_session_expansion(explicit_opt_in).await?;
    let _ = rust_tun_transparent_routing_execution(explicit_opt_in).await?;
    Ok(())
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
    for (component, label) in wider_fallback_retirement_evidence_components() {
        let evidence_path = fallback_retirement_evidence_path(component)?;
        let evidence = fs::read_to_string(&evidence_path).await.ok();
        let evidence_yaml = evidence
            .as_deref()
            .and_then(|yaml| serde_yaml_ng::from_str::<Value>(yaml).ok());
        if evidence_yaml.is_none() {
            blockers.push(format!("{label} evidence artifact is missing").into());
            continue;
        }
        if let Some(value) = evidence_yaml.as_ref() {
            let status = value.get("status").and_then(Value::as_str).unwrap_or_default();
            if status != "passed" {
                blockers.push(format!("{label} evidence status is {status}").into());
            }
            if value
                .get("blockers")
                .and_then(Value::as_sequence)
                .map(|blockers| !blockers.is_empty())
                .unwrap_or(false)
            {
                blockers.push(format!("{label} evidence contains blockers").into());
            }
            if value
                .get("mihomoFallback")
                .and_then(Value::as_bool)
                .map(|mihomo_fallback| !mihomo_fallback)
                .unwrap_or(false)
            {
                blockers.push(format!("{label} evidence did not retain unsupported fallback").into());
            }
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
            "wider-scope Mihomo fallback retirement execution is ready".into()
        } else {
            "wider-scope Mihomo fallback retirement execution is blocked".into()
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
            "execution scope is limited to bounded Rust DNS, adapter, protocol, encrypted-session, and transparent packet evidence".into(),
            "Mihomo remains fallback for SOCKS non-loopback UDP plus fragment queues/timeouts, unsupported encrypted protocols, packet capture, route install, and full binary removal".into(),
        ],
        facts: vec![
            "execution can run the wider evidence suite before writing a durable manifest and emergency rollback checkpoint".into(),
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
        MihomoFallbackRetirementExecutionScope {
            scope: "direct-reject-adapter-forwarding".into(),
            rust_owned_path: "Rust DIRECT/REJECT adapter execution with target/reject evidence".into(),
            fallback_retired_for_scope: true,
            mihomo_fallback_retained_for: retained_fallback_scope(),
            evidence: vec!["rust-protocol-adapter-forwarding-expansion/evidence.yaml".into()],
        },
        MihomoFallbackRetirementExecutionScope {
            scope: "bounded-remote-tcp-connect-adapter".into(),
            rust_owned_path: "Rust TCP CONNECT-style remote adapter transport evidence".into(),
            fallback_retired_for_scope: true,
            mihomo_fallback_retained_for: retained_fallback_scope(),
            evidence: vec!["rust-remote-adapter-transport-expansion/evidence.yaml".into()],
        },
        MihomoFallbackRetirementExecutionScope {
            scope: "http-connect-proxy-adapter".into(),
            rust_owned_path: "Rust HTTP CONNECT adapter tunnel with bidirectional byte evidence".into(),
            fallback_retired_for_scope: true,
            mihomo_fallback_retained_for: retained_fallback_scope(),
            evidence: vec!["rust-http-connect-proxy-adapter/evidence.yaml".into()],
        },
        MihomoFallbackRetirementExecutionScope {
            scope: "shadowsocks-aead-tcp-session".into(),
            rust_owned_path: "Rust Shadowsocks AEAD TCP address frame, canary, and multi-chunk session evidence".into(),
            fallback_retired_for_scope: true,
            mihomo_fallback_retained_for: retained_fallback_scope(),
            evidence: vec![
                "rust-shadowsocks-aead-adapter-canary/evidence.yaml".into(),
                "rust-encrypted-proxy-session-expansion/evidence.yaml".into(),
            ],
        },
        MihomoFallbackRetirementExecutionScope {
            scope: "bounded-transparent-ipv4-tcp-route".into(),
            rust_owned_path:
                "Rust synthetic IPv4/TCP packet parse, destination extraction, and loopback execution evidence".into(),
            fallback_retired_for_scope: true,
            mihomo_fallback_retained_for: retained_fallback_scope(),
            evidence: vec!["rust-tun-transparent-routing-execution/evidence.yaml".into()],
        },
    ]
}

fn retained_fallback_scope() -> Vec<String> {
    vec![
        "SOCKS non-loopback UDP and fragment queues/timeouts".into(),
        "remote adapter protocols beyond bounded TCP CONNECT".into(),
        "system-wide TUN packet capture".into(),
        "OS route install and transparent proxy defaults".into(),
        "VMess, VLESS, Trojan TLS, Shadowsocks UDP associate, and plugin transports".into(),
        "unsupported DNS features beyond bounded fake-ip/fallback-filter/nameserver-policy execution: fake-ip persistent cache lifecycle/eviction, fake-ip wildcard filters, fallback-filter full GeoIP database/upstream execution, nameserver-policy geosite/wildcard/upstream execution".into(),
        "full Mihomo fallback binary removal".into(),
    ]
}

fn wider_fallback_retirement_evidence_components() -> Vec<(&'static str, &'static str)> {
    vec![
        (
            "rust-protocol-adapter-forwarding-expansion",
            "protocol adapter forwarding",
        ),
        ("rust-remote-adapter-transport-expansion", "remote adapter transport"),
        ("rust-http-connect-proxy-adapter", "HTTP CONNECT adapter"),
        (
            "rust-encrypted-proxy-protocol-preflight",
            "encrypted proxy protocol preflight",
        ),
        (
            "rust-shadowsocks-aead-adapter-canary",
            "Shadowsocks AEAD adapter canary",
        ),
        (
            "rust-encrypted-proxy-session-expansion",
            "encrypted proxy session expansion",
        ),
        (
            "rust-tun-transparent-routing-execution",
            "TUN transparent routing execution",
        ),
    ]
}

fn rust_runtime_real_canary_evidence_path() -> Result<std::path::PathBuf> {
    Ok(dirs::app_runtime_dir()?
        .join(RUST_RUNTIME_REAL_CANARY_COMPONENT)
        .join(RUST_RUNTIME_REAL_CANARY_EVIDENCE_FILE))
}

fn fallback_retirement_evidence_path(component: &str) -> Result<std::path::PathBuf> {
    Ok(dirs::app_runtime_dir()?.join(component).join("evidence.yaml"))
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
