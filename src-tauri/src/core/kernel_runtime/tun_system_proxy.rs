use super::{
    RUST_RUNTIME_ID, RustTunSystemProxyMode, RustTunSystemProxyParityApplyReport,
    RustTunSystemProxyParityPreflightReport, RustTunSystemProxyParityRollbackReport, RustTunSystemProxyParityStatus,
    RustTunSystemProxyRoutePatch, RustTunSystemProxyRouteSnapshot,
};
use crate::{
    app::config::patch_verge,
    config::{Config, IVerge},
    core::sysopt::Sysopt,
    utils::dirs,
};
use anyhow::{Result, bail};
use serde::{Deserialize, Serialize};
use smartstring::alias::String;
use sysproxy::Sysproxy;
use tokio::fs;

const RUST_TUN_SYSTEM_PROXY_COMPONENT: &str = "rust-tun-system-proxy-parity";
const RUST_TUN_SYSTEM_PROXY_KERNEL_AREA: &str = "tun-system-proxy";
const RUST_TUN_SYSTEM_PROXY_NEXT_BATCH: &str = "rust-fallback-retirement-readiness";
const RUST_TUN_SYSTEM_PROXY_ROLLBACK_FILE: &str = "rollback.yaml";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RustTunSystemProxyRollbackRecord {
    requested_mode: RustTunSystemProxyMode,
    previous_snapshot: RustTunSystemProxyRouteSnapshot,
    applied_patch: RustTunSystemProxyRoutePatch,
}

pub async fn rust_tun_system_proxy_parity_preflight(
    requested_mode: Option<String>,
) -> Result<RustTunSystemProxyParityPreflightReport> {
    let requested_mode = parse_tun_system_proxy_mode(requested_mode.as_deref())?;
    let current_snapshot = rust_tun_system_proxy_snapshot().await;
    let route_patch = rust_tun_system_proxy_route_patch(requested_mode);
    let system_proxy_os_apply = current_snapshot.enable_system_proxy != route_patch.enable_system_proxy;
    let tun_runtime_apply = current_snapshot.enable_tun_mode != route_patch.enable_tun_mode;
    let mut blockers = Vec::new();
    let mut warnings = Vec::new();

    if requested_mode == RustTunSystemProxyMode::Tun {
        warnings.push("TUN parity still uses the existing Mihomo/service TUN backend for packet capture".into());
        warnings.push("Rust owns the route-mode decision, patch, lifecycle record, and rollback boundary".into());
    }
    if requested_mode == RustTunSystemProxyMode::SystemProxy && current_snapshot.proxy_auto_config {
        warnings.push("system proxy mode will preserve the existing PAC preference".into());
    }
    if requested_mode != RustTunSystemProxyMode::Off
        && current_snapshot.mixed_port == 0
        && requested_mode == RustTunSystemProxyMode::SystemProxy
    {
        blockers.push("system proxy mode requires a non-zero mixed-port".into());
    }

    let status = if blockers.is_empty() {
        RustTunSystemProxyParityStatus::Ready
    } else {
        RustTunSystemProxyParityStatus::Blocked
    };

    Ok(RustTunSystemProxyParityPreflightReport {
        runtime_id: RUST_RUNTIME_ID.into(),
        component: RUST_TUN_SYSTEM_PROXY_COMPONENT.into(),
        kernel_area: RUST_TUN_SYSTEM_PROXY_KERNEL_AREA.into(),
        status,
        reason: if status == RustTunSystemProxyParityStatus::Ready {
            "Rust TUN/system proxy route patch is ready for explicit opt-in apply".into()
        } else {
            "Rust TUN/system proxy route patch is blocked".into()
        },
        requested_mode,
        current_snapshot,
        route_patch,
        explicit_opt_in_required: true,
        mutates_runtime: false,
        reload_mihomo: tun_runtime_apply,
        system_proxy_os_apply,
        tun_runtime_apply,
        mihomo_fallback: true,
        rollback_supported: true,
        blockers,
        warnings,
        facts: rust_tun_system_proxy_facts(),
        next_safe_batch: RUST_TUN_SYSTEM_PROXY_NEXT_BATCH.into(),
    })
}

pub async fn apply_rust_tun_system_proxy_parity(
    requested_mode: Option<String>,
    explicit_opt_in: bool,
) -> Result<RustTunSystemProxyParityApplyReport> {
    let preflight = rust_tun_system_proxy_parity_preflight(requested_mode).await?;
    let mut blockers = preflight.blockers.clone();
    if !explicit_opt_in {
        blockers.push("explicit opt-in is required to apply Rust TUN/system proxy parity".into());
    }
    let previous_snapshot = preflight.current_snapshot.clone();
    if !blockers.is_empty() {
        return Ok(RustTunSystemProxyParityApplyReport {
            status: RustTunSystemProxyParityStatus::Blocked,
            reason: "Rust TUN/system proxy parity apply is blocked".into(),
            requested_mode: preflight.requested_mode,
            applied_snapshot: previous_snapshot.clone(),
            previous_snapshot,
            rollback_record_path: None,
            explicit_opt_in,
            mutates_runtime: false,
            reload_mihomo: false,
            system_proxy_os_apply: false,
            tun_runtime_apply: false,
            mihomo_fallback: true,
            blockers,
            warnings: preflight.warnings.clone(),
            facts: rust_tun_system_proxy_facts(),
            preflight,
        });
    }

    let rollback_record_path = rust_tun_system_proxy_rollback_record_path()?;
    if let Some(parent) = rollback_record_path.parent() {
        fs::create_dir_all(parent).await?;
    }
    let rollback_record = RustTunSystemProxyRollbackRecord {
        requested_mode: preflight.requested_mode,
        previous_snapshot: previous_snapshot.clone(),
        applied_patch: preflight.route_patch.clone(),
    };
    fs::write(
        &rollback_record_path,
        serde_yaml_ng::to_string(&rollback_record)?.as_bytes(),
    )
    .await?;

    patch_verge(
        &IVerge {
            enable_system_proxy: Some(preflight.route_patch.enable_system_proxy),
            enable_tun_mode: Some(preflight.route_patch.enable_tun_mode),
            ..IVerge::default()
        },
        false,
    )
    .await?;

    let applied_snapshot = rust_tun_system_proxy_snapshot().await;
    Ok(RustTunSystemProxyParityApplyReport {
        status: RustTunSystemProxyParityStatus::Applied,
        reason: "Rust TUN/system proxy route patch applied".into(),
        requested_mode: preflight.requested_mode,
        previous_snapshot,
        applied_snapshot,
        rollback_record_path: Some(rollback_record_path.to_string_lossy().to_string().into()),
        explicit_opt_in,
        mutates_runtime: true,
        reload_mihomo: preflight.tun_runtime_apply,
        system_proxy_os_apply: preflight.system_proxy_os_apply,
        tun_runtime_apply: preflight.tun_runtime_apply,
        mihomo_fallback: true,
        blockers: Vec::new(),
        warnings: preflight.warnings.clone(),
        facts: rust_tun_system_proxy_facts(),
        preflight,
    })
}

pub async fn rollback_rust_tun_system_proxy_parity() -> Result<RustTunSystemProxyParityRollbackReport> {
    let rollback_record_path = rust_tun_system_proxy_rollback_record_path()?;
    let record_yaml = fs::read_to_string(&rollback_record_path).await?;
    let record: RustTunSystemProxyRollbackRecord = serde_yaml_ng::from_str(&record_yaml)?;
    let current_snapshot = rust_tun_system_proxy_snapshot().await;
    let system_proxy_os_apply = current_snapshot.enable_system_proxy != record.previous_snapshot.enable_system_proxy;
    let tun_runtime_apply = current_snapshot.enable_tun_mode != record.previous_snapshot.enable_tun_mode;

    patch_verge(
        &IVerge {
            enable_system_proxy: Some(record.previous_snapshot.enable_system_proxy),
            enable_tun_mode: Some(record.previous_snapshot.enable_tun_mode),
            ..IVerge::default()
        },
        false,
    )
    .await?;
    let restored_snapshot = rust_tun_system_proxy_snapshot().await;

    Ok(RustTunSystemProxyParityRollbackReport {
        status: RustTunSystemProxyParityStatus::Restored,
        reason: "Rust TUN/system proxy route patch rolled back".into(),
        restored_snapshot,
        rollback_record_path: Some(rollback_record_path.to_string_lossy().to_string().into()),
        mutates_runtime: true,
        reload_mihomo: tun_runtime_apply,
        system_proxy_os_apply,
        tun_runtime_apply,
        mihomo_fallback: true,
        blockers: Vec::new(),
        warnings: vec![
            "rollback restores route-mode booleans; proxy host, bypass, and PAC preferences remain unchanged".into(),
        ],
        facts: rust_tun_system_proxy_facts(),
    })
}

fn parse_tun_system_proxy_mode(value: Option<&str>) -> Result<RustTunSystemProxyMode> {
    match value.unwrap_or("off").trim() {
        "off" | "disabled" => Ok(RustTunSystemProxyMode::Off),
        "systemProxy" | "system-proxy" | "proxy" => Ok(RustTunSystemProxyMode::SystemProxy),
        "tun" | "tunMode" | "tun-mode" => Ok(RustTunSystemProxyMode::Tun),
        value => bail!("unsupported Rust TUN/system proxy mode `{value}`"),
    }
}

fn rust_tun_system_proxy_route_patch(mode: RustTunSystemProxyMode) -> RustTunSystemProxyRoutePatch {
    match mode {
        RustTunSystemProxyMode::Off => RustTunSystemProxyRoutePatch {
            enable_system_proxy: false,
            enable_tun_mode: false,
        },
        RustTunSystemProxyMode::SystemProxy => RustTunSystemProxyRoutePatch {
            enable_system_proxy: true,
            enable_tun_mode: false,
        },
        RustTunSystemProxyMode::Tun => RustTunSystemProxyRoutePatch {
            enable_system_proxy: false,
            enable_tun_mode: true,
        },
    }
}

async fn rust_tun_system_proxy_snapshot() -> RustTunSystemProxyRouteSnapshot {
    let verge = Config::verge().await.latest_arc();
    let clash = Config::clash().await.latest_arc();
    let mixed_port = verge.verge_mixed_port.unwrap_or_else(|| clash.get_mixed_port());
    let clash_tun_enabled = clash
        .0
        .get("tun")
        .and_then(serde_yaml_ng::Value::as_mapping)
        .and_then(|tun| tun.get("enable"))
        .and_then(serde_yaml_ng::Value::as_bool);
    let (os_system_proxy_enabled, os_system_proxy_server) = rust_tun_system_proxy_os_snapshot().await;

    RustTunSystemProxyRouteSnapshot {
        enable_system_proxy: verge.enable_system_proxy.unwrap_or(false),
        enable_tun_mode: verge.enable_tun_mode.unwrap_or(false),
        proxy_auto_config: verge.proxy_auto_config.unwrap_or(false),
        proxy_host: verge.proxy_host.clone(),
        mixed_port,
        system_proxy_bypass: verge.system_proxy_bypass.clone(),
        use_default_bypass: verge.use_default_bypass.unwrap_or(true),
        os_system_proxy_enabled,
        os_system_proxy_server,
        clash_tun_enabled,
    }
}

async fn rust_tun_system_proxy_os_snapshot() -> (Option<bool>, Option<String>) {
    Sysopt::global().wait_idle().await;
    tokio::task::spawn_blocking(|| {
        Sysproxy::get_system_proxy()
            .map(|proxy| {
                (
                    Some(proxy.enable),
                    Some(format!("{}:{}", proxy.host, proxy.port).into()),
                )
            })
            .unwrap_or((None, None))
    })
    .await
    .unwrap_or((None, None))
}

fn rust_tun_system_proxy_rollback_record_path() -> Result<std::path::PathBuf> {
    Ok(dirs::app_runtime_dir()?
        .join("rust-tun-system-proxy-parity")
        .join(RUST_TUN_SYSTEM_PROXY_ROLLBACK_FILE))
}

fn rust_tun_system_proxy_facts() -> Vec<String> {
    vec![
        "Rust owns the route-mode decision for off/system-proxy/TUN".into(),
        "system proxy apply uses the Rust Sysopt/sysproxy path".into(),
        "TUN apply uses Rust config generation and the existing Mihomo/service TUN backend".into(),
        "mode apply writes a rollback record before mutating route state".into(),
        "Mihomo remains fallback for packet capture and transparent forwarding until later parity".into(),
    ]
}
