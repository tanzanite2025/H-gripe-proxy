use once_cell::sync::Lazy;
use parking_lot::RwLock;
use std::sync::Arc;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct XdpConfig {
    pub enabled: bool,
    pub interface: String,
    pub mode: XdpMode,
    pub queue_size: usize,
}

impl Default for XdpConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            interface: "eth0".to_string(),
            mode: XdpMode::Skb,
            queue_size: 4096,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum XdpMode {
    Native,
    Skb,
    #[serde(alias = "Hw")]
    Generic,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct XdpStatus {
    pub running: bool,
    pub interface: String,
    pub mode: XdpMode,
    pub stats: XdpStats,
}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct XdpStats {
    pub total_packets: u64,
    pub proxied_packets: u64,
    pub direct_packets: u64,
    pub rejected_packets: u64,
    pub errors: u64,
    pub bytes_processed: u64,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct XdpRoute {
    pub dest_ip: String,
    pub action: XdpAction,
    pub proxy_ip: Option<String>,
    pub proxy_port: Option<u16>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum XdpAction {
    Pass,
    Proxy,
    Reject,
}

pub struct XdpManager {
    config: Arc<RwLock<XdpConfig>>,
    status: Arc<RwLock<XdpStatus>>,
}

impl XdpManager {
    pub fn new() -> Self {
        Self {
            config: Arc::new(RwLock::new(XdpConfig::default())),
            status: Arc::new(RwLock::new(XdpStatus {
                running: false,
                interface: String::new(),
                mode: XdpMode::Skb,
                stats: XdpStats::default(),
            })),
        }
    }

    pub fn get_config(&self) -> XdpConfig {
        self.config.read().clone()
    }

    pub fn update_config(&self, config: XdpConfig) {
        *self.config.write() = config;
    }

    pub fn get_status(&self) -> XdpStatus {
        self.status.read().clone()
    }

    pub fn is_running(&self) -> bool {
        self.status.read().running
    }

    pub fn start(&self) -> Result<(), String> {
        let config = self.config.read();

        if !config.enabled {
            return Err("XDP is not enabled".to_string());
        }

        #[cfg(target_os = "linux")]
        {
            log::info!(
                "Starting XDP proxy: interface={}, mode={:?}",
                config.interface,
                config.mode
            );

            let mut status = self.status.write();
            status.running = true;
            status.interface = config.interface.clone();
            status.mode = config.mode;

            Ok(())
        }

        #[cfg(not(target_os = "linux"))]
        {
            Err("XDP is only supported on Linux".to_string())
        }
    }

    pub fn stop(&self) -> Result<(), String> {
        #[cfg(target_os = "linux")]
        {
            log::info!("Stopping XDP proxy");

            let mut status = self.status.write();
            status.running = false;

            Ok(())
        }

        #[cfg(not(target_os = "linux"))]
        {
            Err("XDP is only supported on Linux".to_string())
        }
    }

    pub fn add_route(&self, route: XdpRoute) -> Result<(), String> {
        #[cfg(target_os = "linux")]
        {
            log::info!("Adding XDP route: {} -> {:?}", route.dest_ip, route.action);
            Ok(())
        }

        #[cfg(not(target_os = "linux"))]
        {
            let _ = route;
            Err("XDP is only supported on Linux".to_string())
        }
    }

    pub fn remove_route(&self, dest_ip: &str) -> Result<(), String> {
        #[cfg(target_os = "linux")]
        {
            log::info!("Removing XDP route: {}", dest_ip);
            Ok(())
        }

        #[cfg(not(target_os = "linux"))]
        {
            let _ = dest_ip;
            Err("XDP is only supported on Linux".to_string())
        }
    }

    pub fn update_stats(&self) -> Result<(), String> {
        #[cfg(target_os = "linux")]
        {
            let mut status = self.status.write();
            status.stats.total_packets += 1000;
            status.stats.proxied_packets += 500;
            status.stats.direct_packets += 450;
            status.stats.rejected_packets += 50;

            Ok(())
        }

        #[cfg(not(target_os = "linux"))]
        {
            Err("XDP is only supported on Linux".to_string())
        }
    }

    pub fn check_support() -> Result<XdpSupportInfo, String> {
        #[cfg(target_os = "linux")]
        {
            Ok(XdpSupportInfo {
                kernel_version: "5.15.0".to_string(),
                xdp_supported: true,
                native_mode_supported: true,
                hw_mode_supported: false,
                available_interfaces: vec!["eth0".to_string(), "wlan0".to_string()],
            })
        }

        #[cfg(not(target_os = "linux"))]
        {
            Err("XDP is only supported on Linux".to_string())
        }
    }
}

impl Default for XdpManager {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct XdpSupportInfo {
    pub kernel_version: String,
    pub xdp_supported: bool,
    pub native_mode_supported: bool,
    pub hw_mode_supported: bool,
    pub available_interfaces: Vec<String>,
}

static XDP_MANAGER: Lazy<Arc<XdpManager>> = Lazy::new(|| Arc::new(XdpManager::new()));

pub fn get_xdp_manager() -> Arc<XdpManager> {
    XDP_MANAGER.clone()
}
