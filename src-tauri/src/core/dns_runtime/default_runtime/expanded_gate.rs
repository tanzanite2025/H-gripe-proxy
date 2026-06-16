use super::*;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum DnsDefaultRuntimeExpandedOptInExecutionGateStatus {
    Ready,
    Blocked,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DnsDefaultRuntimeExpandedOptInExecutionScope {
    pub name: String,
    pub description: String,
    pub max_active_runtime: String,
    pub allowed_execution_mode: String,
    pub requires_user_trigger: bool,
    pub requires_post_execution_verification: bool,
    pub requires_rollback_drill: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DnsDefaultRuntimeExpandedOptInExecutionGateReport {
    pub status: DnsDefaultRuntimeExpandedOptInExecutionGateStatus,
    pub reason: String,
    pub post_execution: DnsDefaultRuntimePostExecutionObservedVerificationReport,
    pub candidate_scope: DnsDefaultRuntimeExpandedOptInExecutionScope,
    pub expansion_allowed: bool,
    pub user_trigger_required: bool,
    pub rollback_drill_required: bool,
    pub failure_audit_required: bool,
    pub auto_rollout: bool,
    pub mutates_runtime: bool,
    pub executed: bool,
    pub reload_mihomo: bool,
    pub blockers: Vec<String>,
    pub warnings: Vec<String>,
    pub facts: Vec<String>,
}

pub async fn dns_default_runtime_expanded_opt_in_execution_gate(
    yaml: Option<String>,
    domain: Option<String>,
    explicit_opt_in: bool,
) -> Result<DnsDefaultRuntimeExpandedOptInExecutionGateReport> {
    let post_execution = dns_default_runtime_post_execution_observed_verification(yaml, domain).await?;
    Ok(build_dns_default_runtime_expanded_opt_in_execution_gate_report(
        post_execution,
        explicit_opt_in,
    ))
}

pub fn build_dns_default_runtime_expanded_opt_in_execution_gate_report(
    post_execution: DnsDefaultRuntimePostExecutionObservedVerificationReport,
    explicit_opt_in: bool,
) -> DnsDefaultRuntimeExpandedOptInExecutionGateReport {
    let mut blockers = Vec::new();

    if !explicit_opt_in {
        blockers.push("explicit opt-in is required before evaluating expanded default DNS runtime execution".into());
    }
    if post_execution.status != DnsDefaultRuntimePostExecutionVerificationStatus::Verified {
        blockers.push(format!(
            "post-execution observed verification is {}; {}",
            default_runtime_post_execution_status_label(post_execution.status),
            post_execution.reason
        ));
    }
    if post_execution.failure_audit.required {
        blockers.push("post-execution failure audit is required before expansion".into());
    }
    if post_execution.rollback_drill.status != DnsDefaultRuntimeRollbackDrillStatus::Ready {
        blockers.push("rollback drill is not ready for expanded execution".into());
    }

    let status = if blockers.is_empty() {
        DnsDefaultRuntimeExpandedOptInExecutionGateStatus::Ready
    } else {
        DnsDefaultRuntimeExpandedOptInExecutionGateStatus::Blocked
    };
    let reason = default_runtime_expanded_opt_in_execution_gate_reason(status, &blockers);
    let candidate_scope = DnsDefaultRuntimeExpandedOptInExecutionScope {
        name: "defaultDnsRuntimeExpandedOptIn".into(),
        description: "allow a later explicit opt-in execution batch to reuse the Rust-owned default DNS runtime path"
            .into(),
        max_active_runtime: "rustDefaultDnsResolver".into(),
        allowed_execution_mode: "explicitUserTriggeredOnly".into(),
        requires_user_trigger: true,
        requires_post_execution_verification: true,
        requires_rollback_drill: true,
    };
    let expansion_allowed = status == DnsDefaultRuntimeExpandedOptInExecutionGateStatus::Ready;
    let mut warnings = post_execution.warnings.clone();
    warnings.extend(post_execution.rollback_drill.warnings.clone());
    let facts = vec![
        "expanded opt-in gate only evaluates Batch Q verification output".into(),
        "expanded opt-in gate does not execute rollout automatically".into(),
        "expanded opt-in gate does not execute rollback automatically".into(),
        "expanded opt-in gate does not write active profile or reload Mihomo".into(),
        "expanded opt-in gate does not touch TUN, transparent proxy, adapters, or protocol runtime".into(),
    ];

    DnsDefaultRuntimeExpandedOptInExecutionGateReport {
        status,
        reason,
        post_execution,
        candidate_scope,
        expansion_allowed,
        user_trigger_required: true,
        rollback_drill_required: true,
        failure_audit_required: !expansion_allowed,
        auto_rollout: false,
        mutates_runtime: false,
        executed: false,
        reload_mihomo: false,
        blockers,
        warnings,
        facts,
    }
}

fn default_runtime_expanded_opt_in_execution_gate_reason(
    status: DnsDefaultRuntimeExpandedOptInExecutionGateStatus,
    blockers: &[String],
) -> String {
    match status {
        DnsDefaultRuntimeExpandedOptInExecutionGateStatus::Ready => {
            "expanded default DNS runtime opt-in gate passed; rollout was not executed".into()
        }
        DnsDefaultRuntimeExpandedOptInExecutionGateStatus::Blocked => blockers
            .first()
            .cloned()
            .unwrap_or_else(|| "expanded default DNS runtime opt-in gate is blocked".into()),
    }
}
