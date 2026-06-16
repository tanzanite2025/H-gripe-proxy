use super::*;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum DnsDefaultRuntimeExpandedStabilityGateStatus {
    Ready,
    Blocked,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DnsDefaultRuntimeExpandedStabilityGateReport {
    pub status: DnsDefaultRuntimeExpandedStabilityGateStatus,
    pub reason: String,
    pub post_execution: DnsDefaultRuntimeExpandedPostExecutionObservedVerificationReport,
    pub keep_active_allowed: bool,
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

pub async fn dns_default_runtime_expanded_stability_gate(
    yaml: Option<String>,
    domain: Option<String>,
    explicit_opt_in: bool,
) -> Result<DnsDefaultRuntimeExpandedStabilityGateReport> {
    let post_execution = dns_default_runtime_expanded_post_execution_observed_verification(yaml, domain).await?;
    Ok(build_dns_default_runtime_expanded_stability_gate_report(
        post_execution,
        explicit_opt_in,
    ))
}

pub fn build_dns_default_runtime_expanded_stability_gate_report(
    post_execution: DnsDefaultRuntimeExpandedPostExecutionObservedVerificationReport,
    explicit_opt_in: bool,
) -> DnsDefaultRuntimeExpandedStabilityGateReport {
    let mut blockers = post_execution.blockers.clone();
    if !explicit_opt_in {
        blockers.push("explicit opt-in is required before evaluating expanded stability".into());
    }
    if post_execution.status != DnsDefaultRuntimeExpandedPostExecutionVerificationStatus::Verified {
        blockers.push(format!(
            "expanded post-execution verification is {}; {}",
            expanded_post_execution_status_label(post_execution.status),
            post_execution.reason
        ));
    }
    if post_execution.failure_audit.required {
        blockers.push("expanded post-execution failure audit is required before keep-active decision".into());
    }
    if post_execution.rollback_drill.status != DnsDefaultRuntimeExpandedRollbackDrillStatus::Ready {
        blockers.push("expanded rollback drill is not ready for keep-active decision".into());
    }
    if post_execution.active_state.as_ref().map(|state| state.state.as_str()) != Some("expandedActiveProfileReloaded") {
        blockers.push("active default DNS runtime state is not expandedActiveProfileReloaded".into());
    }

    let status = if blockers.is_empty() {
        DnsDefaultRuntimeExpandedStabilityGateStatus::Ready
    } else {
        DnsDefaultRuntimeExpandedStabilityGateStatus::Blocked
    };
    let keep_active_allowed = status == DnsDefaultRuntimeExpandedStabilityGateStatus::Ready;
    let rollback_recommended = !keep_active_allowed && post_execution.active_state.is_some();
    let recommended_action = if keep_active_allowed {
        "keepExpandedRuntimeActiveForCurrentSession".into()
    } else if rollback_recommended {
        "runExplicitExpandedRollbackBeforeContinuing".into()
    } else {
        "completeExpandedExecutionBeforeStabilityGate".into()
    };
    let mut warnings = post_execution.warnings.clone();
    if keep_active_allowed {
        warnings.push(
            "expanded stability gate is session-scoped; it does not promote Rust DNS as permanent default".into(),
        );
    }
    let facts = vec![
        "expanded stability gate consumes Batch U expanded post-execution verification".into(),
        "expanded stability gate only decides whether the current active reload can remain active".into(),
        "expanded stability gate never auto-rolls out or auto-rolls back".into(),
        "expanded stability gate does not touch TUN, transparent proxy, adapters, or protocol runtime".into(),
    ];

    DnsDefaultRuntimeExpandedStabilityGateReport {
        status,
        reason: default_runtime_expanded_stability_gate_reason(status, &blockers),
        post_execution,
        keep_active_allowed,
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

fn default_runtime_expanded_stability_gate_reason(
    status: DnsDefaultRuntimeExpandedStabilityGateStatus,
    blockers: &[String],
) -> String {
    match status {
        DnsDefaultRuntimeExpandedStabilityGateStatus::Ready => {
            "expanded default DNS runtime is verified enough to remain active for this session".into()
        }
        DnsDefaultRuntimeExpandedStabilityGateStatus::Blocked => blockers
            .first()
            .cloned()
            .unwrap_or_else(|| "expanded default DNS runtime stability gate is blocked".into()),
    }
}

fn expanded_post_execution_status_label(
    status: DnsDefaultRuntimeExpandedPostExecutionVerificationStatus,
) -> &'static str {
    match status {
        DnsDefaultRuntimeExpandedPostExecutionVerificationStatus::Verified => "verified",
        DnsDefaultRuntimeExpandedPostExecutionVerificationStatus::Failed => "failed",
        DnsDefaultRuntimeExpandedPostExecutionVerificationStatus::Blocked => "blocked",
    }
}
