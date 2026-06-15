use crate::{
    config::{PrfItem, profiles::profiles_draft_update_item_safe},
    core::handle,
    subscription::{
        executor::SubscriptionUpdateFailure,
        model::{SubscriptionArtifactRecord, SubscriptionUpdateAttempt, UpdateFinalStatus, UpdateStage},
        persist::{
            SubscriptionArtifactPublishResult, build_finished_attempt_record, persist_attempt_result,
            restore_subscription_active_artifact,
        },
        transport::TransportKind,
    },
};
use anyhow::{Result, bail};
use clash_verge_logging::{Type, logging};
use smartstring::alias::String;

pub fn notify_subscription_attempt_started(attempt: &SubscriptionUpdateAttempt) {
    handle::Handle::notify_subscription_attempt_started(attempt);
}

pub fn notify_subscription_stage_changed(
    attempt: &SubscriptionUpdateAttempt,
    stage: UpdateStage,
    transport: Option<TransportKind>,
) {
    handle::Handle::notify_subscription_stage_changed(attempt, stage, transport);
}

pub fn record_and_notify_subscription_stage(
    attempt: &mut SubscriptionUpdateAttempt,
    stage: UpdateStage,
    transport: Option<TransportKind>,
) {
    attempt.record_stage_changed(stage, transport);
    handle::Handle::notify_subscription_stage_changed(attempt, stage, transport);
}

pub async fn restore_published_subscription_artifact(source_id: &String, publish: &SubscriptionArtifactPublishResult) {
    if let Err(err) =
        restore_subscription_active_artifact(source_id, publish.previous_active_artifact_version.clone()).await
    {
        logging!(
            warn,
            Type::Config,
            "Warning: [Subscription Update] failed to restore active artifact for {} after publish rollback: {}",
            source_id,
            err
        );
    }
}

pub async fn apply_legacy_profile_compatibility_projection(
    uid: &String,
    legacy_profile_projection: &mut PrfItem,
    publish: &SubscriptionArtifactPublishResult,
) -> Result<()> {
    if let Err(err) = profiles_draft_update_item_safe(uid, legacy_profile_projection).await {
        restore_published_subscription_artifact(uid, publish).await;
        bail!("failed to commit legacy profile compatibility projection: {err}");
    }

    Ok(())
}

pub async fn notify_and_persist_subscription_update_failure(
    source_id: &String,
    failure: &SubscriptionUpdateFailure,
    active_artifact_unchanged: bool,
) {
    let artifact_version = failure.artifact.as_ref().map(|artifact| artifact.version.clone());
    let error_message = failure.error.clone();

    handle::Handle::notify_subscription_update_failed(
        &failure.attempt,
        failure.stage,
        failure.transport,
        artifact_version,
        error_message.clone(),
        active_artifact_unchanged,
    );

    persist_finished_subscription_attempt(
        source_id,
        &failure.attempt,
        UpdateFinalStatus::Failed,
        failure.stage,
        failure.transport,
        failure.artifact.as_ref(),
        Some(error_message),
        false,
        active_artifact_unchanged,
    )
    .await;
}

pub async fn notify_and_persist_subscription_update_success(
    source_id: &String,
    attempt: &SubscriptionUpdateAttempt,
    transport: TransportKind,
    artifact: &SubscriptionArtifactRecord,
    runtime_activated: bool,
    active_artifact_unchanged: bool,
) {
    handle::Handle::notify_subscription_update_succeeded(
        attempt,
        transport,
        UpdateStage::EmitFinalResult,
        artifact.version.clone(),
        runtime_activated,
        active_artifact_unchanged,
    );

    persist_finished_subscription_attempt(
        source_id,
        attempt,
        UpdateFinalStatus::Succeeded,
        UpdateStage::EmitFinalResult,
        Some(transport),
        Some(artifact),
        None,
        runtime_activated,
        active_artifact_unchanged,
    )
    .await;
}

async fn persist_finished_subscription_attempt(
    source_id: &String,
    attempt: &SubscriptionUpdateAttempt,
    final_status: UpdateFinalStatus,
    stage: UpdateStage,
    transport: Option<TransportKind>,
    artifact: Option<&SubscriptionArtifactRecord>,
    error: Option<String>,
    runtime_activated: bool,
    active_artifact_unchanged: bool,
) {
    let finished_attempt = build_finished_attempt_record(
        attempt,
        final_status,
        stage,
        transport,
        artifact.map(|artifact| artifact.version.clone()),
        error,
        runtime_activated,
        active_artifact_unchanged,
    );

    if let Err(err) = persist_attempt_result(source_id, artifact, &finished_attempt).await {
        logging!(
            warn,
            Type::Config,
            "Warning: [Subscription Update] failed to persist subscription attempt state for {}: {}",
            source_id,
            err
        );
    }
}
