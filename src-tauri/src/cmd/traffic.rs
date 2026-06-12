use super::{CmdResult, StringifyErr as _};
/**
 * 流量功能 Tauri 命令
 */
use crate::traffic::{ObfuscationProfile, ObfuscationStats, TrafficObfuscationConfig};

/// 应用混淆配置（供内部调用，委托 feat 层）
pub async fn apply_traffic_obfuscation_config(config: TrafficObfuscationConfig) -> CmdResult<()> {
    crate::core::traffic_runtime::apply_traffic_obfuscation_config(config)
        .await
        .stringify_err()
}

/// 获取流量混淆配置
#[tauri::command]
pub async fn traffic_obfuscation_get_config() -> CmdResult<TrafficObfuscationConfig> {
    Ok(crate::core::traffic_runtime::traffic_obfuscation_get_config().await)
}

/// 更新流量混淆配置
#[tauri::command]
pub async fn traffic_obfuscation_update_config(config: TrafficObfuscationConfig) -> CmdResult<()> {
    crate::core::traffic_runtime::traffic_obfuscation_update_config(config)
        .await
        .stringify_err()
}

/// 启动流量混淆
#[tauri::command]
pub async fn traffic_obfuscation_start() -> CmdResult<()> {
    crate::core::traffic_runtime::traffic_obfuscation_start()
        .await
        .stringify_err()
}

/// 停止流量混淆
#[tauri::command]
pub async fn traffic_obfuscation_stop() -> CmdResult<()> {
    crate::core::traffic_runtime::traffic_obfuscation_stop()
        .await
        .stringify_err()
}

/// 获取流量混淆统计
#[tauri::command]
pub async fn traffic_obfuscation_get_stats() -> CmdResult<ObfuscationStats> {
    Ok(crate::core::traffic_runtime::traffic_obfuscation_get_stats().await)
}

/// 重置流量混淆统计
#[tauri::command]
pub async fn traffic_obfuscation_reset_stats() -> CmdResult<()> {
    crate::core::traffic_runtime::traffic_obfuscation_reset_stats()
        .await
        .stringify_err()
}

/// 检查流量混淆是否正在运行
#[tauri::command]
pub async fn traffic_obfuscation_is_running() -> CmdResult<bool> {
    Ok(crate::core::traffic_runtime::traffic_obfuscation_is_running().await)
}

/// 应用预设 Profile，返回生成的配置
#[tauri::command]
pub async fn traffic_obfuscation_apply_profile(profile: ObfuscationProfile) -> CmdResult<TrafficObfuscationConfig> {
    crate::core::traffic_runtime::traffic_obfuscation_apply_profile(profile)
        .await
        .stringify_err()
}
