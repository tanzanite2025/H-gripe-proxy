use crate::{
    config::Config,
    core::dns_runtime::{DnsResolverPlanStatus, build_dns_resolver_plan},
    utils::{dirs, help},
};
mod handoff;
mod projection;
mod sessions;
mod types;

pub use handoff::{
    accept_app_runtime_dns_handoff, closeout_app_runtime_staged_activation_lifecycle,
    complete_app_runtime_control_plane, complete_app_runtime_staged_activation_lifecycle,
    decide_app_runtime_runtime_apply_boundary,
};
pub use projection::{
    activate_app_runtime_projection_artifact, apply_app_runtime_projection_artifact_to_runtime,
    build_app_runtime_projection_artifact, build_app_runtime_projection_runtime_post_apply_hold,
    closeout_app_runtime_projection_runtime_apply_verification, list_app_runtime_projection_runtime_apply_audits,
    list_app_runtime_projection_runtime_verification_closeouts, persist_app_runtime_projection_artifact,
    preflight_app_runtime_projection_activation, project_app_runtime_plan_to_mihomo,
    rollback_app_runtime_projection_activation, verify_app_runtime_projection_runtime_apply,
};
pub use sessions::{
    evaluate_app_runtime_session, finish_app_runtime_session, list_app_runtime_sessions,
    record_app_runtime_session_observation, start_app_runtime_session, verify_app_runtime_session_leak,
};
pub use types::*;

#[cfg(test)]
pub(super) use handoff::*;
#[cfg(test)]
pub(super) use projection::*;
#[cfg(test)]
pub(super) use sessions::*;

use anyhow::{Result, bail};
use chrono::Local;
use projection::{
    diagnostic_severity, projection_artifact_validation_warnings, yaml_patch_validation_details,
    yaml_patch_validation_message, yaml_patch_validation_status,
};
use smartstring::alias::String;
use std::{
    collections::{BTreeMap, BTreeSet},
    time::{SystemTime, UNIX_EPOCH},
};
use tokio::fs;

const RUST_ADAPTER_EGRESS_PARITY_ROLLBACK_FILE: &str = "rollback.yaml";
const RUST_ADAPTER_EGRESS_SUPPORTED_PROTOCOLS: [&str; 17] = [
    "http",
    "https",
    "socks",
    "socks5",
    "ss",
    "ssr",
    "vmess",
    "vless",
    "trojan",
    "hysteria",
    "hysteria2",
    "tuic",
    "wireguard",
    "relay",
    "selector",
    "url-test",
    "fallback",
];

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct RustAdapterEgressRollbackRecord {
    previous_patch_yaml: String,
    applied_patch_yaml: String,
    app_id: String,
    created_at_epoch_seconds: u64,
}

pub async fn read_app_runtime_state_document() -> Result<AppRuntimeStateDocument> {
    let path = dirs::app_runtime_state_path()?;
    if !tokio::fs::try_exists(&path).await.unwrap_or(false) {
        return Ok(AppRuntimeStateDocument::default());
    }
    help::read_yaml(&path).await
}

pub fn build_app_runtime_demo_seed_document() -> AppRuntimeStateDocument {
    let updated_at = now_millis();
    let mut platform_metadata = BTreeMap::new();
    platform_metadata.insert("seed".into(), "demo".into());
    AppRuntimeStateDocument {
        apps: vec![AppRegistryEntry {
            app_id: "demo-browser".into(),
            name: "Demo Browser".into(),
            executable_path: Some("C:\\Program Files\\Demo Browser\\browser.exe".into()),
            bundle_id: None,
            launch_args: Vec::new(),
            working_directory: None,
            env: Vec::new(),
            process_matchers: vec![AppProcessMatcher {
                kind: AppProcessMatcherKind::ProcessName,
                pattern: "browser.exe".into(),
            }],
            platform_metadata,
            tags: vec!["demo".into(), "browser".into()],
            updated_at,
        }],
        node_pools: vec![NodePool {
            pool_id: "demo-stable-proxy".into(),
            name: "Demo Stable Proxy".into(),
            tags: vec!["demo".into(), "stable".into()],
            region: Some("US".into()),
            protocols: vec!["trojan".into(), "vless".into()],
            purpose: Some("general".into()),
            cost_tier: Some("paid".into()),
            health_constraints: NodePoolHealthConstraints {
                max_latency_ms: Some(300),
                require_alive: Some(true),
                min_available_nodes: Some(1),
            },
            candidate_nodes: vec![NodePoolCandidate {
                node_name: "demo-us-1".into(),
                proxy_group: Some("Proxy".into()),
                protocol: Some("trojan".into()),
                region: Some("US".into()),
                tags: vec!["demo".into(), "stable".into()],
                priority: Some(1),
            }],
            updated_at,
        }],
        dns_profiles: vec![DnsProfile {
            profile_id: "demo-dns".into(),
            name: "Demo DNS".into(),
            config_yaml: r#"
dns:
  enable: true
  enhanced-mode: normal
  nameserver:
    - 1.1.1.1
"#
            .into(),
            test_domain: Some("example.com".into()),
            tags: vec!["demo".into()],
            updated_at,
        }],
        security_profiles: vec![SecurityProfile {
            profile_id: "demo-strict".into(),
            name: "Demo Strict App Runtime".into(),
            controls: SecurityProfileControls {
                require_node_pool: true,
                require_dns_profile: true,
                min_runtime_supported_nameservers: Some(1),
                allowed_routing_intents: vec![AppRoutingIntent::Proxy, AppRoutingIntent::Auto],
            },
            tags: vec!["demo".into(), "strict".into()],
            updated_at,
        }],
        policy_bindings: vec![AppPolicyBinding {
            binding_id: "demo-browser-policy".into(),
            app_id: "demo-browser".into(),
            node_pool_id: Some("demo-stable-proxy".into()),
            dns_profile_id: Some("demo-dns".into()),
            security_profile_id: Some("demo-strict".into()),
            routing_intent: AppRoutingIntent::Proxy,
            enabled: true,
            updated_at,
        }],
        sessions: Vec::new(),
        runtime_apply_audits: Vec::new(),
        active_projection: None,
    }
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

pub async fn rust_adapter_egress_parity(
    request: AppRuntimePlanRequest,
    explicit_opt_in: bool,
    apply_runtime: bool,
) -> Result<RustAdapterEgressParityReport> {
    let state = read_app_runtime_state_document().await?;
    let plan = explain_app_runtime_plan(&state, request.clone());
    let mihomo_projection = project_app_runtime_plan_to_mihomo(&state, request)?;
    let (decision, candidates) = rust_adapter_egress_decision(&plan, &mihomo_projection);
    let mut report = rust_adapter_egress_parity_report(
        plan,
        mihomo_projection,
        decision,
        candidates,
        explicit_opt_in,
        apply_runtime,
    );

    if apply_runtime && report.blockers.is_empty() {
        match apply_rust_adapter_egress_runtime_patch(&report.plan.app_id, &report.runtime_patch_yaml).await {
            Ok((previous_patch_yaml, rollback_record_path)) => {
                report.status = RustAdapterEgressParityStatus::Applied;
                report.reason = "Rust adapter egress runtime patch applied through the runtime config bridge".into();
                report.previous_patch_yaml = Some(previous_patch_yaml);
                report.rollback_record_path = Some(rollback_record_path);
                report.mutates_runtime = true;
                report.reload_mihomo = true;
            }
            Err(error) => {
                let message = error.to_string();
                report.status = RustAdapterEgressParityStatus::Blocked;
                report.reason = format!("Rust adapter egress runtime apply failed: {message}").into();
                report.blockers.push(message.clone().into());
                crate::core::runtime_snapshot::record_and_persist_runtime_lifecycle_event(
                    "rust_adapter_egress_parity_apply",
                    false,
                    Some(message),
                    None,
                );
            }
        }
    }

    Ok(report)
}

pub async fn rust_adapter_egress_parity_rollback() -> Result<RustAdapterEgressParityReport> {
    let rollback_record_path = rust_adapter_egress_rollback_record_path()?;
    let record_yaml = fs::read_to_string(&rollback_record_path)
        .await
        .map_err(|error| anyhow::anyhow!("failed to read {}: {}", rollback_record_path.display(), error))?;
    let record: RustAdapterEgressRollbackRecord = serde_yaml_ng::from_str(&record_yaml)?;
    let patch = runtime_adapter_egress_mapping(&record.previous_patch_yaml)?;
    Config::runtime().await.edit_draft(|draft| {
        draft.replace_adapter_egress_runtime_config(&patch);
    });
    crate::core::runtime_lifecycle::update_runtime_config_checked("adapter-egress-rollback").await?;
    crate::core::handle::Handle::refresh_clash();
    crate::core::runtime_snapshot::record_and_persist_runtime_lifecycle_event(
        "rust_adapter_egress_parity_rollback",
        true,
        None,
        Some("restored previous adapter egress runtime patch".into()),
    );

    let empty_state = AppRuntimeStateDocument::default();
    let request = AppRuntimePlanRequest {
        app_id: record.app_id,
        session_id: None,
    };
    let plan = explain_app_runtime_plan(&empty_state, request.clone());
    let projection = AppRuntimeMihomoProjection {
        status: AppRuntimePlanStatus::Rejected,
        reason: "rollback restored previous runtime adapter egress patch".into(),
        app_id: request.app_id.clone(),
        session_id: None,
        mutates_runtime: false,
        proxy_groups: Vec::new(),
        rules: Vec::new(),
        dns: None,
        yaml_patch: record.previous_patch_yaml.clone(),
        facts: Vec::new(),
        warnings: Vec::new(),
    };

    Ok(RustAdapterEgressParityReport {
        status: RustAdapterEgressParityStatus::Restored,
        reason: "previous adapter egress runtime patch restored".into(),
        plan,
        mihomo_projection: projection,
        decision: RustAdapterEgressDecision {
            target_kind: RustAdapterEgressTargetKind::MihomoFallback,
            target: "MIHOMO_FALLBACK".into(),
            selected_node: None,
            selected_protocol: None,
            candidate_count: 0,
            supported_candidate_count: 0,
            fallback_to_mihomo: true,
        },
        candidates: Vec::new(),
        runtime_patch_yaml: record.previous_patch_yaml.clone(),
        previous_patch_yaml: Some(record.previous_patch_yaml),
        rollback_record_path: Some(rollback_record_path.to_string_lossy().to_string().into()),
        explicit_opt_in: true,
        apply_runtime: true,
        mutates_runtime: true,
        reload_mihomo: true,
        mihomo_fallback_retained: true,
        blockers: Vec::new(),
        warnings: Vec::new(),
        facts: vec![
            "rollback uses the Rust-owned adapter egress rollback record".into(),
            "rollback restores proxy-groups/rules through the runtime config bridge".into(),
            "Mihomo remains the fallback runtime after rollback".into(),
        ],
    })
}

fn rust_adapter_egress_parity_report(
    plan: AppRuntimePlan,
    mihomo_projection: AppRuntimeMihomoProjection,
    decision: RustAdapterEgressDecision,
    candidates: Vec<RustAdapterEgressCandidateReport>,
    explicit_opt_in: bool,
    apply_runtime: bool,
) -> RustAdapterEgressParityReport {
    let mut blockers = Vec::new();
    let mut warnings = plan.warnings.clone();
    warnings.extend(mihomo_projection.warnings.clone());

    if plan.status != AppRuntimePlanStatus::Ready {
        blockers.push(plan.reason.clone());
    }
    if apply_runtime && !explicit_opt_in {
        blockers.push("runtime adapter egress parity apply requires explicit opt-in".into());
    }
    if apply_runtime && mihomo_projection.yaml_patch.trim().is_empty() {
        blockers.push("adapter egress projection produced no runtime patch".into());
    }
    if matches!(
        plan.routing_intent,
        Some(AppRoutingIntent::Proxy | AppRoutingIntent::Auto | AppRoutingIntent::Fallback)
    ) && decision.supported_candidate_count == 0
    {
        blockers.push("proxy-like adapter egress requires at least one supported candidate".into());
    }
    for candidate in candidates
        .iter()
        .filter(|candidate| candidate.status == RustAdapterEgressCandidateStatus::Unsupported)
    {
        warnings.push(
            format!(
                "candidate `{}` remains on Mihomo fallback: {}",
                candidate.node_name, candidate.reason
            )
            .into(),
        );
    }

    let status = if blockers.is_empty() {
        RustAdapterEgressParityStatus::Ready
    } else {
        RustAdapterEgressParityStatus::Blocked
    };
    let reason = match status {
        RustAdapterEgressParityStatus::Ready if apply_runtime => {
            "Rust adapter egress parity patch is ready for explicit opt-in apply"
        }
        RustAdapterEgressParityStatus::Ready => "Rust adapter egress parity patch is ready for shadow comparison",
        RustAdapterEgressParityStatus::Blocked => "Rust adapter egress parity is blocked",
        RustAdapterEgressParityStatus::Applied | RustAdapterEgressParityStatus::Restored => {
            "Rust adapter egress parity completed"
        }
    }
    .into();
    let facts = vec![
        "Rust app runtime state chooses the adapter egress target before Mihomo receives rules".into(),
        "Rust validates DIRECT/REJECT/proxy-group compatibility before opt-in apply".into(),
        "proxy-groups/rules are patched through a Rust runtime config bridge with rollback".into(),
        format!(
            "target={}; supportedCandidates={}/{}",
            decision.target, decision.supported_candidate_count, decision.candidate_count
        )
        .into(),
    ];

    RustAdapterEgressParityReport {
        status,
        reason,
        plan,
        mihomo_projection: mihomo_projection.clone(),
        decision,
        candidates,
        runtime_patch_yaml: mihomo_projection.yaml_patch.clone(),
        previous_patch_yaml: None,
        rollback_record_path: None,
        explicit_opt_in,
        apply_runtime,
        mutates_runtime: false,
        reload_mihomo: false,
        mihomo_fallback_retained: true,
        blockers,
        warnings,
        facts,
    }
}

fn rust_adapter_egress_decision(
    plan: &AppRuntimePlan,
    projection: &AppRuntimeMihomoProjection,
) -> (RustAdapterEgressDecision, Vec<RustAdapterEgressCandidateReport>) {
    let candidates = plan
        .node_pool
        .as_ref()
        .map(rust_adapter_egress_candidates)
        .unwrap_or_default();
    let supported_candidates = candidates
        .iter()
        .filter(|candidate| candidate.status == RustAdapterEgressCandidateStatus::Supported)
        .collect::<Vec<_>>();
    let selected = supported_candidates.first().copied();
    let fallback_to_mihomo = selected.is_none()
        && matches!(
            plan.routing_intent,
            Some(AppRoutingIntent::Proxy | AppRoutingIntent::Auto | AppRoutingIntent::Fallback)
        );

    let (target_kind, target) = match plan.routing_intent {
        Some(AppRoutingIntent::Direct) => (RustAdapterEgressTargetKind::Direct, "DIRECT".into()),
        Some(AppRoutingIntent::Reject) => (RustAdapterEgressTargetKind::Reject, "REJECT".into()),
        Some(AppRoutingIntent::Proxy | AppRoutingIntent::Auto | AppRoutingIntent::Fallback)
            if !projection.proxy_groups.is_empty() =>
        {
            (
                RustAdapterEgressTargetKind::ProxyGroup,
                projection.proxy_groups[0].name.clone(),
            )
        }
        _ => (RustAdapterEgressTargetKind::MihomoFallback, "MIHOMO_FALLBACK".into()),
    };

    (
        RustAdapterEgressDecision {
            target_kind,
            target,
            selected_node: selected.map(|candidate| candidate.node_name.clone()),
            selected_protocol: selected.and_then(|candidate| candidate.protocol.clone()),
            candidate_count: candidates.len(),
            supported_candidate_count: supported_candidates.len(),
            fallback_to_mihomo,
        },
        candidates,
    )
}

fn rust_adapter_egress_candidates(node_pool: &NodePoolPlanView) -> Vec<RustAdapterEgressCandidateReport> {
    let ordered_names = projection::sorted_candidate_node_names(&node_pool.candidates);
    ordered_names
        .into_iter()
        .filter_map(|node_name| {
            let candidate = node_pool
                .candidates
                .iter()
                .find(|candidate| candidate.node_name == node_name)?;
            let protocol = candidate.protocol.as_ref().map(|protocol| protocol.trim());
            let supported = protocol
                .map(|protocol| {
                    protocol.is_empty()
                        || RUST_ADAPTER_EGRESS_SUPPORTED_PROTOCOLS
                            .iter()
                            .any(|supported| protocol.eq_ignore_ascii_case(supported))
                })
                .unwrap_or(true);
            Some(RustAdapterEgressCandidateReport {
                node_name: candidate.node_name.clone(),
                proxy_group: candidate.proxy_group.clone(),
                protocol: candidate.protocol.clone(),
                priority: candidate.priority,
                status: if supported {
                    RustAdapterEgressCandidateStatus::Supported
                } else {
                    RustAdapterEgressCandidateStatus::Unsupported
                },
                reason: if supported {
                    "candidate can be selected by the Rust adapter egress subset".into()
                } else {
                    format!(
                        "protocol `{}` is outside the Rust adapter egress subset",
                        protocol.unwrap_or_default()
                    )
                    .into()
                },
            })
        })
        .collect()
}

async fn apply_rust_adapter_egress_runtime_patch(app_id: &str, patch_yaml: &str) -> Result<(String, String)> {
    let patch = runtime_adapter_egress_mapping(patch_yaml)?;
    let previous_patch = current_runtime_adapter_egress_patch().await;
    let previous_patch_yaml = adapter_egress_mapping_to_yaml(&previous_patch)?;
    let rollback_record_path = rust_adapter_egress_rollback_record_path()?;
    let record = RustAdapterEgressRollbackRecord {
        previous_patch_yaml: previous_patch_yaml.clone(),
        applied_patch_yaml: patch_yaml.into(),
        app_id: app_id.into(),
        created_at_epoch_seconds: rust_adapter_egress_epoch_seconds(),
    };
    if let Some(parent) = rollback_record_path.parent() {
        fs::create_dir_all(parent).await?;
    }
    fs::write(&rollback_record_path, serde_yaml_ng::to_string(&record)?.as_bytes()).await?;

    Config::runtime().await.edit_draft(|draft| {
        draft.patch_adapter_egress_runtime_config(&patch);
    });
    crate::core::runtime_lifecycle::update_runtime_config_checked("adapter-egress-apply").await?;
    crate::core::handle::Handle::refresh_clash();
    crate::core::runtime_snapshot::record_and_persist_runtime_lifecycle_event(
        "rust_adapter_egress_parity_apply",
        true,
        None,
        Some(format!("applied Rust adapter egress patch for app `{app_id}`").into()),
    );
    Ok((
        previous_patch_yaml,
        rollback_record_path.to_string_lossy().to_string().into(),
    ))
}

async fn current_runtime_adapter_egress_patch() -> serde_yaml_ng::Mapping {
    let runtime = Config::runtime().await;
    let config = runtime.latest_arc().config.clone();
    let mut patch = serde_yaml_ng::Mapping::new();
    for key in ["proxy-groups", "rules"] {
        if let Some(value) = config.as_ref().and_then(|config| config.get(key)).cloned() {
            patch.insert(key.into(), value);
        } else {
            patch.insert(key.into(), serde_yaml_ng::Value::Null);
        }
    }
    patch
}

fn runtime_adapter_egress_mapping(yaml: &str) -> Result<serde_yaml_ng::Mapping> {
    if yaml.trim().is_empty() {
        return Ok(serde_yaml_ng::Mapping::new());
    }
    let value = serde_yaml_ng::from_str::<serde_yaml_ng::Value>(yaml)?;
    value
        .as_mapping()
        .cloned()
        .ok_or_else(|| anyhow::anyhow!("adapter egress runtime patch root must be a mapping"))
}

fn adapter_egress_mapping_to_yaml(mapping: &serde_yaml_ng::Mapping) -> Result<String> {
    Ok(serde_yaml_ng::to_string(&serde_yaml_ng::Value::Mapping(mapping.clone()))?.into())
}

fn rust_adapter_egress_rollback_record_path() -> Result<std::path::PathBuf> {
    Ok(dirs::app_runtime_dir()?
        .join("rust-adapter-egress-parity")
        .join(RUST_ADAPTER_EGRESS_PARITY_ROLLBACK_FILE))
}

fn rust_adapter_egress_epoch_seconds() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or_default()
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

fn now_millis() -> i64 {
    Local::now().timestamp_millis()
}

#[cfg(test)]
mod tests;
