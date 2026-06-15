use crate::{
    core::dns_runtime::{DnsResolverPlan, DnsResolverPlanStatus, build_dns_resolver_plan},
    utils::{dirs, help},
};
use anyhow::{Result, bail};
use chrono::Local;
use serde::{Deserialize, Serialize};
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

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
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

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
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

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
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

#[derive(Debug, Clone, Default, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AppRuntimeDiagnosticsSummary {
    pub passed: usize,
    pub warnings: usize,
    pub failed: usize,
    pub skipped: usize,
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
    fn mihomo_projection_emits_process_rule_proxy_group_and_yaml_patch() {
        let state = AppRuntimeStateDocument {
            apps: vec![sample_app()],
            node_pools: vec![sample_pool()],
            dns_profiles: vec![sample_dns_profile()],
            security_profiles: vec![sample_security_profile()],
            policy_bindings: vec![sample_binding()],
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
