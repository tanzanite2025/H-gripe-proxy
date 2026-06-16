use super::*;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum DnsDefaultRuntimeExpandedReverifyStatus {
    Recorded,
    RollbackRecommended,
    Blocked,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DnsDefaultRuntimeExpandedReverifyRecord {
    pub event_id: String,
    pub action: String,
    pub active_execution_event_id: Option<String>,
    pub hold_status: DnsDefaultRuntimeExpandedHoldPolicyStatus,
    pub stability_status: DnsDefaultRuntimeExpandedStabilityGateStatus,
    pub post_execution_status: DnsDefaultRuntimeExpandedPostExecutionVerificationStatus,
    pub active_age_seconds: Option<u64>,
    pub keep_active_allowed: bool,
    pub next_verification_required: bool,
    pub rollback_recommended: bool,
    pub next_verification_after_epoch_seconds: Option<u64>,
    pub hold_expires_at_epoch_seconds: Option<u64>,
    pub created_at_epoch_seconds: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DnsDefaultRuntimeExpandedReverifyReport {
    pub status: DnsDefaultRuntimeExpandedReverifyStatus,
    pub reason: String,
    pub hold_policy: DnsDefaultRuntimeExpandedHoldPolicyReport,
    pub reverify_record: DnsDefaultRuntimeExpandedReverifyRecord,
    pub reverify_record_path: Option<String>,
    pub reverify_persisted: bool,
    pub keep_active_allowed: bool,
    pub next_verification_required: bool,
    pub rollback_recommended: bool,
    pub user_trigger_required: bool,
    pub auto_rollout: bool,
    pub auto_rollback: bool,
    pub mutates_runtime: bool,
    pub reload_mihomo: bool,
    pub blockers: Vec<String>,
    pub warnings: Vec<String>,
    pub facts: Vec<String>,
}

pub async fn dns_default_runtime_expanded_reverify(
    yaml: Option<String>,
    domain: Option<String>,
    explicit_opt_in: bool,
) -> Result<DnsDefaultRuntimeExpandedReverifyReport> {
    let hold_policy = dns_default_runtime_expanded_hold_policy(yaml, domain, explicit_opt_in).await?;
    Ok(persist_dns_default_runtime_expanded_reverify_report(hold_policy).await)
}

pub fn build_dns_default_runtime_expanded_reverify_report(
    hold_policy: DnsDefaultRuntimeExpandedHoldPolicyReport,
    reverify_persisted: bool,
    reverify_record_path: Option<String>,
    mut persist_errors: Vec<String>,
    created_at_epoch_seconds: u64,
) -> DnsDefaultRuntimeExpandedReverifyReport {
    let active_execution_event_id = hold_policy
        .stability_gate
        .post_execution
        .active_state
        .as_ref()
        .map(|state| state.execution_event_id.clone());
    let reverify_record = DnsDefaultRuntimeExpandedReverifyRecord {
        event_id: format!("dns-default-runtime-expanded-reverify-{created_at_epoch_seconds}"),
        action: "defaultDnsRuntimeExpandedReverify".into(),
        active_execution_event_id,
        hold_status: hold_policy.status,
        stability_status: hold_policy.stability_gate.status,
        post_execution_status: hold_policy.stability_gate.post_execution.status,
        active_age_seconds: hold_policy.active_age_seconds,
        keep_active_allowed: hold_policy.keep_active_allowed,
        next_verification_required: hold_policy.next_verification_required,
        rollback_recommended: hold_policy.rollback_recommended,
        next_verification_after_epoch_seconds: hold_policy.next_verification_after_epoch_seconds,
        hold_expires_at_epoch_seconds: hold_policy.hold_expires_at_epoch_seconds,
        created_at_epoch_seconds,
    };
    let mut blockers = hold_policy.blockers.clone();
    if hold_policy.status == DnsDefaultRuntimeExpandedHoldPolicyStatus::Blocked {
        blockers.push("expanded reverify requires a non-blocked hold policy".into());
    }
    if !reverify_persisted {
        blockers.append(&mut persist_errors);
    }

    let status = if !blockers.is_empty() {
        DnsDefaultRuntimeExpandedReverifyStatus::Blocked
    } else if hold_policy.rollback_recommended {
        DnsDefaultRuntimeExpandedReverifyStatus::RollbackRecommended
    } else {
        DnsDefaultRuntimeExpandedReverifyStatus::Recorded
    };
    let mut warnings = hold_policy.warnings.clone();
    if hold_policy.next_verification_required {
        warnings.push("expanded runtime must be reverified after the minimum hold window".into());
    }
    let facts = vec![
        "expanded reverify records one explicit hold-window evaluation".into(),
        "expanded reverify can be repeated by explicit user trigger".into(),
        "expanded reverify never auto-rolls out or auto-rolls back".into(),
        "expanded reverify does not mutate runtime or reload Mihomo".into(),
    ];

    DnsDefaultRuntimeExpandedReverifyReport {
        status,
        reason: default_runtime_expanded_reverify_reason(status, &blockers),
        keep_active_allowed: hold_policy.keep_active_allowed,
        next_verification_required: hold_policy.next_verification_required,
        rollback_recommended: hold_policy.rollback_recommended,
        hold_policy,
        reverify_record,
        reverify_record_path,
        reverify_persisted,
        user_trigger_required: true,
        auto_rollout: false,
        auto_rollback: false,
        mutates_runtime: false,
        reload_mihomo: false,
        blockers,
        warnings,
        facts,
    }
}

async fn persist_dns_default_runtime_expanded_reverify_report(
    hold_policy: DnsDefaultRuntimeExpandedHoldPolicyReport,
) -> DnsDefaultRuntimeExpandedReverifyReport {
    let created_at_epoch_seconds = default_runtime_epoch_seconds();
    let event_id = format!("dns-default-runtime-expanded-reverify-{created_at_epoch_seconds}");
    let mut persist_errors = Vec::new();
    let path = match default_runtime_expanded_reverify_record_path(&event_id) {
        Ok(path) => path,
        Err(error) => {
            return build_dns_default_runtime_expanded_reverify_report(
                hold_policy,
                false,
                None,
                vec![format!("failed to resolve expanded reverify record path: {error}")],
                created_at_epoch_seconds,
            );
        }
    };
    if let Some(parent) = path.parent() {
        if let Err(error) = fs::create_dir_all(parent).await {
            persist_errors.push(format!("failed to create expanded reverify directory: {error}"));
        }
    }

    let report = build_dns_default_runtime_expanded_reverify_report(
        hold_policy,
        persist_errors.is_empty(),
        Some(path.to_string_lossy().to_string()),
        persist_errors,
        created_at_epoch_seconds,
    );
    let persisted = report.blockers.is_empty()
        && persist_default_runtime_guard_yaml(&path, &report.reverify_record, &mut Vec::new()).await;
    if persisted {
        report
    } else {
        build_dns_default_runtime_expanded_reverify_report(
            report.hold_policy,
            false,
            Some(path.to_string_lossy().to_string()),
            vec!["failed to persist expanded reverify record".into()],
            created_at_epoch_seconds,
        )
    }
}

fn default_runtime_expanded_reverify_record_path(event_id: &str) -> Result<std::path::PathBuf> {
    Ok(default_runtime_state_dir()?
        .join("expanded-reverify")
        .join(safe_dns_runtime_guard_segment(event_id))
        .join("reverify.yaml"))
}

fn default_runtime_expanded_reverify_reason(
    status: DnsDefaultRuntimeExpandedReverifyStatus,
    blockers: &[String],
) -> String {
    match status {
        DnsDefaultRuntimeExpandedReverifyStatus::Recorded => {
            "expanded default DNS runtime reverify record was persisted".into()
        }
        DnsDefaultRuntimeExpandedReverifyStatus::RollbackRecommended => {
            "expanded default DNS runtime reverify recommends explicit rollback".into()
        }
        DnsDefaultRuntimeExpandedReverifyStatus::Blocked => blockers
            .first()
            .cloned()
            .unwrap_or_else(|| "expanded default DNS runtime reverify is blocked".into()),
    }
}
