use super::{CmdResult, StringifyErr as _};
/**
 * 流量功能 Tauri 命令
 */
use crate::{
    core::{connection_metrics::ConnectionMetricsSnapshot, handle::Handle},
    traffic::{ObfuscationProfile, ObfuscationStats, TrafficObfuscationConfig},
};
use tauri_plugin_mihomo::models::Connections;

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

/// 获取 Rust 聚合的连接/流量指标快照。
#[tauri::command]
pub async fn traffic_get_connection_metrics_snapshot() -> CmdResult<ConnectionMetricsSnapshot> {
    crate::core::connection_metrics::refresh_connection_metrics_snapshot()
        .await
        .stringify_err()
}

#[tauri::command]
pub async fn get_runtime_connections() -> CmdResult<Connections> {
    let payload = crate::core::runtime_snapshot::read_runtime_connections()
        .await
        .stringify_err()?;
    crate::core::connection_metrics::ingest_connection_metrics_snapshot(&payload).await;
    Ok(payload)
}

#[tauri::command]
pub async fn close_runtime_connection(connection_id: String) -> CmdResult<()> {
    Handle::mihomo()
        .await
        .close_connection(&connection_id)
        .await
        .stringify_err()
}

#[tauri::command]
pub async fn close_all_runtime_connections() -> CmdResult<()> {
    Err("close_all_runtime_connections through the Go/Mihomo plugin API is retired; use the Rust runtime connection path".into())
}

/// 重置 Rust 连接/流量指标聚合状态。
#[tauri::command]
pub async fn traffic_reset_connection_metrics() -> CmdResult<()> {
    crate::core::connection_metrics::reset_connection_metrics().await;
    Ok(())
}

/// 启动 Rust 连接监控后台任务。
#[tauri::command]
pub async fn connection_monitor_start() -> CmdResult<()> {
    crate::core::connection_monitor::global().start();
    Ok(())
}

/// 停止 Rust 连接监控后台任务。
#[tauri::command]
pub async fn connection_monitor_stop() -> CmdResult<()> {
    crate::core::connection_monitor::global().stop();
    Ok(())
}

/// 查询连接监控是否运行中。
#[tauri::command]
pub async fn connection_monitor_is_running() -> CmdResult<bool> {
    Ok(crate::core::connection_monitor::global().is_running())
}
