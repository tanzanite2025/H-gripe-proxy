use anyhow::{Result, anyhow};
use std::path::PathBuf;

use crate::utils::dirs;

async fn open_path(path: PathBuf) -> Result<()> {
    open::that(path)?;
    Ok(())
}

async fn open_log_path(path: PathBuf) -> Result<()> {
    #[cfg(target_os = "windows")]
    let path = crate::utils::help::snapshot_path(path.as_path())?;

    open::that(path)?;
    Ok(())
}

pub async fn open_app_dir() -> Result<()> {
    open_path(dirs::app_home_dir()?).await
}

pub async fn open_core_dir() -> Result<()> {
    let core_dir = tauri::utils::platform::current_exe()?;
    let core_dir = core_dir.parent().ok_or_else(|| anyhow!("failed to get core dir"))?;
    open::that(core_dir)?;
    Ok(())
}

pub async fn open_logs_dir() -> Result<()> {
    open_path(dirs::app_logs_dir()?).await
}

pub async fn open_app_log() -> Result<()> {
    open_log_path(dirs::app_latest_log()?).await
}

pub async fn open_core_log() -> Result<()> {
    open_log_path(dirs::clash_latest_log()?).await
}
