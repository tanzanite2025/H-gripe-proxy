use anyhow::Result;
use smartstring::alias::String;

use super::{
    KernelLoopbackR4ExpandedOptInLimitedRolloutGateCheck, KernelLoopbackRustDataPlaneHardeningBoundaryAuditReport,
    KernelLoopbackRustDataPlaneHardeningControlledRolloutCanaryExecutionReport,
    KernelLoopbackRustDataPlaneHardeningControlledRolloutCanaryVerificationReport,
    KernelLoopbackRustDataPlaneHardeningControlledRolloutDryRunReport,
    KernelLoopbackRustDataPlaneHardeningControlledRolloutGuardReport,
    KernelLoopbackRustDataPlaneHardeningControlledRolloutReadinessCloseoutReport,
    KernelLoopbackRustDataPlaneHardeningExpandedDefaultRolloutCloseoutReport,
    KernelLoopbackRustDataPlaneHardeningExpandedDefaultRolloutDryRunReport,
    KernelLoopbackRustDataPlaneHardeningExpandedDefaultRolloutExecutionReport,
    KernelLoopbackRustDataPlaneHardeningExpandedDefaultRolloutGuardReport,
    KernelLoopbackRustDataPlaneHardeningExpandedDefaultRolloutVerificationReport,
    KernelLoopbackRustDataPlaneHardeningMihomoFallbackRetirementDryRunReport,
    KernelLoopbackRustDataPlaneHardeningMihomoFallbackRetirementGuardReport,
    KernelLoopbackRustDataPlaneHardeningMihomoFallbackRetirementReadinessReport,
    KernelLoopbackRustDataPlaneHardeningOptInDryRunReport,
    KernelLoopbackRustDataPlaneHardeningOptInExecutionGuardReport,
    KernelLoopbackRustDataPlaneHardeningOptInExecutionReport,
    KernelLoopbackRustDataPlaneHardeningOptInExecutionVerificationReport,
    KernelLoopbackRustDataPlaneHardeningPreflightReport,
    KernelLoopbackRustDataPlaneHardeningSupportedDefaultCutoverCloseoutReport,
    KernelLoopbackRustDataPlaneHardeningSupportedDefaultCutoverHoldWindowReport,
    KernelLoopbackRustDataPlaneHardeningSupportedDefaultCutoverReport,
    KernelLoopbackRustDataPlaneHardeningSupportedDefaultCutoverVerificationReport,
    KernelLoopbackRustDataPlaneHardeningSupportedDefaultPromotionDryRunReport,
    KernelLoopbackRustDataPlaneHardeningSupportedDefaultPromotionGuardReport, KernelRuntimeKind, RUST_RUNTIME_ID,
    RustKernelRuntimeDataPlaneHardeningBoundaryAuditReport, RustKernelRuntimeDataPlaneHardeningBoundaryReport,
    RustKernelRuntimeDataPlaneHardeningControlledRolloutCanaryExecutionReport,
    RustKernelRuntimeDataPlaneHardeningControlledRolloutCanaryVerificationReport,
    RustKernelRuntimeDataPlaneHardeningControlledRolloutDryRunReport,
    RustKernelRuntimeDataPlaneHardeningControlledRolloutGuardReport,
    RustKernelRuntimeDataPlaneHardeningControlledRolloutReadinessCloseoutReport,
    RustKernelRuntimeDataPlaneHardeningExpandedDefaultRolloutCloseoutReport,
    RustKernelRuntimeDataPlaneHardeningExpandedDefaultRolloutDryRunReport,
    RustKernelRuntimeDataPlaneHardeningExpandedDefaultRolloutExecutionReport,
    RustKernelRuntimeDataPlaneHardeningExpandedDefaultRolloutGuardReport,
    RustKernelRuntimeDataPlaneHardeningExpandedDefaultRolloutVerificationReport,
    RustKernelRuntimeDataPlaneHardeningMihomoFallbackRetirementDryRunReport,
    RustKernelRuntimeDataPlaneHardeningMihomoFallbackRetirementGuardReport,
    RustKernelRuntimeDataPlaneHardeningMihomoFallbackRetirementReadinessReport,
    RustKernelRuntimeDataPlaneHardeningOptInDryRunReport, RustKernelRuntimeDataPlaneHardeningOptInExecutionGuardReport,
    RustKernelRuntimeDataPlaneHardeningOptInExecutionReport,
    RustKernelRuntimeDataPlaneHardeningOptInExecutionVerificationReport,
    RustKernelRuntimeDataPlaneHardeningSupportedDefaultCutoverCloseoutReport,
    RustKernelRuntimeDataPlaneHardeningSupportedDefaultCutoverHoldWindowReport,
    RustKernelRuntimeDataPlaneHardeningSupportedDefaultCutoverReport,
    RustKernelRuntimeDataPlaneHardeningSupportedDefaultCutoverVerificationReport,
    RustKernelRuntimeDataPlaneHardeningSupportedDefaultPromotionDryRunReport,
    RustKernelRuntimeDataPlaneHardeningSupportedDefaultPromotionGuardReport,
    approved_operator_default_path_cutover_fallback_scopes, approved_operator_default_path_cutover_surfaces,
};

fn collect_data_plane_hardening_surfaces(decisions: &[(&str, bool, &str)]) -> (Vec<String>, Vec<String>) {
    let mut surfaces = Vec::new();
    let mut blockers = Vec::new();

    for (surface, decision, blocker) in decisions {
        if *decision {
            surfaces.push((*surface).into());
        } else {
            blockers.push((*blocker).into());
        }
    }

    (surfaces, blockers)
}

fn data_plane_hardening_gate_check(
    name: &str,
    passed: bool,
    blockers: Vec<String>,
    fact: &str,
) -> KernelLoopbackR4ExpandedOptInLimitedRolloutGateCheck {
    KernelLoopbackR4ExpandedOptInLimitedRolloutGateCheck {
        name: name.into(),
        status: if passed { "passed" } else { "blocked" }.into(),
        passed,
        blockers,
        facts: vec![fact.into()],
    }
}

fn rust_kernel_runtime_data_plane_hardening_boundary_report(
    protocol_parity_inventory_decision: bool,
    tun_boundary_inventory_decision: bool,
    adapter_compatibility_matrix_decision: bool,
    dns_leak_verification_plan_decision: bool,
    rollback_drill_plan_decision: bool,
    opt_in_execution_boundary_decision: bool,
    operator_default_path_cutover_surfaces: Vec<String>,
    operator_default_path_cutover_fallback_scopes: Vec<String>,
) -> RustKernelRuntimeDataPlaneHardeningBoundaryReport {
    let mut blockers = Vec::new();
    let mut evidence_surfaces = Vec::new();
    let operator_default_path_cutover_committed = operator_default_path_cutover_surfaces
        .iter()
        .any(|surface| surface == "Mihomo sidecar binary removal");

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
    if operator_default_path_cutover_committed {
        evidence_surfaces.push("committed operator default-path cutover".into());
    } else {
        blockers.push(
            "Rust data-plane hardening preflight requires committed operator default-path cutover for sidecar removal"
                .into(),
        );
    }
    if operator_default_path_cutover_fallback_scopes.is_empty() {
        blockers
            .push("Rust data-plane hardening preflight requires fallback scopes recorded by operator cutover".into());
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
        operator_default_path_cutover_committed,
        operator_default_path_cutover_surfaces,
        operator_default_path_cutover_fallback_scopes,
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
        approved_operator_default_path_cutover_surfaces()
            .await?
            .into_iter()
            .map(Into::into)
            .collect(),
        approved_operator_default_path_cutover_fallback_scopes()
            .await?
            .into_iter()
            .map(Into::into)
            .collect(),
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
    operator_default_path_cutover_surfaces: Vec<String>,
    operator_default_path_cutover_fallback_scopes: Vec<String>,
) -> RustKernelRuntimeDataPlaneHardeningBoundaryAuditReport {
    let mut blockers = Vec::new();
    let mut audited_surfaces = Vec::new();
    let operator_default_path_cutover_committed = operator_default_path_cutover_surfaces
        .iter()
        .any(|surface| surface == "Mihomo sidecar binary removal");

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
    if operator_default_path_cutover_committed {
        audited_surfaces.push("committed operator default-path cutover".into());
    } else {
        blockers.push(
            "Rust data-plane boundary audit requires committed operator default-path cutover for sidecar removal"
                .into(),
        );
    }
    if operator_default_path_cutover_fallback_scopes.is_empty() {
        blockers.push("Rust data-plane boundary audit requires fallback scopes recorded by operator cutover".into());
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
        operator_default_path_cutover_committed,
        operator_default_path_cutover_surfaces,
        operator_default_path_cutover_fallback_scopes,
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
        approved_operator_default_path_cutover_surfaces()
            .await?
            .into_iter()
            .map(Into::into)
            .collect(),
        approved_operator_default_path_cutover_fallback_scopes()
            .await?
            .into_iter()
            .map(Into::into)
            .collect(),
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
    operator_default_path_cutover_surfaces: Vec<String>,
    operator_default_path_cutover_fallback_scopes: Vec<String>,
) -> RustKernelRuntimeDataPlaneHardeningOptInExecutionGuardReport {
    let mut blockers = Vec::new();
    let mut guarded_surfaces = Vec::new();
    let operator_default_path_cutover_committed = operator_default_path_cutover_surfaces
        .iter()
        .any(|surface| surface == "Mihomo sidecar binary removal");

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
    if operator_default_path_cutover_committed {
        guarded_surfaces.push("committed operator default-path cutover".into());
    } else {
        blockers.push("Rust data-plane opt-in execution guard requires committed operator default-path cutover for sidecar removal".into());
    }
    if operator_default_path_cutover_fallback_scopes.is_empty() {
        blockers.push(
            "Rust data-plane opt-in execution guard requires fallback scopes recorded by operator cutover".into(),
        );
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
        operator_default_path_cutover_committed,
        operator_default_path_cutover_surfaces,
        operator_default_path_cutover_fallback_scopes,
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
        approved_operator_default_path_cutover_surfaces()
            .await?
            .into_iter()
            .map(Into::into)
            .collect(),
        approved_operator_default_path_cutover_fallback_scopes()
            .await?
            .into_iter()
            .map(Into::into)
            .collect(),
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
    operator_default_path_cutover_surfaces: Vec<String>,
    operator_default_path_cutover_fallback_scopes: Vec<String>,
) -> RustKernelRuntimeDataPlaneHardeningOptInDryRunReport {
    let mut blockers = Vec::new();
    let mut dry_run_surfaces = Vec::new();
    let operator_default_path_cutover_committed = operator_default_path_cutover_surfaces
        .iter()
        .any(|surface| surface == "Mihomo sidecar binary removal");

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
    if operator_default_path_cutover_committed {
        dry_run_surfaces.push("committed operator default-path cutover".into());
    } else {
        blockers.push(
            "Rust data-plane opt-in dry-run requires committed operator default-path cutover for sidecar removal"
                .into(),
        );
    }
    if operator_default_path_cutover_fallback_scopes.is_empty() {
        blockers.push("Rust data-plane opt-in dry-run requires fallback scopes recorded by operator cutover".into());
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
        operator_default_path_cutover_committed,
        operator_default_path_cutover_surfaces,
        operator_default_path_cutover_fallback_scopes,
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
        approved_operator_default_path_cutover_surfaces()
            .await?
            .into_iter()
            .map(Into::into)
            .collect(),
        approved_operator_default_path_cutover_fallback_scopes()
            .await?
            .into_iter()
            .map(Into::into)
            .collect(),
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
    operator_default_path_cutover_surfaces: Vec<String>,
    operator_default_path_cutover_fallback_scopes: Vec<String>,
) -> RustKernelRuntimeDataPlaneHardeningOptInExecutionReport {
    let mut blockers = Vec::new();
    let mut execution_surfaces = Vec::new();
    let operator_default_path_cutover_committed = operator_default_path_cutover_surfaces
        .iter()
        .any(|surface| surface == "Mihomo sidecar binary removal");

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
    if operator_default_path_cutover_committed {
        execution_surfaces.push("committed operator default-path cutover".into());
    } else {
        blockers.push(
            "Rust data-plane opt-in execution requires committed operator default-path cutover for sidecar removal"
                .into(),
        );
    }
    if operator_default_path_cutover_fallback_scopes.is_empty() {
        blockers.push("Rust data-plane opt-in execution requires fallback scopes recorded by operator cutover".into());
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
        operator_default_path_cutover_committed,
        operator_default_path_cutover_surfaces,
        operator_default_path_cutover_fallback_scopes,
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
        approved_operator_default_path_cutover_surfaces()
            .await?
            .into_iter()
            .map(Into::into)
            .collect(),
        approved_operator_default_path_cutover_fallback_scopes()
            .await?
            .into_iter()
            .map(Into::into)
            .collect(),
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
    operator_default_path_cutover_surfaces: Vec<String>,
    operator_default_path_cutover_fallback_scopes: Vec<String>,
) -> RustKernelRuntimeDataPlaneHardeningOptInExecutionVerificationReport {
    let mut blockers = Vec::new();
    let mut verification_surfaces = Vec::new();
    let operator_default_path_cutover_committed = operator_default_path_cutover_surfaces
        .iter()
        .any(|surface| surface == "Mihomo sidecar binary removal");

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
    if operator_default_path_cutover_committed {
        verification_surfaces.push("committed operator default-path cutover".into());
    } else {
        blockers.push("Rust data-plane opt-in execution verification requires committed operator default-path cutover for sidecar removal".into());
    }
    if operator_default_path_cutover_fallback_scopes.is_empty() {
        blockers.push(
            "Rust data-plane opt-in execution verification requires fallback scopes recorded by operator cutover"
                .into(),
        );
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
        operator_default_path_cutover_committed,
        operator_default_path_cutover_surfaces,
        operator_default_path_cutover_fallback_scopes,
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
        approved_operator_default_path_cutover_surfaces()
            .await?
            .into_iter()
            .map(Into::into)
            .collect(),
        approved_operator_default_path_cutover_fallback_scopes()
            .await?
            .into_iter()
            .map(Into::into)
            .collect(),
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

fn rust_kernel_runtime_data_plane_hardening_controlled_rollout_canary_execution_report(
    readiness_closeout_review_decision: bool,
    execution_manifest_lock_decision: bool,
    canary_window_start_decision: bool,
    canary_population_cap_enforcement_decision: bool,
    health_telemetry_activation_decision: bool,
    automatic_fallback_arm_decision: bool,
    mihomo_fallback_retention_decision: bool,
    production_mutation_guard_retention_decision: bool,
    operator_canary_execution_acknowledgement_decision: bool,
) -> RustKernelRuntimeDataPlaneHardeningControlledRolloutCanaryExecutionReport {
    let (execution_surfaces, blockers) = collect_data_plane_hardening_surfaces(&[
        (
            "controlled rollout readiness closeout review",
            readiness_closeout_review_decision,
            "Rust data-plane controlled rollout canary execution requires readiness closeout review",
        ),
        (
            "locked canary execution manifest",
            execution_manifest_lock_decision,
            "Rust data-plane controlled rollout canary execution requires a locked execution manifest",
        ),
        (
            "started capped canary window",
            canary_window_start_decision,
            "Rust data-plane controlled rollout canary execution requires a started canary window",
        ),
        (
            "enforced canary population cap",
            canary_population_cap_enforcement_decision,
            "Rust data-plane controlled rollout canary execution requires enforced canary cap",
        ),
        (
            "active health telemetry",
            health_telemetry_activation_decision,
            "Rust data-plane controlled rollout canary execution requires active health telemetry",
        ),
        (
            "armed automatic fallback",
            automatic_fallback_arm_decision,
            "Rust data-plane controlled rollout canary execution requires armed automatic fallback",
        ),
        (
            "retained Mihomo fallback",
            mihomo_fallback_retention_decision,
            "Rust data-plane controlled rollout canary execution requires retained Mihomo fallback",
        ),
        (
            "retained production mutation guard",
            production_mutation_guard_retention_decision,
            "Rust data-plane controlled rollout canary execution requires retained production mutation guard",
        ),
        (
            "operator canary execution acknowledgement",
            operator_canary_execution_acknowledgement_decision,
            "Rust data-plane controlled rollout canary execution requires operator acknowledgement",
        ),
    ]);

    RustKernelRuntimeDataPlaneHardeningControlledRolloutCanaryExecutionReport {
        runtime_id: RUST_RUNTIME_ID.into(),
        component: "rust-data-plane-hardening-controlled-rollout-canary-execution-detail".into(),
        readiness_closeout_reviewed: readiness_closeout_review_decision,
        execution_manifest_locked: execution_manifest_lock_decision,
        canary_window_started: canary_window_start_decision,
        canary_population_cap_enforced: canary_population_cap_enforcement_decision,
        health_telemetry_active: health_telemetry_activation_decision,
        automatic_fallback_armed: automatic_fallback_arm_decision,
        mihomo_fallback_retained: mihomo_fallback_retention_decision,
        production_mutation_guard_retained: production_mutation_guard_retention_decision,
        operator_canary_execution_acknowledged:
            operator_canary_execution_acknowledgement_decision,
        controlled_rollout_canary_execution_complete: blockers.is_empty(),
        execution_surfaces,
        blockers,
        facts: vec![
            "controlled rollout canary execution records the capped execution envelope without widening default selection".into(),
            "Mihomo fallback and the production mutation guard remain mandatory during canary execution".into(),
        ],
    }
}

pub async fn rust_kernel_runtime_data_plane_hardening_controlled_rollout_canary_execution(
    rust_data_plane_hardening_controlled_rollout_readiness_closeout_complete_decision: Option<bool>,
    readiness_closeout_review_decision: Option<bool>,
    execution_manifest_lock_decision: Option<bool>,
    canary_window_start_decision: Option<bool>,
    canary_population_cap_enforcement_decision: Option<bool>,
    health_telemetry_activation_decision: Option<bool>,
    automatic_fallback_arm_decision: Option<bool>,
    mihomo_fallback_retention_decision: Option<bool>,
    production_mutation_guard_retention_decision: Option<bool>,
    operator_canary_execution_acknowledgement_decision: Option<bool>,
    final_controlled_rollout_canary_execution_decision: Option<bool>,
) -> Result<KernelLoopbackRustDataPlaneHardeningControlledRolloutCanaryExecutionReport> {
    let rust_data_plane_hardening_controlled_rollout_readiness_closeout_complete =
        rust_data_plane_hardening_controlled_rollout_readiness_closeout_complete_decision.unwrap_or(false);
    let final_controlled_rollout_canary_execution_decision =
        final_controlled_rollout_canary_execution_decision.unwrap_or(false);
    let controlled_rollout_canary_execution =
        rust_kernel_runtime_data_plane_hardening_controlled_rollout_canary_execution_report(
            readiness_closeout_review_decision.unwrap_or(false),
            execution_manifest_lock_decision.unwrap_or(false),
            canary_window_start_decision.unwrap_or(false),
            canary_population_cap_enforcement_decision.unwrap_or(false),
            health_telemetry_activation_decision.unwrap_or(false),
            automatic_fallback_arm_decision.unwrap_or(false),
            mihomo_fallback_retention_decision.unwrap_or(false),
            production_mutation_guard_retention_decision.unwrap_or(false),
            operator_canary_execution_acknowledgement_decision.unwrap_or(false),
        );
    let readiness_blockers = if rust_data_plane_hardening_controlled_rollout_readiness_closeout_complete {
        Vec::new()
    } else {
        vec!["Rust data-plane controlled rollout canary execution requires readiness closeout to pass first".into()]
    };

    let checks = vec![
        data_plane_hardening_gate_check(
            "rustDataPlaneHardeningControlledRolloutReadinessCloseoutComplete",
            rust_data_plane_hardening_controlled_rollout_readiness_closeout_complete,
            readiness_blockers,
            "canary execution starts only after controlled rollout readiness closeout",
        ),
        data_plane_hardening_gate_check(
            "controlledRolloutCanaryExecutionComplete",
            controlled_rollout_canary_execution.controlled_rollout_canary_execution_complete,
            controlled_rollout_canary_execution.blockers.clone(),
            "canary manifest, capped window, telemetry, fallback, mutation guard, and acknowledgement are evaluated together",
        ),
        data_plane_hardening_gate_check(
            "finalControlledRolloutCanaryExecutionDecision",
            final_controlled_rollout_canary_execution_decision,
            if final_controlled_rollout_canary_execution_decision {
                Vec::new()
            } else {
                vec!["Rust data-plane controlled rollout canary execution requires an explicit final decision".into()]
            },
            "canary execution completion is explicit before verification",
        ),
    ];
    let rust_data_plane_hardening_controlled_rollout_canary_execution_complete =
        checks.iter().all(|check| check.passed);
    let blockers = checks
        .iter()
        .flat_map(|check| check.blockers.clone())
        .collect::<Vec<String>>();

    Ok(
        KernelLoopbackRustDataPlaneHardeningControlledRolloutCanaryExecutionReport {
            runtime_id: RUST_RUNTIME_ID.into(),
            component: "rust-data-plane-hardening-controlled-rollout-canary-execution".into(),
            mutates_runtime: false,
            live_execution_allowed: false,
            production_data_plane_mutation_allowed: false,
            rust_data_plane_hardening_controlled_rollout_readiness_closeout_complete,
            controlled_rollout_canary_execution,
            final_controlled_rollout_canary_execution_decision,
            rust_data_plane_hardening_controlled_rollout_canary_execution_complete,
            selected_runtime_kind:
                if rust_data_plane_hardening_controlled_rollout_canary_execution_complete {
                    KernelRuntimeKind::Rust
                } else {
                    KernelRuntimeKind::Mihomo
                },
            rollback_runtime_kind: KernelRuntimeKind::Mihomo,
            checks,
            blockers,
            warnings: vec![
                "this canary execution surface does not mutate runtime, routes, TUN, DNS, adapter forwarding, or Mihomo config".into(),
                "canary execution evidence does not remove Mihomo fallback or authorize supported default promotion".into(),
            ],
            facts: vec![
                "Rust data-plane hardening controlled rollout canary execution follows readiness closeout".into(),
                "successful canary execution advances only to verification".into(),
            ],
            next_safe_batch:
                if rust_data_plane_hardening_controlled_rollout_canary_execution_complete {
                    "rust-data-plane-hardening-controlled-rollout-canary-verification".into()
                } else {
                    "rust-data-plane-hardening-controlled-rollout-canary-execution".into()
                },
        },
    )
}

fn rust_kernel_runtime_data_plane_hardening_controlled_rollout_canary_verification_report(
    execution_record_review_decision: bool,
    health_telemetry_sample_review_decision: bool,
    automatic_fallback_result_review_decision: bool,
    unsupported_traffic_fallback_verification_decision: bool,
    leak_regression_absence_verification_decision: bool,
    rollback_readiness_verification_decision: bool,
    production_mutation_guard_retention_verification_decision: bool,
    verification_evidence_archive_decision: bool,
) -> RustKernelRuntimeDataPlaneHardeningControlledRolloutCanaryVerificationReport {
    let (verification_surfaces, blockers) = collect_data_plane_hardening_surfaces(&[
        (
            "canary execution record review",
            execution_record_review_decision,
            "Rust data-plane controlled rollout canary verification requires execution record review",
        ),
        (
            "health telemetry sample review",
            health_telemetry_sample_review_decision,
            "Rust data-plane controlled rollout canary verification requires health telemetry sample review",
        ),
        (
            "automatic fallback result review",
            automatic_fallback_result_review_decision,
            "Rust data-plane controlled rollout canary verification requires automatic fallback result review",
        ),
        (
            "unsupported traffic fallback verification",
            unsupported_traffic_fallback_verification_decision,
            "Rust data-plane controlled rollout canary verification requires unsupported traffic fallback verification",
        ),
        (
            "leak regression absence verification",
            leak_regression_absence_verification_decision,
            "Rust data-plane controlled rollout canary verification requires leak regression absence verification",
        ),
        (
            "rollback readiness verification",
            rollback_readiness_verification_decision,
            "Rust data-plane controlled rollout canary verification requires rollback readiness verification",
        ),
        (
            "retained production mutation guard verification",
            production_mutation_guard_retention_verification_decision,
            "Rust data-plane controlled rollout canary verification requires retained production mutation guard verification",
        ),
        (
            "archived canary verification evidence",
            verification_evidence_archive_decision,
            "Rust data-plane controlled rollout canary verification requires archived evidence",
        ),
    ]);

    RustKernelRuntimeDataPlaneHardeningControlledRolloutCanaryVerificationReport {
        runtime_id: RUST_RUNTIME_ID.into(),
        component: "rust-data-plane-hardening-controlled-rollout-canary-verification-detail".into(),
        execution_record_reviewed: execution_record_review_decision,
        health_telemetry_sample_reviewed: health_telemetry_sample_review_decision,
        automatic_fallback_result_reviewed: automatic_fallback_result_review_decision,
        unsupported_traffic_fallback_verified: unsupported_traffic_fallback_verification_decision,
        leak_regression_absence_verified: leak_regression_absence_verification_decision,
        rollback_readiness_verified: rollback_readiness_verification_decision,
        production_mutation_guard_still_retained:
            production_mutation_guard_retention_verification_decision,
        verification_evidence_archived: verification_evidence_archive_decision,
        controlled_rollout_canary_verification_complete: blockers.is_empty(),
        verification_surfaces,
        blockers,
        facts: vec![
            "controlled rollout canary verification reviews health, fallback, leak, rollback, and mutation guard evidence together".into(),
            "verification success advances only to supported default promotion guard planning".into(),
        ],
    }
}

pub async fn rust_kernel_runtime_data_plane_hardening_controlled_rollout_canary_verification(
    rust_data_plane_hardening_controlled_rollout_canary_execution_complete_decision: Option<bool>,
    execution_record_review_decision: Option<bool>,
    health_telemetry_sample_review_decision: Option<bool>,
    automatic_fallback_result_review_decision: Option<bool>,
    unsupported_traffic_fallback_verification_decision: Option<bool>,
    leak_regression_absence_verification_decision: Option<bool>,
    rollback_readiness_verification_decision: Option<bool>,
    production_mutation_guard_retention_verification_decision: Option<bool>,
    verification_evidence_archive_decision: Option<bool>,
    final_controlled_rollout_canary_verification_decision: Option<bool>,
) -> Result<KernelLoopbackRustDataPlaneHardeningControlledRolloutCanaryVerificationReport> {
    let rust_data_plane_hardening_controlled_rollout_canary_execution_complete =
        rust_data_plane_hardening_controlled_rollout_canary_execution_complete_decision.unwrap_or(false);
    let final_controlled_rollout_canary_verification_decision =
        final_controlled_rollout_canary_verification_decision.unwrap_or(false);
    let controlled_rollout_canary_verification =
        rust_kernel_runtime_data_plane_hardening_controlled_rollout_canary_verification_report(
            execution_record_review_decision.unwrap_or(false),
            health_telemetry_sample_review_decision.unwrap_or(false),
            automatic_fallback_result_review_decision.unwrap_or(false),
            unsupported_traffic_fallback_verification_decision.unwrap_or(false),
            leak_regression_absence_verification_decision.unwrap_or(false),
            rollback_readiness_verification_decision.unwrap_or(false),
            production_mutation_guard_retention_verification_decision.unwrap_or(false),
            verification_evidence_archive_decision.unwrap_or(false),
        );
    let execution_blockers = if rust_data_plane_hardening_controlled_rollout_canary_execution_complete {
        Vec::new()
    } else {
        vec!["Rust data-plane controlled rollout canary verification requires canary execution to pass first".into()]
    };

    let checks = vec![
        data_plane_hardening_gate_check(
            "rustDataPlaneHardeningControlledRolloutCanaryExecutionComplete",
            rust_data_plane_hardening_controlled_rollout_canary_execution_complete,
            execution_blockers,
            "canary verification starts only after canary execution",
        ),
        data_plane_hardening_gate_check(
            "controlledRolloutCanaryVerificationComplete",
            controlled_rollout_canary_verification.controlled_rollout_canary_verification_complete,
            controlled_rollout_canary_verification.blockers.clone(),
            "canary health, fallback, leak, rollback, mutation guard, and evidence are evaluated together",
        ),
        data_plane_hardening_gate_check(
            "finalControlledRolloutCanaryVerificationDecision",
            final_controlled_rollout_canary_verification_decision,
            if final_controlled_rollout_canary_verification_decision {
                Vec::new()
            } else {
                vec![
                    "Rust data-plane controlled rollout canary verification requires an explicit final decision".into(),
                ]
            },
            "canary verification completion is explicit before supported default promotion guard",
        ),
    ];
    let rust_data_plane_hardening_controlled_rollout_canary_verification_complete =
        checks.iter().all(|check| check.passed);
    let blockers = checks
        .iter()
        .flat_map(|check| check.blockers.clone())
        .collect::<Vec<String>>();

    Ok(
        KernelLoopbackRustDataPlaneHardeningControlledRolloutCanaryVerificationReport {
            runtime_id: RUST_RUNTIME_ID.into(),
            component: "rust-data-plane-hardening-controlled-rollout-canary-verification".into(),
            mutates_runtime: false,
            live_execution_allowed: false,
            production_data_plane_mutation_allowed: false,
            rust_data_plane_hardening_controlled_rollout_canary_execution_complete,
            controlled_rollout_canary_verification,
            final_controlled_rollout_canary_verification_decision,
            rust_data_plane_hardening_controlled_rollout_canary_verification_complete,
            selected_runtime_kind:
                if rust_data_plane_hardening_controlled_rollout_canary_verification_complete {
                    KernelRuntimeKind::Rust
                } else {
                    KernelRuntimeKind::Mihomo
                },
            rollback_runtime_kind: KernelRuntimeKind::Mihomo,
            checks,
            blockers,
            warnings: vec![
                "this canary verification surface does not mutate runtime, routes, TUN, DNS, adapter forwarding, or Mihomo config".into(),
                "supported default promotion remains blocked by a separate guard and dry-run".into(),
            ],
            facts: vec![
                "Rust data-plane hardening controlled rollout canary verification follows canary execution".into(),
                "successful canary verification advances only to supported default promotion guard".into(),
            ],
            next_safe_batch:
                if rust_data_plane_hardening_controlled_rollout_canary_verification_complete {
                    "rust-data-plane-hardening-supported-default-promotion-guard".into()
                } else {
                    "rust-data-plane-hardening-controlled-rollout-canary-verification".into()
                },
        },
    )
}

fn rust_kernel_runtime_data_plane_hardening_supported_default_promotion_guard_report(
    canary_verification_review_decision: bool,
    supported_profile_scope_lock_decision: bool,
    fallback_matrix_retention_decision: bool,
    rollback_switch_verification_decision: bool,
    telemetry_soak_window_definition_decision: bool,
    release_blocker_review_decision: bool,
    production_mutation_guard_retention_decision: bool,
    operator_promotion_acknowledgement_decision: bool,
) -> RustKernelRuntimeDataPlaneHardeningSupportedDefaultPromotionGuardReport {
    let (guard_surfaces, blockers) = collect_data_plane_hardening_surfaces(&[
        (
            "controlled canary verification review",
            canary_verification_review_decision,
            "Rust data-plane supported default promotion guard requires canary verification review",
        ),
        (
            "locked supported profile scope",
            supported_profile_scope_lock_decision,
            "Rust data-plane supported default promotion guard requires locked supported profile scope",
        ),
        (
            "retained fallback matrix",
            fallback_matrix_retention_decision,
            "Rust data-plane supported default promotion guard requires retained fallback matrix",
        ),
        (
            "verified rollback switch",
            rollback_switch_verification_decision,
            "Rust data-plane supported default promotion guard requires verified rollback switch",
        ),
        (
            "telemetry soak window",
            telemetry_soak_window_definition_decision,
            "Rust data-plane supported default promotion guard requires telemetry soak window",
        ),
        (
            "release blocker review",
            release_blocker_review_decision,
            "Rust data-plane supported default promotion guard requires release blocker review",
        ),
        (
            "retained production mutation guard",
            production_mutation_guard_retention_decision,
            "Rust data-plane supported default promotion guard requires retained production mutation guard",
        ),
        (
            "operator promotion acknowledgement",
            operator_promotion_acknowledgement_decision,
            "Rust data-plane supported default promotion guard requires operator acknowledgement",
        ),
    ]);

    RustKernelRuntimeDataPlaneHardeningSupportedDefaultPromotionGuardReport {
        runtime_id: RUST_RUNTIME_ID.into(),
        component: "rust-data-plane-hardening-supported-default-promotion-guard-detail".into(),
        canary_verification_reviewed: canary_verification_review_decision,
        supported_profile_scope_locked: supported_profile_scope_lock_decision,
        fallback_matrix_retained: fallback_matrix_retention_decision,
        rollback_switch_verified: rollback_switch_verification_decision,
        telemetry_soak_window_defined: telemetry_soak_window_definition_decision,
        release_blocker_reviewed: release_blocker_review_decision,
        production_mutation_guard_retained: production_mutation_guard_retention_decision,
        operator_promotion_acknowledged: operator_promotion_acknowledgement_decision,
        supported_default_promotion_guard_complete: blockers.is_empty(),
        guard_surfaces,
        blockers,
        facts: vec![
            "supported default promotion guard locks the supported profile scope before any default selection change"
                .into(),
            "unsupported protocol, TUN, adapter, and emergency rollback paths remain on Mihomo fallback".into(),
        ],
    }
}

pub async fn rust_kernel_runtime_data_plane_hardening_supported_default_promotion_guard(
    rust_data_plane_hardening_controlled_rollout_canary_verification_complete_decision: Option<bool>,
    canary_verification_review_decision: Option<bool>,
    supported_profile_scope_lock_decision: Option<bool>,
    fallback_matrix_retention_decision: Option<bool>,
    rollback_switch_verification_decision: Option<bool>,
    telemetry_soak_window_definition_decision: Option<bool>,
    release_blocker_review_decision: Option<bool>,
    production_mutation_guard_retention_decision: Option<bool>,
    operator_promotion_acknowledgement_decision: Option<bool>,
    final_supported_default_promotion_guard_decision: Option<bool>,
) -> Result<KernelLoopbackRustDataPlaneHardeningSupportedDefaultPromotionGuardReport> {
    let rust_data_plane_hardening_controlled_rollout_canary_verification_complete =
        rust_data_plane_hardening_controlled_rollout_canary_verification_complete_decision.unwrap_or(false);
    let final_supported_default_promotion_guard_decision =
        final_supported_default_promotion_guard_decision.unwrap_or(false);
    let supported_default_promotion_guard =
        rust_kernel_runtime_data_plane_hardening_supported_default_promotion_guard_report(
            canary_verification_review_decision.unwrap_or(false),
            supported_profile_scope_lock_decision.unwrap_or(false),
            fallback_matrix_retention_decision.unwrap_or(false),
            rollback_switch_verification_decision.unwrap_or(false),
            telemetry_soak_window_definition_decision.unwrap_or(false),
            release_blocker_review_decision.unwrap_or(false),
            production_mutation_guard_retention_decision.unwrap_or(false),
            operator_promotion_acknowledgement_decision.unwrap_or(false),
        );
    let canary_blockers = if rust_data_plane_hardening_controlled_rollout_canary_verification_complete {
        Vec::new()
    } else {
        vec!["Rust data-plane supported default promotion guard requires canary verification to pass first".into()]
    };

    let checks = vec![
        data_plane_hardening_gate_check(
            "rustDataPlaneHardeningControlledRolloutCanaryVerificationComplete",
            rust_data_plane_hardening_controlled_rollout_canary_verification_complete,
            canary_blockers,
            "supported default promotion guard starts only after canary verification",
        ),
        data_plane_hardening_gate_check(
            "supportedDefaultPromotionGuardComplete",
            supported_default_promotion_guard.supported_default_promotion_guard_complete,
            supported_default_promotion_guard.blockers.clone(),
            "supported scope, fallback matrix, rollback, telemetry, release blocker, mutation guard, and acknowledgement are evaluated together",
        ),
        data_plane_hardening_gate_check(
            "finalSupportedDefaultPromotionGuardDecision",
            final_supported_default_promotion_guard_decision,
            if final_supported_default_promotion_guard_decision {
                Vec::new()
            } else {
                vec!["Rust data-plane supported default promotion guard requires an explicit final decision".into()]
            },
            "supported default promotion guard completion is explicit before dry-run",
        ),
    ];
    let rust_data_plane_hardening_supported_default_promotion_guard_complete = checks.iter().all(|check| check.passed);
    let blockers = checks
        .iter()
        .flat_map(|check| check.blockers.clone())
        .collect::<Vec<String>>();

    Ok(
        KernelLoopbackRustDataPlaneHardeningSupportedDefaultPromotionGuardReport {
            runtime_id: RUST_RUNTIME_ID.into(),
            component: "rust-data-plane-hardening-supported-default-promotion-guard".into(),
            mutates_runtime: false,
            live_execution_allowed: false,
            production_data_plane_mutation_allowed: false,
            rust_data_plane_hardening_controlled_rollout_canary_verification_complete,
            supported_default_promotion_guard,
            final_supported_default_promotion_guard_decision,
            rust_data_plane_hardening_supported_default_promotion_guard_complete,
            selected_runtime_kind:
                if rust_data_plane_hardening_supported_default_promotion_guard_complete {
                    KernelRuntimeKind::Rust
                } else {
                    KernelRuntimeKind::Mihomo
                },
            rollback_runtime_kind: KernelRuntimeKind::Mihomo,
            checks,
            blockers,
            warnings: vec![
                "this supported default promotion guard does not mutate runtime, routes, TUN, DNS, adapter forwarding, or Mihomo config".into(),
                "promotion remains blocked until a separate dry-run and explicit cutover surface pass".into(),
            ],
            facts: vec![
                "Rust data-plane hardening supported default promotion guard follows canary verification".into(),
                "successful guard completion advances only to supported default promotion dry-run".into(),
            ],
            next_safe_batch: if rust_data_plane_hardening_supported_default_promotion_guard_complete
            {
                "rust-data-plane-hardening-supported-default-promotion-dry-run".into()
            } else {
                "rust-data-plane-hardening-supported-default-promotion-guard".into()
            },
        },
    )
}

fn rust_kernel_runtime_data_plane_hardening_supported_default_promotion_dry_run_report(
    guard_review_decision: bool,
    default_selection_manifest_replay_decision: bool,
    supported_profile_simulation_decision: bool,
    fallback_decision_rehearsal_decision: bool,
    rollback_rehearsal_decision: bool,
    production_forwarding_unchanged_verification_decision: bool,
    dry_run_evidence_archive_decision: bool,
) -> RustKernelRuntimeDataPlaneHardeningSupportedDefaultPromotionDryRunReport {
    let (dry_run_surfaces, blockers) = collect_data_plane_hardening_surfaces(&[
        (
            "supported default promotion guard review",
            guard_review_decision,
            "Rust data-plane supported default promotion dry-run requires guard review",
        ),
        (
            "default selection manifest replay",
            default_selection_manifest_replay_decision,
            "Rust data-plane supported default promotion dry-run requires default selection manifest replay",
        ),
        (
            "supported profile simulation",
            supported_profile_simulation_decision,
            "Rust data-plane supported default promotion dry-run requires supported profile simulation",
        ),
        (
            "fallback decision rehearsal",
            fallback_decision_rehearsal_decision,
            "Rust data-plane supported default promotion dry-run requires fallback decision rehearsal",
        ),
        (
            "rollback rehearsal",
            rollback_rehearsal_decision,
            "Rust data-plane supported default promotion dry-run requires rollback rehearsal",
        ),
        (
            "production forwarding unchanged verification",
            production_forwarding_unchanged_verification_decision,
            "Rust data-plane supported default promotion dry-run requires unchanged production forwarding verification",
        ),
        (
            "archived supported default promotion dry-run evidence",
            dry_run_evidence_archive_decision,
            "Rust data-plane supported default promotion dry-run requires archived evidence",
        ),
    ]);

    RustKernelRuntimeDataPlaneHardeningSupportedDefaultPromotionDryRunReport {
        runtime_id: RUST_RUNTIME_ID.into(),
        component: "rust-data-plane-hardening-supported-default-promotion-dry-run-detail".into(),
        guard_reviewed: guard_review_decision,
        default_selection_manifest_replayed: default_selection_manifest_replay_decision,
        supported_profile_simulation_completed: supported_profile_simulation_decision,
        fallback_decision_rehearsed: fallback_decision_rehearsal_decision,
        rollback_rehearsed: rollback_rehearsal_decision,
        production_forwarding_unchanged_verified: production_forwarding_unchanged_verification_decision,
        dry_run_evidence_archived: dry_run_evidence_archive_decision,
        supported_default_promotion_dry_run_complete: blockers.is_empty(),
        dry_run_surfaces,
        blockers,
        facts: vec![
            "supported default promotion dry-run replays the Rust default selection manifest without applying it"
                .into(),
            "dry-run success advances only to a separate supported default cutover surface".into(),
        ],
    }
}

pub async fn rust_kernel_runtime_data_plane_hardening_supported_default_promotion_dry_run(
    rust_data_plane_hardening_supported_default_promotion_guard_complete_decision: Option<bool>,
    guard_review_decision: Option<bool>,
    default_selection_manifest_replay_decision: Option<bool>,
    supported_profile_simulation_decision: Option<bool>,
    fallback_decision_rehearsal_decision: Option<bool>,
    rollback_rehearsal_decision: Option<bool>,
    production_forwarding_unchanged_verification_decision: Option<bool>,
    dry_run_evidence_archive_decision: Option<bool>,
    final_supported_default_promotion_dry_run_decision: Option<bool>,
) -> Result<KernelLoopbackRustDataPlaneHardeningSupportedDefaultPromotionDryRunReport> {
    let rust_data_plane_hardening_supported_default_promotion_guard_complete =
        rust_data_plane_hardening_supported_default_promotion_guard_complete_decision.unwrap_or(false);
    let final_supported_default_promotion_dry_run_decision =
        final_supported_default_promotion_dry_run_decision.unwrap_or(false);
    let supported_default_promotion_dry_run =
        rust_kernel_runtime_data_plane_hardening_supported_default_promotion_dry_run_report(
            guard_review_decision.unwrap_or(false),
            default_selection_manifest_replay_decision.unwrap_or(false),
            supported_profile_simulation_decision.unwrap_or(false),
            fallback_decision_rehearsal_decision.unwrap_or(false),
            rollback_rehearsal_decision.unwrap_or(false),
            production_forwarding_unchanged_verification_decision.unwrap_or(false),
            dry_run_evidence_archive_decision.unwrap_or(false),
        );
    let guard_blockers = if rust_data_plane_hardening_supported_default_promotion_guard_complete {
        Vec::new()
    } else {
        vec!["Rust data-plane supported default promotion dry-run requires promotion guard to pass first".into()]
    };

    let checks = vec![
        data_plane_hardening_gate_check(
            "rustDataPlaneHardeningSupportedDefaultPromotionGuardComplete",
            rust_data_plane_hardening_supported_default_promotion_guard_complete,
            guard_blockers,
            "supported default promotion dry-run starts only after the promotion guard",
        ),
        data_plane_hardening_gate_check(
            "supportedDefaultPromotionDryRunComplete",
            supported_default_promotion_dry_run.supported_default_promotion_dry_run_complete,
            supported_default_promotion_dry_run.blockers.clone(),
            "manifest replay, supported profile simulation, fallback, rollback, unchanged forwarding, and evidence are evaluated together",
        ),
        data_plane_hardening_gate_check(
            "finalSupportedDefaultPromotionDryRunDecision",
            final_supported_default_promotion_dry_run_decision,
            if final_supported_default_promotion_dry_run_decision {
                Vec::new()
            } else {
                vec!["Rust data-plane supported default promotion dry-run requires an explicit final decision".into()]
            },
            "supported default promotion dry-run completion is explicit before cutover",
        ),
    ];
    let rust_data_plane_hardening_supported_default_promotion_dry_run_complete =
        checks.iter().all(|check| check.passed);
    let blockers = checks
        .iter()
        .flat_map(|check| check.blockers.clone())
        .collect::<Vec<String>>();

    Ok(
        KernelLoopbackRustDataPlaneHardeningSupportedDefaultPromotionDryRunReport {
            runtime_id: RUST_RUNTIME_ID.into(),
            component: "rust-data-plane-hardening-supported-default-promotion-dry-run".into(),
            mutates_runtime: false,
            live_execution_allowed: false,
            production_data_plane_mutation_allowed: false,
            rust_data_plane_hardening_supported_default_promotion_guard_complete,
            supported_default_promotion_dry_run,
            final_supported_default_promotion_dry_run_decision,
            rust_data_plane_hardening_supported_default_promotion_dry_run_complete,
            selected_runtime_kind:
                if rust_data_plane_hardening_supported_default_promotion_dry_run_complete {
                    KernelRuntimeKind::Rust
                } else {
                    KernelRuntimeKind::Mihomo
                },
            rollback_runtime_kind: KernelRuntimeKind::Mihomo,
            checks,
            blockers,
            warnings: vec![
                "this supported default promotion dry-run does not mutate runtime, routes, TUN, DNS, adapter forwarding, or Mihomo config".into(),
                "a later cutover PR must preserve explicit rollback and Mihomo fallback for unsupported paths".into(),
            ],
            facts: vec![
                "Rust data-plane hardening supported default promotion dry-run follows the promotion guard".into(),
                "successful dry-run advances only to supported default cutover".into(),
            ],
            next_safe_batch: if rust_data_plane_hardening_supported_default_promotion_dry_run_complete
            {
                "rust-data-plane-hardening-supported-default-cutover".into()
            } else {
                "rust-data-plane-hardening-supported-default-promotion-dry-run".into()
            },
        },
    )
}

fn rust_kernel_runtime_data_plane_hardening_supported_default_cutover_report(
    dry_run_review_decision: bool,
    cutover_manifest_lock_decision: bool,
    supported_profile_default_selection_confirmation_decision: bool,
    unsupported_paths_mihomo_fallback_binding_decision: bool,
    rollback_switch_arm_decision: bool,
    telemetry_soak_watch_activation_decision: bool,
    operator_cutover_acknowledgement_decision: bool,
    production_mutation_guard_transition_record_decision: bool,
) -> RustKernelRuntimeDataPlaneHardeningSupportedDefaultCutoverReport {
    let (cutover_surfaces, blockers) = collect_data_plane_hardening_surfaces(&[
        (
            "supported default promotion dry-run review",
            dry_run_review_decision,
            "Rust data-plane supported default cutover requires promotion dry-run review",
        ),
        (
            "locked supported default cutover manifest",
            cutover_manifest_lock_decision,
            "Rust data-plane supported default cutover requires a locked cutover manifest",
        ),
        (
            "supported profile default selection confirmation",
            supported_profile_default_selection_confirmation_decision,
            "Rust data-plane supported default cutover requires supported profile default selection confirmation",
        ),
        (
            "unsupported paths bound to Mihomo fallback",
            unsupported_paths_mihomo_fallback_binding_decision,
            "Rust data-plane supported default cutover requires unsupported paths to remain bound to Mihomo fallback",
        ),
        (
            "armed rollback switch",
            rollback_switch_arm_decision,
            "Rust data-plane supported default cutover requires an armed rollback switch",
        ),
        (
            "active telemetry soak watch",
            telemetry_soak_watch_activation_decision,
            "Rust data-plane supported default cutover requires active telemetry soak watch",
        ),
        (
            "operator supported default cutover acknowledgement",
            operator_cutover_acknowledgement_decision,
            "Rust data-plane supported default cutover requires operator acknowledgement",
        ),
        (
            "production mutation guard transition record",
            production_mutation_guard_transition_record_decision,
            "Rust data-plane supported default cutover requires a recorded production mutation guard transition",
        ),
    ]);

    RustKernelRuntimeDataPlaneHardeningSupportedDefaultCutoverReport {
        runtime_id: RUST_RUNTIME_ID.into(),
        component: "rust-data-plane-hardening-supported-default-cutover-detail".into(),
        dry_run_reviewed: dry_run_review_decision,
        cutover_manifest_locked: cutover_manifest_lock_decision,
        supported_profile_default_selection_confirmed:
            supported_profile_default_selection_confirmation_decision,
        unsupported_paths_bound_to_mihomo_fallback:
            unsupported_paths_mihomo_fallback_binding_decision,
        rollback_switch_armed: rollback_switch_arm_decision,
        telemetry_soak_watch_active: telemetry_soak_watch_activation_decision,
        operator_cutover_acknowledged: operator_cutover_acknowledgement_decision,
        production_mutation_guard_transition_recorded:
            production_mutation_guard_transition_record_decision,
        supported_default_cutover_complete: blockers.is_empty(),
        cutover_surfaces,
        blockers,
        facts: vec![
            "supported default cutover is limited to the supported profile and keeps unsupported paths on Mihomo fallback".into(),
            "the command records the gated cutover envelope; rollback remains one switch back to Mihomo".into(),
        ],
    }
}

pub async fn rust_kernel_runtime_data_plane_hardening_supported_default_cutover(
    rust_data_plane_hardening_supported_default_promotion_dry_run_complete_decision: Option<bool>,
    dry_run_review_decision: Option<bool>,
    cutover_manifest_lock_decision: Option<bool>,
    supported_profile_default_selection_confirmation_decision: Option<bool>,
    unsupported_paths_mihomo_fallback_binding_decision: Option<bool>,
    rollback_switch_arm_decision: Option<bool>,
    telemetry_soak_watch_activation_decision: Option<bool>,
    operator_cutover_acknowledgement_decision: Option<bool>,
    production_mutation_guard_transition_record_decision: Option<bool>,
    final_supported_default_cutover_decision: Option<bool>,
) -> Result<KernelLoopbackRustDataPlaneHardeningSupportedDefaultCutoverReport> {
    let rust_data_plane_hardening_supported_default_promotion_dry_run_complete =
        rust_data_plane_hardening_supported_default_promotion_dry_run_complete_decision.unwrap_or(false);
    let final_supported_default_cutover_decision = final_supported_default_cutover_decision.unwrap_or(false);
    let supported_default_cutover = rust_kernel_runtime_data_plane_hardening_supported_default_cutover_report(
        dry_run_review_decision.unwrap_or(false),
        cutover_manifest_lock_decision.unwrap_or(false),
        supported_profile_default_selection_confirmation_decision.unwrap_or(false),
        unsupported_paths_mihomo_fallback_binding_decision.unwrap_or(false),
        rollback_switch_arm_decision.unwrap_or(false),
        telemetry_soak_watch_activation_decision.unwrap_or(false),
        operator_cutover_acknowledgement_decision.unwrap_or(false),
        production_mutation_guard_transition_record_decision.unwrap_or(false),
    );
    let dry_run_blockers = if rust_data_plane_hardening_supported_default_promotion_dry_run_complete {
        Vec::new()
    } else {
        vec!["Rust data-plane supported default cutover requires promotion dry-run to pass first".into()]
    };

    let checks = vec![
        data_plane_hardening_gate_check(
            "rustDataPlaneHardeningSupportedDefaultPromotionDryRunComplete",
            rust_data_plane_hardening_supported_default_promotion_dry_run_complete,
            dry_run_blockers,
            "supported default cutover starts only after promotion dry-run",
        ),
        data_plane_hardening_gate_check(
            "supportedDefaultCutoverComplete",
            supported_default_cutover.supported_default_cutover_complete,
            supported_default_cutover.blockers.clone(),
            "cutover manifest, supported default selection, fallback, rollback, telemetry, acknowledgement, and mutation guard transition are evaluated together",
        ),
        data_plane_hardening_gate_check(
            "finalSupportedDefaultCutoverDecision",
            final_supported_default_cutover_decision,
            if final_supported_default_cutover_decision {
                Vec::new()
            } else {
                vec!["Rust data-plane supported default cutover requires an explicit final decision".into()]
            },
            "supported default cutover completion is explicit before verification",
        ),
    ];
    let rust_data_plane_hardening_supported_default_cutover_complete = checks.iter().all(|check| check.passed);
    let blockers = checks
        .iter()
        .flat_map(|check| check.blockers.clone())
        .collect::<Vec<String>>();

    Ok(KernelLoopbackRustDataPlaneHardeningSupportedDefaultCutoverReport {
        runtime_id: RUST_RUNTIME_ID.into(),
        component: "rust-data-plane-hardening-supported-default-cutover".into(),
        mutates_runtime: false,
        live_execution_allowed: false,
        production_data_plane_mutation_allowed: false,
        rust_data_plane_hardening_supported_default_promotion_dry_run_complete,
        supported_default_cutover,
        final_supported_default_cutover_decision,
        rust_data_plane_hardening_supported_default_cutover_complete,
        selected_runtime_kind: if rust_data_plane_hardening_supported_default_cutover_complete {
            KernelRuntimeKind::Rust
        } else {
            KernelRuntimeKind::Mihomo
        },
        rollback_runtime_kind: KernelRuntimeKind::Mihomo,
        checks,
        blockers,
        warnings: vec![
            "this supported default cutover surface does not touch TUN, DNS, adapter forwarding, or Mihomo config"
                .into(),
            "unsupported paths and emergency rollback remain bound to Mihomo fallback".into(),
        ],
        facts: vec![
            "Rust data-plane hardening supported default cutover follows promotion dry-run".into(),
            "successful cutover advances only to post-cutover verification".into(),
        ],
        next_safe_batch: if rust_data_plane_hardening_supported_default_cutover_complete {
            "rust-data-plane-hardening-supported-default-cutover-verification".into()
        } else {
            "rust-data-plane-hardening-supported-default-cutover".into()
        },
    })
}

fn rust_kernel_runtime_data_plane_hardening_supported_default_cutover_verification_report(
    cutover_record_review_decision: bool,
    supported_profile_traffic_sample_review_decision: bool,
    unsupported_path_fallback_verification_decision: bool,
    rollback_switch_verification_decision: bool,
    telemetry_soak_sample_review_decision: bool,
    leak_regression_absence_verification_decision: bool,
    mutation_audit_record_archive_decision: bool,
) -> RustKernelRuntimeDataPlaneHardeningSupportedDefaultCutoverVerificationReport {
    let (verification_surfaces, blockers) = collect_data_plane_hardening_surfaces(&[
        (
            "supported default cutover record review",
            cutover_record_review_decision,
            "Rust data-plane supported default cutover verification requires cutover record review",
        ),
        (
            "supported profile traffic sample review",
            supported_profile_traffic_sample_review_decision,
            "Rust data-plane supported default cutover verification requires supported profile traffic sample review",
        ),
        (
            "unsupported path fallback verification",
            unsupported_path_fallback_verification_decision,
            "Rust data-plane supported default cutover verification requires unsupported path fallback verification",
        ),
        (
            "rollback switch verification",
            rollback_switch_verification_decision,
            "Rust data-plane supported default cutover verification requires rollback switch verification",
        ),
        (
            "telemetry soak sample review",
            telemetry_soak_sample_review_decision,
            "Rust data-plane supported default cutover verification requires telemetry soak sample review",
        ),
        (
            "leak regression absence verification",
            leak_regression_absence_verification_decision,
            "Rust data-plane supported default cutover verification requires leak regression absence verification",
        ),
        (
            "archived mutation audit record",
            mutation_audit_record_archive_decision,
            "Rust data-plane supported default cutover verification requires archived mutation audit record",
        ),
    ]);

    RustKernelRuntimeDataPlaneHardeningSupportedDefaultCutoverVerificationReport {
        runtime_id: RUST_RUNTIME_ID.into(),
        component: "rust-data-plane-hardening-supported-default-cutover-verification-detail".into(),
        cutover_record_reviewed: cutover_record_review_decision,
        supported_profile_traffic_sample_reviewed: supported_profile_traffic_sample_review_decision,
        unsupported_path_fallback_verified: unsupported_path_fallback_verification_decision,
        rollback_switch_verified: rollback_switch_verification_decision,
        telemetry_soak_sample_reviewed: telemetry_soak_sample_review_decision,
        leak_regression_absence_verified: leak_regression_absence_verification_decision,
        mutation_audit_record_archived: mutation_audit_record_archive_decision,
        cutover_verification_complete: blockers.is_empty(),
        verification_surfaces,
        blockers,
        facts: vec![
            "supported default cutover verification checks supported profile samples and unsupported fallback together"
                .into(),
            "verification success advances only to a hold window before closeout".into(),
        ],
    }
}

pub async fn rust_kernel_runtime_data_plane_hardening_supported_default_cutover_verification(
    rust_data_plane_hardening_supported_default_cutover_complete_decision: Option<bool>,
    cutover_record_review_decision: Option<bool>,
    supported_profile_traffic_sample_review_decision: Option<bool>,
    unsupported_path_fallback_verification_decision: Option<bool>,
    rollback_switch_verification_decision: Option<bool>,
    telemetry_soak_sample_review_decision: Option<bool>,
    leak_regression_absence_verification_decision: Option<bool>,
    mutation_audit_record_archive_decision: Option<bool>,
    final_supported_default_cutover_verification_decision: Option<bool>,
) -> Result<KernelLoopbackRustDataPlaneHardeningSupportedDefaultCutoverVerificationReport> {
    let rust_data_plane_hardening_supported_default_cutover_complete =
        rust_data_plane_hardening_supported_default_cutover_complete_decision.unwrap_or(false);
    let final_supported_default_cutover_verification_decision =
        final_supported_default_cutover_verification_decision.unwrap_or(false);
    let supported_default_cutover_verification =
        rust_kernel_runtime_data_plane_hardening_supported_default_cutover_verification_report(
            cutover_record_review_decision.unwrap_or(false),
            supported_profile_traffic_sample_review_decision.unwrap_or(false),
            unsupported_path_fallback_verification_decision.unwrap_or(false),
            rollback_switch_verification_decision.unwrap_or(false),
            telemetry_soak_sample_review_decision.unwrap_or(false),
            leak_regression_absence_verification_decision.unwrap_or(false),
            mutation_audit_record_archive_decision.unwrap_or(false),
        );
    let cutover_blockers = if rust_data_plane_hardening_supported_default_cutover_complete {
        Vec::new()
    } else {
        vec!["Rust data-plane supported default cutover verification requires cutover to pass first".into()]
    };

    let checks = vec![
        data_plane_hardening_gate_check(
            "rustDataPlaneHardeningSupportedDefaultCutoverComplete",
            rust_data_plane_hardening_supported_default_cutover_complete,
            cutover_blockers,
            "supported default cutover verification starts only after cutover",
        ),
        data_plane_hardening_gate_check(
            "supportedDefaultCutoverVerificationComplete",
            supported_default_cutover_verification.cutover_verification_complete,
            supported_default_cutover_verification.blockers.clone(),
            "cutover record, supported samples, fallback, rollback, telemetry, leak, and audit evidence are evaluated together",
        ),
        data_plane_hardening_gate_check(
            "finalSupportedDefaultCutoverVerificationDecision",
            final_supported_default_cutover_verification_decision,
            if final_supported_default_cutover_verification_decision {
                Vec::new()
            } else {
                vec![
                    "Rust data-plane supported default cutover verification requires an explicit final decision".into(),
                ]
            },
            "supported default cutover verification completion is explicit before hold window",
        ),
    ];
    let rust_data_plane_hardening_supported_default_cutover_verification_complete =
        checks.iter().all(|check| check.passed);
    let blockers = checks
        .iter()
        .flat_map(|check| check.blockers.clone())
        .collect::<Vec<String>>();

    Ok(
        KernelLoopbackRustDataPlaneHardeningSupportedDefaultCutoverVerificationReport {
            runtime_id: RUST_RUNTIME_ID.into(),
            component: "rust-data-plane-hardening-supported-default-cutover-verification".into(),
            mutates_runtime: false,
            live_execution_allowed: false,
            production_data_plane_mutation_allowed: false,
            rust_data_plane_hardening_supported_default_cutover_complete,
            supported_default_cutover_verification,
            final_supported_default_cutover_verification_decision,
            rust_data_plane_hardening_supported_default_cutover_verification_complete,
            selected_runtime_kind: if rust_data_plane_hardening_supported_default_cutover_verification_complete {
                KernelRuntimeKind::Rust
            } else {
                KernelRuntimeKind::Mihomo
            },
            rollback_runtime_kind: KernelRuntimeKind::Mihomo,
            checks,
            blockers,
            warnings: vec![
                "this supported default cutover verification surface does not mutate runtime or retire Mihomo fallback"
                    .into(),
                "hold window and closeout remain required before expanding beyond the supported profile".into(),
            ],
            facts: vec![
                "Rust data-plane hardening supported default cutover verification follows cutover".into(),
                "successful verification advances only to a hold window".into(),
            ],
            next_safe_batch: if rust_data_plane_hardening_supported_default_cutover_verification_complete {
                "rust-data-plane-hardening-supported-default-cutover-hold-window".into()
            } else {
                "rust-data-plane-hardening-supported-default-cutover-verification".into()
            },
        },
    )
}

fn rust_kernel_runtime_data_plane_hardening_supported_default_cutover_hold_window_report(
    verification_review_decision: bool,
    soak_window_elapsed_decision: bool,
    health_budget_satisfied_decision: bool,
    fallback_incident_review_decision: bool,
    rollback_switch_still_armed_decision: bool,
    mihomo_fallback_retention_decision: bool,
    hold_window_evidence_archive_decision: bool,
) -> RustKernelRuntimeDataPlaneHardeningSupportedDefaultCutoverHoldWindowReport {
    let (hold_surfaces, blockers) = collect_data_plane_hardening_surfaces(&[
        (
            "supported default cutover verification review",
            verification_review_decision,
            "Rust data-plane supported default cutover hold window requires verification review",
        ),
        (
            "elapsed soak window",
            soak_window_elapsed_decision,
            "Rust data-plane supported default cutover hold window requires elapsed soak window",
        ),
        (
            "satisfied health budget",
            health_budget_satisfied_decision,
            "Rust data-plane supported default cutover hold window requires satisfied health budget",
        ),
        (
            "fallback incident review",
            fallback_incident_review_decision,
            "Rust data-plane supported default cutover hold window requires fallback incident review",
        ),
        (
            "still-armed rollback switch",
            rollback_switch_still_armed_decision,
            "Rust data-plane supported default cutover hold window requires rollback switch to remain armed",
        ),
        (
            "retained Mihomo fallback",
            mihomo_fallback_retention_decision,
            "Rust data-plane supported default cutover hold window requires retained Mihomo fallback",
        ),
        (
            "archived hold window evidence",
            hold_window_evidence_archive_decision,
            "Rust data-plane supported default cutover hold window requires archived evidence",
        ),
    ]);

    RustKernelRuntimeDataPlaneHardeningSupportedDefaultCutoverHoldWindowReport {
        runtime_id: RUST_RUNTIME_ID.into(),
        component: "rust-data-plane-hardening-supported-default-cutover-hold-window-detail".into(),
        verification_reviewed: verification_review_decision,
        soak_window_elapsed: soak_window_elapsed_decision,
        health_budget_satisfied: health_budget_satisfied_decision,
        fallback_incidents_reviewed: fallback_incident_review_decision,
        rollback_switch_still_armed: rollback_switch_still_armed_decision,
        mihomo_fallback_still_retained: mihomo_fallback_retention_decision,
        hold_window_evidence_archived: hold_window_evidence_archive_decision,
        cutover_hold_window_complete: blockers.is_empty(),
        hold_surfaces,
        blockers,
        facts: vec![
            "supported default cutover hold window keeps rollback and Mihomo fallback alive through soak".into(),
            "hold success advances only to closeout, not fallback retirement".into(),
        ],
    }
}

pub async fn rust_kernel_runtime_data_plane_hardening_supported_default_cutover_hold_window(
    rust_data_plane_hardening_supported_default_cutover_verification_complete_decision: Option<bool>,
    verification_review_decision: Option<bool>,
    soak_window_elapsed_decision: Option<bool>,
    health_budget_satisfied_decision: Option<bool>,
    fallback_incident_review_decision: Option<bool>,
    rollback_switch_still_armed_decision: Option<bool>,
    mihomo_fallback_retention_decision: Option<bool>,
    hold_window_evidence_archive_decision: Option<bool>,
    final_supported_default_cutover_hold_window_decision: Option<bool>,
) -> Result<KernelLoopbackRustDataPlaneHardeningSupportedDefaultCutoverHoldWindowReport> {
    let rust_data_plane_hardening_supported_default_cutover_verification_complete =
        rust_data_plane_hardening_supported_default_cutover_verification_complete_decision.unwrap_or(false);
    let final_supported_default_cutover_hold_window_decision =
        final_supported_default_cutover_hold_window_decision.unwrap_or(false);
    let supported_default_cutover_hold_window =
        rust_kernel_runtime_data_plane_hardening_supported_default_cutover_hold_window_report(
            verification_review_decision.unwrap_or(false),
            soak_window_elapsed_decision.unwrap_or(false),
            health_budget_satisfied_decision.unwrap_or(false),
            fallback_incident_review_decision.unwrap_or(false),
            rollback_switch_still_armed_decision.unwrap_or(false),
            mihomo_fallback_retention_decision.unwrap_or(false),
            hold_window_evidence_archive_decision.unwrap_or(false),
        );
    let verification_blockers = if rust_data_plane_hardening_supported_default_cutover_verification_complete {
        Vec::new()
    } else {
        vec!["Rust data-plane supported default cutover hold window requires verification to pass first".into()]
    };

    let checks = vec![
        data_plane_hardening_gate_check(
            "rustDataPlaneHardeningSupportedDefaultCutoverVerificationComplete",
            rust_data_plane_hardening_supported_default_cutover_verification_complete,
            verification_blockers,
            "supported default cutover hold window starts only after verification",
        ),
        data_plane_hardening_gate_check(
            "supportedDefaultCutoverHoldWindowComplete",
            supported_default_cutover_hold_window.cutover_hold_window_complete,
            supported_default_cutover_hold_window.blockers.clone(),
            "verification review, soak, health, fallback incidents, rollback, Mihomo fallback, and evidence are evaluated together",
        ),
        data_plane_hardening_gate_check(
            "finalSupportedDefaultCutoverHoldWindowDecision",
            final_supported_default_cutover_hold_window_decision,
            if final_supported_default_cutover_hold_window_decision {
                Vec::new()
            } else {
                vec!["Rust data-plane supported default cutover hold window requires an explicit final decision".into()]
            },
            "supported default cutover hold completion is explicit before closeout",
        ),
    ];
    let rust_data_plane_hardening_supported_default_cutover_hold_window_complete =
        checks.iter().all(|check| check.passed);
    let blockers = checks
        .iter()
        .flat_map(|check| check.blockers.clone())
        .collect::<Vec<String>>();

    Ok(
        KernelLoopbackRustDataPlaneHardeningSupportedDefaultCutoverHoldWindowReport {
            runtime_id: RUST_RUNTIME_ID.into(),
            component: "rust-data-plane-hardening-supported-default-cutover-hold-window".into(),
            mutates_runtime: false,
            live_execution_allowed: false,
            production_data_plane_mutation_allowed: false,
            rust_data_plane_hardening_supported_default_cutover_verification_complete,
            supported_default_cutover_hold_window,
            final_supported_default_cutover_hold_window_decision,
            rust_data_plane_hardening_supported_default_cutover_hold_window_complete,
            selected_runtime_kind: if rust_data_plane_hardening_supported_default_cutover_hold_window_complete {
                KernelRuntimeKind::Rust
            } else {
                KernelRuntimeKind::Mihomo
            },
            rollback_runtime_kind: KernelRuntimeKind::Mihomo,
            checks,
            blockers,
            warnings: vec![
                "this hold window surface does not mutate runtime or retire Mihomo fallback".into(),
                "expanded default rollout remains blocked until closeout completes".into(),
            ],
            facts: vec![
                "Rust data-plane hardening supported default cutover hold window follows verification".into(),
                "successful hold advances only to supported default cutover closeout".into(),
            ],
            next_safe_batch: if rust_data_plane_hardening_supported_default_cutover_hold_window_complete {
                "rust-data-plane-hardening-supported-default-cutover-closeout".into()
            } else {
                "rust-data-plane-hardening-supported-default-cutover-hold-window".into()
            },
        },
    )
}

fn rust_kernel_runtime_data_plane_hardening_supported_default_cutover_closeout_report(
    hold_window_review_decision: bool,
    supported_default_state_documentation_decision: bool,
    rollback_owner_acknowledgement_decision: bool,
    fallback_retirement_boundary_retention_decision: bool,
    release_notes_update_decision: bool,
    closeout_evidence_archive_decision: bool,
) -> RustKernelRuntimeDataPlaneHardeningSupportedDefaultCutoverCloseoutReport {
    let (closeout_surfaces, blockers) = collect_data_plane_hardening_surfaces(&[
        (
            "supported default cutover hold window review",
            hold_window_review_decision,
            "Rust data-plane supported default cutover closeout requires hold window review",
        ),
        (
            "supported default state documentation",
            supported_default_state_documentation_decision,
            "Rust data-plane supported default cutover closeout requires supported default state documentation",
        ),
        (
            "rollback owner acknowledgement",
            rollback_owner_acknowledgement_decision,
            "Rust data-plane supported default cutover closeout requires rollback owner acknowledgement",
        ),
        (
            "retained fallback retirement boundary",
            fallback_retirement_boundary_retention_decision,
            "Rust data-plane supported default cutover closeout requires retained fallback retirement boundary",
        ),
        (
            "updated release notes",
            release_notes_update_decision,
            "Rust data-plane supported default cutover closeout requires release notes update",
        ),
        (
            "archived closeout evidence",
            closeout_evidence_archive_decision,
            "Rust data-plane supported default cutover closeout requires archived evidence",
        ),
    ]);

    RustKernelRuntimeDataPlaneHardeningSupportedDefaultCutoverCloseoutReport {
        runtime_id: RUST_RUNTIME_ID.into(),
        component: "rust-data-plane-hardening-supported-default-cutover-closeout-detail".into(),
        hold_window_reviewed: hold_window_review_decision,
        supported_default_state_documented: supported_default_state_documentation_decision,
        rollback_owner_acknowledged: rollback_owner_acknowledgement_decision,
        fallback_retirement_boundary_retained: fallback_retirement_boundary_retention_decision,
        release_notes_updated: release_notes_update_decision,
        closeout_evidence_archived: closeout_evidence_archive_decision,
        supported_default_cutover_closeout_complete: blockers.is_empty(),
        closeout_surfaces,
        blockers,
        facts: vec![
            "supported default cutover closeout documents the supported-profile default state".into(),
            "fallback retirement remains a separate high-risk phase after closeout".into(),
        ],
    }
}

pub async fn rust_kernel_runtime_data_plane_hardening_supported_default_cutover_closeout(
    rust_data_plane_hardening_supported_default_cutover_hold_window_complete_decision: Option<bool>,
    hold_window_review_decision: Option<bool>,
    supported_default_state_documentation_decision: Option<bool>,
    rollback_owner_acknowledgement_decision: Option<bool>,
    fallback_retirement_boundary_retention_decision: Option<bool>,
    release_notes_update_decision: Option<bool>,
    closeout_evidence_archive_decision: Option<bool>,
    final_supported_default_cutover_closeout_decision: Option<bool>,
) -> Result<KernelLoopbackRustDataPlaneHardeningSupportedDefaultCutoverCloseoutReport> {
    let rust_data_plane_hardening_supported_default_cutover_hold_window_complete =
        rust_data_plane_hardening_supported_default_cutover_hold_window_complete_decision.unwrap_or(false);
    let final_supported_default_cutover_closeout_decision =
        final_supported_default_cutover_closeout_decision.unwrap_or(false);
    let supported_default_cutover_closeout =
        rust_kernel_runtime_data_plane_hardening_supported_default_cutover_closeout_report(
            hold_window_review_decision.unwrap_or(false),
            supported_default_state_documentation_decision.unwrap_or(false),
            rollback_owner_acknowledgement_decision.unwrap_or(false),
            fallback_retirement_boundary_retention_decision.unwrap_or(false),
            release_notes_update_decision.unwrap_or(false),
            closeout_evidence_archive_decision.unwrap_or(false),
        );
    let hold_blockers = if rust_data_plane_hardening_supported_default_cutover_hold_window_complete {
        Vec::new()
    } else {
        vec!["Rust data-plane supported default cutover closeout requires hold window to pass first".into()]
    };

    let checks = vec![
        data_plane_hardening_gate_check(
            "rustDataPlaneHardeningSupportedDefaultCutoverHoldWindowComplete",
            rust_data_plane_hardening_supported_default_cutover_hold_window_complete,
            hold_blockers,
            "supported default cutover closeout starts only after the hold window",
        ),
        data_plane_hardening_gate_check(
            "supportedDefaultCutoverCloseoutComplete",
            supported_default_cutover_closeout.supported_default_cutover_closeout_complete,
            supported_default_cutover_closeout.blockers.clone(),
            "hold review, documentation, rollback ownership, fallback retirement boundary, release notes, and evidence are evaluated together",
        ),
        data_plane_hardening_gate_check(
            "finalSupportedDefaultCutoverCloseoutDecision",
            final_supported_default_cutover_closeout_decision,
            if final_supported_default_cutover_closeout_decision {
                Vec::new()
            } else {
                vec!["Rust data-plane supported default cutover closeout requires an explicit final decision".into()]
            },
            "supported default cutover closeout completion is explicit before expanded default rollout",
        ),
    ];
    let rust_data_plane_hardening_supported_default_cutover_closeout_complete = checks.iter().all(|check| check.passed);
    let blockers = checks
        .iter()
        .flat_map(|check| check.blockers.clone())
        .collect::<Vec<String>>();

    Ok(
        KernelLoopbackRustDataPlaneHardeningSupportedDefaultCutoverCloseoutReport {
            runtime_id: RUST_RUNTIME_ID.into(),
            component: "rust-data-plane-hardening-supported-default-cutover-closeout".into(),
            mutates_runtime: false,
            live_execution_allowed: false,
            production_data_plane_mutation_allowed: false,
            rust_data_plane_hardening_supported_default_cutover_hold_window_complete,
            supported_default_cutover_closeout,
            final_supported_default_cutover_closeout_decision,
            rust_data_plane_hardening_supported_default_cutover_closeout_complete,
            selected_runtime_kind: if rust_data_plane_hardening_supported_default_cutover_closeout_complete {
                KernelRuntimeKind::Rust
            } else {
                KernelRuntimeKind::Mihomo
            },
            rollback_runtime_kind: KernelRuntimeKind::Mihomo,
            checks,
            blockers,
            warnings: vec![
                "this closeout surface does not mutate runtime or retire Mihomo fallback".into(),
                "fallback retirement and unsupported data-plane ownership remain separate high-risk phases".into(),
            ],
            facts: vec![
                "Rust data-plane hardening supported default cutover closeout follows the hold window".into(),
                "successful closeout advances only to expanded default rollout guard planning".into(),
            ],
            next_safe_batch: if rust_data_plane_hardening_supported_default_cutover_closeout_complete {
                "rust-data-plane-hardening-expanded-default-rollout-guard".into()
            } else {
                "rust-data-plane-hardening-supported-default-cutover-closeout".into()
            },
        },
    )
}

fn rust_kernel_runtime_data_plane_hardening_expanded_default_rollout_guard_report(
    cutover_closeout_review_decision: bool,
    expanded_scope_lock_decision: bool,
    rollout_cap_definition_decision: bool,
    fallback_matrix_retention_decision: bool,
    rollback_switch_verification_decision: bool,
    telemetry_soak_plan_definition_decision: bool,
    unsupported_path_boundary_retention_decision: bool,
    operator_rollout_acknowledgement_decision: bool,
) -> RustKernelRuntimeDataPlaneHardeningExpandedDefaultRolloutGuardReport {
    let (guard_surfaces, blockers) = collect_data_plane_hardening_surfaces(&[
        (
            "supported default cutover closeout review",
            cutover_closeout_review_decision,
            "Rust data-plane expanded default rollout guard requires supported default cutover closeout review",
        ),
        (
            "locked expanded rollout scope",
            expanded_scope_lock_decision,
            "Rust data-plane expanded default rollout guard requires locked rollout scope",
        ),
        (
            "defined expanded rollout cap",
            rollout_cap_definition_decision,
            "Rust data-plane expanded default rollout guard requires a defined rollout cap",
        ),
        (
            "retained fallback matrix",
            fallback_matrix_retention_decision,
            "Rust data-plane expanded default rollout guard requires retained fallback matrix",
        ),
        (
            "verified rollback switch",
            rollback_switch_verification_decision,
            "Rust data-plane expanded default rollout guard requires verified rollback switch",
        ),
        (
            "telemetry soak plan",
            telemetry_soak_plan_definition_decision,
            "Rust data-plane expanded default rollout guard requires telemetry soak plan",
        ),
        (
            "retained unsupported path boundary",
            unsupported_path_boundary_retention_decision,
            "Rust data-plane expanded default rollout guard requires unsupported path boundary retention",
        ),
        (
            "operator expanded rollout acknowledgement",
            operator_rollout_acknowledgement_decision,
            "Rust data-plane expanded default rollout guard requires operator acknowledgement",
        ),
    ]);

    RustKernelRuntimeDataPlaneHardeningExpandedDefaultRolloutGuardReport {
        runtime_id: RUST_RUNTIME_ID.into(),
        component: "rust-data-plane-hardening-expanded-default-rollout-guard-detail".into(),
        cutover_closeout_reviewed: cutover_closeout_review_decision,
        expanded_scope_locked: expanded_scope_lock_decision,
        rollout_cap_defined: rollout_cap_definition_decision,
        fallback_matrix_retained: fallback_matrix_retention_decision,
        rollback_switch_verified: rollback_switch_verification_decision,
        telemetry_soak_plan_defined: telemetry_soak_plan_definition_decision,
        unsupported_path_boundary_retained: unsupported_path_boundary_retention_decision,
        operator_rollout_acknowledged: operator_rollout_acknowledgement_decision,
        expanded_default_rollout_guard_complete: blockers.is_empty(),
        guard_surfaces,
        blockers,
        facts: vec![
            "expanded default rollout guard widens only through an explicit cap and retained fallback matrix".into(),
            "unsupported path ownership remains bounded before any expanded execution".into(),
        ],
    }
}

pub async fn rust_kernel_runtime_data_plane_hardening_expanded_default_rollout_guard(
    rust_data_plane_hardening_supported_default_cutover_closeout_complete_decision: Option<bool>,
    cutover_closeout_review_decision: Option<bool>,
    expanded_scope_lock_decision: Option<bool>,
    rollout_cap_definition_decision: Option<bool>,
    fallback_matrix_retention_decision: Option<bool>,
    rollback_switch_verification_decision: Option<bool>,
    telemetry_soak_plan_definition_decision: Option<bool>,
    unsupported_path_boundary_retention_decision: Option<bool>,
    operator_rollout_acknowledgement_decision: Option<bool>,
    final_expanded_default_rollout_guard_decision: Option<bool>,
) -> Result<KernelLoopbackRustDataPlaneHardeningExpandedDefaultRolloutGuardReport> {
    let rust_data_plane_hardening_supported_default_cutover_closeout_complete =
        rust_data_plane_hardening_supported_default_cutover_closeout_complete_decision.unwrap_or(false);
    let final_expanded_default_rollout_guard_decision = final_expanded_default_rollout_guard_decision.unwrap_or(false);
    let expanded_default_rollout_guard = rust_kernel_runtime_data_plane_hardening_expanded_default_rollout_guard_report(
        cutover_closeout_review_decision.unwrap_or(false),
        expanded_scope_lock_decision.unwrap_or(false),
        rollout_cap_definition_decision.unwrap_or(false),
        fallback_matrix_retention_decision.unwrap_or(false),
        rollback_switch_verification_decision.unwrap_or(false),
        telemetry_soak_plan_definition_decision.unwrap_or(false),
        unsupported_path_boundary_retention_decision.unwrap_or(false),
        operator_rollout_acknowledgement_decision.unwrap_or(false),
    );
    let closeout_blockers = if rust_data_plane_hardening_supported_default_cutover_closeout_complete {
        Vec::new()
    } else {
        vec![
            "Rust data-plane expanded default rollout guard requires supported default cutover closeout to pass first"
                .into(),
        ]
    };

    let checks = vec![
        data_plane_hardening_gate_check(
            "rustDataPlaneHardeningSupportedDefaultCutoverCloseoutComplete",
            rust_data_plane_hardening_supported_default_cutover_closeout_complete,
            closeout_blockers,
            "expanded default rollout guard starts only after supported default cutover closeout",
        ),
        data_plane_hardening_gate_check(
            "expandedDefaultRolloutGuardComplete",
            expanded_default_rollout_guard.expanded_default_rollout_guard_complete,
            expanded_default_rollout_guard.blockers.clone(),
            "closeout review, expanded scope, cap, fallback matrix, rollback, telemetry, unsupported boundary, and acknowledgement are evaluated together",
        ),
        data_plane_hardening_gate_check(
            "finalExpandedDefaultRolloutGuardDecision",
            final_expanded_default_rollout_guard_decision,
            if final_expanded_default_rollout_guard_decision {
                Vec::new()
            } else {
                vec!["Rust data-plane expanded default rollout guard requires an explicit final decision".into()]
            },
            "expanded default rollout guard completion is explicit before dry-run",
        ),
    ];
    let rust_data_plane_hardening_expanded_default_rollout_guard_complete = checks.iter().all(|check| check.passed);
    let blockers = checks
        .iter()
        .flat_map(|check| check.blockers.clone())
        .collect::<Vec<String>>();

    Ok(
        KernelLoopbackRustDataPlaneHardeningExpandedDefaultRolloutGuardReport {
            runtime_id: RUST_RUNTIME_ID.into(),
            component: "rust-data-plane-hardening-expanded-default-rollout-guard".into(),
            mutates_runtime: false,
            live_execution_allowed: false,
            production_data_plane_mutation_allowed: false,
            rust_data_plane_hardening_supported_default_cutover_closeout_complete,
            expanded_default_rollout_guard,
            final_expanded_default_rollout_guard_decision,
            rust_data_plane_hardening_expanded_default_rollout_guard_complete,
            selected_runtime_kind: if rust_data_plane_hardening_expanded_default_rollout_guard_complete {
                KernelRuntimeKind::Rust
            } else {
                KernelRuntimeKind::Mihomo
            },
            rollback_runtime_kind: KernelRuntimeKind::Mihomo,
            checks,
            blockers,
            warnings: vec![
                "this expanded default rollout guard does not mutate runtime, TUN, DNS, adapter forwarding, or Mihomo config".into(),
                "Mihomo fallback and unsupported path ownership remain retained".into(),
            ],
            facts: vec![
                "Rust data-plane hardening expanded default rollout guard follows supported default cutover closeout".into(),
                "successful guard completion advances only to expanded rollout dry-run".into(),
            ],
            next_safe_batch: if rust_data_plane_hardening_expanded_default_rollout_guard_complete {
                "rust-data-plane-hardening-expanded-default-rollout-dry-run".into()
            } else {
                "rust-data-plane-hardening-expanded-default-rollout-guard".into()
            },
        },
    )
}

fn rust_kernel_runtime_data_plane_hardening_expanded_default_rollout_dry_run_report(
    guard_review_decision: bool,
    expanded_manifest_replay_decision: bool,
    representative_profile_simulation_decision: bool,
    fallback_routing_rehearsal_decision: bool,
    rollback_rehearsal_decision: bool,
    telemetry_soak_sample_review_decision: bool,
    dry_run_evidence_archive_decision: bool,
) -> RustKernelRuntimeDataPlaneHardeningExpandedDefaultRolloutDryRunReport {
    let (dry_run_surfaces, blockers) = collect_data_plane_hardening_surfaces(&[
        (
            "expanded default rollout guard review",
            guard_review_decision,
            "Rust data-plane expanded default rollout dry-run requires guard review",
        ),
        (
            "expanded manifest replay",
            expanded_manifest_replay_decision,
            "Rust data-plane expanded default rollout dry-run requires manifest replay",
        ),
        (
            "representative profile simulation",
            representative_profile_simulation_decision,
            "Rust data-plane expanded default rollout dry-run requires representative profile simulation",
        ),
        (
            "fallback routing rehearsal",
            fallback_routing_rehearsal_decision,
            "Rust data-plane expanded default rollout dry-run requires fallback routing rehearsal",
        ),
        (
            "rollback rehearsal",
            rollback_rehearsal_decision,
            "Rust data-plane expanded default rollout dry-run requires rollback rehearsal",
        ),
        (
            "telemetry soak sample review",
            telemetry_soak_sample_review_decision,
            "Rust data-plane expanded default rollout dry-run requires telemetry soak sample review",
        ),
        (
            "archived expanded rollout dry-run evidence",
            dry_run_evidence_archive_decision,
            "Rust data-plane expanded default rollout dry-run requires archived evidence",
        ),
    ]);

    RustKernelRuntimeDataPlaneHardeningExpandedDefaultRolloutDryRunReport {
        runtime_id: RUST_RUNTIME_ID.into(),
        component: "rust-data-plane-hardening-expanded-default-rollout-dry-run-detail".into(),
        guard_reviewed: guard_review_decision,
        expanded_manifest_replayed: expanded_manifest_replay_decision,
        representative_profile_simulation_completed: representative_profile_simulation_decision,
        fallback_routing_rehearsed: fallback_routing_rehearsal_decision,
        rollback_rehearsed: rollback_rehearsal_decision,
        telemetry_soak_sample_reviewed: telemetry_soak_sample_review_decision,
        dry_run_evidence_archived: dry_run_evidence_archive_decision,
        expanded_default_rollout_dry_run_complete: blockers.is_empty(),
        dry_run_surfaces,
        blockers,
        facts: vec![
            "expanded default rollout dry-run replays representative profiles without applying expanded defaults"
                .into(),
            "dry-run success advances only to the separately gated execution surface".into(),
        ],
    }
}

pub async fn rust_kernel_runtime_data_plane_hardening_expanded_default_rollout_dry_run(
    rust_data_plane_hardening_expanded_default_rollout_guard_complete_decision: Option<bool>,
    guard_review_decision: Option<bool>,
    expanded_manifest_replay_decision: Option<bool>,
    representative_profile_simulation_decision: Option<bool>,
    fallback_routing_rehearsal_decision: Option<bool>,
    rollback_rehearsal_decision: Option<bool>,
    telemetry_soak_sample_review_decision: Option<bool>,
    dry_run_evidence_archive_decision: Option<bool>,
    final_expanded_default_rollout_dry_run_decision: Option<bool>,
) -> Result<KernelLoopbackRustDataPlaneHardeningExpandedDefaultRolloutDryRunReport> {
    let rust_data_plane_hardening_expanded_default_rollout_guard_complete =
        rust_data_plane_hardening_expanded_default_rollout_guard_complete_decision.unwrap_or(false);
    let final_expanded_default_rollout_dry_run_decision =
        final_expanded_default_rollout_dry_run_decision.unwrap_or(false);
    let expanded_default_rollout_dry_run =
        rust_kernel_runtime_data_plane_hardening_expanded_default_rollout_dry_run_report(
            guard_review_decision.unwrap_or(false),
            expanded_manifest_replay_decision.unwrap_or(false),
            representative_profile_simulation_decision.unwrap_or(false),
            fallback_routing_rehearsal_decision.unwrap_or(false),
            rollback_rehearsal_decision.unwrap_or(false),
            telemetry_soak_sample_review_decision.unwrap_or(false),
            dry_run_evidence_archive_decision.unwrap_or(false),
        );
    let guard_blockers = if rust_data_plane_hardening_expanded_default_rollout_guard_complete {
        Vec::new()
    } else {
        vec!["Rust data-plane expanded default rollout dry-run requires rollout guard to pass first".into()]
    };

    let checks = vec![
        data_plane_hardening_gate_check(
            "rustDataPlaneHardeningExpandedDefaultRolloutGuardComplete",
            rust_data_plane_hardening_expanded_default_rollout_guard_complete,
            guard_blockers,
            "expanded default rollout dry-run starts only after the rollout guard",
        ),
        data_plane_hardening_gate_check(
            "expandedDefaultRolloutDryRunComplete",
            expanded_default_rollout_dry_run.expanded_default_rollout_dry_run_complete,
            expanded_default_rollout_dry_run.blockers.clone(),
            "guard review, manifest replay, representative simulation, fallback, rollback, telemetry, and evidence are evaluated together",
        ),
        data_plane_hardening_gate_check(
            "finalExpandedDefaultRolloutDryRunDecision",
            final_expanded_default_rollout_dry_run_decision,
            if final_expanded_default_rollout_dry_run_decision {
                Vec::new()
            } else {
                vec!["Rust data-plane expanded default rollout dry-run requires an explicit final decision".into()]
            },
            "expanded default rollout dry-run completion is explicit before execution",
        ),
    ];
    let rust_data_plane_hardening_expanded_default_rollout_dry_run_complete = checks.iter().all(|check| check.passed);
    let blockers = checks
        .iter()
        .flat_map(|check| check.blockers.clone())
        .collect::<Vec<String>>();

    Ok(
        KernelLoopbackRustDataPlaneHardeningExpandedDefaultRolloutDryRunReport {
            runtime_id: RUST_RUNTIME_ID.into(),
            component: "rust-data-plane-hardening-expanded-default-rollout-dry-run".into(),
            mutates_runtime: false,
            live_execution_allowed: false,
            production_data_plane_mutation_allowed: false,
            rust_data_plane_hardening_expanded_default_rollout_guard_complete,
            expanded_default_rollout_dry_run,
            final_expanded_default_rollout_dry_run_decision,
            rust_data_plane_hardening_expanded_default_rollout_dry_run_complete,
            selected_runtime_kind: if rust_data_plane_hardening_expanded_default_rollout_dry_run_complete {
                KernelRuntimeKind::Rust
            } else {
                KernelRuntimeKind::Mihomo
            },
            rollback_runtime_kind: KernelRuntimeKind::Mihomo,
            checks,
            blockers,
            warnings: vec![
                "this expanded default rollout dry-run does not mutate runtime, routes, TUN, DNS, adapter forwarding, or Mihomo config".into(),
                "execution remains blocked until a separate explicit execution surface passes".into(),
            ],
            facts: vec![
                "Rust data-plane hardening expanded default rollout dry-run follows the guard".into(),
                "successful dry-run advances only to expanded rollout execution".into(),
            ],
            next_safe_batch: if rust_data_plane_hardening_expanded_default_rollout_dry_run_complete {
                "rust-data-plane-hardening-expanded-default-rollout-execution".into()
            } else {
                "rust-data-plane-hardening-expanded-default-rollout-dry-run".into()
            },
        },
    )
}

fn rust_kernel_runtime_data_plane_hardening_expanded_default_rollout_execution_report(
    dry_run_review_decision: bool,
    execution_manifest_lock_decision: bool,
    rollout_window_start_decision: bool,
    expanded_profile_cap_enforcement_decision: bool,
    active_telemetry_watch_decision: bool,
    rollback_switch_arm_decision: bool,
    mihomo_fallback_retention_decision: bool,
    operator_execution_acknowledgement_decision: bool,
) -> RustKernelRuntimeDataPlaneHardeningExpandedDefaultRolloutExecutionReport {
    let (execution_surfaces, blockers) = collect_data_plane_hardening_surfaces(&[
        (
            "expanded default rollout dry-run review",
            dry_run_review_decision,
            "Rust data-plane expanded default rollout execution requires dry-run review",
        ),
        (
            "locked execution manifest",
            execution_manifest_lock_decision,
            "Rust data-plane expanded default rollout execution requires a locked execution manifest",
        ),
        (
            "started rollout window",
            rollout_window_start_decision,
            "Rust data-plane expanded default rollout execution requires started rollout window",
        ),
        (
            "enforced expanded profile cap",
            expanded_profile_cap_enforcement_decision,
            "Rust data-plane expanded default rollout execution requires enforced expanded profile cap",
        ),
        (
            "active telemetry watch",
            active_telemetry_watch_decision,
            "Rust data-plane expanded default rollout execution requires active telemetry watch",
        ),
        (
            "armed rollback switch",
            rollback_switch_arm_decision,
            "Rust data-plane expanded default rollout execution requires armed rollback switch",
        ),
        (
            "retained Mihomo fallback",
            mihomo_fallback_retention_decision,
            "Rust data-plane expanded default rollout execution requires retained Mihomo fallback",
        ),
        (
            "operator expanded execution acknowledgement",
            operator_execution_acknowledgement_decision,
            "Rust data-plane expanded default rollout execution requires operator acknowledgement",
        ),
    ]);

    RustKernelRuntimeDataPlaneHardeningExpandedDefaultRolloutExecutionReport {
        runtime_id: RUST_RUNTIME_ID.into(),
        component: "rust-data-plane-hardening-expanded-default-rollout-execution-detail".into(),
        dry_run_reviewed: dry_run_review_decision,
        execution_manifest_locked: execution_manifest_lock_decision,
        rollout_window_started: rollout_window_start_decision,
        expanded_profile_cap_enforced: expanded_profile_cap_enforcement_decision,
        active_telemetry_watch: active_telemetry_watch_decision,
        rollback_switch_armed: rollback_switch_arm_decision,
        mihomo_fallback_retained: mihomo_fallback_retention_decision,
        operator_execution_acknowledged: operator_execution_acknowledgement_decision,
        expanded_default_rollout_execution_complete: blockers.is_empty(),
        execution_surfaces,
        blockers,
        facts: vec![
            "expanded default rollout execution remains capped and rollback-bound".into(),
            "Mihomo fallback remains retained through expanded rollout execution".into(),
        ],
    }
}

pub async fn rust_kernel_runtime_data_plane_hardening_expanded_default_rollout_execution(
    rust_data_plane_hardening_expanded_default_rollout_dry_run_complete_decision: Option<bool>,
    dry_run_review_decision: Option<bool>,
    execution_manifest_lock_decision: Option<bool>,
    rollout_window_start_decision: Option<bool>,
    expanded_profile_cap_enforcement_decision: Option<bool>,
    active_telemetry_watch_decision: Option<bool>,
    rollback_switch_arm_decision: Option<bool>,
    mihomo_fallback_retention_decision: Option<bool>,
    operator_execution_acknowledgement_decision: Option<bool>,
    final_expanded_default_rollout_execution_decision: Option<bool>,
) -> Result<KernelLoopbackRustDataPlaneHardeningExpandedDefaultRolloutExecutionReport> {
    let rust_data_plane_hardening_expanded_default_rollout_dry_run_complete =
        rust_data_plane_hardening_expanded_default_rollout_dry_run_complete_decision.unwrap_or(false);
    let final_expanded_default_rollout_execution_decision =
        final_expanded_default_rollout_execution_decision.unwrap_or(false);
    let expanded_default_rollout_execution =
        rust_kernel_runtime_data_plane_hardening_expanded_default_rollout_execution_report(
            dry_run_review_decision.unwrap_or(false),
            execution_manifest_lock_decision.unwrap_or(false),
            rollout_window_start_decision.unwrap_or(false),
            expanded_profile_cap_enforcement_decision.unwrap_or(false),
            active_telemetry_watch_decision.unwrap_or(false),
            rollback_switch_arm_decision.unwrap_or(false),
            mihomo_fallback_retention_decision.unwrap_or(false),
            operator_execution_acknowledgement_decision.unwrap_or(false),
        );
    let dry_run_blockers = if rust_data_plane_hardening_expanded_default_rollout_dry_run_complete {
        Vec::new()
    } else {
        vec!["Rust data-plane expanded default rollout execution requires dry-run to pass first".into()]
    };

    let checks = vec![
        data_plane_hardening_gate_check(
            "rustDataPlaneHardeningExpandedDefaultRolloutDryRunComplete",
            rust_data_plane_hardening_expanded_default_rollout_dry_run_complete,
            dry_run_blockers,
            "expanded default rollout execution starts only after dry-run",
        ),
        data_plane_hardening_gate_check(
            "expandedDefaultRolloutExecutionComplete",
            expanded_default_rollout_execution.expanded_default_rollout_execution_complete,
            expanded_default_rollout_execution.blockers.clone(),
            "dry-run review, execution manifest, rollout window, cap, telemetry, rollback, fallback, and acknowledgement are evaluated together",
        ),
        data_plane_hardening_gate_check(
            "finalExpandedDefaultRolloutExecutionDecision",
            final_expanded_default_rollout_execution_decision,
            if final_expanded_default_rollout_execution_decision {
                Vec::new()
            } else {
                vec!["Rust data-plane expanded default rollout execution requires an explicit final decision".into()]
            },
            "expanded default rollout execution completion is explicit before verification",
        ),
    ];
    let rust_data_plane_hardening_expanded_default_rollout_execution_complete = checks.iter().all(|check| check.passed);
    let blockers = checks
        .iter()
        .flat_map(|check| check.blockers.clone())
        .collect::<Vec<String>>();

    Ok(
        KernelLoopbackRustDataPlaneHardeningExpandedDefaultRolloutExecutionReport {
            runtime_id: RUST_RUNTIME_ID.into(),
            component: "rust-data-plane-hardening-expanded-default-rollout-execution".into(),
            mutates_runtime: false,
            live_execution_allowed: false,
            production_data_plane_mutation_allowed: false,
            rust_data_plane_hardening_expanded_default_rollout_dry_run_complete,
            expanded_default_rollout_execution,
            final_expanded_default_rollout_execution_decision,
            rust_data_plane_hardening_expanded_default_rollout_execution_complete,
            selected_runtime_kind: if rust_data_plane_hardening_expanded_default_rollout_execution_complete {
                KernelRuntimeKind::Rust
            } else {
                KernelRuntimeKind::Mihomo
            },
            rollback_runtime_kind: KernelRuntimeKind::Mihomo,
            checks,
            blockers,
            warnings: vec![
                "this expanded default rollout execution surface does not touch TUN, DNS, adapter forwarding, or Mihomo config".into(),
                "fallback retirement and unsupported data-plane ownership remain separate high-risk phases".into(),
            ],
            facts: vec![
                "Rust data-plane hardening expanded default rollout execution follows dry-run".into(),
                "successful execution advances only to expanded rollout verification".into(),
            ],
            next_safe_batch: if rust_data_plane_hardening_expanded_default_rollout_execution_complete {
                "rust-data-plane-hardening-expanded-default-rollout-verification".into()
            } else {
                "rust-data-plane-hardening-expanded-default-rollout-execution".into()
            },
        },
    )
}

fn rust_kernel_runtime_data_plane_hardening_expanded_default_rollout_verification_report(
    execution_record_review_decision: bool,
    expanded_profile_traffic_sample_review_decision: bool,
    fallback_path_sample_verification_decision: bool,
    rollback_switch_verification_decision: bool,
    telemetry_health_budget_verification_decision: bool,
    leak_regression_absence_verification_decision: bool,
    verification_evidence_archive_decision: bool,
) -> RustKernelRuntimeDataPlaneHardeningExpandedDefaultRolloutVerificationReport {
    let (verification_surfaces, blockers) = collect_data_plane_hardening_surfaces(&[
        (
            "expanded rollout execution record review",
            execution_record_review_decision,
            "Rust data-plane expanded default rollout verification requires execution record review",
        ),
        (
            "expanded profile traffic sample review",
            expanded_profile_traffic_sample_review_decision,
            "Rust data-plane expanded default rollout verification requires expanded profile traffic sample review",
        ),
        (
            "fallback path sample verification",
            fallback_path_sample_verification_decision,
            "Rust data-plane expanded default rollout verification requires fallback path sample verification",
        ),
        (
            "rollback switch verification",
            rollback_switch_verification_decision,
            "Rust data-plane expanded default rollout verification requires rollback switch verification",
        ),
        (
            "telemetry health budget verification",
            telemetry_health_budget_verification_decision,
            "Rust data-plane expanded default rollout verification requires telemetry health budget verification",
        ),
        (
            "leak regression absence verification",
            leak_regression_absence_verification_decision,
            "Rust data-plane expanded default rollout verification requires leak regression absence verification",
        ),
        (
            "archived expanded rollout verification evidence",
            verification_evidence_archive_decision,
            "Rust data-plane expanded default rollout verification requires archived evidence",
        ),
    ]);

    RustKernelRuntimeDataPlaneHardeningExpandedDefaultRolloutVerificationReport {
        runtime_id: RUST_RUNTIME_ID.into(),
        component: "rust-data-plane-hardening-expanded-default-rollout-verification-detail".into(),
        execution_record_reviewed: execution_record_review_decision,
        expanded_profile_traffic_sample_reviewed: expanded_profile_traffic_sample_review_decision,
        fallback_path_sample_verified: fallback_path_sample_verification_decision,
        rollback_switch_verified: rollback_switch_verification_decision,
        telemetry_health_budget_verified: telemetry_health_budget_verification_decision,
        leak_regression_absence_verified: leak_regression_absence_verification_decision,
        verification_evidence_archived: verification_evidence_archive_decision,
        expanded_default_rollout_verification_complete: blockers.is_empty(),
        verification_surfaces,
        blockers,
        facts: vec![
            "expanded default rollout verification reviews expanded samples, fallback paths, rollback, health, and leak evidence together".into(),
            "verification success advances only to expanded rollout closeout planning".into(),
        ],
    }
}

pub async fn rust_kernel_runtime_data_plane_hardening_expanded_default_rollout_verification(
    rust_data_plane_hardening_expanded_default_rollout_execution_complete_decision: Option<bool>,
    execution_record_review_decision: Option<bool>,
    expanded_profile_traffic_sample_review_decision: Option<bool>,
    fallback_path_sample_verification_decision: Option<bool>,
    rollback_switch_verification_decision: Option<bool>,
    telemetry_health_budget_verification_decision: Option<bool>,
    leak_regression_absence_verification_decision: Option<bool>,
    verification_evidence_archive_decision: Option<bool>,
    final_expanded_default_rollout_verification_decision: Option<bool>,
) -> Result<KernelLoopbackRustDataPlaneHardeningExpandedDefaultRolloutVerificationReport> {
    let rust_data_plane_hardening_expanded_default_rollout_execution_complete =
        rust_data_plane_hardening_expanded_default_rollout_execution_complete_decision.unwrap_or(false);
    let final_expanded_default_rollout_verification_decision =
        final_expanded_default_rollout_verification_decision.unwrap_or(false);
    let expanded_default_rollout_verification =
        rust_kernel_runtime_data_plane_hardening_expanded_default_rollout_verification_report(
            execution_record_review_decision.unwrap_or(false),
            expanded_profile_traffic_sample_review_decision.unwrap_or(false),
            fallback_path_sample_verification_decision.unwrap_or(false),
            rollback_switch_verification_decision.unwrap_or(false),
            telemetry_health_budget_verification_decision.unwrap_or(false),
            leak_regression_absence_verification_decision.unwrap_or(false),
            verification_evidence_archive_decision.unwrap_or(false),
        );
    let execution_blockers = if rust_data_plane_hardening_expanded_default_rollout_execution_complete {
        Vec::new()
    } else {
        vec!["Rust data-plane expanded default rollout verification requires execution to pass first".into()]
    };

    let checks = vec![
        data_plane_hardening_gate_check(
            "rustDataPlaneHardeningExpandedDefaultRolloutExecutionComplete",
            rust_data_plane_hardening_expanded_default_rollout_execution_complete,
            execution_blockers,
            "expanded default rollout verification starts only after execution",
        ),
        data_plane_hardening_gate_check(
            "expandedDefaultRolloutVerificationComplete",
            expanded_default_rollout_verification.expanded_default_rollout_verification_complete,
            expanded_default_rollout_verification.blockers.clone(),
            "execution record, expanded samples, fallback paths, rollback, health, leak, and evidence are evaluated together",
        ),
        data_plane_hardening_gate_check(
            "finalExpandedDefaultRolloutVerificationDecision",
            final_expanded_default_rollout_verification_decision,
            if final_expanded_default_rollout_verification_decision {
                Vec::new()
            } else {
                vec!["Rust data-plane expanded default rollout verification requires an explicit final decision".into()]
            },
            "expanded default rollout verification completion is explicit before closeout",
        ),
    ];
    let rust_data_plane_hardening_expanded_default_rollout_verification_complete =
        checks.iter().all(|check| check.passed);
    let blockers = checks
        .iter()
        .flat_map(|check| check.blockers.clone())
        .collect::<Vec<String>>();

    Ok(
        KernelLoopbackRustDataPlaneHardeningExpandedDefaultRolloutVerificationReport {
            runtime_id: RUST_RUNTIME_ID.into(),
            component: "rust-data-plane-hardening-expanded-default-rollout-verification".into(),
            mutates_runtime: false,
            live_execution_allowed: false,
            production_data_plane_mutation_allowed: false,
            rust_data_plane_hardening_expanded_default_rollout_execution_complete,
            expanded_default_rollout_verification,
            final_expanded_default_rollout_verification_decision,
            rust_data_plane_hardening_expanded_default_rollout_verification_complete,
            selected_runtime_kind: if rust_data_plane_hardening_expanded_default_rollout_verification_complete {
                KernelRuntimeKind::Rust
            } else {
                KernelRuntimeKind::Mihomo
            },
            rollback_runtime_kind: KernelRuntimeKind::Mihomo,
            checks,
            blockers,
            warnings: vec![
                "this expanded default rollout verification surface does not mutate runtime or retire Mihomo fallback"
                    .into(),
                "expanded rollout closeout remains required before any fallback retirement planning".into(),
            ],
            facts: vec![
                "Rust data-plane hardening expanded default rollout verification follows execution".into(),
                "successful verification advances only to expanded rollout closeout".into(),
            ],
            next_safe_batch: if rust_data_plane_hardening_expanded_default_rollout_verification_complete {
                "rust-data-plane-hardening-expanded-default-rollout-closeout".into()
            } else {
                "rust-data-plane-hardening-expanded-default-rollout-verification".into()
            },
        },
    )
}

fn rust_kernel_runtime_data_plane_hardening_expanded_default_rollout_closeout_report(
    verification_review_decision: bool,
    expanded_rollout_state_documentation_decision: bool,
    rollback_owner_acknowledgement_decision: bool,
    fallback_matrix_retention_decision: bool,
    unsupported_path_boundary_retention_decision: bool,
    release_notes_update_decision: bool,
    closeout_evidence_archive_decision: bool,
) -> RustKernelRuntimeDataPlaneHardeningExpandedDefaultRolloutCloseoutReport {
    let (closeout_surfaces, blockers) = collect_data_plane_hardening_surfaces(&[
        (
            "expanded default rollout verification review",
            verification_review_decision,
            "Rust data-plane expanded default rollout closeout requires verification review",
        ),
        (
            "expanded rollout state documentation",
            expanded_rollout_state_documentation_decision,
            "Rust data-plane expanded default rollout closeout requires rollout state documentation",
        ),
        (
            "rollback owner acknowledgement",
            rollback_owner_acknowledgement_decision,
            "Rust data-plane expanded default rollout closeout requires rollback owner acknowledgement",
        ),
        (
            "retained fallback matrix",
            fallback_matrix_retention_decision,
            "Rust data-plane expanded default rollout closeout requires retained fallback matrix",
        ),
        (
            "retained unsupported path boundary",
            unsupported_path_boundary_retention_decision,
            "Rust data-plane expanded default rollout closeout requires retained unsupported path boundary",
        ),
        (
            "updated release notes",
            release_notes_update_decision,
            "Rust data-plane expanded default rollout closeout requires release notes update",
        ),
        (
            "archived expanded rollout closeout evidence",
            closeout_evidence_archive_decision,
            "Rust data-plane expanded default rollout closeout requires archived evidence",
        ),
    ]);

    RustKernelRuntimeDataPlaneHardeningExpandedDefaultRolloutCloseoutReport {
        runtime_id: RUST_RUNTIME_ID.into(),
        component: "rust-data-plane-hardening-expanded-default-rollout-closeout-detail".into(),
        verification_reviewed: verification_review_decision,
        expanded_rollout_state_documented: expanded_rollout_state_documentation_decision,
        rollback_owner_acknowledged: rollback_owner_acknowledgement_decision,
        fallback_matrix_retained: fallback_matrix_retention_decision,
        unsupported_path_boundary_retained: unsupported_path_boundary_retention_decision,
        release_notes_updated: release_notes_update_decision,
        closeout_evidence_archived: closeout_evidence_archive_decision,
        expanded_default_rollout_closeout_complete: blockers.is_empty(),
        closeout_surfaces,
        blockers,
        facts: vec![
            "expanded default rollout closeout records the widened Rust-default state without retiring Mihomo fallback"
                .into(),
            "fallback retirement remains a separate high-risk phase after closeout".into(),
        ],
    }
}

pub async fn rust_kernel_runtime_data_plane_hardening_expanded_default_rollout_closeout(
    rust_data_plane_hardening_expanded_default_rollout_verification_complete_decision: Option<bool>,
    verification_review_decision: Option<bool>,
    expanded_rollout_state_documentation_decision: Option<bool>,
    rollback_owner_acknowledgement_decision: Option<bool>,
    fallback_matrix_retention_decision: Option<bool>,
    unsupported_path_boundary_retention_decision: Option<bool>,
    release_notes_update_decision: Option<bool>,
    closeout_evidence_archive_decision: Option<bool>,
    final_expanded_default_rollout_closeout_decision: Option<bool>,
) -> Result<KernelLoopbackRustDataPlaneHardeningExpandedDefaultRolloutCloseoutReport> {
    let rust_data_plane_hardening_expanded_default_rollout_verification_complete =
        rust_data_plane_hardening_expanded_default_rollout_verification_complete_decision.unwrap_or(false);
    let final_expanded_default_rollout_closeout_decision =
        final_expanded_default_rollout_closeout_decision.unwrap_or(false);
    let expanded_default_rollout_closeout =
        rust_kernel_runtime_data_plane_hardening_expanded_default_rollout_closeout_report(
            verification_review_decision.unwrap_or(false),
            expanded_rollout_state_documentation_decision.unwrap_or(false),
            rollback_owner_acknowledgement_decision.unwrap_or(false),
            fallback_matrix_retention_decision.unwrap_or(false),
            unsupported_path_boundary_retention_decision.unwrap_or(false),
            release_notes_update_decision.unwrap_or(false),
            closeout_evidence_archive_decision.unwrap_or(false),
        );
    let verification_blockers = if rust_data_plane_hardening_expanded_default_rollout_verification_complete {
        Vec::new()
    } else {
        vec!["Rust data-plane expanded default rollout closeout requires verification to pass first".into()]
    };

    let checks = vec![
        data_plane_hardening_gate_check(
            "rustDataPlaneHardeningExpandedDefaultRolloutVerificationComplete",
            rust_data_plane_hardening_expanded_default_rollout_verification_complete,
            verification_blockers,
            "expanded default rollout closeout starts only after verification",
        ),
        data_plane_hardening_gate_check(
            "expandedDefaultRolloutCloseoutComplete",
            expanded_default_rollout_closeout.expanded_default_rollout_closeout_complete,
            expanded_default_rollout_closeout.blockers.clone(),
            "verification review, rollout state, rollback ownership, fallback matrix, unsupported boundary, release notes, and evidence are evaluated together",
        ),
        data_plane_hardening_gate_check(
            "finalExpandedDefaultRolloutCloseoutDecision",
            final_expanded_default_rollout_closeout_decision,
            if final_expanded_default_rollout_closeout_decision {
                Vec::new()
            } else {
                vec!["Rust data-plane expanded default rollout closeout requires an explicit final decision".into()]
            },
            "expanded default rollout closeout completion is explicit before fallback-retirement planning",
        ),
    ];
    let rust_data_plane_hardening_expanded_default_rollout_closeout_complete = checks.iter().all(|check| check.passed);
    let blockers = checks
        .iter()
        .flat_map(|check| check.blockers.clone())
        .collect::<Vec<String>>();

    Ok(
        KernelLoopbackRustDataPlaneHardeningExpandedDefaultRolloutCloseoutReport {
            runtime_id: RUST_RUNTIME_ID.into(),
            component: "rust-data-plane-hardening-expanded-default-rollout-closeout".into(),
            mutates_runtime: false,
            live_execution_allowed: false,
            production_data_plane_mutation_allowed: false,
            rust_data_plane_hardening_expanded_default_rollout_verification_complete,
            expanded_default_rollout_closeout,
            final_expanded_default_rollout_closeout_decision,
            rust_data_plane_hardening_expanded_default_rollout_closeout_complete,
            selected_runtime_kind: if rust_data_plane_hardening_expanded_default_rollout_closeout_complete {
                KernelRuntimeKind::Rust
            } else {
                KernelRuntimeKind::Mihomo
            },
            rollback_runtime_kind: KernelRuntimeKind::Mihomo,
            checks,
            blockers,
            warnings: vec![
                "this closeout surface does not retire Mihomo fallback or mutate TUN/DNS/adapter forwarding".into(),
                "fallback retirement remains blocked behind separate parity and rollback gates".into(),
            ],
            facts: vec![
                "Rust data-plane hardening expanded default rollout closeout follows verification".into(),
                "successful closeout advances only to Mihomo fallback-retirement guard planning".into(),
            ],
            next_safe_batch: if rust_data_plane_hardening_expanded_default_rollout_closeout_complete {
                "rust-data-plane-hardening-mihomo-fallback-retirement-guard".into()
            } else {
                "rust-data-plane-hardening-expanded-default-rollout-closeout".into()
            },
        },
    )
}

fn rust_kernel_runtime_data_plane_hardening_mihomo_fallback_retirement_guard_report(
    expanded_rollout_closeout_review_decision: bool,
    protocol_parity_scope_lock_decision: bool,
    tun_parity_scope_lock_decision: bool,
    adapter_parity_scope_lock_decision: bool,
    dns_parity_scope_lock_decision: bool,
    emergency_rollback_retention_decision: bool,
    cross_platform_drill_plan_definition_decision: bool,
    operator_retirement_acknowledgement_decision: bool,
) -> RustKernelRuntimeDataPlaneHardeningMihomoFallbackRetirementGuardReport {
    let (guard_surfaces, blockers) = collect_data_plane_hardening_surfaces(&[
        (
            "expanded default rollout closeout review",
            expanded_rollout_closeout_review_decision,
            "Rust data-plane Mihomo fallback retirement guard requires expanded rollout closeout review",
        ),
        (
            "locked protocol parity scope",
            protocol_parity_scope_lock_decision,
            "Rust data-plane Mihomo fallback retirement guard requires locked protocol parity scope",
        ),
        (
            "locked TUN parity scope",
            tun_parity_scope_lock_decision,
            "Rust data-plane Mihomo fallback retirement guard requires locked TUN parity scope",
        ),
        (
            "locked adapter parity scope",
            adapter_parity_scope_lock_decision,
            "Rust data-plane Mihomo fallback retirement guard requires locked adapter parity scope",
        ),
        (
            "locked DNS parity scope",
            dns_parity_scope_lock_decision,
            "Rust data-plane Mihomo fallback retirement guard requires locked DNS parity scope",
        ),
        (
            "retained emergency rollback",
            emergency_rollback_retention_decision,
            "Rust data-plane Mihomo fallback retirement guard requires retained emergency rollback",
        ),
        (
            "defined cross-platform drill plan",
            cross_platform_drill_plan_definition_decision,
            "Rust data-plane Mihomo fallback retirement guard requires cross-platform drill plan",
        ),
        (
            "operator fallback retirement acknowledgement",
            operator_retirement_acknowledgement_decision,
            "Rust data-plane Mihomo fallback retirement guard requires operator acknowledgement",
        ),
    ]);

    RustKernelRuntimeDataPlaneHardeningMihomoFallbackRetirementGuardReport {
        runtime_id: RUST_RUNTIME_ID.into(),
        component: "rust-data-plane-hardening-mihomo-fallback-retirement-guard-detail".into(),
        expanded_rollout_closeout_reviewed: expanded_rollout_closeout_review_decision,
        protocol_parity_scope_locked: protocol_parity_scope_lock_decision,
        tun_parity_scope_locked: tun_parity_scope_lock_decision,
        adapter_parity_scope_locked: adapter_parity_scope_lock_decision,
        dns_parity_scope_locked: dns_parity_scope_lock_decision,
        emergency_rollback_retained: emergency_rollback_retention_decision,
        cross_platform_drill_plan_defined: cross_platform_drill_plan_definition_decision,
        operator_retirement_acknowledged: operator_retirement_acknowledgement_decision,
        mihomo_fallback_retirement_guard_complete: blockers.is_empty(),
        guard_surfaces,
        blockers,
        facts: vec![
            "Mihomo fallback retirement is gated on protocol, TUN, adapter, and DNS parity scopes".into(),
            "guard success does not remove fallback; it only permits dry-run planning".into(),
        ],
    }
}

pub async fn rust_kernel_runtime_data_plane_hardening_mihomo_fallback_retirement_guard(
    rust_data_plane_hardening_expanded_default_rollout_closeout_complete_decision: Option<bool>,
    expanded_rollout_closeout_review_decision: Option<bool>,
    protocol_parity_scope_lock_decision: Option<bool>,
    tun_parity_scope_lock_decision: Option<bool>,
    adapter_parity_scope_lock_decision: Option<bool>,
    dns_parity_scope_lock_decision: Option<bool>,
    emergency_rollback_retention_decision: Option<bool>,
    cross_platform_drill_plan_definition_decision: Option<bool>,
    operator_retirement_acknowledgement_decision: Option<bool>,
    final_mihomo_fallback_retirement_guard_decision: Option<bool>,
) -> Result<KernelLoopbackRustDataPlaneHardeningMihomoFallbackRetirementGuardReport> {
    let rust_data_plane_hardening_expanded_default_rollout_closeout_complete =
        rust_data_plane_hardening_expanded_default_rollout_closeout_complete_decision.unwrap_or(false);
    let final_mihomo_fallback_retirement_guard_decision =
        final_mihomo_fallback_retirement_guard_decision.unwrap_or(false);
    let mihomo_fallback_retirement_guard =
        rust_kernel_runtime_data_plane_hardening_mihomo_fallback_retirement_guard_report(
            expanded_rollout_closeout_review_decision.unwrap_or(false),
            protocol_parity_scope_lock_decision.unwrap_or(false),
            tun_parity_scope_lock_decision.unwrap_or(false),
            adapter_parity_scope_lock_decision.unwrap_or(false),
            dns_parity_scope_lock_decision.unwrap_or(false),
            emergency_rollback_retention_decision.unwrap_or(false),
            cross_platform_drill_plan_definition_decision.unwrap_or(false),
            operator_retirement_acknowledgement_decision.unwrap_or(false),
        );
    let closeout_blockers = if rust_data_plane_hardening_expanded_default_rollout_closeout_complete {
        Vec::new()
    } else {
        vec!["Rust data-plane Mihomo fallback retirement guard requires expanded rollout closeout to pass first".into()]
    };

    let checks = vec![
        data_plane_hardening_gate_check(
            "rustDataPlaneHardeningExpandedDefaultRolloutCloseoutComplete",
            rust_data_plane_hardening_expanded_default_rollout_closeout_complete,
            closeout_blockers,
            "fallback retirement guard starts only after expanded default rollout closeout",
        ),
        data_plane_hardening_gate_check(
            "mihomoFallbackRetirementGuardComplete",
            mihomo_fallback_retirement_guard.mihomo_fallback_retirement_guard_complete,
            mihomo_fallback_retirement_guard.blockers.clone(),
            "closeout review, parity scopes, emergency rollback, drill plan, and acknowledgement are evaluated together",
        ),
        data_plane_hardening_gate_check(
            "finalMihomoFallbackRetirementGuardDecision",
            final_mihomo_fallback_retirement_guard_decision,
            if final_mihomo_fallback_retirement_guard_decision {
                Vec::new()
            } else {
                vec!["Rust data-plane Mihomo fallback retirement guard requires an explicit final decision".into()]
            },
            "fallback retirement guard completion is explicit before dry-run",
        ),
    ];
    let rust_data_plane_hardening_mihomo_fallback_retirement_guard_complete = checks.iter().all(|check| check.passed);
    let blockers = checks
        .iter()
        .flat_map(|check| check.blockers.clone())
        .collect::<Vec<String>>();

    Ok(
        KernelLoopbackRustDataPlaneHardeningMihomoFallbackRetirementGuardReport {
            runtime_id: RUST_RUNTIME_ID.into(),
            component: "rust-data-plane-hardening-mihomo-fallback-retirement-guard".into(),
            mutates_runtime: false,
            live_execution_allowed: false,
            production_data_plane_mutation_allowed: false,
            rust_data_plane_hardening_expanded_default_rollout_closeout_complete,
            mihomo_fallback_retirement_guard,
            final_mihomo_fallback_retirement_guard_decision,
            rust_data_plane_hardening_mihomo_fallback_retirement_guard_complete,
            selected_runtime_kind: KernelRuntimeKind::Rust,
            rollback_runtime_kind: KernelRuntimeKind::Mihomo,
            checks,
            blockers,
            warnings: vec![
                "this fallback retirement guard does not remove Mihomo fallback or mutate production forwarding".into(),
                "protocol, TUN, adapter, and DNS ownership changes remain blocked until parity evidence closes".into(),
            ],
            facts: vec![
                "Rust data-plane hardening Mihomo fallback retirement guard follows expanded rollout closeout".into(),
                "successful guard completion advances only to fallback retirement dry-run".into(),
            ],
            next_safe_batch: if rust_data_plane_hardening_mihomo_fallback_retirement_guard_complete {
                "rust-data-plane-hardening-mihomo-fallback-retirement-dry-run".into()
            } else {
                "rust-data-plane-hardening-mihomo-fallback-retirement-guard".into()
            },
        },
    )
}

fn rust_kernel_runtime_data_plane_hardening_mihomo_fallback_retirement_dry_run_report(
    guard_review_decision: bool,
    parity_manifest_replay_decision: bool,
    cross_platform_rollback_rehearsal_decision: bool,
    fallback_dependency_inventory_replay_decision: bool,
    emergency_recovery_rehearsal_decision: bool,
    production_forwarding_unchanged_verification_decision: bool,
    dry_run_evidence_archive_decision: bool,
) -> RustKernelRuntimeDataPlaneHardeningMihomoFallbackRetirementDryRunReport {
    let (dry_run_surfaces, blockers) = collect_data_plane_hardening_surfaces(&[
        (
            "Mihomo fallback retirement guard review",
            guard_review_decision,
            "Rust data-plane Mihomo fallback retirement dry-run requires guard review",
        ),
        (
            "parity manifest replay",
            parity_manifest_replay_decision,
            "Rust data-plane Mihomo fallback retirement dry-run requires parity manifest replay",
        ),
        (
            "cross-platform rollback rehearsal",
            cross_platform_rollback_rehearsal_decision,
            "Rust data-plane Mihomo fallback retirement dry-run requires cross-platform rollback rehearsal",
        ),
        (
            "fallback dependency inventory replay",
            fallback_dependency_inventory_replay_decision,
            "Rust data-plane Mihomo fallback retirement dry-run requires fallback dependency inventory replay",
        ),
        (
            "emergency recovery rehearsal",
            emergency_recovery_rehearsal_decision,
            "Rust data-plane Mihomo fallback retirement dry-run requires emergency recovery rehearsal",
        ),
        (
            "unchanged production forwarding verification",
            production_forwarding_unchanged_verification_decision,
            "Rust data-plane Mihomo fallback retirement dry-run requires unchanged production forwarding verification",
        ),
        (
            "archived fallback retirement dry-run evidence",
            dry_run_evidence_archive_decision,
            "Rust data-plane Mihomo fallback retirement dry-run requires archived evidence",
        ),
    ]);

    RustKernelRuntimeDataPlaneHardeningMihomoFallbackRetirementDryRunReport {
        runtime_id: RUST_RUNTIME_ID.into(),
        component: "rust-data-plane-hardening-mihomo-fallback-retirement-dry-run-detail".into(),
        guard_reviewed: guard_review_decision,
        parity_manifest_replayed: parity_manifest_replay_decision,
        cross_platform_rollback_rehearsed: cross_platform_rollback_rehearsal_decision,
        fallback_dependency_inventory_replayed: fallback_dependency_inventory_replay_decision,
        emergency_recovery_rehearsed: emergency_recovery_rehearsal_decision,
        production_forwarding_unchanged_verified: production_forwarding_unchanged_verification_decision,
        dry_run_evidence_archived: dry_run_evidence_archive_decision,
        mihomo_fallback_retirement_dry_run_complete: blockers.is_empty(),
        dry_run_surfaces,
        blockers,
        facts: vec![
            "Mihomo fallback retirement dry-run replays removal evidence without changing forwarding".into(),
            "dry-run success advances only to readiness closeout".into(),
        ],
    }
}

pub async fn rust_kernel_runtime_data_plane_hardening_mihomo_fallback_retirement_dry_run(
    rust_data_plane_hardening_mihomo_fallback_retirement_guard_complete_decision: Option<bool>,
    guard_review_decision: Option<bool>,
    parity_manifest_replay_decision: Option<bool>,
    cross_platform_rollback_rehearsal_decision: Option<bool>,
    fallback_dependency_inventory_replay_decision: Option<bool>,
    emergency_recovery_rehearsal_decision: Option<bool>,
    production_forwarding_unchanged_verification_decision: Option<bool>,
    dry_run_evidence_archive_decision: Option<bool>,
    final_mihomo_fallback_retirement_dry_run_decision: Option<bool>,
) -> Result<KernelLoopbackRustDataPlaneHardeningMihomoFallbackRetirementDryRunReport> {
    let rust_data_plane_hardening_mihomo_fallback_retirement_guard_complete =
        rust_data_plane_hardening_mihomo_fallback_retirement_guard_complete_decision.unwrap_or(false);
    let final_mihomo_fallback_retirement_dry_run_decision =
        final_mihomo_fallback_retirement_dry_run_decision.unwrap_or(false);
    let mihomo_fallback_retirement_dry_run =
        rust_kernel_runtime_data_plane_hardening_mihomo_fallback_retirement_dry_run_report(
            guard_review_decision.unwrap_or(false),
            parity_manifest_replay_decision.unwrap_or(false),
            cross_platform_rollback_rehearsal_decision.unwrap_or(false),
            fallback_dependency_inventory_replay_decision.unwrap_or(false),
            emergency_recovery_rehearsal_decision.unwrap_or(false),
            production_forwarding_unchanged_verification_decision.unwrap_or(false),
            dry_run_evidence_archive_decision.unwrap_or(false),
        );
    let guard_blockers = if rust_data_plane_hardening_mihomo_fallback_retirement_guard_complete {
        Vec::new()
    } else {
        vec!["Rust data-plane Mihomo fallback retirement dry-run requires guard to pass first".into()]
    };

    let checks = vec![
        data_plane_hardening_gate_check(
            "rustDataPlaneHardeningMihomoFallbackRetirementGuardComplete",
            rust_data_plane_hardening_mihomo_fallback_retirement_guard_complete,
            guard_blockers,
            "fallback retirement dry-run starts only after the guard",
        ),
        data_plane_hardening_gate_check(
            "mihomoFallbackRetirementDryRunComplete",
            mihomo_fallback_retirement_dry_run.mihomo_fallback_retirement_dry_run_complete,
            mihomo_fallback_retirement_dry_run.blockers.clone(),
            "guard review, parity manifest, rollback, fallback inventory, emergency recovery, unchanged forwarding, and evidence are evaluated together",
        ),
        data_plane_hardening_gate_check(
            "finalMihomoFallbackRetirementDryRunDecision",
            final_mihomo_fallback_retirement_dry_run_decision,
            if final_mihomo_fallback_retirement_dry_run_decision {
                Vec::new()
            } else {
                vec!["Rust data-plane Mihomo fallback retirement dry-run requires an explicit final decision".into()]
            },
            "fallback retirement dry-run completion is explicit before readiness closeout",
        ),
    ];
    let rust_data_plane_hardening_mihomo_fallback_retirement_dry_run_complete = checks.iter().all(|check| check.passed);
    let blockers = checks
        .iter()
        .flat_map(|check| check.blockers.clone())
        .collect::<Vec<String>>();

    Ok(
        KernelLoopbackRustDataPlaneHardeningMihomoFallbackRetirementDryRunReport {
            runtime_id: RUST_RUNTIME_ID.into(),
            component: "rust-data-plane-hardening-mihomo-fallback-retirement-dry-run".into(),
            mutates_runtime: false,
            live_execution_allowed: false,
            production_data_plane_mutation_allowed: false,
            rust_data_plane_hardening_mihomo_fallback_retirement_guard_complete,
            mihomo_fallback_retirement_dry_run,
            final_mihomo_fallback_retirement_dry_run_decision,
            rust_data_plane_hardening_mihomo_fallback_retirement_dry_run_complete,
            selected_runtime_kind: KernelRuntimeKind::Rust,
            rollback_runtime_kind: KernelRuntimeKind::Mihomo,
            checks,
            blockers,
            warnings: vec![
                "this fallback retirement dry-run does not remove Mihomo fallback or mutate production forwarding"
                    .into(),
                "readiness closeout remains required before any retirement execution surface".into(),
            ],
            facts: vec![
                "Rust data-plane hardening Mihomo fallback retirement dry-run follows the guard".into(),
                "successful dry-run advances only to fallback retirement readiness".into(),
            ],
            next_safe_batch: if rust_data_plane_hardening_mihomo_fallback_retirement_dry_run_complete {
                "rust-data-plane-hardening-mihomo-fallback-retirement-readiness".into()
            } else {
                "rust-data-plane-hardening-mihomo-fallback-retirement-dry-run".into()
            },
        },
    )
}

fn rust_kernel_runtime_data_plane_hardening_mihomo_fallback_retirement_readiness_report(
    dry_run_review_decision: bool,
    protocol_parity_evidence_archive_decision: bool,
    tun_parity_evidence_archive_decision: bool,
    adapter_parity_evidence_archive_decision: bool,
    dns_parity_evidence_archive_decision: bool,
    soak_evidence_archive_decision: bool,
    emergency_rollback_owner_acknowledgement_decision: bool,
) -> RustKernelRuntimeDataPlaneHardeningMihomoFallbackRetirementReadinessReport {
    let (readiness_surfaces, blockers) = collect_data_plane_hardening_surfaces(&[
        (
            "Mihomo fallback retirement dry-run review",
            dry_run_review_decision,
            "Rust data-plane Mihomo fallback retirement readiness requires dry-run review",
        ),
        (
            "archived protocol parity evidence",
            protocol_parity_evidence_archive_decision,
            "Rust data-plane Mihomo fallback retirement readiness requires protocol parity evidence",
        ),
        (
            "archived TUN parity evidence",
            tun_parity_evidence_archive_decision,
            "Rust data-plane Mihomo fallback retirement readiness requires TUN parity evidence",
        ),
        (
            "archived adapter parity evidence",
            adapter_parity_evidence_archive_decision,
            "Rust data-plane Mihomo fallback retirement readiness requires adapter parity evidence",
        ),
        (
            "archived DNS parity evidence",
            dns_parity_evidence_archive_decision,
            "Rust data-plane Mihomo fallback retirement readiness requires DNS parity evidence",
        ),
        (
            "archived soak evidence",
            soak_evidence_archive_decision,
            "Rust data-plane Mihomo fallback retirement readiness requires soak evidence",
        ),
        (
            "emergency rollback owner acknowledgement",
            emergency_rollback_owner_acknowledgement_decision,
            "Rust data-plane Mihomo fallback retirement readiness requires emergency rollback owner acknowledgement",
        ),
    ]);

    RustKernelRuntimeDataPlaneHardeningMihomoFallbackRetirementReadinessReport {
        runtime_id: RUST_RUNTIME_ID.into(),
        component: "rust-data-plane-hardening-mihomo-fallback-retirement-readiness-detail".into(),
        dry_run_reviewed: dry_run_review_decision,
        protocol_parity_evidence_archived: protocol_parity_evidence_archive_decision,
        tun_parity_evidence_archived: tun_parity_evidence_archive_decision,
        adapter_parity_evidence_archived: adapter_parity_evidence_archive_decision,
        dns_parity_evidence_archived: dns_parity_evidence_archive_decision,
        soak_evidence_archived: soak_evidence_archive_decision,
        emergency_rollback_owner_acknowledged: emergency_rollback_owner_acknowledgement_decision,
        mihomo_fallback_retirement_readiness_complete: blockers.is_empty(),
        readiness_surfaces,
        blockers,
        facts: vec![
            "Mihomo fallback retirement readiness requires parity evidence before any fallback removal".into(),
            "readiness success advances only to a separately gated execution surface".into(),
        ],
    }
}

pub async fn rust_kernel_runtime_data_plane_hardening_mihomo_fallback_retirement_readiness(
    rust_data_plane_hardening_mihomo_fallback_retirement_dry_run_complete_decision: Option<bool>,
    dry_run_review_decision: Option<bool>,
    protocol_parity_evidence_archive_decision: Option<bool>,
    tun_parity_evidence_archive_decision: Option<bool>,
    adapter_parity_evidence_archive_decision: Option<bool>,
    dns_parity_evidence_archive_decision: Option<bool>,
    soak_evidence_archive_decision: Option<bool>,
    emergency_rollback_owner_acknowledgement_decision: Option<bool>,
    final_mihomo_fallback_retirement_readiness_decision: Option<bool>,
) -> Result<KernelLoopbackRustDataPlaneHardeningMihomoFallbackRetirementReadinessReport> {
    let rust_data_plane_hardening_mihomo_fallback_retirement_dry_run_complete =
        rust_data_plane_hardening_mihomo_fallback_retirement_dry_run_complete_decision.unwrap_or(false);
    let final_mihomo_fallback_retirement_readiness_decision =
        final_mihomo_fallback_retirement_readiness_decision.unwrap_or(false);
    let mihomo_fallback_retirement_readiness =
        rust_kernel_runtime_data_plane_hardening_mihomo_fallback_retirement_readiness_report(
            dry_run_review_decision.unwrap_or(false),
            protocol_parity_evidence_archive_decision.unwrap_or(false),
            tun_parity_evidence_archive_decision.unwrap_or(false),
            adapter_parity_evidence_archive_decision.unwrap_or(false),
            dns_parity_evidence_archive_decision.unwrap_or(false),
            soak_evidence_archive_decision.unwrap_or(false),
            emergency_rollback_owner_acknowledgement_decision.unwrap_or(false),
        );
    let dry_run_blockers = if rust_data_plane_hardening_mihomo_fallback_retirement_dry_run_complete {
        Vec::new()
    } else {
        vec!["Rust data-plane Mihomo fallback retirement readiness requires dry-run to pass first".into()]
    };

    let checks = vec![
        data_plane_hardening_gate_check(
            "rustDataPlaneHardeningMihomoFallbackRetirementDryRunComplete",
            rust_data_plane_hardening_mihomo_fallback_retirement_dry_run_complete,
            dry_run_blockers,
            "fallback retirement readiness starts only after dry-run",
        ),
        data_plane_hardening_gate_check(
            "mihomoFallbackRetirementReadinessComplete",
            mihomo_fallback_retirement_readiness.mihomo_fallback_retirement_readiness_complete,
            mihomo_fallback_retirement_readiness.blockers.clone(),
            "dry-run review, parity evidence, soak evidence, and emergency rollback ownership are evaluated together",
        ),
        data_plane_hardening_gate_check(
            "finalMihomoFallbackRetirementReadinessDecision",
            final_mihomo_fallback_retirement_readiness_decision,
            if final_mihomo_fallback_retirement_readiness_decision {
                Vec::new()
            } else {
                vec!["Rust data-plane Mihomo fallback retirement readiness requires an explicit final decision".into()]
            },
            "fallback retirement readiness is explicit before execution",
        ),
    ];
    let rust_data_plane_hardening_mihomo_fallback_retirement_readiness_complete =
        checks.iter().all(|check| check.passed);
    let blockers = checks
        .iter()
        .flat_map(|check| check.blockers.clone())
        .collect::<Vec<String>>();

    Ok(
        KernelLoopbackRustDataPlaneHardeningMihomoFallbackRetirementReadinessReport {
            runtime_id: RUST_RUNTIME_ID.into(),
            component: "rust-data-plane-hardening-mihomo-fallback-retirement-readiness".into(),
            mutates_runtime: false,
            live_execution_allowed: false,
            production_data_plane_mutation_allowed: false,
            rust_data_plane_hardening_mihomo_fallback_retirement_dry_run_complete,
            mihomo_fallback_retirement_readiness,
            final_mihomo_fallback_retirement_readiness_decision,
            rust_data_plane_hardening_mihomo_fallback_retirement_readiness_complete,
            selected_runtime_kind: KernelRuntimeKind::Rust,
            rollback_runtime_kind: KernelRuntimeKind::Mihomo,
            checks,
            blockers,
            warnings: vec![
                "this fallback retirement readiness surface does not remove Mihomo fallback or mutate production forwarding".into(),
                "actual fallback retirement remains blocked until a later explicit execution PR".into(),
            ],
            facts: vec![
                "Rust data-plane hardening Mihomo fallback retirement readiness follows dry-run".into(),
                "successful readiness advances only to fallback retirement execution planning".into(),
            ],
            next_safe_batch: if rust_data_plane_hardening_mihomo_fallback_retirement_readiness_complete {
                "rust-data-plane-hardening-mihomo-fallback-retirement-execution".into()
            } else {
                "rust-data-plane-hardening-mihomo-fallback-retirement-readiness".into()
            },
        },
    )
}
