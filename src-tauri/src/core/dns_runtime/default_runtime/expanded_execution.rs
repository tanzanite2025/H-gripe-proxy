use super::*;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum DnsDefaultRuntimeExpandedOptInExecutionStatus {
    Executed,
    Blocked,
    Failed,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum DnsDefaultRuntimeExpandedRollbackStatus {
    Restored,
    Blocked,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DnsDefaultRuntimeExpandedOptInExecutionReport {
    pub status: DnsDefaultRuntimeExpandedOptInExecutionStatus,
    pub reason: String,
    pub preflight: DnsDefaultRuntimeExpandedOptInExecutionPreflightReport,
    pub execution_record: DnsDefaultRuntimeExecutionRecord,
    pub active_state: Option<DnsDefaultRuntimeActiveState>,
    pub active_state_path: Option<String>,
    pub execution_record_path: Option<String>,
    pub dns_config_apply_attempted: bool,
    pub dns_config_applied: bool,
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
pub struct DnsDefaultRuntimeExpandedRollbackReport {
    pub status: DnsDefaultRuntimeExpandedRollbackStatus,
    pub reason: String,
    pub previous_state: Option<DnsDefaultRuntimeActiveState>,
    pub restored_state: Option<DnsDefaultRuntimeActiveState>,
    pub rollback_record: DnsDefaultRuntimeExecutionRecord,
    pub active_state_path: Option<String>,
    pub rollback_record_path: Option<String>,
    pub dns_config_restore_attempted: bool,
    pub dns_config_restored: bool,
    pub mutates_runtime: bool,
    pub reload_mihomo: bool,
    pub blockers: Vec<String>,
    pub warnings: Vec<String>,
    pub facts: Vec<String>,
}

pub async fn dns_default_runtime_expanded_opt_in_execution(
    yaml: Option<String>,
    domain: Option<String>,
    explicit_opt_in: bool,
) -> Result<DnsDefaultRuntimeExpandedOptInExecutionReport> {
    let preflight = dns_default_runtime_expanded_opt_in_execution_preflight(yaml, domain, explicit_opt_in).await?;
    Ok(run_default_runtime_expanded_execution(preflight).await)
}

pub async fn dns_default_runtime_expanded_rollback() -> Result<DnsDefaultRuntimeExpandedRollbackReport> {
    Ok(run_default_runtime_expanded_rollback().await)
}

pub fn build_dns_default_runtime_expanded_opt_in_execution_report(
    preflight: DnsDefaultRuntimeExpandedOptInExecutionPreflightReport,
    dns_config_apply_attempted: bool,
    dns_config_applied: bool,
    persist_errors: Vec<String>,
) -> DnsDefaultRuntimeExpandedOptInExecutionReport {
    let mut blockers = preflight.blockers.clone();
    if preflight.status != DnsDefaultRuntimeExpandedOptInExecutionPreflightStatus::Ready {
        blockers.push("expanded execution preflight is not ready; execution is blocked".into());
    }
    if !preflight.preflight_persisted {
        blockers.push("expanded execution preflight record is not persisted".into());
    }

    let created_at_epoch_seconds = default_runtime_epoch_seconds();
    let execution_event_id = format!("dns-default-runtime-expanded-execution-{created_at_epoch_seconds}");
    let mut execution_record = DnsDefaultRuntimeExecutionRecord {
        event_id: execution_event_id.clone(),
        action: "defaultDnsRuntimeExpandedOptInExecution".into(),
        status: "blocked".into(),
        guard_event_id: preflight.preflight_record.event_id.clone(),
        previous_runtime: preflight.preflight_record.mutation_plan.previous_runtime.clone(),
        candidate_runtime: preflight.preflight_record.mutation_plan.candidate_runtime.clone(),
        created_at_epoch_seconds,
        metadata_verified: preflight.preflight_persisted,
        error: blockers.first().cloned(),
    };
    let mut active_state = None;

    let status = if !blockers.is_empty() {
        DnsDefaultRuntimeExpandedOptInExecutionStatus::Blocked
    } else if !dns_config_applied || !persist_errors.is_empty() {
        execution_record.status = "failed".into();
        execution_record.error = persist_errors
            .first()
            .cloned()
            .or_else(|| Some("failed to apply DNS config to active runtime".into()));
        blockers.extend(persist_errors);
        DnsDefaultRuntimeExpandedOptInExecutionStatus::Failed
    } else {
        execution_record.status = "executed".into();
        execution_record.error = None;
        active_state = Some(DnsDefaultRuntimeActiveState {
            active_runtime: preflight.preflight_record.mutation_plan.candidate_runtime.clone(),
            previous_runtime: preflight.preflight_record.mutation_plan.previous_runtime.clone(),
            state: "expandedActiveProfileReloaded".into(),
            execution_event_id,
            activated_at_epoch_seconds: created_at_epoch_seconds,
            rollback_marker_path: None,
            audit_record_path: preflight.preflight_record_path.clone(),
        });
        DnsDefaultRuntimeExpandedOptInExecutionStatus::Executed
    };
    let facts = vec![
        "expanded execution requires Batch S preflight to be ready and persisted".into(),
        "expanded execution is explicitly user triggered only".into(),
        "expanded execution applies the DNS config through the existing Mihomo config reload path".into(),
        "expanded execution records Rust-owned active state and rollback audit metadata".into(),
        "expanded execution does not touch TUN, transparent proxy, adapters, or protocol runtime".into(),
    ];

    DnsDefaultRuntimeExpandedOptInExecutionReport {
        status,
        reason: default_runtime_expanded_execution_reason(status, &blockers),
        preflight,
        execution_record,
        active_state,
        active_state_path: default_runtime_active_state_path()
            .ok()
            .map(|path| path.to_string_lossy().to_string()),
        execution_record_path: None,
        dns_config_apply_attempted,
        dns_config_applied,
        rollback_available: status == DnsDefaultRuntimeExpandedOptInExecutionStatus::Executed,
        mutates_runtime: dns_config_applied,
        executed: status == DnsDefaultRuntimeExpandedOptInExecutionStatus::Executed,
        reload_mihomo: dns_config_applied,
        blockers,
        warnings: Vec::new(),
        facts,
    }
}

async fn run_default_runtime_expanded_execution(
    preflight: DnsDefaultRuntimeExpandedOptInExecutionPreflightReport,
) -> DnsDefaultRuntimeExpandedOptInExecutionReport {
    let mut blockers = preflight.blockers.clone();
    if preflight.status != DnsDefaultRuntimeExpandedOptInExecutionPreflightStatus::Ready {
        blockers.push("expanded execution preflight is not ready; execution is blocked".into());
    }
    if !preflight.preflight_persisted {
        blockers.push("expanded execution preflight record is not persisted".into());
    }

    let created_at_epoch_seconds = default_runtime_epoch_seconds();
    let execution_event_id = format!("dns-default-runtime-expanded-execution-{created_at_epoch_seconds}");
    let mut execution_record = DnsDefaultRuntimeExecutionRecord {
        event_id: execution_event_id.clone(),
        action: "defaultDnsRuntimeExpandedOptInExecution".into(),
        status: "blocked".into(),
        guard_event_id: preflight.preflight_record.event_id.clone(),
        previous_runtime: preflight.preflight_record.mutation_plan.previous_runtime.clone(),
        candidate_runtime: preflight.preflight_record.mutation_plan.candidate_runtime.clone(),
        created_at_epoch_seconds,
        metadata_verified: preflight.preflight_persisted,
        error: blockers.first().cloned(),
    };
    let mut active_state = None;
    let mut active_state_path = default_runtime_active_state_path()
        .ok()
        .map(|path| path.to_string_lossy().to_string());
    let mut execution_record_path = default_runtime_execution_record_path(&execution_event_id)
        .ok()
        .map(|path| path.to_string_lossy().to_string());
    let mut dns_config_apply_attempted = false;
    let mut dns_config_applied = false;
    let mut persist_errors = Vec::new();

    if blockers.is_empty() {
        dns_config_apply_attempted = true;
        match crate::app::runtime::apply_dns_config(true).await {
            Ok(()) => {
                dns_config_applied = true;
                execution_record.status = "executed".into();
                execution_record.error = None;
                let next_active_state = DnsDefaultRuntimeActiveState {
                    active_runtime: preflight.preflight_record.mutation_plan.candidate_runtime.clone(),
                    previous_runtime: preflight.preflight_record.mutation_plan.previous_runtime.clone(),
                    state: "expandedActiveProfileReloaded".into(),
                    execution_event_id: execution_event_id.clone(),
                    activated_at_epoch_seconds: created_at_epoch_seconds,
                    rollback_marker_path: None,
                    audit_record_path: preflight.preflight_record_path.clone(),
                };
                let execution_persisted = persist_default_runtime_execution_record(
                    &execution_record,
                    &mut execution_record_path,
                    &mut persist_errors,
                )
                .await;
                let active_persisted = execution_persisted
                    && persist_default_runtime_active_state(
                        &next_active_state,
                        &mut active_state_path,
                        &mut persist_errors,
                    )
                    .await;
                if active_persisted {
                    active_state = Some(next_active_state);
                } else {
                    execution_record.status = "failed".into();
                    execution_record.error = persist_errors.first().cloned();
                    blockers.extend(persist_errors);
                }
            }
            Err(error) => {
                execution_record.status = "failed".into();
                execution_record.error = Some(format!("failed to apply DNS config to active runtime: {error}"));
                blockers.push(format!("failed to apply DNS config to active runtime: {error}"));
            }
        }
    }

    let status = if !blockers.is_empty() {
        if dns_config_apply_attempted {
            DnsDefaultRuntimeExpandedOptInExecutionStatus::Failed
        } else {
            DnsDefaultRuntimeExpandedOptInExecutionStatus::Blocked
        }
    } else {
        DnsDefaultRuntimeExpandedOptInExecutionStatus::Executed
    };
    let facts = vec![
        "expanded execution requires Batch S preflight to be ready and persisted".into(),
        "expanded execution is explicitly user triggered only".into(),
        "expanded execution applies the DNS config through the existing Mihomo config reload path".into(),
        "expanded execution records Rust-owned active state and rollback audit metadata".into(),
        "expanded execution does not touch TUN, transparent proxy, adapters, or protocol runtime".into(),
    ];

    DnsDefaultRuntimeExpandedOptInExecutionReport {
        status,
        reason: default_runtime_expanded_execution_reason(status, &blockers),
        preflight,
        execution_record,
        active_state,
        active_state_path,
        execution_record_path,
        dns_config_apply_attempted,
        dns_config_applied,
        rollback_available: status == DnsDefaultRuntimeExpandedOptInExecutionStatus::Executed,
        mutates_runtime: dns_config_applied,
        executed: status == DnsDefaultRuntimeExpandedOptInExecutionStatus::Executed,
        reload_mihomo: dns_config_applied,
        blockers,
        warnings: Vec::new(),
        facts,
    }
}

async fn run_default_runtime_expanded_rollback() -> DnsDefaultRuntimeExpandedRollbackReport {
    let mut blockers = Vec::new();
    let warnings = Vec::new();
    let created_at_epoch_seconds = default_runtime_epoch_seconds();
    let rollback_event_id = format!("dns-default-runtime-expanded-rollback-{created_at_epoch_seconds}");
    let mut active_state_path = default_runtime_active_state_path()
        .ok()
        .map(|path| path.to_string_lossy().to_string());
    let mut rollback_record_path = default_runtime_execution_record_path(&rollback_event_id)
        .ok()
        .map(|path| path.to_string_lossy().to_string());
    let previous_state = read_default_runtime_active_state(&mut blockers).await;
    let mut rollback_record = DnsDefaultRuntimeExecutionRecord {
        event_id: rollback_event_id.clone(),
        action: "defaultDnsRuntimeExpandedRollback".into(),
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
    let mut dns_config_restore_attempted = false;
    let mut dns_config_restored = false;

    if let Some(previous_state) = previous_state.clone() {
        if previous_state.active_runtime != "rustDefaultDnsResolver" {
            blockers.push(
                "active default DNS runtime is not rustDefaultDnsResolver; expanded rollback is not needed".into(),
            );
        }
        if previous_state.state != "expandedActiveProfileReloaded" {
            blockers.push("active default DNS runtime was not created by expanded execution".into());
        }
        if blockers.is_empty() {
            dns_config_restore_attempted = true;
            match crate::app::runtime::apply_dns_config(false).await {
                Ok(()) => {
                    dns_config_restored = true;
                    rollback_record.status = "restored".into();
                    rollback_record.error = None;
                    let next_state = DnsDefaultRuntimeActiveState {
                        active_runtime: previous_state.previous_runtime.clone(),
                        previous_runtime: previous_state.active_runtime.clone(),
                        state: "expandedRolledBack".into(),
                        execution_event_id: rollback_event_id.clone(),
                        activated_at_epoch_seconds: created_at_epoch_seconds,
                        rollback_marker_path: None,
                        audit_record_path: previous_state.audit_record_path.clone(),
                    };
                    let mut persist_errors = Vec::new();
                    let record_persisted = persist_default_runtime_execution_record(
                        &rollback_record,
                        &mut rollback_record_path,
                        &mut persist_errors,
                    )
                    .await;
                    let state_persisted = record_persisted
                        && persist_default_runtime_active_state(
                            &next_state,
                            &mut active_state_path,
                            &mut persist_errors,
                        )
                        .await;
                    if state_persisted {
                        restored_state = Some(next_state);
                    } else {
                        rollback_record.status = "failed".into();
                        rollback_record.error = persist_errors.first().cloned();
                        blockers.extend(persist_errors);
                    }
                }
                Err(error) => {
                    rollback_record.status = "failed".into();
                    rollback_record.error = Some(format!("failed to restore Mihomo-managed DNS config: {error}"));
                    blockers.push(format!("failed to restore Mihomo-managed DNS config: {error}"));
                }
            }
        }
    }

    let status = if blockers.is_empty() && restored_state.is_some() {
        DnsDefaultRuntimeExpandedRollbackStatus::Restored
    } else if dns_config_restore_attempted {
        DnsDefaultRuntimeExpandedRollbackStatus::Failed
    } else {
        DnsDefaultRuntimeExpandedRollbackStatus::Blocked
    };
    let facts = vec![
        "expanded rollback requires active state created by expanded execution".into(),
        "expanded rollback restores Mihomo-managed DNS config through the existing reload path".into(),
        "expanded rollback records Rust-owned active state and audit metadata".into(),
        "expanded rollback does not touch TUN, transparent proxy, adapters, or protocol runtime".into(),
    ];

    DnsDefaultRuntimeExpandedRollbackReport {
        status,
        reason: default_runtime_expanded_rollback_reason(status, &blockers),
        previous_state,
        restored_state,
        rollback_record,
        active_state_path,
        rollback_record_path,
        dns_config_restore_attempted,
        dns_config_restored,
        mutates_runtime: dns_config_restored,
        reload_mihomo: dns_config_restored,
        blockers,
        warnings,
        facts,
    }
}

fn default_runtime_expanded_execution_reason(
    status: DnsDefaultRuntimeExpandedOptInExecutionStatus,
    blockers: &[String],
) -> String {
    match status {
        DnsDefaultRuntimeExpandedOptInExecutionStatus::Executed => {
            "expanded default DNS runtime execution applied DNS config through the active runtime reload path".into()
        }
        DnsDefaultRuntimeExpandedOptInExecutionStatus::Blocked => blockers
            .first()
            .cloned()
            .unwrap_or_else(|| "expanded default DNS runtime execution is blocked".into()),
        DnsDefaultRuntimeExpandedOptInExecutionStatus::Failed => blockers
            .first()
            .cloned()
            .unwrap_or_else(|| "expanded default DNS runtime execution failed".into()),
    }
}

fn default_runtime_expanded_rollback_reason(
    status: DnsDefaultRuntimeExpandedRollbackStatus,
    blockers: &[String],
) -> String {
    match status {
        DnsDefaultRuntimeExpandedRollbackStatus::Restored => {
            "expanded default DNS runtime rollback restored Mihomo-managed DNS runtime".into()
        }
        DnsDefaultRuntimeExpandedRollbackStatus::Blocked => blockers
            .first()
            .cloned()
            .unwrap_or_else(|| "expanded default DNS runtime rollback is blocked".into()),
        DnsDefaultRuntimeExpandedRollbackStatus::Failed => blockers
            .first()
            .cloned()
            .unwrap_or_else(|| "expanded default DNS runtime rollback failed".into()),
    }
}
