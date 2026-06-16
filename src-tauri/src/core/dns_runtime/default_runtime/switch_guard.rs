use super::*;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum DnsDefaultRuntimeOptInSwitchGuardStatus {
    Ready,
    Blocked,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DnsDefaultRuntimeRollbackPlan {
    pub required: bool,
    pub supported: bool,
    pub strategy: String,
    pub previous_runtime: String,
    pub candidate_runtime: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DnsDefaultRuntimeOptInSwitchGuardReport {
    pub status: DnsDefaultRuntimeOptInSwitchGuardStatus,
    pub reason: String,
    pub readiness: DnsDefaultRuntimeReadinessReport,
    pub shadow_evidence: DnsDefaultRuntimeShadowEvidenceReport,
    pub rollback_plan: DnsDefaultRuntimeRollbackPlan,
    pub explicit_opt_in: bool,
    pub mutates_runtime: bool,
    pub activation_mode: String,
    pub blockers: Vec<String>,
    pub warnings: Vec<String>,
    pub facts: Vec<String>,
}

pub async fn dns_default_runtime_opt_in_switch_guard(
    yaml: Option<String>,
    domain: Option<String>,
    explicit_opt_in: bool,
) -> Result<DnsDefaultRuntimeOptInSwitchGuardReport> {
    let shadow_evidence = dns_default_runtime_shadow_evidence(yaml, domain).await?;
    Ok(build_dns_default_runtime_opt_in_switch_guard_report(
        shadow_evidence,
        explicit_opt_in,
    ))
}

pub fn build_dns_default_runtime_opt_in_switch_guard_report(
    shadow_evidence: DnsDefaultRuntimeShadowEvidenceReport,
    explicit_opt_in: bool,
) -> DnsDefaultRuntimeOptInSwitchGuardReport {
    let readiness = shadow_evidence.readiness.clone();
    let rollback_plan = default_runtime_rollback_plan();
    let mut blockers = Vec::new();
    let mut warnings = Vec::new();

    if !explicit_opt_in {
        blockers.push("explicit user opt-in is required before default DNS runtime switch preflight can pass".into());
    }
    if readiness.status != DnsDefaultRuntimeReadinessStatus::Ready {
        blockers.push(format!(
            "readiness gate is {}",
            dns_readiness_status_label(readiness.status)
        ));
    }
    if matches!(
        shadow_evidence.status,
        DnsDefaultRuntimeShadowEvidenceStatus::Blocked | DnsDefaultRuntimeShadowEvidenceStatus::Incomplete
    ) {
        blockers.push(format!(
            "shadow evidence is {}",
            dns_shadow_status_label(shadow_evidence.status)
        ));
    }
    if !rollback_plan.supported {
        blockers.push("runtime rollback plan is not supported".into());
    }
    if shadow_evidence.status == DnsDefaultRuntimeShadowEvidenceStatus::Mismatched {
        warnings.push("shadow evidence is mismatched; opt-in switch remains guarded and experimental".into());
    }
    warnings.extend(shadow_evidence.warnings.clone());

    let status = if blockers.is_empty() {
        DnsDefaultRuntimeOptInSwitchGuardStatus::Ready
    } else {
        DnsDefaultRuntimeOptInSwitchGuardStatus::Blocked
    };
    let reason = default_runtime_opt_in_switch_reason(status, &blockers);
    let facts = vec![
        "guard preflight is explicit opt-in only".into(),
        "guard preflight does not switch default DNS runtime".into(),
        format!("rollback strategy={}", rollback_plan.strategy),
        format!("shadow status={}", dns_shadow_status_label(shadow_evidence.status)),
    ];

    DnsDefaultRuntimeOptInSwitchGuardReport {
        status,
        reason,
        readiness,
        shadow_evidence,
        rollback_plan,
        explicit_opt_in,
        mutates_runtime: false,
        activation_mode: "preflightOnly".into(),
        blockers,
        warnings,
        facts,
    }
}

fn default_runtime_rollback_plan() -> DnsDefaultRuntimeRollbackPlan {
    DnsDefaultRuntimeRollbackPlan {
        required: true,
        supported: true,
        strategy: "restoreMihomoManagedDefaultDnsRuntime".into(),
        previous_runtime: "mihomoManagedDefaultDns".into(),
        candidate_runtime: "rustDefaultDnsResolver".into(),
    }
}

fn default_runtime_opt_in_switch_reason(
    status: DnsDefaultRuntimeOptInSwitchGuardStatus,
    blockers: &[String],
) -> String {
    match status {
        DnsDefaultRuntimeOptInSwitchGuardStatus::Ready => {
            "default DNS runtime switch guard passed; no runtime switch was executed".into()
        }
        DnsDefaultRuntimeOptInSwitchGuardStatus::Blocked => blockers
            .first()
            .cloned()
            .unwrap_or_else(|| "default DNS runtime switch guard is blocked".into()),
    }
}
