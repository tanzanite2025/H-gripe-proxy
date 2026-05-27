/**
 * TLS 指纹伪装 Tauri 命令
 */

use crate::tls_fingerprint::{TlsFingerprint, TlsFingerprintLibrary, TlsFingerprintService};
use once_cell::sync::Lazy;
use parking_lot::RwLock;
use std::sync::Arc;

static TLS_FINGERPRINT_SERVICE: Lazy<Arc<RwLock<TlsFingerprintService>>> =
    Lazy::new(|| Arc::new(RwLock::new(TlsFingerprintService::new())));

/// 获取所有预定义指纹
#[tauri::command]
pub fn tls_fingerprint_get_all() -> Result<Vec<TlsFingerprint>, String> {
    Ok(TlsFingerprintLibrary::get_all())
}

/// 根据名称获取指纹
#[tauri::command]
pub fn tls_fingerprint_get_by_name(name: String) -> Result<Option<TlsFingerprint>, String> {
    Ok(TlsFingerprintLibrary::get_by_name(&name))
}

/// 设置当前指纹
#[tauri::command]
pub fn tls_fingerprint_set(fingerprint: TlsFingerprint) -> Result<(), String> {
    let service = TLS_FINGERPRINT_SERVICE.read();
    service.set_fingerprint(fingerprint);
    Ok(())
}

/// 设置当前指纹（通过名称）
#[tauri::command]
pub fn tls_fingerprint_set_by_name(name: String) -> Result<(), String> {
    let fingerprint = TlsFingerprintLibrary::get_by_name(&name)
        .ok_or_else(|| format!("Fingerprint not found: {}", name))?;

    let service = TLS_FINGERPRINT_SERVICE.read();
    service.set_fingerprint(fingerprint);
    Ok(())
}

/// 获取当前指纹
#[tauri::command]
pub fn tls_fingerprint_get_current() -> Result<Option<TlsFingerprint>, String> {
    let service = TLS_FINGERPRINT_SERVICE.read();
    Ok(service.get_fingerprint())
}

/// 生成 Clash 配置
#[tauri::command]
pub fn tls_fingerprint_generate_config() -> Result<Option<serde_json::Value>, String> {
    let service = TLS_FINGERPRINT_SERVICE.read();
    Ok(service.generate_clash_config())
}

/// 清除当前指纹
#[tauri::command]
pub fn tls_fingerprint_clear() -> Result<(), String> {
    let mut service = TLS_FINGERPRINT_SERVICE.write();
    *service = TlsFingerprintService::new();
    Ok(())
}
