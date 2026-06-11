use crate::{
    config::{Config, IProfiles, PrfItem, PrfOption, profiles::profiles_draft_update_item_safe},
    core::{CoreManager, handle, tray, validate::ValidationOutcome},
    subscription::{
        fetch::fetch_remote_profile,
        model::{SubscriptionArtifactRecord, SubscriptionUpdateAttempt, UpdateFinalStatus, UpdateStage, UpdateTrigger},
        persist::{build_artifact_record, build_finished_attempt_record, persist_artifact, persist_attempt_result},
        transport::{TransportKind, TransportPlan, apply_transport_to_option},
    },
    utils::help::{mask_err, mask_url},
};
use anyhow::{Result, anyhow, bail};
use clash_verge_logging::{Type, logging, logging_error};
use scopeguard::defer;
use smartstring::alias::String;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;
use tauri::Emitter as _;
use tokio::fs;

static CURRENT_SWITCHING_PROFILE: AtomicBool = AtomicBool::new(false);

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

/// Toggle proxy profile 闁?directly calls the same logic as patch_profiles_config_by_profile_index
pub async fn toggle_proxy_profile(profile_index: String) {
    logging_error!(
        Type::Config,
        patch_profiles_config_by_profile_index(profile_index).await
    );
}

/// 闁哄秷顫夊畵涔竢ofile name濞ｅ浂鍠楅弫绱乺ofiles (feat 閻忕偛鍊绘晶妤呭嫉椤掑﹦绀夐弶鈺傛煥濞?anyhow::Result)
pub async fn patch_profiles_config_by_profile_index(profile_index: String) -> Result<ValidationOutcome> {
    let profiles = IProfiles {
        current: Some(profile_index),
        items: None,
    };
    patch_profiles_config(profiles).await
}

/// Update profiles config (feature-layer entrypoint)
pub async fn patch_profiles_config(profiles: IProfiles) -> Result<ValidationOutcome> {
    if CURRENT_SWITCHING_PROFILE
        .compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed)
        .is_err()
    {
        logging!(info, Type::Config, "Profile switch is already in progress, skip the new request");
        return Ok(ValidationOutcome::Busy);
    }

    let target_profile = profiles.current.as_ref();

    logging!(
        info,
        Type::Config,
        "鐎殿喒鍋撳┑顔碱儎閹便劑寮ㄨぐ鎺戝赋缂傚喚鍠楅弸鍐╃鐠佸湱绀夐柣鈺婂枟閻栴柖rofile: {:?}",
        target_profile
    );

    let previous_profile = Config::profiles().await.data_arc().current.clone();
    logging!(info, Type::Config, "鐟滅増鎸告晶鐘绘煀瀹ュ洨鏋? {:?}", previous_profile);

    Config::profiles().await.edit_draft(|d| d.patch_config(&profiles));

    perform_config_update(target_profile, previous_profile.as_ref()).await
}

async fn restore_previous_profile(prev_profile: &String) -> Result<()> {
    logging!(info, Type::Config, "閻忓繑绻嗛惁顖炲箒閵忕媭妲婚柛鎺楊暒缁狅綁宕滃鍥ㄧ暠闂佹澘绉堕悿? {}", prev_profile);
    let restore_profiles = IProfiles {
        current: Some(prev_profile.to_owned()),
        items: None,
    };
    Config::profiles()
        .await
        .edit_draft(|d| d.patch_config(&restore_profiles));
    Config::profiles().await.apply();
    crate::process::AsyncHandler::spawn(|| async move {
        if let Err(e) = crate::config::profiles::profiles_save_file_safe().await {
            logging!(warn, Type::Config, "Warning: 鐎殿喖鍊归鐐寸┍濠靛棛鎽犻柟顓滃灩椤︽煡鏌婂鍥╂瀭闁哄倸娲ｅ▎銏″緞鏉堫偉袝: {e}");
        }
    });
    logging!(info, Type::Config, "Restored the previous profile after update failure");
    Ok(())
}

async fn handle_success(current_value: Option<&String>) -> Result<ValidationOutcome> {
    Config::profiles().await.apply();
    handle::Handle::refresh_clash();

    if let Err(e) = tray::Tray::global().update_tooltip().await {
        logging!(warn, Type::Config, "Warning: 鐎殿喖鍊归鐐哄即鐎涙ɑ鐓€闁瑰灚顭囧ú蹇涘箵閹邦喓浠涘鎯扮簿鐟? {e}");
    }

    if let Err(e) = tray::Tray::global().update_menu().await {
        logging!(warn, Type::Config, "Warning: 鐎殿喖鍊归鐐哄即鐎涙ɑ鐓€闁瑰灚顭囧ú蹇涙嚕濠婂啫绀嬪鎯扮簿鐟? {e}");
    }

    if let Err(e) = crate::config::profiles::profiles_save_file_safe().await {
        logging!(warn, Type::Config, "Warning: 鐎殿喖鍊归鐐寸┍濠靛棛鎽犻梺鏉跨Ф閻ゅ棝寮崶锔筋偨濠㈡儼绮剧憴? {e}");
    }

    if let Some(current) = current_value
        && crate::utils::window_manager::WindowManager::get_main_window().is_some()
    {
        logging!(info, Type::Config, "闁告碍鍨垫晶鐘电博椤栨艾绲洪梺顐＄窔閸樸倗绱旈鐓庣秮闁哄洨绻濈花銊︾? {}", current);
        handle::Handle::notify_profile_changed(current);
    }

    Ok(ValidationOutcome::Valid)
}

async fn discard_and_restore(current_profile: Option<&String>) -> Result<()> {
    Config::profiles().await.discard();
    if let Some(prev_profile) = current_profile {
        restore_previous_profile(prev_profile).await?;
    }
    Ok(())
}

async fn handle_validation_failure(
    outcome: ValidationOutcome,
    current_profile: Option<&String>,
) -> Result<ValidationOutcome> {
    logging!(warn, Type::Config, "闂佹澘绉堕悿鍡橆殽瀹€鍐濠㈡儼绮剧憴? {}", outcome);
    discard_and_restore(current_profile).await?;
    crate::cmd::validate::handle_validation_notice(
        &outcome,
        crate::cmd::validate::ValidationNoticeTarget::Runtime,
        "runtime config",
    );
    Ok(outcome)
}

async fn handle_update_error<E: std::fmt::Display>(
    e: E,
    current_profile: Option<&String>,
) -> Result<ValidationOutcome> {
    logging!(warn, Type::Config, "闁哄洤鐡ㄩ弻濠冩交閸モ斁鏌ら柛娆愬灩閺佹捇鏌ㄥ▎鎺濆殩: {}", e);
    discard_and_restore(current_profile).await?;
    let message: String = e.to_string().into();
    handle::Handle::notice_message("config_validate::boot_error", message.clone());
    Ok(ValidationOutcome::invalid_from_message(message))
}

async fn handle_timeout(current_profile: Option<&String>) -> Result<ValidationOutcome> {
    let timeout_msg: String = "Config update timed out after 30 seconds; validation or core communication may be blocked".into();
    logging!(error, Type::Config, "{}", timeout_msg);
    discard_and_restore(current_profile).await?;
    handle::Handle::notice_message("config_validate::timeout", timeout_msg.clone());
    Ok(ValidationOutcome::invalid_from_message(timeout_msg))
}

async fn perform_config_update(
    current_value: Option<&String>,
    current_profile: Option<&String>,
) -> Result<ValidationOutcome> {
    defer! {
        CURRENT_SWITCHING_PROFILE.store(false, Ordering::Release);
    }
    let update_result =
        tokio::time::timeout(Duration::from_secs(30), CoreManager::global().update_config_forced()).await;

    match update_result {
        Ok(Ok(outcome)) if outcome.is_valid() => handle_success(current_value).await,
        Ok(Ok(outcome)) => handle_validation_failure(outcome, current_profile).await,
        Ok(Err(e)) => handle_update_error(e, current_profile).await,
        Err(_) => handle_timeout(current_profile).await,
    }
}

pub async fn switch_proxy_node(group_name: &str, proxy_name: &str) {
    match handle::Handle::mihomo()
        .await
        .select_node_for_group(group_name, proxy_name)
        .await
    {
        Ok(_) => {
            logging!(info, Type::Tray, "闁告帒娲﹀畷鍙夌閿濆洦鍊為柟瀛樺姇婵? {} -> {}", group_name, proxy_name);
            if let Err(error) = crate::feat::sync_runtime_stable_egress_selection().await {
                logging!(
                    warn,
                    Type::Tray,
                    "缂佸鍟块悾楣冨礄閸濆嫬缍撻弶鈺傚姌椤㈡垿骞€娴ｅ憡绀€闁告劖鐟ラ妵鎴犳嫻? {} -> {}, 闂佹寧鐟ㄩ? {}",
                    group_name,
                    proxy_name,
                    error
                );
            }
            let _ = handle::Handle::app_handle().emit("verge://refresh-proxy-config", ());
            let _ = tray::Tray::global().update_menu().await;
            return;
        }
        Err(err) => {
            logging!(
                error,
                Type::Tray,
                "闁告帒娲﹀畷鍙夌閿濆洦鍊炲鎯扮簿鐟? {} -> {}, 闂佹寧鐟ㄩ? {:?}",
                group_name,
                proxy_name,
                err
            );
        }
    }

    match handle::Handle::mihomo()
        .await
        .select_node_for_group(group_name, proxy_name)
        .await
    {
        Ok(_) => {
            logging!(info, Type::Tray, "濞寸媴绲块幃濠囧礆閸ャ劌搴婇柛銉у仱閳ь兘鍋撻柟瀛樺姇婵? {} -> {}", group_name, proxy_name);
            if let Err(error) = crate::feat::sync_runtime_stable_egress_selection().await {
                logging!(
                    warn,
                    Type::Tray,
                    "缂佸鍟块悾楣冨礄閸濆嫬缍撻弶鈺傚姌椤㈡垿骞€娴ｅ憡绀€闁告劖鐟ラ妵鎴犳嫻? {} -> {}, 闂佹寧鐟ㄩ? {}",
                    group_name,
                    proxy_name,
                    error
                );
            }
            let _ = tray::Tray::global().update_menu().await;
        }
        Err(err) => {
            logging!(
                error,
                Type::Tray,
                "濞寸媴绲块幃濠囧礆閸ャ劌搴婇柡鍫氬亾缂備礁鐗嗛妵鎴犳嫻? {} -> {}, 闂佹寧鐟ㄩ? {:?}",
                group_name,
                proxy_name,
                err
            );
        }
    }
}

async fn should_update_profile(uid: &String, ignore_auto_update: bool) -> Result<Option<(String, Option<PrfOption>)>> {
    let profiles = Config::profiles().await;
    let profiles = profiles.latest_arc();
    let item = profiles.get_item(uid)?;
    let is_remote = item.itype.as_ref().is_some_and(|s| s == "remote");

    if !is_remote {
        logging!(info, Type::Config, "[Subscription Update] {uid} is not a remote subscription, skip update");
        Ok(None)
    } else if item.url.is_none() {
        logging!(warn, Type::Config, "Warning: [Subscription Update] {uid} is missing a URL and cannot be updated");
        bail!("failed to get the profile item url");
    } else if !ignore_auto_update && !item.option.as_ref().and_then(|o| o.allow_auto_update).unwrap_or(true) {
        logging!(info, Type::Config, "[Subscription Update] {} has auto update disabled, skip update", uid);
        Ok(None)
    } else {
        logging!(
            info,
            Type::Config,
            "[閻犱降鍨藉Σ鍕即鐎涙ɑ鐓€] {} 闁哄嫷鍨电换娆戠矙鐎ｎ収鍚傞梻鍐ㄦ嫅缁辨紛RL: {}",
            uid,
            mask_url(
                item.url
                    .as_ref()
                    .ok_or_else(|| anyhow::anyhow!("Profile URL is None"))?
            )
        );
        Ok(Some((
            item.url.clone().ok_or_else(|| anyhow::anyhow!("Profile URL is None"))?,
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
            let path = crate::utils::dirs::app_profiles_dir()?.join(file.as_str());
            match fs::try_exists(&path).await {
                Ok(true) => Some(fs::read_to_string(path).await?.into()),
                Ok(false) => {
                    logging!(
                        warn,
                        Type::Config,
                        "Warning: [閻犱降鍨藉Σ鍕即鐎涙ɑ鐓€] current profile file is missing before update, will recreate it: {}",
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
        let path = crate::utils::dirs::app_profiles_dir()?.join(file.as_str());
        fs::write(path, file_data.as_bytes()).await?;
    }

    let uid = snapshot
        .item
        .uid
        .clone()
        .ok_or_else(|| anyhow::anyhow!("profile update snapshot is missing uid"))?;
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

async fn perform_profile_update_v2(
    uid: &String,
    url: &String,
    opt: Option<&PrfOption>,
    option: Option<&PrfOption>,
    is_mannual_trigger: bool,
) -> std::result::Result<ProfileUpdateExecution, ProfileUpdateFailure> {
    logging!(info, Type::Config, "[Subscription Update] start downloading remote subscription");
    let merged_opt = PrfOption::merge(opt, option);
    let is_current = {
        let profiles = Config::profiles().await;
        profiles.latest_arc().is_current_profile_index(uid)
    };

    let attempt = SubscriptionUpdateAttempt::new(
        uid.clone(),
        if is_mannual_trigger {
            UpdateTrigger::Manual
        } else {
            UpdateTrigger::Automatic
        },
    );

    handle::Handle::notify_subscription_attempt_started(&attempt);
    handle::Handle::notify_subscription_stage_changed(&attempt, UpdateStage::ResolveSource, None);
    handle::Handle::notify_subscription_stage_changed(&attempt, UpdateStage::ResolveTransportPlan, None);

    let transport_plan = TransportPlan::for_subscription_update().await;
    let mut last_err = None;
    let mut last_stage = UpdateStage::FetchPayload;
    let mut last_transport = None;
    let mut last_artifact = None;

    for candidate in transport_plan.ordered_candidates {
        let transport = candidate.kind;
        let attempt_option = apply_transport_to_option(merged_opt.as_ref(), transport);

        if matches!(transport, TransportKind::LocalProxy) {
            if let Err(err) = crate::feat::ensure_mihomo_core_ready().await {
                logging!(
                    warn,
                    Type::Config,
                    "Warning: [Subscription Update] {} skipped because Mihomo core is not ready: {}",
                    transport.label(),
                    mask_err(&err.to_string())
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

        let fetched = match fetch_remote_profile(url, Some(&attempt_option)).await {
            Ok(fetched) => fetched,
            Err(err) => {
                logging!(
                    warn,
                    Type::Config,
                    "Warning: [Subscription Update] {} failed: {}",
                    transport.label(),
                    mask_err(&err.to_string())
                );
                log_profile_update_fetch_error(transport.label(), &err);
                last_stage = UpdateStage::FetchPayload;
                last_transport = Some(transport);
                last_artifact = None;
                last_err = Some(err);
                continue;
            }
        };

        handle::Handle::notify_subscription_stage_changed(&attempt, UpdateStage::MaterializeArtifact, Some(transport));

        let raw_body = fetched.body.clone();
        let artifact = build_artifact_record(
            raw_body.as_str(),
            chrono::Local::now().timestamp_millis(),
            fetched.metadata.content_type.clone(),
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

        let mut item = match PrfItem::from_fetched_payload(url, fetched, None, None, Some(&attempt_option)).await {
            Ok(item) => item,
            Err(err) => {
                logging!(
                    warn,
                    Type::Config,
                    "Warning: [Subscription Update] {} returned an invalid payload: {}",
                    transport.label(),
                    mask_err(&err.to_string())
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
        error: format!("failed to update profile after all transport attempts: {last_err}").into(),
    })
}

pub async fn update_profile(
    uid: &String,
    option: Option<&PrfOption>,
    auto_refresh: bool,
    ignore_auto_update: bool,
    is_mannual_trigger: bool,
) -> Result<()> {
    logging!(info, Type::Config, "[Subscription Update] start updating subscription {}", uid);

    let Some((url, opt)) = should_update_profile(uid, ignore_auto_update).await? else {
        return Ok(());
    };

    let rollback_snapshot = snapshot_profile_update(uid).await?;

    let update_execution = match perform_profile_update_v2(uid, &url, opt.as_ref(), option, is_mannual_trigger).await {
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

            let artifact_version = failure
                .artifact
                .as_ref()
                .map(|artifact| artifact.version.clone());
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
        logging!(info, Type::Config, "[Subscription Update] applying updated profile to runtime");

        match CoreManager::global()
            .update_config_without_restart_with_force(is_mannual_trigger)
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
            Ok(outcome @ (ValidationOutcome::Skipped { .. } | ValidationOutcome::Busy)) if !is_mannual_trigger => {
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
                logging!(error, Type::Config, "[Subscription Update] runtime activation failed: {}", message);
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
                logging!(error, Type::Config, "[Subscription Update] runtime activation failed: {}", message);
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

pub async fn enhance_profiles() -> Result<ValidationOutcome> {
    CoreManager::global().update_config_forced().await
}
