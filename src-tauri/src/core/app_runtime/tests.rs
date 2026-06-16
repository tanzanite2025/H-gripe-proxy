use super::*;
use crate::core::connection_metrics::{self, ConnectionAttributionCandidate, ConnectionMetricsSnapshot};
use crate::core::dns_runtime::{
    DnsDefaultRuntimeActiveState, DnsDefaultRuntimeExpandedHoldPolicyStatus,
    DnsDefaultRuntimeExpandedPostExecutionVerificationStatus, DnsDefaultRuntimeExpandedReverifyRecord,
    DnsDefaultRuntimeExpandedStabilityGateStatus, build_dns_default_runtime_expanded_control_plane_completion_report,
    build_dns_default_runtime_expanded_lifecycle_closeout_report,
    build_dns_default_runtime_expanded_reverify_history_report,
};
use std::collections::BTreeMap;

#[test]
fn plan_explain_uses_registered_app_policy_and_pool() {
    let state = AppRuntimeStateDocument {
        apps: vec![sample_app()],
        node_pools: vec![sample_pool()],
        dns_profiles: vec![sample_dns_profile()],
        security_profiles: vec![sample_security_profile()],
        policy_bindings: vec![sample_binding()],
        sessions: Vec::new(),
        runtime_apply_audits: Vec::new(),
        active_projection: None,
    };

    let plan = explain_app_runtime_plan(
        &state,
        AppRuntimePlanRequest {
            app_id: "browser".into(),
            session_id: Some("session-a".into()),
        },
    );

    assert_eq!(plan.status, AppRuntimePlanStatus::Ready);
    assert_eq!(plan.routing_intent, Some(AppRoutingIntent::Proxy));
    assert_eq!(plan.node_pool.as_ref().map(|pool| pool.candidate_count), Some(1));
    assert_eq!(
        plan.dns_profile
            .as_ref()
            .map(|profile| profile.resolver_plan.nameservers.len()),
        Some(1)
    );
    assert_eq!(
        plan.security_profile
            .as_ref()
            .map(|profile| profile.profile_id.as_str()),
        Some("strict")
    );
    assert!(!plan.projection.mutates_runtime);
}

#[test]
fn demo_seed_builds_ready_app_runtime_plan() {
    let state = build_app_runtime_demo_seed_document();

    let plan = explain_app_runtime_plan(
        &state,
        AppRuntimePlanRequest {
            app_id: "demo-browser".into(),
            session_id: None,
        },
    );

    assert_eq!(plan.status, AppRuntimePlanStatus::Ready);
    assert_eq!(plan.routing_intent, Some(AppRoutingIntent::Proxy));
    assert_eq!(state.apps.len(), 1);
    assert_eq!(state.node_pools.len(), 1);
    assert_eq!(state.dns_profiles.len(), 1);
    assert_eq!(state.security_profiles.len(), 1);
    assert_eq!(state.policy_bindings.len(), 1);
}

#[test]
fn app_runtime_dns_handoff_accepts_completed_dns_control_plane() {
    let report = sample_accepted_dns_handoff();

    assert_eq!(report.status, AppRuntimeDnsHandoffStatus::Accepted);
    assert!(report.app_runtime_accepts_handoff);
    assert!(report.handoff_record_persisted);
    assert!(!report.phase8_allowed);
    assert!(!report.promotion_allowed);
    assert!(!report.auto_rollout);
    assert!(!report.auto_rollback);
    assert!(!report.mutates_runtime);
}

#[test]
fn app_runtime_control_plane_completion_combines_handoff_artifact_and_preflight() {
    let dns_handoff = sample_accepted_dns_handoff();
    let state = AppRuntimeStateDocument {
        apps: vec![sample_app()],
        node_pools: vec![sample_pool()],
        dns_profiles: vec![sample_dns_profile()],
        security_profiles: vec![sample_security_profile()],
        policy_bindings: vec![sample_binding()],
        sessions: Vec::new(),
        runtime_apply_audits: Vec::new(),
        active_projection: None,
    };
    let mut artifact = build_app_runtime_projection_artifact(
        &state,
        AppRuntimePlanRequest {
            app_id: "browser".into(),
            session_id: None,
        },
    )
    .unwrap();
    artifact.storage_path = Some("artifact.yaml".into());
    let preflight = AppRuntimeProjectionActivationPreflightReport {
        status: AppRuntimeDiagnosticStatus::Healthy,
        reason: "ready".into(),
        artifact_id: artifact.artifact_id.clone(),
        app_id: Some("browser".into()),
        checksum: Some(artifact.checksum.clone()),
        storage_path: Some("artifact.yaml".into()),
        activation_mode: Some(AppRuntimeProjectionActivationMode::Staged),
        mutates_runtime: Some(false),
        checks: Vec::new(),
        summary: AppRuntimeDiagnosticsSummary {
            passed: 1,
            warnings: 0,
            failed: 0,
            skipped: 0,
        },
        facts: Vec::new(),
        warnings: Vec::new(),
    };

    let report = build_app_runtime_control_plane_completion_report(
        dns_handoff,
        artifact,
        Some("artifact.yaml".into()),
        true,
        preflight,
    );

    assert_eq!(report.status, AppRuntimeControlPlaneCompletionStatus::Ready);
    assert!(report.ready_for_staged_activation);
    assert!(!report.runtime_apply_allowed);
    assert!(!report.phase8_allowed);
    assert!(!report.auto_rollout);
    assert!(!report.auto_rollback);
    assert!(!report.mutates_runtime);
}

#[test]
fn plan_rejects_missing_policy_binding() {
    let state = AppRuntimeStateDocument {
        apps: vec![sample_app()],
        node_pools: vec![sample_pool()],
        dns_profiles: vec![sample_dns_profile()],
        security_profiles: vec![sample_security_profile()],
        policy_bindings: Vec::new(),
        sessions: Vec::new(),
        runtime_apply_audits: Vec::new(),
        active_projection: None,
    };

    let plan = explain_app_runtime_plan(
        &state,
        AppRuntimePlanRequest {
            app_id: "browser".into(),
            session_id: None,
        },
    );

    assert_eq!(plan.status, AppRuntimePlanStatus::Rejected);
    assert_eq!(plan.reason, "app `browser` has no enabled policy binding");
}

#[test]
fn validation_rejects_duplicate_process_matchers() {
    let mut app = sample_app();
    app.process_matchers.push(AppProcessMatcher {
        kind: AppProcessMatcherKind::ProcessName,
        pattern: "browser.exe".into(),
    });

    assert!(validate_app(&app).is_err());
}

#[test]
fn plan_warns_when_binding_references_missing_dns_profile() {
    let state = AppRuntimeStateDocument {
        apps: vec![sample_app()],
        node_pools: vec![sample_pool()],
        dns_profiles: Vec::new(),
        security_profiles: vec![sample_security_profile()],
        policy_bindings: vec![sample_binding()],
        sessions: Vec::new(),
        runtime_apply_audits: Vec::new(),
        active_projection: None,
    };

    let plan = explain_app_runtime_plan(
        &state,
        AppRuntimePlanRequest {
            app_id: "browser".into(),
            session_id: None,
        },
    );

    assert_eq!(plan.status, AppRuntimePlanStatus::Ready);
    assert!(plan.dns_profile.is_none());
    assert!(
        plan.warnings
            .iter()
            .any(|warning| warning.contains("missing dns_profile_id `default`"))
    );
}

#[test]
fn diagnostics_report_combines_plan_projection_and_security_checks() {
    let state = AppRuntimeStateDocument {
        apps: vec![sample_app()],
        node_pools: vec![sample_pool()],
        dns_profiles: vec![sample_dns_profile()],
        security_profiles: vec![sample_security_profile()],
        policy_bindings: vec![sample_binding()],
        sessions: Vec::new(),
        runtime_apply_audits: Vec::new(),
        active_projection: None,
    };

    let report = diagnose_app_runtime(
        &state,
        AppRuntimePlanRequest {
            app_id: "browser".into(),
            session_id: Some("session-a".into()),
        },
    )
    .unwrap();

    assert_eq!(report.status, AppRuntimeDiagnosticStatus::Healthy);
    assert_eq!(report.summary.failed, 0);
    assert_eq!(report.summary.warnings, 0);
    assert_eq!(report.plan.status, AppRuntimePlanStatus::Ready);
    assert_eq!(report.mihomo_projection.rules.len(), 1);
    assert!(
        report
            .checks
            .iter()
            .any(|check| check.check_id == "security_requires_dns_profile"
                && check.status == AppRuntimeDiagnosticCheckStatus::Passed)
    );
}

#[test]
fn diagnostics_report_blocks_when_security_policy_is_not_satisfied() {
    let mut binding = sample_binding();
    binding.dns_profile_id = None;
    let state = AppRuntimeStateDocument {
        apps: vec![sample_app()],
        node_pools: vec![sample_pool()],
        dns_profiles: Vec::new(),
        security_profiles: vec![sample_security_profile()],
        policy_bindings: vec![binding],
        sessions: Vec::new(),
        runtime_apply_audits: Vec::new(),
        active_projection: None,
    };

    let report = diagnose_app_runtime(
        &state,
        AppRuntimePlanRequest {
            app_id: "browser".into(),
            session_id: None,
        },
    )
    .unwrap();

    assert_eq!(report.status, AppRuntimeDiagnosticStatus::Blocked);
    assert!(
        report
            .checks
            .iter()
            .any(|check| check.check_id == "security_requires_dns_profile"
                && check.status == AppRuntimeDiagnosticCheckStatus::Failed)
    );
}

#[test]
fn diagnostics_report_marks_projection_without_rules_as_degraded() {
    let mut app = sample_app();
    app.process_matchers = vec![AppProcessMatcher {
        kind: AppProcessMatcherKind::BundleId,
        pattern: "com.example.browser".into(),
    }];
    let state = AppRuntimeStateDocument {
        apps: vec![app],
        node_pools: vec![sample_pool()],
        dns_profiles: vec![sample_dns_profile()],
        security_profiles: vec![sample_security_profile()],
        policy_bindings: vec![sample_binding()],
        sessions: Vec::new(),
        runtime_apply_audits: Vec::new(),
        active_projection: None,
    };

    let report = diagnose_app_runtime(
        &state,
        AppRuntimePlanRequest {
            app_id: "browser".into(),
            session_id: None,
        },
    )
    .unwrap();

    assert_eq!(report.status, AppRuntimeDiagnosticStatus::Degraded);
    assert!(
        report
            .checks
            .iter()
            .any(|check| check.check_id == "mihomo_projection_rules"
                && check.status == AppRuntimeDiagnosticCheckStatus::Warning)
    );
}

#[test]
fn session_record_snapshots_diagnostics_without_runtime_mutation() {
    let state = AppRuntimeStateDocument {
        apps: vec![sample_app()],
        node_pools: vec![sample_pool()],
        dns_profiles: vec![sample_dns_profile()],
        security_profiles: vec![sample_security_profile()],
        policy_bindings: vec![sample_binding()],
        sessions: Vec::new(),
        runtime_apply_audits: Vec::new(),
        active_projection: None,
    };

    let report = diagnose_app_runtime(
        &state,
        AppRuntimePlanRequest {
            app_id: "browser".into(),
            session_id: Some("session-a".into()),
        },
    )
    .unwrap();
    let session = session_record_from_diagnostics("session-a".into(), &report);

    assert_eq!(session.session_id, "session-a");
    assert_eq!(session.status, AppRuntimeSessionStatus::Planned);
    assert_eq!(session.plan_status, AppRuntimePlanStatus::Ready);
    assert_eq!(session.diagnostics_status, AppRuntimeDiagnosticStatus::Healthy);
    assert_eq!(session.projected_rules, vec!["PROCESS-NAME,browser.exe,app-browser"]);
    assert_eq!(session.projected_proxy_groups, vec!["app-browser"]);
    assert!(session.ended_at.is_none());
}

#[test]
fn session_record_marks_blocked_diagnostics_as_blocked() {
    let state = AppRuntimeStateDocument {
        apps: vec![sample_app()],
        node_pools: vec![sample_pool()],
        dns_profiles: vec![sample_dns_profile()],
        security_profiles: vec![sample_security_profile()],
        policy_bindings: Vec::new(),
        sessions: Vec::new(),
        runtime_apply_audits: Vec::new(),
        active_projection: None,
    };

    let report = diagnose_app_runtime(
        &state,
        AppRuntimePlanRequest {
            app_id: "browser".into(),
            session_id: Some("session-a".into()),
        },
    )
    .unwrap();
    let session = session_record_from_diagnostics("session-a".into(), &report);

    assert_eq!(session.status, AppRuntimeSessionStatus::Blocked);
    assert_eq!(session.plan_status, AppRuntimePlanStatus::Rejected);
    assert_eq!(session.diagnostics_status, AppRuntimeDiagnosticStatus::Blocked);
}

#[test]
fn session_observation_snapshots_connection_metrics_without_app_attribution() {
    let state = AppRuntimeStateDocument {
        apps: vec![sample_app()],
        node_pools: vec![sample_pool()],
        dns_profiles: vec![sample_dns_profile()],
        security_profiles: vec![sample_security_profile()],
        policy_bindings: vec![sample_binding()],
        sessions: Vec::new(),
        runtime_apply_audits: Vec::new(),
        active_projection: None,
    };
    let report = diagnose_app_runtime(
        &state,
        AppRuntimePlanRequest {
            app_id: "browser".into(),
            session_id: Some("session-a".into()),
        },
    )
    .unwrap();
    let session = session_record_from_diagnostics("session-a".into(), &report);
    let metrics = ConnectionMetricsSnapshot {
        traffic: connection_metrics::TrafficSnapshot {
            upload_total: 100,
            download_total: 200,
            upload_speed: 10,
            download_speed: 20,
            active_connection_count: 2,
            closed_since_last: 1,
            memory: 42,
        },
        speeds: vec![connection_metrics::ConnectionSpeed {
            id: "conn-a".into(),
            cur_upload: 10,
            cur_download: 20,
        }],
        attribution_candidates: Vec::new(),
        stale: false,
    };

    let observation = session_observation_from_metrics(&session, &metrics);

    assert_eq!(observation.session_id, "session-a");
    assert_eq!(
        observation.source,
        AppRuntimeSessionObservationSource::ConnectionMetricsSnapshot
    );
    assert_eq!(
        observation.attribution_status,
        AppRuntimeSessionAttributionStatus::Unattributed
    );
    assert_eq!(observation.traffic.active_connection_count, 2);
    assert_eq!(observation.connection_speed_count, 1);
    assert!(
        observation
            .warnings
            .iter()
            .any(|warning| warning.contains("No connection attribution candidates"))
    );
}

#[test]
fn session_observation_matches_connection_candidates_against_projected_rules() {
    let state = AppRuntimeStateDocument {
        apps: vec![sample_app()],
        node_pools: vec![sample_pool()],
        dns_profiles: vec![sample_dns_profile()],
        security_profiles: vec![sample_security_profile()],
        policy_bindings: vec![sample_binding()],
        sessions: Vec::new(),
        runtime_apply_audits: Vec::new(),
        active_projection: None,
    };
    let report = diagnose_app_runtime(
        &state,
        AppRuntimePlanRequest {
            app_id: "browser".into(),
            session_id: Some("session-a".into()),
        },
    )
    .unwrap();
    let session = session_record_from_diagnostics("session-a".into(), &report);
    let metrics = ConnectionMetricsSnapshot {
        traffic: connection_metrics::TrafficSnapshot {
            upload_total: 100,
            download_total: 200,
            upload_speed: 10,
            download_speed: 20,
            active_connection_count: 1,
            closed_since_last: 0,
            memory: 42,
        },
        speeds: Vec::new(),
        attribution_candidates: vec![connection_metrics::ConnectionAttributionCandidate {
            id: "conn-a".into(),
            process: "browser.exe".into(),
            process_path: "C:\\Program Files\\Browser\\browser.exe".into(),
            host: "example.com".into(),
            rule: "ProcessName".into(),
            rule_payload: "browser.exe".into(),
            chains: vec!["app-browser".into()],
            upload: 100,
            download: 200,
        }],
        stale: false,
    };

    let observation = session_observation_from_metrics(&session, &metrics);

    assert_eq!(
        observation.attribution_status,
        AppRuntimeSessionAttributionStatus::AppMatched
    );
    assert_eq!(observation.attribution_candidates.len(), 1);
    assert!(
        observation.attribution_candidates[0]
            .matched_by
            .iter()
            .any(|matched_by| matched_by == "proxyGroup:app-browser")
    );
    assert!(
        observation.attribution_candidates[0]
            .matched_by
            .iter()
            .any(|matched_by| matched_by.starts_with("projectedRule:PROCESS-NAME,browser.exe"))
    );
}

#[test]
fn session_evaluation_summarizes_matched_observations_as_healthy() {
    let state = AppRuntimeStateDocument {
        apps: vec![sample_app()],
        node_pools: vec![sample_pool()],
        dns_profiles: vec![sample_dns_profile()],
        security_profiles: vec![sample_security_profile()],
        policy_bindings: vec![sample_binding()],
        sessions: Vec::new(),
        runtime_apply_audits: Vec::new(),
        active_projection: None,
    };
    let report = diagnose_app_runtime(
        &state,
        AppRuntimePlanRequest {
            app_id: "browser".into(),
            session_id: Some("session-a".into()),
        },
    )
    .unwrap();
    let mut session = session_record_from_diagnostics("session-a".into(), &report);
    let metrics = ConnectionMetricsSnapshot {
        traffic: connection_metrics::TrafficSnapshot {
            upload_total: 100,
            download_total: 200,
            upload_speed: 10,
            download_speed: 20,
            active_connection_count: 1,
            closed_since_last: 0,
            memory: 42,
        },
        speeds: Vec::new(),
        attribution_candidates: vec![connection_metrics::ConnectionAttributionCandidate {
            id: "conn-a".into(),
            process: "browser.exe".into(),
            process_path: "C:\\Program Files\\Browser\\browser.exe".into(),
            host: "example.com".into(),
            rule: "ProcessName".into(),
            rule_payload: "browser.exe".into(),
            chains: vec!["app-browser".into()],
            upload: 100,
            download: 200,
        }],
        stale: false,
    };
    session
        .observations
        .push(session_observation_from_metrics(&session, &metrics));

    let evaluation = evaluation_report_from_session(&session);

    assert_eq!(evaluation.status, AppRuntimeDiagnosticStatus::Healthy);
    assert_eq!(evaluation.summary.observation_count, 1);
    assert_eq!(evaluation.summary.matched_observations, 1);
    assert_eq!(evaluation.summary.attribution_candidate_count, 1);
    assert_eq!(evaluation.summary.observed_chains, vec!["app-browser"]);
    assert_eq!(evaluation.summary.observed_hosts, vec!["example.com"]);
    assert!(evaluation.warnings.is_empty());
}

#[test]
fn session_evaluation_marks_missing_observations_as_degraded() {
    let state = AppRuntimeStateDocument {
        apps: vec![sample_app()],
        node_pools: vec![sample_pool()],
        dns_profiles: vec![sample_dns_profile()],
        security_profiles: vec![sample_security_profile()],
        policy_bindings: vec![sample_binding()],
        sessions: Vec::new(),
        runtime_apply_audits: Vec::new(),
        active_projection: None,
    };
    let report = diagnose_app_runtime(
        &state,
        AppRuntimePlanRequest {
            app_id: "browser".into(),
            session_id: Some("session-a".into()),
        },
    )
    .unwrap();
    let session = session_record_from_diagnostics("session-a".into(), &report);

    let evaluation = evaluation_report_from_session(&session);

    assert_eq!(evaluation.status, AppRuntimeDiagnosticStatus::Degraded);
    assert_eq!(evaluation.summary.observation_count, 0);
    assert!(
        evaluation
            .warnings
            .iter()
            .any(|warning| warning.contains("no connection metrics observations"))
    );
}

#[test]
fn session_leak_verification_reports_healthy_for_matched_proxy_session() {
    let state = sample_state();
    let mut session = planned_session(&state);
    let metrics = candidate_metrics(vec![attribution_candidate("browser.exe", vec!["app-browser"])]);
    session
        .observations
        .push(session_observation_from_metrics(&session, &metrics));

    let report = leak_report_from_session(&state, &session);

    assert_eq!(report.status, AppRuntimeDiagnosticStatus::Healthy);
    assert_eq!(report.routing_intent, Some(AppRoutingIntent::Proxy));
    assert_eq!(report.summary.fail, 0);
    assert_eq!(report.summary.warn, 0);
    assert_eq!(report.summary.pass, 4);
    assert!(report.warnings.is_empty());
    assert!(report.checks.iter().all(|check| matches!(
        check.status,
        AppRuntimeLeakCheckStatus::Pass | AppRuntimeLeakCheckStatus::NotApplicable
    )));
    assert!(report.facts.iter().any(|fact| fact.contains("planning-only")));
}

#[test]
fn session_leak_verification_flags_missing_observations_as_degraded() {
    let state = sample_state();
    let session = planned_session(&state);

    let report = leak_report_from_session(&state, &session);

    assert_eq!(report.status, AppRuntimeDiagnosticStatus::Degraded);
    assert_eq!(report.summary.fail, 0);
    assert!(report.summary.warn >= 1);
    assert_eq!(
        leak_check_status(&report, AppRuntimeLeakDimension::DnsLeak),
        AppRuntimeLeakCheckStatus::Pass
    );
    assert_eq!(
        leak_check_status(&report, AppRuntimeLeakDimension::ProxyLeak),
        AppRuntimeLeakCheckStatus::Warn
    );
    assert_eq!(
        leak_check_status(&report, AppRuntimeLeakDimension::ExitVerification),
        AppRuntimeLeakCheckStatus::Warn
    );
}

#[test]
fn session_leak_verification_fails_on_proxy_mismatch() {
    let state = sample_state();
    let mut session = planned_session(&state);
    let metrics = candidate_metrics(vec![attribution_candidate("other.exe", vec!["other-group"])]);
    let observation = session_observation_from_metrics(&session, &metrics);
    assert_eq!(
        observation.attribution_status,
        AppRuntimeSessionAttributionStatus::AppMismatch
    );
    session.observations.push(observation);

    let report = leak_report_from_session(&state, &session);

    assert_eq!(report.status, AppRuntimeDiagnosticStatus::Blocked);
    assert_eq!(
        leak_check_status(&report, AppRuntimeLeakDimension::ProxyLeak),
        AppRuntimeLeakCheckStatus::Fail
    );
    assert!(report.warnings.iter().any(|warning| warning.contains("proxy leak")));
}

#[test]
fn session_leak_verification_warns_on_node_pool_inconsistency() {
    let state = sample_state();
    let mut session = planned_session(&state);
    let metrics = candidate_metrics(vec![attribution_candidate(
        "browser.exe",
        vec!["app-browser", "rogue-node"],
    )]);
    session
        .observations
        .push(session_observation_from_metrics(&session, &metrics));

    let report = leak_report_from_session(&state, &session);

    assert_eq!(report.status, AppRuntimeDiagnosticStatus::Degraded);
    assert_eq!(
        leak_check_status(&report, AppRuntimeLeakDimension::ProxyLeak),
        AppRuntimeLeakCheckStatus::Pass
    );
    assert_eq!(
        leak_check_status(&report, AppRuntimeLeakDimension::NodePoolConsistency),
        AppRuntimeLeakCheckStatus::Warn
    );
    assert!(report.warnings.iter().any(|warning| warning.contains("rogue-node")));
}

#[test]
fn session_leak_verification_warns_for_direct_routing_without_dns_profile() {
    let mut binding = sample_binding();
    binding.node_pool_id = None;
    binding.dns_profile_id = None;
    binding.security_profile_id = None;
    binding.routing_intent = AppRoutingIntent::Direct;
    let state = AppRuntimeStateDocument {
        apps: vec![sample_app()],
        node_pools: vec![sample_pool()],
        dns_profiles: vec![sample_dns_profile()],
        security_profiles: vec![sample_security_profile()],
        policy_bindings: vec![binding],
        sessions: Vec::new(),
        runtime_apply_audits: Vec::new(),
        active_projection: None,
    };
    let session = planned_session(&state);

    let report = leak_report_from_session(&state, &session);

    assert_eq!(report.status, AppRuntimeDiagnosticStatus::Degraded);
    assert_eq!(report.routing_intent, Some(AppRoutingIntent::Direct));
    assert_eq!(
        leak_check_status(&report, AppRuntimeLeakDimension::ProxyLeak),
        AppRuntimeLeakCheckStatus::NotApplicable
    );
    assert_eq!(
        leak_check_status(&report, AppRuntimeLeakDimension::ExitVerification),
        AppRuntimeLeakCheckStatus::NotApplicable
    );
    assert_eq!(
        leak_check_status(&report, AppRuntimeLeakDimension::DnsLeak),
        AppRuntimeLeakCheckStatus::Warn
    );
    assert!(
        report
            .warnings
            .iter()
            .any(|warning| warning.contains("system resolver"))
    );
}

#[test]
fn mihomo_projection_emits_process_rule_proxy_group_and_yaml_patch() {
    let state = AppRuntimeStateDocument {
        apps: vec![sample_app()],
        node_pools: vec![sample_pool()],
        dns_profiles: vec![sample_dns_profile()],
        security_profiles: vec![sample_security_profile()],
        policy_bindings: vec![sample_binding()],
        sessions: Vec::new(),
        runtime_apply_audits: Vec::new(),
        active_projection: None,
    };

    let projection = project_app_runtime_plan_to_mihomo(
        &state,
        AppRuntimePlanRequest {
            app_id: "browser".into(),
            session_id: Some("session-a".into()),
        },
    )
    .unwrap();

    assert_eq!(projection.status, AppRuntimePlanStatus::Ready);
    assert!(!projection.mutates_runtime);
    assert_eq!(projection.proxy_groups.len(), 1);
    assert_eq!(projection.proxy_groups[0].name, "app-browser");
    assert_eq!(projection.proxy_groups[0].group_type, "select");
    assert_eq!(projection.proxy_groups[0].proxies, vec!["us-1"]);
    assert_eq!(projection.rules.len(), 1);
    assert_eq!(projection.rules[0].rule, "PROCESS-NAME,browser.exe,app-browser");
    assert_eq!(
        projection.dns.as_ref().map(|dns| dns.nameservers.clone()),
        Some(vec!["1.1.1.1".into()])
    );
    assert!(projection.yaml_patch.contains("proxy-groups:"));
    assert!(projection.yaml_patch.contains("PROCESS-NAME,browser.exe,app-browser"));
}

#[test]
fn mihomo_projection_maps_direct_binding_without_proxy_group() {
    let mut binding = sample_binding();
    binding.node_pool_id = None;
    binding.routing_intent = AppRoutingIntent::Direct;
    let state = AppRuntimeStateDocument {
        apps: vec![sample_app()],
        node_pools: vec![sample_pool()],
        dns_profiles: vec![sample_dns_profile()],
        security_profiles: vec![sample_security_profile()],
        policy_bindings: vec![binding],
        sessions: Vec::new(),
        runtime_apply_audits: Vec::new(),
        active_projection: None,
    };

    let projection = project_app_runtime_plan_to_mihomo(
        &state,
        AppRuntimePlanRequest {
            app_id: "browser".into(),
            session_id: None,
        },
    )
    .unwrap();

    assert!(projection.proxy_groups.is_empty());
    assert_eq!(projection.rules[0].rule, "PROCESS-NAME,browser.exe,DIRECT");
    assert!(projection.yaml_patch.contains("PROCESS-NAME,browser.exe,DIRECT"));
}

#[test]
fn mihomo_projection_warns_for_unsupported_matchers() {
    let mut app = sample_app();
    app.process_matchers = vec![AppProcessMatcher {
        kind: AppProcessMatcherKind::BundleId,
        pattern: "com.example.browser".into(),
    }];
    let state = AppRuntimeStateDocument {
        apps: vec![app],
        node_pools: vec![sample_pool()],
        dns_profiles: vec![sample_dns_profile()],
        security_profiles: vec![sample_security_profile()],
        policy_bindings: vec![sample_binding()],
        sessions: Vec::new(),
        runtime_apply_audits: Vec::new(),
        active_projection: None,
    };

    let projection = project_app_runtime_plan_to_mihomo(
        &state,
        AppRuntimePlanRequest {
            app_id: "browser".into(),
            session_id: None,
        },
    )
    .unwrap();

    assert!(projection.rules.is_empty());
    assert!(
        projection
            .warnings
            .iter()
            .any(|warning| warning.contains("cannot be projected to a Mihomo rule"))
    );
}

#[test]
fn projection_artifact_is_staged_and_validates_yaml_patch() {
    let artifact = build_app_runtime_projection_artifact(
        &sample_state(),
        AppRuntimePlanRequest {
            app_id: "browser".into(),
            session_id: Some("session-a".into()),
        },
    )
    .unwrap();

    assert_eq!(artifact.activation_mode, AppRuntimeProjectionActivationMode::Staged);
    assert!(!artifact.mutates_runtime);
    assert_eq!(artifact.validation.status, AppRuntimeDiagnosticStatus::Healthy);
    assert_eq!(artifact.validation.summary.failed, 0);
    assert_eq!(artifact.projection.rules.len(), 1);
    assert_eq!(artifact.storage_path, None);
    assert!(artifact.artifact_id.starts_with("app-runtime-browser-"));
    assert_eq!(artifact.checksum.len(), 64);
    assert!(
        artifact
            .validation
            .checks
            .iter()
            .any(|check| check.check_id == "artifact_yaml_patch_parse"
                && check.status == AppRuntimeDiagnosticCheckStatus::Passed)
    );
}

fn sample_state() -> AppRuntimeStateDocument {
    AppRuntimeStateDocument {
        apps: vec![sample_app()],
        node_pools: vec![sample_pool()],
        dns_profiles: vec![sample_dns_profile()],
        security_profiles: vec![sample_security_profile()],
        policy_bindings: vec![sample_binding()],
        sessions: Vec::new(),
        runtime_apply_audits: Vec::new(),
        active_projection: None,
    }
}

fn planned_session(state: &AppRuntimeStateDocument) -> AppRuntimeSessionRecord {
    let report = diagnose_app_runtime(
        state,
        AppRuntimePlanRequest {
            app_id: "browser".into(),
            session_id: Some("session-a".into()),
        },
    )
    .unwrap();
    session_record_from_diagnostics("session-a".into(), &report)
}

fn attribution_candidate(process: &str, chains: Vec<&str>) -> ConnectionAttributionCandidate {
    ConnectionAttributionCandidate {
        id: "conn-a".into(),
        process: process.into(),
        process_path: format!("C:\\Program Files\\App\\{process}").into(),
        host: "example.com".into(),
        rule: "ProcessName".into(),
        rule_payload: process.into(),
        chains: chains.into_iter().map(Into::into).collect(),
        upload: 100,
        download: 200,
    }
}

fn candidate_metrics(candidates: Vec<ConnectionAttributionCandidate>) -> ConnectionMetricsSnapshot {
    ConnectionMetricsSnapshot {
        traffic: connection_metrics::TrafficSnapshot {
            upload_total: 100,
            download_total: 200,
            upload_speed: 10,
            download_speed: 20,
            active_connection_count: 1,
            closed_since_last: 0,
            memory: 42,
        },
        speeds: Vec::new(),
        attribution_candidates: candidates,
        stale: false,
    }
}

fn leak_check_status(
    report: &AppRuntimeSessionLeakReport,
    dimension: AppRuntimeLeakDimension,
) -> AppRuntimeLeakCheckStatus {
    report
        .checks
        .iter()
        .find(|check| check.dimension == dimension)
        .map(|check| check.status)
        .expect("leak check dimension present")
}

#[test]
fn activation_preflight_blocks_before_runtime_mutation() {
    let request = AppRuntimeProjectionActivationPreflightRequest {
        artifact_id: "app-runtime-browser-abc123".into(),
        expected_checksum: Some("checksum-a".into()),
    };
    let report = app_runtime_activation_preflight_report_from_yaml(
        &request,
        "app-runtime/artifacts/app-runtime-browser-abc123/artifact.yaml".into(),
        r#"
artifactId: app-runtime-browser-abc123
appId: browser
checksum: checksum-a
activationMode: staged
mutatesRuntime: false
validation:
  status: healthy
"#,
    );

    assert_eq!(report.status, AppRuntimeDiagnosticStatus::Blocked);
    assert_eq!(report.app_id, Some("browser".into()));
    assert_eq!(report.checksum, Some("checksum-a".into()));
    assert!(report.summary.passed >= 5);
    assert!(
        report
            .checks
            .iter()
            .any(|check| check.check_id == "activation_executor_guard"
                && check.status == AppRuntimeDiagnosticCheckStatus::Failed)
    );
}

#[test]
fn active_projection_rollback_marker_restores_previous_artifact_metadata() {
    let previous_artifact = persisted_projection_artifact(
        "projection-browser-previous",
        "checksum-previous",
        "app-runtime/artifacts/previous/artifact.yaml",
    );
    let previous = app_runtime_active_projection_record_from_artifact(&previous_artifact, "state_marker", None, 10);

    let current_artifact = persisted_projection_artifact(
        "projection-browser-current",
        "checksum-current",
        "app-runtime/artifacts/current/artifact.yaml",
    );
    let current =
        app_runtime_active_projection_record_from_artifact(&current_artifact, "state_marker", Some(&previous), 20);

    let restored = app_runtime_active_projection_record_from_artifact(
        &previous_artifact,
        "state_marker_rollback",
        Some(&current),
        30,
    );

    assert_eq!(restored.artifact_id, previous_artifact.artifact_id);
    assert_eq!(restored.checksum, previous_artifact.checksum);
    assert_eq!(restored.storage_path, "app-runtime/artifacts/previous/artifact.yaml");
    assert_eq!(restored.activation_kind, "state_marker_rollback");
    assert_eq!(
        restored.rollback.previous_artifact_id,
        Some(current_artifact.artifact_id)
    );
    assert_eq!(restored.rollback.previous_checksum, Some(current_artifact.checksum));
    assert_eq!(
        restored.rollback.previous_storage_path,
        Some("app-runtime/artifacts/current/artifact.yaml".into())
    );
}

#[test]
fn runtime_apply_marker_records_runtime_mutation_boundary() {
    let artifact = persisted_projection_artifact(
        "projection-browser-runtime",
        "checksum-runtime",
        "app-runtime/artifacts/runtime/artifact.yaml",
    );
    let marker = app_runtime_active_projection_record_from_artifact_with_runtime(
        &artifact,
        "runtime_profile_merge",
        None,
        40,
        true,
    );

    assert!(marker.mutates_runtime);
    assert_eq!(marker.activation_kind, "runtime_profile_merge");
    assert_eq!(
        marker.rollback.rollback_strategy,
        "restore_runtime_from_profile_and_previous_marker"
    );
}

#[test]
fn runtime_apply_audit_records_candidate_and_previous_marker() {
    let artifact = persisted_projection_artifact(
        "projection-browser-runtime",
        "checksum-runtime",
        "app-runtime/artifacts/runtime/artifact.yaml",
    );
    let previous = app_runtime_active_projection_record_from_artifact(&artifact, "state_marker", None, 40);
    let summary = AppRuntimeProjectionRuntimeApplyCandidateSummary {
        profile_item_uid: "m-app-runtime-runtime".into(),
        profile_item_file: "m-app-runtime-runtime.yaml".into(),
        proxy_group_count: 1,
        rule_count: 1,
        dns_profile_projected: false,
    };

    let audit = app_runtime_projection_runtime_apply_audit_record(
        &artifact,
        "runtime_profile_merge",
        Some(&previous),
        &summary,
        "valid".into(),
        50,
    );

    assert_eq!(audit.artifact_id, artifact.artifact_id);
    assert_eq!(audit.status, AppRuntimeProjectionRuntimeApplyAuditStatus::Active);
    assert_eq!(audit.candidate_summary.proxy_group_count, 1);
    assert_eq!(audit.previous_marker.unwrap().activation_kind, "state_marker");
    assert_eq!(
        audit.rollback_strategy,
        "restore_runtime_from_profile_and_previous_marker"
    );
}

#[test]
fn runtime_apply_audit_status_tracks_rollback_and_supersede() {
    let artifact = persisted_projection_artifact(
        "projection-browser-runtime",
        "checksum-runtime",
        "app-runtime/artifacts/runtime/artifact.yaml",
    );
    let summary = AppRuntimeProjectionRuntimeApplyCandidateSummary {
        profile_item_uid: "m-app-runtime-runtime".into(),
        profile_item_file: "m-app-runtime-runtime.yaml".into(),
        proxy_group_count: 1,
        rule_count: 1,
        dns_profile_projected: false,
    };
    let first = app_runtime_projection_runtime_apply_audit_record(
        &artifact,
        "runtime_profile_merge",
        None,
        &summary,
        "valid".into(),
        50,
    );
    let next = app_runtime_projection_runtime_apply_audit_record(
        &artifact,
        "runtime_profile_merge",
        None,
        &summary,
        "valid".into(),
        60,
    );
    let mut audits = vec![first];
    mark_runtime_apply_audits_superseded(&mut audits, &next, 60);
    assert_eq!(
        audits[0].status,
        AppRuntimeProjectionRuntimeApplyAuditStatus::Superseded
    );

    let marker = app_runtime_active_projection_record_from_artifact_with_runtime(
        &artifact,
        "runtime_profile_merge",
        None,
        70,
        true,
    );
    audits.push(next);
    mark_runtime_apply_audits_rolled_back(&mut audits, &marker, 80);
    assert_eq!(
        audits[1].status,
        AppRuntimeProjectionRuntimeApplyAuditStatus::RolledBack
    );
}

#[test]
fn runtime_verification_observes_projected_rules_and_groups() {
    let artifact = persisted_projection_artifact(
        "projection-browser-runtime",
        "checksum-runtime",
        "app-runtime/artifacts/runtime/artifact.yaml",
    );
    let runtime_yaml = r#"
proxy-groups:
  - name: premium-us
    type: select
    proxies:
      - us-1
rules:
  - PROCESS-NAME,browser.exe,premium-us
"#;
    let runtime_config = serde_yaml_ng::from_str::<serde_yaml_ng::Value>(runtime_yaml)
        .unwrap()
        .as_mapping()
        .unwrap()
        .clone();

    let group_check = runtime_verification_proxy_groups_check(&runtime_config, &artifact.projection.proxy_groups);
    let rule_check = runtime_verification_rules_check(&runtime_config, &artifact.projection.rules);

    assert_eq!(group_check.status, AppRuntimeDiagnosticCheckStatus::Passed);
    assert_eq!(rule_check.status, AppRuntimeDiagnosticCheckStatus::Passed);
}

#[test]
fn runtime_merge_candidate_appends_projection_rules_and_groups() {
    let current = r#"
rules:
  - DOMAIN,existing.example,DIRECT
proxy-groups:
  - name: Existing
    type: select
    proxies:
      - DIRECT
"#;
    let patch = r#"
rules:
  - PROCESS-NAME,browser.exe,premium-us
proxy-groups:
  - name: premium-us
    type: select
    proxies:
      - us-1
"#;

    let merged = app_runtime_projection_runtime_merge_yaml(Some(current), patch).unwrap();
    let value = serde_yaml_ng::from_str::<serde_yaml_ng::Value>(&merged).unwrap();
    let mapping = value.as_mapping().unwrap();
    let rules = mapping.get("rules").unwrap().as_sequence().unwrap();
    let groups = mapping.get("proxy-groups").unwrap().as_sequence().unwrap();

    assert_eq!(rules.len(), 2);
    assert_eq!(groups.len(), 2);
    assert_eq!(rules[0].as_str(), Some("DOMAIN,existing.example,DIRECT"));
    assert_eq!(rules[1].as_str(), Some("PROCESS-NAME,browser.exe,premium-us"));
}

fn persisted_projection_artifact(
    artifact_id: &str,
    checksum: &str,
    storage_path: &str,
) -> PersistedAppRuntimeProjectionArtifact {
    PersistedAppRuntimeProjectionArtifact {
        artifact_id: artifact_id.into(),
        app_id: "browser".into(),
        storage_path: Some(storage_path.into()),
        activation_mode: AppRuntimeProjectionActivationMode::Staged,
        mutates_runtime: false,
        checksum: checksum.into(),
        projection: PersistedAppRuntimeMihomoProjection {
            proxy_groups: vec![MihomoProxyGroupProjection {
                name: "premium-us".into(),
                group_type: "select".into(),
                proxies: vec!["us-1".into()],
                url: None,
                interval: None,
            }],
            rules: vec![MihomoRuleProjection {
                matcher: "PROCESS-NAME".into(),
                value: "browser.exe".into(),
                target: "premium-us".into(),
                rule: "PROCESS-NAME,browser.exe,premium-us".into(),
            }],
            dns: None,
            yaml_patch: "proxy-groups:\n- name: premium-us\n  type: select\n  proxies:\n  - us-1\nrules:\n- PROCESS-NAME,browser.exe,premium-us\n".into(),
        },
        validation: PersistedAppRuntimeProjectionValidation {
            status: AppRuntimeDiagnosticStatus::Healthy,
        },
    }
}

fn sample_app() -> AppRegistryEntry {
    AppRegistryEntry {
        app_id: "browser".into(),
        name: "Browser".into(),
        executable_path: Some("C:\\Program Files\\Browser\\browser.exe".into()),
        bundle_id: None,
        launch_args: Vec::new(),
        working_directory: None,
        env: Vec::new(),
        process_matchers: vec![AppProcessMatcher {
            kind: AppProcessMatcherKind::ProcessName,
            pattern: "browser.exe".into(),
        }],
        platform_metadata: BTreeMap::new(),
        tags: vec!["desktop".into()],
        updated_at: 1,
    }
}

fn sample_pool() -> NodePool {
    NodePool {
        pool_id: "premium-us".into(),
        name: "Premium US".into(),
        tags: vec!["stable".into()],
        region: Some("US".into()),
        protocols: vec!["trojan".into()],
        purpose: Some("streaming".into()),
        cost_tier: Some("paid".into()),
        health_constraints: NodePoolHealthConstraints {
            max_latency_ms: Some(300),
            require_alive: Some(true),
            min_available_nodes: Some(1),
        },
        candidate_nodes: vec![NodePoolCandidate {
            node_name: "us-1".into(),
            proxy_group: Some("Proxy".into()),
            protocol: Some("trojan".into()),
            region: Some("US".into()),
            tags: vec!["stable".into()],
            priority: Some(1),
        }],
        updated_at: 1,
    }
}

fn sample_dns_profile() -> DnsProfile {
    DnsProfile {
        profile_id: "default".into(),
        name: "Default DNS".into(),
        config_yaml: r#"
dns:
  enable: true
  nameserver:
    - 1.1.1.1
"#
        .into(),
        test_domain: Some("example.com".into()),
        tags: vec!["default".into()],
        updated_at: 1,
    }
}

fn sample_security_profile() -> SecurityProfile {
    SecurityProfile {
        profile_id: "strict".into(),
        name: "Strict App Runtime".into(),
        controls: SecurityProfileControls {
            require_node_pool: true,
            require_dns_profile: true,
            min_runtime_supported_nameservers: Some(1),
            allowed_routing_intents: vec![AppRoutingIntent::Proxy, AppRoutingIntent::Auto],
        },
        tags: vec!["strict".into()],
        updated_at: 1,
    }
}

fn sample_binding() -> AppPolicyBinding {
    AppPolicyBinding {
        binding_id: "browser-policy".into(),
        app_id: "browser".into(),
        node_pool_id: Some("premium-us".into()),
        dns_profile_id: Some("default".into()),
        security_profile_id: Some("strict".into()),
        routing_intent: AppRoutingIntent::Proxy,
        enabled: true,
        updated_at: 1,
    }
}

fn sample_accepted_dns_handoff() -> AppRuntimeDnsHandoffReport {
    let history = build_dns_default_runtime_expanded_reverify_history_report(
        vec![sample_dns_reverify_record(100), sample_dns_reverify_record(200)],
        Vec::new(),
    );
    let closeout = build_dns_default_runtime_expanded_lifecycle_closeout_report(
        history,
        Some(sample_dns_active_state()),
        Vec::new(),
    );
    let dns_completion = build_dns_default_runtime_expanded_control_plane_completion_report(
        closeout,
        true,
        Some("dns-handoff.yaml".into()),
        Vec::new(),
        300,
    );

    build_app_runtime_dns_handoff_report(
        dns_completion,
        Some("app-runtime-handoff.yaml".into()),
        true,
        Vec::new(),
        400,
    )
}

fn sample_dns_reverify_record(created_at_epoch_seconds: u64) -> DnsDefaultRuntimeExpandedReverifyRecord {
    DnsDefaultRuntimeExpandedReverifyRecord {
        event_id: format!("reverify-{created_at_epoch_seconds}"),
        action: "defaultDnsRuntimeExpandedReverify".into(),
        active_execution_event_id: Some("execution-1".into()),
        hold_status: DnsDefaultRuntimeExpandedHoldPolicyStatus::Ready,
        stability_status: DnsDefaultRuntimeExpandedStabilityGateStatus::Ready,
        post_execution_status: DnsDefaultRuntimeExpandedPostExecutionVerificationStatus::Verified,
        active_age_seconds: Some(600),
        keep_active_allowed: true,
        next_verification_required: false,
        rollback_recommended: false,
        next_verification_after_epoch_seconds: None,
        hold_expires_at_epoch_seconds: Some(created_at_epoch_seconds + 3_600),
        created_at_epoch_seconds,
    }
}

fn sample_dns_active_state() -> DnsDefaultRuntimeActiveState {
    DnsDefaultRuntimeActiveState {
        active_runtime: "fake-ip".into(),
        previous_runtime: "normal".into(),
        state: "expandedActiveProfileReloaded".into(),
        execution_event_id: "execution-1".into(),
        activated_at_epoch_seconds: 100,
        rollback_marker_path: None,
        audit_record_path: Some("audit.yaml".into()),
    }
}
