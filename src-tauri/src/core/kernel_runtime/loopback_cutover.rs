use super::*;

pub async fn mihomo_kernel_loopback_r4_expanded_opt_in_preflight(
    listener_port: Option<u16>,
    target_port: Option<u16>,
    hold_started_at_epoch_ms: Option<u64>,
    observed_rollback_platforms: Option<Vec<String>>,
    explicit_decision: Option<bool>,
) -> Result<KernelLoopbackR4ExpandedOptInPreflightReport> {
    let listener_port = listener_port.unwrap_or(DEFAULT_LOOPBACK_FORWARDING_LISTENER_PORT);
    let target_port = target_port.unwrap_or(DEFAULT_LOOPBACK_FORWARDING_TARGET_PORT);
    let explicit_decision = explicit_decision.unwrap_or(false);
    let hold_window =
        mihomo_kernel_loopback_hold_window(Some(listener_port), Some(target_port), hold_started_at_epoch_ms).await?;
    let required_platforms = LOOPBACK_PLATFORM_MATRIX_PLATFORMS
        .iter()
        .map(|platform| (*platform).into())
        .collect::<Vec<String>>();
    let observed_rollback_platforms = observed_rollback_platforms
        .unwrap_or_default()
        .into_iter()
        .filter(|platform| LOOPBACK_PLATFORM_MATRIX_PLATFORMS.contains(&platform.as_str()))
        .collect::<BTreeSet<String>>();
    let pending_rollback_platforms = LOOPBACK_PLATFORM_MATRIX_PLATFORMS
        .iter()
        .filter(|platform| !observed_rollback_platforms.contains(**platform))
        .map(|platform| (*platform).into())
        .collect::<Vec<String>>();
    let observed_rollback_platforms = observed_rollback_platforms.into_iter().collect::<Vec<String>>();

    let rows = LOOPBACK_PLATFORM_MATRIX_PLATFORMS
        .iter()
        .map(|platform| {
            let current_platform = *platform == hold_window.current_platform;
            let rollback_drill_observed = observed_rollback_platforms.iter().any(|observed| observed == platform);
            let hold_window_satisfied = current_platform.then_some(hold_window.current_platform_hold_window_satisfied);
            let mut blockers = Vec::new();
            if !rollback_drill_observed {
                blockers.push(format!("missing observed rollback drill evidence for {platform}").into());
            }
            if current_platform && !hold_window.current_platform_hold_window_satisfied {
                blockers.push("current platform hold-window evidence is not satisfied".into());
            }

            KernelLoopbackR4ExpandedOptInPreflightRow {
                platform: (*platform).into(),
                current_platform,
                rollback_drill_observed,
                hold_window_satisfied,
                evidence_status: if blockers.is_empty() {
                    "ready".into()
                } else {
                    "blocked".into()
                },
                blockers,
                facts: vec![
                    "R4 preflight consumes platform rollback evidence without re-running rollback drills".into(),
                ],
            }
        })
        .collect::<Vec<KernelLoopbackR4ExpandedOptInPreflightRow>>();

    let mut checks = Vec::new();
    let matrix_passed = hold_window.platform_matrix.current_platform_passed;
    checks.push(KernelLoopbackR4ExpandedOptInPreflightCheck {
        name: "currentPlatformMatrix".into(),
        status: if matrix_passed { "passed" } else { "blocked" }.into(),
        passed: matrix_passed,
        blockers: if matrix_passed {
            Vec::new()
        } else {
            vec!["current platform matrix evidence is not passing".into()]
        },
        facts: vec!["preflight reuses read-only platform matrix evidence".into()],
    });
    let hold_passed = hold_window.current_platform_hold_window_satisfied;
    checks.push(KernelLoopbackR4ExpandedOptInPreflightCheck {
        name: "currentPlatformHoldWindow".into(),
        status: if hold_passed { "passed" } else { "blocked" }.into(),
        passed: hold_passed,
        blockers: if hold_passed {
            Vec::new()
        } else {
            vec!["current platform hold window is not satisfied".into()]
        },
        facts: vec!["hold-window evidence is read-only and session-scoped".into()],
    });
    let rollback_passed = pending_rollback_platforms.is_empty();
    checks.push(KernelLoopbackR4ExpandedOptInPreflightCheck {
        name: "allPlatformRollbackDrills".into(),
        status: if rollback_passed { "passed" } else { "blocked" }.into(),
        passed: rollback_passed,
        blockers: if rollback_passed {
            Vec::new()
        } else {
            vec![
                format!(
                    "pending rollback drill platform evidence: {}",
                    pending_rollback_platforms.join(", ")
                )
                .into(),
            ]
        },
        facts: vec!["rollback drill observations must cover Windows, macOS, and Linux".into()],
    });
    checks.push(KernelLoopbackR4ExpandedOptInPreflightCheck {
        name: "explicitDecision".into(),
        status: if explicit_decision { "passed" } else { "blocked" }.into(),
        passed: explicit_decision,
        blockers: if explicit_decision {
            Vec::new()
        } else {
            vec!["R4 expanded opt-in requires an explicit decision".into()]
        },
        facts: vec!["readiness evidence alone is not rollout permission".into()],
    });

    let mut blockers = checks
        .iter()
        .flat_map(|check| check.blockers.clone())
        .collect::<Vec<String>>();
    blockers.push("dedicated expanded opt-in execution is not implemented in this preflight batch".into());
    let preflight_passed = checks.iter().all(|check| check.passed);

    Ok(KernelLoopbackR4ExpandedOptInPreflightReport {
        runtime_id: MIHOMO_RUNTIME_ID.into(),
        component: "loopback-r4-expanded-opt-in-preflight".into(),
        kernel_area: "forwarding".into(),
        mutates_runtime: false,
        live_execution_allowed: false,
        current_platform: hold_window.current_platform.clone(),
        current_arch: hold_window.current_arch.clone(),
        listener_port,
        target_port,
        explicit_decision,
        required_platforms,
        observed_rollback_platforms,
        pending_rollback_platforms,
        current_platform_hold_window_satisfied: hold_window.current_platform_hold_window_satisfied,
        preflight_passed,
        expanded_opt_in_allowed: false,
        hold_window,
        rows,
        checks,
        default_route: false,
        forwards_traffic: false,
        outbound_adapters_used: false,
        mihomo_fallback: true,
        passed: false,
        blockers,
        warnings: vec![
            "R4 expanded opt-in preflight is read-only and does not enable real adapter/TUN/protocol/default cutover"
                .into(),
        ],
        facts: vec![
            "checks platform evidence readiness without running rollback drills".into(),
            "requires explicit decision separate from accumulated evidence".into(),
            "keeps expanded opt-in execution blocked for a dedicated later batch".into(),
        ],
        next_safe_batch: "loopback-r4-expanded-opt-in-execution-plan".into(),
    })
}

pub async fn mihomo_kernel_loopback_r4_expanded_opt_in_execution_plan(
    listener_port: Option<u16>,
    target_port: Option<u16>,
    hold_started_at_epoch_ms: Option<u64>,
    observed_rollback_platforms: Option<Vec<String>>,
    explicit_decision: Option<bool>,
) -> Result<KernelLoopbackR4ExpandedOptInExecutionPlanReport> {
    let preflight = mihomo_kernel_loopback_r4_expanded_opt_in_preflight(
        listener_port,
        target_port,
        hold_started_at_epoch_ms,
        observed_rollback_platforms,
        explicit_decision,
    )
    .await?;
    let explicit_decision = preflight.explicit_decision;
    let plan_ready = preflight.preflight_passed;

    let steps = vec![
        KernelLoopbackR4ExpandedOptInExecutionPlanStep {
            order: 1,
            name: "revalidateReadOnlyPreflight".into(),
            action: "call get_runtime_kernel_loopback_r4_expanded_opt_in_preflight before any execution attempt".into(),
            mutates_runtime: false,
            requires_explicit_decision: false,
            enabled_in_this_batch: true,
            blockers: Vec::new(),
            facts: vec!["preflight must stay fresh and read-only".into()],
        },
        KernelLoopbackR4ExpandedOptInExecutionPlanStep {
            order: 2,
            name: "requireExplicitExpandedOptInDecision".into(),
            action: "require a separate user decision scoped to R4 expanded opt-in".into(),
            mutates_runtime: false,
            requires_explicit_decision: true,
            enabled_in_this_batch: true,
            blockers: if explicit_decision {
                Vec::new()
            } else {
                vec!["explicit R4 decision is missing".into()]
            },
            facts: vec!["evidence readiness is not rollout permission".into()],
        },
        KernelLoopbackR4ExpandedOptInExecutionPlanStep {
            order: 3,
            name: "executeLoopbackOnlyExpandedRuntime".into(),
            action: "future batch may run only bounded loopback synthetic forwarding with rollback state capture"
                .into(),
            mutates_runtime: true,
            requires_explicit_decision: true,
            enabled_in_this_batch: false,
            blockers: vec!["execution guard is not implemented in this planning batch".into()],
            facts: vec!["real adapters, TUN, protocol handlers, and default route remain out of scope".into()],
        },
        KernelLoopbackR4ExpandedOptInExecutionPlanStep {
            order: 4,
            name: "verifyAndRollback".into(),
            action: "future batch must verify no leaked sockets or config drift and provide explicit rollback".into(),
            mutates_runtime: true,
            requires_explicit_decision: true,
            enabled_in_this_batch: false,
            blockers: vec!["verification and rollback execution are reserved for a dedicated batch".into()],
            facts: vec!["default cutover cannot be part of R4 expanded opt-in execution".into()],
        },
    ];

    let mut blockers = preflight.blockers.clone();
    blockers.push("execution plan is descriptive only; expanded opt-in execution remains blocked".into());

    Ok(KernelLoopbackR4ExpandedOptInExecutionPlanReport {
        runtime_id: MIHOMO_RUNTIME_ID.into(),
        component: "loopback-r4-expanded-opt-in-execution-plan".into(),
        kernel_area: "forwarding".into(),
        mutates_runtime: false,
        live_execution_allowed: false,
        current_platform: preflight.current_platform.clone(),
        current_arch: preflight.current_arch.clone(),
        listener_port: preflight.listener_port,
        target_port: preflight.target_port,
        candidate_scope: "loopbackSyntheticOnly".into(),
        explicit_decision,
        plan_ready,
        execution_allowed: false,
        expanded_opt_in_allowed: false,
        preflight,
        steps,
        default_route: false,
        forwards_traffic: false,
        outbound_adapters_used: false,
        mihomo_fallback: true,
        passed: false,
        blockers,
        warnings: vec![
            "execution plan is read-only documentation in data form and does not authorize runtime mutation".into(),
        ],
        facts: vec![
            "keeps R4 execution split from readiness preflight".into(),
            "limits any future execution candidate to synthetic loopback scope".into(),
            "keeps default cutover blocked for a later dedicated phase".into(),
        ],
        next_safe_batch: "loopback-r4-expanded-opt-in-execution-guard".into(),
    })
}

pub async fn mihomo_kernel_loopback_r4_expanded_opt_in_execution_guard(
    listener_port: Option<u16>,
    target_port: Option<u16>,
    hold_started_at_epoch_ms: Option<u64>,
    observed_rollback_platforms: Option<Vec<String>>,
    explicit_decision: Option<bool>,
    requested_execution: Option<bool>,
) -> Result<KernelLoopbackR4ExpandedOptInExecutionGuardReport> {
    let requested_execution = requested_execution.unwrap_or(false);
    let plan = mihomo_kernel_loopback_r4_expanded_opt_in_execution_plan(
        listener_port,
        target_port,
        hold_started_at_epoch_ms,
        observed_rollback_platforms,
        explicit_decision,
    )
    .await?;
    let explicit_decision = plan.explicit_decision;
    let plan_ready = plan.plan_ready;

    let guard_checks = vec![
        KernelLoopbackR4ExpandedOptInExecutionGuardCheck {
            name: "executionRequested".into(),
            status: if requested_execution { "passed" } else { "blocked" }.into(),
            passed: requested_execution,
            required_for_execution: true,
            blockers: if requested_execution {
                Vec::new()
            } else {
                vec!["guard requires an explicit execution request separate from evidence collection".into()]
            },
            facts: vec!["preflight and planning commands do not imply execution intent".into()],
        },
        KernelLoopbackR4ExpandedOptInExecutionGuardCheck {
            name: "executionPlanReady".into(),
            status: if plan_ready { "passed" } else { "blocked" }.into(),
            passed: plan_ready,
            required_for_execution: true,
            blockers: if plan_ready {
                Vec::new()
            } else {
                vec!["execution plan is not ready because one or more preflight gates are blocked".into()]
            },
            facts: vec!["guard consumes the read-only R4 execution plan".into()],
        },
        KernelLoopbackR4ExpandedOptInExecutionGuardCheck {
            name: "explicitDecision".into(),
            status: if explicit_decision { "passed" } else { "blocked" }.into(),
            passed: explicit_decision,
            required_for_execution: true,
            blockers: if explicit_decision {
                Vec::new()
            } else {
                vec!["explicit R4 expanded opt-in decision is missing".into()]
            },
            facts: vec!["execution intent must be distinct from roadmap progress".into()],
        },
        KernelLoopbackR4ExpandedOptInExecutionGuardCheck {
            name: "implementationBoundary".into(),
            status: "passed".into(),
            passed: true,
            required_for_execution: true,
            blockers: Vec::new(),
            facts: vec!["synthetic loopback execution is implemented behind this guard".into()],
        },
    ];

    let verification_plan = vec![
        KernelLoopbackR4ExpandedOptInSafetyPlanStep {
            order: 1,
            phase: "preExecution".into(),
            action: "capture runtime config, system proxy, TUN, and loopback port state".into(),
            mutates_runtime: false,
            required_before_expansion: true,
            enabled_in_this_batch: true,
            blockers: Vec::new(),
            facts: vec!["verification must compare the same state after execution".into()],
        },
        KernelLoopbackR4ExpandedOptInSafetyPlanStep {
            order: 2,
            phase: "postExecution".into(),
            action: "verify synthetic listener and target ports are released and no isolated listener remains running"
                .into(),
            mutates_runtime: false,
            required_before_expansion: true,
            enabled_in_this_batch: true,
            blockers: Vec::new(),
            facts: vec!["port release remains the primary loopback leak signal".into()],
        },
        KernelLoopbackR4ExpandedOptInSafetyPlanStep {
            order: 3,
            phase: "postExecution".into(),
            action: "verify system proxy, TUN, runtime config, and Mihomo fallback boundaries are unchanged".into(),
            mutates_runtime: false,
            required_before_expansion: true,
            enabled_in_this_batch: true,
            blockers: Vec::new(),
            facts: vec!["R4 loopback expansion must not become default cutover".into()],
        },
    ];
    let rollback_plan = vec![
        KernelLoopbackR4ExpandedOptInSafetyPlanStep {
            order: 1,
            phase: "rollback".into(),
            action: "stop any app-owned loopback listener and release synthetic target sockets".into(),
            mutates_runtime: true,
            required_before_expansion: true,
            enabled_in_this_batch: false,
            blockers: vec!["rollback execution is reserved for the synthetic execution batch".into()],
            facts: vec!["rollback must not call Mihomo adapter or TUN mutation paths".into()],
        },
        KernelLoopbackR4ExpandedOptInSafetyPlanStep {
            order: 2,
            phase: "rollback".into(),
            action: "restore captured runtime config if a future synthetic execution changes it".into(),
            mutates_runtime: true,
            required_before_expansion: true,
            enabled_in_this_batch: false,
            blockers: vec!["runtime restore is not needed until execution is implemented".into()],
            facts: vec!["the current guard command does not mutate runtime state".into()],
        },
    ];

    let guard_ready = guard_checks.iter().all(|check| check.passed);
    let synthetic_execution_allowed = guard_ready;
    let blockers = guard_checks
        .iter()
        .flat_map(|check| check.blockers.clone())
        .collect::<Vec<String>>();

    Ok(KernelLoopbackR4ExpandedOptInExecutionGuardReport {
        runtime_id: MIHOMO_RUNTIME_ID.into(),
        component: "loopback-r4-expanded-opt-in-execution-guard".into(),
        kernel_area: "forwarding".into(),
        mutates_runtime: false,
        live_execution_allowed: false,
        current_platform: plan.current_platform.clone(),
        current_arch: plan.current_arch.clone(),
        listener_port: plan.listener_port,
        target_port: plan.target_port,
        requested_execution,
        explicit_decision,
        guard_ready,
        synthetic_execution_allowed,
        execution_allowed: false,
        expanded_opt_in_allowed: false,
        plan,
        guard_checks,
        verification_plan,
        rollback_plan,
        default_route: false,
        forwards_traffic: false,
        outbound_adapters_used: false,
        mihomo_fallback: true,
        passed: guard_ready,
        blockers,
        warnings: vec![
            "execution guard is read-only and does not start expanded opt-in execution".into(),
            "synthetic execution permission is not default cutover permission".into(),
        ],
        facts: vec![
            "bundles execution guard checks with verification and rollback plans".into(),
            "keeps future execution constrained to synthetic loopback scope".into(),
            "keeps default cutover, real adapters, TUN, and protocol handlers blocked".into(),
        ],
        next_safe_batch: "loopback-r4-expanded-opt-in-synthetic-execution".into(),
    })
}

fn build_blocked_r4_synthetic_execution_closeout(
    blockers: Vec<String>,
) -> KernelLoopbackR4ExpandedOptInSyntheticExecutionCloseout {
    KernelLoopbackR4ExpandedOptInSyntheticExecutionCloseout {
        rollback_drill_passed: false,
        leak_check_passed: false,
        ports_released: false,
        system_proxy_unchanged: false,
        tun_unchanged: false,
        runtime_config_unchanged: false,
        isolated_test_listener_stopped: false,
        default_route: false,
        forwards_traffic: false,
        outbound_adapters_used: false,
        mihomo_fallback: true,
        passed: false,
        blockers,
        warnings: vec!["synthetic execution was not attempted because guard checks blocked it".into()],
        facts: vec!["blocked closeout records no runtime mutation evidence".into()],
    }
}

pub async fn mihomo_kernel_loopback_r4_expanded_opt_in_synthetic_execution(
    listener_port: Option<u16>,
    target_port: Option<u16>,
    hold_started_at_epoch_ms: Option<u64>,
    observed_rollback_platforms: Option<Vec<String>>,
    explicit_decision: Option<bool>,
    requested_execution: Option<bool>,
) -> Result<KernelLoopbackR4ExpandedOptInSyntheticExecutionReport> {
    let requested_execution = requested_execution.unwrap_or(false);
    let guard = mihomo_kernel_loopback_r4_expanded_opt_in_execution_guard(
        listener_port,
        target_port,
        hold_started_at_epoch_ms,
        observed_rollback_platforms,
        explicit_decision,
        Some(requested_execution),
    )
    .await?;
    let synthetic_execution_allowed = guard.synthetic_execution_allowed && requested_execution;
    let listener_port = guard.listener_port;
    let target_port = guard.target_port;

    if !synthetic_execution_allowed {
        let blockers = guard.blockers.clone();
        return Ok(KernelLoopbackR4ExpandedOptInSyntheticExecutionReport {
            runtime_id: MIHOMO_RUNTIME_ID.into(),
            component: "loopback-r4-expanded-opt-in-synthetic-execution".into(),
            kernel_area: "forwarding".into(),
            mutates_runtime: false,
            live_execution_allowed: false,
            current_platform: guard.current_platform.clone(),
            current_arch: guard.current_arch.clone(),
            listener_port,
            target_port,
            requested_execution,
            explicit_decision: guard.explicit_decision,
            synthetic_execution_allowed,
            execution_attempted: false,
            expanded_opt_in_allowed: false,
            closeout: build_blocked_r4_synthetic_execution_closeout(blockers.clone()),
            guard,
            rollback_drill: None,
            leak_check: None,
            default_route: false,
            forwards_traffic: false,
            outbound_adapters_used: false,
            mihomo_fallback: true,
            passed: false,
            blockers,
            warnings: vec!["R4 synthetic execution remains blocked until guard checks pass".into()],
            facts: vec!["no sockets were opened because execution was not allowed".into()],
            next_safe_batch: "loopback-r4-expanded-opt-in-synthetic-execution".into(),
        });
    }

    let rollback_drill =
        mihomo_kernel_loopback_forwarding_rollback_drill(Some(listener_port), Some(target_port)).await?;
    let leak_check = mihomo_kernel_loopback_forwarding_leak_check(Some(listener_port), Some(target_port)).await?;
    let ports_released =
        rollback_drill.ports_released && leak_check.listener_port_released && leak_check.target_port_released;
    let isolated_test_listener_stopped = !leak_check.isolated_test_listener_running;

    let mut closeout_blockers = Vec::new();
    if !rollback_drill.passed {
        closeout_blockers.extend(rollback_drill.blockers.clone());
    }
    if !leak_check.passed {
        closeout_blockers.extend(leak_check.blockers.clone());
    }
    if !ports_released {
        closeout_blockers.push("synthetic execution ports were not released after closeout".into());
    }
    if !isolated_test_listener_stopped {
        closeout_blockers.push("isolated test listener remained running after synthetic execution".into());
    }

    let closeout_passed = closeout_blockers.is_empty();
    let closeout = KernelLoopbackR4ExpandedOptInSyntheticExecutionCloseout {
        rollback_drill_passed: rollback_drill.passed,
        leak_check_passed: leak_check.passed,
        ports_released,
        system_proxy_unchanged: rollback_drill.system_proxy_unchanged,
        tun_unchanged: rollback_drill.tun_unchanged,
        runtime_config_unchanged: rollback_drill.runtime_config_unchanged,
        isolated_test_listener_stopped,
        default_route: false,
        forwards_traffic: false,
        outbound_adapters_used: false,
        mihomo_fallback: true,
        passed: closeout_passed,
        blockers: closeout_blockers.clone(),
        warnings: vec!["closeout proves only synthetic loopback execution cleanup".into()],
        facts: vec![
            "synthetic execution delegates to the loopback forwarding rollback drill".into(),
            "leak check revalidates listener, target, and isolated listener state after execution".into(),
        ],
    };

    Ok(KernelLoopbackR4ExpandedOptInSyntheticExecutionReport {
        runtime_id: MIHOMO_RUNTIME_ID.into(),
        component: "loopback-r4-expanded-opt-in-synthetic-execution".into(),
        kernel_area: "forwarding".into(),
        mutates_runtime: true,
        live_execution_allowed: true,
        current_platform: guard.current_platform.clone(),
        current_arch: guard.current_arch.clone(),
        listener_port,
        target_port,
        requested_execution,
        explicit_decision: guard.explicit_decision,
        synthetic_execution_allowed,
        execution_attempted: true,
        expanded_opt_in_allowed: false,
        guard,
        rollback_drill: Some(rollback_drill),
        leak_check: Some(leak_check),
        closeout,
        default_route: false,
        forwards_traffic: false,
        outbound_adapters_used: false,
        mihomo_fallback: true,
        passed: closeout_passed,
        blockers: closeout_blockers,
        warnings: vec![
            "synthetic execution uses loopback-only rollback drill evidence and is not production forwarding".into(),
            "expanded opt-in remains blocked for real adapters, TUN, protocol handlers, and default cutover".into(),
        ],
        facts: vec![
            "executes only temporary 127.0.0.1 listener and target sockets".into(),
            "runs closeout leak evidence immediately after synthetic execution".into(),
        ],
        next_safe_batch: "loopback-r4-expanded-opt-in-post-execution-hold".into(),
    })
}

pub async fn mihomo_kernel_loopback_r4_expanded_opt_in_post_execution_hold(
    listener_port: Option<u16>,
    target_port: Option<u16>,
    hold_started_at_epoch_ms: Option<u64>,
    observed_rollback_platforms: Option<Vec<String>>,
    explicit_decision: Option<bool>,
    requested_execution: Option<bool>,
    post_execution_hold_started_at_epoch_ms: Option<u64>,
) -> Result<KernelLoopbackR4ExpandedOptInPostExecutionHoldReport> {
    let requested_execution = requested_execution.unwrap_or(false);
    let synthetic_execution = mihomo_kernel_loopback_r4_expanded_opt_in_synthetic_execution(
        listener_port,
        target_port,
        hold_started_at_epoch_ms,
        observed_rollback_platforms,
        explicit_decision,
        Some(requested_execution),
    )
    .await?;
    let observed_at_epoch_ms = current_epoch_ms();
    let post_execution_hold_started_at_epoch_ms =
        post_execution_hold_started_at_epoch_ms.unwrap_or(observed_at_epoch_ms);
    let hold_start_in_future = post_execution_hold_started_at_epoch_ms > observed_at_epoch_ms;
    let elapsed_hold_seconds = observed_at_epoch_ms
        .saturating_sub(post_execution_hold_started_at_epoch_ms)
        .saturating_div(1000);
    let post_execution_hold_satisfied = !hold_start_in_future
        && synthetic_execution.passed
        && synthetic_execution.execution_attempted
        && elapsed_hold_seconds >= LOOPBACK_HOLD_WINDOW_MIN_SECONDS;

    let mut blockers = Vec::new();
    if hold_start_in_future {
        blockers.push("post-execution hold start timestamp is later than observation time".into());
    }
    if !synthetic_execution.execution_attempted {
        blockers.push("synthetic execution was not attempted before post-execution hold".into());
    }
    if !synthetic_execution.passed {
        blockers.extend(synthetic_execution.blockers.clone());
    }
    if elapsed_hold_seconds < LOOPBACK_HOLD_WINDOW_MIN_SECONDS {
        blockers.push(
            format!("observe at least {LOOPBACK_HOLD_WINDOW_MIN_SECONDS} second(s) after synthetic execution closeout")
                .into(),
        );
    }

    Ok(KernelLoopbackR4ExpandedOptInPostExecutionHoldReport {
        runtime_id: MIHOMO_RUNTIME_ID.into(),
        component: "loopback-r4-expanded-opt-in-post-execution-hold".into(),
        kernel_area: "forwarding".into(),
        mutates_runtime: synthetic_execution.execution_attempted,
        live_execution_allowed: synthetic_execution.synthetic_execution_allowed,
        current_platform: synthetic_execution.current_platform.clone(),
        current_arch: synthetic_execution.current_arch.clone(),
        listener_port: synthetic_execution.listener_port,
        target_port: synthetic_execution.target_port,
        requested_execution,
        explicit_decision: synthetic_execution.explicit_decision,
        post_execution_hold_started_at_epoch_ms,
        observed_at_epoch_ms,
        minimum_hold_seconds: LOOPBACK_HOLD_WINDOW_MIN_SECONDS,
        elapsed_hold_seconds,
        post_execution_hold_satisfied,
        execution_attempted: synthetic_execution.execution_attempted,
        synthetic_execution_passed: synthetic_execution.passed,
        closeout_passed: synthetic_execution.closeout.passed,
        expanded_opt_in_allowed: false,
        synthetic_execution,
        default_route: false,
        forwards_traffic: false,
        outbound_adapters_used: false,
        mihomo_fallback: true,
        passed: post_execution_hold_satisfied,
        blockers,
        warnings: vec![
            "post-execution hold observes only synthetic loopback closeout evidence".into(),
            "wider opt-in remains blocked until a separate decision-readiness gate".into(),
        ],
        facts: vec![
            "post-execution hold is independent from the preflight hold window".into(),
            "hold evidence does not authorize real adapters, TUN, protocol handlers, or default cutover".into(),
        ],
        next_safe_batch: "loopback-r4-expanded-opt-in-decision-readiness".into(),
    })
}

pub async fn mihomo_kernel_loopback_r4_expanded_opt_in_decision_readiness(
    listener_port: Option<u16>,
    target_port: Option<u16>,
    hold_started_at_epoch_ms: Option<u64>,
    observed_rollback_platforms: Option<Vec<String>>,
    explicit_decision: Option<bool>,
    requested_execution: Option<bool>,
    post_execution_hold_started_at_epoch_ms: Option<u64>,
    wider_opt_in_decision: Option<bool>,
) -> Result<KernelLoopbackR4ExpandedOptInDecisionReadinessReport> {
    let wider_opt_in_decision = wider_opt_in_decision.unwrap_or(false);
    let requested_execution = requested_execution.unwrap_or(false);
    let post_execution_hold = mihomo_kernel_loopback_r4_expanded_opt_in_post_execution_hold(
        listener_port,
        target_port,
        hold_started_at_epoch_ms,
        observed_rollback_platforms,
        explicit_decision,
        Some(requested_execution),
        post_execution_hold_started_at_epoch_ms,
    )
    .await?;

    let checks = vec![
        KernelLoopbackR4ExpandedOptInDecisionReadinessCheck {
            name: "postExecutionHold".into(),
            status: if post_execution_hold.post_execution_hold_satisfied {
                "passed"
            } else {
                "blocked"
            }
            .into(),
            passed: post_execution_hold.post_execution_hold_satisfied,
            blockers: if post_execution_hold.post_execution_hold_satisfied {
                Vec::new()
            } else {
                post_execution_hold.blockers.clone()
            },
            facts: vec!["synthetic execution closeout must remain stable through the hold window".into()],
        },
        KernelLoopbackR4ExpandedOptInDecisionReadinessCheck {
            name: "widerOptInDecision".into(),
            status: if wider_opt_in_decision { "passed" } else { "blocked" }.into(),
            passed: wider_opt_in_decision,
            blockers: if wider_opt_in_decision {
                Vec::new()
            } else {
                vec!["wider R4 opt-in requires an explicit decision after post-execution hold".into()]
            },
            facts: vec!["synthetic success alone is not wider opt-in permission".into()],
        },
        KernelLoopbackR4ExpandedOptInDecisionReadinessCheck {
            name: "defaultCutoverBoundary".into(),
            status: "passed".into(),
            passed: true,
            blockers: Vec::new(),
            facts: vec!["decision readiness can only target bounded loopback-expanded opt-in".into()],
        },
    ];
    let decision_ready = checks.iter().all(|check| check.passed);
    let blockers = checks
        .iter()
        .flat_map(|check| check.blockers.clone())
        .collect::<Vec<String>>();

    Ok(KernelLoopbackR4ExpandedOptInDecisionReadinessReport {
        runtime_id: MIHOMO_RUNTIME_ID.into(),
        component: "loopback-r4-expanded-opt-in-decision-readiness".into(),
        kernel_area: "forwarding".into(),
        mutates_runtime: post_execution_hold.mutates_runtime,
        live_execution_allowed: post_execution_hold.live_execution_allowed,
        current_platform: post_execution_hold.current_platform.clone(),
        current_arch: post_execution_hold.current_arch.clone(),
        listener_port: post_execution_hold.listener_port,
        target_port: post_execution_hold.target_port,
        requested_execution,
        explicit_decision: post_execution_hold.explicit_decision,
        wider_opt_in_decision,
        decision_ready,
        wider_opt_in_allowed: false,
        expanded_opt_in_allowed: false,
        post_execution_hold,
        checks,
        default_route: false,
        forwards_traffic: false,
        outbound_adapters_used: false,
        mihomo_fallback: true,
        passed: decision_ready,
        blockers,
        warnings: vec!["decision readiness is still not default cutover or production forwarding permission".into()],
        facts: vec![
            "bundles post-execution hold and explicit wider-decision readiness".into(),
            "keeps real adapter/TUN/protocol/default route replacement blocked".into(),
        ],
        next_safe_batch: "loopback-r4-expanded-opt-in-limited-rollout-gate".into(),
    })
}

pub async fn mihomo_kernel_loopback_r4_expanded_opt_in_limited_rollout_gate(
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
) -> Result<KernelLoopbackR4ExpandedOptInLimitedRolloutGateReport> {
    let limited_rollout_decision = limited_rollout_decision.unwrap_or(false);
    let canary_scope = canary_scope.unwrap_or_else(|| "loopbackSyntheticCanary".into());
    let max_canary_sessions = max_canary_sessions.unwrap_or(1);
    let requested_execution = requested_execution.unwrap_or(false);
    let decision_readiness = mihomo_kernel_loopback_r4_expanded_opt_in_decision_readiness(
        listener_port,
        target_port,
        hold_started_at_epoch_ms,
        observed_rollback_platforms,
        explicit_decision,
        Some(requested_execution),
        post_execution_hold_started_at_epoch_ms,
        wider_opt_in_decision,
    )
    .await?;

    let canary_scope_passed = canary_scope == "loopbackSyntheticCanary";
    let session_limit_passed = (1..=3).contains(&max_canary_sessions);
    let checks = vec![
        KernelLoopbackR4ExpandedOptInLimitedRolloutGateCheck {
            name: "decisionReadiness".into(),
            status: if decision_readiness.decision_ready {
                "passed"
            } else {
                "blocked"
            }
            .into(),
            passed: decision_readiness.decision_ready,
            blockers: if decision_readiness.decision_ready {
                Vec::new()
            } else {
                decision_readiness.blockers.clone()
            },
            facts: vec!["limited rollout gate consumes post-execution hold plus wider-decision readiness".into()],
        },
        KernelLoopbackR4ExpandedOptInLimitedRolloutGateCheck {
            name: "limitedRolloutDecision".into(),
            status: if limited_rollout_decision { "passed" } else { "blocked" }.into(),
            passed: limited_rollout_decision,
            blockers: if limited_rollout_decision {
                Vec::new()
            } else {
                vec!["limited rollout requires a separate explicit decision".into()]
            },
            facts: vec!["limited rollout decision is distinct from wider opt-in readiness".into()],
        },
        KernelLoopbackR4ExpandedOptInLimitedRolloutGateCheck {
            name: "canaryScope".into(),
            status: if canary_scope_passed { "passed" } else { "blocked" }.into(),
            passed: canary_scope_passed,
            blockers: if canary_scope_passed {
                Vec::new()
            } else {
                vec!["canary scope must remain loopbackSyntheticCanary".into()]
            },
            facts: vec!["canary scope excludes real adapters, TUN, and default route".into()],
        },
        KernelLoopbackR4ExpandedOptInLimitedRolloutGateCheck {
            name: "canarySessionLimit".into(),
            status: if session_limit_passed { "passed" } else { "blocked" }.into(),
            passed: session_limit_passed,
            blockers: if session_limit_passed {
                Vec::new()
            } else {
                vec!["limited rollout canary session cap must be between 1 and 3".into()]
            },
            facts: vec!["session cap keeps rollout bounded and reversible".into()],
        },
    ];
    let gate_ready = checks.iter().all(|check| check.passed);
    let blockers = checks
        .iter()
        .flat_map(|check| check.blockers.clone())
        .collect::<Vec<String>>();

    Ok(KernelLoopbackR4ExpandedOptInLimitedRolloutGateReport {
        runtime_id: MIHOMO_RUNTIME_ID.into(),
        component: "loopback-r4-expanded-opt-in-limited-rollout-gate".into(),
        kernel_area: "forwarding".into(),
        mutates_runtime: decision_readiness.mutates_runtime,
        live_execution_allowed: decision_readiness.live_execution_allowed,
        current_platform: decision_readiness.current_platform.clone(),
        current_arch: decision_readiness.current_arch.clone(),
        listener_port: decision_readiness.listener_port,
        target_port: decision_readiness.target_port,
        requested_execution,
        explicit_decision: decision_readiness.explicit_decision,
        wider_opt_in_decision: decision_readiness.wider_opt_in_decision,
        limited_rollout_decision,
        canary_scope,
        max_canary_sessions,
        gate_ready,
        limited_rollout_allowed: false,
        expanded_opt_in_allowed: false,
        decision_readiness,
        checks,
        default_route: false,
        forwards_traffic: false,
        outbound_adapters_used: false,
        mihomo_fallback: true,
        passed: gate_ready,
        blockers,
        warnings: vec!["limited rollout gate is readiness evidence only and does not start rollout".into()],
        facts: vec![
            "permits only bounded loopback-synthetic canary readiness".into(),
            "keeps real adapter/TUN/protocol/default-route cutover outside R4".into(),
        ],
        next_safe_batch: "loopback-r4-expanded-opt-in-rollout-audit".into(),
    })
}

pub async fn mihomo_kernel_loopback_r4_expanded_opt_in_rollout_audit(
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
) -> Result<KernelLoopbackR4ExpandedOptInRolloutAuditReport> {
    let gate = mihomo_kernel_loopback_r4_expanded_opt_in_limited_rollout_gate(
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
    )
    .await?;
    let rows = vec![
        KernelLoopbackR4ExpandedOptInRolloutAuditRow {
            name: "gateReady".into(),
            status: if gate.gate_ready { "passed" } else { "blocked" }.into(),
            passed: gate.gate_ready,
            blockers: if gate.gate_ready {
                Vec::new()
            } else {
                gate.blockers.clone()
            },
            facts: vec!["audit records the limited rollout gate result".into()],
        },
        KernelLoopbackR4ExpandedOptInRolloutAuditRow {
            name: "rollbackBinding".into(),
            status: "passed".into(),
            passed: true,
            blockers: Vec::new(),
            facts: vec!["rollback remains bound to synthetic closeout and loopback leak evidence".into()],
        },
        KernelLoopbackR4ExpandedOptInRolloutAuditRow {
            name: "defaultCutoverBoundary".into(),
            status: "passed".into(),
            passed: true,
            blockers: Vec::new(),
            facts: vec!["audit scope excludes default route, system proxy, TUN, and real adapters".into()],
        },
    ];
    let audit_ready = rows.iter().all(|row| row.passed);
    let blockers = rows
        .iter()
        .flat_map(|row| row.blockers.clone())
        .collect::<Vec<String>>();

    Ok(KernelLoopbackR4ExpandedOptInRolloutAuditReport {
        runtime_id: MIHOMO_RUNTIME_ID.into(),
        component: "loopback-r4-expanded-opt-in-rollout-audit".into(),
        kernel_area: "forwarding".into(),
        mutates_runtime: gate.mutates_runtime,
        live_execution_allowed: gate.live_execution_allowed,
        current_platform: gate.current_platform.clone(),
        current_arch: gate.current_arch.clone(),
        canary_scope: gate.canary_scope.clone(),
        max_canary_sessions: gate.max_canary_sessions,
        audit_ready,
        limited_rollout_allowed: false,
        expanded_opt_in_allowed: false,
        gate,
        rows,
        default_route: false,
        forwards_traffic: false,
        outbound_adapters_used: false,
        mihomo_fallback: true,
        passed: audit_ready,
        blockers,
        warnings: vec!["rollout audit records readiness only and does not run canary rollout".into()],
        facts: vec![
            "bundles gate, rollback binding, and cutover boundary audit rows".into(),
            "keeps R4 limited rollout separated from production traffic cutover".into(),
        ],
        next_safe_batch: "loopback-r4-expanded-opt-in-closeout-readiness".into(),
    })
}

pub async fn mihomo_kernel_loopback_r4_expanded_opt_in_closeout_readiness(
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
) -> Result<KernelLoopbackR4ExpandedOptInCloseoutReadinessReport> {
    let closeout_decision = closeout_decision.unwrap_or(false);
    let audit = mihomo_kernel_loopback_r4_expanded_opt_in_rollout_audit(
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
    )
    .await?;
    let checks = vec![
        KernelLoopbackR4ExpandedOptInLimitedRolloutGateCheck {
            name: "rolloutAudit".into(),
            status: if audit.audit_ready { "passed" } else { "blocked" }.into(),
            passed: audit.audit_ready,
            blockers: if audit.audit_ready {
                Vec::new()
            } else {
                audit.blockers.clone()
            },
            facts: vec!["closeout readiness consumes rollout audit evidence".into()],
        },
        KernelLoopbackR4ExpandedOptInLimitedRolloutGateCheck {
            name: "closeoutDecision".into(),
            status: if closeout_decision { "passed" } else { "blocked" }.into(),
            passed: closeout_decision,
            blockers: if closeout_decision {
                Vec::new()
            } else {
                vec!["R4 closeout requires an explicit closeout decision".into()]
            },
            facts: vec!["closeout decision is separate from rollout gate decisions".into()],
        },
    ];
    let closeout_ready = checks.iter().all(|check| check.passed);
    let blockers = checks
        .iter()
        .flat_map(|check| check.blockers.clone())
        .collect::<Vec<String>>();

    Ok(KernelLoopbackR4ExpandedOptInCloseoutReadinessReport {
        runtime_id: MIHOMO_RUNTIME_ID.into(),
        component: "loopback-r4-expanded-opt-in-closeout-readiness".into(),
        kernel_area: "forwarding".into(),
        mutates_runtime: audit.mutates_runtime,
        live_execution_allowed: audit.live_execution_allowed,
        current_platform: audit.current_platform.clone(),
        current_arch: audit.current_arch.clone(),
        closeout_decision,
        closeout_ready,
        limited_rollout_allowed: false,
        expanded_opt_in_allowed: false,
        audit,
        checks,
        default_route: false,
        forwards_traffic: false,
        outbound_adapters_used: false,
        mihomo_fallback: true,
        passed: closeout_ready,
        blockers,
        warnings: vec!["closeout readiness does not authorize production forwarding or default cutover".into()],
        facts: vec![
            "collects final R4 readiness evidence for a separate closeout report".into(),
            "leaves Go Mihomo data plane ownership unchanged".into(),
        ],
        next_safe_batch: "loopback-r4-expanded-opt-in-closeout-report".into(),
    })
}

pub async fn mihomo_kernel_loopback_r4_expanded_opt_in_closeout_report(
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
) -> Result<KernelLoopbackR4ExpandedOptInCloseoutReport> {
    let requested_execution = requested_execution.unwrap_or(false);
    let closeout_readiness = mihomo_kernel_loopback_r4_expanded_opt_in_closeout_readiness(
        listener_port,
        target_port,
        hold_started_at_epoch_ms,
        observed_rollback_platforms,
        explicit_decision,
        Some(requested_execution),
        post_execution_hold_started_at_epoch_ms,
        wider_opt_in_decision,
        limited_rollout_decision,
        canary_scope,
        max_canary_sessions,
        closeout_decision,
    )
    .await?;
    let r4_closeout_complete = closeout_readiness.closeout_ready;
    let mut evidence = Vec::new();
    evidence.extend(closeout_readiness.checks.clone());
    evidence.push(KernelLoopbackR4ExpandedOptInLimitedRolloutGateCheck {
        name: "r4Boundary".into(),
        status: "passed".into(),
        passed: true,
        blockers: Vec::new(),
        facts: vec!["R4 closeout report keeps R4 bounded to synthetic loopback evidence".into()],
    });
    evidence.push(KernelLoopbackR4ExpandedOptInLimitedRolloutGateCheck {
        name: "goDataPlaneBoundary".into(),
        status: "passed".into(),
        passed: true,
        blockers: Vec::new(),
        facts: vec!["Mihomo remains the production data plane after R4 closeout".into()],
    });
    let blockers = evidence
        .iter()
        .flat_map(|check| check.blockers.clone())
        .collect::<Vec<String>>();

    Ok(KernelLoopbackR4ExpandedOptInCloseoutReport {
        runtime_id: MIHOMO_RUNTIME_ID.into(),
        component: "loopback-r4-expanded-opt-in-closeout-report".into(),
        kernel_area: "forwarding".into(),
        mutates_runtime: closeout_readiness.mutates_runtime,
        live_execution_allowed: closeout_readiness.live_execution_allowed,
        current_platform: closeout_readiness.current_platform.clone(),
        current_arch: closeout_readiness.current_arch.clone(),
        requested_execution,
        explicit_decision: closeout_readiness.audit.gate.decision_readiness.explicit_decision,
        closeout_decision: closeout_readiness.closeout_decision,
        closeout_ready: closeout_readiness.closeout_ready,
        r4_closeout_complete,
        limited_rollout_allowed: false,
        expanded_opt_in_allowed: false,
        closeout_readiness,
        evidence,
        default_route: false,
        forwards_traffic: false,
        outbound_adapters_used: false,
        mihomo_fallback: true,
        passed: r4_closeout_complete,
        blockers,
        warnings: vec!["R4 closeout is not default cutover or production forwarding permission".into()],
        facts: vec![
            "summarizes R4 synthetic execution, hold, decision, rollout gate, audit, and closeout readiness".into(),
            "keeps real adapters, TUN, protocol handlers, system proxy, and default route blocked".into(),
        ],
        next_safe_batch: "loopback-r4-expanded-opt-in-completion-summary".into(),
    })
}

pub async fn mihomo_kernel_loopback_r4_expanded_opt_in_completion_summary(
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
) -> Result<KernelLoopbackR4ExpandedOptInCompletionReport> {
    let closeout_report = mihomo_kernel_loopback_r4_expanded_opt_in_closeout_report(
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
    )
    .await?;
    let r4_complete = closeout_report.r4_closeout_complete;
    let blockers = if r4_complete {
        Vec::new()
    } else {
        closeout_report.blockers.clone()
    };

    Ok(KernelLoopbackR4ExpandedOptInCompletionReport {
        runtime_id: MIHOMO_RUNTIME_ID.into(),
        component: "loopback-r4-expanded-opt-in-completion-summary".into(),
        kernel_area: "forwarding".into(),
        mutates_runtime: closeout_report.mutates_runtime,
        live_execution_allowed: closeout_report.live_execution_allowed,
        current_platform: closeout_report.current_platform.clone(),
        current_arch: closeout_report.current_arch.clone(),
        r4_complete,
        completed_batches: vec![
            "loopback-r4-expanded-opt-in-preflight".into(),
            "loopback-r4-expanded-opt-in-execution-plan".into(),
            "loopback-r4-expanded-opt-in-execution-guard".into(),
            "loopback-r4-expanded-opt-in-synthetic-execution".into(),
            "loopback-r4-expanded-opt-in-post-execution-hold".into(),
            "loopback-r4-expanded-opt-in-decision-readiness".into(),
            "loopback-r4-expanded-opt-in-limited-rollout-gate".into(),
            "loopback-r4-expanded-opt-in-rollout-audit".into(),
            "loopback-r4-expanded-opt-in-closeout-readiness".into(),
            "loopback-r4-expanded-opt-in-closeout-report".into(),
        ],
        open_boundaries: vec![
            "realAdapterForwarding".into(),
            "tunForwarding".into(),
            "protocolHandlers".into(),
            "systemProxyCutover".into(),
            "defaultRouteCutover".into(),
        ],
        next_phase_candidate: "loopback-r5-default-cutover-preflight".into(),
        default_cutover_allowed: false,
        expanded_opt_in_allowed: false,
        closeout_report,
        passed: r4_complete,
        blockers,
        warnings: vec!["R4 completion summary does not enter R5 automatically".into()],
        facts: vec![
            "R4 completion is a documentation and evidence boundary only".into(),
            "R5 must start with a separate preflight before any default cutover work".into(),
        ],
        next_safe_batch: "loopback-r4-expanded-opt-in-next-phase-handoff".into(),
    })
}

pub async fn mihomo_kernel_loopback_r4_expanded_opt_in_next_phase_handoff(
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
) -> Result<KernelLoopbackR4ExpandedOptInNextPhaseHandoffReport> {
    let handoff_decision = handoff_decision.unwrap_or(false);
    let completion = mihomo_kernel_loopback_r4_expanded_opt_in_completion_summary(
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
    )
    .await?;
    let checks = vec![
        KernelLoopbackR4ExpandedOptInLimitedRolloutGateCheck {
            name: "r4Completion".into(),
            status: if completion.r4_complete { "passed" } else { "blocked" }.into(),
            passed: completion.r4_complete,
            blockers: if completion.r4_complete {
                Vec::new()
            } else {
                completion.blockers.clone()
            },
            facts: vec!["handoff requires completed R4 closeout report evidence".into()],
        },
        KernelLoopbackR4ExpandedOptInLimitedRolloutGateCheck {
            name: "handoffDecision".into(),
            status: if handoff_decision { "passed" } else { "blocked" }.into(),
            passed: handoff_decision,
            blockers: if handoff_decision {
                Vec::new()
            } else {
                vec!["next phase handoff requires an explicit handoff decision".into()]
            },
            facts: vec!["handoff decision only allows planning the next preflight".into()],
        },
    ];
    let handoff_ready = checks.iter().all(|check| check.passed);
    let blockers = checks
        .iter()
        .flat_map(|check| check.blockers.clone())
        .collect::<Vec<String>>();

    Ok(KernelLoopbackR4ExpandedOptInNextPhaseHandoffReport {
        runtime_id: MIHOMO_RUNTIME_ID.into(),
        component: "loopback-r4-expanded-opt-in-next-phase-handoff".into(),
        kernel_area: "forwarding".into(),
        mutates_runtime: completion.mutates_runtime,
        live_execution_allowed: completion.live_execution_allowed,
        current_platform: completion.current_platform.clone(),
        current_arch: completion.current_arch.clone(),
        handoff_decision,
        handoff_ready,
        next_phase: completion.next_phase_candidate.clone(),
        default_cutover_allowed: false,
        expanded_opt_in_allowed: false,
        completion,
        checks,
        default_route: false,
        forwards_traffic: false,
        outbound_adapters_used: false,
        mihomo_fallback: true,
        passed: handoff_ready,
        blockers,
        warnings: vec!["handoff readiness does not authorize R5 execution or default cutover".into()],
        facts: vec![
            "next phase starts at preflight only".into(),
            "Mihomo remains the active kernel and production data plane".into(),
        ],
        next_safe_batch: "loopback-r5-default-cutover-preflight".into(),
    })
}
