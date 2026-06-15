use super::{CmdResult, StringifyErr as _};
use crate::core::app_runtime::{
    AppPolicyBinding, AppRegistryEntry, AppRuntimeMihomoProjection, AppRuntimePlan, AppRuntimePlanRequest,
    AppRuntimeStateDocument, DnsProfile, NodePool, delete_app_policy_binding as delete_app_policy_binding_record,
    delete_app_registry_entry as delete_app_registry_entry_record, delete_dns_profile as delete_dns_profile_record,
    delete_node_pool as delete_node_pool_record, explain_app_runtime_plan as build_app_runtime_plan,
    project_app_runtime_plan_to_mihomo as build_app_runtime_mihomo_projection, read_app_runtime_state_document,
    upsert_app_policy_binding as upsert_app_policy_binding_record,
    upsert_app_registry_entry as upsert_app_registry_entry_record, upsert_dns_profile as upsert_dns_profile_record,
    upsert_node_pool as upsert_node_pool_record,
};

#[tauri::command]
pub async fn get_app_runtime_state() -> CmdResult<AppRuntimeStateDocument> {
    read_app_runtime_state_document().await.stringify_err()
}

#[tauri::command]
pub async fn upsert_app_registry_entry(entry: AppRegistryEntry) -> CmdResult<AppRuntimeStateDocument> {
    upsert_app_registry_entry_record(entry).await.stringify_err()
}

#[tauri::command]
pub async fn delete_app_registry_entry(app_id: String) -> CmdResult<AppRuntimeStateDocument> {
    delete_app_registry_entry_record(app_id.as_str()).await.stringify_err()
}

#[tauri::command]
pub async fn upsert_node_pool(node_pool: NodePool) -> CmdResult<AppRuntimeStateDocument> {
    upsert_node_pool_record(node_pool).await.stringify_err()
}

#[tauri::command]
pub async fn delete_node_pool(pool_id: String) -> CmdResult<AppRuntimeStateDocument> {
    delete_node_pool_record(pool_id.as_str()).await.stringify_err()
}

#[tauri::command]
pub async fn upsert_dns_profile(dns_profile: DnsProfile) -> CmdResult<AppRuntimeStateDocument> {
    upsert_dns_profile_record(dns_profile).await.stringify_err()
}

#[tauri::command]
pub async fn delete_dns_profile(profile_id: String) -> CmdResult<AppRuntimeStateDocument> {
    delete_dns_profile_record(profile_id.as_str()).await.stringify_err()
}

#[tauri::command]
pub async fn upsert_app_policy_binding(binding: AppPolicyBinding) -> CmdResult<AppRuntimeStateDocument> {
    upsert_app_policy_binding_record(binding).await.stringify_err()
}

#[tauri::command]
pub async fn delete_app_policy_binding(binding_id: String) -> CmdResult<AppRuntimeStateDocument> {
    delete_app_policy_binding_record(binding_id.as_str())
        .await
        .stringify_err()
}

#[tauri::command]
pub async fn explain_app_runtime_plan(request: AppRuntimePlanRequest) -> CmdResult<AppRuntimePlan> {
    let state = read_app_runtime_state_document().await.stringify_err()?;
    Ok(build_app_runtime_plan(&state, request))
}

#[tauri::command]
pub async fn project_app_runtime_plan_to_mihomo(
    request: AppRuntimePlanRequest,
) -> CmdResult<AppRuntimeMihomoProjection> {
    let state = read_app_runtime_state_document().await.stringify_err()?;
    build_app_runtime_mihomo_projection(&state, request).stringify_err()
}
