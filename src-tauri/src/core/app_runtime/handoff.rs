use super::*;
use crate::core::dns_runtime::{
    DnsDefaultRuntimeExpandedControlPlaneCompletionReport, DnsDefaultRuntimeExpandedControlPlaneCompletionStatus,
    dns_default_runtime_expanded_control_plane_completion,
};
use crate::utils::{dirs, help};
use tokio::fs;

pub async fn accept_app_runtime_dns_handoff() -> Result<AppRuntimeDnsHandoffReport> {
    let dns_completion = dns_default_runtime_expanded_control_plane_completion().await?;
    persist_app_runtime_dns_handoff_report(dns_completion).await
}

pub async fn complete_app_runtime_control_plane(
    request: AppRuntimePlanRequest,
) -> Result<AppRuntimeControlPlaneCompletionReport> {
    let dns_handoff = accept_app_runtime_dns_handoff().await?;
    let state = read_app_runtime_state_document().await?;
    let mut projection_artifact = build_app_runtime_projection_artifact(&state, request)?;
    let projection_artifact_path = persist_app_runtime_projection_artifact(&projection_artifact).await?;
    projection_artifact.storage_path = Some(projection_artifact_path.clone());
    let activation_preflight =
        preflight_app_runtime_projection_activation(AppRuntimeProjectionActivationPreflightRequest {
            artifact_id: projection_artifact.artifact_id.clone(),
            expected_checksum: Some(projection_artifact.checksum.clone()),
        })
        .await?;

    Ok(build_app_runtime_control_plane_completion_report(
        dns_handoff,
        projection_artifact,
        Some(projection_artifact_path),
        true,
        activation_preflight,
    ))
}

pub async fn complete_app_runtime_staged_activation_lifecycle(
    request: AppRuntimePlanRequest,
) -> Result<AppRuntimeStagedActivationLifecycleReport> {
    let control_plane_completion = complete_app_runtime_control_plane(request).await?;
    if !control_plane_completion.ready_for_staged_activation {
        return Ok(build_app_runtime_staged_activation_lifecycle_report(
            control_plane_completion,
            None,
            false,
        ));
    }

    let state = activate_app_runtime_projection_artifact(AppRuntimeProjectionActivationPreflightRequest {
        artifact_id: control_plane_completion.projection_artifact.artifact_id.clone(),
        expected_checksum: Some(control_plane_completion.projection_artifact.checksum.clone()),
    })
    .await?;

    Ok(build_app_runtime_staged_activation_lifecycle_report(
        control_plane_completion,
        state.active_projection,
        true,
    ))
}

pub async fn closeout_app_runtime_staged_activation_lifecycle(
    request: AppRuntimePlanRequest,
) -> Result<AppRuntimeStagedActivationCloseoutReport> {
    let lifecycle = complete_app_runtime_staged_activation_lifecycle(request).await?;
    persist_app_runtime_staged_activation_closeout_report(lifecycle).await
}

pub async fn decide_app_runtime_runtime_apply_boundary(
    request: AppRuntimeRuntimeApplyBoundaryDecisionRequest,
) -> Result<AppRuntimeRuntimeApplyBoundaryDecisionReport> {
    let closeout = closeout_app_runtime_staged_activation_lifecycle(AppRuntimePlanRequest {
        app_id: request.app_id.clone(),
        session_id: None,
    })
    .await?;
    persist_app_runtime_runtime_apply_boundary_decision_report(closeout, request).await
}

pub fn build_app_runtime_dns_handoff_report(
    dns_completion: DnsDefaultRuntimeExpandedControlPlaneCompletionReport,
    handoff_record_path: Option<String>,
    handoff_record_persisted: bool,
    mut persist_errors: Vec<String>,
    created_at: i64,
) -> AppRuntimeDnsHandoffReport {
    let handoff_id = format!("app-runtime-dns-handoff-{created_at}");
    let app_runtime_accepts_handoff = dns_completion.dns_control_plane_complete;
    let next_app_runtime_step: String = if dns_completion.rollback_recommended {
        "runExplicitDnsExpandedRollbackBeforeAppRuntimeFollowup"
    } else if app_runtime_accepts_handoff {
        "continueAppRuntimeProjectionAndDiagnosticsCompletion"
    } else {
        "continueDnsExpandedObservationBeforeAppRuntimeFollowup"
    }
    .into();
    let handoff_record = AppRuntimeDnsHandoffRecord {
        handoff_id: handoff_id.into(),
        action: "acceptAppRuntimeDnsHandoff".into(),
        dns_completion_status: dns_completion.status,
        dns_control_plane_complete: dns_completion.dns_control_plane_complete,
        dns_handoff_ready: dns_completion.handoff_ready,
        dns_manifest_path: dns_completion.handoff_manifest_path.clone().map(Into::into),
        app_runtime_accepts_handoff,
        app_runtime_followup_scope: "app-runtime-control-plane".into(),
        next_app_runtime_step: next_app_runtime_step.clone(),
        phase8_allowed: false,
        promotion_allowed: false,
        auto_rollout: false,
        auto_rollback: false,
        mutates_runtime: false,
        reload_mihomo: false,
        created_at,
    };
    let mut blockers: Vec<String> = dns_completion.blockers.iter().cloned().map(Into::into).collect();
    if !handoff_record_persisted {
        blockers.append(&mut persist_errors);
    }
    let status = if !blockers.is_empty() {
        AppRuntimeDnsHandoffStatus::Blocked
    } else if dns_completion.status == DnsDefaultRuntimeExpandedControlPlaneCompletionStatus::RollbackRecommended {
        AppRuntimeDnsHandoffStatus::RollbackRecommended
    } else if app_runtime_accepts_handoff {
        AppRuntimeDnsHandoffStatus::Accepted
    } else {
        AppRuntimeDnsHandoffStatus::Watching
    };
    let mut warnings: Vec<String> = dns_completion.warnings.iter().cloned().map(Into::into).collect();
    warnings.push("App runtime DNS handoff intake is not a Phase 8 runtime migration".into());
    let facts = vec![
        "app-runtime DNS handoff intake consumes DNS expanded completion".into(),
        "app-runtime DNS handoff intake persists an app-runtime handoff record".into(),
        "app-runtime DNS handoff intake keeps phase8Allowed=false".into(),
        "app-runtime DNS handoff intake does not mutate runtime or reload Mihomo".into(),
    ];

    AppRuntimeDnsHandoffReport {
        status,
        reason: app_runtime_dns_handoff_reason(status, &blockers),
        dns_completion,
        handoff_record,
        handoff_record_path,
        handoff_record_persisted,
        app_runtime_accepts_handoff,
        next_app_runtime_step,
        phase8_allowed: false,
        promotion_allowed: false,
        user_trigger_required: true,
        auto_rollout: false,
        auto_rollback: false,
        mutates_runtime: false,
        reload_mihomo: false,
        blockers,
        warnings,
        facts,
    }
}

pub fn build_app_runtime_staged_activation_closeout_report(
    lifecycle: AppRuntimeStagedActivationLifecycleReport,
    boundary_manifest_path: Option<String>,
    boundary_manifest_persisted: bool,
    mut persist_errors: Vec<String>,
    created_at: i64,
) -> AppRuntimeStagedActivationCloseoutReport {
    let artifact = &lifecycle.control_plane_completion.projection_artifact;
    let next_app_runtime_step: String = if lifecycle.status == AppRuntimeStagedActivationLifecycleStatus::Ready {
        "holdAtRuntimeApplyBoundaryUntilExplicitUserDecision"
    } else {
        "resolveStagedLifecycleBeforeRuntimeApplyBoundary"
    }
    .into();
    let boundary_manifest = AppRuntimeRuntimeApplyBoundaryManifest {
        manifest_id: format!("app-runtime-boundary-{created_at}").into(),
        app_id: artifact.app_id.clone(),
        artifact_id: artifact.artifact_id.clone(),
        checksum: artifact.checksum.clone(),
        active_marker_matches_artifact: lifecycle.active_marker_matches_artifact,
        rollback_boundary_available: lifecycle.rollback_boundary_available,
        rollback_strategy: lifecycle.rollback_strategy.clone(),
        runtime_apply_allowed: false,
        phase8_allowed: false,
        promotion_allowed: false,
        auto_rollout: false,
        auto_rollback: false,
        mutates_runtime: false,
        reload_mihomo: false,
        next_app_runtime_step: next_app_runtime_step.clone(),
        created_at,
    };
    let mut blockers = lifecycle.blockers.clone();
    if lifecycle.status == AppRuntimeStagedActivationLifecycleStatus::Blocked {
        blockers.push("staged activation closeout requires ready lifecycle".into());
    }
    if !lifecycle.active_marker_matches_artifact {
        blockers.push("staged activation closeout requires active marker to match artifact".into());
    }
    if !lifecycle.rollback_boundary_available {
        blockers.push("staged activation closeout requires rollback boundary".into());
    }
    if !boundary_manifest_persisted {
        blockers.append(&mut persist_errors);
    }
    let status = if !blockers.is_empty() {
        AppRuntimeStagedActivationCloseoutStatus::Blocked
    } else if lifecycle.status == AppRuntimeStagedActivationLifecycleStatus::Degraded {
        AppRuntimeStagedActivationCloseoutStatus::Degraded
    } else {
        AppRuntimeStagedActivationCloseoutStatus::Complete
    };
    let mut warnings = lifecycle.warnings.clone();
    warnings.push("Closeout manifest is a boundary marker; it does not allow automatic runtime apply".into());
    warnings.sort();
    warnings.dedup();
    let facts = vec![
        "app-runtime staged activation closeout consumes staged lifecycle".into(),
        "app-runtime staged activation closeout persists runtime-apply boundary manifest".into(),
        "runtime apply remains explicit and disabled by default".into(),
        "app-runtime staged activation closeout does not reload Mihomo".into(),
    ];

    AppRuntimeStagedActivationCloseoutReport {
        status,
        reason: app_runtime_staged_activation_closeout_reason(status, &blockers),
        lifecycle,
        boundary_manifest,
        boundary_manifest_path,
        boundary_manifest_persisted,
        closeout_complete: status == AppRuntimeStagedActivationCloseoutStatus::Complete,
        runtime_apply_allowed: false,
        phase8_allowed: false,
        promotion_allowed: false,
        user_trigger_required: true,
        auto_rollout: false,
        auto_rollback: false,
        mutates_runtime: false,
        reload_mihomo: false,
        next_app_runtime_step,
        blockers,
        warnings,
        facts,
    }
}

pub fn build_app_runtime_runtime_apply_boundary_decision_report(
    closeout: AppRuntimeStagedActivationCloseoutReport,
    request: AppRuntimeRuntimeApplyBoundaryDecisionRequest,
    decision_record_path: Option<String>,
    decision_record_persisted: bool,
    mut persist_errors: Vec<String>,
    created_at: i64,
) -> AppRuntimeRuntimeApplyBoundaryDecisionReport {
    let boundary = &closeout.boundary_manifest;
    let boundary_ready = closeout.closeout_complete
        && closeout.boundary_manifest_persisted
        && boundary.active_marker_matches_artifact
        && boundary.rollback_boundary_available;
    let runtime_apply_candidate_allowed =
        boundary_ready && request.decision == AppRuntimeRuntimeApplyBoundaryDecision::AllowRuntimeCandidate;
    let rollback_recommended = request.decision == AppRuntimeRuntimeApplyBoundaryDecision::RecommendRollback;
    let next_app_runtime_step: String = if runtime_apply_candidate_allowed {
        "userMayExplicitlyApplyRuntimeCandidate".into()
    } else if rollback_recommended {
        "runExplicitStagedActivationRollbackBeforeRuntimeApply".into()
    } else {
        "keepHoldingAtRuntimeApplyBoundary".into()
    };
    let mut blockers = closeout.blockers.clone();
    if request.app_id != boundary.app_id {
        blockers.push("runtime-apply boundary decision app id does not match boundary manifest".into());
    }
    if request.decision == AppRuntimeRuntimeApplyBoundaryDecision::AllowRuntimeCandidate && !boundary_ready {
        blockers.push("runtime-apply boundary decision requires complete closeout and rollback boundary".into());
    }
    if !decision_record_persisted {
        blockers.append(&mut persist_errors);
    }
    let status = if !blockers.is_empty() {
        AppRuntimeRuntimeApplyBoundaryDecisionStatus::Blocked
    } else {
        match request.decision {
            AppRuntimeRuntimeApplyBoundaryDecision::AllowRuntimeCandidate => {
                AppRuntimeRuntimeApplyBoundaryDecisionStatus::Accepted
            }
            AppRuntimeRuntimeApplyBoundaryDecision::DeferRuntimeApply => {
                AppRuntimeRuntimeApplyBoundaryDecisionStatus::Deferred
            }
            AppRuntimeRuntimeApplyBoundaryDecision::RecommendRollback => {
                AppRuntimeRuntimeApplyBoundaryDecisionStatus::RollbackRecommended
            }
        }
    };
    let decision_record = AppRuntimeRuntimeApplyBoundaryDecisionRecord {
        decision_id: format!("app-runtime-runtime-apply-decision-{created_at}").into(),
        app_id: boundary.app_id.clone(),
        artifact_id: boundary.artifact_id.clone(),
        checksum: boundary.checksum.clone(),
        boundary_manifest_id: boundary.manifest_id.clone(),
        boundary_manifest_path: closeout.boundary_manifest_path.clone(),
        decision: request.decision,
        rationale: request.rationale,
        decision_accepted: status != AppRuntimeRuntimeApplyBoundaryDecisionStatus::Blocked,
        runtime_apply_candidate_allowed,
        rollback_recommended,
        runtime_apply_allowed: runtime_apply_candidate_allowed,
        phase8_allowed: false,
        promotion_allowed: false,
        auto_rollout: false,
        auto_rollback: false,
        mutates_runtime: false,
        reload_mihomo: false,
        next_app_runtime_step: next_app_runtime_step.clone(),
        created_at,
    };
    let mut warnings = closeout.warnings.clone();
    warnings.push("Runtime-apply boundary decision records user intent but does not apply runtime".into());
    warnings.sort();
    warnings.dedup();
    let facts = vec![
        "runtime-apply boundary decision consumes staged closeout manifest".into(),
        "runtime-apply boundary decision persists an explicit audit record".into(),
        "runtime-apply boundary decision does not reload Mihomo".into(),
        "runtime-apply boundary decision keeps phase8Allowed=false".into(),
    ];

    AppRuntimeRuntimeApplyBoundaryDecisionReport {
        status,
        reason: app_runtime_runtime_apply_boundary_decision_reason(status, &blockers),
        closeout,
        decision_record,
        decision_record_path,
        decision_record_persisted,
        runtime_apply_candidate_allowed,
        rollback_recommended,
        runtime_apply_allowed: runtime_apply_candidate_allowed,
        phase8_allowed: false,
        promotion_allowed: false,
        user_trigger_required: true,
        auto_rollout: false,
        auto_rollback: false,
        mutates_runtime: false,
        reload_mihomo: false,
        next_app_runtime_step,
        blockers,
        warnings,
        facts,
    }
}

pub fn build_app_runtime_staged_activation_lifecycle_report(
    control_plane_completion: AppRuntimeControlPlaneCompletionReport,
    active_projection: Option<AppRuntimeActiveProjectionRecord>,
    marker_activated: bool,
) -> AppRuntimeStagedActivationLifecycleReport {
    let artifact = &control_plane_completion.projection_artifact;
    let active_marker_matches_artifact = active_projection
        .as_ref()
        .is_some_and(|active| active.artifact_id == artifact.artifact_id && active.checksum == artifact.checksum);
    let rollback_strategy = active_projection
        .as_ref()
        .map(|active| active.rollback.rollback_strategy.clone());
    let rollback_boundary_available = active_projection.is_some();
    let mut blockers = control_plane_completion.blockers.clone();
    if control_plane_completion.status == AppRuntimeControlPlaneCompletionStatus::Blocked {
        blockers.push("staged activation lifecycle requires completed app-runtime control plane".into());
    }
    if marker_activated && !active_marker_matches_artifact {
        blockers.push("staged activation marker does not match generated projection artifact".into());
    }
    if active_projection.as_ref().is_some_and(|active| active.mutates_runtime) {
        blockers.push("staged activation marker must not record runtime mutation".into());
    }

    let status = if !blockers.is_empty() {
        AppRuntimeStagedActivationLifecycleStatus::Blocked
    } else if marker_activated && active_marker_matches_artifact {
        AppRuntimeStagedActivationLifecycleStatus::Ready
    } else {
        AppRuntimeStagedActivationLifecycleStatus::Degraded
    };
    let mut warnings = control_plane_completion.warnings.clone();
    if !marker_activated {
        warnings.push("Staged activation marker was skipped because completion is not ready".into());
    }
    warnings.push("Staged activation lifecycle does not perform runtime apply".into());
    warnings.sort();
    warnings.dedup();
    let facts = vec![
        "app-runtime staged activation lifecycle consumes control-plane completion".into(),
        "staged activation marker records app-runtime state only".into(),
        "staged activation lifecycle keeps runtimeApplyAllowed=false".into(),
        "staged activation lifecycle does not reload Mihomo".into(),
    ];
    let next_app_runtime_step = if status == AppRuntimeStagedActivationLifecycleStatus::Ready {
        "reviewStagedMarkerBeforeExplicitRuntimeApplyDecision"
    } else if status == AppRuntimeStagedActivationLifecycleStatus::Degraded {
        "rerunControlPlaneCompletionBeforeStagedActivation"
    } else {
        "resolveStagedActivationLifecycleBlockers"
    }
    .into();

    AppRuntimeStagedActivationLifecycleReport {
        status,
        reason: app_runtime_staged_activation_lifecycle_reason(status, &blockers),
        app_id: artifact.app_id.clone(),
        control_plane_completion,
        active_projection,
        marker_activated,
        active_marker_matches_artifact,
        rollback_boundary_available,
        rollback_strategy,
        runtime_apply_allowed: false,
        phase8_allowed: false,
        promotion_allowed: false,
        user_trigger_required: true,
        auto_rollout: false,
        auto_rollback: false,
        mutates_runtime: false,
        reload_mihomo: false,
        next_app_runtime_step,
        blockers,
        warnings,
        facts,
    }
}

async fn persist_app_runtime_staged_activation_closeout_report(
    lifecycle: AppRuntimeStagedActivationLifecycleReport,
) -> Result<AppRuntimeStagedActivationCloseoutReport> {
    let created_at = now_millis();
    let manifest_id = format!("app-runtime-boundary-{created_at}");
    let path = app_runtime_staged_activation_closeout_path(&manifest_id)?;
    let mut persist_errors = Vec::new();
    if let Some(parent) = path.parent() {
        if let Err(error) = fs::create_dir_all(parent).await {
            persist_errors.push(format!("failed to create app-runtime staged closeout directory: {error}").into());
        }
    }
    let report = build_app_runtime_staged_activation_closeout_report(
        lifecycle,
        Some(path.to_string_lossy().to_string().into()),
        persist_errors.is_empty(),
        persist_errors,
        created_at,
    );
    if report.boundary_manifest_persisted {
        if let Err(error) = help::save_yaml(&path, &report.boundary_manifest, None).await {
            return Ok(build_app_runtime_staged_activation_closeout_report(
                report.lifecycle,
                Some(path.to_string_lossy().to_string().into()),
                false,
                vec![format!("failed to persist app-runtime staged closeout manifest: {error}").into()],
                created_at,
            ));
        }
    }
    Ok(report)
}

async fn persist_app_runtime_runtime_apply_boundary_decision_report(
    closeout: AppRuntimeStagedActivationCloseoutReport,
    request: AppRuntimeRuntimeApplyBoundaryDecisionRequest,
) -> Result<AppRuntimeRuntimeApplyBoundaryDecisionReport> {
    let created_at = now_millis();
    let decision_id = format!("app-runtime-runtime-apply-decision-{created_at}");
    let path = app_runtime_runtime_apply_boundary_decision_path(&decision_id)?;
    let mut persist_errors = Vec::new();
    if let Some(parent) = path.parent() {
        if let Err(error) = fs::create_dir_all(parent).await {
            persist_errors
                .push(format!("failed to create app-runtime runtime-apply decision directory: {error}").into());
        }
    }
    let report = build_app_runtime_runtime_apply_boundary_decision_report(
        closeout,
        request,
        Some(path.to_string_lossy().to_string().into()),
        persist_errors.is_empty(),
        persist_errors,
        created_at,
    );
    if report.decision_record_persisted {
        if let Err(error) = help::save_yaml(&path, &report.decision_record, None).await {
            return Ok(build_app_runtime_runtime_apply_boundary_decision_report(
                report.closeout,
                AppRuntimeRuntimeApplyBoundaryDecisionRequest {
                    app_id: report.decision_record.app_id,
                    decision: report.decision_record.decision,
                    rationale: report.decision_record.rationale,
                },
                Some(path.to_string_lossy().to_string().into()),
                false,
                vec![format!("failed to persist app-runtime runtime-apply decision record: {error}").into()],
                created_at,
            ));
        }
    }
    Ok(report)
}

pub fn build_app_runtime_control_plane_completion_report(
    dns_handoff: AppRuntimeDnsHandoffReport,
    projection_artifact: AppRuntimeProjectionArtifact,
    projection_artifact_path: Option<String>,
    projection_artifact_persisted: bool,
    activation_preflight: AppRuntimeProjectionActivationPreflightReport,
) -> AppRuntimeControlPlaneCompletionReport {
    let mut blockers = Vec::new();
    if dns_handoff.status != AppRuntimeDnsHandoffStatus::Accepted {
        blockers.push("app-runtime control-plane completion requires accepted DNS handoff".into());
    }
    if !projection_artifact_persisted {
        blockers.push("app-runtime projection artifact was not persisted".into());
    }
    if projection_artifact.validation.status == AppRuntimeDiagnosticStatus::Blocked {
        blockers.push("app-runtime projection artifact validation is blocked".into());
    }
    if activation_preflight.status == AppRuntimeDiagnosticStatus::Blocked {
        blockers.push("app-runtime staged activation preflight is blocked".into());
    }
    if projection_artifact.mutates_runtime {
        blockers.push("app-runtime projection artifact must remain planning-only".into());
    }
    blockers.extend(dns_handoff.blockers.iter().cloned());

    let ready_for_staged_activation = blockers.is_empty()
        && activation_preflight.status == AppRuntimeDiagnosticStatus::Healthy
        && projection_artifact.validation.status == AppRuntimeDiagnosticStatus::Healthy;
    let status = if !blockers.is_empty() {
        AppRuntimeControlPlaneCompletionStatus::Blocked
    } else if activation_preflight.status == AppRuntimeDiagnosticStatus::Degraded
        || projection_artifact.validation.status == AppRuntimeDiagnosticStatus::Degraded
    {
        AppRuntimeControlPlaneCompletionStatus::Degraded
    } else {
        AppRuntimeControlPlaneCompletionStatus::Ready
    };
    let mut warnings = projection_artifact.warnings.clone();
    warnings.extend(activation_preflight.warnings.iter().cloned());
    warnings.extend(dns_handoff.warnings.iter().cloned());
    warnings.push("App runtime control-plane completion stops before runtime apply".into());
    warnings.sort();
    warnings.dedup();
    let facts = vec![
        "app-runtime control-plane completion consumes DNS handoff intake".into(),
        "app-runtime control-plane completion persists a projection artifact".into(),
        "app-runtime control-plane completion runs staged activation preflight".into(),
        "app-runtime control-plane completion does not apply runtime or reload Mihomo".into(),
    ];
    let next_app_runtime_step = if ready_for_staged_activation {
        "userMayTriggerStagedActivationMarkerBeforeRuntimeApply"
    } else if status == AppRuntimeControlPlaneCompletionStatus::Degraded {
        "reviewWarningsBeforeStagedActivationMarker"
    } else {
        "resolveControlPlaneCompletionBlockers"
    }
    .into();

    AppRuntimeControlPlaneCompletionReport {
        status,
        reason: app_runtime_control_plane_completion_reason(status, &blockers),
        app_id: projection_artifact.app_id.clone(),
        dns_handoff,
        projection_artifact,
        projection_artifact_path,
        projection_artifact_persisted,
        activation_preflight,
        ready_for_staged_activation,
        runtime_apply_allowed: false,
        phase8_allowed: false,
        promotion_allowed: false,
        user_trigger_required: true,
        auto_rollout: false,
        auto_rollback: false,
        mutates_runtime: false,
        reload_mihomo: false,
        next_app_runtime_step,
        blockers,
        warnings,
        facts,
    }
}

async fn persist_app_runtime_dns_handoff_report(
    dns_completion: DnsDefaultRuntimeExpandedControlPlaneCompletionReport,
) -> Result<AppRuntimeDnsHandoffReport> {
    let created_at = now_millis();
    let handoff_id = format!("app-runtime-dns-handoff-{created_at}");
    let path = app_runtime_dns_handoff_path(&handoff_id)?;
    let mut persist_errors = Vec::new();
    if let Some(parent) = path.parent() {
        if let Err(error) = fs::create_dir_all(parent).await {
            persist_errors.push(format!("failed to create app-runtime DNS handoff directory: {error}").into());
        }
    }
    let report = build_app_runtime_dns_handoff_report(
        dns_completion,
        Some(path.to_string_lossy().to_string().into()),
        persist_errors.is_empty(),
        persist_errors,
        created_at,
    );
    if report.handoff_record_persisted {
        if let Err(error) = help::save_yaml(&path, &report.handoff_record, None).await {
            return Ok(build_app_runtime_dns_handoff_report(
                report.dns_completion,
                Some(path.to_string_lossy().to_string().into()),
                false,
                vec![format!("failed to persist app-runtime DNS handoff record: {error}").into()],
                created_at,
            ));
        }
    }
    Ok(report)
}

fn app_runtime_dns_handoff_path(handoff_id: &str) -> Result<std::path::PathBuf> {
    let safe_segment = safe_app_runtime_handoff_segment(handoff_id);
    Ok(dirs::app_runtime_dir()?
        .join("dns-handoffs")
        .join(safe_segment)
        .join("handoff.yaml"))
}

fn app_runtime_staged_activation_closeout_path(manifest_id: &str) -> Result<std::path::PathBuf> {
    let safe_segment = safe_app_runtime_handoff_segment(manifest_id);
    Ok(dirs::app_runtime_dir()?
        .join("staged-closeout")
        .join(safe_segment)
        .join("runtime-apply-boundary.yaml"))
}

fn app_runtime_runtime_apply_boundary_decision_path(decision_id: &str) -> Result<std::path::PathBuf> {
    let safe_segment = safe_app_runtime_handoff_segment(decision_id);
    Ok(dirs::app_runtime_dir()?
        .join("runtime-apply-decisions")
        .join(safe_segment)
        .join("decision.yaml"))
}

fn safe_app_runtime_handoff_segment(input: &str) -> std::string::String {
    input
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' {
                ch
            } else {
                '_'
            }
        })
        .collect()
}

fn app_runtime_dns_handoff_reason(status: AppRuntimeDnsHandoffStatus, blockers: &[String]) -> String {
    match status {
        AppRuntimeDnsHandoffStatus::Accepted => "app runtime accepted DNS expanded control-plane handoff".into(),
        AppRuntimeDnsHandoffStatus::Watching => "app runtime DNS handoff is waiting for DNS expanded completion".into(),
        AppRuntimeDnsHandoffStatus::RollbackRecommended => {
            "app runtime DNS handoff recommends explicit DNS rollback before continuing".into()
        }
        AppRuntimeDnsHandoffStatus::Blocked => blockers
            .first()
            .cloned()
            .unwrap_or_else(|| "app runtime DNS handoff is blocked".into()),
    }
}

fn app_runtime_control_plane_completion_reason(
    status: AppRuntimeControlPlaneCompletionStatus,
    blockers: &[String],
) -> String {
    match status {
        AppRuntimeControlPlaneCompletionStatus::Ready => {
            "app runtime control-plane completion is ready for staged activation".into()
        }
        AppRuntimeControlPlaneCompletionStatus::Degraded => {
            "app runtime control-plane completion is ready with warnings".into()
        }
        AppRuntimeControlPlaneCompletionStatus::Blocked => blockers
            .first()
            .cloned()
            .unwrap_or_else(|| "app runtime control-plane completion is blocked".into()),
    }
}

fn app_runtime_staged_activation_lifecycle_reason(
    status: AppRuntimeStagedActivationLifecycleStatus,
    blockers: &[String],
) -> String {
    match status {
        AppRuntimeStagedActivationLifecycleStatus::Ready => "app runtime staged activation marker is ready".into(),
        AppRuntimeStagedActivationLifecycleStatus::Degraded => {
            "app runtime staged activation marker is waiting for a ready completion".into()
        }
        AppRuntimeStagedActivationLifecycleStatus::Blocked => blockers
            .first()
            .cloned()
            .unwrap_or_else(|| "app runtime staged activation lifecycle is blocked".into()),
    }
}

fn app_runtime_staged_activation_closeout_reason(
    status: AppRuntimeStagedActivationCloseoutStatus,
    blockers: &[String],
) -> String {
    match status {
        AppRuntimeStagedActivationCloseoutStatus::Complete => {
            "app runtime staged activation closeout completed at runtime-apply boundary".into()
        }
        AppRuntimeStagedActivationCloseoutStatus::Degraded => {
            "app runtime staged activation closeout completed with warnings".into()
        }
        AppRuntimeStagedActivationCloseoutStatus::Blocked => blockers
            .first()
            .cloned()
            .unwrap_or_else(|| "app runtime staged activation closeout is blocked".into()),
    }
}

fn app_runtime_runtime_apply_boundary_decision_reason(
    status: AppRuntimeRuntimeApplyBoundaryDecisionStatus,
    blockers: &[String],
) -> String {
    match status {
        AppRuntimeRuntimeApplyBoundaryDecisionStatus::Accepted => {
            "runtime-apply boundary decision allows explicit runtime candidate apply".into()
        }
        AppRuntimeRuntimeApplyBoundaryDecisionStatus::Deferred => {
            "runtime-apply boundary decision keeps holding at staged boundary".into()
        }
        AppRuntimeRuntimeApplyBoundaryDecisionStatus::RollbackRecommended => {
            "runtime-apply boundary decision recommends explicit staged rollback".into()
        }
        AppRuntimeRuntimeApplyBoundaryDecisionStatus::Blocked => blockers
            .first()
            .cloned()
            .unwrap_or_else(|| "runtime-apply boundary decision is blocked".into()),
    }
}
