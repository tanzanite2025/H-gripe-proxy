use super::{IClashTemp, IProfiles, IVerge};
use crate::{
    config::{PrfItem, profiles_append_item_safe, runtime::IRuntime},
    constants::{files, timing},
    core::{
        CoreManager,
        handle::{self, Handle},
        service, tray,
        validate::CoreConfigValidator,
    },
    enhance,
    process::AsyncHandler,
    utils::{dirs, help},
};
use anyhow::{Result, anyhow};
use backon::{ConstantBuilder, ExponentialBuilder, Retryable as _};
use clash_verge_draft::Draft;
use clash_verge_logging::{Type, logging, logging_error};
use serde_yaml_ng::Mapping;
use smartstring::alias::String;
#[cfg(feature = "verge-dev")]
use std::path::Path;
use std::path::PathBuf;
use tauri_plugin_clash_verge_sysinfo::is_current_app_handle_admin;
use tokio::sync::OnceCell;
use tokio::time::sleep;

async fn is_tun_runtime_available_at_startup() -> bool {
    if is_current_app_handle_admin(Handle::app_handle()) {
        return true;
    }

    let service_config = service::ServiceManager::config();
    let backoff = ConstantBuilder::default()
        .with_delay(service_config.retry_delay)
        .with_max_times(service_config.max_retries);

    (|| async {
        if !service::is_service_ipc_path_exists() {
            return Err(anyhow!("Service IPC not ready"));
        }

        service::is_service_available().await?;
        Ok(())
    })
    .retry(backoff)
    .await
    .is_ok()
}

pub struct Config {
    clash_config: Draft<IClashTemp>,
    verge_config: Draft<IVerge>,
    profiles_config: Draft<IProfiles>,
    runtime_config: Draft<IRuntime>,
}

impl Config {
    pub async fn global() -> &'static Self {
        static CONFIG: OnceCell<Config> = OnceCell::const_new();
        CONFIG
            .get_or_init(|| async {
                Self {
                    clash_config: Draft::new(IClashTemp::new().await),
                    verge_config: Draft::new(IVerge::new().await),
                    profiles_config: Draft::new(IProfiles::new().await),
                    runtime_config: Draft::new(IRuntime::new()),
                }
            })
            .await
    }

    pub async fn clash() -> Draft<IClashTemp> {
        Self::global().await.clash_config.clone()
    }

    pub async fn verge() -> Draft<IVerge> {
        Self::global().await.verge_config.clone()
    }

    pub async fn profiles() -> Draft<IProfiles> {
        Self::global().await.profiles_config.clone()
    }

    pub async fn runtime() -> Draft<IRuntime> {
        Self::global().await.runtime_config.clone()
    }

    /// 初始化订阅
    pub async fn init_config() -> Result<()> {
        #[cfg(feature = "verge-dev")]
        Self::bootstrap_dev_profiles_from_release().await?;

        Self::ensure_default_profile_items().await?;

        let verge = Self::verge().await.latest_arc();
        clash_verge_i18n::sync_locale(verge.language.as_deref());
        let tun_enabled = verge.enable_tun_mode.unwrap_or(false);

        if tun_enabled && !is_tun_runtime_available_at_startup().await {
            logging!(
                warn,
                Type::Core,
                "TUN runtime unavailable during startup, disabling persisted TUN mode"
            );
            let verge = Self::verge().await;
            verge.edit_draft(|d| {
                d.enable_tun_mode = Some(false);
            });
            verge.apply();
            let _ = tray::Tray::global().update_menu().await;

            // 分离数据获取和异步调用避免Send问题
            let verge_data = Self::verge().await.latest_arc();
            logging_error!(Type::Core, verge_data.save_file().await);
        }

        let validation_result = Self::generate_and_validate().await?;

        if let Some((msg_type, msg_content)) = validation_result {
            sleep(timing::STARTUP_ERROR_DELAY).await;
            handle::Handle::notice_message(msg_type, msg_content);
        }

        {
            let profiles = Self::profiles().await.data_arc();
            // Logging error internally
            let _ = profiles.cleanup_orphaned_files().await;
        }

        Ok(())
    }

    // Ensure "Merge" and "Script" profile items exist, adding them if missing.
    async fn ensure_default_profile_items() -> Result<()> {
        let profiles = Self::profiles().await;
        if profiles.latest_arc().get_item("Merge").is_err() {
            let merge_item = &mut PrfItem::from_merge(Some("Merge".into()))?;
            profiles_append_item_safe(merge_item).await?;
        }
        if profiles.latest_arc().get_item("Script").is_err() {
            let script_item = &mut PrfItem::from_script(Some("Script".into()))?;
            profiles_append_item_safe(script_item).await?;
        }
        Ok(())
    }

    #[cfg(feature = "verge-dev")]
    async fn bootstrap_dev_profiles_from_release() -> Result<()> {
        use tokio::fs;

        let dev_profiles_path = dirs::profiles_path()?;
        let current_dev_profiles = match help::read_yaml::<IProfiles>(&dev_profiles_path).await {
            Ok(profiles) => profiles,
            Err(_) => IProfiles::default(),
        };

        if current_dev_profiles.has_primary_profiles() {
            return Ok(());
        }

        let release_home = match dirs::release_app_home_dir() {
            Ok(path) => path,
            Err(err) => {
                logging!(
                    warn,
                    Type::Config,
                    "Skipped dev profile bootstrap because release app directory is unavailable: {err}"
                );
                return Ok(());
            }
        };

        let release_profiles_path = release_home.join(dirs::PROFILE_YAML);
        if !release_profiles_path.exists() {
            return Ok(());
        }

        let release_profiles = match help::read_yaml::<IProfiles>(&release_profiles_path).await {
            Ok(profiles) => profiles,
            Err(err) => {
                logging!(
                    warn,
                    Type::Config,
                    "Skipped dev profile bootstrap because release profiles could not be read: {err}"
                );
                return Ok(());
            }
        };

        if !release_profiles.has_primary_profiles() {
            return Ok(());
        }

        let dev_home = dirs::app_home_dir()?;
        let dev_profiles_dir = dirs::app_profiles_dir()?;
        let release_profiles_dir = release_home.join("profiles");

        fs::create_dir_all(&dev_home).await?;
        fs::create_dir_all(&dev_profiles_dir).await?;

        Self::copy_directory_contents(&release_profiles_dir, &dev_profiles_dir).await?;
        fs::copy(&release_profiles_path, &dev_profiles_path)
            .await
            .map(|_| ())
            .map_err(anyhow::Error::from)?;

        logging!(
            info,
            Type::Config,
            "Bootstrapped dev profiles from release app data after detecting an empty dev subscription list"
        );

        Ok(())
    }

    #[cfg(feature = "verge-dev")]
    async fn copy_directory_contents(source: &Path, target: &Path) -> Result<()> {
        use tokio::fs;

        if !source.exists() {
            return Ok(());
        }

        fs::create_dir_all(target).await?;

        let mut entries = fs::read_dir(source).await?;
        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            if !path.is_file() {
                continue;
            }

            let Some(file_name) = path.file_name() else {
                continue;
            };

            fs::copy(&path, target.join(file_name))
                .await
                .map(|_| ())
                .map_err(anyhow::Error::from)?;
        }

        Ok(())
    }

    async fn generate_and_validate() -> Result<Option<(&'static str, String)>> {
        // 生成运行时配置
        if let Err(err) = Self::generate().await {
            let error_msg: String = err.to_string().into();
            logging!(error, Type::Config, "生成运行时配置失败: {}", error_msg);
            CoreManager::global()
                .use_default_config("config_validate::boot_error", &error_msg)
                .await?;
            return Ok(Some(("config_validate::boot_error", error_msg)));
        }
        logging!(info, Type::Config, "生成运行时配置成功");

        // 生成运行时配置文件并验证
        let config_result = Self::generate_file(ConfigType::Run).await;

        if config_result.is_ok() {
            // 验证配置文件
            logging!(info, Type::Config, "开始验证配置");

            match CoreConfigValidator::global().validate_config_outcome().await {
                Ok(outcome) if outcome.is_valid() => {
                    logging!(info, Type::Config, "配置验证成功");
                    // 前端没有必要知道验证成功的消息，也没有事件驱动
                    // Some(("config_validate::success", String::new()))
                    Ok(None)
                }
                Ok(outcome) => {
                    let error_msg: String = outcome.to_string().into();
                    logging!(
                        warn,
                        Type::Config,
                        "[首次启动] 配置验证未通过，使用默认最小配置启动: {}",
                        error_msg
                    );
                    CoreManager::global()
                        .use_default_config("config_validate::boot_error", &error_msg)
                        .await?;
                    Ok(Some(("config_validate::boot_error", error_msg)))
                }
                Err(err) => {
                    logging!(warn, Type::Config, "验证过程执行失败: {}", err);
                    CoreManager::global()
                        .use_default_config("config_validate::process_terminated", "")
                        .await?;
                    Ok(Some(("config_validate::process_terminated", String::new())))
                }
            }
        } else {
            logging!(warn, Type::Config, "生成配置文件失败，使用默认配置");
            CoreManager::global()
                .use_default_config("config_validate::error", "")
                .await?;
            Ok(Some(("config_validate::error", String::new())))
        }
    }

    pub async fn generate_file(typ: ConfigType) -> Result<PathBuf> {
        let path = match typ {
            ConfigType::Run => dirs::app_home_dir()?.join(files::RUNTIME_CONFIG),
            ConfigType::Check => dirs::app_home_dir()?.join(files::CHECK_CONFIG),
        };

        let runtime = Self::runtime().await;
        let runtime_lastest = runtime.latest_arc();
        // Fall back to committed config if runtime config is missing
        let runtime_data = runtime.data_arc();
        let config = runtime_lastest
            .config
            .as_ref()
            .or_else(|| runtime_data.config.as_ref())
            .ok_or_else(|| anyhow!("failed to generate runtime config, might need to restart application"))?;

        help::save_yaml(&path, config, Some("# Generated by Clash Verge")).await?;
        Ok(path)
    }

    pub async fn generate() -> Result<()> {
        let (mut config, exists_keys, logs) = enhance::enhance().await?;

        strip_tunnel_proxy_overrides(&mut config);

        Self::runtime().await.edit_draft(|d| {
            *d = IRuntime {
                config: Some(config),
                exists_keys,
                chain_logs: logs,
            }
        });

        Ok(())
    }

    pub async fn verify_config_initialization() {
        let backoff = ExponentialBuilder::default()
            .with_min_delay(std::time::Duration::from_millis(100))
            .with_max_delay(std::time::Duration::from_secs(2))
            .with_factor(2.0)
            .with_max_times(10);

        if let Err(e) = (|| async {
            if Self::runtime().await.latest_arc().config.is_some() {
                return Ok::<(), anyhow::Error>(());
            }
            Self::generate().await
        })
        .retry(backoff)
        .await
        {
            logging!(error, Type::Setup, "Config init verification failed: {}", e);
        }
    }

    // 升级草稿为正式数据，并写入文件。避免用户行为丢失。
    // 仅在应用退出、重启、关机监听事件启用
    pub async fn apply_all_and_save_file() {
        logging!(info, Type::Config, "save all draft data");
        let save_clash_task = AsyncHandler::spawn(|| async {
            let clash = Self::clash().await;
            clash.apply();
            logging_error!(Type::Config, clash.data_arc().save_config().await);
        });

        let save_verge_task = AsyncHandler::spawn(|| async {
            let verge = Self::verge().await;
            verge.apply();
            logging_error!(Type::Config, verge.data_arc().save_file().await);
        });

        let save_profiles_task = AsyncHandler::spawn(|| async {
            let profiles = Self::profiles().await;
            profiles.apply();
            logging_error!(Type::Config, profiles.data_arc().save_file().await);
        });

        let _ = tokio::join!(save_clash_task, save_verge_task, save_profiles_task);
        logging!(info, Type::Config, "save all draft data finished");
    }
}

fn strip_tunnel_proxy_overrides(config: &mut Mapping) {
    let Some(tunnels) = config.get_mut("tunnels").and_then(|v| v.as_sequence_mut()) else {
        return;
    };

    for item in tunnels {
        let Some(tunnel) = item.as_mapping_mut() else { continue };
        tunnel.remove("proxy");
    }
}

#[derive(Debug)]
pub enum ConfigType {
    Run,
    Check,
}
#[cfg(test)]
mod tests {
    use super::*;
    use std::mem;

    #[test]
    #[allow(unused_variables)]
    #[allow(clippy::expect_used)]
    fn test_prfitem_from_merge_size() {
        let merge_item = PrfItem::from_merge(Some("Merge".into())).expect("Failed to create merge item in test");
        let prfitem_size = mem::size_of_val(&merge_item);
        // Boxed version
        let boxed_merge_item = Box::new(merge_item);
        let box_prfitem_size = mem::size_of_val(&boxed_merge_item);
        // The size of Box<T> is always pointer-sized (usually 8 bytes on 64-bit)
        // assert_eq!(box_prfitem_size, mem::size_of::<Box<PrfItem>>());
        assert!(box_prfitem_size < prfitem_size);
    }

    #[test]
    #[allow(unused_variables)]
    fn test_draft_size_non_boxed() {
        let draft = Draft::new(IRuntime::new());
        let iruntime_size = std::mem::size_of_val(&draft);
        assert_eq!(iruntime_size, std::mem::size_of::<Draft<IRuntime>>());
    }

    #[test]
    #[allow(unused_variables)]
    fn test_draft_size_boxed() {
        let draft = Draft::new(Box::new(IRuntime::new()));
        let box_iruntime_size = std::mem::size_of_val(&draft);
        assert_eq!(box_iruntime_size, std::mem::size_of::<Draft<Box<IRuntime>>>());
    }
}
