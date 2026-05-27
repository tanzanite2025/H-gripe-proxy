/**
 * 反主动探测 Tauri 命令
 */

use crate::anti_probe::{AntiProbeConfig, AntiProbeService};
use once_cell::sync::Lazy;
use parking_lot::RwLock;
use std::net::IpAddr;
use std::sync::Arc;

static ANTI_PROBE_SERVICE: Lazy<Arc<RwLock<AntiProbeService>>> = Lazy::new(|| {
    Arc::new(RwLock::new(AntiProbeService::new(
        AntiProbeConfig::default(),
    )))
});

/// 验证握手暗号
#[tauri::command]
pub fn anti_probe_verify_handshake(client_ip: String, token: String) -> Result<bool, String> {
    let ip: IpAddr = client_ip
        .parse()
        .map_err(|e| format!("Invalid IP address: {}", e))?;

    let service = ANTI_PROBE_SERVICE.read();
    Ok(service.verify_handshake(&ip, &token))
}

/// 生成握手暗号
#[tauri::command]
pub fn anti_probe_generate_token() -> Result<String, String> {
    let service = ANTI_PROBE_SERVICE.read();
    Ok(service.generate_token())
}

/// 更新配置
#[tauri::command]
pub fn anti_probe_update_config(config: AntiProbeConfigDto) -> Result<(), String> {
    let config = AntiProbeConfig {
        enabled: config.enabled,
        secret_key: config.secret_key,
        time_window: config.time_window,
        whitelist: config
            .whitelist
            .iter()
            .filter_map(|s| s.parse().ok())
            .collect(),
        strict_mode: config.strict_mode,
    };

    let service = ANTI_PROBE_SERVICE.write();
    service.update_config(config);
    Ok(())
}

/// 获取配置
#[tauri::command]
pub fn anti_probe_get_config() -> Result<AntiProbeConfigDto, String> {
    let service = ANTI_PROBE_SERVICE.read();
    let config = service.get_config();

    Ok(AntiProbeConfigDto {
        enabled: config.enabled,
        secret_key: config.secret_key,
        time_window: config.time_window,
        whitelist: config.whitelist.iter().map(|ip| ip.to_string()).collect(),
        strict_mode: config.strict_mode,
    })
}

/// 清理过期缓存
#[tauri::command]
pub fn anti_probe_cleanup() -> Result<(), String> {
    let service = ANTI_PROBE_SERVICE.read();
    service.cleanup_expired();
    Ok(())
}

/// 配置 DTO（用于前端交互）
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AntiProbeConfigDto {
    pub enabled: bool,
    pub secret_key: String,
    pub time_window: u64,
    pub whitelist: Vec<String>,
    pub strict_mode: bool,
}
