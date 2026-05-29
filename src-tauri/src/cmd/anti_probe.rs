/**
 * 反主动探测 Tauri 命令
 */

use crate::anti_probe::AntiProbeConfig;
use crate::config::AdvancedConfig;
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

/// 更新配置
#[tauri::command]
pub fn anti_probe_update_config(config: AntiProbeConfigDto) -> Result<(), String> {
    use crate::utils::dirs;

    // 1. 读取当前 advanced.yaml
    let path = dirs::app_home_dir()
        .map_err(|e| e.to_string())?
        .join("advanced.yaml");
    let mut advanced = AdvancedConfig::load(&path).map_err(|e| e.to_string())?;

    // 2. 用前端传入的 DTO 替换 security.anti_probe 配置
    advanced.security.anti_probe = AntiProbeConfig {
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

    // 3. 持久化回 advanced.yaml
    advanced.save(&path).map_err(|e| e.to_string())?;

    // 4. 通过协调器应用新的高级配置到运行时
    let coordinator = crate::cmd::coordinator::get_coordinator();
    coordinator
        .apply_advanced_config(&advanced)
        .map_err(|e| e.to_string())?;

    Ok(())
}

/// 获取配置
#[tauri::command]
pub fn anti_probe_get_config() -> Result<AntiProbeConfigDto, String> {
    use crate::utils::dirs;

    // 从 advanced.yaml 中读取当前的 security.anti_probe 配置
    let path = dirs::app_home_dir()
        .map_err(|e| e.to_string())?
        .join("advanced.yaml");
    let advanced = AdvancedConfig::load(&path).map_err(|e| e.to_string())?;
    let config = advanced.security.anti_probe;

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
    let coordinator = crate::cmd::coordinator::get_coordinator();
    let service = coordinator.anti_probe();

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
