use super::CmdResult;
use super::StringifyErr as _;
use crate::{
    app,
    config::{
        Config, IProfiles, PrfItem, PrfOption, ProfilesView,
        profiles::{
            profiles_append_item_with_filedata_safe, profiles_delete_item_safe, profiles_patch_item_safe,
            profiles_reorder_safe, profiles_save_file_safe,
        },
        profiles_append_item_safe,
    },
    core::{
        CoreManager, handle, timer::Timer, tray::Tray,
        validate::{ValidationNoticeTarget, ValidationOutcome, handle_validation_notice},
    },
    utils::{dirs, help},
};
use clash_verge_logging::{Type, logging};
use smartstring::alias::String;
use std::path::Path;
use tokio::fs;

fn profile_import_error(err: &anyhow::Error) -> std::string::String {
    if let Some(cause) = err.chain().find(|cause| cause.to_string().contains("TLS 1.0/1.1")) {
        return cause.to_string();
    }

    format!("导入订阅失败: {err:#}")
}

#[tauri::command]
pub async fn get_profiles() -> CmdResult<ProfilesView> {
    logging!(debug, Type::Cmd, "获取配置文件列表");
    let draft = Config::profiles().await;
    let data = draft.data_arc();
    Ok(data.to_view())
}

/// 增强配置文件
#[tauri::command]
pub async fn enhance_profiles() -> CmdResult<ValidationOutcome> {
    match app::profile::reactivate_profiles().await {
        Ok(outcome) if outcome.is_valid() => {
            handle::Handle::refresh_clash();
            Ok(outcome)
        }
        Ok(outcome) => {
            logging!(
                warn,
                Type::Cmd,
                "Reactivate profiles command failed validation: {}",
                outcome
            );
            handle_validation_notice(&outcome, ValidationNoticeTarget::Runtime, "运行时配置");
            Ok(outcome)
        }
        Err(e) => {
            logging!(error, Type::Cmd, "{}", e);
            Err(e.to_string().into())
        }
    }
}

/// 导入配置文件
#[tauri::command]
pub async fn import_profile(url: std::string::String, option: Option<PrfOption>) -> CmdResult {
    logging!(info, Type::Cmd, "[导入订阅] 开始导入: {}", help::mask_url(&url));

    let item = &mut match PrfItem::from_url_with_pipeline(&url, None, None, option.as_ref()).await {
        Ok(it) => {
            logging!(info, Type::Cmd, "[导入订阅] 下载完成，开始保存配置");
            it
        }
        Err(e) => {
            logging!(error, Type::Cmd, "[导入订阅] 下载失败: {}", e);
            return Err(profile_import_error(&e).into());
        }
    };

    match profiles_append_item_safe(item).await {
        Ok(_) => match profiles_save_file_safe().await {
            Ok(_) => {
                logging!(info, Type::Cmd, "[导入订阅] 配置文件保存成功");
            }
            Err(e) => {
                logging!(error, Type::Cmd, "[导入订阅] 保存配置文件失败: {}", e);
            }
        },
        Err(e) => {
            logging!(error, Type::Cmd, "[导入订阅] 保存配置失败: {}", e);
            return Err(format!("导入订阅失败: {}", e).into());
        }
    }

    if let Some(uid) = &item.uid {
        logging!(info, Type::Cmd, "[导入订阅] 发送配置变更通知: {}", uid);
        handle::Handle::notify_profile_changed(uid);
    }

    logging!(info, Type::Cmd, "[导入订阅] 导入完成: {}", help::mask_url(&url));
    Ok(())
}

/// 调整profile的顺序
#[tauri::command]
pub async fn reorder_profile(active_id: String, over_id: String) -> CmdResult {
    match profiles_reorder_safe(&active_id, &over_id).await {
        Ok(_) => {
            logging!(info, Type::Cmd, "重新排序配置文件");
            Ok(())
        }
        Err(err) => {
            logging!(error, Type::Cmd, "重新排序配置文件失败: {}", err);
            Err(format!("重新排序配置文件失败: {}", err).into())
        }
    }
}

/// 创建新的profile
#[tauri::command]
pub async fn create_profile(item: PrfItem, file_data: Option<String>) -> CmdResult {
    match profiles_append_item_with_filedata_safe(&item, file_data).await {
        Ok(_) => {
            profiles_save_file_safe().await.stringify_err()?;
            if let Some(uid) = &item.uid {
                logging!(info, Type::Cmd, "[创建订阅] 发送配置变更通知: {}", uid);
                handle::Handle::notify_profile_changed(uid);
            }
            Ok(())
        }
        Err(err) => match err.to_string().as_str() {
            "the file already exists" => Err("the file already exists".into()),
            _ => Err(format!("add profile error: {err}").into()),
        },
    }
}

/// 从本地文件导入配置文件
#[tauri::command]
pub async fn create_profile_from_local_path(item: PrfItem, path: String) -> CmdResult {
    if item.itype.as_deref() != Some("local") {
        return Err("only local profiles are supported".into());
    }

    let path = path.trim();
    if path.is_empty() {
        return Err("file path is empty".into());
    }

    let extension = Path::new(path)
        .extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| ext.to_ascii_lowercase());
    if !matches!(extension.as_deref(), Some("yaml" | "yml")) {
        return Err("only yaml files are supported".into());
    }

    let metadata = fs::metadata(path).await.stringify_err()?;
    if !metadata.is_file() {
        return Err("file path is invalid".into());
    }

    let file_data = fs::read_to_string(path).await.stringify_err()?;
    create_profile(item, Some(file_data.into())).await
}

/// 更新配置文件
#[tauri::command]
pub async fn update_profile(index: String, option: Option<PrfOption>) -> CmdResult {
    match crate::app::subscription::update_profile(&index, option.as_ref(), true, true, true).await {
        Ok(_) => Ok(()),
        Err(e) => {
            logging!(error, Type::Cmd, "{}", e);
            Err(e.to_string().into())
        }
    }
}

/// 删除配置文件
#[tauri::command]
pub async fn delete_profile(index: String) -> CmdResult {
    let should_update = profiles_delete_item_safe(&index).await.stringify_err()?;
    profiles_save_file_safe().await.stringify_err()?;
    if let Err(e) = Tray::global().update_tooltip().await {
        logging!(warn, Type::Cmd, "Warning: 异步更新托盘提示失败: {e}");
    }

    if let Err(e) = Tray::global().update_menu().await {
        logging!(warn, Type::Cmd, "Warning: 异步更新托盘菜单失败: {e}");
    }
    if should_update {
        match CoreManager::global().update_config_forced().await {
            Ok(outcome) if outcome.is_valid() => {
                handle::Handle::refresh_clash();
                logging!(info, Type::Cmd, "[删除订阅] 发送配置变更通知: {}", index);
                handle::Handle::notify_profile_changed(&index);
            }
            Ok(outcome) => {
                logging!(warn, Type::Cmd, "删除订阅后更新配置失败: {}", outcome);
                handle_validation_notice(&outcome, ValidationNoticeTarget::Runtime, "运行时配置");
                return Err(outcome.to_string().into());
            }
            Err(e) => {
                logging!(error, Type::Cmd, "{}", e);
                return Err(e.to_string().into());
            }
        }
    }
    Timer::global().refresh().await.stringify_err()?;
    Ok(())
}

/// 修改profiles的配置
#[tauri::command]
pub async fn patch_profiles_config(profiles: IProfiles) -> CmdResult<ValidationOutcome> {
    crate::app::profile::publish_profile_activation(profiles)
        .await
        .stringify_err()
}

/// 根据profile name修改profiles
#[tauri::command]
pub async fn patch_profiles_config_by_profile_index(profile_index: String) -> CmdResult<ValidationOutcome> {
    crate::app::profile::publish_profile_activation_by_index(profile_index)
        .await
        .stringify_err()
}

/// 修改某个profile item的
#[tauri::command]
pub async fn patch_profile(index: String, profile: PrfItem) -> CmdResult {
    let profiles = Config::profiles().await;
    let should_refresh_timer = if let Ok(old_profile) = profiles.latest_arc().get_item(&index)
        && let Some(new_option) = profile.option.as_ref()
    {
        let old_interval = old_profile.option.as_ref().and_then(|o| o.update_interval);
        let new_interval = new_option.update_interval;
        let old_allow_auto_update = old_profile.option.as_ref().and_then(|o| o.allow_auto_update);
        let new_allow_auto_update = new_option.allow_auto_update;
        (old_interval != new_interval) || (old_allow_auto_update != new_allow_auto_update)
    } else {
        false
    };

    profiles_patch_item_safe(&index, &profile).await.stringify_err()?;

    if should_refresh_timer {
        crate::process::AsyncHandler::spawn(move || async move {
            logging!(info, Type::Timer, "Timer update settings changed, refreshing timer...");
            if let Err(e) = crate::core::Timer::global().refresh().await {
                logging!(error, Type::Timer, "Failed to refresh timer: {}", e);
            } else {
                crate::core::handle::Handle::notify_timer_updated(&index);
            }
        });
    }

    Ok(())
}

/// 查看配置文件
#[tauri::command]
pub async fn view_profile(index: String) -> CmdResult {
    let profiles = Config::profiles().await;
    let profiles_ref = profiles.latest_arc();
    let file = profiles_ref
        .get_item(&index)
        .stringify_err()?
        .file
        .as_ref()
        .ok_or("the file field is null")?;

    let path = dirs::app_profiles_dir().stringify_err()?.join(file.as_str());
    if !path.exists() {
        return CmdResult::Err(format!("file not found \"{}\"", path.display()).into());
    }

    help::open_file(path).stringify_err()
}

/// 读取配置文件内容
#[tauri::command]
pub async fn read_profile_file(index: String) -> CmdResult<String> {
    let item = {
        let profiles = Config::profiles().await;
        let profiles_ref = profiles.latest_arc();
        PrfItem {
            file: profiles_ref.get_item(&index).stringify_err()?.file.to_owned(),
            ..Default::default()
        }
    };
    let data = item.read_file().await.stringify_err()?;
    Ok(data)
}

/// 获取下一次更新时间
#[tauri::command]
pub async fn get_next_update_time(uid: String) -> CmdResult<Option<i64>> {
    let timer = Timer::global();
    let next_time = timer.get_next_update_time(&uid).await;
    Ok(next_time)
}
