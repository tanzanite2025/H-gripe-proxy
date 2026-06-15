use crate::{
    config::{Config, PrfItem, PrfOption, profiles::profiles_draft_update_item_safe},
    core::{CoreManager, handle, mihomo_runtime_guard, validate::ValidationOutcome},
    subscription::{
        control_plane::{
            fetch_subscription_update_via_control_plane, subscription_update_uses_dedicated_control_plane,
        },
        fetch::fetch_remote_profile,
        format::parse_clash_yaml_subscription,
        model::{SubscriptionArtifactRecord, SubscriptionUpdateAttempt, UpdateFinalStatus, UpdateStage, UpdateTrigger},
        persist::{build_artifact_record, build_finished_attempt_record, persist_artifact, persist_attempt_result},
        transport::{TransportKind, TransportPlan, apply_transport_to_option, transport_kind_from_option},
    },
    utils::help::{mask_err, mask_url},
};
use anyhow::{Result, anyhow, bail};
use clash_verge_logging::{Type, logging};
use smartstring::alias::String;
use tokio::fs;

struct ProfileUpdateSnapshot {
    item: PrfItem,
    file_data: Option<String>,
}

struct ProfileUpdateExecution {
    attempt: SubscriptionUpdateAttempt,
    is_current: bool,
    transport: TransportKind,
    artifact: SubscriptionArtifactRecord,
}

struct ProfileUpdateFailure {
    attempt: SubscriptionUpdateAttempt,
    stage: UpdateStage,
    transport: Option<TransportKind>,
    artifact: Option<SubscriptionArtifactRecord>,
    error: String,
}

fn log_profile_update_fetch_error(stage: &str, err: &anyhow::Error) {
    logging!(
        debug,
        Type::Config,
        "[Subscription Update] {} detailed error chain: {}",
        stage,
        mask_err(&format!("{err:#}"))
    );
}

fn format_subscription_update_error(err: &anyhow::Error) -> String {
    mask_err(&format!("{err:#}"))
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>()
        .join(": ")
        .into()
}

fn append_subscription_update_note(message: impl Into<std::string::String>, note: Option<&String>) -> String {
    let mut message = message.into();

    if let Some(note) = note.map(|note| note.trim()).filter(|note| !note.is_empty()) {
        message.push_str(" Note: ");
        message.push_str(note);
    }

    message.into()
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
    logging!(
        info,
        Type::Config,
        "[Subscription Update] start downloading remote subscription"
    );
    let merged_opt = PrfOption::merge(opt, option);
    let persisted_option = opt.cloned();
    let is_current = {
        let profiles = Config::profiles().await;
        profiles.latest_arc().is_current_profile_index(uid)
    };

    let attempt = SubscriptionUpdateAttempt::new(
        uid.clone(),
        if is_manual_trigger {
            UpdateTrigger::Manual
        } else {
            UpdateTrigger::Automatic
        },
    );

    handle::Handle::notify_subscription_attempt_started(&attempt);
    handle::Handle::notify_subscription_stage_changed(&attempt, UpdateStage::ResolveSource, None);
    handle::Handle::notify_subscription_stage_changed(&attempt, UpdateStage::ResolveTransportPlan, None);

    let transport_plan =
        TransportPlan::for_subscription_update(Some(transport_kind_from_option(merged_opt.as_ref()))).await;
    if let Some(note) = transport_plan.note.as_ref() {
        logging!(
            info,
            Type::Config,
            "[Subscription Update] transport plan note: {}",
            note
        );
    }
    let use_dedicated_control_plane = subscription_update_uses_dedicated_control_plane().await;
    let transport_plan_note = transport_plan.note.clone();
    let mut last_err = None;
    let mut last_stage = UpdateStage::FetchPayload;
    let mut last_transport = None;
    let mut last_artifact = None;

    for candidate in transport_plan.ordered_candidates {
        let transport = candidate.kind;
        let attempt_option = apply_transport_to_option(merged_opt.as_ref(), transport);

        if matches!(transport, TransportKind::LocalProxy) {
            if let Err(err) = mihomo_runtime_guard::ensure_mihomo_core_ready().await {
                logging!(
                    warn,
                    Type::Config,
                    "Warning: [Subscription Update] {} skipped because Mihomo core is not ready: {}",
                    transport.label(),
                    format_subscription_update_error(&err)
                );
                log_profile_update_fetch_error("ensure mihomo core ready", &err);
                last_stage = UpdateStage::FetchPayload;
                last_transport = Some(transport);
                last_artifact = None;
                last_err = Some(err);
                continue;
            }
        }

        handle::Handle::notify_subscription_stage_changed(&attempt, UpdateStage::FetchPayload, Some(transport));

        let fetched = match if use_dedicated_control_plane && matches!(transport, TransportKind::LocalProxy) {
            fetch_subscription_update_via_control_plane(url, &attempt_option).await
        } else {
            fetch_remote_profile(url, Some(&attempt_option)).await
        } {
            Ok(fetched) => fetched,
            Err(err) => {
                logging!(
                    warn,
                    Type::Config,
                    "Warning: [Subscription Update] {} failed: {}",
                    transport.label(),
                    format_subscription_update_error(&err)
                );
                log_profile_update_fetch_error(transport.label(), &err);
                last_stage = UpdateStage::FetchPayload;
                last_transport = Some(transport);
                last_artifact = None;
                last_err = Some(err);
                continue;
            }
        };

        handle::Handle::notify_subscription_stage_changed(&attempt, UpdateStage::DecodePayload, Some(transport));

        let raw_body = fetched.body.clone();
        let format_detection =
            match parse_clash_yaml_subscription(raw_body.as_str(), fetched.metadata.content_type.as_deref()) {
                Ok((_, detection)) => detection,
                Err(err) => {
                    logging!(
                        warn,
                        Type::Config,
                        "Warning: [Subscription Update] {} returned an unsupported payload format: {}",
                        transport.label(),
                        format_subscription_update_error(&err)
                    );
                    log_profile_update_fetch_error("decode payload", &err);
                    last_stage = UpdateStage::DecodePayload;
                    last_transport = Some(transport);
                    last_artifact = None;
                    last_err = Some(err);
                    continue;
                }
            };

        handle::Handle::notify_subscription_stage_changed(&attempt, UpdateStage::MaterializeArtifact, Some(transport));

        let artifact = build_artifact_record(
            raw_body.as_str(),
            chrono::Local::now().timestamp_millis(),
            fetched.metadata.content_type.clone(),
            Some(format_detection.format),
        );

        if let Err(err) = persist_artifact(uid.as_str(), &artifact, raw_body.as_str()).await {
            return Err(ProfileUpdateFailure {
                attempt: attempt.clone(),
                stage: UpdateStage::MaterializeArtifact,
                transport: Some(transport),
                artifact: Some(artifact.clone()),
                error: format!("failed to persist subscription artifact: {err}").into(),
            });
        }

        let mut item = match PrfItem::from_fetched_payload(url, fetched, None, None, persisted_option.as_ref()).await {
            Ok(item) => item,
            Err(err) => {
                logging!(
                    warn,
                    Type::Config,
                    "Warning: [Subscription Update] {} returned an invalid payload: {}",
                    transport.label(),
                    format_subscription_update_error(&err)
                );
                log_profile_update_fetch_error("materialize artifact", &err);
                last_stage = UpdateStage::MaterializeArtifact;
                last_transport = Some(transport);
                last_artifact = Some(artifact.clone());
                last_err = Some(err);
                continue;
            }
        };

        if let Err(err) = profiles_draft_update_item_safe(uid, &mut item).await {
            return Err(ProfileUpdateFailure {
                attempt: attempt.clone(),
                stage: UpdateStage::MaterializeArtifact,
                transport: Some(transport),
                artifact: Some(artifact.clone()),
                error: format!("failed to apply materialized subscription profile: {err}").into(),
            });
        }

        logging!(
            info,
            Type::Config,
            "[Subscription Update] subscription fetch succeeded via {}",
            transport.label()
        );
        return Ok(ProfileUpdateExecution {
            attempt,
            is_current,
            transport,
            artifact,
        });
    }

    let last_err = last_err.unwrap_or_else(|| anyhow!("subscription update transport plan produced no attempts"));
    Err(ProfileUpdateFailure {
        attempt,
        stage: last_stage,
        transport: last_transport,
        artifact: last_artifact,
        error: append_subscription_update_note(
            format!(
                "failed to update profile after all transport attempts: {}",
                format_subscription_update_error(&last_err)
            ),
            transport_plan_note.as_ref(),
        ),
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

    let update_execution = match perform_profile_update(uid, &url, opt.as_ref(), option, is_manual_trigger).await {
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
        handle::Handle::notify_subscription_stage_changed(
            &update_execution.attempt,
            UpdateStage::ActivateRuntime,
            None,
        );
        logging!(
            info,
            Type::Config,
            "[Subscription Update] applying updated profile to runtime"
        );

        match CoreManager::global()
            .update_config_without_restart_with_force(is_manual_trigger)
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
                    true,
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
                    true,
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
            true,
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
            true,
        )
        .await;
    }

    Ok(())
}
