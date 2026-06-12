use crate::core::egress_identity::EgressSelectionContext;
use crate::core::session_affinity::{
    SessionAffinityManager, process_detection,
};
use anyhow::Result;
use std::sync::Arc;

fn session_affinity_manager() -> Arc<SessionAffinityManager> {
    crate::core::session_affinity::get_session_affinity_manager()
}

fn prioritize_available_nodes(available_nodes: Vec<String>, preferred_node: &str) -> Vec<String> {
    if let Some(index) = available_nodes.iter().position(|node| node == preferred_node) {
        let mut reordered = Vec::with_capacity(available_nodes.len());
        reordered.push(available_nodes[index].clone());
        reordered.extend(
            available_nodes
                .into_iter()
                .enumerate()
                .filter_map(|(current_index, node)| if current_index == index { None } else { Some(node) }),
        );
        reordered
    } else {
        available_nodes
    }
}

pub async fn session_affinity_select_node_for_domain(domain: &str, available_nodes: Vec<String>) -> Result<String> {
    let _ = crate::core::coordinator::sync_coordinator_from_advanced_config_async().await;
    let coordinator = crate::core::coordinator::get_coordinator();
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
    let selected_node = session_affinity_manager()
        .select_node_for_domain(domain, &ordered_nodes)
        .await?;

    let _ = coordinator
        .egress_identity_manager()
        .record_assignment(egress_context, selected_node.clone());

    Ok(selected_node)
}

pub async fn session_affinity_select_node_for_process(
    source_port: u16,
    available_nodes: Vec<String>,
) -> Result<String> {
    let _ = crate::core::coordinator::sync_coordinator_from_advanced_config_async().await;
    let process_name = process_detection::get_process_name_by_port(source_port).ok();
    let coordinator = crate::core::coordinator::get_coordinator();
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
    let selected_node = session_affinity_manager()
        .select_node_for_process(source_port, &ordered_nodes)
        .await?;

    if let Some(egress_context) = egress_context {
        let _ = coordinator
            .egress_identity_manager()
            .record_assignment(egress_context, selected_node.clone());
    }

    Ok(selected_node)
}

pub async fn session_affinity_select_node_for_connection(
    source_ip: &str,
    source_port: u16,
    available_nodes: Vec<String>,
) -> Result<String> {
    let _ = crate::core::coordinator::sync_coordinator_from_advanced_config_async().await;
    let coordinator = crate::core::coordinator::get_coordinator();
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
    let selected_node = session_affinity_manager()
        .select_node_for_connection(source_ip, source_port, &ordered_nodes)
        .await?;

    let _ = coordinator
        .egress_identity_manager()
        .record_assignment(egress_context, selected_node.clone());

    Ok(selected_node)
}
