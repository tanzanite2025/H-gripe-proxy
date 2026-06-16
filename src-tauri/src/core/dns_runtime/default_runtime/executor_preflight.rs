use super::*;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum DnsDefaultRuntimeExecutorPreflightStatus {
    Ready,
    Blocked,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DnsDefaultRuntimeMutationDiff {
    pub previous_runtime: String,
    pub candidate_runtime: String,
    pub runtime_owner_before: String,
    pub runtime_owner_after: String,
    pub nameserver_targets: Vec<String>,
    pub plan_only_features: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DnsDefaultRuntimeExecutorAuditRecord {
    pub event_id: String,
    pub action: String,
    pub dry_run: bool,
    pub created_at_epoch_seconds: u64,
    pub guard_status: DnsDefaultRuntimeOptInSwitchGuardStatus,
    pub readiness_status: DnsDefaultRuntimeReadinessStatus,
    pub shadow_status: DnsDefaultRuntimeShadowEvidenceStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DnsDefaultRuntimeExecutorRollbackMarker {
    pub required: bool,
    pub prepared: bool,
    pub strategy: String,
    pub restores_runtime: bool,
    pub previous_runtime: String,
    pub candidate_runtime: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DnsDefaultRuntimeOptInExecutorPreflightReport {
    pub status: DnsDefaultRuntimeExecutorPreflightStatus,
    pub reason: String,
    pub guard: DnsDefaultRuntimeOptInSwitchGuardReport,
    pub mutation_diff: DnsDefaultRuntimeMutationDiff,
    pub audit_record: DnsDefaultRuntimeExecutorAuditRecord,
    pub rollback_marker: DnsDefaultRuntimeExecutorRollbackMarker,
    pub dry_run: bool,
    pub would_mutate_runtime: bool,
    pub executed: bool,
    pub reload_mihomo: bool,
    pub blockers: Vec<String>,
    pub warnings: Vec<String>,
    pub facts: Vec<String>,
}

pub async fn dns_default_runtime_opt_in_executor_preflight(
    yaml: Option<String>,
    domain: Option<String>,
    explicit_opt_in: bool,
) -> Result<DnsDefaultRuntimeOptInExecutorPreflightReport> {
    let guard = dns_default_runtime_opt_in_switch_guard(yaml, domain, explicit_opt_in).await?;
    Ok(build_dns_default_runtime_opt_in_executor_preflight_report(guard))
}

pub fn build_dns_default_runtime_opt_in_executor_preflight_report(
    guard: DnsDefaultRuntimeOptInSwitchGuardReport,
) -> DnsDefaultRuntimeOptInExecutorPreflightReport {
    let mutation_diff = default_runtime_mutation_diff(&guard);
    let audit_record = default_runtime_executor_audit_record(&guard);
    let rollback_marker = default_runtime_executor_rollback_marker(&guard);
    let mut blockers = guard.blockers.clone();
    let mut warnings = guard.warnings.clone();

    if guard.status != DnsDefaultRuntimeOptInSwitchGuardStatus::Ready {
        blockers.push("opt-in switch guard is not ready; executor preflight cannot proceed".into());
    }
    if !rollback_marker.prepared {
        blockers.push("executor rollback marker is not prepared".into());
    }
    if guard.shadow_evidence.status == DnsDefaultRuntimeShadowEvidenceStatus::Mismatched {
        warnings.push("executor preflight is dry-run only because shadow evidence is mismatched".into());
    }

    let status = if blockers.is_empty() {
        DnsDefaultRuntimeExecutorPreflightStatus::Ready
    } else {
        DnsDefaultRuntimeExecutorPreflightStatus::Blocked
    };
    let reason = default_runtime_executor_preflight_reason(status, &blockers);
    let facts = vec![
        "executor preflight is dry-run only".into(),
        "executor preflight does not write active profile".into(),
        "executor preflight does not reload Mihomo".into(),
        format!("audit event={}", audit_record.event_id),
    ];

    DnsDefaultRuntimeOptInExecutorPreflightReport {
        status,
        reason,
        guard,
        mutation_diff,
        audit_record,
        rollback_marker,
        dry_run: true,
        would_mutate_runtime: true,
        executed: false,
        reload_mihomo: false,
        blockers,
        warnings,
        facts,
    }
}

fn default_runtime_mutation_diff(guard: &DnsDefaultRuntimeOptInSwitchGuardReport) -> DnsDefaultRuntimeMutationDiff {
    let plan = &guard.readiness.plan;
    DnsDefaultRuntimeMutationDiff {
        previous_runtime: guard.rollback_plan.previous_runtime.clone(),
        candidate_runtime: guard.rollback_plan.candidate_runtime.clone(),
        runtime_owner_before: "mihomo".into(),
        runtime_owner_after: "rust".into(),
        nameserver_targets: plan
            .nameservers
            .iter()
            .filter(|server| server.runtime_supported)
            .map(|server| server.server.clone())
            .collect(),
        plan_only_features: default_runtime_plan_only_features(plan),
    }
}

fn default_runtime_plan_only_features(plan: &DnsResolverPlan) -> Vec<String> {
    let mut features = Vec::new();
    if plan.runtime_projection.fake_ip.configured && !plan.runtime_projection.fake_ip.runtime_applied {
        features.push("fake-ip".into());
    }
    if plan.runtime_projection.fallback_filter.configured && !plan.runtime_projection.fallback_filter.runtime_applied {
        features.push("fallback-filter".into());
    }
    if plan.runtime_projection.nameserver_policy.configured
        && !plan.runtime_projection.nameserver_policy.runtime_applied
    {
        features.push("nameserver-policy".into());
    }
    features
}

fn default_runtime_executor_audit_record(
    guard: &DnsDefaultRuntimeOptInSwitchGuardReport,
) -> DnsDefaultRuntimeExecutorAuditRecord {
    let created_at_epoch_seconds = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    DnsDefaultRuntimeExecutorAuditRecord {
        event_id: format!("dns-default-runtime-executor-preflight-{created_at_epoch_seconds}"),
        action: "defaultDnsRuntimeOptInExecutorPreflight".into(),
        dry_run: true,
        created_at_epoch_seconds,
        guard_status: guard.status,
        readiness_status: guard.readiness.status,
        shadow_status: guard.shadow_evidence.status,
    }
}

fn default_runtime_executor_rollback_marker(
    guard: &DnsDefaultRuntimeOptInSwitchGuardReport,
) -> DnsDefaultRuntimeExecutorRollbackMarker {
    DnsDefaultRuntimeExecutorRollbackMarker {
        required: guard.rollback_plan.required,
        prepared: guard.rollback_plan.supported,
        strategy: guard.rollback_plan.strategy.clone(),
        restores_runtime: true,
        previous_runtime: guard.rollback_plan.previous_runtime.clone(),
        candidate_runtime: guard.rollback_plan.candidate_runtime.clone(),
    }
}

fn default_runtime_executor_preflight_reason(
    status: DnsDefaultRuntimeExecutorPreflightStatus,
    blockers: &[String],
) -> String {
    match status {
        DnsDefaultRuntimeExecutorPreflightStatus::Ready => {
            "default DNS runtime executor preflight passed; dry-run only".into()
        }
        DnsDefaultRuntimeExecutorPreflightStatus::Blocked => blockers
            .first()
            .cloned()
            .unwrap_or_else(|| "default DNS runtime executor preflight is blocked".into()),
    }
}
