use crate::{
    config::AdvancedConfig,
    core::egress_monitor::{EgressIpProbeResult, EgressMonitorConfig, EgressMonitorStats, egress_monitor},
};
use anyhow::Result;

pub async fn egress_monitor_get_config() -> EgressMonitorConfig {
    AdvancedConfig::load_default().egress_monitor
}

pub async fn apply_egress_monitor_config(config: EgressMonitorConfig) -> Result<()> {
    egress_monitor().update_config(config.clone())?;
    if config.enabled {
        egress_monitor().start();
    } else {
        egress_monitor().stop();
    }
    Ok(())
}

pub async fn egress_monitor_update_config(config: EgressMonitorConfig) -> Result<()> {
    config.validate()?;
    persist_egress_monitor_config(&config)?;
    apply_egress_monitor_config(config).await
}

pub async fn egress_monitor_start() -> Result<()> {
    let mut config = AdvancedConfig::load_default().egress_monitor;
    config.enabled = true;
    egress_monitor_update_config(config).await
}

pub async fn egress_monitor_stop() -> Result<()> {
    let mut config = AdvancedConfig::load_default().egress_monitor;
    config.enabled = false;
    egress_monitor_update_config(config).await
}

pub async fn egress_monitor_get_stats() -> EgressMonitorStats {
    egress_monitor().get_stats()
}

pub async fn egress_monitor_reset_stats() {
    egress_monitor().reset_stats();
}

pub async fn egress_monitor_probe_now() -> Result<EgressIpProbeResult> {
    egress_monitor().probe_now().await
}

pub async fn egress_monitor_is_running() -> bool {
    egress_monitor().is_running()
}

fn persist_egress_monitor_config(config: &EgressMonitorConfig) -> Result<()> {
    let mut advanced = AdvancedConfig::load_default_strict()?;
    advanced.egress_monitor = config.clone();
    advanced.validate()?;
    advanced.save_default()?;
    crate::feat::get_coordinator().hydrate_from_advanced_config(&advanced)?;
    Ok(())
}
