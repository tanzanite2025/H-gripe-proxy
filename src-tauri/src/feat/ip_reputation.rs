use crate::{config::AdvancedConfig, core::ip_reputation::*};
use anyhow::Result;
use once_cell::sync::Lazy;
use std::sync::Arc;

fn sanitize_ip_reputation_config(mut config: IpReputationConfig) -> IpReputationConfig {
    config.metadata_provider = crate::core::ip_intelligence::IpIntelligenceProviderConfig::default();
    config
}

/// 全局 IP 信誉度管理器实例
static IP_REPUTATION_MANAGER: Lazy<Arc<IpReputationManager>> = Lazy::new(|| {
    let config = sanitize_ip_reputation_config(AdvancedConfig::load_default().ip_reputation);
    Arc::new(IpReputationManager::from_config(config))
});

/// 获取 IP 信誉度管理器实例
pub fn get_ip_reputation_manager() -> Arc<IpReputationManager> {
    IP_REPUTATION_MANAGER.clone()
}

pub async fn ip_reputation_get_config() -> Result<IpReputationConfig> {
    Ok(sanitize_ip_reputation_config(AdvancedConfig::load_default().ip_reputation))
}

pub async fn apply_ip_reputation_config(config: IpReputationConfig) -> Result<()> {
    get_ip_reputation_manager()
        .update_config(sanitize_ip_reputation_config(config))
        .await
}

pub async fn ip_reputation_update_config(config: IpReputationConfig) -> Result<()> {
    let mut advanced = AdvancedConfig::load_default_strict()?;
    advanced.ip_reputation = sanitize_ip_reputation_config(config);
    crate::feat::save_advanced_config(&advanced).await
}

pub async fn ip_reputation_check_ip(ip: &str) -> Result<IpReputation> {
    get_ip_reputation_manager().check_ip_reputation(ip).await
}

pub async fn ip_reputation_probe_metadata_provider(
    _provider_config: crate::core::ip_intelligence::IpIntelligenceProviderConfig,
    target_ip: Option<&str>,
) -> crate::core::ip_intelligence::IpIntelligenceProviderHealthReport {
    let local_provider = crate::core::ip_intelligence::IpIntelligenceProviderConfig::default();
    crate::core::ip_intelligence::probe_provider(&local_provider, target_ip).await
}

pub fn ip_reputation_get_predefined_rules() -> Vec<RiskRoutingRule> {
    get_predefined_routing_rules()
}

pub async fn ip_reputation_select_node_for_domain(
    domain: &str,
    available_nodes: &[(String, String)],
) -> Result<String> {
    get_ip_reputation_manager()
        .select_node_for_domain(domain, available_nodes)
        .await
}

pub async fn ip_reputation_clear_cache() -> Result<()> {
    get_ip_reputation_manager().clear_cache().await
}

pub async fn ip_reputation_get_cache_stats() -> (usize, usize) {
    get_ip_reputation_manager().get_cache_stats().await
}

pub async fn ip_reputation_get_cache_entries() -> Vec<IpReputation> {
    get_ip_reputation_manager().get_cache_entries().await
}
