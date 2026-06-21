use anyhow::Result;
use smartstring::alias::String;

use super::{
    KernelLoopbackR4ExpandedOptInLimitedRolloutGateCheck, KernelLoopbackRustDataPlaneHardeningBoundaryAuditReport,
    KernelLoopbackRustDataPlaneHardeningPreflightReport, KernelRuntimeKind, RUST_RUNTIME_ID,
    RustKernelRuntimeDataPlaneHardeningBoundaryAuditReport, RustKernelRuntimeDataPlaneHardeningBoundaryReport,
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
