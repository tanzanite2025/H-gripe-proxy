use super::*;
use crate::core::kernel_runtime::{
    GoToRustMigrationFinalReviewReport, MihomoFallbackRetirementExecutionReport, RustDefaultDataPlaneCloseoutReport,
    RustDefaultForwardingHoldBlockerReport, RustDnsCutoverHoldBlockerReport, RustDnsDefaultPathBlockerReport,
    RustEncryptedProtocolsBundleReport, RustEncryptedProxyProtocolPreflightReport,
    RustEncryptedProxySessionExpansionReport, RustFallbackRetirementReadinessLockReport,
    RustFallbackRetirementReadinessManifest, RustHttpConnectProxyAdapterReport,
    RustMihomoFallbackRetirementBundleReport, RustPluginProcessSupervisionBlockerReport,
    RustProtocolAdapterForwardingExpansionReport, RustProtocolDefaultPathBlockerReport,
    RustProtocolForwardingSubsetPreflightReport, RustProtocolForwardingSubsetSmokeEvidenceReport,
    RustProtocolForwardingSubsetStartReport, RustProtocolForwardingSubsetStatusReport,
    RustProtocolForwardingSubsetStopReport, RustQuicUdpProfileBlockerReport, RustRemoteAdapterTransportExpansionReport,
    RustRoutePacketCaptureBlockerReport, RustRuntimeRealCanaryEvidenceReport, RustShadowsocksAeadAdapterCanaryReport,
    RustShadowsocksAeadAdapterExecutionReport, RustSidecarIndependentRollbackReport, RustSocksAuthExecutionReport,
    RustSocksBindExecutionReport, RustSocksTcpConnectExecutionReport, RustSocksUdpAssociateExecutionReport,
    RustSocksUdpFragmentsExecutionReport, RustTunPacketCaptureHoldBundleReport, RustTunSystemProxyParityApplyReport,
    RustTunSystemProxyParityPreflightReport, RustTunSystemProxyParityRollbackReport,
    RustTunTransparentRoutingExecutionReport, RustUdpPluginTransportBundleReport, apply_rust_tun_system_proxy_parity,
    closeout_rust_default_data_plane, execute_mihomo_fallback_retirement, go_to_rust_migration_final_review,
    lock_rust_fallback_retirement_readiness, mihomo_fallback_retirement_execution_plan,
    rollback_mihomo_fallback_retirement_execution, rollback_rust_tun_system_proxy_parity,
    rust_default_data_plane_closeout_plan, rust_default_forwarding_hold_blocker_reduction,
    rust_dns_cutover_hold_blocker_reduction, rust_dns_default_path_blocker_reduction,
    rust_encrypted_protocols_bundle_execution, rust_encrypted_proxy_protocol_preflight_evidence,
    rust_encrypted_proxy_session_expansion, rust_fallback_retirement_readiness_manifest,
    rust_http_connect_proxy_adapter_evidence, rust_mihomo_fallback_retirement_bundle_execution,
    rust_plugin_process_supervision_blocker_reduction, rust_protocol_adapter_forwarding_expansion_evidence,
    rust_protocol_default_path_blocker_reduction, rust_protocol_forwarding_subset_preflight,
    rust_protocol_forwarding_subset_smoke_evidence, rust_protocol_forwarding_subset_status,
    rust_quic_udp_profile_blocker_reduction, rust_remote_adapter_transport_expansion_evidence,
    rust_route_packet_capture_blocker_reduction, rust_runtime_real_canary_evidence,
    rust_shadowsocks_aead_adapter_canary, rust_shadowsocks_aead_adapter_execution,
    rust_sidecar_independent_rollback_archive, rust_socks_auth_execution, rust_socks_bind_execution,
    rust_socks_tcp_connect_execution, rust_socks_udp_associate_execution, rust_socks_udp_fragments_execution,
    rust_tun_packet_capture_hold_bundle_execution, rust_tun_system_proxy_parity_preflight,
    rust_tun_transparent_routing_execution, rust_udp_plugin_transport_bundle_execution,
    start_rust_protocol_forwarding_subset, stop_rust_protocol_forwarding_subset,
};

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
pub async fn get_runtime_kernel_rust_protocol_forwarding_subset_preflight(
    listener_port: Option<u16>,
    target_host: Option<String>,
    target_port: Option<u16>,
) -> CmdResult<RustProtocolForwardingSubsetPreflightReport> {
    Ok(rust_protocol_forwarding_subset_preflight(listener_port, target_host, target_port).await)
}

#[tauri::command]
pub async fn start_runtime_kernel_rust_protocol_forwarding_subset(
    listener_port: Option<u16>,
    target_host: Option<String>,
    target_port: Option<u16>,
    explicit_opt_in: bool,
) -> CmdResult<RustProtocolForwardingSubsetStartReport> {
    start_rust_protocol_forwarding_subset(listener_port, target_host, target_port, explicit_opt_in)
        .await
        .stringify_err()
}

#[tauri::command]
pub async fn get_runtime_kernel_rust_protocol_forwarding_subset_status()
-> CmdResult<RustProtocolForwardingSubsetStatusReport> {
    Ok(rust_protocol_forwarding_subset_status().await)
}

#[tauri::command]
pub async fn stop_runtime_kernel_rust_protocol_forwarding_subset() -> CmdResult<RustProtocolForwardingSubsetStopReport>
{
    Ok(stop_rust_protocol_forwarding_subset().await)
}

#[tauri::command]
pub async fn get_runtime_kernel_rust_protocol_forwarding_subset_smoke_evidence(
    listener_port: Option<u16>,
    target_port: Option<u16>,
) -> CmdResult<RustProtocolForwardingSubsetSmokeEvidenceReport> {
    rust_protocol_forwarding_subset_smoke_evidence(listener_port, target_port)
        .await
        .stringify_err()
}

#[tauri::command]
pub async fn get_runtime_kernel_rust_tun_system_proxy_parity_preflight(
    requested_mode: Option<String>,
) -> CmdResult<RustTunSystemProxyParityPreflightReport> {
    rust_tun_system_proxy_parity_preflight(requested_mode)
        .await
        .stringify_err()
}

#[tauri::command]
pub async fn apply_runtime_kernel_rust_tun_system_proxy_parity(
    requested_mode: Option<String>,
    explicit_opt_in: bool,
) -> CmdResult<RustTunSystemProxyParityApplyReport> {
    apply_rust_tun_system_proxy_parity(requested_mode, explicit_opt_in)
        .await
        .stringify_err()
}

#[tauri::command]
pub async fn rollback_runtime_kernel_rust_tun_system_proxy_parity() -> CmdResult<RustTunSystemProxyParityRollbackReport>
{
    rollback_rust_tun_system_proxy_parity().await.stringify_err()
}

#[tauri::command]
pub async fn run_runtime_kernel_rust_tun_transparent_routing_execution(
    explicit_opt_in: bool,
) -> CmdResult<RustTunTransparentRoutingExecutionReport> {
    rust_tun_transparent_routing_execution(explicit_opt_in)
        .await
        .stringify_err()
}

#[tauri::command]
pub async fn get_runtime_kernel_rust_fallback_retirement_readiness_manifest()
-> CmdResult<RustFallbackRetirementReadinessManifest> {
    rust_fallback_retirement_readiness_manifest().await.stringify_err()
}

#[tauri::command]
pub async fn lock_runtime_kernel_rust_fallback_retirement_readiness(
    explicit_opt_in: bool,
) -> CmdResult<RustFallbackRetirementReadinessLockReport> {
    lock_rust_fallback_retirement_readiness(explicit_opt_in)
        .await
        .stringify_err()
}

#[tauri::command]
pub async fn run_runtime_kernel_rust_runtime_real_canary(
    canary_profile: Option<String>,
    explicit_opt_in: bool,
) -> CmdResult<RustRuntimeRealCanaryEvidenceReport> {
    rust_runtime_real_canary_evidence(canary_profile, explicit_opt_in)
        .await
        .stringify_err()
}

#[tauri::command]
pub async fn run_runtime_kernel_rust_protocol_adapter_forwarding_expansion(
    explicit_opt_in: bool,
) -> CmdResult<RustProtocolAdapterForwardingExpansionReport> {
    rust_protocol_adapter_forwarding_expansion_evidence(explicit_opt_in)
        .await
        .stringify_err()
}

#[tauri::command]
pub async fn run_runtime_kernel_rust_remote_adapter_transport_expansion(
    explicit_opt_in: bool,
) -> CmdResult<RustRemoteAdapterTransportExpansionReport> {
    rust_remote_adapter_transport_expansion_evidence(explicit_opt_in)
        .await
        .stringify_err()
}

#[tauri::command]
pub async fn run_runtime_kernel_rust_http_connect_proxy_adapter(
    explicit_opt_in: bool,
) -> CmdResult<RustHttpConnectProxyAdapterReport> {
    rust_http_connect_proxy_adapter_evidence(explicit_opt_in)
        .await
        .stringify_err()
}

#[tauri::command]
pub async fn run_runtime_kernel_rust_encrypted_proxy_protocol_preflight(
    explicit_opt_in: bool,
) -> CmdResult<RustEncryptedProxyProtocolPreflightReport> {
    rust_encrypted_proxy_protocol_preflight_evidence(explicit_opt_in)
        .await
        .stringify_err()
}

#[tauri::command]
pub async fn run_runtime_kernel_rust_encrypted_proxy_session_expansion(
    explicit_opt_in: bool,
) -> CmdResult<RustEncryptedProxySessionExpansionReport> {
    rust_encrypted_proxy_session_expansion(explicit_opt_in)
        .await
        .stringify_err()
}

#[tauri::command]
pub async fn run_runtime_kernel_rust_encrypted_protocols_bundle(
    explicit_opt_in: bool,
) -> CmdResult<RustEncryptedProtocolsBundleReport> {
    rust_encrypted_protocols_bundle_execution(explicit_opt_in)
        .await
        .stringify_err()
}

#[tauri::command]
pub async fn run_runtime_kernel_rust_shadowsocks_aead_adapter_execution(
    explicit_opt_in: bool,
) -> CmdResult<RustShadowsocksAeadAdapterExecutionReport> {
    rust_shadowsocks_aead_adapter_execution(explicit_opt_in)
        .await
        .stringify_err()
}

#[tauri::command]
pub async fn run_runtime_kernel_rust_shadowsocks_aead_adapter_canary(
    explicit_opt_in: bool,
) -> CmdResult<RustShadowsocksAeadAdapterCanaryReport> {
    rust_shadowsocks_aead_adapter_canary(explicit_opt_in)
        .await
        .stringify_err()
}

#[tauri::command]
pub async fn get_runtime_kernel_mihomo_fallback_retirement_execution_plan()
-> CmdResult<MihomoFallbackRetirementExecutionReport> {
    mihomo_fallback_retirement_execution_plan().await.stringify_err()
}

#[tauri::command]
pub async fn execute_runtime_kernel_mihomo_fallback_retirement(
    explicit_opt_in: bool,
    run_canary: bool,
) -> CmdResult<MihomoFallbackRetirementExecutionReport> {
    execute_mihomo_fallback_retirement(explicit_opt_in, run_canary)
        .await
        .stringify_err()
}

#[tauri::command]
pub async fn rollback_runtime_kernel_mihomo_fallback_retirement_execution()
-> CmdResult<MihomoFallbackRetirementExecutionReport> {
    rollback_mihomo_fallback_retirement_execution().await.stringify_err()
}

#[tauri::command]
pub async fn get_runtime_kernel_rust_default_data_plane_closeout_plan() -> CmdResult<RustDefaultDataPlaneCloseoutReport>
{
    rust_default_data_plane_closeout_plan().await.stringify_err()
}

#[tauri::command]
pub async fn closeout_runtime_kernel_rust_default_data_plane(
    explicit_opt_in: bool,
) -> CmdResult<RustDefaultDataPlaneCloseoutReport> {
    closeout_rust_default_data_plane(explicit_opt_in).await.stringify_err()
}

#[tauri::command]
pub async fn run_runtime_kernel_rust_socks_auth_execution(
    explicit_opt_in: bool,
) -> CmdResult<RustSocksAuthExecutionReport> {
    rust_socks_auth_execution(explicit_opt_in).await.stringify_err()
}

#[tauri::command]
pub async fn run_runtime_kernel_rust_socks_tcp_connect_execution(
    explicit_opt_in: bool,
) -> CmdResult<RustSocksTcpConnectExecutionReport> {
    rust_socks_tcp_connect_execution(explicit_opt_in).await.stringify_err()
}

#[tauri::command]
pub async fn run_runtime_kernel_rust_socks_bind_execution(
    explicit_opt_in: bool,
) -> CmdResult<RustSocksBindExecutionReport> {
    rust_socks_bind_execution(explicit_opt_in).await.stringify_err()
}

#[tauri::command]
pub async fn run_runtime_kernel_rust_socks_udp_associate_execution(
    explicit_opt_in: bool,
) -> CmdResult<RustSocksUdpAssociateExecutionReport> {
    rust_socks_udp_associate_execution(explicit_opt_in)
        .await
        .stringify_err()
}

#[tauri::command]
pub async fn run_runtime_kernel_rust_socks_udp_fragments_execution(
    explicit_opt_in: bool,
) -> CmdResult<RustSocksUdpFragmentsExecutionReport> {
    rust_socks_udp_fragments_execution(explicit_opt_in)
        .await
        .stringify_err()
}

#[tauri::command]
pub async fn run_runtime_kernel_rust_udp_plugin_transport_bundle(
    explicit_opt_in: bool,
) -> CmdResult<RustUdpPluginTransportBundleReport> {
    rust_udp_plugin_transport_bundle_execution(explicit_opt_in)
        .await
        .stringify_err()
}

#[tauri::command]
pub async fn run_runtime_kernel_rust_tun_packet_capture_hold_bundle(
    explicit_opt_in: bool,
) -> CmdResult<RustTunPacketCaptureHoldBundleReport> {
    rust_tun_packet_capture_hold_bundle_execution(explicit_opt_in)
        .await
        .stringify_err()
}

#[tauri::command]
pub async fn run_runtime_kernel_rust_mihomo_fallback_retirement_bundle(
    explicit_opt_in: bool,
) -> CmdResult<RustMihomoFallbackRetirementBundleReport> {
    rust_mihomo_fallback_retirement_bundle_execution(explicit_opt_in)
        .await
        .stringify_err()
}

#[tauri::command]
pub async fn run_runtime_kernel_go_to_rust_migration_final_review(
    explicit_opt_in: bool,
) -> CmdResult<GoToRustMigrationFinalReviewReport> {
    go_to_rust_migration_final_review(explicit_opt_in).await.stringify_err()
}

#[tauri::command]
pub async fn run_runtime_kernel_rust_sidecar_independent_rollback_archive(
    explicit_opt_in: bool,
) -> CmdResult<RustSidecarIndependentRollbackReport> {
    rust_sidecar_independent_rollback_archive(explicit_opt_in)
        .await
        .stringify_err()
}

#[tauri::command]
pub async fn run_runtime_kernel_rust_dns_default_path_blocker_reduction(
    explicit_opt_in: bool,
) -> CmdResult<RustDnsDefaultPathBlockerReport> {
    rust_dns_default_path_blocker_reduction(explicit_opt_in)
        .await
        .stringify_err()
}

#[tauri::command]
pub async fn run_runtime_kernel_rust_route_packet_capture_blocker_reduction(
    explicit_opt_in: bool,
) -> CmdResult<RustRoutePacketCaptureBlockerReport> {
    rust_route_packet_capture_blocker_reduction(explicit_opt_in)
        .await
        .stringify_err()
}

#[tauri::command]
pub async fn run_runtime_kernel_rust_protocol_default_path_blocker_reduction(
    explicit_opt_in: bool,
) -> CmdResult<RustProtocolDefaultPathBlockerReport> {
    rust_protocol_default_path_blocker_reduction(explicit_opt_in)
        .await
        .stringify_err()
}

#[tauri::command]
pub async fn run_runtime_kernel_rust_plugin_process_supervision_blocker_reduction(
    explicit_opt_in: bool,
) -> CmdResult<RustPluginProcessSupervisionBlockerReport> {
    rust_plugin_process_supervision_blocker_reduction(explicit_opt_in)
        .await
        .stringify_err()
}

#[tauri::command]
pub async fn run_runtime_kernel_rust_quic_udp_profile_blocker_reduction(
    explicit_opt_in: bool,
) -> CmdResult<RustQuicUdpProfileBlockerReport> {
    rust_quic_udp_profile_blocker_reduction(explicit_opt_in)
        .await
        .stringify_err()
}

#[tauri::command]
pub async fn run_runtime_kernel_rust_default_forwarding_hold_blocker_reduction(
    explicit_opt_in: bool,
) -> CmdResult<RustDefaultForwardingHoldBlockerReport> {
    rust_default_forwarding_hold_blocker_reduction(explicit_opt_in)
        .await
        .stringify_err()
}

#[tauri::command]
pub async fn run_runtime_kernel_rust_dns_cutover_hold_blocker_reduction(
    explicit_opt_in: bool,
) -> CmdResult<RustDnsCutoverHoldBlockerReport> {
    rust_dns_cutover_hold_blocker_reduction(explicit_opt_in)
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
pub async fn get_runtime_kernel_loopback_go_mihomo_retirement_audit(
    full_rust_runtime_hardened_decision: Option<bool>,
    sidecar_source_audit_decision: Option<bool>,
    bundled_mihomo_audit_decision: Option<bool>,
    ipc_fallback_audit_decision: Option<bool>,
    docs_audit_decision: Option<bool>,
    emergency_rollback_retained: Option<bool>,
    final_retirement_audit_decision: Option<bool>,
) -> CmdResult<KernelLoopbackGoMihomoRetirementAuditReport> {
    rust_kernel_runtime_go_mihomo_retirement_audit(
        full_rust_runtime_hardened_decision,
        sidecar_source_audit_decision,
        bundled_mihomo_audit_decision,
        ipc_fallback_audit_decision,
        docs_audit_decision,
        emergency_rollback_retained,
        final_retirement_audit_decision,
    )
    .await
    .stringify_err()
}

#[tauri::command]
pub async fn get_runtime_kernel_loopback_go_mihomo_retirement_plan(
    go_mihomo_retirement_audit_complete_decision: Option<bool>,
    sidecar_source_removal_plan_decision: Option<bool>,
    bundled_artifact_deprecation_plan_decision: Option<bool>,
    ipc_fallback_replacement_plan_decision: Option<bool>,
    emergency_rollback_preservation_plan_decision: Option<bool>,
    release_rollout_plan_decision: Option<bool>,
    final_retirement_plan_decision: Option<bool>,
) -> CmdResult<KernelLoopbackGoMihomoRetirementPlanReport> {
    rust_kernel_runtime_go_mihomo_retirement_plan(
        go_mihomo_retirement_audit_complete_decision,
        sidecar_source_removal_plan_decision,
        bundled_artifact_deprecation_plan_decision,
        ipc_fallback_replacement_plan_decision,
        emergency_rollback_preservation_plan_decision,
        release_rollout_plan_decision,
        final_retirement_plan_decision,
    )
    .await
    .stringify_err()
}

#[tauri::command]
pub async fn get_runtime_kernel_loopback_go_mihomo_retirement_execution_guard(
    go_mihomo_retirement_plan_complete_decision: Option<bool>,
    removal_manifest_decision: Option<bool>,
    abort_plan_decision: Option<bool>,
    staged_rollout_guard_decision: Option<bool>,
    emergency_rollback_drill_decision: Option<bool>,
    operator_acknowledgement_decision: Option<bool>,
    final_execution_guard_decision: Option<bool>,
) -> CmdResult<KernelLoopbackGoMihomoRetirementExecutionGuardReport> {
    rust_kernel_runtime_go_mihomo_retirement_execution_guard(
        go_mihomo_retirement_plan_complete_decision,
        removal_manifest_decision,
        abort_plan_decision,
        staged_rollout_guard_decision,
        emergency_rollback_drill_decision,
        operator_acknowledgement_decision,
        final_execution_guard_decision,
    )
    .await
    .stringify_err()
}

#[tauri::command]
pub async fn get_runtime_kernel_loopback_go_mihomo_retirement_dry_run(
    go_mihomo_retirement_execution_guard_complete_decision: Option<bool>,
    dry_run_manifest_replay_decision: Option<bool>,
    no_source_mutations_decision: Option<bool>,
    no_bundled_artifact_mutations_decision: Option<bool>,
    rollback_rehearsal_decision: Option<bool>,
    dry_run_report_archived_decision: Option<bool>,
    final_dry_run_decision: Option<bool>,
) -> CmdResult<KernelLoopbackGoMihomoRetirementDryRunReport> {
    rust_kernel_runtime_go_mihomo_retirement_dry_run(
        go_mihomo_retirement_execution_guard_complete_decision,
        dry_run_manifest_replay_decision,
        no_source_mutations_decision,
        no_bundled_artifact_mutations_decision,
        rollback_rehearsal_decision,
        dry_run_report_archived_decision,
        final_dry_run_decision,
    )
    .await
    .stringify_err()
}

#[tauri::command]
pub async fn get_runtime_kernel_loopback_go_mihomo_retirement_closeout(
    go_mihomo_retirement_dry_run_complete_decision: Option<bool>,
    dry_run_evidence_review_decision: Option<bool>,
    closeout_report_archived_decision: Option<bool>,
    rollback_checkpoint_verified_decision: Option<bool>,
    artifact_inventory_frozen_decision: Option<bool>,
    no_removal_mutations_decision: Option<bool>,
    final_closeout_decision: Option<bool>,
) -> CmdResult<KernelLoopbackGoMihomoRetirementCloseoutReport> {
    rust_kernel_runtime_go_mihomo_retirement_closeout(
        go_mihomo_retirement_dry_run_complete_decision,
        dry_run_evidence_review_decision,
        closeout_report_archived_decision,
        rollback_checkpoint_verified_decision,
        artifact_inventory_frozen_decision,
        no_removal_mutations_decision,
        final_closeout_decision,
    )
    .await
    .stringify_err()
}

#[tauri::command]
pub async fn get_runtime_kernel_loopback_go_mihomo_retirement_final_removal_gate(
    go_mihomo_retirement_closeout_complete_decision: Option<bool>,
    closeout_evidence_acceptance_decision: Option<bool>,
    rollback_boundary_lock_decision: Option<bool>,
    removal_scope_lock_decision: Option<bool>,
    release_blocker_review_decision: Option<bool>,
    final_operator_approval_decision: Option<bool>,
    final_removal_decision: Option<bool>,
) -> CmdResult<KernelLoopbackGoMihomoRetirementFinalRemovalGateReport> {
    rust_kernel_runtime_go_mihomo_retirement_final_removal_gate(
        go_mihomo_retirement_closeout_complete_decision,
        closeout_evidence_acceptance_decision,
        rollback_boundary_lock_decision,
        removal_scope_lock_decision,
        release_blocker_review_decision,
        final_operator_approval_decision,
        final_removal_decision,
    )
    .await
    .stringify_err()
}

#[tauri::command]
pub async fn get_runtime_kernel_loopback_go_mihomo_retirement_execution(
    go_mihomo_retirement_final_removal_gate_complete_decision: Option<bool>,
    rollback_checkpoint_created_decision: Option<bool>,
    execution_manifest_application_decision: Option<bool>,
    source_removal_record_decision: Option<bool>,
    artifact_removal_record_decision: Option<bool>,
    post_execution_validation_decision: Option<bool>,
    final_execution_decision: Option<bool>,
) -> CmdResult<KernelLoopbackGoMihomoRetirementExecutionReport> {
    rust_kernel_runtime_go_mihomo_retirement_execution(
        go_mihomo_retirement_final_removal_gate_complete_decision,
        rollback_checkpoint_created_decision,
        execution_manifest_application_decision,
        source_removal_record_decision,
        artifact_removal_record_decision,
        post_execution_validation_decision,
        final_execution_decision,
    )
    .await
    .stringify_err()
}

#[tauri::command]
pub async fn get_runtime_kernel_loopback_go_mihomo_retirement_post_execution_verification(
    go_mihomo_retirement_execution_complete_decision: Option<bool>,
    rust_only_boundary_verification_decision: Option<bool>,
    rollback_checkpoint_retention_decision: Option<bool>,
    source_removal_verification_decision: Option<bool>,
    artifact_removal_verification_decision: Option<bool>,
    fallback_ipc_absence_verification_decision: Option<bool>,
    final_verification_decision: Option<bool>,
) -> CmdResult<KernelLoopbackGoMihomoRetirementPostExecutionVerificationReport> {
    rust_kernel_runtime_go_mihomo_retirement_post_execution_verification(
        go_mihomo_retirement_execution_complete_decision,
        rust_only_boundary_verification_decision,
        rollback_checkpoint_retention_decision,
        source_removal_verification_decision,
        artifact_removal_verification_decision,
        fallback_ipc_absence_verification_decision,
        final_verification_decision,
    )
    .await
    .stringify_err()
}

#[tauri::command]
pub async fn get_runtime_kernel_loopback_go_mihomo_retirement_rollback_surface_retirement(
    go_mihomo_retirement_post_execution_verification_complete_decision: Option<bool>,
    post_execution_verification_review_decision: Option<bool>,
    replacement_recovery_path_verification_decision: Option<bool>,
    rollback_surface_inventory_lock_decision: Option<bool>,
    rollback_surface_retirement_plan_archive_decision: Option<bool>,
    emergency_recovery_drill_decision: Option<bool>,
    final_rollback_surface_retirement_decision: Option<bool>,
) -> CmdResult<KernelLoopbackGoMihomoRetirementRollbackSurfaceRetirementReport> {
    rust_kernel_runtime_go_mihomo_retirement_rollback_surface_retirement(
        go_mihomo_retirement_post_execution_verification_complete_decision,
        post_execution_verification_review_decision,
        replacement_recovery_path_verification_decision,
        rollback_surface_inventory_lock_decision,
        rollback_surface_retirement_plan_archive_decision,
        emergency_recovery_drill_decision,
        final_rollback_surface_retirement_decision,
    )
    .await
    .stringify_err()
}

#[tauri::command]
pub async fn get_runtime_kernel_loopback_go_mihomo_retirement_completion_closeout(
    go_mihomo_retirement_rollback_surface_retirement_complete_decision: Option<bool>,
    rollback_surface_retirement_review_decision: Option<bool>,
    recovery_boundary_evidence_retention_decision: Option<bool>,
    completion_report_archive_decision: Option<bool>,
    release_notes_update_decision: Option<bool>,
    migration_state_freeze_decision: Option<bool>,
    final_completion_decision: Option<bool>,
) -> CmdResult<KernelLoopbackGoMihomoRetirementCompletionCloseoutReport> {
    rust_kernel_runtime_go_mihomo_retirement_completion_closeout(
        go_mihomo_retirement_rollback_surface_retirement_complete_decision,
        rollback_surface_retirement_review_decision,
        recovery_boundary_evidence_retention_decision,
        completion_report_archive_decision,
        release_notes_update_decision,
        migration_state_freeze_decision,
        final_completion_decision,
    )
    .await
    .stringify_err()
}

#[tauri::command]
pub async fn get_runtime_kernel_loopback_rust_data_plane_hardening_preflight(
    go_mihomo_retirement_complete_decision: Option<bool>,
    protocol_parity_inventory_decision: Option<bool>,
    tun_boundary_inventory_decision: Option<bool>,
    adapter_compatibility_matrix_decision: Option<bool>,
    dns_leak_verification_plan_decision: Option<bool>,
    rollback_drill_plan_decision: Option<bool>,
    opt_in_execution_boundary_decision: Option<bool>,
    final_preflight_decision: Option<bool>,
) -> CmdResult<KernelLoopbackRustDataPlaneHardeningPreflightReport> {
    rust_kernel_runtime_data_plane_hardening_preflight(
        go_mihomo_retirement_complete_decision,
        protocol_parity_inventory_decision,
        tun_boundary_inventory_decision,
        adapter_compatibility_matrix_decision,
        dns_leak_verification_plan_decision,
        rollback_drill_plan_decision,
        opt_in_execution_boundary_decision,
        final_preflight_decision,
    )
    .await
    .stringify_err()
}

#[tauri::command]
pub async fn get_runtime_kernel_loopback_rust_data_plane_hardening_boundary_audit(
    rust_data_plane_hardening_preflight_complete_decision: Option<bool>,
    preflight_review_decision: Option<bool>,
    protocol_boundary_audit_decision: Option<bool>,
    tun_boundary_audit_decision: Option<bool>,
    adapter_boundary_audit_decision: Option<bool>,
    dns_leak_boundary_audit_decision: Option<bool>,
    rollback_boundary_audit_decision: Option<bool>,
    opt_in_boundary_audit_decision: Option<bool>,
    final_boundary_audit_decision: Option<bool>,
) -> CmdResult<KernelLoopbackRustDataPlaneHardeningBoundaryAuditReport> {
    rust_kernel_runtime_data_plane_hardening_boundary_audit(
        rust_data_plane_hardening_preflight_complete_decision,
        preflight_review_decision,
        protocol_boundary_audit_decision,
        tun_boundary_audit_decision,
        adapter_boundary_audit_decision,
        dns_leak_boundary_audit_decision,
        rollback_boundary_audit_decision,
        opt_in_boundary_audit_decision,
        final_boundary_audit_decision,
    )
    .await
    .stringify_err()
}

#[tauri::command]
pub async fn get_runtime_kernel_loopback_rust_data_plane_hardening_opt_in_execution_guard(
    rust_data_plane_hardening_boundary_audit_complete_decision: Option<bool>,
    boundary_audit_review_decision: Option<bool>,
    opt_in_scope_lock_decision: Option<bool>,
    rollout_guard_definition_decision: Option<bool>,
    abort_plan_approval_decision: Option<bool>,
    telemetry_watch_configuration_decision: Option<bool>,
    rollback_switch_verification_decision: Option<bool>,
    operator_acknowledgement_decision: Option<bool>,
    final_execution_guard_decision: Option<bool>,
) -> CmdResult<KernelLoopbackRustDataPlaneHardeningOptInExecutionGuardReport> {
    rust_kernel_runtime_data_plane_hardening_opt_in_execution_guard(
        rust_data_plane_hardening_boundary_audit_complete_decision,
        boundary_audit_review_decision,
        opt_in_scope_lock_decision,
        rollout_guard_definition_decision,
        abort_plan_approval_decision,
        telemetry_watch_configuration_decision,
        rollback_switch_verification_decision,
        operator_acknowledgement_decision,
        final_execution_guard_decision,
    )
    .await
    .stringify_err()
}

#[tauri::command]
pub async fn get_runtime_kernel_loopback_rust_data_plane_hardening_opt_in_dry_run(
    rust_data_plane_hardening_opt_in_execution_guard_complete_decision: Option<bool>,
    execution_guard_review_decision: Option<bool>,
    dry_run_scope_lock_decision: Option<bool>,
    manifest_replay_decision: Option<bool>,
    synthetic_flow_plan_decision: Option<bool>,
    leak_watch_plan_verification_decision: Option<bool>,
    rollback_rehearsal_decision: Option<bool>,
    production_forwarding_unchanged_verification_decision: Option<bool>,
    dry_run_evidence_archive_decision: Option<bool>,
    final_dry_run_decision: Option<bool>,
) -> CmdResult<KernelLoopbackRustDataPlaneHardeningOptInDryRunReport> {
    rust_kernel_runtime_data_plane_hardening_opt_in_dry_run(
        rust_data_plane_hardening_opt_in_execution_guard_complete_decision,
        execution_guard_review_decision,
        dry_run_scope_lock_decision,
        manifest_replay_decision,
        synthetic_flow_plan_decision,
        leak_watch_plan_verification_decision,
        rollback_rehearsal_decision,
        production_forwarding_unchanged_verification_decision,
        dry_run_evidence_archive_decision,
        final_dry_run_decision,
    )
    .await
    .stringify_err()
}

#[tauri::command]
pub async fn get_runtime_kernel_loopback_rust_data_plane_hardening_opt_in_execution(
    rust_data_plane_hardening_opt_in_dry_run_complete_decision: Option<bool>,
    dry_run_review_decision: Option<bool>,
    execution_manifest_lock_decision: Option<bool>,
    staged_opt_in_window_decision: Option<bool>,
    telemetry_watch_activation_decision: Option<bool>,
    rollback_switch_arm_decision: Option<bool>,
    production_mutation_guard_retention_decision: Option<bool>,
    operator_execution_acknowledgement_decision: Option<bool>,
    final_execution_decision: Option<bool>,
) -> CmdResult<KernelLoopbackRustDataPlaneHardeningOptInExecutionReport> {
    rust_kernel_runtime_data_plane_hardening_opt_in_execution(
        rust_data_plane_hardening_opt_in_dry_run_complete_decision,
        dry_run_review_decision,
        execution_manifest_lock_decision,
        staged_opt_in_window_decision,
        telemetry_watch_activation_decision,
        rollback_switch_arm_decision,
        production_mutation_guard_retention_decision,
        operator_execution_acknowledgement_decision,
        final_execution_decision,
    )
    .await
    .stringify_err()
}

#[tauri::command]
pub async fn get_runtime_kernel_loopback_rust_data_plane_hardening_opt_in_execution_verification(
    rust_data_plane_hardening_opt_in_execution_complete_decision: Option<bool>,
    execution_record_review_decision: Option<bool>,
    telemetry_sample_review_decision: Option<bool>,
    rollback_readiness_verification_decision: Option<bool>,
    production_mutation_guard_retention_verification_decision: Option<bool>,
    production_forwarding_unchanged_verification_decision: Option<bool>,
    leak_regression_absence_verification_decision: Option<bool>,
    verification_evidence_archive_decision: Option<bool>,
    final_verification_decision: Option<bool>,
) -> CmdResult<KernelLoopbackRustDataPlaneHardeningOptInExecutionVerificationReport> {
    rust_kernel_runtime_data_plane_hardening_opt_in_execution_verification(
        rust_data_plane_hardening_opt_in_execution_complete_decision,
        execution_record_review_decision,
        telemetry_sample_review_decision,
        rollback_readiness_verification_decision,
        production_mutation_guard_retention_verification_decision,
        production_forwarding_unchanged_verification_decision,
        leak_regression_absence_verification_decision,
        verification_evidence_archive_decision,
        final_verification_decision,
    )
    .await
    .stringify_err()
}

#[tauri::command]
pub async fn get_runtime_kernel_loopback_rust_data_plane_hardening_controlled_rollout_guard(
    rust_data_plane_hardening_opt_in_execution_verification_complete_decision: Option<bool>,
    opt_in_verification_review_decision: Option<bool>,
    controlled_rollout_scope_lock_decision: Option<bool>,
    canary_population_cap_definition_decision: Option<bool>,
    health_rollback_trigger_definition_decision: Option<bool>,
    telemetry_hold_window_configuration_decision: Option<bool>,
    mihomo_fallback_retention_decision: Option<bool>,
    production_mutation_guard_retention_decision: Option<bool>,
    operator_rollout_guard_acknowledgement_decision: Option<bool>,
    final_controlled_rollout_guard_decision: Option<bool>,
) -> CmdResult<KernelLoopbackRustDataPlaneHardeningControlledRolloutGuardReport> {
    rust_kernel_runtime_data_plane_hardening_controlled_rollout_guard(
        rust_data_plane_hardening_opt_in_execution_verification_complete_decision,
        opt_in_verification_review_decision,
        controlled_rollout_scope_lock_decision,
        canary_population_cap_definition_decision,
        health_rollback_trigger_definition_decision,
        telemetry_hold_window_configuration_decision,
        mihomo_fallback_retention_decision,
        production_mutation_guard_retention_decision,
        operator_rollout_guard_acknowledgement_decision,
        final_controlled_rollout_guard_decision,
    )
    .await
    .stringify_err()
}

#[tauri::command]
pub async fn get_runtime_kernel_loopback_rust_data_plane_hardening_controlled_rollout_dry_run(
    rust_data_plane_hardening_controlled_rollout_guard_complete_decision: Option<bool>,
    guard_review_decision: Option<bool>,
    dry_run_manifest_replay_decision: Option<bool>,
    capped_canary_simulation_decision: Option<bool>,
    fallback_trigger_rehearsal_decision: Option<bool>,
    telemetry_hold_sample_review_decision: Option<bool>,
    rollback_switch_rehearsal_decision: Option<bool>,
    production_forwarding_unchanged_verification_decision: Option<bool>,
    dry_run_evidence_archive_decision: Option<bool>,
    final_controlled_rollout_dry_run_decision: Option<bool>,
) -> CmdResult<KernelLoopbackRustDataPlaneHardeningControlledRolloutDryRunReport> {
    rust_kernel_runtime_data_plane_hardening_controlled_rollout_dry_run(
        rust_data_plane_hardening_controlled_rollout_guard_complete_decision,
        guard_review_decision,
        dry_run_manifest_replay_decision,
        capped_canary_simulation_decision,
        fallback_trigger_rehearsal_decision,
        telemetry_hold_sample_review_decision,
        rollback_switch_rehearsal_decision,
        production_forwarding_unchanged_verification_decision,
        dry_run_evidence_archive_decision,
        final_controlled_rollout_dry_run_decision,
    )
    .await
    .stringify_err()
}

#[tauri::command]
pub async fn get_runtime_kernel_loopback_rust_data_plane_hardening_controlled_rollout_readiness_closeout(
    rust_data_plane_hardening_controlled_rollout_dry_run_complete_decision: Option<bool>,
    dry_run_review_decision: Option<bool>,
    rollout_window_approval_decision: Option<bool>,
    canary_population_cap_enforcement_decision: Option<bool>,
    automatic_fallback_arm_decision: Option<bool>,
    telemetry_watch_activation_decision: Option<bool>,
    rollback_owner_acknowledgement_decision: Option<bool>,
    production_mutation_guard_retention_decision: Option<bool>,
    closeout_evidence_archive_decision: Option<bool>,
    final_controlled_rollout_readiness_decision: Option<bool>,
) -> CmdResult<KernelLoopbackRustDataPlaneHardeningControlledRolloutReadinessCloseoutReport> {
    rust_kernel_runtime_data_plane_hardening_controlled_rollout_readiness_closeout(
        rust_data_plane_hardening_controlled_rollout_dry_run_complete_decision,
        dry_run_review_decision,
        rollout_window_approval_decision,
        canary_population_cap_enforcement_decision,
        automatic_fallback_arm_decision,
        telemetry_watch_activation_decision,
        rollback_owner_acknowledgement_decision,
        production_mutation_guard_retention_decision,
        closeout_evidence_archive_decision,
        final_controlled_rollout_readiness_decision,
    )
    .await
    .stringify_err()
}

#[tauri::command]
pub async fn get_runtime_kernel_loopback_rust_data_plane_hardening_controlled_rollout_canary_execution(
    rust_data_plane_hardening_controlled_rollout_readiness_closeout_complete_decision: Option<bool>,
    readiness_closeout_review_decision: Option<bool>,
    execution_manifest_lock_decision: Option<bool>,
    canary_window_start_decision: Option<bool>,
    canary_population_cap_enforcement_decision: Option<bool>,
    health_telemetry_activation_decision: Option<bool>,
    automatic_fallback_arm_decision: Option<bool>,
    mihomo_fallback_retention_decision: Option<bool>,
    production_mutation_guard_retention_decision: Option<bool>,
    operator_canary_execution_acknowledgement_decision: Option<bool>,
    final_controlled_rollout_canary_execution_decision: Option<bool>,
) -> CmdResult<KernelLoopbackRustDataPlaneHardeningControlledRolloutCanaryExecutionReport> {
    rust_kernel_runtime_data_plane_hardening_controlled_rollout_canary_execution(
        rust_data_plane_hardening_controlled_rollout_readiness_closeout_complete_decision,
        readiness_closeout_review_decision,
        execution_manifest_lock_decision,
        canary_window_start_decision,
        canary_population_cap_enforcement_decision,
        health_telemetry_activation_decision,
        automatic_fallback_arm_decision,
        mihomo_fallback_retention_decision,
        production_mutation_guard_retention_decision,
        operator_canary_execution_acknowledgement_decision,
        final_controlled_rollout_canary_execution_decision,
    )
    .await
    .stringify_err()
}

#[tauri::command]
pub async fn get_runtime_kernel_loopback_rust_data_plane_hardening_controlled_rollout_canary_verification(
    rust_data_plane_hardening_controlled_rollout_canary_execution_complete_decision: Option<bool>,
    execution_record_review_decision: Option<bool>,
    health_telemetry_sample_review_decision: Option<bool>,
    automatic_fallback_result_review_decision: Option<bool>,
    unsupported_traffic_fallback_verification_decision: Option<bool>,
    leak_regression_absence_verification_decision: Option<bool>,
    rollback_readiness_verification_decision: Option<bool>,
    production_mutation_guard_retention_verification_decision: Option<bool>,
    verification_evidence_archive_decision: Option<bool>,
    final_controlled_rollout_canary_verification_decision: Option<bool>,
) -> CmdResult<KernelLoopbackRustDataPlaneHardeningControlledRolloutCanaryVerificationReport> {
    rust_kernel_runtime_data_plane_hardening_controlled_rollout_canary_verification(
        rust_data_plane_hardening_controlled_rollout_canary_execution_complete_decision,
        execution_record_review_decision,
        health_telemetry_sample_review_decision,
        automatic_fallback_result_review_decision,
        unsupported_traffic_fallback_verification_decision,
        leak_regression_absence_verification_decision,
        rollback_readiness_verification_decision,
        production_mutation_guard_retention_verification_decision,
        verification_evidence_archive_decision,
        final_controlled_rollout_canary_verification_decision,
    )
    .await
    .stringify_err()
}

#[tauri::command]
pub async fn get_runtime_kernel_loopback_rust_data_plane_hardening_supported_default_promotion_guard(
    rust_data_plane_hardening_controlled_rollout_canary_verification_complete_decision: Option<bool>,
    canary_verification_review_decision: Option<bool>,
    supported_profile_scope_lock_decision: Option<bool>,
    fallback_matrix_retention_decision: Option<bool>,
    rollback_switch_verification_decision: Option<bool>,
    telemetry_soak_window_definition_decision: Option<bool>,
    release_blocker_review_decision: Option<bool>,
    production_mutation_guard_retention_decision: Option<bool>,
    operator_promotion_acknowledgement_decision: Option<bool>,
    final_supported_default_promotion_guard_decision: Option<bool>,
) -> CmdResult<KernelLoopbackRustDataPlaneHardeningSupportedDefaultPromotionGuardReport> {
    rust_kernel_runtime_data_plane_hardening_supported_default_promotion_guard(
        rust_data_plane_hardening_controlled_rollout_canary_verification_complete_decision,
        canary_verification_review_decision,
        supported_profile_scope_lock_decision,
        fallback_matrix_retention_decision,
        rollback_switch_verification_decision,
        telemetry_soak_window_definition_decision,
        release_blocker_review_decision,
        production_mutation_guard_retention_decision,
        operator_promotion_acknowledgement_decision,
        final_supported_default_promotion_guard_decision,
    )
    .await
    .stringify_err()
}

#[tauri::command]
pub async fn get_runtime_kernel_loopback_rust_data_plane_hardening_supported_default_promotion_dry_run(
    rust_data_plane_hardening_supported_default_promotion_guard_complete_decision: Option<bool>,
    guard_review_decision: Option<bool>,
    default_selection_manifest_replay_decision: Option<bool>,
    supported_profile_simulation_decision: Option<bool>,
    fallback_decision_rehearsal_decision: Option<bool>,
    rollback_rehearsal_decision: Option<bool>,
    production_forwarding_unchanged_verification_decision: Option<bool>,
    dry_run_evidence_archive_decision: Option<bool>,
    final_supported_default_promotion_dry_run_decision: Option<bool>,
) -> CmdResult<KernelLoopbackRustDataPlaneHardeningSupportedDefaultPromotionDryRunReport> {
    rust_kernel_runtime_data_plane_hardening_supported_default_promotion_dry_run(
        rust_data_plane_hardening_supported_default_promotion_guard_complete_decision,
        guard_review_decision,
        default_selection_manifest_replay_decision,
        supported_profile_simulation_decision,
        fallback_decision_rehearsal_decision,
        rollback_rehearsal_decision,
        production_forwarding_unchanged_verification_decision,
        dry_run_evidence_archive_decision,
        final_supported_default_promotion_dry_run_decision,
    )
    .await
    .stringify_err()
}

#[tauri::command]
pub async fn get_runtime_kernel_loopback_rust_data_plane_hardening_supported_default_cutover(
    rust_data_plane_hardening_supported_default_promotion_dry_run_complete_decision: Option<bool>,
    dry_run_review_decision: Option<bool>,
    cutover_manifest_lock_decision: Option<bool>,
    supported_profile_default_selection_confirmation_decision: Option<bool>,
    unsupported_paths_mihomo_fallback_binding_decision: Option<bool>,
    rollback_switch_arm_decision: Option<bool>,
    telemetry_soak_watch_activation_decision: Option<bool>,
    operator_cutover_acknowledgement_decision: Option<bool>,
    production_mutation_guard_transition_record_decision: Option<bool>,
    final_supported_default_cutover_decision: Option<bool>,
) -> CmdResult<KernelLoopbackRustDataPlaneHardeningSupportedDefaultCutoverReport> {
    rust_kernel_runtime_data_plane_hardening_supported_default_cutover(
        rust_data_plane_hardening_supported_default_promotion_dry_run_complete_decision,
        dry_run_review_decision,
        cutover_manifest_lock_decision,
        supported_profile_default_selection_confirmation_decision,
        unsupported_paths_mihomo_fallback_binding_decision,
        rollback_switch_arm_decision,
        telemetry_soak_watch_activation_decision,
        operator_cutover_acknowledgement_decision,
        production_mutation_guard_transition_record_decision,
        final_supported_default_cutover_decision,
    )
    .await
    .stringify_err()
}

#[tauri::command]
pub async fn get_runtime_kernel_loopback_rust_data_plane_hardening_supported_default_cutover_verification(
    rust_data_plane_hardening_supported_default_cutover_complete_decision: Option<bool>,
    cutover_record_review_decision: Option<bool>,
    supported_profile_traffic_sample_review_decision: Option<bool>,
    unsupported_path_fallback_verification_decision: Option<bool>,
    rollback_switch_verification_decision: Option<bool>,
    telemetry_soak_sample_review_decision: Option<bool>,
    leak_regression_absence_verification_decision: Option<bool>,
    mutation_audit_record_archive_decision: Option<bool>,
    final_supported_default_cutover_verification_decision: Option<bool>,
) -> CmdResult<KernelLoopbackRustDataPlaneHardeningSupportedDefaultCutoverVerificationReport> {
    rust_kernel_runtime_data_plane_hardening_supported_default_cutover_verification(
        rust_data_plane_hardening_supported_default_cutover_complete_decision,
        cutover_record_review_decision,
        supported_profile_traffic_sample_review_decision,
        unsupported_path_fallback_verification_decision,
        rollback_switch_verification_decision,
        telemetry_soak_sample_review_decision,
        leak_regression_absence_verification_decision,
        mutation_audit_record_archive_decision,
        final_supported_default_cutover_verification_decision,
    )
    .await
    .stringify_err()
}

#[tauri::command]
pub async fn get_runtime_kernel_loopback_rust_data_plane_hardening_supported_default_cutover_hold_window(
    rust_data_plane_hardening_supported_default_cutover_verification_complete_decision: Option<bool>,
    verification_review_decision: Option<bool>,
    soak_window_elapsed_decision: Option<bool>,
    health_budget_satisfied_decision: Option<bool>,
    fallback_incident_review_decision: Option<bool>,
    rollback_switch_still_armed_decision: Option<bool>,
    mihomo_fallback_retention_decision: Option<bool>,
    hold_window_evidence_archive_decision: Option<bool>,
    final_supported_default_cutover_hold_window_decision: Option<bool>,
) -> CmdResult<KernelLoopbackRustDataPlaneHardeningSupportedDefaultCutoverHoldWindowReport> {
    rust_kernel_runtime_data_plane_hardening_supported_default_cutover_hold_window(
        rust_data_plane_hardening_supported_default_cutover_verification_complete_decision,
        verification_review_decision,
        soak_window_elapsed_decision,
        health_budget_satisfied_decision,
        fallback_incident_review_decision,
        rollback_switch_still_armed_decision,
        mihomo_fallback_retention_decision,
        hold_window_evidence_archive_decision,
        final_supported_default_cutover_hold_window_decision,
    )
    .await
    .stringify_err()
}

#[tauri::command]
pub async fn get_runtime_kernel_loopback_rust_data_plane_hardening_supported_default_cutover_closeout(
    rust_data_plane_hardening_supported_default_cutover_hold_window_complete_decision: Option<bool>,
    hold_window_review_decision: Option<bool>,
    supported_default_state_documentation_decision: Option<bool>,
    rollback_owner_acknowledgement_decision: Option<bool>,
    fallback_retirement_boundary_retention_decision: Option<bool>,
    release_notes_update_decision: Option<bool>,
    closeout_evidence_archive_decision: Option<bool>,
    final_supported_default_cutover_closeout_decision: Option<bool>,
) -> CmdResult<KernelLoopbackRustDataPlaneHardeningSupportedDefaultCutoverCloseoutReport> {
    rust_kernel_runtime_data_plane_hardening_supported_default_cutover_closeout(
        rust_data_plane_hardening_supported_default_cutover_hold_window_complete_decision,
        hold_window_review_decision,
        supported_default_state_documentation_decision,
        rollback_owner_acknowledgement_decision,
        fallback_retirement_boundary_retention_decision,
        release_notes_update_decision,
        closeout_evidence_archive_decision,
        final_supported_default_cutover_closeout_decision,
    )
    .await
    .stringify_err()
}

#[tauri::command]
pub async fn get_runtime_kernel_loopback_rust_data_plane_hardening_expanded_default_rollout_guard(
    rust_data_plane_hardening_supported_default_cutover_closeout_complete_decision: Option<bool>,
    cutover_closeout_review_decision: Option<bool>,
    expanded_scope_lock_decision: Option<bool>,
    rollout_cap_definition_decision: Option<bool>,
    fallback_matrix_retention_decision: Option<bool>,
    rollback_switch_verification_decision: Option<bool>,
    telemetry_soak_plan_definition_decision: Option<bool>,
    unsupported_path_boundary_retention_decision: Option<bool>,
    operator_rollout_acknowledgement_decision: Option<bool>,
    final_expanded_default_rollout_guard_decision: Option<bool>,
) -> CmdResult<KernelLoopbackRustDataPlaneHardeningExpandedDefaultRolloutGuardReport> {
    rust_kernel_runtime_data_plane_hardening_expanded_default_rollout_guard(
        rust_data_plane_hardening_supported_default_cutover_closeout_complete_decision,
        cutover_closeout_review_decision,
        expanded_scope_lock_decision,
        rollout_cap_definition_decision,
        fallback_matrix_retention_decision,
        rollback_switch_verification_decision,
        telemetry_soak_plan_definition_decision,
        unsupported_path_boundary_retention_decision,
        operator_rollout_acknowledgement_decision,
        final_expanded_default_rollout_guard_decision,
    )
    .await
    .stringify_err()
}

#[tauri::command]
pub async fn get_runtime_kernel_loopback_rust_data_plane_hardening_expanded_default_rollout_dry_run(
    rust_data_plane_hardening_expanded_default_rollout_guard_complete_decision: Option<bool>,
    guard_review_decision: Option<bool>,
    expanded_manifest_replay_decision: Option<bool>,
    representative_profile_simulation_decision: Option<bool>,
    fallback_routing_rehearsal_decision: Option<bool>,
    rollback_rehearsal_decision: Option<bool>,
    telemetry_soak_sample_review_decision: Option<bool>,
    dry_run_evidence_archive_decision: Option<bool>,
    final_expanded_default_rollout_dry_run_decision: Option<bool>,
) -> CmdResult<KernelLoopbackRustDataPlaneHardeningExpandedDefaultRolloutDryRunReport> {
    rust_kernel_runtime_data_plane_hardening_expanded_default_rollout_dry_run(
        rust_data_plane_hardening_expanded_default_rollout_guard_complete_decision,
        guard_review_decision,
        expanded_manifest_replay_decision,
        representative_profile_simulation_decision,
        fallback_routing_rehearsal_decision,
        rollback_rehearsal_decision,
        telemetry_soak_sample_review_decision,
        dry_run_evidence_archive_decision,
        final_expanded_default_rollout_dry_run_decision,
    )
    .await
    .stringify_err()
}

#[tauri::command]
pub async fn get_runtime_kernel_loopback_rust_data_plane_hardening_expanded_default_rollout_execution(
    rust_data_plane_hardening_expanded_default_rollout_dry_run_complete_decision: Option<bool>,
    dry_run_review_decision: Option<bool>,
    execution_manifest_lock_decision: Option<bool>,
    rollout_window_start_decision: Option<bool>,
    expanded_profile_cap_enforcement_decision: Option<bool>,
    active_telemetry_watch_decision: Option<bool>,
    rollback_switch_arm_decision: Option<bool>,
    mihomo_fallback_retention_decision: Option<bool>,
    operator_execution_acknowledgement_decision: Option<bool>,
    final_expanded_default_rollout_execution_decision: Option<bool>,
) -> CmdResult<KernelLoopbackRustDataPlaneHardeningExpandedDefaultRolloutExecutionReport> {
    rust_kernel_runtime_data_plane_hardening_expanded_default_rollout_execution(
        rust_data_plane_hardening_expanded_default_rollout_dry_run_complete_decision,
        dry_run_review_decision,
        execution_manifest_lock_decision,
        rollout_window_start_decision,
        expanded_profile_cap_enforcement_decision,
        active_telemetry_watch_decision,
        rollback_switch_arm_decision,
        mihomo_fallback_retention_decision,
        operator_execution_acknowledgement_decision,
        final_expanded_default_rollout_execution_decision,
    )
    .await
    .stringify_err()
}

#[tauri::command]
pub async fn get_runtime_kernel_loopback_rust_data_plane_hardening_expanded_default_rollout_verification(
    rust_data_plane_hardening_expanded_default_rollout_execution_complete_decision: Option<bool>,
    execution_record_review_decision: Option<bool>,
    expanded_profile_traffic_sample_review_decision: Option<bool>,
    fallback_path_sample_verification_decision: Option<bool>,
    rollback_switch_verification_decision: Option<bool>,
    telemetry_health_budget_verification_decision: Option<bool>,
    leak_regression_absence_verification_decision: Option<bool>,
    verification_evidence_archive_decision: Option<bool>,
    final_expanded_default_rollout_verification_decision: Option<bool>,
) -> CmdResult<KernelLoopbackRustDataPlaneHardeningExpandedDefaultRolloutVerificationReport> {
    rust_kernel_runtime_data_plane_hardening_expanded_default_rollout_verification(
        rust_data_plane_hardening_expanded_default_rollout_execution_complete_decision,
        execution_record_review_decision,
        expanded_profile_traffic_sample_review_decision,
        fallback_path_sample_verification_decision,
        rollback_switch_verification_decision,
        telemetry_health_budget_verification_decision,
        leak_regression_absence_verification_decision,
        verification_evidence_archive_decision,
        final_expanded_default_rollout_verification_decision,
    )
    .await
    .stringify_err()
}

#[tauri::command]
pub async fn get_runtime_kernel_loopback_rust_data_plane_hardening_expanded_default_rollout_closeout(
    rust_data_plane_hardening_expanded_default_rollout_verification_complete_decision: Option<bool>,
    verification_review_decision: Option<bool>,
    expanded_rollout_state_documentation_decision: Option<bool>,
    rollback_owner_acknowledgement_decision: Option<bool>,
    fallback_matrix_retention_decision: Option<bool>,
    unsupported_path_boundary_retention_decision: Option<bool>,
    release_notes_update_decision: Option<bool>,
    closeout_evidence_archive_decision: Option<bool>,
    final_expanded_default_rollout_closeout_decision: Option<bool>,
) -> CmdResult<KernelLoopbackRustDataPlaneHardeningExpandedDefaultRolloutCloseoutReport> {
    rust_kernel_runtime_data_plane_hardening_expanded_default_rollout_closeout(
        rust_data_plane_hardening_expanded_default_rollout_verification_complete_decision,
        verification_review_decision,
        expanded_rollout_state_documentation_decision,
        rollback_owner_acknowledgement_decision,
        fallback_matrix_retention_decision,
        unsupported_path_boundary_retention_decision,
        release_notes_update_decision,
        closeout_evidence_archive_decision,
        final_expanded_default_rollout_closeout_decision,
    )
    .await
    .stringify_err()
}

#[tauri::command]
pub async fn get_runtime_kernel_loopback_rust_data_plane_hardening_mihomo_fallback_retirement_guard(
    rust_data_plane_hardening_expanded_default_rollout_closeout_complete_decision: Option<bool>,
    expanded_rollout_closeout_review_decision: Option<bool>,
    protocol_parity_scope_lock_decision: Option<bool>,
    tun_parity_scope_lock_decision: Option<bool>,
    adapter_parity_scope_lock_decision: Option<bool>,
    dns_parity_scope_lock_decision: Option<bool>,
    emergency_rollback_retention_decision: Option<bool>,
    cross_platform_drill_plan_definition_decision: Option<bool>,
    operator_retirement_acknowledgement_decision: Option<bool>,
    final_mihomo_fallback_retirement_guard_decision: Option<bool>,
) -> CmdResult<KernelLoopbackRustDataPlaneHardeningMihomoFallbackRetirementGuardReport> {
    rust_kernel_runtime_data_plane_hardening_mihomo_fallback_retirement_guard(
        rust_data_plane_hardening_expanded_default_rollout_closeout_complete_decision,
        expanded_rollout_closeout_review_decision,
        protocol_parity_scope_lock_decision,
        tun_parity_scope_lock_decision,
        adapter_parity_scope_lock_decision,
        dns_parity_scope_lock_decision,
        emergency_rollback_retention_decision,
        cross_platform_drill_plan_definition_decision,
        operator_retirement_acknowledgement_decision,
        final_mihomo_fallback_retirement_guard_decision,
    )
    .await
    .stringify_err()
}

#[tauri::command]
pub async fn get_runtime_kernel_loopback_rust_data_plane_hardening_mihomo_fallback_retirement_dry_run(
    rust_data_plane_hardening_mihomo_fallback_retirement_guard_complete_decision: Option<bool>,
    guard_review_decision: Option<bool>,
    parity_manifest_replay_decision: Option<bool>,
    cross_platform_rollback_rehearsal_decision: Option<bool>,
    fallback_dependency_inventory_replay_decision: Option<bool>,
    emergency_recovery_rehearsal_decision: Option<bool>,
    production_forwarding_unchanged_verification_decision: Option<bool>,
    dry_run_evidence_archive_decision: Option<bool>,
    final_mihomo_fallback_retirement_dry_run_decision: Option<bool>,
) -> CmdResult<KernelLoopbackRustDataPlaneHardeningMihomoFallbackRetirementDryRunReport> {
    rust_kernel_runtime_data_plane_hardening_mihomo_fallback_retirement_dry_run(
        rust_data_plane_hardening_mihomo_fallback_retirement_guard_complete_decision,
        guard_review_decision,
        parity_manifest_replay_decision,
        cross_platform_rollback_rehearsal_decision,
        fallback_dependency_inventory_replay_decision,
        emergency_recovery_rehearsal_decision,
        production_forwarding_unchanged_verification_decision,
        dry_run_evidence_archive_decision,
        final_mihomo_fallback_retirement_dry_run_decision,
    )
    .await
    .stringify_err()
}

#[tauri::command]
pub async fn get_runtime_kernel_loopback_rust_data_plane_hardening_mihomo_fallback_retirement_readiness(
    rust_data_plane_hardening_mihomo_fallback_retirement_dry_run_complete_decision: Option<bool>,
    dry_run_review_decision: Option<bool>,
    protocol_parity_evidence_archive_decision: Option<bool>,
    tun_parity_evidence_archive_decision: Option<bool>,
    adapter_parity_evidence_archive_decision: Option<bool>,
    dns_parity_evidence_archive_decision: Option<bool>,
    soak_evidence_archive_decision: Option<bool>,
    emergency_rollback_owner_acknowledgement_decision: Option<bool>,
    final_mihomo_fallback_retirement_readiness_decision: Option<bool>,
) -> CmdResult<KernelLoopbackRustDataPlaneHardeningMihomoFallbackRetirementReadinessReport> {
    rust_kernel_runtime_data_plane_hardening_mihomo_fallback_retirement_readiness(
        rust_data_plane_hardening_mihomo_fallback_retirement_dry_run_complete_decision,
        dry_run_review_decision,
        protocol_parity_evidence_archive_decision,
        tun_parity_evidence_archive_decision,
        adapter_parity_evidence_archive_decision,
        dns_parity_evidence_archive_decision,
        soak_evidence_archive_decision,
        emergency_rollback_owner_acknowledgement_decision,
        final_mihomo_fallback_retirement_readiness_decision,
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
