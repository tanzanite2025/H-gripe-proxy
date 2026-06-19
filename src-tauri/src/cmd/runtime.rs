use super::CmdResult;
use crate::{
    cmd::StringifyErr as _,
    config::Config,
    core::{
        CoreManager,
        current_egress_identity::{CurrentEgressIdentity, build_current_egress_identity},
        handle::Handle,
        kernel_runtime::{
            KernelReplacementReadiness, KernelRuntimePreflightReport, mihomo_kernel_apply_preflight,
            mihomo_kernel_replacement_readiness,
        },
        runtime_diagnostics::{
            build_dns_leak_test_result, build_dns_runtime_status, build_proxy_detection_result,
            build_runtime_diagnostics_summary,
        },
        runtime_snapshot::RuntimeSnapshotService,
        runtime_status::{DnsLeakTestResult, DnsRuntimeStatus, ProxyDetectionResult},
    },
};
use anyhow::{Context as _, anyhow};
use clash_verge_logging::{Type, logging};
use serde_yaml_ng::Mapping;
use smartstring::alias::String;
use std::collections::{HashMap, HashSet};
use tauri_plugin_mihomo::models::{
    BaseConfig, BufferPoolStats, CoreUpdaterChannel, DnsMetrics, EngineStats, HotReloadStatus, MihomoVersion,
    PerfStats, Proxies, ProxyDelay, ProxyProviders, RuleProviders, RuleTrafficSnapshot, Rules, TLSFingerprintStats,
    TLSRotationResult, XDPStatus,
};
// Diagnostic builders have been moved into core::runtime_diagnostics; this command module keeps only thin wrappers.

#[tauri::command]
pub async fn get_runtime_kernel_replacement_readiness() -> CmdResult<KernelReplacementReadiness> {
    Ok(mihomo_kernel_replacement_readiness().await)
}

#[tauri::command]
pub async fn get_runtime_kernel_apply_preflight(
    artifact_id: Option<String>,
) -> CmdResult<KernelRuntimePreflightReport> {
    Ok(mihomo_kernel_apply_preflight(artifact_id).await)
}

#[tauri::command]
pub async fn get_runtime_kernel_shadow_components() -> CmdResult<KernelShadowComponentsReport> {
    Ok(mihomo_kernel_shadow_components().await)
}

#[tauri::command]
pub async fn get_runtime_kernel_dns_shadow_evidence(
    yaml: Option<String>,
    domain: Option<String>,
) -> CmdResult<KernelDnsShadowEvidenceReport> {
    mihomo_kernel_dns_shadow_evidence(yaml, domain).await.stringify_err()
}

#[tauri::command]
pub async fn get_runtime_kernel_rule_shadow_evidence() -> CmdResult<KernelRuleShadowEvidenceReport> {
    mihomo_kernel_rule_shadow_evidence().await.stringify_err()
}

#[tauri::command]
pub async fn get_runtime_kernel_adapter_capability_report() -> CmdResult<KernelAdapterCapabilityReport> {
    mihomo_kernel_adapter_capability_report().await.stringify_err()
}

/// 获取运行时配置
#[tauri::command]
pub async fn get_runtime_config() -> CmdResult<Option<Mapping>> {
    Ok(Config::runtime().await.latest_arc().config.clone())
}

/// 获取运行时YAML配置
#[tauri::command]
pub async fn get_runtime_yaml() -> CmdResult<String> {
    let runtime = Config::runtime().await;
    let runtime = runtime.latest_arc();

    let config = runtime.config.as_ref();
    config
        .ok_or_else(|| anyhow!("failed to parse config to yaml file"))
        .and_then(|config| {
            serde_yaml_ng::to_string(config)
                .context("failed to convert config to yaml")
                .map(|s| s.into())
        })
        .stringify_err()
}

#[tauri::command]
pub async fn get_runtime_version() -> CmdResult<MihomoVersion> {
    Handle::mihomo().await.get_version().await.stringify_err()
}

#[tauri::command]
pub async fn get_runtime_base_config() -> CmdResult<BaseConfig> {
    Handle::mihomo().await.get_base_config().await.stringify_err()
}

#[tauri::command]
pub async fn patch_runtime_base_config(data: serde_json::Value) -> CmdResult<()> {
    Handle::mihomo().await.patch_base_config(&data).await.stringify_err()
}

#[tauri::command]
pub async fn update_runtime_geo() -> CmdResult<()> {
    match Handle::mihomo().await.update_geo().await {
        Ok(()) => {
            crate::core::runtime_snapshot::record_and_persist_runtime_lifecycle_event(
                LIFECYCLE_UPDATE_GEO,
                true,
                None,
                None,
            );
            Ok(())
        }
        Err(error) => {
            crate::core::runtime_snapshot::record_and_persist_runtime_lifecycle_event(
                LIFECYCLE_UPDATE_GEO,
                false,
                Some(error.to_string()),
                None,
            );
            Err(error).stringify_err()
        }
    }
}

#[tauri::command]
pub async fn get_runtime_dns_metrics() -> CmdResult<DnsMetrics> {
    Handle::mihomo().await.get_dns_metrics().await.stringify_err()
}

#[tauri::command]
pub async fn runtime_dns_warmup() -> CmdResult<()> {
    Handle::mihomo().await.dns_warmup().await.stringify_err()
}

#[tauri::command]
pub async fn get_runtime_engine_stats() -> CmdResult<EngineStats> {
    Handle::mihomo().await.get_engine_stats().await.stringify_err()
}

#[tauri::command]
pub async fn get_runtime_perf_stats() -> CmdResult<PerfStats> {
    Handle::mihomo().await.get_perf_stats().await.stringify_err()
}

#[tauri::command]
pub async fn get_runtime_buffer_pool_stats() -> CmdResult<BufferPoolStats> {
    Handle::mihomo().await.get_buffer_pool_stats().await.stringify_err()
}

#[tauri::command]
pub async fn get_runtime_hot_reload_status() -> CmdResult<HotReloadStatus> {
    Handle::mihomo().await.get_hot_reload_status().await.stringify_err()
}

#[tauri::command]
pub async fn get_runtime_xdp_status() -> CmdResult<XDPStatus> {
    Handle::mihomo().await.get_xdp_status().await.stringify_err()
}

#[tauri::command]
pub async fn get_runtime_rule_traffic() -> CmdResult<HashMap<std::string::String, RuleTrafficSnapshot>> {
    Handle::mihomo().await.get_rule_traffic().await.stringify_err()
}

#[tauri::command]
pub async fn get_runtime_tls_fingerprint_stats() -> CmdResult<TLSFingerprintStats> {
    Handle::mihomo().await.get_tls_fingerprint_stats().await.stringify_err()
}

/// 强制轮换 TLS 指纹（app-owned 门禁，记录生命周期审计）
#[tauri::command]
pub async fn force_runtime_tls_rotation() -> CmdResult<TLSRotationResult> {
    match Handle::mihomo().await.force_tls_rotation().await {
        Ok(result) => {
            crate::core::runtime_snapshot::record_and_persist_runtime_lifecycle_event(
                LIFECYCLE_TLS_ROTATION,
                true,
                None,
                Some(result.new_fingerprint.clone()),
            );
            Ok(result)
        }
        Err(error) => {
            crate::core::runtime_snapshot::record_and_persist_runtime_lifecycle_event(
                LIFECYCLE_TLS_ROTATION,
                false,
                Some(error.to_string()),
                None,
            );
            Err(error).stringify_err()
        }
    }
}

#[tauri::command]
pub async fn get_dns_runtime_status() -> CmdResult<DnsRuntimeStatus> {
    build_dns_runtime_status().await.stringify_err()
}

#[tauri::command]
pub async fn test_dns_leak() -> CmdResult<DnsLeakTestResult> {
    build_dns_leak_test_result().await.stringify_err()
}

#[tauri::command]
pub async fn test_proxy_detection() -> CmdResult<ProxyDetectionResult> {
    build_proxy_detection_result().await.stringify_err()
}

#[tauri::command]
pub async fn get_runtime_diagnostics_summary() -> CmdResult<crate::core::runtime_diagnostics::RuntimeDiagnosticsSummary>
{
    build_runtime_diagnostics_summary().await.stringify_err()
}

#[tauri::command]
pub async fn get_current_egress_identity(app_handle: tauri::AppHandle) -> CmdResult<CurrentEgressIdentity> {
    build_current_egress_identity(Some(&app_handle)).await.stringify_err()
}

/// 获取运行时存在的键
#[tauri::command]
pub async fn get_runtime_exists() -> CmdResult<HashSet<String>> {
    Ok(Config::runtime().await.latest_arc().exists_keys.clone())
}

#[tauri::command]
pub async fn get_runtime_proxy_topology() -> CmdResult<Proxies> {
    crate::core::runtime_snapshot::load_runtime_proxy_selection_state_from_disk().stringify_err()?;
    crate::core::runtime_snapshot::load_runtime_proxy_delay_state_from_disk().stringify_err()?;
    RuntimeSnapshotService::global()
        .refresh_proxy_topology_from_runtime_config()
        .await
        .map(|snapshot| {
            snapshot.proxies.unwrap_or_else(|| Proxies {
                proxies: HashMap::new(),
            })
        })
        .stringify_err()
}

#[tauri::command]
pub async fn get_runtime_proxy_selection_state() -> CmdResult<HashMap<std::string::String, std::string::String>> {
    crate::core::runtime_snapshot::load_runtime_proxy_selection_state_from_disk().stringify_err()?;
    Ok(crate::core::runtime_snapshot::runtime_proxy_selection_state())
}

#[tauri::command]
pub async fn get_runtime_proxy_delay_state() -> CmdResult<crate::core::runtime_snapshot::RuntimeProxyDelayState> {
    crate::core::runtime_snapshot::load_runtime_proxy_delay_state_from_disk().stringify_err()?;
    Ok(crate::core::runtime_snapshot::runtime_proxy_delay_state())
}

#[tauri::command]
pub async fn get_runtime_provider_health_state() -> CmdResult<crate::core::runtime_snapshot::RuntimeProviderHealthState>
{
    crate::core::runtime_snapshot::load_runtime_provider_health_state_from_disk().stringify_err()?;
    Ok(crate::core::runtime_snapshot::runtime_provider_health_state())
}

const LIFECYCLE_RESTART_CORE: &str = "restart_core";
const LIFECYCLE_RESTART_APP: &str = "restart_app";
const LIFECYCLE_RELOAD_CONFIG: &str = "reload_config";
const LIFECYCLE_UPDATE_GEO: &str = "update_geo";
const LIFECYCLE_TLS_ROTATION: &str = "tls_rotation";

const UPGRADE_CORE: &str = "core";
const UPGRADE_UI: &str = "ui";
const UPGRADE_GEO: &str = "geo";

#[tauri::command]
pub async fn get_runtime_upgrade_history() -> CmdResult<crate::core::runtime_snapshot::RuntimeUpgradeHistoryState> {
    crate::core::runtime_snapshot::load_runtime_upgrade_history_from_disk().stringify_err()?;
    Ok(crate::core::runtime_snapshot::runtime_upgrade_history_state())
}

/// 升级 Mihomo 内核（app-owned 门禁，记录升级历史）
#[tauri::command]
pub async fn upgrade_runtime_core(channel: CoreUpdaterChannel, force: bool) -> CmdResult<()> {
    let detail = Some(format!("{channel} · force={force}"));
    match Handle::mihomo().await.upgrade_core(channel, force).await {
        Ok(()) => {
            crate::core::runtime_snapshot::record_and_persist_runtime_upgrade_event(UPGRADE_CORE, true, None, detail);
            Ok(())
        }
        Err(error) => {
            crate::core::runtime_snapshot::record_and_persist_runtime_upgrade_event(
                UPGRADE_CORE,
                false,
                Some(error.to_string()),
                detail,
            );
            Err(error).stringify_err()
        }
    }
}

/// 升级 Mihomo 控制面板 UI（app-owned 门禁，记录升级历史）
#[tauri::command]
pub async fn upgrade_runtime_ui() -> CmdResult<()> {
    match Handle::mihomo().await.upgrade_ui().await {
        Ok(()) => {
            crate::core::runtime_snapshot::record_and_persist_runtime_upgrade_event(UPGRADE_UI, true, None, None);
            Ok(())
        }
        Err(error) => {
            crate::core::runtime_snapshot::record_and_persist_runtime_upgrade_event(
                UPGRADE_UI,
                false,
                Some(error.to_string()),
                None,
            );
            Err(error).stringify_err()
        }
    }
}

/// 升级 Geo 数据库（app-owned 门禁，记录升级历史）
#[tauri::command]
pub async fn upgrade_runtime_geo() -> CmdResult<()> {
    match Handle::mihomo().await.upgrade_geo().await {
        Ok(()) => {
            crate::core::runtime_snapshot::record_and_persist_runtime_upgrade_event(UPGRADE_GEO, true, None, None);
            Ok(())
        }
        Err(error) => {
            crate::core::runtime_snapshot::record_and_persist_runtime_upgrade_event(
                UPGRADE_GEO,
                false,
                Some(error.to_string()),
                None,
            );
            Err(error).stringify_err()
        }
    }
}

#[tauri::command]
pub async fn get_runtime_lifecycle_state() -> CmdResult<crate::core::runtime_snapshot::RuntimeLifecycleState> {
    crate::core::runtime_snapshot::load_runtime_lifecycle_state_from_disk().stringify_err()?;
    Ok(crate::core::runtime_snapshot::runtime_lifecycle_state())
}

/// 重启核心（app-owned 生命周期门禁，记录审计）
#[tauri::command]
pub async fn restart_runtime_core() -> CmdResult<()> {
    match crate::app::runtime::restart_core().await {
        Ok(()) => {
            crate::core::runtime_snapshot::record_and_persist_runtime_lifecycle_event(
                LIFECYCLE_RESTART_CORE,
                true,
                None,
                None,
            );
            Ok(())
        }
        Err(error) => {
            crate::core::runtime_snapshot::record_and_persist_runtime_lifecycle_event(
                LIFECYCLE_RESTART_CORE,
                false,
                Some(error.to_string()),
                None,
            );
            Err(error).stringify_err()
        }
    }
}

/// 重载运行时配置（app-owned 生命周期门禁，记录审计）
#[tauri::command]
pub async fn reload_runtime_config() -> CmdResult<()> {
    let outcome = CoreManager::global().update_config_forced().await;
    match outcome {
        Ok(outcome) if outcome.is_valid() => {
            crate::core::runtime_snapshot::record_and_persist_runtime_lifecycle_event(
                LIFECYCLE_RELOAD_CONFIG,
                true,
                None,
                None,
            );
            Ok(())
        }
        Ok(outcome) => {
            let message = outcome.to_string();
            crate::core::runtime_snapshot::record_and_persist_runtime_lifecycle_event(
                LIFECYCLE_RELOAD_CONFIG,
                false,
                Some(message.clone()),
                None,
            );
            Err(message.into())
        }
        Err(error) => {
            crate::core::runtime_snapshot::record_and_persist_runtime_lifecycle_event(
                LIFECYCLE_RELOAD_CONFIG,
                false,
                Some(error.to_string()),
                None,
            );
            Err(error).stringify_err()
        }
    }
}

/// 重启应用（app-owned 生命周期门禁，记录审计后再重启）
#[tauri::command]
pub async fn restart_runtime_app() -> CmdResult<()> {
    crate::core::runtime_snapshot::record_and_persist_runtime_lifecycle_event(LIFECYCLE_RESTART_APP, true, None, None);
    crate::app::runtime::restart_app().await;
    Ok(())
}

#[tauri::command]
pub async fn apply_runtime_proxy_selection(
    group_name: std::string::String,
    proxy_name: std::string::String,
) -> CmdResult<()> {
    let mihomo = Handle::mihomo().await;
    mihomo
        .select_node_for_group(&group_name, &proxy_name)
        .await
        .stringify_err()?;
    crate::core::runtime_snapshot::record_and_persist_runtime_proxy_selection(&group_name, &proxy_name);
    Ok(())
}

#[tauri::command]
pub async fn get_runtime_proxy_providers() -> CmdResult<ProxyProviders> {
    crate::core::runtime_snapshot::load_runtime_proxy_delay_state_from_disk().stringify_err()?;
    let runtime = Config::runtime().await;
    let runtime = runtime.latest_arc();
    let config = runtime
        .config
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("runtime config is not available"))
        .stringify_err()?;
    Ok(crate::core::runtime_snapshot::build_proxy_providers_from_runtime_config(config))
}

#[tauri::command]
pub async fn update_runtime_proxy_provider(provider_name: String) -> CmdResult<()> {
    Handle::mihomo()
        .await
        .update_proxy_provider(&provider_name)
        .await
        .stringify_err()
}

#[tauri::command]
pub async fn healthcheck_runtime_proxy_provider(provider_name: String) -> CmdResult<()> {
    let result = Handle::mihomo().await.healthcheck_proxy_provider(&provider_name).await;
    match result {
        Ok(()) => {
            crate::core::runtime_snapshot::record_and_persist_runtime_provider_health(&provider_name, true, None);
            Ok(())
        }
        Err(error) => {
            crate::core::runtime_snapshot::record_and_persist_runtime_provider_health(
                &provider_name,
                false,
                Some(error.to_string()),
            );
            Err(error).stringify_err()
        }
    }
}

#[tauri::command]
pub async fn delay_runtime_group(
    group_name: String,
    test_url: String,
    timeout: u32,
    keep_fixed: bool,
) -> CmdResult<HashMap<std::string::String, u32>> {
    let mihomo = Handle::mihomo().await;
    let fixed = if keep_fixed {
        mihomo.get_group_by_name(&group_name).await.stringify_err()?.fixed
    } else {
        None
    };

    let result = mihomo
        .delay_group(&group_name, &test_url, timeout)
        .await
        .stringify_err()?;

    for (proxy_name, delay) in &result {
        crate::core::runtime_snapshot::record_and_persist_runtime_proxy_delay(
            &group_name,
            proxy_name,
            *delay,
            &test_url,
        );
    }

    if keep_fixed
        && let Some(fixed) = fixed
        && !fixed.is_empty()
    {
        mihomo
            .select_node_for_group(&group_name, &fixed)
            .await
            .stringify_err()?;
        crate::core::runtime_snapshot::record_and_persist_runtime_proxy_selection(&group_name, &fixed);
    }

    Ok(result)
}

#[tauri::command]
pub async fn delay_runtime_proxy(
    proxy_name: String,
    test_url: String,
    timeout: u32,
    group_name: Option<String>,
) -> CmdResult<ProxyDelay> {
    let result = Handle::mihomo()
        .await
        .delay_proxy_by_name(&proxy_name, &test_url, timeout)
        .await
        .stringify_err()?;
    if let Some(group_name) = group_name.filter(|value| !value.is_empty()) {
        crate::core::runtime_snapshot::record_and_persist_runtime_proxy_delay(
            &group_name,
            &proxy_name,
            result.delay,
            &test_url,
        );
    }
    Ok(result)
}

#[tauri::command]
pub async fn get_runtime_rules() -> CmdResult<Rules> {
    let runtime = Config::runtime().await;
    let runtime = runtime.latest_arc();
    let config = runtime
        .config
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("runtime config is not available"))
        .stringify_err()?;
    Ok(crate::core::runtime_snapshot::build_rules_from_runtime_config(config))
}

#[tauri::command]
pub async fn get_runtime_rule_providers() -> CmdResult<RuleProviders> {
    let runtime = Config::runtime().await;
    let runtime = runtime.latest_arc();
    let config = runtime
        .config
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("runtime config is not available"))
        .stringify_err()?;
    Ok(crate::core::runtime_snapshot::build_rule_providers_from_runtime_config(
        config,
    ))
}

#[tauri::command]
pub async fn disable_runtime_rules(payload: HashMap<i32, bool>) -> CmdResult<()> {
    Handle::mihomo().await.disable_rules(&payload).await.stringify_err()
}

#[tauri::command]
pub async fn delete_runtime_rule(index: i32) -> CmdResult<()> {
    Handle::mihomo().await.delete_rule(index).await.stringify_err()
}

#[tauri::command]
pub async fn create_runtime_rule(
    rule_type: String,
    payload: String,
    proxy: String,
    source: Option<String>,
    sub_rule: Option<String>,
    position: Option<String>,
) -> CmdResult<i32> {
    Handle::mihomo()
        .await
        .create_rule(
            &rule_type,
            &payload,
            &proxy,
            source.as_deref(),
            sub_rule.as_deref(),
            position.as_deref(),
        )
        .await
        .stringify_err()
}

#[tauri::command]
pub async fn update_runtime_rule_provider(provider_name: String) -> CmdResult<()> {
    Handle::mihomo()
        .await
        .update_rule_provider(&provider_name)
        .await
        .stringify_err()
}

/// 获取运行时日志
#[tauri::command]
pub async fn get_runtime_logs() -> CmdResult<HashMap<String, Vec<(String, String)>>> {
    Ok(Config::runtime().await.latest_arc().chain_logs.clone())
}

#[tauri::command]
pub async fn get_runtime_proxy_chain_config(proxy_chain_exit_node: String) -> CmdResult<String> {
    let runtime = Config::runtime().await;
    let runtime = runtime.latest_arc();

    let config = runtime
        .config
        .as_ref()
        .ok_or_else(|| anyhow!("failed to parse config to yaml file"))
        .stringify_err()?;

    if let Some(serde_yaml_ng::Value::Sequence(proxies)) = config.get("proxies") {
        let mut proxy_name = Some(Some(proxy_chain_exit_node.as_str()));
        let mut proxies_chain = Vec::new();

        while let Some(proxy) = proxies.iter().find(|proxy| {
            if let serde_yaml_ng::Value::Mapping(proxy_map) = proxy {
                proxy_map.get("name").map(|x| x.as_str()) == proxy_name && proxy_map.get("dialer-proxy").is_some()
            } else {
                false
            }
        }) {
            proxies_chain.push(proxy.to_owned());
            proxy_name = proxy.get("dialer-proxy").map(|x| x.as_str());
        }

        if let Some(entry_proxy) = proxies
            .iter()
            .find(|proxy| proxy.get("name").map(|x| x.as_str()) == proxy_name)
            && !proxies_chain.is_empty()
        {
            // 添加第一个节点
            proxies_chain.push(entry_proxy.to_owned());
        }

        proxies_chain.reverse();

        let mut config: HashMap<String, Vec<serde_yaml_ng::Value>> = HashMap::new();

        config.insert("proxies".into(), proxies_chain);

        serde_yaml_ng::to_string(&config)
            .context("YAML generation failed")
            .map(|s| s.into())
            .stringify_err()
    } else {
        Err("failed to get proxies or proxy-groups".into())
    }
}

/// 更新运行时链式代理配置
#[tauri::command]
pub async fn update_proxy_chain_config_in_runtime(proxy_chain_config: Option<serde_yaml_ng::Value>) -> CmdResult<()> {
    {
        let runtime = Config::runtime().await;
        runtime.edit_draft(|d| d.update_proxy_chain_config(proxy_chain_config));
        // 我们需要在 CoreManager 中验证并应用配置，这里不应该直接调用 runtime.apply()
    }
    match CoreManager::global().apply_generate_config().await {
        Ok(outcome) if outcome.is_valid() => {}
        Ok(outcome) => logging!(
            warn,
            Type::Core,
            "Failed to apply runtime proxy chain config: {}",
            outcome
        ),
        Err(err) => logging!(error, Type::Core, "Failed to apply runtime proxy chain config: {}", err),
    }

    Ok(())
}
