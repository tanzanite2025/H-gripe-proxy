use std::collections::{HashMap, HashSet};
use std::net::IpAddr;

use crate::core::egress_identity::{
    EgressNodeMetadata, EgressSelectionContext, ResolvedEgressIdentity,
};

use super::coordinator::{get_coordinator, sync_coordinator_from_advanced_config};

async fn resolve_server_ip(server: &str) -> Option<String> {
    if let Ok(ip_addr) = server.parse::<IpAddr>() {
        return Some(ip_addr.to_string());
    }

    let resolved = tokio::net::lookup_host((server, 0)).await.ok()?;
    let resolved_addresses = resolved.collect::<Vec<_>>();

    resolved_addresses
        .iter()
        .find(|socket_addr| matches!(socket_addr.ip(), IpAddr::V4(_)))
        .or_else(|| resolved_addresses.first())
        .map(|socket_addr| socket_addr.ip().to_string())
}

pub async fn enrich_egress_selection_context(
    mut ctx: EgressSelectionContext,
) -> EgressSelectionContext {
    let multipath_config = get_coordinator().multipath_manager().get_config();
    let requested_nodes = if ctx.available_nodes.is_empty() {
        None
    } else {
        Some(ctx.available_nodes.iter().cloned().collect::<HashSet<_>>())
    };

    let mut metadata_index = HashMap::<String, EgressNodeMetadata>::new();
    let mut ordered_nodes = Vec::<String>::new();

    for pool in multipath_config.node_pools.iter().filter(|pool| pool.enabled) {
        for node in pool.nodes.iter().filter(|node| node.enabled) {
            let should_include = requested_nodes
                .as_ref()
                .map(|nodes| nodes.contains(&node.name))
                .unwrap_or(true);

            if !should_include {
                continue;
            }

            if !metadata_index.contains_key(&node.name) {
                ordered_nodes.push(node.name.clone());
            }

            metadata_index.entry(node.name.clone()).or_insert_with(|| EgressNodeMetadata {
                name: node.name.clone(),
                server: Some(node.server.clone()),
                pool_name: Some(pool.name.clone()),
                pool_type: Some(format!("{:?}", pool.pool_type)),
                ip_type: None,
                fraud_score: None,
            });
        }
    }

    if ctx.available_nodes.is_empty() {
        ctx.available_nodes = ordered_nodes;
    }

    for node_name in &ctx.available_nodes {
        metadata_index
            .entry(node_name.clone())
            .or_insert_with(|| EgressNodeMetadata {
                name: node_name.clone(),
                ..Default::default()
            });
    }

    let ip_reputation_manager = crate::cmd::ip_reputation::get_ip_reputation_manager();

    for node_name in &ctx.available_nodes {
        if let Some(metadata) = metadata_index.get_mut(node_name) {
            if let Some(server) = metadata.server.clone() {
                if let Some(server_ip) = resolve_server_ip(&server).await {
                    match ip_reputation_manager.inspect_ip_metadata(&server_ip).await {
                        Ok(reputation) => {
                            metadata.ip_type = Some(reputation.ip_type);
                            metadata.fraud_score = Some(reputation.fraud_score);
                        }
                        Err(error) => {
                            log::warn!(
                                "[EgressIdentity] 检测节点 {} 的 IP 元数据失败: {}",
                                node_name,
                                error
                            );
                        }
                    }
                } else {
                    log::warn!(
                        "[EgressIdentity] 无法解析节点 {} 的 server 地址 {}",
                        node_name,
                        server
                    );
                }
            }
        }
    }

    ctx.available_node_metadata = ctx
        .available_nodes
        .iter()
        .filter_map(|node_name| metadata_index.get(node_name).cloned())
        .collect::<Vec<_>>();

    ctx
}

#[tauri::command]
pub async fn egress_identity_preview_match(
    process_name: Option<String>,
    exe_path: Option<String>,
    shortcut_id: Option<String>,
    domain: Option<String>,
    source_ip: Option<String>,
    source_port: Option<u16>,
    available_nodes: Option<Vec<String>>,
) -> Result<ResolvedEgressIdentity, String> {
    let _ = sync_coordinator_from_advanced_config();
    let ctx = enrich_egress_selection_context(EgressSelectionContext {
        shortcut_id,
        process_name,
        exe_path,
        domain,
        source_ip,
        source_port,
        available_nodes: available_nodes.unwrap_or_default(),
        ..Default::default()
    })
    .await;

    get_coordinator()
        .egress_identity_manager()
        .preview_match(ctx)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn egress_identity_assign_match(
    process_name: Option<String>,
    exe_path: Option<String>,
    shortcut_id: Option<String>,
    domain: Option<String>,
    source_ip: Option<String>,
    source_port: Option<u16>,
    available_nodes: Option<Vec<String>>,
) -> Result<ResolvedEgressIdentity, String> {
    let _ = sync_coordinator_from_advanced_config();
    let ctx = enrich_egress_selection_context(EgressSelectionContext {
        shortcut_id,
        process_name,
        exe_path,
        domain,
        source_ip,
        source_port,
        available_nodes: available_nodes.unwrap_or_default(),
        ..Default::default()
    })
    .await;

    get_coordinator()
        .egress_identity_manager()
        .assign(ctx)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn egress_identity_get_active_assignments() -> Result<Vec<ResolvedEgressIdentity>, String> {
    let _ = sync_coordinator_from_advanced_config();

    Ok(get_coordinator()
        .egress_identity_manager()
        .get_active_assignments())
}

#[tauri::command]
pub fn egress_identity_clear_assignment(key: String) -> Result<(), String> {
    get_coordinator()
        .egress_identity_manager()
        .clear_assignment(&key);
    Ok(())
}
