/**
 * 安全功能 Tauri 命令
 */

use crate::security::{
    config_decoy::{ConfigDecoy, SecureConfigStorage, generate_encryption_key},
    SecurityMonitor,
};
use once_cell::sync::Lazy;
use parking_lot::RwLock;
use std::path::PathBuf;
use std::sync::Arc;

static SECURITY_MONITOR: Lazy<Arc<RwLock<SecurityMonitor>>> =
    Lazy::new(|| Arc::new(RwLock::new(SecurityMonitor::new())));

/// 启动安全监控
#[tauri::command]
pub fn security_start_monitor() -> Result<(), String> {
    let monitor = SECURITY_MONITOR.read();
    monitor.start();
    log::info!("✅ 安全监控已启动");
    Ok(())
}

/// 停止安全监控
#[tauri::command]
pub fn security_stop_monitor() -> Result<(), String> {
    let monitor = SECURITY_MONITOR.read();
    monitor.stop();
    log::info!("✅ 安全监控已停止");
    Ok(())
}

/// 检查安全状态
#[tauri::command]
pub fn security_check_status() -> Result<SecurityStatus, String> {
    Ok(SecurityStatus {
        compromised: crate::security::is_security_compromised(),
        debugger_present: crate::security::anti_debug::is_debugger_present(),
        memory_scanning: crate::security::memory_honeypot::detect_memory_scanning(),
    })
}

/// 部署假配置文件
#[tauri::command]
pub fn security_deploy_decoy(decoy_path: String) -> Result<(), String> {
    let path = PathBuf::from(decoy_path);
    let decoy = ConfigDecoy::new(path);
    decoy.deploy()
}

/// 清除假配置文件
#[tauri::command]
pub fn security_cleanup_decoy(decoy_path: String) -> Result<(), String> {
    let path = PathBuf::from(decoy_path);
    let decoy = ConfigDecoy::new(path);
    decoy.cleanup()
}

/// 检查假配置是否被访问
#[tauri::command]
pub fn security_check_decoy_access(decoy_path: String) -> Result<bool, String> {
    let path = PathBuf::from(decoy_path);
    let decoy = ConfigDecoy::new(path);
    Ok(decoy.check_access())
}

/// 生成加密密钥
#[tauri::command]
pub fn security_generate_encryption_key() -> Result<String, String> {
    Ok(generate_encryption_key())
}

/// 加密数据
#[tauri::command]
pub fn security_encrypt_data(data: Vec<u8>) -> Result<Vec<u8>, String> {
    let storage = SecureConfigStorage::new();
    if !storage.is_key_available() {
        return Err("加密密钥未设置，请设置环境变量 CLASH_VERGE_SECURE_KEY".to_string());
    }
    storage.encrypt(&data)
}

/// 解密数据
#[tauri::command]
pub fn security_decrypt_data(data: Vec<u8>) -> Result<Vec<u8>, String> {
    let storage = SecureConfigStorage::new();
    if !storage.is_key_available() {
        return Err("加密密钥未设置，请设置环境变量 CLASH_VERGE_SECURE_KEY".to_string());
    }
    storage.decrypt(&data)
}

/// 检查加密密钥是否可用
#[tauri::command]
pub fn security_check_encryption_key() -> Result<bool, String> {
    let storage = SecureConfigStorage::new();
    Ok(storage.is_key_available())
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
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SecurityStatus {
    pub compromised: bool,
    pub debugger_present: bool,
    pub memory_scanning: bool,
}
