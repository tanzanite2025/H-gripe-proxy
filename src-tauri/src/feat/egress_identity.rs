use crate::core::egress_identity::{EgressIdentityConfig, EgressSelectionContext, ResolvedEgressIdentity};
use crate::core::stable_egress::enrich_egress_selection_context as core_enrich_context;
use anyhow::Result;

/// feat 层 enrich_egress_selection_context
pub async fn enrich_egress_selection_context(ctx: EgressSelectionContext) -> EgressSelectionContext {
    let coordinator = crate::feat::get_coordinator();
    let ip_reputation_manager = crate::feat::get_ip_reputation_manager();
    core_enrich_context(ctx, &coordinator.multipath_manager(), &ip_reputation_manager).await
}

pub fn egress_identity_get_config() -> EgressIdentityConfig {
    let _ = crate::feat::sync_coordinator_from_advanced_config();
    crate::feat::get_coordinator().egress_identity_manager().get_config()
}

pub async fn egress_identity_preview_match(
    process_name: Option<String>,
    exe_path: Option<String>,
    shortcut_id: Option<String>,
    domain: Option<String>,
    source_ip: Option<String>,
    source_port: Option<u16>,
    available_nodes: Option<Vec<String>>,
) -> Result<ResolvedEgressIdentity> {
    let _ = crate::feat::sync_coordinator_from_advanced_config_async().await;
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

    crate::feat::get_coordinator()
        .egress_identity_manager()
        .preview_match(ctx)
}

pub async fn egress_identity_assign_match(
    process_name: Option<String>,
    exe_path: Option<String>,
    shortcut_id: Option<String>,
    domain: Option<String>,
    source_ip: Option<String>,
    source_port: Option<u16>,
    available_nodes: Option<Vec<String>>,
) -> Result<ResolvedEgressIdentity> {
    let _ = crate::feat::sync_coordinator_from_advanced_config_async().await;
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

    crate::feat::get_coordinator().egress_identity_manager().assign(ctx)
}

pub fn egress_identity_get_active_assignments() -> Vec<ResolvedEgressIdentity> {
    let _ = crate::feat::sync_coordinator_from_advanced_config();
    crate::feat::get_coordinator()
        .egress_identity_manager()
        .get_active_assignments()
}

pub fn egress_identity_clear_assignment(key: &str) {
    crate::feat::get_coordinator()
        .egress_identity_manager()
        .clear_assignment(key);
}
