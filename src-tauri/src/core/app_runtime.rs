use crate::{
    core::{
        connection_metrics::{self, ConnectionAttributionCandidate, ConnectionMetricsSnapshot},
        dns_runtime::{DnsResolverPlan, DnsResolverPlanStatus, build_dns_resolver_plan},
    },
    utils::{dirs, help},
};
use anyhow::{Result, bail};
use chrono::Local;
use serde::{Deserialize, Serialize};
use sha2::{Digest as _, Sha256};
use smartstring::alias::String;
use std::collections::{BTreeMap, BTreeSet};

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

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct MihomoRuleProjection {
    pub matcher: String,
    pub value: String,
    pub target: String,
    pub rule: String,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
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

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct MihomoDnsProjection {
    pub profile_id: String,
    pub name: String,
    pub nameservers: Vec<String>,
    pub runtime_supported_nameservers: usize,
}

#[derive(Debug, Serialize)]
struct MihomoYamlPatch {
    #[serde(rename = "proxy-groups", skip_serializing_if = "Vec::is_empty")]
    proxy_groups: Vec<MihomoProxyGroupProjection>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    rules: Vec<String>,
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

pub async fn read_app_runtime_state_document() -> Result<AppRuntimeStateDocument> {
    let path = dirs::app_runtime_state_path()?;
    if !tokio::fs::try_exists(&path).await.unwrap_or(false) {
        return Ok(AppRuntimeStateDocument::default());
    }
    help::read_yaml(&path).await
}

pub async fn upsert_app_registry_entry(mut entry: AppRegistryEntry) -> Result<AppRuntimeStateDocument> {
    validate_app(&entry)?;
    entry.updated_at = now_millis();
    update_state_document(|state| {
        upsert_by(&mut state.apps, entry, |app| app.app_id.clone());
        Ok(())
    })
    .await
}

pub async fn delete_app_registry_entry(app_id: &str) -> Result<AppRuntimeStateDocument> {
    let app_id = normalize_id(app_id, "app_id")?;
    update_state_document(|state| {
        state.apps.retain(|app| app.app_id != app_id);
        state.policy_bindings.retain(|binding| binding.app_id != app_id);
        Ok(())
    })
    .await
}

pub async fn upsert_node_pool(mut node_pool: NodePool) -> Result<AppRuntimeStateDocument> {
    validate_node_pool(&node_pool)?;
    node_pool.updated_at = now_millis();
    update_state_document(|state| {
        upsert_by(&mut state.node_pools, node_pool, |pool| pool.pool_id.clone());
        Ok(())
    })
    .await
}

pub async fn delete_node_pool(pool_id: &str) -> Result<AppRuntimeStateDocument> {
    let pool_id = normalize_id(pool_id, "pool_id")?;
    update_state_document(|state| {
        state.node_pools.retain(|pool| pool.pool_id != pool_id);
        for binding in &mut state.policy_bindings {
            if binding.node_pool_id.as_deref() == Some(pool_id.as_str()) {
                binding.node_pool_id = None;
                binding.updated_at = now_millis();
            }
        }
        Ok(())
    })
    .await
}

pub async fn upsert_dns_profile(mut dns_profile: DnsProfile) -> Result<AppRuntimeStateDocument> {
    validate_dns_profile(&dns_profile)?;
    dns_profile.updated_at = now_millis();
    update_state_document(|state| {
        upsert_by(&mut state.dns_profiles, dns_profile, |profile| {
            profile.profile_id.clone()
        });
        Ok(())
    })
    .await
}

pub async fn delete_dns_profile(profile_id: &str) -> Result<AppRuntimeStateDocument> {
    let profile_id = normalize_id(profile_id, "profile_id")?;
    update_state_document(|state| {
        state.dns_profiles.retain(|profile| profile.profile_id != profile_id);
        for binding in &mut state.policy_bindings {
            if binding.dns_profile_id.as_deref() == Some(profile_id.as_str()) {
                binding.dns_profile_id = None;
                binding.updated_at = now_millis();
            }
        }
        Ok(())
    })
    .await
}

pub async fn upsert_security_profile(mut security_profile: SecurityProfile) -> Result<AppRuntimeStateDocument> {
    validate_security_profile(&security_profile)?;
    security_profile.updated_at = now_millis();
    update_state_document(|state| {
        upsert_by(&mut state.security_profiles, security_profile, |profile| {
            profile.profile_id.clone()
        });
        Ok(())
    })
    .await
}

pub async fn delete_security_profile(profile_id: &str) -> Result<AppRuntimeStateDocument> {
    let profile_id = normalize_id(profile_id, "profile_id")?;
    update_state_document(|state| {
        state
            .security_profiles
            .retain(|profile| profile.profile_id != profile_id);
        for binding in &mut state.policy_bindings {
            if binding.security_profile_id.as_deref() == Some(profile_id.as_str()) {
                binding.security_profile_id = None;
                binding.updated_at = now_millis();
            }
        }
        Ok(())
    })
    .await
}

pub async fn upsert_app_policy_binding(mut binding: AppPolicyBinding) -> Result<AppRuntimeStateDocument> {
    validate_policy_binding(&binding)?;
    binding.updated_at = now_millis();
    update_state_document(|state| {
        if state.apps.iter().all(|app| app.app_id != binding.app_id) {
            bail!("policy binding references missing app_id `{}`", binding.app_id);
        }
        if let Some(pool_id) = binding.node_pool_id.as_ref()
            && state.node_pools.iter().all(|pool| &pool.pool_id != pool_id)
        {
            bail!("policy binding references missing node_pool_id `{pool_id}`");
        }
        if let Some(profile_id) = binding.dns_profile_id.as_ref()
            && state
                .dns_profiles
                .iter()
                .all(|profile| &profile.profile_id != profile_id)
        {
            bail!("policy binding references missing dns_profile_id `{profile_id}`");
        }
        if let Some(profile_id) = binding.security_profile_id.as_ref()
            && state
                .security_profiles
                .iter()
                .all(|profile| &profile.profile_id != profile_id)
        {
            bail!("policy binding references missing security_profile_id `{profile_id}`");
        }
        upsert_by(&mut state.policy_bindings, binding, |stored| stored.binding_id.clone());
        Ok(())
    })
    .await
}

pub async fn delete_app_policy_binding(binding_id: &str) -> Result<AppRuntimeStateDocument> {
    let binding_id = normalize_id(binding_id, "binding_id")?;
    update_state_document(|state| {
        state.policy_bindings.retain(|binding| binding.binding_id != binding_id);
        Ok(())
    })
    .await
}

pub async fn list_app_runtime_sessions(app_id: Option<String>) -> Result<Vec<AppRuntimeSessionRecord>> {
    let state = read_app_runtime_state_document().await?;
    let app_id = app_id
        .as_deref()
        .map(|value| normalize_id(value, "app_id"))
        .transpose()?;
    let mut sessions: Vec<_> = state
        .sessions
        .into_iter()
        .filter(|session| app_id.as_ref().is_none_or(|app_id| &session.app_id == app_id))
        .collect();
    sessions.sort_by(|left, right| {
        right
            .started_at
            .cmp(&left.started_at)
            .then_with(|| right.session_id.cmp(&left.session_id))
    });
    Ok(sessions)
}

pub async fn start_app_runtime_session(request: AppRuntimePlanRequest) -> Result<AppRuntimeSessionStartReport> {
    let app_id = normalize_id(&request.app_id, "app_id")?;
    let session_id = runtime_session_id(&app_id, request.session_id.as_deref())?;
    let request = AppRuntimePlanRequest {
        app_id,
        session_id: Some(session_id.clone()),
    };

    let mut state = read_app_runtime_state_document().await?;
    let diagnostics = diagnose_app_runtime(&state, request)?;
    let session = session_record_from_diagnostics(session_id, &diagnostics);
    upsert_by(&mut state.sessions, session.clone(), |stored| stored.session_id.clone());
    save_app_runtime_state_document(&state).await?;

    Ok(AppRuntimeSessionStartReport { session, diagnostics })
}

pub async fn finish_app_runtime_session(request: AppRuntimeSessionFinishRequest) -> Result<AppRuntimeSessionRecord> {
    let session_id = normalize_id(&request.session_id, "session_id")?;
    if matches!(request.status, AppRuntimeSessionStatus::Planned) {
        bail!("finished app runtime session status cannot be planned");
    }

    let mut state = read_app_runtime_state_document().await?;
    let Some(session) = state
        .sessions
        .iter_mut()
        .find(|session| session.session_id == session_id)
    else {
        bail!("app runtime session `{session_id}` was not found");
    };
    session.status = request.status;
    session.ended_at = Some(now_millis());
    if let Some(reason) = request.reason.as_ref().filter(|reason| !reason.trim().is_empty()) {
        session.reason = reason.trim().into();
    }
    let session = session.clone();
    save_app_runtime_state_document(&state).await?;
    Ok(session)
}

pub async fn record_app_runtime_session_observation(session_id: &str) -> Result<AppRuntimeSessionRecord> {
    let session_id = normalize_id(session_id, "session_id")?;
    let metrics = connection_metrics::get_connection_metrics_snapshot().await;

    let mut state = read_app_runtime_state_document().await?;
    let Some(session) = state
        .sessions
        .iter_mut()
        .find(|session| session.session_id == session_id)
    else {
        bail!("app runtime session `{session_id}` was not found");
    };
    let observation = session_observation_from_metrics(session, &metrics);
    session.observations.push(observation);
    let session = session.clone();
    save_app_runtime_state_document(&state).await?;
    Ok(session)
}

pub async fn evaluate_app_runtime_session(session_id: &str) -> Result<AppRuntimeSessionEvaluationReport> {
    let session_id = normalize_id(session_id, "session_id")?;
    let state = read_app_runtime_state_document().await?;
    let Some(session) = state.sessions.iter().find(|session| session.session_id == session_id) else {
        bail!("app runtime session `{session_id}` was not found");
    };

    Ok(evaluation_report_from_session(session))
}

pub async fn verify_app_runtime_session_leak(session_id: &str) -> Result<AppRuntimeSessionLeakReport> {
    let session_id = normalize_id(session_id, "session_id")?;
    let state = read_app_runtime_state_document().await?;
    let Some(session) = state.sessions.iter().find(|session| session.session_id == session_id) else {
        bail!("app runtime session `{session_id}` was not found");
    };

    Ok(leak_report_from_session(&state, session))
}

pub fn explain_app_runtime_plan(state: &AppRuntimeStateDocument, request: AppRuntimePlanRequest) -> AppRuntimePlan {
    let app_id = request.app_id.trim().into();
    let projection = RuntimeProjectionPlan {
        status: RuntimeProjectionStatus::PlanningOnly,
        backend: "mihomo_config_projection".into(),
        mutates_runtime: false,
        outputs: vec![
            "app_policy_runtime_plan".into(),
            "dns_resolver_plan_projection".into(),
            "future_mihomo_rules_projection".into(),
            "future_proxy_group_projection".into(),
        ],
    };
    let facts = vec![
        "Rust AppRuntimeStateDocument is the only source of app/pool/dns-profile/security-profile/policy facts".into(),
        "first slice only explains a runtime plan and does not mutate Mihomo runtime".into(),
    ];

    let Some(app) = state.apps.iter().find(|entry| entry.app_id == app_id).cloned() else {
        return AppRuntimePlan {
            status: AppRuntimePlanStatus::Rejected,
            reason: format!("app `{app_id}` is not registered").into(),
            app_id,
            session_id: request.session_id,
            app: None,
            policy_binding: None,
            node_pool: None,
            dns_profile: None,
            security_profile: None,
            routing_intent: None,
            projection,
            facts,
            warnings: Vec::new(),
        };
    };

    let Some(binding) = state
        .policy_bindings
        .iter()
        .find(|binding| binding.app_id == app_id && binding.enabled)
        .cloned()
    else {
        return AppRuntimePlan {
            status: AppRuntimePlanStatus::Rejected,
            reason: format!("app `{app_id}` has no enabled policy binding").into(),
            app_id,
            session_id: request.session_id,
            app: Some(app),
            policy_binding: None,
            node_pool: None,
            dns_profile: None,
            security_profile: None,
            routing_intent: None,
            projection,
            facts,
            warnings: Vec::new(),
        };
    };

    let node_pool = binding
        .node_pool_id
        .as_ref()
        .and_then(|pool_id| state.node_pools.iter().find(|pool| &pool.pool_id == pool_id))
        .cloned();
    let mut warnings = Vec::new();
    let dns_profile = binding
        .dns_profile_id
        .as_ref()
        .and_then(|profile_id| {
            state
                .dns_profiles
                .iter()
                .find(|profile| &profile.profile_id == profile_id)
        })
        .cloned();
    let security_profile = binding
        .security_profile_id
        .as_ref()
        .and_then(|profile_id| {
            state
                .security_profiles
                .iter()
                .find(|profile| &profile.profile_id == profile_id)
        })
        .cloned();

    if requires_node_pool(binding.routing_intent) && node_pool.is_none() {
        warnings.push(format!("routing intent `{:?}` has no node pool", binding.routing_intent).into());
    }
    if let Some(profile_id) = binding.dns_profile_id.as_ref()
        && dns_profile.is_none()
    {
        warnings.push(format!("policy binding references missing dns_profile_id `{profile_id}`").into());
    }
    if let Some(profile_id) = binding.security_profile_id.as_ref()
        && security_profile.is_none()
    {
        warnings.push(format!("policy binding references missing security_profile_id `{profile_id}`").into());
    }

    AppRuntimePlan {
        status: AppRuntimePlanStatus::Ready,
        reason: plan_reason(&app, &binding, node_pool.as_ref()),
        app_id,
        session_id: request.session_id,
        app: Some(app),
        policy_binding: Some(binding.clone()),
        node_pool: node_pool.map(node_pool_plan_view),
        dns_profile: dns_profile.and_then(|profile| match dns_profile_plan_view(profile) {
            Ok(plan) => Some(plan),
            Err(error) => {
                warnings.push(format!("dns profile plan failed: {error}").into());
                None
            }
        }),
        security_profile: security_profile.map(security_profile_plan_view),
        routing_intent: Some(binding.routing_intent),
        projection,
        facts,
        warnings,
    }
}

pub fn project_app_runtime_plan_to_mihomo(
    state: &AppRuntimeStateDocument,
    request: AppRuntimePlanRequest,
) -> Result<AppRuntimeMihomoProjection> {
    let plan = explain_app_runtime_plan(state, request);
    let mut facts = plan.facts.clone();
    facts.push("Mihomo projection is an execution artifact; Rust app runtime state remains the source of truth".into());

    if plan.status == AppRuntimePlanStatus::Rejected {
        return Ok(AppRuntimeMihomoProjection {
            status: plan.status,
            reason: plan.reason,
            app_id: plan.app_id,
            session_id: plan.session_id,
            mutates_runtime: false,
            proxy_groups: Vec::new(),
            rules: Vec::new(),
            dns: None,
            yaml_patch: String::new(),
            facts,
            warnings: plan.warnings,
        });
    }

    let mut warnings = plan.warnings.clone();
    let Some(app) = plan.app.as_ref() else {
        warnings.push("runtime plan is ready but missing app facts".into());
        return ready_projection_without_yaml(plan, facts, warnings);
    };
    let routing_intent = plan.routing_intent.unwrap_or(AppRoutingIntent::Direct);
    let mut proxy_groups = Vec::new();
    let target = mihomo_target_for_plan(&plan, routing_intent, &mut proxy_groups, &mut warnings);
    let rules = target
        .as_ref()
        .map(|target| mihomo_rules_for_app(app, target, &mut warnings))
        .unwrap_or_default();
    let dns = plan.dns_profile.as_ref().map(mihomo_dns_projection);
    let yaml_patch = mihomo_yaml_patch(&proxy_groups, &rules)?;
    let reason = if rules.is_empty() {
        format!("app `{}` produced no Mihomo-compatible rule projection", plan.app_id).into()
    } else {
        format!(
            "app `{}` projected {} Mihomo rule(s) and {} proxy group(s)",
            plan.app_id,
            rules.len(),
            proxy_groups.len()
        )
        .into()
    };

    Ok(AppRuntimeMihomoProjection {
        status: plan.status,
        reason,
        app_id: plan.app_id,
        session_id: plan.session_id,
        mutates_runtime: false,
        proxy_groups,
        rules,
        dns,
        yaml_patch,
        facts,
        warnings,
    })
}

pub fn build_app_runtime_projection_artifact(
    state: &AppRuntimeStateDocument,
    request: AppRuntimePlanRequest,
) -> Result<AppRuntimeProjectionArtifact> {
    let diagnostics = diagnose_app_runtime(state, request)?;
    let plan = diagnostics.plan.clone();
    let projection = diagnostics.mihomo_projection.clone();
    let validation = validate_app_runtime_projection_artifact(&plan, &projection, &diagnostics);
    let checksum = app_runtime_projection_checksum(&projection);
    let binding = plan.policy_binding.as_ref();
    let generated_at = Local::now().timestamp_millis();
    let artifact_id = format!("app-runtime-{}-{}", plan.app_id, &checksum[..12]);
    let mut facts = plan.facts.clone();
    facts.push("Projection artifact is generated from Rust AppRuntimeStateDocument and RuntimePlan".into());
    facts.push("Artifact activation is staged; this command does not reload or mutate Mihomo runtime".into());
    let mut warnings = projection.warnings.clone();
    warnings.extend(validation.warnings.iter().cloned());
    warnings.sort();
    warnings.dedup();

    Ok(AppRuntimeProjectionArtifact {
        artifact_id: artifact_id.into(),
        app_id: plan.app_id.clone(),
        session_id: plan.session_id.clone(),
        binding_id: binding.map(|item| item.binding_id.clone()),
        node_pool_id: binding.and_then(|item| item.node_pool_id.clone()),
        dns_profile_id: binding.and_then(|item| item.dns_profile_id.clone()),
        security_profile_id: binding.and_then(|item| item.security_profile_id.clone()),
        generated_at,
        storage_path: None,
        activation_mode: AppRuntimeProjectionActivationMode::Staged,
        mutates_runtime: false,
        checksum: checksum.into(),
        plan,
        projection,
        diagnostics,
        validation,
        facts,
        warnings,
    })
}

pub async fn persist_app_runtime_projection_artifact(artifact: &AppRuntimeProjectionArtifact) -> Result<String> {
    let path = app_runtime_projection_artifact_path(&artifact.artifact_id)?;
    if let Some(parent) = path.parent() {
        tokio::fs::create_dir_all(parent).await?;
    }
    let storage_path: String = path.to_string_lossy().to_string().into();
    let mut persisted_artifact = artifact.clone();
    persisted_artifact.storage_path = Some(storage_path.clone());
    help::save_yaml(&path, &persisted_artifact, None).await?;

    Ok(storage_path)
}

pub async fn preflight_app_runtime_projection_activation(
    request: AppRuntimeProjectionActivationPreflightRequest,
) -> Result<AppRuntimeProjectionActivationPreflightReport> {
    let path = app_runtime_projection_artifact_path(&request.artifact_id)?;
    let storage_path: String = path.to_string_lossy().to_string().into();
    let raw_yaml = match tokio::fs::read_to_string(&path).await {
        Ok(raw_yaml) => raw_yaml,
        Err(err) => {
            return Ok(app_runtime_activation_preflight_missing_artifact_report(
                request,
                storage_path,
                err.to_string().into(),
            ));
        }
    };

    Ok(app_runtime_activation_preflight_report_from_yaml(
        &request,
        storage_path,
        raw_yaml.as_str(),
    ))
}

pub fn diagnose_app_runtime(
    state: &AppRuntimeStateDocument,
    request: AppRuntimePlanRequest,
) -> Result<AppRuntimeDiagnosticsReport> {
    let plan = explain_app_runtime_plan(state, request.clone());
    let mihomo_projection = project_app_runtime_plan_to_mihomo(state, request)?;
    let mut checks = Vec::new();

    checks.push(diagnostic_check(
        "app_registered",
        AppRuntimeDiagnosticCategory::Registry,
        if plan.app.is_some() {
            AppRuntimeDiagnosticCheckStatus::Passed
        } else {
            AppRuntimeDiagnosticCheckStatus::Failed
        },
        if plan.app.is_some() {
            format!("app `{}` is registered", plan.app_id).into()
        } else {
            format!("app `{}` is not registered", plan.app_id).into()
        },
        Vec::new(),
    ));

    checks.push(diagnostic_check(
        "enabled_policy_binding",
        AppRuntimeDiagnosticCategory::PolicyBinding,
        if plan.policy_binding.is_some() {
            AppRuntimeDiagnosticCheckStatus::Passed
        } else if plan.app.is_some() {
            AppRuntimeDiagnosticCheckStatus::Failed
        } else {
            AppRuntimeDiagnosticCheckStatus::Skipped
        },
        if let Some(binding) = plan.policy_binding.as_ref() {
            format!("enabled policy binding `{}` selected", binding.binding_id).into()
        } else if plan.app.is_some() {
            format!("app `{}` has no enabled policy binding", plan.app_id).into()
        } else {
            "policy binding check skipped because app is missing".into()
        },
        Vec::new(),
    ));

    append_node_pool_diagnostics(&plan, &mut checks);
    append_dns_diagnostics(&plan, &mut checks);
    append_security_diagnostics(&plan, &mut checks);
    append_projection_diagnostics(&mihomo_projection, &mut checks);

    checks.push(diagnostic_check(
        "runtime_mutation_boundary",
        AppRuntimeDiagnosticCategory::RuntimeBoundary,
        AppRuntimeDiagnosticCheckStatus::Passed,
        "diagnostics and projection do not mutate Mihomo runtime".into(),
        vec!["mutatesRuntime=false".into()],
    ));

    let summary = diagnostics_summary(&checks);
    let status = diagnostics_status(&summary);
    let reason = diagnostics_reason(status, &summary);
    let warnings = combined_diagnostic_warnings(&plan, &mihomo_projection, &checks);
    let mut facts = plan.facts.clone();
    facts.push("App-scoped diagnostics aggregate plan, DNS, security and Mihomo projection readiness".into());

    Ok(AppRuntimeDiagnosticsReport {
        status,
        reason,
        app_id: plan.app_id.clone(),
        session_id: plan.session_id.clone(),
        plan,
        mihomo_projection,
        checks,
        summary,
        facts,
        warnings,
    })
}

async fn update_state_document(
    update: impl FnOnce(&mut AppRuntimeStateDocument) -> Result<()>,
) -> Result<AppRuntimeStateDocument> {
    let mut state = read_app_runtime_state_document().await?;
    update(&mut state)?;
    save_app_runtime_state_document(&state).await?;
    Ok(state)
}

async fn save_app_runtime_state_document(state: &AppRuntimeStateDocument) -> Result<()> {
    let dir = dirs::app_runtime_dir()?;
    tokio::fs::create_dir_all(&dir).await?;
    help::save_yaml(&dirs::app_runtime_state_path()?, state, None).await
}

fn upsert_by<T>(items: &mut Vec<T>, item: T, key: impl Fn(&T) -> String) {
    let item_key = key(&item);
    if let Some(stored) = items.iter_mut().find(|stored| key(stored) == item_key) {
        *stored = item;
    } else {
        items.push(item);
    }
}

fn runtime_session_id(app_id: &str, requested_session_id: Option<&str>) -> Result<String> {
    if let Some(session_id) = requested_session_id.filter(|session_id| !session_id.trim().is_empty()) {
        return normalize_id(session_id, "session_id");
    }
    Ok(format!("{app_id}-{}", now_millis()).into())
}

fn session_record_from_diagnostics(
    session_id: String,
    diagnostics: &AppRuntimeDiagnosticsReport,
) -> AppRuntimeSessionRecord {
    let status = if diagnostics.status == AppRuntimeDiagnosticStatus::Blocked
        || diagnostics.plan.status == AppRuntimePlanStatus::Rejected
    {
        AppRuntimeSessionStatus::Blocked
    } else {
        AppRuntimeSessionStatus::Planned
    };

    AppRuntimeSessionRecord {
        session_id,
        app_id: diagnostics.app_id.clone(),
        status,
        plan_status: diagnostics.plan.status,
        diagnostics_status: diagnostics.status,
        diagnostics_summary: diagnostics.summary.clone(),
        reason: diagnostics.reason.clone(),
        started_at: now_millis(),
        ended_at: None,
        projected_rules: diagnostics
            .mihomo_projection
            .rules
            .iter()
            .map(|rule| rule.rule.clone())
            .collect(),
        projected_proxy_groups: diagnostics
            .mihomo_projection
            .proxy_groups
            .iter()
            .map(|group| group.name.clone())
            .collect(),
        observations: Vec::new(),
        facts: diagnostics.facts.clone(),
        warnings: diagnostics.warnings.clone(),
    }
}

fn session_observation_from_metrics(
    session: &AppRuntimeSessionRecord,
    metrics: &ConnectionMetricsSnapshot,
) -> AppRuntimeSessionObservationRecord {
    let recorded_at = now_millis();
    let attribution_candidates = metrics
        .attribution_candidates
        .iter()
        .filter_map(|candidate| session_attribution_candidate(session, candidate))
        .collect::<Vec<_>>();
    let attribution_status = if metrics.attribution_candidates.is_empty() {
        AppRuntimeSessionAttributionStatus::Unattributed
    } else if attribution_candidates.is_empty() {
        AppRuntimeSessionAttributionStatus::AppMismatch
    } else {
        AppRuntimeSessionAttributionStatus::AppMatched
    };
    let mut facts = vec![
        "Observation snapshots reuse the Rust connection metrics path".into(),
        format!(
            "inspected {} connection attribution candidate(s)",
            metrics.attribution_candidates.len()
        )
        .into(),
    ];
    if !attribution_candidates.is_empty() {
        facts.push(
            format!(
                "matched {} candidate(s) against session projected rules or proxy groups",
                attribution_candidates.len()
            )
            .into(),
        );
    }

    AppRuntimeSessionObservationRecord {
        observation_id: format!("{}-{recorded_at}", session.session_id).into(),
        session_id: session.session_id.clone(),
        recorded_at,
        source: AppRuntimeSessionObservationSource::ConnectionMetricsSnapshot,
        attribution_status,
        traffic: AppRuntimeSessionTrafficObservation {
            upload_total: metrics.traffic.upload_total,
            download_total: metrics.traffic.download_total,
            upload_speed: metrics.traffic.upload_speed,
            download_speed: metrics.traffic.download_speed,
            active_connection_count: metrics.traffic.active_connection_count,
            closed_since_last: metrics.traffic.closed_since_last,
            memory: metrics.traffic.memory,
            stale: metrics.stale,
        },
        connection_speed_count: metrics.speeds.len(),
        attribution_candidates,
        facts,
        warnings: session_observation_warnings(attribution_status),
    }
}

fn session_attribution_candidate(
    session: &AppRuntimeSessionRecord,
    candidate: &ConnectionAttributionCandidate,
) -> Option<AppRuntimeSessionAttributionCandidate> {
    let matched_by = session_candidate_matches(session, candidate);
    if matched_by.is_empty() {
        return None;
    }

    Some(AppRuntimeSessionAttributionCandidate {
        connection_id: candidate.id.clone().into(),
        process: candidate.process.clone().into(),
        process_path: candidate.process_path.clone().into(),
        host: candidate.host.clone().into(),
        rule: candidate.rule.clone().into(),
        rule_payload: candidate.rule_payload.clone().into(),
        chains: candidate.chains.iter().map(|chain| chain.as_str().into()).collect(),
        upload: candidate.upload,
        download: candidate.download,
        matched_by,
    })
}

fn session_candidate_matches(
    session: &AppRuntimeSessionRecord,
    candidate: &ConnectionAttributionCandidate,
) -> Vec<String> {
    let mut matched_by = Vec::new();
    for proxy_group in &session.projected_proxy_groups {
        if candidate
            .chains
            .iter()
            .any(|chain| chain.as_str() == proxy_group.as_str())
        {
            matched_by.push(format!("proxyGroup:{proxy_group}").into());
        }
    }
    for rule in &session.projected_rules {
        if projected_rule_matches_candidate(rule, candidate) {
            matched_by.push(format!("projectedRule:{rule}").into());
        }
    }
    matched_by
}

fn projected_rule_matches_candidate(rule: &str, candidate: &ConnectionAttributionCandidate) -> bool {
    let mut parts = rule.splitn(3, ',');
    let Some(kind) = parts.next() else {
        return false;
    };
    let Some(payload) = parts.next() else {
        return false;
    };
    match kind {
        "PROCESS-NAME" => {
            candidate.process.eq_ignore_ascii_case(payload)
                || candidate
                    .process_path
                    .rsplit(['/', '\\'])
                    .next()
                    .is_some_and(|name| name.eq_ignore_ascii_case(payload))
        }
        "PROCESS-PATH" => candidate.process_path.eq_ignore_ascii_case(payload),
        _ => false,
    }
}

fn session_observation_warnings(status: AppRuntimeSessionAttributionStatus) -> Vec<String> {
    match status {
        AppRuntimeSessionAttributionStatus::AppMatched => Vec::new(),
        AppRuntimeSessionAttributionStatus::AppMismatch => {
            vec!["Latest connection metadata did not match this session's projected rules or proxy groups".into()]
        }
        AppRuntimeSessionAttributionStatus::Unattributed => {
            vec!["No connection attribution candidates are available in the latest metrics snapshot".into()]
        }
    }
}

fn evaluation_report_from_session(session: &AppRuntimeSessionRecord) -> AppRuntimeSessionEvaluationReport {
    let summary = session_evaluation_summary(session);
    let status = session_evaluation_status(session, &summary);
    let warnings = session_evaluation_warnings(session, status, &summary);
    let reason = session_evaluation_reason(status, &summary);
    let facts = vec![
        format!("session status is {:?}", session.status).into(),
        format!("evaluated {} recorded observation(s)", summary.observation_count).into(),
        format!(
            "matched {} attribution candidate(s)",
            summary.attribution_candidate_count
        )
        .into(),
    ];

    AppRuntimeSessionEvaluationReport {
        session_id: session.session_id.clone(),
        app_id: session.app_id.clone(),
        status,
        reason,
        summary,
        facts,
        warnings,
    }
}

fn session_evaluation_summary(session: &AppRuntimeSessionRecord) -> AppRuntimeSessionEvaluationSummary {
    let mut summary = AppRuntimeSessionEvaluationSummary {
        observation_count: session.observations.len(),
        ..AppRuntimeSessionEvaluationSummary::default()
    };
    let mut observed_chains = BTreeSet::new();
    let mut observed_hosts = BTreeSet::new();
    let mut matched_by = BTreeSet::new();

    for observation in &session.observations {
        match observation.attribution_status {
            AppRuntimeSessionAttributionStatus::AppMatched => summary.matched_observations += 1,
            AppRuntimeSessionAttributionStatus::AppMismatch => summary.mismatch_observations += 1,
            AppRuntimeSessionAttributionStatus::Unattributed => summary.unattributed_observations += 1,
        }
        if observation.traffic.stale {
            summary.stale_observations += 1;
        }
        summary.upload_total = summary.upload_total.max(observation.traffic.upload_total);
        summary.download_total = summary.download_total.max(observation.traffic.download_total);
        summary.max_active_connections = summary
            .max_active_connections
            .max(observation.traffic.active_connection_count);
        summary.attribution_candidate_count += observation.attribution_candidates.len();

        for candidate in &observation.attribution_candidates {
            if !candidate.host.is_empty() {
                observed_hosts.insert(candidate.host.clone());
            }
            for chain in &candidate.chains {
                observed_chains.insert(chain.clone());
            }
            for matcher in &candidate.matched_by {
                matched_by.insert(matcher.clone());
            }
        }
    }

    summary.observed_chains = observed_chains.into_iter().collect();
    summary.observed_hosts = observed_hosts.into_iter().collect();
    summary.matched_by = matched_by.into_iter().collect();
    summary
}

fn session_evaluation_status(
    session: &AppRuntimeSessionRecord,
    summary: &AppRuntimeSessionEvaluationSummary,
) -> AppRuntimeDiagnosticStatus {
    if matches!(
        session.status,
        AppRuntimeSessionStatus::Blocked | AppRuntimeSessionStatus::Failed
    ) {
        return AppRuntimeDiagnosticStatus::Blocked;
    }
    if summary.observation_count == 0
        || summary.mismatch_observations > 0
        || summary.unattributed_observations > 0
        || summary.stale_observations > 0
    {
        return AppRuntimeDiagnosticStatus::Degraded;
    }
    AppRuntimeDiagnosticStatus::Healthy
}

fn session_evaluation_reason(
    status: AppRuntimeDiagnosticStatus,
    summary: &AppRuntimeSessionEvaluationSummary,
) -> String {
    match status {
        AppRuntimeDiagnosticStatus::Healthy => {
            "all recorded app session observations matched projected runtime artifacts".into()
        }
        AppRuntimeDiagnosticStatus::Degraded if summary.observation_count == 0 => {
            "app runtime session has no recorded observations yet".into()
        }
        AppRuntimeDiagnosticStatus::Degraded => {
            "app runtime session observations require attribution or freshness review".into()
        }
        AppRuntimeDiagnosticStatus::Blocked => {
            "app runtime session was blocked or failed before a healthy evaluation could be confirmed".into()
        }
    }
}

fn session_evaluation_warnings(
    session: &AppRuntimeSessionRecord,
    status: AppRuntimeDiagnosticStatus,
    summary: &AppRuntimeSessionEvaluationSummary,
) -> Vec<String> {
    let mut warnings = Vec::new();
    if matches!(
        session.status,
        AppRuntimeSessionStatus::Blocked | AppRuntimeSessionStatus::Failed
    ) {
        warnings.push("session ended in blocked or failed state".into());
    }
    if summary.observation_count == 0 {
        warnings.push("no connection metrics observations have been recorded for this session".into());
    }
    if summary.mismatch_observations > 0 {
        warnings.push("one or more observations had connection metadata but no app projection match".into());
    }
    if summary.unattributed_observations > 0 {
        warnings.push("one or more observations had no attribution candidates".into());
    }
    if summary.stale_observations > 0 {
        warnings.push("one or more observations were marked stale by connection metrics".into());
    }
    if status == AppRuntimeDiagnosticStatus::Healthy && summary.attribution_candidate_count == 0 {
        warnings.push("healthy status requires matched observations with attribution candidates".into());
    }
    warnings
}

fn leak_report_from_session(
    state: &AppRuntimeStateDocument,
    session: &AppRuntimeSessionRecord,
) -> AppRuntimeSessionLeakReport {
    let plan = explain_app_runtime_plan(
        state,
        AppRuntimePlanRequest {
            app_id: session.app_id.clone(),
            session_id: Some(session.session_id.clone()),
        },
    );
    let summary = session_evaluation_summary(session);
    let routing_intent = plan.routing_intent;
    let checks = vec![
        leak_proxy_check(routing_intent, session, &summary),
        leak_dns_check(&plan),
        leak_exit_check(session, &plan, routing_intent, &summary),
        leak_node_pool_check(routing_intent, &plan, session, &summary),
    ];
    let leak_summary = leak_summary_counts(&checks);
    let status = leak_status(&leak_summary);
    let reason = leak_reason(status, &leak_summary);
    let warnings = leak_report_warnings(&checks);
    let facts = vec![
        "App-scoped leak verification is planning-only; no TUN, Mihomo runtime, or live exit/DNS probe is performed"
            .into(),
        format!(
            "evaluated {} recorded observation(s) against projected proxy/DNS/node-pool artifacts",
            summary.observation_count
        )
        .into(),
        match routing_intent {
            Some(intent) => format!("routing intent under verification is `{}`", intent_label(intent)).into(),
            None => "routing intent under verification is unavailable because the plan was rejected".into(),
        },
    ];

    AppRuntimeSessionLeakReport {
        session_id: session.session_id.clone(),
        app_id: session.app_id.clone(),
        status,
        reason,
        routing_intent,
        evaluation_summary: summary,
        checks,
        summary: leak_summary,
        facts,
        warnings,
    }
}

fn leak_proxy_check(
    routing_intent: Option<AppRoutingIntent>,
    session: &AppRuntimeSessionRecord,
    summary: &AppRuntimeSessionEvaluationSummary,
) -> AppRuntimeLeakCheck {
    let intent = routing_intent.unwrap_or(AppRoutingIntent::Direct);
    if !requires_node_pool(intent) {
        return leak_check(
            AppRuntimeLeakDimension::ProxyLeak,
            AppRuntimeLeakCheckStatus::NotApplicable,
            format!(
                "routing intent `{}` does not route through a proxy group",
                intent_label(intent)
            )
            .into(),
            vec!["direct or reject routing has no proxy tunnel to leak from".into()],
            Vec::new(),
        );
    }
    if session.projected_proxy_groups.is_empty() {
        return leak_check(
            AppRuntimeLeakDimension::ProxyLeak,
            AppRuntimeLeakCheckStatus::Warn,
            "proxy routing intent has no projected proxy group to verify traffic against".into(),
            Vec::new(),
            vec!["projected proxy groups are empty, so proxy routing cannot be confirmed".into()],
        );
    }
    if summary.observation_count == 0 {
        return leak_check(
            AppRuntimeLeakDimension::ProxyLeak,
            AppRuntimeLeakCheckStatus::Warn,
            "no observations recorded, so proxy routing has not been verified".into(),
            Vec::new(),
            vec!["record session observations before verifying proxy routing".into()],
        );
    }

    let direct_egress = summary
        .observed_chains
        .iter()
        .filter(|chain| is_builtin_outbound(chain))
        .cloned()
        .collect::<Vec<_>>();
    if summary.mismatch_observations > 0 || !direct_egress.is_empty() {
        let mut warnings = Vec::new();
        if summary.mismatch_observations > 0 {
            warnings.push("connection metadata matched the app but not its projected proxy group or rules".into());
        }
        if !direct_egress.is_empty() {
            warnings.push(
                format!(
                    "observed direct or reject egress chains: {}",
                    join_chain_list(&direct_egress)
                )
                .into(),
            );
        }
        return leak_check(
            AppRuntimeLeakDimension::ProxyLeak,
            AppRuntimeLeakCheckStatus::Fail,
            "app traffic appears to bypass the projected proxy group (possible proxy leak)".into(),
            Vec::new(),
            warnings,
        );
    }
    if summary.matched_observations == 0 {
        return leak_check(
            AppRuntimeLeakDimension::ProxyLeak,
            AppRuntimeLeakCheckStatus::Warn,
            "observations were recorded but none were attributed to the app's projected proxy group".into(),
            Vec::new(),
            vec!["no attribution candidates matched the projected proxy group or rules".into()],
        );
    }
    if summary.unattributed_observations > 0 || summary.stale_observations > 0 {
        return leak_check(
            AppRuntimeLeakDimension::ProxyLeak,
            AppRuntimeLeakCheckStatus::Warn,
            "some observations were unattributed or stale, so proxy routing is only partially verified".into(),
            vec![
                format!(
                    "{} matched, {} unattributed, {} stale observation(s)",
                    summary.matched_observations, summary.unattributed_observations, summary.stale_observations
                )
                .into(),
            ],
            vec!["partial observation coverage prevents a fully confirmed proxy routing result".into()],
        );
    }

    leak_check(
        AppRuntimeLeakDimension::ProxyLeak,
        AppRuntimeLeakCheckStatus::Pass,
        format!(
            "all {} attributed observation(s) traversed the projected proxy group(s)",
            summary.matched_observations
        )
        .into(),
        vec![format!("observed proxy chains: {}", join_chain_list(&summary.observed_chains)).into()],
        Vec::new(),
    )
}

fn leak_dns_check(plan: &AppRuntimePlan) -> AppRuntimeLeakCheck {
    let planning_fact: String = "DNS leak verification is planning-only; no live DNS query is issued".into();
    let require_dns = plan
        .security_profile
        .as_ref()
        .map(|profile| profile.controls.require_dns_profile)
        .unwrap_or(false);
    let Some(dns) = plan.dns_profile.as_ref() else {
        if require_dns {
            return leak_check(
                AppRuntimeLeakDimension::DnsLeak,
                AppRuntimeLeakCheckStatus::Fail,
                "security profile requires a DNS profile but none is bound; DNS may leak to the system resolver".into(),
                vec![planning_fact],
                vec!["bind a DNS profile to satisfy the security profile and prevent DNS leaks".into()],
            );
        }
        return leak_check(
            AppRuntimeLeakDimension::DnsLeak,
            AppRuntimeLeakCheckStatus::Warn,
            "no DNS profile is bound; app DNS queries may fall back to the system resolver".into(),
            vec![planning_fact],
            vec!["bind a DNS profile to route app DNS through the tunnel".into()],
        );
    };
    if dns.resolver_plan.status != DnsResolverPlanStatus::Ready {
        return leak_check(
            AppRuntimeLeakDimension::DnsLeak,
            AppRuntimeLeakCheckStatus::Warn,
            format!(
                "DNS profile `{}` resolver plan is `{:?}`; tunneled DNS resolution is not confirmed",
                dns.profile_id, dns.resolver_plan.status
            )
            .into(),
            vec![planning_fact],
            vec![
                format!(
                    "review DNS profile `{}` so its resolver plan becomes ready",
                    dns.profile_id
                )
                .into(),
            ],
        );
    }
    let runtime_supported = dns
        .resolver_plan
        .nameservers
        .iter()
        .filter(|nameserver| nameserver.runtime_supported)
        .count();
    if runtime_supported == 0 {
        return leak_check(
            AppRuntimeLeakDimension::DnsLeak,
            AppRuntimeLeakCheckStatus::Warn,
            format!(
                "DNS profile `{}` has no runtime-supported nameservers; DNS may fall back to the system resolver",
                dns.profile_id
            )
            .into(),
            vec![planning_fact],
            vec!["add at least one runtime-supported nameserver to reduce DNS leak risk".into()],
        );
    }

    leak_check(
        AppRuntimeLeakDimension::DnsLeak,
        AppRuntimeLeakCheckStatus::Pass,
        format!(
            "DNS profile `{}` provides {runtime_supported} runtime-supported nameserver(s) for leak-resistant resolution",
            dns.profile_id
        )
        .into(),
        vec![planning_fact],
        Vec::new(),
    )
}

fn leak_exit_check(
    session: &AppRuntimeSessionRecord,
    plan: &AppRuntimePlan,
    routing_intent: Option<AppRoutingIntent>,
    summary: &AppRuntimeSessionEvaluationSummary,
) -> AppRuntimeLeakCheck {
    let planning_fact: String = "exit verification is planning-only; no real exit IP is fetched".into();
    if matches!(
        session.status,
        AppRuntimeSessionStatus::Blocked | AppRuntimeSessionStatus::Failed
    ) || plan.status == AppRuntimePlanStatus::Rejected
    {
        return leak_check(
            AppRuntimeLeakDimension::ExitVerification,
            AppRuntimeLeakCheckStatus::Fail,
            "session was blocked or failed, so exit verification cannot be planned".into(),
            vec![planning_fact],
            vec!["resolve session diagnostics before planning exit verification".into()],
        );
    }
    let intent = routing_intent.unwrap_or(AppRoutingIntent::Direct);
    if !requires_node_pool(intent) {
        return leak_check(
            AppRuntimeLeakDimension::ExitVerification,
            AppRuntimeLeakCheckStatus::NotApplicable,
            format!(
                "routing intent `{}` exits directly, so there is no proxy exit to verify",
                intent_label(intent)
            )
            .into(),
            vec![planning_fact],
            Vec::new(),
        );
    }
    let has_candidates = plan
        .node_pool
        .as_ref()
        .map(|pool| pool.candidate_count > 0)
        .unwrap_or(false);
    if !has_candidates {
        return leak_check(
            AppRuntimeLeakDimension::ExitVerification,
            AppRuntimeLeakCheckStatus::Warn,
            "no node pool candidates are available, so the exit verification target is undefined".into(),
            vec![planning_fact],
            vec!["bind a node pool with candidates to define an exit verification target".into()],
        );
    }
    if summary.observation_count == 0 {
        return leak_check(
            AppRuntimeLeakDimension::ExitVerification,
            AppRuntimeLeakCheckStatus::Warn,
            "no observations recorded, so exit verification readiness is not established".into(),
            vec![planning_fact],
            vec!["record observations before planning exit verification".into()],
        );
    }
    if summary.matched_observations == 0 {
        return leak_check(
            AppRuntimeLeakDimension::ExitVerification,
            AppRuntimeLeakCheckStatus::Warn,
            "no attributed observations, so exit verification cannot target a confirmed connection".into(),
            vec![planning_fact],
            vec!["attribute at least one observation to the app before exit verification".into()],
        );
    }

    leak_check(
        AppRuntimeLeakDimension::ExitVerification,
        AppRuntimeLeakCheckStatus::Pass,
        format!(
            "exit verification can be planned against {} attributed observation(s)",
            summary.matched_observations
        )
        .into(),
        vec![planning_fact],
        Vec::new(),
    )
}

fn leak_node_pool_check(
    routing_intent: Option<AppRoutingIntent>,
    plan: &AppRuntimePlan,
    session: &AppRuntimeSessionRecord,
    summary: &AppRuntimeSessionEvaluationSummary,
) -> AppRuntimeLeakCheck {
    let intent = routing_intent.unwrap_or(AppRoutingIntent::Direct);
    if !requires_node_pool(intent) {
        return leak_check(
            AppRuntimeLeakDimension::NodePoolConsistency,
            AppRuntimeLeakCheckStatus::NotApplicable,
            format!("routing intent `{}` does not use a node pool", intent_label(intent)).into(),
            Vec::new(),
            Vec::new(),
        );
    }
    let Some(pool) = plan.node_pool.as_ref() else {
        return leak_check(
            AppRuntimeLeakDimension::NodePoolConsistency,
            AppRuntimeLeakCheckStatus::Fail,
            "proxy routing intent has no node pool, so node-pool consistency cannot be verified".into(),
            Vec::new(),
            vec!["bind a node pool so observed proxy chains can be validated".into()],
        );
    };
    if summary.observation_count == 0 {
        return leak_check(
            AppRuntimeLeakDimension::NodePoolConsistency,
            AppRuntimeLeakCheckStatus::Warn,
            format!(
                "no observations recorded to compare against node pool `{}`",
                pool.pool_id
            )
            .into(),
            Vec::new(),
            vec!["record observations to verify proxy chains stay within the node pool".into()],
        );
    }
    if summary.matched_observations == 0 {
        return leak_check(
            AppRuntimeLeakDimension::NodePoolConsistency,
            AppRuntimeLeakCheckStatus::Warn,
            "no attributed observations, so node-pool consistency cannot be checked".into(),
            Vec::new(),
            vec!["attribute observations to the app before checking node-pool consistency".into()],
        );
    }

    let mut expected = BTreeSet::new();
    for candidate in &pool.candidates {
        expected.insert(candidate.node_name.clone());
    }
    for group in &session.projected_proxy_groups {
        expected.insert(group.clone());
    }
    let unexpected = summary
        .observed_chains
        .iter()
        .filter(|chain| !expected.contains(*chain) && !is_builtin_outbound(chain))
        .cloned()
        .collect::<Vec<_>>();
    if unexpected.is_empty() {
        return leak_check(
            AppRuntimeLeakDimension::NodePoolConsistency,
            AppRuntimeLeakCheckStatus::Pass,
            format!(
                "all observed proxy chains belong to node pool `{}` or its projected proxy group",
                pool.pool_id
            )
            .into(),
            vec![
                format!(
                    "node pool `{}` declares {} candidate(s)",
                    pool.pool_id, pool.candidate_count
                )
                .into(),
            ],
            Vec::new(),
        );
    }

    leak_check(
        AppRuntimeLeakDimension::NodePoolConsistency,
        AppRuntimeLeakCheckStatus::Warn,
        format!(
            "observed proxy chain(s) {} are not declared in node pool `{}`",
            join_chain_list(&unexpected),
            pool.pool_id
        )
        .into(),
        Vec::new(),
        vec!["verify selector or group membership, or update the node pool to include observed nodes".into()],
    )
}

fn leak_check(
    dimension: AppRuntimeLeakDimension,
    status: AppRuntimeLeakCheckStatus,
    message: String,
    facts: Vec<String>,
    warnings: Vec<String>,
) -> AppRuntimeLeakCheck {
    AppRuntimeLeakCheck {
        dimension,
        severity: leak_severity(status),
        status,
        message,
        facts,
        warnings,
    }
}

fn leak_severity(status: AppRuntimeLeakCheckStatus) -> AppRuntimeDiagnosticSeverity {
    match status {
        AppRuntimeLeakCheckStatus::Pass | AppRuntimeLeakCheckStatus::NotApplicable => {
            AppRuntimeDiagnosticSeverity::Info
        }
        AppRuntimeLeakCheckStatus::Warn => AppRuntimeDiagnosticSeverity::Warning,
        AppRuntimeLeakCheckStatus::Fail => AppRuntimeDiagnosticSeverity::Error,
    }
}

fn leak_summary_counts(checks: &[AppRuntimeLeakCheck]) -> AppRuntimeLeakSummary {
    let mut summary = AppRuntimeLeakSummary::default();
    for check in checks {
        match check.status {
            AppRuntimeLeakCheckStatus::Pass => summary.pass += 1,
            AppRuntimeLeakCheckStatus::Warn => summary.warn += 1,
            AppRuntimeLeakCheckStatus::Fail => summary.fail += 1,
            AppRuntimeLeakCheckStatus::NotApplicable => summary.not_applicable += 1,
        }
    }
    summary
}

fn leak_status(summary: &AppRuntimeLeakSummary) -> AppRuntimeDiagnosticStatus {
    if summary.fail > 0 {
        AppRuntimeDiagnosticStatus::Blocked
    } else if summary.warn > 0 {
        AppRuntimeDiagnosticStatus::Degraded
    } else {
        AppRuntimeDiagnosticStatus::Healthy
    }
}

fn leak_reason(status: AppRuntimeDiagnosticStatus, summary: &AppRuntimeLeakSummary) -> String {
    match status {
        AppRuntimeDiagnosticStatus::Healthy => {
            "app session shows no proxy, DNS, exit, or node-pool leak indicators in recorded observations".into()
        }
        AppRuntimeDiagnosticStatus::Degraded => format!(
            "{} leak verification check(s) need attention before exit or leak verification can be confirmed",
            summary.warn
        )
        .into(),
        AppRuntimeDiagnosticStatus::Blocked => format!(
            "{} leak verification check(s) failed; resolve before performing exit or leak verification",
            summary.fail
        )
        .into(),
    }
}

fn leak_report_warnings(checks: &[AppRuntimeLeakCheck]) -> Vec<String> {
    let mut seen = BTreeSet::new();
    let mut warnings = Vec::new();
    for check in checks {
        if matches!(
            check.status,
            AppRuntimeLeakCheckStatus::Warn | AppRuntimeLeakCheckStatus::Fail
        ) && seen.insert(check.message.clone())
        {
            warnings.push(check.message.clone());
        }
        for warning in &check.warnings {
            if seen.insert(warning.clone()) {
                warnings.push(warning.clone());
            }
        }
    }
    warnings
}

fn intent_label(intent: AppRoutingIntent) -> String {
    format!("{intent:?}").to_ascii_lowercase().into()
}

fn join_chain_list(items: &[String]) -> String {
    if items.is_empty() {
        return "none".into();
    }
    items
        .iter()
        .map(|item| item.as_str())
        .collect::<Vec<_>>()
        .join(", ")
        .into()
}

fn is_builtin_outbound(chain: &str) -> bool {
    matches!(
        chain.to_ascii_uppercase().as_str(),
        "DIRECT" | "REJECT" | "REJECT-DROP" | "PASS" | "COMPATIBLE"
    )
}

fn validate_app(entry: &AppRegistryEntry) -> Result<()> {
    normalize_id(&entry.app_id, "app_id")?;
    ensure_non_empty(&entry.name, "name")?;
    ensure_unique_matchers(&entry.process_matchers)?;
    ensure_env_keys(&entry.env)?;
    Ok(())
}

fn validate_node_pool(node_pool: &NodePool) -> Result<()> {
    normalize_id(&node_pool.pool_id, "pool_id")?;
    ensure_non_empty(&node_pool.name, "name")?;
    for candidate in &node_pool.candidate_nodes {
        ensure_non_empty(&candidate.node_name, "candidate node_name")?;
    }
    Ok(())
}

fn validate_dns_profile(dns_profile: &DnsProfile) -> Result<()> {
    normalize_id(&dns_profile.profile_id, "profile_id")?;
    ensure_non_empty(&dns_profile.name, "name")?;
    ensure_non_empty(&dns_profile.config_yaml, "config_yaml")?;
    build_dns_resolver_plan(&dns_profile.config_yaml)?;
    Ok(())
}

fn validate_security_profile(security_profile: &SecurityProfile) -> Result<()> {
    normalize_id(&security_profile.profile_id, "profile_id")?;
    ensure_non_empty(&security_profile.name, "name")?;
    let mut seen = BTreeSet::new();
    for intent in &security_profile.controls.allowed_routing_intents {
        if !seen.insert(format!("{intent:?}")) {
            bail!("duplicate allowed routing intent `{intent:?}`");
        }
    }
    Ok(())
}

fn validate_policy_binding(binding: &AppPolicyBinding) -> Result<()> {
    normalize_id(&binding.binding_id, "binding_id")?;
    normalize_id(&binding.app_id, "app_id")?;
    if let Some(pool_id) = binding.node_pool_id.as_ref() {
        normalize_id(pool_id, "node_pool_id")?;
    }
    if let Some(profile_id) = binding.dns_profile_id.as_ref() {
        normalize_id(profile_id, "dns_profile_id")?;
    }
    if let Some(profile_id) = binding.security_profile_id.as_ref() {
        normalize_id(profile_id, "security_profile_id")?;
    }
    Ok(())
}

fn normalize_id(value: &str, field: &str) -> Result<String> {
    let value = value.trim();
    if value.is_empty() {
        bail!("{field} is required");
    }
    if !value
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.'))
    {
        bail!("{field} may only contain ASCII letters, numbers, '.', '_' or '-'");
    }
    Ok(value.into())
}

fn ensure_non_empty(value: &str, field: &str) -> Result<()> {
    if value.trim().is_empty() {
        bail!("{field} is required");
    }
    Ok(())
}

fn ensure_unique_matchers(matchers: &[AppProcessMatcher]) -> Result<()> {
    let mut seen = BTreeSet::new();
    for matcher in matchers {
        ensure_non_empty(&matcher.pattern, "process matcher pattern")?;
        let key = format!("{:?}:{}", matcher.kind, matcher.pattern);
        if !seen.insert(key) {
            bail!("duplicate process matcher");
        }
    }
    Ok(())
}

fn ensure_env_keys(env: &[AppEnvironmentVariable]) -> Result<()> {
    let mut seen = BTreeSet::new();
    for item in env {
        ensure_non_empty(&item.key, "env key")?;
        if !seen.insert(item.key.to_ascii_uppercase()) {
            bail!("duplicate env key `{}`", item.key);
        }
    }
    Ok(())
}

fn requires_node_pool(intent: AppRoutingIntent) -> bool {
    matches!(
        intent,
        AppRoutingIntent::Proxy | AppRoutingIntent::Auto | AppRoutingIntent::Fallback
    )
}

fn node_pool_plan_view(node_pool: NodePool) -> NodePoolPlanView {
    NodePoolPlanView {
        pool_id: node_pool.pool_id,
        name: node_pool.name,
        candidate_count: node_pool.candidate_nodes.len(),
        protocols: node_pool.protocols,
        tags: node_pool.tags,
        constraints: node_pool.health_constraints,
        candidates: node_pool.candidate_nodes,
    }
}

fn dns_profile_plan_view(dns_profile: DnsProfile) -> Result<DnsProfilePlanView> {
    let resolver_plan = build_dns_resolver_plan(&dns_profile.config_yaml)?;
    Ok(DnsProfilePlanView {
        profile_id: dns_profile.profile_id,
        name: dns_profile.name,
        test_domain: dns_profile.test_domain,
        tags: dns_profile.tags,
        resolver_plan,
    })
}

fn security_profile_plan_view(security_profile: SecurityProfile) -> SecurityProfilePlanView {
    SecurityProfilePlanView {
        profile_id: security_profile.profile_id,
        name: security_profile.name,
        controls: security_profile.controls,
        tags: security_profile.tags,
    }
}

fn append_node_pool_diagnostics(plan: &AppRuntimePlan, checks: &mut Vec<AppRuntimeDiagnosticCheck>) {
    let routing_intent = plan.routing_intent.unwrap_or(AppRoutingIntent::Direct);
    let requires_pool = requires_node_pool(routing_intent);
    checks.push(diagnostic_check(
        "node_pool_binding",
        AppRuntimeDiagnosticCategory::NodePool,
        match (requires_pool, plan.node_pool.as_ref()) {
            (true, Some(_)) => AppRuntimeDiagnosticCheckStatus::Passed,
            (true, None) => AppRuntimeDiagnosticCheckStatus::Failed,
            (false, _) => AppRuntimeDiagnosticCheckStatus::Skipped,
        },
        match (requires_pool, plan.node_pool.as_ref()) {
            (true, Some(pool)) => {
                format!("routing intent `{routing_intent:?}` uses node pool `{}`", pool.pool_id).into()
            }
            (true, None) => format!("routing intent `{routing_intent:?}` requires a node pool").into(),
            (false, _) => format!("routing intent `{routing_intent:?}` does not require a node pool").into(),
        },
        Vec::new(),
    ));

    checks.push(diagnostic_check(
        "node_pool_candidates",
        AppRuntimeDiagnosticCategory::NodePool,
        match plan.node_pool.as_ref() {
            Some(pool) if pool.candidate_count > 0 => AppRuntimeDiagnosticCheckStatus::Passed,
            Some(_) => AppRuntimeDiagnosticCheckStatus::Failed,
            None => AppRuntimeDiagnosticCheckStatus::Skipped,
        },
        match plan.node_pool.as_ref() {
            Some(pool) if pool.candidate_count > 0 => {
                format!("node pool `{}` has {} candidate(s)", pool.pool_id, pool.candidate_count).into()
            }
            Some(pool) => format!("node pool `{}` has no candidates", pool.pool_id).into(),
            None => "node pool candidate check skipped because no pool is bound".into(),
        },
        plan.node_pool
            .as_ref()
            .map(|pool| {
                pool.candidates
                    .iter()
                    .map(|candidate| candidate.node_name.clone())
                    .collect()
            })
            .unwrap_or_default(),
    ));
}

fn append_dns_diagnostics(plan: &AppRuntimePlan, checks: &mut Vec<AppRuntimeDiagnosticCheck>) {
    let dns_profile_id = plan
        .policy_binding
        .as_ref()
        .and_then(|binding| binding.dns_profile_id.clone());
    checks.push(diagnostic_check(
        "dns_profile_binding",
        AppRuntimeDiagnosticCategory::Dns,
        match (dns_profile_id.as_ref(), plan.dns_profile.as_ref()) {
            (Some(_), Some(profile)) if profile.resolver_plan.status == DnsResolverPlanStatus::Ready => {
                AppRuntimeDiagnosticCheckStatus::Passed
            }
            (Some(_), Some(_)) => AppRuntimeDiagnosticCheckStatus::Warning,
            (Some(_), None) => AppRuntimeDiagnosticCheckStatus::Failed,
            (None, _) => AppRuntimeDiagnosticCheckStatus::Skipped,
        },
        match (dns_profile_id.as_ref(), plan.dns_profile.as_ref()) {
            (Some(_), Some(profile)) => format!(
                "DNS profile `{}` resolver plan is `{:?}`",
                profile.profile_id, profile.resolver_plan.status
            )
            .into(),
            (Some(profile_id), None) => format!("DNS profile `{profile_id}` could not be resolved").into(),
            (None, _) => "DNS profile check skipped because no DNS profile is bound".into(),
        },
        plan.dns_profile
            .as_ref()
            .map(|profile| {
                profile
                    .resolver_plan
                    .nameservers
                    .iter()
                    .map(|nameserver| nameserver.server.as_str().into())
                    .collect()
            })
            .unwrap_or_default(),
    ));
}

fn append_security_diagnostics(plan: &AppRuntimePlan, checks: &mut Vec<AppRuntimeDiagnosticCheck>) {
    let security_profile_id = plan
        .policy_binding
        .as_ref()
        .and_then(|binding| binding.security_profile_id.clone());
    checks.push(diagnostic_check(
        "security_profile_binding",
        AppRuntimeDiagnosticCategory::Security,
        match (security_profile_id.as_ref(), plan.security_profile.as_ref()) {
            (Some(_), Some(_)) => AppRuntimeDiagnosticCheckStatus::Passed,
            (Some(_), None) => AppRuntimeDiagnosticCheckStatus::Failed,
            (None, _) => AppRuntimeDiagnosticCheckStatus::Skipped,
        },
        match (security_profile_id.as_ref(), plan.security_profile.as_ref()) {
            (Some(_), Some(profile)) => format!("security profile `{}` selected", profile.profile_id).into(),
            (Some(profile_id), None) => format!("security profile `{profile_id}` could not be resolved").into(),
            (None, _) => "security profile check skipped because no profile is bound".into(),
        },
        Vec::new(),
    ));

    let Some(profile) = plan.security_profile.as_ref() else {
        return;
    };

    if profile.controls.require_node_pool {
        checks.push(diagnostic_check(
            "security_requires_node_pool",
            AppRuntimeDiagnosticCategory::Security,
            if plan.node_pool.is_some() {
                AppRuntimeDiagnosticCheckStatus::Passed
            } else {
                AppRuntimeDiagnosticCheckStatus::Failed
            },
            if plan.node_pool.is_some() {
                format!(
                    "security profile `{}` requires and found a node pool",
                    profile.profile_id
                )
                .into()
            } else {
                format!("security profile `{}` requires a node pool", profile.profile_id).into()
            },
            Vec::new(),
        ));
    }

    if profile.controls.require_dns_profile {
        checks.push(diagnostic_check(
            "security_requires_dns_profile",
            AppRuntimeDiagnosticCategory::Security,
            if plan.dns_profile.is_some() {
                AppRuntimeDiagnosticCheckStatus::Passed
            } else {
                AppRuntimeDiagnosticCheckStatus::Failed
            },
            if plan.dns_profile.is_some() {
                format!(
                    "security profile `{}` requires and found a DNS profile",
                    profile.profile_id
                )
                .into()
            } else {
                format!("security profile `{}` requires a DNS profile", profile.profile_id).into()
            },
            Vec::new(),
        ));
    }

    if let Some(min_nameservers) = profile.controls.min_runtime_supported_nameservers {
        let supported = plan
            .dns_profile
            .as_ref()
            .map(|dns| {
                dns.resolver_plan
                    .nameservers
                    .iter()
                    .filter(|nameserver| nameserver.runtime_supported)
                    .count()
            })
            .unwrap_or_default();
        checks.push(diagnostic_check(
            "security_min_runtime_supported_nameservers",
            AppRuntimeDiagnosticCategory::Security,
            if supported >= min_nameservers {
                AppRuntimeDiagnosticCheckStatus::Passed
            } else {
                AppRuntimeDiagnosticCheckStatus::Failed
            },
            format!(
                "security profile `{}` requires {min_nameservers} runtime-supported DNS nameserver(s), found {supported}",
                profile.profile_id
            )
            .into(),
            Vec::new(),
        ));
    }

    if !profile.controls.allowed_routing_intents.is_empty() {
        let routing_intent = plan.routing_intent.unwrap_or(AppRoutingIntent::Direct);
        checks.push(diagnostic_check(
            "security_allowed_routing_intent",
            AppRuntimeDiagnosticCategory::Security,
            if profile.controls.allowed_routing_intents.contains(&routing_intent) {
                AppRuntimeDiagnosticCheckStatus::Passed
            } else {
                AppRuntimeDiagnosticCheckStatus::Failed
            },
            format!(
                "routing intent `{routing_intent:?}` checked against security profile `{}`",
                profile.profile_id
            )
            .into(),
            profile
                .controls
                .allowed_routing_intents
                .iter()
                .map(|intent| format!("{intent:?}").into())
                .collect(),
        ));
    }
}

fn append_projection_diagnostics(projection: &AppRuntimeMihomoProjection, checks: &mut Vec<AppRuntimeDiagnosticCheck>) {
    checks.push(diagnostic_check(
        "mihomo_projection_mutation",
        AppRuntimeDiagnosticCategory::Projection,
        if projection.mutates_runtime {
            AppRuntimeDiagnosticCheckStatus::Failed
        } else {
            AppRuntimeDiagnosticCheckStatus::Passed
        },
        if projection.mutates_runtime {
            "Mihomo projection would mutate runtime".into()
        } else {
            "Mihomo projection is preview-only".into()
        },
        Vec::new(),
    ));

    checks.push(diagnostic_check(
        "mihomo_projection_rules",
        AppRuntimeDiagnosticCategory::Projection,
        if projection.status == AppRuntimePlanStatus::Rejected {
            AppRuntimeDiagnosticCheckStatus::Skipped
        } else if projection.rules.is_empty() {
            AppRuntimeDiagnosticCheckStatus::Warning
        } else {
            AppRuntimeDiagnosticCheckStatus::Passed
        },
        if projection.status == AppRuntimePlanStatus::Rejected {
            "Mihomo rule projection skipped because runtime plan is rejected".into()
        } else if projection.rules.is_empty() {
            format!("app `{}` produced no Mihomo-compatible rules", projection.app_id).into()
        } else {
            format!("projected {} Mihomo rule(s)", projection.rules.len()).into()
        },
        projection.rules.iter().map(|rule| rule.rule.clone()).collect(),
    ));
}

fn diagnostic_check(
    check_id: &str,
    category: AppRuntimeDiagnosticCategory,
    status: AppRuntimeDiagnosticCheckStatus,
    message: String,
    details: Vec<String>,
) -> AppRuntimeDiagnosticCheck {
    AppRuntimeDiagnosticCheck {
        check_id: check_id.into(),
        category,
        severity: diagnostic_severity(status),
        status,
        message,
        details,
    }
}

fn validate_app_runtime_projection_artifact(
    plan: &AppRuntimePlan,
    projection: &AppRuntimeMihomoProjection,
    diagnostics: &AppRuntimeDiagnosticsReport,
) -> AppRuntimeProjectionValidationReport {
    let mut checks = Vec::new();

    checks.push(diagnostic_check(
        "artifact_plan_ready",
        AppRuntimeDiagnosticCategory::Projection,
        if plan.status == AppRuntimePlanStatus::Ready {
            AppRuntimeDiagnosticCheckStatus::Passed
        } else {
            AppRuntimeDiagnosticCheckStatus::Failed
        },
        if plan.status == AppRuntimePlanStatus::Ready {
            format!("runtime plan for `{}` is ready", plan.app_id).into()
        } else {
            format!("runtime plan for `{}` is rejected", plan.app_id).into()
        },
        vec![plan.reason.clone()],
    ));

    checks.push(diagnostic_check(
        "artifact_diagnostics_gate",
        AppRuntimeDiagnosticCategory::Projection,
        match diagnostics.status {
            AppRuntimeDiagnosticStatus::Healthy => AppRuntimeDiagnosticCheckStatus::Passed,
            AppRuntimeDiagnosticStatus::Degraded => AppRuntimeDiagnosticCheckStatus::Warning,
            AppRuntimeDiagnosticStatus::Blocked => AppRuntimeDiagnosticCheckStatus::Failed,
        },
        "projection artifact reuses app-runtime diagnostics gate".into(),
        vec![diagnostics.reason.clone()],
    ));

    checks.push(diagnostic_check(
        "artifact_yaml_patch_parse",
        AppRuntimeDiagnosticCategory::Projection,
        yaml_patch_validation_status(plan, projection),
        yaml_patch_validation_message(plan, projection),
        yaml_patch_validation_details(projection),
    ));

    checks.push(diagnostic_check(
        "artifact_rule_projection",
        AppRuntimeDiagnosticCategory::Projection,
        if plan.status == AppRuntimePlanStatus::Ready && projection.rules.is_empty() {
            AppRuntimeDiagnosticCheckStatus::Warning
        } else if plan.status == AppRuntimePlanStatus::Ready {
            AppRuntimeDiagnosticCheckStatus::Passed
        } else {
            AppRuntimeDiagnosticCheckStatus::Skipped
        },
        if projection.rules.is_empty() {
            "projection has no Mihomo-compatible app rule".into()
        } else {
            format!("projection contains {} Mihomo rule(s)", projection.rules.len()).into()
        },
        projection.rules.iter().map(|rule| rule.rule.clone()).collect(),
    ));

    checks.push(diagnostic_check(
        "artifact_runtime_boundary",
        AppRuntimeDiagnosticCategory::RuntimeBoundary,
        if projection.mutates_runtime {
            AppRuntimeDiagnosticCheckStatus::Failed
        } else {
            AppRuntimeDiagnosticCheckStatus::Passed
        },
        "projection artifact is staged and does not mutate Mihomo runtime".into(),
        vec!["activationMode=staged".into(), "mutatesRuntime=false".into()],
    ));

    let summary = diagnostics_summary(&checks);
    let status = diagnostics_status(&summary);
    let reason = diagnostics_reason(status, &summary);
    let warnings = projection_artifact_validation_warnings(&checks);
    let facts = vec![
        "Projection artifact validation is dry-run only".into(),
        "Active profile switch and Mihomo reload are outside this gate".into(),
    ];

    AppRuntimeProjectionValidationReport {
        status,
        reason,
        checks,
        summary,
        facts,
        warnings,
    }
}

fn app_runtime_projection_checksum(projection: &AppRuntimeMihomoProjection) -> String {
    let mut hasher = Sha256::new();
    hasher.update(projection.app_id.as_bytes());
    if let Some(session_id) = projection.session_id.as_ref() {
        hasher.update(session_id.as_bytes());
    }
    hasher.update(projection.yaml_patch.as_bytes());
    for rule in &projection.rules {
        hasher.update(rule.rule.as_bytes());
    }
    for group in &projection.proxy_groups {
        hasher.update(group.name.as_bytes());
        hasher.update(group.group_type.as_bytes());
        for proxy in &group.proxies {
            hasher.update(proxy.as_bytes());
        }
    }
    format!("{:x}", hasher.finalize()).into()
}

fn app_runtime_projection_artifact_path(artifact_id: &str) -> Result<std::path::PathBuf> {
    Ok(dirs::app_runtime_projection_artifacts_dir()?
        .join(safe_app_runtime_artifact_segment(artifact_id))
        .join("artifact.yaml"))
}

fn safe_app_runtime_artifact_segment(value: &str) -> String {
    let segment: std::string::String = value
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.') {
                ch
            } else {
                '-'
            }
        })
        .collect();
    let segment = segment.trim_matches('-');

    if segment.is_empty() {
        "artifact".into()
    } else {
        segment.into()
    }
}

fn app_runtime_activation_preflight_missing_artifact_report(
    request: AppRuntimeProjectionActivationPreflightRequest,
    storage_path: String,
    error: String,
) -> AppRuntimeProjectionActivationPreflightReport {
    let checks = vec![diagnostic_check(
        "activation_artifact_exists",
        AppRuntimeDiagnosticCategory::Projection,
        AppRuntimeDiagnosticCheckStatus::Failed,
        format!("projection artifact `{}` was not found", request.artifact_id).into(),
        vec![storage_path.clone(), error],
    )];
    app_runtime_activation_preflight_report(request.artifact_id, None, None, Some(storage_path), None, None, checks)
}

fn app_runtime_activation_preflight_report_from_yaml(
    request: &AppRuntimeProjectionActivationPreflightRequest,
    storage_path: String,
    raw_yaml: &str,
) -> AppRuntimeProjectionActivationPreflightReport {
    let mut checks = vec![diagnostic_check(
        "activation_artifact_exists",
        AppRuntimeDiagnosticCategory::Projection,
        AppRuntimeDiagnosticCheckStatus::Passed,
        format!("projection artifact `{}` is persisted", request.artifact_id).into(),
        vec![storage_path.clone()],
    )];
    let parsed = match serde_yaml_ng::from_str::<serde_yaml_ng::Value>(raw_yaml) {
        Ok(value) => value,
        Err(err) => {
            checks.push(diagnostic_check(
                "activation_artifact_parse",
                AppRuntimeDiagnosticCategory::Projection,
                AppRuntimeDiagnosticCheckStatus::Failed,
                "projection artifact YAML could not be parsed".into(),
                vec![err.to_string().into()],
            ));
            return app_runtime_activation_preflight_report(
                request.artifact_id.clone(),
                None,
                None,
                Some(storage_path),
                None,
                None,
                checks,
            );
        }
    };

    checks.push(diagnostic_check(
        "activation_artifact_parse",
        AppRuntimeDiagnosticCategory::Projection,
        AppRuntimeDiagnosticCheckStatus::Passed,
        "projection artifact YAML is parseable".into(),
        Vec::new(),
    ));

    let Some(mapping) = parsed.as_mapping() else {
        checks.push(diagnostic_check(
            "activation_artifact_shape",
            AppRuntimeDiagnosticCategory::Projection,
            AppRuntimeDiagnosticCheckStatus::Failed,
            "projection artifact YAML is not an object".into(),
            Vec::new(),
        ));
        return app_runtime_activation_preflight_report(
            request.artifact_id.clone(),
            None,
            None,
            Some(storage_path),
            None,
            None,
            checks,
        );
    };

    let artifact_id = yaml_string_field(mapping, "artifactId").unwrap_or_else(|| request.artifact_id.clone());
    let app_id = yaml_string_field(mapping, "appId");
    let checksum = yaml_string_field(mapping, "checksum");
    let activation_mode = yaml_string_field(mapping, "activationMode");
    let mutates_runtime = yaml_bool_field(mapping, "mutatesRuntime");
    let validation_status =
        yaml_mapping_field(mapping, "validation").and_then(|validation| yaml_string_field(validation, "status"));

    checks.push(diagnostic_check(
        "activation_artifact_id_match",
        AppRuntimeDiagnosticCategory::Projection,
        if artifact_id == request.artifact_id {
            AppRuntimeDiagnosticCheckStatus::Passed
        } else {
            AppRuntimeDiagnosticCheckStatus::Failed
        },
        if artifact_id == request.artifact_id {
            "persisted artifact id matches the requested artifact".into()
        } else {
            "persisted artifact id does not match the requested artifact".into()
        },
        vec![
            format!("requested={}", request.artifact_id).into(),
            format!("persisted={artifact_id}").into(),
        ],
    ));

    checks.push(diagnostic_check(
        "activation_checksum_match",
        AppRuntimeDiagnosticCategory::Projection,
        checksum_preflight_status(checksum.as_ref(), request.expected_checksum.as_ref()),
        checksum_preflight_message(checksum.as_ref(), request.expected_checksum.as_ref()),
        checksum_preflight_details(checksum.as_ref(), request.expected_checksum.as_ref()),
    ));

    checks.push(diagnostic_check(
        "activation_validation_gate",
        AppRuntimeDiagnosticCategory::Projection,
        validation_status_preflight_status(validation_status.as_deref()),
        validation_status_preflight_message(validation_status.as_deref()),
        validation_status
            .as_ref()
            .map(|status| vec![format!("validation.status={status}").into()])
            .unwrap_or_default(),
    ));

    let runtime_boundary_passed = activation_mode.as_deref() == Some("staged") && mutates_runtime == Some(false);
    checks.push(diagnostic_check(
        "activation_runtime_boundary",
        AppRuntimeDiagnosticCategory::RuntimeBoundary,
        if runtime_boundary_passed {
            AppRuntimeDiagnosticCheckStatus::Passed
        } else {
            AppRuntimeDiagnosticCheckStatus::Failed
        },
        "activation preflight requires staged artifact and mutatesRuntime=false".into(),
        vec![
            format!("activationMode={}", activation_mode.as_deref().unwrap_or("missing")).into(),
            format!(
                "mutatesRuntime={}",
                mutates_runtime
                    .map(|value| value.to_string())
                    .unwrap_or_else(|| "missing".into())
            )
            .into(),
        ],
    ));

    checks.push(diagnostic_check(
        "activation_executor_guard",
        AppRuntimeDiagnosticCategory::RuntimeBoundary,
        AppRuntimeDiagnosticCheckStatus::Failed,
        "controlled activation executor is not enabled in this preflight batch".into(),
        vec![
            "No Mihomo reload/restart was performed".into(),
            "Next activation PR must add runtime apply and rollback metadata before this guard can pass".into(),
        ],
    ));

    app_runtime_activation_preflight_report(
        artifact_id,
        app_id,
        checksum,
        Some(storage_path),
        activation_mode.and_then(|mode| {
            if mode == "staged" {
                Some(AppRuntimeProjectionActivationMode::Staged)
            } else {
                None
            }
        }),
        mutates_runtime,
        checks,
    )
}

fn app_runtime_activation_preflight_report(
    artifact_id: String,
    app_id: Option<String>,
    checksum: Option<String>,
    storage_path: Option<String>,
    activation_mode: Option<AppRuntimeProjectionActivationMode>,
    mutates_runtime: Option<bool>,
    checks: Vec<AppRuntimeDiagnosticCheck>,
) -> AppRuntimeProjectionActivationPreflightReport {
    let summary = diagnostics_summary(&checks);
    let status = diagnostics_status(&summary);
    let reason = diagnostics_reason(status, &summary);
    let warnings = projection_artifact_validation_warnings(&checks);
    let facts = vec![
        "Activation preflight reads a persisted Rust projection artifact".into(),
        "This command never reloads, restarts, or mutates Mihomo runtime".into(),
    ];

    AppRuntimeProjectionActivationPreflightReport {
        status,
        reason,
        artifact_id,
        app_id,
        checksum,
        storage_path,
        activation_mode,
        mutates_runtime,
        checks,
        summary,
        facts,
        warnings,
    }
}

fn yaml_value_for_key<'a>(mapping: &'a serde_yaml_ng::Mapping, key: &str) -> Option<&'a serde_yaml_ng::Value> {
    mapping.get(&serde_yaml_ng::Value::String(std::string::String::from(key)))
}

fn yaml_string_field(mapping: &serde_yaml_ng::Mapping, key: &str) -> Option<String> {
    yaml_value_for_key(mapping, key)
        .and_then(serde_yaml_ng::Value::as_str)
        .map(Into::into)
}

fn yaml_bool_field(mapping: &serde_yaml_ng::Mapping, key: &str) -> Option<bool> {
    yaml_value_for_key(mapping, key).and_then(serde_yaml_ng::Value::as_bool)
}

fn yaml_mapping_field<'a>(mapping: &'a serde_yaml_ng::Mapping, key: &str) -> Option<&'a serde_yaml_ng::Mapping> {
    yaml_value_for_key(mapping, key).and_then(serde_yaml_ng::Value::as_mapping)
}

fn checksum_preflight_status(
    checksum: Option<&String>,
    expected_checksum: Option<&String>,
) -> AppRuntimeDiagnosticCheckStatus {
    match (checksum, expected_checksum) {
        (Some(checksum), Some(expected_checksum)) if checksum == expected_checksum => {
            AppRuntimeDiagnosticCheckStatus::Passed
        }
        (Some(_), Some(_)) | (None, Some(_)) => AppRuntimeDiagnosticCheckStatus::Failed,
        (_, None) => AppRuntimeDiagnosticCheckStatus::Warning,
    }
}

fn checksum_preflight_message(checksum: Option<&String>, expected_checksum: Option<&String>) -> String {
    match (checksum, expected_checksum) {
        (Some(checksum), Some(expected_checksum)) if checksum == expected_checksum => {
            "persisted artifact checksum matches the selected artifact".into()
        }
        (Some(_), Some(_)) => "persisted artifact checksum differs from the selected artifact".into(),
        (None, Some(_)) => "persisted artifact is missing checksum".into(),
        (_, None) => "selected artifact checksum was not provided for comparison".into(),
    }
}

fn checksum_preflight_details(checksum: Option<&String>, expected_checksum: Option<&String>) -> Vec<String> {
    vec![
        format!("persisted={}", checksum.map(String::as_str).unwrap_or("missing")).into(),
        format!(
            "expected={}",
            expected_checksum.map(String::as_str).unwrap_or("missing")
        )
        .into(),
    ]
}

fn validation_status_preflight_status(status: Option<&str>) -> AppRuntimeDiagnosticCheckStatus {
    match status {
        Some("healthy") => AppRuntimeDiagnosticCheckStatus::Passed,
        Some("degraded") => AppRuntimeDiagnosticCheckStatus::Warning,
        Some("blocked") | None => AppRuntimeDiagnosticCheckStatus::Failed,
        Some(_) => AppRuntimeDiagnosticCheckStatus::Failed,
    }
}

fn validation_status_preflight_message(status: Option<&str>) -> String {
    match status {
        Some("healthy") => "artifact validation gate is healthy".into(),
        Some("degraded") => "artifact validation gate is degraded".into(),
        Some("blocked") => "artifact validation gate is blocked".into(),
        Some(status) => format!("artifact validation gate has unknown status `{status}`").into(),
        None => "artifact validation gate status is missing".into(),
    }
}

fn yaml_patch_validation_status(
    plan: &AppRuntimePlan,
    projection: &AppRuntimeMihomoProjection,
) -> AppRuntimeDiagnosticCheckStatus {
    if projection.yaml_patch.trim().is_empty() {
        return if plan.status == AppRuntimePlanStatus::Ready {
            AppRuntimeDiagnosticCheckStatus::Warning
        } else {
            AppRuntimeDiagnosticCheckStatus::Skipped
        };
    }

    match serde_yaml_ng::from_str::<serde_yaml_ng::Value>(&projection.yaml_patch) {
        Ok(_) => AppRuntimeDiagnosticCheckStatus::Passed,
        Err(_) => AppRuntimeDiagnosticCheckStatus::Failed,
    }
}

fn yaml_patch_validation_message(plan: &AppRuntimePlan, projection: &AppRuntimeMihomoProjection) -> String {
    if projection.yaml_patch.trim().is_empty() {
        return if plan.status == AppRuntimePlanStatus::Ready {
            "ready plan produced an empty YAML patch".into()
        } else {
            "YAML patch parse skipped for rejected plan".into()
        };
    }

    match serde_yaml_ng::from_str::<serde_yaml_ng::Value>(&projection.yaml_patch) {
        Ok(_) => "projection YAML patch parses successfully".into(),
        Err(error) => format!("projection YAML patch failed to parse: {error}").into(),
    }
}

fn yaml_patch_validation_details(projection: &AppRuntimeMihomoProjection) -> Vec<String> {
    if projection.yaml_patch.trim().is_empty() {
        return Vec::new();
    }

    vec![format!("checksum={}", app_runtime_projection_checksum(projection)).into()]
}

fn projection_artifact_validation_warnings(checks: &[AppRuntimeDiagnosticCheck]) -> Vec<String> {
    checks
        .iter()
        .filter(|check| {
            check.status == AppRuntimeDiagnosticCheckStatus::Warning
                || check.status == AppRuntimeDiagnosticCheckStatus::Failed
        })
        .map(|check| check.message.clone())
        .collect()
}

fn diagnostic_severity(status: AppRuntimeDiagnosticCheckStatus) -> AppRuntimeDiagnosticSeverity {
    match status {
        AppRuntimeDiagnosticCheckStatus::Passed | AppRuntimeDiagnosticCheckStatus::Skipped => {
            AppRuntimeDiagnosticSeverity::Info
        }
        AppRuntimeDiagnosticCheckStatus::Warning => AppRuntimeDiagnosticSeverity::Warning,
        AppRuntimeDiagnosticCheckStatus::Failed => AppRuntimeDiagnosticSeverity::Error,
    }
}

fn diagnostics_summary(checks: &[AppRuntimeDiagnosticCheck]) -> AppRuntimeDiagnosticsSummary {
    let mut summary = AppRuntimeDiagnosticsSummary::default();
    for check in checks {
        match check.status {
            AppRuntimeDiagnosticCheckStatus::Passed => summary.passed += 1,
            AppRuntimeDiagnosticCheckStatus::Warning => summary.warnings += 1,
            AppRuntimeDiagnosticCheckStatus::Failed => summary.failed += 1,
            AppRuntimeDiagnosticCheckStatus::Skipped => summary.skipped += 1,
        }
    }
    summary
}

fn diagnostics_status(summary: &AppRuntimeDiagnosticsSummary) -> AppRuntimeDiagnosticStatus {
    if summary.failed > 0 {
        AppRuntimeDiagnosticStatus::Blocked
    } else if summary.warnings > 0 {
        AppRuntimeDiagnosticStatus::Degraded
    } else {
        AppRuntimeDiagnosticStatus::Healthy
    }
}

fn diagnostics_reason(status: AppRuntimeDiagnosticStatus, summary: &AppRuntimeDiagnosticsSummary) -> String {
    match status {
        AppRuntimeDiagnosticStatus::Healthy => format!("{} diagnostic check(s) passed", summary.passed).into(),
        AppRuntimeDiagnosticStatus::Degraded => format!(
            "{} warning diagnostic check(s), {} passed",
            summary.warnings, summary.passed
        )
        .into(),
        AppRuntimeDiagnosticStatus::Blocked => format!(
            "{} failed diagnostic check(s), {} warning(s)",
            summary.failed, summary.warnings
        )
        .into(),
    }
}

fn combined_diagnostic_warnings(
    plan: &AppRuntimePlan,
    projection: &AppRuntimeMihomoProjection,
    checks: &[AppRuntimeDiagnosticCheck],
) -> Vec<String> {
    let mut seen = BTreeSet::new();
    let mut warnings = Vec::new();
    for warning in plan.warnings.iter().chain(projection.warnings.iter()) {
        if seen.insert(warning.clone()) {
            warnings.push(warning.clone());
        }
    }
    for check in checks.iter().filter(|check| {
        matches!(
            check.status,
            AppRuntimeDiagnosticCheckStatus::Warning | AppRuntimeDiagnosticCheckStatus::Failed
        )
    }) {
        if seen.insert(check.message.clone()) {
            warnings.push(check.message.clone());
        }
    }
    warnings
}

fn ready_projection_without_yaml(
    plan: AppRuntimePlan,
    facts: Vec<String>,
    warnings: Vec<String>,
) -> Result<AppRuntimeMihomoProjection> {
    Ok(AppRuntimeMihomoProjection {
        status: plan.status,
        reason: "runtime plan could not be projected to Mihomo YAML".into(),
        app_id: plan.app_id,
        session_id: plan.session_id,
        mutates_runtime: false,
        proxy_groups: Vec::new(),
        rules: Vec::new(),
        dns: None,
        yaml_patch: String::new(),
        facts,
        warnings,
    })
}

fn mihomo_target_for_plan(
    plan: &AppRuntimePlan,
    routing_intent: AppRoutingIntent,
    proxy_groups: &mut Vec<MihomoProxyGroupProjection>,
    warnings: &mut Vec<String>,
) -> Option<String> {
    match routing_intent {
        AppRoutingIntent::Direct => Some("DIRECT".into()),
        AppRoutingIntent::Reject => Some("REJECT".into()),
        AppRoutingIntent::Proxy | AppRoutingIntent::Auto | AppRoutingIntent::Fallback => {
            let Some(node_pool) = plan.node_pool.as_ref() else {
                warnings.push("Mihomo projection requires a node pool for proxy-like routing intents".into());
                return None;
            };
            let proxies = sorted_candidate_node_names(&node_pool.candidates);
            if proxies.is_empty() {
                warnings.push(format!("node pool `{}` has no Mihomo proxy candidates", node_pool.pool_id).into());
                return None;
            }
            let group = mihomo_proxy_group(&plan.app_id, routing_intent, proxies);
            let target = group.name.clone();
            proxy_groups.push(group);
            Some(target)
        }
    }
}

fn mihomo_proxy_group(
    app_id: &str,
    routing_intent: AppRoutingIntent,
    proxies: Vec<String>,
) -> MihomoProxyGroupProjection {
    let (group_type, url, interval) = match routing_intent {
        AppRoutingIntent::Auto => (
            "url-test",
            Some("https://www.gstatic.com/generate_204".into()),
            Some(300),
        ),
        AppRoutingIntent::Fallback => (
            "fallback",
            Some("https://www.gstatic.com/generate_204".into()),
            Some(300),
        ),
        _ => ("select", None, None),
    };

    MihomoProxyGroupProjection {
        name: format!("app-{app_id}").into(),
        group_type: group_type.into(),
        proxies,
        url,
        interval,
    }
}

fn sorted_candidate_node_names(candidates: &[NodePoolCandidate]) -> Vec<String> {
    let mut ordered = candidates.to_vec();
    ordered.sort_by(|left, right| {
        left.priority
            .unwrap_or(u32::MAX)
            .cmp(&right.priority.unwrap_or(u32::MAX))
            .then_with(|| left.node_name.cmp(&right.node_name))
    });

    let mut seen = BTreeSet::new();
    ordered
        .into_iter()
        .filter_map(|candidate| {
            let node_name = candidate.node_name.trim();
            if node_name.is_empty() || !seen.insert(node_name.to_owned()) {
                None
            } else {
                Some(node_name.into())
            }
        })
        .collect()
}

fn mihomo_rules_for_app(app: &AppRegistryEntry, target: &str, warnings: &mut Vec<String>) -> Vec<MihomoRuleProjection> {
    let mut rules = Vec::new();
    for matcher in &app.process_matchers {
        let Some(mihomo_matcher) = mihomo_matcher_kind(matcher.kind) else {
            warnings.push(
                format!(
                    "process matcher `{:?}` cannot be projected to a Mihomo rule",
                    matcher.kind
                )
                .into(),
            );
            continue;
        };
        let value = matcher.pattern.trim();
        if value.is_empty() {
            warnings.push(format!("process matcher `{mihomo_matcher}` has an empty pattern").into());
            continue;
        }
        if value.contains(',') {
            warnings.push(format!("process matcher `{mihomo_matcher}` contains ',' and cannot be projected").into());
            continue;
        }

        rules.push(MihomoRuleProjection {
            matcher: mihomo_matcher.into(),
            value: value.into(),
            target: target.into(),
            rule: format!("{mihomo_matcher},{value},{target}").into(),
        });
    }

    if rules.is_empty() {
        warnings.push(format!("app `{}` has no Mihomo-compatible process matchers", app.app_id).into());
    }

    rules
}

fn mihomo_matcher_kind(kind: AppProcessMatcherKind) -> Option<&'static str> {
    match kind {
        AppProcessMatcherKind::ProcessName => Some("PROCESS-NAME"),
        AppProcessMatcherKind::ProcessPath => Some("PROCESS-PATH"),
        AppProcessMatcherKind::ProcessNameRegex
        | AppProcessMatcherKind::ProcessPathRegex
        | AppProcessMatcherKind::BundleId => None,
    }
}

fn mihomo_dns_projection(profile: &DnsProfilePlanView) -> MihomoDnsProjection {
    MihomoDnsProjection {
        profile_id: profile.profile_id.clone(),
        name: profile.name.clone(),
        nameservers: profile
            .resolver_plan
            .nameservers
            .iter()
            .map(|nameserver| nameserver.server.as_str().into())
            .collect(),
        runtime_supported_nameservers: profile
            .resolver_plan
            .nameservers
            .iter()
            .filter(|nameserver| nameserver.runtime_supported)
            .count(),
    }
}

fn mihomo_yaml_patch(proxy_groups: &[MihomoProxyGroupProjection], rules: &[MihomoRuleProjection]) -> Result<String> {
    if proxy_groups.is_empty() && rules.is_empty() {
        return Ok(String::new());
    }

    Ok(serde_yaml_ng::to_string(&MihomoYamlPatch {
        proxy_groups: proxy_groups.to_vec(),
        rules: rules.iter().map(|rule| rule.rule.clone()).collect(),
    })?
    .into())
}

fn plan_reason(app: &AppRegistryEntry, binding: &AppPolicyBinding, node_pool: Option<&NodePool>) -> String {
    match (binding.routing_intent, node_pool) {
        (AppRoutingIntent::Direct, _) => format!("app `{}` will use direct routing intent", app.app_id).into(),
        (AppRoutingIntent::Reject, _) => format!("app `{}` will use reject routing intent", app.app_id).into(),
        (_, Some(pool)) => format!(
            "app `{}` will use `{}` routing intent with node pool `{}`",
            app.app_id,
            format!("{:?}", binding.routing_intent).to_ascii_lowercase(),
            pool.pool_id
        )
        .into(),
        _ => format!(
            "app `{}` has `{}` routing intent but no node pool",
            app.app_id,
            format!("{:?}", binding.routing_intent).to_ascii_lowercase()
        )
        .into(),
    }
}

fn default_enabled() -> bool {
    true
}

fn now_millis() -> i64 {
    Local::now().timestamp_millis()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plan_explain_uses_registered_app_policy_and_pool() {
        let state = AppRuntimeStateDocument {
            apps: vec![sample_app()],
            node_pools: vec![sample_pool()],
            dns_profiles: vec![sample_dns_profile()],
            security_profiles: vec![sample_security_profile()],
            policy_bindings: vec![sample_binding()],
            sessions: Vec::new(),
        };

        let plan = explain_app_runtime_plan(
            &state,
            AppRuntimePlanRequest {
                app_id: "browser".into(),
                session_id: Some("session-a".into()),
            },
        );

        assert_eq!(plan.status, AppRuntimePlanStatus::Ready);
        assert_eq!(plan.routing_intent, Some(AppRoutingIntent::Proxy));
        assert_eq!(plan.node_pool.as_ref().map(|pool| pool.candidate_count), Some(1));
        assert_eq!(
            plan.dns_profile
                .as_ref()
                .map(|profile| profile.resolver_plan.nameservers.len()),
            Some(1)
        );
        assert_eq!(
            plan.security_profile
                .as_ref()
                .map(|profile| profile.profile_id.as_str()),
            Some("strict")
        );
        assert!(!plan.projection.mutates_runtime);
    }

    #[test]
    fn plan_rejects_missing_policy_binding() {
        let state = AppRuntimeStateDocument {
            apps: vec![sample_app()],
            node_pools: vec![sample_pool()],
            dns_profiles: vec![sample_dns_profile()],
            security_profiles: vec![sample_security_profile()],
            policy_bindings: Vec::new(),
            sessions: Vec::new(),
        };

        let plan = explain_app_runtime_plan(
            &state,
            AppRuntimePlanRequest {
                app_id: "browser".into(),
                session_id: None,
            },
        );

        assert_eq!(plan.status, AppRuntimePlanStatus::Rejected);
        assert_eq!(plan.reason, "app `browser` has no enabled policy binding");
    }

    #[test]
    fn validation_rejects_duplicate_process_matchers() {
        let mut app = sample_app();
        app.process_matchers.push(AppProcessMatcher {
            kind: AppProcessMatcherKind::ProcessName,
            pattern: "browser.exe".into(),
        });

        assert!(validate_app(&app).is_err());
    }

    #[test]
    fn plan_warns_when_binding_references_missing_dns_profile() {
        let state = AppRuntimeStateDocument {
            apps: vec![sample_app()],
            node_pools: vec![sample_pool()],
            dns_profiles: Vec::new(),
            security_profiles: vec![sample_security_profile()],
            policy_bindings: vec![sample_binding()],
            sessions: Vec::new(),
        };

        let plan = explain_app_runtime_plan(
            &state,
            AppRuntimePlanRequest {
                app_id: "browser".into(),
                session_id: None,
            },
        );

        assert_eq!(plan.status, AppRuntimePlanStatus::Ready);
        assert!(plan.dns_profile.is_none());
        assert!(
            plan.warnings
                .iter()
                .any(|warning| warning.contains("missing dns_profile_id `default`"))
        );
    }

    #[test]
    fn diagnostics_report_combines_plan_projection_and_security_checks() {
        let state = AppRuntimeStateDocument {
            apps: vec![sample_app()],
            node_pools: vec![sample_pool()],
            dns_profiles: vec![sample_dns_profile()],
            security_profiles: vec![sample_security_profile()],
            policy_bindings: vec![sample_binding()],
            sessions: Vec::new(),
        };

        let report = diagnose_app_runtime(
            &state,
            AppRuntimePlanRequest {
                app_id: "browser".into(),
                session_id: Some("session-a".into()),
            },
        )
        .unwrap();

        assert_eq!(report.status, AppRuntimeDiagnosticStatus::Healthy);
        assert_eq!(report.summary.failed, 0);
        assert_eq!(report.summary.warnings, 0);
        assert_eq!(report.plan.status, AppRuntimePlanStatus::Ready);
        assert_eq!(report.mihomo_projection.rules.len(), 1);
        assert!(
            report
                .checks
                .iter()
                .any(|check| check.check_id == "security_requires_dns_profile"
                    && check.status == AppRuntimeDiagnosticCheckStatus::Passed)
        );
    }

    #[test]
    fn diagnostics_report_blocks_when_security_policy_is_not_satisfied() {
        let mut binding = sample_binding();
        binding.dns_profile_id = None;
        let state = AppRuntimeStateDocument {
            apps: vec![sample_app()],
            node_pools: vec![sample_pool()],
            dns_profiles: Vec::new(),
            security_profiles: vec![sample_security_profile()],
            policy_bindings: vec![binding],
            sessions: Vec::new(),
        };

        let report = diagnose_app_runtime(
            &state,
            AppRuntimePlanRequest {
                app_id: "browser".into(),
                session_id: None,
            },
        )
        .unwrap();

        assert_eq!(report.status, AppRuntimeDiagnosticStatus::Blocked);
        assert!(
            report
                .checks
                .iter()
                .any(|check| check.check_id == "security_requires_dns_profile"
                    && check.status == AppRuntimeDiagnosticCheckStatus::Failed)
        );
    }

    #[test]
    fn diagnostics_report_marks_projection_without_rules_as_degraded() {
        let mut app = sample_app();
        app.process_matchers = vec![AppProcessMatcher {
            kind: AppProcessMatcherKind::BundleId,
            pattern: "com.example.browser".into(),
        }];
        let state = AppRuntimeStateDocument {
            apps: vec![app],
            node_pools: vec![sample_pool()],
            dns_profiles: vec![sample_dns_profile()],
            security_profiles: vec![sample_security_profile()],
            policy_bindings: vec![sample_binding()],
            sessions: Vec::new(),
        };

        let report = diagnose_app_runtime(
            &state,
            AppRuntimePlanRequest {
                app_id: "browser".into(),
                session_id: None,
            },
        )
        .unwrap();

        assert_eq!(report.status, AppRuntimeDiagnosticStatus::Degraded);
        assert!(
            report
                .checks
                .iter()
                .any(|check| check.check_id == "mihomo_projection_rules"
                    && check.status == AppRuntimeDiagnosticCheckStatus::Warning)
        );
    }

    #[test]
    fn session_record_snapshots_diagnostics_without_runtime_mutation() {
        let state = AppRuntimeStateDocument {
            apps: vec![sample_app()],
            node_pools: vec![sample_pool()],
            dns_profiles: vec![sample_dns_profile()],
            security_profiles: vec![sample_security_profile()],
            policy_bindings: vec![sample_binding()],
            sessions: Vec::new(),
        };

        let report = diagnose_app_runtime(
            &state,
            AppRuntimePlanRequest {
                app_id: "browser".into(),
                session_id: Some("session-a".into()),
            },
        )
        .unwrap();
        let session = session_record_from_diagnostics("session-a".into(), &report);

        assert_eq!(session.session_id, "session-a");
        assert_eq!(session.status, AppRuntimeSessionStatus::Planned);
        assert_eq!(session.plan_status, AppRuntimePlanStatus::Ready);
        assert_eq!(session.diagnostics_status, AppRuntimeDiagnosticStatus::Healthy);
        assert_eq!(session.projected_rules, vec!["PROCESS-NAME,browser.exe,app-browser"]);
        assert_eq!(session.projected_proxy_groups, vec!["app-browser"]);
        assert!(session.ended_at.is_none());
    }

    #[test]
    fn session_record_marks_blocked_diagnostics_as_blocked() {
        let state = AppRuntimeStateDocument {
            apps: vec![sample_app()],
            node_pools: vec![sample_pool()],
            dns_profiles: vec![sample_dns_profile()],
            security_profiles: vec![sample_security_profile()],
            policy_bindings: Vec::new(),
            sessions: Vec::new(),
        };

        let report = diagnose_app_runtime(
            &state,
            AppRuntimePlanRequest {
                app_id: "browser".into(),
                session_id: Some("session-a".into()),
            },
        )
        .unwrap();
        let session = session_record_from_diagnostics("session-a".into(), &report);

        assert_eq!(session.status, AppRuntimeSessionStatus::Blocked);
        assert_eq!(session.plan_status, AppRuntimePlanStatus::Rejected);
        assert_eq!(session.diagnostics_status, AppRuntimeDiagnosticStatus::Blocked);
    }

    #[test]
    fn session_observation_snapshots_connection_metrics_without_app_attribution() {
        let state = AppRuntimeStateDocument {
            apps: vec![sample_app()],
            node_pools: vec![sample_pool()],
            dns_profiles: vec![sample_dns_profile()],
            security_profiles: vec![sample_security_profile()],
            policy_bindings: vec![sample_binding()],
            sessions: Vec::new(),
        };
        let report = diagnose_app_runtime(
            &state,
            AppRuntimePlanRequest {
                app_id: "browser".into(),
                session_id: Some("session-a".into()),
            },
        )
        .unwrap();
        let session = session_record_from_diagnostics("session-a".into(), &report);
        let metrics = ConnectionMetricsSnapshot {
            traffic: connection_metrics::TrafficSnapshot {
                upload_total: 100,
                download_total: 200,
                upload_speed: 10,
                download_speed: 20,
                active_connection_count: 2,
                closed_since_last: 1,
                memory: 42,
            },
            speeds: vec![connection_metrics::ConnectionSpeed {
                id: "conn-a".into(),
                cur_upload: 10,
                cur_download: 20,
            }],
            attribution_candidates: Vec::new(),
            stale: false,
        };

        let observation = session_observation_from_metrics(&session, &metrics);

        assert_eq!(observation.session_id, "session-a");
        assert_eq!(
            observation.source,
            AppRuntimeSessionObservationSource::ConnectionMetricsSnapshot
        );
        assert_eq!(
            observation.attribution_status,
            AppRuntimeSessionAttributionStatus::Unattributed
        );
        assert_eq!(observation.traffic.active_connection_count, 2);
        assert_eq!(observation.connection_speed_count, 1);
        assert!(
            observation
                .warnings
                .iter()
                .any(|warning| warning.contains("No connection attribution candidates"))
        );
    }

    #[test]
    fn session_observation_matches_connection_candidates_against_projected_rules() {
        let state = AppRuntimeStateDocument {
            apps: vec![sample_app()],
            node_pools: vec![sample_pool()],
            dns_profiles: vec![sample_dns_profile()],
            security_profiles: vec![sample_security_profile()],
            policy_bindings: vec![sample_binding()],
            sessions: Vec::new(),
        };
        let report = diagnose_app_runtime(
            &state,
            AppRuntimePlanRequest {
                app_id: "browser".into(),
                session_id: Some("session-a".into()),
            },
        )
        .unwrap();
        let session = session_record_from_diagnostics("session-a".into(), &report);
        let metrics = ConnectionMetricsSnapshot {
            traffic: connection_metrics::TrafficSnapshot {
                upload_total: 100,
                download_total: 200,
                upload_speed: 10,
                download_speed: 20,
                active_connection_count: 1,
                closed_since_last: 0,
                memory: 42,
            },
            speeds: Vec::new(),
            attribution_candidates: vec![connection_metrics::ConnectionAttributionCandidate {
                id: "conn-a".into(),
                process: "browser.exe".into(),
                process_path: "C:\\Program Files\\Browser\\browser.exe".into(),
                host: "example.com".into(),
                rule: "ProcessName".into(),
                rule_payload: "browser.exe".into(),
                chains: vec!["app-browser".into()],
                upload: 100,
                download: 200,
            }],
            stale: false,
        };

        let observation = session_observation_from_metrics(&session, &metrics);

        assert_eq!(
            observation.attribution_status,
            AppRuntimeSessionAttributionStatus::AppMatched
        );
        assert_eq!(observation.attribution_candidates.len(), 1);
        assert!(
            observation.attribution_candidates[0]
                .matched_by
                .iter()
                .any(|matched_by| matched_by == "proxyGroup:app-browser")
        );
        assert!(
            observation.attribution_candidates[0]
                .matched_by
                .iter()
                .any(|matched_by| matched_by.starts_with("projectedRule:PROCESS-NAME,browser.exe"))
        );
    }

    #[test]
    fn session_evaluation_summarizes_matched_observations_as_healthy() {
        let state = AppRuntimeStateDocument {
            apps: vec![sample_app()],
            node_pools: vec![sample_pool()],
            dns_profiles: vec![sample_dns_profile()],
            security_profiles: vec![sample_security_profile()],
            policy_bindings: vec![sample_binding()],
            sessions: Vec::new(),
        };
        let report = diagnose_app_runtime(
            &state,
            AppRuntimePlanRequest {
                app_id: "browser".into(),
                session_id: Some("session-a".into()),
            },
        )
        .unwrap();
        let mut session = session_record_from_diagnostics("session-a".into(), &report);
        let metrics = ConnectionMetricsSnapshot {
            traffic: connection_metrics::TrafficSnapshot {
                upload_total: 100,
                download_total: 200,
                upload_speed: 10,
                download_speed: 20,
                active_connection_count: 1,
                closed_since_last: 0,
                memory: 42,
            },
            speeds: Vec::new(),
            attribution_candidates: vec![connection_metrics::ConnectionAttributionCandidate {
                id: "conn-a".into(),
                process: "browser.exe".into(),
                process_path: "C:\\Program Files\\Browser\\browser.exe".into(),
                host: "example.com".into(),
                rule: "ProcessName".into(),
                rule_payload: "browser.exe".into(),
                chains: vec!["app-browser".into()],
                upload: 100,
                download: 200,
            }],
            stale: false,
        };
        session
            .observations
            .push(session_observation_from_metrics(&session, &metrics));

        let evaluation = evaluation_report_from_session(&session);

        assert_eq!(evaluation.status, AppRuntimeDiagnosticStatus::Healthy);
        assert_eq!(evaluation.summary.observation_count, 1);
        assert_eq!(evaluation.summary.matched_observations, 1);
        assert_eq!(evaluation.summary.attribution_candidate_count, 1);
        assert_eq!(evaluation.summary.observed_chains, vec!["app-browser"]);
        assert_eq!(evaluation.summary.observed_hosts, vec!["example.com"]);
        assert!(evaluation.warnings.is_empty());
    }

    #[test]
    fn session_evaluation_marks_missing_observations_as_degraded() {
        let state = AppRuntimeStateDocument {
            apps: vec![sample_app()],
            node_pools: vec![sample_pool()],
            dns_profiles: vec![sample_dns_profile()],
            security_profiles: vec![sample_security_profile()],
            policy_bindings: vec![sample_binding()],
            sessions: Vec::new(),
        };
        let report = diagnose_app_runtime(
            &state,
            AppRuntimePlanRequest {
                app_id: "browser".into(),
                session_id: Some("session-a".into()),
            },
        )
        .unwrap();
        let session = session_record_from_diagnostics("session-a".into(), &report);

        let evaluation = evaluation_report_from_session(&session);

        assert_eq!(evaluation.status, AppRuntimeDiagnosticStatus::Degraded);
        assert_eq!(evaluation.summary.observation_count, 0);
        assert!(
            evaluation
                .warnings
                .iter()
                .any(|warning| warning.contains("no connection metrics observations"))
        );
    }

    #[test]
    fn session_leak_verification_reports_healthy_for_matched_proxy_session() {
        let state = sample_state();
        let mut session = planned_session(&state);
        let metrics = candidate_metrics(vec![attribution_candidate("browser.exe", vec!["app-browser"])]);
        session
            .observations
            .push(session_observation_from_metrics(&session, &metrics));

        let report = leak_report_from_session(&state, &session);

        assert_eq!(report.status, AppRuntimeDiagnosticStatus::Healthy);
        assert_eq!(report.routing_intent, Some(AppRoutingIntent::Proxy));
        assert_eq!(report.summary.fail, 0);
        assert_eq!(report.summary.warn, 0);
        assert_eq!(report.summary.pass, 4);
        assert!(report.warnings.is_empty());
        assert!(report.checks.iter().all(|check| matches!(
            check.status,
            AppRuntimeLeakCheckStatus::Pass | AppRuntimeLeakCheckStatus::NotApplicable
        )));
        assert!(report.facts.iter().any(|fact| fact.contains("planning-only")));
    }

    #[test]
    fn session_leak_verification_flags_missing_observations_as_degraded() {
        let state = sample_state();
        let session = planned_session(&state);

        let report = leak_report_from_session(&state, &session);

        assert_eq!(report.status, AppRuntimeDiagnosticStatus::Degraded);
        assert_eq!(report.summary.fail, 0);
        assert!(report.summary.warn >= 1);
        assert_eq!(
            leak_check_status(&report, AppRuntimeLeakDimension::DnsLeak),
            AppRuntimeLeakCheckStatus::Pass
        );
        assert_eq!(
            leak_check_status(&report, AppRuntimeLeakDimension::ProxyLeak),
            AppRuntimeLeakCheckStatus::Warn
        );
        assert_eq!(
            leak_check_status(&report, AppRuntimeLeakDimension::ExitVerification),
            AppRuntimeLeakCheckStatus::Warn
        );
    }

    #[test]
    fn session_leak_verification_fails_on_proxy_mismatch() {
        let state = sample_state();
        let mut session = planned_session(&state);
        let metrics = candidate_metrics(vec![attribution_candidate("other.exe", vec!["other-group"])]);
        let observation = session_observation_from_metrics(&session, &metrics);
        assert_eq!(
            observation.attribution_status,
            AppRuntimeSessionAttributionStatus::AppMismatch
        );
        session.observations.push(observation);

        let report = leak_report_from_session(&state, &session);

        assert_eq!(report.status, AppRuntimeDiagnosticStatus::Blocked);
        assert_eq!(
            leak_check_status(&report, AppRuntimeLeakDimension::ProxyLeak),
            AppRuntimeLeakCheckStatus::Fail
        );
        assert!(report.warnings.iter().any(|warning| warning.contains("proxy leak")));
    }

    #[test]
    fn session_leak_verification_warns_on_node_pool_inconsistency() {
        let state = sample_state();
        let mut session = planned_session(&state);
        let metrics = candidate_metrics(vec![attribution_candidate(
            "browser.exe",
            vec!["app-browser", "rogue-node"],
        )]);
        session
            .observations
            .push(session_observation_from_metrics(&session, &metrics));

        let report = leak_report_from_session(&state, &session);

        assert_eq!(report.status, AppRuntimeDiagnosticStatus::Degraded);
        assert_eq!(
            leak_check_status(&report, AppRuntimeLeakDimension::ProxyLeak),
            AppRuntimeLeakCheckStatus::Pass
        );
        assert_eq!(
            leak_check_status(&report, AppRuntimeLeakDimension::NodePoolConsistency),
            AppRuntimeLeakCheckStatus::Warn
        );
        assert!(report.warnings.iter().any(|warning| warning.contains("rogue-node")));
    }

    #[test]
    fn session_leak_verification_warns_for_direct_routing_without_dns_profile() {
        let mut binding = sample_binding();
        binding.node_pool_id = None;
        binding.dns_profile_id = None;
        binding.security_profile_id = None;
        binding.routing_intent = AppRoutingIntent::Direct;
        let state = AppRuntimeStateDocument {
            apps: vec![sample_app()],
            node_pools: vec![sample_pool()],
            dns_profiles: vec![sample_dns_profile()],
            security_profiles: vec![sample_security_profile()],
            policy_bindings: vec![binding],
            sessions: Vec::new(),
        };
        let session = planned_session(&state);

        let report = leak_report_from_session(&state, &session);

        assert_eq!(report.status, AppRuntimeDiagnosticStatus::Degraded);
        assert_eq!(report.routing_intent, Some(AppRoutingIntent::Direct));
        assert_eq!(
            leak_check_status(&report, AppRuntimeLeakDimension::ProxyLeak),
            AppRuntimeLeakCheckStatus::NotApplicable
        );
        assert_eq!(
            leak_check_status(&report, AppRuntimeLeakDimension::ExitVerification),
            AppRuntimeLeakCheckStatus::NotApplicable
        );
        assert_eq!(
            leak_check_status(&report, AppRuntimeLeakDimension::DnsLeak),
            AppRuntimeLeakCheckStatus::Warn
        );
        assert!(
            report
                .warnings
                .iter()
                .any(|warning| warning.contains("system resolver"))
        );
    }

    #[test]
    fn mihomo_projection_emits_process_rule_proxy_group_and_yaml_patch() {
        let state = AppRuntimeStateDocument {
            apps: vec![sample_app()],
            node_pools: vec![sample_pool()],
            dns_profiles: vec![sample_dns_profile()],
            security_profiles: vec![sample_security_profile()],
            policy_bindings: vec![sample_binding()],
            sessions: Vec::new(),
        };

        let projection = project_app_runtime_plan_to_mihomo(
            &state,
            AppRuntimePlanRequest {
                app_id: "browser".into(),
                session_id: Some("session-a".into()),
            },
        )
        .unwrap();

        assert_eq!(projection.status, AppRuntimePlanStatus::Ready);
        assert!(!projection.mutates_runtime);
        assert_eq!(projection.proxy_groups.len(), 1);
        assert_eq!(projection.proxy_groups[0].name, "app-browser");
        assert_eq!(projection.proxy_groups[0].group_type, "select");
        assert_eq!(projection.proxy_groups[0].proxies, vec!["us-1"]);
        assert_eq!(projection.rules.len(), 1);
        assert_eq!(projection.rules[0].rule, "PROCESS-NAME,browser.exe,app-browser");
        assert_eq!(
            projection.dns.as_ref().map(|dns| dns.nameservers.clone()),
            Some(vec!["1.1.1.1".into()])
        );
        assert!(projection.yaml_patch.contains("proxy-groups:"));
        assert!(projection.yaml_patch.contains("PROCESS-NAME,browser.exe,app-browser"));
    }

    #[test]
    fn mihomo_projection_maps_direct_binding_without_proxy_group() {
        let mut binding = sample_binding();
        binding.node_pool_id = None;
        binding.routing_intent = AppRoutingIntent::Direct;
        let state = AppRuntimeStateDocument {
            apps: vec![sample_app()],
            node_pools: vec![sample_pool()],
            dns_profiles: vec![sample_dns_profile()],
            security_profiles: vec![sample_security_profile()],
            policy_bindings: vec![binding],
            sessions: Vec::new(),
        };

        let projection = project_app_runtime_plan_to_mihomo(
            &state,
            AppRuntimePlanRequest {
                app_id: "browser".into(),
                session_id: None,
            },
        )
        .unwrap();

        assert!(projection.proxy_groups.is_empty());
        assert_eq!(projection.rules[0].rule, "PROCESS-NAME,browser.exe,DIRECT");
        assert!(projection.yaml_patch.contains("PROCESS-NAME,browser.exe,DIRECT"));
    }

    #[test]
    fn mihomo_projection_warns_for_unsupported_matchers() {
        let mut app = sample_app();
        app.process_matchers = vec![AppProcessMatcher {
            kind: AppProcessMatcherKind::BundleId,
            pattern: "com.example.browser".into(),
        }];
        let state = AppRuntimeStateDocument {
            apps: vec![app],
            node_pools: vec![sample_pool()],
            dns_profiles: vec![sample_dns_profile()],
            security_profiles: vec![sample_security_profile()],
            policy_bindings: vec![sample_binding()],
            sessions: Vec::new(),
        };

        let projection = project_app_runtime_plan_to_mihomo(
            &state,
            AppRuntimePlanRequest {
                app_id: "browser".into(),
                session_id: None,
            },
        )
        .unwrap();

        assert!(projection.rules.is_empty());
        assert!(
            projection
                .warnings
                .iter()
                .any(|warning| warning.contains("cannot be projected to a Mihomo rule"))
        );
    }

    #[test]
    fn projection_artifact_is_staged_and_validates_yaml_patch() {
        let artifact = build_app_runtime_projection_artifact(
            &sample_state(),
            AppRuntimePlanRequest {
                app_id: "browser".into(),
                session_id: Some("session-a".into()),
            },
        )
        .unwrap();

        assert_eq!(artifact.activation_mode, AppRuntimeProjectionActivationMode::Staged);
        assert!(!artifact.mutates_runtime);
        assert_eq!(artifact.validation.status, AppRuntimeDiagnosticStatus::Healthy);
        assert_eq!(artifact.validation.summary.failed, 0);
        assert_eq!(artifact.projection.rules.len(), 1);
        assert_eq!(artifact.storage_path, None);
        assert!(artifact.artifact_id.starts_with("app-runtime-browser-"));
        assert_eq!(artifact.checksum.len(), 64);
        assert!(
            artifact
                .validation
                .checks
                .iter()
                .any(|check| check.check_id == "artifact_yaml_patch_parse"
                    && check.status == AppRuntimeDiagnosticCheckStatus::Passed)
        );
    }

    fn sample_state() -> AppRuntimeStateDocument {
        AppRuntimeStateDocument {
            apps: vec![sample_app()],
            node_pools: vec![sample_pool()],
            dns_profiles: vec![sample_dns_profile()],
            security_profiles: vec![sample_security_profile()],
            policy_bindings: vec![sample_binding()],
            sessions: Vec::new(),
        }
    }

    fn planned_session(state: &AppRuntimeStateDocument) -> AppRuntimeSessionRecord {
        let report = diagnose_app_runtime(
            state,
            AppRuntimePlanRequest {
                app_id: "browser".into(),
                session_id: Some("session-a".into()),
            },
        )
        .unwrap();
        session_record_from_diagnostics("session-a".into(), &report)
    }

    fn attribution_candidate(process: &str, chains: Vec<&str>) -> ConnectionAttributionCandidate {
        ConnectionAttributionCandidate {
            id: "conn-a".into(),
            process: process.into(),
            process_path: format!("C:\\Program Files\\App\\{process}").into(),
            host: "example.com".into(),
            rule: "ProcessName".into(),
            rule_payload: process.into(),
            chains: chains.into_iter().map(Into::into).collect(),
            upload: 100,
            download: 200,
        }
    }

    fn candidate_metrics(candidates: Vec<ConnectionAttributionCandidate>) -> ConnectionMetricsSnapshot {
        ConnectionMetricsSnapshot {
            traffic: connection_metrics::TrafficSnapshot {
                upload_total: 100,
                download_total: 200,
                upload_speed: 10,
                download_speed: 20,
                active_connection_count: 1,
                closed_since_last: 0,
                memory: 42,
            },
            speeds: Vec::new(),
            attribution_candidates: candidates,
            stale: false,
        }
    }

    fn leak_check_status(
        report: &AppRuntimeSessionLeakReport,
        dimension: AppRuntimeLeakDimension,
    ) -> AppRuntimeLeakCheckStatus {
        report
            .checks
            .iter()
            .find(|check| check.dimension == dimension)
            .map(|check| check.status)
            .expect("leak check dimension present")
    }

    #[test]
    fn activation_preflight_blocks_before_runtime_mutation() {
        let request = AppRuntimeProjectionActivationPreflightRequest {
            artifact_id: "app-runtime-browser-abc123".into(),
            expected_checksum: Some("checksum-a".into()),
        };
        let report = app_runtime_activation_preflight_report_from_yaml(
            &request,
            "app-runtime/artifacts/app-runtime-browser-abc123/artifact.yaml".into(),
            r#"
artifactId: app-runtime-browser-abc123
appId: browser
checksum: checksum-a
activationMode: staged
mutatesRuntime: false
validation:
  status: healthy
"#,
        );

        assert_eq!(report.status, AppRuntimeDiagnosticStatus::Blocked);
        assert_eq!(report.app_id, Some("browser".into()));
        assert_eq!(report.checksum, Some("checksum-a".into()));
        assert!(report.summary.passed >= 5);
        assert!(
            report
                .checks
                .iter()
                .any(|check| check.check_id == "activation_executor_guard"
                    && check.status == AppRuntimeDiagnosticCheckStatus::Failed)
        );
    }

    fn sample_app() -> AppRegistryEntry {
        AppRegistryEntry {
            app_id: "browser".into(),
            name: "Browser".into(),
            executable_path: Some("C:\\Program Files\\Browser\\browser.exe".into()),
            bundle_id: None,
            launch_args: Vec::new(),
            working_directory: None,
            env: Vec::new(),
            process_matchers: vec![AppProcessMatcher {
                kind: AppProcessMatcherKind::ProcessName,
                pattern: "browser.exe".into(),
            }],
            platform_metadata: BTreeMap::new(),
            tags: vec!["desktop".into()],
            updated_at: 1,
        }
    }

    fn sample_pool() -> NodePool {
        NodePool {
            pool_id: "premium-us".into(),
            name: "Premium US".into(),
            tags: vec!["stable".into()],
            region: Some("US".into()),
            protocols: vec!["trojan".into()],
            purpose: Some("streaming".into()),
            cost_tier: Some("paid".into()),
            health_constraints: NodePoolHealthConstraints {
                max_latency_ms: Some(300),
                require_alive: Some(true),
                min_available_nodes: Some(1),
            },
            candidate_nodes: vec![NodePoolCandidate {
                node_name: "us-1".into(),
                proxy_group: Some("Proxy".into()),
                protocol: Some("trojan".into()),
                region: Some("US".into()),
                tags: vec!["stable".into()],
                priority: Some(1),
            }],
            updated_at: 1,
        }
    }

    fn sample_dns_profile() -> DnsProfile {
        DnsProfile {
            profile_id: "default".into(),
            name: "Default DNS".into(),
            config_yaml: r#"
dns:
  enable: true
  nameserver:
    - 1.1.1.1
"#
            .into(),
            test_domain: Some("example.com".into()),
            tags: vec!["default".into()],
            updated_at: 1,
        }
    }

    fn sample_security_profile() -> SecurityProfile {
        SecurityProfile {
            profile_id: "strict".into(),
            name: "Strict App Runtime".into(),
            controls: SecurityProfileControls {
                require_node_pool: true,
                require_dns_profile: true,
                min_runtime_supported_nameservers: Some(1),
                allowed_routing_intents: vec![AppRoutingIntent::Proxy, AppRoutingIntent::Auto],
            },
            tags: vec!["strict".into()],
            updated_at: 1,
        }
    }

    fn sample_binding() -> AppPolicyBinding {
        AppPolicyBinding {
            binding_id: "browser-policy".into(),
            app_id: "browser".into(),
            node_pool_id: Some("premium-us".into()),
            dns_profile_id: Some("default".into()),
            security_profile_id: Some("strict".into()),
            routing_intent: AppRoutingIntent::Proxy,
            enabled: true,
            updated_at: 1,
        }
    }
}
