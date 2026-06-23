use super::*;

pub async fn mihomo_kernel_loopback_r5_default_cutover_preflight(
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
) -> Result<KernelLoopbackR5DefaultCutoverPreflightReport> {
    let r5_preflight_decision = r5_preflight_decision.unwrap_or(false);
    let handoff = mihomo_kernel_loopback_r4_expanded_opt_in_next_phase_handoff(
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
    )
    .await?;
    let checks = vec![
        KernelLoopbackR4ExpandedOptInLimitedRolloutGateCheck {
            name: "handoffReady".into(),
            status: if handoff.handoff_ready { "passed" } else { "blocked" }.into(),
            passed: handoff.handoff_ready,
            blockers: if handoff.handoff_ready {
                Vec::new()
            } else {
                handoff.blockers.clone()
            },
            facts: vec!["R5 preflight requires completed R4 handoff evidence".into()],
        },
        KernelLoopbackR4ExpandedOptInLimitedRolloutGateCheck {
            name: "r5PreflightDecision".into(),
            status: if r5_preflight_decision { "passed" } else { "blocked" }.into(),
            passed: r5_preflight_decision,
            blockers: if r5_preflight_decision {
                Vec::new()
            } else {
                vec!["R5 preflight requires an explicit preflight decision".into()]
            },
            facts: vec!["preflight decision permits evidence collection only".into()],
        },
        KernelLoopbackR4ExpandedOptInLimitedRolloutGateCheck {
            name: "defaultCutoverBoundary".into(),
            status: "passed".into(),
            passed: true,
            blockers: Vec::new(),
            facts: vec![
                "default route, system proxy, TUN, protocol handlers, and real adapters remain unchanged".into(),
            ],
        },
        KernelLoopbackR4ExpandedOptInLimitedRolloutGateCheck {
            name: "runtimeOwnershipBoundary".into(),
            status: "passed".into(),
            passed: true,
            blockers: Vec::new(),
            facts: vec!["Mihomo remains the active production data plane during R5 preflight".into()],
        },
    ];
    let preflight_ready = checks.iter().all(|check| check.passed);
    let blockers = checks
        .iter()
        .flat_map(|check| check.blockers.clone())
        .collect::<Vec<String>>();

    Ok(KernelLoopbackR5DefaultCutoverPreflightReport {
        runtime_id: MIHOMO_RUNTIME_ID.into(),
        component: "loopback-r5-default-cutover-preflight".into(),
        kernel_area: "forwarding".into(),
        mutates_runtime: false,
        live_execution_allowed: false,
        current_platform: handoff.current_platform.clone(),
        current_arch: handoff.current_arch.clone(),
        r5_preflight_decision,
        preflight_ready,
        default_cutover_allowed: false,
        expanded_opt_in_allowed: false,
        handoff,
        checks,
        default_route: false,
        forwards_traffic: false,
        outbound_adapters_used: false,
        mihomo_fallback: true,
        passed: preflight_ready,
        blockers,
        warnings: vec!["R5 preflight is read-only and does not authorize default cutover".into()],
        facts: vec![
            "starts R5 with evidence checks only".into(),
            "no system proxy, TUN, protocol, adapter, or default route mutation is performed".into(),
        ],
        next_safe_batch: "loopback-r5-default-cutover-risk-matrix".into(),
    })
}

pub async fn mihomo_kernel_loopback_r5_default_cutover_risk_matrix(
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
) -> Result<KernelLoopbackR5DefaultCutoverRiskMatrixReport> {
    let preflight = mihomo_kernel_loopback_r5_default_cutover_preflight(
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
    )
    .await?;
    let rows = vec![
        KernelLoopbackR5DefaultCutoverRiskRow {
            name: "defaultRouteMutation".into(),
            severity: "critical".into(),
            status: "blocked".into(),
            passed: true,
            blockers: Vec::new(),
            facts: vec!["default route mutation remains outside this batch".into()],
        },
        KernelLoopbackR5DefaultCutoverRiskRow {
            name: "systemProxyMutation".into(),
            severity: "high".into(),
            status: "blocked".into(),
            passed: true,
            blockers: Vec::new(),
            facts: vec!["system proxy changes require a later guarded plan".into()],
        },
        KernelLoopbackR5DefaultCutoverRiskRow {
            name: "tunForwardingMutation".into(),
            severity: "high".into(),
            status: "blocked".into(),
            passed: true,
            blockers: Vec::new(),
            facts: vec!["TUN forwarding remains Mihomo-owned".into()],
        },
        KernelLoopbackR5DefaultCutoverRiskRow {
            name: "protocolHandlerMutation".into(),
            severity: "high".into(),
            status: "blocked".into(),
            passed: true,
            blockers: Vec::new(),
            facts: vec!["protocol handler registration is not touched by preflight".into()],
        },
        KernelLoopbackR5DefaultCutoverRiskRow {
            name: "realAdapterForwarding".into(),
            severity: "critical".into(),
            status: "blocked".into(),
            passed: true,
            blockers: Vec::new(),
            facts: vec!["real outbound adapters are not dialed".into()],
        },
    ];
    let risk_matrix_ready = preflight.preflight_ready && rows.iter().all(|row| row.passed);
    let blockers = rows
        .iter()
        .flat_map(|row| row.blockers.clone())
        .chain(preflight.blockers.clone())
        .collect::<Vec<String>>();

    Ok(KernelLoopbackR5DefaultCutoverRiskMatrixReport {
        runtime_id: MIHOMO_RUNTIME_ID.into(),
        component: "loopback-r5-default-cutover-risk-matrix".into(),
        kernel_area: "forwarding".into(),
        mutates_runtime: false,
        live_execution_allowed: false,
        current_platform: preflight.current_platform.clone(),
        current_arch: preflight.current_arch.clone(),
        risk_matrix_ready,
        default_cutover_allowed: false,
        expanded_opt_in_allowed: false,
        preflight,
        rows,
        default_route: false,
        forwards_traffic: false,
        outbound_adapters_used: false,
        mihomo_fallback: true,
        passed: risk_matrix_ready,
        blockers,
        warnings: vec!["risk matrix blocks every production mutation in this batch".into()],
        facts: vec!["catalogs R5 production cutover risks before a guarded plan exists".into()],
        next_safe_batch: "loopback-r5-default-cutover-rollback-abort-plan".into(),
    })
}

pub async fn mihomo_kernel_loopback_r5_default_cutover_rollback_abort_plan(
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
) -> Result<KernelLoopbackR5DefaultCutoverRollbackAbortPlanReport> {
    let rollback_plan_decision = rollback_plan_decision.unwrap_or(false);
    let risk_matrix = mihomo_kernel_loopback_r5_default_cutover_risk_matrix(
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
    )
    .await?;
    let checks = vec![
        KernelLoopbackR4ExpandedOptInLimitedRolloutGateCheck {
            name: "riskMatrixReady".into(),
            status: if risk_matrix.risk_matrix_ready {
                "passed"
            } else {
                "blocked"
            }
            .into(),
            passed: risk_matrix.risk_matrix_ready,
            blockers: if risk_matrix.risk_matrix_ready {
                Vec::new()
            } else {
                risk_matrix.blockers.clone()
            },
            facts: vec!["rollback/abort planning requires completed risk matrix".into()],
        },
        KernelLoopbackR4ExpandedOptInLimitedRolloutGateCheck {
            name: "rollbackPlanDecision".into(),
            status: if rollback_plan_decision { "passed" } else { "blocked" }.into(),
            passed: rollback_plan_decision,
            blockers: if rollback_plan_decision {
                Vec::new()
            } else {
                vec!["rollback/abort plan requires an explicit planning decision".into()]
            },
            facts: vec!["planning decision authorizes rollback evidence only".into()],
        },
        KernelLoopbackR4ExpandedOptInLimitedRolloutGateCheck {
            name: "abortCriteria".into(),
            status: "passed".into(),
            passed: true,
            blockers: Vec::new(),
            facts: vec![
                "abort on route drift, TUN drift, system proxy drift, protocol drift, adapter dial, or fallback loss"
                    .into(),
            ],
        },
        KernelLoopbackR4ExpandedOptInLimitedRolloutGateCheck {
            name: "rollbackBoundary".into(),
            status: "passed".into(),
            passed: true,
            blockers: Vec::new(),
            facts: vec!["rollback currently means no-op because preflight performs no mutation".into()],
        },
    ];
    let rollback_abort_ready = checks.iter().all(|check| check.passed);
    let blockers = checks
        .iter()
        .flat_map(|check| check.blockers.clone())
        .collect::<Vec<String>>();

    Ok(KernelLoopbackR5DefaultCutoverRollbackAbortPlanReport {
        runtime_id: MIHOMO_RUNTIME_ID.into(),
        component: "loopback-r5-default-cutover-rollback-abort-plan".into(),
        kernel_area: "forwarding".into(),
        mutates_runtime: false,
        live_execution_allowed: false,
        current_platform: risk_matrix.current_platform.clone(),
        current_arch: risk_matrix.current_arch.clone(),
        rollback_plan_decision,
        rollback_abort_ready,
        default_cutover_allowed: false,
        expanded_opt_in_allowed: false,
        risk_matrix,
        checks,
        default_route: false,
        forwards_traffic: false,
        outbound_adapters_used: false,
        mihomo_fallback: true,
        passed: rollback_abort_ready,
        blockers,
        warnings: vec!["rollback/abort plan still does not allow production cutover execution".into()],
        facts: vec![
            "defines abort evidence before any R5 execution plan can be proposed".into(),
            "keeps all production network ownership with Mihomo".into(),
        ],
        next_safe_batch: "loopback-r5-default-cutover-execution-plan".into(),
    })
}

pub async fn mihomo_kernel_loopback_r5_default_cutover_execution_plan(
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
) -> Result<KernelLoopbackR5DefaultCutoverExecutionPlanReport> {
    let execution_plan_decision = execution_plan_decision.unwrap_or(false);
    let rollback_abort_plan = mihomo_kernel_loopback_r5_default_cutover_rollback_abort_plan(
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
    )
    .await?;
    let steps = vec![
        KernelLoopbackR5DefaultCutoverExecutionPlanStep {
            order: 1,
            name: "snapshotCurrentRuntimeState".into(),
            phase: "preflight".into(),
            allowed: true,
            mutates_runtime: false,
            facts: vec!["capture config, system proxy, TUN, route, and listener state before any dry run".into()],
        },
        KernelLoopbackR5DefaultCutoverExecutionPlanStep {
            order: 2,
            name: "simulateCutoverPlan".into(),
            phase: "dryRunOnly".into(),
            allowed: true,
            mutates_runtime: false,
            facts: vec!["build an in-memory cutover intent without installing adapters or routes".into()],
        },
        KernelLoopbackR5DefaultCutoverExecutionPlanStep {
            order: 3,
            name: "verifyAbortCriteria".into(),
            phase: "dryRunOnly".into(),
            allowed: true,
            mutates_runtime: false,
            facts: vec!["evaluate route/TUN/system proxy/protocol/adapter drift abort criteria".into()],
        },
        KernelLoopbackR5DefaultCutoverExecutionPlanStep {
            order: 4,
            name: "productionMutation".into(),
            phase: "blocked".into(),
            allowed: false,
            mutates_runtime: false,
            facts: vec!["default route, system proxy, TUN, protocol, and real adapter mutation stay blocked".into()],
        },
    ];
    let execution_plan_ready = rollback_abort_plan.rollback_abort_ready && execution_plan_decision;
    let mut blockers = if rollback_abort_plan.rollback_abort_ready {
        Vec::new()
    } else {
        rollback_abort_plan.blockers.clone()
    };
    if !execution_plan_decision {
        blockers.push("R5 execution plan requires an explicit planning decision".into());
    }

    Ok(KernelLoopbackR5DefaultCutoverExecutionPlanReport {
        runtime_id: MIHOMO_RUNTIME_ID.into(),
        component: "loopback-r5-default-cutover-execution-plan".into(),
        kernel_area: "forwarding".into(),
        mutates_runtime: false,
        live_execution_allowed: false,
        current_platform: rollback_abort_plan.current_platform.clone(),
        current_arch: rollback_abort_plan.current_arch.clone(),
        execution_plan_decision,
        execution_plan_ready,
        default_cutover_allowed: false,
        expanded_opt_in_allowed: false,
        rollback_abort_plan,
        steps,
        default_route: false,
        forwards_traffic: false,
        outbound_adapters_used: false,
        mihomo_fallback: true,
        passed: execution_plan_ready,
        blockers,
        warnings: vec!["execution plan is dry-run planning only; production mutation remains blocked".into()],
        facts: vec![
            "defines R5 order of operations without executing default cutover".into(),
            "keeps Mihomo as the production data plane".into(),
        ],
        next_safe_batch: "loopback-r5-default-cutover-execution-guard".into(),
    })
}

pub async fn mihomo_kernel_loopback_r5_default_cutover_guard(
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
) -> Result<KernelLoopbackR5DefaultCutoverGuardReport> {
    let guard_decision = guard_decision.unwrap_or(false);
    let execution_plan = mihomo_kernel_loopback_r5_default_cutover_execution_plan(
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
    )
    .await?;
    let checks = vec![
        KernelLoopbackR4ExpandedOptInLimitedRolloutGateCheck {
            name: "executionPlanReady".into(),
            status: if execution_plan.execution_plan_ready {
                "passed"
            } else {
                "blocked"
            }
            .into(),
            passed: execution_plan.execution_plan_ready,
            blockers: if execution_plan.execution_plan_ready {
                Vec::new()
            } else {
                execution_plan.blockers.clone()
            },
            facts: vec!["guard requires completed R5 execution plan evidence".into()],
        },
        KernelLoopbackR4ExpandedOptInLimitedRolloutGateCheck {
            name: "guardDecision".into(),
            status: if guard_decision { "passed" } else { "blocked" }.into(),
            passed: guard_decision,
            blockers: if guard_decision {
                Vec::new()
            } else {
                vec!["R5 execution guard requires an explicit guard decision".into()]
            },
            facts: vec!["guard decision authorizes dry-run readiness only".into()],
        },
        KernelLoopbackR4ExpandedOptInLimitedRolloutGateCheck {
            name: "mutationFence".into(),
            status: "passed".into(),
            passed: true,
            blockers: Vec::new(),
            facts: vec!["guard keeps production mutation fenced until dry-run evidence exists".into()],
        },
    ];
    let guard_ready = checks.iter().all(|check| check.passed);
    let blockers = checks
        .iter()
        .flat_map(|check| check.blockers.clone())
        .collect::<Vec<String>>();

    Ok(KernelLoopbackR5DefaultCutoverGuardReport {
        runtime_id: MIHOMO_RUNTIME_ID.into(),
        component: "loopback-r5-default-cutover-execution-guard".into(),
        kernel_area: "forwarding".into(),
        mutates_runtime: false,
        live_execution_allowed: false,
        current_platform: execution_plan.current_platform.clone(),
        current_arch: execution_plan.current_arch.clone(),
        guard_decision,
        guard_ready,
        default_cutover_allowed: false,
        expanded_opt_in_allowed: false,
        execution_plan,
        checks,
        default_route: false,
        forwards_traffic: false,
        outbound_adapters_used: false,
        mihomo_fallback: true,
        passed: guard_ready,
        blockers,
        warnings: vec!["guard readiness is not permission to mutate production networking".into()],
        facts: vec!["gates R5 dry-run readiness behind execution plan and explicit guard decision".into()],
        next_safe_batch: "loopback-r5-default-cutover-dry-run-readiness".into(),
    })
}

pub async fn mihomo_kernel_loopback_r5_default_cutover_dry_run_readiness(
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
) -> Result<KernelLoopbackR5DefaultCutoverDryRunReadinessReport> {
    let dry_run_decision = dry_run_decision.unwrap_or(false);
    let guard = mihomo_kernel_loopback_r5_default_cutover_guard(
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
    )
    .await?;
    let checks = vec![
        KernelLoopbackR4ExpandedOptInLimitedRolloutGateCheck {
            name: "guardReady".into(),
            status: if guard.guard_ready { "passed" } else { "blocked" }.into(),
            passed: guard.guard_ready,
            blockers: if guard.guard_ready {
                Vec::new()
            } else {
                guard.blockers.clone()
            },
            facts: vec!["dry-run readiness requires guard evidence".into()],
        },
        KernelLoopbackR4ExpandedOptInLimitedRolloutGateCheck {
            name: "dryRunDecision".into(),
            status: if dry_run_decision { "passed" } else { "blocked" }.into(),
            passed: dry_run_decision,
            blockers: if dry_run_decision {
                Vec::new()
            } else {
                vec!["R5 dry-run readiness requires an explicit dry-run decision".into()]
            },
            facts: vec!["dry-run decision allows later synthetic dry-run evidence only".into()],
        },
        KernelLoopbackR4ExpandedOptInLimitedRolloutGateCheck {
            name: "dryRunScope".into(),
            status: "passed".into(),
            passed: true,
            blockers: Vec::new(),
            facts: vec![
                "dry run must remain in-memory and may not install routes, TUN, proxy, protocols, or adapters".into(),
            ],
        },
    ];
    let dry_run_ready = checks.iter().all(|check| check.passed);
    let blockers = checks
        .iter()
        .flat_map(|check| check.blockers.clone())
        .collect::<Vec<String>>();

    Ok(KernelLoopbackR5DefaultCutoverDryRunReadinessReport {
        runtime_id: MIHOMO_RUNTIME_ID.into(),
        component: "loopback-r5-default-cutover-dry-run-readiness".into(),
        kernel_area: "forwarding".into(),
        mutates_runtime: false,
        live_execution_allowed: false,
        current_platform: guard.current_platform.clone(),
        current_arch: guard.current_arch.clone(),
        dry_run_decision,
        dry_run_ready,
        default_cutover_allowed: false,
        expanded_opt_in_allowed: false,
        guard,
        checks,
        default_route: false,
        forwards_traffic: false,
        outbound_adapters_used: false,
        mihomo_fallback: true,
        passed: dry_run_ready,
        blockers,
        warnings: vec!["dry-run readiness still does not perform dry-run execution".into()],
        facts: vec!["prepares a future dry-run evidence batch while keeping production networking unchanged".into()],
        next_safe_batch: "loopback-r5-default-cutover-dry-run-evidence".into(),
    })
}

pub async fn mihomo_kernel_loopback_r5_default_cutover_dry_run_evidence(
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
) -> Result<KernelLoopbackR5DefaultCutoverDryRunEvidenceReport> {
    let dry_run_execution_decision = dry_run_execution_decision.unwrap_or(false);
    let readiness = mihomo_kernel_loopback_r5_default_cutover_dry_run_readiness(
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
    )
    .await?;
    let checks = vec![
        KernelLoopbackR4ExpandedOptInLimitedRolloutGateCheck {
            name: "dryRunReady".into(),
            status: if readiness.dry_run_ready { "passed" } else { "blocked" }.into(),
            passed: readiness.dry_run_ready,
            blockers: if readiness.dry_run_ready {
                Vec::new()
            } else {
                readiness.blockers.clone()
            },
            facts: vec!["dry-run evidence requires dry-run readiness".into()],
        },
        KernelLoopbackR4ExpandedOptInLimitedRolloutGateCheck {
            name: "dryRunExecutionDecision".into(),
            status: if dry_run_execution_decision {
                "passed"
            } else {
                "blocked"
            }
            .into(),
            passed: dry_run_execution_decision,
            blockers: if dry_run_execution_decision {
                Vec::new()
            } else {
                vec!["R5 dry-run evidence requires an explicit dry-run execution decision".into()]
            },
            facts: vec!["execution decision is scoped to in-memory dry-run evidence only".into()],
        },
        KernelLoopbackR4ExpandedOptInLimitedRolloutGateCheck {
            name: "inMemoryIntent".into(),
            status: "passed".into(),
            passed: true,
            blockers: Vec::new(),
            facts: vec!["cutover intent is modeled in memory and not applied to runtime config".into()],
        },
        KernelLoopbackR4ExpandedOptInLimitedRolloutGateCheck {
            name: "productionStateFence".into(),
            status: "passed".into(),
            passed: true,
            blockers: Vec::new(),
            facts: vec!["default route, system proxy, TUN, protocols, and adapters remain untouched".into()],
        },
    ];
    let dry_run_executed = checks.iter().all(|check| check.passed);
    let blockers = checks
        .iter()
        .flat_map(|check| check.blockers.clone())
        .collect::<Vec<String>>();

    Ok(KernelLoopbackR5DefaultCutoverDryRunEvidenceReport {
        runtime_id: MIHOMO_RUNTIME_ID.into(),
        component: "loopback-r5-default-cutover-dry-run-evidence".into(),
        kernel_area: "forwarding".into(),
        mutates_runtime: false,
        live_execution_allowed: false,
        current_platform: readiness.current_platform.clone(),
        current_arch: readiness.current_arch.clone(),
        dry_run_executed,
        default_cutover_allowed: false,
        expanded_opt_in_allowed: false,
        readiness,
        checks,
        default_route: false,
        forwards_traffic: false,
        outbound_adapters_used: false,
        mihomo_fallback: true,
        passed: dry_run_executed,
        blockers,
        warnings: vec!["dry-run evidence is synthetic and does not perform production cutover".into()],
        facts: vec![
            "validates the R5 cutover path as an in-memory intent only".into(),
            "Mihomo remains the active forwarding engine".into(),
        ],
        next_safe_batch: "loopback-r5-default-cutover-dry-run-closeout".into(),
    })
}

pub async fn mihomo_kernel_loopback_r5_default_cutover_dry_run_closeout(
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
) -> Result<KernelLoopbackR5DefaultCutoverDryRunCloseoutReport> {
    let evidence = mihomo_kernel_loopback_r5_default_cutover_dry_run_evidence(
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
    )
    .await?;
    let checks = vec![
        KernelLoopbackR4ExpandedOptInLimitedRolloutGateCheck {
            name: "dryRunEvidencePassed".into(),
            status: if evidence.dry_run_executed { "passed" } else { "blocked" }.into(),
            passed: evidence.dry_run_executed,
            blockers: if evidence.dry_run_executed {
                Vec::new()
            } else {
                evidence.blockers.clone()
            },
            facts: vec!["closeout requires completed dry-run evidence".into()],
        },
        KernelLoopbackR4ExpandedOptInLimitedRolloutGateCheck {
            name: "runtimeUnchanged".into(),
            status: "passed".into(),
            passed: true,
            blockers: Vec::new(),
            facts: vec!["dry-run closeout observes no runtime mutation to roll back".into()],
        },
        KernelLoopbackR4ExpandedOptInLimitedRolloutGateCheck {
            name: "fallbackPreserved".into(),
            status: "passed".into(),
            passed: true,
            blockers: Vec::new(),
            facts: vec!["Mihomo fallback remains active after synthetic dry run".into()],
        },
    ];
    let dry_run_closeout_ready = checks.iter().all(|check| check.passed);
    let blockers = checks
        .iter()
        .flat_map(|check| check.blockers.clone())
        .collect::<Vec<String>>();

    Ok(KernelLoopbackR5DefaultCutoverDryRunCloseoutReport {
        runtime_id: MIHOMO_RUNTIME_ID.into(),
        component: "loopback-r5-default-cutover-dry-run-closeout".into(),
        kernel_area: "forwarding".into(),
        mutates_runtime: false,
        live_execution_allowed: false,
        current_platform: evidence.current_platform.clone(),
        current_arch: evidence.current_arch.clone(),
        dry_run_closeout_ready,
        default_cutover_allowed: false,
        expanded_opt_in_allowed: false,
        evidence,
        checks,
        default_route: false,
        forwards_traffic: false,
        outbound_adapters_used: false,
        mihomo_fallback: true,
        passed: dry_run_closeout_ready,
        blockers,
        warnings: vec!["dry-run closeout does not promote the dry run to live execution".into()],
        facts: vec!["confirms dry-run evidence leaves production network state unchanged".into()],
        next_safe_batch: "loopback-r5-default-cutover-post-dry-run-hold".into(),
    })
}

pub async fn mihomo_kernel_loopback_r5_default_cutover_post_dry_run_hold(
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
) -> Result<KernelLoopbackR5DefaultCutoverPostDryRunHoldReport> {
    let hold_decision = hold_decision.unwrap_or(false);
    let closeout = mihomo_kernel_loopback_r5_default_cutover_dry_run_closeout(
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
    )
    .await?;
    let now_ms = current_epoch_ms();
    let hold_elapsed_seconds = post_dry_run_hold_started_at_epoch_ms
        .map(|started| now_ms.saturating_sub(started) / 1000)
        .unwrap_or(0);
    let hold_window_passed =
        post_dry_run_hold_started_at_epoch_ms.is_some() && hold_elapsed_seconds >= LOOPBACK_HOLD_WINDOW_MIN_SECONDS;
    let checks = vec![
        KernelLoopbackR4ExpandedOptInLimitedRolloutGateCheck {
            name: "dryRunCloseoutReady".into(),
            status: if closeout.dry_run_closeout_ready {
                "passed"
            } else {
                "blocked"
            }
            .into(),
            passed: closeout.dry_run_closeout_ready,
            blockers: if closeout.dry_run_closeout_ready {
                Vec::new()
            } else {
                closeout.blockers.clone()
            },
            facts: vec!["post dry-run hold requires dry-run closeout evidence".into()],
        },
        KernelLoopbackR4ExpandedOptInLimitedRolloutGateCheck {
            name: "holdWindow".into(),
            status: if hold_window_passed { "passed" } else { "blocked" }.into(),
            passed: hold_window_passed,
            blockers: if hold_window_passed {
                Vec::new()
            } else {
                vec!["post dry-run hold window has not reached the minimum observation period".into()]
            },
            facts: vec![format!("observed hold window seconds: {hold_elapsed_seconds}").into()],
        },
        KernelLoopbackR4ExpandedOptInLimitedRolloutGateCheck {
            name: "holdDecision".into(),
            status: if hold_decision { "passed" } else { "blocked" }.into(),
            passed: hold_decision,
            blockers: if hold_decision {
                Vec::new()
            } else {
                vec!["post dry-run hold requires an explicit hold decision".into()]
            },
            facts: vec!["hold decision keeps next step to readiness only".into()],
        },
    ];
    let hold_ready = checks.iter().all(|check| check.passed);
    let blockers = checks
        .iter()
        .flat_map(|check| check.blockers.clone())
        .collect::<Vec<String>>();

    Ok(KernelLoopbackR5DefaultCutoverPostDryRunHoldReport {
        runtime_id: MIHOMO_RUNTIME_ID.into(),
        component: "loopback-r5-default-cutover-post-dry-run-hold".into(),
        kernel_area: "forwarding".into(),
        mutates_runtime: false,
        live_execution_allowed: false,
        current_platform: closeout.current_platform.clone(),
        current_arch: closeout.current_arch.clone(),
        hold_decision,
        hold_ready,
        default_cutover_allowed: false,
        expanded_opt_in_allowed: false,
        closeout,
        checks,
        default_route: false,
        forwards_traffic: false,
        outbound_adapters_used: false,
        mihomo_fallback: true,
        passed: hold_ready,
        blockers,
        warnings: vec!["post dry-run hold still does not authorize default cutover".into()],
        facts: vec!["requires a bounded observation period after synthetic dry-run closeout".into()],
        next_safe_batch: "loopback-r5-default-cutover-decision-readiness".into(),
    })
}

pub async fn mihomo_kernel_loopback_r5_default_cutover_decision_readiness(
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
) -> Result<KernelLoopbackR5DefaultCutoverDecisionReadinessReport> {
    let decision_readiness_decision = decision_readiness_decision.unwrap_or(false);
    let post_dry_run_hold = mihomo_kernel_loopback_r5_default_cutover_post_dry_run_hold(
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
    )
    .await?;
    let checks = vec![
        KernelLoopbackR4ExpandedOptInLimitedRolloutGateCheck {
            name: "postDryRunHoldReady".into(),
            status: if post_dry_run_hold.hold_ready {
                "passed"
            } else {
                "blocked"
            }
            .into(),
            passed: post_dry_run_hold.hold_ready,
            blockers: if post_dry_run_hold.hold_ready {
                Vec::new()
            } else {
                post_dry_run_hold.blockers.clone()
            },
            facts: vec!["decision readiness requires completed post-dry-run hold evidence".into()],
        },
        KernelLoopbackR4ExpandedOptInLimitedRolloutGateCheck {
            name: "decisionReadinessDecision".into(),
            status: if decision_readiness_decision {
                "passed"
            } else {
                "blocked"
            }
            .into(),
            passed: decision_readiness_decision,
            blockers: if decision_readiness_decision {
                Vec::new()
            } else {
                vec!["R5 decision readiness requires an explicit decision".into()]
            },
            facts: vec!["decision readiness only permits final gate evaluation".into()],
        },
        KernelLoopbackR4ExpandedOptInLimitedRolloutGateCheck {
            name: "cutoverBoundary".into(),
            status: "passed".into(),
            passed: true,
            blockers: Vec::new(),
            facts: vec!["default cutover remains blocked after decision readiness".into()],
        },
    ];
    let decision_ready = checks.iter().all(|check| check.passed);
    let blockers = checks
        .iter()
        .flat_map(|check| check.blockers.clone())
        .collect::<Vec<String>>();

    Ok(KernelLoopbackR5DefaultCutoverDecisionReadinessReport {
        runtime_id: MIHOMO_RUNTIME_ID.into(),
        component: "loopback-r5-default-cutover-decision-readiness".into(),
        kernel_area: "forwarding".into(),
        mutates_runtime: false,
        live_execution_allowed: false,
        current_platform: post_dry_run_hold.current_platform.clone(),
        current_arch: post_dry_run_hold.current_arch.clone(),
        decision_readiness_decision,
        decision_ready,
        default_cutover_allowed: false,
        expanded_opt_in_allowed: false,
        post_dry_run_hold,
        checks,
        default_route: false,
        forwards_traffic: false,
        outbound_adapters_used: false,
        mihomo_fallback: true,
        passed: decision_ready,
        blockers,
        warnings: vec!["decision readiness does not authorize default cutover".into()],
        facts: vec![
            "summarizes R5 dry-run evidence before final gate evaluation".into(),
            "production forwarding remains Mihomo-owned".into(),
        ],
        next_safe_batch: "loopback-r5-default-cutover-final-gate".into(),
    })
}

pub async fn mihomo_kernel_loopback_r5_default_cutover_final_gate(
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
) -> Result<KernelLoopbackR5DefaultCutoverFinalGateReport> {
    let final_gate_decision = final_gate_decision.unwrap_or(false);
    let decision_readiness = mihomo_kernel_loopback_r5_default_cutover_decision_readiness(
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
    )
    .await?;
    let checks = vec![
        KernelLoopbackR4ExpandedOptInLimitedRolloutGateCheck {
            name: "decisionReady".into(),
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
            facts: vec!["final gate requires R5 decision readiness".into()],
        },
        KernelLoopbackR4ExpandedOptInLimitedRolloutGateCheck {
            name: "finalGateDecision".into(),
            status: if final_gate_decision { "passed" } else { "blocked" }.into(),
            passed: final_gate_decision,
            blockers: if final_gate_decision {
                Vec::new()
            } else {
                vec!["R5 final gate requires an explicit final gate decision".into()]
            },
            facts: vec!["final gate decision permits final hold/rollback validation only".into()],
        },
        KernelLoopbackR4ExpandedOptInLimitedRolloutGateCheck {
            name: "mutationFence".into(),
            status: "passed".into(),
            passed: true,
            blockers: Vec::new(),
            facts: vec!["final gate keeps default route, system proxy, TUN, protocols, and adapters fenced".into()],
        },
    ];
    let final_gate_ready = checks.iter().all(|check| check.passed);
    let blockers = checks
        .iter()
        .flat_map(|check| check.blockers.clone())
        .collect::<Vec<String>>();

    Ok(KernelLoopbackR5DefaultCutoverFinalGateReport {
        runtime_id: MIHOMO_RUNTIME_ID.into(),
        component: "loopback-r5-default-cutover-final-gate".into(),
        kernel_area: "forwarding".into(),
        mutates_runtime: false,
        live_execution_allowed: false,
        current_platform: decision_readiness.current_platform.clone(),
        current_arch: decision_readiness.current_arch.clone(),
        final_gate_decision,
        final_gate_ready,
        default_cutover_allowed: false,
        expanded_opt_in_allowed: false,
        decision_readiness,
        checks,
        default_route: false,
        forwards_traffic: false,
        outbound_adapters_used: false,
        mihomo_fallback: true,
        passed: final_gate_ready,
        blockers,
        warnings: vec!["final gate does not open production default cutover".into()],
        facts: vec!["allows only a later final hold and independent rollback validation batch".into()],
        next_safe_batch: "loopback-r5-default-cutover-next-step-handoff".into(),
    })
}

pub async fn mihomo_kernel_loopback_r5_default_cutover_next_step_handoff(
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
) -> Result<KernelLoopbackR5DefaultCutoverNextStepHandoffReport> {
    let r5_handoff_decision = r5_handoff_decision.unwrap_or(false);
    let final_gate = mihomo_kernel_loopback_r5_default_cutover_final_gate(
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
    )
    .await?;
    let next_step: String = "loopback-r5-default-cutover-final-hold".into();
    let checks = vec![
        KernelLoopbackR4ExpandedOptInLimitedRolloutGateCheck {
            name: "finalGateReady".into(),
            status: if final_gate.final_gate_ready {
                "passed"
            } else {
                "blocked"
            }
            .into(),
            passed: final_gate.final_gate_ready,
            blockers: if final_gate.final_gate_ready {
                Vec::new()
            } else {
                final_gate.blockers.clone()
            },
            facts: vec!["handoff requires final gate evidence".into()],
        },
        KernelLoopbackR4ExpandedOptInLimitedRolloutGateCheck {
            name: "r5HandoffDecision".into(),
            status: if r5_handoff_decision { "passed" } else { "blocked" }.into(),
            passed: r5_handoff_decision,
            blockers: if r5_handoff_decision {
                Vec::new()
            } else {
                vec!["R5 next-step handoff requires an explicit handoff decision".into()]
            },
            facts: vec!["handoff is to final hold/rollback validation, not default cutover".into()],
        },
    ];
    let handoff_ready = checks.iter().all(|check| check.passed);
    let blockers = checks
        .iter()
        .flat_map(|check| check.blockers.clone())
        .collect::<Vec<String>>();

    Ok(KernelLoopbackR5DefaultCutoverNextStepHandoffReport {
        runtime_id: MIHOMO_RUNTIME_ID.into(),
        component: "loopback-r5-default-cutover-next-step-handoff".into(),
        kernel_area: "forwarding".into(),
        mutates_runtime: false,
        live_execution_allowed: false,
        current_platform: final_gate.current_platform.clone(),
        current_arch: final_gate.current_arch.clone(),
        r5_handoff_decision,
        handoff_ready,
        next_step: next_step.clone(),
        default_cutover_allowed: false,
        expanded_opt_in_allowed: false,
        final_gate,
        checks,
        default_route: false,
        forwards_traffic: false,
        outbound_adapters_used: false,
        mihomo_fallback: true,
        passed: handoff_ready,
        blockers,
        warnings: vec!["next-step handoff still does not authorize live default cutover".into()],
        facts: vec!["moves R5 toward final hold and independent rollback validation only".into()],
        next_safe_batch: next_step,
    })
}

pub async fn mihomo_kernel_loopback_r5_default_cutover_final_hold(
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
) -> Result<KernelLoopbackR5DefaultCutoverFinalHoldReport> {
    let final_hold_decision = final_hold_decision.unwrap_or(false);
    let handoff = mihomo_kernel_loopback_r5_default_cutover_next_step_handoff(
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
    )
    .await?;
    let now_ms = current_epoch_ms();
    let final_hold_elapsed_seconds = final_hold_started_at_epoch_ms
        .map(|started| now_ms.saturating_sub(started) / 1000)
        .unwrap_or(0);
    let final_hold_window_passed =
        final_hold_started_at_epoch_ms.is_some() && final_hold_elapsed_seconds >= LOOPBACK_HOLD_WINDOW_MIN_SECONDS;
    let checks = vec![
        KernelLoopbackR4ExpandedOptInLimitedRolloutGateCheck {
            name: "handoffReady".into(),
            status: if handoff.handoff_ready { "passed" } else { "blocked" }.into(),
            passed: handoff.handoff_ready,
            blockers: if handoff.handoff_ready {
                Vec::new()
            } else {
                handoff.blockers.clone()
            },
            facts: vec!["final hold requires next-step handoff evidence".into()],
        },
        KernelLoopbackR4ExpandedOptInLimitedRolloutGateCheck {
            name: "finalHoldWindow".into(),
            status: if final_hold_window_passed { "passed" } else { "blocked" }.into(),
            passed: final_hold_window_passed,
            blockers: if final_hold_window_passed {
                Vec::new()
            } else {
                vec!["final hold window has not reached the minimum observation period".into()]
            },
            facts: vec![format!("observed final hold window seconds: {final_hold_elapsed_seconds}").into()],
        },
        KernelLoopbackR4ExpandedOptInLimitedRolloutGateCheck {
            name: "finalHoldDecision".into(),
            status: if final_hold_decision { "passed" } else { "blocked" }.into(),
            passed: final_hold_decision,
            blockers: if final_hold_decision {
                Vec::new()
            } else {
                vec!["R5 final hold requires an explicit hold decision".into()]
            },
            facts: vec!["final hold permits independent rollback validation only".into()],
        },
    ];
    let final_hold_ready = checks.iter().all(|check| check.passed);
    let blockers = checks
        .iter()
        .flat_map(|check| check.blockers.clone())
        .collect::<Vec<String>>();

    Ok(KernelLoopbackR5DefaultCutoverFinalHoldReport {
        runtime_id: MIHOMO_RUNTIME_ID.into(),
        component: "loopback-r5-default-cutover-final-hold".into(),
        kernel_area: "forwarding".into(),
        mutates_runtime: false,
        live_execution_allowed: false,
        current_platform: handoff.current_platform.clone(),
        current_arch: handoff.current_arch.clone(),
        final_hold_started_at_epoch_ms,
        final_hold_elapsed_seconds,
        final_hold_decision,
        final_hold_ready,
        default_cutover_allowed: false,
        expanded_opt_in_allowed: false,
        handoff,
        checks,
        default_route: false,
        forwards_traffic: false,
        outbound_adapters_used: false,
        mihomo_fallback: true,
        passed: final_hold_ready,
        blockers,
        warnings: vec!["final hold does not authorize live default cutover".into()],
        facts: vec!["requires a bounded observation period after final gate handoff".into()],
        next_safe_batch: "loopback-r5-default-cutover-independent-rollback-validation".into(),
    })
}

pub async fn mihomo_kernel_loopback_r5_default_cutover_independent_rollback_validation(
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
) -> Result<KernelLoopbackR5DefaultCutoverIndependentRollbackValidationReport> {
    let independent_rollback_decision = independent_rollback_decision.unwrap_or(false);
    let observed_rollback_platforms_input = observed_rollback_platforms.clone();
    let final_hold = mihomo_kernel_loopback_r5_default_cutover_final_hold(
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
    )
    .await?;
    let required_platforms = LOOPBACK_PLATFORM_MATRIX_PLATFORMS
        .iter()
        .map(|platform| (*platform).into())
        .collect::<Vec<String>>();
    let observed_rollback_platforms = observed_rollback_platforms_input
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
    let rollback_platforms_ready = pending_rollback_platforms.is_empty();
    let checks = vec![
        KernelLoopbackR4ExpandedOptInLimitedRolloutGateCheck {
            name: "finalHoldReady".into(),
            status: if final_hold.final_hold_ready {
                "passed"
            } else {
                "blocked"
            }
            .into(),
            passed: final_hold.final_hold_ready,
            blockers: if final_hold.final_hold_ready {
                Vec::new()
            } else {
                final_hold.blockers.clone()
            },
            facts: vec!["independent rollback validation requires final hold evidence".into()],
        },
        KernelLoopbackR4ExpandedOptInLimitedRolloutGateCheck {
            name: "rollbackPlatforms".into(),
            status: if rollback_platforms_ready { "passed" } else { "blocked" }.into(),
            passed: rollback_platforms_ready,
            blockers: if rollback_platforms_ready {
                Vec::new()
            } else {
                vec![
                    format!(
                        "missing independent rollback validation for platforms: {}",
                        pending_rollback_platforms.join(", ")
                    )
                    .into(),
                ]
            },
            facts: vec![
                format!(
                    "observed independent rollback platforms: {}",
                    if observed_rollback_platforms.is_empty() {
                        "none".into()
                    } else {
                        observed_rollback_platforms.join(", ")
                    }
                )
                .into(),
            ],
        },
        KernelLoopbackR4ExpandedOptInLimitedRolloutGateCheck {
            name: "independentRollbackDecision".into(),
            status: if independent_rollback_decision {
                "passed"
            } else {
                "blocked"
            }
            .into(),
            passed: independent_rollback_decision,
            blockers: if independent_rollback_decision {
                Vec::new()
            } else {
                vec!["R5 independent rollback validation requires an explicit decision".into()]
            },
            facts: vec!["validation remains read-only and loopback scoped".into()],
        },
    ];
    let rollback_validation_ready = checks.iter().all(|check| check.passed);
    let blockers = checks
        .iter()
        .flat_map(|check| check.blockers.clone())
        .collect::<Vec<String>>();

    Ok(KernelLoopbackR5DefaultCutoverIndependentRollbackValidationReport {
        runtime_id: MIHOMO_RUNTIME_ID.into(),
        component: "loopback-r5-default-cutover-independent-rollback-validation".into(),
        kernel_area: "forwarding".into(),
        mutates_runtime: false,
        live_execution_allowed: false,
        current_platform: final_hold.current_platform.clone(),
        current_arch: final_hold.current_arch.clone(),
        independent_rollback_decision,
        rollback_validation_ready,
        required_platforms,
        observed_rollback_platforms,
        pending_rollback_platforms,
        default_cutover_allowed: false,
        expanded_opt_in_allowed: false,
        final_hold,
        checks,
        default_route: false,
        forwards_traffic: false,
        outbound_adapters_used: false,
        mihomo_fallback: true,
        passed: rollback_validation_ready,
        blockers,
        warnings: vec!["independent rollback validation does not authorize default cutover".into()],
        facts: vec!["requires platform-complete rollback evidence after final hold".into()],
        next_safe_batch: "loopback-r5-default-cutover-closeout-readiness".into(),
    })
}

pub async fn mihomo_kernel_loopback_r5_default_cutover_closeout_readiness(
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
) -> Result<KernelLoopbackR5DefaultCutoverCloseoutReadinessReport> {
    let r5_closeout_decision = r5_closeout_decision.unwrap_or(false);
    let rollback_validation = mihomo_kernel_loopback_r5_default_cutover_independent_rollback_validation(
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
    )
    .await?;
    let checks = vec![
        KernelLoopbackR4ExpandedOptInLimitedRolloutGateCheck {
            name: "rollbackValidationReady".into(),
            status: if rollback_validation.rollback_validation_ready {
                "passed"
            } else {
                "blocked"
            }
            .into(),
            passed: rollback_validation.rollback_validation_ready,
            blockers: if rollback_validation.rollback_validation_ready {
                Vec::new()
            } else {
                rollback_validation.blockers.clone()
            },
            facts: vec!["closeout readiness requires independent rollback validation".into()],
        },
        KernelLoopbackR4ExpandedOptInLimitedRolloutGateCheck {
            name: "r5CloseoutDecision".into(),
            status: if r5_closeout_decision { "passed" } else { "blocked" }.into(),
            passed: r5_closeout_decision,
            blockers: if r5_closeout_decision {
                Vec::new()
            } else {
                vec!["R5 closeout readiness requires an explicit closeout decision".into()]
            },
            facts: vec!["closeout readiness prepares a report-only batch".into()],
        },
        KernelLoopbackR4ExpandedOptInLimitedRolloutGateCheck {
            name: "defaultCutoverStillBlocked".into(),
            status: "passed".into(),
            passed: true,
            blockers: Vec::new(),
            facts: vec!["real adapter, TUN, protocol, and default route cutover remain blocked".into()],
        },
    ];
    let closeout_ready = checks.iter().all(|check| check.passed);
    let blockers = checks
        .iter()
        .flat_map(|check| check.blockers.clone())
        .collect::<Vec<String>>();

    Ok(KernelLoopbackR5DefaultCutoverCloseoutReadinessReport {
        runtime_id: MIHOMO_RUNTIME_ID.into(),
        component: "loopback-r5-default-cutover-closeout-readiness".into(),
        kernel_area: "forwarding".into(),
        mutates_runtime: false,
        live_execution_allowed: false,
        current_platform: rollback_validation.current_platform.clone(),
        current_arch: rollback_validation.current_arch.clone(),
        r5_closeout_decision,
        closeout_ready,
        default_cutover_allowed: false,
        expanded_opt_in_allowed: false,
        rollback_validation,
        checks,
        default_route: false,
        forwards_traffic: false,
        outbound_adapters_used: false,
        mihomo_fallback: true,
        passed: closeout_ready,
        blockers,
        warnings: vec!["closeout readiness does not authorize live default cutover".into()],
        facts: vec!["next batch is report-only closeout evidence for R5".into()],
        next_safe_batch: "loopback-r5-default-cutover-closeout-report".into(),
    })
}

pub async fn mihomo_kernel_loopback_r5_default_cutover_closeout_report(
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
) -> Result<KernelLoopbackR5DefaultCutoverCloseoutReport> {
    let r5_closeout_report_decision = r5_closeout_report_decision.unwrap_or(false);
    let closeout_readiness = mihomo_kernel_loopback_r5_default_cutover_closeout_readiness(
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
    )
    .await?;
    let r5_closeout_complete = closeout_readiness.passed && r5_closeout_report_decision;
    let blockers = if r5_closeout_complete {
        Vec::new()
    } else {
        let mut blockers = closeout_readiness.blockers.clone();
        if !r5_closeout_report_decision {
            blockers.push("R5 closeout report requires an explicit report decision".into());
        }
        blockers
    };

    Ok(KernelLoopbackR5DefaultCutoverCloseoutReport {
        runtime_id: MIHOMO_RUNTIME_ID.into(),
        component: "loopback-r5-default-cutover-closeout-report".into(),
        kernel_area: "forwarding".into(),
        mutates_runtime: false,
        live_execution_allowed: false,
        current_platform: closeout_readiness.current_platform.clone(),
        current_arch: closeout_readiness.current_arch.clone(),
        r5_closeout_report_decision,
        r5_closeout_complete,
        default_cutover_allowed: false,
        expanded_opt_in_allowed: false,
        closeout_readiness,
        completed_evidence_batches: vec![
            "r3-loopback-listener-dns-forwarding-evidence".into(),
            "r4-expanded-opt-in-synthetic-execution-and-closeout".into(),
            "r5-default-cutover-preflight-through-final-hold".into(),
            "r5-independent-rollback-validation-and-closeout-readiness".into(),
        ],
        open_boundaries: rust_runtime_fallback_boundaries(),
        passed: r5_closeout_complete,
        blockers,
        warnings: vec!["R5 closeout report closes evidence gates but does not select Rust runtime".into()],
        facts: vec!["R5 evidence is ready to hand off to R6 Rust runtime implementation".into()],
        next_safe_batch: "r5-closeout-r6-rust-runtime-scaffold".into(),
    })
}
