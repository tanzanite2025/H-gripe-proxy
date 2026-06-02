use super::{CmdResult, StringifyErr as _};
use crate::core::identity_consistency::{
    IdentityConsistencyInput, IdentityConsistencyReport, IdentityConsistencySnapshot,
    append_identity_consistency_snapshot, build_identity_consistency_report,
};
use crate::core::{
    current_egress_identity::build_current_egress_identity,
    runtime_diagnostics::{build_dns_leak_test_result, build_dns_runtime_status},
};
use crate::utils::dirs;

const IDENTITY_CONSISTENCY_HISTORY_FILE: &str = "identity_consistency_history.json";
const IDENTITY_CONSISTENCY_HISTORY_LIMIT: usize = 24;

#[tauri::command]
pub async fn get_identity_consistency_report(
    app_handle: tauri::AppHandle,
) -> CmdResult<IdentityConsistencyReport> {
    let current_identity = build_current_egress_identity(Some(&app_handle))
        .await
        .stringify_err()?;
    let dns_runtime = build_dns_runtime_status().await.ok();
    let dns_leak = build_dns_leak_test_result().await.ok();
    let tls_fingerprint = crate::feat::tls_fingerprint_get_current();

    let report = build_identity_consistency_report(IdentityConsistencyInput {
        current_identity: &current_identity,
        dns_runtime: dns_runtime.as_ref(),
        dns_leak: dns_leak.as_ref(),
        tls_fingerprint: tls_fingerprint.as_ref(),
    });

    let _ = persist_identity_consistency_snapshot(report.clone()).await;

    Ok(report)
}

#[tauri::command]
pub async fn get_identity_consistency_history() -> CmdResult<Vec<IdentityConsistencySnapshot>> {
    read_identity_consistency_history().await.stringify_err()
}

async fn persist_identity_consistency_snapshot(report: IdentityConsistencyReport) -> anyhow::Result<()> {
    let history = read_identity_consistency_history().await.unwrap_or_default();
    let observed_at = chrono::Utc::now().to_rfc3339();
    let history = append_identity_consistency_snapshot(
        history,
        report,
        observed_at,
        IDENTITY_CONSISTENCY_HISTORY_LIMIT,
    );
    let path = identity_consistency_history_path()?;

    if let Some(parent) = path.parent() {
        tokio::fs::create_dir_all(parent).await?;
    }

    tokio::fs::write(path, serde_json::to_vec_pretty(&history)?).await?;
    Ok(())
}

async fn read_identity_consistency_history() -> anyhow::Result<Vec<IdentityConsistencySnapshot>> {
    let path = identity_consistency_history_path()?;

    if !path.exists() {
        return Ok(Vec::new());
    }

    let content = tokio::fs::read_to_string(path).await?;
    Ok(serde_json::from_str(&content).unwrap_or_default())
}

fn identity_consistency_history_path() -> anyhow::Result<std::path::PathBuf> {
    Ok(dirs::app_home_dir()?.join(IDENTITY_CONSISTENCY_HISTORY_FILE))
}
