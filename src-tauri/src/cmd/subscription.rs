use super::{CmdResult, StringifyErr as _};
use crate::subscription::{
    artifact::SubscriptionArtifactDiagnostics,
    model::{SubscriptionSourceState, SubscriptionStateDocument},
    persist::{
        cleanup_subscription_artifacts, list_subscription_artifact_metadata,
        list_subscription_artifact_summaries as list_subscription_artifact_summary_records,
        read_subscription_artifact_content, read_subscription_artifact_diagnostics,
        read_subscription_artifact_metadata, read_subscription_source_state,
        read_subscription_state_document,
        SubscriptionArtifactCleanupResult, SubscriptionArtifactContent,
        SubscriptionArtifactContentKind, SubscriptionArtifactMetadata,
        SubscriptionArtifactSummary,
    },
};

#[tauri::command]
pub async fn get_subscription_state() -> CmdResult<SubscriptionStateDocument> {
    read_subscription_state_document().await.stringify_err()
}

#[tauri::command]
pub async fn get_subscription_source_state(
    source_id: String,
) -> CmdResult<Option<SubscriptionSourceState>> {
    read_subscription_source_state(source_id.as_str()).await.stringify_err()
}

#[tauri::command]
pub async fn get_subscription_artifact_diagnostics(
    source_id: String,
    version: String,
) -> CmdResult<Option<SubscriptionArtifactDiagnostics>> {
    read_subscription_artifact_diagnostics(source_id.as_str(), version.as_str())
        .await
        .stringify_err()
}

#[tauri::command]
pub async fn get_subscription_artifact_metadata(
    source_id: String,
    version: String,
) -> CmdResult<Option<SubscriptionArtifactMetadata>> {
    read_subscription_artifact_metadata(source_id.as_str(), version.as_str())
        .await
        .stringify_err()
}

#[tauri::command]
pub async fn get_subscription_artifact_content(
    source_id: String,
    version: String,
    content_kind: SubscriptionArtifactContentKind,
) -> CmdResult<Option<SubscriptionArtifactContent>> {
    read_subscription_artifact_content(
        source_id.as_str(),
        version.as_str(),
        content_kind,
    )
    .await
    .stringify_err()
}

#[tauri::command]
pub async fn list_subscription_artifacts(
    source_id: String,
) -> CmdResult<Vec<SubscriptionArtifactMetadata>> {
    list_subscription_artifact_metadata(source_id.as_str())
        .await
        .stringify_err()
}

#[tauri::command]
pub async fn list_subscription_artifact_summaries(
    source_id: String,
) -> CmdResult<Vec<SubscriptionArtifactSummary>> {
    list_subscription_artifact_summary_records(source_id.as_str())
        .await
        .stringify_err()
}

#[tauri::command]
pub async fn cleanup_subscription_artifacts_by_retention(
    source_id: String,
    retain_count: Option<usize>,
) -> CmdResult<SubscriptionArtifactCleanupResult> {
    cleanup_subscription_artifacts(source_id.as_str(), retain_count)
        .await
        .stringify_err()
}
