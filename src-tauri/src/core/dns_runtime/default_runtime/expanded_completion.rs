use super::*;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum DnsDefaultRuntimeExpandedControlPlaneCompletionStatus {
    Complete,
    Watching,
    RollbackRecommended,
    Blocked,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DnsDefaultRuntimeExpandedHandoffManifest {
    pub manifest_id: String,
    pub action: String,
    pub closeout_status: DnsDefaultRuntimeExpandedLifecycleCloseoutStatus,
    pub history_status: DnsDefaultRuntimeExpandedReverifyHistoryStatus,
    pub active_execution_event_id: Option<String>,
    pub active_state: Option<String>,
    pub history_record_count: usize,
    pub stable_streak: usize,
    pub required_stable_records: usize,
    pub observation_closed: bool,
    pub handoff_ready: bool,
    pub rollback_recommended: bool,
    pub next_control_plane_step: String,
    pub phase8_allowed: bool,
    pub promotion_allowed: bool,
    pub auto_rollout: bool,
    pub auto_rollback: bool,
    pub mutates_runtime: bool,
    pub reload_mihomo: bool,
    pub created_at_epoch_seconds: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DnsDefaultRuntimeExpandedControlPlaneCompletionReport {
    pub status: DnsDefaultRuntimeExpandedControlPlaneCompletionStatus,
    pub reason: String,
    pub closeout: DnsDefaultRuntimeExpandedLifecycleCloseoutReport,
    pub handoff_manifest: DnsDefaultRuntimeExpandedHandoffManifest,
    pub handoff_manifest_path: Option<String>,
    pub handoff_manifest_persisted: bool,
    pub dns_control_plane_complete: bool,
    pub observation_closed: bool,
    pub handoff_ready: bool,
    pub rollback_recommended: bool,
    pub next_control_plane_step: String,
    pub phase8_allowed: bool,
    pub promotion_allowed: bool,
    pub user_trigger_required: bool,
    pub auto_rollout: bool,
    pub auto_rollback: bool,
    pub mutates_runtime: bool,
    pub reload_mihomo: bool,
    pub blockers: Vec<String>,
    pub warnings: Vec<String>,
    pub facts: Vec<String>,
}

pub async fn dns_default_runtime_expanded_control_plane_completion()
-> Result<DnsDefaultRuntimeExpandedControlPlaneCompletionReport> {
    let closeout = dns_default_runtime_expanded_lifecycle_closeout().await?;
    Ok(persist_dns_default_runtime_expanded_control_plane_completion_report(closeout).await)
}

pub fn build_dns_default_runtime_expanded_control_plane_completion_report(
    closeout: DnsDefaultRuntimeExpandedLifecycleCloseoutReport,
    handoff_manifest_persisted: bool,
    handoff_manifest_path: Option<String>,
    mut persist_errors: Vec<String>,
    created_at_epoch_seconds: u64,
) -> DnsDefaultRuntimeExpandedControlPlaneCompletionReport {
    let active_execution_event_id = closeout
        .active_state
        .as_ref()
        .map(|state| state.execution_event_id.clone());
    let active_state = closeout.active_state.as_ref().map(|state| state.state.clone());
    let manifest_id = format!("dns-default-runtime-expanded-control-plane-completion-{created_at_epoch_seconds}");
    let handoff_manifest = DnsDefaultRuntimeExpandedHandoffManifest {
        manifest_id,
        action: "defaultDnsRuntimeExpandedControlPlaneCompletion".into(),
        closeout_status: closeout.status,
        history_status: closeout.history.status,
        active_execution_event_id,
        active_state,
        history_record_count: closeout.history.record_count,
        stable_streak: closeout.history.stable_streak,
        required_stable_records: closeout.history.required_stable_records,
        observation_closed: closeout.observation_closed,
        handoff_ready: closeout.handoff_ready,
        rollback_recommended: closeout.rollback_recommended,
        next_control_plane_step: closeout.next_control_plane_step.clone(),
        phase8_allowed: false,
        promotion_allowed: false,
        auto_rollout: false,
        auto_rollback: false,
        mutates_runtime: false,
        reload_mihomo: false,
        created_at_epoch_seconds,
    };
    let mut blockers = closeout.blockers.clone();
    if closeout.status == DnsDefaultRuntimeExpandedLifecycleCloseoutStatus::Blocked {
        blockers.push("expanded control-plane completion requires non-blocked lifecycle closeout".into());
    }
    if !handoff_manifest_persisted {
        blockers.append(&mut persist_errors);
    }
    let status = if !blockers.is_empty() {
        DnsDefaultRuntimeExpandedControlPlaneCompletionStatus::Blocked
    } else if closeout.rollback_recommended {
        DnsDefaultRuntimeExpandedControlPlaneCompletionStatus::RollbackRecommended
    } else if closeout.handoff_ready {
        DnsDefaultRuntimeExpandedControlPlaneCompletionStatus::Complete
    } else {
        DnsDefaultRuntimeExpandedControlPlaneCompletionStatus::Watching
    };
    let dns_control_plane_complete = status == DnsDefaultRuntimeExpandedControlPlaneCompletionStatus::Complete;
    let mut warnings = closeout.warnings.clone();
    if dns_control_plane_complete {
        warnings.push("DNS expanded control-plane completion is not permission to enter TUN/protocol runtime".into());
    }
    let facts = vec![
        "expanded control-plane completion consumes lifecycle closeout".into(),
        "expanded control-plane completion persists a handoff manifest".into(),
        "expanded control-plane completion keeps phase8Allowed=false".into(),
        "expanded control-plane completion does not mutate runtime or reload Mihomo".into(),
    ];

    DnsDefaultRuntimeExpandedControlPlaneCompletionReport {
        status,
        reason: default_runtime_expanded_control_plane_completion_reason(status, &blockers),
        observation_closed: closeout.observation_closed,
        handoff_ready: closeout.handoff_ready,
        rollback_recommended: closeout.rollback_recommended,
        next_control_plane_step: closeout.next_control_plane_step.clone(),
        closeout,
        handoff_manifest,
        handoff_manifest_path,
        handoff_manifest_persisted,
        dns_control_plane_complete,
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

async fn persist_dns_default_runtime_expanded_control_plane_completion_report(
    closeout: DnsDefaultRuntimeExpandedLifecycleCloseoutReport,
) -> DnsDefaultRuntimeExpandedControlPlaneCompletionReport {
    let created_at_epoch_seconds = default_runtime_epoch_seconds();
    let manifest_id = format!("dns-default-runtime-expanded-control-plane-completion-{created_at_epoch_seconds}");
    let mut persist_errors = Vec::new();
    let path = match default_runtime_expanded_completion_manifest_path(&manifest_id) {
        Ok(path) => path,
        Err(error) => {
            return build_dns_default_runtime_expanded_control_plane_completion_report(
                closeout,
                false,
                None,
                vec![format!("failed to resolve expanded completion manifest path: {error}")],
                created_at_epoch_seconds,
            );
        }
    };
    if let Some(parent) = path.parent() {
        if let Err(error) = fs::create_dir_all(parent).await {
            persist_errors.push(format!(
                "failed to create expanded completion manifest directory: {error}"
            ));
        }
    }
    let report = build_dns_default_runtime_expanded_control_plane_completion_report(
        closeout,
        persist_errors.is_empty(),
        Some(path.to_string_lossy().to_string()),
        persist_errors,
        created_at_epoch_seconds,
    );
    let persisted = persist_default_runtime_guard_yaml(&path, &report.handoff_manifest, &mut Vec::new()).await;
    if persisted {
        report
    } else {
        build_dns_default_runtime_expanded_control_plane_completion_report(
            report.closeout,
            false,
            Some(path.to_string_lossy().to_string()),
            vec!["failed to persist expanded control-plane handoff manifest".into()],
            created_at_epoch_seconds,
        )
    }
}

fn default_runtime_expanded_completion_manifest_path(manifest_id: &str) -> Result<std::path::PathBuf> {
    Ok(default_runtime_state_dir()?
        .join("expanded-completion")
        .join(safe_dns_runtime_guard_segment(manifest_id))
        .join("handoff.yaml"))
}

fn default_runtime_expanded_control_plane_completion_reason(
    status: DnsDefaultRuntimeExpandedControlPlaneCompletionStatus,
    blockers: &[String],
) -> String {
    match status {
        DnsDefaultRuntimeExpandedControlPlaneCompletionStatus::Complete => {
            "expanded default DNS runtime control-plane completion is ready for handoff".into()
        }
        DnsDefaultRuntimeExpandedControlPlaneCompletionStatus::Watching => {
            "expanded default DNS runtime control-plane completion is still watching".into()
        }
        DnsDefaultRuntimeExpandedControlPlaneCompletionStatus::RollbackRecommended => {
            "expanded default DNS runtime control-plane completion recommends explicit rollback".into()
        }
        DnsDefaultRuntimeExpandedControlPlaneCompletionStatus::Blocked => blockers
            .first()
            .cloned()
            .unwrap_or_else(|| "expanded default DNS runtime control-plane completion is blocked".into()),
    }
}
