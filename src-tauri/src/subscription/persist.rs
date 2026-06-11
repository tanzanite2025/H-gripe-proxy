use crate::{
    subscription::model::{
        SubscriptionArtifactRecord, SubscriptionAttemptRecord, SubscriptionSourceState, SubscriptionStateDocument,
        UpdateFinalStatus,
    },
    utils::{dirs, help},
};
use anyhow::Result;
use chrono::Local;
use serde::Serialize;
use sha2::{Digest, Sha256};
use smartstring::alias::String;
use tokio::fs;

#[derive(Debug, Clone, Serialize)]
struct PersistedArtifactMetadata<'a> {
    source_id: &'a str,
    artifact: &'a SubscriptionArtifactRecord,
}

pub fn build_artifact_record(
    raw_body: &str,
    fetched_at: i64,
    content_type: Option<String>,
) -> SubscriptionArtifactRecord {
    let content_hash: String = hex::encode(Sha256::digest(raw_body.as_bytes())).into();
    let suffix_len = content_hash.len().min(12);
    let version = format!("{fetched_at}-{}", &content_hash[..suffix_len]).into();

    SubscriptionArtifactRecord {
        version,
        content_hash,
        fetched_at,
        content_length: raw_body.len(),
        content_type,
    }
}

async fn load_state_document() -> Result<SubscriptionStateDocument> {
    let path = dirs::subscription_state_path()?;
    if !tokio::fs::try_exists(&path).await.unwrap_or(false) {
        return Ok(SubscriptionStateDocument::default());
    }

    help::read_yaml(&path).await
}

async fn save_state_document(state: &SubscriptionStateDocument) -> Result<()> {
    let subscriptions_dir = dirs::subscriptions_dir()?;
    fs::create_dir_all(&subscriptions_dir).await?;

    let path = dirs::subscription_state_path()?;
    help::save_yaml(&path, state, Some("# Subscription State for Clash Verge")).await
}

pub async fn persist_artifact(
    source_id: &str,
    artifact: &SubscriptionArtifactRecord,
    raw_body: &str,
) -> Result<()> {
    let dir = dirs::subscription_artifact_version_dir(source_id, artifact.version.as_str())?;
    fs::create_dir_all(&dir).await?;

    fs::write(dir.join("raw.body"), raw_body.as_bytes()).await?;

    let metadata = PersistedArtifactMetadata { source_id, artifact };
    let metadata_path = dir.join("metadata.yaml");
    help::save_yaml(&metadata_path, &metadata, Some("# Subscription Artifact Metadata")).await
}

pub async fn persist_attempt_result(
    source_id: &String,
    artifact: Option<&SubscriptionArtifactRecord>,
    attempt: &SubscriptionAttemptRecord,
) -> Result<()> {
    let mut state = load_state_document().await?;

    let source_state = match state
        .sources
        .iter_mut()
        .find(|source_state| source_state.source_id == *source_id)
    {
        Some(existing) => existing,
        None => {
            state.sources.push(SubscriptionSourceState {
                source_id: source_id.clone(),
                active_artifact_version: None,
                latest_artifact: None,
                latest_attempt: None,
                latest_success: None,
            });
            state
                .sources
                .last_mut()
                .expect("just inserted subscription source state")
        }
    };

    if let Some(artifact) = artifact {
        source_state.latest_artifact = Some(artifact.clone());
        if attempt.runtime_activated {
            source_state.active_artifact_version = Some(artifact.version.clone());
        }
    }

    source_state.latest_attempt = Some(attempt.clone());
    if attempt.final_status == UpdateFinalStatus::Succeeded {
        source_state.latest_success = Some(attempt.clone());
    }

    save_state_document(&state).await
}

pub fn build_finished_attempt_record(
    attempt: &crate::subscription::model::SubscriptionUpdateAttempt,
    final_status: UpdateFinalStatus,
    stage: crate::subscription::model::UpdateStage,
    transport: Option<crate::subscription::transport::TransportKind>,
    artifact_version: Option<String>,
    error: Option<String>,
    runtime_activated: bool,
    active_artifact_unchanged: bool,
) -> SubscriptionAttemptRecord {
    SubscriptionAttemptRecord {
        attempt_id: attempt.attempt_id.clone(),
        trigger: attempt.trigger,
        started_at: attempt.started_at,
        finished_at: Local::now().timestamp_millis(),
        final_status,
        stage,
        transport,
        artifact_version,
        error,
        runtime_activated,
        active_artifact_unchanged,
    }
}
