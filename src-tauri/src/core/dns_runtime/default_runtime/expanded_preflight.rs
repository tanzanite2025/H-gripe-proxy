use super::*;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum DnsDefaultRuntimeExpandedOptInExecutionPreflightStatus {
    Ready,
    Blocked,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DnsDefaultRuntimeExpandedRuntimeMutationPlan {
    pub previous_runtime: String,
    pub candidate_runtime: String,
    pub execution_mode: String,
    pub active_profile_write: bool,
    pub mihomo_reload: bool,
    pub profile_source: String,
    pub rollback_strategy: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DnsDefaultRuntimeExpandedOptInExecutionPreflightRecord {
    pub event_id: String,
    pub gate_status: DnsDefaultRuntimeExpandedOptInExecutionGateStatus,
    pub scope_name: String,
    pub mutation_plan: DnsDefaultRuntimeExpandedRuntimeMutationPlan,
    pub created_at_epoch_seconds: u64,
    pub explicit_opt_in: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DnsDefaultRuntimeExpandedOptInExecutionPreflightReport {
    pub status: DnsDefaultRuntimeExpandedOptInExecutionPreflightStatus,
    pub reason: String,
    pub gate: DnsDefaultRuntimeExpandedOptInExecutionGateReport,
    pub preflight_record: DnsDefaultRuntimeExpandedOptInExecutionPreflightRecord,
    pub preflight_record_path: Option<String>,
    pub preflight_persisted: bool,
    pub user_trigger_required: bool,
    pub would_mutate_runtime: bool,
    pub mutates_runtime: bool,
    pub executed: bool,
    pub reload_mihomo: bool,
    pub blockers: Vec<String>,
    pub warnings: Vec<String>,
    pub facts: Vec<String>,
}

pub async fn dns_default_runtime_expanded_opt_in_execution_preflight(
    yaml: Option<String>,
    domain: Option<String>,
    explicit_opt_in: bool,
) -> Result<DnsDefaultRuntimeExpandedOptInExecutionPreflightReport> {
    let gate = dns_default_runtime_expanded_opt_in_execution_gate(yaml, domain, explicit_opt_in).await?;
    Ok(build_and_persist_dns_default_runtime_expanded_opt_in_execution_preflight_report(gate, explicit_opt_in).await)
}

pub fn build_dns_default_runtime_expanded_opt_in_execution_preflight_report(
    gate: DnsDefaultRuntimeExpandedOptInExecutionGateReport,
    explicit_opt_in: bool,
    preflight_persisted: bool,
    preflight_record_path: Option<String>,
) -> DnsDefaultRuntimeExpandedOptInExecutionPreflightReport {
    let mut blockers = gate.blockers.clone();
    let created_at_epoch_seconds = default_runtime_epoch_seconds();
    let preflight_event_id = format!("dns-default-runtime-expanded-preflight-{created_at_epoch_seconds}");
    let mutation_plan = DnsDefaultRuntimeExpandedRuntimeMutationPlan {
        previous_runtime: "rustDefaultDnsResolverLimitedOptIn".into(),
        candidate_runtime: gate.candidate_scope.max_active_runtime.clone(),
        execution_mode: "explicitUserTriggeredActiveProfileReloadCandidate".into(),
        active_profile_write: true,
        mihomo_reload: true,
        profile_source: "currentRuntimeDnsConfigPlusRustOwnedDefaultDnsActiveState".into(),
        rollback_strategy: "restoreMihomoManagedDefaultDnsRuntimeThenReload".into(),
    };

    if gate.status != DnsDefaultRuntimeExpandedOptInExecutionGateStatus::Ready || !gate.expansion_allowed {
        blockers.push("expanded opt-in gate is not ready; runtime mutation preflight is blocked".into());
    }
    if !explicit_opt_in {
        blockers.push("explicit opt-in is required before runtime mutation preflight".into());
    }
    if !preflight_persisted && blockers.is_empty() {
        blockers.push("expanded runtime mutation preflight record was not persisted".into());
    }

    let status = if blockers.is_empty() {
        DnsDefaultRuntimeExpandedOptInExecutionPreflightStatus::Ready
    } else {
        DnsDefaultRuntimeExpandedOptInExecutionPreflightStatus::Blocked
    };
    let preflight_record = DnsDefaultRuntimeExpandedOptInExecutionPreflightRecord {
        event_id: preflight_event_id,
        gate_status: gate.status,
        scope_name: gate.candidate_scope.name.clone(),
        mutation_plan,
        created_at_epoch_seconds,
        explicit_opt_in,
    };
    let warnings = gate.warnings.clone();
    let facts = vec![
        "expanded execution preflight consumes Batch R expanded opt-in gate output".into(),
        "expanded execution preflight defines the next active profile write + Mihomo reload candidate".into(),
        "expanded execution preflight persists an audit record before any real runtime mutation batch".into(),
        "expanded execution preflight does not write active profile or reload Mihomo".into(),
        "expanded execution preflight does not touch TUN, transparent proxy, adapters, or protocol runtime".into(),
    ];

    DnsDefaultRuntimeExpandedOptInExecutionPreflightReport {
        status,
        reason: default_runtime_expanded_opt_in_execution_preflight_reason(status, &blockers),
        gate,
        preflight_record,
        preflight_record_path,
        preflight_persisted,
        user_trigger_required: true,
        would_mutate_runtime: status == DnsDefaultRuntimeExpandedOptInExecutionPreflightStatus::Ready,
        mutates_runtime: false,
        executed: false,
        reload_mihomo: false,
        blockers,
        warnings,
        facts,
    }
}

async fn build_and_persist_dns_default_runtime_expanded_opt_in_execution_preflight_report(
    gate: DnsDefaultRuntimeExpandedOptInExecutionGateReport,
    explicit_opt_in: bool,
) -> DnsDefaultRuntimeExpandedOptInExecutionPreflightReport {
    let mut report =
        build_dns_default_runtime_expanded_opt_in_execution_preflight_report(gate, explicit_opt_in, true, None);
    if !report.blockers.is_empty() {
        report.preflight_persisted = false;
        return report;
    }

    let mut persist_errors = Vec::new();
    let path = match default_runtime_expanded_preflight_record_path(&report.preflight_record.event_id) {
        Ok(path) => path,
        Err(error) => {
            report.preflight_persisted = false;
            report
                .blockers
                .push(format!("failed to resolve expanded preflight record path: {error}"));
            report.status = DnsDefaultRuntimeExpandedOptInExecutionPreflightStatus::Blocked;
            report.reason = default_runtime_expanded_opt_in_execution_preflight_reason(report.status, &report.blockers);
            return report;
        }
    };
    if let Some(parent) = path.parent() {
        if let Err(error) = fs::create_dir_all(parent).await {
            persist_errors.push(format!("failed to create expanded preflight directory: {error}"));
        }
    }
    let persisted = persist_errors.is_empty()
        && persist_default_runtime_guard_yaml(&path, &report.preflight_record, &mut persist_errors).await;
    report.preflight_record_path = Some(path.to_string_lossy().to_string());
    report.preflight_persisted = persisted;
    if !persisted {
        report.blockers.extend(persist_errors);
        report.status = DnsDefaultRuntimeExpandedOptInExecutionPreflightStatus::Blocked;
        report.reason = default_runtime_expanded_opt_in_execution_preflight_reason(report.status, &report.blockers);
        report.would_mutate_runtime = false;
    }
    report
}

fn default_runtime_expanded_preflight_record_path(event_id: &str) -> Result<std::path::PathBuf> {
    Ok(default_runtime_state_dir()?
        .join("expanded-preflights")
        .join(safe_dns_runtime_guard_segment(event_id))
        .join("preflight.yaml"))
}

fn default_runtime_expanded_opt_in_execution_preflight_reason(
    status: DnsDefaultRuntimeExpandedOptInExecutionPreflightStatus,
    blockers: &[String],
) -> String {
    match status {
        DnsDefaultRuntimeExpandedOptInExecutionPreflightStatus::Ready => {
            "expanded default DNS runtime mutation preflight is ready for a later explicit execution batch".into()
        }
        DnsDefaultRuntimeExpandedOptInExecutionPreflightStatus::Blocked => blockers
            .first()
            .cloned()
            .unwrap_or_else(|| "expanded default DNS runtime mutation preflight is blocked".into()),
    }
}
