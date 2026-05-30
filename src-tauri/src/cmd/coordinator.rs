/**
 * 核心协调器 Tauri 命令
 */

use crate::config::AdvancedConfig;
use crate::core::coordinator_status::CoordinatorStatus;
use super::{CmdResult, StringifyErr};

/// 初始化协调器
#[tauri::command]
pub fn coordinator_initialize() -> CmdResult<()> {
    crate::feat::get_coordinator().initialize().stringify_err()
}

/// 关闭协调器
#[tauri::command]
pub fn coordinator_shutdown() -> CmdResult<()> {
    crate::feat::get_coordinator().shutdown().stringify_err()
}

/// 获取高级配置（从 coordinator 内存缓存读取）
#[tauri::command]
pub fn get_advanced_config() -> CmdResult<AdvancedConfig> {
    Ok(crate::feat::get_coordinator().get_advanced_config())
}

/// 保存高级配置
#[tauri::command]
pub async fn save_advanced_config(config: AdvancedConfig) -> CmdResult<()> {
    crate::feat::save_advanced_config(&config).await.stringify_err()
}

/// 获取推荐配置
#[tauri::command]
pub fn get_recommended_advanced_config() -> CmdResult<AdvancedConfig> {
    Ok(AdvancedConfig::recommended())
}

/// 验证高级配置
#[tauri::command]
pub fn validate_advanced_config(config: AdvancedConfig) -> CmdResult<()> {
    config.validate().stringify_err()
}

/// 获取协调器状态
#[tauri::command]
pub async fn coordinator_get_status() -> CmdResult<CoordinatorStatus> {
    crate::feat::coordinator_get_status().await.stringify_err()
}
