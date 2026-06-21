use super::*;

#[tauri::command]

pub async fn get_runtime_kernel_replacement_readiness() -> CmdResult<KernelReplacementReadiness> {
    Ok(mihomo_kernel_replacement_readiness().await)
}

#[tauri::command]
pub async fn get_runtime_kernel_apply_preflight(
    artifact_id: Option<String>,
) -> CmdResult<KernelRuntimePreflightReport> {
    Ok(mihomo_kernel_apply_preflight(artifact_id).await)
}

#[tauri::command]
pub async fn get_runtime_kernel_shadow_components() -> CmdResult<KernelShadowComponentsReport> {
    Ok(mihomo_kernel_shadow_components().await)
}

#[tauri::command]
pub async fn get_runtime_kernel_rust_runtime_candidate() -> CmdResult<RustKernelRuntimeCandidateReport> {
    Ok(rust_kernel_runtime_candidate_report().await)
}

#[tauri::command]
pub async fn get_runtime_kernel_runtime_selection_scaffold(
    requested_runtime_kind: Option<String>,
    rust_runtime_opt_in_decision: Option<bool>,
) -> CmdResult<KernelRuntimeSelectionScaffoldReport> {
    Ok(kernel_runtime_selection_scaffold(requested_runtime_kind, rust_runtime_opt_in_decision).await)
}

#[tauri::command]
pub async fn get_runtime_kernel_dns_shadow_evidence(
    yaml: Option<String>,
    domain: Option<String>,
) -> CmdResult<KernelDnsShadowEvidenceReport> {
    mihomo_kernel_dns_shadow_evidence(yaml, domain).await.stringify_err()
}

#[tauri::command]
pub async fn get_runtime_kernel_rule_shadow_evidence() -> CmdResult<KernelRuleShadowEvidenceReport> {
    mihomo_kernel_rule_shadow_evidence().await.stringify_err()
}

#[tauri::command]
pub async fn get_runtime_kernel_adapter_capability_report() -> CmdResult<KernelAdapterCapabilityReport> {
    mihomo_kernel_adapter_capability_report().await.stringify_err()
}

#[tauri::command]
pub async fn get_runtime_kernel_connection_session_shadow() -> CmdResult<KernelConnectionSessionShadowReport> {
    mihomo_kernel_connection_session_shadow().await.stringify_err()
}

#[tauri::command]
pub async fn get_runtime_kernel_isolated_listener_preflight(
    port: Option<u16>,
) -> CmdResult<KernelIsolatedListenerPreflightReport> {
    mihomo_kernel_isolated_listener_preflight(port).await.stringify_err()
}

#[tauri::command]
pub async fn get_runtime_kernel_loopback_dns_preflight(
    port: Option<u16>,
) -> CmdResult<KernelLoopbackDnsPreflightReport> {
    mihomo_kernel_loopback_dns_preflight(port).await.stringify_err()
}

#[tauri::command]
pub async fn get_runtime_kernel_loopback_dns_smoke_evidence(
    port: Option<u16>,
) -> CmdResult<KernelLoopbackDnsSmokeEvidenceReport> {
    mihomo_kernel_loopback_dns_smoke_evidence(port).await.stringify_err()
}

#[tauri::command]
pub async fn get_runtime_kernel_loopback_forwarding_preflight(
    listener_port: Option<u16>,
    target_port: Option<u16>,
) -> CmdResult<KernelLoopbackForwardingPreflightReport> {
    mihomo_kernel_loopback_forwarding_preflight(listener_port, target_port)
        .await
        .stringify_err()
}

#[tauri::command]
pub async fn get_runtime_kernel_loopback_forwarding_smoke_evidence(
    listener_port: Option<u16>,
    target_port: Option<u16>,
) -> CmdResult<KernelLoopbackForwardingSmokeEvidenceReport> {
    mihomo_kernel_loopback_forwarding_smoke_evidence(listener_port, target_port)
        .await
        .stringify_err()
}

#[tauri::command]
pub async fn get_runtime_kernel_loopback_forwarding_rollback_drill(
    listener_port: Option<u16>,
    target_port: Option<u16>,
) -> CmdResult<KernelLoopbackForwardingRollbackDrillReport> {
    mihomo_kernel_loopback_forwarding_rollback_drill(listener_port, target_port)
        .await
        .stringify_err()
}

#[tauri::command]
pub async fn get_runtime_kernel_loopback_forwarding_leak_check(
    listener_port: Option<u16>,
    target_port: Option<u16>,
) -> CmdResult<KernelLoopbackForwardingLeakCheckReport> {
    mihomo_kernel_loopback_forwarding_leak_check(listener_port, target_port)
        .await
        .stringify_err()
}

#[tauri::command]
pub async fn get_runtime_kernel_loopback_platform_matrix(
    listener_port: Option<u16>,
    target_port: Option<u16>,
) -> CmdResult<KernelLoopbackPlatformMatrixReport> {
    mihomo_kernel_loopback_platform_matrix(listener_port, target_port)
        .await
        .stringify_err()
}

#[tauri::command]
pub async fn get_runtime_kernel_loopback_hold_window(
    listener_port: Option<u16>,
    target_port: Option<u16>,
    hold_started_at_epoch_ms: Option<u64>,
) -> CmdResult<KernelLoopbackHoldWindowReport> {
    mihomo_kernel_loopback_hold_window(listener_port, target_port, hold_started_at_epoch_ms)
        .await
        .stringify_err()
}

#[tauri::command]
pub async fn get_runtime_kernel_loopback_platform_rollback_drills(
    listener_port: Option<u16>,
    target_port: Option<u16>,
    hold_started_at_epoch_ms: Option<u64>,
) -> CmdResult<KernelLoopbackPlatformRollbackDrillsReport> {
    mihomo_kernel_loopback_platform_rollback_drills(listener_port, target_port, hold_started_at_epoch_ms)
        .await
        .stringify_err()
}

#[tauri::command]
pub async fn get_runtime_kernel_loopback_r4_expanded_opt_in_preflight(
    listener_port: Option<u16>,
    target_port: Option<u16>,
    hold_started_at_epoch_ms: Option<u64>,
    observed_rollback_platforms: Option<Vec<String>>,
    explicit_decision: Option<bool>,
) -> CmdResult<KernelLoopbackR4ExpandedOptInPreflightReport> {
    mihomo_kernel_loopback_r4_expanded_opt_in_preflight(
        listener_port,
        target_port,
        hold_started_at_epoch_ms,
        observed_rollback_platforms,
        explicit_decision,
    )
    .await
    .stringify_err()
}

#[tauri::command]
pub async fn get_runtime_kernel_loopback_r4_expanded_opt_in_execution_plan(
    listener_port: Option<u16>,
    target_port: Option<u16>,
    hold_started_at_epoch_ms: Option<u64>,
    observed_rollback_platforms: Option<Vec<String>>,
    explicit_decision: Option<bool>,
) -> CmdResult<KernelLoopbackR4ExpandedOptInExecutionPlanReport> {
    mihomo_kernel_loopback_r4_expanded_opt_in_execution_plan(
        listener_port,
        target_port,
        hold_started_at_epoch_ms,
        observed_rollback_platforms,
        explicit_decision,
    )
    .await
    .stringify_err()
}

#[tauri::command]
pub async fn get_runtime_kernel_loopback_r4_expanded_opt_in_execution_guard(
    listener_port: Option<u16>,
    target_port: Option<u16>,
    hold_started_at_epoch_ms: Option<u64>,
    observed_rollback_platforms: Option<Vec<String>>,
    explicit_decision: Option<bool>,
    requested_execution: Option<bool>,
) -> CmdResult<KernelLoopbackR4ExpandedOptInExecutionGuardReport> {
    mihomo_kernel_loopback_r4_expanded_opt_in_execution_guard(
        listener_port,
        target_port,
        hold_started_at_epoch_ms,
        observed_rollback_platforms,
        explicit_decision,
        requested_execution,
    )
    .await
    .stringify_err()
}

#[tauri::command]
pub async fn get_runtime_kernel_loopback_r4_expanded_opt_in_synthetic_execution(
    listener_port: Option<u16>,
    target_port: Option<u16>,
    hold_started_at_epoch_ms: Option<u64>,
    observed_rollback_platforms: Option<Vec<String>>,
    explicit_decision: Option<bool>,
    requested_execution: Option<bool>,
) -> CmdResult<KernelLoopbackR4ExpandedOptInSyntheticExecutionReport> {
    mihomo_kernel_loopback_r4_expanded_opt_in_synthetic_execution(
        listener_port,
        target_port,
        hold_started_at_epoch_ms,
        observed_rollback_platforms,
        explicit_decision,
        requested_execution,
    )
    .await
    .stringify_err()
}

#[tauri::command]
pub async fn get_runtime_kernel_loopback_r4_expanded_opt_in_post_execution_hold(
    listener_port: Option<u16>,
    target_port: Option<u16>,
    hold_started_at_epoch_ms: Option<u64>,
    observed_rollback_platforms: Option<Vec<String>>,
    explicit_decision: Option<bool>,
    requested_execution: Option<bool>,
    post_execution_hold_started_at_epoch_ms: Option<u64>,
) -> CmdResult<KernelLoopbackR4ExpandedOptInPostExecutionHoldReport> {
    mihomo_kernel_loopback_r4_expanded_opt_in_post_execution_hold(
        listener_port,
        target_port,
        hold_started_at_epoch_ms,
        observed_rollback_platforms,
        explicit_decision,
        requested_execution,
        post_execution_hold_started_at_epoch_ms,
    )
    .await
    .stringify_err()
}

#[tauri::command]
pub async fn get_runtime_kernel_loopback_r4_expanded_opt_in_decision_readiness(
    listener_port: Option<u16>,
    target_port: Option<u16>,
    hold_started_at_epoch_ms: Option<u64>,
    observed_rollback_platforms: Option<Vec<String>>,
    explicit_decision: Option<bool>,
    requested_execution: Option<bool>,
    post_execution_hold_started_at_epoch_ms: Option<u64>,
    wider_opt_in_decision: Option<bool>,
) -> CmdResult<KernelLoopbackR4ExpandedOptInDecisionReadinessReport> {
    mihomo_kernel_loopback_r4_expanded_opt_in_decision_readiness(
        listener_port,
        target_port,
        hold_started_at_epoch_ms,
        observed_rollback_platforms,
        explicit_decision,
        requested_execution,
        post_execution_hold_started_at_epoch_ms,
        wider_opt_in_decision,
    )
    .await
    .stringify_err()
}

#[tauri::command]
pub async fn get_runtime_kernel_loopback_r4_expanded_opt_in_limited_rollout_gate(
    listener_port: Option<u16>,
    target_port: Option<u16>,
    hold_started_at_epoch_ms: Option<u64>,
    observed_rollback_platforms: Option<Vec<String>>,
    explicit_decision: Option<bool>,
    requested_execution: Option<bool>,
    post_execution_hold_started_at_epoch_ms: Option<u64>,
    wider_opt_in_decision: Option<bool>,
    limited_rollout_decision: Option<bool>,
    canary_scope: Option<String>,
    max_canary_sessions: Option<u16>,
) -> CmdResult<KernelLoopbackR4ExpandedOptInLimitedRolloutGateReport> {
    mihomo_kernel_loopback_r4_expanded_opt_in_limited_rollout_gate(
        listener_port,
        target_port,
        hold_started_at_epoch_ms,
        observed_rollback_platforms,
        explicit_decision,
        requested_execution,
        post_execution_hold_started_at_epoch_ms,
        wider_opt_in_decision,
        limited_rollout_decision,
        canary_scope,
        max_canary_sessions,
    )
    .await
    .stringify_err()
}

#[tauri::command]
pub async fn get_runtime_kernel_loopback_r4_expanded_opt_in_rollout_audit(
    listener_port: Option<u16>,
    target_port: Option<u16>,
    hold_started_at_epoch_ms: Option<u64>,
    observed_rollback_platforms: Option<Vec<String>>,
    explicit_decision: Option<bool>,
    requested_execution: Option<bool>,
    post_execution_hold_started_at_epoch_ms: Option<u64>,
    wider_opt_in_decision: Option<bool>,
    limited_rollout_decision: Option<bool>,
    canary_scope: Option<String>,
    max_canary_sessions: Option<u16>,
) -> CmdResult<KernelLoopbackR4ExpandedOptInRolloutAuditReport> {
    mihomo_kernel_loopback_r4_expanded_opt_in_rollout_audit(
        listener_port,
        target_port,
        hold_started_at_epoch_ms,
        observed_rollback_platforms,
        explicit_decision,
        requested_execution,
        post_execution_hold_started_at_epoch_ms,
        wider_opt_in_decision,
        limited_rollout_decision,
        canary_scope,
        max_canary_sessions,
    )
    .await
    .stringify_err()
}

#[tauri::command]
pub async fn get_runtime_kernel_loopback_r4_expanded_opt_in_closeout_readiness(
    listener_port: Option<u16>,
    target_port: Option<u16>,
    hold_started_at_epoch_ms: Option<u64>,
    observed_rollback_platforms: Option<Vec<String>>,
    explicit_decision: Option<bool>,
    requested_execution: Option<bool>,
    post_execution_hold_started_at_epoch_ms: Option<u64>,
    wider_opt_in_decision: Option<bool>,
    limited_rollout_decision: Option<bool>,
    canary_scope: Option<String>,
    max_canary_sessions: Option<u16>,
    closeout_decision: Option<bool>,
) -> CmdResult<KernelLoopbackR4ExpandedOptInCloseoutReadinessReport> {
    mihomo_kernel_loopback_r4_expanded_opt_in_closeout_readiness(
        listener_port,
        target_port,
        hold_started_at_epoch_ms,
        observed_rollback_platforms,
        explicit_decision,
        requested_execution,
        post_execution_hold_started_at_epoch_ms,
        wider_opt_in_decision,
        limited_rollout_decision,
        canary_scope,
        max_canary_sessions,
        closeout_decision,
    )
    .await
    .stringify_err()
}

#[tauri::command]
pub async fn get_runtime_kernel_loopback_r4_expanded_opt_in_closeout_report(
    listener_port: Option<u16>,
    target_port: Option<u16>,
    hold_started_at_epoch_ms: Option<u64>,
    observed_rollback_platforms: Option<Vec<String>>,
    explicit_decision: Option<bool>,
    requested_execution: Option<bool>,
    post_execution_hold_started_at_epoch_ms: Option<u64>,
    wider_opt_in_decision: Option<bool>,
    limited_rollout_decision: Option<bool>,
    canary_scope: Option<String>,
    max_canary_sessions: Option<u16>,
    closeout_decision: Option<bool>,
) -> CmdResult<KernelLoopbackR4ExpandedOptInCloseoutReport> {
    mihomo_kernel_loopback_r4_expanded_opt_in_closeout_report(
        listener_port,
        target_port,
        hold_started_at_epoch_ms,
        observed_rollback_platforms,
        explicit_decision,
        requested_execution,
        post_execution_hold_started_at_epoch_ms,
        wider_opt_in_decision,
        limited_rollout_decision,
        canary_scope,
        max_canary_sessions,
        closeout_decision,
    )
    .await
    .stringify_err()
}

#[tauri::command]
pub async fn get_runtime_kernel_loopback_r4_expanded_opt_in_completion_summary(
    listener_port: Option<u16>,
    target_port: Option<u16>,
    hold_started_at_epoch_ms: Option<u64>,
    observed_rollback_platforms: Option<Vec<String>>,
    explicit_decision: Option<bool>,
    requested_execution: Option<bool>,
    post_execution_hold_started_at_epoch_ms: Option<u64>,
    wider_opt_in_decision: Option<bool>,
    limited_rollout_decision: Option<bool>,
    canary_scope: Option<String>,
    max_canary_sessions: Option<u16>,
    closeout_decision: Option<bool>,
) -> CmdResult<KernelLoopbackR4ExpandedOptInCompletionReport> {
    mihomo_kernel_loopback_r4_expanded_opt_in_completion_summary(
        listener_port,
        target_port,
        hold_started_at_epoch_ms,
        observed_rollback_platforms,
        explicit_decision,
        requested_execution,
        post_execution_hold_started_at_epoch_ms,
        wider_opt_in_decision,
        limited_rollout_decision,
        canary_scope,
        max_canary_sessions,
        closeout_decision,
    )
    .await
    .stringify_err()
}

#[tauri::command]
pub async fn get_runtime_kernel_loopback_r4_expanded_opt_in_next_phase_handoff(
    listener_port: Option<u16>,
    target_port: Option<u16>,
    hold_started_at_epoch_ms: Option<u64>,
    observed_rollback_platforms: Option<Vec<String>>,
    explicit_decision: Option<bool>,
    requested_execution: Option<bool>,
    post_execution_hold_started_at_epoch_ms: Option<u64>,
    wider_opt_in_decision: Option<bool>,
    limited_rollout_decision: Option<bool>,
    canary_scope: Option<String>,
    max_canary_sessions: Option<u16>,
    closeout_decision: Option<bool>,
    handoff_decision: Option<bool>,
) -> CmdResult<KernelLoopbackR4ExpandedOptInNextPhaseHandoffReport> {
    mihomo_kernel_loopback_r4_expanded_opt_in_next_phase_handoff(
        listener_port,
        target_port,
        hold_started_at_epoch_ms,
        observed_rollback_platforms,
        explicit_decision,
        requested_execution,
        post_execution_hold_started_at_epoch_ms,
        wider_opt_in_decision,
        limited_rollout_decision,
        canary_scope,
        max_canary_sessions,
        closeout_decision,
        handoff_decision,
    )
    .await
    .stringify_err()
}

#[tauri::command]
pub async fn get_runtime_kernel_loopback_r5_default_cutover_preflight(
    listener_port: Option<u16>,
    target_port: Option<u16>,
    hold_started_at_epoch_ms: Option<u64>,
    observed_rollback_platforms: Option<Vec<String>>,
    explicit_decision: Option<bool>,
    requested_execution: Option<bool>,
    post_execution_hold_started_at_epoch_ms: Option<u64>,
    wider_opt_in_decision: Option<bool>,
    limited_rollout_decision: Option<bool>,
    canary_scope: Option<String>,
    max_canary_sessions: Option<u16>,
    closeout_decision: Option<bool>,
    handoff_decision: Option<bool>,
    r5_preflight_decision: Option<bool>,
) -> CmdResult<KernelLoopbackR5DefaultCutoverPreflightReport> {
    mihomo_kernel_loopback_r5_default_cutover_preflight(
        listener_port,
        target_port,
        hold_started_at_epoch_ms,
        observed_rollback_platforms,
        explicit_decision,
        requested_execution,
        post_execution_hold_started_at_epoch_ms,
        wider_opt_in_decision,
        limited_rollout_decision,
        canary_scope,
        max_canary_sessions,
        closeout_decision,
        handoff_decision,
        r5_preflight_decision,
    )
    .await
    .stringify_err()
}

#[tauri::command]
pub async fn get_runtime_kernel_loopback_r5_default_cutover_risk_matrix(
    listener_port: Option<u16>,
    target_port: Option<u16>,
    hold_started_at_epoch_ms: Option<u64>,
    observed_rollback_platforms: Option<Vec<String>>,
    explicit_decision: Option<bool>,
    requested_execution: Option<bool>,
    post_execution_hold_started_at_epoch_ms: Option<u64>,
    wider_opt_in_decision: Option<bool>,
    limited_rollout_decision: Option<bool>,
    canary_scope: Option<String>,
    max_canary_sessions: Option<u16>,
    closeout_decision: Option<bool>,
    handoff_decision: Option<bool>,
    r5_preflight_decision: Option<bool>,
) -> CmdResult<KernelLoopbackR5DefaultCutoverRiskMatrixReport> {
    mihomo_kernel_loopback_r5_default_cutover_risk_matrix(
        listener_port,
        target_port,
        hold_started_at_epoch_ms,
        observed_rollback_platforms,
        explicit_decision,
        requested_execution,
        post_execution_hold_started_at_epoch_ms,
        wider_opt_in_decision,
        limited_rollout_decision,
        canary_scope,
        max_canary_sessions,
        closeout_decision,
        handoff_decision,
        r5_preflight_decision,
    )
    .await
    .stringify_err()
}

#[tauri::command]
pub async fn get_runtime_kernel_loopback_r5_default_cutover_rollback_abort_plan(
    listener_port: Option<u16>,
    target_port: Option<u16>,
    hold_started_at_epoch_ms: Option<u64>,
    observed_rollback_platforms: Option<Vec<String>>,
    explicit_decision: Option<bool>,
    requested_execution: Option<bool>,
    post_execution_hold_started_at_epoch_ms: Option<u64>,
    wider_opt_in_decision: Option<bool>,
    limited_rollout_decision: Option<bool>,
    canary_scope: Option<String>,
    max_canary_sessions: Option<u16>,
    closeout_decision: Option<bool>,
    handoff_decision: Option<bool>,
    r5_preflight_decision: Option<bool>,
    rollback_plan_decision: Option<bool>,
) -> CmdResult<KernelLoopbackR5DefaultCutoverRollbackAbortPlanReport> {
    mihomo_kernel_loopback_r5_default_cutover_rollback_abort_plan(
        listener_port,
        target_port,
        hold_started_at_epoch_ms,
        observed_rollback_platforms,
        explicit_decision,
        requested_execution,
        post_execution_hold_started_at_epoch_ms,
        wider_opt_in_decision,
        limited_rollout_decision,
        canary_scope,
        max_canary_sessions,
        closeout_decision,
        handoff_decision,
        r5_preflight_decision,
        rollback_plan_decision,
    )
    .await
    .stringify_err()
}

#[tauri::command]
pub async fn get_runtime_kernel_loopback_r5_default_cutover_execution_plan(
    listener_port: Option<u16>,
    target_port: Option<u16>,
    hold_started_at_epoch_ms: Option<u64>,
    observed_rollback_platforms: Option<Vec<String>>,
    explicit_decision: Option<bool>,
    requested_execution: Option<bool>,
    post_execution_hold_started_at_epoch_ms: Option<u64>,
    wider_opt_in_decision: Option<bool>,
    limited_rollout_decision: Option<bool>,
    canary_scope: Option<String>,
    max_canary_sessions: Option<u16>,
    closeout_decision: Option<bool>,
    handoff_decision: Option<bool>,
    r5_preflight_decision: Option<bool>,
    rollback_plan_decision: Option<bool>,
    execution_plan_decision: Option<bool>,
) -> CmdResult<KernelLoopbackR5DefaultCutoverExecutionPlanReport> {
    mihomo_kernel_loopback_r5_default_cutover_execution_plan(
        listener_port,
        target_port,
        hold_started_at_epoch_ms,
        observed_rollback_platforms,
        explicit_decision,
        requested_execution,
        post_execution_hold_started_at_epoch_ms,
        wider_opt_in_decision,
        limited_rollout_decision,
        canary_scope,
        max_canary_sessions,
        closeout_decision,
        handoff_decision,
        r5_preflight_decision,
        rollback_plan_decision,
        execution_plan_decision,
    )
    .await
    .stringify_err()
}

#[tauri::command]
pub async fn get_runtime_kernel_loopback_r5_default_cutover_guard(
    listener_port: Option<u16>,
    target_port: Option<u16>,
    hold_started_at_epoch_ms: Option<u64>,
    observed_rollback_platforms: Option<Vec<String>>,
    explicit_decision: Option<bool>,
    requested_execution: Option<bool>,
    post_execution_hold_started_at_epoch_ms: Option<u64>,
    wider_opt_in_decision: Option<bool>,
    limited_rollout_decision: Option<bool>,
    canary_scope: Option<String>,
    max_canary_sessions: Option<u16>,
    closeout_decision: Option<bool>,
    handoff_decision: Option<bool>,
    r5_preflight_decision: Option<bool>,
    rollback_plan_decision: Option<bool>,
    execution_plan_decision: Option<bool>,
    guard_decision: Option<bool>,
) -> CmdResult<KernelLoopbackR5DefaultCutoverGuardReport> {
    mihomo_kernel_loopback_r5_default_cutover_guard(
        listener_port,
        target_port,
        hold_started_at_epoch_ms,
        observed_rollback_platforms,
        explicit_decision,
        requested_execution,
        post_execution_hold_started_at_epoch_ms,
        wider_opt_in_decision,
        limited_rollout_decision,
        canary_scope,
        max_canary_sessions,
        closeout_decision,
        handoff_decision,
        r5_preflight_decision,
        rollback_plan_decision,
        execution_plan_decision,
        guard_decision,
    )
    .await
    .stringify_err()
}

#[tauri::command]
pub async fn get_runtime_kernel_loopback_r5_default_cutover_dry_run_readiness(
    listener_port: Option<u16>,
    target_port: Option<u16>,
    hold_started_at_epoch_ms: Option<u64>,
    observed_rollback_platforms: Option<Vec<String>>,
    explicit_decision: Option<bool>,
    requested_execution: Option<bool>,
    post_execution_hold_started_at_epoch_ms: Option<u64>,
    wider_opt_in_decision: Option<bool>,
    limited_rollout_decision: Option<bool>,
    canary_scope: Option<String>,
    max_canary_sessions: Option<u16>,
    closeout_decision: Option<bool>,
    handoff_decision: Option<bool>,
    r5_preflight_decision: Option<bool>,
    rollback_plan_decision: Option<bool>,
    execution_plan_decision: Option<bool>,
    guard_decision: Option<bool>,
    dry_run_decision: Option<bool>,
) -> CmdResult<KernelLoopbackR5DefaultCutoverDryRunReadinessReport> {
    mihomo_kernel_loopback_r5_default_cutover_dry_run_readiness(
        listener_port,
        target_port,
        hold_started_at_epoch_ms,
        observed_rollback_platforms,
        explicit_decision,
        requested_execution,
        post_execution_hold_started_at_epoch_ms,
        wider_opt_in_decision,
        limited_rollout_decision,
        canary_scope,
        max_canary_sessions,
        closeout_decision,
        handoff_decision,
        r5_preflight_decision,
        rollback_plan_decision,
        execution_plan_decision,
        guard_decision,
        dry_run_decision,
    )
    .await
    .stringify_err()
}

#[tauri::command]
pub async fn get_runtime_kernel_loopback_r5_default_cutover_dry_run_evidence(
    listener_port: Option<u16>,
    target_port: Option<u16>,
    hold_started_at_epoch_ms: Option<u64>,
    observed_rollback_platforms: Option<Vec<String>>,
    explicit_decision: Option<bool>,
    requested_execution: Option<bool>,
    post_execution_hold_started_at_epoch_ms: Option<u64>,
    wider_opt_in_decision: Option<bool>,
    limited_rollout_decision: Option<bool>,
    canary_scope: Option<String>,
    max_canary_sessions: Option<u16>,
    closeout_decision: Option<bool>,
    handoff_decision: Option<bool>,
    r5_preflight_decision: Option<bool>,
    rollback_plan_decision: Option<bool>,
    execution_plan_decision: Option<bool>,
    guard_decision: Option<bool>,
    dry_run_decision: Option<bool>,
    dry_run_execution_decision: Option<bool>,
) -> CmdResult<KernelLoopbackR5DefaultCutoverDryRunEvidenceReport> {
    mihomo_kernel_loopback_r5_default_cutover_dry_run_evidence(
        listener_port,
        target_port,
        hold_started_at_epoch_ms,
        observed_rollback_platforms,
        explicit_decision,
        requested_execution,
        post_execution_hold_started_at_epoch_ms,
        wider_opt_in_decision,
        limited_rollout_decision,
        canary_scope,
        max_canary_sessions,
        closeout_decision,
        handoff_decision,
        r5_preflight_decision,
        rollback_plan_decision,
        execution_plan_decision,
        guard_decision,
        dry_run_decision,
        dry_run_execution_decision,
    )
    .await
    .stringify_err()
}

#[tauri::command]
pub async fn get_runtime_kernel_loopback_r5_default_cutover_dry_run_closeout(
    listener_port: Option<u16>,
    target_port: Option<u16>,
    hold_started_at_epoch_ms: Option<u64>,
    observed_rollback_platforms: Option<Vec<String>>,
    explicit_decision: Option<bool>,
    requested_execution: Option<bool>,
    post_execution_hold_started_at_epoch_ms: Option<u64>,
    wider_opt_in_decision: Option<bool>,
    limited_rollout_decision: Option<bool>,
    canary_scope: Option<String>,
    max_canary_sessions: Option<u16>,
    closeout_decision: Option<bool>,
    handoff_decision: Option<bool>,
    r5_preflight_decision: Option<bool>,
    rollback_plan_decision: Option<bool>,
    execution_plan_decision: Option<bool>,
    guard_decision: Option<bool>,
    dry_run_decision: Option<bool>,
    dry_run_execution_decision: Option<bool>,
) -> CmdResult<KernelLoopbackR5DefaultCutoverDryRunCloseoutReport> {
    mihomo_kernel_loopback_r5_default_cutover_dry_run_closeout(
        listener_port,
        target_port,
        hold_started_at_epoch_ms,
        observed_rollback_platforms,
        explicit_decision,
        requested_execution,
        post_execution_hold_started_at_epoch_ms,
        wider_opt_in_decision,
        limited_rollout_decision,
        canary_scope,
        max_canary_sessions,
        closeout_decision,
        handoff_decision,
        r5_preflight_decision,
        rollback_plan_decision,
        execution_plan_decision,
        guard_decision,
        dry_run_decision,
        dry_run_execution_decision,
    )
    .await
    .stringify_err()
}

#[tauri::command]
pub async fn get_runtime_kernel_loopback_r5_default_cutover_post_dry_run_hold(
    listener_port: Option<u16>,
    target_port: Option<u16>,
    hold_started_at_epoch_ms: Option<u64>,
    observed_rollback_platforms: Option<Vec<String>>,
    explicit_decision: Option<bool>,
    requested_execution: Option<bool>,
    post_execution_hold_started_at_epoch_ms: Option<u64>,
    wider_opt_in_decision: Option<bool>,
    limited_rollout_decision: Option<bool>,
    canary_scope: Option<String>,
    max_canary_sessions: Option<u16>,
    closeout_decision: Option<bool>,
    handoff_decision: Option<bool>,
    r5_preflight_decision: Option<bool>,
    rollback_plan_decision: Option<bool>,
    execution_plan_decision: Option<bool>,
    guard_decision: Option<bool>,
    dry_run_decision: Option<bool>,
    dry_run_execution_decision: Option<bool>,
    post_dry_run_hold_started_at_epoch_ms: Option<u64>,
    hold_decision: Option<bool>,
) -> CmdResult<KernelLoopbackR5DefaultCutoverPostDryRunHoldReport> {
    mihomo_kernel_loopback_r5_default_cutover_post_dry_run_hold(
        listener_port,
        target_port,
        hold_started_at_epoch_ms,
        observed_rollback_platforms,
        explicit_decision,
        requested_execution,
        post_execution_hold_started_at_epoch_ms,
        wider_opt_in_decision,
        limited_rollout_decision,
        canary_scope,
        max_canary_sessions,
        closeout_decision,
        handoff_decision,
        r5_preflight_decision,
        rollback_plan_decision,
        execution_plan_decision,
        guard_decision,
        dry_run_decision,
        dry_run_execution_decision,
        post_dry_run_hold_started_at_epoch_ms,
        hold_decision,
    )
    .await
    .stringify_err()
}

#[tauri::command]
pub async fn get_runtime_kernel_loopback_r5_default_cutover_decision_readiness(
    listener_port: Option<u16>,
    target_port: Option<u16>,
    hold_started_at_epoch_ms: Option<u64>,
    observed_rollback_platforms: Option<Vec<String>>,
    explicit_decision: Option<bool>,
    requested_execution: Option<bool>,
    post_execution_hold_started_at_epoch_ms: Option<u64>,
    wider_opt_in_decision: Option<bool>,
    limited_rollout_decision: Option<bool>,
    canary_scope: Option<String>,
    max_canary_sessions: Option<u16>,
    closeout_decision: Option<bool>,
    handoff_decision: Option<bool>,
    r5_preflight_decision: Option<bool>,
    rollback_plan_decision: Option<bool>,
    execution_plan_decision: Option<bool>,
    guard_decision: Option<bool>,
    dry_run_decision: Option<bool>,
    dry_run_execution_decision: Option<bool>,
    post_dry_run_hold_started_at_epoch_ms: Option<u64>,
    hold_decision: Option<bool>,
    decision_readiness_decision: Option<bool>,
) -> CmdResult<KernelLoopbackR5DefaultCutoverDecisionReadinessReport> {
    mihomo_kernel_loopback_r5_default_cutover_decision_readiness(
        listener_port,
        target_port,
        hold_started_at_epoch_ms,
        observed_rollback_platforms,
        explicit_decision,
        requested_execution,
        post_execution_hold_started_at_epoch_ms,
        wider_opt_in_decision,
        limited_rollout_decision,
        canary_scope,
        max_canary_sessions,
        closeout_decision,
        handoff_decision,
        r5_preflight_decision,
        rollback_plan_decision,
        execution_plan_decision,
        guard_decision,
        dry_run_decision,
        dry_run_execution_decision,
        post_dry_run_hold_started_at_epoch_ms,
        hold_decision,
        decision_readiness_decision,
    )
    .await
    .stringify_err()
}

#[tauri::command]
pub async fn get_runtime_kernel_loopback_r5_default_cutover_final_gate(
    listener_port: Option<u16>,
    target_port: Option<u16>,
    hold_started_at_epoch_ms: Option<u64>,
    observed_rollback_platforms: Option<Vec<String>>,
    explicit_decision: Option<bool>,
    requested_execution: Option<bool>,
    post_execution_hold_started_at_epoch_ms: Option<u64>,
    wider_opt_in_decision: Option<bool>,
    limited_rollout_decision: Option<bool>,
    canary_scope: Option<String>,
    max_canary_sessions: Option<u16>,
    closeout_decision: Option<bool>,
    handoff_decision: Option<bool>,
    r5_preflight_decision: Option<bool>,
    rollback_plan_decision: Option<bool>,
    execution_plan_decision: Option<bool>,
    guard_decision: Option<bool>,
    dry_run_decision: Option<bool>,
    dry_run_execution_decision: Option<bool>,
    post_dry_run_hold_started_at_epoch_ms: Option<u64>,
    hold_decision: Option<bool>,
    decision_readiness_decision: Option<bool>,
    final_gate_decision: Option<bool>,
) -> CmdResult<KernelLoopbackR5DefaultCutoverFinalGateReport> {
    mihomo_kernel_loopback_r5_default_cutover_final_gate(
        listener_port,
        target_port,
        hold_started_at_epoch_ms,
        observed_rollback_platforms,
        explicit_decision,
        requested_execution,
        post_execution_hold_started_at_epoch_ms,
        wider_opt_in_decision,
        limited_rollout_decision,
        canary_scope,
        max_canary_sessions,
        closeout_decision,
        handoff_decision,
        r5_preflight_decision,
        rollback_plan_decision,
        execution_plan_decision,
        guard_decision,
        dry_run_decision,
        dry_run_execution_decision,
        post_dry_run_hold_started_at_epoch_ms,
        hold_decision,
        decision_readiness_decision,
        final_gate_decision,
    )
    .await
    .stringify_err()
}

#[tauri::command]
pub async fn get_runtime_kernel_loopback_r5_default_cutover_next_step_handoff(
    listener_port: Option<u16>,
    target_port: Option<u16>,
    hold_started_at_epoch_ms: Option<u64>,
    observed_rollback_platforms: Option<Vec<String>>,
    explicit_decision: Option<bool>,
    requested_execution: Option<bool>,
    post_execution_hold_started_at_epoch_ms: Option<u64>,
    wider_opt_in_decision: Option<bool>,
    limited_rollout_decision: Option<bool>,
    canary_scope: Option<String>,
    max_canary_sessions: Option<u16>,
    closeout_decision: Option<bool>,
    handoff_decision: Option<bool>,
    r5_preflight_decision: Option<bool>,
    rollback_plan_decision: Option<bool>,
    execution_plan_decision: Option<bool>,
    guard_decision: Option<bool>,
    dry_run_decision: Option<bool>,
    dry_run_execution_decision: Option<bool>,
    post_dry_run_hold_started_at_epoch_ms: Option<u64>,
    hold_decision: Option<bool>,
    decision_readiness_decision: Option<bool>,
    final_gate_decision: Option<bool>,
    r5_handoff_decision: Option<bool>,
) -> CmdResult<KernelLoopbackR5DefaultCutoverNextStepHandoffReport> {
    mihomo_kernel_loopback_r5_default_cutover_next_step_handoff(
        listener_port,
        target_port,
        hold_started_at_epoch_ms,
        observed_rollback_platforms,
        explicit_decision,
        requested_execution,
        post_execution_hold_started_at_epoch_ms,
        wider_opt_in_decision,
        limited_rollout_decision,
        canary_scope,
        max_canary_sessions,
        closeout_decision,
        handoff_decision,
        r5_preflight_decision,
        rollback_plan_decision,
        execution_plan_decision,
        guard_decision,
        dry_run_decision,
        dry_run_execution_decision,
        post_dry_run_hold_started_at_epoch_ms,
        hold_decision,
        decision_readiness_decision,
        final_gate_decision,
        r5_handoff_decision,
    )
    .await
    .stringify_err()
}

#[tauri::command]
pub async fn get_runtime_kernel_loopback_r5_default_cutover_final_hold(
    listener_port: Option<u16>,
    target_port: Option<u16>,
    hold_started_at_epoch_ms: Option<u64>,
    observed_rollback_platforms: Option<Vec<String>>,
    explicit_decision: Option<bool>,
    requested_execution: Option<bool>,
    post_execution_hold_started_at_epoch_ms: Option<u64>,
    wider_opt_in_decision: Option<bool>,
    limited_rollout_decision: Option<bool>,
    canary_scope: Option<String>,
    max_canary_sessions: Option<u16>,
    closeout_decision: Option<bool>,
    handoff_decision: Option<bool>,
    r5_preflight_decision: Option<bool>,
    rollback_plan_decision: Option<bool>,
    execution_plan_decision: Option<bool>,
    guard_decision: Option<bool>,
    dry_run_decision: Option<bool>,
    dry_run_execution_decision: Option<bool>,
    post_dry_run_hold_started_at_epoch_ms: Option<u64>,
    hold_decision: Option<bool>,
    decision_readiness_decision: Option<bool>,
    final_gate_decision: Option<bool>,
    r5_handoff_decision: Option<bool>,
    final_hold_started_at_epoch_ms: Option<u64>,
    final_hold_decision: Option<bool>,
) -> CmdResult<KernelLoopbackR5DefaultCutoverFinalHoldReport> {
    mihomo_kernel_loopback_r5_default_cutover_final_hold(
        listener_port,
        target_port,
        hold_started_at_epoch_ms,
        observed_rollback_platforms,
        explicit_decision,
        requested_execution,
        post_execution_hold_started_at_epoch_ms,
        wider_opt_in_decision,
        limited_rollout_decision,
        canary_scope,
        max_canary_sessions,
        closeout_decision,
        handoff_decision,
        r5_preflight_decision,
        rollback_plan_decision,
        execution_plan_decision,
        guard_decision,
        dry_run_decision,
        dry_run_execution_decision,
        post_dry_run_hold_started_at_epoch_ms,
        hold_decision,
        decision_readiness_decision,
        final_gate_decision,
        r5_handoff_decision,
        final_hold_started_at_epoch_ms,
        final_hold_decision,
    )
    .await
    .stringify_err()
}

#[tauri::command]
pub async fn get_runtime_kernel_loopback_r5_default_cutover_independent_rollback_validation(
    listener_port: Option<u16>,
    target_port: Option<u16>,
    hold_started_at_epoch_ms: Option<u64>,
    observed_rollback_platforms: Option<Vec<String>>,
    explicit_decision: Option<bool>,
    requested_execution: Option<bool>,
    post_execution_hold_started_at_epoch_ms: Option<u64>,
    wider_opt_in_decision: Option<bool>,
    limited_rollout_decision: Option<bool>,
    canary_scope: Option<String>,
    max_canary_sessions: Option<u16>,
    closeout_decision: Option<bool>,
    handoff_decision: Option<bool>,
    r5_preflight_decision: Option<bool>,
    rollback_plan_decision: Option<bool>,
    execution_plan_decision: Option<bool>,
    guard_decision: Option<bool>,
    dry_run_decision: Option<bool>,
    dry_run_execution_decision: Option<bool>,
    post_dry_run_hold_started_at_epoch_ms: Option<u64>,
    hold_decision: Option<bool>,
    decision_readiness_decision: Option<bool>,
    final_gate_decision: Option<bool>,
    r5_handoff_decision: Option<bool>,
    final_hold_started_at_epoch_ms: Option<u64>,
    final_hold_decision: Option<bool>,
    independent_rollback_decision: Option<bool>,
) -> CmdResult<KernelLoopbackR5DefaultCutoverIndependentRollbackValidationReport> {
    mihomo_kernel_loopback_r5_default_cutover_independent_rollback_validation(
        listener_port,
        target_port,
        hold_started_at_epoch_ms,
        observed_rollback_platforms,
        explicit_decision,
        requested_execution,
        post_execution_hold_started_at_epoch_ms,
        wider_opt_in_decision,
        limited_rollout_decision,
        canary_scope,
        max_canary_sessions,
        closeout_decision,
        handoff_decision,
        r5_preflight_decision,
        rollback_plan_decision,
        execution_plan_decision,
        guard_decision,
        dry_run_decision,
        dry_run_execution_decision,
        post_dry_run_hold_started_at_epoch_ms,
        hold_decision,
        decision_readiness_decision,
        final_gate_decision,
        r5_handoff_decision,
        final_hold_started_at_epoch_ms,
        final_hold_decision,
        independent_rollback_decision,
    )
    .await
    .stringify_err()
}

#[tauri::command]
pub async fn get_runtime_kernel_loopback_r5_default_cutover_closeout_readiness(
    listener_port: Option<u16>,
    target_port: Option<u16>,
    hold_started_at_epoch_ms: Option<u64>,
    observed_rollback_platforms: Option<Vec<String>>,
    explicit_decision: Option<bool>,
    requested_execution: Option<bool>,
    post_execution_hold_started_at_epoch_ms: Option<u64>,
    wider_opt_in_decision: Option<bool>,
    limited_rollout_decision: Option<bool>,
    canary_scope: Option<String>,
    max_canary_sessions: Option<u16>,
    closeout_decision: Option<bool>,
    handoff_decision: Option<bool>,
    r5_preflight_decision: Option<bool>,
    rollback_plan_decision: Option<bool>,
    execution_plan_decision: Option<bool>,
    guard_decision: Option<bool>,
    dry_run_decision: Option<bool>,
    dry_run_execution_decision: Option<bool>,
    post_dry_run_hold_started_at_epoch_ms: Option<u64>,
    hold_decision: Option<bool>,
    decision_readiness_decision: Option<bool>,
    final_gate_decision: Option<bool>,
    r5_handoff_decision: Option<bool>,
    final_hold_started_at_epoch_ms: Option<u64>,
    final_hold_decision: Option<bool>,
    independent_rollback_decision: Option<bool>,
    r5_closeout_decision: Option<bool>,
) -> CmdResult<KernelLoopbackR5DefaultCutoverCloseoutReadinessReport> {
    mihomo_kernel_loopback_r5_default_cutover_closeout_readiness(
        listener_port,
        target_port,
        hold_started_at_epoch_ms,
        observed_rollback_platforms,
        explicit_decision,
        requested_execution,
        post_execution_hold_started_at_epoch_ms,
        wider_opt_in_decision,
        limited_rollout_decision,
        canary_scope,
        max_canary_sessions,
        closeout_decision,
        handoff_decision,
        r5_preflight_decision,
        rollback_plan_decision,
        execution_plan_decision,
        guard_decision,
        dry_run_decision,
        dry_run_execution_decision,
        post_dry_run_hold_started_at_epoch_ms,
        hold_decision,
        decision_readiness_decision,
        final_gate_decision,
        r5_handoff_decision,
        final_hold_started_at_epoch_ms,
        final_hold_decision,
        independent_rollback_decision,
        r5_closeout_decision,
    )
    .await
    .stringify_err()
}

#[tauri::command]
pub async fn get_runtime_kernel_loopback_r5_default_cutover_closeout_report(
    listener_port: Option<u16>,
    target_port: Option<u16>,
    hold_started_at_epoch_ms: Option<u64>,
    observed_rollback_platforms: Option<Vec<String>>,
    explicit_decision: Option<bool>,
    requested_execution: Option<bool>,
    post_execution_hold_started_at_epoch_ms: Option<u64>,
    wider_opt_in_decision: Option<bool>,
    limited_rollout_decision: Option<bool>,
    canary_scope: Option<String>,
    max_canary_sessions: Option<u16>,
    closeout_decision: Option<bool>,
    handoff_decision: Option<bool>,
    r5_preflight_decision: Option<bool>,
    rollback_plan_decision: Option<bool>,
    execution_plan_decision: Option<bool>,
    guard_decision: Option<bool>,
    dry_run_decision: Option<bool>,
    dry_run_execution_decision: Option<bool>,
    post_dry_run_hold_started_at_epoch_ms: Option<u64>,
    hold_decision: Option<bool>,
    decision_readiness_decision: Option<bool>,
    final_gate_decision: Option<bool>,
    r5_handoff_decision: Option<bool>,
    final_hold_started_at_epoch_ms: Option<u64>,
    final_hold_decision: Option<bool>,
    independent_rollback_decision: Option<bool>,
    r5_closeout_decision: Option<bool>,
    r5_closeout_report_decision: Option<bool>,
) -> CmdResult<KernelLoopbackR5DefaultCutoverCloseoutReport> {
    mihomo_kernel_loopback_r5_default_cutover_closeout_report(
        listener_port,
        target_port,
        hold_started_at_epoch_ms,
        observed_rollback_platforms,
        explicit_decision,
        requested_execution,
        post_execution_hold_started_at_epoch_ms,
        wider_opt_in_decision,
        limited_rollout_decision,
        canary_scope,
        max_canary_sessions,
        closeout_decision,
        handoff_decision,
        r5_preflight_decision,
        rollback_plan_decision,
        execution_plan_decision,
        guard_decision,
        dry_run_decision,
        dry_run_execution_decision,
        post_dry_run_hold_started_at_epoch_ms,
        hold_decision,
        decision_readiness_decision,
        final_gate_decision,
        r5_handoff_decision,
        final_hold_started_at_epoch_ms,
        final_hold_decision,
        independent_rollback_decision,
        r5_closeout_decision,
        r5_closeout_report_decision,
    )
    .await
    .stringify_err()
}

#[tauri::command]
pub async fn get_runtime_kernel_loopback_r5_closeout_r6_rust_runtime_scaffold(
    listener_port: Option<u16>,
    target_port: Option<u16>,
    hold_started_at_epoch_ms: Option<u64>,
    observed_rollback_platforms: Option<Vec<String>>,
    explicit_decision: Option<bool>,
    requested_execution: Option<bool>,
    post_execution_hold_started_at_epoch_ms: Option<u64>,
    wider_opt_in_decision: Option<bool>,
    limited_rollout_decision: Option<bool>,
    canary_scope: Option<String>,
    max_canary_sessions: Option<u16>,
    closeout_decision: Option<bool>,
    handoff_decision: Option<bool>,
    r5_preflight_decision: Option<bool>,
    rollback_plan_decision: Option<bool>,
    execution_plan_decision: Option<bool>,
    guard_decision: Option<bool>,
    dry_run_decision: Option<bool>,
    dry_run_execution_decision: Option<bool>,
    post_dry_run_hold_started_at_epoch_ms: Option<u64>,
    hold_decision: Option<bool>,
    decision_readiness_decision: Option<bool>,
    final_gate_decision: Option<bool>,
    r5_handoff_decision: Option<bool>,
    final_hold_started_at_epoch_ms: Option<u64>,
    final_hold_decision: Option<bool>,
    independent_rollback_decision: Option<bool>,
    r5_closeout_decision: Option<bool>,
    r5_closeout_report_decision: Option<bool>,
    requested_runtime_kind: Option<String>,
    rust_runtime_opt_in_decision: Option<bool>,
    rust_runtime_scaffold_decision: Option<bool>,
) -> CmdResult<KernelLoopbackR5CloseoutR6RustRuntimeScaffoldReport> {
    Box::pin(mihomo_kernel_loopback_r5_closeout_r6_rust_runtime_scaffold(
        listener_port,
        target_port,
        hold_started_at_epoch_ms,
        observed_rollback_platforms,
        explicit_decision,
        requested_execution,
        post_execution_hold_started_at_epoch_ms,
        wider_opt_in_decision,
        limited_rollout_decision,
        canary_scope,
        max_canary_sessions,
        closeout_decision,
        handoff_decision,
        r5_preflight_decision,
        rollback_plan_decision,
        execution_plan_decision,
        guard_decision,
        dry_run_decision,
        dry_run_execution_decision,
        post_dry_run_hold_started_at_epoch_ms,
        hold_decision,
        decision_readiness_decision,
        final_gate_decision,
        r5_handoff_decision,
        final_hold_started_at_epoch_ms,
        final_hold_decision,
        independent_rollback_decision,
        r5_closeout_decision,
        r5_closeout_report_decision,
        requested_runtime_kind,
        rust_runtime_opt_in_decision,
        rust_runtime_scaffold_decision,
    ))
    .await
    .stringify_err()
}

#[tauri::command]
pub async fn get_runtime_kernel_loopback_r6_opt_in_rust_runtime_mvp(
    listener_port: Option<u16>,
    target_port: Option<u16>,
    hold_started_at_epoch_ms: Option<u64>,
    observed_rollback_platforms: Option<Vec<String>>,
    explicit_decision: Option<bool>,
    requested_execution: Option<bool>,
    post_execution_hold_started_at_epoch_ms: Option<u64>,
    wider_opt_in_decision: Option<bool>,
    limited_rollout_decision: Option<bool>,
    canary_scope: Option<String>,
    max_canary_sessions: Option<u16>,
    closeout_decision: Option<bool>,
    handoff_decision: Option<bool>,
    r5_preflight_decision: Option<bool>,
    rollback_plan_decision: Option<bool>,
    execution_plan_decision: Option<bool>,
    guard_decision: Option<bool>,
    dry_run_decision: Option<bool>,
    dry_run_execution_decision: Option<bool>,
    post_dry_run_hold_started_at_epoch_ms: Option<u64>,
    hold_decision: Option<bool>,
    decision_readiness_decision: Option<bool>,
    final_gate_decision: Option<bool>,
    r5_handoff_decision: Option<bool>,
    final_hold_started_at_epoch_ms: Option<u64>,
    final_hold_decision: Option<bool>,
    independent_rollback_decision: Option<bool>,
    r5_closeout_decision: Option<bool>,
    r5_closeout_report_decision: Option<bool>,
    requested_runtime_kind: Option<String>,
    rust_runtime_opt_in_decision: Option<bool>,
    rust_runtime_scaffold_decision: Option<bool>,
) -> CmdResult<KernelLoopbackR6OptInRustRuntimeMvpReport> {
    Box::pin(rust_kernel_runtime_r6_opt_in_mvp(
        listener_port,
        target_port,
        hold_started_at_epoch_ms,
        observed_rollback_platforms,
        explicit_decision,
        requested_execution,
        post_execution_hold_started_at_epoch_ms,
        wider_opt_in_decision,
        limited_rollout_decision,
        canary_scope,
        max_canary_sessions,
        closeout_decision,
        handoff_decision,
        r5_preflight_decision,
        rollback_plan_decision,
        execution_plan_decision,
        guard_decision,
        dry_run_decision,
        dry_run_execution_decision,
        post_dry_run_hold_started_at_epoch_ms,
        hold_decision,
        decision_readiness_decision,
        final_gate_decision,
        r5_handoff_decision,
        final_hold_started_at_epoch_ms,
        final_hold_decision,
        independent_rollback_decision,
        r5_closeout_decision,
        r5_closeout_report_decision,
        requested_runtime_kind,
        rust_runtime_opt_in_decision,
        rust_runtime_scaffold_decision,
    ))
    .await
    .stringify_err()
}

#[tauri::command]
pub async fn get_runtime_kernel_loopback_r6_rust_default_canary(
    listener_port: Option<u16>,
    target_port: Option<u16>,
    hold_started_at_epoch_ms: Option<u64>,
    observed_rollback_platforms: Option<Vec<String>>,
    explicit_decision: Option<bool>,
    requested_execution: Option<bool>,
    post_execution_hold_started_at_epoch_ms: Option<u64>,
    wider_opt_in_decision: Option<bool>,
    limited_rollout_decision: Option<bool>,
    canary_scope: Option<String>,
    max_canary_sessions: Option<u16>,
    closeout_decision: Option<bool>,
    handoff_decision: Option<bool>,
    r5_preflight_decision: Option<bool>,
    rollback_plan_decision: Option<bool>,
    execution_plan_decision: Option<bool>,
    guard_decision: Option<bool>,
    dry_run_decision: Option<bool>,
    dry_run_execution_decision: Option<bool>,
    post_dry_run_hold_started_at_epoch_ms: Option<u64>,
    hold_decision: Option<bool>,
    decision_readiness_decision: Option<bool>,
    final_gate_decision: Option<bool>,
    r5_handoff_decision: Option<bool>,
    final_hold_started_at_epoch_ms: Option<u64>,
    final_hold_decision: Option<bool>,
    independent_rollback_decision: Option<bool>,
    r5_closeout_decision: Option<bool>,
    r5_closeout_report_decision: Option<bool>,
    requested_runtime_kind: Option<String>,
    rust_runtime_opt_in_decision: Option<bool>,
    rust_runtime_scaffold_decision: Option<bool>,
    canary_default_decision: Option<bool>,
    health_check_passed: Option<bool>,
    rollback_triggered: Option<bool>,
) -> CmdResult<KernelLoopbackR6RustDefaultCanaryReport> {
    Box::pin(rust_kernel_runtime_r6_default_canary(
        listener_port,
        target_port,
        hold_started_at_epoch_ms,
        observed_rollback_platforms,
        explicit_decision,
        requested_execution,
        post_execution_hold_started_at_epoch_ms,
        wider_opt_in_decision,
        limited_rollout_decision,
        canary_scope,
        max_canary_sessions,
        closeout_decision,
        handoff_decision,
        r5_preflight_decision,
        rollback_plan_decision,
        execution_plan_decision,
        guard_decision,
        dry_run_decision,
        dry_run_execution_decision,
        post_dry_run_hold_started_at_epoch_ms,
        hold_decision,
        decision_readiness_decision,
        final_gate_decision,
        r5_handoff_decision,
        final_hold_started_at_epoch_ms,
        final_hold_decision,
        independent_rollback_decision,
        r5_closeout_decision,
        r5_closeout_report_decision,
        requested_runtime_kind,
        rust_runtime_opt_in_decision,
        rust_runtime_scaffold_decision,
        canary_default_decision,
        health_check_passed,
        rollback_triggered,
    ))
    .await
    .stringify_err()
}

#[tauri::command]
pub async fn get_runtime_kernel_loopback_r7_rust_default_cutover(
    listener_port: Option<u16>,
    target_port: Option<u16>,
    hold_started_at_epoch_ms: Option<u64>,
    observed_rollback_platforms: Option<Vec<String>>,
    explicit_decision: Option<bool>,
    requested_execution: Option<bool>,
    post_execution_hold_started_at_epoch_ms: Option<u64>,
    wider_opt_in_decision: Option<bool>,
    limited_rollout_decision: Option<bool>,
    canary_scope: Option<String>,
    max_canary_sessions: Option<u16>,
    closeout_decision: Option<bool>,
    handoff_decision: Option<bool>,
    r5_preflight_decision: Option<bool>,
    rollback_plan_decision: Option<bool>,
    execution_plan_decision: Option<bool>,
    guard_decision: Option<bool>,
    dry_run_decision: Option<bool>,
    dry_run_execution_decision: Option<bool>,
    post_dry_run_hold_started_at_epoch_ms: Option<u64>,
    hold_decision: Option<bool>,
    decision_readiness_decision: Option<bool>,
    final_gate_decision: Option<bool>,
    r5_handoff_decision: Option<bool>,
    final_hold_started_at_epoch_ms: Option<u64>,
    final_hold_decision: Option<bool>,
    independent_rollback_decision: Option<bool>,
    r5_closeout_decision: Option<bool>,
    r5_closeout_report_decision: Option<bool>,
    requested_runtime_kind: Option<String>,
    rust_runtime_opt_in_decision: Option<bool>,
    rust_runtime_scaffold_decision: Option<bool>,
    canary_default_decision: Option<bool>,
    health_check_passed: Option<bool>,
    rollback_triggered: Option<bool>,
    r7_cutover_decision: Option<bool>,
    rollback_hold_decision: Option<bool>,
    rollback_switch_requested: Option<bool>,
    profile_scope: Option<String>,
) -> CmdResult<KernelLoopbackR7RustDefaultCutoverReport> {
    Box::pin(rust_kernel_runtime_r7_default_cutover(
        listener_port,
        target_port,
        hold_started_at_epoch_ms,
        observed_rollback_platforms,
        explicit_decision,
        requested_execution,
        post_execution_hold_started_at_epoch_ms,
        wider_opt_in_decision,
        limited_rollout_decision,
        canary_scope,
        max_canary_sessions,
        closeout_decision,
        handoff_decision,
        r5_preflight_decision,
        rollback_plan_decision,
        execution_plan_decision,
        guard_decision,
        dry_run_decision,
        dry_run_execution_decision,
        post_dry_run_hold_started_at_epoch_ms,
        hold_decision,
        decision_readiness_decision,
        final_gate_decision,
        r5_handoff_decision,
        final_hold_started_at_epoch_ms,
        final_hold_decision,
        independent_rollback_decision,
        r5_closeout_decision,
        r5_closeout_report_decision,
        requested_runtime_kind,
        rust_runtime_opt_in_decision,
        rust_runtime_scaffold_decision,
        canary_default_decision,
        health_check_passed,
        rollback_triggered,
        r7_cutover_decision,
        rollback_hold_decision,
        rollback_switch_requested,
        profile_scope,
    ))
    .await
    .stringify_err()
}

#[tauri::command]
pub async fn get_runtime_kernel_loopback_r7_mihomo_fallback_retirement(
    listener_port: Option<u16>,
    target_port: Option<u16>,
    hold_started_at_epoch_ms: Option<u64>,
    observed_rollback_platforms: Option<Vec<String>>,
    explicit_decision: Option<bool>,
    requested_execution: Option<bool>,
    post_execution_hold_started_at_epoch_ms: Option<u64>,
    wider_opt_in_decision: Option<bool>,
    limited_rollout_decision: Option<bool>,
    canary_scope: Option<String>,
    max_canary_sessions: Option<u16>,
    closeout_decision: Option<bool>,
    handoff_decision: Option<bool>,
    r5_preflight_decision: Option<bool>,
    rollback_plan_decision: Option<bool>,
    execution_plan_decision: Option<bool>,
    guard_decision: Option<bool>,
    dry_run_decision: Option<bool>,
    dry_run_execution_decision: Option<bool>,
    post_dry_run_hold_started_at_epoch_ms: Option<u64>,
    hold_decision: Option<bool>,
    decision_readiness_decision: Option<bool>,
    final_gate_decision: Option<bool>,
    r5_handoff_decision: Option<bool>,
    final_hold_started_at_epoch_ms: Option<u64>,
    final_hold_decision: Option<bool>,
    independent_rollback_decision: Option<bool>,
    r5_closeout_decision: Option<bool>,
    r5_closeout_report_decision: Option<bool>,
    requested_runtime_kind: Option<String>,
    rust_runtime_opt_in_decision: Option<bool>,
    rust_runtime_scaffold_decision: Option<bool>,
    canary_default_decision: Option<bool>,
    health_check_passed: Option<bool>,
    rollback_triggered: Option<bool>,
    r7_cutover_decision: Option<bool>,
    rollback_hold_decision: Option<bool>,
    rollback_switch_requested: Option<bool>,
    profile_scope: Option<String>,
    protocol_parity_decision: Option<bool>,
    tun_parity_decision: Option<bool>,
    adapter_parity_decision: Option<bool>,
    dns_runtime_parity_decision: Option<bool>,
    cross_platform_rollback_decision: Option<bool>,
    soak_evidence_decision: Option<bool>,
    fallback_retirement_decision: Option<bool>,
    emergency_rollback_decision: Option<bool>,
) -> CmdResult<KernelLoopbackR7MihomoFallbackRetirementReport> {
    Box::pin(rust_kernel_runtime_r7_mihomo_fallback_retirement(
        listener_port,
        target_port,
        hold_started_at_epoch_ms,
        observed_rollback_platforms,
        explicit_decision,
        requested_execution,
        post_execution_hold_started_at_epoch_ms,
        wider_opt_in_decision,
        limited_rollout_decision,
        canary_scope,
        max_canary_sessions,
        closeout_decision,
        handoff_decision,
        r5_preflight_decision,
        rollback_plan_decision,
        execution_plan_decision,
        guard_decision,
        dry_run_decision,
        dry_run_execution_decision,
        post_dry_run_hold_started_at_epoch_ms,
        hold_decision,
        decision_readiness_decision,
        final_gate_decision,
        r5_handoff_decision,
        final_hold_started_at_epoch_ms,
        final_hold_decision,
        independent_rollback_decision,
        r5_closeout_decision,
        r5_closeout_report_decision,
        requested_runtime_kind,
        rust_runtime_opt_in_decision,
        rust_runtime_scaffold_decision,
        canary_default_decision,
        health_check_passed,
        rollback_triggered,
        r7_cutover_decision,
        rollback_hold_decision,
        rollback_switch_requested,
        profile_scope,
        protocol_parity_decision,
        tun_parity_decision,
        adapter_parity_decision,
        dns_runtime_parity_decision,
        cross_platform_rollback_decision,
        soak_evidence_decision,
        fallback_retirement_decision,
        emergency_rollback_decision,
    ))
    .await
    .stringify_err()
}

#[tauri::command]
pub async fn get_runtime_kernel_loopback_full_rust_runtime_hardening(
    r7_fallback_retirement_passed: Option<bool>,
    observed_soak_hours: Option<u32>,
    health_regression_count: Option<u32>,
    rollback_trigger_count: Option<u32>,
    rollback_event_count: Option<u32>,
    last_rollback_event_ts: Option<u64>,
    rollback_telemetry_decision: Option<bool>,
    emergency_rollback_decision: Option<bool>,
    windows_service_hardening_decision: Option<bool>,
    macos_service_hardening_decision: Option<bool>,
    linux_service_hardening_decision: Option<bool>,
    final_hardening_decision: Option<bool>,
) -> CmdResult<KernelLoopbackFullRustRuntimeHardeningReport> {
    rust_kernel_runtime_full_rust_runtime_hardening(
        r7_fallback_retirement_passed,
        observed_soak_hours,
        health_regression_count,
        rollback_trigger_count,
        rollback_event_count,
        last_rollback_event_ts,
        rollback_telemetry_decision,
        emergency_rollback_decision,
        windows_service_hardening_decision,
        macos_service_hardening_decision,
        linux_service_hardening_decision,
        final_hardening_decision,
    )
    .await
    .stringify_err()
}

#[tauri::command]
pub async fn get_runtime_kernel_isolated_test_listener_status() -> CmdResult<KernelIsolatedTestListenerStatus> {
    Ok(mihomo_kernel_isolated_test_listener_status().await)
}

#[tauri::command]
pub async fn get_runtime_kernel_isolated_test_listener_smoke_evidence(
    port: Option<u16>,
) -> CmdResult<KernelIsolatedTestListenerSmokeEvidenceReport> {
    mihomo_kernel_isolated_test_listener_smoke_evidence(port)
        .await
        .stringify_err()
}

#[tauri::command]
pub async fn start_runtime_kernel_isolated_test_listener(
    port: Option<u16>,
) -> CmdResult<KernelIsolatedTestListenerStatus> {
    mihomo_kernel_start_isolated_test_listener(port).await.stringify_err()
}

#[tauri::command]
pub async fn stop_runtime_kernel_isolated_test_listener() -> CmdResult<KernelIsolatedTestListenerStatus> {
    Ok(mihomo_kernel_stop_isolated_test_listener().await)
}
