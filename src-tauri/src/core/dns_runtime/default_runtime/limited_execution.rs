use super::*;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum DnsDefaultRuntimeLimitedExecutionStatus {
    Executed,
    Blocked,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum DnsDefaultRuntimeLimitedRollbackStatus {
    Restored,
    Blocked,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DnsDefaultRuntimeExecutionRecord {
    pub event_id: String,
    pub action: String,
    pub status: String,
    pub guard_event_id: String,
    pub previous_runtime: String,
    pub candidate_runtime: String,
    pub created_at_epoch_seconds: u64,
    pub metadata_verified: bool,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DnsDefaultRuntimeActiveState {
    pub active_runtime: String,
    pub previous_runtime: String,
    pub state: String,
    pub execution_event_id: String,
    pub activated_at_epoch_seconds: u64,
    pub rollback_marker_path: Option<String>,
    pub audit_record_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DnsDefaultRuntimeLimitedOptInExecutionReport {
    pub status: DnsDefaultRuntimeLimitedExecutionStatus,
    pub reason: String,
    pub guard: DnsDefaultRuntimeOptInExecutionGuardReport,
    pub execution_record: DnsDefaultRuntimeExecutionRecord,
    pub active_state: Option<DnsDefaultRuntimeActiveState>,
    pub active_state_path: Option<String>,
    pub execution_record_path: Option<String>,
    pub metadata_verified: bool,
    pub rollback_available: bool,
    pub mutates_runtime: bool,
    pub executed: bool,
    pub reload_mihomo: bool,
    pub blockers: Vec<String>,
    pub warnings: Vec<String>,
    pub facts: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DnsDefaultRuntimeLimitedRollbackReport {
    pub status: DnsDefaultRuntimeLimitedRollbackStatus,
    pub reason: String,
    pub previous_state: Option<DnsDefaultRuntimeActiveState>,
    pub restored_state: Option<DnsDefaultRuntimeActiveState>,
    pub rollback_record: DnsDefaultRuntimeExecutionRecord,
    pub active_state_path: Option<String>,
    pub rollback_record_path: Option<String>,
    pub mutates_runtime: bool,
    pub reload_mihomo: bool,
    pub blockers: Vec<String>,
    pub warnings: Vec<String>,
    pub facts: Vec<String>,
}

pub async fn dns_default_runtime_limited_opt_in_execution(
    yaml: Option<String>,
    domain: Option<String>,
    explicit_opt_in: bool,
) -> Result<DnsDefaultRuntimeLimitedOptInExecutionReport> {
    let guard = dns_default_runtime_opt_in_execution_guard(yaml, domain, explicit_opt_in).await?;
    Ok(run_default_runtime_limited_execution(guard).await)
}

pub async fn dns_default_runtime_limited_rollback() -> Result<DnsDefaultRuntimeLimitedRollbackReport> {
    Ok(run_default_runtime_limited_rollback().await)
}

async fn run_default_runtime_limited_execution(
    guard: DnsDefaultRuntimeOptInExecutionGuardReport,
) -> DnsDefaultRuntimeLimitedOptInExecutionReport {
    let mut blockers = guard.blockers.clone();
    let warnings = guard.warnings.clone();
    if guard.status != DnsDefaultRuntimeExecutionGuardStatus::Ready {
        blockers.push("execution guard is not ready; limited opt-in execution is blocked".into());
    }
    if !guard.execution_allowed {
        blockers.push("execution guard did not allow default DNS runtime execution".into());
    }

    let mut metadata_errors = Vec::new();
    let metadata_verified = verify_default_runtime_execution_guard_metadata(&guard, &mut metadata_errors).await;
    if !metadata_verified {
        blockers.push("persisted execution guard metadata could not be verified".into());
    }
    blockers.extend(metadata_errors);

    let created_at_epoch_seconds = default_runtime_epoch_seconds();
    let execution_event_id = format!("dns-default-runtime-limited-execution-{created_at_epoch_seconds}");
    let mut execution_record = DnsDefaultRuntimeExecutionRecord {
        event_id: execution_event_id.clone(),
        action: "defaultDnsRuntimeLimitedOptInExecution".into(),
        status: "blocked".into(),
        guard_event_id: guard.preflight.audit_record.event_id.clone(),
        previous_runtime: guard.preflight.mutation_diff.previous_runtime.clone(),
        candidate_runtime: guard.preflight.mutation_diff.candidate_runtime.clone(),
        created_at_epoch_seconds,
        metadata_verified,
        error: blockers.first().cloned(),
    };
    let mut active_state = None;
    let mut active_state_path = default_runtime_active_state_path()
        .ok()
        .map(|path| path.to_string_lossy().to_string());
    let mut execution_record_path = default_runtime_execution_record_path(&execution_event_id)
        .ok()
        .map(|path| path.to_string_lossy().to_string());
    let mut persist_errors = Vec::new();

    if blockers.is_empty() {
        execution_record.status = "executed".into();
        execution_record.error = None;
        let next_active_state = DnsDefaultRuntimeActiveState {
            active_runtime: guard.preflight.mutation_diff.candidate_runtime.clone(),
            previous_runtime: guard.preflight.mutation_diff.previous_runtime.clone(),
            state: "active".into(),
            execution_event_id: execution_event_id.clone(),
            activated_at_epoch_seconds: created_at_epoch_seconds,
            rollback_marker_path: guard.persistence.rollback_marker_path.clone(),
            audit_record_path: guard.persistence.audit_record_path.clone(),
        };

        let execution_persisted = persist_default_runtime_execution_record(
            &execution_record,
            &mut execution_record_path,
            &mut persist_errors,
        )
        .await;
        let active_persisted = execution_persisted
            && persist_default_runtime_active_state(&next_active_state, &mut active_state_path, &mut persist_errors)
                .await;
        if active_persisted {
            active_state = Some(next_active_state);
        } else {
            execution_record.status = "failed".into();
            execution_record.error = persist_errors.first().cloned();
            blockers.extend(persist_errors);
        }
    }

    let status = if blockers.is_empty() {
        DnsDefaultRuntimeLimitedExecutionStatus::Executed
    } else {
        DnsDefaultRuntimeLimitedExecutionStatus::Blocked
    };
    let reason = default_runtime_limited_execution_reason(status, &blockers);
    let facts = vec![
        "limited execution requires persisted execution guard metadata".into(),
        "limited execution writes Rust-owned default DNS runtime active state only".into(),
        "limited execution does not write active profile".into(),
        "limited execution does not reload Mihomo".into(),
    ];

    DnsDefaultRuntimeLimitedOptInExecutionReport {
        status,
        reason,
        guard,
        execution_record,
        active_state,
        active_state_path,
        execution_record_path,
        metadata_verified,
        rollback_available: status == DnsDefaultRuntimeLimitedExecutionStatus::Executed,
        mutates_runtime: status == DnsDefaultRuntimeLimitedExecutionStatus::Executed,
        executed: status == DnsDefaultRuntimeLimitedExecutionStatus::Executed,
        reload_mihomo: false,
        blockers,
        warnings,
        facts,
    }
}

async fn run_default_runtime_limited_rollback() -> DnsDefaultRuntimeLimitedRollbackReport {
    let mut blockers = Vec::new();
    let warnings = Vec::new();
    let created_at_epoch_seconds = default_runtime_epoch_seconds();
    let rollback_event_id = format!("dns-default-runtime-limited-rollback-{created_at_epoch_seconds}");
    let active_state_path = default_runtime_active_state_path()
        .ok()
        .map(|path| path.to_string_lossy().to_string());
    let rollback_record_path = default_runtime_execution_record_path(&rollback_event_id)
        .ok()
        .map(|path| path.to_string_lossy().to_string());
    let previous_state = read_default_runtime_active_state(&mut blockers).await;
    let mut rollback_record = DnsDefaultRuntimeExecutionRecord {
        event_id: rollback_event_id.clone(),
        action: "defaultDnsRuntimeLimitedRollback".into(),
        status: "blocked".into(),
        guard_event_id: previous_state
            .as_ref()
            .map(|state| state.execution_event_id.clone())
            .unwrap_or_default(),
        previous_runtime: previous_state
            .as_ref()
            .map(|state| state.active_runtime.clone())
            .unwrap_or_else(|| "unknown".into()),
        candidate_runtime: previous_state
            .as_ref()
            .map(|state| state.previous_runtime.clone())
            .unwrap_or_else(|| "mihomoManagedDefaultDns".into()),
        created_at_epoch_seconds,
        metadata_verified: previous_state.is_some(),
        error: None,
    };
    let mut restored_state = None;

    if let Some(previous_state) = previous_state.clone() {
        if previous_state.active_runtime != "rustDefaultDnsResolver" {
            blockers.push("active default DNS runtime is not rustDefaultDnsResolver; rollback is not needed".into());
        }
        if previous_state.previous_runtime != "mihomoManagedDefaultDns" {
            blockers.push("rollback target is not mihomoManagedDefaultDns".into());
        }
        if blockers.is_empty() {
            let next_state = DnsDefaultRuntimeActiveState {
                active_runtime: previous_state.previous_runtime.clone(),
                previous_runtime: previous_state.active_runtime.clone(),
                state: "rolledBack".into(),
                execution_event_id: rollback_event_id.clone(),
                activated_at_epoch_seconds: created_at_epoch_seconds,
                rollback_marker_path: previous_state.rollback_marker_path.clone(),
                audit_record_path: previous_state.audit_record_path.clone(),
            };
            let mut persist_errors = Vec::new();
            rollback_record.status = "restored".into();
            let mut next_active_state_path = active_state_path.clone();
            let mut next_rollback_record_path = rollback_record_path.clone();
            let rollback_persisted = persist_default_runtime_execution_record(
                &rollback_record,
                &mut next_rollback_record_path,
                &mut persist_errors,
            )
            .await;
            let active_persisted = rollback_persisted
                && persist_default_runtime_active_state(&next_state, &mut next_active_state_path, &mut persist_errors)
                    .await;
            if active_persisted {
                restored_state = Some(next_state);
            } else {
                rollback_record.status = "failed".into();
                rollback_record.error = persist_errors.first().cloned();
                blockers.extend(persist_errors);
            }
        }
    } else {
        blockers.push("default DNS runtime active state was not found".into());
    }

    if !blockers.is_empty() {
        rollback_record.error = blockers.first().cloned();
    }
    let status = if blockers.is_empty() {
        DnsDefaultRuntimeLimitedRollbackStatus::Restored
    } else {
        DnsDefaultRuntimeLimitedRollbackStatus::Blocked
    };
    let reason = default_runtime_limited_rollback_reason(status, &blockers);
    let facts = vec![
        "limited rollback restores Rust-owned default DNS runtime active state".into(),
        "limited rollback does not write active profile".into(),
        "limited rollback does not reload Mihomo".into(),
    ];

    DnsDefaultRuntimeLimitedRollbackReport {
        status,
        reason,
        previous_state,
        restored_state,
        rollback_record,
        active_state_path,
        rollback_record_path,
        mutates_runtime: status == DnsDefaultRuntimeLimitedRollbackStatus::Restored,
        reload_mihomo: false,
        blockers,
        warnings,
        facts,
    }
}

fn default_runtime_limited_execution_reason(
    status: DnsDefaultRuntimeLimitedExecutionStatus,
    blockers: &[String],
) -> String {
    match status {
        DnsDefaultRuntimeLimitedExecutionStatus::Executed => {
            "limited default DNS runtime execution activated the Rust-owned runtime state".into()
        }
        DnsDefaultRuntimeLimitedExecutionStatus::Blocked => blockers
            .first()
            .cloned()
            .unwrap_or_else(|| "limited default DNS runtime execution is blocked".into()),
    }
}

fn default_runtime_limited_rollback_reason(
    status: DnsDefaultRuntimeLimitedRollbackStatus,
    blockers: &[String],
) -> String {
    match status {
        DnsDefaultRuntimeLimitedRollbackStatus::Restored => {
            "default DNS runtime rollback restored Mihomo-managed runtime state".into()
        }
        DnsDefaultRuntimeLimitedRollbackStatus::Blocked => blockers
            .first()
            .cloned()
            .unwrap_or_else(|| "default DNS runtime rollback is blocked".into()),
    }
}

fn default_runtime_rollback_drill_reason(status: DnsDefaultRuntimeRollbackDrillStatus, blockers: &[String]) -> String {
    match status {
        DnsDefaultRuntimeRollbackDrillStatus::Ready => "rollback drill is ready; rollback was not executed".into(),
        DnsDefaultRuntimeRollbackDrillStatus::Blocked => blockers
            .first()
            .cloned()
            .unwrap_or_else(|| "default DNS runtime rollback drill is blocked".into()),
    }
}
