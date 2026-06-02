use crate::{config::AdvancedConfig, core::CoreManager, core::blackhole_breaker::*};
use anyhow::Result;
use once_cell::sync::Lazy;
use std::sync::Arc;

static BLACKHOLE_BREAKER_MANAGER: Lazy<Arc<BlackholeBreakerManager>> =
    Lazy::new(|| Arc::new(BlackholeBreakerManager::new()));

pub fn get_blackhole_breaker_manager() -> Arc<BlackholeBreakerManager> {
    BLACKHOLE_BREAKER_MANAGER.clone()
}

pub async fn blackhole_breaker_get_config() -> BlackholeBreakerConfig {
    AdvancedConfig::load_default().blackhole_breaker
}

pub async fn apply_blackhole_breaker_config(config: BlackholeBreakerConfig) {
    get_blackhole_breaker_manager().update_config(config).await
}

pub async fn blackhole_breaker_update_config(config: BlackholeBreakerConfig) -> Result<()> {
    persist_blackhole_breaker_config(&config)?;
    apply_blackhole_breaker_config(config).await;
    CoreManager::global().update_config_checked().await?;
    Ok(())
}

pub async fn blackhole_breaker_get_states() -> Vec<BreakerRuntimeState> {
    get_blackhole_breaker_manager().get_all_states().await
}

pub async fn blackhole_breaker_record_result(rule_id: &str, success: bool) {
    get_blackhole_breaker_manager().record_result(rule_id, success).await
}

pub async fn blackhole_breaker_should_block_domain(domain: &str) -> bool {
    get_blackhole_breaker_manager().should_block_domain(domain).await
}

pub async fn blackhole_breaker_should_block_node(node_name: &str) -> bool {
    get_blackhole_breaker_manager().should_block_node(node_name).await
}

pub async fn blackhole_breaker_reset_rule(rule_id: &str) -> anyhow::Result<()> {
    get_blackhole_breaker_manager().reset_rule(rule_id).await
}

pub async fn blackhole_breaker_trip_rule(rule_id: &str) -> anyhow::Result<()> {
    get_blackhole_breaker_manager().trip_rule(rule_id).await
}

pub async fn blackhole_breaker_generate_reject_rules() -> Vec<(String, String)> {
    get_blackhole_breaker_manager().generate_reject_rules().await
}

pub async fn blackhole_breaker_record_fraud_score(domain: &str, fraud_score: u8) {
    get_blackhole_breaker_manager()
        .record_fraud_score(domain, fraud_score)
        .await
}

fn persist_blackhole_breaker_config(config: &BlackholeBreakerConfig) -> Result<()> {
    let mut advanced = AdvancedConfig::load_default_strict()?;
    advanced.blackhole_breaker = config.clone();
    advanced.validate()?;
    advanced.save_default()?;
    crate::feat::get_coordinator().hydrate_from_advanced_config(&advanced)?;
    Ok(())
}
