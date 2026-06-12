use super::{CmdResult, StringifyErr};
use crate::core::egress_monitor::{EgressIpProbeResult, EgressMonitorConfig, EgressMonitorStats};

/// 获取出口监控配置
#[tauri::command]
pub async fn egress_monitor_get_config() -> CmdResult<EgressMonitorConfig> {
    Ok(crate::core::coordinator::get_coordinator()
        .get_advanced_config()
        .egress_monitor)
}

/// 更新出口监控配置
#[tauri::command]
pub async fn egress_monitor_update_config(config: EgressMonitorConfig) -> CmdResult<()> {
    crate::core::coordinator::update_advanced_config(move |advanced| {
        advanced.egress_monitor = config;
    })
    .await
    .stringify_err()
}

/// 启动出口监控
#[tauri::command]
pub async fn egress_monitor_start() -> CmdResult<()> {
    crate::core::coordinator::update_advanced_config(|advanced| {
        advanced.egress_monitor.enabled = true;
    })
        .await
        .stringify_err()
}

/// 停止出口监控
#[tauri::command]
pub async fn egress_monitor_stop() -> CmdResult<()> {
    crate::core::coordinator::update_advanced_config(|advanced| {
        advanced.egress_monitor.enabled = false;
    })
        .await
        .stringify_err()
}

/// 获取出口监控统计
#[tauri::command]
pub async fn egress_monitor_get_stats() -> CmdResult<EgressMonitorStats> {
    Ok(crate::core::egress_monitor::egress_monitor().get_stats())
}

/// 重置出口监控统计
#[tauri::command]
pub async fn egress_monitor_reset_stats() -> CmdResult<()> {
    crate::core::egress_monitor::egress_monitor().reset_stats();
    Ok(())
}

/// 手动探测出口 IP
#[tauri::command]
pub async fn egress_monitor_probe_now() -> CmdResult<EgressIpProbeResult> {
    crate::core::egress_monitor::egress_monitor()
        .probe_now()
        .await
        .stringify_err()
}

/// 查询出口监控是否运行中
#[tauri::command]
pub async fn egress_monitor_is_running() -> CmdResult<bool> {
    Ok(crate::core::egress_monitor::egress_monitor().is_running())
}
