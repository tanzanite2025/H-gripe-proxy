use crate::core::egress_identity::{
    EgressSelectionContext, ResolvedEgressIdentity,
};
use crate::core::stable_egress::enrich_egress_selection_context as core_enrich_context;

use super::coordinator::{get_coordinator, sync_coordinator_from_advanced_config};
use super::ip_reputation::get_ip_reputation_manager;

/// cmd 层薄包装：调用 core 层 enrich_egress_selection_context
pub async fn enrich_egress_selection_context(
    ctx: EgressSelectionContext,
) -> EgressSelectionContext {
    let coordinator = get_coordinator();
    let ip_reputation_manager = get_ip_reputation_manager();
    core_enrich_context(ctx, &coordinator.multipath_manager(), &ip_reputation_manager).await
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
