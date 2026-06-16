use super::*;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum DnsDefaultRuntimeReadinessStatus {
    Ready,
    Degraded,
    Blocked,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum DnsDefaultRuntimeReadinessCheckStatus {
    Passed,
    Warning,
    Failed,
    Skipped,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DnsDefaultRuntimeReadinessCheck {
    pub check_id: String,
    pub status: DnsDefaultRuntimeReadinessCheckStatus,
    pub message: String,
    pub details: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DnsDefaultRuntimeReadinessSummary {
    pub passed: usize,
    pub warnings: usize,
    pub failed: usize,
    pub skipped: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DnsDefaultRuntimeReadinessReport {
    pub status: DnsDefaultRuntimeReadinessStatus,
    pub reason: String,
    pub plan: DnsResolverPlan,
    pub probe_summary: Option<DnsResolverRuntimeProbeSummary>,
    pub checks: Vec<DnsDefaultRuntimeReadinessCheck>,
    pub summary: DnsDefaultRuntimeReadinessSummary,
    pub blockers: Vec<String>,
    pub warnings: Vec<String>,
    pub facts: Vec<String>,
}

pub async fn dns_default_runtime_readiness(
    yaml: Option<String>,
    probe_report: Option<DnsResolverRuntimeProbeReport>,
) -> Result<DnsDefaultRuntimeReadinessReport> {
    let yaml = runtime_dns_shadow_yaml(yaml, "readiness").await?;
    build_dns_default_runtime_readiness_report(&yaml, probe_report)
}

pub fn build_dns_default_runtime_readiness_report(
    yaml: &str,
    probe_report: Option<DnsResolverRuntimeProbeReport>,
) -> Result<DnsDefaultRuntimeReadinessReport> {
    let plan = build_dns_resolver_plan(yaml)?;
    let value: Value = serde_yaml_ng::from_str(yaml).context("YAML syntax error")?;
    let root = value
        .as_mapping()
        .ok_or_else(|| anyhow!("config root must be a YAML mapping"))?;
    let dns = dns_mapping(root);
    let probe_summary = probe_report.as_ref().map(|report| report.summary.clone());
    let mut checks = Vec::new();

    checks.push(default_runtime_plan_check(&plan));
    checks.push(default_runtime_nameserver_coverage_check(&plan));
    if let Some(dns) = dns {
        checks.push(default_runtime_optional_server_section_coverage_check(
            dns,
            "fallback",
            "default_dns_fallback_coverage",
        ));
        checks.push(default_runtime_optional_server_section_coverage_check(
            dns,
            "proxy-server-nameserver",
            "default_dns_proxy_server_nameserver_coverage",
        ));
    }
    checks.push(default_runtime_probe_check(&plan, probe_report.as_ref()));
    checks.push(default_runtime_feature_check(
        "fake-ip",
        &plan.runtime_projection.fake_ip,
    ));
    checks.push(default_runtime_feature_check(
        "fallback-filter",
        &plan.runtime_projection.fallback_filter,
    ));
    checks.push(default_runtime_feature_check(
        "nameserver-policy",
        &plan.runtime_projection.nameserver_policy,
    ));

    let summary = default_runtime_readiness_summary(&checks);
    let status = default_runtime_readiness_status(&summary);
    let blockers = checks
        .iter()
        .filter(|check| check.status == DnsDefaultRuntimeReadinessCheckStatus::Failed)
        .map(|check| check.message.clone())
        .collect::<Vec<_>>();
    let warnings = checks
        .iter()
        .filter(|check| check.status == DnsDefaultRuntimeReadinessCheckStatus::Warning)
        .map(|check| check.message.clone())
        .collect::<Vec<_>>();
    let facts = vec![
        format!(
            "{}/{} nameserver target(s) are supported by the Rust resolver plan",
            plan.nameservers.iter().filter(|item| item.runtime_supported).count(),
            plan.nameservers.len()
        ),
        "readiness gate is read-only and does not switch default DNS runtime".into(),
    ];
    let reason = default_runtime_readiness_reason(status, &summary);

    Ok(DnsDefaultRuntimeReadinessReport {
        status,
        reason,
        plan,
        probe_summary,
        checks,
        summary,
        blockers,
        warnings,
        facts,
    })
}

pub(crate) fn dns_readiness_status_label(status: DnsDefaultRuntimeReadinessStatus) -> &'static str {
    match status {
        DnsDefaultRuntimeReadinessStatus::Ready => "ready",
        DnsDefaultRuntimeReadinessStatus::Degraded => "degraded",
        DnsDefaultRuntimeReadinessStatus::Blocked => "blocked",
    }
}

fn default_runtime_plan_check(plan: &DnsResolverPlan) -> DnsDefaultRuntimeReadinessCheck {
    match plan.status {
        DnsResolverPlanStatus::Ready => readiness_check(
            "default_dns_resolver_plan",
            DnsDefaultRuntimeReadinessCheckStatus::Passed,
            "runtime DNS config can build a Rust resolver plan",
            vec![plan.reason.clone()],
        ),
        DnsResolverPlanStatus::Disabled => readiness_check(
            "default_dns_resolver_plan",
            DnsDefaultRuntimeReadinessCheckStatus::Failed,
            "runtime DNS config disables DNS, so default Rust DNS runtime cannot be enabled",
            vec![plan.reason.clone()],
        ),
        DnsResolverPlanStatus::Rejected => readiness_check(
            "default_dns_resolver_plan",
            DnsDefaultRuntimeReadinessCheckStatus::Failed,
            "runtime DNS config cannot build a complete Rust resolver plan",
            vec![plan.reason.clone()],
        ),
    }
}

fn default_runtime_nameserver_coverage_check(plan: &DnsResolverPlan) -> DnsDefaultRuntimeReadinessCheck {
    let total = plan.nameservers.len();
    let supported = plan
        .nameservers
        .iter()
        .filter(|server| server.runtime_supported)
        .count();
    if total == 0 {
        return readiness_check(
            "default_dns_nameserver_coverage",
            DnsDefaultRuntimeReadinessCheckStatus::Failed,
            "runtime DNS config has no nameserver targets for Rust resolver coverage",
            Vec::new(),
        );
    }
    if supported == total {
        readiness_check(
            "default_dns_nameserver_coverage",
            DnsDefaultRuntimeReadinessCheckStatus::Passed,
            "all runtime DNS nameserver targets are Rust-runtime supported",
            vec![format!("supported={supported}/{total}")],
        )
    } else {
        let unsupported = plan
            .nameservers
            .iter()
            .filter(|server| !server.runtime_supported)
            .map(|server| format!("{} ({})", server.server, server.reason))
            .collect::<Vec<_>>();
        readiness_check(
            "default_dns_nameserver_coverage",
            DnsDefaultRuntimeReadinessCheckStatus::Failed,
            "some runtime DNS nameserver targets are not Rust-runtime supported",
            std::iter::once(format!("supported={supported}/{total}"))
                .chain(unsupported)
                .collect(),
        )
    }
}

fn default_runtime_optional_server_section_coverage_check(
    dns: &Mapping,
    section_key: &str,
    check_id: &str,
) -> DnsDefaultRuntimeReadinessCheck {
    let mut warnings = Vec::new();
    let servers = extract_server_values(dns.get(section_key), &format!("dns.{section_key}"), &mut warnings)
        .into_iter()
        .map(build_nameserver_plan)
        .collect::<Vec<_>>();
    if !warnings.is_empty() {
        return readiness_check(
            check_id,
            DnsDefaultRuntimeReadinessCheckStatus::Failed,
            format!("dns.{section_key} cannot be evaluated for Rust-runtime support"),
            warnings,
        );
    }
    if servers.is_empty() {
        return readiness_check(
            check_id,
            DnsDefaultRuntimeReadinessCheckStatus::Passed,
            format!("dns.{section_key} is not configured"),
            Vec::new(),
        );
    }

    let total = servers.len();
    let supported = servers.iter().filter(|server| server.runtime_supported).count();
    if supported == total {
        readiness_check(
            check_id,
            DnsDefaultRuntimeReadinessCheckStatus::Passed,
            format!("all dns.{section_key} targets are Rust-runtime supported"),
            vec![format!("supported={supported}/{total}")],
        )
    } else {
        let unsupported = servers
            .iter()
            .filter(|server| !server.runtime_supported)
            .map(|server| format!("{} ({})", server.server, server.reason))
            .collect::<Vec<_>>();
        readiness_check(
            check_id,
            DnsDefaultRuntimeReadinessCheckStatus::Failed,
            format!("some dns.{section_key} targets are not Rust-runtime supported"),
            std::iter::once(format!("supported={supported}/{total}"))
                .chain(unsupported)
                .collect(),
        )
    }
}

fn default_runtime_probe_check(
    plan: &DnsResolverPlan,
    probe_report: Option<&DnsResolverRuntimeProbeReport>,
) -> DnsDefaultRuntimeReadinessCheck {
    match probe_report {
        Some(report) if report.summary.runtime_supported_targets == 0 => readiness_check(
            "default_dns_controlled_probe",
            DnsDefaultRuntimeReadinessCheckStatus::Failed,
            "controlled probe has no Rust-supported runtime targets",
            vec![format!("testDomain={}", report.test_domain)],
        ),
        Some(report) if report.summary.healthy_targets == 0 => readiness_check(
            "default_dns_controlled_probe",
            DnsDefaultRuntimeReadinessCheckStatus::Failed,
            "controlled probe did not observe any healthy Rust DNS target",
            vec![format!("testDomain={}", report.test_domain)],
        ),
        Some(report) if !probe_report_matches_plan(plan, report) => readiness_check(
            "default_dns_controlled_probe",
            DnsDefaultRuntimeReadinessCheckStatus::Warning,
            "controlled probe evidence does not match the current readiness plan",
            vec![
                format!("readinessTargets={}", normalized_plan_servers(plan).join(",")),
                format!("probeTargets={}", normalized_probe_servers(report).join(",")),
            ],
        ),
        Some(report) if report.summary.failed_targets > 0 || report.summary.unsupported_targets > 0 => readiness_check(
            "default_dns_controlled_probe",
            DnsDefaultRuntimeReadinessCheckStatus::Warning,
            "controlled probe has partial DNS runtime health",
            vec![
                format!(
                    "healthy={}/{}",
                    report.summary.healthy_targets, report.summary.runtime_supported_targets
                ),
                format!("failed={}", report.summary.failed_targets),
                format!("unsupported={}", report.summary.unsupported_targets),
            ],
        ),
        Some(report) => readiness_check(
            "default_dns_controlled_probe",
            DnsDefaultRuntimeReadinessCheckStatus::Passed,
            "controlled probe observed healthy Rust DNS runtime targets",
            vec![format!(
                "healthy={}/{}",
                report.summary.healthy_targets, report.summary.runtime_supported_targets
            )],
        ),
        None => readiness_check(
            "default_dns_controlled_probe",
            DnsDefaultRuntimeReadinessCheckStatus::Warning,
            "controlled probe evidence is missing for default DNS runtime readiness",
            vec!["run DNS controlled probe before considering a default runtime switch".into()],
        ),
    }
}

fn probe_report_matches_plan(plan: &DnsResolverPlan, report: &DnsResolverRuntimeProbeReport) -> bool {
    normalized_plan_servers(plan) == normalized_probe_servers(report)
}

fn normalized_plan_servers(plan: &DnsResolverPlan) -> Vec<String> {
    let mut servers = plan
        .nameservers
        .iter()
        .map(|server| server.server.clone())
        .collect::<Vec<_>>();
    servers.sort();
    servers
}

fn normalized_probe_servers(report: &DnsResolverRuntimeProbeReport) -> Vec<String> {
    let mut servers = report
        .plan
        .nameservers
        .iter()
        .map(|server| server.server.clone())
        .collect::<Vec<_>>();
    servers.sort();
    servers
}

fn default_runtime_feature_check(
    feature: &str,
    feature_plan: &DnsResolverRuntimeFeaturePlan,
) -> DnsDefaultRuntimeReadinessCheck {
    if !feature_plan.configured {
        return readiness_check(
            format!("default_dns_{feature}_coverage").replace('-', "_"),
            DnsDefaultRuntimeReadinessCheckStatus::Passed,
            format!("{feature} is not configured in runtime DNS"),
            vec![feature_plan.reason.clone()],
        );
    }
    if feature_plan.runtime_applied {
        readiness_check(
            format!("default_dns_{feature}_coverage").replace('-', "_"),
            DnsDefaultRuntimeReadinessCheckStatus::Passed,
            format!("{feature} has Rust runtime coverage"),
            vec![feature_plan.reason.clone()],
        )
    } else {
        readiness_check(
            format!("default_dns_{feature}_coverage").replace('-', "_"),
            DnsDefaultRuntimeReadinessCheckStatus::Failed,
            format!("{feature} is configured but still plan-only"),
            vec![feature_plan.reason.clone()],
        )
    }
}

fn readiness_check(
    check_id: impl Into<String>,
    status: DnsDefaultRuntimeReadinessCheckStatus,
    message: impl Into<String>,
    details: Vec<String>,
) -> DnsDefaultRuntimeReadinessCheck {
    DnsDefaultRuntimeReadinessCheck {
        check_id: check_id.into(),
        status,
        message: message.into(),
        details,
    }
}

fn default_runtime_readiness_summary(checks: &[DnsDefaultRuntimeReadinessCheck]) -> DnsDefaultRuntimeReadinessSummary {
    let mut summary = DnsDefaultRuntimeReadinessSummary::default();
    for check in checks {
        match check.status {
            DnsDefaultRuntimeReadinessCheckStatus::Passed => summary.passed += 1,
            DnsDefaultRuntimeReadinessCheckStatus::Warning => summary.warnings += 1,
            DnsDefaultRuntimeReadinessCheckStatus::Failed => summary.failed += 1,
            DnsDefaultRuntimeReadinessCheckStatus::Skipped => summary.skipped += 1,
        }
    }
    summary
}

fn default_runtime_readiness_status(summary: &DnsDefaultRuntimeReadinessSummary) -> DnsDefaultRuntimeReadinessStatus {
    if summary.failed > 0 {
        DnsDefaultRuntimeReadinessStatus::Blocked
    } else if summary.warnings > 0 {
        DnsDefaultRuntimeReadinessStatus::Degraded
    } else {
        DnsDefaultRuntimeReadinessStatus::Ready
    }
}

fn default_runtime_readiness_reason(
    status: DnsDefaultRuntimeReadinessStatus,
    summary: &DnsDefaultRuntimeReadinessSummary,
) -> String {
    match status {
        DnsDefaultRuntimeReadinessStatus::Ready => "default DNS runtime readiness checks passed".into(),
        DnsDefaultRuntimeReadinessStatus::Degraded => format!(
            "default DNS runtime readiness is incomplete: {} warning(s)",
            summary.warnings
        ),
        DnsDefaultRuntimeReadinessStatus::Blocked => {
            format!(
                "default DNS runtime is blocked by {} readiness check(s)",
                summary.failed
            )
        }
    }
}
