use super::*;

pub async fn mihomo_kernel_loopback_r5_closeout_r6_rust_runtime_scaffold(
    listener_port: Option<u16>,
    target_port: Option<u16>,
    hold_started_at_epoch_ms: Option<u64>,
    observed_rollback_platforms: Option<Vec<String>>,
    explicit_decision: Option<bool>,
    requested_execution: Option<bool>,
    post_execution_hold_started_at_epoch_ms: Option<u64>,
    wider_opt_in_decision: Option<bool>,
    limited_rollout_decision: Option<bool>,
    canary_scope: Option<String>,
    max_canary_sessions: Option<u16>,
    closeout_decision: Option<bool>,
    handoff_decision: Option<bool>,
    r5_preflight_decision: Option<bool>,
    rollback_plan_decision: Option<bool>,
    execution_plan_decision: Option<bool>,
    guard_decision: Option<bool>,
    dry_run_decision: Option<bool>,
    dry_run_execution_decision: Option<bool>,
    post_dry_run_hold_started_at_epoch_ms: Option<u64>,
    hold_decision: Option<bool>,
    decision_readiness_decision: Option<bool>,
    final_gate_decision: Option<bool>,
    r5_handoff_decision: Option<bool>,
    final_hold_started_at_epoch_ms: Option<u64>,
    final_hold_decision: Option<bool>,
    independent_rollback_decision: Option<bool>,
    r5_closeout_decision: Option<bool>,
    r5_closeout_report_decision: Option<bool>,
    requested_runtime_kind: Option<String>,
    rust_runtime_opt_in_decision: Option<bool>,
    rust_runtime_scaffold_decision: Option<bool>,
) -> Result<KernelLoopbackR5CloseoutR6RustRuntimeScaffoldReport> {
    let rust_runtime_scaffold_decision = rust_runtime_scaffold_decision.unwrap_or(false);
    let r5_closeout = mihomo_kernel_loopback_r5_default_cutover_closeout_report(
        listener_port,
        target_port,
        hold_started_at_epoch_ms,
        observed_rollback_platforms,
        explicit_decision,
        requested_execution,
        post_execution_hold_started_at_epoch_ms,
        wider_opt_in_decision,
        limited_rollout_decision,
        canary_scope,
        max_canary_sessions,
        closeout_decision,
        handoff_decision,
        r5_preflight_decision,
        rollback_plan_decision,
        execution_plan_decision,
        guard_decision,
        dry_run_decision,
        dry_run_execution_decision,
        post_dry_run_hold_started_at_epoch_ms,
        hold_decision,
        decision_readiness_decision,
        final_gate_decision,
        r5_handoff_decision,
        final_hold_started_at_epoch_ms,
        final_hold_decision,
        independent_rollback_decision,
        r5_closeout_decision,
        r5_closeout_report_decision,
    )
    .await?;
    let runtime_selection =
        kernel_runtime_selection_scaffold(requested_runtime_kind, rust_runtime_opt_in_decision).await;
    let checks = vec![
        KernelLoopbackR4ExpandedOptInLimitedRolloutGateCheck {
            name: "r5CloseoutComplete".into(),
            status: if r5_closeout.passed { "passed" } else { "blocked" }.into(),
            passed: r5_closeout.passed,
            blockers: r5_closeout.blockers.clone(),
            facts: vec!["R5 closeout report is bundled with R6 scaffold".into()],
        },
        KernelLoopbackR4ExpandedOptInLimitedRolloutGateCheck {
            name: "rustRuntimeScaffoldDecision".into(),
            status: if rust_runtime_scaffold_decision {
                "passed"
            } else {
                "blocked"
            }
            .into(),
            passed: rust_runtime_scaffold_decision,
            blockers: if rust_runtime_scaffold_decision {
                Vec::new()
            } else {
                vec!["R6 Rust runtime scaffold requires an explicit scaffold decision".into()]
            },
            facts: vec!["Rust runtime kind and fallback boundaries are modeled".into()],
        },
        KernelLoopbackR4ExpandedOptInLimitedRolloutGateCheck {
            name: "mihomoRemainsSelectedDefault".into(),
            status: "passed".into(),
            passed: true,
            blockers: Vec::new(),
            facts: vec!["scaffold does not change the selected default runtime".into()],
        },
    ];
    let scaffold_ready = checks.iter().all(|check| check.passed);
    let blockers = checks
        .iter()
        .flat_map(|check| check.blockers.clone())
        .collect::<Vec<String>>();

    Ok(KernelLoopbackR5CloseoutR6RustRuntimeScaffoldReport {
        runtime_id: MIHOMO_RUNTIME_ID.into(),
        component: "r5-closeout-r6-rust-runtime-scaffold".into(),
        mutates_runtime: false,
        live_execution_allowed: false,
        rust_runtime_scaffold_decision,
        scaffold_ready,
        default_cutover_allowed: false,
        r5_closeout,
        runtime_selection,
        checks,
        blockers,
        warnings: vec!["R6 scaffold is selectable metadata only; Rust runtime remains disabled".into()],
        facts: vec!["next batch can implement explicit opt-in Rust runtime MVP without more R5 gates".into()],
        next_safe_batch: "r6-opt-in-rust-runtime-mvp".into(),
    })
}

pub async fn rust_kernel_runtime_r6_opt_in_mvp(
    listener_port: Option<u16>,
    target_port: Option<u16>,
    hold_started_at_epoch_ms: Option<u64>,
    observed_rollback_platforms: Option<Vec<String>>,
    explicit_decision: Option<bool>,
    requested_execution: Option<bool>,
    post_execution_hold_started_at_epoch_ms: Option<u64>,
    wider_opt_in_decision: Option<bool>,
    limited_rollout_decision: Option<bool>,
    canary_scope: Option<String>,
    max_canary_sessions: Option<u16>,
    closeout_decision: Option<bool>,
    handoff_decision: Option<bool>,
    r5_preflight_decision: Option<bool>,
    rollback_plan_decision: Option<bool>,
    execution_plan_decision: Option<bool>,
    guard_decision: Option<bool>,
    dry_run_decision: Option<bool>,
    dry_run_execution_decision: Option<bool>,
    post_dry_run_hold_started_at_epoch_ms: Option<u64>,
    hold_decision: Option<bool>,
    decision_readiness_decision: Option<bool>,
    final_gate_decision: Option<bool>,
    r5_handoff_decision: Option<bool>,
    final_hold_started_at_epoch_ms: Option<u64>,
    final_hold_decision: Option<bool>,
    independent_rollback_decision: Option<bool>,
    r5_closeout_decision: Option<bool>,
    r5_closeout_report_decision: Option<bool>,
    requested_runtime_kind: Option<String>,
    rust_runtime_opt_in_decision: Option<bool>,
    rust_runtime_scaffold_decision: Option<bool>,
) -> Result<KernelLoopbackR6OptInRustRuntimeMvpReport> {
    let requested_runtime_kind_for_parse = requested_runtime_kind.clone();
    let rust_runtime_opt_in_decision = rust_runtime_opt_in_decision.unwrap_or(false);
    let requested_runtime_kind = parse_kernel_runtime_kind(requested_runtime_kind_for_parse);
    let scaffold = Box::pin(mihomo_kernel_loopback_r5_closeout_r6_rust_runtime_scaffold(
        listener_port,
        target_port,
        hold_started_at_epoch_ms,
        observed_rollback_platforms,
        explicit_decision,
        requested_execution,
        post_execution_hold_started_at_epoch_ms,
        wider_opt_in_decision,
        limited_rollout_decision,
        canary_scope,
        max_canary_sessions,
        closeout_decision,
        handoff_decision,
        r5_preflight_decision,
        rollback_plan_decision,
        execution_plan_decision,
        guard_decision,
        dry_run_decision,
        dry_run_execution_decision,
        post_dry_run_hold_started_at_epoch_ms,
        hold_decision,
        decision_readiness_decision,
        final_gate_decision,
        r5_handoff_decision,
        final_hold_started_at_epoch_ms,
        final_hold_decision,
        independent_rollback_decision,
        r5_closeout_decision,
        r5_closeout_report_decision,
        Some("rust".into()),
        Some(rust_runtime_opt_in_decision),
        rust_runtime_scaffold_decision,
    ))
    .await?;
    let supported_subset = rust_kernel_runtime_supported_subset_report().await?;
    let subset_ready = supported_subset.blockers.is_empty()
        && supported_subset.rule_decision_owned
        && supported_subset.dns_decision_owned
        && supported_subset.adapter_decision_owned
        && supported_subset.forwarding_surface_owned;
    let requested_rust = matches!(requested_runtime_kind, KernelRuntimeKind::Rust);
    let pre_health_ready = scaffold.scaffold_ready && rust_runtime_opt_in_decision && requested_rust && subset_ready;
    let loopback_forwarding_evidence = if pre_health_ready {
        Some(mihomo_kernel_loopback_forwarding_rollback_drill(listener_port, target_port).await?)
    } else {
        None
    };
    let health_state = rust_kernel_runtime_health_state_report(pre_health_ready, loopback_forwarding_evidence.as_ref());
    let checks = vec![
        KernelLoopbackR4ExpandedOptInLimitedRolloutGateCheck {
            name: "r6ScaffoldReady".into(),
            status: if scaffold.scaffold_ready { "passed" } else { "blocked" }.into(),
            passed: scaffold.scaffold_ready,
            blockers: scaffold.blockers.clone(),
            facts: vec!["R5 closeout and Rust runtime scaffold are required before opt-in".into()],
        },
        KernelLoopbackR4ExpandedOptInLimitedRolloutGateCheck {
            name: "requestedRustRuntime".into(),
            status: if requested_rust { "passed" } else { "blocked" }.into(),
            passed: requested_rust,
            blockers: if requested_rust {
                Vec::new()
            } else {
                vec!["R6 opt-in MVP requires requested_runtime_kind=rust".into()]
            },
            facts: vec!["Mihomo remains selected unless Rust is explicitly requested".into()],
        },
        KernelLoopbackR4ExpandedOptInLimitedRolloutGateCheck {
            name: "rustOptInDecision".into(),
            status: if rust_runtime_opt_in_decision {
                "passed"
            } else {
                "blocked"
            }
            .into(),
            passed: rust_runtime_opt_in_decision,
            blockers: if rust_runtime_opt_in_decision {
                Vec::new()
            } else {
                vec!["R6 Rust runtime MVP requires explicit opt-in decision".into()]
            },
            facts: vec!["opt-in is scoped to supported subset and Mihomo fallback".into()],
        },
        KernelLoopbackR4ExpandedOptInLimitedRolloutGateCheck {
            name: "supportedSubsetDecisionPath".into(),
            status: if subset_ready { "passed" } else { "blocked" }.into(),
            passed: subset_ready,
            blockers: supported_subset.blockers.clone(),
            facts: vec!["Rust owns rule, DNS, and adapter decisions for the supported subset".into()],
        },
        KernelLoopbackR4ExpandedOptInLimitedRolloutGateCheck {
            name: "healthAndRollbackState".into(),
            status: if health_state.health_ready { "passed" } else { "blocked" }.into(),
            passed: health_state.health_ready,
            blockers: health_state.blockers.clone(),
            facts: vec!["loopback rollback evidence arms health and fallback state".into()],
        },
    ];
    let opt_in_ready = checks.iter().all(|check| check.passed);
    let selected_runtime_kind = if opt_in_ready {
        KernelRuntimeKind::Rust
    } else {
        KernelRuntimeKind::Mihomo
    };
    let blockers = checks
        .iter()
        .flat_map(|check| check.blockers.clone())
        .collect::<Vec<String>>();

    Ok(KernelLoopbackR6OptInRustRuntimeMvpReport {
        runtime_id: RUST_RUNTIME_ID.into(),
        component: "r6-opt-in-rust-runtime-mvp".into(),
        mutates_runtime: loopback_forwarding_evidence.is_some(),
        live_execution_allowed: opt_in_ready,
        rust_runtime_opt_in_decision,
        requested_runtime_kind,
        selected_runtime_kind,
        opt_in_ready,
        default_cutover_allowed: false,
        mihomo_fallback: true,
        scaffold,
        supported_subset,
        health_state,
        loopback_forwarding_evidence,
        checks,
        blockers,
        warnings: vec![
            "R6 MVP enables explicit opt-in metadata and loopback execution only".into(),
            "default Rust runtime still requires canary gate and automatic fallback evidence".into(),
        ],
        facts: vec![
            "Rust runtime is selectable for the supported subset only after explicit opt-in".into(),
            "Mihomo fallback remains active for unsupported protocols, TUN, adapters, and emergency rollback".into(),
        ],
        next_safe_batch: "r6-rust-default-canary".into(),
    })
}

fn rust_kernel_runtime_canary_profile_report(
    canary_scope: Option<String>,
    max_canary_sessions: Option<u16>,
) -> RustKernelRuntimeCanaryProfileReport {
    let canary_scope = canary_scope.unwrap_or_else(|| "loopbackSyntheticCanary".into());
    let max_canary_sessions = max_canary_sessions.unwrap_or(1);
    let mut blockers = Vec::new();

    if canary_scope != "loopbackSyntheticCanary" {
        blockers.push("R6 default canary is capped to loopbackSyntheticCanary".into());
    }
    if !(1..=3).contains(&max_canary_sessions) {
        blockers.push("R6 default canary allows 1 to 3 synthetic sessions only".into());
    }

    RustKernelRuntimeCanaryProfileReport {
        runtime_id: RUST_RUNTIME_ID.into(),
        component: "r6-rust-default-canary-profile".into(),
        canary_scope,
        max_canary_sessions,
        capped_profile: blockers.is_empty(),
        supported_safe_subset: rust_runtime_supported_safe_subset(),
        fallback_boundaries: rust_runtime_fallback_boundaries(),
        blockers,
        warnings: vec!["canary profile is a bounded default for the supported safe subset only".into()],
        facts: vec![
            "unsupported protocols, TUN, and production adapter egress remain Mihomo fallback".into(),
            "canary scope reuses the existing loopback-only safety cap".into(),
        ],
    }
}

fn rust_kernel_runtime_automatic_fallback_report(
    r6_opt_in: &KernelLoopbackR6OptInRustRuntimeMvpReport,
    health_check_passed: Option<bool>,
    rollback_triggered: Option<bool>,
) -> RustKernelRuntimeAutomaticFallbackReport {
    let health_check_passed = health_check_passed.unwrap_or(r6_opt_in.health_state.health_ready);
    let rollback_triggered = rollback_triggered.unwrap_or(false);
    let health_ready = r6_opt_in.health_state.health_ready && health_check_passed;
    let rollback_armed = r6_opt_in.health_state.rollback_armed && r6_opt_in.mihomo_fallback;
    let mut triggers = Vec::new();

    if !r6_opt_in.opt_in_ready {
        triggers.push("r6-opt-in-not-ready".into());
    }
    if !health_ready {
        triggers.push("health-check-not-ready".into());
    }
    if rollback_triggered {
        triggers.push("rollback-triggered".into());
    }
    if !rollback_armed {
        triggers.push("rollback-not-armed".into());
    }

    let fallback_activated = !triggers.is_empty();
    let selected_runtime_kind = if fallback_activated {
        KernelRuntimeKind::Mihomo
    } else {
        KernelRuntimeKind::Rust
    };
    let blockers = if fallback_activated {
        triggers
            .iter()
            .map(|trigger| format!("automatic fallback selected Mihomo: {trigger}").into())
            .collect()
    } else {
        Vec::new()
    };

    RustKernelRuntimeAutomaticFallbackReport {
        runtime_id: RUST_RUNTIME_ID.into(),
        component: "r6-rust-default-canary-automatic-fallback".into(),
        health_check_passed,
        rollback_triggered,
        health_ready,
        rollback_armed,
        fallback_activated,
        selected_runtime_kind,
        fallback_runtime_kind: KernelRuntimeKind::Mihomo,
        triggers,
        blockers,
        facts: vec![
            "Rust canary default selects Mihomo immediately on health or rollback triggers".into(),
            "fallback does not retire the Mihomo sidecar or unsupported runtime paths".into(),
        ],
    }
}

pub async fn rust_kernel_runtime_r6_default_canary(
    listener_port: Option<u16>,
    target_port: Option<u16>,
    hold_started_at_epoch_ms: Option<u64>,
    observed_rollback_platforms: Option<Vec<String>>,
    explicit_decision: Option<bool>,
    requested_execution: Option<bool>,
    post_execution_hold_started_at_epoch_ms: Option<u64>,
    wider_opt_in_decision: Option<bool>,
    limited_rollout_decision: Option<bool>,
    canary_scope: Option<String>,
    max_canary_sessions: Option<u16>,
    closeout_decision: Option<bool>,
    handoff_decision: Option<bool>,
    r5_preflight_decision: Option<bool>,
    rollback_plan_decision: Option<bool>,
    execution_plan_decision: Option<bool>,
    guard_decision: Option<bool>,
    dry_run_decision: Option<bool>,
    dry_run_execution_decision: Option<bool>,
    post_dry_run_hold_started_at_epoch_ms: Option<u64>,
    hold_decision: Option<bool>,
    decision_readiness_decision: Option<bool>,
    final_gate_decision: Option<bool>,
    r5_handoff_decision: Option<bool>,
    final_hold_started_at_epoch_ms: Option<u64>,
    final_hold_decision: Option<bool>,
    independent_rollback_decision: Option<bool>,
    r5_closeout_decision: Option<bool>,
    r5_closeout_report_decision: Option<bool>,
    requested_runtime_kind: Option<String>,
    rust_runtime_opt_in_decision: Option<bool>,
    rust_runtime_scaffold_decision: Option<bool>,
    canary_default_decision: Option<bool>,
    health_check_passed: Option<bool>,
    rollback_triggered: Option<bool>,
) -> Result<KernelLoopbackR6RustDefaultCanaryReport> {
    let canary_default_decision = canary_default_decision.unwrap_or(false);
    let requested_runtime_kind_for_parse = requested_runtime_kind.clone();
    let r6_opt_in = Box::pin(rust_kernel_runtime_r6_opt_in_mvp(
        listener_port,
        target_port,
        hold_started_at_epoch_ms,
        observed_rollback_platforms,
        explicit_decision,
        requested_execution,
        post_execution_hold_started_at_epoch_ms,
        wider_opt_in_decision,
        limited_rollout_decision,
        canary_scope.clone(),
        max_canary_sessions,
        closeout_decision,
        handoff_decision,
        r5_preflight_decision,
        rollback_plan_decision,
        execution_plan_decision,
        guard_decision,
        dry_run_decision,
        dry_run_execution_decision,
        post_dry_run_hold_started_at_epoch_ms,
        hold_decision,
        decision_readiness_decision,
        final_gate_decision,
        r5_handoff_decision,
        final_hold_started_at_epoch_ms,
        final_hold_decision,
        independent_rollback_decision,
        r5_closeout_decision,
        r5_closeout_report_decision,
        requested_runtime_kind,
        rust_runtime_opt_in_decision,
        rust_runtime_scaffold_decision,
    ))
    .await?;
    let canary_profile = rust_kernel_runtime_canary_profile_report(canary_scope, max_canary_sessions);
    let automatic_fallback =
        rust_kernel_runtime_automatic_fallback_report(&r6_opt_in, health_check_passed, rollback_triggered);
    let requested_runtime_kind = parse_kernel_runtime_kind(requested_runtime_kind_for_parse);
    let fallback_ready = automatic_fallback.rollback_armed && !automatic_fallback.fallback_activated;
    let checks = vec![
        KernelLoopbackR4ExpandedOptInLimitedRolloutGateCheck {
            name: "r6OptInReady".into(),
            status: if r6_opt_in.opt_in_ready { "passed" } else { "blocked" }.into(),
            passed: r6_opt_in.opt_in_ready,
            blockers: r6_opt_in.blockers.clone(),
            facts: vec!["R6 default canary builds on the explicit opt-in MVP".into()],
        },
        KernelLoopbackR4ExpandedOptInLimitedRolloutGateCheck {
            name: "canaryDefaultDecision".into(),
            status: if canary_default_decision { "passed" } else { "blocked" }.into(),
            passed: canary_default_decision,
            blockers: if canary_default_decision {
                Vec::new()
            } else {
                vec!["R6 Rust default canary requires an explicit canary default decision".into()]
            },
            facts: vec!["the canary decision is separate from production default cutover".into()],
        },
        KernelLoopbackR4ExpandedOptInLimitedRolloutGateCheck {
            name: "cappedCanaryProfile".into(),
            status: if canary_profile.capped_profile {
                "passed"
            } else {
                "blocked"
            }
            .into(),
            passed: canary_profile.capped_profile,
            blockers: canary_profile.blockers.clone(),
            facts: vec!["canary scope and session cap keep the default bounded".into()],
        },
        KernelLoopbackR4ExpandedOptInLimitedRolloutGateCheck {
            name: "automaticFallbackHealthy".into(),
            status: if fallback_ready { "passed" } else { "blocked" }.into(),
            passed: fallback_ready,
            blockers: automatic_fallback.blockers.clone(),
            facts: vec!["health and rollback triggers return selection to Mihomo".into()],
        },
        KernelLoopbackR4ExpandedOptInLimitedRolloutGateCheck {
            name: "productionDefaultBlocked".into(),
            status: "passed".into(),
            passed: true,
            blockers: Vec::new(),
            facts: vec!["R6 canary does not authorize R7 production default cutover".into()],
        },
    ];
    let canary_default_allowed = checks.iter().all(|check| check.passed);
    let selected_runtime_kind = if canary_default_allowed {
        KernelRuntimeKind::Rust
    } else {
        KernelRuntimeKind::Mihomo
    };
    let blockers = checks
        .iter()
        .flat_map(|check| check.blockers.clone())
        .collect::<Vec<String>>();

    Ok(KernelLoopbackR6RustDefaultCanaryReport {
        runtime_id: RUST_RUNTIME_ID.into(),
        component: "r6-rust-default-canary".into(),
        mutates_runtime: r6_opt_in.mutates_runtime,
        live_execution_allowed: canary_default_allowed,
        rust_runtime_opt_in_decision: r6_opt_in.rust_runtime_opt_in_decision,
        canary_default_decision,
        requested_runtime_kind,
        selected_runtime_kind,
        canary_default_allowed,
        production_default_allowed: false,
        mihomo_fallback: true,
        r6_opt_in,
        canary_profile,
        automatic_fallback,
        checks,
        blockers,
        warnings: vec![
            "R6 canary default is limited to the capped safe subset".into(),
            "R7 must complete canary closeout before Rust can become the wider default".into(),
        ],
        facts: vec![
            "Rust runtime is the selected default only inside the capped canary when all health gates pass".into(),
            "Mihomo fallback remains the selected runtime for unsupported paths and rollback triggers".into(),
        ],
        next_safe_batch: "r7-rust-default-cutover".into(),
    })
}

fn rust_kernel_runtime_r7_canary_closeout_summary(
    r6_canary: &KernelLoopbackR6RustDefaultCanaryReport,
    rollback_hold_decision: bool,
) -> RustKernelRuntimeCanaryCloseoutSummaryReport {
    let canary_health_ready = r6_canary.automatic_fallback.health_ready
        && r6_canary.automatic_fallback.health_check_passed
        && !r6_canary.automatic_fallback.rollback_triggered;
    let automatic_fallback_armed = r6_canary.automatic_fallback.rollback_armed
        && matches!(
            r6_canary.automatic_fallback.fallback_runtime_kind,
            KernelRuntimeKind::Mihomo
        );
    let closeout_ready =
        r6_canary.canary_default_allowed && canary_health_ready && automatic_fallback_armed && rollback_hold_decision;
    let mut blockers = Vec::new();

    if !r6_canary.canary_default_allowed {
        blockers.push("R7 cutover requires a passing R6 Rust default canary".into());
    }
    if !canary_health_ready {
        blockers.push("R7 cutover requires canary health checks to remain ready".into());
    }
    if !automatic_fallback_armed {
        blockers.push("R7 cutover requires automatic Mihomo fallback to stay armed".into());
    }
    if !rollback_hold_decision {
        blockers.push("R7 cutover requires rollback hold evidence before widening the default".into());
    }

    RustKernelRuntimeCanaryCloseoutSummaryReport {
        runtime_id: RUST_RUNTIME_ID.into(),
        component: "r7-rust-default-cutover-canary-closeout".into(),
        canary_default_allowed: r6_canary.canary_default_allowed,
        canary_health_ready,
        automatic_fallback_armed,
        rollback_hold_passed: rollback_hold_decision,
        closeout_ready,
        evidence: vec![
            "get_runtime_kernel_loopback_r6_rust_default_canary".into(),
            "canary health check".into(),
            "automatic Mihomo fallback state".into(),
            "rollback hold decision".into(),
        ],
        blockers,
        facts: vec![
            "R7 consumes R6 canary closeout evidence instead of retiring Mihomo fallback".into(),
            "rollback hold is required before Rust becomes the supported profile default".into(),
        ],
    }
}

fn rust_kernel_runtime_supported_profile_default_report(
    profile_scope: Option<String>,
    canary_closeout: &RustKernelRuntimeCanaryCloseoutSummaryReport,
    r7_cutover_decision: bool,
    rollback_switch_requested: bool,
) -> RustKernelRuntimeSupportedProfileDefaultReport {
    let profile_scope = profile_scope.unwrap_or_else(|| "supportedDefaultProfile".into());
    let mut blockers = Vec::new();

    if profile_scope != "supportedDefaultProfile" {
        blockers.push("R7 Rust default cutover is limited to supportedDefaultProfile".into());
    }
    if !canary_closeout.closeout_ready {
        blockers.extend(canary_closeout.blockers.clone());
    }
    if !r7_cutover_decision {
        blockers.push("R7 Rust default cutover requires an explicit cutover decision".into());
    }
    if rollback_switch_requested {
        blockers.push("one-switch rollback currently selects Mihomo as the default".into());
    }

    let supported_profile_default = blockers.is_empty();
    let selected_runtime_kind = if supported_profile_default {
        KernelRuntimeKind::Rust
    } else {
        KernelRuntimeKind::Mihomo
    };

    RustKernelRuntimeSupportedProfileDefaultReport {
        runtime_id: RUST_RUNTIME_ID.into(),
        component: "r7-rust-supported-profile-default".into(),
        profile_scope,
        supported_profile_default,
        selected_runtime_kind,
        fallback_runtime_kind: KernelRuntimeKind::Mihomo,
        supported_safe_subset: rust_runtime_supported_safe_subset(),
        fallback_boundaries: rust_runtime_fallback_boundaries(),
        blockers,
        warnings: vec![
            "R7 default applies only to the supported profile; unsupported protocol, TUN, and adapter paths stay on Mihomo fallback".into(),
        ],
        facts: vec![
            "Rust runtime is selected as the wider default only after canary closeout and rollback hold pass".into(),
            "Mihomo remains available without app restart through the rollback switch".into(),
        ],
    }
}

fn rust_kernel_runtime_r7_fallback_state_report(
    r6_canary: &KernelLoopbackR6RustDefaultCanaryReport,
    rollback_switch_requested: bool,
    supported_profile_default: bool,
) -> RustKernelRuntimeFallbackStateReport {
    let health_ready = r6_canary.automatic_fallback.health_ready
        && r6_canary.automatic_fallback.health_check_passed
        && !r6_canary.automatic_fallback.rollback_triggered;
    let rollback_armed = r6_canary.automatic_fallback.rollback_armed && r6_canary.mihomo_fallback;
    let mut triggers = Vec::new();

    if rollback_switch_requested {
        triggers.push("rollback-switch-requested".into());
    }
    if !supported_profile_default {
        triggers.push("supported-profile-default-not-ready".into());
    }
    if !health_ready {
        triggers.push("health-check-not-ready".into());
    }
    if !rollback_armed {
        triggers.push("rollback-not-armed".into());
    }

    let fallback_active = !triggers.is_empty();
    let selected_runtime_kind = if fallback_active {
        KernelRuntimeKind::Mihomo
    } else {
        KernelRuntimeKind::Rust
    };
    let blockers = if fallback_active && !rollback_switch_requested {
        triggers
            .iter()
            .map(|trigger| format!("R7 fallback keeps Mihomo selected: {trigger}").into())
            .collect()
    } else {
        Vec::new()
    };

    RustKernelRuntimeFallbackStateReport {
        runtime_id: RUST_RUNTIME_ID.into(),
        component: "r7-rust-default-cutover-fallback-state".into(),
        rollback_switch_requested,
        restart_required: false,
        health_ready,
        rollback_armed,
        fallback_active,
        selected_runtime_kind,
        fallback_runtime_kind: KernelRuntimeKind::Mihomo,
        triggers,
        blockers,
        facts: vec![
            "one-switch rollback restores Mihomo default selection without app restart".into(),
            "fallback state is queryable over IPC before and after R7 cutover".into(),
        ],
    }
}

pub async fn rust_kernel_runtime_r7_default_cutover(
    listener_port: Option<u16>,
    target_port: Option<u16>,
    hold_started_at_epoch_ms: Option<u64>,
    observed_rollback_platforms: Option<Vec<String>>,
    explicit_decision: Option<bool>,
    requested_execution: Option<bool>,
    post_execution_hold_started_at_epoch_ms: Option<u64>,
    wider_opt_in_decision: Option<bool>,
    limited_rollout_decision: Option<bool>,
    canary_scope: Option<String>,
    max_canary_sessions: Option<u16>,
    closeout_decision: Option<bool>,
    handoff_decision: Option<bool>,
    r5_preflight_decision: Option<bool>,
    rollback_plan_decision: Option<bool>,
    execution_plan_decision: Option<bool>,
    guard_decision: Option<bool>,
    dry_run_decision: Option<bool>,
    dry_run_execution_decision: Option<bool>,
    post_dry_run_hold_started_at_epoch_ms: Option<u64>,
    hold_decision: Option<bool>,
    decision_readiness_decision: Option<bool>,
    final_gate_decision: Option<bool>,
    r5_handoff_decision: Option<bool>,
    final_hold_started_at_epoch_ms: Option<u64>,
    final_hold_decision: Option<bool>,
    independent_rollback_decision: Option<bool>,
    r5_closeout_decision: Option<bool>,
    r5_closeout_report_decision: Option<bool>,
    requested_runtime_kind: Option<String>,
    rust_runtime_opt_in_decision: Option<bool>,
    rust_runtime_scaffold_decision: Option<bool>,
    canary_default_decision: Option<bool>,
    health_check_passed: Option<bool>,
    rollback_triggered: Option<bool>,
    r7_cutover_decision: Option<bool>,
    rollback_hold_decision: Option<bool>,
    rollback_switch_requested: Option<bool>,
    profile_scope: Option<String>,
) -> Result<KernelLoopbackR7RustDefaultCutoverReport> {
    let r7_cutover_decision = r7_cutover_decision.unwrap_or(false);
    let rollback_hold_decision = rollback_hold_decision.unwrap_or(false);
    let rollback_switch_requested = rollback_switch_requested.unwrap_or(false);
    let requested_runtime_kind_for_parse = requested_runtime_kind.clone();
    let r6_canary = Box::pin(rust_kernel_runtime_r6_default_canary(
        listener_port,
        target_port,
        hold_started_at_epoch_ms,
        observed_rollback_platforms,
        explicit_decision,
        requested_execution,
        post_execution_hold_started_at_epoch_ms,
        wider_opt_in_decision,
        limited_rollout_decision,
        canary_scope,
        max_canary_sessions,
        closeout_decision,
        handoff_decision,
        r5_preflight_decision,
        rollback_plan_decision,
        execution_plan_decision,
        guard_decision,
        dry_run_decision,
        dry_run_execution_decision,
        post_dry_run_hold_started_at_epoch_ms,
        hold_decision,
        decision_readiness_decision,
        final_gate_decision,
        r5_handoff_decision,
        final_hold_started_at_epoch_ms,
        final_hold_decision,
        independent_rollback_decision,
        r5_closeout_decision,
        r5_closeout_report_decision,
        requested_runtime_kind,
        rust_runtime_opt_in_decision,
        rust_runtime_scaffold_decision,
        canary_default_decision,
        health_check_passed,
        rollback_triggered,
    ))
    .await?;
    let canary_closeout = rust_kernel_runtime_r7_canary_closeout_summary(&r6_canary, rollback_hold_decision);
    let supported_profile = rust_kernel_runtime_supported_profile_default_report(
        profile_scope,
        &canary_closeout,
        r7_cutover_decision,
        rollback_switch_requested,
    );
    let fallback_state = rust_kernel_runtime_r7_fallback_state_report(
        &r6_canary,
        rollback_switch_requested,
        supported_profile.supported_profile_default,
    );
    let checks = vec![
        KernelLoopbackR4ExpandedOptInLimitedRolloutGateCheck {
            name: "r6CanaryCloseoutReady".into(),
            status: if canary_closeout.closeout_ready {
                "passed"
            } else {
                "blocked"
            }
            .into(),
            passed: canary_closeout.closeout_ready,
            blockers: canary_closeout.blockers.clone(),
            facts: vec!["R7 cutover consumes R6 canary health and rollback hold evidence".into()],
        },
        KernelLoopbackR4ExpandedOptInLimitedRolloutGateCheck {
            name: "r7CutoverDecision".into(),
            status: if r7_cutover_decision { "passed" } else { "blocked" }.into(),
            passed: r7_cutover_decision,
            blockers: if r7_cutover_decision {
                Vec::new()
            } else {
                vec!["R7 Rust default cutover requires an explicit cutover decision".into()]
            },
            facts: vec!["cutover decision widens Rust default selection only for supported profile".into()],
        },
        KernelLoopbackR4ExpandedOptInLimitedRolloutGateCheck {
            name: "supportedProfileDefault".into(),
            status: if supported_profile.supported_profile_default {
                "passed"
            } else {
                "blocked"
            }
            .into(),
            passed: supported_profile.supported_profile_default,
            blockers: supported_profile.blockers.clone(),
            facts: vec!["unsupported protocol, TUN, and adapter paths remain Mihomo fallback".into()],
        },
        KernelLoopbackR4ExpandedOptInLimitedRolloutGateCheck {
            name: "oneSwitchRollbackPath".into(),
            status: if fallback_state.rollback_armed && !fallback_state.restart_required {
                "passed"
            } else {
                "blocked"
            }
            .into(),
            passed: fallback_state.rollback_armed && !fallback_state.restart_required,
            blockers: fallback_state.blockers.clone(),
            facts: vec!["rollback switch restores Mihomo default without app restart".into()],
        },
    ];
    let supported_profile_default_allowed = checks.iter().all(|check| check.passed) && !fallback_state.fallback_active;
    let selected_runtime_kind = if supported_profile_default_allowed {
        KernelRuntimeKind::Rust
    } else {
        KernelRuntimeKind::Mihomo
    };
    let blockers = checks
        .iter()
        .flat_map(|check| check.blockers.clone())
        .collect::<Vec<String>>();

    Ok(KernelLoopbackR7RustDefaultCutoverReport {
        runtime_id: RUST_RUNTIME_ID.into(),
        component: "r7-rust-default-cutover".into(),
        mutates_runtime: r6_canary.mutates_runtime,
        live_execution_allowed: supported_profile_default_allowed,
        rust_runtime_opt_in_decision: r6_canary.rust_runtime_opt_in_decision,
        canary_default_decision: r6_canary.canary_default_decision,
        r7_cutover_decision,
        rollback_hold_decision,
        rollback_switch_requested,
        requested_runtime_kind: parse_kernel_runtime_kind(requested_runtime_kind_for_parse),
        selected_runtime_kind,
        supported_profile_default_allowed,
        production_default_allowed: false,
        mihomo_fallback: true,
        r6_canary,
        canary_closeout,
        supported_profile,
        fallback_state,
        checks,
        blockers,
        warnings: vec![
            "R7 selects Rust only for the supported profile; full Mihomo fallback retirement remains blocked".into(),
            "TUN, transparent proxy, protocol stacks, and production adapter egress are not replaced in this batch"
                .into(),
        ],
        facts: vec![
            "Rust runtime becomes the supported profile default only after canary closeout and rollback hold pass"
                .into(),
            "Mihomo fallback remains available for unsupported paths and one-switch rollback".into(),
        ],
        next_safe_batch: "r7-mihomo-fallback-retirement".into(),
    })
}

fn rust_kernel_runtime_r7_fallback_retirement_parity_report(
    r7_cutover: &KernelLoopbackR7RustDefaultCutoverReport,
    protocol_parity_decision: bool,
    tun_parity_decision: bool,
    adapter_parity_decision: bool,
    dns_runtime_parity_decision: bool,
    cross_platform_rollback_decision: bool,
    soak_evidence_decision: bool,
) -> RustKernelRuntimeFallbackRetirementParityReport {
    let mut blockers = Vec::new();

    if !r7_cutover.supported_profile_default_allowed {
        blockers.push("fallback retirement requires R7 supported profile cutover to be ready".into());
    }
    if !protocol_parity_decision {
        blockers.push("fallback retirement requires outbound and inbound protocol parity evidence".into());
    }
    if !tun_parity_decision {
        blockers.push("fallback retirement requires TUN and transparent proxy parity evidence".into());
    }
    if !adapter_parity_decision {
        blockers.push("fallback retirement requires production adapter runtime parity evidence".into());
    }
    if !dns_runtime_parity_decision {
        blockers.push("fallback retirement requires default DNS runtime parity evidence".into());
    }
    if !cross_platform_rollback_decision {
        blockers.push("fallback retirement requires cross-platform rollback drills".into());
    }
    if !soak_evidence_decision {
        blockers.push("fallback retirement requires cross-platform soak evidence".into());
    }

    RustKernelRuntimeFallbackRetirementParityReport {
        runtime_id: RUST_RUNTIME_ID.into(),
        component: "r7-mihomo-fallback-retirement-parity".into(),
        protocol_parity_passed: protocol_parity_decision,
        tun_parity_passed: tun_parity_decision,
        adapter_parity_passed: adapter_parity_decision,
        dns_runtime_parity_passed: dns_runtime_parity_decision,
        cross_platform_rollback_passed: cross_platform_rollback_decision,
        soak_evidence_passed: soak_evidence_decision,
        parity_complete: blockers.is_empty(),
        retained_boundaries: vec![
            "protocol stacks remain blocked until explicit parity evidence passes".into(),
            "TUN and transparent proxy remain blocked until explicit parity evidence passes".into(),
            "adapter runtime and default DNS remain blocked until explicit parity evidence passes".into(),
        ],
        blockers,
        facts: vec![
            "fallback retirement consumes R7 cutover readiness before evaluating data-plane parity".into(),
            "fallback retirement is blocked by default; every high-risk data-plane area needs explicit evidence".into(),
        ],
    }
}

fn rust_kernel_runtime_r7_fallback_retirement_plan_report(
    parity: &RustKernelRuntimeFallbackRetirementParityReport,
    fallback_retirement_decision: bool,
    emergency_rollback_decision: bool,
    rollback_switch_requested: bool,
) -> RustKernelRuntimeFallbackRetirementPlanReport {
    let mut blockers = parity.blockers.clone();
    let mut warnings = Vec::new();

    if !fallback_retirement_decision {
        blockers.push("Mihomo fallback retirement requires an explicit retirement decision".into());
    }
    if !emergency_rollback_decision {
        blockers.push("Mihomo fallback retirement requires an emergency rollback decision".into());
    }
    if rollback_switch_requested {
        blockers.push("one-switch rollback currently keeps Mihomo as the selected runtime".into());
    }

    if parity.parity_complete && !fallback_retirement_decision {
        warnings.push(
            "parity evidence is present, but fallback retirement remains disabled until explicitly decided".into(),
        );
    }

    let fallback_retirement_allowed = blockers.is_empty();

    RustKernelRuntimeFallbackRetirementPlanReport {
        runtime_id: RUST_RUNTIME_ID.into(),
        component: "r7-mihomo-fallback-retirement-plan".into(),
        fallback_retirement_decision,
        emergency_rollback_decision,
        rollback_switch_requested,
        fallback_retirement_allowed,
        selected_runtime_kind: if fallback_retirement_allowed {
            KernelRuntimeKind::Rust
        } else {
            KernelRuntimeKind::Mihomo
        },
        rollback_runtime_kind: KernelRuntimeKind::Mihomo,
        restart_required: false,
        blockers,
        warnings,
        facts: vec![
            "emergency rollback remains a one-switch Mihomo selection and does not require app restart".into(),
            "retirement only removes fallback dependence after parity, rollback drills, and soak evidence pass".into(),
        ],
    }
}

pub async fn rust_kernel_runtime_r7_mihomo_fallback_retirement(
    listener_port: Option<u16>,
    target_port: Option<u16>,
    hold_started_at_epoch_ms: Option<u64>,
    observed_rollback_platforms: Option<Vec<String>>,
    explicit_decision: Option<bool>,
    requested_execution: Option<bool>,
    post_execution_hold_started_at_epoch_ms: Option<u64>,
    wider_opt_in_decision: Option<bool>,
    limited_rollout_decision: Option<bool>,
    canary_scope: Option<String>,
    max_canary_sessions: Option<u16>,
    closeout_decision: Option<bool>,
    handoff_decision: Option<bool>,
    r5_preflight_decision: Option<bool>,
    rollback_plan_decision: Option<bool>,
    execution_plan_decision: Option<bool>,
    guard_decision: Option<bool>,
    dry_run_decision: Option<bool>,
    dry_run_execution_decision: Option<bool>,
    post_dry_run_hold_started_at_epoch_ms: Option<u64>,
    hold_decision: Option<bool>,
    decision_readiness_decision: Option<bool>,
    final_gate_decision: Option<bool>,
    r5_handoff_decision: Option<bool>,
    final_hold_started_at_epoch_ms: Option<u64>,
    final_hold_decision: Option<bool>,
    independent_rollback_decision: Option<bool>,
    r5_closeout_decision: Option<bool>,
    r5_closeout_report_decision: Option<bool>,
    requested_runtime_kind: Option<String>,
    rust_runtime_opt_in_decision: Option<bool>,
    rust_runtime_scaffold_decision: Option<bool>,
    canary_default_decision: Option<bool>,
    health_check_passed: Option<bool>,
    rollback_triggered: Option<bool>,
    r7_cutover_decision: Option<bool>,
    rollback_hold_decision: Option<bool>,
    rollback_switch_requested: Option<bool>,
    profile_scope: Option<String>,
    protocol_parity_decision: Option<bool>,
    tun_parity_decision: Option<bool>,
    adapter_parity_decision: Option<bool>,
    dns_runtime_parity_decision: Option<bool>,
    cross_platform_rollback_decision: Option<bool>,
    soak_evidence_decision: Option<bool>,
    fallback_retirement_decision: Option<bool>,
    emergency_rollback_decision: Option<bool>,
) -> Result<KernelLoopbackR7MihomoFallbackRetirementReport> {
    let protocol_parity_decision = protocol_parity_decision.unwrap_or(false);
    let tun_parity_decision = tun_parity_decision.unwrap_or(false);
    let adapter_parity_decision = adapter_parity_decision.unwrap_or(false);
    let dns_runtime_parity_decision = dns_runtime_parity_decision.unwrap_or(false);
    let cross_platform_rollback_decision = cross_platform_rollback_decision.unwrap_or(false);
    let soak_evidence_decision = soak_evidence_decision.unwrap_or(false);
    let fallback_retirement_decision = fallback_retirement_decision.unwrap_or(false);
    let emergency_rollback_decision = emergency_rollback_decision.unwrap_or(false);
    let rollback_switch_requested_value = rollback_switch_requested.unwrap_or(false);
    let r7_cutover = Box::pin(rust_kernel_runtime_r7_default_cutover(
        listener_port,
        target_port,
        hold_started_at_epoch_ms,
        observed_rollback_platforms,
        explicit_decision,
        requested_execution,
        post_execution_hold_started_at_epoch_ms,
        wider_opt_in_decision,
        limited_rollout_decision,
        canary_scope,
        max_canary_sessions,
        closeout_decision,
        handoff_decision,
        r5_preflight_decision,
        rollback_plan_decision,
        execution_plan_decision,
        guard_decision,
        dry_run_decision,
        dry_run_execution_decision,
        post_dry_run_hold_started_at_epoch_ms,
        hold_decision,
        decision_readiness_decision,
        final_gate_decision,
        r5_handoff_decision,
        final_hold_started_at_epoch_ms,
        final_hold_decision,
        independent_rollback_decision,
        r5_closeout_decision,
        r5_closeout_report_decision,
        requested_runtime_kind,
        rust_runtime_opt_in_decision,
        rust_runtime_scaffold_decision,
        canary_default_decision,
        health_check_passed,
        rollback_triggered,
        r7_cutover_decision,
        rollback_hold_decision,
        Some(rollback_switch_requested_value),
        profile_scope,
    ))
    .await?;
    let parity = rust_kernel_runtime_r7_fallback_retirement_parity_report(
        &r7_cutover,
        protocol_parity_decision,
        tun_parity_decision,
        adapter_parity_decision,
        dns_runtime_parity_decision,
        cross_platform_rollback_decision,
        soak_evidence_decision,
    );
    let retirement_plan = rust_kernel_runtime_r7_fallback_retirement_plan_report(
        &parity,
        fallback_retirement_decision,
        emergency_rollback_decision,
        rollback_switch_requested_value,
    );
    let checks = vec![
        KernelLoopbackR4ExpandedOptInLimitedRolloutGateCheck {
            name: "r7SupportedProfileCutover".into(),
            status: if r7_cutover.supported_profile_default_allowed {
                "passed"
            } else {
                "blocked"
            }
            .into(),
            passed: r7_cutover.supported_profile_default_allowed,
            blockers: r7_cutover.blockers.clone(),
            facts: vec!["Mihomo fallback cannot retire before R7 supported profile cutover is ready".into()],
        },
        KernelLoopbackR4ExpandedOptInLimitedRolloutGateCheck {
            name: "protocolTunAdapterDnsParity".into(),
            status: if parity.parity_complete { "passed" } else { "blocked" }.into(),
            passed: parity.parity_complete,
            blockers: parity.blockers.clone(),
            facts: vec!["protocol, TUN, adapter, DNS, rollback drill, and soak evidence are evaluated together".into()],
        },
        KernelLoopbackR4ExpandedOptInLimitedRolloutGateCheck {
            name: "fallbackRetirementDecision".into(),
            status: if fallback_retirement_decision {
                "passed"
            } else {
                "blocked"
            }
            .into(),
            passed: fallback_retirement_decision,
            blockers: if fallback_retirement_decision {
                Vec::new()
            } else {
                vec!["explicit Mihomo fallback retirement decision is required".into()]
            },
            facts: vec!["retirement is an explicit high-risk data-plane decision".into()],
        },
        KernelLoopbackR4ExpandedOptInLimitedRolloutGateCheck {
            name: "emergencyRollbackPath".into(),
            status: if emergency_rollback_decision && !retirement_plan.restart_required {
                "passed"
            } else {
                "blocked"
            }
            .into(),
            passed: emergency_rollback_decision && !retirement_plan.restart_required,
            blockers: if emergency_rollback_decision {
                Vec::new()
            } else {
                vec!["emergency one-switch Mihomo rollback path must remain available".into()]
            },
            facts: vec!["fallback retirement keeps a restart-free rollback selector".into()],
        },
    ];
    let mihomo_fallback_retired =
        checks.iter().all(|check| check.passed) && retirement_plan.fallback_retirement_allowed;
    let selected_runtime_kind = if mihomo_fallback_retired {
        KernelRuntimeKind::Rust
    } else {
        KernelRuntimeKind::Mihomo
    };
    let blockers = checks
        .iter()
        .flat_map(|check| check.blockers.clone())
        .collect::<Vec<String>>();

    Ok(KernelLoopbackR7MihomoFallbackRetirementReport {
        runtime_id: RUST_RUNTIME_ID.into(),
        component: "r7-mihomo-fallback-retirement".into(),
        mutates_runtime: false,
        live_execution_allowed: mihomo_fallback_retired,
        r7_cutover,
        parity,
        retirement_plan,
        production_default_allowed: mihomo_fallback_retired,
        mihomo_fallback_retired,
        selected_runtime_kind,
        rollback_runtime_kind: KernelRuntimeKind::Mihomo,
        checks,
        blockers,
        warnings: vec![
            "fallback retirement is blocked by default and requires protocol/TUN/adapter/DNS parity evidence".into(),
            "this IPC surface reports retirement readiness; production mutation remains app-owned and explicitly gated"
                .into(),
        ],
        facts: vec![
            "R7 fallback retirement consumes R7 cutover readiness before considering full replacement".into(),
            "Mihomo remains the rollback runtime even when retirement readiness passes".into(),
        ],
        next_safe_batch: if mihomo_fallback_retired {
            "full-rust-runtime-hardening".into()
        } else {
            "r7-mihomo-fallback-retirement".into()
        },
    })
}

fn rust_kernel_runtime_full_hardening_extended_soak_report(
    observed_soak_hours: Option<u32>,
    health_regression_count: Option<u32>,
    rollback_trigger_count: Option<u32>,
) -> RustKernelRuntimeExtendedSoakReport {
    let observed_soak_hours = observed_soak_hours.unwrap_or(0);
    let health_regression_count = health_regression_count.unwrap_or(0);
    let rollback_trigger_count = rollback_trigger_count.unwrap_or(0);
    let mut blockers = Vec::new();

    if observed_soak_hours < FULL_RUST_RUNTIME_HARDENING_MIN_SOAK_HOURS {
        blockers.push(
            format!(
                "full Rust runtime hardening requires at least {} soak hours",
                FULL_RUST_RUNTIME_HARDENING_MIN_SOAK_HOURS
            )
            .into(),
        );
    }
    if health_regression_count > 0 {
        blockers.push("full Rust runtime hardening requires zero health regressions during soak".into());
    }
    if rollback_trigger_count > 0 {
        blockers.push("full Rust runtime hardening requires zero rollback triggers during soak".into());
    }

    RustKernelRuntimeExtendedSoakReport {
        runtime_id: RUST_RUNTIME_ID.into(),
        component: "full-rust-runtime-hardening-extended-soak".into(),
        min_soak_hours: FULL_RUST_RUNTIME_HARDENING_MIN_SOAK_HOURS,
        observed_soak_hours,
        health_regression_count,
        rollback_trigger_count,
        soak_complete: blockers.is_empty(),
        blockers,
        facts: vec![
            "hardening requires extended soak after R7 fallback retirement readiness".into(),
            "soak evidence is blocked by default and must be supplied explicitly".into(),
        ],
    }
}

fn rust_kernel_runtime_full_hardening_rollback_telemetry_report(
    rollback_telemetry_decision: bool,
    emergency_rollback_ready: bool,
    rollback_event_count: Option<u32>,
    last_rollback_event_ts: Option<u64>,
) -> RustKernelRuntimeRollbackTelemetryReport {
    let rollback_event_count = rollback_event_count.unwrap_or(0);
    let mut blockers = Vec::new();

    if !rollback_telemetry_decision {
        blockers.push("full Rust runtime hardening requires explicit rollback telemetry closeout".into());
    }
    if !emergency_rollback_ready {
        blockers.push("full Rust runtime hardening requires emergency Mihomo rollback readiness".into());
    }
    if rollback_event_count > 0 {
        blockers.push("full Rust runtime hardening requires zero unresolved rollback events".into());
    }

    RustKernelRuntimeRollbackTelemetryReport {
        runtime_id: RUST_RUNTIME_ID.into(),
        component: "full-rust-runtime-hardening-rollback-telemetry".into(),
        rollback_telemetry_decision,
        emergency_rollback_ready,
        rollback_event_count,
        last_rollback_event_ts,
        telemetry_complete: blockers.is_empty(),
        blockers,
        facts: vec![
            "rollback telemetry must remain queryable after fallback retirement readiness".into(),
            "Mihomo remains the restart-free emergency rollback runtime during hardening".into(),
        ],
    }
}

fn rust_kernel_runtime_full_hardening_platform_follow_up_report(
    windows_service_hardening: bool,
    macos_service_hardening: bool,
    linux_service_hardening: bool,
) -> RustKernelRuntimePlatformHardeningFollowUpReport {
    let mut blockers = Vec::new();

    if !windows_service_hardening {
        blockers.push("Windows service hardening follow-up is required".into());
    }
    if !macos_service_hardening {
        blockers.push("macOS service hardening follow-up is required".into());
    }
    if !linux_service_hardening {
        blockers.push("Linux service hardening follow-up is required".into());
    }

    RustKernelRuntimePlatformHardeningFollowUpReport {
        runtime_id: RUST_RUNTIME_ID.into(),
        component: "full-rust-runtime-hardening-platform-follow-up".into(),
        windows_service_hardening,
        macos_service_hardening,
        linux_service_hardening,
        platform_follow_up_complete: blockers.is_empty(),
        blockers,
        facts: vec![
            "platform hardening follows up service, sidecar, and rollback semantics per OS".into(),
            "all platform decisions are explicit to avoid silently retiring Go/Mihomo boundaries".into(),
        ],
    }
}

#[allow(clippy::too_many_arguments)]
pub async fn rust_kernel_runtime_full_rust_runtime_hardening(
    r7_fallback_retirement_passed: Option<bool>,
    observed_soak_hours: Option<u32>,
    health_regression_count: Option<u32>,
    rollback_trigger_count: Option<u32>,
    rollback_event_count: Option<u32>,
    last_rollback_event_ts: Option<u64>,
    rollback_telemetry_decision: Option<bool>,
    emergency_rollback_decision: Option<bool>,
    windows_service_hardening_decision: Option<bool>,
    macos_service_hardening_decision: Option<bool>,
    linux_service_hardening_decision: Option<bool>,
    final_hardening_decision: Option<bool>,
) -> Result<KernelLoopbackFullRustRuntimeHardeningReport> {
    let r7_fallback_retirement_passed = r7_fallback_retirement_passed.unwrap_or(false);
    let hardening_decision = final_hardening_decision.unwrap_or(false);
    let extended_soak = rust_kernel_runtime_full_hardening_extended_soak_report(
        observed_soak_hours,
        health_regression_count,
        rollback_trigger_count,
    );
    let rollback_telemetry = rust_kernel_runtime_full_hardening_rollback_telemetry_report(
        rollback_telemetry_decision.unwrap_or(false),
        emergency_rollback_decision.unwrap_or(false),
        rollback_event_count,
        last_rollback_event_ts,
    );
    let platform_follow_up = rust_kernel_runtime_full_hardening_platform_follow_up_report(
        windows_service_hardening_decision.unwrap_or(false),
        macos_service_hardening_decision.unwrap_or(false),
        linux_service_hardening_decision.unwrap_or(false),
    );
    let mut r7_blockers = Vec::new();

    if !r7_fallback_retirement_passed {
        r7_blockers.push("full Rust runtime hardening requires the R7 fallback retirement gate to pass".into());
    }

    let checks = vec![
        KernelLoopbackR4ExpandedOptInLimitedRolloutGateCheck {
            name: "r7FallbackRetirementReady".into(),
            status: if r7_fallback_retirement_passed {
                "passed"
            } else {
                "blocked"
            }
            .into(),
            passed: r7_fallback_retirement_passed,
            blockers: r7_blockers,
            facts: vec!["full Rust runtime hardening consumes the R7 retirement gate".into()],
        },
        KernelLoopbackR4ExpandedOptInLimitedRolloutGateCheck {
            name: "extendedSoakComplete".into(),
            status: if extended_soak.soak_complete {
                "passed"
            } else {
                "blocked"
            }
            .into(),
            passed: extended_soak.soak_complete,
            blockers: extended_soak.blockers.clone(),
            facts: vec!["extended soak must show no health regression or rollback trigger".into()],
        },
        KernelLoopbackR4ExpandedOptInLimitedRolloutGateCheck {
            name: "rollbackTelemetryComplete".into(),
            status: if rollback_telemetry.telemetry_complete {
                "passed"
            } else {
                "blocked"
            }
            .into(),
            passed: rollback_telemetry.telemetry_complete,
            blockers: rollback_telemetry.blockers.clone(),
            facts: vec!["rollback telemetry stays available after hardening".into()],
        },
        KernelLoopbackR4ExpandedOptInLimitedRolloutGateCheck {
            name: "platformHardeningFollowUp".into(),
            status: if platform_follow_up.platform_follow_up_complete {
                "passed"
            } else {
                "blocked"
            }
            .into(),
            passed: platform_follow_up.platform_follow_up_complete,
            blockers: platform_follow_up.blockers.clone(),
            facts: vec!["Windows, macOS, and Linux service hardening must all pass".into()],
        },
        KernelLoopbackR4ExpandedOptInLimitedRolloutGateCheck {
            name: "finalHardeningDecision".into(),
            status: if hardening_decision { "passed" } else { "blocked" }.into(),
            passed: hardening_decision,
            blockers: if hardening_decision {
                Vec::new()
            } else {
                vec!["full Rust runtime hardening requires an explicit final decision".into()]
            },
            facts: vec!["final hardening is an explicit app-owned Rust gate".into()],
        },
    ];
    let full_rust_runtime_hardened = checks.iter().all(|check| check.passed);
    let blockers = checks
        .iter()
        .flat_map(|check| check.blockers.clone())
        .collect::<Vec<String>>();

    Ok(KernelLoopbackFullRustRuntimeHardeningReport {
        runtime_id: RUST_RUNTIME_ID.into(),
        component: "full-rust-runtime-hardening".into(),
        mutates_runtime: false,
        live_execution_allowed: full_rust_runtime_hardened,
        hardening_decision,
        r7_fallback_retirement_passed,
        extended_soak,
        rollback_telemetry,
        platform_follow_up,
        full_rust_runtime_hardened,
        production_default_allowed: full_rust_runtime_hardened,
        selected_runtime_kind: if full_rust_runtime_hardened {
            KernelRuntimeKind::Rust
        } else {
            KernelRuntimeKind::Mihomo
        },
        rollback_runtime_kind: KernelRuntimeKind::Mihomo,
        checks,
        blockers,
        warnings: vec![
            "full Rust runtime hardening is blocked by default and does not mutate runtime state"
                .into(),
            "Mihomo remains the emergency rollback runtime until hardening closeout passes".into(),
        ],
        facts: vec![
            "this gate follows R7 fallback retirement and closes extended soak, rollback telemetry, and platform follow-up together".into(),
            "successful hardening advances the roadmap beyond Go/Mihomo fallback dependence".into(),
        ],
        next_safe_batch: if full_rust_runtime_hardened {
            "go-mihomo-retirement-audit".into()
        } else {
            "full-rust-runtime-hardening".into()
        },
    })
}

fn rust_kernel_runtime_go_mihomo_retirement_surface_audit_report(
    sidecar_source_audit_decision: bool,
    bundled_mihomo_audit_decision: bool,
    ipc_fallback_audit_decision: bool,
    docs_audit_decision: bool,
    emergency_rollback_retained: bool,
) -> RustKernelRuntimeGoMihomoRetirementSurfaceAuditReport {
    let mut blockers = Vec::new();
    let mut remaining_surfaces = Vec::new();

    if !sidecar_source_audit_decision {
        remaining_surfaces.push("mihomo sidecar source tree".into());
        blockers.push("Go/Mihomo retirement audit requires sidecar source inventory".into());
    }
    if !bundled_mihomo_audit_decision {
        remaining_surfaces.push("bundled Mihomo binary and updater artifacts".into());
        blockers.push("Go/Mihomo retirement audit requires bundled artifact inventory".into());
    }
    if !ipc_fallback_audit_decision {
        remaining_surfaces.push("IPC fallback and emergency rollback commands".into());
        blockers.push("Go/Mihomo retirement audit requires IPC fallback surface inventory".into());
    }
    if !docs_audit_decision {
        remaining_surfaces.push("operator docs and migration rollback runbooks".into());
        blockers.push("Go/Mihomo retirement audit requires docs and runbook inventory".into());
    }
    if !emergency_rollback_retained {
        blockers.push("Go/Mihomo retirement audit must retain emergency rollback until a later removal plan".into());
    }

    RustKernelRuntimeGoMihomoRetirementSurfaceAuditReport {
        runtime_id: RUST_RUNTIME_ID.into(),
        component: "go-mihomo-retirement-surface-audit".into(),
        sidecar_source_audit_passed: sidecar_source_audit_decision,
        bundled_mihomo_audit_passed: bundled_mihomo_audit_decision,
        ipc_fallback_audit_passed: ipc_fallback_audit_decision,
        docs_audit_passed: docs_audit_decision,
        emergency_rollback_retained,
        audit_complete: blockers.is_empty(),
        remaining_surfaces,
        blockers,
        facts: vec![
            "this audit inventories Go/Mihomo surfaces without deleting source, binaries, or rollback paths".into(),
            "emergency rollback remains a required retained surface for the next planning batch".into(),
        ],
    }
}

pub async fn rust_kernel_runtime_go_mihomo_retirement_audit(
    full_rust_runtime_hardened_decision: Option<bool>,
    sidecar_source_audit_decision: Option<bool>,
    bundled_mihomo_audit_decision: Option<bool>,
    ipc_fallback_audit_decision: Option<bool>,
    docs_audit_decision: Option<bool>,
    emergency_rollback_retained: Option<bool>,
    final_retirement_audit_decision: Option<bool>,
) -> Result<KernelLoopbackGoMihomoRetirementAuditReport> {
    let full_rust_runtime_hardened = full_rust_runtime_hardened_decision.unwrap_or(false);
    let final_retirement_audit_decision = final_retirement_audit_decision.unwrap_or(false);
    let surface_audit = rust_kernel_runtime_go_mihomo_retirement_surface_audit_report(
        sidecar_source_audit_decision.unwrap_or(false),
        bundled_mihomo_audit_decision.unwrap_or(false),
        ipc_fallback_audit_decision.unwrap_or(false),
        docs_audit_decision.unwrap_or(false),
        emergency_rollback_retained.unwrap_or(false),
    );
    let mut hardening_blockers = Vec::new();

    if !full_rust_runtime_hardened {
        hardening_blockers.push("Go/Mihomo retirement audit requires full Rust runtime hardening to pass".into());
    }

    let checks = vec![
        KernelLoopbackR4ExpandedOptInLimitedRolloutGateCheck {
            name: "fullRustRuntimeHardened".into(),
            status: if full_rust_runtime_hardened {
                "passed"
            } else {
                "blocked"
            }
            .into(),
            passed: full_rust_runtime_hardened,
            blockers: hardening_blockers,
            facts: vec!["retirement audit starts only after full Rust runtime hardening".into()],
        },
        KernelLoopbackR4ExpandedOptInLimitedRolloutGateCheck {
            name: "goMihomoSurfaceAuditComplete".into(),
            status: if surface_audit.audit_complete {
                "passed"
            } else {
                "blocked"
            }
            .into(),
            passed: surface_audit.audit_complete,
            blockers: surface_audit.blockers.clone(),
            facts: vec!["source, artifact, IPC, docs, and rollback surfaces are audited together".into()],
        },
        KernelLoopbackR4ExpandedOptInLimitedRolloutGateCheck {
            name: "finalRetirementAuditDecision".into(),
            status: if final_retirement_audit_decision {
                "passed"
            } else {
                "blocked"
            }
            .into(),
            passed: final_retirement_audit_decision,
            blockers: if final_retirement_audit_decision {
                Vec::new()
            } else {
                vec!["Go/Mihomo retirement audit requires an explicit final audit decision".into()]
            },
            facts: vec!["the audit is explicit and does not remove Mihomo".into()],
        },
    ];
    let go_mihomo_retirement_audit_complete = checks.iter().all(|check| check.passed);
    let blockers = checks
        .iter()
        .flat_map(|check| check.blockers.clone())
        .collect::<Vec<String>>();

    Ok(KernelLoopbackGoMihomoRetirementAuditReport {
        runtime_id: RUST_RUNTIME_ID.into(),
        component: "go-mihomo-retirement-audit".into(),
        mutates_runtime: false,
        live_execution_allowed: go_mihomo_retirement_audit_complete,
        full_rust_runtime_hardened,
        surface_audit,
        final_retirement_audit_decision,
        go_mihomo_retirement_audit_complete,
        selected_runtime_kind: if go_mihomo_retirement_audit_complete {
            KernelRuntimeKind::Rust
        } else {
            KernelRuntimeKind::Mihomo
        },
        rollback_runtime_kind: KernelRuntimeKind::Mihomo,
        checks,
        blockers,
        warnings: vec![
            "this audit does not delete Mihomo source, binaries, IPC commands, or rollback paths".into(),
            "emergency rollback must stay retained until a dedicated retirement plan passes".into(),
        ],
        facts: vec![
            "Go/Mihomo retirement audit is the first post-hardening inventory gate".into(),
            "successful audit advances to a separate retirement plan rather than direct removal".into(),
        ],
        next_safe_batch: if go_mihomo_retirement_audit_complete {
            "go-mihomo-retirement-plan".into()
        } else {
            "go-mihomo-retirement-audit".into()
        },
    })
}
