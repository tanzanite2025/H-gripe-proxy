use super::{CmdResult, StringifyErr};
use crate::config::ResidentialProxy;
use crate::core::ip_reputation::*;
use crate::core::residential_verification::ResidentialProxyVerification;

/// 获取 IP 信誉度配置
#[tauri::command]
pub async fn ip_reputation_get_config() -> CmdResult<IpReputationConfig> {
    crate::feat::ip_reputation_get_config().await.stringify_err()
}

/// 更新 IP 信誉度配置
#[tauri::command]
pub async fn ip_reputation_update_config(config: IpReputationConfig) -> CmdResult<()> {
    crate::feat::ip_reputation_update_config(config).await.stringify_err()
}

/// 检测 IP 信誉度
#[tauri::command]
pub async fn ip_reputation_check_ip(ip: String) -> CmdResult<IpReputation> {
    crate::feat::ip_reputation_check_ip(&ip).await.stringify_err()
}

/// 获取预定义路由规则
#[tauri::command]
pub async fn ip_reputation_get_predefined_rules() -> CmdResult<Vec<RiskRoutingRule>> {
    Ok(crate::feat::ip_reputation_get_predefined_rules())
}

/// 为域名选择节点
#[tauri::command]
pub async fn ip_reputation_select_node_for_domain(
    domain: String,
    available_nodes: Vec<(String, String)>,
) -> CmdResult<String> {
    crate::feat::ip_reputation_select_node_for_domain(&domain, &available_nodes)
        .await
        .stringify_err()
}

/// 清除缓存
#[tauri::command]
pub async fn ip_reputation_clear_cache() -> CmdResult<()> {
    crate::feat::ip_reputation_clear_cache().await.stringify_err()
}

/// 获取缓存统计
#[tauri::command]
pub async fn ip_reputation_get_cache_stats() -> CmdResult<(usize, usize)> {
    Ok(crate::feat::ip_reputation_get_cache_stats().await)
}

/// 获取缓存中所有条目
#[tauri::command]
pub async fn ip_reputation_get_cache_entries() -> CmdResult<Vec<IpReputation>> {
    Ok(crate::feat::ip_reputation_get_cache_entries().await)
}

/// 验证住宅代理实际出口
#[tauri::command]
pub async fn ip_reputation_verify_residential_proxy(
    app_handle: tauri::AppHandle,
    proxy: ResidentialProxy,
) -> CmdResult<ResidentialProxyVerification> {
    crate::core::residential_verification::verify_residential_proxy(proxy, Some(&app_handle))
        .await
        .stringify_err()
}
