/**
 * XDP 代理 Tauri 命令
 */

use crate::xdp::{
    get_xdp_manager, XdpAction, XdpConfig, XdpRoute, XdpStatus, XdpSupportInfo,
};

/// 获取 XDP 配置
#[tauri::command]
pub fn xdp_get_config() -> Result<XdpConfig, String> {
    let manager = get_xdp_manager();
    Ok(manager.get_config())
}

/// 更新 XDP 配置
#[tauri::command]
pub fn xdp_update_config(config: XdpConfig) -> Result<(), String> {
    let manager = get_xdp_manager();
    manager.update_config(config);
    Ok(())
}

/// 获取 XDP 状态
#[tauri::command]
pub fn xdp_get_status() -> Result<XdpStatus, String> {
    let manager = get_xdp_manager();
    Ok(manager.get_status())
}

/// 启动 XDP 代理
#[tauri::command]
pub fn xdp_start() -> Result<(), String> {
    let manager = get_xdp_manager();
    manager.start()
}

/// 停止 XDP 代理
#[tauri::command]
pub fn xdp_stop() -> Result<(), String> {
    let manager = get_xdp_manager();
    manager.stop()
}

/// 添加路由规则
#[tauri::command]
pub fn xdp_add_route(route: XdpRoute) -> Result<(), String> {
    let manager = get_xdp_manager();
    manager.add_route(route)
}

/// 删除路由规则
#[tauri::command]
pub fn xdp_remove_route(dest_ip: String) -> Result<(), String> {
    let manager = get_xdp_manager();
    manager.remove_route(&dest_ip)
}

/// 更新统计信息
#[tauri::command]
pub fn xdp_update_stats() -> Result<(), String> {
    let manager = get_xdp_manager();
    manager.update_stats()
}

/// 检查系统支持
#[tauri::command]
pub fn xdp_check_support() -> Result<XdpSupportInfo, String> {
    crate::xdp::XdpManager::check_support()
}

/// 获取可用网卡列表
#[tauri::command]
pub fn xdp_get_interfaces() -> Result<Vec<String>, String> {
    #[cfg(target_os = "linux")]
    {
        use std::fs;
        
        let mut interfaces = Vec::new();
        
        if let Ok(entries) = fs::read_dir("/sys/class/net") {
            for entry in entries.flatten() {
                if let Some(name) = entry.file_name().to_str() {
                    // 排除回环接口
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
