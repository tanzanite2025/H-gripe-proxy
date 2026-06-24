use crate::{
    config::{Config, IProfiles, PrfItem},
    core::{
        handle, runtime_lifecycle,
        validate::{CoreConfigValidator, ValidationErrorKind, ValidationOutcome},
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
        item.current_proxies().map_or("Proxies", String::as_str),
        item.current_groups().map_or("Groups", String::as_str),
    ]
    .contains(&index)
}

fn find_runtime_dependency_issue(profiles: &IProfiles) -> Result<Option<String>> {
    let Some(current_uid) = profiles.current_primary_uid() else {
        return Ok(None);
    };

    let current_item = profiles.get_item(&current_uid)?;
    let profiles_dir = dirs::app_profiles_dir()?;

    let dependency_uids = [
        ("主订阅", current_uid.as_str()),
        ("合并配置", current_item.current_merge().map_or("Merge", String::as_str)),
        (
            "脚本配置",
            current_item.current_script().map_or("Script", String::as_str),
        ),
        (
            "节点覆盖",
            current_item.current_proxies().map_or("Proxies", String::as_str),
        ),
        (
            "分组覆盖",
            current_item.current_groups().map_or("Groups", String::as_str),
        ),
    ];

    for (label, uid) in dependency_uids {
        let item = match profiles.get_item(uid) {
            Ok(item) => item,
            Err(_) => {
                return Ok(Some(
                    format!("当前运行配置关联项缺失：{label} {uid} 不存在。请先恢复或刷新当前订阅，再保存。").into(),
                ));
            }
        };

        let Some(file) = item.file.as_ref() else {
            return Ok(Some(
                format!("当前运行配置关联项缺失：{label} {uid} 没有 file 字段。请先恢复或刷新当前订阅，再保存。")
                    .into(),
            ));
        };

        let path = profiles_dir.join(file.as_str());
        if !path.is_file() {
            return Ok(Some(
                format!(
                    "当前运行配置关联文件缺失：{label} {uid} -> {}。请先恢复或刷新当前订阅，再保存。",
                    path.display()
                )
                .into(),
            ));
        }
    }

    Ok(None)
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

    let (rel_path, is_merge_file, is_script_file, affects_runtime, runtime_issue) = {
        let profiles = Config::profiles().await;
        let profiles_guard = profiles.latest_arc();
        let item = profiles_guard.get_item(index)?;
        let is_merge = item.itype.as_ref().is_some_and(|t| t == "merge");
        let path = item.file.clone().ok_or_else(|| anyhow::anyhow!("file field is null"))?;
        let is_script = item.itype.as_ref().is_some_and(|t| t == "script") || path.ends_with(".js");
        let affects_runtime = profile_affects_runtime(&profiles_guard, index);
        let runtime_issue = if affects_runtime {
            find_runtime_dependency_issue(&profiles_guard)?
        } else {
            None
        };
        (path, is_merge, is_script, affects_runtime, runtime_issue)
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

    if let Some(message) = runtime_issue {
        handle::Handle::notice_message("config_validate::error", message.clone());
        return Ok(ValidationOutcome::invalid(ValidationErrorKind::FileMissing, message));
    }

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
    use crate::core::validate::{ValidationNoticeTarget, handle_validation_notice};

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
    match runtime_lifecycle::update_runtime_config_forced("save-profile-runtime-apply").await {
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
