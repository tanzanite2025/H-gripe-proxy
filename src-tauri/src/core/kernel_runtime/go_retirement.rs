use anyhow::Result;
use smartstring::alias::String;

use super::{
    KernelLoopbackGoMihomoRetirementCloseoutReport, KernelLoopbackGoMihomoRetirementDryRunReport,
    KernelLoopbackGoMihomoRetirementExecutionGuardReport, KernelLoopbackGoMihomoRetirementFinalRemovalGateReport,
    KernelLoopbackGoMihomoRetirementPlanReport, KernelLoopbackR4ExpandedOptInLimitedRolloutGateCheck,
    KernelRuntimeKind, RUST_RUNTIME_ID, RustKernelRuntimeGoMihomoRetirementCloseoutReport,
    RustKernelRuntimeGoMihomoRetirementDryRunReport, RustKernelRuntimeGoMihomoRetirementExecutionGuardReport,
    RustKernelRuntimeGoMihomoRetirementFinalRemovalGateReport, RustKernelRuntimeGoMihomoRetirementRemovalPlanReport,
};

fn rust_kernel_runtime_go_mihomo_retirement_removal_plan_report(
    sidecar_source_removal_plan_decision: bool,
    bundled_artifact_deprecation_plan_decision: bool,
    ipc_fallback_replacement_plan_decision: bool,
    emergency_rollback_preservation_plan_decision: bool,
    release_rollout_plan_decision: bool,
) -> RustKernelRuntimeGoMihomoRetirementRemovalPlanReport {
    let mut blockers = Vec::new();
    let mut planned_removal_surfaces = Vec::new();

    if sidecar_source_removal_plan_decision {
        planned_removal_surfaces.push("mihomo sidecar source tree".into());
    } else {
        blockers.push("Go/Mihomo retirement plan requires a sidecar source removal plan".into());
    }
    if bundled_artifact_deprecation_plan_decision {
        planned_removal_surfaces.push("bundled Mihomo binary and updater artifacts".into());
    } else {
        blockers.push("Go/Mihomo retirement plan requires bundled artifact deprecation".into());
    }
    if ipc_fallback_replacement_plan_decision {
        planned_removal_surfaces.push("IPC fallback command replacement".into());
    } else {
        blockers.push("Go/Mihomo retirement plan requires IPC fallback replacement planning".into());
    }
    if !emergency_rollback_preservation_plan_decision {
        blockers.push("Go/Mihomo retirement plan must preserve emergency rollback planning".into());
    }
    if !release_rollout_plan_decision {
        blockers.push("Go/Mihomo retirement plan requires release rollout and abort planning".into());
    }

    RustKernelRuntimeGoMihomoRetirementRemovalPlanReport {
        runtime_id: RUST_RUNTIME_ID.into(),
        component: "go-mihomo-retirement-removal-plan".into(),
        sidecar_source_removal_plan: sidecar_source_removal_plan_decision,
        bundled_artifact_deprecation_plan: bundled_artifact_deprecation_plan_decision,
        ipc_fallback_replacement_plan: ipc_fallback_replacement_plan_decision,
        emergency_rollback_preservation_plan: emergency_rollback_preservation_plan_decision,
        release_rollout_plan: release_rollout_plan_decision,
        removal_plan_complete: blockers.is_empty(),
        planned_removal_surfaces,
        blockers,
        facts: vec![
            "this plan describes future removal without deleting Go/Mihomo assets".into(),
            "emergency rollback preservation remains mandatory for any removal execution guard".into(),
        ],
    }
}

pub async fn rust_kernel_runtime_go_mihomo_retirement_plan(
    go_mihomo_retirement_audit_complete_decision: Option<bool>,
    sidecar_source_removal_plan_decision: Option<bool>,
    bundled_artifact_deprecation_plan_decision: Option<bool>,
    ipc_fallback_replacement_plan_decision: Option<bool>,
    emergency_rollback_preservation_plan_decision: Option<bool>,
    release_rollout_plan_decision: Option<bool>,
    final_retirement_plan_decision: Option<bool>,
) -> Result<KernelLoopbackGoMihomoRetirementPlanReport> {
    let go_mihomo_retirement_audit_complete = go_mihomo_retirement_audit_complete_decision.unwrap_or(false);
    let final_retirement_plan_decision = final_retirement_plan_decision.unwrap_or(false);
    let removal_plan = rust_kernel_runtime_go_mihomo_retirement_removal_plan_report(
        sidecar_source_removal_plan_decision.unwrap_or(false),
        bundled_artifact_deprecation_plan_decision.unwrap_or(false),
        ipc_fallback_replacement_plan_decision.unwrap_or(false),
        emergency_rollback_preservation_plan_decision.unwrap_or(false),
        release_rollout_plan_decision.unwrap_or(false),
    );
    let mut audit_blockers = Vec::new();

    if !go_mihomo_retirement_audit_complete {
        audit_blockers.push("Go/Mihomo retirement plan requires the retirement audit to pass first".into());
    }

    let checks = vec![
        KernelLoopbackR4ExpandedOptInLimitedRolloutGateCheck {
            name: "goMihomoRetirementAuditComplete".into(),
            status: if go_mihomo_retirement_audit_complete {
                "passed"
            } else {
                "blocked"
            }
            .into(),
            passed: go_mihomo_retirement_audit_complete,
            blockers: audit_blockers,
            facts: vec!["retirement planning starts only after the audit inventories all Mihomo surfaces".into()],
        },
        KernelLoopbackR4ExpandedOptInLimitedRolloutGateCheck {
            name: "goMihomoRemovalPlanComplete".into(),
            status: if removal_plan.removal_plan_complete {
                "passed"
            } else {
                "blocked"
            }
            .into(),
            passed: removal_plan.removal_plan_complete,
            blockers: removal_plan.blockers.clone(),
            facts: vec!["source, artifact, IPC, rollback, and release plans are evaluated together".into()],
        },
        KernelLoopbackR4ExpandedOptInLimitedRolloutGateCheck {
            name: "finalRetirementPlanDecision".into(),
            status: if final_retirement_plan_decision {
                "passed"
            } else {
                "blocked"
            }
            .into(),
            passed: final_retirement_plan_decision,
            blockers: if final_retirement_plan_decision {
                Vec::new()
            } else {
                vec!["Go/Mihomo retirement plan requires an explicit final plan decision".into()]
            },
            facts: vec!["the plan is explicit and does not execute removal".into()],
        },
    ];
    let go_mihomo_retirement_plan_complete = checks.iter().all(|check| check.passed);
    let blockers = checks
        .iter()
        .flat_map(|check| check.blockers.clone())
        .collect::<Vec<String>>();

    Ok(KernelLoopbackGoMihomoRetirementPlanReport {
        runtime_id: RUST_RUNTIME_ID.into(),
        component: "go-mihomo-retirement-plan".into(),
        mutates_runtime: false,
        live_execution_allowed: go_mihomo_retirement_plan_complete,
        go_mihomo_retirement_audit_complete,
        removal_plan,
        final_retirement_plan_decision,
        go_mihomo_retirement_plan_complete,
        selected_runtime_kind: if go_mihomo_retirement_plan_complete {
            KernelRuntimeKind::Rust
        } else {
            KernelRuntimeKind::Mihomo
        },
        rollback_runtime_kind: KernelRuntimeKind::Mihomo,
        checks,
        blockers,
        warnings: vec![
            "this plan does not delete Mihomo source, binaries, IPC commands, or rollback paths".into(),
            "actual removal requires a later execution guard and abort plan".into(),
        ],
        facts: vec![
            "Go/Mihomo retirement plan is a planning gate after the audit gate".into(),
            "successful planning advances to execution guard readiness instead of direct removal".into(),
        ],
        next_safe_batch: if go_mihomo_retirement_plan_complete {
            "go-mihomo-retirement-execution-guard".into()
        } else {
            "go-mihomo-retirement-plan".into()
        },
    })
}

fn rust_kernel_runtime_go_mihomo_retirement_execution_guard_report(
    removal_manifest_decision: bool,
    abort_plan_decision: bool,
    staged_rollout_guard_decision: bool,
    emergency_rollback_drill_decision: bool,
    operator_acknowledgement_decision: bool,
) -> RustKernelRuntimeGoMihomoRetirementExecutionGuardReport {
    let mut blockers = Vec::new();
    let mut guarded_execution_surfaces = Vec::new();

    if removal_manifest_decision {
        guarded_execution_surfaces.push("source and bundled artifact removal manifest".into());
    } else {
        blockers.push("Go/Mihomo retirement execution guard requires a removal manifest".into());
    }
    if abort_plan_decision {
        guarded_execution_surfaces.push("abort plan and rollback checkpoint".into());
    } else {
        blockers.push("Go/Mihomo retirement execution guard requires an abort plan".into());
    }
    if staged_rollout_guard_decision {
        guarded_execution_surfaces.push("staged rollout guard".into());
    } else {
        blockers.push("Go/Mihomo retirement execution guard requires staged rollout guards".into());
    }
    if !emergency_rollback_drill_decision {
        blockers.push("Go/Mihomo retirement execution guard requires an emergency rollback drill".into());
    }
    if !operator_acknowledgement_decision {
        blockers.push("Go/Mihomo retirement execution guard requires operator acknowledgement".into());
    }

    RustKernelRuntimeGoMihomoRetirementExecutionGuardReport {
        runtime_id: RUST_RUNTIME_ID.into(),
        component: "go-mihomo-retirement-execution-guard-detail".into(),
        removal_manifest_ready: removal_manifest_decision,
        abort_plan_ready: abort_plan_decision,
        staged_rollout_guard_ready: staged_rollout_guard_decision,
        emergency_rollback_drill_passed: emergency_rollback_drill_decision,
        operator_acknowledgement: operator_acknowledgement_decision,
        execution_guard_complete: blockers.is_empty(),
        guarded_execution_surfaces,
        blockers,
        facts: vec![
            "this guard prepares future execution without deleting Go/Mihomo assets".into(),
            "abort and emergency rollback evidence remain mandatory before dry-run removal".into(),
        ],
    }
}

pub async fn rust_kernel_runtime_go_mihomo_retirement_execution_guard(
    go_mihomo_retirement_plan_complete_decision: Option<bool>,
    removal_manifest_decision: Option<bool>,
    abort_plan_decision: Option<bool>,
    staged_rollout_guard_decision: Option<bool>,
    emergency_rollback_drill_decision: Option<bool>,
    operator_acknowledgement_decision: Option<bool>,
    final_execution_guard_decision: Option<bool>,
) -> Result<KernelLoopbackGoMihomoRetirementExecutionGuardReport> {
    let go_mihomo_retirement_plan_complete = go_mihomo_retirement_plan_complete_decision.unwrap_or(false);
    let final_execution_guard_decision = final_execution_guard_decision.unwrap_or(false);
    let execution_guard = rust_kernel_runtime_go_mihomo_retirement_execution_guard_report(
        removal_manifest_decision.unwrap_or(false),
        abort_plan_decision.unwrap_or(false),
        staged_rollout_guard_decision.unwrap_or(false),
        emergency_rollback_drill_decision.unwrap_or(false),
        operator_acknowledgement_decision.unwrap_or(false),
    );
    let mut plan_blockers = Vec::new();

    if !go_mihomo_retirement_plan_complete {
        plan_blockers.push("Go/Mihomo retirement execution guard requires the retirement plan to pass first".into());
    }

    let checks = vec![
        KernelLoopbackR4ExpandedOptInLimitedRolloutGateCheck {
            name: "goMihomoRetirementPlanComplete".into(),
            status: if go_mihomo_retirement_plan_complete {
                "passed"
            } else {
                "blocked"
            }
            .into(),
            passed: go_mihomo_retirement_plan_complete,
            blockers: plan_blockers,
            facts: vec!["execution guard starts only after the retirement plan closes".into()],
        },
        KernelLoopbackR4ExpandedOptInLimitedRolloutGateCheck {
            name: "goMihomoExecutionGuardComplete".into(),
            status: if execution_guard.execution_guard_complete {
                "passed"
            } else {
                "blocked"
            }
            .into(),
            passed: execution_guard.execution_guard_complete,
            blockers: execution_guard.blockers.clone(),
            facts: vec![
                "manifest, abort plan, rollout guard, rollback drill, and acknowledgement are evaluated together"
                    .into(),
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
                vec!["Go/Mihomo retirement execution guard requires an explicit final guard decision".into()]
            },
            facts: vec!["the guard is explicit and does not execute removal".into()],
        },
    ];
    let go_mihomo_retirement_execution_guard_complete = checks.iter().all(|check| check.passed);
    let blockers = checks
        .iter()
        .flat_map(|check| check.blockers.clone())
        .collect::<Vec<String>>();

    Ok(KernelLoopbackGoMihomoRetirementExecutionGuardReport {
        runtime_id: RUST_RUNTIME_ID.into(),
        component: "go-mihomo-retirement-execution-guard".into(),
        mutates_runtime: false,
        live_execution_allowed: go_mihomo_retirement_execution_guard_complete,
        go_mihomo_retirement_plan_complete,
        execution_guard,
        final_execution_guard_decision,
        go_mihomo_retirement_execution_guard_complete,
        selected_runtime_kind: if go_mihomo_retirement_execution_guard_complete {
            KernelRuntimeKind::Rust
        } else {
            KernelRuntimeKind::Mihomo
        },
        rollback_runtime_kind: KernelRuntimeKind::Mihomo,
        checks,
        blockers,
        warnings: vec![
            "this execution guard does not delete Mihomo source, binaries, IPC commands, or rollback paths".into(),
            "actual removal still requires a later dry-run batch and explicit abort boundary".into(),
        ],
        facts: vec![
            "Go/Mihomo retirement execution guard follows the plan gate".into(),
            "successful guard readiness advances to dry-run removal instead of direct deletion".into(),
        ],
        next_safe_batch: if go_mihomo_retirement_execution_guard_complete {
            "go-mihomo-retirement-dry-run".into()
        } else {
            "go-mihomo-retirement-execution-guard".into()
        },
    })
}

fn rust_kernel_runtime_go_mihomo_retirement_dry_run_report(
    dry_run_manifest_replay_decision: bool,
    no_source_mutations_decision: bool,
    no_bundled_artifact_mutations_decision: bool,
    rollback_rehearsal_decision: bool,
    dry_run_report_archived_decision: bool,
) -> RustKernelRuntimeGoMihomoRetirementDryRunReport {
    let mut blockers = Vec::new();
    let mut simulated_removal_surfaces = Vec::new();

    if dry_run_manifest_replay_decision {
        simulated_removal_surfaces.push("removal manifest replay".into());
    } else {
        blockers.push("Go/Mihomo retirement dry run requires manifest replay evidence".into());
    }
    if no_source_mutations_decision {
        simulated_removal_surfaces.push("sidecar source mutation check".into());
    } else {
        blockers.push("Go/Mihomo retirement dry run must prove no source mutations".into());
    }
    if no_bundled_artifact_mutations_decision {
        simulated_removal_surfaces.push("bundled artifact mutation check".into());
    } else {
        blockers.push("Go/Mihomo retirement dry run must prove no artifact mutations".into());
    }
    if !rollback_rehearsal_decision {
        blockers.push("Go/Mihomo retirement dry run requires rollback rehearsal evidence".into());
    }
    if !dry_run_report_archived_decision {
        blockers.push("Go/Mihomo retirement dry run requires archived dry-run evidence".into());
    }

    RustKernelRuntimeGoMihomoRetirementDryRunReport {
        runtime_id: RUST_RUNTIME_ID.into(),
        component: "go-mihomo-retirement-dry-run-detail".into(),
        dry_run_manifest_replayed: dry_run_manifest_replay_decision,
        no_source_mutations_observed: no_source_mutations_decision,
        no_bundled_artifact_mutations_observed: no_bundled_artifact_mutations_decision,
        rollback_rehearsal_passed: rollback_rehearsal_decision,
        dry_run_report_archived: dry_run_report_archived_decision,
        dry_run_complete: blockers.is_empty(),
        simulated_removal_surfaces,
        blockers,
        facts: vec![
            "this dry run simulates retirement without deleting Go/Mihomo assets".into(),
            "mutation checks must remain clean before any real removal closeout".into(),
        ],
    }
}

pub async fn rust_kernel_runtime_go_mihomo_retirement_dry_run(
    go_mihomo_retirement_execution_guard_complete_decision: Option<bool>,
    dry_run_manifest_replay_decision: Option<bool>,
    no_source_mutations_decision: Option<bool>,
    no_bundled_artifact_mutations_decision: Option<bool>,
    rollback_rehearsal_decision: Option<bool>,
    dry_run_report_archived_decision: Option<bool>,
    final_dry_run_decision: Option<bool>,
) -> Result<KernelLoopbackGoMihomoRetirementDryRunReport> {
    let go_mihomo_retirement_execution_guard_complete =
        go_mihomo_retirement_execution_guard_complete_decision.unwrap_or(false);
    let final_dry_run_decision = final_dry_run_decision.unwrap_or(false);
    let dry_run = rust_kernel_runtime_go_mihomo_retirement_dry_run_report(
        dry_run_manifest_replay_decision.unwrap_or(false),
        no_source_mutations_decision.unwrap_or(false),
        no_bundled_artifact_mutations_decision.unwrap_or(false),
        rollback_rehearsal_decision.unwrap_or(false),
        dry_run_report_archived_decision.unwrap_or(false),
    );
    let mut guard_blockers = Vec::new();

    if !go_mihomo_retirement_execution_guard_complete {
        guard_blockers.push("Go/Mihomo retirement dry run requires the execution guard to pass first".into());
    }

    let checks = vec![
        KernelLoopbackR4ExpandedOptInLimitedRolloutGateCheck {
            name: "goMihomoRetirementExecutionGuardComplete".into(),
            status: if go_mihomo_retirement_execution_guard_complete {
                "passed"
            } else {
                "blocked"
            }
            .into(),
            passed: go_mihomo_retirement_execution_guard_complete,
            blockers: guard_blockers,
            facts: vec!["dry run starts only after execution guard readiness".into()],
        },
        KernelLoopbackR4ExpandedOptInLimitedRolloutGateCheck {
            name: "goMihomoRetirementDryRunComplete".into(),
            status: if dry_run.dry_run_complete { "passed" } else { "blocked" }.into(),
            passed: dry_run.dry_run_complete,
            blockers: dry_run.blockers.clone(),
            facts: vec![
                "manifest replay, mutation checks, rollback rehearsal, and archived evidence are evaluated together"
                    .into(),
            ],
        },
        KernelLoopbackR4ExpandedOptInLimitedRolloutGateCheck {
            name: "finalDryRunDecision".into(),
            status: if final_dry_run_decision { "passed" } else { "blocked" }.into(),
            passed: final_dry_run_decision,
            blockers: if final_dry_run_decision {
                Vec::new()
            } else {
                vec!["Go/Mihomo retirement dry run requires an explicit final dry-run decision".into()]
            },
            facts: vec!["the dry run is explicit and does not execute removal".into()],
        },
    ];
    let go_mihomo_retirement_dry_run_complete = checks.iter().all(|check| check.passed);
    let blockers = checks
        .iter()
        .flat_map(|check| check.blockers.clone())
        .collect::<Vec<String>>();

    Ok(KernelLoopbackGoMihomoRetirementDryRunReport {
        runtime_id: RUST_RUNTIME_ID.into(),
        component: "go-mihomo-retirement-dry-run".into(),
        mutates_runtime: false,
        live_execution_allowed: go_mihomo_retirement_dry_run_complete,
        go_mihomo_retirement_execution_guard_complete,
        dry_run,
        final_dry_run_decision,
        go_mihomo_retirement_dry_run_complete,
        selected_runtime_kind: if go_mihomo_retirement_dry_run_complete {
            KernelRuntimeKind::Rust
        } else {
            KernelRuntimeKind::Mihomo
        },
        rollback_runtime_kind: KernelRuntimeKind::Mihomo,
        checks,
        blockers,
        warnings: vec![
            "this dry run does not delete Mihomo source, binaries, IPC commands, or rollback paths".into(),
            "actual removal still requires a later closeout and final removal gate".into(),
        ],
        facts: vec![
            "Go/Mihomo retirement dry run follows the execution guard gate".into(),
            "successful dry run advances to closeout instead of direct deletion".into(),
        ],
        next_safe_batch: if go_mihomo_retirement_dry_run_complete {
            "go-mihomo-retirement-closeout".into()
        } else {
            "go-mihomo-retirement-dry-run".into()
        },
    })
}

fn rust_kernel_runtime_go_mihomo_retirement_closeout_report(
    dry_run_evidence_review_decision: bool,
    closeout_report_archived_decision: bool,
    rollback_checkpoint_verified_decision: bool,
    artifact_inventory_frozen_decision: bool,
    no_removal_mutations_decision: bool,
) -> RustKernelRuntimeGoMihomoRetirementCloseoutReport {
    let mut blockers = Vec::new();
    let mut closed_out_surfaces = Vec::new();

    if dry_run_evidence_review_decision {
        closed_out_surfaces.push("dry-run evidence review".into());
    } else {
        blockers.push("Go/Mihomo retirement closeout requires reviewed dry-run evidence".into());
    }
    if closeout_report_archived_decision {
        closed_out_surfaces.push("closeout report archive".into());
    } else {
        blockers.push("Go/Mihomo retirement closeout requires archived closeout report".into());
    }
    if rollback_checkpoint_verified_decision {
        closed_out_surfaces.push("rollback checkpoint verification".into());
    } else {
        blockers.push("Go/Mihomo retirement closeout requires rollback checkpoint verification".into());
    }
    if artifact_inventory_frozen_decision {
        closed_out_surfaces.push("frozen artifact inventory".into());
    } else {
        blockers.push("Go/Mihomo retirement closeout requires frozen artifact inventory".into());
    }
    if !no_removal_mutations_decision {
        blockers.push("Go/Mihomo retirement closeout must prove no removal mutations".into());
    }

    RustKernelRuntimeGoMihomoRetirementCloseoutReport {
        runtime_id: RUST_RUNTIME_ID.into(),
        component: "go-mihomo-retirement-closeout-detail".into(),
        dry_run_evidence_reviewed: dry_run_evidence_review_decision,
        closeout_report_archived: closeout_report_archived_decision,
        rollback_checkpoint_verified: rollback_checkpoint_verified_decision,
        artifact_inventory_frozen: artifact_inventory_frozen_decision,
        no_removal_mutations_observed: no_removal_mutations_decision,
        closeout_complete: blockers.is_empty(),
        closed_out_surfaces,
        blockers,
        facts: vec![
            "this closeout summarizes dry-run evidence without deleting Go/Mihomo assets".into(),
            "frozen inventory and rollback checkpoint evidence gate any later final removal".into(),
        ],
    }
}

pub async fn rust_kernel_runtime_go_mihomo_retirement_closeout(
    go_mihomo_retirement_dry_run_complete_decision: Option<bool>,
    dry_run_evidence_review_decision: Option<bool>,
    closeout_report_archived_decision: Option<bool>,
    rollback_checkpoint_verified_decision: Option<bool>,
    artifact_inventory_frozen_decision: Option<bool>,
    no_removal_mutations_decision: Option<bool>,
    final_closeout_decision: Option<bool>,
) -> Result<KernelLoopbackGoMihomoRetirementCloseoutReport> {
    let go_mihomo_retirement_dry_run_complete = go_mihomo_retirement_dry_run_complete_decision.unwrap_or(false);
    let final_closeout_decision = final_closeout_decision.unwrap_or(false);
    let closeout = rust_kernel_runtime_go_mihomo_retirement_closeout_report(
        dry_run_evidence_review_decision.unwrap_or(false),
        closeout_report_archived_decision.unwrap_or(false),
        rollback_checkpoint_verified_decision.unwrap_or(false),
        artifact_inventory_frozen_decision.unwrap_or(false),
        no_removal_mutations_decision.unwrap_or(false),
    );
    let mut dry_run_blockers = Vec::new();

    if !go_mihomo_retirement_dry_run_complete {
        dry_run_blockers.push("Go/Mihomo retirement closeout requires the dry run to pass first".into());
    }

    let checks = vec![
        KernelLoopbackR4ExpandedOptInLimitedRolloutGateCheck {
            name: "goMihomoRetirementDryRunComplete".into(),
            status: if go_mihomo_retirement_dry_run_complete {
                "passed"
            } else {
                "blocked"
            }
            .into(),
            passed: go_mihomo_retirement_dry_run_complete,
            blockers: dry_run_blockers,
            facts: vec!["closeout starts only after dry-run readiness".into()],
        },
        KernelLoopbackR4ExpandedOptInLimitedRolloutGateCheck {
            name: "goMihomoRetirementCloseoutComplete".into(),
            status: if closeout.closeout_complete {
                "passed"
            } else {
                "blocked"
            }
            .into(),
            passed: closeout.closeout_complete,
            blockers: closeout.blockers.clone(),
            facts: vec![
                "evidence review, archived report, rollback checkpoint, frozen inventory, and mutation checks are evaluated together".into(),
            ],
        },
        KernelLoopbackR4ExpandedOptInLimitedRolloutGateCheck {
            name: "finalCloseoutDecision".into(),
            status: if final_closeout_decision {
                "passed"
            } else {
                "blocked"
            }
            .into(),
            passed: final_closeout_decision,
            blockers: if final_closeout_decision {
                Vec::new()
            } else {
                vec!["Go/Mihomo retirement closeout requires an explicit final closeout decision".into()]
            },
            facts: vec!["the closeout is explicit and does not execute removal".into()],
        },
    ];
    let go_mihomo_retirement_closeout_complete = checks.iter().all(|check| check.passed);
    let blockers = checks
        .iter()
        .flat_map(|check| check.blockers.clone())
        .collect::<Vec<String>>();

    Ok(KernelLoopbackGoMihomoRetirementCloseoutReport {
        runtime_id: RUST_RUNTIME_ID.into(),
        component: "go-mihomo-retirement-closeout".into(),
        mutates_runtime: false,
        live_execution_allowed: go_mihomo_retirement_closeout_complete,
        go_mihomo_retirement_dry_run_complete,
        closeout,
        final_closeout_decision,
        go_mihomo_retirement_closeout_complete,
        selected_runtime_kind: if go_mihomo_retirement_closeout_complete {
            KernelRuntimeKind::Rust
        } else {
            KernelRuntimeKind::Mihomo
        },
        rollback_runtime_kind: KernelRuntimeKind::Mihomo,
        checks,
        blockers,
        warnings: vec![
            "this closeout does not delete Mihomo source, binaries, IPC commands, or rollback paths".into(),
            "actual removal still requires a final removal gate and explicit rollback boundary".into(),
        ],
        facts: vec![
            "Go/Mihomo retirement closeout follows the dry-run gate".into(),
            "successful closeout advances to final removal gate readiness instead of direct deletion".into(),
        ],
        next_safe_batch: if go_mihomo_retirement_closeout_complete {
            "go-mihomo-retirement-final-removal-gate".into()
        } else {
            "go-mihomo-retirement-closeout".into()
        },
    })
}

fn rust_kernel_runtime_go_mihomo_retirement_final_removal_gate_report(
    closeout_evidence_acceptance_decision: bool,
    rollback_boundary_lock_decision: bool,
    removal_scope_lock_decision: bool,
    release_blocker_review_decision: bool,
    final_operator_approval_decision: bool,
) -> RustKernelRuntimeGoMihomoRetirementFinalRemovalGateReport {
    let mut blockers = Vec::new();
    let mut approved_removal_surfaces = Vec::new();

    if closeout_evidence_acceptance_decision {
        approved_removal_surfaces.push("accepted closeout evidence".into());
    } else {
        blockers.push("Go/Mihomo final removal gate requires accepted closeout evidence".into());
    }
    if rollback_boundary_lock_decision {
        approved_removal_surfaces.push("locked rollback boundary".into());
    } else {
        blockers.push("Go/Mihomo final removal gate requires a locked rollback boundary".into());
    }
    if removal_scope_lock_decision {
        approved_removal_surfaces.push("locked removal scope".into());
    } else {
        blockers.push("Go/Mihomo final removal gate requires locked removal scope".into());
    }
    if !release_blocker_review_decision {
        blockers.push("Go/Mihomo final removal gate requires release blocker review".into());
    }
    if !final_operator_approval_decision {
        blockers.push("Go/Mihomo final removal gate requires final operator approval".into());
    }

    RustKernelRuntimeGoMihomoRetirementFinalRemovalGateReport {
        runtime_id: RUST_RUNTIME_ID.into(),
        component: "go-mihomo-retirement-final-removal-gate-detail".into(),
        closeout_evidence_accepted: closeout_evidence_acceptance_decision,
        rollback_boundary_locked: rollback_boundary_lock_decision,
        removal_scope_locked: removal_scope_lock_decision,
        release_blocker_review_passed: release_blocker_review_decision,
        final_operator_approval: final_operator_approval_decision,
        final_removal_gate_complete: blockers.is_empty(),
        approved_removal_surfaces,
        blockers,
        facts: vec![
            "this final removal gate records readiness without deleting Go/Mihomo assets".into(),
            "rollback boundary and removal scope must be locked before any later execution".into(),
        ],
    }
}

pub async fn rust_kernel_runtime_go_mihomo_retirement_final_removal_gate(
    go_mihomo_retirement_closeout_complete_decision: Option<bool>,
    closeout_evidence_acceptance_decision: Option<bool>,
    rollback_boundary_lock_decision: Option<bool>,
    removal_scope_lock_decision: Option<bool>,
    release_blocker_review_decision: Option<bool>,
    final_operator_approval_decision: Option<bool>,
    final_removal_decision: Option<bool>,
) -> Result<KernelLoopbackGoMihomoRetirementFinalRemovalGateReport> {
    let go_mihomo_retirement_closeout_complete = go_mihomo_retirement_closeout_complete_decision.unwrap_or(false);
    let final_removal_decision = final_removal_decision.unwrap_or(false);
    let final_removal_gate = rust_kernel_runtime_go_mihomo_retirement_final_removal_gate_report(
        closeout_evidence_acceptance_decision.unwrap_or(false),
        rollback_boundary_lock_decision.unwrap_or(false),
        removal_scope_lock_decision.unwrap_or(false),
        release_blocker_review_decision.unwrap_or(false),
        final_operator_approval_decision.unwrap_or(false),
    );
    let mut closeout_blockers = Vec::new();

    if !go_mihomo_retirement_closeout_complete {
        closeout_blockers.push("Go/Mihomo final removal gate requires retirement closeout to pass first".into());
    }

    let checks = vec![
        KernelLoopbackR4ExpandedOptInLimitedRolloutGateCheck {
            name: "goMihomoRetirementCloseoutComplete".into(),
            status: if go_mihomo_retirement_closeout_complete {
                "passed"
            } else {
                "blocked"
            }
            .into(),
            passed: go_mihomo_retirement_closeout_complete,
            blockers: closeout_blockers,
            facts: vec!["final removal gate starts only after closeout readiness".into()],
        },
        KernelLoopbackR4ExpandedOptInLimitedRolloutGateCheck {
            name: "goMihomoFinalRemovalGateComplete".into(),
            status: if final_removal_gate.final_removal_gate_complete {
                "passed"
            } else {
                "blocked"
            }
            .into(),
            passed: final_removal_gate.final_removal_gate_complete,
            blockers: final_removal_gate.blockers.clone(),
            facts: vec![
                "closeout acceptance, rollback boundary, removal scope, release blockers, and operator approval are evaluated together".into(),
            ],
        },
        KernelLoopbackR4ExpandedOptInLimitedRolloutGateCheck {
            name: "finalRemovalDecision".into(),
            status: if final_removal_decision {
                "passed"
            } else {
                "blocked"
            }
            .into(),
            passed: final_removal_decision,
            blockers: if final_removal_decision {
                Vec::new()
            } else {
                vec!["Go/Mihomo final removal gate requires an explicit final removal decision".into()]
            },
            facts: vec!["the final removal gate is explicit and does not execute removal".into()],
        },
    ];
    let go_mihomo_retirement_final_removal_gate_complete = checks.iter().all(|check| check.passed);
    let blockers = checks
        .iter()
        .flat_map(|check| check.blockers.clone())
        .collect::<Vec<String>>();

    Ok(KernelLoopbackGoMihomoRetirementFinalRemovalGateReport {
        runtime_id: RUST_RUNTIME_ID.into(),
        component: "go-mihomo-retirement-final-removal-gate".into(),
        mutates_runtime: false,
        live_execution_allowed: go_mihomo_retirement_final_removal_gate_complete,
        go_mihomo_retirement_closeout_complete,
        final_removal_gate,
        final_removal_decision,
        go_mihomo_retirement_final_removal_gate_complete,
        selected_runtime_kind: if go_mihomo_retirement_final_removal_gate_complete {
            KernelRuntimeKind::Rust
        } else {
            KernelRuntimeKind::Mihomo
        },
        rollback_runtime_kind: KernelRuntimeKind::Mihomo,
        checks,
        blockers,
        warnings: vec![
            "this final removal gate does not delete Mihomo source, binaries, IPC commands, or rollback paths".into(),
            "actual deletion still requires a dedicated execution batch and rollback checkpoint".into(),
        ],
        facts: vec![
            "Go/Mihomo final removal gate follows closeout readiness".into(),
            "successful final gate advances to a separate execution batch instead of deleting assets".into(),
        ],
        next_safe_batch: if go_mihomo_retirement_final_removal_gate_complete {
            "go-mihomo-retirement-execution".into()
        } else {
            "go-mihomo-retirement-final-removal-gate".into()
        },
    })
}
