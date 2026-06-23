use crate::core::dns_runtime::DnsDefaultRuntimeShadowEvidenceReport;
use serde::{Deserialize, Serialize};
use smartstring::alias::String;
use std::{
    collections::BTreeMap,
    sync::{Arc, atomic::AtomicU64},
};
use tokio::sync::oneshot;

pub(super) struct KernelIsolatedTestListenerState {
    pub(super) port: u16,
    pub(super) started_at_epoch_ms: u64,
    pub(super) accepted_connections: Arc<AtomicU64>,
    pub(super) stop_tx: oneshot::Sender<()>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KernelReplacementBlocker {
    pub area: String,
    pub reason: String,
    pub required_next_step: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KernelRuntimeStatus {
    pub runtime_id: String,
    pub active_kernel: String,
    pub controller_transport: String,
    pub mutates_runtime: bool,
    pub mihomo_fallback: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KernelRuntimePreflightReport {
    pub runtime_id: String,
    pub artifact_id: Option<String>,
    pub mutates_runtime: bool,
    pub can_apply_with_rust_kernel: bool,
    pub mihomo_fallback: bool,
    pub facts: Vec<String>,
    pub blocked_replacement_areas: Vec<KernelReplacementBlocker>,
    pub next_safe_batch: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KernelShadowComponent {
    pub component: String,
    pub kernel_area: String,
    pub status: String,
    pub mutates_runtime: bool,
    pub live_execution_allowed: bool,
    pub evidence: Vec<String>,
    pub next_step: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KernelShadowComponentsReport {
    pub runtime_id: String,
    pub active_kernel: String,
    pub mutates_runtime: bool,
    pub components: Vec<KernelShadowComponent>,
    pub live_execution_blockers: Vec<KernelReplacementBlocker>,
    pub next_safe_batch: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KernelDnsShadowEvidenceReport {
    pub runtime_id: String,
    pub component: String,
    pub kernel_area: String,
    pub mutates_runtime: bool,
    pub live_execution_allowed: bool,
    pub evidence: DnsDefaultRuntimeShadowEvidenceReport,
    pub blockers: Vec<String>,
    pub next_safe_batch: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KernelRuleShadowRule {
    pub index: i32,
    pub rule_type: String,
    pub payload: String,
    pub proxy: String,
    pub source: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KernelRuleShadowSample {
    pub sample_index: usize,
    pub app_rule: Option<KernelRuleShadowRule>,
    pub mihomo_rule: Option<KernelRuleShadowRule>,
    pub matched: bool,
    pub mismatch_reason: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KernelRuleShadowEvidenceReport {
    pub runtime_id: String,
    pub component: String,
    pub kernel_area: String,
    pub mutates_runtime: bool,
    pub live_execution_allowed: bool,
    pub status: String,
    pub app_rule_count: usize,
    pub mihomo_rule_count: usize,
    pub compared_sample_size: usize,
    pub matched_sample_count: usize,
    pub mismatched_sample_count: usize,
    pub samples: Vec<KernelRuleShadowSample>,
    pub blockers: Vec<String>,
    pub warnings: Vec<String>,
    pub facts: Vec<String>,
    pub next_safe_batch: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KernelAdapterCapabilityEntry {
    pub proxy_type: String,
    pub app_count: usize,
    pub mihomo_count: usize,
    pub inventory_matched: bool,
    pub rust_shadow_supported: bool,
    pub live_execution_allowed: bool,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KernelAdapterCapabilityReport {
    pub runtime_id: String,
    pub component: String,
    pub kernel_area: String,
    pub mutates_runtime: bool,
    pub live_execution_allowed: bool,
    pub app_proxy_count: usize,
    pub mihomo_proxy_count: usize,
    pub capabilities: Vec<KernelAdapterCapabilityEntry>,
    pub blockers: Vec<String>,
    pub warnings: Vec<String>,
    pub facts: Vec<String>,
    pub next_safe_batch: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KernelConnectionSessionSample {
    pub sample_index: usize,
    pub network: String,
    pub connection_type: String,
    pub chain_len: usize,
    pub provider_chain_len: usize,
    pub has_host: bool,
    pub has_process: bool,
    pub has_remote_destination: bool,
    pub rule: String,
    pub uploaded_bytes: u64,
    pub downloaded_bytes: u64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KernelConnectionSessionShadowReport {
    pub runtime_id: String,
    pub component: String,
    pub kernel_area: String,
    pub mutates_runtime: bool,
    pub live_execution_allowed: bool,
    pub connection_count: usize,
    pub upload_total: u64,
    pub download_total: u64,
    pub memory: u32,
    pub network_counts: BTreeMap<String, usize>,
    pub connection_type_counts: BTreeMap<String, usize>,
    pub rule_counts: BTreeMap<String, usize>,
    pub samples: Vec<KernelConnectionSessionSample>,
    pub blockers: Vec<String>,
    pub warnings: Vec<String>,
    pub facts: Vec<String>,
    pub next_safe_batch: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KernelIsolatedListenerPortCheck {
    pub host: String,
    pub port: u16,
    pub available: bool,
    pub conflicts_with_runtime_port: bool,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KernelIsolatedListenerPreflightReport {
    pub runtime_id: String,
    pub component: String,
    pub kernel_area: String,
    pub mutates_runtime: bool,
    pub live_execution_allowed: bool,
    pub requested_host: String,
    pub requested_port: u16,
    pub can_start_after_opt_in: bool,
    pub port_check: KernelIsolatedListenerPortCheck,
    pub runtime_ports: BTreeMap<String, u16>,
    pub system_proxy_enabled: bool,
    pub tun_enabled: bool,
    pub blockers: Vec<String>,
    pub warnings: Vec<String>,
    pub facts: Vec<String>,
    pub next_safe_batch: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KernelIsolatedTestListenerStatus {
    pub runtime_id: String,
    pub component: String,
    pub kernel_area: String,
    pub mutates_runtime: bool,
    pub live_execution_allowed: bool,
    pub running: bool,
    pub host: String,
    pub port: Option<u16>,
    pub started_at_epoch_ms: Option<u64>,
    pub accepted_connections: u64,
    pub loopback_only: bool,
    pub default_route: bool,
    pub forwards_traffic: bool,
    pub mihomo_fallback: bool,
    pub blockers: Vec<String>,
    pub warnings: Vec<String>,
    pub facts: Vec<String>,
    pub next_safe_batch: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KernelIsolatedTestListenerSmokeEvidenceReport {
    pub runtime_id: String,
    pub component: String,
    pub kernel_area: String,
    pub mutates_runtime: bool,
    pub live_execution_allowed: bool,
    pub requested_host: String,
    pub requested_port: u16,
    pub started_by_smoke: bool,
    pub response_status: Option<String>,
    pub accepted_connections_before: u64,
    pub accepted_connections_after: u64,
    pub status_incremented: bool,
    pub stopped_after_smoke: bool,
    pub system_proxy_unchanged: bool,
    pub tun_unchanged: bool,
    pub runtime_config_unchanged: bool,
    pub default_route: bool,
    pub forwards_traffic: bool,
    pub mihomo_fallback: bool,
    pub passed: bool,
    pub blockers: Vec<String>,
    pub warnings: Vec<String>,
    pub facts: Vec<String>,
    pub next_safe_batch: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KernelLoopbackDnsPortCheck {
    pub host: String,
    pub port: u16,
    pub udp_available: bool,
    pub tcp_available: bool,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KernelLoopbackDnsPreflightReport {
    pub runtime_id: String,
    pub component: String,
    pub kernel_area: String,
    pub mutates_runtime: bool,
    pub live_execution_allowed: bool,
    pub requested_host: String,
    pub requested_port: u16,
    pub can_start_after_opt_in: bool,
    pub port_check: KernelLoopbackDnsPortCheck,
    pub runtime_dns_present: bool,
    pub app_dns_settings_enabled: bool,
    pub system_proxy_enabled: bool,
    pub tun_enabled: bool,
    pub default_route: bool,
    pub forwards_traffic: bool,
    pub mihomo_fallback: bool,
    pub blockers: Vec<String>,
    pub warnings: Vec<String>,
    pub facts: Vec<String>,
    pub next_safe_batch: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KernelLoopbackDnsSmokeEvidenceReport {
    pub runtime_id: String,
    pub component: String,
    pub kernel_area: String,
    pub mutates_runtime: bool,
    pub live_execution_allowed: bool,
    pub requested_host: String,
    pub requested_port: u16,
    pub query_name: String,
    pub udp_bound: bool,
    pub local_response_received: bool,
    pub response_address: Option<String>,
    pub system_proxy_unchanged: bool,
    pub tun_unchanged: bool,
    pub runtime_config_unchanged: bool,
    pub default_route: bool,
    pub forwards_traffic: bool,
    pub mihomo_fallback: bool,
    pub passed: bool,
    pub blockers: Vec<String>,
    pub warnings: Vec<String>,
    pub facts: Vec<String>,
    pub next_safe_batch: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KernelLoopbackForwardingPortCheck {
    pub host: String,
    pub listener_port: u16,
    pub target_port: u16,
    pub listener_available: bool,
    pub target_available: bool,
    pub target_loopback_only: bool,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KernelLoopbackForwardingPreflightReport {
    pub runtime_id: String,
    pub component: String,
    pub kernel_area: String,
    pub mutates_runtime: bool,
    pub live_execution_allowed: bool,
    pub requested_host: String,
    pub listener_port: u16,
    pub target_port: u16,
    pub can_start_after_opt_in: bool,
    pub port_check: KernelLoopbackForwardingPortCheck,
    pub system_proxy_enabled: bool,
    pub tun_enabled: bool,
    pub default_route: bool,
    pub forwards_traffic: bool,
    pub outbound_adapters_allowed: bool,
    pub mihomo_fallback: bool,
    pub blockers: Vec<String>,
    pub warnings: Vec<String>,
    pub facts: Vec<String>,
    pub next_safe_batch: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KernelLoopbackForwardingSmokeEvidenceReport {
    pub runtime_id: String,
    pub component: String,
    pub kernel_area: String,
    pub mutates_runtime: bool,
    pub live_execution_allowed: bool,
    pub requested_host: String,
    pub listener_port: u16,
    pub target_port: u16,
    pub request_path: String,
    pub listener_accepted: bool,
    pub target_received: bool,
    pub response_status: Option<String>,
    pub bytes_from_client: u64,
    pub bytes_from_target: u64,
    pub loopback_forwarded: bool,
    pub system_proxy_unchanged: bool,
    pub tun_unchanged: bool,
    pub runtime_config_unchanged: bool,
    pub default_route: bool,
    pub forwards_traffic: bool,
    pub outbound_adapters_used: bool,
    pub mihomo_fallback: bool,
    pub passed: bool,
    pub blockers: Vec<String>,
    pub warnings: Vec<String>,
    pub facts: Vec<String>,
    pub next_safe_batch: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KernelLoopbackForwardingRollbackDrillReport {
    pub runtime_id: String,
    pub component: String,
    pub kernel_area: String,
    pub mutates_runtime: bool,
    pub live_execution_allowed: bool,
    pub listener_port: u16,
    pub target_port: u16,
    pub smoke_passed: bool,
    pub ports_released: bool,
    pub post_preflight: KernelLoopbackForwardingPreflightReport,
    pub system_proxy_unchanged: bool,
    pub tun_unchanged: bool,
    pub runtime_config_unchanged: bool,
    pub default_route: bool,
    pub forwards_traffic: bool,
    pub outbound_adapters_used: bool,
    pub mihomo_fallback: bool,
    pub passed: bool,
    pub blockers: Vec<String>,
    pub warnings: Vec<String>,
    pub facts: Vec<String>,
    pub next_safe_batch: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KernelLoopbackForwardingLeakCheckReport {
    pub runtime_id: String,
    pub component: String,
    pub kernel_area: String,
    pub mutates_runtime: bool,
    pub live_execution_allowed: bool,
    pub listener_port: u16,
    pub target_port: u16,
    pub listener_port_released: bool,
    pub target_port_released: bool,
    pub isolated_test_listener_running: bool,
    pub preflight: KernelLoopbackForwardingPreflightReport,
    pub default_route: bool,
    pub forwards_traffic: bool,
    pub outbound_adapters_used: bool,
    pub mihomo_fallback: bool,
    pub passed: bool,
    pub blockers: Vec<String>,
    pub warnings: Vec<String>,
    pub facts: Vec<String>,
    pub next_safe_batch: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum RustProtocolForwardingSubsetStatus {
    Ready,
    Running,
    Stopped,
    Blocked,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RustProtocolForwardingSubsetAccounting {
    pub accepted_connections: u64,
    pub completed_connections: u64,
    pub failed_connections: u64,
    pub bytes_from_client: u64,
    pub bytes_from_target: u64,
    pub last_error: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RustProtocolForwardingSubsetPreflightReport {
    pub runtime_id: String,
    pub component: String,
    pub kernel_area: String,
    pub status: RustProtocolForwardingSubsetStatus,
    pub reason: String,
    pub listener_host: String,
    pub listener_port: u16,
    pub target_host: String,
    pub target_port: u16,
    pub can_start_after_opt_in: bool,
    pub explicit_opt_in_required: bool,
    pub loopback_only: bool,
    pub supported_protocols: Vec<String>,
    pub mutates_runtime: bool,
    pub live_execution_allowed: bool,
    pub default_route: bool,
    pub forwards_traffic: bool,
    pub outbound_adapters_used: bool,
    pub mihomo_fallback: bool,
    pub blockers: Vec<String>,
    pub warnings: Vec<String>,
    pub facts: Vec<String>,
    pub next_safe_batch: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RustProtocolForwardingSubsetStatusReport {
    pub runtime_id: String,
    pub component: String,
    pub kernel_area: String,
    pub status: RustProtocolForwardingSubsetStatus,
    pub reason: String,
    pub running: bool,
    pub listener_host: String,
    pub listener_port: Option<u16>,
    pub target_host: Option<String>,
    pub target_port: Option<u16>,
    pub started_at_epoch_ms: Option<u64>,
    pub accounting: RustProtocolForwardingSubsetAccounting,
    pub loopback_only: bool,
    pub supported_protocols: Vec<String>,
    pub mutates_runtime: bool,
    pub live_execution_allowed: bool,
    pub default_route: bool,
    pub forwards_traffic: bool,
    pub outbound_adapters_used: bool,
    pub mihomo_fallback: bool,
    pub blockers: Vec<String>,
    pub warnings: Vec<String>,
    pub facts: Vec<String>,
    pub next_safe_batch: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RustProtocolForwardingSubsetStartReport {
    pub preflight: RustProtocolForwardingSubsetPreflightReport,
    pub status: RustProtocolForwardingSubsetStatusReport,
    pub explicit_opt_in: bool,
    pub started: bool,
    pub blockers: Vec<String>,
    pub warnings: Vec<String>,
    pub facts: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RustProtocolForwardingSubsetStopReport {
    pub status: RustProtocolForwardingSubsetStatus,
    pub reason: String,
    pub stopped: bool,
    pub previous_status: RustProtocolForwardingSubsetStatusReport,
    pub after_status: RustProtocolForwardingSubsetStatusReport,
    pub blockers: Vec<String>,
    pub warnings: Vec<String>,
    pub facts: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RustProtocolForwardingSubsetSmokeEvidenceReport {
    pub runtime_id: String,
    pub component: String,
    pub kernel_area: String,
    pub status: RustProtocolForwardingSubsetStatus,
    pub listener_port: u16,
    pub target_port: u16,
    pub target_received: bool,
    pub response_status: Option<String>,
    pub accounting: RustProtocolForwardingSubsetAccounting,
    pub stop_report: Option<RustProtocolForwardingSubsetStopReport>,
    pub passed: bool,
    pub mutates_runtime: bool,
    pub live_execution_allowed: bool,
    pub default_route: bool,
    pub forwards_traffic: bool,
    pub outbound_adapters_used: bool,
    pub mihomo_fallback: bool,
    pub blockers: Vec<String>,
    pub warnings: Vec<String>,
    pub facts: Vec<String>,
    pub next_safe_batch: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum RustTunSystemProxyMode {
    Off,
    SystemProxy,
    Tun,
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum RustTunSystemProxyParityStatus {
    Ready,
    Applied,
    Restored,
    Blocked,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RustTunSystemProxyRouteSnapshot {
    pub enable_system_proxy: bool,
    pub enable_tun_mode: bool,
    pub proxy_auto_config: bool,
    pub proxy_host: Option<String>,
    pub mixed_port: u16,
    pub system_proxy_bypass: Option<String>,
    pub use_default_bypass: bool,
    pub os_system_proxy_enabled: Option<bool>,
    pub os_system_proxy_server: Option<String>,
    pub clash_tun_enabled: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RustTunSystemProxyRoutePatch {
    pub enable_system_proxy: bool,
    pub enable_tun_mode: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RustTunSystemProxyParityPreflightReport {
    pub runtime_id: String,
    pub component: String,
    pub kernel_area: String,
    pub status: RustTunSystemProxyParityStatus,
    pub reason: String,
    pub requested_mode: RustTunSystemProxyMode,
    pub current_snapshot: RustTunSystemProxyRouteSnapshot,
    pub route_patch: RustTunSystemProxyRoutePatch,
    pub explicit_opt_in_required: bool,
    pub mutates_runtime: bool,
    pub reload_mihomo: bool,
    pub system_proxy_os_apply: bool,
    pub tun_runtime_apply: bool,
    pub mihomo_fallback: bool,
    pub rollback_supported: bool,
    pub blockers: Vec<String>,
    pub warnings: Vec<String>,
    pub facts: Vec<String>,
    pub next_safe_batch: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RustTunSystemProxyParityApplyReport {
    pub status: RustTunSystemProxyParityStatus,
    pub reason: String,
    pub requested_mode: RustTunSystemProxyMode,
    pub preflight: RustTunSystemProxyParityPreflightReport,
    pub previous_snapshot: RustTunSystemProxyRouteSnapshot,
    pub applied_snapshot: RustTunSystemProxyRouteSnapshot,
    pub rollback_record_path: Option<String>,
    pub explicit_opt_in: bool,
    pub mutates_runtime: bool,
    pub reload_mihomo: bool,
    pub system_proxy_os_apply: bool,
    pub tun_runtime_apply: bool,
    pub mihomo_fallback: bool,
    pub blockers: Vec<String>,
    pub warnings: Vec<String>,
    pub facts: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RustTunSystemProxyParityRollbackReport {
    pub status: RustTunSystemProxyParityStatus,
    pub reason: String,
    pub restored_snapshot: RustTunSystemProxyRouteSnapshot,
    pub rollback_record_path: Option<String>,
    pub mutates_runtime: bool,
    pub reload_mihomo: bool,
    pub system_proxy_os_apply: bool,
    pub tun_runtime_apply: bool,
    pub mihomo_fallback: bool,
    pub blockers: Vec<String>,
    pub warnings: Vec<String>,
    pub facts: Vec<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum RustFallbackRetirementReadinessStatus {
    Ready,
    Locked,
    Blocked,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RustFallbackRetirementScopeArea {
    pub area: String,
    pub rust_owned_capability: String,
    pub mihomo_fallback_scope: String,
    pub rollback_record_path: Option<String>,
    pub rollback_record_present: bool,
    pub canary_evidence_required: bool,
    pub fallback_retirement_allowed: bool,
    pub blockers: Vec<String>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RustFallbackRetirementReadinessManifest {
    pub runtime_id: String,
    pub component: String,
    pub kernel_area: String,
    pub status: RustFallbackRetirementReadinessStatus,
    pub generated_at_epoch_seconds: u64,
    pub supported_scope: Vec<RustFallbackRetirementScopeArea>,
    pub unsupported_fallback_scope: Vec<String>,
    pub emergency_rollback_paths: Vec<String>,
    pub manifest_path: Option<String>,
    pub fallback_retirement_execution_allowed: bool,
    pub mutates_runtime: bool,
    pub removes_mihomo_fallback: bool,
    pub blockers: Vec<String>,
    pub warnings: Vec<String>,
    pub facts: Vec<String>,
    pub next_safe_batch: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RustFallbackRetirementReadinessLockReport {
    pub status: RustFallbackRetirementReadinessStatus,
    pub reason: String,
    pub manifest: RustFallbackRetirementReadinessManifest,
    pub explicit_opt_in: bool,
    pub manifest_path: Option<String>,
    pub mutates_runtime: bool,
    pub removes_mihomo_fallback: bool,
    pub blockers: Vec<String>,
    pub warnings: Vec<String>,
    pub facts: Vec<String>,
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum RustRuntimeRealCanaryStatus {
    Passed,
    Failed,
    Blocked,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RustRuntimeRealCanaryEvidenceReport {
    pub runtime_id: String,
    pub component: String,
    pub kernel_area: String,
    pub status: RustRuntimeRealCanaryStatus,
    pub reason: String,
    pub canary_profile: String,
    pub started_at_epoch_seconds: u64,
    pub explicit_opt_in: bool,
    pub dns_smoke_evidence: Option<KernelLoopbackDnsSmokeEvidenceReport>,
    pub protocol_forwarding_evidence: Option<RustProtocolForwardingSubsetSmokeEvidenceReport>,
    pub tun_system_proxy_preflight: Option<RustTunSystemProxyParityPreflightReport>,
    pub fallback_readiness_manifest: Option<RustFallbackRetirementReadinessManifest>,
    pub evidence_path: Option<String>,
    pub mutates_runtime: bool,
    pub writes_evidence_artifact: bool,
    pub removes_mihomo_fallback: bool,
    pub mihomo_fallback: bool,
    pub blockers: Vec<String>,
    pub warnings: Vec<String>,
    pub facts: Vec<String>,
    pub next_safe_batch: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum MihomoFallbackRetirementExecutionStatus {
    Planned,
    Executed,
    Restored,
    Blocked,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct MihomoFallbackRetirementExecutionScope {
    pub scope: String,
    pub rust_owned_path: String,
    pub fallback_retired_for_scope: bool,
    pub mihomo_fallback_retained_for: Vec<String>,
    pub evidence: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct MihomoFallbackRetirementEmergencyCheckpoint {
    pub checkpoint_path: Option<String>,
    pub canary_evidence_path: Option<String>,
    pub previous_execution_manifest_path: Option<String>,
    pub retained_fallback_scope: Vec<String>,
    pub created_at_epoch_seconds: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MihomoFallbackRetirementExecutionReport {
    pub runtime_id: String,
    pub component: String,
    pub kernel_area: String,
    pub status: MihomoFallbackRetirementExecutionStatus,
    pub reason: String,
    pub explicit_opt_in: bool,
    pub supported_scope: Vec<MihomoFallbackRetirementExecutionScope>,
    pub emergency_checkpoint: MihomoFallbackRetirementEmergencyCheckpoint,
    pub execution_manifest_path: Option<String>,
    pub mutates_runtime: bool,
    pub writes_execution_manifest: bool,
    pub retires_supported_fallback: bool,
    pub removes_mihomo_fallback_binary: bool,
    pub unsupported_mihomo_fallback_retained: bool,
    pub blockers: Vec<String>,
    pub warnings: Vec<String>,
    pub facts: Vec<String>,
    pub next_safe_batch: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum RustDefaultDataPlaneCloseoutStatus {
    Ready,
    ClosedOut,
    Blocked,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RustDefaultDataPlaneCloseoutEvidenceOwnership {
    pub scope: String,
    pub rust_owned_path: String,
    pub evidence: Vec<String>,
    pub mihomo_fallback_retained_for: Vec<String>,
    pub default_eligible: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RustDefaultDataPlaneUnsupportedBlocker {
    pub blocker: String,
    pub mihomo_owner: String,
    pub retirement_requirement: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RustDefaultDataPlaneCloseoutReport {
    pub runtime_id: String,
    pub component: String,
    pub kernel_area: String,
    pub status: RustDefaultDataPlaneCloseoutStatus,
    pub reason: String,
    pub explicit_opt_in: bool,
    pub mutates_runtime: bool,
    pub writes_closeout_manifest: bool,
    pub closeout_manifest_path: Option<String>,
    pub fallback_retirement_manifest_path: Option<String>,
    pub evidence_ownership: Vec<RustDefaultDataPlaneCloseoutEvidenceOwnership>,
    pub unsupported_blockers: Vec<RustDefaultDataPlaneUnsupportedBlocker>,
    pub ownership_reconciled: bool,
    pub default_scope_locked_to_passed_evidence: bool,
    pub unsupported_mihomo_fallback_retained: bool,
    pub removes_mihomo_fallback_binary: bool,
    pub blockers: Vec<String>,
    pub warnings: Vec<String>,
    pub facts: Vec<String>,
    pub next_safe_batch: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum RustSocksUdpAssociateExecutionStatus {
    Planned,
    Executed,
    Blocked,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RustSocksUdpAssociatePacketEvidence {
    pub request_atyp: String,
    pub target_addr: String,
    pub target_port: u16,
    pub request_payload_bytes: usize,
    pub response_payload_bytes: usize,
    pub response_payload_prefix: String,
    pub datagram_round_trip: bool,
    pub frag_supported: bool,
    pub loopback_only: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RustSocksUdpAssociateRollbackEvidence {
    pub checkpoint_path: String,
    pub fallback_retained_for: Vec<String>,
    pub created_at_epoch_seconds: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RustSocksUdpAssociateLeakEvidence {
    pub passed: bool,
    pub no_system_packet_capture: bool,
    pub no_non_loopback_target: bool,
    pub no_mihomo_binary_removal: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RustSocksUdpAssociateExecutionReport {
    pub runtime_id: String,
    pub component: String,
    pub kernel_area: String,
    pub status: RustSocksUdpAssociateExecutionStatus,
    pub reason: String,
    pub explicit_opt_in: bool,
    pub rust_owned_scope: String,
    pub mutates_runtime: bool,
    pub writes_evidence: bool,
    pub evidence_path: Option<String>,
    pub packet_evidence: Option<RustSocksUdpAssociatePacketEvidence>,
    pub rollback_evidence: Option<RustSocksUdpAssociateRollbackEvidence>,
    pub leak_evidence: Option<RustSocksUdpAssociateLeakEvidence>,
    pub mihomo_fallback_retained_for: Vec<String>,
    pub blockers: Vec<String>,
    pub warnings: Vec<String>,
    pub facts: Vec<String>,
    pub next_safe_batch: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum RustSocksUdpFragmentsExecutionStatus {
    Planned,
    Executed,
    Blocked,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RustSocksUdpFragmentsPacketEvidence {
    pub target_addr: String,
    pub target_port: u16,
    pub fragment_count: usize,
    pub first_fragment: String,
    pub final_fragment: String,
    pub request_payload_bytes: usize,
    pub reassembled_payload_bytes: usize,
    pub target_received_bytes: usize,
    pub response_payload_bytes: usize,
    pub response_payload_prefix: String,
    pub fragments_reassembled: bool,
    pub datagram_round_trip: bool,
    pub loopback_only: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RustSocksUdpFragmentsRollbackEvidence {
    pub checkpoint_path: String,
    pub fallback_retained_for: Vec<String>,
    pub created_at_epoch_seconds: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RustSocksUdpFragmentsLeakEvidence {
    pub passed: bool,
    pub no_system_packet_capture: bool,
    pub no_non_loopback_target: bool,
    pub no_mihomo_binary_removal: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RustSocksUdpFragmentsExecutionReport {
    pub runtime_id: String,
    pub component: String,
    pub kernel_area: String,
    pub status: RustSocksUdpFragmentsExecutionStatus,
    pub reason: String,
    pub explicit_opt_in: bool,
    pub rust_owned_scope: String,
    pub mutates_runtime: bool,
    pub writes_evidence: bool,
    pub evidence_path: Option<String>,
    pub packet_evidence: Option<RustSocksUdpFragmentsPacketEvidence>,
    pub rollback_evidence: Option<RustSocksUdpFragmentsRollbackEvidence>,
    pub leak_evidence: Option<RustSocksUdpFragmentsLeakEvidence>,
    pub mihomo_fallback_retained_for: Vec<String>,
    pub blockers: Vec<String>,
    pub warnings: Vec<String>,
    pub facts: Vec<String>,
    pub next_safe_batch: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum RustSocksAuthExecutionStatus {
    Planned,
    Executed,
    Blocked,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RustSocksAuthExecutionEvidence {
    pub listener_addr: String,
    pub selected_method: String,
    pub username_bytes: usize,
    pub password_bytes: usize,
    pub auth_version: u8,
    pub method_negotiated: bool,
    pub auth_accepted: bool,
    pub connect_command: String,
    pub connect_atyp: String,
    pub connect_request_validated: bool,
    pub loopback_only: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RustSocksAuthRollbackEvidence {
    pub checkpoint_path: String,
    pub fallback_retained_for: Vec<String>,
    pub created_at_epoch_seconds: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RustSocksAuthLeakEvidence {
    pub passed: bool,
    pub no_system_packet_capture: bool,
    pub no_non_loopback_target: bool,
    pub no_mihomo_binary_removal: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RustSocksAuthExecutionReport {
    pub runtime_id: String,
    pub component: String,
    pub kernel_area: String,
    pub status: RustSocksAuthExecutionStatus,
    pub reason: String,
    pub explicit_opt_in: bool,
    pub rust_owned_scope: String,
    pub mutates_runtime: bool,
    pub writes_evidence: bool,
    pub evidence_path: Option<String>,
    pub auth_evidence: Option<RustSocksAuthExecutionEvidence>,
    pub rollback_evidence: Option<RustSocksAuthRollbackEvidence>,
    pub leak_evidence: Option<RustSocksAuthLeakEvidence>,
    pub mihomo_fallback_retained_for: Vec<String>,
    pub blockers: Vec<String>,
    pub warnings: Vec<String>,
    pub facts: Vec<String>,
    pub next_safe_batch: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum RustSocksTcpConnectExecutionStatus {
    Planned,
    Executed,
    Blocked,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RustSocksTcpConnectForwardEvidence {
    pub proxy_listener_addr: String,
    pub target_addr: String,
    pub selected_method: String,
    pub auth_negotiated: bool,
    pub connect_command: String,
    pub connect_atyp: String,
    pub request_bytes: usize,
    pub target_received_bytes: usize,
    pub response_bytes: usize,
    pub response_prefix: String,
    pub data_forwarded: bool,
    pub loopback_only: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RustSocksTcpConnectRollbackEvidence {
    pub checkpoint_path: String,
    pub fallback_retained_for: Vec<String>,
    pub created_at_epoch_seconds: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RustSocksTcpConnectLeakEvidence {
    pub passed: bool,
    pub no_system_packet_capture: bool,
    pub no_non_loopback_target: bool,
    pub no_mihomo_binary_removal: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RustSocksTcpConnectExecutionReport {
    pub runtime_id: String,
    pub component: String,
    pub kernel_area: String,
    pub status: RustSocksTcpConnectExecutionStatus,
    pub reason: String,
    pub explicit_opt_in: bool,
    pub rust_owned_scope: String,
    pub mutates_runtime: bool,
    pub writes_evidence: bool,
    pub evidence_path: Option<String>,
    pub forward_evidence: Option<RustSocksTcpConnectForwardEvidence>,
    pub rollback_evidence: Option<RustSocksTcpConnectRollbackEvidence>,
    pub leak_evidence: Option<RustSocksTcpConnectLeakEvidence>,
    pub mihomo_fallback_retained_for: Vec<String>,
    pub blockers: Vec<String>,
    pub warnings: Vec<String>,
    pub facts: Vec<String>,
    pub next_safe_batch: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum RustSocksBindExecutionStatus {
    Planned,
    Executed,
    Blocked,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RustSocksBindForwardEvidence {
    pub proxy_listener_addr: String,
    pub bind_addr: String,
    pub peer_addr: String,
    pub selected_method: String,
    pub auth_negotiated: bool,
    pub bind_command: String,
    pub bind_atyp: String,
    pub first_reply_sent: bool,
    pub second_reply_sent: bool,
    pub request_bytes: usize,
    pub peer_received_bytes: usize,
    pub response_bytes: usize,
    pub response_prefix: String,
    pub data_forwarded: bool,
    pub loopback_only: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RustSocksBindRollbackEvidence {
    pub checkpoint_path: String,
    pub fallback_retained_for: Vec<String>,
    pub created_at_epoch_seconds: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RustSocksBindLeakEvidence {
    pub passed: bool,
    pub no_system_packet_capture: bool,
    pub no_non_loopback_peer: bool,
    pub no_mihomo_binary_removal: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RustSocksBindExecutionReport {
    pub runtime_id: String,
    pub component: String,
    pub kernel_area: String,
    pub status: RustSocksBindExecutionStatus,
    pub reason: String,
    pub explicit_opt_in: bool,
    pub rust_owned_scope: String,
    pub mutates_runtime: bool,
    pub writes_evidence: bool,
    pub evidence_path: Option<String>,
    pub forward_evidence: Option<RustSocksBindForwardEvidence>,
    pub rollback_evidence: Option<RustSocksBindRollbackEvidence>,
    pub leak_evidence: Option<RustSocksBindLeakEvidence>,
    pub mihomo_fallback_retained_for: Vec<String>,
    pub blockers: Vec<String>,
    pub warnings: Vec<String>,
    pub facts: Vec<String>,
    pub next_safe_batch: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum RustProtocolAdapterForwardingStatus {
    Passed,
    Failed,
    Blocked,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum RustProtocolAdapterForwardingAdapterKind {
    Direct,
    Reject,
    MihomoFallback,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RustProtocolAdapterForwardingDecisionEvidence {
    pub adapter_kind: RustProtocolAdapterForwardingAdapterKind,
    pub listener_port: u16,
    pub target_port: Option<u16>,
    pub target_received: bool,
    pub response_status: Option<String>,
    pub accepted_connections: u64,
    pub bytes_from_client: u64,
    pub bytes_from_target: u64,
    pub passed: bool,
    pub blockers: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RustProtocolAdapterForwardingExpansionReport {
    pub runtime_id: String,
    pub component: String,
    pub kernel_area: String,
    pub status: RustProtocolAdapterForwardingStatus,
    pub reason: String,
    pub explicit_opt_in: bool,
    pub direct_evidence: Option<RustProtocolAdapterForwardingDecisionEvidence>,
    pub reject_evidence: Option<RustProtocolAdapterForwardingDecisionEvidence>,
    pub evidence_path: Option<String>,
    pub loopback_only: bool,
    pub mutates_runtime: bool,
    pub forwards_traffic: bool,
    pub outbound_adapters_used: bool,
    pub writes_evidence_artifact: bool,
    pub mihomo_fallback: bool,
    pub blockers: Vec<String>,
    pub warnings: Vec<String>,
    pub facts: Vec<String>,
    pub next_safe_batch: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum RustRemoteAdapterTransportStatus {
    Passed,
    Failed,
    Blocked,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum RustRemoteAdapterTransportKind {
    TcpConnect,
    UnsupportedProxyProtocol,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RustRemoteAdapterTransportEvidence {
    pub transport_kind: RustRemoteAdapterTransportKind,
    pub adapter_name: String,
    pub control_port: Option<u16>,
    pub target_port: Option<u16>,
    pub target_received: bool,
    pub response_status: Option<String>,
    pub bytes_to_remote: u64,
    pub bytes_from_remote: u64,
    pub fallback_retained: bool,
    pub passed: bool,
    pub blockers: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RustRemoteAdapterTransportExpansionReport {
    pub runtime_id: String,
    pub component: String,
    pub kernel_area: String,
    pub status: RustRemoteAdapterTransportStatus,
    pub reason: String,
    pub explicit_opt_in: bool,
    pub tcp_connect_evidence: Option<RustRemoteAdapterTransportEvidence>,
    pub unsupported_protocol_evidence: Option<RustRemoteAdapterTransportEvidence>,
    pub evidence_path: Option<String>,
    pub loopback_remote_only: bool,
    pub mutates_runtime: bool,
    pub forwards_traffic: bool,
    pub outbound_adapters_used: bool,
    pub writes_evidence_artifact: bool,
    pub mihomo_fallback: bool,
    pub blockers: Vec<String>,
    pub warnings: Vec<String>,
    pub facts: Vec<String>,
    pub next_safe_batch: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum RustHttpConnectProxyAdapterStatus {
    Passed,
    Failed,
    Blocked,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RustHttpConnectProxyAdapterEvidence {
    pub adapter_name: String,
    pub listener_port: u16,
    pub target_port: u16,
    pub connect_authority: String,
    pub connect_established: bool,
    pub target_received: bool,
    pub response_status: Option<String>,
    pub bytes_from_client: u64,
    pub bytes_from_target: u64,
    pub passed: bool,
    pub blockers: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RustHttpConnectProxyAdapterReport {
    pub runtime_id: String,
    pub component: String,
    pub kernel_area: String,
    pub status: RustHttpConnectProxyAdapterStatus,
    pub reason: String,
    pub explicit_opt_in: bool,
    pub connect_evidence: Option<RustHttpConnectProxyAdapterEvidence>,
    pub unsupported_protocols: Vec<String>,
    pub evidence_path: Option<String>,
    pub loopback_remote_only: bool,
    pub mutates_runtime: bool,
    pub forwards_traffic: bool,
    pub outbound_adapters_used: bool,
    pub writes_evidence_artifact: bool,
    pub mihomo_fallback: bool,
    pub blockers: Vec<String>,
    pub warnings: Vec<String>,
    pub facts: Vec<String>,
    pub next_safe_batch: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum RustEncryptedProxyProtocolStatus {
    Passed,
    Failed,
    Blocked,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum RustEncryptedProxyProtocolKind {
    ShadowsocksAead,
    TrojanAuth,
    UnsupportedEncryptedProtocol,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RustEncryptedProxyProtocolEvidence {
    pub protocol: RustEncryptedProxyProtocolKind,
    pub adapter_name: String,
    pub listener_port: Option<u16>,
    pub target_port: Option<u16>,
    pub target_received: bool,
    pub response_status: Option<String>,
    pub encrypted_request_bytes: u64,
    pub decrypted_request_bytes: u64,
    pub encrypted_response_bytes: u64,
    pub decrypted_response_bytes: u64,
    pub fallback_retained: bool,
    pub passed: bool,
    pub blockers: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RustEncryptedProxyProtocolPreflightReport {
    pub runtime_id: String,
    pub component: String,
    pub kernel_area: String,
    pub status: RustEncryptedProxyProtocolStatus,
    pub reason: String,
    pub explicit_opt_in: bool,
    pub shadowsocks_aead_evidence: Option<RustEncryptedProxyProtocolEvidence>,
    pub trojan_auth_evidence: Option<RustEncryptedProxyProtocolEvidence>,
    pub unsupported_protocol_evidence: Vec<RustEncryptedProxyProtocolEvidence>,
    pub evidence_path: Option<String>,
    pub loopback_remote_only: bool,
    pub mutates_runtime: bool,
    pub forwards_traffic: bool,
    pub outbound_adapters_used: bool,
    pub writes_evidence_artifact: bool,
    pub mihomo_fallback: bool,
    pub blockers: Vec<String>,
    pub warnings: Vec<String>,
    pub facts: Vec<String>,
    pub next_safe_batch: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum RustShadowsocksAeadAdapterExecutionStatus {
    Passed,
    Failed,
    Blocked,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RustShadowsocksAeadAdapterExecutionEvidence {
    pub adapter_name: String,
    pub cipher: String,
    pub listener_port: u16,
    pub target_port: u16,
    pub target_address: String,
    pub accepted_connections: u64,
    pub target_received: bool,
    pub response_status: Option<String>,
    pub encrypted_request_bytes: u64,
    pub decrypted_request_bytes: u64,
    pub encrypted_response_bytes: u64,
    pub decrypted_response_bytes: u64,
    pub address_frame_validated: bool,
    pub rollback_checkpoint_path: Option<String>,
    pub fallback_retained_for_unsupported: bool,
    pub passed: bool,
    pub blockers: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RustShadowsocksAeadAdapterExecutionReport {
    pub runtime_id: String,
    pub component: String,
    pub kernel_area: String,
    pub status: RustShadowsocksAeadAdapterExecutionStatus,
    pub reason: String,
    pub explicit_opt_in: bool,
    pub execution_evidence: Option<RustShadowsocksAeadAdapterExecutionEvidence>,
    pub unsupported_protocols: Vec<String>,
    pub evidence_path: Option<String>,
    pub rollback_checkpoint_path: Option<String>,
    pub loopback_remote_only: bool,
    pub mutates_runtime: bool,
    pub forwards_traffic: bool,
    pub outbound_adapters_used: bool,
    pub writes_evidence_artifact: bool,
    pub mihomo_fallback: bool,
    pub blockers: Vec<String>,
    pub warnings: Vec<String>,
    pub facts: Vec<String>,
    pub next_safe_batch: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum RustShadowsocksAeadAdapterCanaryStatus {
    Passed,
    Failed,
    Blocked,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RustShadowsocksAeadAdapterCanaryFallbackEvidence {
    pub trigger_name: String,
    pub unsupported_protocol: String,
    pub fallback_triggered: bool,
    pub rust_adapter_bypassed: bool,
    pub mihomo_fallback_retained: bool,
    pub passed: bool,
    pub blockers: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RustShadowsocksAeadAdapterCanaryRollbackEvidence {
    pub checkpoint_path: Option<String>,
    pub checkpoint_readable: bool,
    pub component: Option<String>,
    pub adapter_name: Option<String>,
    pub fallback_retained_for_unsupported: bool,
    pub rollback_action: Option<String>,
    pub passed: bool,
    pub blockers: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RustShadowsocksAeadAdapterCanaryHealthEvidence {
    pub execution_evidence_path: Option<String>,
    pub execution_passed: bool,
    pub loopback_remote_only: bool,
    pub target_received: bool,
    pub response_status: Option<String>,
    pub byte_accounting_passed: bool,
    pub no_runtime_mutation: bool,
    pub passed: bool,
    pub blockers: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RustShadowsocksAeadAdapterCanaryReport {
    pub runtime_id: String,
    pub component: String,
    pub kernel_area: String,
    pub status: RustShadowsocksAeadAdapterCanaryStatus,
    pub reason: String,
    pub explicit_opt_in: bool,
    pub execution_report: Option<RustShadowsocksAeadAdapterExecutionReport>,
    pub fallback_trigger_evidence: Option<RustShadowsocksAeadAdapterCanaryFallbackEvidence>,
    pub rollback_checkpoint_evidence: Option<RustShadowsocksAeadAdapterCanaryRollbackEvidence>,
    pub health_evidence: Option<RustShadowsocksAeadAdapterCanaryHealthEvidence>,
    pub evidence_path: Option<String>,
    pub loopback_remote_only: bool,
    pub mutates_runtime: bool,
    pub forwards_traffic: bool,
    pub outbound_adapters_used: bool,
    pub writes_evidence_artifact: bool,
    pub mihomo_fallback: bool,
    pub blockers: Vec<String>,
    pub warnings: Vec<String>,
    pub facts: Vec<String>,
    pub next_safe_batch: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum RustEncryptedProxySessionExpansionStatus {
    Passed,
    Failed,
    Blocked,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RustEncryptedProxySessionChunkEvidence {
    pub chunk_index: u16,
    pub request_marker: String,
    pub response_marker: Option<String>,
    pub encrypted_request_bytes: u64,
    pub decrypted_request_bytes: u64,
    pub target_response_bytes: u64,
    pub encrypted_response_bytes: u64,
    pub passed: bool,
    pub blockers: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RustEncryptedProxySessionEvidence {
    pub protocol: RustEncryptedProxyProtocolKind,
    pub adapter_name: String,
    pub listener_port: u16,
    pub target_port: u16,
    pub target_address: String,
    pub address_frame_validated: bool,
    pub session_established: bool,
    pub chunks_forwarded: u64,
    pub encrypted_request_bytes: u64,
    pub decrypted_request_bytes: u64,
    pub encrypted_response_bytes: u64,
    pub decrypted_response_bytes: u64,
    pub target_sessions: u64,
    pub target_chunks_received: u64,
    pub chunk_evidence: Vec<RustEncryptedProxySessionChunkEvidence>,
    pub passed: bool,
    pub blockers: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RustEncryptedProxySessionFallbackEvidence {
    pub unsupported_protocols: Vec<String>,
    pub fallback_retained: bool,
    pub unsupported_sessions_bypassed: bool,
    pub passed: bool,
    pub blockers: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RustEncryptedProxySessionExpansionReport {
    pub runtime_id: String,
    pub component: String,
    pub kernel_area: String,
    pub status: RustEncryptedProxySessionExpansionStatus,
    pub reason: String,
    pub explicit_opt_in: bool,
    pub session_evidence: Option<RustEncryptedProxySessionEvidence>,
    pub fallback_evidence: Option<RustEncryptedProxySessionFallbackEvidence>,
    pub evidence_path: Option<String>,
    pub loopback_remote_only: bool,
    pub mutates_runtime: bool,
    pub forwards_traffic: bool,
    pub outbound_adapters_used: bool,
    pub writes_evidence_artifact: bool,
    pub mihomo_fallback: bool,
    pub blockers: Vec<String>,
    pub warnings: Vec<String>,
    pub facts: Vec<String>,
    pub next_safe_batch: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum RustTunTransparentRoutingExecutionStatus {
    Passed,
    Failed,
    Blocked,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RustTunTransparentRoutingPacketEvidence {
    pub packet_source: String,
    pub packet_destination: String,
    pub packet_destination_port: u16,
    pub ipv4_packet_parsed: bool,
    pub tcp_destination_extracted: bool,
    pub payload_bytes: u64,
    pub target_received: bool,
    pub response_status: Option<String>,
    pub response_bytes: u64,
    pub passed: bool,
    pub blockers: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RustTunTransparentRoutingRollbackEvidence {
    pub checkpoint_path: Option<String>,
    pub checkpoint_written: bool,
    pub route_owner_before: String,
    pub route_owner_after: String,
    pub rollback_action: String,
    pub packet_capture_default_unchanged: bool,
    pub passed: bool,
    pub blockers: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RustTunTransparentRoutingLeakEvidence {
    pub loopback_only: bool,
    pub os_route_mutation_attempted: bool,
    pub system_proxy_mutation_attempted: bool,
    pub tun_device_mutation_attempted: bool,
    pub unsupported_packet_capture_fallback: bool,
    pub passed: bool,
    pub blockers: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RustTunTransparentRoutingExecutionReport {
    pub runtime_id: String,
    pub component: String,
    pub kernel_area: String,
    pub status: RustTunTransparentRoutingExecutionStatus,
    pub reason: String,
    pub explicit_opt_in: bool,
    pub packet_evidence: Option<RustTunTransparentRoutingPacketEvidence>,
    pub rollback_evidence: Option<RustTunTransparentRoutingRollbackEvidence>,
    pub leak_evidence: Option<RustTunTransparentRoutingLeakEvidence>,
    pub evidence_path: Option<String>,
    pub rollback_checkpoint_path: Option<String>,
    pub loopback_remote_only: bool,
    pub mutates_runtime: bool,
    pub forwards_traffic: bool,
    pub packet_capture_owned: bool,
    pub writes_evidence_artifact: bool,
    pub mihomo_fallback: bool,
    pub blockers: Vec<String>,
    pub warnings: Vec<String>,
    pub facts: Vec<String>,
    pub next_safe_batch: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KernelLoopbackPlatformMatrixRow {
    pub platform: String,
    pub current_platform: bool,
    pub evidence_status: String,
    pub listener_port_released: Option<bool>,
    pub target_port_released: Option<bool>,
    pub isolated_test_listener_stopped: Option<bool>,
    pub default_route: bool,
    pub forwards_traffic: bool,
    pub outbound_adapters_used: bool,
    pub mihomo_fallback: bool,
    pub blockers: Vec<String>,
    pub facts: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KernelLoopbackPlatformMatrixReport {
    pub runtime_id: String,
    pub component: String,
    pub kernel_area: String,
    pub mutates_runtime: bool,
    pub live_execution_allowed: bool,
    pub current_platform: String,
    pub current_arch: String,
    pub listener_port: u16,
    pub target_port: u16,
    pub required_platforms: Vec<String>,
    pub covered_platforms: Vec<String>,
    pub pending_platforms: Vec<String>,
    pub current_platform_passed: bool,
    pub expanded_opt_in_allowed: bool,
    pub leak_check: KernelLoopbackForwardingLeakCheckReport,
    pub rows: Vec<KernelLoopbackPlatformMatrixRow>,
    pub default_route: bool,
    pub forwards_traffic: bool,
    pub outbound_adapters_used: bool,
    pub mihomo_fallback: bool,
    pub passed: bool,
    pub blockers: Vec<String>,
    pub warnings: Vec<String>,
    pub facts: Vec<String>,
    pub next_safe_batch: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KernelLoopbackHoldWindowRow {
    pub platform: String,
    pub current_platform: bool,
    pub evidence_status: String,
    pub hold_started_at_epoch_ms: Option<u64>,
    pub observed_at_epoch_ms: Option<u64>,
    pub minimum_hold_seconds: u64,
    pub elapsed_hold_seconds: Option<u64>,
    pub hold_window_satisfied: bool,
    pub platform_matrix_passed: Option<bool>,
    pub leak_check_passed: Option<bool>,
    pub default_route: bool,
    pub forwards_traffic: bool,
    pub outbound_adapters_used: bool,
    pub mihomo_fallback: bool,
    pub blockers: Vec<String>,
    pub facts: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KernelLoopbackHoldWindowReport {
    pub runtime_id: String,
    pub component: String,
    pub kernel_area: String,
    pub mutates_runtime: bool,
    pub live_execution_allowed: bool,
    pub current_platform: String,
    pub current_arch: String,
    pub listener_port: u16,
    pub target_port: u16,
    pub hold_started_at_epoch_ms: u64,
    pub observed_at_epoch_ms: u64,
    pub minimum_hold_seconds: u64,
    pub elapsed_hold_seconds: u64,
    pub required_platforms: Vec<String>,
    pub covered_hold_platforms: Vec<String>,
    pub pending_hold_platforms: Vec<String>,
    pub current_platform_passed: bool,
    pub current_platform_hold_window_satisfied: bool,
    pub expanded_opt_in_allowed: bool,
    pub platform_matrix: KernelLoopbackPlatformMatrixReport,
    pub rows: Vec<KernelLoopbackHoldWindowRow>,
    pub default_route: bool,
    pub forwards_traffic: bool,
    pub outbound_adapters_used: bool,
    pub mihomo_fallback: bool,
    pub passed: bool,
    pub blockers: Vec<String>,
    pub warnings: Vec<String>,
    pub facts: Vec<String>,
    pub next_safe_batch: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KernelLoopbackPlatformRollbackDrillRow {
    pub platform: String,
    pub current_platform: bool,
    pub evidence_status: String,
    pub smoke_passed: Option<bool>,
    pub ports_released: Option<bool>,
    pub system_proxy_unchanged: Option<bool>,
    pub tun_unchanged: Option<bool>,
    pub runtime_config_unchanged: Option<bool>,
    pub hold_window_satisfied: Option<bool>,
    pub default_route: bool,
    pub forwards_traffic: bool,
    pub outbound_adapters_used: bool,
    pub mihomo_fallback: bool,
    pub blockers: Vec<String>,
    pub facts: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KernelLoopbackPlatformRollbackDrillsReport {
    pub runtime_id: String,
    pub component: String,
    pub kernel_area: String,
    pub mutates_runtime: bool,
    pub live_execution_allowed: bool,
    pub current_platform: String,
    pub current_arch: String,
    pub listener_port: u16,
    pub target_port: u16,
    pub required_platforms: Vec<String>,
    pub covered_rollback_platforms: Vec<String>,
    pub pending_rollback_platforms: Vec<String>,
    pub current_platform_passed: bool,
    pub expanded_opt_in_allowed: bool,
    pub hold_window: KernelLoopbackHoldWindowReport,
    pub rollback_drill: KernelLoopbackForwardingRollbackDrillReport,
    pub rows: Vec<KernelLoopbackPlatformRollbackDrillRow>,
    pub default_route: bool,
    pub forwards_traffic: bool,
    pub outbound_adapters_used: bool,
    pub mihomo_fallback: bool,
    pub passed: bool,
    pub blockers: Vec<String>,
    pub warnings: Vec<String>,
    pub facts: Vec<String>,
    pub next_safe_batch: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KernelLoopbackR4ExpandedOptInPreflightCheck {
    pub name: String,
    pub status: String,
    pub passed: bool,
    pub blockers: Vec<String>,
    pub facts: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KernelLoopbackR4ExpandedOptInPreflightRow {
    pub platform: String,
    pub current_platform: bool,
    pub rollback_drill_observed: bool,
    pub hold_window_satisfied: Option<bool>,
    pub evidence_status: String,
    pub blockers: Vec<String>,
    pub facts: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KernelLoopbackR4ExpandedOptInPreflightReport {
    pub runtime_id: String,
    pub component: String,
    pub kernel_area: String,
    pub mutates_runtime: bool,
    pub live_execution_allowed: bool,
    pub current_platform: String,
    pub current_arch: String,
    pub listener_port: u16,
    pub target_port: u16,
    pub explicit_decision: bool,
    pub required_platforms: Vec<String>,
    pub observed_rollback_platforms: Vec<String>,
    pub pending_rollback_platforms: Vec<String>,
    pub current_platform_hold_window_satisfied: bool,
    pub preflight_passed: bool,
    pub expanded_opt_in_allowed: bool,
    pub hold_window: KernelLoopbackHoldWindowReport,
    pub rows: Vec<KernelLoopbackR4ExpandedOptInPreflightRow>,
    pub checks: Vec<KernelLoopbackR4ExpandedOptInPreflightCheck>,
    pub default_route: bool,
    pub forwards_traffic: bool,
    pub outbound_adapters_used: bool,
    pub mihomo_fallback: bool,
    pub passed: bool,
    pub blockers: Vec<String>,
    pub warnings: Vec<String>,
    pub facts: Vec<String>,
    pub next_safe_batch: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KernelLoopbackR4ExpandedOptInExecutionPlanStep {
    pub order: u8,
    pub name: String,
    pub action: String,
    pub mutates_runtime: bool,
    pub requires_explicit_decision: bool,
    pub enabled_in_this_batch: bool,
    pub blockers: Vec<String>,
    pub facts: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KernelLoopbackR4ExpandedOptInExecutionPlanReport {
    pub runtime_id: String,
    pub component: String,
    pub kernel_area: String,
    pub mutates_runtime: bool,
    pub live_execution_allowed: bool,
    pub current_platform: String,
    pub current_arch: String,
    pub listener_port: u16,
    pub target_port: u16,
    pub candidate_scope: String,
    pub explicit_decision: bool,
    pub plan_ready: bool,
    pub execution_allowed: bool,
    pub expanded_opt_in_allowed: bool,
    pub preflight: KernelLoopbackR4ExpandedOptInPreflightReport,
    pub steps: Vec<KernelLoopbackR4ExpandedOptInExecutionPlanStep>,
    pub default_route: bool,
    pub forwards_traffic: bool,
    pub outbound_adapters_used: bool,
    pub mihomo_fallback: bool,
    pub passed: bool,
    pub blockers: Vec<String>,
    pub warnings: Vec<String>,
    pub facts: Vec<String>,
    pub next_safe_batch: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KernelLoopbackR4ExpandedOptInExecutionGuardCheck {
    pub name: String,
    pub status: String,
    pub passed: bool,
    pub required_for_execution: bool,
    pub blockers: Vec<String>,
    pub facts: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KernelLoopbackR4ExpandedOptInSafetyPlanStep {
    pub order: u8,
    pub phase: String,
    pub action: String,
    pub mutates_runtime: bool,
    pub required_before_expansion: bool,
    pub enabled_in_this_batch: bool,
    pub blockers: Vec<String>,
    pub facts: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KernelLoopbackR4ExpandedOptInExecutionGuardReport {
    pub runtime_id: String,
    pub component: String,
    pub kernel_area: String,
    pub mutates_runtime: bool,
    pub live_execution_allowed: bool,
    pub current_platform: String,
    pub current_arch: String,
    pub listener_port: u16,
    pub target_port: u16,
    pub requested_execution: bool,
    pub explicit_decision: bool,
    pub guard_ready: bool,
    pub synthetic_execution_allowed: bool,
    pub execution_allowed: bool,
    pub expanded_opt_in_allowed: bool,
    pub plan: KernelLoopbackR4ExpandedOptInExecutionPlanReport,
    pub guard_checks: Vec<KernelLoopbackR4ExpandedOptInExecutionGuardCheck>,
    pub verification_plan: Vec<KernelLoopbackR4ExpandedOptInSafetyPlanStep>,
    pub rollback_plan: Vec<KernelLoopbackR4ExpandedOptInSafetyPlanStep>,
    pub default_route: bool,
    pub forwards_traffic: bool,
    pub outbound_adapters_used: bool,
    pub mihomo_fallback: bool,
    pub passed: bool,
    pub blockers: Vec<String>,
    pub warnings: Vec<String>,
    pub facts: Vec<String>,
    pub next_safe_batch: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KernelLoopbackR4ExpandedOptInSyntheticExecutionCloseout {
    pub rollback_drill_passed: bool,
    pub leak_check_passed: bool,
    pub ports_released: bool,
    pub system_proxy_unchanged: bool,
    pub tun_unchanged: bool,
    pub runtime_config_unchanged: bool,
    pub isolated_test_listener_stopped: bool,
    pub default_route: bool,
    pub forwards_traffic: bool,
    pub outbound_adapters_used: bool,
    pub mihomo_fallback: bool,
    pub passed: bool,
    pub blockers: Vec<String>,
    pub warnings: Vec<String>,
    pub facts: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KernelLoopbackR4ExpandedOptInSyntheticExecutionReport {
    pub runtime_id: String,
    pub component: String,
    pub kernel_area: String,
    pub mutates_runtime: bool,
    pub live_execution_allowed: bool,
    pub current_platform: String,
    pub current_arch: String,
    pub listener_port: u16,
    pub target_port: u16,
    pub requested_execution: bool,
    pub explicit_decision: bool,
    pub synthetic_execution_allowed: bool,
    pub execution_attempted: bool,
    pub expanded_opt_in_allowed: bool,
    pub guard: KernelLoopbackR4ExpandedOptInExecutionGuardReport,
    pub rollback_drill: Option<KernelLoopbackForwardingRollbackDrillReport>,
    pub leak_check: Option<KernelLoopbackForwardingLeakCheckReport>,
    pub closeout: KernelLoopbackR4ExpandedOptInSyntheticExecutionCloseout,
    pub default_route: bool,
    pub forwards_traffic: bool,
    pub outbound_adapters_used: bool,
    pub mihomo_fallback: bool,
    pub passed: bool,
    pub blockers: Vec<String>,
    pub warnings: Vec<String>,
    pub facts: Vec<String>,
    pub next_safe_batch: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KernelLoopbackR4ExpandedOptInPostExecutionHoldReport {
    pub runtime_id: String,
    pub component: String,
    pub kernel_area: String,
    pub mutates_runtime: bool,
    pub live_execution_allowed: bool,
    pub current_platform: String,
    pub current_arch: String,
    pub listener_port: u16,
    pub target_port: u16,
    pub requested_execution: bool,
    pub explicit_decision: bool,
    pub post_execution_hold_started_at_epoch_ms: u64,
    pub observed_at_epoch_ms: u64,
    pub minimum_hold_seconds: u64,
    pub elapsed_hold_seconds: u64,
    pub post_execution_hold_satisfied: bool,
    pub execution_attempted: bool,
    pub synthetic_execution_passed: bool,
    pub closeout_passed: bool,
    pub expanded_opt_in_allowed: bool,
    pub synthetic_execution: KernelLoopbackR4ExpandedOptInSyntheticExecutionReport,
    pub default_route: bool,
    pub forwards_traffic: bool,
    pub outbound_adapters_used: bool,
    pub mihomo_fallback: bool,
    pub passed: bool,
    pub blockers: Vec<String>,
    pub warnings: Vec<String>,
    pub facts: Vec<String>,
    pub next_safe_batch: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KernelLoopbackR4ExpandedOptInDecisionReadinessCheck {
    pub name: String,
    pub status: String,
    pub passed: bool,
    pub blockers: Vec<String>,
    pub facts: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KernelLoopbackR4ExpandedOptInDecisionReadinessReport {
    pub runtime_id: String,
    pub component: String,
    pub kernel_area: String,
    pub mutates_runtime: bool,
    pub live_execution_allowed: bool,
    pub current_platform: String,
    pub current_arch: String,
    pub listener_port: u16,
    pub target_port: u16,
    pub requested_execution: bool,
    pub explicit_decision: bool,
    pub wider_opt_in_decision: bool,
    pub decision_ready: bool,
    pub wider_opt_in_allowed: bool,
    pub expanded_opt_in_allowed: bool,
    pub post_execution_hold: KernelLoopbackR4ExpandedOptInPostExecutionHoldReport,
    pub checks: Vec<KernelLoopbackR4ExpandedOptInDecisionReadinessCheck>,
    pub default_route: bool,
    pub forwards_traffic: bool,
    pub outbound_adapters_used: bool,
    pub mihomo_fallback: bool,
    pub passed: bool,
    pub blockers: Vec<String>,
    pub warnings: Vec<String>,
    pub facts: Vec<String>,
    pub next_safe_batch: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KernelLoopbackR4ExpandedOptInLimitedRolloutGateCheck {
    pub name: String,
    pub status: String,
    pub passed: bool,
    pub blockers: Vec<String>,
    pub facts: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KernelLoopbackR4ExpandedOptInLimitedRolloutGateReport {
    pub runtime_id: String,
    pub component: String,
    pub kernel_area: String,
    pub mutates_runtime: bool,
    pub live_execution_allowed: bool,
    pub current_platform: String,
    pub current_arch: String,
    pub listener_port: u16,
    pub target_port: u16,
    pub requested_execution: bool,
    pub explicit_decision: bool,
    pub wider_opt_in_decision: bool,
    pub limited_rollout_decision: bool,
    pub canary_scope: String,
    pub max_canary_sessions: u16,
    pub gate_ready: bool,
    pub limited_rollout_allowed: bool,
    pub expanded_opt_in_allowed: bool,
    pub decision_readiness: KernelLoopbackR4ExpandedOptInDecisionReadinessReport,
    pub checks: Vec<KernelLoopbackR4ExpandedOptInLimitedRolloutGateCheck>,
    pub default_route: bool,
    pub forwards_traffic: bool,
    pub outbound_adapters_used: bool,
    pub mihomo_fallback: bool,
    pub passed: bool,
    pub blockers: Vec<String>,
    pub warnings: Vec<String>,
    pub facts: Vec<String>,
    pub next_safe_batch: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KernelLoopbackR4ExpandedOptInRolloutAuditRow {
    pub name: String,
    pub status: String,
    pub passed: bool,
    pub blockers: Vec<String>,
    pub facts: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KernelLoopbackR4ExpandedOptInRolloutAuditReport {
    pub runtime_id: String,
    pub component: String,
    pub kernel_area: String,
    pub mutates_runtime: bool,
    pub live_execution_allowed: bool,
    pub current_platform: String,
    pub current_arch: String,
    pub canary_scope: String,
    pub max_canary_sessions: u16,
    pub audit_ready: bool,
    pub limited_rollout_allowed: bool,
    pub expanded_opt_in_allowed: bool,
    pub gate: KernelLoopbackR4ExpandedOptInLimitedRolloutGateReport,
    pub rows: Vec<KernelLoopbackR4ExpandedOptInRolloutAuditRow>,
    pub default_route: bool,
    pub forwards_traffic: bool,
    pub outbound_adapters_used: bool,
    pub mihomo_fallback: bool,
    pub passed: bool,
    pub blockers: Vec<String>,
    pub warnings: Vec<String>,
    pub facts: Vec<String>,
    pub next_safe_batch: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KernelLoopbackR4ExpandedOptInCloseoutReadinessReport {
    pub runtime_id: String,
    pub component: String,
    pub kernel_area: String,
    pub mutates_runtime: bool,
    pub live_execution_allowed: bool,
    pub current_platform: String,
    pub current_arch: String,
    pub closeout_decision: bool,
    pub closeout_ready: bool,
    pub limited_rollout_allowed: bool,
    pub expanded_opt_in_allowed: bool,
    pub audit: KernelLoopbackR4ExpandedOptInRolloutAuditReport,
    pub checks: Vec<KernelLoopbackR4ExpandedOptInLimitedRolloutGateCheck>,
    pub default_route: bool,
    pub forwards_traffic: bool,
    pub outbound_adapters_used: bool,
    pub mihomo_fallback: bool,
    pub passed: bool,
    pub blockers: Vec<String>,
    pub warnings: Vec<String>,
    pub facts: Vec<String>,
    pub next_safe_batch: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KernelLoopbackR4ExpandedOptInCloseoutReport {
    pub runtime_id: String,
    pub component: String,
    pub kernel_area: String,
    pub mutates_runtime: bool,
    pub live_execution_allowed: bool,
    pub current_platform: String,
    pub current_arch: String,
    pub requested_execution: bool,
    pub explicit_decision: bool,
    pub closeout_decision: bool,
    pub closeout_ready: bool,
    pub r4_closeout_complete: bool,
    pub limited_rollout_allowed: bool,
    pub expanded_opt_in_allowed: bool,
    pub closeout_readiness: KernelLoopbackR4ExpandedOptInCloseoutReadinessReport,
    pub evidence: Vec<KernelLoopbackR4ExpandedOptInLimitedRolloutGateCheck>,
    pub default_route: bool,
    pub forwards_traffic: bool,
    pub outbound_adapters_used: bool,
    pub mihomo_fallback: bool,
    pub passed: bool,
    pub blockers: Vec<String>,
    pub warnings: Vec<String>,
    pub facts: Vec<String>,
    pub next_safe_batch: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KernelLoopbackR4ExpandedOptInCompletionReport {
    pub runtime_id: String,
    pub component: String,
    pub kernel_area: String,
    pub mutates_runtime: bool,
    pub live_execution_allowed: bool,
    pub current_platform: String,
    pub current_arch: String,
    pub r4_complete: bool,
    pub completed_batches: Vec<String>,
    pub open_boundaries: Vec<String>,
    pub next_phase_candidate: String,
    pub default_cutover_allowed: bool,
    pub expanded_opt_in_allowed: bool,
    pub closeout_report: KernelLoopbackR4ExpandedOptInCloseoutReport,
    pub passed: bool,
    pub blockers: Vec<String>,
    pub warnings: Vec<String>,
    pub facts: Vec<String>,
    pub next_safe_batch: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KernelLoopbackR4ExpandedOptInNextPhaseHandoffReport {
    pub runtime_id: String,
    pub component: String,
    pub kernel_area: String,
    pub mutates_runtime: bool,
    pub live_execution_allowed: bool,
    pub current_platform: String,
    pub current_arch: String,
    pub handoff_decision: bool,
    pub handoff_ready: bool,
    pub next_phase: String,
    pub default_cutover_allowed: bool,
    pub expanded_opt_in_allowed: bool,
    pub completion: KernelLoopbackR4ExpandedOptInCompletionReport,
    pub checks: Vec<KernelLoopbackR4ExpandedOptInLimitedRolloutGateCheck>,
    pub default_route: bool,
    pub forwards_traffic: bool,
    pub outbound_adapters_used: bool,
    pub mihomo_fallback: bool,
    pub passed: bool,
    pub blockers: Vec<String>,
    pub warnings: Vec<String>,
    pub facts: Vec<String>,
    pub next_safe_batch: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KernelLoopbackR5DefaultCutoverPreflightReport {
    pub runtime_id: String,
    pub component: String,
    pub kernel_area: String,
    pub mutates_runtime: bool,
    pub live_execution_allowed: bool,
    pub current_platform: String,
    pub current_arch: String,
    pub r5_preflight_decision: bool,
    pub preflight_ready: bool,
    pub default_cutover_allowed: bool,
    pub expanded_opt_in_allowed: bool,
    pub handoff: KernelLoopbackR4ExpandedOptInNextPhaseHandoffReport,
    pub checks: Vec<KernelLoopbackR4ExpandedOptInLimitedRolloutGateCheck>,
    pub default_route: bool,
    pub forwards_traffic: bool,
    pub outbound_adapters_used: bool,
    pub mihomo_fallback: bool,
    pub passed: bool,
    pub blockers: Vec<String>,
    pub warnings: Vec<String>,
    pub facts: Vec<String>,
    pub next_safe_batch: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KernelLoopbackR5DefaultCutoverRiskRow {
    pub name: String,
    pub severity: String,
    pub status: String,
    pub passed: bool,
    pub blockers: Vec<String>,
    pub facts: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KernelLoopbackR5DefaultCutoverRiskMatrixReport {
    pub runtime_id: String,
    pub component: String,
    pub kernel_area: String,
    pub mutates_runtime: bool,
    pub live_execution_allowed: bool,
    pub current_platform: String,
    pub current_arch: String,
    pub risk_matrix_ready: bool,
    pub default_cutover_allowed: bool,
    pub expanded_opt_in_allowed: bool,
    pub preflight: KernelLoopbackR5DefaultCutoverPreflightReport,
    pub rows: Vec<KernelLoopbackR5DefaultCutoverRiskRow>,
    pub default_route: bool,
    pub forwards_traffic: bool,
    pub outbound_adapters_used: bool,
    pub mihomo_fallback: bool,
    pub passed: bool,
    pub blockers: Vec<String>,
    pub warnings: Vec<String>,
    pub facts: Vec<String>,
    pub next_safe_batch: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KernelLoopbackR5DefaultCutoverRollbackAbortPlanReport {
    pub runtime_id: String,
    pub component: String,
    pub kernel_area: String,
    pub mutates_runtime: bool,
    pub live_execution_allowed: bool,
    pub current_platform: String,
    pub current_arch: String,
    pub rollback_plan_decision: bool,
    pub rollback_abort_ready: bool,
    pub default_cutover_allowed: bool,
    pub expanded_opt_in_allowed: bool,
    pub risk_matrix: KernelLoopbackR5DefaultCutoverRiskMatrixReport,
    pub checks: Vec<KernelLoopbackR4ExpandedOptInLimitedRolloutGateCheck>,
    pub default_route: bool,
    pub forwards_traffic: bool,
    pub outbound_adapters_used: bool,
    pub mihomo_fallback: bool,
    pub passed: bool,
    pub blockers: Vec<String>,
    pub warnings: Vec<String>,
    pub facts: Vec<String>,
    pub next_safe_batch: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KernelLoopbackR5DefaultCutoverExecutionPlanStep {
    pub order: u8,
    pub name: String,
    pub phase: String,
    pub allowed: bool,
    pub mutates_runtime: bool,
    pub facts: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KernelLoopbackR5DefaultCutoverExecutionPlanReport {
    pub runtime_id: String,
    pub component: String,
    pub kernel_area: String,
    pub mutates_runtime: bool,
    pub live_execution_allowed: bool,
    pub current_platform: String,
    pub current_arch: String,
    pub execution_plan_decision: bool,
    pub execution_plan_ready: bool,
    pub default_cutover_allowed: bool,
    pub expanded_opt_in_allowed: bool,
    pub rollback_abort_plan: KernelLoopbackR5DefaultCutoverRollbackAbortPlanReport,
    pub steps: Vec<KernelLoopbackR5DefaultCutoverExecutionPlanStep>,
    pub default_route: bool,
    pub forwards_traffic: bool,
    pub outbound_adapters_used: bool,
    pub mihomo_fallback: bool,
    pub passed: bool,
    pub blockers: Vec<String>,
    pub warnings: Vec<String>,
    pub facts: Vec<String>,
    pub next_safe_batch: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KernelLoopbackR5DefaultCutoverGuardReport {
    pub runtime_id: String,
    pub component: String,
    pub kernel_area: String,
    pub mutates_runtime: bool,
    pub live_execution_allowed: bool,
    pub current_platform: String,
    pub current_arch: String,
    pub guard_decision: bool,
    pub guard_ready: bool,
    pub default_cutover_allowed: bool,
    pub expanded_opt_in_allowed: bool,
    pub execution_plan: KernelLoopbackR5DefaultCutoverExecutionPlanReport,
    pub checks: Vec<KernelLoopbackR4ExpandedOptInLimitedRolloutGateCheck>,
    pub default_route: bool,
    pub forwards_traffic: bool,
    pub outbound_adapters_used: bool,
    pub mihomo_fallback: bool,
    pub passed: bool,
    pub blockers: Vec<String>,
    pub warnings: Vec<String>,
    pub facts: Vec<String>,
    pub next_safe_batch: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KernelLoopbackR5DefaultCutoverDryRunReadinessReport {
    pub runtime_id: String,
    pub component: String,
    pub kernel_area: String,
    pub mutates_runtime: bool,
    pub live_execution_allowed: bool,
    pub current_platform: String,
    pub current_arch: String,
    pub dry_run_decision: bool,
    pub dry_run_ready: bool,
    pub default_cutover_allowed: bool,
    pub expanded_opt_in_allowed: bool,
    pub guard: KernelLoopbackR5DefaultCutoverGuardReport,
    pub checks: Vec<KernelLoopbackR4ExpandedOptInLimitedRolloutGateCheck>,
    pub default_route: bool,
    pub forwards_traffic: bool,
    pub outbound_adapters_used: bool,
    pub mihomo_fallback: bool,
    pub passed: bool,
    pub blockers: Vec<String>,
    pub warnings: Vec<String>,
    pub facts: Vec<String>,
    pub next_safe_batch: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KernelLoopbackR5DefaultCutoverDryRunEvidenceReport {
    pub runtime_id: String,
    pub component: String,
    pub kernel_area: String,
    pub mutates_runtime: bool,
    pub live_execution_allowed: bool,
    pub current_platform: String,
    pub current_arch: String,
    pub dry_run_executed: bool,
    pub default_cutover_allowed: bool,
    pub expanded_opt_in_allowed: bool,
    pub readiness: KernelLoopbackR5DefaultCutoverDryRunReadinessReport,
    pub checks: Vec<KernelLoopbackR4ExpandedOptInLimitedRolloutGateCheck>,
    pub default_route: bool,
    pub forwards_traffic: bool,
    pub outbound_adapters_used: bool,
    pub mihomo_fallback: bool,
    pub passed: bool,
    pub blockers: Vec<String>,
    pub warnings: Vec<String>,
    pub facts: Vec<String>,
    pub next_safe_batch: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KernelLoopbackR5DefaultCutoverDryRunCloseoutReport {
    pub runtime_id: String,
    pub component: String,
    pub kernel_area: String,
    pub mutates_runtime: bool,
    pub live_execution_allowed: bool,
    pub current_platform: String,
    pub current_arch: String,
    pub dry_run_closeout_ready: bool,
    pub default_cutover_allowed: bool,
    pub expanded_opt_in_allowed: bool,
    pub evidence: KernelLoopbackR5DefaultCutoverDryRunEvidenceReport,
    pub checks: Vec<KernelLoopbackR4ExpandedOptInLimitedRolloutGateCheck>,
    pub default_route: bool,
    pub forwards_traffic: bool,
    pub outbound_adapters_used: bool,
    pub mihomo_fallback: bool,
    pub passed: bool,
    pub blockers: Vec<String>,
    pub warnings: Vec<String>,
    pub facts: Vec<String>,
    pub next_safe_batch: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KernelLoopbackR5DefaultCutoverPostDryRunHoldReport {
    pub runtime_id: String,
    pub component: String,
    pub kernel_area: String,
    pub mutates_runtime: bool,
    pub live_execution_allowed: bool,
    pub current_platform: String,
    pub current_arch: String,
    pub hold_decision: bool,
    pub hold_ready: bool,
    pub default_cutover_allowed: bool,
    pub expanded_opt_in_allowed: bool,
    pub closeout: KernelLoopbackR5DefaultCutoverDryRunCloseoutReport,
    pub checks: Vec<KernelLoopbackR4ExpandedOptInLimitedRolloutGateCheck>,
    pub default_route: bool,
    pub forwards_traffic: bool,
    pub outbound_adapters_used: bool,
    pub mihomo_fallback: bool,
    pub passed: bool,
    pub blockers: Vec<String>,
    pub warnings: Vec<String>,
    pub facts: Vec<String>,
    pub next_safe_batch: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KernelLoopbackR5DefaultCutoverDecisionReadinessReport {
    pub runtime_id: String,
    pub component: String,
    pub kernel_area: String,
    pub mutates_runtime: bool,
    pub live_execution_allowed: bool,
    pub current_platform: String,
    pub current_arch: String,
    pub decision_readiness_decision: bool,
    pub decision_ready: bool,
    pub default_cutover_allowed: bool,
    pub expanded_opt_in_allowed: bool,
    pub post_dry_run_hold: KernelLoopbackR5DefaultCutoverPostDryRunHoldReport,
    pub checks: Vec<KernelLoopbackR4ExpandedOptInLimitedRolloutGateCheck>,
    pub default_route: bool,
    pub forwards_traffic: bool,
    pub outbound_adapters_used: bool,
    pub mihomo_fallback: bool,
    pub passed: bool,
    pub blockers: Vec<String>,
    pub warnings: Vec<String>,
    pub facts: Vec<String>,
    pub next_safe_batch: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KernelLoopbackR5DefaultCutoverFinalGateReport {
    pub runtime_id: String,
    pub component: String,
    pub kernel_area: String,
    pub mutates_runtime: bool,
    pub live_execution_allowed: bool,
    pub current_platform: String,
    pub current_arch: String,
    pub final_gate_decision: bool,
    pub final_gate_ready: bool,
    pub default_cutover_allowed: bool,
    pub expanded_opt_in_allowed: bool,
    pub decision_readiness: KernelLoopbackR5DefaultCutoverDecisionReadinessReport,
    pub checks: Vec<KernelLoopbackR4ExpandedOptInLimitedRolloutGateCheck>,
    pub default_route: bool,
    pub forwards_traffic: bool,
    pub outbound_adapters_used: bool,
    pub mihomo_fallback: bool,
    pub passed: bool,
    pub blockers: Vec<String>,
    pub warnings: Vec<String>,
    pub facts: Vec<String>,
    pub next_safe_batch: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KernelLoopbackR5DefaultCutoverNextStepHandoffReport {
    pub runtime_id: String,
    pub component: String,
    pub kernel_area: String,
    pub mutates_runtime: bool,
    pub live_execution_allowed: bool,
    pub current_platform: String,
    pub current_arch: String,
    pub r5_handoff_decision: bool,
    pub handoff_ready: bool,
    pub next_step: String,
    pub default_cutover_allowed: bool,
    pub expanded_opt_in_allowed: bool,
    pub final_gate: KernelLoopbackR5DefaultCutoverFinalGateReport,
    pub checks: Vec<KernelLoopbackR4ExpandedOptInLimitedRolloutGateCheck>,
    pub default_route: bool,
    pub forwards_traffic: bool,
    pub outbound_adapters_used: bool,
    pub mihomo_fallback: bool,
    pub passed: bool,
    pub blockers: Vec<String>,
    pub warnings: Vec<String>,
    pub facts: Vec<String>,
    pub next_safe_batch: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KernelLoopbackR5DefaultCutoverFinalHoldReport {
    pub runtime_id: String,
    pub component: String,
    pub kernel_area: String,
    pub mutates_runtime: bool,
    pub live_execution_allowed: bool,
    pub current_platform: String,
    pub current_arch: String,
    pub final_hold_started_at_epoch_ms: Option<u64>,
    pub final_hold_elapsed_seconds: u64,
    pub final_hold_decision: bool,
    pub final_hold_ready: bool,
    pub default_cutover_allowed: bool,
    pub expanded_opt_in_allowed: bool,
    pub handoff: KernelLoopbackR5DefaultCutoverNextStepHandoffReport,
    pub checks: Vec<KernelLoopbackR4ExpandedOptInLimitedRolloutGateCheck>,
    pub default_route: bool,
    pub forwards_traffic: bool,
    pub outbound_adapters_used: bool,
    pub mihomo_fallback: bool,
    pub passed: bool,
    pub blockers: Vec<String>,
    pub warnings: Vec<String>,
    pub facts: Vec<String>,
    pub next_safe_batch: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KernelLoopbackR5DefaultCutoverIndependentRollbackValidationReport {
    pub runtime_id: String,
    pub component: String,
    pub kernel_area: String,
    pub mutates_runtime: bool,
    pub live_execution_allowed: bool,
    pub current_platform: String,
    pub current_arch: String,
    pub independent_rollback_decision: bool,
    pub rollback_validation_ready: bool,
    pub required_platforms: Vec<String>,
    pub observed_rollback_platforms: Vec<String>,
    pub pending_rollback_platforms: Vec<String>,
    pub default_cutover_allowed: bool,
    pub expanded_opt_in_allowed: bool,
    pub final_hold: KernelLoopbackR5DefaultCutoverFinalHoldReport,
    pub checks: Vec<KernelLoopbackR4ExpandedOptInLimitedRolloutGateCheck>,
    pub default_route: bool,
    pub forwards_traffic: bool,
    pub outbound_adapters_used: bool,
    pub mihomo_fallback: bool,
    pub passed: bool,
    pub blockers: Vec<String>,
    pub warnings: Vec<String>,
    pub facts: Vec<String>,
    pub next_safe_batch: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KernelLoopbackR5DefaultCutoverCloseoutReadinessReport {
    pub runtime_id: String,
    pub component: String,
    pub kernel_area: String,
    pub mutates_runtime: bool,
    pub live_execution_allowed: bool,
    pub current_platform: String,
    pub current_arch: String,
    pub r5_closeout_decision: bool,
    pub closeout_ready: bool,
    pub default_cutover_allowed: bool,
    pub expanded_opt_in_allowed: bool,
    pub rollback_validation: KernelLoopbackR5DefaultCutoverIndependentRollbackValidationReport,
    pub checks: Vec<KernelLoopbackR4ExpandedOptInLimitedRolloutGateCheck>,
    pub default_route: bool,
    pub forwards_traffic: bool,
    pub outbound_adapters_used: bool,
    pub mihomo_fallback: bool,
    pub passed: bool,
    pub blockers: Vec<String>,
    pub warnings: Vec<String>,
    pub facts: Vec<String>,
    pub next_safe_batch: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KernelLoopbackR5DefaultCutoverCloseoutReport {
    pub runtime_id: String,
    pub component: String,
    pub kernel_area: String,
    pub mutates_runtime: bool,
    pub live_execution_allowed: bool,
    pub current_platform: String,
    pub current_arch: String,
    pub r5_closeout_report_decision: bool,
    pub r5_closeout_complete: bool,
    pub default_cutover_allowed: bool,
    pub expanded_opt_in_allowed: bool,
    pub closeout_readiness: KernelLoopbackR5DefaultCutoverCloseoutReadinessReport,
    pub completed_evidence_batches: Vec<String>,
    pub open_boundaries: Vec<String>,
    pub passed: bool,
    pub blockers: Vec<String>,
    pub warnings: Vec<String>,
    pub facts: Vec<String>,
    pub next_safe_batch: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum KernelRuntimeKind {
    Mihomo,
    Rust,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KernelRuntimeCapability {
    pub name: String,
    pub status: String,
    pub supported: bool,
    pub fallback_required: bool,
    pub facts: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RustKernelRuntimeCandidateReport {
    pub runtime_id: String,
    pub kind: KernelRuntimeKind,
    pub mutates_runtime: bool,
    pub selectable: bool,
    pub default_allowed: bool,
    pub mihomo_fallback: bool,
    pub supported_safe_subset: Vec<String>,
    pub fallback_boundaries: Vec<String>,
    pub capabilities: Vec<KernelRuntimeCapability>,
    pub blockers: Vec<String>,
    pub warnings: Vec<String>,
    pub facts: Vec<String>,
    pub next_safe_batch: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KernelRuntimeSelectionScaffoldReport {
    pub runtime_id: String,
    pub component: String,
    pub mutates_runtime: bool,
    pub current_default_runtime_kind: KernelRuntimeKind,
    pub requested_runtime_kind: KernelRuntimeKind,
    pub selected_runtime_kind: KernelRuntimeKind,
    pub rust_runtime_opt_in_decision: bool,
    pub rust_candidate_available: bool,
    pub rust_candidate_default_allowed: bool,
    pub mihomo_fallback: bool,
    pub rust_candidate: RustKernelRuntimeCandidateReport,
    pub blockers: Vec<String>,
    pub warnings: Vec<String>,
    pub facts: Vec<String>,
    pub next_safe_batch: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KernelLoopbackR5CloseoutR6RustRuntimeScaffoldReport {
    pub runtime_id: String,
    pub component: String,
    pub mutates_runtime: bool,
    pub live_execution_allowed: bool,
    pub rust_runtime_scaffold_decision: bool,
    pub scaffold_ready: bool,
    pub default_cutover_allowed: bool,
    pub r5_closeout: KernelLoopbackR5DefaultCutoverCloseoutReport,
    pub runtime_selection: KernelRuntimeSelectionScaffoldReport,
    pub checks: Vec<KernelLoopbackR4ExpandedOptInLimitedRolloutGateCheck>,
    pub blockers: Vec<String>,
    pub warnings: Vec<String>,
    pub facts: Vec<String>,
    pub next_safe_batch: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RustKernelRuntimeSupportedSubsetReport {
    pub runtime_id: String,
    pub component: String,
    pub mutates_runtime: bool,
    pub rule_decision_owned: bool,
    pub dns_decision_owned: bool,
    pub adapter_decision_owned: bool,
    pub forwarding_surface_owned: bool,
    pub app_rule_count: usize,
    pub app_proxy_count: usize,
    pub supported_subset: Vec<String>,
    pub fallback_boundaries: Vec<String>,
    pub blockers: Vec<String>,
    pub warnings: Vec<String>,
    pub facts: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RustKernelRuntimeHealthStateReport {
    pub runtime_id: String,
    pub component: String,
    pub status: String,
    pub health_ready: bool,
    pub rollback_armed: bool,
    pub mihomo_fallback: bool,
    pub observed_checks: Vec<String>,
    pub blockers: Vec<String>,
    pub warnings: Vec<String>,
    pub facts: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KernelLoopbackR6OptInRustRuntimeMvpReport {
    pub runtime_id: String,
    pub component: String,
    pub mutates_runtime: bool,
    pub live_execution_allowed: bool,
    pub rust_runtime_opt_in_decision: bool,
    pub requested_runtime_kind: KernelRuntimeKind,
    pub selected_runtime_kind: KernelRuntimeKind,
    pub opt_in_ready: bool,
    pub default_cutover_allowed: bool,
    pub mihomo_fallback: bool,
    pub scaffold: KernelLoopbackR5CloseoutR6RustRuntimeScaffoldReport,
    pub supported_subset: RustKernelRuntimeSupportedSubsetReport,
    pub health_state: RustKernelRuntimeHealthStateReport,
    pub loopback_forwarding_evidence: Option<KernelLoopbackForwardingRollbackDrillReport>,
    pub checks: Vec<KernelLoopbackR4ExpandedOptInLimitedRolloutGateCheck>,
    pub blockers: Vec<String>,
    pub warnings: Vec<String>,
    pub facts: Vec<String>,
    pub next_safe_batch: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RustKernelRuntimeCanaryProfileReport {
    pub runtime_id: String,
    pub component: String,
    pub canary_scope: String,
    pub max_canary_sessions: u16,
    pub capped_profile: bool,
    pub supported_safe_subset: Vec<String>,
    pub fallback_boundaries: Vec<String>,
    pub blockers: Vec<String>,
    pub warnings: Vec<String>,
    pub facts: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RustKernelRuntimeAutomaticFallbackReport {
    pub runtime_id: String,
    pub component: String,
    pub health_check_passed: bool,
    pub rollback_triggered: bool,
    pub health_ready: bool,
    pub rollback_armed: bool,
    pub fallback_activated: bool,
    pub selected_runtime_kind: KernelRuntimeKind,
    pub fallback_runtime_kind: KernelRuntimeKind,
    pub triggers: Vec<String>,
    pub blockers: Vec<String>,
    pub facts: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KernelLoopbackR6RustDefaultCanaryReport {
    pub runtime_id: String,
    pub component: String,
    pub mutates_runtime: bool,
    pub live_execution_allowed: bool,
    pub rust_runtime_opt_in_decision: bool,
    pub canary_default_decision: bool,
    pub requested_runtime_kind: KernelRuntimeKind,
    pub selected_runtime_kind: KernelRuntimeKind,
    pub canary_default_allowed: bool,
    pub production_default_allowed: bool,
    pub mihomo_fallback: bool,
    pub r6_opt_in: KernelLoopbackR6OptInRustRuntimeMvpReport,
    pub canary_profile: RustKernelRuntimeCanaryProfileReport,
    pub automatic_fallback: RustKernelRuntimeAutomaticFallbackReport,
    pub checks: Vec<KernelLoopbackR4ExpandedOptInLimitedRolloutGateCheck>,
    pub blockers: Vec<String>,
    pub warnings: Vec<String>,
    pub facts: Vec<String>,
    pub next_safe_batch: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RustKernelRuntimeCanaryCloseoutSummaryReport {
    pub runtime_id: String,
    pub component: String,
    pub canary_default_allowed: bool,
    pub canary_health_ready: bool,
    pub automatic_fallback_armed: bool,
    pub rollback_hold_passed: bool,
    pub closeout_ready: bool,
    pub evidence: Vec<String>,
    pub blockers: Vec<String>,
    pub facts: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RustKernelRuntimeSupportedProfileDefaultReport {
    pub runtime_id: String,
    pub component: String,
    pub profile_scope: String,
    pub supported_profile_default: bool,
    pub selected_runtime_kind: KernelRuntimeKind,
    pub fallback_runtime_kind: KernelRuntimeKind,
    pub supported_safe_subset: Vec<String>,
    pub fallback_boundaries: Vec<String>,
    pub blockers: Vec<String>,
    pub warnings: Vec<String>,
    pub facts: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RustKernelRuntimeFallbackStateReport {
    pub runtime_id: String,
    pub component: String,
    pub rollback_switch_requested: bool,
    pub restart_required: bool,
    pub health_ready: bool,
    pub rollback_armed: bool,
    pub fallback_active: bool,
    pub selected_runtime_kind: KernelRuntimeKind,
    pub fallback_runtime_kind: KernelRuntimeKind,
    pub triggers: Vec<String>,
    pub blockers: Vec<String>,
    pub facts: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KernelLoopbackR7RustDefaultCutoverReport {
    pub runtime_id: String,
    pub component: String,
    pub mutates_runtime: bool,
    pub live_execution_allowed: bool,
    pub rust_runtime_opt_in_decision: bool,
    pub canary_default_decision: bool,
    pub r7_cutover_decision: bool,
    pub rollback_hold_decision: bool,
    pub rollback_switch_requested: bool,
    pub requested_runtime_kind: KernelRuntimeKind,
    pub selected_runtime_kind: KernelRuntimeKind,
    pub supported_profile_default_allowed: bool,
    pub production_default_allowed: bool,
    pub mihomo_fallback: bool,
    pub r6_canary: KernelLoopbackR6RustDefaultCanaryReport,
    pub canary_closeout: RustKernelRuntimeCanaryCloseoutSummaryReport,
    pub supported_profile: RustKernelRuntimeSupportedProfileDefaultReport,
    pub fallback_state: RustKernelRuntimeFallbackStateReport,
    pub checks: Vec<KernelLoopbackR4ExpandedOptInLimitedRolloutGateCheck>,
    pub blockers: Vec<String>,
    pub warnings: Vec<String>,
    pub facts: Vec<String>,
    pub next_safe_batch: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RustKernelRuntimeFallbackRetirementParityReport {
    pub runtime_id: String,
    pub component: String,
    pub protocol_parity_passed: bool,
    pub tun_parity_passed: bool,
    pub adapter_parity_passed: bool,
    pub dns_runtime_parity_passed: bool,
    pub cross_platform_rollback_passed: bool,
    pub soak_evidence_passed: bool,
    pub parity_complete: bool,
    pub retained_boundaries: Vec<String>,
    pub blockers: Vec<String>,
    pub facts: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RustKernelRuntimeFallbackRetirementPlanReport {
    pub runtime_id: String,
    pub component: String,
    pub fallback_retirement_decision: bool,
    pub emergency_rollback_decision: bool,
    pub rollback_switch_requested: bool,
    pub fallback_retirement_allowed: bool,
    pub selected_runtime_kind: KernelRuntimeKind,
    pub rollback_runtime_kind: KernelRuntimeKind,
    pub restart_required: bool,
    pub blockers: Vec<String>,
    pub warnings: Vec<String>,
    pub facts: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KernelLoopbackR7MihomoFallbackRetirementReport {
    pub runtime_id: String,
    pub component: String,
    pub mutates_runtime: bool,
    pub live_execution_allowed: bool,
    pub r7_cutover: KernelLoopbackR7RustDefaultCutoverReport,
    pub parity: RustKernelRuntimeFallbackRetirementParityReport,
    pub retirement_plan: RustKernelRuntimeFallbackRetirementPlanReport,
    pub production_default_allowed: bool,
    pub mihomo_fallback_retired: bool,
    pub selected_runtime_kind: KernelRuntimeKind,
    pub rollback_runtime_kind: KernelRuntimeKind,
    pub checks: Vec<KernelLoopbackR4ExpandedOptInLimitedRolloutGateCheck>,
    pub blockers: Vec<String>,
    pub warnings: Vec<String>,
    pub facts: Vec<String>,
    pub next_safe_batch: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RustKernelRuntimeExtendedSoakReport {
    pub runtime_id: String,
    pub component: String,
    pub min_soak_hours: u32,
    pub observed_soak_hours: u32,
    pub health_regression_count: u32,
    pub rollback_trigger_count: u32,
    pub soak_complete: bool,
    pub blockers: Vec<String>,
    pub facts: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RustKernelRuntimeRollbackTelemetryReport {
    pub runtime_id: String,
    pub component: String,
    pub rollback_telemetry_decision: bool,
    pub emergency_rollback_ready: bool,
    pub rollback_event_count: u32,
    pub last_rollback_event_ts: Option<u64>,
    pub telemetry_complete: bool,
    pub blockers: Vec<String>,
    pub facts: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RustKernelRuntimePlatformHardeningFollowUpReport {
    pub runtime_id: String,
    pub component: String,
    pub windows_service_hardening: bool,
    pub macos_service_hardening: bool,
    pub linux_service_hardening: bool,
    pub platform_follow_up_complete: bool,
    pub blockers: Vec<String>,
    pub facts: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KernelLoopbackFullRustRuntimeHardeningReport {
    pub runtime_id: String,
    pub component: String,
    pub mutates_runtime: bool,
    pub live_execution_allowed: bool,
    pub hardening_decision: bool,
    pub r7_fallback_retirement_passed: bool,
    pub extended_soak: RustKernelRuntimeExtendedSoakReport,
    pub rollback_telemetry: RustKernelRuntimeRollbackTelemetryReport,
    pub platform_follow_up: RustKernelRuntimePlatformHardeningFollowUpReport,
    pub full_rust_runtime_hardened: bool,
    pub production_default_allowed: bool,
    pub selected_runtime_kind: KernelRuntimeKind,
    pub rollback_runtime_kind: KernelRuntimeKind,
    pub checks: Vec<KernelLoopbackR4ExpandedOptInLimitedRolloutGateCheck>,
    pub blockers: Vec<String>,
    pub warnings: Vec<String>,
    pub facts: Vec<String>,
    pub next_safe_batch: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RustKernelRuntimeGoMihomoRetirementSurfaceAuditReport {
    pub runtime_id: String,
    pub component: String,
    pub sidecar_source_audit_passed: bool,
    pub bundled_mihomo_audit_passed: bool,
    pub ipc_fallback_audit_passed: bool,
    pub docs_audit_passed: bool,
    pub emergency_rollback_retained: bool,
    pub audit_complete: bool,
    pub remaining_surfaces: Vec<String>,
    pub blockers: Vec<String>,
    pub facts: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KernelLoopbackGoMihomoRetirementAuditReport {
    pub runtime_id: String,
    pub component: String,
    pub mutates_runtime: bool,
    pub live_execution_allowed: bool,
    pub full_rust_runtime_hardened: bool,
    pub surface_audit: RustKernelRuntimeGoMihomoRetirementSurfaceAuditReport,
    pub final_retirement_audit_decision: bool,
    pub go_mihomo_retirement_audit_complete: bool,
    pub selected_runtime_kind: KernelRuntimeKind,
    pub rollback_runtime_kind: KernelRuntimeKind,
    pub checks: Vec<KernelLoopbackR4ExpandedOptInLimitedRolloutGateCheck>,
    pub blockers: Vec<String>,
    pub warnings: Vec<String>,
    pub facts: Vec<String>,
    pub next_safe_batch: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RustKernelRuntimeGoMihomoRetirementRemovalPlanReport {
    pub runtime_id: String,
    pub component: String,
    pub sidecar_source_removal_plan: bool,
    pub bundled_artifact_deprecation_plan: bool,
    pub ipc_fallback_replacement_plan: bool,
    pub emergency_rollback_preservation_plan: bool,
    pub release_rollout_plan: bool,
    pub removal_plan_complete: bool,
    pub planned_removal_surfaces: Vec<String>,
    pub blockers: Vec<String>,
    pub facts: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KernelLoopbackGoMihomoRetirementPlanReport {
    pub runtime_id: String,
    pub component: String,
    pub mutates_runtime: bool,
    pub live_execution_allowed: bool,
    pub go_mihomo_retirement_audit_complete: bool,
    pub removal_plan: RustKernelRuntimeGoMihomoRetirementRemovalPlanReport,
    pub final_retirement_plan_decision: bool,
    pub go_mihomo_retirement_plan_complete: bool,
    pub selected_runtime_kind: KernelRuntimeKind,
    pub rollback_runtime_kind: KernelRuntimeKind,
    pub checks: Vec<KernelLoopbackR4ExpandedOptInLimitedRolloutGateCheck>,
    pub blockers: Vec<String>,
    pub warnings: Vec<String>,
    pub facts: Vec<String>,
    pub next_safe_batch: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RustKernelRuntimeGoMihomoRetirementExecutionGuardReport {
    pub runtime_id: String,
    pub component: String,
    pub removal_manifest_ready: bool,
    pub abort_plan_ready: bool,
    pub staged_rollout_guard_ready: bool,
    pub emergency_rollback_drill_passed: bool,
    pub operator_acknowledgement: bool,
    pub execution_guard_complete: bool,
    pub guarded_execution_surfaces: Vec<String>,
    pub blockers: Vec<String>,
    pub facts: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KernelLoopbackGoMihomoRetirementExecutionGuardReport {
    pub runtime_id: String,
    pub component: String,
    pub mutates_runtime: bool,
    pub live_execution_allowed: bool,
    pub go_mihomo_retirement_plan_complete: bool,
    pub execution_guard: RustKernelRuntimeGoMihomoRetirementExecutionGuardReport,
    pub final_execution_guard_decision: bool,
    pub go_mihomo_retirement_execution_guard_complete: bool,
    pub selected_runtime_kind: KernelRuntimeKind,
    pub rollback_runtime_kind: KernelRuntimeKind,
    pub checks: Vec<KernelLoopbackR4ExpandedOptInLimitedRolloutGateCheck>,
    pub blockers: Vec<String>,
    pub warnings: Vec<String>,
    pub facts: Vec<String>,
    pub next_safe_batch: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RustKernelRuntimeGoMihomoRetirementDryRunReport {
    pub runtime_id: String,
    pub component: String,
    pub dry_run_manifest_replayed: bool,
    pub no_source_mutations_observed: bool,
    pub no_bundled_artifact_mutations_observed: bool,
    pub rollback_rehearsal_passed: bool,
    pub dry_run_report_archived: bool,
    pub dry_run_complete: bool,
    pub simulated_removal_surfaces: Vec<String>,
    pub blockers: Vec<String>,
    pub facts: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KernelLoopbackGoMihomoRetirementDryRunReport {
    pub runtime_id: String,
    pub component: String,
    pub mutates_runtime: bool,
    pub live_execution_allowed: bool,
    pub go_mihomo_retirement_execution_guard_complete: bool,
    pub dry_run: RustKernelRuntimeGoMihomoRetirementDryRunReport,
    pub final_dry_run_decision: bool,
    pub go_mihomo_retirement_dry_run_complete: bool,
    pub selected_runtime_kind: KernelRuntimeKind,
    pub rollback_runtime_kind: KernelRuntimeKind,
    pub checks: Vec<KernelLoopbackR4ExpandedOptInLimitedRolloutGateCheck>,
    pub blockers: Vec<String>,
    pub warnings: Vec<String>,
    pub facts: Vec<String>,
    pub next_safe_batch: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RustKernelRuntimeGoMihomoRetirementCloseoutReport {
    pub runtime_id: String,
    pub component: String,
    pub dry_run_evidence_reviewed: bool,
    pub closeout_report_archived: bool,
    pub rollback_checkpoint_verified: bool,
    pub artifact_inventory_frozen: bool,
    pub no_removal_mutations_observed: bool,
    pub closeout_complete: bool,
    pub closed_out_surfaces: Vec<String>,
    pub blockers: Vec<String>,
    pub facts: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KernelLoopbackGoMihomoRetirementCloseoutReport {
    pub runtime_id: String,
    pub component: String,
    pub mutates_runtime: bool,
    pub live_execution_allowed: bool,
    pub go_mihomo_retirement_dry_run_complete: bool,
    pub closeout: RustKernelRuntimeGoMihomoRetirementCloseoutReport,
    pub final_closeout_decision: bool,
    pub go_mihomo_retirement_closeout_complete: bool,
    pub selected_runtime_kind: KernelRuntimeKind,
    pub rollback_runtime_kind: KernelRuntimeKind,
    pub checks: Vec<KernelLoopbackR4ExpandedOptInLimitedRolloutGateCheck>,
    pub blockers: Vec<String>,
    pub warnings: Vec<String>,
    pub facts: Vec<String>,
    pub next_safe_batch: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RustKernelRuntimeGoMihomoRetirementFinalRemovalGateReport {
    pub runtime_id: String,
    pub component: String,
    pub closeout_evidence_accepted: bool,
    pub rollback_boundary_locked: bool,
    pub removal_scope_locked: bool,
    pub release_blocker_review_passed: bool,
    pub final_operator_approval: bool,
    pub final_removal_gate_complete: bool,
    pub approved_removal_surfaces: Vec<String>,
    pub blockers: Vec<String>,
    pub facts: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KernelLoopbackGoMihomoRetirementFinalRemovalGateReport {
    pub runtime_id: String,
    pub component: String,
    pub mutates_runtime: bool,
    pub live_execution_allowed: bool,
    pub go_mihomo_retirement_closeout_complete: bool,
    pub final_removal_gate: RustKernelRuntimeGoMihomoRetirementFinalRemovalGateReport,
    pub final_removal_decision: bool,
    pub go_mihomo_retirement_final_removal_gate_complete: bool,
    pub selected_runtime_kind: KernelRuntimeKind,
    pub rollback_runtime_kind: KernelRuntimeKind,
    pub checks: Vec<KernelLoopbackR4ExpandedOptInLimitedRolloutGateCheck>,
    pub blockers: Vec<String>,
    pub warnings: Vec<String>,
    pub facts: Vec<String>,
    pub next_safe_batch: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RustKernelRuntimeGoMihomoRetirementExecutionReport {
    pub runtime_id: String,
    pub component: String,
    pub rollback_checkpoint_created: bool,
    pub execution_manifest_applied: bool,
    pub source_removal_recorded: bool,
    pub artifact_removal_recorded: bool,
    pub post_execution_validation_passed: bool,
    pub execution_complete: bool,
    pub executed_surfaces: Vec<String>,
    pub blockers: Vec<String>,
    pub facts: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KernelLoopbackGoMihomoRetirementExecutionReport {
    pub runtime_id: String,
    pub component: String,
    pub mutates_runtime: bool,
    pub live_execution_allowed: bool,
    pub go_mihomo_retirement_final_removal_gate_complete: bool,
    pub execution: RustKernelRuntimeGoMihomoRetirementExecutionReport,
    pub final_execution_decision: bool,
    pub go_mihomo_retirement_execution_complete: bool,
    pub selected_runtime_kind: KernelRuntimeKind,
    pub rollback_runtime_kind: KernelRuntimeKind,
    pub checks: Vec<KernelLoopbackR4ExpandedOptInLimitedRolloutGateCheck>,
    pub blockers: Vec<String>,
    pub warnings: Vec<String>,
    pub facts: Vec<String>,
    pub next_safe_batch: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RustKernelRuntimeGoMihomoRetirementPostExecutionVerificationReport {
    pub runtime_id: String,
    pub component: String,
    pub rust_only_boundary_verified: bool,
    pub rollback_checkpoint_retained: bool,
    pub source_removal_verified: bool,
    pub artifact_removal_verified: bool,
    pub fallback_ipc_absence_verified: bool,
    pub post_execution_verification_complete: bool,
    pub verified_surfaces: Vec<String>,
    pub blockers: Vec<String>,
    pub facts: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KernelLoopbackGoMihomoRetirementPostExecutionVerificationReport {
    pub runtime_id: String,
    pub component: String,
    pub mutates_runtime: bool,
    pub live_execution_allowed: bool,
    pub go_mihomo_retirement_execution_complete: bool,
    pub post_execution_verification: RustKernelRuntimeGoMihomoRetirementPostExecutionVerificationReport,
    pub final_verification_decision: bool,
    pub go_mihomo_retirement_post_execution_verification_complete: bool,
    pub selected_runtime_kind: KernelRuntimeKind,
    pub rollback_runtime_kind: KernelRuntimeKind,
    pub checks: Vec<KernelLoopbackR4ExpandedOptInLimitedRolloutGateCheck>,
    pub blockers: Vec<String>,
    pub warnings: Vec<String>,
    pub facts: Vec<String>,
    pub next_safe_batch: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RustKernelRuntimeGoMihomoRetirementRollbackSurfaceRetirementReport {
    pub runtime_id: String,
    pub component: String,
    pub post_execution_verification_reviewed: bool,
    pub replacement_recovery_path_verified: bool,
    pub rollback_surface_inventory_locked: bool,
    pub rollback_surface_retirement_plan_archived: bool,
    pub emergency_recovery_drill_passed: bool,
    pub rollback_surface_retirement_complete: bool,
    pub planned_retirement_surfaces: Vec<String>,
    pub blockers: Vec<String>,
    pub facts: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KernelLoopbackGoMihomoRetirementRollbackSurfaceRetirementReport {
    pub runtime_id: String,
    pub component: String,
    pub mutates_runtime: bool,
    pub live_execution_allowed: bool,
    pub go_mihomo_retirement_post_execution_verification_complete: bool,
    pub rollback_surface_retirement: RustKernelRuntimeGoMihomoRetirementRollbackSurfaceRetirementReport,
    pub final_rollback_surface_retirement_decision: bool,
    pub go_mihomo_retirement_rollback_surface_retirement_complete: bool,
    pub selected_runtime_kind: KernelRuntimeKind,
    pub rollback_runtime_kind: KernelRuntimeKind,
    pub checks: Vec<KernelLoopbackR4ExpandedOptInLimitedRolloutGateCheck>,
    pub blockers: Vec<String>,
    pub warnings: Vec<String>,
    pub facts: Vec<String>,
    pub next_safe_batch: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RustKernelRuntimeGoMihomoRetirementCompletionCloseoutReport {
    pub runtime_id: String,
    pub component: String,
    pub rollback_surface_retirement_reviewed: bool,
    pub recovery_boundary_evidence_retained: bool,
    pub completion_report_archived: bool,
    pub release_notes_updated: bool,
    pub migration_state_frozen: bool,
    pub completion_closeout_complete: bool,
    pub closeout_surfaces: Vec<String>,
    pub blockers: Vec<String>,
    pub facts: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KernelLoopbackGoMihomoRetirementCompletionCloseoutReport {
    pub runtime_id: String,
    pub component: String,
    pub mutates_runtime: bool,
    pub live_execution_allowed: bool,
    pub go_mihomo_retirement_rollback_surface_retirement_complete: bool,
    pub completion_closeout: RustKernelRuntimeGoMihomoRetirementCompletionCloseoutReport,
    pub final_completion_decision: bool,
    pub go_mihomo_retirement_completion_closeout_complete: bool,
    pub selected_runtime_kind: KernelRuntimeKind,
    pub rollback_runtime_kind: KernelRuntimeKind,
    pub checks: Vec<KernelLoopbackR4ExpandedOptInLimitedRolloutGateCheck>,
    pub blockers: Vec<String>,
    pub warnings: Vec<String>,
    pub facts: Vec<String>,
    pub next_safe_batch: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RustKernelRuntimeDataPlaneHardeningBoundaryReport {
    pub runtime_id: String,
    pub component: String,
    pub protocol_parity_inventory_complete: bool,
    pub tun_boundary_inventory_complete: bool,
    pub adapter_compatibility_matrix_complete: bool,
    pub dns_leak_verification_plan_complete: bool,
    pub rollback_drill_plan_complete: bool,
    pub opt_in_execution_boundary_locked: bool,
    pub preflight_boundary_complete: bool,
    pub evidence_surfaces: Vec<String>,
    pub blockers: Vec<String>,
    pub facts: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KernelLoopbackRustDataPlaneHardeningPreflightReport {
    pub runtime_id: String,
    pub component: String,
    pub mutates_runtime: bool,
    pub live_execution_allowed: bool,
    pub production_data_plane_mutation_allowed: bool,
    pub go_mihomo_retirement_complete: bool,
    pub boundary: RustKernelRuntimeDataPlaneHardeningBoundaryReport,
    pub final_preflight_decision: bool,
    pub rust_data_plane_hardening_preflight_complete: bool,
    pub selected_runtime_kind: KernelRuntimeKind,
    pub rollback_runtime_kind: KernelRuntimeKind,
    pub checks: Vec<KernelLoopbackR4ExpandedOptInLimitedRolloutGateCheck>,
    pub blockers: Vec<String>,
    pub warnings: Vec<String>,
    pub facts: Vec<String>,
    pub next_safe_batch: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RustKernelRuntimeDataPlaneHardeningBoundaryAuditReport {
    pub runtime_id: String,
    pub component: String,
    pub preflight_reviewed: bool,
    pub protocol_boundary_audited: bool,
    pub tun_boundary_audited: bool,
    pub adapter_boundary_audited: bool,
    pub dns_leak_boundary_audited: bool,
    pub rollback_boundary_audited: bool,
    pub opt_in_boundary_audited: bool,
    pub boundary_audit_complete: bool,
    pub audited_surfaces: Vec<String>,
    pub blockers: Vec<String>,
    pub facts: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KernelLoopbackRustDataPlaneHardeningBoundaryAuditReport {
    pub runtime_id: String,
    pub component: String,
    pub mutates_runtime: bool,
    pub live_execution_allowed: bool,
    pub production_data_plane_mutation_allowed: bool,
    pub rust_data_plane_hardening_preflight_complete: bool,
    pub boundary_audit: RustKernelRuntimeDataPlaneHardeningBoundaryAuditReport,
    pub final_boundary_audit_decision: bool,
    pub rust_data_plane_hardening_boundary_audit_complete: bool,
    pub selected_runtime_kind: KernelRuntimeKind,
    pub rollback_runtime_kind: KernelRuntimeKind,
    pub checks: Vec<KernelLoopbackR4ExpandedOptInLimitedRolloutGateCheck>,
    pub blockers: Vec<String>,
    pub warnings: Vec<String>,
    pub facts: Vec<String>,
    pub next_safe_batch: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RustKernelRuntimeDataPlaneHardeningOptInExecutionGuardReport {
    pub runtime_id: String,
    pub component: String,
    pub boundary_audit_reviewed: bool,
    pub opt_in_scope_locked: bool,
    pub rollout_guard_defined: bool,
    pub abort_plan_approved: bool,
    pub telemetry_watch_configured: bool,
    pub rollback_switch_verified: bool,
    pub operator_acknowledged: bool,
    pub opt_in_execution_guard_complete: bool,
    pub guarded_surfaces: Vec<String>,
    pub blockers: Vec<String>,
    pub facts: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KernelLoopbackRustDataPlaneHardeningOptInExecutionGuardReport {
    pub runtime_id: String,
    pub component: String,
    pub mutates_runtime: bool,
    pub live_execution_allowed: bool,
    pub production_data_plane_mutation_allowed: bool,
    pub rust_data_plane_hardening_boundary_audit_complete: bool,
    pub opt_in_execution_guard: RustKernelRuntimeDataPlaneHardeningOptInExecutionGuardReport,
    pub final_execution_guard_decision: bool,
    pub rust_data_plane_hardening_opt_in_execution_guard_complete: bool,
    pub selected_runtime_kind: KernelRuntimeKind,
    pub rollback_runtime_kind: KernelRuntimeKind,
    pub checks: Vec<KernelLoopbackR4ExpandedOptInLimitedRolloutGateCheck>,
    pub blockers: Vec<String>,
    pub warnings: Vec<String>,
    pub facts: Vec<String>,
    pub next_safe_batch: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RustKernelRuntimeDataPlaneHardeningOptInDryRunReport {
    pub runtime_id: String,
    pub component: String,
    pub execution_guard_reviewed: bool,
    pub dry_run_scope_locked: bool,
    pub manifest_replay_completed: bool,
    pub synthetic_flow_plan_completed: bool,
    pub leak_watch_plan_verified: bool,
    pub rollback_rehearsal_completed: bool,
    pub production_forwarding_unchanged_verified: bool,
    pub dry_run_evidence_archived: bool,
    pub opt_in_dry_run_complete: bool,
    pub dry_run_surfaces: Vec<String>,
    pub blockers: Vec<String>,
    pub facts: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KernelLoopbackRustDataPlaneHardeningOptInDryRunReport {
    pub runtime_id: String,
    pub component: String,
    pub mutates_runtime: bool,
    pub live_execution_allowed: bool,
    pub production_data_plane_mutation_allowed: bool,
    pub rust_data_plane_hardening_opt_in_execution_guard_complete: bool,
    pub opt_in_dry_run: RustKernelRuntimeDataPlaneHardeningOptInDryRunReport,
    pub final_dry_run_decision: bool,
    pub rust_data_plane_hardening_opt_in_dry_run_complete: bool,
    pub selected_runtime_kind: KernelRuntimeKind,
    pub rollback_runtime_kind: KernelRuntimeKind,
    pub checks: Vec<KernelLoopbackR4ExpandedOptInLimitedRolloutGateCheck>,
    pub blockers: Vec<String>,
    pub warnings: Vec<String>,
    pub facts: Vec<String>,
    pub next_safe_batch: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RustKernelRuntimeDataPlaneHardeningOptInExecutionReport {
    pub runtime_id: String,
    pub component: String,
    pub dry_run_reviewed: bool,
    pub execution_manifest_locked: bool,
    pub staged_opt_in_window_defined: bool,
    pub telemetry_watch_active: bool,
    pub rollback_switch_armed: bool,
    pub production_mutation_guard_retained: bool,
    pub operator_execution_acknowledged: bool,
    pub opt_in_execution_complete: bool,
    pub execution_surfaces: Vec<String>,
    pub blockers: Vec<String>,
    pub facts: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KernelLoopbackRustDataPlaneHardeningOptInExecutionReport {
    pub runtime_id: String,
    pub component: String,
    pub mutates_runtime: bool,
    pub live_execution_allowed: bool,
    pub production_data_plane_mutation_allowed: bool,
    pub rust_data_plane_hardening_opt_in_dry_run_complete: bool,
    pub opt_in_execution: RustKernelRuntimeDataPlaneHardeningOptInExecutionReport,
    pub final_execution_decision: bool,
    pub rust_data_plane_hardening_opt_in_execution_complete: bool,
    pub selected_runtime_kind: KernelRuntimeKind,
    pub rollback_runtime_kind: KernelRuntimeKind,
    pub checks: Vec<KernelLoopbackR4ExpandedOptInLimitedRolloutGateCheck>,
    pub blockers: Vec<String>,
    pub warnings: Vec<String>,
    pub facts: Vec<String>,
    pub next_safe_batch: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RustKernelRuntimeDataPlaneHardeningOptInExecutionVerificationReport {
    pub runtime_id: String,
    pub component: String,
    pub execution_record_reviewed: bool,
    pub telemetry_sample_reviewed: bool,
    pub rollback_readiness_verified: bool,
    pub production_mutation_guard_still_retained: bool,
    pub production_forwarding_unchanged_verified: bool,
    pub leak_regression_absence_verified: bool,
    pub verification_evidence_archived: bool,
    pub opt_in_execution_verification_complete: bool,
    pub verification_surfaces: Vec<String>,
    pub blockers: Vec<String>,
    pub facts: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KernelLoopbackRustDataPlaneHardeningOptInExecutionVerificationReport {
    pub runtime_id: String,
    pub component: String,
    pub mutates_runtime: bool,
    pub live_execution_allowed: bool,
    pub production_data_plane_mutation_allowed: bool,
    pub rust_data_plane_hardening_opt_in_execution_complete: bool,
    pub opt_in_execution_verification: RustKernelRuntimeDataPlaneHardeningOptInExecutionVerificationReport,
    pub final_verification_decision: bool,
    pub rust_data_plane_hardening_opt_in_execution_verification_complete: bool,
    pub selected_runtime_kind: KernelRuntimeKind,
    pub rollback_runtime_kind: KernelRuntimeKind,
    pub checks: Vec<KernelLoopbackR4ExpandedOptInLimitedRolloutGateCheck>,
    pub blockers: Vec<String>,
    pub warnings: Vec<String>,
    pub facts: Vec<String>,
    pub next_safe_batch: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RustKernelRuntimeDataPlaneHardeningControlledRolloutGuardReport {
    pub runtime_id: String,
    pub component: String,
    pub opt_in_verification_reviewed: bool,
    pub controlled_rollout_scope_locked: bool,
    pub canary_population_cap_defined: bool,
    pub health_rollback_triggers_defined: bool,
    pub telemetry_hold_window_configured: bool,
    pub mihomo_fallback_retained: bool,
    pub production_mutation_guard_retained: bool,
    pub operator_rollout_guard_acknowledged: bool,
    pub controlled_rollout_guard_complete: bool,
    pub guarded_surfaces: Vec<String>,
    pub blockers: Vec<String>,
    pub facts: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KernelLoopbackRustDataPlaneHardeningControlledRolloutGuardReport {
    pub runtime_id: String,
    pub component: String,
    pub mutates_runtime: bool,
    pub live_execution_allowed: bool,
    pub production_data_plane_mutation_allowed: bool,
    pub rust_data_plane_hardening_opt_in_execution_verification_complete: bool,
    pub controlled_rollout_guard: RustKernelRuntimeDataPlaneHardeningControlledRolloutGuardReport,
    pub final_controlled_rollout_guard_decision: bool,
    pub rust_data_plane_hardening_controlled_rollout_guard_complete: bool,
    pub selected_runtime_kind: KernelRuntimeKind,
    pub rollback_runtime_kind: KernelRuntimeKind,
    pub checks: Vec<KernelLoopbackR4ExpandedOptInLimitedRolloutGateCheck>,
    pub blockers: Vec<String>,
    pub warnings: Vec<String>,
    pub facts: Vec<String>,
    pub next_safe_batch: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RustKernelRuntimeDataPlaneHardeningControlledRolloutDryRunReport {
    pub runtime_id: String,
    pub component: String,
    pub guard_reviewed: bool,
    pub dry_run_manifest_replayed: bool,
    pub capped_canary_simulation_completed: bool,
    pub fallback_trigger_rehearsed: bool,
    pub telemetry_hold_sample_reviewed: bool,
    pub rollback_switch_rehearsed: bool,
    pub production_forwarding_unchanged_verified: bool,
    pub dry_run_evidence_archived: bool,
    pub controlled_rollout_dry_run_complete: bool,
    pub dry_run_surfaces: Vec<String>,
    pub blockers: Vec<String>,
    pub facts: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KernelLoopbackRustDataPlaneHardeningControlledRolloutDryRunReport {
    pub runtime_id: String,
    pub component: String,
    pub mutates_runtime: bool,
    pub live_execution_allowed: bool,
    pub production_data_plane_mutation_allowed: bool,
    pub rust_data_plane_hardening_controlled_rollout_guard_complete: bool,
    pub controlled_rollout_dry_run: RustKernelRuntimeDataPlaneHardeningControlledRolloutDryRunReport,
    pub final_controlled_rollout_dry_run_decision: bool,
    pub rust_data_plane_hardening_controlled_rollout_dry_run_complete: bool,
    pub selected_runtime_kind: KernelRuntimeKind,
    pub rollback_runtime_kind: KernelRuntimeKind,
    pub checks: Vec<KernelLoopbackR4ExpandedOptInLimitedRolloutGateCheck>,
    pub blockers: Vec<String>,
    pub warnings: Vec<String>,
    pub facts: Vec<String>,
    pub next_safe_batch: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RustKernelRuntimeDataPlaneHardeningControlledRolloutReadinessCloseoutReport {
    pub runtime_id: String,
    pub component: String,
    pub dry_run_reviewed: bool,
    pub rollout_window_approved: bool,
    pub canary_population_cap_enforced: bool,
    pub automatic_fallback_armed: bool,
    pub telemetry_watch_active: bool,
    pub rollback_owner_acknowledged: bool,
    pub production_mutation_guard_retained: bool,
    pub closeout_evidence_archived: bool,
    pub controlled_rollout_readiness_closeout_complete: bool,
    pub closeout_surfaces: Vec<String>,
    pub blockers: Vec<String>,
    pub facts: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KernelLoopbackRustDataPlaneHardeningControlledRolloutReadinessCloseoutReport {
    pub runtime_id: String,
    pub component: String,
    pub mutates_runtime: bool,
    pub live_execution_allowed: bool,
    pub production_data_plane_mutation_allowed: bool,
    pub rust_data_plane_hardening_controlled_rollout_dry_run_complete: bool,
    pub controlled_rollout_readiness_closeout:
        RustKernelRuntimeDataPlaneHardeningControlledRolloutReadinessCloseoutReport,
    pub final_controlled_rollout_readiness_decision: bool,
    pub rust_data_plane_hardening_controlled_rollout_readiness_closeout_complete: bool,
    pub selected_runtime_kind: KernelRuntimeKind,
    pub rollback_runtime_kind: KernelRuntimeKind,
    pub checks: Vec<KernelLoopbackR4ExpandedOptInLimitedRolloutGateCheck>,
    pub blockers: Vec<String>,
    pub warnings: Vec<String>,
    pub facts: Vec<String>,
    pub next_safe_batch: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RustKernelRuntimeDataPlaneHardeningControlledRolloutCanaryExecutionReport {
    pub runtime_id: String,
    pub component: String,
    pub readiness_closeout_reviewed: bool,
    pub execution_manifest_locked: bool,
    pub canary_window_started: bool,
    pub canary_population_cap_enforced: bool,
    pub health_telemetry_active: bool,
    pub automatic_fallback_armed: bool,
    pub mihomo_fallback_retained: bool,
    pub production_mutation_guard_retained: bool,
    pub operator_canary_execution_acknowledged: bool,
    pub controlled_rollout_canary_execution_complete: bool,
    pub execution_surfaces: Vec<String>,
    pub blockers: Vec<String>,
    pub facts: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KernelLoopbackRustDataPlaneHardeningControlledRolloutCanaryExecutionReport {
    pub runtime_id: String,
    pub component: String,
    pub mutates_runtime: bool,
    pub live_execution_allowed: bool,
    pub production_data_plane_mutation_allowed: bool,
    pub rust_data_plane_hardening_controlled_rollout_readiness_closeout_complete: bool,
    pub controlled_rollout_canary_execution: RustKernelRuntimeDataPlaneHardeningControlledRolloutCanaryExecutionReport,
    pub final_controlled_rollout_canary_execution_decision: bool,
    pub rust_data_plane_hardening_controlled_rollout_canary_execution_complete: bool,
    pub selected_runtime_kind: KernelRuntimeKind,
    pub rollback_runtime_kind: KernelRuntimeKind,
    pub checks: Vec<KernelLoopbackR4ExpandedOptInLimitedRolloutGateCheck>,
    pub blockers: Vec<String>,
    pub warnings: Vec<String>,
    pub facts: Vec<String>,
    pub next_safe_batch: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RustKernelRuntimeDataPlaneHardeningControlledRolloutCanaryVerificationReport {
    pub runtime_id: String,
    pub component: String,
    pub execution_record_reviewed: bool,
    pub health_telemetry_sample_reviewed: bool,
    pub automatic_fallback_result_reviewed: bool,
    pub unsupported_traffic_fallback_verified: bool,
    pub leak_regression_absence_verified: bool,
    pub rollback_readiness_verified: bool,
    pub production_mutation_guard_still_retained: bool,
    pub verification_evidence_archived: bool,
    pub controlled_rollout_canary_verification_complete: bool,
    pub verification_surfaces: Vec<String>,
    pub blockers: Vec<String>,
    pub facts: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KernelLoopbackRustDataPlaneHardeningControlledRolloutCanaryVerificationReport {
    pub runtime_id: String,
    pub component: String,
    pub mutates_runtime: bool,
    pub live_execution_allowed: bool,
    pub production_data_plane_mutation_allowed: bool,
    pub rust_data_plane_hardening_controlled_rollout_canary_execution_complete: bool,
    pub controlled_rollout_canary_verification:
        RustKernelRuntimeDataPlaneHardeningControlledRolloutCanaryVerificationReport,
    pub final_controlled_rollout_canary_verification_decision: bool,
    pub rust_data_plane_hardening_controlled_rollout_canary_verification_complete: bool,
    pub selected_runtime_kind: KernelRuntimeKind,
    pub rollback_runtime_kind: KernelRuntimeKind,
    pub checks: Vec<KernelLoopbackR4ExpandedOptInLimitedRolloutGateCheck>,
    pub blockers: Vec<String>,
    pub warnings: Vec<String>,
    pub facts: Vec<String>,
    pub next_safe_batch: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RustKernelRuntimeDataPlaneHardeningSupportedDefaultPromotionGuardReport {
    pub runtime_id: String,
    pub component: String,
    pub canary_verification_reviewed: bool,
    pub supported_profile_scope_locked: bool,
    pub fallback_matrix_retained: bool,
    pub rollback_switch_verified: bool,
    pub telemetry_soak_window_defined: bool,
    pub release_blocker_reviewed: bool,
    pub production_mutation_guard_retained: bool,
    pub operator_promotion_acknowledged: bool,
    pub supported_default_promotion_guard_complete: bool,
    pub guard_surfaces: Vec<String>,
    pub blockers: Vec<String>,
    pub facts: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KernelLoopbackRustDataPlaneHardeningSupportedDefaultPromotionGuardReport {
    pub runtime_id: String,
    pub component: String,
    pub mutates_runtime: bool,
    pub live_execution_allowed: bool,
    pub production_data_plane_mutation_allowed: bool,
    pub rust_data_plane_hardening_controlled_rollout_canary_verification_complete: bool,
    pub supported_default_promotion_guard: RustKernelRuntimeDataPlaneHardeningSupportedDefaultPromotionGuardReport,
    pub final_supported_default_promotion_guard_decision: bool,
    pub rust_data_plane_hardening_supported_default_promotion_guard_complete: bool,
    pub selected_runtime_kind: KernelRuntimeKind,
    pub rollback_runtime_kind: KernelRuntimeKind,
    pub checks: Vec<KernelLoopbackR4ExpandedOptInLimitedRolloutGateCheck>,
    pub blockers: Vec<String>,
    pub warnings: Vec<String>,
    pub facts: Vec<String>,
    pub next_safe_batch: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RustKernelRuntimeDataPlaneHardeningSupportedDefaultPromotionDryRunReport {
    pub runtime_id: String,
    pub component: String,
    pub guard_reviewed: bool,
    pub default_selection_manifest_replayed: bool,
    pub supported_profile_simulation_completed: bool,
    pub fallback_decision_rehearsed: bool,
    pub rollback_rehearsed: bool,
    pub production_forwarding_unchanged_verified: bool,
    pub dry_run_evidence_archived: bool,
    pub supported_default_promotion_dry_run_complete: bool,
    pub dry_run_surfaces: Vec<String>,
    pub blockers: Vec<String>,
    pub facts: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KernelLoopbackRustDataPlaneHardeningSupportedDefaultPromotionDryRunReport {
    pub runtime_id: String,
    pub component: String,
    pub mutates_runtime: bool,
    pub live_execution_allowed: bool,
    pub production_data_plane_mutation_allowed: bool,
    pub rust_data_plane_hardening_supported_default_promotion_guard_complete: bool,
    pub supported_default_promotion_dry_run: RustKernelRuntimeDataPlaneHardeningSupportedDefaultPromotionDryRunReport,
    pub final_supported_default_promotion_dry_run_decision: bool,
    pub rust_data_plane_hardening_supported_default_promotion_dry_run_complete: bool,
    pub selected_runtime_kind: KernelRuntimeKind,
    pub rollback_runtime_kind: KernelRuntimeKind,
    pub checks: Vec<KernelLoopbackR4ExpandedOptInLimitedRolloutGateCheck>,
    pub blockers: Vec<String>,
    pub warnings: Vec<String>,
    pub facts: Vec<String>,
    pub next_safe_batch: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RustKernelRuntimeDataPlaneHardeningSupportedDefaultCutoverReport {
    pub runtime_id: String,
    pub component: String,
    pub dry_run_reviewed: bool,
    pub cutover_manifest_locked: bool,
    pub supported_profile_default_selection_confirmed: bool,
    pub unsupported_paths_bound_to_mihomo_fallback: bool,
    pub rollback_switch_armed: bool,
    pub telemetry_soak_watch_active: bool,
    pub operator_cutover_acknowledged: bool,
    pub production_mutation_guard_transition_recorded: bool,
    pub supported_default_cutover_complete: bool,
    pub cutover_surfaces: Vec<String>,
    pub blockers: Vec<String>,
    pub facts: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KernelLoopbackRustDataPlaneHardeningSupportedDefaultCutoverReport {
    pub runtime_id: String,
    pub component: String,
    pub mutates_runtime: bool,
    pub live_execution_allowed: bool,
    pub production_data_plane_mutation_allowed: bool,
    pub rust_data_plane_hardening_supported_default_promotion_dry_run_complete: bool,
    pub supported_default_cutover: RustKernelRuntimeDataPlaneHardeningSupportedDefaultCutoverReport,
    pub final_supported_default_cutover_decision: bool,
    pub rust_data_plane_hardening_supported_default_cutover_complete: bool,
    pub selected_runtime_kind: KernelRuntimeKind,
    pub rollback_runtime_kind: KernelRuntimeKind,
    pub checks: Vec<KernelLoopbackR4ExpandedOptInLimitedRolloutGateCheck>,
    pub blockers: Vec<String>,
    pub warnings: Vec<String>,
    pub facts: Vec<String>,
    pub next_safe_batch: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RustKernelRuntimeDataPlaneHardeningSupportedDefaultCutoverVerificationReport {
    pub runtime_id: String,
    pub component: String,
    pub cutover_record_reviewed: bool,
    pub supported_profile_traffic_sample_reviewed: bool,
    pub unsupported_path_fallback_verified: bool,
    pub rollback_switch_verified: bool,
    pub telemetry_soak_sample_reviewed: bool,
    pub leak_regression_absence_verified: bool,
    pub mutation_audit_record_archived: bool,
    pub cutover_verification_complete: bool,
    pub verification_surfaces: Vec<String>,
    pub blockers: Vec<String>,
    pub facts: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KernelLoopbackRustDataPlaneHardeningSupportedDefaultCutoverVerificationReport {
    pub runtime_id: String,
    pub component: String,
    pub mutates_runtime: bool,
    pub live_execution_allowed: bool,
    pub production_data_plane_mutation_allowed: bool,
    pub rust_data_plane_hardening_supported_default_cutover_complete: bool,
    pub supported_default_cutover_verification:
        RustKernelRuntimeDataPlaneHardeningSupportedDefaultCutoverVerificationReport,
    pub final_supported_default_cutover_verification_decision: bool,
    pub rust_data_plane_hardening_supported_default_cutover_verification_complete: bool,
    pub selected_runtime_kind: KernelRuntimeKind,
    pub rollback_runtime_kind: KernelRuntimeKind,
    pub checks: Vec<KernelLoopbackR4ExpandedOptInLimitedRolloutGateCheck>,
    pub blockers: Vec<String>,
    pub warnings: Vec<String>,
    pub facts: Vec<String>,
    pub next_safe_batch: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RustKernelRuntimeDataPlaneHardeningSupportedDefaultCutoverHoldWindowReport {
    pub runtime_id: String,
    pub component: String,
    pub verification_reviewed: bool,
    pub soak_window_elapsed: bool,
    pub health_budget_satisfied: bool,
    pub fallback_incidents_reviewed: bool,
    pub rollback_switch_still_armed: bool,
    pub mihomo_fallback_still_retained: bool,
    pub hold_window_evidence_archived: bool,
    pub cutover_hold_window_complete: bool,
    pub hold_surfaces: Vec<String>,
    pub blockers: Vec<String>,
    pub facts: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KernelLoopbackRustDataPlaneHardeningSupportedDefaultCutoverHoldWindowReport {
    pub runtime_id: String,
    pub component: String,
    pub mutates_runtime: bool,
    pub live_execution_allowed: bool,
    pub production_data_plane_mutation_allowed: bool,
    pub rust_data_plane_hardening_supported_default_cutover_verification_complete: bool,
    pub supported_default_cutover_hold_window:
        RustKernelRuntimeDataPlaneHardeningSupportedDefaultCutoverHoldWindowReport,
    pub final_supported_default_cutover_hold_window_decision: bool,
    pub rust_data_plane_hardening_supported_default_cutover_hold_window_complete: bool,
    pub selected_runtime_kind: KernelRuntimeKind,
    pub rollback_runtime_kind: KernelRuntimeKind,
    pub checks: Vec<KernelLoopbackR4ExpandedOptInLimitedRolloutGateCheck>,
    pub blockers: Vec<String>,
    pub warnings: Vec<String>,
    pub facts: Vec<String>,
    pub next_safe_batch: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RustKernelRuntimeDataPlaneHardeningSupportedDefaultCutoverCloseoutReport {
    pub runtime_id: String,
    pub component: String,
    pub hold_window_reviewed: bool,
    pub supported_default_state_documented: bool,
    pub rollback_owner_acknowledged: bool,
    pub fallback_retirement_boundary_retained: bool,
    pub release_notes_updated: bool,
    pub closeout_evidence_archived: bool,
    pub supported_default_cutover_closeout_complete: bool,
    pub closeout_surfaces: Vec<String>,
    pub blockers: Vec<String>,
    pub facts: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KernelLoopbackRustDataPlaneHardeningSupportedDefaultCutoverCloseoutReport {
    pub runtime_id: String,
    pub component: String,
    pub mutates_runtime: bool,
    pub live_execution_allowed: bool,
    pub production_data_plane_mutation_allowed: bool,
    pub rust_data_plane_hardening_supported_default_cutover_hold_window_complete: bool,
    pub supported_default_cutover_closeout: RustKernelRuntimeDataPlaneHardeningSupportedDefaultCutoverCloseoutReport,
    pub final_supported_default_cutover_closeout_decision: bool,
    pub rust_data_plane_hardening_supported_default_cutover_closeout_complete: bool,
    pub selected_runtime_kind: KernelRuntimeKind,
    pub rollback_runtime_kind: KernelRuntimeKind,
    pub checks: Vec<KernelLoopbackR4ExpandedOptInLimitedRolloutGateCheck>,
    pub blockers: Vec<String>,
    pub warnings: Vec<String>,
    pub facts: Vec<String>,
    pub next_safe_batch: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RustKernelRuntimeDataPlaneHardeningExpandedDefaultRolloutGuardReport {
    pub runtime_id: String,
    pub component: String,
    pub cutover_closeout_reviewed: bool,
    pub expanded_scope_locked: bool,
    pub rollout_cap_defined: bool,
    pub fallback_matrix_retained: bool,
    pub rollback_switch_verified: bool,
    pub telemetry_soak_plan_defined: bool,
    pub unsupported_path_boundary_retained: bool,
    pub operator_rollout_acknowledged: bool,
    pub expanded_default_rollout_guard_complete: bool,
    pub guard_surfaces: Vec<String>,
    pub blockers: Vec<String>,
    pub facts: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KernelLoopbackRustDataPlaneHardeningExpandedDefaultRolloutGuardReport {
    pub runtime_id: String,
    pub component: String,
    pub mutates_runtime: bool,
    pub live_execution_allowed: bool,
    pub production_data_plane_mutation_allowed: bool,
    pub rust_data_plane_hardening_supported_default_cutover_closeout_complete: bool,
    pub expanded_default_rollout_guard: RustKernelRuntimeDataPlaneHardeningExpandedDefaultRolloutGuardReport,
    pub final_expanded_default_rollout_guard_decision: bool,
    pub rust_data_plane_hardening_expanded_default_rollout_guard_complete: bool,
    pub selected_runtime_kind: KernelRuntimeKind,
    pub rollback_runtime_kind: KernelRuntimeKind,
    pub checks: Vec<KernelLoopbackR4ExpandedOptInLimitedRolloutGateCheck>,
    pub blockers: Vec<String>,
    pub warnings: Vec<String>,
    pub facts: Vec<String>,
    pub next_safe_batch: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RustKernelRuntimeDataPlaneHardeningExpandedDefaultRolloutDryRunReport {
    pub runtime_id: String,
    pub component: String,
    pub guard_reviewed: bool,
    pub expanded_manifest_replayed: bool,
    pub representative_profile_simulation_completed: bool,
    pub fallback_routing_rehearsed: bool,
    pub rollback_rehearsed: bool,
    pub telemetry_soak_sample_reviewed: bool,
    pub dry_run_evidence_archived: bool,
    pub expanded_default_rollout_dry_run_complete: bool,
    pub dry_run_surfaces: Vec<String>,
    pub blockers: Vec<String>,
    pub facts: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KernelLoopbackRustDataPlaneHardeningExpandedDefaultRolloutDryRunReport {
    pub runtime_id: String,
    pub component: String,
    pub mutates_runtime: bool,
    pub live_execution_allowed: bool,
    pub production_data_plane_mutation_allowed: bool,
    pub rust_data_plane_hardening_expanded_default_rollout_guard_complete: bool,
    pub expanded_default_rollout_dry_run: RustKernelRuntimeDataPlaneHardeningExpandedDefaultRolloutDryRunReport,
    pub final_expanded_default_rollout_dry_run_decision: bool,
    pub rust_data_plane_hardening_expanded_default_rollout_dry_run_complete: bool,
    pub selected_runtime_kind: KernelRuntimeKind,
    pub rollback_runtime_kind: KernelRuntimeKind,
    pub checks: Vec<KernelLoopbackR4ExpandedOptInLimitedRolloutGateCheck>,
    pub blockers: Vec<String>,
    pub warnings: Vec<String>,
    pub facts: Vec<String>,
    pub next_safe_batch: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RustKernelRuntimeDataPlaneHardeningExpandedDefaultRolloutExecutionReport {
    pub runtime_id: String,
    pub component: String,
    pub dry_run_reviewed: bool,
    pub execution_manifest_locked: bool,
    pub rollout_window_started: bool,
    pub expanded_profile_cap_enforced: bool,
    pub active_telemetry_watch: bool,
    pub rollback_switch_armed: bool,
    pub mihomo_fallback_retained: bool,
    pub operator_execution_acknowledged: bool,
    pub expanded_default_rollout_execution_complete: bool,
    pub execution_surfaces: Vec<String>,
    pub blockers: Vec<String>,
    pub facts: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KernelLoopbackRustDataPlaneHardeningExpandedDefaultRolloutExecutionReport {
    pub runtime_id: String,
    pub component: String,
    pub mutates_runtime: bool,
    pub live_execution_allowed: bool,
    pub production_data_plane_mutation_allowed: bool,
    pub rust_data_plane_hardening_expanded_default_rollout_dry_run_complete: bool,
    pub expanded_default_rollout_execution: RustKernelRuntimeDataPlaneHardeningExpandedDefaultRolloutExecutionReport,
    pub final_expanded_default_rollout_execution_decision: bool,
    pub rust_data_plane_hardening_expanded_default_rollout_execution_complete: bool,
    pub selected_runtime_kind: KernelRuntimeKind,
    pub rollback_runtime_kind: KernelRuntimeKind,
    pub checks: Vec<KernelLoopbackR4ExpandedOptInLimitedRolloutGateCheck>,
    pub blockers: Vec<String>,
    pub warnings: Vec<String>,
    pub facts: Vec<String>,
    pub next_safe_batch: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RustKernelRuntimeDataPlaneHardeningExpandedDefaultRolloutVerificationReport {
    pub runtime_id: String,
    pub component: String,
    pub execution_record_reviewed: bool,
    pub expanded_profile_traffic_sample_reviewed: bool,
    pub fallback_path_sample_verified: bool,
    pub rollback_switch_verified: bool,
    pub telemetry_health_budget_verified: bool,
    pub leak_regression_absence_verified: bool,
    pub verification_evidence_archived: bool,
    pub expanded_default_rollout_verification_complete: bool,
    pub verification_surfaces: Vec<String>,
    pub blockers: Vec<String>,
    pub facts: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KernelLoopbackRustDataPlaneHardeningExpandedDefaultRolloutVerificationReport {
    pub runtime_id: String,
    pub component: String,
    pub mutates_runtime: bool,
    pub live_execution_allowed: bool,
    pub production_data_plane_mutation_allowed: bool,
    pub rust_data_plane_hardening_expanded_default_rollout_execution_complete: bool,
    pub expanded_default_rollout_verification:
        RustKernelRuntimeDataPlaneHardeningExpandedDefaultRolloutVerificationReport,
    pub final_expanded_default_rollout_verification_decision: bool,
    pub rust_data_plane_hardening_expanded_default_rollout_verification_complete: bool,
    pub selected_runtime_kind: KernelRuntimeKind,
    pub rollback_runtime_kind: KernelRuntimeKind,
    pub checks: Vec<KernelLoopbackR4ExpandedOptInLimitedRolloutGateCheck>,
    pub blockers: Vec<String>,
    pub warnings: Vec<String>,
    pub facts: Vec<String>,
    pub next_safe_batch: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RustKernelRuntimeDataPlaneHardeningExpandedDefaultRolloutCloseoutReport {
    pub runtime_id: String,
    pub component: String,
    pub verification_reviewed: bool,
    pub expanded_rollout_state_documented: bool,
    pub rollback_owner_acknowledged: bool,
    pub fallback_matrix_retained: bool,
    pub unsupported_path_boundary_retained: bool,
    pub release_notes_updated: bool,
    pub closeout_evidence_archived: bool,
    pub expanded_default_rollout_closeout_complete: bool,
    pub closeout_surfaces: Vec<String>,
    pub blockers: Vec<String>,
    pub facts: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KernelLoopbackRustDataPlaneHardeningExpandedDefaultRolloutCloseoutReport {
    pub runtime_id: String,
    pub component: String,
    pub mutates_runtime: bool,
    pub live_execution_allowed: bool,
    pub production_data_plane_mutation_allowed: bool,
    pub rust_data_plane_hardening_expanded_default_rollout_verification_complete: bool,
    pub expanded_default_rollout_closeout: RustKernelRuntimeDataPlaneHardeningExpandedDefaultRolloutCloseoutReport,
    pub final_expanded_default_rollout_closeout_decision: bool,
    pub rust_data_plane_hardening_expanded_default_rollout_closeout_complete: bool,
    pub selected_runtime_kind: KernelRuntimeKind,
    pub rollback_runtime_kind: KernelRuntimeKind,
    pub checks: Vec<KernelLoopbackR4ExpandedOptInLimitedRolloutGateCheck>,
    pub blockers: Vec<String>,
    pub warnings: Vec<String>,
    pub facts: Vec<String>,
    pub next_safe_batch: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RustKernelRuntimeDataPlaneHardeningMihomoFallbackRetirementGuardReport {
    pub runtime_id: String,
    pub component: String,
    pub expanded_rollout_closeout_reviewed: bool,
    pub protocol_parity_scope_locked: bool,
    pub tun_parity_scope_locked: bool,
    pub adapter_parity_scope_locked: bool,
    pub dns_parity_scope_locked: bool,
    pub emergency_rollback_retained: bool,
    pub cross_platform_drill_plan_defined: bool,
    pub operator_retirement_acknowledged: bool,
    pub mihomo_fallback_retirement_guard_complete: bool,
    pub guard_surfaces: Vec<String>,
    pub blockers: Vec<String>,
    pub facts: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KernelLoopbackRustDataPlaneHardeningMihomoFallbackRetirementGuardReport {
    pub runtime_id: String,
    pub component: String,
    pub mutates_runtime: bool,
    pub live_execution_allowed: bool,
    pub production_data_plane_mutation_allowed: bool,
    pub rust_data_plane_hardening_expanded_default_rollout_closeout_complete: bool,
    pub mihomo_fallback_retirement_guard: RustKernelRuntimeDataPlaneHardeningMihomoFallbackRetirementGuardReport,
    pub final_mihomo_fallback_retirement_guard_decision: bool,
    pub rust_data_plane_hardening_mihomo_fallback_retirement_guard_complete: bool,
    pub selected_runtime_kind: KernelRuntimeKind,
    pub rollback_runtime_kind: KernelRuntimeKind,
    pub checks: Vec<KernelLoopbackR4ExpandedOptInLimitedRolloutGateCheck>,
    pub blockers: Vec<String>,
    pub warnings: Vec<String>,
    pub facts: Vec<String>,
    pub next_safe_batch: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RustKernelRuntimeDataPlaneHardeningMihomoFallbackRetirementDryRunReport {
    pub runtime_id: String,
    pub component: String,
    pub guard_reviewed: bool,
    pub parity_manifest_replayed: bool,
    pub cross_platform_rollback_rehearsed: bool,
    pub fallback_dependency_inventory_replayed: bool,
    pub emergency_recovery_rehearsed: bool,
    pub production_forwarding_unchanged_verified: bool,
    pub dry_run_evidence_archived: bool,
    pub mihomo_fallback_retirement_dry_run_complete: bool,
    pub dry_run_surfaces: Vec<String>,
    pub blockers: Vec<String>,
    pub facts: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KernelLoopbackRustDataPlaneHardeningMihomoFallbackRetirementDryRunReport {
    pub runtime_id: String,
    pub component: String,
    pub mutates_runtime: bool,
    pub live_execution_allowed: bool,
    pub production_data_plane_mutation_allowed: bool,
    pub rust_data_plane_hardening_mihomo_fallback_retirement_guard_complete: bool,
    pub mihomo_fallback_retirement_dry_run: RustKernelRuntimeDataPlaneHardeningMihomoFallbackRetirementDryRunReport,
    pub final_mihomo_fallback_retirement_dry_run_decision: bool,
    pub rust_data_plane_hardening_mihomo_fallback_retirement_dry_run_complete: bool,
    pub selected_runtime_kind: KernelRuntimeKind,
    pub rollback_runtime_kind: KernelRuntimeKind,
    pub checks: Vec<KernelLoopbackR4ExpandedOptInLimitedRolloutGateCheck>,
    pub blockers: Vec<String>,
    pub warnings: Vec<String>,
    pub facts: Vec<String>,
    pub next_safe_batch: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RustKernelRuntimeDataPlaneHardeningMihomoFallbackRetirementReadinessReport {
    pub runtime_id: String,
    pub component: String,
    pub dry_run_reviewed: bool,
    pub protocol_parity_evidence_archived: bool,
    pub tun_parity_evidence_archived: bool,
    pub adapter_parity_evidence_archived: bool,
    pub dns_parity_evidence_archived: bool,
    pub soak_evidence_archived: bool,
    pub emergency_rollback_owner_acknowledged: bool,
    pub mihomo_fallback_retirement_readiness_complete: bool,
    pub readiness_surfaces: Vec<String>,
    pub blockers: Vec<String>,
    pub facts: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KernelLoopbackRustDataPlaneHardeningMihomoFallbackRetirementReadinessReport {
    pub runtime_id: String,
    pub component: String,
    pub mutates_runtime: bool,
    pub live_execution_allowed: bool,
    pub production_data_plane_mutation_allowed: bool,
    pub rust_data_plane_hardening_mihomo_fallback_retirement_dry_run_complete: bool,
    pub mihomo_fallback_retirement_readiness:
        RustKernelRuntimeDataPlaneHardeningMihomoFallbackRetirementReadinessReport,
    pub final_mihomo_fallback_retirement_readiness_decision: bool,
    pub rust_data_plane_hardening_mihomo_fallback_retirement_readiness_complete: bool,
    pub selected_runtime_kind: KernelRuntimeKind,
    pub rollback_runtime_kind: KernelRuntimeKind,
    pub checks: Vec<KernelLoopbackR4ExpandedOptInLimitedRolloutGateCheck>,
    pub blockers: Vec<String>,
    pub warnings: Vec<String>,
    pub facts: Vec<String>,
    pub next_safe_batch: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KernelReplacementReadiness {
    pub mutates_runtime: bool,
    pub active_kernel: String,
    pub controller_transport: String,
    pub rust_owned_control_plane: Vec<String>,
    pub mihomo_owned_data_plane: Vec<String>,
    pub blocked_replacement_areas: Vec<KernelReplacementBlocker>,
    pub next_safe_batch: String,
}
