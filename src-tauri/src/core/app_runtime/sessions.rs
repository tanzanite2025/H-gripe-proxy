use super::*;
use crate::core::connection_metrics::{self, ConnectionAttributionCandidate, ConnectionMetricsSnapshot};
use anyhow::Result;
use smartstring::alias::String;
use std::collections::BTreeSet;

pub async fn list_app_runtime_sessions(app_id: Option<String>) -> Result<Vec<AppRuntimeSessionRecord>> {
    let state = read_app_runtime_state_document().await?;
    let app_id = app_id
        .as_deref()
        .map(|value| normalize_id(value, "app_id"))
        .transpose()?;
    let mut sessions: Vec<_> = state
        .sessions
        .into_iter()
        .filter(|session| app_id.as_ref().is_none_or(|app_id| &session.app_id == app_id))
        .collect();
    sessions.sort_by(|left, right| {
        right
            .started_at
            .cmp(&left.started_at)
            .then_with(|| right.session_id.cmp(&left.session_id))
    });
    Ok(sessions)
}

pub async fn start_app_runtime_session(request: AppRuntimePlanRequest) -> Result<AppRuntimeSessionStartReport> {
    let app_id = normalize_id(&request.app_id, "app_id")?;
    let session_id = runtime_session_id(&app_id, request.session_id.as_deref())?;
    let request = AppRuntimePlanRequest {
        app_id,
        session_id: Some(session_id.clone()),
    };

    let mut state = read_app_runtime_state_document().await?;
    let diagnostics = diagnose_app_runtime(&state, request)?;
    let session = session_record_from_diagnostics(session_id, &diagnostics);
    upsert_by(&mut state.sessions, session.clone(), |stored| stored.session_id.clone());
    save_app_runtime_state_document(&state).await?;

    Ok(AppRuntimeSessionStartReport { session, diagnostics })
}

pub async fn finish_app_runtime_session(request: AppRuntimeSessionFinishRequest) -> Result<AppRuntimeSessionRecord> {
    let session_id = normalize_id(&request.session_id, "session_id")?;
    if matches!(request.status, AppRuntimeSessionStatus::Planned) {
        bail!("finished app runtime session status cannot be planned");
    }

    let mut state = read_app_runtime_state_document().await?;
    let Some(session) = state
        .sessions
        .iter_mut()
        .find(|session| session.session_id == session_id)
    else {
        bail!("app runtime session `{session_id}` was not found");
    };
    session.status = request.status;
    session.ended_at = Some(now_millis());
    if let Some(reason) = request.reason.as_ref().filter(|reason| !reason.trim().is_empty()) {
        session.reason = reason.trim().into();
    }
    let session = session.clone();
    save_app_runtime_state_document(&state).await?;
    Ok(session)
}

pub async fn record_app_runtime_session_observation(session_id: &str) -> Result<AppRuntimeSessionRecord> {
    let session_id = normalize_id(session_id, "session_id")?;
    let metrics = connection_metrics::get_connection_metrics_snapshot().await;

    let mut state = read_app_runtime_state_document().await?;
    let Some(session) = state
        .sessions
        .iter_mut()
        .find(|session| session.session_id == session_id)
    else {
        bail!("app runtime session `{session_id}` was not found");
    };
    let observation = session_observation_from_metrics(session, &metrics);
    session.observations.push(observation);
    let session = session.clone();
    save_app_runtime_state_document(&state).await?;
    Ok(session)
}

pub async fn evaluate_app_runtime_session(session_id: &str) -> Result<AppRuntimeSessionEvaluationReport> {
    let session_id = normalize_id(session_id, "session_id")?;
    let state = read_app_runtime_state_document().await?;
    let Some(session) = state.sessions.iter().find(|session| session.session_id == session_id) else {
        bail!("app runtime session `{session_id}` was not found");
    };

    Ok(evaluation_report_from_session(session))
}

pub async fn verify_app_runtime_session_leak(session_id: &str) -> Result<AppRuntimeSessionLeakReport> {
    let session_id = normalize_id(session_id, "session_id")?;
    let state = read_app_runtime_state_document().await?;
    let Some(session) = state.sessions.iter().find(|session| session.session_id == session_id) else {
        bail!("app runtime session `{session_id}` was not found");
    };

    Ok(leak_report_from_session(&state, session))
}

pub(super) fn runtime_session_id(app_id: &str, requested_session_id: Option<&str>) -> Result<String> {
    if let Some(session_id) = requested_session_id.filter(|session_id| !session_id.trim().is_empty()) {
        return normalize_id(session_id, "session_id");
    }
    Ok(format!("{app_id}-{}", now_millis()).into())
}

pub(super) fn session_record_from_diagnostics(
    session_id: String,
    diagnostics: &AppRuntimeDiagnosticsReport,
) -> AppRuntimeSessionRecord {
    let status = if diagnostics.status == AppRuntimeDiagnosticStatus::Blocked
        || diagnostics.plan.status == AppRuntimePlanStatus::Rejected
    {
        AppRuntimeSessionStatus::Blocked
    } else {
        AppRuntimeSessionStatus::Planned
    };

    AppRuntimeSessionRecord {
        session_id,
        app_id: diagnostics.app_id.clone(),
        status,
        plan_status: diagnostics.plan.status,
        diagnostics_status: diagnostics.status,
        diagnostics_summary: diagnostics.summary.clone(),
        reason: diagnostics.reason.clone(),
        started_at: now_millis(),
        ended_at: None,
        projected_rules: diagnostics
            .mihomo_projection
            .rules
            .iter()
            .map(|rule| rule.rule.clone())
            .collect(),
        projected_proxy_groups: diagnostics
            .mihomo_projection
            .proxy_groups
            .iter()
            .map(|group| group.name.clone())
            .collect(),
        observations: Vec::new(),
        facts: diagnostics.facts.clone(),
        warnings: diagnostics.warnings.clone(),
    }
}

pub(super) fn session_observation_from_metrics(
    session: &AppRuntimeSessionRecord,
    metrics: &ConnectionMetricsSnapshot,
) -> AppRuntimeSessionObservationRecord {
    let recorded_at = now_millis();
    let attribution_candidates = metrics
        .attribution_candidates
        .iter()
        .filter_map(|candidate| session_attribution_candidate(session, candidate))
        .collect::<Vec<_>>();
    let attribution_status = if metrics.attribution_candidates.is_empty() {
        AppRuntimeSessionAttributionStatus::Unattributed
    } else if attribution_candidates.is_empty() {
        AppRuntimeSessionAttributionStatus::AppMismatch
    } else {
        AppRuntimeSessionAttributionStatus::AppMatched
    };
    let mut facts = vec![
        "Observation snapshots reuse the Rust connection metrics path".into(),
        format!(
            "inspected {} connection attribution candidate(s)",
            metrics.attribution_candidates.len()
        )
        .into(),
    ];
    if !attribution_candidates.is_empty() {
        facts.push(
            format!(
                "matched {} candidate(s) against session projected rules or proxy groups",
                attribution_candidates.len()
            )
            .into(),
        );
    }

    AppRuntimeSessionObservationRecord {
        observation_id: format!("{}-{recorded_at}", session.session_id).into(),
        session_id: session.session_id.clone(),
        recorded_at,
        source: AppRuntimeSessionObservationSource::ConnectionMetricsSnapshot,
        attribution_status,
        traffic: AppRuntimeSessionTrafficObservation {
            upload_total: metrics.traffic.upload_total,
            download_total: metrics.traffic.download_total,
            upload_speed: metrics.traffic.upload_speed,
            download_speed: metrics.traffic.download_speed,
            active_connection_count: metrics.traffic.active_connection_count,
            closed_since_last: metrics.traffic.closed_since_last,
            memory: metrics.traffic.memory,
            stale: metrics.stale,
        },
        connection_speed_count: metrics.speeds.len(),
        attribution_candidates,
        facts,
        warnings: session_observation_warnings(attribution_status),
    }
}

pub(super) fn session_attribution_candidate(
    session: &AppRuntimeSessionRecord,
    candidate: &ConnectionAttributionCandidate,
) -> Option<AppRuntimeSessionAttributionCandidate> {
    let matched_by = session_candidate_matches(session, candidate);
    if matched_by.is_empty() {
        return None;
    }

    Some(AppRuntimeSessionAttributionCandidate {
        connection_id: candidate.id.clone().into(),
        process: candidate.process.clone().into(),
        process_path: candidate.process_path.clone().into(),
        host: candidate.host.clone().into(),
        rule: candidate.rule.clone().into(),
        rule_payload: candidate.rule_payload.clone().into(),
        chains: candidate.chains.iter().map(|chain| chain.as_str().into()).collect(),
        upload: candidate.upload,
        download: candidate.download,
        matched_by,
    })
}

pub(super) fn session_candidate_matches(
    session: &AppRuntimeSessionRecord,
    candidate: &ConnectionAttributionCandidate,
) -> Vec<String> {
    let mut matched_by = Vec::new();
    for proxy_group in &session.projected_proxy_groups {
        if candidate
            .chains
            .iter()
            .any(|chain| chain.as_str() == proxy_group.as_str())
        {
            matched_by.push(format!("proxyGroup:{proxy_group}").into());
        }
    }
    for rule in &session.projected_rules {
        if projected_rule_matches_candidate(rule, candidate) {
            matched_by.push(format!("projectedRule:{rule}").into());
        }
    }
    matched_by
}

pub(super) fn projected_rule_matches_candidate(rule: &str, candidate: &ConnectionAttributionCandidate) -> bool {
    let mut parts = rule.splitn(3, ',');
    let Some(kind) = parts.next() else {
        return false;
    };
    let Some(payload) = parts.next() else {
        return false;
    };
    match kind {
        "PROCESS-NAME" => {
            candidate.process.eq_ignore_ascii_case(payload)
                || candidate
                    .process_path
                    .rsplit(['/', '\\'])
                    .next()
                    .is_some_and(|name| name.eq_ignore_ascii_case(payload))
        }
        "PROCESS-PATH" => candidate.process_path.eq_ignore_ascii_case(payload),
        _ => false,
    }
}

pub(super) fn session_observation_warnings(status: AppRuntimeSessionAttributionStatus) -> Vec<String> {
    match status {
        AppRuntimeSessionAttributionStatus::AppMatched => Vec::new(),
        AppRuntimeSessionAttributionStatus::AppMismatch => {
            vec!["Latest connection metadata did not match this session's projected rules or proxy groups".into()]
        }
        AppRuntimeSessionAttributionStatus::Unattributed => {
            vec!["No connection attribution candidates are available in the latest metrics snapshot".into()]
        }
    }
}

pub(super) fn evaluation_report_from_session(session: &AppRuntimeSessionRecord) -> AppRuntimeSessionEvaluationReport {
    let summary = session_evaluation_summary(session);
    let status = session_evaluation_status(session, &summary);
    let warnings = session_evaluation_warnings(session, status, &summary);
    let reason = session_evaluation_reason(status, &summary);
    let facts = vec![
        format!("session status is {:?}", session.status).into(),
        format!("evaluated {} recorded observation(s)", summary.observation_count).into(),
        format!(
            "matched {} attribution candidate(s)",
            summary.attribution_candidate_count
        )
        .into(),
    ];

    AppRuntimeSessionEvaluationReport {
        session_id: session.session_id.clone(),
        app_id: session.app_id.clone(),
        status,
        reason,
        summary,
        facts,
        warnings,
    }
}

pub(super) fn session_evaluation_summary(session: &AppRuntimeSessionRecord) -> AppRuntimeSessionEvaluationSummary {
    let mut summary = AppRuntimeSessionEvaluationSummary {
        observation_count: session.observations.len(),
        ..AppRuntimeSessionEvaluationSummary::default()
    };
    let mut observed_chains = BTreeSet::new();
    let mut observed_hosts = BTreeSet::new();
    let mut matched_by = BTreeSet::new();

    for observation in &session.observations {
        match observation.attribution_status {
            AppRuntimeSessionAttributionStatus::AppMatched => summary.matched_observations += 1,
            AppRuntimeSessionAttributionStatus::AppMismatch => summary.mismatch_observations += 1,
            AppRuntimeSessionAttributionStatus::Unattributed => summary.unattributed_observations += 1,
        }
        if observation.traffic.stale {
            summary.stale_observations += 1;
        }
        summary.upload_total = summary.upload_total.max(observation.traffic.upload_total);
        summary.download_total = summary.download_total.max(observation.traffic.download_total);
        summary.max_active_connections = summary
            .max_active_connections
            .max(observation.traffic.active_connection_count);
        summary.attribution_candidate_count += observation.attribution_candidates.len();

        for candidate in &observation.attribution_candidates {
            if !candidate.host.is_empty() {
                observed_hosts.insert(candidate.host.clone());
            }
            for chain in &candidate.chains {
                observed_chains.insert(chain.clone());
            }
            for matcher in &candidate.matched_by {
                matched_by.insert(matcher.clone());
            }
        }
    }

    summary.observed_chains = observed_chains.into_iter().collect();
    summary.observed_hosts = observed_hosts.into_iter().collect();
    summary.matched_by = matched_by.into_iter().collect();
    summary
}

pub(super) fn session_evaluation_status(
    session: &AppRuntimeSessionRecord,
    summary: &AppRuntimeSessionEvaluationSummary,
) -> AppRuntimeDiagnosticStatus {
    if matches!(
        session.status,
        AppRuntimeSessionStatus::Blocked | AppRuntimeSessionStatus::Failed
    ) {
        return AppRuntimeDiagnosticStatus::Blocked;
    }
    if summary.observation_count == 0
        || summary.mismatch_observations > 0
        || summary.unattributed_observations > 0
        || summary.stale_observations > 0
    {
        return AppRuntimeDiagnosticStatus::Degraded;
    }
    AppRuntimeDiagnosticStatus::Healthy
}

pub(super) fn session_evaluation_reason(
    status: AppRuntimeDiagnosticStatus,
    summary: &AppRuntimeSessionEvaluationSummary,
) -> String {
    match status {
        AppRuntimeDiagnosticStatus::Healthy => {
            "all recorded app session observations matched projected runtime artifacts".into()
        }
        AppRuntimeDiagnosticStatus::Degraded if summary.observation_count == 0 => {
            "app runtime session has no recorded observations yet".into()
        }
        AppRuntimeDiagnosticStatus::Degraded => {
            "app runtime session observations require attribution or freshness review".into()
        }
        AppRuntimeDiagnosticStatus::Blocked => {
            "app runtime session was blocked or failed before a healthy evaluation could be confirmed".into()
        }
    }
}

pub(super) fn session_evaluation_warnings(
    session: &AppRuntimeSessionRecord,
    status: AppRuntimeDiagnosticStatus,
    summary: &AppRuntimeSessionEvaluationSummary,
) -> Vec<String> {
    let mut warnings = Vec::new();
    if matches!(
        session.status,
        AppRuntimeSessionStatus::Blocked | AppRuntimeSessionStatus::Failed
    ) {
        warnings.push("session ended in blocked or failed state".into());
    }
    if summary.observation_count == 0 {
        warnings.push("no connection metrics observations have been recorded for this session".into());
    }
    if summary.mismatch_observations > 0 {
        warnings.push("one or more observations had connection metadata but no app projection match".into());
    }
    if summary.unattributed_observations > 0 {
        warnings.push("one or more observations had no attribution candidates".into());
    }
    if summary.stale_observations > 0 {
        warnings.push("one or more observations were marked stale by connection metrics".into());
    }
    if status == AppRuntimeDiagnosticStatus::Healthy && summary.attribution_candidate_count == 0 {
        warnings.push("healthy status requires matched observations with attribution candidates".into());
    }
    warnings
}

pub(super) fn leak_report_from_session(
    state: &AppRuntimeStateDocument,
    session: &AppRuntimeSessionRecord,
) -> AppRuntimeSessionLeakReport {
    let plan = explain_app_runtime_plan(
        state,
        AppRuntimePlanRequest {
            app_id: session.app_id.clone(),
            session_id: Some(session.session_id.clone()),
        },
    );
    let summary = session_evaluation_summary(session);
    let routing_intent = plan.routing_intent;
    let checks = vec![
        leak_proxy_check(routing_intent, session, &summary),
        leak_dns_check(&plan),
        leak_exit_check(session, &plan, routing_intent, &summary),
        leak_node_pool_check(routing_intent, &plan, session, &summary),
    ];
    let leak_summary = leak_summary_counts(&checks);
    let status = leak_status(&leak_summary);
    let reason = leak_reason(status, &leak_summary);
    let warnings = leak_report_warnings(&checks);
    let facts = vec![
        "App-scoped leak verification is planning-only; no TUN, Mihomo runtime, or live exit/DNS probe is performed"
            .into(),
        format!(
            "evaluated {} recorded observation(s) against projected proxy/DNS/node-pool artifacts",
            summary.observation_count
        )
        .into(),
        match routing_intent {
            Some(intent) => format!("routing intent under verification is `{}`", intent_label(intent)).into(),
            None => "routing intent under verification is unavailable because the plan was rejected".into(),
        },
    ];

    AppRuntimeSessionLeakReport {
        session_id: session.session_id.clone(),
        app_id: session.app_id.clone(),
        status,
        reason,
        routing_intent,
        evaluation_summary: summary,
        checks,
        summary: leak_summary,
        facts,
        warnings,
    }
}

pub(super) fn leak_proxy_check(
    routing_intent: Option<AppRoutingIntent>,
    session: &AppRuntimeSessionRecord,
    summary: &AppRuntimeSessionEvaluationSummary,
) -> AppRuntimeLeakCheck {
    let intent = routing_intent.unwrap_or(AppRoutingIntent::Direct);
    if !requires_node_pool(intent) {
        return leak_check(
            AppRuntimeLeakDimension::ProxyLeak,
            AppRuntimeLeakCheckStatus::NotApplicable,
            format!(
                "routing intent `{}` does not route through a proxy group",
                intent_label(intent)
            )
            .into(),
            vec!["direct or reject routing has no proxy tunnel to leak from".into()],
            Vec::new(),
        );
    }
    if session.projected_proxy_groups.is_empty() {
        return leak_check(
            AppRuntimeLeakDimension::ProxyLeak,
            AppRuntimeLeakCheckStatus::Warn,
            "proxy routing intent has no projected proxy group to verify traffic against".into(),
            Vec::new(),
            vec!["projected proxy groups are empty, so proxy routing cannot be confirmed".into()],
        );
    }
    if summary.observation_count == 0 {
        return leak_check(
            AppRuntimeLeakDimension::ProxyLeak,
            AppRuntimeLeakCheckStatus::Warn,
            "no observations recorded, so proxy routing has not been verified".into(),
            Vec::new(),
            vec!["record session observations before verifying proxy routing".into()],
        );
    }

    let direct_egress = summary
        .observed_chains
        .iter()
        .filter(|chain| is_builtin_outbound(chain))
        .cloned()
        .collect::<Vec<_>>();
    if summary.mismatch_observations > 0 || !direct_egress.is_empty() {
        let mut warnings = Vec::new();
        if summary.mismatch_observations > 0 {
            warnings.push("connection metadata matched the app but not its projected proxy group or rules".into());
        }
        if !direct_egress.is_empty() {
            warnings.push(
                format!(
                    "observed direct or reject egress chains: {}",
                    join_chain_list(&direct_egress)
                )
                .into(),
            );
        }
        return leak_check(
            AppRuntimeLeakDimension::ProxyLeak,
            AppRuntimeLeakCheckStatus::Fail,
            "app traffic appears to bypass the projected proxy group (possible proxy leak)".into(),
            Vec::new(),
            warnings,
        );
    }
    if summary.matched_observations == 0 {
        return leak_check(
            AppRuntimeLeakDimension::ProxyLeak,
            AppRuntimeLeakCheckStatus::Warn,
            "observations were recorded but none were attributed to the app's projected proxy group".into(),
            Vec::new(),
            vec!["no attribution candidates matched the projected proxy group or rules".into()],
        );
    }
    if summary.unattributed_observations > 0 || summary.stale_observations > 0 {
        return leak_check(
            AppRuntimeLeakDimension::ProxyLeak,
            AppRuntimeLeakCheckStatus::Warn,
            "some observations were unattributed or stale, so proxy routing is only partially verified".into(),
            vec![
                format!(
                    "{} matched, {} unattributed, {} stale observation(s)",
                    summary.matched_observations, summary.unattributed_observations, summary.stale_observations
                )
                .into(),
            ],
            vec!["partial observation coverage prevents a fully confirmed proxy routing result".into()],
        );
    }

    leak_check(
        AppRuntimeLeakDimension::ProxyLeak,
        AppRuntimeLeakCheckStatus::Pass,
        format!(
            "all {} attributed observation(s) traversed the projected proxy group(s)",
            summary.matched_observations
        )
        .into(),
        vec![format!("observed proxy chains: {}", join_chain_list(&summary.observed_chains)).into()],
        Vec::new(),
    )
}

pub(super) fn leak_dns_check(plan: &AppRuntimePlan) -> AppRuntimeLeakCheck {
    let planning_fact: String = "DNS leak verification is planning-only; no live DNS query is issued".into();
    let require_dns = plan
        .security_profile
        .as_ref()
        .map(|profile| profile.controls.require_dns_profile)
        .unwrap_or(false);
    let Some(dns) = plan.dns_profile.as_ref() else {
        if require_dns {
            return leak_check(
                AppRuntimeLeakDimension::DnsLeak,
                AppRuntimeLeakCheckStatus::Fail,
                "security profile requires a DNS profile but none is bound; DNS may leak to the system resolver".into(),
                vec![planning_fact],
                vec!["bind a DNS profile to satisfy the security profile and prevent DNS leaks".into()],
            );
        }
        return leak_check(
            AppRuntimeLeakDimension::DnsLeak,
            AppRuntimeLeakCheckStatus::Warn,
            "no DNS profile is bound; app DNS queries may fall back to the system resolver".into(),
            vec![planning_fact],
            vec!["bind a DNS profile to route app DNS through the tunnel".into()],
        );
    };
    if dns.resolver_plan.status != DnsResolverPlanStatus::Ready {
        return leak_check(
            AppRuntimeLeakDimension::DnsLeak,
            AppRuntimeLeakCheckStatus::Warn,
            format!(
                "DNS profile `{}` resolver plan is `{:?}`; tunneled DNS resolution is not confirmed",
                dns.profile_id, dns.resolver_plan.status
            )
            .into(),
            vec![planning_fact],
            vec![
                format!(
                    "review DNS profile `{}` so its resolver plan becomes ready",
                    dns.profile_id
                )
                .into(),
            ],
        );
    }
    let runtime_supported = dns
        .resolver_plan
        .nameservers
        .iter()
        .filter(|nameserver| nameserver.runtime_supported)
        .count();
    if runtime_supported == 0 {
        return leak_check(
            AppRuntimeLeakDimension::DnsLeak,
            AppRuntimeLeakCheckStatus::Warn,
            format!(
                "DNS profile `{}` has no runtime-supported nameservers; DNS may fall back to the system resolver",
                dns.profile_id
            )
            .into(),
            vec![planning_fact],
            vec!["add at least one runtime-supported nameserver to reduce DNS leak risk".into()],
        );
    }

    leak_check(
        AppRuntimeLeakDimension::DnsLeak,
        AppRuntimeLeakCheckStatus::Pass,
        format!(
            "DNS profile `{}` provides {runtime_supported} runtime-supported nameserver(s) for leak-resistant resolution",
            dns.profile_id
        )
        .into(),
        vec![planning_fact],
        Vec::new(),
    )
}

pub(super) fn leak_exit_check(
    session: &AppRuntimeSessionRecord,
    plan: &AppRuntimePlan,
    routing_intent: Option<AppRoutingIntent>,
    summary: &AppRuntimeSessionEvaluationSummary,
) -> AppRuntimeLeakCheck {
    let planning_fact: String = "exit verification is planning-only; no real exit IP is fetched".into();
    if matches!(
        session.status,
        AppRuntimeSessionStatus::Blocked | AppRuntimeSessionStatus::Failed
    ) || plan.status == AppRuntimePlanStatus::Rejected
    {
        return leak_check(
            AppRuntimeLeakDimension::ExitVerification,
            AppRuntimeLeakCheckStatus::Fail,
            "session was blocked or failed, so exit verification cannot be planned".into(),
            vec![planning_fact],
            vec!["resolve session diagnostics before planning exit verification".into()],
        );
    }
    let intent = routing_intent.unwrap_or(AppRoutingIntent::Direct);
    if !requires_node_pool(intent) {
        return leak_check(
            AppRuntimeLeakDimension::ExitVerification,
            AppRuntimeLeakCheckStatus::NotApplicable,
            format!(
                "routing intent `{}` exits directly, so there is no proxy exit to verify",
                intent_label(intent)
            )
            .into(),
            vec![planning_fact],
            Vec::new(),
        );
    }
    let has_candidates = plan
        .node_pool
        .as_ref()
        .map(|pool| pool.candidate_count > 0)
        .unwrap_or(false);
    if !has_candidates {
        return leak_check(
            AppRuntimeLeakDimension::ExitVerification,
            AppRuntimeLeakCheckStatus::Warn,
            "no node pool candidates are available, so the exit verification target is undefined".into(),
            vec![planning_fact],
            vec!["bind a node pool with candidates to define an exit verification target".into()],
        );
    }
    if summary.observation_count == 0 {
        return leak_check(
            AppRuntimeLeakDimension::ExitVerification,
            AppRuntimeLeakCheckStatus::Warn,
            "no observations recorded, so exit verification readiness is not established".into(),
            vec![planning_fact],
            vec!["record observations before planning exit verification".into()],
        );
    }
    if summary.matched_observations == 0 {
        return leak_check(
            AppRuntimeLeakDimension::ExitVerification,
            AppRuntimeLeakCheckStatus::Warn,
            "no attributed observations, so exit verification cannot target a confirmed connection".into(),
            vec![planning_fact],
            vec!["attribute at least one observation to the app before exit verification".into()],
        );
    }

    leak_check(
        AppRuntimeLeakDimension::ExitVerification,
        AppRuntimeLeakCheckStatus::Pass,
        format!(
            "exit verification can be planned against {} attributed observation(s)",
            summary.matched_observations
        )
        .into(),
        vec![planning_fact],
        Vec::new(),
    )
}

pub(super) fn leak_node_pool_check(
    routing_intent: Option<AppRoutingIntent>,
    plan: &AppRuntimePlan,
    session: &AppRuntimeSessionRecord,
    summary: &AppRuntimeSessionEvaluationSummary,
) -> AppRuntimeLeakCheck {
    let intent = routing_intent.unwrap_or(AppRoutingIntent::Direct);
    if !requires_node_pool(intent) {
        return leak_check(
            AppRuntimeLeakDimension::NodePoolConsistency,
            AppRuntimeLeakCheckStatus::NotApplicable,
            format!("routing intent `{}` does not use a node pool", intent_label(intent)).into(),
            Vec::new(),
            Vec::new(),
        );
    }
    let Some(pool) = plan.node_pool.as_ref() else {
        return leak_check(
            AppRuntimeLeakDimension::NodePoolConsistency,
            AppRuntimeLeakCheckStatus::Fail,
            "proxy routing intent has no node pool, so node-pool consistency cannot be verified".into(),
            Vec::new(),
            vec!["bind a node pool so observed proxy chains can be validated".into()],
        );
    };
    if summary.observation_count == 0 {
        return leak_check(
            AppRuntimeLeakDimension::NodePoolConsistency,
            AppRuntimeLeakCheckStatus::Warn,
            format!(
                "no observations recorded to compare against node pool `{}`",
                pool.pool_id
            )
            .into(),
            Vec::new(),
            vec!["record observations to verify proxy chains stay within the node pool".into()],
        );
    }
    if summary.matched_observations == 0 {
        return leak_check(
            AppRuntimeLeakDimension::NodePoolConsistency,
            AppRuntimeLeakCheckStatus::Warn,
            "no attributed observations, so node-pool consistency cannot be checked".into(),
            Vec::new(),
            vec!["attribute observations to the app before checking node-pool consistency".into()],
        );
    }

    let mut expected = BTreeSet::new();
    for candidate in &pool.candidates {
        expected.insert(candidate.node_name.clone());
    }
    for group in &session.projected_proxy_groups {
        expected.insert(group.clone());
    }
    let unexpected = summary
        .observed_chains
        .iter()
        .filter(|chain| !expected.contains(*chain) && !is_builtin_outbound(chain))
        .cloned()
        .collect::<Vec<_>>();
    if unexpected.is_empty() {
        return leak_check(
            AppRuntimeLeakDimension::NodePoolConsistency,
            AppRuntimeLeakCheckStatus::Pass,
            format!(
                "all observed proxy chains belong to node pool `{}` or its projected proxy group",
                pool.pool_id
            )
            .into(),
            vec![
                format!(
                    "node pool `{}` declares {} candidate(s)",
                    pool.pool_id, pool.candidate_count
                )
                .into(),
            ],
            Vec::new(),
        );
    }

    leak_check(
        AppRuntimeLeakDimension::NodePoolConsistency,
        AppRuntimeLeakCheckStatus::Warn,
        format!(
            "observed proxy chain(s) {} are not declared in node pool `{}`",
            join_chain_list(&unexpected),
            pool.pool_id
        )
        .into(),
        Vec::new(),
        vec!["verify selector or group membership, or update the node pool to include observed nodes".into()],
    )
}

pub(super) fn leak_check(
    dimension: AppRuntimeLeakDimension,
    status: AppRuntimeLeakCheckStatus,
    message: String,
    facts: Vec<String>,
    warnings: Vec<String>,
) -> AppRuntimeLeakCheck {
    AppRuntimeLeakCheck {
        dimension,
        severity: leak_severity(status),
        status,
        message,
        facts,
        warnings,
    }
}

pub(super) fn leak_severity(status: AppRuntimeLeakCheckStatus) -> AppRuntimeDiagnosticSeverity {
    match status {
        AppRuntimeLeakCheckStatus::Pass | AppRuntimeLeakCheckStatus::NotApplicable => {
            AppRuntimeDiagnosticSeverity::Info
        }
        AppRuntimeLeakCheckStatus::Warn => AppRuntimeDiagnosticSeverity::Warning,
        AppRuntimeLeakCheckStatus::Fail => AppRuntimeDiagnosticSeverity::Error,
    }
}

pub(super) fn leak_summary_counts(checks: &[AppRuntimeLeakCheck]) -> AppRuntimeLeakSummary {
    let mut summary = AppRuntimeLeakSummary::default();
    for check in checks {
        match check.status {
            AppRuntimeLeakCheckStatus::Pass => summary.pass += 1,
            AppRuntimeLeakCheckStatus::Warn => summary.warn += 1,
            AppRuntimeLeakCheckStatus::Fail => summary.fail += 1,
            AppRuntimeLeakCheckStatus::NotApplicable => summary.not_applicable += 1,
        }
    }
    summary
}

pub(super) fn leak_status(summary: &AppRuntimeLeakSummary) -> AppRuntimeDiagnosticStatus {
    if summary.fail > 0 {
        AppRuntimeDiagnosticStatus::Blocked
    } else if summary.warn > 0 {
        AppRuntimeDiagnosticStatus::Degraded
    } else {
        AppRuntimeDiagnosticStatus::Healthy
    }
}

pub(super) fn leak_reason(status: AppRuntimeDiagnosticStatus, summary: &AppRuntimeLeakSummary) -> String {
    match status {
        AppRuntimeDiagnosticStatus::Healthy => {
            "app session shows no proxy, DNS, exit, or node-pool leak indicators in recorded observations".into()
        }
        AppRuntimeDiagnosticStatus::Degraded => format!(
            "{} leak verification check(s) need attention before exit or leak verification can be confirmed",
            summary.warn
        )
        .into(),
        AppRuntimeDiagnosticStatus::Blocked => format!(
            "{} leak verification check(s) failed; resolve before performing exit or leak verification",
            summary.fail
        )
        .into(),
    }
}

pub(super) fn leak_report_warnings(checks: &[AppRuntimeLeakCheck]) -> Vec<String> {
    let mut seen = BTreeSet::new();
    let mut warnings = Vec::new();
    for check in checks {
        if matches!(
            check.status,
            AppRuntimeLeakCheckStatus::Warn | AppRuntimeLeakCheckStatus::Fail
        ) && seen.insert(check.message.clone())
        {
            warnings.push(check.message.clone());
        }
        for warning in &check.warnings {
            if seen.insert(warning.clone()) {
                warnings.push(warning.clone());
            }
        }
    }
    warnings
}

pub(super) fn intent_label(intent: AppRoutingIntent) -> String {
    format!("{intent:?}").to_ascii_lowercase().into()
}

pub(super) fn join_chain_list(items: &[String]) -> String {
    if items.is_empty() {
        return "none".into();
    }
    items
        .iter()
        .map(|item| item.as_str())
        .collect::<Vec<_>>()
        .join(", ")
        .into()
}

pub(super) fn is_builtin_outbound(chain: &str) -> bool {
    matches!(
        chain.to_ascii_uppercase().as_str(),
        "DIRECT" | "REJECT" | "REJECT-DROP" | "PASS" | "COMPATIBLE"
    )
}
