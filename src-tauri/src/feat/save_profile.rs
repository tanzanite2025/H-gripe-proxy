use crate::{
    config::{Config, IProfiles, PrfItem},
    core::{
        CoreManager, handle,
        validate::{CoreConfigValidator, ValidationOutcome},
    },
    module::auto_backup::{AutoBackupManager, AutoBackupTrigger},
    utils::dirs,
};
use anyhow::Result;
use clash_verge_logging::{Type, logging};
use smartstring::alias::String;
use tokio::fs;

/// 检查 profile 是否影响当前运行时
pub fn profile_affects_runtime(profiles: &IProfiles, index: &str) -> bool {
    let Some(current_uid) = profiles.get_current() else {
        return false;
    };
    if current_uid == index {
        return true;
    }

    let Ok(item) = profiles.get_item(current_uid) else {
        return false;
    };
    [
        item.current_merge().map_or("Merge", String::as_str),
        item.current_script().map_or("Script", String::as_str),
        item.current_rules().map_or("Rules", String::as_str),
        item.current_proxies().map_or("Proxies", String::as_str),
        item.current_groups().map_or("Groups", String::as_str),
    ]
    .contains(&index)
}

/// 保存配置文件（核心业务逻辑）
pub async fn save_profile_file(index: &str, file_data: Option<String>) -> Result<ValidationOutcome> {
    let file_data = match file_data {
        Some(d) => d,
        None => return Ok(ValidationOutcome::Valid),
    };

    let backup_trigger = match index {
        "Merge" => Some(AutoBackupTrigger::GlobalMerge),
        "Script" => Some(AutoBackupTrigger::GlobalScript),
        _ => None,
    };

    let (rel_path, is_merge_file, is_script_file, affects_runtime) = {
        let profiles = Config::profiles().await;
        let profiles_guard = profiles.latest_arc();
        let item = profiles_guard.get_item(index)?;
        let is_merge = item.itype.as_ref().is_some_and(|t| t == "merge");
        let path = item.file.clone().ok_or_else(|| anyhow::anyhow!("file field is null"))?;
        let is_script = item.itype.as_ref().is_some_and(|t| t == "script") || path.ends_with(".js");
        let affects_runtime = profile_affects_runtime(&profiles_guard, index);
        (path, is_merge, is_script, affects_runtime)
    };

    let original_content = PrfItem {
        file: Some(rel_path.clone()),
        ..Default::default()
    }
    .read_file()
    .await?;

    let profiles_dir = dirs::app_profiles_dir()?;
    let file_path = profiles_dir.join(rel_path.as_str());
    let file_path_str = file_path.to_string_lossy().to_string();

    fs::write(&file_path, &file_data).await?;

    logging!(
        info,
        Type::Config,
        "[cmd配置save] 开始验证配置文件: {}, 是否为merge文件: {}",
        file_path_str,
        is_merge_file
    );

    let changes_applied = handle_saved_profile_file(
        &file_path_str,
        &file_path,
        &original_content,
        is_merge_file,
        is_script_file,
        affects_runtime,
    )
    .await?;

    if changes_applied.is_valid()
        && let Some(trigger) = backup_trigger
    {
        AutoBackupManager::trigger_backup(trigger);
    }

    Ok(changes_applied)
}

async fn restore_original(file_path: &std::path::Path, original_content: &str) -> Result<()> {
    fs::write(file_path, original_content).await?;
    Ok(())
}

async fn handle_saved_profile_file(
    file_path_str: &str,
    file_path: &std::path::Path,
    original_content: &str,
    is_merge_file: bool,
    is_script_file: bool,
    affects_runtime: bool,
) -> Result<ValidationOutcome> {
    use crate::cmd::validate::{ValidationNoticeTarget, handle_validation_notice};

    let (target, file_type) = if is_script_file {
        (ValidationNoticeTarget::Script, "脚本文件")
    } else if is_merge_file {
        (ValidationNoticeTarget::Merge, "合并配置文件")
    } else {
        (ValidationNoticeTarget::Runtime, "YAML配置文件")
    };

    logging!(
        info,
        Type::Config,
        "[cmd配置save] 开始{}验证: {}",
        file_type,
        file_path_str
    );

    match CoreConfigValidator::validate_config_file_outcome(file_path_str, Some(is_merge_file)).await {
        Ok(outcome) if outcome.is_valid() => {
            logging!(info, Type::Config, "[cmd配置save] 文件验证通过: {}", file_path_str);
        }
        Ok(outcome) => {
            logging!(warn, Type::Config, "[cmd配置save] 文件验证失败: {}", outcome);
            restore_original(file_path, original_content).await?;
            handle_validation_notice(&outcome, target, file_type);
            return Ok(outcome);
        }
        Err(e) => {
            logging!(error, Type::Config, "[cmd配置save] 验证过程发生错误: {}", e);
            restore_original(file_path, original_content).await?;
            return Err(e);
        }
    }

    if !affects_runtime {
        return Ok(ValidationOutcome::Valid);
    }

    logging!(
        info,
        Type::Config,
        "[cmd配置save] 保存项影响当前运行时配置，开始统一应用"
    );
    match CoreManager::global().update_config_forced().await {
        Ok(outcome) if outcome.is_valid() => {
            handle::Handle::refresh_clash();
            Ok(ValidationOutcome::Valid)
        }
        Ok(outcome) => {
            logging!(warn, Type::Config, "[cmd配置save] 运行时配置应用失败: {}", outcome);
            restore_original(file_path, original_content).await?;
            handle_validation_notice(&outcome, ValidationNoticeTarget::Runtime, "运行时配置");
            Ok(outcome)
        }
        Err(err) => {
            logging!(error, Type::Config, "[cmd配置save] 运行时配置应用错误: {}", err);
            restore_original(file_path, original_content).await?;
            Err(err)
        }
    }
}
