use crate::core::egress_monitor::{
    egress_monitor, EgressMonitorConfig, EgressMonitorStats, EgressIpProbeResult,
};
use anyhow::Result;

pub async fn egress_monitor_get_config() -> EgressMonitorConfig {
    egress_monitor().get_config()
}

pub async fn egress_monitor_update_config(config: EgressMonitorConfig) -> Result<()> {
    egress_monitor().update_config(config)
}

pub async fn egress_monitor_start() {
    egress_monitor().start();
}

pub async fn egress_monitor_stop() {
    egress_monitor().stop();
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
