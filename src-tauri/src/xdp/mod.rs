/**
 * XDP 代理集成模块
 * 
 * 将 XDP 代理功能集成到 Clash Verge
 */

use std::sync::Arc;
use parking_lot::RwLock;
use once_cell::sync::Lazy;

/// XDP 代理配置
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct XdpConfig {
    /// 是否启用 XDP 代理
    pub enabled: bool,
    /// 网卡接口名称
    pub interface: String,
    /// XDP 模式（native, skb, hw）
    pub mode: XdpMode,
    /// 队列大小
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

/// XDP 模式
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum XdpMode {
    /// Native 模式（最高性能，需要驱动支持）
    Native,
    /// SKB 模式（兼容性好，性能较低）
    Skb,
    /// Generic 模式（通用模式，所有网卡都支持）
    Generic,
}

/// XDP 代理状态
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct XdpStatus {
    /// 是否运行中
    pub running: bool,
    /// 当前接口
    pub interface: String,
    /// 当前模式
    pub mode: XdpMode,
    /// 统计信息
    pub stats: XdpStats,
}

/// XDP 统计信息
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct XdpStats {
    pub total_packets: u64,
    pub proxied_packets: u64,
    pub direct_packets: u64,
    pub rejected_packets: u64,
    pub errors: u64,
    pub bytes_processed: u64,
}

/// XDP 路由规则
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct XdpRoute {
    /// 目标 IP 地址
    pub dest_ip: String,
    /// 动作
    pub action: XdpAction,
    /// 代理服务器 IP（仅 Proxy 动作需要）
    pub proxy_ip: Option<String>,
    /// 代理服务器端口（仅 Proxy 动作需要）
    pub proxy_port: Option<u16>,
}

/// XDP 动作
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum XdpAction {
    /// 直连
    Pass,
    /// 代理
    Proxy,
    /// 拒绝
    Reject,
}

/// XDP 代理管理器
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

    /// 获取配置
    pub fn get_config(&self) -> XdpConfig {
        self.config.read().clone()
    }

    /// 更新配置
    pub fn update_config(&self, config: XdpConfig) {
        *self.config.write() = config;
    }

    /// 获取状态
    pub fn get_status(&self) -> XdpStatus {
        self.status.read().clone()
    }

    /// 检查是否运行中
    pub fn is_running(&self) -> bool {
        self.status.read().running
    }

    /// 启动 XDP 代理
    pub fn start(&self) -> Result<(), String> {
        let config = self.config.read();

        if !config.enabled {
            return Err("XDP 代理未启用".to_string());
        }

        #[cfg(target_os = "linux")]
        {
            // TODO: 实际加载 XDP 程序
            log::info!("启动 XDP 代理: interface={}, mode={:?}", config.interface, config.mode);
            
            let mut status = self.status.write();
            status.running = true;
            status.interface = config.interface.clone();
            status.mode = config.mode;

            Ok(())
        }

        #[cfg(not(target_os = "linux"))]
        {
            Err("XDP 仅支持 Linux 系统".to_string())
        }
    }

    /// 停止 XDP 代理
    pub fn stop(&self) -> Result<(), String> {
        #[cfg(target_os = "linux")]
        {
            log::info!("停止 XDP 代理");
            
            let mut status = self.status.write();
            status.running = false;

            Ok(())
        }

        #[cfg(not(target_os = "linux"))]
        {
            Err("XDP 仅支持 Linux 系统".to_string())
        }
    }

    /// 添加路由规则
    pub fn add_route(&self, route: XdpRoute) -> Result<(), String> {
        #[cfg(target_os = "linux")]
        {
            log::info!("添加 XDP 路由: {} -> {:?}", route.dest_ip, route.action);
            // TODO: 实际更新 eBPF Map
            Ok(())
        }

        #[cfg(not(target_os = "linux"))]
        {
            Err("XDP 仅支持 Linux 系统".to_string())
        }
    }

    /// 删除路由规则
    pub fn remove_route(&self, dest_ip: &str) -> Result<(), String> {
        #[cfg(target_os = "linux")]
        {
            log::info!("删除 XDP 路由: {}", dest_ip);
            // TODO: 实际更新 eBPF Map
            Ok(())
        }

        #[cfg(not(target_os = "linux"))]
        {
            Err("XDP 仅支持 Linux 系统".to_string())
        }
    }

    /// 更新统计信息
    pub fn update_stats(&self) -> Result<(), String> {
        #[cfg(target_os = "linux")]
        {
            // TODO: 从 eBPF Map 读取统计
            let mut status = self.status.write();
            status.stats.total_packets += 1000;
            status.stats.proxied_packets += 500;
            status.stats.direct_packets += 450;
            status.stats.rejected_packets += 50;
            
            Ok(())
        }

        #[cfg(not(target_os = "linux"))]
        {
            Err("XDP 仅支持 Linux 系统".to_string())
        }
    }

    /// 检查系统支持
    pub fn check_support() -> Result<XdpSupportInfo, String> {
        #[cfg(target_os = "linux")]
        {
            // TODO: 实际检查内核版本和网卡支持
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
            Err("XDP 仅支持 Linux 系统".to_string())
        }
    }
}

impl Default for XdpManager {
    fn default() -> Self {
        Self::new()
    }
}

/// XDP 支持信息
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct XdpSupportInfo {
    pub kernel_version: String,
    pub xdp_supported: bool,
    pub native_mode_supported: bool,
    pub hw_mode_supported: bool,
    pub available_interfaces: Vec<String>,
}

/// 全局 XDP 管理器
static XDP_MANAGER: Lazy<Arc<XdpManager>> = Lazy::new(|| Arc::new(XdpManager::new()));

/// 获取全局 XDP 管理器
pub fn get_xdp_manager() -> Arc<XdpManager> {
    XDP_MANAGER.clone()
}
