use super::*;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum DnsDefaultRuntimeExecutionGuardStatus {
    Ready,
    Blocked,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DnsDefaultRuntimeExecutionSupersededState {
    pub previous_runtime: String,
    pub candidate_runtime: String,
    pub state: String,
    pub superseded_at_epoch_seconds: u64,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DnsDefaultRuntimeExecutionPersistence {
    pub requested: bool,
    pub prepared: bool,
    pub audit_record_path: Option<String>,
    pub rollback_marker_path: Option<String>,
    pub superseded_state_path: Option<String>,
    pub audit_persisted: bool,
    pub rollback_marker_persisted: bool,
    pub superseded_state_persisted: bool,
    pub errors: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DnsDefaultRuntimeOptInExecutionGuardReport {
    pub status: DnsDefaultRuntimeExecutionGuardStatus,
    pub reason: String,
    pub preflight: DnsDefaultRuntimeOptInExecutorPreflightReport,
    pub persistence: DnsDefaultRuntimeExecutionPersistence,
    pub superseded_state: DnsDefaultRuntimeExecutionSupersededState,
    pub execution_allowed: bool,
    pub user_trigger_required: bool,
    pub mutates_runtime: bool,
    pub executed: bool,
    pub reload_mihomo: bool,
    pub blockers: Vec<String>,
    pub warnings: Vec<String>,
    pub facts: Vec<String>,
}

pub async fn dns_default_runtime_opt_in_execution_guard(
    yaml: Option<String>,
    domain: Option<String>,
    explicit_opt_in: bool,
) -> Result<DnsDefaultRuntimeOptInExecutionGuardReport> {
    let preflight = dns_default_runtime_opt_in_executor_preflight(yaml, domain, explicit_opt_in).await?;
    let (persistence, superseded_state) = persist_default_runtime_execution_guard_state(&preflight).await;
    Ok(build_dns_default_runtime_opt_in_execution_guard_report(
        preflight,
        persistence,
        superseded_state,
    ))
}

pub fn build_dns_default_runtime_opt_in_execution_guard_report(
    preflight: DnsDefaultRuntimeOptInExecutorPreflightReport,
    persistence: DnsDefaultRuntimeExecutionPersistence,
    superseded_state: DnsDefaultRuntimeExecutionSupersededState,
) -> DnsDefaultRuntimeOptInExecutionGuardReport {
    let mut blockers = preflight.blockers.clone();
    let warnings = preflight.warnings.clone();

    if preflight.status != DnsDefaultRuntimeExecutorPreflightStatus::Ready {
        blockers.push("executor preflight is not ready; execution guard cannot allow runtime mutation".into());
    }
    if !persistence.prepared {
        blockers.push("execution audit and rollback marker persistence is not prepared".into());
    }
    blockers.extend(persistence.errors.clone());

    let status = if blockers.is_empty() {
        DnsDefaultRuntimeExecutionGuardStatus::Ready
    } else {
        DnsDefaultRuntimeExecutionGuardStatus::Blocked
    };
    let reason = default_runtime_execution_guard_reason(status, &blockers);
    let facts = vec![
        "execution guard requires explicit user trigger".into(),
        "execution guard persisted audit and rollback metadata before any runtime mutation".into(),
        "execution guard does not write active profile".into(),
        "execution guard does not reload Mihomo".into(),
        format!("superseded state={}", superseded_state.state),
    ];

    DnsDefaultRuntimeOptInExecutionGuardReport {
        status,
        reason,
        preflight,
        persistence,
        superseded_state,
        execution_allowed: status == DnsDefaultRuntimeExecutionGuardStatus::Ready,
        user_trigger_required: true,
        mutates_runtime: false,
        executed: false,
        reload_mihomo: false,
        blockers,
        warnings,
        facts,
    }
}

pub(crate) fn default_runtime_execution_superseded_state(
    preflight: &DnsDefaultRuntimeOptInExecutorPreflightReport,
) -> DnsDefaultRuntimeExecutionSupersededState {
    DnsDefaultRuntimeExecutionSupersededState {
        previous_runtime: preflight.mutation_diff.previous_runtime.clone(),
        candidate_runtime: preflight.mutation_diff.candidate_runtime.clone(),
        state: "pendingExecution".into(),
        superseded_at_epoch_seconds: preflight.audit_record.created_at_epoch_seconds,
        reason: "prepared before any default DNS runtime execution".into(),
    }
}

fn default_runtime_execution_guard_reason(
    status: DnsDefaultRuntimeExecutionGuardStatus,
    blockers: &[String],
) -> String {
    match status {
        DnsDefaultRuntimeExecutionGuardStatus::Ready => {
            "default DNS runtime execution guard passed; execution was not performed".into()
        }
        DnsDefaultRuntimeExecutionGuardStatus::Blocked => blockers
            .first()
            .cloned()
            .unwrap_or_else(|| "default DNS runtime execution guard is blocked".into()),
    }
}
