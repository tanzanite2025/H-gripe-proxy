/**
 * 反主动探测 Tauri 命令
 */

use std::net::IpAddr;

/// 验证握手暗号
#[tauri::command]
pub fn anti_probe_verify_handshake(client_ip: String, token: String) -> Result<bool, String> {
    let ip: IpAddr = client_ip
        .parse()
        .map_err(|e| format!("Invalid IP address: {}", e))?;

    let coordinator = crate::cmd::coordinator::get_coordinator();
    let service = coordinator.anti_probe();

    Ok(service.verify_handshake(&ip, &token))
}

/// 生成握手暗号
#[tauri::command]
pub fn anti_probe_generate_token() -> Result<String, String> {
    let coordinator = crate::cmd::coordinator::get_coordinator();
    let service = coordinator.anti_probe();

    Ok(service.generate_token())
}

/// 清理过期缓存
#[tauri::command]
pub fn anti_probe_cleanup() -> Result<(), String> {
    let coordinator = crate::cmd::coordinator::get_coordinator();
    let service = coordinator.anti_probe();

    service.cleanup_expired();
    Ok(())
}
