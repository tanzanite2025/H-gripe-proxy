use crate::{config::AdvancedConfig, core::CoreManager, core::timezone_spoof::TimezoneSpoofConfig};
use anyhow::Result;

pub fn timezone_spoof_get_config() -> TimezoneSpoofConfig {
    AdvancedConfig::load_default().timezone_spoof
}

pub async fn timezone_spoof_update_config(config: TimezoneSpoofConfig) -> Result<()> {
    let mut advanced = AdvancedConfig::load_default_strict()?;
    advanced.timezone_spoof = config;
    advanced.validate()?;
    advanced.save_default()?;
    crate::feat::get_coordinator().hydrate_from_advanced_config(&advanced)?;
    CoreManager::global().update_config_checked().await?;
    log::info!("[TimezoneSpoof] config updated");
    Ok(())
}

pub fn timezone_spoof_get_ntp_server(country_code: &str) -> String {
    crate::core::timezone_spoof::select_ntp_server(country_code)
}

pub fn timezone_spoof_get_timezone(country_code: &str) -> String {
    crate::core::timezone_spoof::country_to_timezone(country_code).to_string()
}

pub fn timezone_spoof_get_locale(timezone: &str) -> String {
    crate::core::timezone_spoof::timezone_to_locale(timezone)
}
