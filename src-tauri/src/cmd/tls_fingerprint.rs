use super::{CmdResult, StringifyErr as _};
/**
 * TLS 指纹伪装 Tauri 命令
 */
use crate::tls_fingerprint::TlsFingerprint;

/// 获取所有预定义指纹
#[tauri::command]
pub fn tls_fingerprint_get_all() -> CmdResult<Vec<TlsFingerprint>> {
    Ok(crate::tls_fingerprint::TlsFingerprintLibrary::get_all())
}

/// 根据名称获取指纹
#[tauri::command]
pub fn tls_fingerprint_get_by_name(name: String) -> CmdResult<Option<TlsFingerprint>> {
    Ok(crate::tls_fingerprint::TlsFingerprintLibrary::get_by_name(&name))
}

/// 获取当前指纹
#[tauri::command]
pub fn tls_fingerprint_get_current() -> CmdResult<Option<TlsFingerprint>> {
    Ok(crate::core::coordinator::get_coordinator()
        .get_advanced_config()
        .security
        .tls_fingerprint
        .as_deref()
        .and_then(crate::tls_fingerprint::TlsFingerprintLibrary::get_by_name))
}

/// 生成 Clash 配置
#[tauri::command]
pub fn tls_fingerprint_generate_config() -> CmdResult<Option<serde_json::Value>> {
    Ok(tls_fingerprint_get_current()?.map(|fp| {
        serde_json::json!({
            "global-client-fingerprint": fp.name,
        })
    }))
}

/// 清除当前指纹
#[tauri::command]
pub fn tls_fingerprint_clear() -> CmdResult {
    crate::core::coordinator::update_advanced_config_blocking(|advanced| {
        advanced.security.tls_fingerprint = None;
    })
    .stringify_err()
}
