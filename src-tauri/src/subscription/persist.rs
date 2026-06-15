use crate::{
    subscription::{
        artifact::SubscriptionArtifactCandidate,
        model::{
            SubscriptionArtifactRecord, SubscriptionAttemptRecord, SubscriptionSourceState,
            SubscriptionStageRecord, SubscriptionStateDocument, UpdateFinalStatus,
        },
    },
    utils::{dirs, help},
};
use anyhow::Result;
use chrono::Local;
use serde::Serialize;
use smartstring::alias::String;
use tokio::fs;

#[derive(Debug, Clone, Serialize)]
struct PersistedArtifactMetadata<'a> {
    source_id: &'a str,
    artifact: &'a SubscriptionArtifactRecord,
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
    help::save_yaml(&path, state, Some("# Subscription State for Clash Verge Optimized")).await
}

pub async fn persist_artifact_candidate(
    source_id: &str,
    candidate: &SubscriptionArtifactCandidate,
) -> Result<()> {
    let dir = dirs::subscription_artifact_version_dir(source_id, candidate.record.version.as_str())?;
    fs::create_dir_all(&dir).await?;

    fs::write(dir.join("raw.body"), candidate.raw_body.as_bytes()).await?;
    fs::write(dir.join("normalized.yaml"), candidate.normalized_yaml.as_bytes())
        .await?;

    let diagnostics_path = dir.join("diagnostics.yaml");
    help::save_yaml(
        &diagnostics_path,
        &candidate.diagnostics,
        Some("# Subscription Artifact Diagnostics"),
    )
    .await?;

    let metadata = PersistedArtifactMetadata {
        source_id,
        artifact: &candidate.record,
    };
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
    let finished_at = Local::now().timestamp_millis();
    let mut stage_history = attempt.stage_history.clone();
    if stage_history
        .last()
        .is_none_or(|record| record.stage != stage || record.transport != transport)
    {
        stage_history.push(SubscriptionStageRecord {
            stage,
            changed_at: finished_at,
            transport,
        });
    }

    SubscriptionAttemptRecord {
        attempt_id: attempt.attempt_id.clone(),
        trigger: attempt.trigger,
        started_at: attempt.started_at,
        finished_at,
        final_status,
        stage,
        transport,
        artifact_version,
        error,
        runtime_activated,
        active_artifact_unchanged,
        stage_history,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::subscription::{
        model::{SubscriptionUpdateAttempt, UpdateStage, UpdateTrigger},
        transport::TransportKind,
    };

    #[test]
    fn finished_attempt_record_copies_stage_history() {
        let mut attempt = SubscriptionUpdateAttempt::new("source-a", UpdateTrigger::Manual);
        attempt.record_stage_changed(UpdateStage::FetchPayload, Some(TransportKind::Direct));

        let record = build_finished_attempt_record(
            &attempt,
            UpdateFinalStatus::Succeeded,
            UpdateStage::EmitFinalResult,
            Some(TransportKind::Direct),
            Some("artifact-a".into()),
            None,
            true,
            false,
        );

        assert_eq!(record.stage_history.len(), 2);
        assert_eq!(record.stage_history[0].stage, UpdateStage::FetchPayload);
        assert_eq!(record.stage_history[1].stage, UpdateStage::EmitFinalResult);
    }
}
