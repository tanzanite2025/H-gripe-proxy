use crate::core::session_affinity::*;
use crate::config::AdvancedConfig;
use crate::core::egress_identity::EgressSelectionContext;
use once_cell::sync::Lazy;
use std::sync::Arc;

use super::coordinator::{get_coordinator, sync_coordinator_from_advanced_config};
use super::egress_identity::enrich_egress_selection_context;

/// 全局会话绑定管理器实例
static SESSION_AFFINITY_MANAGER: Lazy<Arc<SessionAffinityManager>> = Lazy::new(|| {
    let config = load_session_affinity_config().unwrap_or_else(|e| {
        log::warn!("[SessionAffinity] 加载配置失败，使用默认配置: {}", e);
        SessionAffinityConfig::default()
    });
    let manager = SessionAffinityManager::new();
    let manager = Arc::new(manager);
    // 应用加载到的配置
    futures::executor::block_on(manager.update_config(config)).ok();
    manager
});

fn advanced_config_path() -> Result<std::path::PathBuf, String> {
    crate::utils::dirs::app_home_dir()
        .map(|dir| dir.join("advanced.yaml"))
        .map_err(|e| e.to_string())
}

fn load_session_affinity_config() -> Result<SessionAffinityConfig, String> {
    let path = advanced_config_path()?;
    let config = AdvancedConfig::load(&path).map_err(|e| e.to_string())?;
    Ok(config.session_affinity)
}

/// 获取会话绑定管理器实例
pub fn get_session_affinity_manager() -> Arc<SessionAffinityManager> {
    SESSION_AFFINITY_MANAGER.clone()
}

fn prioritize_available_nodes(
    available_nodes: Vec<String>,
    preferred_node: &str,
) -> Vec<String> {
    if let Some(index) = available_nodes.iter().position(|node| node == preferred_node) {
        let mut reordered = Vec::with_capacity(available_nodes.len());
        reordered.push(available_nodes[index].clone());
        reordered.extend(
            available_nodes
                .into_iter()
                .enumerate()
                .filter_map(|(current_index, node)| {
                    if current_index == index {
                        None
                    } else {
                        Some(node)
                    }
                }),
        );
        reordered
    } else {
        available_nodes
    }
}

/// 获取所有绑定信息
#[tauri::command]
pub async fn session_affinity_get_bindings() -> Result<Vec<BindingInfo>, String> {
    SESSION_AFFINITY_MANAGER.get_all_bindings().await.map_err(|e| e.to_string())
}

/// 清除域名绑定
#[tauri::command]
pub async fn session_affinity_clear_binding(domain: String) -> Result<(), String> {
    SESSION_AFFINITY_MANAGER
        .clear_domain_binding(&domain)
        .await
        .map_err(|e| e.to_string())
}

/// 获取预定义规则
#[tauri::command]
pub async fn session_affinity_get_predefined_rules() -> Result<Vec<DomainBindingRule>, String> {
    Ok(get_predefined_rules())
}

/// 清理过期绑定
#[tauri::command]
pub async fn session_affinity_cleanup_expired() -> Result<(), String> {
    SESSION_AFFINITY_MANAGER
        .cleanup_expired_bindings()
        .await
        .map_err(|e| e.to_string())
}


/// 为域名选择节点
#[tauri::command]
pub async fn session_affinity_select_node_for_domain(
    domain: String,
    available_nodes: Vec<String>,
) -> Result<String, String> {
    let _ = sync_coordinator_from_advanced_config();
    let coordinator = get_coordinator();
    let egress_context = enrich_egress_selection_context(EgressSelectionContext {
        domain: Some(domain.clone()),
        available_nodes: available_nodes.clone(),
        ..Default::default()
    })
    .await;
    let effective_available_nodes = egress_context.available_nodes.clone();
    let ordered_nodes = coordinator
        .egress_identity_manager()
        .assign(egress_context.clone())
        .map(|resolved| {
            prioritize_available_nodes(effective_available_nodes.clone(), &resolved.selected_node)
        })
        .unwrap_or_else(|_| effective_available_nodes.clone());
    let selected_node = SESSION_AFFINITY_MANAGER
        .select_node_for_domain(&domain, &ordered_nodes)
        .await
        .map_err(|e| e.to_string())?;

    let _ = coordinator
        .egress_identity_manager()
        .record_assignment(egress_context, selected_node.clone());

    Ok(selected_node)
}

/// 为进程选择节点
#[tauri::command]
pub async fn session_affinity_select_node_for_process(
    source_port: u16,
    available_nodes: Vec<String>,
) -> Result<String, String> {
    let _ = sync_coordinator_from_advanced_config();
    let process_name = process_detection::get_process_name_by_port(source_port).ok();
    let coordinator = get_coordinator();
    let enriched_context = enrich_egress_selection_context(EgressSelectionContext {
        process_name: process_name.clone(),
        source_port: Some(source_port),
        available_nodes: available_nodes.clone(),
        ..Default::default()
    })
    .await;
    let egress_context = process_name.map(|_| enriched_context.clone());
    let effective_available_nodes = enriched_context.available_nodes.clone();
    let ordered_nodes = if let Some(ctx) = egress_context.clone() {
        coordinator
            .egress_identity_manager()
            .assign(ctx)
            .ok()
            .map(|resolved| {
                prioritize_available_nodes(effective_available_nodes.clone(), &resolved.selected_node)
            })
            .unwrap_or_else(|| effective_available_nodes.clone())
    } else {
        effective_available_nodes.clone()
    };
    let selected_node = SESSION_AFFINITY_MANAGER
        .select_node_for_process(source_port, &ordered_nodes)
        .await
        .map_err(|e| e.to_string())?;

    if let Some(egress_context) = egress_context {
        let _ = coordinator
            .egress_identity_manager()
            .record_assignment(egress_context, selected_node.clone());
    }

    Ok(selected_node)
}

/// 为连接选择节点
#[tauri::command]
pub async fn session_affinity_select_node_for_connection(
    source_ip: String,
    source_port: u16,
    available_nodes: Vec<String>,
) -> Result<String, String> {
    let _ = sync_coordinator_from_advanced_config();
    let coordinator = get_coordinator();
    let egress_context = enrich_egress_selection_context(EgressSelectionContext {
        source_ip: Some(source_ip.clone()),
        source_port: Some(source_port),
        available_nodes: available_nodes.clone(),
        ..Default::default()
    })
    .await;
    let effective_available_nodes = egress_context.available_nodes.clone();
    let ordered_nodes = coordinator
        .egress_identity_manager()
        .assign(egress_context.clone())
        .map(|resolved| {
            prioritize_available_nodes(effective_available_nodes.clone(), &resolved.selected_node)
        })
        .unwrap_or_else(|_| effective_available_nodes.clone());
    let selected_node = SESSION_AFFINITY_MANAGER
        .select_node_for_connection(&source_ip, source_port, &ordered_nodes)
        .await
        .map_err(|e| e.to_string())?;

    let _ = coordinator
        .egress_identity_manager()
        .record_assignment(egress_context, selected_node.clone());

    Ok(selected_node)
}

/// 启动后台清理任务
pub fn start_cleanup_task() {
    let manager = SESSION_AFFINITY_MANAGER.clone();
    manager.start_cleanup_task();
}
