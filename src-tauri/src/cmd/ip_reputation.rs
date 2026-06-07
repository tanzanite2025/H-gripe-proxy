use super::{CmdResult, StringifyErr};
use crate::config::ResidentialProxy;
use crate::core::ip_intelligence::{
    IpIntelligenceProviderConfig, IpIntelligenceProviderHealthReport, IpIntelligenceProviderRegistration,
};
use crate::core::ip_reputation::*;
use crate::core::residential_verification::ResidentialProxyVerification;

#[tauri::command]
pub async fn ip_reputation_get_config() -> CmdResult<IpReputationConfig> {
    crate::feat::ip_reputation_get_config().await.stringify_err()
}

#[tauri::command]
pub async fn ip_reputation_update_config(config: IpReputationConfig) -> CmdResult<()> {
    crate::feat::ip_reputation_update_config(config).await.stringify_err()
}

#[tauri::command]
pub async fn ip_reputation_check_ip(ip: String) -> CmdResult<IpReputation> {
    crate::feat::ip_reputation_check_ip(&ip).await.stringify_err()
}

#[tauri::command]
pub async fn ip_reputation_get_registered_metadata_providers() -> CmdResult<Vec<IpIntelligenceProviderRegistration>> {
    Ok(crate::feat::ip_reputation_get_registered_metadata_providers())
}

#[tauri::command]
pub async fn ip_reputation_probe_metadata_provider(
    provider_config: IpIntelligenceProviderConfig,
    target_ip: Option<String>,
) -> CmdResult<IpIntelligenceProviderHealthReport> {
    Ok(crate::feat::ip_reputation_probe_metadata_provider(provider_config, target_ip.as_deref()).await)
}

#[tauri::command]
pub async fn ip_reputation_get_predefined_rules() -> CmdResult<Vec<RiskRoutingRule>> {
    Ok(crate::feat::ip_reputation_get_predefined_rules())
}

#[tauri::command]
pub async fn ip_reputation_select_node_for_domain(
    domain: String,
    available_nodes: Vec<(String, String)>,
) -> CmdResult<String> {
    crate::feat::ip_reputation_select_node_for_domain(&domain, &available_nodes)
        .await
        .stringify_err()
}

#[tauri::command]
pub async fn ip_reputation_clear_cache() -> CmdResult<()> {
    crate::feat::ip_reputation_clear_cache().await.stringify_err()
}

#[tauri::command]
pub async fn ip_reputation_get_cache_stats() -> CmdResult<(usize, usize)> {
    Ok(crate::feat::ip_reputation_get_cache_stats().await)
}

#[tauri::command]
pub async fn ip_reputation_get_cache_entries() -> CmdResult<Vec<IpReputation>> {
    Ok(crate::feat::ip_reputation_get_cache_entries().await)
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
