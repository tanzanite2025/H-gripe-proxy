use crate::{
    config::AdvancedConfig,
    feat::get_coordinator,
    xdp::{XdpConfig, XdpManager, XdpRoute, XdpStatus, XdpSupportInfo},
};
use anyhow::Result;

pub fn xdp_get_config() -> XdpConfig {
    AdvancedConfig::load_default().xdp
}

pub fn xdp_update_config(config: XdpConfig) -> Result<()> {
    let mut advanced = AdvancedConfig::load_default_strict()?;
    advanced.xdp = config;
    advanced.validate()?;
    advanced.save_default()?;
    get_coordinator().apply_advanced_config(&advanced)?;
    Ok(())
}

pub fn xdp_get_status() -> XdpStatus {
    let manager = get_coordinator().xdp_manager();
    manager.get_status()
}

pub fn xdp_start() -> Result<()> {
    let mut config = AdvancedConfig::load_default().xdp;
    config.enabled = true;
    xdp_update_config(config)
}

pub fn xdp_stop() -> Result<()> {
    let mut config = AdvancedConfig::load_default().xdp;
    config.enabled = false;
    xdp_update_config(config)
}

pub fn xdp_add_route(route: XdpRoute) -> Result<()> {
    let manager = get_coordinator().xdp_manager();
    manager.add_route(route)
}

pub fn xdp_remove_route(dest_ip: &str) -> Result<()> {
    let manager = get_coordinator().xdp_manager();
    manager.remove_route(dest_ip)
}

pub fn xdp_update_stats() -> Result<()> {
    let manager = get_coordinator().xdp_manager();
    manager.update_stats()
}

pub fn xdp_check_support() -> Result<XdpSupportInfo> {
    XdpManager::check_support()
}

pub fn xdp_get_interfaces() -> Result<Vec<String>> {
    #[cfg(target_os = "linux")]
    {
        use std::fs;

        let mut interfaces = Vec::new();

        if let Ok(entries) = fs::read_dir("/sys/class/net") {
            for entry in entries.flatten() {
                if let Some(name) = entry.file_name().to_str() {
                    if name != "lo" {
                        interfaces.push(name.to_string());
                    }
                }
            }
        }

        Ok(interfaces)
    }

    #[cfg(not(target_os = "linux"))]
    {
        Err(anyhow::anyhow!("XDP 仅支持 Linux 系统"))
    }
}
