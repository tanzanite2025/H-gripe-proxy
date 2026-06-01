use crate::core::blackhole_breaker::*;
use once_cell::sync::Lazy;
use std::sync::Arc;

static BLACKHOLE_BREAKER_MANAGER: Lazy<Arc<BlackholeBreakerManager>> =
    Lazy::new(|| Arc::new(BlackholeBreakerManager::new()));

pub fn get_blackhole_breaker_manager() -> Arc<BlackholeBreakerManager> {
    BLACKHOLE_BREAKER_MANAGER.clone()
}

pub async fn blackhole_breaker_get_config() -> BlackholeBreakerConfig {
    get_blackhole_breaker_manager().get_config().await
}

pub async fn blackhole_breaker_update_config(config: BlackholeBreakerConfig) {
    get_blackhole_breaker_manager().update_config(config).await
}

pub async fn blackhole_breaker_get_states() -> Vec<BreakerRuntimeState> {
    get_blackhole_breaker_manager().get_all_states().await
}

pub async fn blackhole_breaker_record_result(rule_id: &str, success: bool) {
    get_blackhole_breaker_manager()
        .record_result(rule_id, success)
        .await
}

pub async fn blackhole_breaker_should_block_domain(domain: &str) -> bool {
    get_blackhole_breaker_manager()
        .should_block_domain(domain)
        .await
}

pub async fn blackhole_breaker_should_block_node(node_name: &str) -> bool {
    get_blackhole_breaker_manager()
        .should_block_node(node_name)
        .await
}

pub async fn blackhole_breaker_reset_rule(rule_id: &str) -> anyhow::Result<()> {
    get_blackhole_breaker_manager().reset_rule(rule_id).await
}

pub async fn blackhole_breaker_trip_rule(rule_id: &str) -> anyhow::Result<()> {
    get_blackhole_breaker_manager().trip_rule(rule_id).await
}

pub async fn blackhole_breaker_generate_reject_rules() -> Vec<(String, String)> {
    get_blackhole_breaker_manager()
        .generate_reject_rules()
        .await
}

pub async fn blackhole_breaker_record_fraud_score(domain: &str, fraud_score: u8) {
    get_blackhole_breaker_manager()
        .record_fraud_score(domain, fraud_score)
        .await
}
