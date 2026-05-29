use crate::core::ip_reputation::*;
use once_cell::sync::Lazy;
use std::sync::Arc;

/// 全局 IP 信誉度管理器实例
static IP_REPUTATION_MANAGER: Lazy<Arc<IpReputationManager>> =
    Lazy::new(|| Arc::new(IpReputationManager::new()));

/// 获取 IP 信誉度管理器实例
pub fn get_ip_reputation_manager() -> Arc<IpReputationManager> {
    IP_REPUTATION_MANAGER.clone()
}

/// 获取 IP 信誉度配置
#[tauri::command]
pub async fn ip_reputation_get_config() -> Result<IpReputationConfig, String> {
    IP_REPUTATION_MANAGER
        .get_config()
        .await
        .map_err(|e| e.to_string())
}

/// 更新 IP 信誉度配置
#[tauri::command]
pub async fn ip_reputation_update_config(config: IpReputationConfig) -> Result<(), String> {
    IP_REPUTATION_MANAGER
        .update_config(config)
        .await
        .map_err(|e| e.to_string())
}

/// 检测 IP 信誉度
#[tauri::command]
pub async fn ip_reputation_check_ip(ip: String) -> Result<IpReputation, String> {
    IP_REPUTATION_MANAGER
        .check_ip_reputation(&ip)
        .await
        .map_err(|e| e.to_string())
}

/// 获取预定义路由规则
#[tauri::command]
pub async fn ip_reputation_get_predefined_rules() -> Result<Vec<RiskRoutingRule>, String> {
    Ok(get_predefined_routing_rules())
}

/// 为域名选择节点
#[tauri::command]
pub async fn ip_reputation_select_node_for_domain(
    domain: String,
    available_nodes: Vec<(String, String)>,
) -> Result<String, String> {
    IP_REPUTATION_MANAGER
        .select_node_for_domain(&domain, &available_nodes)
        .await
        .map_err(|e| e.to_string())
}

/// 清除缓存
#[tauri::command]
pub async fn ip_reputation_clear_cache() -> Result<(), String> {
    IP_REPUTATION_MANAGER
        .clear_cache()
        .await
        .map_err(|e| e.to_string())
}

/// 获取缓存统计
#[tauri::command]
pub async fn ip_reputation_get_cache_stats() -> Result<(usize, usize), String> {
    Ok(IP_REPUTATION_MANAGER.get_cache_stats().await)
}
