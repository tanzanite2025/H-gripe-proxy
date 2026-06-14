use crate::core::{CoreManager, handle, manager::RunningMode};
use anyhow::Result;
use async_trait::async_trait;
use clash_verge_logging::{Type, logging};
use once_cell::sync::OnceCell;
#[cfg(unix)]
use std::iter;
use std::{
    fs,
    path::{Path, PathBuf},
};
use tauri::Manager as _;

#[cfg(not(feature = "verge-dev"))]
pub static APP_ID: &str = "io.github.tanzanite2025.clash-verge-optimized";
#[cfg(not(feature = "verge-dev"))]
pub static LEGACY_APP_IDS: &[&str] = &["io.github.clash-verge-rev.clash-verge-rev"];
#[cfg(not(feature = "verge-dev"))]
pub static BACKUP_DIR: &str = "clash-verge-optimized-backup";
#[cfg(not(feature = "verge-dev"))]
pub static LEGACY_BACKUP_DIRS: &[&str] = &["clash-verge-rev-backup"];

#[cfg(feature = "verge-dev")]
pub static APP_ID: &str = "io.github.tanzanite2025.clash-verge-optimized.dev";
#[cfg(feature = "verge-dev")]
pub static LEGACY_APP_IDS: &[&str] = &["io.github.clash-verge-rev.clash-verge-rev.dev"];
#[cfg(feature = "verge-dev")]
pub static BACKUP_DIR: &str = "clash-verge-optimized-backup-dev";
#[cfg(feature = "verge-dev")]
pub static LEGACY_BACKUP_DIRS: &[&str] = &["clash-verge-rev-backup-dev"];
#[cfg(feature = "verge-dev")]
pub static RELEASE_APP_ID: &str = "io.github.tanzanite2025.clash-verge-optimized";
#[cfg(feature = "verge-dev")]
pub static RELEASE_LEGACY_APP_IDS: &[&str] = &["io.github.clash-verge-rev.clash-verge-rev"];

pub static PORTABLE_FLAG: OnceCell<bool> = OnceCell::new();

pub static CLASH_CONFIG: &str = "config.yaml";
pub static VERGE_CONFIG: &str = "verge.yaml";
pub static PROFILE_YAML: &str = "profiles.yaml";
pub static CHINA_RULES_CONFIG: &str = "china-rules.yaml";
pub static SUBSCRIPTIONS_DIR: &str = "subscriptions";
pub static SUBSCRIPTION_STATE_FILE: &str = "state.yaml";

fn migrate_dir_if_needed(from: &Path, to: &Path, label: &str) -> Result<()> {
    if to.exists() || !from.exists() {
        return Ok(());
    }

    if let Some(parent) = to.parent() {
        fs::create_dir_all(parent)?;
    }

    fs::rename(from, to)?;
    logging!(info, Type::File, "Migrated {label}: {:?} -> {:?}", from, to);
    Ok(())
}

fn migrate_from_legacy_candidates(base_dir: &Path, current_name: &str, legacy_names: &[&str], label: &str) -> PathBuf {
    let current_dir = base_dir.join(current_name);
    if current_dir.exists() {
        return current_dir;
    }

    for legacy_name in legacy_names {
        let legacy_dir = base_dir.join(legacy_name);
        if let Err(e) = migrate_dir_if_needed(&legacy_dir, &current_dir, label) {
            logging!(warn, Type::File, "Failed to migrate legacy {label}: {e}");
        }
        if current_dir.exists() {
            break;
        }
    }

    current_dir
}

/// init portable flag
pub fn init_portable_flag() -> Result<()> {
    use tauri::utils::platform::current_exe;

    let app_exe = current_exe()?;
    if let Some(dir) = app_exe.parent() {
        let dir = PathBuf::from(dir).join(".config/PORTABLE");

        if dir.exists() {
            PORTABLE_FLAG.get_or_init(|| true);
        }
    }
    PORTABLE_FLAG.get_or_init(|| false);
    Ok(())
}

/// get the verge app home dir
pub fn app_home_dir() -> Result<PathBuf> {
    app_home_dir_with_ids(APP_ID, LEGACY_APP_IDS)
}

#[cfg(feature = "verge-dev")]
pub fn release_app_home_dir() -> Result<PathBuf> {
    app_home_dir_with_ids(RELEASE_APP_ID, RELEASE_LEGACY_APP_IDS)
}

fn app_home_dir_with_ids(app_id: &str, legacy_app_ids: &[&str]) -> Result<PathBuf> {
    use tauri::utils::platform::current_exe;

    let flag = PORTABLE_FLAG.get().unwrap_or(&false);
    if *flag {
        let app_exe = current_exe()?;
        let app_exe = dunce::canonicalize(app_exe)?;
        let app_dir = app_exe
            .parent()
            .ok_or_else(|| anyhow::anyhow!("failed to get the portable app dir"))?;
        let config_dir = PathBuf::from(app_dir).join(".config");
        return Ok(migrate_from_legacy_candidates(
            &config_dir,
            app_id,
            legacy_app_ids,
            "app home directory",
        ));
    }

    // 避免在Handle未初始化时崩溃
    let app_handle = handle::Handle::app_handle();

    match app_handle.path().data_dir() {
        Ok(dir) => Ok(migrate_from_legacy_candidates(
            &dir,
            app_id,
            legacy_app_ids,
            "app home directory",
        )),
        Err(e) => {
            logging!(error, Type::File, "Failed to get the app home directory: {e}");
            Err(anyhow::anyhow!("Failed to get the app homedirectory"))
        }
    }
}

/// get the resources dir
pub fn app_resources_dir() -> Result<PathBuf> {
    // 避免在Handle未初始化时崩溃
    let app_handle = handle::Handle::app_handle();

    match app_handle.path().resource_dir() {
        Ok(dir) => Ok(dir.join("resources")),
        Err(e) => {
            logging!(error, Type::File, "Failed to get the resource directory: {e}");
            Err(anyhow::anyhow!("Failed to get the resource directory"))
        }
    }
}

/// profiles dir
pub fn app_profiles_dir() -> Result<PathBuf> {
    Ok(app_home_dir()?.join("profiles"))
}

/// icons dir
pub fn app_icons_dir() -> Result<PathBuf> {
    Ok(app_home_dir()?.join("icons"))
}

pub fn find_target_icons(target: &str) -> Result<Option<String>> {
    let icons_dir = app_icons_dir()?;
    if !icons_dir.exists() {
        let _ = std::fs::create_dir_all(&icons_dir);
        return Ok(None);
    }
    let icon_path = fs::read_dir(&icons_dir)?
        .filter_map(|entry| entry.ok().map(|e| e.path()))
        .find(|path| {
            let prefix_matches = path
                .file_prefix()
                .and_then(|p| p.to_str())
                .is_some_and(|prefix| prefix.starts_with(target));
            let ext_matches = path
                .extension()
                .and_then(|e| e.to_str())
                .is_some_and(|ext| ext.eq_ignore_ascii_case("ico") || ext.eq_ignore_ascii_case("png"));
            prefix_matches && ext_matches
        });

    icon_path.map(|path| path_to_str(&path).map(|s| s.into())).transpose()
}

/// logs dir
pub fn app_logs_dir() -> Result<PathBuf> {
    Ok(app_home_dir()?.join("logs"))
}

// latest verge log
pub fn app_latest_log() -> Result<PathBuf> {
    Ok(app_logs_dir()?.join("latest.log"))
}

/// local backups dir
pub fn local_backup_dir() -> Result<PathBuf> {
    let app_dir = app_home_dir()?;
    let dir = app_dir.join(BACKUP_DIR);
    for legacy_backup_dir in LEGACY_BACKUP_DIRS {
        let legacy_dir = app_dir.join(legacy_backup_dir);
        if let Err(e) = migrate_dir_if_needed(&legacy_dir, &dir, "backup directory") {
            logging!(warn, Type::File, "Failed to migrate legacy backup directory: {e}");
        }
        if dir.exists() {
            break;
        }
    }
    fs::create_dir_all(&dir)?;
    Ok(dir)
}

pub fn clash_path() -> Result<PathBuf> {
    Ok(app_home_dir()?.join(CLASH_CONFIG))
}

pub fn verge_path() -> Result<PathBuf> {
    Ok(app_home_dir()?.join(VERGE_CONFIG))
}

pub fn profiles_path() -> Result<PathBuf> {
    Ok(app_home_dir()?.join(PROFILE_YAML))
}

pub fn china_rules_path() -> Result<PathBuf> {
    Ok(app_home_dir()?.join(CHINA_RULES_CONFIG))
}

pub fn subscriptions_dir() -> Result<PathBuf> {
    Ok(app_home_dir()?.join(SUBSCRIPTIONS_DIR))
}

pub fn subscription_state_path() -> Result<PathBuf> {
    Ok(subscriptions_dir()?.join(SUBSCRIPTION_STATE_FILE))
}

pub fn subscription_artifacts_dir(source_id: &str) -> Result<PathBuf> {
    Ok(subscriptions_dir()?.join("artifacts").join(source_id))
}

pub fn subscription_artifact_version_dir(source_id: &str, version: &str) -> Result<PathBuf> {
    Ok(subscription_artifacts_dir(source_id)?.join(version))
}

#[cfg(target_os = "macos")]
pub fn service_path() -> Result<PathBuf> {
    let res_dir = app_resources_dir()?;
    Ok(res_dir.join("clash-verge-service"))
}

#[cfg(windows)]
pub fn service_path() -> Result<PathBuf> {
    let res_dir = app_resources_dir()?;
    Ok(res_dir.join("clash-verge-service.exe"))
}

pub fn sidecar_log_dir() -> Result<PathBuf> {
    let log_dir = app_logs_dir()?.join("sidecar");
    let _ = std::fs::create_dir_all(&log_dir);

    Ok(log_dir)
}

pub fn service_log_dir() -> Result<PathBuf> {
    let log_dir = app_logs_dir()?.join("service");
    let _ = std::fs::create_dir_all(&log_dir);

    Ok(log_dir)
}

pub fn clash_latest_log() -> Result<PathBuf> {
    match *CoreManager::global().get_running_mode() {
        RunningMode::Service => Ok(service_log_dir()?.join("service_latest.log")),
        RunningMode::Sidecar | RunningMode::NotRunning => Ok(sidecar_log_dir()?.join("sidecar_latest.log")),
    }
}

pub fn path_to_str(path: &PathBuf) -> Result<&str> {
    let path_str = path
        .as_os_str()
        .to_str()
        .ok_or_else(|| anyhow::anyhow!("failed to get path from {:?}", path))?;
    Ok(path_str)
}

pub fn get_encryption_key() -> Result<Vec<u8>> {
    let app_dir = app_home_dir()?;
    let key_path = app_dir.join(".encryption_key");

    if key_path.exists() {
        // Read existing key
        fs::read(&key_path).map_err(|e| anyhow::anyhow!("Failed to read encryption key: {}", e))
    } else {
        // Generate and save new key
        let mut key = vec![0u8; 32];
        getrandom::fill(&mut key)?;

        // Ensure directory exists
        if let Some(parent) = key_path.parent() {
            fs::create_dir_all(parent).map_err(|e| anyhow::anyhow!("Failed to create key directory: {}", e))?;
        }
        // Save key
        fs::write(&key_path, &key).map_err(|e| anyhow::anyhow!("Failed to save encryption key: {}", e))?;
        Ok(key)
    }
}

#[cfg(unix)]
pub fn ensure_mihomo_safe_dir() -> Option<PathBuf> {
    iter::once("/tmp")
        .map(PathBuf::from)
        .find(|path| path.exists())
        .or_else(|| {
            std::env::var_os("HOME").and_then(|home| {
                let home_config = PathBuf::from(home).join(".config");
                if home_config.exists() || fs::create_dir_all(&home_config).is_ok() {
                    Some(home_config)
                } else {
                    logging!(error, Type::File, "Failed to create safe directory: {home_config:?}");
                    None
                }
            })
        })
}

#[cfg(unix)]
pub fn ipc_path() -> Result<PathBuf> {
    ensure_mihomo_safe_dir()
        .map(|base_dir| base_dir.join("verge").join("verge-mihomo.sock"))
        .or_else(|| {
            app_home_dir()
                .ok()
                .map(|dir| dir.join("verge").join("verge-mihomo.sock"))
        })
        .ok_or_else(|| anyhow::anyhow!("Failed to determine ipc path"))
}

#[cfg(all(target_os = "windows", feature = "verge-dev"))]
pub fn ipc_path() -> Result<PathBuf> {
    Ok(PathBuf::from(r"\\.\pipe\verge-mihomo-dev"))
}

#[cfg(all(target_os = "windows", not(feature = "verge-dev")))]
pub fn ipc_path() -> Result<PathBuf> {
    Ok(PathBuf::from(r"\\.\pipe\verge-mihomo"))
}
#[async_trait]
pub trait PathBufExec {
    async fn remove_if_exists(&self) -> Result<()>;
}

#[async_trait]
impl PathBufExec for PathBuf {
    async fn remove_if_exists(&self) -> Result<()> {
        if self.exists() {
            tokio::fs::remove_file(self).await?;
            logging!(info, Type::File, "Removed file: {:?}", self);
        }
        Ok(())
    }
}
