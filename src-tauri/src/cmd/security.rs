/**
 * 安全功能 Tauri 命令
 */

use crate::cmd::{CmdResult, StringifyErr};
use crate::core::security_runtime;
use crate::core::security_runtime::SecurityStatus;
use std::path::PathBuf;

/// 启动安全监控
#[tauri::command]
pub async fn security_start_monitor() -> CmdResult {
    security_runtime::start_monitor().await;
    log::info!("✅ 安全监控已启动");
    Ok(())
}

/// 停止安全监控
#[tauri::command]
pub fn security_stop_monitor() -> CmdResult {
    security_runtime::stop_monitor();
    log::info!("✅ 安全监控已停止");
    Ok(())
}

/// 检查安全状态
#[tauri::command]
pub async fn security_check_status() -> CmdResult<SecurityStatus> {
    Ok(security_runtime::check_status().await)
}

/// 部署假配置文件
#[tauri::command]
pub fn security_deploy_decoy(decoy_path: String) -> CmdResult {
    security_runtime::deploy_decoy(PathBuf::from(decoy_path)).stringify_err()
}

/// 清除假配置文件
#[tauri::command]
pub fn security_cleanup_decoy(decoy_path: String) -> CmdResult {
    security_runtime::cleanup_decoy(PathBuf::from(decoy_path)).stringify_err()
}

/// 检查假配置是否被访问
#[tauri::command]
pub fn security_check_decoy_access(decoy_path: String) -> CmdResult<bool> {
    security_runtime::check_decoy_access(PathBuf::from(decoy_path)).stringify_err()
}

/// 生成加密密钥
#[tauri::command]
pub fn security_generate_encryption_key() -> CmdResult<String> {
    Ok(security_runtime::generate_key())
}

/// 加密数据
#[tauri::command]
pub fn security_encrypt_data(data: Vec<u8>) -> CmdResult<Vec<u8>> {
    security_runtime::encrypt_data(data).stringify_err()
}

/// 解密数据
#[tauri::command]
pub fn security_decrypt_data(data: Vec<u8>) -> CmdResult<Vec<u8>> {
    security_runtime::decrypt_data(data).stringify_err()
}

/// 检查加密密钥是否可用
#[tauri::command]
pub fn security_check_encryption_key() -> CmdResult<bool> {
    Ok(security_runtime::is_key_available())
}

/// 触发自毁（需要确认）
#[tauri::command]
pub fn security_self_destruct(confirmation: String) -> Result<(), String> {
    if confirmation != "CONFIRM_SELF_DESTRUCT" {
        return Err("需要确认码".to_string());
    }

    log::warn!("🚨 用户手动触发自毁");
    crate::security::self_destruct::execute();
    Ok(())
}

/// 安全状态
// ==================== 本地安全监控命令 ====================

/// 获取本地安全配置
#[tauri::command]
pub async fn local_security_get_config() -> CmdResult<security_runtime::LocalSecurityConfig> {
    Ok(security_runtime::local_security_get_config().await)
}

/// 更新本地安全配置
#[tauri::command]
pub async fn local_security_update_config(
    config: security_runtime::LocalSecurityConfig,
) -> CmdResult {
    security_runtime::local_security_update_config(config).await;
    log::info!("✅ 本地安全配置已更新");
    Ok(())
}

/// 获取泄漏监控状态
#[tauri::command]
pub async fn local_security_get_status() -> CmdResult<security_runtime::LeakMonitorStatus> {
    Ok(security_runtime::local_security_get_status().await)
}

/// 立即执行安全检查
#[tauri::command]
pub async fn local_security_check_now(port: u16) -> CmdResult<security_runtime::LeakMonitorStatus> {
    security_runtime::local_security_check_now(port)
        .await
        .stringify_err()
}

/// 检查本地绑定是否安全
#[tauri::command]
pub async fn local_security_check_binding(port: u16) -> CmdResult<bool> {
    security_runtime::local_security_check_binding(port)
        .await
        .stringify_err()
}

/// 检查端口冲突
#[tauri::command]
pub async fn local_security_check_port_conflict(port: u16) -> CmdResult<bool> {
    security_runtime::local_security_check_port_conflict(port)
        .await
        .stringify_err()
}

/// 查找可用端口
#[tauri::command]
pub async fn local_security_find_available_port() -> CmdResult<u16> {
    security_runtime::local_security_find_available_port()
        .await
        .stringify_err()
}

/// 配置防火墙规则
#[tauri::command]
pub async fn local_security_configure_firewall(port: u16) -> CmdResult {
    security_runtime::local_security_configure_firewall(port)
        .await
        .map_err(|e| e.to_string())?;
    log::info!("✅ 防火墙规则已配置 (端口: {})", port);
    Ok(())
}

/// 删除防火墙规则
#[tauri::command]
pub async fn local_security_remove_firewall(port: u16) -> CmdResult {
    security_runtime::local_security_remove_firewall(port)
        .await
        .map_err(|e| e.to_string())?;
    log::info!("✅ 防火墙规则已删除 (端口: {})", port);
    Ok(())
}

// ==================== 泄漏监控循环命令 ====================

/// 启动泄漏监控循环
#[tauri::command]
pub async fn leak_monitor_start(port: u16) -> CmdResult {
    security_runtime::leak_monitor_start(port)
        .await
        .map_err(|e| format!("启动泄漏监控失败: {}", e))?;
    log::info!("✅ 泄漏监控已启动 (端口: {})", port);
    Ok(())
}

/// 停止泄漏监控循环
#[tauri::command]
pub async fn leak_monitor_stop() -> CmdResult {
    security_runtime::leak_monitor_stop()
        .await
        .map_err(|e| e.to_string())?;
    log::info!("✅ 泄漏监控已停止");
    Ok(())
}

/// 检查泄漏监控是否正在运行
#[tauri::command]
pub async fn leak_monitor_is_running() -> CmdResult<bool> {
    Ok(security_runtime::leak_monitor_is_running().await)
}

/// 更新泄漏监控端口
#[tauri::command]
pub async fn leak_monitor_set_port(port: u16) -> CmdResult {
    security_runtime::leak_monitor_set_port(port)
        .await
        .map_err(|e| e.to_string())?;
    log::info!("✅ 泄漏监控端口已更新: {}", port);
    Ok(())
}

/// 获取泄漏监控端口
#[tauri::command]
pub async fn leak_monitor_get_port() -> CmdResult<u16> {
    security_runtime::leak_monitor_get_port()
        .await
        .stringify_err()
}
