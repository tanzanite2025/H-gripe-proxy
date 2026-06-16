use super::*;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum DnsDefaultRuntimePostExecutionVerificationStatus {
    Verified,
    Failed,
    Blocked,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum DnsDefaultRuntimeRollbackDrillStatus {
    Ready,
    Blocked,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DnsDefaultRuntimeRollbackDrillReport {
    pub status: DnsDefaultRuntimeRollbackDrillStatus,
    pub reason: String,
    pub active_state: Option<DnsDefaultRuntimeActiveState>,
    pub execution_record: Option<DnsDefaultRuntimeExecutionRecord>,
    pub rollback_marker: Option<DnsDefaultRuntimeExecutorRollbackMarker>,
    pub would_rollback: bool,
    pub would_restore_runtime: String,
    pub auto_rollback: bool,
    pub mutates_runtime: bool,
    pub reload_mihomo: bool,
    pub blockers: Vec<String>,
    pub warnings: Vec<String>,
    pub facts: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DnsDefaultRuntimePostExecutionFailureAudit {
    pub required: bool,
    pub event_id: String,
    pub active_execution_event_id: Option<String>,
    pub reasons: Vec<String>,
    pub rollback_drill_required: bool,
    pub created_at_epoch_seconds: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DnsDefaultRuntimePostExecutionObservedVerificationReport {
    pub status: DnsDefaultRuntimePostExecutionVerificationStatus,
    pub reason: String,
    pub active_state: Option<DnsDefaultRuntimeActiveState>,
    pub execution_record: Option<DnsDefaultRuntimeExecutionRecord>,
    pub pre_execution_audit_record: Option<DnsDefaultRuntimeExecutorAuditRecord>,
    pub observed_evidence: DnsDefaultRuntimeShadowEvidenceReport,
    pub rollback_drill: DnsDefaultRuntimeRollbackDrillReport,
    pub failure_audit: DnsDefaultRuntimePostExecutionFailureAudit,
    pub mutates_runtime: bool,
    pub reload_mihomo: bool,
    pub blockers: Vec<String>,
    pub warnings: Vec<String>,
    pub facts: Vec<String>,
}

pub async fn dns_default_runtime_rollback_drill() -> Result<DnsDefaultRuntimeRollbackDrillReport> {
    Ok(read_default_runtime_rollback_drill().await)
}

pub async fn dns_default_runtime_post_execution_observed_verification(
    yaml: Option<String>,
    domain: Option<String>,
) -> Result<DnsDefaultRuntimePostExecutionObservedVerificationReport> {
    let yaml = runtime_dns_shadow_yaml(yaml, "post execution observed verification").await?;
    let domain = normalize_shadow_domain(domain);
    let readiness = build_dns_default_runtime_readiness_report(&yaml, None)?;
    let controller = DnsResolverRuntimeController::new(HickoryDnsResolverRuntime);
    let rust_report = controller.query(readiness.plan.clone(), domain.clone()).await;
    let system_result = dns_query_with_options(
        domain,
        None,
        None,
        DnsRuntimeQueryOptions {
            timeout_ms: readiness.plan.timeout_ms,
            attempts: readiness.plan.attempts,
        },
    )
    .await?;
    let observed_evidence = build_dns_default_runtime_shadow_evidence_report(readiness, rust_report, system_result);

    let mut metadata_errors = Vec::new();
    let active_state = read_default_runtime_active_state(&mut metadata_errors).await;
    let execution_record =
        read_default_runtime_execution_record_from_active(active_state.as_ref(), &mut metadata_errors).await;
    let pre_execution_audit_record = read_default_runtime_guard_yaml::<DnsDefaultRuntimeExecutorAuditRecord>(
        active_state
            .as_ref()
            .and_then(|state| state.audit_record_path.as_deref()),
        "pre-execution audit record",
        &mut metadata_errors,
    )
    .await;
    let rollback_marker = read_default_runtime_guard_yaml::<DnsDefaultRuntimeExecutorRollbackMarker>(
        active_state
            .as_ref()
            .and_then(|state| state.rollback_marker_path.as_deref()),
        "rollback marker",
        &mut metadata_errors,
    )
    .await;
    let rollback_drill = build_dns_default_runtime_rollback_drill_report(
        active_state.clone(),
        execution_record.clone(),
        rollback_marker,
        metadata_errors.clone(),
    );

    Ok(build_dns_default_runtime_post_execution_observed_verification_report(
        active_state,
        execution_record,
        pre_execution_audit_record,
        observed_evidence,
        rollback_drill,
        metadata_errors,
    ))
}

async fn read_default_runtime_rollback_drill() -> DnsDefaultRuntimeRollbackDrillReport {
    let mut metadata_errors = Vec::new();
    let active_state = read_default_runtime_active_state(&mut metadata_errors).await;
    let execution_record =
        read_default_runtime_execution_record_from_active(active_state.as_ref(), &mut metadata_errors).await;
    let rollback_marker = read_default_runtime_guard_yaml::<DnsDefaultRuntimeExecutorRollbackMarker>(
        active_state
            .as_ref()
            .and_then(|state| state.rollback_marker_path.as_deref()),
        "rollback marker",
        &mut metadata_errors,
    )
    .await;

    build_dns_default_runtime_rollback_drill_report(active_state, execution_record, rollback_marker, metadata_errors)
}

pub fn build_dns_default_runtime_rollback_drill_report(
    active_state: Option<DnsDefaultRuntimeActiveState>,
    execution_record: Option<DnsDefaultRuntimeExecutionRecord>,
    rollback_marker: Option<DnsDefaultRuntimeExecutorRollbackMarker>,
    mut blockers: Vec<String>,
) -> DnsDefaultRuntimeRollbackDrillReport {
    if let Some(active_state) = active_state.as_ref() {
        if active_state.active_runtime != "rustDefaultDnsResolver" {
            blockers.push("active default DNS runtime is not rustDefaultDnsResolver".into());
        }
        if active_state.previous_runtime != "mihomoManagedDefaultDns" {
            blockers.push("rollback target is not mihomoManagedDefaultDns".into());
        }
        if active_state.state != "active" {
            blockers.push("default DNS runtime active state is not active".into());
        }
    } else {
        blockers.push("default DNS runtime active state was not found".into());
    }

    if let Some(execution_record) = execution_record.as_ref() {
        if execution_record.action != "defaultDnsRuntimeLimitedOptInExecution" {
            blockers.push("execution audit is not for limited opt-in execution".into());
        }
        if execution_record.status != "executed" {
            blockers.push("limited execution audit is not executed".into());
        }
        if !execution_record.metadata_verified {
            blockers.push("limited execution audit metadata was not verified".into());
        }
    } else {
        blockers.push("limited execution audit record was not found".into());
    }

    if let Some(rollback_marker) = rollback_marker.as_ref() {
        if !rollback_marker.prepared || !rollback_marker.restores_runtime {
            blockers.push("rollback marker is not prepared to restore the runtime".into());
        }
        if rollback_marker.previous_runtime != "mihomoManagedDefaultDns" {
            blockers.push("rollback marker does not restore mihomoManagedDefaultDns".into());
        }
        if rollback_marker.candidate_runtime != "rustDefaultDnsResolver" {
            blockers.push("rollback marker candidate is not rustDefaultDnsResolver".into());
        }
    } else {
        blockers.push("rollback marker was not found".into());
    }

    let status = if blockers.is_empty() {
        DnsDefaultRuntimeRollbackDrillStatus::Ready
    } else {
        DnsDefaultRuntimeRollbackDrillStatus::Blocked
    };
    let would_restore_runtime = rollback_marker
        .as_ref()
        .map(|marker| marker.previous_runtime.clone())
        .or_else(|| active_state.as_ref().map(|state| state.previous_runtime.clone()))
        .unwrap_or_else(|| "mihomoManagedDefaultDns".into());
    let reason = default_runtime_rollback_drill_reason(status, &blockers);
    let facts = vec![
        "rollback drill only reads Batch P active state and audit metadata".into(),
        "rollback drill does not execute rollback automatically".into(),
        "rollback drill does not write active profile".into(),
        "rollback drill does not reload Mihomo".into(),
    ];

    DnsDefaultRuntimeRollbackDrillReport {
        status,
        reason,
        active_state,
        execution_record,
        rollback_marker,
        would_rollback: false,
        would_restore_runtime,
        auto_rollback: false,
        mutates_runtime: false,
        reload_mihomo: false,
        blockers,
        warnings: Vec::new(),
        facts,
    }
}

pub fn build_dns_default_runtime_post_execution_observed_verification_report(
    active_state: Option<DnsDefaultRuntimeActiveState>,
    execution_record: Option<DnsDefaultRuntimeExecutionRecord>,
    pre_execution_audit_record: Option<DnsDefaultRuntimeExecutorAuditRecord>,
    observed_evidence: DnsDefaultRuntimeShadowEvidenceReport,
    rollback_drill: DnsDefaultRuntimeRollbackDrillReport,
    mut blockers: Vec<String>,
) -> DnsDefaultRuntimePostExecutionObservedVerificationReport {
    let created_at_epoch_seconds = default_runtime_epoch_seconds();
    let mut failure_reasons = Vec::new();

    if let Some(active_state) = active_state.as_ref() {
        if active_state.active_runtime != "rustDefaultDnsResolver" {
            blockers.push("active default DNS runtime is not rustDefaultDnsResolver".into());
        }
        if active_state.state != "active" {
            blockers.push("default DNS runtime active state is not active".into());
        }
    } else {
        blockers.push("default DNS runtime active state was not found".into());
    }

    if let Some(execution_record) = execution_record.as_ref() {
        if execution_record.action != "defaultDnsRuntimeLimitedOptInExecution" {
            blockers.push("execution audit is not for limited opt-in execution".into());
        }
        if execution_record.status != "executed" {
            blockers.push("limited execution audit is not executed".into());
        }
    } else {
        blockers.push("limited execution audit record was not found".into());
    }

    if pre_execution_audit_record.is_none() {
        blockers.push("pre-execution shadow evidence audit was not found".into());
    }

    if observed_evidence.status != DnsDefaultRuntimeShadowEvidenceStatus::Matched {
        failure_reasons.push(format!(
            "observed DNS query verification is {}; {}",
            dns_shadow_status_label(observed_evidence.status),
            observed_evidence.reason
        ));
    }

    if let Some(pre_execution_audit_record) = pre_execution_audit_record.as_ref() {
        if pre_execution_audit_record.shadow_status != observed_evidence.status {
            failure_reasons.push(format!(
                "observed shadow status {} differs from pre-execution shadow status {}",
                dns_shadow_status_label(observed_evidence.status),
                dns_shadow_status_label(pre_execution_audit_record.shadow_status)
            ));
        }
    }

    if rollback_drill.status != DnsDefaultRuntimeRollbackDrillStatus::Ready {
        failure_reasons.push("rollback drill is not ready; expansion is unsafe".into());
    }

    let status = if !blockers.is_empty() {
        DnsDefaultRuntimePostExecutionVerificationStatus::Blocked
    } else if failure_reasons.is_empty() {
        DnsDefaultRuntimePostExecutionVerificationStatus::Verified
    } else {
        DnsDefaultRuntimePostExecutionVerificationStatus::Failed
    };
    let reason = default_runtime_post_execution_verification_reason(status, &blockers, &failure_reasons);
    let active_execution_event_id = active_state
        .as_ref()
        .map(|state| state.execution_event_id.clone())
        .or_else(|| execution_record.as_ref().map(|record| record.event_id.clone()));
    let failure_audit = DnsDefaultRuntimePostExecutionFailureAudit {
        required: status != DnsDefaultRuntimePostExecutionVerificationStatus::Verified,
        event_id: format!("dns-default-runtime-post-execution-failure-{created_at_epoch_seconds}"),
        active_execution_event_id,
        reasons: if blockers.is_empty() {
            failure_reasons.clone()
        } else {
            blockers.clone()
        },
        rollback_drill_required: true,
        created_at_epoch_seconds,
    };
    let mut warnings = observed_evidence.warnings.clone();
    warnings.extend(rollback_drill.warnings.clone());
    let facts = vec![
        "post-execution verification only reads Batch P active state and audit metadata".into(),
        "post-execution verification compares observed evidence with pre-execution shadow status".into(),
        "post-execution verification does not execute rollback automatically".into(),
        "post-execution verification does not write active profile or reload Mihomo".into(),
    ];

    DnsDefaultRuntimePostExecutionObservedVerificationReport {
        status,
        reason,
        active_state,
        execution_record,
        pre_execution_audit_record,
        observed_evidence,
        rollback_drill,
        failure_audit,
        mutates_runtime: false,
        reload_mihomo: false,
        blockers,
        warnings,
        facts,
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

fn default_runtime_post_execution_verification_reason(
    status: DnsDefaultRuntimePostExecutionVerificationStatus,
    blockers: &[String],
    failure_reasons: &[String],
) -> String {
    match status {
        DnsDefaultRuntimePostExecutionVerificationStatus::Verified => {
            "post-execution observed verification matched pre-execution shadow evidence".into()
        }
        DnsDefaultRuntimePostExecutionVerificationStatus::Failed => failure_reasons
            .first()
            .cloned()
            .unwrap_or_else(|| "post-execution observed verification failed".into()),
        DnsDefaultRuntimePostExecutionVerificationStatus::Blocked => blockers
            .first()
            .cloned()
            .unwrap_or_else(|| "post-execution observed verification is blocked".into()),
    }
}

pub(crate) fn default_runtime_post_execution_status_label(
    status: DnsDefaultRuntimePostExecutionVerificationStatus,
) -> &'static str {
    match status {
        DnsDefaultRuntimePostExecutionVerificationStatus::Verified => "verified",
        DnsDefaultRuntimePostExecutionVerificationStatus::Failed => "failed",
        DnsDefaultRuntimePostExecutionVerificationStatus::Blocked => "blocked",
    }
}
