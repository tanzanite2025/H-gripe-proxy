use super::{CmdResult, StringifyErr};
/**
 * 时区/NTP 伪装 Tauri 命令
 */
use crate::core::timezone_spoof::TimezoneSpoofConfig;

/// 获取时区伪装配置
#[tauri::command]
pub fn timezone_spoof_get_config() -> CmdResult<TimezoneSpoofConfig> {
    Ok(crate::core::coordinator::get_coordinator()
        .get_advanced_config()
        .timezone_spoof)
}

/// 更新时区伪装配置
#[tauri::command]
pub async fn timezone_spoof_update_config(config: TimezoneSpoofConfig) -> CmdResult<()> {
    crate::core::coordinator::update_advanced_config(move |advanced| {
        advanced.timezone_spoof = config;
    })
        .await
        .stringify_err()
}

/// 根据国家代码获取推荐的 NTP 服务器
#[tauri::command]
pub fn timezone_spoof_get_ntp_server(country_code: String) -> CmdResult<String> {
    Ok(crate::core::timezone_spoof::select_ntp_server(&country_code))
}

/// 根据国家代码获取时区
#[tauri::command]
pub fn timezone_spoof_get_timezone(country_code: String) -> CmdResult<String> {
    Ok(crate::core::timezone_spoof::country_to_timezone(&country_code).to_string())
}

/// 根据时区获取 locale
#[tauri::command]
pub fn timezone_spoof_get_locale(timezone: String) -> CmdResult<String> {
    Ok(crate::core::timezone_spoof::timezone_to_locale(&timezone))
}
