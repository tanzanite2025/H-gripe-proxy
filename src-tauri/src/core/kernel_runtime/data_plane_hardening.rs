use anyhow::Result;
use smartstring::alias::String;

use super::{
    KernelLoopbackR4ExpandedOptInLimitedRolloutGateCheck, KernelLoopbackRustDataPlaneHardeningBoundaryAuditReport,
    KernelLoopbackRustDataPlaneHardeningControlledRolloutDryRunReport,
    KernelLoopbackRustDataPlaneHardeningControlledRolloutGuardReport,
    KernelLoopbackRustDataPlaneHardeningControlledRolloutReadinessCloseoutReport,
    KernelLoopbackRustDataPlaneHardeningOptInDryRunReport,
    KernelLoopbackRustDataPlaneHardeningOptInExecutionGuardReport,
    KernelLoopbackRustDataPlaneHardeningOptInExecutionReport,
    KernelLoopbackRustDataPlaneHardeningOptInExecutionVerificationReport,
    KernelLoopbackRustDataPlaneHardeningPreflightReport, KernelRuntimeKind, RUST_RUNTIME_ID,
    RustKernelRuntimeDataPlaneHardeningBoundaryAuditReport, RustKernelRuntimeDataPlaneHardeningBoundaryReport,
    RustKernelRuntimeDataPlaneHardeningControlledRolloutDryRunReport,
    RustKernelRuntimeDataPlaneHardeningControlledRolloutGuardReport,
    RustKernelRuntimeDataPlaneHardeningControlledRolloutReadinessCloseoutReport,
    RustKernelRuntimeDataPlaneHardeningOptInDryRunReport, RustKernelRuntimeDataPlaneHardeningOptInExecutionGuardReport,
    RustKernelRuntimeDataPlaneHardeningOptInExecutionReport,
    RustKernelRuntimeDataPlaneHardeningOptInExecutionVerificationReport,
};

fn rust_kernel_runtime_data_plane_hardening_boundary_report(
    protocol_parity_inventory_decision: bool,
    tun_boundary_inventory_decision: bool,
    adapter_compatibility_matrix_decision: bool,
    dns_leak_verification_plan_decision: bool,
    rollback_drill_plan_decision: bool,
    opt_in_execution_boundary_decision: bool,
) -> RustKernelRuntimeDataPlaneHardeningBoundaryReport {
    let mut blockers = Vec::new();
    let mut evidence_surfaces = Vec::new();

    if protocol_parity_inventory_decision {
        evidence_surfaces.push("protocol parity inventory".into());
    } else {
        blockers.push("Rust data-plane hardening requires protocol parity inventory".into());
    }
    if tun_boundary_inventory_decision {
        evidence_surfaces.push("TUN boundary inventory".into());
    } else {
        blockers.push("Rust data-plane hardening requires TUN boundary inventory".into());
    }
    if adapter_compatibility_matrix_decision {
        evidence_surfaces.push("adapter compatibility matrix".into());
    } else {
        blockers.push("Rust data-plane hardening requires adapter compatibility matrix".into());
    }
    if dns_leak_verification_plan_decision {
        evidence_surfaces.push("DNS leak verification plan".into());
    } else {
        blockers.push("Rust data-plane hardening requires DNS leak verification plan".into());
    }
    if rollback_drill_plan_decision {
        evidence_surfaces.push("platform rollback drill plan".into());
    } else {
        blockers.push("Rust data-plane hardening requires platform rollback drill plan".into());
    }
    if opt_in_execution_boundary_decision {
        evidence_surfaces.push("opt-in execution boundary".into());
    } else {
        blockers.push("Rust data-plane hardening requires a locked opt-in execution boundary".into());
    }

    RustKernelRuntimeDataPlaneHardeningBoundaryReport {
        runtime_id: RUST_RUNTIME_ID.into(),
        component: "rust-data-plane-hardening-preflight-boundary".into(),
        protocol_parity_inventory_complete: protocol_parity_inventory_decision,
        tun_boundary_inventory_complete: tun_boundary_inventory_decision,
        adapter_compatibility_matrix_complete: adapter_compatibility_matrix_decision,
        dns_leak_verification_plan_complete: dns_leak_verification_plan_decision,
        rollback_drill_plan_complete: rollback_drill_plan_decision,
        opt_in_execution_boundary_locked: opt_in_execution_boundary_decision,
        preflight_boundary_complete: blockers.is_empty(),
        evidence_surfaces,
        blockers,
        facts: vec![
            "preflight records data-plane hardening boundaries before any forwarding mutation".into(),
            "Mihomo rollback remains the production safety boundary during this phase".into(),
        ],
    }
}

pub async fn rust_kernel_runtime_data_plane_hardening_preflight(
    go_mihomo_retirement_complete_decision: Option<bool>,
    protocol_parity_inventory_decision: Option<bool>,
    tun_boundary_inventory_decision: Option<bool>,
    adapter_compatibility_matrix_decision: Option<bool>,
    dns_leak_verification_plan_decision: Option<bool>,
    rollback_drill_plan_decision: Option<bool>,
    opt_in_execution_boundary_decision: Option<bool>,
    final_preflight_decision: Option<bool>,
) -> Result<KernelLoopbackRustDataPlaneHardeningPreflightReport> {
    let go_mihomo_retirement_complete = go_mihomo_retirement_complete_decision.unwrap_or(false);
    let final_preflight_decision = final_preflight_decision.unwrap_or(false);
    let boundary = rust_kernel_runtime_data_plane_hardening_boundary_report(
        protocol_parity_inventory_decision.unwrap_or(false),
        tun_boundary_inventory_decision.unwrap_or(false),
        adapter_compatibility_matrix_decision.unwrap_or(false),
        dns_leak_verification_plan_decision.unwrap_or(false),
        rollback_drill_plan_decision.unwrap_or(false),
        opt_in_execution_boundary_decision.unwrap_or(false),
    );
    let mut retirement_blockers = Vec::new();

    if !go_mihomo_retirement_complete {
        retirement_blockers.push("Rust data-plane hardening preflight requires Go/Mihomo retirement closeout".into());
    }

    let checks = vec![
        KernelLoopbackR4ExpandedOptInLimitedRolloutGateCheck {
            name: "goMihomoRetirementComplete".into(),
            status: if go_mihomo_retirement_complete {
                "passed"
            } else {
                "blocked"
            }
            .into(),
            passed: go_mihomo_retirement_complete,
            blockers: retirement_blockers,
            facts: vec!["high-risk hardening starts only after Go/Mihomo retirement closeout".into()],
        },
        KernelLoopbackR4ExpandedOptInLimitedRolloutGateCheck {
            name: "rustDataPlaneHardeningBoundaryComplete".into(),
            status: if boundary.preflight_boundary_complete {
                "passed"
            } else {
                "blocked"
            }
            .into(),
            passed: boundary.preflight_boundary_complete,
            blockers: boundary.blockers.clone(),
            facts: vec![
                "protocol, TUN, adapter, DNS leak, rollback, and opt-in boundaries are evaluated together".into(),
            ],
        },
        KernelLoopbackR4ExpandedOptInLimitedRolloutGateCheck {
            name: "finalPreflightDecision".into(),
            status: if final_preflight_decision { "passed" } else { "blocked" }.into(),
            passed: final_preflight_decision,
            blockers: if final_preflight_decision {
                Vec::new()
            } else {
                vec!["Rust data-plane hardening preflight requires an explicit final decision".into()]
            },
            facts: vec!["preflight completion is explicit before any later opt-in execution gate".into()],
        },
    ];
    let rust_data_plane_hardening_preflight_complete = checks.iter().all(|check| check.passed);
    let blockers = checks
        .iter()
        .flat_map(|check| check.blockers.clone())
        .collect::<Vec<String>>();

    Ok(KernelLoopbackRustDataPlaneHardeningPreflightReport {
        runtime_id: RUST_RUNTIME_ID.into(),
        component: "rust-data-plane-hardening-preflight".into(),
        mutates_runtime: false,
        live_execution_allowed: false,
        production_data_plane_mutation_allowed: false,
        go_mihomo_retirement_complete,
        boundary,
        final_preflight_decision,
        rust_data_plane_hardening_preflight_complete,
        selected_runtime_kind: if rust_data_plane_hardening_preflight_complete {
            KernelRuntimeKind::Rust
        } else {
            KernelRuntimeKind::Mihomo
        },
        rollback_runtime_kind: KernelRuntimeKind::Mihomo,
        checks,
        blockers,
        warnings: vec![
            "this preflight does not mutate runtime, routes, TUN, DNS, adapter forwarding, or Mihomo config".into(),
            "production data-plane mutation remains blocked until a separate opt-in execution gate".into(),
        ],
        facts: vec![
            "Rust data-plane hardening follows the completed Go/Mihomo retirement sequence".into(),
            "successful preflight advances only to a boundary audit batch".into(),
        ],
        next_safe_batch: if rust_data_plane_hardening_preflight_complete {
            "rust-data-plane-hardening-boundary-audit".into()
        } else {
            "rust-data-plane-hardening-preflight".into()
        },
    })
}

fn rust_kernel_runtime_data_plane_hardening_boundary_audit_report(
    preflight_review_decision: bool,
    protocol_boundary_audit_decision: bool,
    tun_boundary_audit_decision: bool,
    adapter_boundary_audit_decision: bool,
    dns_leak_boundary_audit_decision: bool,
    rollback_boundary_audit_decision: bool,
    opt_in_boundary_audit_decision: bool,
) -> RustKernelRuntimeDataPlaneHardeningBoundaryAuditReport {
    let mut blockers = Vec::new();
    let mut audited_surfaces = Vec::new();

    if preflight_review_decision {
        audited_surfaces.push("preflight review".into());
    } else {
        blockers.push("Rust data-plane boundary audit requires reviewed preflight evidence".into());
    }
    if protocol_boundary_audit_decision {
        audited_surfaces.push("protocol replacement boundary".into());
    } else {
        blockers.push("Rust data-plane boundary audit requires protocol boundary audit".into());
    }
    if tun_boundary_audit_decision {
        audited_surfaces.push("TUN replacement boundary".into());
    } else {
        blockers.push("Rust data-plane boundary audit requires TUN boundary audit".into());
    }
    if adapter_boundary_audit_decision {
        audited_surfaces.push("adapter forwarding boundary".into());
    } else {
        blockers.push("Rust data-plane boundary audit requires adapter boundary audit".into());
    }
    if dns_leak_boundary_audit_decision {
        audited_surfaces.push("DNS leak boundary".into());
    } else {
        blockers.push("Rust data-plane boundary audit requires DNS leak boundary audit".into());
    }
    if rollback_boundary_audit_decision {
        audited_surfaces.push("rollback drill boundary".into());
    } else {
        blockers.push("Rust data-plane boundary audit requires rollback boundary audit".into());
    }
    if opt_in_boundary_audit_decision {
        audited_surfaces.push("opt-in execution boundary".into());
    } else {
        blockers.push("Rust data-plane boundary audit requires opt-in boundary audit".into());
    }

    RustKernelRuntimeDataPlaneHardeningBoundaryAuditReport {
        runtime_id: RUST_RUNTIME_ID.into(),
        component: "rust-data-plane-hardening-boundary-audit-detail".into(),
        preflight_reviewed: preflight_review_decision,
        protocol_boundary_audited: protocol_boundary_audit_decision,
        tun_boundary_audited: tun_boundary_audit_decision,
        adapter_boundary_audited: adapter_boundary_audit_decision,
        dns_leak_boundary_audited: dns_leak_boundary_audit_decision,
        rollback_boundary_audited: rollback_boundary_audit_decision,
        opt_in_boundary_audited: opt_in_boundary_audit_decision,
        boundary_audit_complete: blockers.is_empty(),
        audited_surfaces,
        blockers,
        facts: vec![
            "boundary audit confirms which data-plane surfaces remain blocked before opt-in execution".into(),
            "protocol, TUN, adapter, DNS leak, rollback, and opt-in boundaries are audited together".into(),
        ],
    }
}

pub async fn rust_kernel_runtime_data_plane_hardening_boundary_audit(
    rust_data_plane_hardening_preflight_complete_decision: Option<bool>,
    preflight_review_decision: Option<bool>,
    protocol_boundary_audit_decision: Option<bool>,
    tun_boundary_audit_decision: Option<bool>,
    adapter_boundary_audit_decision: Option<bool>,
    dns_leak_boundary_audit_decision: Option<bool>,
    rollback_boundary_audit_decision: Option<bool>,
    opt_in_boundary_audit_decision: Option<bool>,
    final_boundary_audit_decision: Option<bool>,
) -> Result<KernelLoopbackRustDataPlaneHardeningBoundaryAuditReport> {
    let rust_data_plane_hardening_preflight_complete =
        rust_data_plane_hardening_preflight_complete_decision.unwrap_or(false);
    let final_boundary_audit_decision = final_boundary_audit_decision.unwrap_or(false);
    let boundary_audit = rust_kernel_runtime_data_plane_hardening_boundary_audit_report(
        preflight_review_decision.unwrap_or(false),
        protocol_boundary_audit_decision.unwrap_or(false),
        tun_boundary_audit_decision.unwrap_or(false),
        adapter_boundary_audit_decision.unwrap_or(false),
        dns_leak_boundary_audit_decision.unwrap_or(false),
        rollback_boundary_audit_decision.unwrap_or(false),
        opt_in_boundary_audit_decision.unwrap_or(false),
    );
    let mut preflight_blockers = Vec::new();

    if !rust_data_plane_hardening_preflight_complete {
        preflight_blockers.push("Rust data-plane boundary audit requires hardening preflight to pass first".into());
    }

    let checks = vec![
        KernelLoopbackR4ExpandedOptInLimitedRolloutGateCheck {
            name: "rustDataPlaneHardeningPreflightComplete".into(),
            status: if rust_data_plane_hardening_preflight_complete {
                "passed"
            } else {
                "blocked"
            }
            .into(),
            passed: rust_data_plane_hardening_preflight_complete,
            blockers: preflight_blockers,
            facts: vec!["boundary audit starts only after the hardening preflight gate".into()],
        },
        KernelLoopbackR4ExpandedOptInLimitedRolloutGateCheck {
            name: "rustDataPlaneHardeningBoundaryAuditComplete".into(),
            status: if boundary_audit.boundary_audit_complete {
                "passed"
            } else {
                "blocked"
            }
            .into(),
            passed: boundary_audit.boundary_audit_complete,
            blockers: boundary_audit.blockers.clone(),
            facts: vec![
                "preflight, protocol, TUN, adapter, DNS leak, rollback, and opt-in boundaries are audited together"
                    .into(),
            ],
        },
        KernelLoopbackR4ExpandedOptInLimitedRolloutGateCheck {
            name: "finalBoundaryAuditDecision".into(),
            status: if final_boundary_audit_decision {
                "passed"
            } else {
                "blocked"
            }
            .into(),
            passed: final_boundary_audit_decision,
            blockers: if final_boundary_audit_decision {
                Vec::new()
            } else {
                vec!["Rust data-plane boundary audit requires an explicit final decision".into()]
            },
            facts: vec!["boundary audit completion is explicit before opt-in execution guard planning".into()],
        },
    ];
    let rust_data_plane_hardening_boundary_audit_complete = checks.iter().all(|check| check.passed);
    let blockers = checks
        .iter()
        .flat_map(|check| check.blockers.clone())
        .collect::<Vec<String>>();

    Ok(KernelLoopbackRustDataPlaneHardeningBoundaryAuditReport {
        runtime_id: RUST_RUNTIME_ID.into(),
        component: "rust-data-plane-hardening-boundary-audit".into(),
        mutates_runtime: false,
        live_execution_allowed: false,
        production_data_plane_mutation_allowed: false,
        rust_data_plane_hardening_preflight_complete,
        boundary_audit,
        final_boundary_audit_decision,
        rust_data_plane_hardening_boundary_audit_complete,
        selected_runtime_kind: if rust_data_plane_hardening_boundary_audit_complete {
            KernelRuntimeKind::Rust
        } else {
            KernelRuntimeKind::Mihomo
        },
        rollback_runtime_kind: KernelRuntimeKind::Mihomo,
        checks,
        blockers,
        warnings: vec![
            "this boundary audit does not mutate runtime, routes, TUN, DNS, adapter forwarding, or Mihomo config".into(),
            "production data-plane mutation remains blocked until a separate opt-in execution guard and execution batch".into(),
        ],
        facts: vec![
            "Rust data-plane hardening boundary audit follows the preflight gate".into(),
            "successful audit advances only to opt-in execution guard planning".into(),
        ],
        next_safe_batch: if rust_data_plane_hardening_boundary_audit_complete {
            "rust-data-plane-hardening-opt-in-execution-guard".into()
        } else {
            "rust-data-plane-hardening-boundary-audit".into()
        },
    })
}

fn rust_kernel_runtime_data_plane_hardening_opt_in_execution_guard_report(
    boundary_audit_review_decision: bool,
    opt_in_scope_lock_decision: bool,
    rollout_guard_definition_decision: bool,
    abort_plan_approval_decision: bool,
    telemetry_watch_configuration_decision: bool,
    rollback_switch_verification_decision: bool,
    operator_acknowledgement_decision: bool,
) -> RustKernelRuntimeDataPlaneHardeningOptInExecutionGuardReport {
    let mut blockers = Vec::new();
    let mut guarded_surfaces = Vec::new();

    if boundary_audit_review_decision {
        guarded_surfaces.push("boundary audit review".into());
    } else {
        blockers.push("Rust data-plane opt-in execution guard requires reviewed boundary audit".into());
    }
    if opt_in_scope_lock_decision {
        guarded_surfaces.push("locked opt-in execution scope".into());
    } else {
        blockers.push("Rust data-plane opt-in execution guard requires locked opt-in scope".into());
    }
    if rollout_guard_definition_decision {
        guarded_surfaces.push("rollout guard definition".into());
    } else {
        blockers.push("Rust data-plane opt-in execution guard requires rollout guard definition".into());
    }
    if abort_plan_approval_decision {
        guarded_surfaces.push("approved abort plan".into());
    } else {
        blockers.push("Rust data-plane opt-in execution guard requires approved abort plan".into());
    }
    if telemetry_watch_configuration_decision {
        guarded_surfaces.push("telemetry watch configuration".into());
    } else {
        blockers.push("Rust data-plane opt-in execution guard requires telemetry watch configuration".into());
    }
    if rollback_switch_verification_decision {
        guarded_surfaces.push("rollback switch verification".into());
    } else {
        blockers.push("Rust data-plane opt-in execution guard requires rollback switch verification".into());
    }
    if operator_acknowledgement_decision {
        guarded_surfaces.push("operator acknowledgement".into());
    } else {
        blockers.push("Rust data-plane opt-in execution guard requires operator acknowledgement".into());
    }

    RustKernelRuntimeDataPlaneHardeningOptInExecutionGuardReport {
        runtime_id: RUST_RUNTIME_ID.into(),
        component: "rust-data-plane-hardening-opt-in-execution-guard-detail".into(),
        boundary_audit_reviewed: boundary_audit_review_decision,
        opt_in_scope_locked: opt_in_scope_lock_decision,
        rollout_guard_defined: rollout_guard_definition_decision,
        abort_plan_approved: abort_plan_approval_decision,
        telemetry_watch_configured: telemetry_watch_configuration_decision,
        rollback_switch_verified: rollback_switch_verification_decision,
        operator_acknowledged: operator_acknowledgement_decision,
        opt_in_execution_guard_complete: blockers.is_empty(),
        guarded_surfaces,
        blockers,
        facts: vec![
            "opt-in execution guard defines the allowed execution envelope without applying it".into(),
            "production data-plane forwarding remains blocked until a separate execution batch".into(),
        ],
    }
}

pub async fn rust_kernel_runtime_data_plane_hardening_opt_in_execution_guard(
    rust_data_plane_hardening_boundary_audit_complete_decision: Option<bool>,
    boundary_audit_review_decision: Option<bool>,
    opt_in_scope_lock_decision: Option<bool>,
    rollout_guard_definition_decision: Option<bool>,
    abort_plan_approval_decision: Option<bool>,
    telemetry_watch_configuration_decision: Option<bool>,
    rollback_switch_verification_decision: Option<bool>,
    operator_acknowledgement_decision: Option<bool>,
    final_execution_guard_decision: Option<bool>,
) -> Result<KernelLoopbackRustDataPlaneHardeningOptInExecutionGuardReport> {
    let rust_data_plane_hardening_boundary_audit_complete =
        rust_data_plane_hardening_boundary_audit_complete_decision.unwrap_or(false);
    let final_execution_guard_decision = final_execution_guard_decision.unwrap_or(false);
    let opt_in_execution_guard = rust_kernel_runtime_data_plane_hardening_opt_in_execution_guard_report(
        boundary_audit_review_decision.unwrap_or(false),
        opt_in_scope_lock_decision.unwrap_or(false),
        rollout_guard_definition_decision.unwrap_or(false),
        abort_plan_approval_decision.unwrap_or(false),
        telemetry_watch_configuration_decision.unwrap_or(false),
        rollback_switch_verification_decision.unwrap_or(false),
        operator_acknowledgement_decision.unwrap_or(false),
    );
    let mut boundary_audit_blockers = Vec::new();

    if !rust_data_plane_hardening_boundary_audit_complete {
        boundary_audit_blockers
            .push("Rust data-plane opt-in execution guard requires boundary audit to pass first".into());
    }

    let checks = vec![
        KernelLoopbackR4ExpandedOptInLimitedRolloutGateCheck {
            name: "rustDataPlaneHardeningBoundaryAuditComplete".into(),
            status: if rust_data_plane_hardening_boundary_audit_complete {
                "passed"
            } else {
                "blocked"
            }
            .into(),
            passed: rust_data_plane_hardening_boundary_audit_complete,
            blockers: boundary_audit_blockers,
            facts: vec!["opt-in execution guard starts only after boundary audit".into()],
        },
        KernelLoopbackR4ExpandedOptInLimitedRolloutGateCheck {
            name: "rustDataPlaneHardeningOptInExecutionGuardComplete".into(),
            status: if opt_in_execution_guard.opt_in_execution_guard_complete {
                "passed"
            } else {
                "blocked"
            }
            .into(),
            passed: opt_in_execution_guard.opt_in_execution_guard_complete,
            blockers: opt_in_execution_guard.blockers.clone(),
            facts: vec![
                "boundary review, scope lock, rollout guard, abort plan, telemetry, rollback, and acknowledgement are evaluated together".into(),
            ],
        },
        KernelLoopbackR4ExpandedOptInLimitedRolloutGateCheck {
            name: "finalExecutionGuardDecision".into(),
            status: if final_execution_guard_decision {
                "passed"
            } else {
                "blocked"
            }
            .into(),
            passed: final_execution_guard_decision,
            blockers: if final_execution_guard_decision {
                Vec::new()
            } else {
                vec!["Rust data-plane opt-in execution guard requires an explicit final decision".into()]
            },
            facts: vec!["execution guard completion is explicit before any opt-in dry run".into()],
        },
    ];
    let rust_data_plane_hardening_opt_in_execution_guard_complete = checks.iter().all(|check| check.passed);
    let blockers = checks
        .iter()
        .flat_map(|check| check.blockers.clone())
        .collect::<Vec<String>>();

    Ok(KernelLoopbackRustDataPlaneHardeningOptInExecutionGuardReport {
        runtime_id: RUST_RUNTIME_ID.into(),
        component: "rust-data-plane-hardening-opt-in-execution-guard".into(),
        mutates_runtime: false,
        live_execution_allowed: false,
        production_data_plane_mutation_allowed: false,
        rust_data_plane_hardening_boundary_audit_complete,
        opt_in_execution_guard,
        final_execution_guard_decision,
        rust_data_plane_hardening_opt_in_execution_guard_complete,
        selected_runtime_kind: if rust_data_plane_hardening_opt_in_execution_guard_complete {
            KernelRuntimeKind::Rust
        } else {
            KernelRuntimeKind::Mihomo
        },
        rollback_runtime_kind: KernelRuntimeKind::Mihomo,
        checks,
        blockers,
        warnings: vec![
            "this execution guard does not mutate runtime, routes, TUN, DNS, adapter forwarding, or Mihomo config"
                .into(),
            "production data-plane mutation remains blocked until a separate opt-in dry-run or execution batch".into(),
        ],
        facts: vec![
            "Rust data-plane hardening opt-in execution guard follows boundary audit".into(),
            "successful guard advances only to opt-in dry-run readiness".into(),
        ],
        next_safe_batch: if rust_data_plane_hardening_opt_in_execution_guard_complete {
            "rust-data-plane-hardening-opt-in-dry-run".into()
        } else {
            "rust-data-plane-hardening-opt-in-execution-guard".into()
        },
    })
}

fn rust_kernel_runtime_data_plane_hardening_opt_in_dry_run_report(
    execution_guard_review_decision: bool,
    dry_run_scope_lock_decision: bool,
    manifest_replay_decision: bool,
    synthetic_flow_plan_decision: bool,
    leak_watch_plan_verification_decision: bool,
    rollback_rehearsal_decision: bool,
    production_forwarding_unchanged_verification_decision: bool,
    dry_run_evidence_archive_decision: bool,
) -> RustKernelRuntimeDataPlaneHardeningOptInDryRunReport {
    let mut blockers = Vec::new();
    let mut dry_run_surfaces = Vec::new();

    if execution_guard_review_decision {
        dry_run_surfaces.push("execution guard review".into());
    } else {
        blockers.push("Rust data-plane opt-in dry-run requires reviewed execution guard".into());
    }
    if dry_run_scope_lock_decision {
        dry_run_surfaces.push("locked dry-run scope".into());
    } else {
        blockers.push("Rust data-plane opt-in dry-run requires locked dry-run scope".into());
    }
    if manifest_replay_decision {
        dry_run_surfaces.push("manifest replay".into());
    } else {
        blockers.push("Rust data-plane opt-in dry-run requires manifest replay".into());
    }
    if synthetic_flow_plan_decision {
        dry_run_surfaces.push("synthetic flow plan".into());
    } else {
        blockers.push("Rust data-plane opt-in dry-run requires synthetic flow plan".into());
    }
    if leak_watch_plan_verification_decision {
        dry_run_surfaces.push("leak watch plan".into());
    } else {
        blockers.push("Rust data-plane opt-in dry-run requires leak watch plan verification".into());
    }
    if rollback_rehearsal_decision {
        dry_run_surfaces.push("rollback rehearsal".into());
    } else {
        blockers.push("Rust data-plane opt-in dry-run requires rollback rehearsal".into());
    }
    if production_forwarding_unchanged_verification_decision {
        dry_run_surfaces.push("production forwarding unchanged verification".into());
    } else {
        blockers.push("Rust data-plane opt-in dry-run requires production forwarding unchanged verification".into());
    }
    if dry_run_evidence_archive_decision {
        dry_run_surfaces.push("archived dry-run evidence".into());
    } else {
        blockers.push("Rust data-plane opt-in dry-run requires archived dry-run evidence".into());
    }

    RustKernelRuntimeDataPlaneHardeningOptInDryRunReport {
        runtime_id: RUST_RUNTIME_ID.into(),
        component: "rust-data-plane-hardening-opt-in-dry-run-detail".into(),
        execution_guard_reviewed: execution_guard_review_decision,
        dry_run_scope_locked: dry_run_scope_lock_decision,
        manifest_replay_completed: manifest_replay_decision,
        synthetic_flow_plan_completed: synthetic_flow_plan_decision,
        leak_watch_plan_verified: leak_watch_plan_verification_decision,
        rollback_rehearsal_completed: rollback_rehearsal_decision,
        production_forwarding_unchanged_verified: production_forwarding_unchanged_verification_decision,
        dry_run_evidence_archived: dry_run_evidence_archive_decision,
        opt_in_dry_run_complete: blockers.is_empty(),
        dry_run_surfaces,
        blockers,
        facts: vec![
            "opt-in dry-run replays the execution envelope without applying production forwarding".into(),
            "production data-plane mutation remains blocked after dry-run completion".into(),
        ],
    }
}

pub async fn rust_kernel_runtime_data_plane_hardening_opt_in_dry_run(
    rust_data_plane_hardening_opt_in_execution_guard_complete_decision: Option<bool>,
    execution_guard_review_decision: Option<bool>,
    dry_run_scope_lock_decision: Option<bool>,
    manifest_replay_decision: Option<bool>,
    synthetic_flow_plan_decision: Option<bool>,
    leak_watch_plan_verification_decision: Option<bool>,
    rollback_rehearsal_decision: Option<bool>,
    production_forwarding_unchanged_verification_decision: Option<bool>,
    dry_run_evidence_archive_decision: Option<bool>,
    final_dry_run_decision: Option<bool>,
) -> Result<KernelLoopbackRustDataPlaneHardeningOptInDryRunReport> {
    let rust_data_plane_hardening_opt_in_execution_guard_complete =
        rust_data_plane_hardening_opt_in_execution_guard_complete_decision.unwrap_or(false);
    let final_dry_run_decision = final_dry_run_decision.unwrap_or(false);
    let opt_in_dry_run = rust_kernel_runtime_data_plane_hardening_opt_in_dry_run_report(
        execution_guard_review_decision.unwrap_or(false),
        dry_run_scope_lock_decision.unwrap_or(false),
        manifest_replay_decision.unwrap_or(false),
        synthetic_flow_plan_decision.unwrap_or(false),
        leak_watch_plan_verification_decision.unwrap_or(false),
        rollback_rehearsal_decision.unwrap_or(false),
        production_forwarding_unchanged_verification_decision.unwrap_or(false),
        dry_run_evidence_archive_decision.unwrap_or(false),
    );
    let mut execution_guard_blockers = Vec::new();

    if !rust_data_plane_hardening_opt_in_execution_guard_complete {
        execution_guard_blockers.push("Rust data-plane opt-in dry-run requires execution guard to pass first".into());
    }

    let checks = vec![
        KernelLoopbackR4ExpandedOptInLimitedRolloutGateCheck {
            name: "rustDataPlaneHardeningOptInExecutionGuardComplete".into(),
            status: if rust_data_plane_hardening_opt_in_execution_guard_complete {
                "passed"
            } else {
                "blocked"
            }
            .into(),
            passed: rust_data_plane_hardening_opt_in_execution_guard_complete,
            blockers: execution_guard_blockers,
            facts: vec!["opt-in dry-run starts only after the execution guard".into()],
        },
        KernelLoopbackR4ExpandedOptInLimitedRolloutGateCheck {
            name: "rustDataPlaneHardeningOptInDryRunComplete".into(),
            status: if opt_in_dry_run.opt_in_dry_run_complete {
                "passed"
            } else {
                "blocked"
            }
            .into(),
            passed: opt_in_dry_run.opt_in_dry_run_complete,
            blockers: opt_in_dry_run.blockers.clone(),
            facts: vec![
                "guard review, dry-run scope, manifest replay, synthetic flow plan, leak watch, rollback, unchanged production forwarding, and evidence archival are evaluated together".into(),
            ],
        },
        KernelLoopbackR4ExpandedOptInLimitedRolloutGateCheck {
            name: "finalDryRunDecision".into(),
            status: if final_dry_run_decision {
                "passed"
            } else {
                "blocked"
            }
            .into(),
            passed: final_dry_run_decision,
            blockers: if final_dry_run_decision {
                Vec::new()
            } else {
                vec!["Rust data-plane opt-in dry-run requires an explicit final decision".into()]
            },
            facts: vec!["dry-run completion is explicit before any opt-in execution batch".into()],
        },
    ];
    let rust_data_plane_hardening_opt_in_dry_run_complete = checks.iter().all(|check| check.passed);
    let blockers = checks
        .iter()
        .flat_map(|check| check.blockers.clone())
        .collect::<Vec<String>>();

    Ok(KernelLoopbackRustDataPlaneHardeningOptInDryRunReport {
        runtime_id: RUST_RUNTIME_ID.into(),
        component: "rust-data-plane-hardening-opt-in-dry-run".into(),
        mutates_runtime: false,
        live_execution_allowed: false,
        production_data_plane_mutation_allowed: false,
        rust_data_plane_hardening_opt_in_execution_guard_complete,
        opt_in_dry_run,
        final_dry_run_decision,
        rust_data_plane_hardening_opt_in_dry_run_complete,
        selected_runtime_kind: if rust_data_plane_hardening_opt_in_dry_run_complete {
            KernelRuntimeKind::Rust
        } else {
            KernelRuntimeKind::Mihomo
        },
        rollback_runtime_kind: KernelRuntimeKind::Mihomo,
        checks,
        blockers,
        warnings: vec![
            "this dry-run does not mutate runtime, routes, TUN, DNS, adapter forwarding, or Mihomo config".into(),
            "production data-plane mutation remains blocked until a separate opt-in execution batch".into(),
        ],
        facts: vec![
            "Rust data-plane hardening opt-in dry-run follows the execution guard".into(),
            "successful dry-run advances only to opt-in execution planning".into(),
        ],
        next_safe_batch: if rust_data_plane_hardening_opt_in_dry_run_complete {
            "rust-data-plane-hardening-opt-in-execution".into()
        } else {
            "rust-data-plane-hardening-opt-in-dry-run".into()
        },
    })
}

fn rust_kernel_runtime_data_plane_hardening_opt_in_execution_report(
    dry_run_review_decision: bool,
    execution_manifest_lock_decision: bool,
    staged_opt_in_window_decision: bool,
    telemetry_watch_activation_decision: bool,
    rollback_switch_arm_decision: bool,
    production_mutation_guard_retention_decision: bool,
    operator_execution_acknowledgement_decision: bool,
) -> RustKernelRuntimeDataPlaneHardeningOptInExecutionReport {
    let mut blockers = Vec::new();
    let mut execution_surfaces = Vec::new();

    if dry_run_review_decision {
        execution_surfaces.push("dry-run review".into());
    } else {
        blockers.push("Rust data-plane opt-in execution requires reviewed dry-run evidence".into());
    }
    if execution_manifest_lock_decision {
        execution_surfaces.push("locked execution manifest".into());
    } else {
        blockers.push("Rust data-plane opt-in execution requires locked execution manifest".into());
    }
    if staged_opt_in_window_decision {
        execution_surfaces.push("staged opt-in window".into());
    } else {
        blockers.push("Rust data-plane opt-in execution requires staged opt-in window definition".into());
    }
    if telemetry_watch_activation_decision {
        execution_surfaces.push("active telemetry watch".into());
    } else {
        blockers.push("Rust data-plane opt-in execution requires active telemetry watch".into());
    }
    if rollback_switch_arm_decision {
        execution_surfaces.push("armed rollback switch".into());
    } else {
        blockers.push("Rust data-plane opt-in execution requires armed rollback switch".into());
    }
    if production_mutation_guard_retention_decision {
        execution_surfaces.push("retained production mutation guard".into());
    } else {
        blockers.push("Rust data-plane opt-in execution requires retained production mutation guard".into());
    }
    if operator_execution_acknowledgement_decision {
        execution_surfaces.push("operator execution acknowledgement".into());
    } else {
        blockers.push("Rust data-plane opt-in execution requires operator acknowledgement".into());
    }

    RustKernelRuntimeDataPlaneHardeningOptInExecutionReport {
        runtime_id: RUST_RUNTIME_ID.into(),
        component: "rust-data-plane-hardening-opt-in-execution-detail".into(),
        dry_run_reviewed: dry_run_review_decision,
        execution_manifest_locked: execution_manifest_lock_decision,
        staged_opt_in_window_defined: staged_opt_in_window_decision,
        telemetry_watch_active: telemetry_watch_activation_decision,
        rollback_switch_armed: rollback_switch_arm_decision,
        production_mutation_guard_retained: production_mutation_guard_retention_decision,
        operator_execution_acknowledged: operator_execution_acknowledgement_decision,
        opt_in_execution_complete: blockers.is_empty(),
        execution_surfaces,
        blockers,
        facts: vec![
            "opt-in execution gate records the staged envelope without changing production forwarding".into(),
            "production data-plane mutation remains blocked by an explicit retained guard".into(),
        ],
    }
}

pub async fn rust_kernel_runtime_data_plane_hardening_opt_in_execution(
    rust_data_plane_hardening_opt_in_dry_run_complete_decision: Option<bool>,
    dry_run_review_decision: Option<bool>,
    execution_manifest_lock_decision: Option<bool>,
    staged_opt_in_window_decision: Option<bool>,
    telemetry_watch_activation_decision: Option<bool>,
    rollback_switch_arm_decision: Option<bool>,
    production_mutation_guard_retention_decision: Option<bool>,
    operator_execution_acknowledgement_decision: Option<bool>,
    final_execution_decision: Option<bool>,
) -> Result<KernelLoopbackRustDataPlaneHardeningOptInExecutionReport> {
    let rust_data_plane_hardening_opt_in_dry_run_complete =
        rust_data_plane_hardening_opt_in_dry_run_complete_decision.unwrap_or(false);
    let final_execution_decision = final_execution_decision.unwrap_or(false);
    let opt_in_execution = rust_kernel_runtime_data_plane_hardening_opt_in_execution_report(
        dry_run_review_decision.unwrap_or(false),
        execution_manifest_lock_decision.unwrap_or(false),
        staged_opt_in_window_decision.unwrap_or(false),
        telemetry_watch_activation_decision.unwrap_or(false),
        rollback_switch_arm_decision.unwrap_or(false),
        production_mutation_guard_retention_decision.unwrap_or(false),
        operator_execution_acknowledgement_decision.unwrap_or(false),
    );
    let mut dry_run_blockers = Vec::new();

    if !rust_data_plane_hardening_opt_in_dry_run_complete {
        dry_run_blockers.push("Rust data-plane opt-in execution requires dry-run to pass first".into());
    }

    let checks = vec![
        KernelLoopbackR4ExpandedOptInLimitedRolloutGateCheck {
            name: "rustDataPlaneHardeningOptInDryRunComplete".into(),
            status: if rust_data_plane_hardening_opt_in_dry_run_complete {
                "passed"
            } else {
                "blocked"
            }
            .into(),
            passed: rust_data_plane_hardening_opt_in_dry_run_complete,
            blockers: dry_run_blockers,
            facts: vec!["opt-in execution starts only after the dry-run gate".into()],
        },
        KernelLoopbackR4ExpandedOptInLimitedRolloutGateCheck {
            name: "rustDataPlaneHardeningOptInExecutionComplete".into(),
            status: if opt_in_execution.opt_in_execution_complete {
                "passed"
            } else {
                "blocked"
            }
            .into(),
            passed: opt_in_execution.opt_in_execution_complete,
            blockers: opt_in_execution.blockers.clone(),
            facts: vec![
                "dry-run review, locked manifest, staged window, telemetry, rollback, retained mutation guard, and operator acknowledgement are evaluated together".into(),
            ],
        },
        KernelLoopbackR4ExpandedOptInLimitedRolloutGateCheck {
            name: "finalExecutionDecision".into(),
            status: if final_execution_decision {
                "passed"
            } else {
                "blocked"
            }
            .into(),
            passed: final_execution_decision,
            blockers: if final_execution_decision {
                Vec::new()
            } else {
                vec!["Rust data-plane opt-in execution requires an explicit final decision".into()]
            },
            facts: vec!["opt-in execution completion is explicit before any verification batch".into()],
        },
    ];
    let rust_data_plane_hardening_opt_in_execution_complete = checks.iter().all(|check| check.passed);
    let blockers = checks
        .iter()
        .flat_map(|check| check.blockers.clone())
        .collect::<Vec<String>>();

    Ok(KernelLoopbackRustDataPlaneHardeningOptInExecutionReport {
        runtime_id: RUST_RUNTIME_ID.into(),
        component: "rust-data-plane-hardening-opt-in-execution".into(),
        mutates_runtime: false,
        live_execution_allowed: false,
        production_data_plane_mutation_allowed: false,
        rust_data_plane_hardening_opt_in_dry_run_complete,
        opt_in_execution,
        final_execution_decision,
        rust_data_plane_hardening_opt_in_execution_complete,
        selected_runtime_kind: if rust_data_plane_hardening_opt_in_execution_complete {
            KernelRuntimeKind::Rust
        } else {
            KernelRuntimeKind::Mihomo
        },
        rollback_runtime_kind: KernelRuntimeKind::Mihomo,
        checks,
        blockers,
        warnings: vec![
            "this opt-in execution gate does not mutate runtime, routes, TUN, DNS, adapter forwarding, or Mihomo config".into(),
            "production data-plane mutation remains blocked by the retained production mutation guard".into(),
        ],
        facts: vec![
            "Rust data-plane hardening opt-in execution follows the non-production dry-run".into(),
            "successful opt-in execution advances only to post-execution verification".into(),
        ],
        next_safe_batch: if rust_data_plane_hardening_opt_in_execution_complete {
            "rust-data-plane-hardening-opt-in-execution-verification".into()
        } else {
            "rust-data-plane-hardening-opt-in-execution".into()
        },
    })
}

fn rust_kernel_runtime_data_plane_hardening_opt_in_execution_verification_report(
    execution_record_review_decision: bool,
    telemetry_sample_review_decision: bool,
    rollback_readiness_verification_decision: bool,
    production_mutation_guard_retention_verification_decision: bool,
    production_forwarding_unchanged_verification_decision: bool,
    leak_regression_absence_verification_decision: bool,
    verification_evidence_archive_decision: bool,
) -> RustKernelRuntimeDataPlaneHardeningOptInExecutionVerificationReport {
    let mut blockers = Vec::new();
    let mut verification_surfaces = Vec::new();

    if execution_record_review_decision {
        verification_surfaces.push("execution record review".into());
    } else {
        blockers.push("Rust data-plane opt-in execution verification requires reviewed execution records".into());
    }
    if telemetry_sample_review_decision {
        verification_surfaces.push("telemetry sample review".into());
    } else {
        blockers.push("Rust data-plane opt-in execution verification requires reviewed telemetry samples".into());
    }
    if rollback_readiness_verification_decision {
        verification_surfaces.push("rollback readiness verification".into());
    } else {
        blockers.push("Rust data-plane opt-in execution verification requires rollback readiness verification".into());
    }
    if production_mutation_guard_retention_verification_decision {
        verification_surfaces.push("retained production mutation guard verification".into());
    } else {
        blockers.push(
            "Rust data-plane opt-in execution verification requires retained production mutation guard verification"
                .into(),
        );
    }
    if production_forwarding_unchanged_verification_decision {
        verification_surfaces.push("production forwarding unchanged verification".into());
    } else {
        blockers.push(
            "Rust data-plane opt-in execution verification requires production forwarding unchanged verification"
                .into(),
        );
    }
    if leak_regression_absence_verification_decision {
        verification_surfaces.push("leak regression absence verification".into());
    } else {
        blockers
            .push("Rust data-plane opt-in execution verification requires leak regression absence verification".into());
    }
    if verification_evidence_archive_decision {
        verification_surfaces.push("archived verification evidence".into());
    } else {
        blockers.push("Rust data-plane opt-in execution verification requires archived verification evidence".into());
    }

    RustKernelRuntimeDataPlaneHardeningOptInExecutionVerificationReport {
        runtime_id: RUST_RUNTIME_ID.into(),
        component: "rust-data-plane-hardening-opt-in-execution-verification-detail".into(),
        execution_record_reviewed: execution_record_review_decision,
        telemetry_sample_reviewed: telemetry_sample_review_decision,
        rollback_readiness_verified: rollback_readiness_verification_decision,
        production_mutation_guard_still_retained: production_mutation_guard_retention_verification_decision,
        production_forwarding_unchanged_verified: production_forwarding_unchanged_verification_decision,
        leak_regression_absence_verified: leak_regression_absence_verification_decision,
        verification_evidence_archived: verification_evidence_archive_decision,
        opt_in_execution_verification_complete: blockers.is_empty(),
        verification_surfaces,
        blockers,
        facts: vec![
            "opt-in execution verification reviews the recorded envelope without changing production forwarding".into(),
            "production data-plane mutation remains blocked after verification".into(),
        ],
    }
}

pub async fn rust_kernel_runtime_data_plane_hardening_opt_in_execution_verification(
    rust_data_plane_hardening_opt_in_execution_complete_decision: Option<bool>,
    execution_record_review_decision: Option<bool>,
    telemetry_sample_review_decision: Option<bool>,
    rollback_readiness_verification_decision: Option<bool>,
    production_mutation_guard_retention_verification_decision: Option<bool>,
    production_forwarding_unchanged_verification_decision: Option<bool>,
    leak_regression_absence_verification_decision: Option<bool>,
    verification_evidence_archive_decision: Option<bool>,
    final_verification_decision: Option<bool>,
) -> Result<KernelLoopbackRustDataPlaneHardeningOptInExecutionVerificationReport> {
    let rust_data_plane_hardening_opt_in_execution_complete =
        rust_data_plane_hardening_opt_in_execution_complete_decision.unwrap_or(false);
    let final_verification_decision = final_verification_decision.unwrap_or(false);
    let opt_in_execution_verification = rust_kernel_runtime_data_plane_hardening_opt_in_execution_verification_report(
        execution_record_review_decision.unwrap_or(false),
        telemetry_sample_review_decision.unwrap_or(false),
        rollback_readiness_verification_decision.unwrap_or(false),
        production_mutation_guard_retention_verification_decision.unwrap_or(false),
        production_forwarding_unchanged_verification_decision.unwrap_or(false),
        leak_regression_absence_verification_decision.unwrap_or(false),
        verification_evidence_archive_decision.unwrap_or(false),
    );
    let mut execution_blockers = Vec::new();

    if !rust_data_plane_hardening_opt_in_execution_complete {
        execution_blockers
            .push("Rust data-plane opt-in execution verification requires opt-in execution to pass first".into());
    }

    let checks = vec![
        KernelLoopbackR4ExpandedOptInLimitedRolloutGateCheck {
            name: "rustDataPlaneHardeningOptInExecutionComplete".into(),
            status: if rust_data_plane_hardening_opt_in_execution_complete {
                "passed"
            } else {
                "blocked"
            }
            .into(),
            passed: rust_data_plane_hardening_opt_in_execution_complete,
            blockers: execution_blockers,
            facts: vec!["opt-in execution verification starts only after the execution gate".into()],
        },
        KernelLoopbackR4ExpandedOptInLimitedRolloutGateCheck {
            name: "rustDataPlaneHardeningOptInExecutionVerificationComplete".into(),
            status: if opt_in_execution_verification.opt_in_execution_verification_complete {
                "passed"
            } else {
                "blocked"
            }
            .into(),
            passed: opt_in_execution_verification.opt_in_execution_verification_complete,
            blockers: opt_in_execution_verification.blockers.clone(),
            facts: vec![
                "execution records, telemetry samples, rollback readiness, mutation guard retention, unchanged forwarding, leak regression absence, and evidence archival are evaluated together".into(),
            ],
        },
        KernelLoopbackR4ExpandedOptInLimitedRolloutGateCheck {
            name: "finalVerificationDecision".into(),
            status: if final_verification_decision {
                "passed"
            } else {
                "blocked"
            }
            .into(),
            passed: final_verification_decision,
            blockers: if final_verification_decision {
                Vec::new()
            } else {
                vec!["Rust data-plane opt-in execution verification requires an explicit final decision".into()]
            },
            facts: vec!["verification completion is explicit before any controlled rollout guard".into()],
        },
    ];
    let rust_data_plane_hardening_opt_in_execution_verification_complete = checks.iter().all(|check| check.passed);
    let blockers = checks
        .iter()
        .flat_map(|check| check.blockers.clone())
        .collect::<Vec<String>>();

    Ok(KernelLoopbackRustDataPlaneHardeningOptInExecutionVerificationReport {
        runtime_id: RUST_RUNTIME_ID.into(),
        component: "rust-data-plane-hardening-opt-in-execution-verification".into(),
        mutates_runtime: false,
        live_execution_allowed: false,
        production_data_plane_mutation_allowed: false,
        rust_data_plane_hardening_opt_in_execution_complete,
        opt_in_execution_verification,
        final_verification_decision,
        rust_data_plane_hardening_opt_in_execution_verification_complete,
        selected_runtime_kind: if rust_data_plane_hardening_opt_in_execution_verification_complete {
            KernelRuntimeKind::Rust
        } else {
            KernelRuntimeKind::Mihomo
        },
        rollback_runtime_kind: KernelRuntimeKind::Mihomo,
        checks,
        blockers,
        warnings: vec![
            "this verification gate does not mutate runtime, routes, TUN, DNS, adapter forwarding, or Mihomo config"
                .into(),
            "production data-plane mutation remains blocked until a separate controlled rollout guard".into(),
        ],
        facts: vec![
            "Rust data-plane hardening opt-in execution verification follows the opt-in execution gate".into(),
            "successful verification advances only to controlled rollout guard planning".into(),
        ],
        next_safe_batch: if rust_data_plane_hardening_opt_in_execution_verification_complete {
            "rust-data-plane-hardening-controlled-rollout-guard".into()
        } else {
            "rust-data-plane-hardening-opt-in-execution-verification".into()
        },
    })
}

fn rust_kernel_runtime_data_plane_hardening_controlled_rollout_guard_report(
    opt_in_verification_review_decision: bool,
    controlled_rollout_scope_lock_decision: bool,
    canary_population_cap_definition_decision: bool,
    health_rollback_trigger_definition_decision: bool,
    telemetry_hold_window_configuration_decision: bool,
    mihomo_fallback_retention_decision: bool,
    production_mutation_guard_retention_decision: bool,
    operator_rollout_guard_acknowledgement_decision: bool,
) -> RustKernelRuntimeDataPlaneHardeningControlledRolloutGuardReport {
    let mut blockers = Vec::new();
    let mut guarded_surfaces = Vec::new();

    if opt_in_verification_review_decision {
        guarded_surfaces.push("opt-in execution verification review".into());
    } else {
        blockers.push("Rust data-plane controlled rollout guard requires reviewed opt-in verification".into());
    }
    if controlled_rollout_scope_lock_decision {
        guarded_surfaces.push("controlled rollout scope lock".into());
    } else {
        blockers.push("Rust data-plane controlled rollout guard requires a locked rollout scope".into());
    }
    if canary_population_cap_definition_decision {
        guarded_surfaces.push("canary population cap".into());
    } else {
        blockers.push("Rust data-plane controlled rollout guard requires a defined canary population cap".into());
    }
    if health_rollback_trigger_definition_decision {
        guarded_surfaces.push("health rollback triggers".into());
    } else {
        blockers.push("Rust data-plane controlled rollout guard requires health rollback triggers".into());
    }
    if telemetry_hold_window_configuration_decision {
        guarded_surfaces.push("telemetry hold window".into());
    } else {
        blockers.push("Rust data-plane controlled rollout guard requires a configured telemetry hold window".into());
    }
    if mihomo_fallback_retention_decision {
        guarded_surfaces.push("retained Mihomo fallback".into());
    } else {
        blockers.push("Rust data-plane controlled rollout guard requires retained Mihomo fallback".into());
    }
    if production_mutation_guard_retention_decision {
        guarded_surfaces.push("retained production mutation guard".into());
    } else {
        blockers.push("Rust data-plane controlled rollout guard requires retained production mutation guard".into());
    }
    if operator_rollout_guard_acknowledgement_decision {
        guarded_surfaces.push("operator rollout guard acknowledgement".into());
    } else {
        blockers.push("Rust data-plane controlled rollout guard requires operator acknowledgement".into());
    }

    RustKernelRuntimeDataPlaneHardeningControlledRolloutGuardReport {
        runtime_id: RUST_RUNTIME_ID.into(),
        component: "rust-data-plane-hardening-controlled-rollout-guard-detail".into(),
        opt_in_verification_reviewed: opt_in_verification_review_decision,
        controlled_rollout_scope_locked: controlled_rollout_scope_lock_decision,
        canary_population_cap_defined: canary_population_cap_definition_decision,
        health_rollback_triggers_defined: health_rollback_trigger_definition_decision,
        telemetry_hold_window_configured: telemetry_hold_window_configuration_decision,
        mihomo_fallback_retained: mihomo_fallback_retention_decision,
        production_mutation_guard_retained: production_mutation_guard_retention_decision,
        operator_rollout_guard_acknowledged: operator_rollout_guard_acknowledgement_decision,
        controlled_rollout_guard_complete: blockers.is_empty(),
        guarded_surfaces,
        blockers,
        facts: vec![
            "controlled rollout guard bundles rollout scope, canary cap, telemetry hold, fallback, and mutation guard decisions".into(),
            "the guard does not change production forwarding or remove Mihomo fallback".into(),
        ],
    }
}

pub async fn rust_kernel_runtime_data_plane_hardening_controlled_rollout_guard(
    rust_data_plane_hardening_opt_in_execution_verification_complete_decision: Option<bool>,
    opt_in_verification_review_decision: Option<bool>,
    controlled_rollout_scope_lock_decision: Option<bool>,
    canary_population_cap_definition_decision: Option<bool>,
    health_rollback_trigger_definition_decision: Option<bool>,
    telemetry_hold_window_configuration_decision: Option<bool>,
    mihomo_fallback_retention_decision: Option<bool>,
    production_mutation_guard_retention_decision: Option<bool>,
    operator_rollout_guard_acknowledgement_decision: Option<bool>,
    final_controlled_rollout_guard_decision: Option<bool>,
) -> Result<KernelLoopbackRustDataPlaneHardeningControlledRolloutGuardReport> {
    let rust_data_plane_hardening_opt_in_execution_verification_complete =
        rust_data_plane_hardening_opt_in_execution_verification_complete_decision.unwrap_or(false);
    let final_controlled_rollout_guard_decision = final_controlled_rollout_guard_decision.unwrap_or(false);
    let controlled_rollout_guard = rust_kernel_runtime_data_plane_hardening_controlled_rollout_guard_report(
        opt_in_verification_review_decision.unwrap_or(false),
        controlled_rollout_scope_lock_decision.unwrap_or(false),
        canary_population_cap_definition_decision.unwrap_or(false),
        health_rollback_trigger_definition_decision.unwrap_or(false),
        telemetry_hold_window_configuration_decision.unwrap_or(false),
        mihomo_fallback_retention_decision.unwrap_or(false),
        production_mutation_guard_retention_decision.unwrap_or(false),
        operator_rollout_guard_acknowledgement_decision.unwrap_or(false),
    );
    let mut verification_blockers = Vec::new();

    if !rust_data_plane_hardening_opt_in_execution_verification_complete {
        verification_blockers.push(
            "Rust data-plane controlled rollout guard requires opt-in execution verification to pass first".into(),
        );
    }

    let checks = vec![
        KernelLoopbackR4ExpandedOptInLimitedRolloutGateCheck {
            name: "rustDataPlaneHardeningOptInExecutionVerificationComplete".into(),
            status: if rust_data_plane_hardening_opt_in_execution_verification_complete {
                "passed"
            } else {
                "blocked"
            }
            .into(),
            passed: rust_data_plane_hardening_opt_in_execution_verification_complete,
            blockers: verification_blockers,
            facts: vec![
                "controlled rollout guard starts only after opt-in execution verification"
                    .into(),
            ],
        },
        KernelLoopbackR4ExpandedOptInLimitedRolloutGateCheck {
            name: "controlledRolloutGuardComplete".into(),
            status: if controlled_rollout_guard.controlled_rollout_guard_complete {
                "passed"
            } else {
                "blocked"
            }
            .into(),
            passed: controlled_rollout_guard.controlled_rollout_guard_complete,
            blockers: controlled_rollout_guard.blockers.clone(),
            facts: vec![
                "scope, canary cap, health rollback, telemetry hold, fallback, mutation guard, and acknowledgement are evaluated together".into(),
            ],
        },
        KernelLoopbackR4ExpandedOptInLimitedRolloutGateCheck {
            name: "finalControlledRolloutGuardDecision".into(),
            status: if final_controlled_rollout_guard_decision {
                "passed"
            } else {
                "blocked"
            }
            .into(),
            passed: final_controlled_rollout_guard_decision,
            blockers: if final_controlled_rollout_guard_decision {
                Vec::new()
            } else {
                vec!["Rust data-plane controlled rollout guard requires an explicit final decision".into()]
            },
            facts: vec!["controlled rollout guard completion is explicit before dry-run".into()],
        },
    ];
    let rust_data_plane_hardening_controlled_rollout_guard_complete = checks.iter().all(|check| check.passed);
    let blockers = checks
        .iter()
        .flat_map(|check| check.blockers.clone())
        .collect::<Vec<String>>();

    Ok(KernelLoopbackRustDataPlaneHardeningControlledRolloutGuardReport {
        runtime_id: RUST_RUNTIME_ID.into(),
        component: "rust-data-plane-hardening-controlled-rollout-guard".into(),
        mutates_runtime: false,
        live_execution_allowed: false,
        production_data_plane_mutation_allowed: false,
        rust_data_plane_hardening_opt_in_execution_verification_complete,
        controlled_rollout_guard,
        final_controlled_rollout_guard_decision,
        rust_data_plane_hardening_controlled_rollout_guard_complete,
        selected_runtime_kind: if rust_data_plane_hardening_controlled_rollout_guard_complete
        {
            KernelRuntimeKind::Rust
        } else {
            KernelRuntimeKind::Mihomo
        },
        rollback_runtime_kind: KernelRuntimeKind::Mihomo,
        checks,
        blockers,
        warnings: vec![
            "this controlled rollout guard does not mutate runtime, routes, TUN, DNS, adapter forwarding, or Mihomo config".into(),
            "Mihomo fallback and the production mutation guard remain mandatory".into(),
        ],
        facts: vec![
            "Rust data-plane hardening controlled rollout guard follows opt-in execution verification".into(),
            "successful guard completion advances only to controlled rollout dry-run".into(),
        ],
        next_safe_batch: if rust_data_plane_hardening_controlled_rollout_guard_complete {
            "rust-data-plane-hardening-controlled-rollout-dry-run".into()
        } else {
            "rust-data-plane-hardening-controlled-rollout-guard".into()
        },
    })
}

fn rust_kernel_runtime_data_plane_hardening_controlled_rollout_dry_run_report(
    guard_review_decision: bool,
    dry_run_manifest_replay_decision: bool,
    capped_canary_simulation_decision: bool,
    fallback_trigger_rehearsal_decision: bool,
    telemetry_hold_sample_review_decision: bool,
    rollback_switch_rehearsal_decision: bool,
    production_forwarding_unchanged_verification_decision: bool,
    dry_run_evidence_archive_decision: bool,
) -> RustKernelRuntimeDataPlaneHardeningControlledRolloutDryRunReport {
    let mut blockers = Vec::new();
    let mut dry_run_surfaces = Vec::new();

    if guard_review_decision {
        dry_run_surfaces.push("controlled rollout guard review".into());
    } else {
        blockers.push("Rust data-plane controlled rollout dry-run requires guard review".into());
    }
    if dry_run_manifest_replay_decision {
        dry_run_surfaces.push("dry-run manifest replay".into());
    } else {
        blockers.push("Rust data-plane controlled rollout dry-run requires manifest replay".into());
    }
    if capped_canary_simulation_decision {
        dry_run_surfaces.push("capped canary simulation".into());
    } else {
        blockers.push("Rust data-plane controlled rollout dry-run requires capped canary simulation".into());
    }
    if fallback_trigger_rehearsal_decision {
        dry_run_surfaces.push("fallback trigger rehearsal".into());
    } else {
        blockers.push("Rust data-plane controlled rollout dry-run requires fallback trigger rehearsal".into());
    }
    if telemetry_hold_sample_review_decision {
        dry_run_surfaces.push("telemetry hold sample review".into());
    } else {
        blockers.push("Rust data-plane controlled rollout dry-run requires telemetry hold sample review".into());
    }
    if rollback_switch_rehearsal_decision {
        dry_run_surfaces.push("rollback switch rehearsal".into());
    } else {
        blockers.push("Rust data-plane controlled rollout dry-run requires rollback switch rehearsal".into());
    }
    if production_forwarding_unchanged_verification_decision {
        dry_run_surfaces.push("production forwarding unchanged verification".into());
    } else {
        blockers.push(
            "Rust data-plane controlled rollout dry-run requires production forwarding unchanged verification".into(),
        );
    }
    if dry_run_evidence_archive_decision {
        dry_run_surfaces.push("archived dry-run evidence".into());
    } else {
        blockers.push("Rust data-plane controlled rollout dry-run requires archived evidence".into());
    }

    RustKernelRuntimeDataPlaneHardeningControlledRolloutDryRunReport {
        runtime_id: RUST_RUNTIME_ID.into(),
        component: "rust-data-plane-hardening-controlled-rollout-dry-run-detail".into(),
        guard_reviewed: guard_review_decision,
        dry_run_manifest_replayed: dry_run_manifest_replay_decision,
        capped_canary_simulation_completed: capped_canary_simulation_decision,
        fallback_trigger_rehearsed: fallback_trigger_rehearsal_decision,
        telemetry_hold_sample_reviewed: telemetry_hold_sample_review_decision,
        rollback_switch_rehearsed: rollback_switch_rehearsal_decision,
        production_forwarding_unchanged_verified: production_forwarding_unchanged_verification_decision,
        dry_run_evidence_archived: dry_run_evidence_archive_decision,
        controlled_rollout_dry_run_complete: blockers.is_empty(),
        dry_run_surfaces,
        blockers,
        facts: vec![
            "controlled rollout dry-run replays the rollout manifest without applying runtime changes".into(),
            "fallback rehearsal and unchanged forwarding checks remain required before readiness closeout".into(),
        ],
    }
}

pub async fn rust_kernel_runtime_data_plane_hardening_controlled_rollout_dry_run(
    rust_data_plane_hardening_controlled_rollout_guard_complete_decision: Option<bool>,
    guard_review_decision: Option<bool>,
    dry_run_manifest_replay_decision: Option<bool>,
    capped_canary_simulation_decision: Option<bool>,
    fallback_trigger_rehearsal_decision: Option<bool>,
    telemetry_hold_sample_review_decision: Option<bool>,
    rollback_switch_rehearsal_decision: Option<bool>,
    production_forwarding_unchanged_verification_decision: Option<bool>,
    dry_run_evidence_archive_decision: Option<bool>,
    final_controlled_rollout_dry_run_decision: Option<bool>,
) -> Result<KernelLoopbackRustDataPlaneHardeningControlledRolloutDryRunReport> {
    let rust_data_plane_hardening_controlled_rollout_guard_complete =
        rust_data_plane_hardening_controlled_rollout_guard_complete_decision.unwrap_or(false);
    let final_controlled_rollout_dry_run_decision = final_controlled_rollout_dry_run_decision.unwrap_or(false);
    let controlled_rollout_dry_run = rust_kernel_runtime_data_plane_hardening_controlled_rollout_dry_run_report(
        guard_review_decision.unwrap_or(false),
        dry_run_manifest_replay_decision.unwrap_or(false),
        capped_canary_simulation_decision.unwrap_or(false),
        fallback_trigger_rehearsal_decision.unwrap_or(false),
        telemetry_hold_sample_review_decision.unwrap_or(false),
        rollback_switch_rehearsal_decision.unwrap_or(false),
        production_forwarding_unchanged_verification_decision.unwrap_or(false),
        dry_run_evidence_archive_decision.unwrap_or(false),
    );
    let mut guard_blockers = Vec::new();

    if !rust_data_plane_hardening_controlled_rollout_guard_complete {
        guard_blockers
            .push("Rust data-plane controlled rollout dry-run requires controlled rollout guard to pass first".into());
    }

    let checks = vec![
        KernelLoopbackR4ExpandedOptInLimitedRolloutGateCheck {
            name: "rustDataPlaneHardeningControlledRolloutGuardComplete".into(),
            status: if rust_data_plane_hardening_controlled_rollout_guard_complete {
                "passed"
            } else {
                "blocked"
            }
            .into(),
            passed: rust_data_plane_hardening_controlled_rollout_guard_complete,
            blockers: guard_blockers,
            facts: vec!["controlled rollout dry-run starts only after the rollout guard".into()],
        },
        KernelLoopbackR4ExpandedOptInLimitedRolloutGateCheck {
            name: "controlledRolloutDryRunComplete".into(),
            status: if controlled_rollout_dry_run.controlled_rollout_dry_run_complete {
                "passed"
            } else {
                "blocked"
            }
            .into(),
            passed: controlled_rollout_dry_run.controlled_rollout_dry_run_complete,
            blockers: controlled_rollout_dry_run.blockers.clone(),
            facts: vec![
                "manifest replay, capped canary simulation, fallback rehearsal, telemetry hold, rollback switch, unchanged forwarding, and archival are evaluated together".into(),
            ],
        },
        KernelLoopbackR4ExpandedOptInLimitedRolloutGateCheck {
            name: "finalControlledRolloutDryRunDecision".into(),
            status: if final_controlled_rollout_dry_run_decision {
                "passed"
            } else {
                "blocked"
            }
            .into(),
            passed: final_controlled_rollout_dry_run_decision,
            blockers: if final_controlled_rollout_dry_run_decision {
                Vec::new()
            } else {
                vec!["Rust data-plane controlled rollout dry-run requires an explicit final decision".into()]
            },
            facts: vec!["controlled rollout dry-run completion is explicit before readiness closeout".into()],
        },
    ];
    let rust_data_plane_hardening_controlled_rollout_dry_run_complete = checks.iter().all(|check| check.passed);
    let blockers = checks
        .iter()
        .flat_map(|check| check.blockers.clone())
        .collect::<Vec<String>>();

    Ok(KernelLoopbackRustDataPlaneHardeningControlledRolloutDryRunReport {
        runtime_id: RUST_RUNTIME_ID.into(),
        component: "rust-data-plane-hardening-controlled-rollout-dry-run".into(),
        mutates_runtime: false,
        live_execution_allowed: false,
        production_data_plane_mutation_allowed: false,
        rust_data_plane_hardening_controlled_rollout_guard_complete,
        controlled_rollout_dry_run,
        final_controlled_rollout_dry_run_decision,
        rust_data_plane_hardening_controlled_rollout_dry_run_complete,
        selected_runtime_kind: if rust_data_plane_hardening_controlled_rollout_dry_run_complete {
            KernelRuntimeKind::Rust
        } else {
            KernelRuntimeKind::Mihomo
        },
        rollback_runtime_kind: KernelRuntimeKind::Mihomo,
        checks,
        blockers,
        warnings: vec![
            "this controlled rollout dry-run does not mutate runtime, routes, TUN, DNS, adapter forwarding, or Mihomo config".into(),
            "dry-run success does not authorize production forwarding mutation".into(),
        ],
        facts: vec![
            "Rust data-plane hardening controlled rollout dry-run follows the rollout guard".into(),
            "successful dry-run advances only to readiness closeout".into(),
        ],
        next_safe_batch: if rust_data_plane_hardening_controlled_rollout_dry_run_complete {
            "rust-data-plane-hardening-controlled-rollout-readiness-closeout".into()
        } else {
            "rust-data-plane-hardening-controlled-rollout-dry-run".into()
        },
    })
}

fn rust_kernel_runtime_data_plane_hardening_controlled_rollout_readiness_closeout_report(
    dry_run_review_decision: bool,
    rollout_window_approval_decision: bool,
    canary_population_cap_enforcement_decision: bool,
    automatic_fallback_arm_decision: bool,
    telemetry_watch_activation_decision: bool,
    rollback_owner_acknowledgement_decision: bool,
    production_mutation_guard_retention_decision: bool,
    closeout_evidence_archive_decision: bool,
) -> RustKernelRuntimeDataPlaneHardeningControlledRolloutReadinessCloseoutReport {
    let mut blockers = Vec::new();
    let mut closeout_surfaces = Vec::new();

    if dry_run_review_decision {
        closeout_surfaces.push("controlled rollout dry-run review".into());
    } else {
        blockers.push("Rust data-plane controlled rollout readiness closeout requires dry-run review".into());
    }
    if rollout_window_approval_decision {
        closeout_surfaces.push("rollout window approval".into());
    } else {
        blockers.push("Rust data-plane controlled rollout readiness closeout requires rollout window approval".into());
    }
    if canary_population_cap_enforcement_decision {
        closeout_surfaces.push("canary population cap enforcement".into());
    } else {
        blockers.push("Rust data-plane controlled rollout readiness closeout requires enforced canary cap".into());
    }
    if automatic_fallback_arm_decision {
        closeout_surfaces.push("armed automatic fallback".into());
    } else {
        blockers.push("Rust data-plane controlled rollout readiness closeout requires armed automatic fallback".into());
    }
    if telemetry_watch_activation_decision {
        closeout_surfaces.push("active telemetry watch".into());
    } else {
        blockers.push("Rust data-plane controlled rollout readiness closeout requires active telemetry watch".into());
    }
    if rollback_owner_acknowledgement_decision {
        closeout_surfaces.push("rollback owner acknowledgement".into());
    } else {
        blockers.push(
            "Rust data-plane controlled rollout readiness closeout requires rollback owner acknowledgement".into(),
        );
    }
    if production_mutation_guard_retention_decision {
        closeout_surfaces.push("retained production mutation guard".into());
    } else {
        blockers.push(
            "Rust data-plane controlled rollout readiness closeout requires retained production mutation guard".into(),
        );
    }
    if closeout_evidence_archive_decision {
        closeout_surfaces.push("archived readiness closeout evidence".into());
    } else {
        blockers.push("Rust data-plane controlled rollout readiness closeout requires archived evidence".into());
    }

    RustKernelRuntimeDataPlaneHardeningControlledRolloutReadinessCloseoutReport {
        runtime_id: RUST_RUNTIME_ID.into(),
        component: "rust-data-plane-hardening-controlled-rollout-readiness-closeout-detail".into(),
        dry_run_reviewed: dry_run_review_decision,
        rollout_window_approved: rollout_window_approval_decision,
        canary_population_cap_enforced: canary_population_cap_enforcement_decision,
        automatic_fallback_armed: automatic_fallback_arm_decision,
        telemetry_watch_active: telemetry_watch_activation_decision,
        rollback_owner_acknowledged: rollback_owner_acknowledgement_decision,
        production_mutation_guard_retained: production_mutation_guard_retention_decision,
        closeout_evidence_archived: closeout_evidence_archive_decision,
        controlled_rollout_readiness_closeout_complete: blockers.is_empty(),
        closeout_surfaces,
        blockers,
        facts: vec![
            "controlled rollout readiness closeout bundles dry-run review, rollout window, canary cap, fallback, telemetry, rollback ownership, mutation guard, and archival".into(),
            "readiness closeout is the last non-mutating gate before a separate canary execution PR".into(),
        ],
    }
}

pub async fn rust_kernel_runtime_data_plane_hardening_controlled_rollout_readiness_closeout(
    rust_data_plane_hardening_controlled_rollout_dry_run_complete_decision: Option<bool>,
    dry_run_review_decision: Option<bool>,
    rollout_window_approval_decision: Option<bool>,
    canary_population_cap_enforcement_decision: Option<bool>,
    automatic_fallback_arm_decision: Option<bool>,
    telemetry_watch_activation_decision: Option<bool>,
    rollback_owner_acknowledgement_decision: Option<bool>,
    production_mutation_guard_retention_decision: Option<bool>,
    closeout_evidence_archive_decision: Option<bool>,
    final_controlled_rollout_readiness_decision: Option<bool>,
) -> Result<KernelLoopbackRustDataPlaneHardeningControlledRolloutReadinessCloseoutReport> {
    let rust_data_plane_hardening_controlled_rollout_dry_run_complete =
        rust_data_plane_hardening_controlled_rollout_dry_run_complete_decision.unwrap_or(false);
    let final_controlled_rollout_readiness_decision = final_controlled_rollout_readiness_decision.unwrap_or(false);
    let controlled_rollout_readiness_closeout =
        rust_kernel_runtime_data_plane_hardening_controlled_rollout_readiness_closeout_report(
            dry_run_review_decision.unwrap_or(false),
            rollout_window_approval_decision.unwrap_or(false),
            canary_population_cap_enforcement_decision.unwrap_or(false),
            automatic_fallback_arm_decision.unwrap_or(false),
            telemetry_watch_activation_decision.unwrap_or(false),
            rollback_owner_acknowledgement_decision.unwrap_or(false),
            production_mutation_guard_retention_decision.unwrap_or(false),
            closeout_evidence_archive_decision.unwrap_or(false),
        );
    let mut dry_run_blockers = Vec::new();

    if !rust_data_plane_hardening_controlled_rollout_dry_run_complete {
        dry_run_blockers
            .push("Rust data-plane controlled rollout readiness closeout requires dry-run to pass first".into());
    }

    let checks = vec![
        KernelLoopbackR4ExpandedOptInLimitedRolloutGateCheck {
            name: "rustDataPlaneHardeningControlledRolloutDryRunComplete".into(),
            status: if rust_data_plane_hardening_controlled_rollout_dry_run_complete {
                "passed"
            } else {
                "blocked"
            }
            .into(),
            passed: rust_data_plane_hardening_controlled_rollout_dry_run_complete,
            blockers: dry_run_blockers,
            facts: vec!["readiness closeout starts only after controlled rollout dry-run".into()],
        },
        KernelLoopbackR4ExpandedOptInLimitedRolloutGateCheck {
            name: "controlledRolloutReadinessCloseoutComplete".into(),
            status: if controlled_rollout_readiness_closeout
                .controlled_rollout_readiness_closeout_complete
            {
                "passed"
            } else {
                "blocked"
            }
            .into(),
            passed: controlled_rollout_readiness_closeout
                .controlled_rollout_readiness_closeout_complete,
            blockers: controlled_rollout_readiness_closeout.blockers.clone(),
            facts: vec![
                "dry-run review, rollout window, canary cap, fallback, telemetry, rollback ownership, mutation guard, and archival are evaluated together".into(),
            ],
        },
        KernelLoopbackR4ExpandedOptInLimitedRolloutGateCheck {
            name: "finalControlledRolloutReadinessDecision".into(),
            status: if final_controlled_rollout_readiness_decision {
                "passed"
            } else {
                "blocked"
            }
            .into(),
            passed: final_controlled_rollout_readiness_decision,
            blockers: if final_controlled_rollout_readiness_decision {
                Vec::new()
            } else {
                vec![
                    "Rust data-plane controlled rollout readiness closeout requires an explicit final decision".into(),
                ]
            },
            facts: vec!["readiness closeout completion is explicit before canary execution".into()],
        },
    ];
    let rust_data_plane_hardening_controlled_rollout_readiness_closeout_complete =
        checks.iter().all(|check| check.passed);
    let blockers = checks
        .iter()
        .flat_map(|check| check.blockers.clone())
        .collect::<Vec<String>>();

    Ok(
        KernelLoopbackRustDataPlaneHardeningControlledRolloutReadinessCloseoutReport {
            runtime_id: RUST_RUNTIME_ID.into(),
            component: "rust-data-plane-hardening-controlled-rollout-readiness-closeout".into(),
            mutates_runtime: false,
            live_execution_allowed: false,
            production_data_plane_mutation_allowed: false,
            rust_data_plane_hardening_controlled_rollout_dry_run_complete,
            controlled_rollout_readiness_closeout,
            final_controlled_rollout_readiness_decision,
            rust_data_plane_hardening_controlled_rollout_readiness_closeout_complete,
            selected_runtime_kind:
                if rust_data_plane_hardening_controlled_rollout_readiness_closeout_complete {
                    KernelRuntimeKind::Rust
                } else {
                    KernelRuntimeKind::Mihomo
                },
            rollback_runtime_kind: KernelRuntimeKind::Mihomo,
            checks,
            blockers,
            warnings: vec![
                "this controlled rollout readiness closeout does not mutate runtime, routes, TUN, DNS, adapter forwarding, or Mihomo config".into(),
                "a later canary execution PR must preserve Mihomo fallback and explicit rollback".into(),
            ],
            facts: vec![
                "Rust data-plane hardening controlled rollout readiness closeout follows the dry-run".into(),
                "successful closeout advances only to a separate canary execution surface".into(),
            ],
            next_safe_batch: if rust_data_plane_hardening_controlled_rollout_readiness_closeout_complete {
                "rust-data-plane-hardening-controlled-rollout-canary-execution".into()
            } else {
                "rust-data-plane-hardening-controlled-rollout-readiness-closeout".into()
            },
        },
    )
}
