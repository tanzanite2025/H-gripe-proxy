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
