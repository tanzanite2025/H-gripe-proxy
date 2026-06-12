use super::{CmdResult, StringifyErr};
use crate::config::ResidentialProxy;
use crate::core::ip_intelligence::{IpIntelligenceProviderConfig, IpIntelligenceProviderHealthReport};
use crate::core::ip_reputation::*;
use crate::core::residential_verification::ResidentialProxyVerification;

#[tauri::command]
pub async fn ip_reputation_get_config() -> CmdResult<IpReputationConfig> {
    Ok(crate::core::coordinator::get_coordinator()
        .get_advanced_config()
        .ip_reputation)
}

#[tauri::command]
pub async fn ip_reputation_update_config(config: IpReputationConfig) -> CmdResult<()> {
    crate::core::coordinator::update_advanced_config(move |advanced| {
        advanced.ip_reputation = config;
    })
    .await
    .stringify_err()
}

#[tauri::command]
pub async fn ip_reputation_check_ip(ip: String) -> CmdResult<IpReputation> {
    crate::core::ip_reputation::get_ip_reputation_manager()
        .check_ip_reputation(&ip)
        .await
        .stringify_err()
}

#[tauri::command]
pub async fn ip_reputation_probe_metadata_provider(
    _provider_config: IpIntelligenceProviderConfig,
    target_ip: Option<String>,
) -> CmdResult<IpIntelligenceProviderHealthReport> {
    Ok(crate::core::ip_reputation::probe_local_metadata_provider(target_ip.as_deref()).await)
}

#[tauri::command]
pub async fn ip_reputation_get_predefined_rules() -> CmdResult<Vec<RiskRoutingRule>> {
    Ok(crate::core::ip_reputation::get_predefined_routing_rules())
}

#[tauri::command]
pub async fn ip_reputation_select_node_for_domain(
    domain: String,
    available_nodes: Vec<(String, String)>,
) -> CmdResult<String> {
    crate::core::ip_reputation::get_ip_reputation_manager()
        .select_node_for_domain(&domain, &available_nodes)
        .await
        .stringify_err()
}

#[tauri::command]
pub async fn ip_reputation_clear_cache() -> CmdResult<()> {
    crate::core::ip_reputation::get_ip_reputation_manager()
        .clear_cache()
        .await
        .stringify_err()
}

#[tauri::command]
pub async fn ip_reputation_get_cache_stats() -> CmdResult<(usize, usize)> {
    Ok(crate::core::ip_reputation::get_ip_reputation_manager()
        .get_cache_stats()
        .await)
}

#[tauri::command]
pub async fn ip_reputation_get_cache_entries() -> CmdResult<Vec<IpReputation>> {
    Ok(crate::core::ip_reputation::get_ip_reputation_manager()
        .get_cache_entries()
        .await)
}

#[tauri::command]
pub async fn ip_reputation_verify_residential_proxy(
    app_handle: tauri::AppHandle,
    proxy: ResidentialProxy,
) -> CmdResult<ResidentialProxyVerification> {
    crate::core::residential_verification::verify_residential_proxy(proxy, Some(&app_handle))
        .await
        .stringify_err()
}
