use crate::core::egress_identity::{EgressIdentityConfig, ResolvedEgressIdentity};

use super::{CmdResult, StringifyErr};

#[tauri::command]
pub fn egress_identity_get_config() -> CmdResult<EgressIdentityConfig> {
    Ok(crate::feat::egress_identity_get_config())
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
) -> CmdResult<ResolvedEgressIdentity> {
    crate::feat::egress_identity_preview_match(
        process_name, exe_path, shortcut_id, domain, source_ip, source_port, available_nodes,
    )
    .await
    .stringify_err()
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
) -> CmdResult<ResolvedEgressIdentity> {
    crate::feat::egress_identity_assign_match(
        process_name, exe_path, shortcut_id, domain, source_ip, source_port, available_nodes,
    )
    .await
    .stringify_err()
}

#[tauri::command]
pub fn egress_identity_get_active_assignments() -> CmdResult<Vec<ResolvedEgressIdentity>> {
    Ok(crate::feat::egress_identity_get_active_assignments())
}

#[tauri::command]
pub fn egress_identity_clear_assignment(key: String) -> CmdResult<()> {
    crate::feat::egress_identity_clear_assignment(&key);
    Ok(())
}
