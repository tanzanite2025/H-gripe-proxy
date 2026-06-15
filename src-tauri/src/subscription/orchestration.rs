use crate::{
    config::{Config, PrfOption},
    core::{handle, validate::ValidationOutcome},
    subscription::{
        executor::{SubscriptionUpdateExecutor, SubscriptionUpdateFailure},
        model::{
            SubscriptionArtifactRecord, SubscriptionSourceConfig, SubscriptionUpdateAttempt, UpdateFinalStatus,
            UpdateStage, UpdateTrigger,
        },
        persist::{
            SubscriptionArtifactPublishResult, build_finished_attempt_record, persist_attempt_result,
            publish_subscription_artifact, read_subscription_source_config, restore_subscription_active_artifact,
        },
        runtime_candidate::{
            activate_subscription_active_artifact_runtime, validate_subscription_artifact_runtime_candidate,
        },
        transport::TransportKind,
    },
    utils::help::mask_url,
};
use anyhow::{Result, bail};
use clash_verge_logging::{Type, logging};
use smartstring::alias::String;

struct ProfileUpdateExecution {
    attempt: SubscriptionUpdateAttempt,
    is_current: bool,
    transport: TransportKind,
    artifact: SubscriptionArtifactRecord,
    publish: SubscriptionArtifactPublishResult,
    source_config: SubscriptionSourceConfig,
}

type ProfileUpdateFailure = SubscriptionUpdateFailure;

pub async fn update_subscription_profile(
    uid: &String,
    option: Option<&PrfOption>,
    auto_refresh: bool,
    ignore_auto_update: bool,
    is_manual_trigger: bool,
) -> Result<()> {
    logging!(
        info,
        Type::Config,
        "[Subscription Update] start updating subscription {}",
        uid
    );

    let Some(source_config) = resolve_subscription_source_config(uid, ignore_auto_update, is_manual_trigger).await?
    else {
        return Ok(());
    };

    let mut update_execution = match perform_profile_update(uid, &source_config, option, is_manual_trigger).await {
        Ok(execution) => execution,
        Err(failure) => {
            logging!(
                error,
                Type::Config,
                "[Subscription Update] update failed at {:?}: {}",
                failure.stage,
                failure.error
            );

            let error_message = failure.error.clone();

            notify_and_persist_subscription_update_failure(uid, &failure, true).await;

            bail!("failed to update profile: {error_message}");
        }
    };

    let should_refresh = update_execution.is_current && auto_refresh;

    if should_refresh {
        record_and_notify_subscription_stage(&mut update_execution.attempt, UpdateStage::ActivateRuntime, None);
        logging!(
            info,
            Type::Config,
            "[Subscription Update] applying updated profile to runtime"
        );

        match activate_subscription_active_artifact_runtime(uid, &update_execution.source_config, is_manual_trigger)
            .await
        {
            Ok(outcome) if outcome.is_valid() => {
                logging!(info, Type::Config, "[Subscription Update] update succeeded");
                notify_and_persist_subscription_update_success(
                    uid,
                    &update_execution.attempt,
                    update_execution.transport,
                    &update_execution.artifact,
                    true,
                    false,
                )
                .await;
                handle::Handle::refresh_clash();
            }
            Ok(outcome @ (ValidationOutcome::Skipped { .. } | ValidationOutcome::Busy)) if !is_manual_trigger => {
                logging!(
                    info,
                    Type::Config,
                    "[Subscription Update] runtime refresh skipped after successful fetch: {}",
                    outcome
                );
                notify_and_persist_subscription_update_success(
                    uid,
                    &update_execution.attempt,
                    update_execution.transport,
                    &update_execution.artifact,
                    false,
                    false,
                )
                .await;
            }
            Ok(outcome) => {
                let message = outcome.to_string();
                logging!(
                    error,
                    Type::Config,
                    "[Subscription Update] runtime activation failed: {}",
                    message
                );
                restore_published_subscription_artifact(uid, &update_execution.publish).await;
                notify_and_persist_subscription_update_failure(
                    uid,
                    &SubscriptionUpdateFailure {
                        attempt: update_execution.attempt.clone(),
                        stage: UpdateStage::ActivateRuntime,
                        transport: Some(update_execution.transport),
                        artifact: Some(update_execution.artifact.clone()),
                        error: message.clone().into(),
                    },
                    true,
                )
                .await;
                bail!("failed to activate subscription runtime: {message}");
            }
            Err(err) => {
                let message = err.to_string();
                logging!(
                    error,
                    Type::Config,
                    "[Subscription Update] runtime activation failed: {}",
                    message
                );
                restore_published_subscription_artifact(uid, &update_execution.publish).await;
                notify_and_persist_subscription_update_failure(
                    uid,
                    &SubscriptionUpdateFailure {
                        attempt: update_execution.attempt.clone(),
                        stage: UpdateStage::ActivateRuntime,
                        transport: Some(update_execution.transport),
                        artifact: Some(update_execution.artifact.clone()),
                        error: message.clone().into(),
                    },
                    true,
                )
                .await;
                logging!(error, Type::Config, "{err}");
                bail!("failed to activate subscription runtime: {message}");
            }
        }
    } else {
        notify_and_persist_subscription_update_success(
            uid,
            &update_execution.attempt,
            update_execution.transport,
            &update_execution.artifact,
            false,
            false,
        )
        .await;
    }

    Ok(())
}

async fn resolve_subscription_source_config(
    uid: &String,
    ignore_auto_update: bool,
    is_manual_trigger: bool,
) -> Result<Option<SubscriptionSourceConfig>> {
    let Some(source_config) = read_subscription_source_config(uid.as_str()).await? else {
        if is_manual_trigger {
            bail!("subscription source config is missing for uid:{uid}");
        }
        logging!(
            info,
            Type::Config,
            "[Subscription Update] {uid} has no subscription source config, skip update"
        );
        return Ok(None);
    };

    if !ignore_auto_update
        && !source_config
            .option
            .as_ref()
            .and_then(|o| o.allow_auto_update)
            .unwrap_or(true)
    {
        logging!(
            info,
            Type::Config,
            "[Subscription Update] {} has auto update disabled, skip update",
            uid
        );
        return Ok(None);
    };

    logging!(
        info,
        Type::Config,
        "[Subscription Update] {} target URL: {}",
        uid,
        mask_url(&source_config.url)
    );

    Ok(Some(source_config))
}

async fn perform_profile_update(
    uid: &String,
    source_config: &SubscriptionSourceConfig,
    option: Option<&PrfOption>,
    is_manual_trigger: bool,
) -> std::result::Result<ProfileUpdateExecution, ProfileUpdateFailure> {
    let is_current = {
        let profiles = Config::profiles().await;
        profiles.latest_arc().is_current_profile_index(uid)
    };
    let trigger = if is_manual_trigger {
        UpdateTrigger::Manual
    } else {
        UpdateTrigger::Automatic
    };
    let mut update = SubscriptionUpdateExecutor::new(
        uid.clone(),
        source_config.url.clone(),
        source_config.option.clone(),
        option.cloned(),
        trigger,
    )
    .execute(notify_subscription_attempt_started, |attempt, stage, transport| {
        notify_subscription_stage_changed(attempt, stage, transport);
    })
    .await?;

    if is_current {
        record_and_notify_subscription_stage(
            &mut update.attempt,
            UpdateStage::GenerateRuntimeConfigCandidate,
            Some(update.transport),
        );
        record_and_notify_subscription_stage(
            &mut update.attempt,
            UpdateStage::ValidateRuntimeCandidate,
            Some(update.transport),
        );

        match validate_subscription_artifact_runtime_candidate(uid, &update.artifact.version, source_config).await {
            Ok(outcome) if outcome.is_valid() => {}
            Ok(outcome) => {
                return Err(SubscriptionUpdateFailure {
                    attempt: update.attempt,
                    stage: UpdateStage::ValidateRuntimeCandidate,
                    transport: Some(update.transport),
                    artifact: Some(update.artifact),
                    error: outcome.to_string().into(),
                });
            }
            Err(err) => {
                return Err(SubscriptionUpdateFailure {
                    attempt: update.attempt,
                    stage: UpdateStage::ValidateRuntimeCandidate,
                    transport: Some(update.transport),
                    artifact: Some(update.artifact),
                    error: format!("failed to validate subscription runtime candidate: {err}").into(),
                });
            }
        }
    }

    record_and_notify_subscription_stage(
        &mut update.attempt,
        UpdateStage::PublishArtifact,
        Some(update.transport),
    );
    let publish = match publish_subscription_artifact(uid, &update.artifact).await {
        Ok(publish) => publish,
        Err(err) => {
            return Err(SubscriptionUpdateFailure {
                attempt: update.attempt,
                stage: UpdateStage::PublishArtifact,
                transport: Some(update.transport),
                artifact: Some(update.artifact),
                error: format!("failed to publish subscription artifact: {err}").into(),
            });
        }
    };

    Ok(ProfileUpdateExecution {
        attempt: update.attempt,
        is_current,
        transport: update.transport,
        artifact: update.artifact,
        publish,
        source_config: source_config.clone(),
    })
}

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
