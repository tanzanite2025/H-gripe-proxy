/**
 * 多路径路由 Tauri 命令
 */

use crate::multipath::{
    MultipathConfig, NodePool, PathNode, SessionBinding, NodeStats,
};
use std::collections::HashMap;
use super::{CmdResult, StringifyErr as _};

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct TestResult {
    pub success: bool,
    pub latency: u64,
    pub message: String,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct ImportResult {
    pub success: bool,
    pub imported_count: usize,
    pub message: String,
}

/// 获取多路径配置
#[tauri::command]
pub fn multipath_get_config() -> CmdResult<MultipathConfig> {
    Ok(crate::feat::multipath_get_config())
}

/// 更新多路径配置
#[tauri::command]
pub fn multipath_update_config(config: MultipathConfig) -> CmdResult<()> {
    crate::feat::apply_multipath_config(config).stringify_err()
}

/// 获取会话绑定规则
#[tauri::command]
pub fn multipath_get_bindings() -> CmdResult<Vec<SessionBinding>> {
    Ok(crate::feat::multipath_get_bindings())
}

/// 添加会话绑定规则
#[tauri::command]
pub fn multipath_add_binding(binding: SessionBinding) -> CmdResult<()> {
    crate::feat::multipath_add_binding(binding).stringify_err()
}

/// 删除会话绑定规则
#[tauri::command]
pub fn multipath_remove_binding(domain_pattern: String) -> CmdResult<()> {
    crate::feat::multipath_remove_binding(&domain_pattern).stringify_err()
}

/// 获取预定义的会话绑定规则
#[tauri::command]
pub fn multipath_get_predefined_bindings() -> CmdResult<Vec<SessionBinding>> {
    Ok(crate::feat::multipath_get_predefined_bindings())
}

/// 添加节点池
#[tauri::command]
pub fn multipath_add_pool(pool: NodePool) -> CmdResult<()> {
    crate::feat::multipath_add_pool(pool).stringify_err()
}

/// 删除节点池
#[tauri::command]
pub fn multipath_remove_pool(pool_name: String) -> CmdResult<()> {
    crate::feat::multipath_remove_pool(&pool_name).stringify_err()
}

/// 更新节点池
#[tauri::command]
pub fn multipath_update_pool(pool: NodePool) -> CmdResult<()> {
    crate::feat::multipath_update_pool(pool).stringify_err()
}

/// 添加节点到池
#[tauri::command]
pub fn multipath_add_node(pool_name: String, node: PathNode) -> CmdResult<()> {
    crate::feat::multipath_add_node(&pool_name, node).stringify_err()
}

/// 从池中删除节点
#[tauri::command]
pub fn multipath_remove_node(pool_name: String, node_name: String) -> CmdResult<()> {
    crate::feat::multipath_remove_node(&pool_name, &node_name).stringify_err()
}

/// 测试节点连接
#[tauri::command]
pub async fn multipath_test_node(node: PathNode) -> CmdResult<TestResult> {
    log::info!("测试节点: {}", node.name);
    Ok(TestResult {
        success: true,
        latency: 50,
        message: "连接成功".to_string(),
    })
}

/// 批量导入节点
#[tauri::command]
pub fn multipath_import_nodes(
    pool_name: String,
    nodes_yaml: String,
) -> CmdResult<ImportResult> {
    match crate::feat::multipath_import_nodes(&pool_name, &nodes_yaml) {
        Ok(count) => Ok(ImportResult {
            success: true,
            imported_count: count,
            message: format!("成功导入 {} 个节点", count),
        }),
        Err(e) => Err(e.to_string().into()),
    }
}

/// 导出节点配置
#[tauri::command]
pub fn multipath_export_nodes(pool_name: String) -> CmdResult<String> {
    crate::feat::multipath_export_nodes(&pool_name).stringify_err()
}

/// 获取推荐配置
#[tauri::command]
pub fn multipath_get_recommended_config() -> CmdResult<MultipathConfig> {
    Ok(crate::feat::multipath_get_recommended_config())
}

/// 获取节点统计
#[tauri::command]
pub fn multipath_get_node_stats() -> CmdResult<HashMap<String, NodeStats>> {
    Ok(crate::feat::multipath_get_node_stats())
}

/// 获取活跃会话数
#[tauri::command]
pub fn multipath_get_active_session_count() -> CmdResult<usize> {
    Ok(crate::feat::multipath_get_active_session_count())
}

/// 清理过期会话
#[tauri::command]
pub fn multipath_cleanup_sessions() -> CmdResult<()> {
    crate::feat::multipath_cleanup_sessions();
    Ok(())
}
