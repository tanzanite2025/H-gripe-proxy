use crate::utils::schtasks;
use crate::{config::Config, core::handle::Handle};
use anyhow::Result;
use clash_verge_logging::{Type, logging};
use tauri_plugin_clash_verge_sysinfo::is_current_app_handle_admin;

pub async fn update_launch() -> Result<()> {
    let enable_auto_launch = { Config::verge().await.latest_arc().enable_auto_launch };
    let is_enable = enable_auto_launch.unwrap_or(false);
    logging!(info, Type::System, "Setting auto-launch enabled state to: {is_enable}");

    let is_admin = is_current_app_handle_admin(Handle::app_handle());
    schtasks::set_auto_launch(is_enable, is_admin).await?;

    Ok(())
}

pub fn get_launch_status() -> Result<bool> {
    let enabled = schtasks::is_auto_launch_enabled();
    if let Ok(status) = enabled {
        logging!(info, Type::System, "Auto-launch status (scheduled task): {status}");
    }
    enabled
}
