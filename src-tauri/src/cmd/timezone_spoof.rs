/**
 * 时区/NTP 伪装 Tauri 命令
 */

use crate::core::timezone_spoof::TimezoneSpoofConfig;
use super::CmdResult;

/// 获取时区伪装配置
#[tauri::command]
pub fn timezone_spoof_get_config() -> CmdResult<TimezoneSpoofConfig> {
    Ok(crate::feat::timezone_spoof_get_config())
}

/// 更新时区伪装配置
#[tauri::command]
pub fn timezone_spoof_update_config(config: TimezoneSpoofConfig) -> CmdResult<()> {
    crate::feat::timezone_spoof_update_config(config);
    Ok(())
}

/// 根据国家代码获取推荐的 NTP 服务器
#[tauri::command]
pub fn timezone_spoof_get_ntp_server(country_code: String) -> CmdResult<String> {
    Ok(crate::feat::timezone_spoof_get_ntp_server(&country_code))
}

/// 根据国家代码获取时区
#[tauri::command]
pub fn timezone_spoof_get_timezone(country_code: String) -> CmdResult<String> {
    Ok(crate::feat::timezone_spoof_get_timezone(&country_code))
}

/// 根据时区获取 locale
#[tauri::command]
pub fn timezone_spoof_get_locale(timezone: String) -> CmdResult<String> {
    Ok(crate::feat::timezone_spoof_get_locale(&timezone))
}
