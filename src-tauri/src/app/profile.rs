use crate::{
    config::{Config, IProfiles},
    core::{
        CoreManager, handle, tray,
        validate::{ValidationNoticeTarget, ValidationOutcome, handle_validation_notice},
    },
};
use anyhow::Result;
use clash_verge_logging::{Type, logging};
use scopeguard::defer;
use smartstring::alias::String;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

static CURRENT_SWITCHING_PROFILE: AtomicBool = AtomicBool::new(false);

pub async fn publish_profile_activation_by_index(
    profile_index: String,
) -> Result<ValidationOutcome> {
    let profiles = IProfiles {
        current: Some(profile_index),
        items: None,
    };
    publish_profile_activation(profiles).await
}

pub async fn publish_profile_activation(profiles: IProfiles) -> Result<ValidationOutcome> {
    if CURRENT_SWITCHING_PROFILE
        .compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed)
        .is_err()
    {
        logging!(info, Type::Config, "Profile switch is already in progress, skip the new request");
        return Ok(ValidationOutcome::Busy);
    }

    let target_profile = profiles.current.as_ref();
    let previous_profile = Config::profiles().await.data_arc().current.clone();

    Config::profiles().await.edit_draft(|d| d.patch_config(&profiles));
    perform_profile_publish(target_profile, previous_profile.as_ref()).await
}

pub async fn toggle_proxy_profile(profile_index: String) {
    let _ = publish_profile_activation_by_index(profile_index).await;
}

pub async fn reactivate_profiles() -> Result<ValidationOutcome> {
    CoreManager::global().update_config_forced().await
}

async fn restore_previous_profile(prev_profile: &String) -> Result<()> {
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
            logging!(warn, Type::Config, "Warning: failed to persist restored profile: {e}");
        }
    });
    Ok(())
}

async fn handle_success(current_value: Option<&String>) -> Result<ValidationOutcome> {
    Config::profiles().await.apply();
    handle::Handle::refresh_clash();

    if let Err(e) = tray::Tray::global().update_tooltip().await {
        logging!(warn, Type::Config, "Warning: failed to update tray tooltip: {e}");
    }

    if let Err(e) = tray::Tray::global().update_menu().await {
        logging!(warn, Type::Config, "Warning: failed to update tray menu: {e}");
    }

    if let Err(e) = crate::config::profiles::profiles_save_file_safe().await {
        logging!(warn, Type::Config, "Warning: failed to save profiles config: {e}");
    }

    if let Some(current) = current_value
        && crate::utils::window_manager::WindowManager::get_main_window().is_some()
    {
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
    discard_and_restore(current_profile).await?;
    handle_validation_notice(&outcome, ValidationNoticeTarget::Runtime, "runtime config");
    Ok(outcome)
}

async fn handle_update_error<E: std::fmt::Display>(
    e: E,
    current_profile: Option<&String>,
) -> Result<ValidationOutcome> {
    discard_and_restore(current_profile).await?;
    let message: String = e.to_string().into();
    handle::Handle::notice_message("config_validate::boot_error", message.clone());
    Ok(ValidationOutcome::invalid_from_message(message))
}

async fn handle_timeout(current_profile: Option<&String>) -> Result<ValidationOutcome> {
    let timeout_msg: String = "Profile activation timed out after 30 seconds".into();
    discard_and_restore(current_profile).await?;
    handle::Handle::notice_message("config_validate::timeout", timeout_msg.clone());
    Ok(ValidationOutcome::invalid_from_message(timeout_msg))
}

async fn perform_profile_publish(
    current_value: Option<&String>,
    current_profile: Option<&String>,
) -> Result<ValidationOutcome> {
    defer! {
        CURRENT_SWITCHING_PROFILE.store(false, Ordering::Release);
    }

    let update_result = tokio::time::timeout(
        Duration::from_secs(30),
        CoreManager::global().update_config_forced(),
    )
    .await;

    match update_result {
        Ok(Ok(outcome)) if outcome.is_valid() => handle_success(current_value).await,
        Ok(Ok(outcome)) => handle_validation_failure(outcome, current_profile).await,
        Ok(Err(e)) => handle_update_error(e, current_profile).await,
        Err(_) => handle_timeout(current_profile).await,
    }
}
