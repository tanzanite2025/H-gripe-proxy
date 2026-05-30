use crate::core::egress_monitor::{EgressMonitorConfig, EgressMonitorStats, EgressIpProbeResult};
use super::{CmdResult, StringifyErr};

/// 获取出口监控配置
#[tauri::command]
pub async fn egress_monitor_get_config() -> CmdResult<EgressMonitorConfig> {
    Ok(crate::feat::egress_monitor_get_config().await)
}

/// 更新出口监控配置
#[tauri::command]
pub async fn egress_monitor_update_config(config: EgressMonitorConfig) -> CmdResult<()> {
    crate::feat::egress_monitor_update_config(config).await.stringify_err()
}

/// 启动出口监控
#[tauri::command]
pub async fn egress_monitor_start() -> CmdResult<()> {
    crate::feat::egress_monitor_start().await;
    Ok(())
}

/// 停止出口监控
#[tauri::command]
pub async fn egress_monitor_stop() -> CmdResult<()> {
    crate::feat::egress_monitor_stop().await;
    Ok(())
}

/// 获取出口监控统计
#[tauri::command]
pub async fn egress_monitor_get_stats() -> CmdResult<EgressMonitorStats> {
    Ok(crate::feat::egress_monitor_get_stats().await)
}

/// 重置出口监控统计
#[tauri::command]
pub async fn egress_monitor_reset_stats() -> CmdResult<()> {
    crate::feat::egress_monitor_reset_stats().await;
    Ok(())
}

/// 手动探测出口 IP
#[tauri::command]
pub async fn egress_monitor_probe_now() -> CmdResult<EgressIpProbeResult> {
    crate::feat::egress_monitor_probe_now().await.stringify_err()
}

/// 查询出口监控是否运行中
#[tauri::command]
pub async fn egress_monitor_is_running() -> CmdResult<bool> {
    Ok(crate::feat::egress_monitor_is_running().await)
}
