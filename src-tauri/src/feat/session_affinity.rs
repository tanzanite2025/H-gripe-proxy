use crate::core::egress_identity::EgressSelectionContext;
use crate::core::session_affinity::SessionAffinityConfig;
use crate::core::session_affinity::SessionAffinityManager;
use crate::core::session_affinity::*;
use anyhow::Result;
use once_cell::sync::Lazy;
use std::sync::Arc;

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

fn load_session_affinity_config() -> anyhow::Result<SessionAffinityConfig> {
    Ok(crate::config::AdvancedConfig::load_default().session_affinity)
}

/// 获取会话绑定管理器实例
pub fn get_session_affinity_manager() -> Arc<SessionAffinityManager> {
    SESSION_AFFINITY_MANAGER.clone()
}

fn prioritize_available_nodes(available_nodes: Vec<String>, preferred_node: &str) -> Vec<String> {
    if let Some(index) = available_nodes.iter().position(|node| node == preferred_node) {
        let mut reordered = Vec::with_capacity(available_nodes.len());
        reordered.push(available_nodes[index].clone());
        reordered.extend(
            available_nodes.into_iter().enumerate().filter_map(
                |(current_index, node)| {
                    if current_index == index { None } else { Some(node) }
                },
            ),
        );
        reordered
    } else {
        available_nodes
    }
}

/// 获取所有绑定信息
pub async fn session_affinity_get_bindings() -> Result<Vec<BindingInfo>> {
    get_session_affinity_manager().get_all_bindings().await
}

/// 清除域名绑定
pub async fn session_affinity_clear_binding(domain: &str) -> Result<()> {
    get_session_affinity_manager().clear_domain_binding(domain).await
}

/// 获取预定义规则
pub fn session_affinity_get_predefined_rules() -> Vec<DomainBindingRule> {
    get_predefined_rules()
}

/// 清理过期绑定
pub async fn session_affinity_cleanup_expired() -> Result<()> {
    get_session_affinity_manager().cleanup_expired_bindings().await
}

/// 为域名选择节点
pub async fn session_affinity_select_node_for_domain(domain: &str, available_nodes: Vec<String>) -> Result<String> {
    let _ = crate::feat::sync_coordinator_from_advanced_config_async().await;
    let coordinator = crate::feat::get_coordinator();
    let egress_context = crate::feat::enrich_egress_selection_context(EgressSelectionContext {
        domain: Some(domain.to_owned()),
        available_nodes: available_nodes.clone(),
        ..Default::default()
    })
    .await;
    let effective_available_nodes = egress_context.available_nodes.clone();
    let ordered_nodes = coordinator
        .egress_identity_manager()
        .assign(egress_context.clone())
        .map(|resolved| prioritize_available_nodes(effective_available_nodes.clone(), &resolved.selected_node))
        .unwrap_or_else(|_| effective_available_nodes.clone());
    let selected_node = get_session_affinity_manager()
        .select_node_for_domain(domain, &ordered_nodes)
        .await?;

    let _ = coordinator
        .egress_identity_manager()
        .record_assignment(egress_context, selected_node.clone());

    Ok(selected_node)
}

/// 为进程选择节点
pub async fn session_affinity_select_node_for_process(
    source_port: u16,
    available_nodes: Vec<String>,
) -> Result<String> {
    let _ = crate::feat::sync_coordinator_from_advanced_config_async().await;
    let process_name = process_detection::get_process_name_by_port(source_port).ok();
    let coordinator = crate::feat::get_coordinator();
    let enriched_context = crate::feat::enrich_egress_selection_context(EgressSelectionContext {
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
            .map(|resolved| prioritize_available_nodes(effective_available_nodes.clone(), &resolved.selected_node))
            .unwrap_or_else(|| effective_available_nodes.clone())
    } else {
        effective_available_nodes.clone()
    };
    let selected_node = get_session_affinity_manager()
        .select_node_for_process(source_port, &ordered_nodes)
        .await?;

    if let Some(egress_context) = egress_context {
        let _ = coordinator
            .egress_identity_manager()
            .record_assignment(egress_context, selected_node.clone());
    }

    Ok(selected_node)
}

/// 为连接选择节点
pub async fn session_affinity_select_node_for_connection(
    source_ip: &str,
    source_port: u16,
    available_nodes: Vec<String>,
) -> Result<String> {
    let _ = crate::feat::sync_coordinator_from_advanced_config_async().await;
    let coordinator = crate::feat::get_coordinator();
    let egress_context = crate::feat::enrich_egress_selection_context(EgressSelectionContext {
        source_ip: Some(source_ip.to_owned()),
        source_port: Some(source_port),
        available_nodes: available_nodes.clone(),
        ..Default::default()
    })
    .await;
    let effective_available_nodes = egress_context.available_nodes.clone();
    let ordered_nodes = coordinator
        .egress_identity_manager()
        .assign(egress_context.clone())
        .map(|resolved| prioritize_available_nodes(effective_available_nodes.clone(), &resolved.selected_node))
        .unwrap_or_else(|_| effective_available_nodes.clone());
    let selected_node = get_session_affinity_manager()
        .select_node_for_connection(source_ip, source_port, &ordered_nodes)
        .await?;

    let _ = coordinator
        .egress_identity_manager()
        .record_assignment(egress_context, selected_node.clone());

    Ok(selected_node)
}

/// 启动后台清理任务
pub fn start_cleanup_task() {
    let manager = get_session_affinity_manager().clone();
    manager.start_cleanup_task();
}
