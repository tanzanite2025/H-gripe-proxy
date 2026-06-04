use crate::{
    config::{Config, IProfiles, PrfItem, PrfOption, profiles::profiles_draft_update_item_safe},
    core::{CoreManager, handle, tray, validate::ValidationOutcome},
    utils::help::{mask_err, mask_url},
};
use anyhow::{Result, bail};
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

/// Toggle proxy profile — directly calls the same logic as patch_profiles_config_by_profile_index
pub async fn toggle_proxy_profile(profile_index: String) {
    logging_error!(
        Type::Config,
        patch_profiles_config_by_profile_index(profile_index).await
    );
}

/// 根据profile name修改profiles (feat 层版本，返回 anyhow::Result)
pub async fn patch_profiles_config_by_profile_index(profile_index: String) -> Result<ValidationOutcome> {
    let profiles = IProfiles {
        current: Some(profile_index),
        items: None,
    };
    patch_profiles_config(profiles).await
}

/// 修改profiles的配置（核心业务逻辑）
pub async fn patch_profiles_config(profiles: IProfiles) -> Result<ValidationOutcome> {
    if CURRENT_SWITCHING_PROFILE
        .compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed)
        .is_err()
    {
        logging!(info, Type::Config, "当前正在切换配置，放弃请求");
        return Ok(ValidationOutcome::Busy);
    }

    let target_profile = profiles.current.as_ref();

    logging!(
        info,
        Type::Config,
        "开始修改配置文件，目标profile: {:?}",
        target_profile
    );

    let previous_profile = Config::profiles().await.data_arc().current.clone();
    logging!(info, Type::Config, "当前配置: {:?}", previous_profile);

    Config::profiles().await.edit_draft(|d| d.patch_config(&profiles));

    perform_config_update(target_profile, previous_profile.as_ref()).await
}

async fn restore_previous_profile(prev_profile: &String) -> Result<()> {
    logging!(info, Type::Config, "尝试恢复到之前的配置: {}", prev_profile);
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
            logging!(warn, Type::Config, "Warning: 异步保存恢复配置文件失败: {e}");
        }
    });
    logging!(info, Type::Config, "成功恢复到之前的配置");
    Ok(())
}

async fn handle_success(current_value: Option<&String>) -> Result<ValidationOutcome> {
    Config::profiles().await.apply();
    handle::Handle::refresh_clash();

    if let Err(e) = tray::Tray::global().update_tooltip().await {
        logging!(warn, Type::Config, "Warning: 异步更新托盘提示失败: {e}");
    }

    if let Err(e) = tray::Tray::global().update_menu().await {
        logging!(warn, Type::Config, "Warning: 异步更新托盘菜单失败: {e}");
    }

    if let Err(e) = crate::config::profiles::profiles_save_file_safe().await {
        logging!(warn, Type::Config, "Warning: 异步保存配置文件失败: {e}");
    }

    if let Some(current) = current_value
        && crate::utils::window_manager::WindowManager::get_main_window().is_some()
    {
        logging!(info, Type::Config, "向前端发送配置变更事件: {}", current);
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
    logging!(warn, Type::Config, "配置验证失败: {}", outcome);
    discard_and_restore(current_profile).await?;
    crate::cmd::validate::handle_validation_notice(
        &outcome,
        crate::cmd::validate::ValidationNoticeTarget::Runtime,
        "运行时配置",
    );
    Ok(outcome)
}

async fn handle_update_error<E: std::fmt::Display>(
    e: E,
    current_profile: Option<&String>,
) -> Result<ValidationOutcome> {
    logging!(warn, Type::Config, "更新过程发生错误: {}", e);
    discard_and_restore(current_profile).await?;
    let message: String = e.to_string().into();
    handle::Handle::notice_message("config_validate::boot_error", message.clone());
    Ok(ValidationOutcome::invalid_from_message(message))
}

async fn handle_timeout(current_profile: Option<&String>) -> Result<ValidationOutcome> {
    let timeout_msg: String = "配置更新超时(30秒)，可能是配置验证或核心通信阻塞".into();
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
            logging!(info, Type::Tray, "切换代理成功: {} -> {}", group_name, proxy_name);
            if let Err(error) = crate::feat::sync_runtime_stable_egress_selection().await {
                logging!(
                    warn,
                    Type::Tray,
                    "稳定出口运行态回写失败: {} -> {}, 错误: {}",
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
                "切换代理失败: {} -> {}, 错误: {:?}",
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
            logging!(info, Type::Tray, "代理切换回退成功: {} -> {}", group_name, proxy_name);
            if let Err(error) = crate::feat::sync_runtime_stable_egress_selection().await {
                logging!(
                    warn,
                    Type::Tray,
                    "稳定出口运行态回写失败: {} -> {}, 错误: {}",
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
                "代理切换最终失败: {} -> {}, 错误: {:?}",
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
        logging!(info, Type::Config, "[订阅更新] {uid} 不是远程订阅，跳过更新");
        Ok(None)
    } else if item.url.is_none() {
        logging!(warn, Type::Config, "Warning: [订阅更新] {uid} 缺少URL，无法更新");
        bail!("failed to get the profile item url");
    } else if !ignore_auto_update && !item.option.as_ref().and_then(|o| o.allow_auto_update).unwrap_or(true) {
        logging!(info, Type::Config, "[订阅更新] {} 禁止自动更新，跳过更新", uid);
        Ok(None)
    } else {
        logging!(
            info,
            Type::Config,
            "[订阅更新] {} 是远程订阅，URL: {}",
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
                        "Warning: [订阅更新] current profile file is missing before update, will recreate it: {}",
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

async fn perform_profile_update(
    uid: &String,
    url: &String,
    opt: Option<&PrfOption>,
    option: Option<&PrfOption>,
    is_mannual_trigger: bool,
) -> Result<bool> {
    logging!(info, Type::Config, "[订阅更新] 开始下载新的订阅内容");
    let mut merged_opt = PrfOption::merge(opt, option);
    let is_current = {
        let profiles = Config::profiles().await;
        profiles.latest_arc().is_current_profile_index(uid)
    };
    let profiles = Config::profiles().await;
    let profiles_arc = profiles.latest_arc();
    let profile_name = profiles_arc
        .get_name_by_uid(uid)
        .cloned()
        .unwrap_or_else(|| String::from("UnKnown Profile"));
    let strict_direct_update =
        option.is_some_and(|option| option.with_proxy == Some(false) && option.self_proxy == Some(false));

    let mut last_err;

    match PrfItem::from_url(url, None, None, merged_opt.as_ref()).await {
        Ok(mut item) => {
            logging!(info, Type::Config, "[订阅更新] 更新订阅配置成功");
            profiles_draft_update_item_safe(uid, &mut item).await?;
            return Ok(is_current);
        }
        Err(err) => {
            logging!(
                warn,
                Type::Config,
                "Warning: [订阅更新] 正常更新失败: {}{}",
                mask_err(&err.to_string()),
                if strict_direct_update {
                    ""
                } else {
                    "，尝试使用Clash代理更新"
                }
            );

            if strict_direct_update {
                if is_mannual_trigger {
                    handle::Handle::notice_message("update_failed", format!("{profile_name} - {err}"));
                }
                bail!(
                    "failed to update profile with direct connection: {}",
                    mask_err(&err.to_string())
                );
            }

            last_err = err;
        }
    }

    merged_opt.get_or_insert_with(PrfOption::default).self_proxy = Some(true);
    merged_opt.get_or_insert_with(PrfOption::default).with_proxy = Some(false);

    match PrfItem::from_url(url, None, None, merged_opt.as_ref()).await {
        Ok(mut item) => {
            logging!(info, Type::Config, "[订阅更新] 使用 Clash代理 更新订阅配置成功");
            profiles_draft_update_item_safe(uid, &mut item).await?;
            handle::Handle::notice_message("update_with_clash_proxy", profile_name);
            drop(last_err);
            return Ok(is_current);
        }
        Err(err) => {
            logging!(
                warn,
                Type::Config,
                "Warning: [订阅更新] Clash代理更新失败: {}，尝试使用系统代理更新",
                mask_err(&err.to_string())
            );
            last_err = err;
        }
    }

    merged_opt.get_or_insert_with(PrfOption::default).self_proxy = Some(false);
    merged_opt.get_or_insert_with(PrfOption::default).with_proxy = Some(true);

    match PrfItem::from_url(url, None, None, merged_opt.as_ref()).await {
        Ok(mut item) => {
            logging!(info, Type::Config, "[订阅更新] 使用 系统代理 更新订阅配置成功");
            profiles_draft_update_item_safe(uid, &mut item).await?;
            handle::Handle::notice_message("update_with_clash_proxy", profile_name);
            drop(last_err);
            return Ok(is_current);
        }
        Err(err) => {
            logging!(
                warn,
                Type::Config,
                "Warning: [订阅更新] 系统代理更新失败: {}，所有重试均已失败",
                mask_err(&err.to_string())
            );
            last_err = err;
        }
    }

    if is_mannual_trigger {
        handle::Handle::notice_message("update_failed_even_with_clash", format!("{profile_name} - {last_err}"));
    }
    bail!("failed to update profile after all proxy attempts: {last_err}")
}

pub async fn update_profile(
    uid: &String,
    option: Option<&PrfOption>,
    auto_refresh: bool,
    ignore_auto_update: bool,
    is_mannual_trigger: bool,
) -> Result<()> {
    logging!(info, Type::Config, "[订阅更新] 开始更新订阅 {}", uid);
    let url_opt = should_update_profile(uid, ignore_auto_update).await?;
    let rollback_snapshot = if url_opt.is_some() {
        snapshot_profile_update(uid).await?
    } else {
        None
    };

    let should_refresh = match url_opt {
        Some((url, opt)) => {
            perform_profile_update(uid, &url, opt.as_ref(), option, is_mannual_trigger).await? && auto_refresh
        }
        None => auto_refresh,
    };

    if should_refresh {
        logging!(info, Type::Config, "[订阅更新] 更新内核配置");
        match CoreManager::global()
            .update_config_without_restart_with_force(is_mannual_trigger)
            .await
        {
            Ok(outcome) if outcome.is_valid() => {
                logging!(info, Type::Config, "[订阅更新] 更新成功");
                handle::Handle::refresh_clash();
            }
            Ok(outcome @ (ValidationOutcome::Skipped { .. } | ValidationOutcome::Busy)) if !is_mannual_trigger => {
                logging!(info, Type::Config, "[订阅更新] 本次配置刷新已跳过: {}", outcome);
            }
            Ok(outcome) => {
                let message = outcome.to_string();
                logging!(error, Type::Config, "[订阅更新] 更新失败: {}", message);
                if let Some(rollback_snapshot) = &rollback_snapshot {
                    restore_profile_update_snapshot(&rollback_snapshot).await?;
                }
                handle::Handle::notice_message("update_failed", message.clone());
                if is_mannual_trigger {
                    bail!("failed to apply updated profile: {message}");
                }
            }
            Err(err) => {
                logging!(error, Type::Config, "[订阅更新] 更新失败: {}", err);
                if let Some(rollback_snapshot) = &rollback_snapshot {
                    restore_profile_update_snapshot(&rollback_snapshot).await?;
                }
                handle::Handle::notice_message("update_failed", format!("{err}"));
                logging!(error, Type::Config, "{err}");
                if is_mannual_trigger {
                    bail!("failed to apply updated profile: {err}");
                }
            }
        }
    }

    Ok(())
}

/// 增强配置
pub async fn enhance_profiles() -> Result<ValidationOutcome> {
    CoreManager::global().update_config_forced().await
}
