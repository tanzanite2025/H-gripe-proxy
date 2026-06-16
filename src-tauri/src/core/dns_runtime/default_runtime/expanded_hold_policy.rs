use super::*;

const EXPANDED_HOLD_MIN_SECONDS: u64 = 300;
const EXPANDED_HOLD_MAX_SECONDS: u64 = 3600;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum DnsDefaultRuntimeExpandedHoldPolicyStatus {
    Ready,
    Holding,
    RollbackRecommended,
    Blocked,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DnsDefaultRuntimeExpandedHoldPolicyReport {
    pub status: DnsDefaultRuntimeExpandedHoldPolicyStatus,
    pub reason: String,
    pub stability_gate: DnsDefaultRuntimeExpandedStabilityGateReport,
    pub active_age_seconds: Option<u64>,
    pub minimum_hold_seconds: u64,
    pub maximum_hold_seconds: u64,
    pub hold_started_at_epoch_seconds: Option<u64>,
    pub next_verification_after_epoch_seconds: Option<u64>,
    pub hold_expires_at_epoch_seconds: Option<u64>,
    pub keep_active_allowed: bool,
    pub next_verification_required: bool,
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

pub async fn dns_default_runtime_expanded_hold_policy(
    yaml: Option<String>,
    domain: Option<String>,
    explicit_opt_in: bool,
) -> Result<DnsDefaultRuntimeExpandedHoldPolicyReport> {
    let stability_gate = dns_default_runtime_expanded_stability_gate(yaml, domain, explicit_opt_in).await?;
    Ok(build_dns_default_runtime_expanded_hold_policy_report(
        stability_gate,
        default_runtime_epoch_seconds(),
    ))
}

pub fn build_dns_default_runtime_expanded_hold_policy_report(
    stability_gate: DnsDefaultRuntimeExpandedStabilityGateReport,
    current_epoch_seconds: u64,
) -> DnsDefaultRuntimeExpandedHoldPolicyReport {
    let mut blockers = Vec::new();
    let active_state = stability_gate.post_execution.active_state.as_ref();
    let active_age_seconds =
        active_state.map(|state| current_epoch_seconds.saturating_sub(state.activated_at_epoch_seconds));
    let hold_started_at_epoch_seconds = active_state.map(|state| state.activated_at_epoch_seconds);
    let next_verification_after_epoch_seconds =
        hold_started_at_epoch_seconds.map(|started_at| started_at.saturating_add(EXPANDED_HOLD_MIN_SECONDS));
    let hold_expires_at_epoch_seconds =
        hold_started_at_epoch_seconds.map(|started_at| started_at.saturating_add(EXPANDED_HOLD_MAX_SECONDS));

    if active_state.is_none() {
        blockers.push("expanded hold policy requires an expanded active runtime state".into());
    }
    let hold_window_expired = active_age_seconds
        .map(|age| age > EXPANDED_HOLD_MAX_SECONDS)
        .unwrap_or(false);

    let status = if !stability_gate.keep_active_allowed {
        if active_state.is_some() {
            DnsDefaultRuntimeExpandedHoldPolicyStatus::RollbackRecommended
        } else {
            blockers.extend(stability_gate.blockers.clone());
            DnsDefaultRuntimeExpandedHoldPolicyStatus::Blocked
        }
    } else if !blockers.is_empty() {
        DnsDefaultRuntimeExpandedHoldPolicyStatus::Blocked
    } else if hold_window_expired {
        DnsDefaultRuntimeExpandedHoldPolicyStatus::RollbackRecommended
    } else if active_age_seconds
        .map(|age| age < EXPANDED_HOLD_MIN_SECONDS)
        .unwrap_or(false)
    {
        DnsDefaultRuntimeExpandedHoldPolicyStatus::Holding
    } else {
        DnsDefaultRuntimeExpandedHoldPolicyStatus::Ready
    };

    let keep_active_allowed = matches!(
        status,
        DnsDefaultRuntimeExpandedHoldPolicyStatus::Ready | DnsDefaultRuntimeExpandedHoldPolicyStatus::Holding
    );
    let next_verification_required = status == DnsDefaultRuntimeExpandedHoldPolicyStatus::Holding;
    let rollback_recommended = status == DnsDefaultRuntimeExpandedHoldPolicyStatus::RollbackRecommended;
    let recommended_action = match status {
        DnsDefaultRuntimeExpandedHoldPolicyStatus::Ready => "keepExpandedRuntimeActiveWithinSessionHoldWindow",
        DnsDefaultRuntimeExpandedHoldPolicyStatus::Holding => "continueHoldAndReverifyAfterMinimumWindow",
        DnsDefaultRuntimeExpandedHoldPolicyStatus::RollbackRecommended => "runExplicitExpandedRollback",
        DnsDefaultRuntimeExpandedHoldPolicyStatus::Blocked => "completeExpandedExecutionAndStabilityGateFirst",
    }
    .into();
    let mut warnings = stability_gate.warnings.clone();
    if hold_window_expired {
        warnings.push("expanded hold window expired; run explicit rollback before continuing".into());
    }
    let facts = vec![
        "expanded hold policy consumes Batch V expanded stability gate".into(),
        "expanded hold policy is time-windowed and session-scoped".into(),
        "expanded hold policy never promotes Rust DNS as a permanent default".into(),
        "expanded hold policy never auto-rolls out or auto-rolls back".into(),
    ];

    DnsDefaultRuntimeExpandedHoldPolicyReport {
        status,
        reason: default_runtime_expanded_hold_policy_reason(status, &blockers),
        stability_gate,
        active_age_seconds,
        minimum_hold_seconds: EXPANDED_HOLD_MIN_SECONDS,
        maximum_hold_seconds: EXPANDED_HOLD_MAX_SECONDS,
        hold_started_at_epoch_seconds,
        next_verification_after_epoch_seconds,
        hold_expires_at_epoch_seconds,
        keep_active_allowed,
        next_verification_required,
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

fn default_runtime_expanded_hold_policy_reason(
    status: DnsDefaultRuntimeExpandedHoldPolicyStatus,
    blockers: &[String],
) -> String {
    match status {
        DnsDefaultRuntimeExpandedHoldPolicyStatus::Ready => {
            "expanded default DNS runtime satisfied the session hold policy".into()
        }
        DnsDefaultRuntimeExpandedHoldPolicyStatus::Holding => {
            "expanded default DNS runtime is still inside the minimum hold observation window".into()
        }
        DnsDefaultRuntimeExpandedHoldPolicyStatus::RollbackRecommended => {
            "expanded default DNS runtime should be explicitly rolled back before continuing".into()
        }
        DnsDefaultRuntimeExpandedHoldPolicyStatus::Blocked => blockers
            .first()
            .cloned()
            .unwrap_or_else(|| "expanded default DNS runtime hold policy is blocked".into()),
    }
}
