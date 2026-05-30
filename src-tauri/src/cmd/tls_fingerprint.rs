/**
 * TLS 指纹伪装 Tauri 命令
 */

use crate::tls_fingerprint::TlsFingerprint;
use super::CmdResult;

/// 获取所有预定义指纹
#[tauri::command]
pub fn tls_fingerprint_get_all() -> CmdResult<Vec<TlsFingerprint>> {
    Ok(crate::feat::tls_fingerprint_get_all())
}

/// 根据名称获取指纹
#[tauri::command]
pub fn tls_fingerprint_get_by_name(name: String) -> CmdResult<Option<TlsFingerprint>> {
    Ok(crate::feat::tls_fingerprint_get_by_name(&name))
}

/// 获取当前指纹
#[tauri::command]
pub fn tls_fingerprint_get_current() -> CmdResult<Option<TlsFingerprint>> {
    Ok(crate::feat::tls_fingerprint_get_current())
}

/// 生成 Clash 配置
#[tauri::command]
pub fn tls_fingerprint_generate_config() -> CmdResult<Option<serde_json::Value>> {
    Ok(crate::feat::tls_fingerprint_generate_config())
}
