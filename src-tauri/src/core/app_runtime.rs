use crate::{
    core::dns_runtime::{DnsResolverPlan, build_dns_resolver_plan},
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
        "Rust AppRuntimeStateDocument is the only source of app/pool/dns-profile/policy facts".into(),
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

    if requires_node_pool(binding.routing_intent) && node_pool.is_none() {
        warnings.push(format!("routing intent `{:?}` has no node pool", binding.routing_intent).into());
    }
    if let Some(profile_id) = binding.dns_profile_id.as_ref()
        && dns_profile.is_none()
    {
        warnings.push(format!("policy binding references missing dns_profile_id `{profile_id}`").into());
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
        routing_intent: Some(binding.routing_intent),
        projection,
        facts,
        warnings,
    }
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

fn validate_policy_binding(binding: &AppPolicyBinding) -> Result<()> {
    normalize_id(&binding.binding_id, "binding_id")?;
    normalize_id(&binding.app_id, "app_id")?;
    if let Some(pool_id) = binding.node_pool_id.as_ref() {
        normalize_id(pool_id, "node_pool_id")?;
    }
    if let Some(profile_id) = binding.dns_profile_id.as_ref() {
        normalize_id(profile_id, "dns_profile_id")?;
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
        assert!(!plan.projection.mutates_runtime);
    }

    #[test]
    fn plan_rejects_missing_policy_binding() {
        let state = AppRuntimeStateDocument {
            apps: vec![sample_app()],
            node_pools: vec![sample_pool()],
            dns_profiles: vec![sample_dns_profile()],
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
