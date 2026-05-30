use crate::core::session_affinity::*;

use super::{CmdResult, StringifyErr as _};

/// 获取所有绑定信息
#[tauri::command]
pub async fn session_affinity_get_bindings() -> CmdResult<Vec<BindingInfo>> {
    crate::feat::session_affinity_get_bindings().await.stringify_err()
}

/// 清除域名绑定
#[tauri::command]
pub async fn session_affinity_clear_binding(domain: String) -> CmdResult<()> {
    crate::feat::session_affinity_clear_binding(&domain).await.stringify_err()
}

/// 获取预定义规则
#[tauri::command]
pub async fn session_affinity_get_predefined_rules() -> CmdResult<Vec<DomainBindingRule>> {
    Ok(crate::feat::session_affinity_get_predefined_rules())
}

/// 清理过期绑定
#[tauri::command]
pub async fn session_affinity_cleanup_expired() -> CmdResult<()> {
    crate::feat::session_affinity_cleanup_expired().await.stringify_err()
}

/// 为域名选择节点
#[tauri::command]
pub async fn session_affinity_select_node_for_domain(
    domain: String,
    available_nodes: Vec<String>,
) -> CmdResult<String> {
    crate::feat::session_affinity_select_node_for_domain(&domain, available_nodes)
        .await
        .stringify_err()
}

/// 为进程选择节点
#[tauri::command]
pub async fn session_affinity_select_node_for_process(
    source_port: u16,
    available_nodes: Vec<String>,
) -> Result<String, String> {
    crate::feat::session_affinity_select_node_for_process(source_port, available_nodes)
        .await
        .map_err(|e| e.to_string())
}

/// 为连接选择节点
#[tauri::command]
pub async fn session_affinity_select_node_for_connection(
    source_ip: String,
    source_port: u16,
    available_nodes: Vec<String>,
) -> CmdResult<String> {
    crate::feat::session_affinity_select_node_for_connection(&source_ip, source_port, available_nodes)
        .await
        .stringify_err()
}
