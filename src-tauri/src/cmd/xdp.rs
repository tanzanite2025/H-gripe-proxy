use super::{CmdResult, StringifyErr};
/**
 * XDP 代理 Tauri 命令
 */
use crate::xdp::{XdpConfig, XdpRoute, XdpStatus, XdpSupportInfo};

/// 获取 XDP 配置
#[tauri::command]
pub fn xdp_get_config() -> CmdResult<XdpConfig> {
    Ok(crate::feat::xdp_get_config())
}

/// 更新 XDP 配置
#[tauri::command]
pub fn xdp_update_config(config: XdpConfig) -> CmdResult<()> {
    crate::feat::xdp_update_config(config).stringify_err()
}

/// 获取 XDP 状态
#[tauri::command]
pub fn xdp_get_status() -> CmdResult<XdpStatus> {
    Ok(crate::feat::xdp_get_status())
}

/// 启动 XDP 代理
#[tauri::command]
pub fn xdp_start() -> CmdResult<()> {
    crate::feat::xdp_start().stringify_err()
}

/// 停止 XDP 代理
#[tauri::command]
pub fn xdp_stop() -> CmdResult<()> {
    crate::feat::xdp_stop().stringify_err()
}

/// 添加路由规则
#[tauri::command]
pub fn xdp_add_route(route: XdpRoute) -> CmdResult<()> {
    crate::feat::xdp_add_route(route).stringify_err()
}

/// 删除路由规则
#[tauri::command]
pub fn xdp_remove_route(dest_ip: String) -> CmdResult<()> {
    crate::feat::xdp_remove_route(&dest_ip).stringify_err()
}

/// 更新统计信息
#[tauri::command]
pub fn xdp_update_stats() -> CmdResult<()> {
    crate::feat::xdp_update_stats().stringify_err()
}

/// 检查系统支持
#[tauri::command]
pub fn xdp_check_support() -> CmdResult<XdpSupportInfo> {
    crate::feat::xdp_check_support().stringify_err()
}

/// 获取可用网卡列表
#[tauri::command]
pub fn xdp_get_interfaces() -> CmdResult<Vec<String>> {
    crate::feat::xdp_get_interfaces().stringify_err()
}
