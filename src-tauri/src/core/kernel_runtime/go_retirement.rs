use anyhow::Result;
use smartstring::alias::String;

use super::{
    KernelLoopbackGoMihomoRetirementPlanReport, KernelLoopbackR4ExpandedOptInLimitedRolloutGateCheck,
    KernelRuntimeKind, RUST_RUNTIME_ID, RustKernelRuntimeGoMihomoRetirementRemovalPlanReport,
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
