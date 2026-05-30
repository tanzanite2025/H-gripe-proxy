/**
 * TLS 指纹伪装 Tauri 命令
 */

use crate::tls_fingerprint::{TlsFingerprint, TlsFingerprintLibrary};

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

/// 获取当前指纹
#[tauri::command]
pub fn tls_fingerprint_get_current() -> Result<Option<TlsFingerprint>, String> {
    let coordinator = crate::cmd::coordinator::get_coordinator();
    let service = coordinator.tls_fingerprint();

    Ok(service.get_fingerprint())
}

/// 生成 Clash 配置
#[tauri::command]
pub fn tls_fingerprint_generate_config() -> Result<Option<serde_json::Value>, String> {
    let coordinator = crate::cmd::coordinator::get_coordinator();
    let service = coordinator.tls_fingerprint();

    Ok(service.generate_clash_config())
}
