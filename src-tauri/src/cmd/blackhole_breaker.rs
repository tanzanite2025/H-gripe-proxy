use super::{CmdResult, StringifyErr};
use crate::core::blackhole_breaker::*;

/// 获取黑洞熔断器配置
#[tauri::command]
pub async fn blackhole_breaker_get_config() -> CmdResult<BlackholeBreakerConfig> {
    Ok(crate::core::coordinator::get_coordinator()
        .get_advanced_config()
        .blackhole_breaker)
}

/// 更新黑洞熔断器配置
#[tauri::command]
pub async fn blackhole_breaker_update_config(config: BlackholeBreakerConfig) -> CmdResult<()> {
    crate::core::coordinator::update_advanced_config(move |advanced| {
        advanced.blackhole_breaker = config;
    })
    .await
    .stringify_err()
}

/// 获取所有熔断规则运行时状态
#[tauri::command]
pub async fn blackhole_breaker_get_states() -> CmdResult<Vec<BreakerRuntimeState>> {
    Ok(crate::core::blackhole_breaker::get_blackhole_breaker_manager()
        .get_all_states()
        .await)
}

/// 记录请求结果
#[tauri::command]
pub async fn blackhole_breaker_record_result(rule_id: String, success: bool) -> CmdResult<()> {
    crate::core::blackhole_breaker::get_blackhole_breaker_manager()
        .record_result(&rule_id, success)
        .await;
    Ok(())
}

/// 检查域名是否被熔断
#[tauri::command]
pub async fn blackhole_breaker_should_block_domain(domain: String) -> CmdResult<bool> {
    Ok(crate::core::blackhole_breaker::get_blackhole_breaker_manager()
        .should_block_domain(&domain)
        .await)
}

/// Check whether a node is blocked by breaker rules.
#[tauri::command]
pub async fn blackhole_breaker_should_block_node(node_name: String) -> CmdResult<bool> {
    Ok(crate::core::blackhole_breaker::get_blackhole_breaker_manager()
        .should_block_node(&node_name)
        .await)
}

/// 手动重置熔断规则
#[tauri::command]
pub async fn blackhole_breaker_reset_rule(rule_id: String) -> CmdResult<()> {
    crate::core::blackhole_breaker::get_blackhole_breaker_manager()
        .reset_rule(&rule_id)
        .await
        .stringify_err()
}

/// 手动触发熔断
#[tauri::command]
pub async fn blackhole_breaker_trip_rule(rule_id: String) -> CmdResult<()> {
    crate::core::blackhole_breaker::get_blackhole_breaker_manager()
        .trip_rule(&rule_id)
        .await
        .stringify_err()
}

/// 记录欺诈评分（IP 信誉集成）
#[tauri::command]
pub async fn blackhole_breaker_record_fraud_score(domain: String, fraud_score: u8) -> CmdResult<()> {
    crate::core::blackhole_breaker::get_blackhole_breaker_manager()
        .record_fraud_score(&domain, fraud_score)
        .await;
    Ok(())
}
