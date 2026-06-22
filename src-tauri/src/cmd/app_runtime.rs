use super::{CmdResult, StringifyErr as _};
use crate::core::app_runtime::{
    AppPolicyBinding, AppRegistryEntry, AppRuntimeControlPlaneCompletionReport, AppRuntimeDiagnosticsReport,
    AppRuntimeDnsHandoffReport, AppRuntimeMihomoProjection, AppRuntimePlan, AppRuntimePlanRequest,
    AppRuntimeProjectionActivationPreflightReport, AppRuntimeProjectionActivationPreflightRequest,
    AppRuntimeProjectionArtifact, AppRuntimeProjectionRuntimeApplyAuditRecord, AppRuntimeProjectionRuntimeApplyRequest,
    AppRuntimeProjectionRuntimePostApplyHoldReport, AppRuntimeProjectionRuntimeVerificationCloseoutRecord,
    AppRuntimeProjectionRuntimeVerificationCloseoutReport, AppRuntimeProjectionRuntimeVerificationReport,
    AppRuntimeProjectionRuntimeVerificationRequest, AppRuntimeRuntimeApplyBoundaryDecisionReport,
    AppRuntimeRuntimeApplyBoundaryDecisionRequest, AppRuntimeSessionEvaluationReport, AppRuntimeSessionFinishRequest,
    AppRuntimeSessionLeakReport, AppRuntimeSessionRecord, AppRuntimeSessionStartReport,
    AppRuntimeStagedActivationCloseoutReport, AppRuntimeStagedActivationLifecycleReport, AppRuntimeStateDocument,
    DnsProfile, NodePool, RustAdapterEgressParityReport, SecurityProfile,
    accept_app_runtime_dns_handoff as accept_app_runtime_dns_handoff_record,
    activate_app_runtime_projection_artifact as activate_app_runtime_projection_artifact_record,
    apply_app_runtime_projection_artifact_to_runtime as apply_app_runtime_projection_artifact_to_runtime_record,
    build_app_runtime_demo_seed_document,
    build_app_runtime_projection_artifact as build_app_runtime_projection_artifact_record,
    build_app_runtime_projection_runtime_post_apply_hold as build_app_runtime_projection_runtime_post_apply_hold_record,
    closeout_app_runtime_projection_runtime_apply_verification as closeout_app_runtime_projection_runtime_apply_verification_record,
    closeout_app_runtime_staged_activation_lifecycle as closeout_app_runtime_staged_activation_lifecycle_record,
    complete_app_runtime_control_plane as complete_app_runtime_control_plane_record,
    complete_app_runtime_staged_activation_lifecycle as complete_app_runtime_staged_activation_lifecycle_record,
    decide_app_runtime_runtime_apply_boundary as decide_app_runtime_runtime_apply_boundary_record,
    delete_app_policy_binding as delete_app_policy_binding_record,
    delete_app_registry_entry as delete_app_registry_entry_record, delete_dns_profile as delete_dns_profile_record,
    delete_node_pool as delete_node_pool_record, delete_security_profile as delete_security_profile_record,
    diagnose_app_runtime as build_app_runtime_diagnostics,
    evaluate_app_runtime_session as evaluate_app_runtime_session_record,
    explain_app_runtime_plan as build_app_runtime_plan,
    finish_app_runtime_session as finish_app_runtime_session_record,
    list_app_runtime_projection_runtime_apply_audits as list_app_runtime_projection_runtime_apply_audit_records,
    list_app_runtime_projection_runtime_verification_closeouts as list_app_runtime_projection_runtime_verification_closeout_records,
    list_app_runtime_sessions as list_app_runtime_session_records,
    persist_app_runtime_projection_artifact as persist_app_runtime_projection_artifact_record,
    preflight_app_runtime_projection_activation as preflight_app_runtime_projection_activation_record,
    project_app_runtime_plan_to_mihomo as build_app_runtime_mihomo_projection, read_app_runtime_state_document,
    record_app_runtime_session_observation as record_app_runtime_session_observation_record,
    rollback_app_runtime_projection_activation as rollback_app_runtime_projection_activation_record,
    rust_adapter_egress_parity as rust_adapter_egress_parity_record,
    rust_adapter_egress_parity_rollback as rust_adapter_egress_parity_rollback_record,
    start_app_runtime_session as start_app_runtime_session_record,
    upsert_app_policy_binding as upsert_app_policy_binding_record,
    upsert_app_registry_entry as upsert_app_registry_entry_record, upsert_dns_profile as upsert_dns_profile_record,
    upsert_node_pool as upsert_node_pool_record, upsert_security_profile as upsert_security_profile_record,
    verify_app_runtime_projection_runtime_apply as verify_app_runtime_projection_runtime_apply_record,
    verify_app_runtime_session_leak as verify_app_runtime_session_leak_record,
};

#[tauri::command]
pub async fn get_app_runtime_state() -> CmdResult<AppRuntimeStateDocument> {
    read_app_runtime_state_document().await.stringify_err()
}

#[tauri::command]
pub async fn build_app_runtime_demo_seed() -> CmdResult<AppRuntimeStateDocument> {
    Ok(build_app_runtime_demo_seed_document())
}

#[tauri::command]
pub async fn accept_app_runtime_dns_handoff() -> CmdResult<AppRuntimeDnsHandoffReport> {
    accept_app_runtime_dns_handoff_record().await.stringify_err()
}

#[tauri::command]
pub async fn complete_app_runtime_control_plane(
    request: AppRuntimePlanRequest,
) -> CmdResult<AppRuntimeControlPlaneCompletionReport> {
    complete_app_runtime_control_plane_record(request).await.stringify_err()
}

#[tauri::command]
pub async fn complete_app_runtime_staged_activation_lifecycle(
    request: AppRuntimePlanRequest,
) -> CmdResult<AppRuntimeStagedActivationLifecycleReport> {
    complete_app_runtime_staged_activation_lifecycle_record(request)
        .await
        .stringify_err()
}

#[tauri::command]
pub async fn closeout_app_runtime_staged_activation_lifecycle(
    request: AppRuntimePlanRequest,
) -> CmdResult<AppRuntimeStagedActivationCloseoutReport> {
    closeout_app_runtime_staged_activation_lifecycle_record(request)
        .await
        .stringify_err()
}

#[tauri::command]
pub async fn decide_app_runtime_runtime_apply_boundary(
    request: AppRuntimeRuntimeApplyBoundaryDecisionRequest,
) -> CmdResult<AppRuntimeRuntimeApplyBoundaryDecisionReport> {
    decide_app_runtime_runtime_apply_boundary_record(request)
        .await
        .stringify_err()
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
pub async fn rust_adapter_egress_parity(
    request: AppRuntimePlanRequest,
    explicit_opt_in: bool,
    apply_runtime: bool,
) -> CmdResult<RustAdapterEgressParityReport> {
    rust_adapter_egress_parity_record(request, explicit_opt_in, apply_runtime)
        .await
        .stringify_err()
}

#[tauri::command]
pub async fn rust_adapter_egress_parity_rollback() -> CmdResult<RustAdapterEgressParityReport> {
    rust_adapter_egress_parity_rollback_record().await.stringify_err()
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
pub async fn apply_app_runtime_projection_artifact_to_runtime(
    request: AppRuntimeProjectionRuntimeApplyRequest,
) -> CmdResult<AppRuntimeStateDocument> {
    apply_app_runtime_projection_artifact_to_runtime_record(request)
        .await
        .stringify_err()
}

#[tauri::command]
pub async fn list_app_runtime_projection_runtime_apply_audits(
    artifact_id: Option<String>,
) -> CmdResult<Vec<AppRuntimeProjectionRuntimeApplyAuditRecord>> {
    list_app_runtime_projection_runtime_apply_audit_records(artifact_id.map(Into::into))
        .await
        .stringify_err()
}

#[tauri::command]
pub async fn verify_app_runtime_projection_runtime_apply(
    request: AppRuntimeProjectionRuntimeVerificationRequest,
) -> CmdResult<AppRuntimeProjectionRuntimeVerificationReport> {
    verify_app_runtime_projection_runtime_apply_record(request)
        .await
        .stringify_err()
}

#[tauri::command]
pub async fn closeout_app_runtime_projection_runtime_apply_verification(
    request: AppRuntimeProjectionRuntimeVerificationRequest,
) -> CmdResult<AppRuntimeProjectionRuntimeVerificationCloseoutReport> {
    closeout_app_runtime_projection_runtime_apply_verification_record(request)
        .await
        .stringify_err()
}

#[tauri::command]
pub async fn list_app_runtime_projection_runtime_verification_closeouts(
    artifact_id: Option<String>,
) -> CmdResult<Vec<AppRuntimeProjectionRuntimeVerificationCloseoutRecord>> {
    list_app_runtime_projection_runtime_verification_closeout_records(artifact_id.map(Into::into))
        .await
        .stringify_err()
}

#[tauri::command]
pub async fn build_app_runtime_projection_runtime_post_apply_hold(
    request: AppRuntimeProjectionRuntimeVerificationRequest,
) -> CmdResult<AppRuntimeProjectionRuntimePostApplyHoldReport> {
    build_app_runtime_projection_runtime_post_apply_hold_record(request)
        .await
        .stringify_err()
}

#[tauri::command]
pub async fn rollback_app_runtime_projection_activation() -> CmdResult<AppRuntimeStateDocument> {
    rollback_app_runtime_projection_activation_record()
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
