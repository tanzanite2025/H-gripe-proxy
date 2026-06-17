use crate::core::dns_runtime::{
    DnsDefaultRuntimeExpandedControlPlaneCompletionReport, DnsDefaultRuntimeExpandedControlPlaneCompletionStatus,
    DnsResolverPlan,
};
use serde::{Deserialize, Serialize};
use smartstring::alias::String;
use std::collections::BTreeMap;

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AppRuntimeStateDocument {
    #[serde(default)]
    pub apps: Vec<AppRegistryEntry>,
    #[serde(default)]
    pub node_pools: Vec<NodePool>,
    #[serde(default)]
    pub dns_profiles: Vec<DnsProfile>,
    #[serde(default)]
    pub security_profiles: Vec<SecurityProfile>,
    #[serde(default)]
    pub policy_bindings: Vec<AppPolicyBinding>,
    #[serde(default)]
    pub sessions: Vec<AppRuntimeSessionRecord>,
    #[serde(default)]
    pub runtime_apply_audits: Vec<AppRuntimeProjectionRuntimeApplyAuditRecord>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub active_projection: Option<AppRuntimeActiveProjectionRecord>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum AppRuntimeDnsHandoffStatus {
    Accepted,
    Watching,
    RollbackRecommended,
    Blocked,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum AppRuntimeControlPlaneCompletionStatus {
    Ready,
    Degraded,
    Blocked,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum AppRuntimeStagedActivationLifecycleStatus {
    Ready,
    Degraded,
    Blocked,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum AppRuntimeStagedActivationCloseoutStatus {
    Complete,
    Degraded,
    Blocked,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum AppRuntimeRuntimeApplyBoundaryDecision {
    AllowRuntimeCandidate,
    DeferRuntimeApply,
    RecommendRollback,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum AppRuntimeRuntimeApplyBoundaryDecisionStatus {
    Accepted,
    Deferred,
    RollbackRecommended,
    Blocked,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppRuntimeDnsHandoffRecord {
    pub handoff_id: String,
    pub action: String,
    pub dns_completion_status: DnsDefaultRuntimeExpandedControlPlaneCompletionStatus,
    pub dns_control_plane_complete: bool,
    pub dns_handoff_ready: bool,
    pub dns_manifest_path: Option<String>,
    pub app_runtime_accepts_handoff: bool,
    pub app_runtime_followup_scope: String,
    pub next_app_runtime_step: String,
    pub phase8_allowed: bool,
    pub promotion_allowed: bool,
    pub auto_rollout: bool,
    pub auto_rollback: bool,
    pub mutates_runtime: bool,
    pub reload_mihomo: bool,
    pub created_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppRuntimeDnsHandoffReport {
    pub status: AppRuntimeDnsHandoffStatus,
    pub reason: String,
    pub dns_completion: DnsDefaultRuntimeExpandedControlPlaneCompletionReport,
    pub handoff_record: AppRuntimeDnsHandoffRecord,
    pub handoff_record_path: Option<String>,
    pub handoff_record_persisted: bool,
    pub app_runtime_accepts_handoff: bool,
    pub next_app_runtime_step: String,
    pub phase8_allowed: bool,
    pub promotion_allowed: bool,
    pub user_trigger_required: bool,
    pub auto_rollout: bool,
    pub auto_rollback: bool,
    pub mutates_runtime: bool,
    pub reload_mihomo: bool,
    pub blockers: Vec<String>,
    pub warnings: Vec<String>,
    pub facts: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppRuntimeControlPlaneCompletionReport {
    pub status: AppRuntimeControlPlaneCompletionStatus,
    pub reason: String,
    pub app_id: String,
    pub dns_handoff: AppRuntimeDnsHandoffReport,
    pub projection_artifact: AppRuntimeProjectionArtifact,
    pub projection_artifact_path: Option<String>,
    pub projection_artifact_persisted: bool,
    pub activation_preflight: AppRuntimeProjectionActivationPreflightReport,
    pub ready_for_staged_activation: bool,
    pub runtime_apply_allowed: bool,
    pub phase8_allowed: bool,
    pub promotion_allowed: bool,
    pub user_trigger_required: bool,
    pub auto_rollout: bool,
    pub auto_rollback: bool,
    pub mutates_runtime: bool,
    pub reload_mihomo: bool,
    pub next_app_runtime_step: String,
    pub blockers: Vec<String>,
    pub warnings: Vec<String>,
    pub facts: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppRuntimeStagedActivationLifecycleReport {
    pub status: AppRuntimeStagedActivationLifecycleStatus,
    pub reason: String,
    pub app_id: String,
    pub control_plane_completion: AppRuntimeControlPlaneCompletionReport,
    pub active_projection: Option<AppRuntimeActiveProjectionRecord>,
    pub marker_activated: bool,
    pub active_marker_matches_artifact: bool,
    pub rollback_boundary_available: bool,
    pub rollback_strategy: Option<String>,
    pub runtime_apply_allowed: bool,
    pub phase8_allowed: bool,
    pub promotion_allowed: bool,
    pub user_trigger_required: bool,
    pub auto_rollout: bool,
    pub auto_rollback: bool,
    pub mutates_runtime: bool,
    pub reload_mihomo: bool,
    pub next_app_runtime_step: String,
    pub blockers: Vec<String>,
    pub warnings: Vec<String>,
    pub facts: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AppRuntimeRuntimeApplyBoundaryManifest {
    pub manifest_id: String,
    pub app_id: String,
    pub artifact_id: String,
    pub checksum: String,
    pub active_marker_matches_artifact: bool,
    pub rollback_boundary_available: bool,
    pub rollback_strategy: Option<String>,
    pub runtime_apply_allowed: bool,
    pub phase8_allowed: bool,
    pub promotion_allowed: bool,
    pub auto_rollout: bool,
    pub auto_rollback: bool,
    pub mutates_runtime: bool,
    pub reload_mihomo: bool,
    pub next_app_runtime_step: String,
    pub created_at: i64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppRuntimeStagedActivationCloseoutReport {
    pub status: AppRuntimeStagedActivationCloseoutStatus,
    pub reason: String,
    pub lifecycle: AppRuntimeStagedActivationLifecycleReport,
    pub boundary_manifest: AppRuntimeRuntimeApplyBoundaryManifest,
    pub boundary_manifest_path: Option<String>,
    pub boundary_manifest_persisted: bool,
    pub closeout_complete: bool,
    pub runtime_apply_allowed: bool,
    pub phase8_allowed: bool,
    pub promotion_allowed: bool,
    pub user_trigger_required: bool,
    pub auto_rollout: bool,
    pub auto_rollback: bool,
    pub mutates_runtime: bool,
    pub reload_mihomo: bool,
    pub next_app_runtime_step: String,
    pub blockers: Vec<String>,
    pub warnings: Vec<String>,
    pub facts: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AppRuntimeRuntimeApplyBoundaryDecisionRequest {
    pub app_id: String,
    pub decision: AppRuntimeRuntimeApplyBoundaryDecision,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rationale: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AppRuntimeRuntimeApplyBoundaryDecisionRecord {
    pub decision_id: String,
    pub app_id: String,
    pub artifact_id: String,
    pub checksum: String,
    pub boundary_manifest_id: String,
    pub boundary_manifest_path: Option<String>,
    pub decision: AppRuntimeRuntimeApplyBoundaryDecision,
    pub rationale: Option<String>,
    pub decision_accepted: bool,
    pub runtime_apply_candidate_allowed: bool,
    pub rollback_recommended: bool,
    pub runtime_apply_allowed: bool,
    pub phase8_allowed: bool,
    pub promotion_allowed: bool,
    pub auto_rollout: bool,
    pub auto_rollback: bool,
    pub mutates_runtime: bool,
    pub reload_mihomo: bool,
    pub next_app_runtime_step: String,
    pub created_at: i64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppRuntimeRuntimeApplyBoundaryDecisionReport {
    pub status: AppRuntimeRuntimeApplyBoundaryDecisionStatus,
    pub reason: String,
    pub closeout: AppRuntimeStagedActivationCloseoutReport,
    pub decision_record: AppRuntimeRuntimeApplyBoundaryDecisionRecord,
    pub decision_record_path: Option<String>,
    pub decision_record_persisted: bool,
    pub runtime_apply_candidate_allowed: bool,
    pub rollback_recommended: bool,
    pub runtime_apply_allowed: bool,
    pub phase8_allowed: bool,
    pub promotion_allowed: bool,
    pub user_trigger_required: bool,
    pub auto_rollout: bool,
    pub auto_rollback: bool,
    pub mutates_runtime: bool,
    pub reload_mihomo: bool,
    pub next_app_runtime_step: String,
    pub blockers: Vec<String>,
    pub warnings: Vec<String>,
    pub facts: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AppRuntimeActiveProjectionRecord {
    pub artifact_id: String,
    pub app_id: String,
    pub checksum: String,
    pub storage_path: String,
    pub activated_at: i64,
    pub activation_kind: String,
    pub mutates_runtime: bool,
    pub rollback: AppRuntimeProjectionRollbackMetadata,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AppRuntimeProjectionRollbackMetadata {
    pub previous_artifact_id: Option<String>,
    pub previous_checksum: Option<String>,
    pub previous_storage_path: Option<String>,
    pub captured_at: i64,
    pub rollback_strategy: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AppRegistryEntry {
    pub app_id: String,
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub executable_path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bundle_id: Option<String>,
    #[serde(default)]
    pub launch_args: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub working_directory: Option<String>,
    #[serde(default)]
    pub env: Vec<AppEnvironmentVariable>,
    #[serde(default)]
    pub process_matchers: Vec<AppProcessMatcher>,
    #[serde(default)]
    pub platform_metadata: BTreeMap<String, String>,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub updated_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AppEnvironmentVariable {
    pub key: String,
    pub value: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AppProcessMatcherKind {
    ProcessName,
    ProcessPath,
    ProcessNameRegex,
    ProcessPathRegex,
    BundleId,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AppProcessMatcher {
    pub kind: AppProcessMatcherKind,
    pub pattern: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct NodePool {
    pub pool_id: String,
    pub name: String,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub region: Option<String>,
    #[serde(default)]
    pub protocols: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub purpose: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cost_tier: Option<String>,
    #[serde(default)]
    pub health_constraints: NodePoolHealthConstraints,
    #[serde(default)]
    pub candidate_nodes: Vec<NodePoolCandidate>,
    #[serde(default)]
    pub updated_at: i64,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct NodePoolHealthConstraints {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_latency_ms: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub require_alive: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min_available_nodes: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct NodePoolCandidate {
    pub node_name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub proxy_group: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub protocol: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub region: Option<String>,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub priority: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DnsProfile {
    pub profile_id: String,
    pub name: String,
    pub config_yaml: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub test_domain: Option<String>,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub updated_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SecurityProfile {
    pub profile_id: String,
    pub name: String,
    #[serde(default)]
    pub controls: SecurityProfileControls,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub updated_at: i64,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SecurityProfileControls {
    #[serde(default)]
    pub require_node_pool: bool,
    #[serde(default)]
    pub require_dns_profile: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min_runtime_supported_nameservers: Option<usize>,
    #[serde(default)]
    pub allowed_routing_intents: Vec<AppRoutingIntent>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AppRoutingIntent {
    Direct,
    Proxy,
    Reject,
    Auto,
    Fallback,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AppPolicyBinding {
    pub binding_id: String,
    pub app_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub node_pool_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dns_profile_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub security_profile_id: Option<String>,
    pub routing_intent: AppRoutingIntent,
    #[serde(default = "default_enabled")]
    pub enabled: bool,
    #[serde(default)]
    pub updated_at: i64,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AppRuntimePlanRequest {
    pub app_id: String,
    #[serde(default)]
    pub session_id: Option<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum AppRuntimePlanStatus {
    Ready,
    Rejected,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AppRuntimePlan {
    pub status: AppRuntimePlanStatus,
    pub reason: String,
    pub app_id: String,
    pub session_id: Option<String>,
    pub app: Option<AppRegistryEntry>,
    pub policy_binding: Option<AppPolicyBinding>,
    pub node_pool: Option<NodePoolPlanView>,
    pub dns_profile: Option<DnsProfilePlanView>,
    pub security_profile: Option<SecurityProfilePlanView>,
    pub routing_intent: Option<AppRoutingIntent>,
    pub projection: RuntimeProjectionPlan,
    pub facts: Vec<String>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct NodePoolPlanView {
    pub pool_id: String,
    pub name: String,
    pub candidate_count: usize,
    pub protocols: Vec<String>,
    pub tags: Vec<String>,
    pub constraints: NodePoolHealthConstraints,
    pub candidates: Vec<NodePoolCandidate>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DnsProfilePlanView {
    pub profile_id: String,
    pub name: String,
    pub test_domain: Option<String>,
    pub tags: Vec<String>,
    pub resolver_plan: DnsResolverPlan,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SecurityProfilePlanView {
    pub profile_id: String,
    pub name: String,
    pub controls: SecurityProfileControls,
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum RuntimeProjectionStatus {
    PlanningOnly,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeProjectionPlan {
    pub status: RuntimeProjectionStatus,
    pub backend: String,
    pub mutates_runtime: bool,
    pub outputs: Vec<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AppRuntimeMihomoProjection {
    pub status: AppRuntimePlanStatus,
    pub reason: String,
    pub app_id: String,
    pub session_id: Option<String>,
    pub mutates_runtime: bool,
    pub proxy_groups: Vec<MihomoProxyGroupProjection>,
    pub rules: Vec<MihomoRuleProjection>,
    pub dns: Option<MihomoDnsProjection>,
    pub yaml_patch: String,
    pub facts: Vec<String>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum AppRuntimeProjectionActivationMode {
    Staged,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AppRuntimeProjectionArtifact {
    pub artifact_id: String,
    pub app_id: String,
    pub session_id: Option<String>,
    pub binding_id: Option<String>,
    pub node_pool_id: Option<String>,
    pub dns_profile_id: Option<String>,
    pub security_profile_id: Option<String>,
    pub generated_at: i64,
    pub storage_path: Option<String>,
    pub activation_mode: AppRuntimeProjectionActivationMode,
    pub mutates_runtime: bool,
    pub checksum: String,
    pub plan: AppRuntimePlan,
    pub projection: AppRuntimeMihomoProjection,
    pub diagnostics: AppRuntimeDiagnosticsReport,
    pub validation: AppRuntimeProjectionValidationReport,
    pub facts: Vec<String>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AppRuntimeProjectionValidationReport {
    pub status: AppRuntimeDiagnosticStatus,
    pub reason: String,
    pub checks: Vec<AppRuntimeDiagnosticCheck>,
    pub summary: AppRuntimeDiagnosticsSummary,
    pub facts: Vec<String>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AppRuntimeProjectionActivationPreflightRequest {
    pub artifact_id: String,
    pub expected_checksum: Option<String>,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AppRuntimeProjectionRuntimeApplyRequest {
    pub artifact_id: String,
    pub expected_checksum: Option<String>,
    #[serde(default)]
    pub runtime_apply_decision_id: Option<String>,
    #[serde(default)]
    pub expected_runtime_apply_decision_checksum: Option<String>,
    #[serde(default = "default_runtime_apply_force")]
    pub force: bool,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AppRuntimeProjectionRuntimeVerificationRequest {
    pub artifact_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AppRuntimeProjectionRuntimeApplyAuditRecord {
    pub audit_id: String,
    pub artifact_id: String,
    pub app_id: String,
    pub checksum: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub runtime_apply_decision_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub runtime_apply_decision_boundary_manifest_id: Option<String>,
    pub activation_kind: String,
    pub applied_at: i64,
    pub validation_outcome: String,
    pub candidate_summary: AppRuntimeProjectionRuntimeApplyCandidateSummary,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub previous_marker: Option<AppRuntimeProjectionRuntimeApplyMarkerSnapshot>,
    pub rollback_strategy: String,
    pub status: AppRuntimeProjectionRuntimeApplyAuditStatus,
    pub status_updated_at: i64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub latest_verification_status: Option<AppRuntimeDiagnosticStatus>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub latest_verification_reason: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub latest_verification_at: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AppRuntimeProjectionRuntimeApplyCandidateSummary {
    pub profile_item_uid: String,
    pub profile_item_file: String,
    pub proxy_group_count: usize,
    pub rule_count: usize,
    pub dns_profile_projected: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AppRuntimeProjectionRuntimeApplyMarkerSnapshot {
    pub artifact_id: String,
    pub checksum: String,
    pub storage_path: String,
    pub activation_kind: String,
    pub mutates_runtime: bool,
    pub activated_at: i64,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum AppRuntimeProjectionRuntimeApplyAuditStatus {
    Active,
    RolledBack,
    Superseded,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AppRuntimeProjectionRuntimeVerificationReport {
    pub status: AppRuntimeDiagnosticStatus,
    pub reason: String,
    pub artifact_id: Option<String>,
    pub checksum: Option<String>,
    pub audit_id: Option<String>,
    pub runtime_apply_decision_id: Option<String>,
    pub runtime_apply_decision_boundary_manifest_id: Option<String>,
    pub runtime_apply_decision_verified: bool,
    pub observed_at: i64,
    pub checks: Vec<AppRuntimeDiagnosticCheck>,
    pub summary: AppRuntimeDiagnosticsSummary,
    pub facts: Vec<String>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum AppRuntimeProjectionRuntimeVerificationCloseoutStatus {
    Complete,
    Blocked,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AppRuntimeProjectionRuntimeVerificationCloseoutRecord {
    pub closeout_id: String,
    pub artifact_id: Option<String>,
    pub checksum: Option<String>,
    pub audit_id: Option<String>,
    pub runtime_apply_decision_id: Option<String>,
    pub runtime_apply_decision_boundary_manifest_id: Option<String>,
    pub verification_status: AppRuntimeDiagnosticStatus,
    pub verification_reason: String,
    pub runtime_apply_decision_verified: bool,
    pub closeout_complete: bool,
    pub runtime_apply_allowed: bool,
    pub phase8_allowed: bool,
    pub promotion_allowed: bool,
    pub auto_rollout: bool,
    pub auto_rollback: bool,
    pub mutates_runtime: bool,
    pub reload_mihomo: bool,
    pub next_app_runtime_step: String,
    pub created_at: i64,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AppRuntimeProjectionRuntimeVerificationCloseoutReport {
    pub status: AppRuntimeProjectionRuntimeVerificationCloseoutStatus,
    pub reason: String,
    pub verification: AppRuntimeProjectionRuntimeVerificationReport,
    pub closeout_record: AppRuntimeProjectionRuntimeVerificationCloseoutRecord,
    pub closeout_record_path: Option<String>,
    pub closeout_record_persisted: bool,
    pub closeout_complete: bool,
    pub runtime_apply_allowed: bool,
    pub phase8_allowed: bool,
    pub promotion_allowed: bool,
    pub user_trigger_required: bool,
    pub auto_rollout: bool,
    pub auto_rollback: bool,
    pub mutates_runtime: bool,
    pub reload_mihomo: bool,
    pub next_app_runtime_step: String,
    pub blockers: Vec<String>,
    pub warnings: Vec<String>,
    pub facts: Vec<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AppRuntimeProjectionActivationPreflightReport {
    pub status: AppRuntimeDiagnosticStatus,
    pub reason: String,
    pub artifact_id: String,
    pub app_id: Option<String>,
    pub checksum: Option<String>,
    pub storage_path: Option<String>,
    pub activation_mode: Option<AppRuntimeProjectionActivationMode>,
    pub mutates_runtime: Option<bool>,
    pub checks: Vec<AppRuntimeDiagnosticCheck>,
    pub summary: AppRuntimeDiagnosticsSummary,
    pub facts: Vec<String>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct MihomoRuleProjection {
    pub matcher: String,
    pub value: String,
    pub target: String,
    pub rule: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct MihomoProxyGroupProjection {
    pub name: String,
    #[serde(rename = "type")]
    pub group_type: String,
    pub proxies: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub interval: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct MihomoDnsProjection {
    pub profile_id: String,
    pub name: String,
    pub nameservers: Vec<String>,
    pub runtime_supported_nameservers: usize,
}

#[derive(Debug, Serialize)]
pub(super) struct MihomoYamlPatch {
    #[serde(rename = "proxy-groups", skip_serializing_if = "Vec::is_empty")]
    pub(super) proxy_groups: Vec<MihomoProxyGroupProjection>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub(super) rules: Vec<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum AppRuntimeDiagnosticStatus {
    Healthy,
    Degraded,
    Blocked,
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum AppRuntimeDiagnosticSeverity {
    Info,
    Warning,
    Error,
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum AppRuntimeDiagnosticCheckStatus {
    Passed,
    Warning,
    Failed,
    Skipped,
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum AppRuntimeDiagnosticCategory {
    Registry,
    PolicyBinding,
    NodePool,
    Dns,
    Security,
    Projection,
    RuntimeBoundary,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AppRuntimeDiagnosticCheck {
    pub check_id: String,
    pub category: AppRuntimeDiagnosticCategory,
    pub severity: AppRuntimeDiagnosticSeverity,
    pub status: AppRuntimeDiagnosticCheckStatus,
    pub message: String,
    pub details: Vec<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AppRuntimeDiagnosticsSummary {
    pub passed: usize,
    pub warnings: usize,
    pub failed: usize,
    pub skipped: usize,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum AppRuntimeSessionStatus {
    Planned,
    Blocked,
    Completed,
    Failed,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum AppRuntimeSessionObservationSource {
    ConnectionMetricsSnapshot,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum AppRuntimeSessionAttributionStatus {
    Unattributed,
    AppMatched,
    AppMismatch,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AppRuntimeSessionTrafficObservation {
    pub upload_total: u64,
    pub download_total: u64,
    pub upload_speed: u64,
    pub download_speed: u64,
    pub active_connection_count: usize,
    pub closed_since_last: usize,
    pub memory: u32,
    pub stale: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AppRuntimeSessionAttributionCandidate {
    pub connection_id: String,
    pub process: String,
    pub process_path: String,
    pub host: String,
    pub rule: String,
    pub rule_payload: String,
    pub chains: Vec<String>,
    pub upload: u64,
    pub download: u64,
    pub matched_by: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AppRuntimeSessionObservationRecord {
    pub observation_id: String,
    pub session_id: String,
    pub recorded_at: i64,
    pub source: AppRuntimeSessionObservationSource,
    pub attribution_status: AppRuntimeSessionAttributionStatus,
    pub traffic: AppRuntimeSessionTrafficObservation,
    pub connection_speed_count: usize,
    #[serde(default)]
    pub attribution_candidates: Vec<AppRuntimeSessionAttributionCandidate>,
    pub facts: Vec<String>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AppRuntimeSessionRecord {
    pub session_id: String,
    pub app_id: String,
    pub status: AppRuntimeSessionStatus,
    pub plan_status: AppRuntimePlanStatus,
    pub diagnostics_status: AppRuntimeDiagnosticStatus,
    pub diagnostics_summary: AppRuntimeDiagnosticsSummary,
    pub reason: String,
    pub started_at: i64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ended_at: Option<i64>,
    pub projected_rules: Vec<String>,
    pub projected_proxy_groups: Vec<String>,
    #[serde(default)]
    pub observations: Vec<AppRuntimeSessionObservationRecord>,
    pub facts: Vec<String>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AppRuntimeSessionStartReport {
    pub session: AppRuntimeSessionRecord,
    pub diagnostics: AppRuntimeDiagnosticsReport,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AppRuntimeSessionFinishRequest {
    pub session_id: String,
    pub status: AppRuntimeSessionStatus,
    #[serde(default)]
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AppRuntimeSessionEvaluationSummary {
    pub observation_count: usize,
    pub matched_observations: usize,
    pub mismatch_observations: usize,
    pub unattributed_observations: usize,
    pub stale_observations: usize,
    pub attribution_candidate_count: usize,
    pub upload_total: u64,
    pub download_total: u64,
    pub max_active_connections: usize,
    pub observed_chains: Vec<String>,
    pub observed_hosts: Vec<String>,
    pub matched_by: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AppRuntimeSessionEvaluationReport {
    pub session_id: String,
    pub app_id: String,
    pub status: AppRuntimeDiagnosticStatus,
    pub reason: String,
    pub summary: AppRuntimeSessionEvaluationSummary,
    pub facts: Vec<String>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum AppRuntimeLeakDimension {
    ProxyLeak,
    DnsLeak,
    ExitVerification,
    NodePoolConsistency,
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum AppRuntimeLeakCheckStatus {
    Pass,
    Warn,
    Fail,
    NotApplicable,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AppRuntimeLeakCheck {
    pub dimension: AppRuntimeLeakDimension,
    pub status: AppRuntimeLeakCheckStatus,
    pub severity: AppRuntimeDiagnosticSeverity,
    pub message: String,
    pub facts: Vec<String>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Default, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AppRuntimeLeakSummary {
    pub pass: usize,
    pub warn: usize,
    pub fail: usize,
    pub not_applicable: usize,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AppRuntimeSessionLeakReport {
    pub session_id: String,
    pub app_id: String,
    pub status: AppRuntimeDiagnosticStatus,
    pub reason: String,
    pub routing_intent: Option<AppRoutingIntent>,
    pub evaluation_summary: AppRuntimeSessionEvaluationSummary,
    pub checks: Vec<AppRuntimeLeakCheck>,
    pub summary: AppRuntimeLeakSummary,
    pub facts: Vec<String>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AppRuntimeDiagnosticsReport {
    pub status: AppRuntimeDiagnosticStatus,
    pub reason: String,
    pub app_id: String,
    pub session_id: Option<String>,
    pub plan: AppRuntimePlan,
    pub mihomo_projection: AppRuntimeMihomoProjection,
    pub checks: Vec<AppRuntimeDiagnosticCheck>,
    pub summary: AppRuntimeDiagnosticsSummary,
    pub facts: Vec<String>,
    pub warnings: Vec<String>,
}

fn default_enabled() -> bool {
    true
}

fn default_runtime_apply_force() -> bool {
    true
}
