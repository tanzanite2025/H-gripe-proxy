use super::*;

const EXPANDED_REVERIFY_STABLE_RECORDS_REQUIRED: usize = 2;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum DnsDefaultRuntimeExpandedReverifyStatus {
    Recorded,
    RollbackRecommended,
    Blocked,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum DnsDefaultRuntimeExpandedReverifyHistoryStatus {
    Ready,
    Watching,
    RollbackRecommended,
    Empty,
    Blocked,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum DnsDefaultRuntimeExpandedLifecycleCloseoutStatus {
    Complete,
    Watching,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DnsDefaultRuntimeExpandedReverifyHistoryReport {
    pub status: DnsDefaultRuntimeExpandedReverifyHistoryStatus,
    pub reason: String,
    pub records: Vec<DnsDefaultRuntimeExpandedReverifyRecord>,
    pub latest_record: Option<DnsDefaultRuntimeExpandedReverifyRecord>,
    pub record_count: usize,
    pub recorded_count: usize,
    pub rollback_recommended_count: usize,
    pub blocked_count: usize,
    pub keep_active_count: usize,
    pub next_verification_required_count: usize,
    pub stable_streak: usize,
    pub required_stable_records: usize,
    pub first_record_at_epoch_seconds: Option<u64>,
    pub latest_record_at_epoch_seconds: Option<u64>,
    pub closeout_ready: bool,
    pub rollback_recommended: bool,
    pub promotion_allowed: bool,
    pub recommended_action: String,
    pub user_trigger_required: bool,
    pub auto_rollout: bool,
    pub auto_rollback: bool,
    pub mutates_runtime: bool,
    pub reload_mihomo: bool,
    pub blockers: Vec<String>,
    pub warnings: Vec<String>,
    pub facts: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DnsDefaultRuntimeExpandedLifecycleCloseoutReport {
    pub status: DnsDefaultRuntimeExpandedLifecycleCloseoutStatus,
    pub reason: String,
    pub history: DnsDefaultRuntimeExpandedReverifyHistoryReport,
    pub active_state: Option<DnsDefaultRuntimeActiveState>,
    pub observation_closed: bool,
    pub handoff_ready: bool,
    pub rollback_recommended: bool,
    pub promotion_allowed: bool,
    pub recommended_action: String,
    pub next_control_plane_step: String,
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

pub async fn dns_default_runtime_expanded_reverify_history() -> Result<DnsDefaultRuntimeExpandedReverifyHistoryReport> {
    let (records, errors) = read_dns_default_runtime_expanded_reverify_records().await;
    Ok(build_dns_default_runtime_expanded_reverify_history_report(
        records, errors,
    ))
}

pub async fn dns_default_runtime_expanded_lifecycle_closeout()
-> Result<DnsDefaultRuntimeExpandedLifecycleCloseoutReport> {
    let history = dns_default_runtime_expanded_reverify_history().await?;
    let mut errors = Vec::new();
    let active_state = read_default_runtime_active_state(&mut errors).await;
    Ok(build_dns_default_runtime_expanded_lifecycle_closeout_report(
        history,
        active_state,
        errors,
    ))
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

pub fn build_dns_default_runtime_expanded_reverify_history_report(
    mut records: Vec<DnsDefaultRuntimeExpandedReverifyRecord>,
    read_errors: Vec<String>,
) -> DnsDefaultRuntimeExpandedReverifyHistoryReport {
    records.sort_by_key(|record| record.created_at_epoch_seconds);
    let latest_record = records.last().cloned();
    let record_count = records.len();
    let recorded_count = records
        .iter()
        .filter(|record| record.keep_active_allowed && !record.rollback_recommended)
        .count();
    let rollback_recommended_count = records.iter().filter(|record| record.rollback_recommended).count();
    let blocked_count = records
        .iter()
        .filter(|record| !record.keep_active_allowed && !record.rollback_recommended)
        .count();
    let keep_active_count = records.iter().filter(|record| record.keep_active_allowed).count();
    let next_verification_required_count = records
        .iter()
        .filter(|record| record.next_verification_required)
        .count();
    let stable_streak = records
        .iter()
        .rev()
        .take_while(|record| {
            record.keep_active_allowed && !record.next_verification_required && !record.rollback_recommended
        })
        .count();
    let first_record_at_epoch_seconds = records.first().map(|record| record.created_at_epoch_seconds);
    let latest_record_at_epoch_seconds = latest_record.as_ref().map(|record| record.created_at_epoch_seconds);
    let rollback_recommended = rollback_recommended_count > 0
        || latest_record
            .as_ref()
            .map(|record| record.rollback_recommended)
            .unwrap_or(false);
    let mut blockers = read_errors;
    let status = if !blockers.is_empty() {
        DnsDefaultRuntimeExpandedReverifyHistoryStatus::Blocked
    } else if record_count == 0 {
        DnsDefaultRuntimeExpandedReverifyHistoryStatus::Empty
    } else if rollback_recommended {
        DnsDefaultRuntimeExpandedReverifyHistoryStatus::RollbackRecommended
    } else if stable_streak >= EXPANDED_REVERIFY_STABLE_RECORDS_REQUIRED {
        DnsDefaultRuntimeExpandedReverifyHistoryStatus::Ready
    } else {
        DnsDefaultRuntimeExpandedReverifyHistoryStatus::Watching
    };
    if status == DnsDefaultRuntimeExpandedReverifyHistoryStatus::Empty {
        blockers.push("expanded reverify history is empty".into());
    }
    let closeout_ready = status == DnsDefaultRuntimeExpandedReverifyHistoryStatus::Ready;
    let recommended_action = match status {
        DnsDefaultRuntimeExpandedReverifyHistoryStatus::Ready => "closeCurrentExpandedRuntimeObservationWindow",
        DnsDefaultRuntimeExpandedReverifyHistoryStatus::Watching => {
            "continueExplicitExpandedReverifyUntilStableThreshold"
        }
        DnsDefaultRuntimeExpandedReverifyHistoryStatus::RollbackRecommended => {
            "runExplicitExpandedRollbackBeforeContinuing"
        }
        DnsDefaultRuntimeExpandedReverifyHistoryStatus::Empty => "runExpandedReverifyBeforeHistorySummary",
        DnsDefaultRuntimeExpandedReverifyHistoryStatus::Blocked => "fixExpandedReverifyHistoryBeforeContinuing",
    }
    .into();
    let mut warnings = Vec::new();
    if status == DnsDefaultRuntimeExpandedReverifyHistoryStatus::Watching {
        warnings.push(format!(
            "stable reverify streak is {stable_streak}/{EXPANDED_REVERIFY_STABLE_RECORDS_REQUIRED}"
        ));
    }
    if closeout_ready {
        warnings
            .push("history closeout remains session-scoped and does not promote Rust DNS as permanent default".into());
    }
    let facts = vec![
        "expanded reverify history only reads persisted reverify audit records".into(),
        "expanded reverify history summarizes repeated explicit user-triggered checks".into(),
        "expanded reverify history never auto-rolls out or auto-rolls back".into(),
        "expanded reverify history does not mutate runtime or reload Mihomo".into(),
    ];

    DnsDefaultRuntimeExpandedReverifyHistoryReport {
        status,
        reason: default_runtime_expanded_reverify_history_reason(status, &blockers),
        records,
        latest_record,
        record_count,
        recorded_count,
        rollback_recommended_count,
        blocked_count,
        keep_active_count,
        next_verification_required_count,
        stable_streak,
        required_stable_records: EXPANDED_REVERIFY_STABLE_RECORDS_REQUIRED,
        first_record_at_epoch_seconds,
        latest_record_at_epoch_seconds,
        closeout_ready,
        rollback_recommended,
        promotion_allowed: false,
        recommended_action,
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

pub fn build_dns_default_runtime_expanded_lifecycle_closeout_report(
    history: DnsDefaultRuntimeExpandedReverifyHistoryReport,
    active_state: Option<DnsDefaultRuntimeActiveState>,
    active_state_errors: Vec<String>,
) -> DnsDefaultRuntimeExpandedLifecycleCloseoutReport {
    let mut blockers = active_state_errors;
    let expanded_active =
        active_state.as_ref().map(|state| state.state.as_str()) == Some("expandedActiveProfileReloaded");
    if history.closeout_ready && !expanded_active {
        blockers.push("expanded lifecycle closeout requires expandedActiveProfileReloaded active state".into());
    }
    if history.status == DnsDefaultRuntimeExpandedReverifyHistoryStatus::Blocked {
        blockers.extend(history.blockers.clone());
    }
    let status = if !blockers.is_empty() {
        DnsDefaultRuntimeExpandedLifecycleCloseoutStatus::Blocked
    } else if history.rollback_recommended {
        DnsDefaultRuntimeExpandedLifecycleCloseoutStatus::RollbackRecommended
    } else if history.closeout_ready {
        DnsDefaultRuntimeExpandedLifecycleCloseoutStatus::Complete
    } else {
        DnsDefaultRuntimeExpandedLifecycleCloseoutStatus::Watching
    };
    let observation_closed = status == DnsDefaultRuntimeExpandedLifecycleCloseoutStatus::Complete;
    let rollback_recommended = status == DnsDefaultRuntimeExpandedLifecycleCloseoutStatus::RollbackRecommended;
    let handoff_ready = observation_closed && !rollback_recommended;
    let recommended_action = match status {
        DnsDefaultRuntimeExpandedLifecycleCloseoutStatus::Complete => {
            "closeExpandedRuntimeObservationAndPlanNextControlPlaneBlock"
        }
        DnsDefaultRuntimeExpandedLifecycleCloseoutStatus::Watching => "continueExpandedReverifyHistoryBeforeCloseout",
        DnsDefaultRuntimeExpandedLifecycleCloseoutStatus::RollbackRecommended => {
            "runExplicitExpandedRollbackBeforeCloseout"
        }
        DnsDefaultRuntimeExpandedLifecycleCloseoutStatus::Blocked => "fixExpandedLifecycleCloseoutBlockers",
    }
    .into();
    let next_control_plane_step = if handoff_ready {
        "evaluate next app-runtime orchestration feature block before any TUN/protocol runtime work"
    } else if rollback_recommended {
        "explicit expanded rollback"
    } else {
        "repeat explicit expanded reverify until history threshold is met"
    }
    .into();
    let mut warnings = history.warnings.clone();
    if handoff_ready {
        warnings.push("lifecycle closeout is a control-plane handoff, not a data-plane migration approval".into());
    }
    let facts = vec![
        "expanded lifecycle closeout consumes reverify history summary and active state".into(),
        "expanded lifecycle closeout does not promote Rust DNS as a permanent default".into(),
        "expanded lifecycle closeout never auto-rolls out or auto-rolls back".into(),
        "expanded lifecycle closeout does not touch TUN, transparent proxy, adapters, or protocol runtime".into(),
    ];

    DnsDefaultRuntimeExpandedLifecycleCloseoutReport {
        status,
        reason: default_runtime_expanded_lifecycle_closeout_reason(status, &blockers),
        history,
        active_state,
        observation_closed,
        handoff_ready,
        rollback_recommended,
        promotion_allowed: false,
        recommended_action,
        next_control_plane_step,
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

async fn read_dns_default_runtime_expanded_reverify_records()
-> (Vec<DnsDefaultRuntimeExpandedReverifyRecord>, Vec<String>) {
    let mut records = Vec::new();
    let mut errors = Vec::new();
    let dir = match default_runtime_state_dir() {
        Ok(dir) => dir.join("expanded-reverify"),
        Err(error) => {
            errors.push(format!("failed to resolve expanded reverify directory: {error}"));
            return (records, errors);
        }
    };
    let mut entries = match fs::read_dir(&dir).await {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return (records, errors),
        Err(error) => {
            errors.push(format!("failed to read expanded reverify directory: {error}"));
            return (records, errors);
        }
    };
    loop {
        let entry = match entries.next_entry().await {
            Ok(Some(entry)) => entry,
            Ok(None) => break,
            Err(error) => {
                errors.push(format!("failed to read expanded reverify entry: {error}"));
                break;
            }
        };
        let path = entry.path().join("reverify.yaml");
        let path_label = path.to_string_lossy().to_string();
        if let Some(record) = read_default_runtime_guard_yaml::<DnsDefaultRuntimeExpandedReverifyRecord>(
            Some(path_label.as_str()),
            "expanded reverify record",
            &mut errors,
        )
        .await
        {
            records.push(record);
        }
    }
    (records, errors)
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

fn default_runtime_expanded_reverify_history_reason(
    status: DnsDefaultRuntimeExpandedReverifyHistoryStatus,
    blockers: &[String],
) -> String {
    match status {
        DnsDefaultRuntimeExpandedReverifyHistoryStatus::Ready => {
            "expanded default DNS runtime reverify history reached the stable threshold".into()
        }
        DnsDefaultRuntimeExpandedReverifyHistoryStatus::Watching => {
            "expanded default DNS runtime needs more explicit reverify records".into()
        }
        DnsDefaultRuntimeExpandedReverifyHistoryStatus::RollbackRecommended => {
            "expanded default DNS runtime history recommends explicit rollback".into()
        }
        DnsDefaultRuntimeExpandedReverifyHistoryStatus::Empty => {
            "expanded default DNS runtime reverify history is empty".into()
        }
        DnsDefaultRuntimeExpandedReverifyHistoryStatus::Blocked => blockers
            .first()
            .cloned()
            .unwrap_or_else(|| "expanded default DNS runtime reverify history is blocked".into()),
    }
}

fn default_runtime_expanded_lifecycle_closeout_reason(
    status: DnsDefaultRuntimeExpandedLifecycleCloseoutStatus,
    blockers: &[String],
) -> String {
    match status {
        DnsDefaultRuntimeExpandedLifecycleCloseoutStatus::Complete => {
            "expanded default DNS runtime lifecycle observation is closed for this session".into()
        }
        DnsDefaultRuntimeExpandedLifecycleCloseoutStatus::Watching => {
            "expanded default DNS runtime lifecycle still needs more reverify history".into()
        }
        DnsDefaultRuntimeExpandedLifecycleCloseoutStatus::RollbackRecommended => {
            "expanded default DNS runtime lifecycle recommends explicit rollback".into()
        }
        DnsDefaultRuntimeExpandedLifecycleCloseoutStatus::Blocked => blockers
            .first()
            .cloned()
            .unwrap_or_else(|| "expanded default DNS runtime lifecycle closeout is blocked".into()),
    }
}
