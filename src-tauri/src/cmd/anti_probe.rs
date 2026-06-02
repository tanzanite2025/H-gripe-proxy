use super::{CmdResult, StringifyErr};
/**
 * 反主动探测 Tauri 命令
 */
use crate::anti_probe::AntiProbeConfig;
use std::net::IpAddr;

#[tauri::command]
pub fn anti_probe_get_config() -> CmdResult<AntiProbeConfig> {
    Ok(crate::feat::anti_probe_get_config())
}

/// 验证握手暗号
#[tauri::command]
pub fn anti_probe_verify_handshake(client_ip: String, token: String) -> CmdResult<bool> {
    let ip: IpAddr = client_ip.parse().stringify_err()?;
    Ok(crate::feat::anti_probe_verify_handshake(&ip, &token))
}

/// 生成握手暗号
#[tauri::command]
pub fn anti_probe_generate_token() -> CmdResult<String> {
    Ok(crate::feat::anti_probe_generate_token())
}

/// 清理过期缓存
#[tauri::command]
pub fn anti_probe_cleanup() -> CmdResult<()> {
    crate::feat::anti_probe_cleanup();
    Ok(())
}
