use crate::cmd::{CmdResult, StringifyErr};
use crate::core::dns_config_explain::{
    DnsConfigExplainReport, DnsConfigProbePlan, explain_dns_config as build_dns_config_explain,
    plan_dns_probe as build_dns_probe_plan,
};
use crate::core::dns_runtime::{
    DnsDefaultRuntimeExpandedControlPlaneCompletionReport, DnsDefaultRuntimeExpandedHoldPolicyReport,
    DnsDefaultRuntimeExpandedLifecycleCloseoutReport, DnsDefaultRuntimeExpandedOptInExecutionGateReport,
    DnsDefaultRuntimeExpandedOptInExecutionPreflightReport, DnsDefaultRuntimeExpandedOptInExecutionReport,
    DnsDefaultRuntimeExpandedPostExecutionObservedVerificationReport, DnsDefaultRuntimeExpandedReverifyHistoryReport,
    DnsDefaultRuntimeExpandedReverifyReport, DnsDefaultRuntimeExpandedRollbackDrillReport,
    DnsDefaultRuntimeExpandedRollbackReport, DnsDefaultRuntimeExpandedStabilityGateReport,
    DnsDefaultRuntimeLimitedOptInExecutionReport, DnsDefaultRuntimeLimitedRollbackReport,
    DnsDefaultRuntimeOptInExecutionGuardReport, DnsDefaultRuntimeOptInExecutorPreflightReport,
    DnsDefaultRuntimeOptInSwitchGuardReport, DnsDefaultRuntimePostExecutionObservedVerificationReport,
    DnsDefaultRuntimeReadinessReport, DnsDefaultRuntimeRollbackDrillReport, DnsDefaultRuntimeShadowEvidenceReport,
    DnsHealthCheckResult, DnsProtocol, DnsQueryResult, DnsResolverPlan, DnsResolverRuntimeProbeReport,
    DnsResolverRuntimeQueryReport, DnsServerProviderHealthReport, DnsServerProviderKind, DnsServerProviderRegistration,
    RustDnsFakeIpCacheRuntimeReport, RustDnsFakeIpRuntimeReport, RustDnsFallbackFilterGeoipRuntimeReport,
    RustDnsFallbackFilterRuntimeReport, RustDnsNameserverPolicyRuntimeReport, RustDnsRuntimeParityReport,
    build_dns_resolver_plan as build_resolver_plan, dns_controlled_runtime_probe as run_dns_controlled_runtime_probe,
    dns_default_runtime_expanded_control_plane_completion as build_dns_default_runtime_expanded_control_plane_completion,
    dns_default_runtime_expanded_hold_policy as build_dns_default_runtime_expanded_hold_policy,
    dns_default_runtime_expanded_lifecycle_closeout as build_dns_default_runtime_expanded_lifecycle_closeout,
    dns_default_runtime_expanded_opt_in_execution as build_dns_default_runtime_expanded_opt_in_execution,
    dns_default_runtime_expanded_opt_in_execution_gate as build_dns_default_runtime_expanded_opt_in_execution_gate,
    dns_default_runtime_expanded_opt_in_execution_preflight as build_dns_default_runtime_expanded_opt_in_execution_preflight,
    dns_default_runtime_expanded_post_execution_observed_verification as build_dns_default_runtime_expanded_post_execution_observed_verification,
    dns_default_runtime_expanded_reverify as build_dns_default_runtime_expanded_reverify,
    dns_default_runtime_expanded_reverify_history as build_dns_default_runtime_expanded_reverify_history,
    dns_default_runtime_expanded_rollback as build_dns_default_runtime_expanded_rollback,
    dns_default_runtime_expanded_rollback_drill as build_dns_default_runtime_expanded_rollback_drill,
    dns_default_runtime_expanded_stability_gate as build_dns_default_runtime_expanded_stability_gate,
    dns_default_runtime_limited_opt_in_execution as build_dns_default_runtime_limited_opt_in_execution,
    dns_default_runtime_limited_rollback as build_dns_default_runtime_limited_rollback,
    dns_default_runtime_opt_in_execution_guard as build_dns_default_runtime_opt_in_execution_guard,
    dns_default_runtime_opt_in_executor_preflight as build_dns_default_runtime_opt_in_executor_preflight,
    dns_default_runtime_opt_in_switch_guard as build_dns_default_runtime_opt_in_switch_guard,
    dns_default_runtime_post_execution_observed_verification as build_dns_default_runtime_post_execution_observed_verification,
    dns_default_runtime_readiness as build_dns_default_runtime_readiness,
    dns_default_runtime_rollback_drill as build_dns_default_runtime_rollback_drill,
    dns_default_runtime_shadow_evidence as build_dns_default_runtime_shadow_evidence,
    dns_health_check as build_dns_health_check, dns_query as build_dns_query,
    dns_runtime_query as run_dns_runtime_query, list_dns_server_provider_registrations, probe_dns_server_provider,
    rust_dns_fake_ip_cache_runtime_execution as build_rust_dns_fake_ip_cache_runtime_execution,
    rust_dns_fake_ip_runtime_execution as build_rust_dns_fake_ip_runtime_execution,
    rust_dns_fallback_filter_geoip_runtime_execution as build_rust_dns_fallback_filter_geoip_runtime_execution,
    rust_dns_fallback_filter_runtime_execution as build_rust_dns_fallback_filter_runtime_execution,
    rust_dns_nameserver_policy_runtime_execution as build_rust_dns_nameserver_policy_runtime_execution,
    rust_dns_runtime_parity as build_rust_dns_runtime_parity,
    rust_dns_runtime_parity_rollback as build_rust_dns_runtime_parity_rollback,
};
use log::error;

/// DNS æŸ¥è¯¢
///
/// æ”¯æŒè‡ªå®šä¹‰ DNS æœåŠ¡å™¨å’Œåè®®ï¼ˆUDP/TCP/DoH/DoTï¼‰
#[tauri::command]
pub async fn dns_query(
    domain: String,
    server: Option<String>,
    protocol: Option<DnsProtocol>,
) -> CmdResult<DnsQueryResult> {
    build_dns_query(domain, server, protocol).await.stringify_err()
}

/// DNS æœåŠ¡å™¨å¥åº·æ£€æŸ¥
///
/// ä½¿ç”¨æŒ‡å®šçš„æµ‹è¯•åŸŸåæ£€æŸ¥ DNS æœåŠ¡å™¨çš„å¥åº·çŠ¶æ€
#[tauri::command]
pub async fn dns_health_check(
    server: String,
    test_domain: Option<String>,
    protocol: Option<DnsProtocol>,
) -> CmdResult<DnsHealthCheckResult> {
    build_dns_health_check(server, test_domain, protocol)
        .await
        .stringify_err()
}

/// æ‰¹é‡ DNS æŸ¥è¯¢
#[tauri::command]
pub async fn dns_batch_query(
    domains: Vec<String>,
    server: Option<String>,
    protocol: Option<DnsProtocol>,
) -> CmdResult<Vec<DnsQueryResult>> {
    let mut results = Vec::new();

    for domain in domains {
        match build_dns_query(domain, server.clone(), protocol.clone()).await {
            Ok(result) => results.push(result),
            Err(e) => {
                error!("DNS batch query error: {}", e);
            }
        }
    }

    Ok(results)
}

/// æ‰¹é‡ DNS å¥åº·æ£€æŸ¥
#[tauri::command]
pub async fn dns_batch_health_check(
    servers: Vec<String>,
    test_domain: Option<String>,
    protocol: Option<DnsProtocol>,
) -> CmdResult<Vec<DnsHealthCheckResult>> {
    let mut results = Vec::new();
    let domain = test_domain.clone();

    for server in servers {
        match build_dns_health_check(server, domain.clone(), protocol.clone()).await {
            Ok(result) => results.push(result),
            Err(e) => {
                error!("DNS batch health check error: {}", e);
            }
        }
    }

    Ok(results)
}

#[tauri::command]
pub async fn dns_get_provider_registrations() -> CmdResult<Vec<DnsServerProviderRegistration>> {
    Ok(list_dns_server_provider_registrations())
}

#[tauri::command]
pub async fn dns_probe_provider(
    kind: DnsServerProviderKind,
    protocol: Option<DnsProtocol>,
    test_domain: Option<String>,
) -> CmdResult<DnsServerProviderHealthReport> {
    Ok(probe_dns_server_provider(kind, protocol, test_domain.as_deref()).await)
}

#[tauri::command]
pub async fn dns_explain_config(yaml: String, test_domain: Option<String>) -> CmdResult<DnsConfigExplainReport> {
    build_dns_config_explain(&yaml, test_domain.as_deref()).stringify_err()
}

#[tauri::command]
pub async fn dns_plan_probe(yaml: String, test_domain: Option<String>) -> CmdResult<DnsConfigProbePlan> {
    build_dns_probe_plan(&yaml, test_domain.as_deref()).stringify_err()
}

#[tauri::command]
pub async fn dns_build_resolver_plan(yaml: String) -> CmdResult<DnsResolverPlan> {
    build_resolver_plan(&yaml).stringify_err()
}

#[tauri::command]
pub async fn dns_runtime_query(yaml: String, domain: String) -> CmdResult<DnsResolverRuntimeQueryReport> {
    run_dns_runtime_query(&yaml, domain).await.stringify_err()
}

#[tauri::command]
pub async fn dns_controlled_runtime_probe(
    yaml: String,
    test_domain: Option<String>,
) -> CmdResult<DnsResolverRuntimeProbeReport> {
    run_dns_controlled_runtime_probe(&yaml, test_domain)
        .await
        .stringify_err()
}

#[tauri::command]
pub async fn rust_dns_runtime_parity(
    yaml: String,
    test_domain: Option<String>,
    explicit_opt_in: bool,
    apply_runtime: bool,
) -> CmdResult<RustDnsRuntimeParityReport> {
    build_rust_dns_runtime_parity(yaml, test_domain, explicit_opt_in, apply_runtime)
        .await
        .stringify_err()
}

#[tauri::command]
pub async fn rust_dns_fake_ip_runtime_execution(
    yaml: String,
    domain: String,
    explicit_opt_in: bool,
) -> CmdResult<RustDnsFakeIpRuntimeReport> {
    build_rust_dns_fake_ip_runtime_execution(yaml, domain, explicit_opt_in)
        .await
        .stringify_err()
}

#[tauri::command]
pub async fn rust_dns_fake_ip_cache_runtime_execution(
    yaml: String,
    domain: String,
    explicit_opt_in: bool,
) -> CmdResult<RustDnsFakeIpCacheRuntimeReport> {
    build_rust_dns_fake_ip_cache_runtime_execution(yaml, domain, explicit_opt_in)
        .await
        .stringify_err()
}

#[tauri::command]
pub async fn rust_dns_fallback_filter_runtime_execution(
    yaml: String,
    domain: String,
    candidate_ip: String,
    explicit_opt_in: bool,
) -> CmdResult<RustDnsFallbackFilterRuntimeReport> {
    build_rust_dns_fallback_filter_runtime_execution(yaml, domain, candidate_ip, explicit_opt_in)
        .await
        .stringify_err()
}

#[tauri::command]
pub async fn rust_dns_fallback_filter_geoip_runtime_execution(
    yaml: String,
    domain: String,
    candidate_ip: String,
    explicit_opt_in: bool,
) -> CmdResult<RustDnsFallbackFilterGeoipRuntimeReport> {
    build_rust_dns_fallback_filter_geoip_runtime_execution(yaml, domain, candidate_ip, explicit_opt_in)
        .await
        .stringify_err()
}

#[tauri::command]
pub async fn rust_dns_nameserver_policy_runtime_execution(
    yaml: String,
    domain: String,
    explicit_opt_in: bool,
) -> CmdResult<RustDnsNameserverPolicyRuntimeReport> {
    build_rust_dns_nameserver_policy_runtime_execution(yaml, domain, explicit_opt_in)
        .await
        .stringify_err()
}

#[tauri::command]
pub async fn rust_dns_runtime_parity_rollback() -> CmdResult<RustDnsRuntimeParityReport> {
    build_rust_dns_runtime_parity_rollback().await.stringify_err()
}

#[tauri::command]
pub async fn dns_default_runtime_readiness(
    yaml: Option<String>,
    probe_report: Option<DnsResolverRuntimeProbeReport>,
) -> CmdResult<DnsDefaultRuntimeReadinessReport> {
    build_dns_default_runtime_readiness(yaml, probe_report)
        .await
        .stringify_err()
}

#[tauri::command]
pub async fn dns_default_runtime_shadow_evidence(
    yaml: Option<String>,
    domain: Option<String>,
) -> CmdResult<DnsDefaultRuntimeShadowEvidenceReport> {
    build_dns_default_runtime_shadow_evidence(yaml, domain)
        .await
        .stringify_err()
}

#[tauri::command]
pub async fn dns_default_runtime_opt_in_switch_guard(
    yaml: Option<String>,
    domain: Option<String>,
    explicit_opt_in: bool,
) -> CmdResult<DnsDefaultRuntimeOptInSwitchGuardReport> {
    build_dns_default_runtime_opt_in_switch_guard(yaml, domain, explicit_opt_in)
        .await
        .stringify_err()
}

#[tauri::command]
pub async fn dns_default_runtime_opt_in_executor_preflight(
    yaml: Option<String>,
    domain: Option<String>,
    explicit_opt_in: bool,
) -> CmdResult<DnsDefaultRuntimeOptInExecutorPreflightReport> {
    build_dns_default_runtime_opt_in_executor_preflight(yaml, domain, explicit_opt_in)
        .await
        .stringify_err()
}

#[tauri::command]
pub async fn dns_default_runtime_opt_in_execution_guard(
    yaml: Option<String>,
    domain: Option<String>,
    explicit_opt_in: bool,
) -> CmdResult<DnsDefaultRuntimeOptInExecutionGuardReport> {
    build_dns_default_runtime_opt_in_execution_guard(yaml, domain, explicit_opt_in)
        .await
        .stringify_err()
}

#[tauri::command]
pub async fn dns_default_runtime_limited_opt_in_execution(
    yaml: Option<String>,
    domain: Option<String>,
    explicit_opt_in: bool,
) -> CmdResult<DnsDefaultRuntimeLimitedOptInExecutionReport> {
    build_dns_default_runtime_limited_opt_in_execution(yaml, domain, explicit_opt_in)
        .await
        .stringify_err()
}

#[tauri::command]
pub async fn dns_default_runtime_rollback_drill() -> CmdResult<DnsDefaultRuntimeRollbackDrillReport> {
    build_dns_default_runtime_rollback_drill().await.stringify_err()
}

#[tauri::command]
pub async fn dns_default_runtime_post_execution_observed_verification(
    yaml: Option<String>,
    domain: Option<String>,
) -> CmdResult<DnsDefaultRuntimePostExecutionObservedVerificationReport> {
    build_dns_default_runtime_post_execution_observed_verification(yaml, domain)
        .await
        .stringify_err()
}

#[tauri::command]
pub async fn dns_default_runtime_expanded_opt_in_execution_gate(
    yaml: Option<String>,
    domain: Option<String>,
    explicit_opt_in: bool,
) -> CmdResult<DnsDefaultRuntimeExpandedOptInExecutionGateReport> {
    build_dns_default_runtime_expanded_opt_in_execution_gate(yaml, domain, explicit_opt_in)
        .await
        .stringify_err()
}

#[tauri::command]
pub async fn dns_default_runtime_expanded_opt_in_execution_preflight(
    yaml: Option<String>,
    domain: Option<String>,
    explicit_opt_in: bool,
) -> CmdResult<DnsDefaultRuntimeExpandedOptInExecutionPreflightReport> {
    build_dns_default_runtime_expanded_opt_in_execution_preflight(yaml, domain, explicit_opt_in)
        .await
        .stringify_err()
}

#[tauri::command]
pub async fn dns_default_runtime_expanded_opt_in_execution(
    yaml: Option<String>,
    domain: Option<String>,
    explicit_opt_in: bool,
) -> CmdResult<DnsDefaultRuntimeExpandedOptInExecutionReport> {
    build_dns_default_runtime_expanded_opt_in_execution(yaml, domain, explicit_opt_in)
        .await
        .stringify_err()
}

#[tauri::command]
pub async fn dns_default_runtime_expanded_rollback() -> CmdResult<DnsDefaultRuntimeExpandedRollbackReport> {
    build_dns_default_runtime_expanded_rollback().await.stringify_err()
}

#[tauri::command]
pub async fn dns_default_runtime_expanded_rollback_drill() -> CmdResult<DnsDefaultRuntimeExpandedRollbackDrillReport> {
    build_dns_default_runtime_expanded_rollback_drill()
        .await
        .stringify_err()
}

#[tauri::command]
pub async fn dns_default_runtime_expanded_post_execution_observed_verification(
    yaml: Option<String>,
    domain: Option<String>,
) -> CmdResult<DnsDefaultRuntimeExpandedPostExecutionObservedVerificationReport> {
    build_dns_default_runtime_expanded_post_execution_observed_verification(yaml, domain)
        .await
        .stringify_err()
}

#[tauri::command]
pub async fn dns_default_runtime_expanded_stability_gate(
    yaml: Option<String>,
    domain: Option<String>,
    explicit_opt_in: bool,
) -> CmdResult<DnsDefaultRuntimeExpandedStabilityGateReport> {
    build_dns_default_runtime_expanded_stability_gate(yaml, domain, explicit_opt_in)
        .await
        .stringify_err()
}

#[tauri::command]
pub async fn dns_default_runtime_expanded_hold_policy(
    yaml: Option<String>,
    domain: Option<String>,
    explicit_opt_in: bool,
) -> CmdResult<DnsDefaultRuntimeExpandedHoldPolicyReport> {
    build_dns_default_runtime_expanded_hold_policy(yaml, domain, explicit_opt_in)
        .await
        .stringify_err()
}

#[tauri::command]
pub async fn dns_default_runtime_expanded_reverify(
    yaml: Option<String>,
    domain: Option<String>,
    explicit_opt_in: bool,
) -> CmdResult<DnsDefaultRuntimeExpandedReverifyReport> {
    build_dns_default_runtime_expanded_reverify(yaml, domain, explicit_opt_in)
        .await
        .stringify_err()
}

#[tauri::command]
pub async fn dns_default_runtime_expanded_reverify_history() -> CmdResult<DnsDefaultRuntimeExpandedReverifyHistoryReport>
{
    build_dns_default_runtime_expanded_reverify_history()
        .await
        .stringify_err()
}

#[tauri::command]
pub async fn dns_default_runtime_expanded_lifecycle_closeout()
-> CmdResult<DnsDefaultRuntimeExpandedLifecycleCloseoutReport> {
    build_dns_default_runtime_expanded_lifecycle_closeout()
        .await
        .stringify_err()
}

#[tauri::command]
pub async fn dns_default_runtime_expanded_control_plane_completion()
-> CmdResult<DnsDefaultRuntimeExpandedControlPlaneCompletionReport> {
    build_dns_default_runtime_expanded_control_plane_completion()
        .await
        .stringify_err()
}

#[tauri::command]
pub async fn dns_default_runtime_limited_rollback() -> CmdResult<DnsDefaultRuntimeLimitedRollbackReport> {
    build_dns_default_runtime_limited_rollback().await.stringify_err()
}
