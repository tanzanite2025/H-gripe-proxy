use crate::core::egress_monitor::{
    egress_monitor, EgressMonitorConfig, EgressMonitorStats, EgressIpProbeResult,
};

/// 获取出口监控配置
#[tauri::command]
pub async fn egress_monitor_get_config() -> Result<EgressMonitorConfig, String> {
    Ok(egress_monitor().get_config())
}

/// 更新出口监控配置
#[tauri::command]
pub async fn egress_monitor_update_config(
    config: EgressMonitorConfig,
) -> Result<(), String> {
    egress_monitor()
        .update_config(config)
        .map_err(|e| e.to_string())
}

/// 启动出口监控
#[tauri::command]
pub async fn egress_monitor_start() -> Result<(), String> {
    egress_monitor().start();
    Ok(())
}

/// 停止出口监控
#[tauri::command]
pub async fn egress_monitor_stop() -> Result<(), String> {
    egress_monitor().stop();
    Ok(())
}

/// 获取出口监控统计
#[tauri::command]
pub async fn egress_monitor_get_stats() -> Result<EgressMonitorStats, String> {
    Ok(egress_monitor().get_stats())
}

/// 重置出口监控统计
#[tauri::command]
pub async fn egress_monitor_reset_stats() -> Result<(), String> {
    egress_monitor().reset_stats();
    Ok(())
}

/// 手动探测出口 IP
#[tauri::command]
pub async fn egress_monitor_probe_now() -> Result<EgressIpProbeResult, String> {
    egress_monitor()
        .probe_now()
        .await
        .map_err(|e| e.to_string())
}

/// 查询出口监控是否运行中
#[tauri::command]
pub async fn egress_monitor_is_running() -> Result<bool, String> {
    Ok(egress_monitor().is_running())
}
