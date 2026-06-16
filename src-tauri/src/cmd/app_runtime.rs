use super::{CmdResult, StringifyErr as _};
use crate::core::app_runtime::{
    AppPolicyBinding, AppRegistryEntry, AppRuntimeDiagnosticsReport, AppRuntimeMihomoProjection, AppRuntimePlan,
    AppRuntimePlanRequest, AppRuntimeProjectionActivationPreflightReport,
    AppRuntimeProjectionActivationPreflightRequest, AppRuntimeProjectionArtifact, AppRuntimeSessionEvaluationReport,
    AppRuntimeSessionFinishRequest, AppRuntimeSessionLeakReport, AppRuntimeSessionRecord, AppRuntimeSessionStartReport,
    AppRuntimeStateDocument, DnsProfile, NodePool, SecurityProfile,
    activate_app_runtime_projection_artifact as activate_app_runtime_projection_artifact_record,
    build_app_runtime_projection_artifact as build_app_runtime_projection_artifact_record,
    delete_app_policy_binding as delete_app_policy_binding_record,
    delete_app_registry_entry as delete_app_registry_entry_record, delete_dns_profile as delete_dns_profile_record,
    delete_node_pool as delete_node_pool_record, delete_security_profile as delete_security_profile_record,
    diagnose_app_runtime as build_app_runtime_diagnostics,
    evaluate_app_runtime_session as evaluate_app_runtime_session_record,
    explain_app_runtime_plan as build_app_runtime_plan,
    finish_app_runtime_session as finish_app_runtime_session_record,
    list_app_runtime_sessions as list_app_runtime_session_records,
    persist_app_runtime_projection_artifact as persist_app_runtime_projection_artifact_record,
    preflight_app_runtime_projection_activation as preflight_app_runtime_projection_activation_record,
    project_app_runtime_plan_to_mihomo as build_app_runtime_mihomo_projection, read_app_runtime_state_document,
    record_app_runtime_session_observation as record_app_runtime_session_observation_record,
    start_app_runtime_session as start_app_runtime_session_record,
    upsert_app_policy_binding as upsert_app_policy_binding_record,
    upsert_app_registry_entry as upsert_app_registry_entry_record, upsert_dns_profile as upsert_dns_profile_record,
    upsert_node_pool as upsert_node_pool_record, upsert_security_profile as upsert_security_profile_record,
    verify_app_runtime_session_leak as verify_app_runtime_session_leak_record,
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
pub async fn upsert_security_profile(security_profile: SecurityProfile) -> CmdResult<AppRuntimeStateDocument> {
    upsert_security_profile_record(security_profile).await.stringify_err()
}

#[tauri::command]
pub async fn delete_security_profile(profile_id: String) -> CmdResult<AppRuntimeStateDocument> {
    delete_security_profile_record(profile_id.as_str())
        .await
        .stringify_err()
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

#[tauri::command]
pub async fn diagnose_app_runtime(request: AppRuntimePlanRequest) -> CmdResult<AppRuntimeDiagnosticsReport> {
    let state = read_app_runtime_state_document().await.stringify_err()?;
    build_app_runtime_diagnostics(&state, request).stringify_err()
}

#[tauri::command]
pub async fn build_app_runtime_projection_artifact(
    request: AppRuntimePlanRequest,
) -> CmdResult<AppRuntimeProjectionArtifact> {
    let state = read_app_runtime_state_document().await.stringify_err()?;
    let mut artifact = build_app_runtime_projection_artifact_record(&state, request).stringify_err()?;
    artifact.storage_path = Some(
        persist_app_runtime_projection_artifact_record(&artifact)
            .await
            .stringify_err()?,
    );
    Ok(artifact)
}

#[tauri::command]
pub async fn preflight_app_runtime_projection_activation(
    request: AppRuntimeProjectionActivationPreflightRequest,
) -> CmdResult<AppRuntimeProjectionActivationPreflightReport> {
    preflight_app_runtime_projection_activation_record(request)
        .await
        .stringify_err()
}

#[tauri::command]
pub async fn activate_app_runtime_projection_artifact(
    request: AppRuntimeProjectionActivationPreflightRequest,
) -> CmdResult<AppRuntimeStateDocument> {
    activate_app_runtime_projection_artifact_record(request)
        .await
        .stringify_err()
}

#[tauri::command]
pub async fn list_app_runtime_sessions(app_id: Option<String>) -> CmdResult<Vec<AppRuntimeSessionRecord>> {
    list_app_runtime_session_records(app_id.map(Into::into))
        .await
        .stringify_err()
}

#[tauri::command]
pub async fn start_app_runtime_session(request: AppRuntimePlanRequest) -> CmdResult<AppRuntimeSessionStartReport> {
    start_app_runtime_session_record(request).await.stringify_err()
}

#[tauri::command]
pub async fn finish_app_runtime_session(request: AppRuntimeSessionFinishRequest) -> CmdResult<AppRuntimeSessionRecord> {
    finish_app_runtime_session_record(request).await.stringify_err()
}

#[tauri::command]
pub async fn record_app_runtime_session_observation(session_id: String) -> CmdResult<AppRuntimeSessionRecord> {
    record_app_runtime_session_observation_record(session_id.as_str())
        .await
        .stringify_err()
}

#[tauri::command]
pub async fn evaluate_app_runtime_session(session_id: String) -> CmdResult<AppRuntimeSessionEvaluationReport> {
    evaluate_app_runtime_session_record(session_id.as_str())
        .await
        .stringify_err()
}

#[tauri::command]
pub async fn verify_app_runtime_session_leak(session_id: String) -> CmdResult<AppRuntimeSessionLeakReport> {
    verify_app_runtime_session_leak_record(session_id.as_str())
        .await
        .stringify_err()
}
