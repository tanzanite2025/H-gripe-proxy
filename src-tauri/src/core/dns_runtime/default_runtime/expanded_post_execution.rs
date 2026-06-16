use super::*;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum DnsDefaultRuntimeExpandedPostExecutionVerificationStatus {
    Verified,
    Failed,
    Blocked,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum DnsDefaultRuntimeExpandedRollbackDrillStatus {
    Ready,
    Blocked,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DnsDefaultRuntimeExpandedRollbackDrillReport {
    pub status: DnsDefaultRuntimeExpandedRollbackDrillStatus,
    pub reason: String,
    pub active_state: Option<DnsDefaultRuntimeActiveState>,
    pub execution_record: Option<DnsDefaultRuntimeExecutionRecord>,
    pub preflight_record: Option<DnsDefaultRuntimeExpandedOptInExecutionPreflightRecord>,
    pub would_rollback: bool,
    pub would_restore_runtime: String,
    pub auto_rollback: bool,
    pub mutates_runtime: bool,
    pub reload_mihomo: bool,
    pub blockers: Vec<String>,
    pub warnings: Vec<String>,
    pub facts: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DnsDefaultRuntimeExpandedPostExecutionObservedVerificationReport {
    pub status: DnsDefaultRuntimeExpandedPostExecutionVerificationStatus,
    pub reason: String,
    pub active_state: Option<DnsDefaultRuntimeActiveState>,
    pub execution_record: Option<DnsDefaultRuntimeExecutionRecord>,
    pub preflight_record: Option<DnsDefaultRuntimeExpandedOptInExecutionPreflightRecord>,
    pub observed_evidence: DnsDefaultRuntimeShadowEvidenceReport,
    pub rollback_drill: DnsDefaultRuntimeExpandedRollbackDrillReport,
    pub failure_audit: DnsDefaultRuntimePostExecutionFailureAudit,
    pub mutates_runtime: bool,
    pub reload_mihomo: bool,
    pub blockers: Vec<String>,
    pub warnings: Vec<String>,
    pub facts: Vec<String>,
}

pub async fn dns_default_runtime_expanded_rollback_drill() -> Result<DnsDefaultRuntimeExpandedRollbackDrillReport> {
    Ok(read_default_runtime_expanded_rollback_drill().await)
}

pub async fn dns_default_runtime_expanded_post_execution_observed_verification(
    yaml: Option<String>,
    domain: Option<String>,
) -> Result<DnsDefaultRuntimeExpandedPostExecutionObservedVerificationReport> {
    let yaml = runtime_dns_shadow_yaml(yaml, "expanded post execution observed verification").await?;
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
    let preflight_record = read_default_runtime_guard_yaml::<DnsDefaultRuntimeExpandedOptInExecutionPreflightRecord>(
        active_state
            .as_ref()
            .and_then(|state| state.audit_record_path.as_deref()),
        "expanded preflight record",
        &mut metadata_errors,
    )
    .await;
    let rollback_drill = build_dns_default_runtime_expanded_rollback_drill_report(
        active_state.clone(),
        execution_record.clone(),
        preflight_record.clone(),
        metadata_errors.clone(),
    );

    Ok(
        build_dns_default_runtime_expanded_post_execution_observed_verification_report(
            active_state,
            execution_record,
            preflight_record,
            observed_evidence,
            rollback_drill,
            metadata_errors,
        ),
    )
}

async fn read_default_runtime_expanded_rollback_drill() -> DnsDefaultRuntimeExpandedRollbackDrillReport {
    let mut metadata_errors = Vec::new();
    let active_state = read_default_runtime_active_state(&mut metadata_errors).await;
    let execution_record =
        read_default_runtime_execution_record_from_active(active_state.as_ref(), &mut metadata_errors).await;
    let preflight_record = read_default_runtime_guard_yaml::<DnsDefaultRuntimeExpandedOptInExecutionPreflightRecord>(
        active_state
            .as_ref()
            .and_then(|state| state.audit_record_path.as_deref()),
        "expanded preflight record",
        &mut metadata_errors,
    )
    .await;

    build_dns_default_runtime_expanded_rollback_drill_report(
        active_state,
        execution_record,
        preflight_record,
        metadata_errors,
    )
}

pub fn build_dns_default_runtime_expanded_rollback_drill_report(
    active_state: Option<DnsDefaultRuntimeActiveState>,
    execution_record: Option<DnsDefaultRuntimeExecutionRecord>,
    preflight_record: Option<DnsDefaultRuntimeExpandedOptInExecutionPreflightRecord>,
    mut blockers: Vec<String>,
) -> DnsDefaultRuntimeExpandedRollbackDrillReport {
    if let Some(active_state) = active_state.as_ref() {
        if active_state.active_runtime != "rustDefaultDnsResolver" {
            blockers.push("active default DNS runtime is not rustDefaultDnsResolver".into());
        }
        if active_state.previous_runtime != "mihomoManagedDefaultDns" {
            blockers.push("expanded rollback target is not mihomoManagedDefaultDns".into());
        }
        if active_state.state != "expandedActiveProfileReloaded" {
            blockers.push("default DNS runtime active state was not created by expanded execution".into());
        }
    } else {
        blockers.push("default DNS runtime active state was not found".into());
    }

    if let Some(execution_record) = execution_record.as_ref() {
        if execution_record.action != "defaultDnsRuntimeExpandedOptInExecution" {
            blockers.push("execution audit is not for expanded opt-in execution".into());
        }
        if execution_record.status != "executed" {
            blockers.push("expanded execution audit is not executed".into());
        }
        if !execution_record.metadata_verified {
            blockers.push("expanded execution audit metadata was not verified".into());
        }
    } else {
        blockers.push("expanded execution audit record was not found".into());
    }

    if let Some(preflight_record) = preflight_record.as_ref() {
        if preflight_record.gate_status != DnsDefaultRuntimeExpandedOptInExecutionGateStatus::Ready {
            blockers.push("expanded preflight did not originate from a ready expanded gate".into());
        }
        if !preflight_record.explicit_opt_in {
            blockers.push("expanded preflight was not explicitly opted in".into());
        }
        if !preflight_record.mutation_plan.active_profile_write || !preflight_record.mutation_plan.mihomo_reload {
            blockers.push("expanded preflight mutation plan is missing active profile reload boundary".into());
        }
    } else {
        blockers.push("expanded preflight record was not found".into());
    }

    let status = if blockers.is_empty() {
        DnsDefaultRuntimeExpandedRollbackDrillStatus::Ready
    } else {
        DnsDefaultRuntimeExpandedRollbackDrillStatus::Blocked
    };
    let would_restore_runtime = active_state
        .as_ref()
        .map(|state| state.previous_runtime.clone())
        .or_else(|| {
            preflight_record
                .as_ref()
                .map(|record| record.mutation_plan.previous_runtime.clone())
        })
        .unwrap_or_else(|| "mihomoManagedDefaultDns".into());
    let facts = vec![
        "expanded rollback drill only reads Batch T active state and audit metadata".into(),
        "expanded rollback drill does not execute rollback automatically".into(),
        "expanded rollback drill does not write active profile".into(),
        "expanded rollback drill does not reload Mihomo".into(),
    ];

    DnsDefaultRuntimeExpandedRollbackDrillReport {
        status,
        reason: default_runtime_expanded_rollback_drill_reason(status, &blockers),
        active_state,
        execution_record,
        preflight_record,
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

pub fn build_dns_default_runtime_expanded_post_execution_observed_verification_report(
    active_state: Option<DnsDefaultRuntimeActiveState>,
    execution_record: Option<DnsDefaultRuntimeExecutionRecord>,
    preflight_record: Option<DnsDefaultRuntimeExpandedOptInExecutionPreflightRecord>,
    observed_evidence: DnsDefaultRuntimeShadowEvidenceReport,
    rollback_drill: DnsDefaultRuntimeExpandedRollbackDrillReport,
    mut blockers: Vec<String>,
) -> DnsDefaultRuntimeExpandedPostExecutionObservedVerificationReport {
    let created_at_epoch_seconds = default_runtime_epoch_seconds();
    let mut failure_reasons = Vec::new();

    if let Some(active_state) = active_state.as_ref() {
        if active_state.active_runtime != "rustDefaultDnsResolver" {
            blockers.push("active default DNS runtime is not rustDefaultDnsResolver".into());
        }
        if active_state.state != "expandedActiveProfileReloaded" {
            blockers.push("default DNS runtime active state was not created by expanded execution".into());
        }
    } else {
        blockers.push("default DNS runtime active state was not found".into());
    }

    if let Some(execution_record) = execution_record.as_ref() {
        if execution_record.action != "defaultDnsRuntimeExpandedOptInExecution" {
            blockers.push("execution audit is not for expanded opt-in execution".into());
        }
        if execution_record.status != "executed" {
            blockers.push("expanded execution audit is not executed".into());
        }
    } else {
        blockers.push("expanded execution audit record was not found".into());
    }

    if preflight_record.is_none() {
        blockers.push("expanded preflight record was not found".into());
    }

    if observed_evidence.status != DnsDefaultRuntimeShadowEvidenceStatus::Matched {
        failure_reasons.push(format!(
            "expanded observed DNS query verification is {}; {}",
            dns_shadow_status_label(observed_evidence.status),
            observed_evidence.reason
        ));
    }

    if rollback_drill.status != DnsDefaultRuntimeExpandedRollbackDrillStatus::Ready {
        failure_reasons.push("expanded rollback drill is not ready".into());
    }

    let status = if !blockers.is_empty() {
        DnsDefaultRuntimeExpandedPostExecutionVerificationStatus::Blocked
    } else if failure_reasons.is_empty() {
        DnsDefaultRuntimeExpandedPostExecutionVerificationStatus::Verified
    } else {
        DnsDefaultRuntimeExpandedPostExecutionVerificationStatus::Failed
    };
    let active_execution_event_id = active_state
        .as_ref()
        .map(|state| state.execution_event_id.clone())
        .or_else(|| execution_record.as_ref().map(|record| record.event_id.clone()));
    let failure_audit = DnsDefaultRuntimePostExecutionFailureAudit {
        required: status != DnsDefaultRuntimeExpandedPostExecutionVerificationStatus::Verified,
        event_id: format!("dns-default-runtime-expanded-post-execution-failure-{created_at_epoch_seconds}"),
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
        "expanded post-execution verification only reads Batch T active state and audit metadata".into(),
        "expanded post-execution verification compares observed evidence after active reload".into(),
        "expanded post-execution verification does not execute rollback automatically".into(),
        "expanded post-execution verification does not write active profile or reload Mihomo".into(),
    ];

    DnsDefaultRuntimeExpandedPostExecutionObservedVerificationReport {
        status,
        reason: default_runtime_expanded_post_execution_verification_reason(status, &blockers, &failure_reasons),
        active_state,
        execution_record,
        preflight_record,
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

fn default_runtime_expanded_rollback_drill_reason(
    status: DnsDefaultRuntimeExpandedRollbackDrillStatus,
    blockers: &[String],
) -> String {
    match status {
        DnsDefaultRuntimeExpandedRollbackDrillStatus::Ready => {
            "expanded rollback drill is ready; rollback was not executed".into()
        }
        DnsDefaultRuntimeExpandedRollbackDrillStatus::Blocked => blockers
            .first()
            .cloned()
            .unwrap_or_else(|| "expanded rollback drill is blocked".into()),
    }
}

fn default_runtime_expanded_post_execution_verification_reason(
    status: DnsDefaultRuntimeExpandedPostExecutionVerificationStatus,
    blockers: &[String],
    failure_reasons: &[String],
) -> String {
    match status {
        DnsDefaultRuntimeExpandedPostExecutionVerificationStatus::Verified => {
            "expanded default DNS runtime post-execution verification passed".into()
        }
        DnsDefaultRuntimeExpandedPostExecutionVerificationStatus::Blocked => blockers
            .first()
            .cloned()
            .unwrap_or_else(|| "expanded default DNS runtime post-execution verification is blocked".into()),
        DnsDefaultRuntimeExpandedPostExecutionVerificationStatus::Failed => failure_reasons
            .first()
            .cloned()
            .unwrap_or_else(|| "expanded default DNS runtime post-execution verification failed".into()),
    }
}
