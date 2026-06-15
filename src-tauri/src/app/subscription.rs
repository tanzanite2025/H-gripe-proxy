use crate::{
    config::{Config, PrfItem, PrfOption, profiles::profiles_draft_update_item_safe},
    core::{handle, validate::ValidationOutcome},
    subscription::{
        executor::{SubscriptionUpdateExecutor, SubscriptionUpdateFailure},
        model::{SubscriptionArtifactRecord, SubscriptionUpdateAttempt, UpdateFinalStatus, UpdateStage, UpdateTrigger},
        persist::{
            SubscriptionArtifactPublishResult, build_finished_attempt_record, persist_attempt_result,
            publish_subscription_artifact, restore_subscription_active_artifact,
        },
        runtime_candidate::{
            SubscriptionRuntimeProfileProjection, activate_subscription_active_artifact_runtime,
            validate_subscription_artifact_runtime_candidate,
        },
        transport::TransportKind,
    },
    utils::help::mask_url,
};
use anyhow::{Result, anyhow, bail};
use clash_verge_logging::{Type, logging};
use smartstring::alias::String;
use tokio::fs;

struct ProfileUpdateSnapshot {
    item: PrfItem,
    file_data: Option<String>,
}

fn record_and_notify_subscription_stage(
    attempt: &mut SubscriptionUpdateAttempt,
    stage: UpdateStage,
    transport: Option<TransportKind>,
) {
    attempt.record_stage_changed(stage, transport);
    handle::Handle::notify_subscription_stage_changed(attempt, stage, transport);
}

struct ProfileUpdateExecution {
    attempt: SubscriptionUpdateAttempt,
    is_current: bool,
    transport: TransportKind,
    artifact: SubscriptionArtifactRecord,
    publish: SubscriptionArtifactPublishResult,
    runtime_projection: SubscriptionRuntimeProfileProjection,
}

type ProfileUpdateFailure = SubscriptionUpdateFailure;

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

async fn restore_published_subscription_artifact(source_id: &String, publish: &SubscriptionArtifactPublishResult) {
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

async fn apply_legacy_profile_compatibility_projection(
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

async fn should_update_profile(uid: &String, ignore_auto_update: bool) -> Result<Option<(String, Option<PrfOption>)>> {
    let profiles = Config::profiles().await;
    let profiles = profiles.latest_arc();
    let item = profiles.get_item(uid)?;
    let is_remote = item.itype.as_ref().is_some_and(|s| s == "remote");

    if !is_remote {
        logging!(
            info,
            Type::Config,
            "[Subscription Update] {uid} is not a remote subscription, skip update"
        );
        Ok(None)
    } else if item.url.is_none() {
        logging!(
            warn,
            Type::Config,
            "Warning: [Subscription Update] {uid} is missing a URL and cannot be updated"
        );
        bail!("failed to get the profile item url");
    } else if !ignore_auto_update && !item.option.as_ref().and_then(|o| o.allow_auto_update).unwrap_or(true) {
        logging!(
            info,
            Type::Config,
            "[Subscription Update] {} has auto update disabled, skip update",
            uid
        );
        Ok(None)
    } else {
        logging!(
            info,
            Type::Config,
            "[Subscription Update] {} target URL: {}",
            uid,
            mask_url(item.url.as_ref().ok_or_else(|| anyhow!("Profile URL is None"))?)
        );
        Ok(Some((
            item.url.clone().ok_or_else(|| anyhow!("Profile URL is None"))?,
            item.option.clone(),
        )))
    }
}

async fn snapshot_profile_update(uid: &String) -> Result<Option<ProfileUpdateSnapshot>> {
    let profiles = Config::profiles().await;
    let profiles_arc = profiles.latest_arc();

    if !profiles_arc.is_current_profile_index(uid) {
        return Ok(None);
    }

    let item = profiles_arc.get_item(uid)?.clone();
    let file_data = match item.file.as_ref() {
        Some(file) => {
            let path = crate::config::profiles::resolve_profile_file_path(file.as_str())?;
            match fs::try_exists(&path).await {
                Ok(true) => Some(fs::read_to_string(path).await?.into()),
                Ok(false) => {
                    logging!(
                        warn,
                        Type::Config,
                        "Warning: [Subscription Update] current profile file is missing before update, will recreate it: {}",
                        file
                    );
                    None
                }
                Err(err) => return Err(err.into()),
            }
        }
        None => None,
    };

    Ok(Some(ProfileUpdateSnapshot { item, file_data }))
}

async fn restore_profile_update_snapshot(snapshot: &ProfileUpdateSnapshot) -> Result<()> {
    if let (Some(file), Some(file_data)) = (snapshot.item.file.as_ref(), snapshot.file_data.as_ref()) {
        let path = crate::config::profiles::resolve_profile_file_path(file.as_str())?;
        fs::write(path, file_data.as_bytes()).await?;
    }

    let uid = snapshot
        .item
        .uid
        .clone()
        .ok_or_else(|| anyhow!("profile update snapshot is missing uid"))?;
    let restored_item = snapshot.item.clone();

    Config::profiles()
        .await
        .with_data_modify(|mut profiles| async move {
            let items = profiles.items.get_or_insert_with(Vec::new);
            let Some(item) = items.iter_mut().find(|item| item.uid.as_ref() == Some(&uid)) else {
                bail!("failed to restore profile update snapshot for uid:{uid}");
            };
            *item = restored_item;
            profiles.save_file().await?;
            Ok((profiles, ()))
        })
        .await
}

async fn perform_profile_update(
    uid: &String,
    url: &String,
    opt: Option<&PrfOption>,
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
    let mut update = SubscriptionUpdateExecutor::new(uid.clone(), url.clone(), opt.cloned(), option.cloned(), trigger)
        .execute(
            handle::Handle::notify_subscription_attempt_started,
            |attempt, stage, transport| {
                handle::Handle::notify_subscription_stage_changed(attempt, stage, transport);
            },
        )
        .await?;
    let mut legacy_profile_projection = update.legacy_profile_projection;
    let runtime_projection = SubscriptionRuntimeProfileProjection::from_profile_item(uid, &legacy_profile_projection);

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

        match validate_subscription_artifact_runtime_candidate(uid, &update.artifact.version, &runtime_projection).await
        {
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

    if let Err(err) = apply_legacy_profile_compatibility_projection(uid, &mut legacy_profile_projection, &publish).await
    {
        return Err(SubscriptionUpdateFailure {
            attempt: update.attempt,
            stage: UpdateStage::PublishArtifact,
            transport: Some(update.transport),
            artifact: Some(update.artifact),
            error: err.to_string().into(),
        });
    }

    Ok(ProfileUpdateExecution {
        attempt: update.attempt,
        is_current,
        transport: update.transport,
        artifact: update.artifact,
        publish,
        runtime_projection,
    })
}

pub async fn update_profile(
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

    let Some((url, opt)) = should_update_profile(uid, ignore_auto_update).await? else {
        return Ok(());
    };

    let rollback_snapshot = snapshot_profile_update(uid).await?;

    let mut update_execution = match perform_profile_update(uid, &url, opt.as_ref(), option, is_manual_trigger).await {
        Ok(execution) => execution,
        Err(failure) => {
            logging!(
                error,
                Type::Config,
                "[Subscription Update] update failed at {:?}: {}",
                failure.stage,
                failure.error
            );

            if let Some(rollback_snapshot) = &rollback_snapshot {
                restore_profile_update_snapshot(rollback_snapshot).await?;
            }

            let artifact_version = failure.artifact.as_ref().map(|artifact| artifact.version.clone());
            let error_message = failure.error.clone();

            handle::Handle::notify_subscription_update_failed(
                &failure.attempt,
                failure.stage,
                failure.transport,
                artifact_version,
                error_message.clone(),
                true,
            );
            persist_finished_subscription_attempt(
                uid,
                &failure.attempt,
                UpdateFinalStatus::Failed,
                failure.stage,
                failure.transport,
                failure.artifact.as_ref(),
                Some(error_message.clone()),
                false,
                true,
            )
            .await;

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

        match activate_subscription_active_artifact_runtime(
            uid,
            &update_execution.runtime_projection,
            is_manual_trigger,
        )
        .await
        {
            Ok(outcome) if outcome.is_valid() => {
                logging!(info, Type::Config, "[Subscription Update] update succeeded");
                handle::Handle::notify_subscription_update_succeeded(
                    &update_execution.attempt,
                    update_execution.transport,
                    UpdateStage::EmitFinalResult,
                    update_execution.artifact.version.clone(),
                    true,
                    false,
                );
                persist_finished_subscription_attempt(
                    uid,
                    &update_execution.attempt,
                    UpdateFinalStatus::Succeeded,
                    UpdateStage::EmitFinalResult,
                    Some(update_execution.transport),
                    Some(&update_execution.artifact),
                    None,
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
                handle::Handle::notify_subscription_update_succeeded(
                    &update_execution.attempt,
                    update_execution.transport,
                    UpdateStage::EmitFinalResult,
                    update_execution.artifact.version.clone(),
                    false,
                    false,
                );
                persist_finished_subscription_attempt(
                    uid,
                    &update_execution.attempt,
                    UpdateFinalStatus::Succeeded,
                    UpdateStage::EmitFinalResult,
                    Some(update_execution.transport),
                    Some(&update_execution.artifact),
                    None,
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
                if let Some(rollback_snapshot) = &rollback_snapshot {
                    restore_profile_update_snapshot(rollback_snapshot).await?;
                }
                restore_published_subscription_artifact(uid, &update_execution.publish).await;
                handle::Handle::notify_subscription_update_failed(
                    &update_execution.attempt,
                    UpdateStage::ActivateRuntime,
                    Some(update_execution.transport),
                    Some(update_execution.artifact.version.clone()),
                    message.clone(),
                    true,
                );
                persist_finished_subscription_attempt(
                    uid,
                    &update_execution.attempt,
                    UpdateFinalStatus::Failed,
                    UpdateStage::ActivateRuntime,
                    Some(update_execution.transport),
                    Some(&update_execution.artifact),
                    Some(message.clone().into()),
                    false,
                    true,
                )
                .await;
                bail!("failed to apply updated profile: {message}");
            }
            Err(err) => {
                let message = err.to_string();
                logging!(
                    error,
                    Type::Config,
                    "[Subscription Update] runtime activation failed: {}",
                    message
                );
                if let Some(rollback_snapshot) = &rollback_snapshot {
                    restore_profile_update_snapshot(rollback_snapshot).await?;
                }
                restore_published_subscription_artifact(uid, &update_execution.publish).await;
                handle::Handle::notify_subscription_update_failed(
                    &update_execution.attempt,
                    UpdateStage::ActivateRuntime,
                    Some(update_execution.transport),
                    Some(update_execution.artifact.version.clone()),
                    message.clone(),
                    true,
                );
                persist_finished_subscription_attempt(
                    uid,
                    &update_execution.attempt,
                    UpdateFinalStatus::Failed,
                    UpdateStage::ActivateRuntime,
                    Some(update_execution.transport),
                    Some(&update_execution.artifact),
                    Some(message.clone().into()),
                    false,
                    true,
                )
                .await;
                logging!(error, Type::Config, "{err}");
                bail!("failed to apply updated profile: {message}");
            }
        }
    } else {
        handle::Handle::notify_subscription_update_succeeded(
            &update_execution.attempt,
            update_execution.transport,
            UpdateStage::EmitFinalResult,
            update_execution.artifact.version.clone(),
            false,
            false,
        );
        persist_finished_subscription_attempt(
            uid,
            &update_execution.attempt,
            UpdateFinalStatus::Succeeded,
            UpdateStage::EmitFinalResult,
            Some(update_execution.transport),
            Some(&update_execution.artifact),
            None,
            false,
            false,
        )
        .await;
    }

    Ok(())
}
