use anyhow::Result;
use smartstring::alias::String;

use super::{
    KernelLoopbackR4ExpandedOptInLimitedRolloutGateCheck, KernelLoopbackRustDataPlaneHardeningPreflightReport,
    KernelRuntimeKind, RUST_RUNTIME_ID, RustKernelRuntimeDataPlaneHardeningBoundaryReport,
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
