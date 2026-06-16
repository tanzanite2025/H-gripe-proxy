use super::*;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum DnsDefaultRuntimeShadowEvidenceStatus {
    Matched,
    Mismatched,
    Blocked,
    Incomplete,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DnsDefaultRuntimeShadowQueryEvidence {
    pub domain: String,
    pub rust_report: DnsResolverRuntimeQueryReport,
    pub system_result: DnsQueryResult,
    pub ip_match: bool,
    pub latency_delta_ms: i64,
    pub mismatch_reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DnsDefaultRuntimeShadowEvidenceReport {
    pub status: DnsDefaultRuntimeShadowEvidenceStatus,
    pub reason: String,
    pub readiness: DnsDefaultRuntimeReadinessReport,
    pub query: DnsDefaultRuntimeShadowQueryEvidence,
    pub blockers: Vec<String>,
    pub warnings: Vec<String>,
    pub facts: Vec<String>,
}

pub async fn dns_default_runtime_shadow_evidence(
    yaml: Option<String>,
    domain: Option<String>,
) -> Result<DnsDefaultRuntimeShadowEvidenceReport> {
    let yaml = runtime_dns_shadow_yaml(yaml, "shadow evidence").await?;
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

    Ok(build_dns_default_runtime_shadow_evidence_report(
        readiness,
        rust_report,
        system_result,
    ))
}

pub fn build_dns_default_runtime_shadow_evidence_report(
    readiness: DnsDefaultRuntimeReadinessReport,
    rust_report: DnsResolverRuntimeQueryReport,
    system_result: DnsQueryResult,
) -> DnsDefaultRuntimeShadowEvidenceReport {
    let query = default_runtime_shadow_query_evidence(rust_report, system_result);
    let mut blockers = readiness.blockers.clone();
    let mut warnings = readiness.warnings.clone();
    if readiness.status != DnsDefaultRuntimeReadinessStatus::Ready {
        blockers.push("readiness gate is not ready; shadow evidence cannot prove default runtime replacement".into());
    }
    if let Some(reason) = query.mismatch_reason.clone() {
        warnings.push(reason);
    }

    let status = default_runtime_shadow_status(readiness.status, &query);
    let reason = default_runtime_shadow_reason(status, &query);
    let facts = vec![
        format!("shadow domain={}", query.domain),
        format!("rust attempted {} target(s)", query.rust_report.attempted_servers.len()),
        "shadow evidence is read-only and does not switch default DNS runtime".into(),
    ];

    DnsDefaultRuntimeShadowEvidenceReport {
        status,
        reason,
        readiness,
        query,
        blockers,
        warnings,
        facts,
    }
}

fn default_runtime_shadow_query_evidence(
    rust_report: DnsResolverRuntimeQueryReport,
    system_result: DnsQueryResult,
) -> DnsDefaultRuntimeShadowQueryEvidence {
    let rust_result = rust_report.result.as_ref();
    let ip_match = rust_result
        .filter(|result| result.success && system_result.success)
        .map(|result| result.ip == system_result.ip)
        .unwrap_or(false);
    let latency_delta_ms = rust_result
        .map(|result| result.latency as i64 - system_result.latency as i64)
        .unwrap_or(-(system_result.latency as i64));
    let mismatch_reason = default_runtime_shadow_mismatch_reason(rust_result, &system_result, ip_match);

    DnsDefaultRuntimeShadowQueryEvidence {
        domain: rust_report.domain.clone(),
        rust_report,
        system_result,
        ip_match,
        latency_delta_ms,
        mismatch_reason,
    }
}

fn default_runtime_shadow_mismatch_reason(
    rust_result: Option<&DnsQueryResult>,
    system_result: &DnsQueryResult,
    ip_match: bool,
) -> Option<String> {
    match (rust_result, system_result.success, ip_match) {
        (None, _, _) => Some("Rust resolver did not return a DNS result".into()),
        (Some(result), _, _) if !result.success => Some(format!(
            "Rust resolver failed: {}",
            result.error.as_deref().unwrap_or("unknown error")
        )),
        (Some(_), false, _) => Some(format!(
            "system resolver failed: {}",
            system_result.error.as_deref().unwrap_or("unknown error")
        )),
        (Some(result), true, false) => Some(format!(
            "Rust resolver returned {}, system resolver returned {}",
            result.ip, system_result.ip
        )),
        (Some(_), true, true) => None,
    }
}

fn default_runtime_shadow_status(
    readiness_status: DnsDefaultRuntimeReadinessStatus,
    query: &DnsDefaultRuntimeShadowQueryEvidence,
) -> DnsDefaultRuntimeShadowEvidenceStatus {
    if readiness_status != DnsDefaultRuntimeReadinessStatus::Ready {
        DnsDefaultRuntimeShadowEvidenceStatus::Blocked
    } else if query.mismatch_reason.is_some() {
        if query.rust_report.result.is_some() && query.system_result.success {
            DnsDefaultRuntimeShadowEvidenceStatus::Mismatched
        } else {
            DnsDefaultRuntimeShadowEvidenceStatus::Incomplete
        }
    } else {
        DnsDefaultRuntimeShadowEvidenceStatus::Matched
    }
}

fn default_runtime_shadow_reason(
    status: DnsDefaultRuntimeShadowEvidenceStatus,
    query: &DnsDefaultRuntimeShadowQueryEvidence,
) -> String {
    match status {
        DnsDefaultRuntimeShadowEvidenceStatus::Matched => {
            "Rust resolver shadow result matches the system resolver result".into()
        }
        DnsDefaultRuntimeShadowEvidenceStatus::Mismatched => query
            .mismatch_reason
            .clone()
            .unwrap_or_else(|| "Rust and system resolver results differ".into()),
        DnsDefaultRuntimeShadowEvidenceStatus::Blocked => {
            "readiness blockers prevent this shadow evidence from proving default DNS replacement".into()
        }
        DnsDefaultRuntimeShadowEvidenceStatus::Incomplete => query
            .mismatch_reason
            .clone()
            .unwrap_or_else(|| "shadow DNS evidence is incomplete".into()),
    }
}

pub(crate) fn dns_shadow_status_label(status: DnsDefaultRuntimeShadowEvidenceStatus) -> &'static str {
    match status {
        DnsDefaultRuntimeShadowEvidenceStatus::Matched => "matched",
        DnsDefaultRuntimeShadowEvidenceStatus::Mismatched => "mismatched",
        DnsDefaultRuntimeShadowEvidenceStatus::Blocked => "blocked",
        DnsDefaultRuntimeShadowEvidenceStatus::Incomplete => "incomplete",
    }
}
