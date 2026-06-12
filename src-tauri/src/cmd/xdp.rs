use super::{CmdResult, StringifyErr};
/**
 * XDP 代理 Tauri 命令
 */
use crate::xdp::{XdpConfig, XdpRoute, XdpStatus, XdpSupportInfo};

/// 获取 XDP 配置
#[tauri::command]
pub fn xdp_get_config() -> CmdResult<XdpConfig> {
    Ok(crate::core::coordinator::get_coordinator().get_advanced_config().xdp)
}

/// 更新 XDP 配置
#[tauri::command]
pub fn xdp_update_config(config: XdpConfig) -> CmdResult<()> {
    crate::core::coordinator::update_advanced_config_blocking(move |advanced| {
        advanced.xdp = config;
    })
    .stringify_err()
}

/// 获取 XDP 状态
#[tauri::command]
pub fn xdp_get_status() -> CmdResult<XdpStatus> {
    Ok(crate::core::coordinator::get_coordinator().xdp_manager().get_status())
}

/// 启动 XDP 代理
#[tauri::command]
pub fn xdp_start() -> CmdResult<()> {
    crate::core::coordinator::update_advanced_config_blocking(|advanced| {
        advanced.xdp.enabled = true;
    })
    .stringify_err()
}

/// 停止 XDP 代理
#[tauri::command]
pub fn xdp_stop() -> CmdResult<()> {
    crate::core::coordinator::update_advanced_config_blocking(|advanced| {
        advanced.xdp.enabled = false;
    })
    .stringify_err()
}

/// 添加路由规则
#[tauri::command]
pub fn xdp_add_route(route: XdpRoute) -> CmdResult<()> {
    crate::core::coordinator::get_coordinator()
        .xdp_manager()
        .add_route(route)
        .stringify_err()
}

/// 删除路由规则
#[tauri::command]
pub fn xdp_remove_route(dest_ip: String) -> CmdResult<()> {
    crate::core::coordinator::get_coordinator()
        .xdp_manager()
        .remove_route(&dest_ip)
        .stringify_err()
}

/// 更新统计信息
#[tauri::command]
pub fn xdp_update_stats() -> CmdResult<()> {
    crate::core::coordinator::get_coordinator()
        .xdp_manager()
        .update_stats()
        .stringify_err()
}

/// 检查系统支持
#[tauri::command]
pub fn xdp_check_support() -> CmdResult<XdpSupportInfo> {
    crate::xdp::XdpManager::check_support().stringify_err()
}

/// 获取可用网卡列表
#[tauri::command]
pub fn xdp_get_interfaces() -> CmdResult<Vec<String>> {
    #[cfg(target_os = "linux")]
    {
        use std::fs;

        let mut interfaces = Vec::new();

        if let Ok(entries) = fs::read_dir("/sys/class/net") {
            for entry in entries.flatten() {
                if let Some(name) = entry.file_name().to_str() {
                    if name != "lo" {
                        interfaces.push(name.to_string());
                    }
                }
            }
        }

        Ok(interfaces)
    }

    #[cfg(not(target_os = "linux"))]
    {
        Err("XDP 仅支持 Linux 系统".to_string())
    }
}
