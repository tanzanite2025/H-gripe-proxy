use super::{CmdResult, StringifyErr};
use crate::core::blackhole_breaker::*;

/// 获取黑洞熔断器配置
#[tauri::command]
pub async fn blackhole_breaker_get_config() -> CmdResult<BlackholeBreakerConfig> {
    Ok(crate::feat::blackhole_breaker_get_config().await)
}

/// 更新黑洞熔断器配置
#[tauri::command]
pub async fn blackhole_breaker_update_config(config: BlackholeBreakerConfig) -> CmdResult<()> {
    crate::feat::blackhole_breaker_update_config(config)
        .await
        .stringify_err()
}

/// 获取所有熔断规则运行时状态
#[tauri::command]
pub async fn blackhole_breaker_get_states() -> CmdResult<Vec<BreakerRuntimeState>> {
    Ok(crate::feat::blackhole_breaker_get_states().await)
}

/// 记录请求结果
#[tauri::command]
pub async fn blackhole_breaker_record_result(rule_id: String, success: bool) -> CmdResult<()> {
    crate::feat::blackhole_breaker_record_result(&rule_id, success).await;
    Ok(())
}

/// 检查域名是否被熔断
#[tauri::command]
pub async fn blackhole_breaker_should_block_domain(domain: String) -> CmdResult<bool> {
    Ok(crate::feat::blackhole_breaker_should_block_domain(&domain).await)
}

/// Check whether a node is blocked by breaker rules.
#[tauri::command]
pub async fn blackhole_breaker_should_block_node(node_name: String) -> CmdResult<bool> {
    Ok(crate::feat::blackhole_breaker_should_block_node(&node_name).await)
}

/// 手动重置熔断规则
#[tauri::command]
pub async fn blackhole_breaker_reset_rule(rule_id: String) -> CmdResult<()> {
    crate::feat::blackhole_breaker_reset_rule(&rule_id)
        .await
        .stringify_err()
}

/// 手动触发熔断
#[tauri::command]
pub async fn blackhole_breaker_trip_rule(rule_id: String) -> CmdResult<()> {
    crate::feat::blackhole_breaker_trip_rule(&rule_id).await.stringify_err()
}

/// 记录欺诈评分（IP 信誉集成）
#[tauri::command]
pub async fn blackhole_breaker_record_fraud_score(domain: String, fraud_score: u8) -> CmdResult<()> {
    crate::feat::blackhole_breaker_record_fraud_score(&domain, fraud_score).await;
    Ok(())
}
