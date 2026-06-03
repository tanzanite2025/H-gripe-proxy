use crate::security::ingress_countermeasure::{
    ClassifierThresholds, DeceptionMode, IngressCountermeasureConfig, IngressCountermeasureRuntime,
    IngressSignalSnapshot, IngressThreatClassifier, ResponseMode, SurfaceBias, ThreatLevel, ThreatReason,
    default_persona_profiles, runtime_persona_profiles, select_persona,
};

#[test]
fn ingress_countermeasure_config_defaults_are_safe() {
    let cfg = IngressCountermeasureConfig::default();
    assert!(cfg.enabled);
    assert_eq!(cfg.deception_mode, DeceptionMode::DecoyPreferred);
    assert!(cfg.persona_profiles.len() >= 2);
    assert!(cfg.egress_stability_support.enabled);
}

#[test]
fn classifier_marks_hostile_when_honeypot_and_probe_failures_stack() {
    let classifier = IngressThreatClassifier::new(ClassifierThresholds::default());
    let result = classifier.classify(IngressSignalSnapshot {
        anti_probe_failed: true,
        honeypot_triggered: true,
        suspicious_header_count: 2,
        repeated_burst_count: 3,
    });

    assert_eq!(result.level, ThreatLevel::Hostile);
    assert!(result.reasons.contains(&ThreatReason::AntiProbeFailure));
    assert!(result.reasons.contains(&ThreatReason::HoneypotTriggered));
    assert!(result.reasons.contains(&ThreatReason::SuspiciousHeaders));
    assert!(result.reasons.contains(&ThreatReason::RepeatedBurst));
}

#[test]
fn classifier_marks_honeypot_only_signal_as_hostile() {
    let classifier = IngressThreatClassifier::new(ClassifierThresholds::default());
    let result = classifier.classify(IngressSignalSnapshot {
        honeypot_triggered: true,
        ..IngressSignalSnapshot::default()
    });

    assert_eq!(result.level, ThreatLevel::Hostile);
    assert_eq!(result.reasons, vec![ThreatReason::HoneypotTriggered]);
}

#[test]
fn classifier_marks_probe_failure_with_burst_as_suspicious() {
    let classifier = IngressThreatClassifier::new(ClassifierThresholds::default());
    let result = classifier.classify(IngressSignalSnapshot {
        anti_probe_failed: true,
        repeated_burst_count: 1,
        ..IngressSignalSnapshot::default()
    });

    assert_eq!(result.level, ThreatLevel::Suspicious);
    assert!(result.reasons.contains(&ThreatReason::AntiProbeFailure));
    assert!(result.reasons.contains(&ThreatReason::RepeatedBurst));
}

#[test]
fn suspicious_flow_uses_non_normal_persona() {
    let personas = default_persona_profiles();
    let persona = select_persona(ThreatLevel::Suspicious, &personas).unwrap();

    assert_ne!(persona.name, "normal-browser");
}

#[test]
fn configured_persona_profiles_shape_runtime_selection() {
    let mut config = IngressCountermeasureConfig::default();
    config.persona_profiles[0].surface_bias = SurfaceBias::Decoy;

    let personas = runtime_persona_profiles(&config.persona_profiles);
    let hostile = select_persona(ThreatLevel::Hostile, &personas).unwrap();

    assert_eq!(hostile.name, config.persona_profiles[0].id);
    assert_eq!(hostile.tls_fingerprint, "randomized");
}

#[test]
fn hostile_flow_prefers_decoy_route() {
    let runtime = IngressCountermeasureRuntime::new(IngressCountermeasureConfig::default());
    let plan = runtime.route_for_level(ThreatLevel::Hostile);

    assert_eq!(plan.mode, ResponseMode::Deception);
    assert!(!plan.fake_surfaces.is_empty());
}

#[test]
fn hostile_flow_falls_back_to_limited_reject_when_deception_is_unavailable() {
    let mut config = IngressCountermeasureConfig::default();
    config.deception_mode = DeceptionMode::Disabled;
    let runtime = IngressCountermeasureRuntime::new(config);

    let plan = runtime.route_for_level(ThreatLevel::Hostile);

    assert_eq!(plan.mode, ResponseMode::LimitedReject);
    assert!(plan.fake_surfaces.is_empty());
}

#[tokio::test]
async fn hostile_flow_requests_stable_egress_support() {
    let runtime = IngressCountermeasureRuntime::new(IngressCountermeasureConfig::default());
    runtime
        .record_signal("198.51.100.8", ThreatReason::HoneypotTriggered)
        .await;

    let policy = runtime.current_egress_support_policy();

    assert!(policy.minimize_drift);
    assert_eq!(policy.strongest_threat_level, ThreatLevel::Hostile);
    assert!(policy.rebind_grace_period_ms > 0);
    assert!(policy.connection_warmup_ms > 0);
}

#[tokio::test]
async fn runtime_records_recent_signals_by_source() {
    let runtime = IngressCountermeasureRuntime::new(IngressCountermeasureConfig::default());
    runtime.record_signal("1.2.3.4", ThreatReason::AntiProbeFailure).await;

    let snapshot = runtime.snapshot_for_source("1.2.3.4").await;

    assert!(snapshot.anti_probe_failed);
}

#[tokio::test]
async fn runtime_accumulates_counts_per_source_without_cross_source_bleed() {
    let runtime = IngressCountermeasureRuntime::new(IngressCountermeasureConfig::default());

    runtime.record_signal("1.2.3.4", ThreatReason::SuspiciousHeaders).await;
    runtime.record_signal("1.2.3.4", ThreatReason::SuspiciousHeaders).await;
    runtime.record_signal("1.2.3.4", ThreatReason::RepeatedBurst).await;
    runtime.record_signal("5.6.7.8", ThreatReason::AntiProbeFailure).await;

    let first = runtime.snapshot_for_source("1.2.3.4").await;
    let second = runtime.snapshot_for_source("5.6.7.8").await;

    assert_eq!(first.suspicious_header_count, 2);
    assert_eq!(first.repeated_burst_count, 1);
    assert!(!first.anti_probe_failed);

    assert!(second.anti_probe_failed);
    assert_eq!(second.suspicious_header_count, 0);
    assert_eq!(second.repeated_burst_count, 0);
}

#[tokio::test]
async fn runtime_refreshes_recent_activity_for_eviction_order() {
    let runtime = IngressCountermeasureRuntime::new(IngressCountermeasureConfig::default());

    for idx in 0..256 {
        runtime
            .record_signal(format!("10.0.0.{idx}"), ThreatReason::SuspiciousHeaders)
            .await;
    }

    runtime.record_signal("10.0.0.0", ThreatReason::RepeatedBurst).await;
    runtime.record_signal("10.0.1.1", ThreatReason::HoneypotTriggered).await;

    let reactivated = runtime.snapshot_for_source("10.0.0.0").await;
    let evicted = runtime.snapshot_for_source("10.0.0.1").await;
    let newest = runtime.snapshot_for_source("10.0.1.1").await;

    assert_eq!(reactivated.suspicious_header_count, 1);
    assert_eq!(reactivated.repeated_burst_count, 1);
    assert!(!reactivated.honeypot_triggered);

    assert_eq!(evicted, IngressSignalSnapshot::default());
    assert!(newest.honeypot_triggered);
}

#[tokio::test]
async fn hostile_signal_history_routes_to_deception_plan() {
    let runtime = IngressCountermeasureRuntime::new(IngressCountermeasureConfig::default());
    runtime
        .record_signal("203.0.113.10", ThreatReason::HoneypotTriggered)
        .await;

    let classification = runtime.classify_source("203.0.113.10").await;
    let plan = runtime.plan_for_source("203.0.113.10").await;

    assert_eq!(classification.level, ThreatLevel::Hostile);
    assert_eq!(plan.mode, ResponseMode::Deception);
    assert!(!plan.fake_surfaces.is_empty());
}
