/**
 * 时区/NTP 伪装 feat 模块
 */

use crate::core::timezone_spoof::TimezoneSpoofConfig;

/// 获取时区伪装配置
pub fn timezone_spoof_get_config() -> TimezoneSpoofConfig {
    crate::feat::get_coordinator()
        .get_advanced_config()
        .timezone_spoof
}

/// 更新时区伪装配置
pub fn timezone_spoof_update_config(config: TimezoneSpoofConfig) {
    let mut advanced = crate::feat::get_coordinator().get_advanced_config();
    advanced.timezone_spoof = config;
    if let Err(e) = crate::feat::get_coordinator().hydrate_from_advanced_config(&advanced) {
        log::error!("[TimezoneSpoof] 配置更新失败: {}", e);
    } else {
        log::info!("[TimezoneSpoof] 配置已更新");
    }
}

/// 根据国家代码获取推荐的 NTP 服务器
pub fn timezone_spoof_get_ntp_server(country_code: &str) -> String {
    crate::core::timezone_spoof::select_ntp_server(country_code)
}

/// 根据国家代码获取时区名
pub fn timezone_spoof_get_timezone(country_code: &str) -> String {
    crate::core::timezone_spoof::country_to_timezone(country_code).to_string()
}

/// 根据时区获取 locale 标签
pub fn timezone_spoof_get_locale(timezone: &str) -> String {
    crate::core::timezone_spoof::timezone_to_locale(timezone)
}
