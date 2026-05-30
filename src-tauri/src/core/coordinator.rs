/**
 * 核心协调器
 * 
 * 统一管理所有高级功能模块：
 * - 安全防御层（反探测、TLS 指纹、内生欺骗）
 * - 路由决策层（多路径路由）
 * - 数据平面层（XDP 代理）
 */

use std::sync::Arc;
use parking_lot::RwLock;
use anyhow::Result;

use crate::anti_probe::{AntiProbeService, AntiProbeConfig};
use crate::config::AdvancedConfig;
use crate::core::egress_identity::EgressIdentityManager;
use crate::core::egress_monitor::EgressMonitor;
use crate::tls_fingerprint::TlsFingerprintService;
use crate::security::SecurityMonitor;
use crate::multipath::MultipathManager;

#[cfg(target_os = "linux")]
use crate::xdp::XdpManager;

/// 协调器配置
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CoordinatorConfig {
    /// 启用安全监控
    pub security_enabled: bool,
    /// 启用反探测
    pub anti_probe_enabled: bool,
    /// TLS 指纹名称
    pub tls_fingerprint: Option<String>,
    pub egress_identity_enabled: bool,
    pub session_affinity_enabled: bool,
    pub egress_monitor_enabled: bool,
    /// 启用多路径路由
    pub multipath_enabled: bool,
    /// 启用 XDP（仅 Linux）
    #[cfg(target_os = "linux")]
    pub xdp_enabled: bool,
}

impl Default for CoordinatorConfig {
    fn default() -> Self {
        Self {
            security_enabled: false,
            anti_probe_enabled: false,
            tls_fingerprint: None,
            egress_identity_enabled: false,
            session_affinity_enabled: false,
            egress_monitor_enabled: false,
            multipath_enabled: false,
            #[cfg(target_os = "linux")]
            xdp_enabled: false,
        }
    }
}


/// 核心协调器
pub struct CoreCoordinator {
    /// 安全监控
    security_monitor: Arc<SecurityMonitor>,
    /// 反探测服务
    anti_probe: Arc<AntiProbeService>,
    /// TLS 指纹服务
    tls_fingerprint: Arc<TlsFingerprintService>,
    /// 多路径管理器
    multipath_manager: Arc<MultipathManager>,
    egress_identity_manager: Arc<EgressIdentityManager>,
    egress_monitor: Arc<EgressMonitor>,
    /// XDP 管理器（Linux）
    #[cfg(target_os = "linux")]
    xdp_manager: Arc<XdpManager>,
    /// 配置
    config: Arc<RwLock<CoordinatorConfig>>,
}

impl CoreCoordinator {
    /// 创建新的协调器
    pub fn new() -> Self {
        Self {
            security_monitor: Arc::new(SecurityMonitor::new()),
            anti_probe: Arc::new(AntiProbeService::new(AntiProbeConfig::default())),
            tls_fingerprint: Arc::new(TlsFingerprintService::new()),
            multipath_manager: Arc::new(MultipathManager::new()),
            egress_identity_manager: Arc::new(EgressIdentityManager::new()),
            egress_monitor: Arc::new(EgressMonitor::new()),
            #[cfg(target_os = "linux")]
            xdp_manager: Arc::new(XdpManager::new()),
            config: Arc::new(RwLock::new(CoordinatorConfig::default())),
        }
    }

    /// 初始化所有模块
    fn config_from_advanced(config: &AdvancedConfig) -> CoordinatorConfig {
        CoordinatorConfig {
            security_enabled: config.security.enabled,
            anti_probe_enabled: config.security.anti_probe.enabled,
            tls_fingerprint: config.security.tls_fingerprint.clone(),
            egress_identity_enabled: config.egress_identity.enabled,
            session_affinity_enabled: config.session_affinity.enabled,
            egress_monitor_enabled: config.egress_monitor.enabled,
            multipath_enabled: config.multipath.enabled,
            #[cfg(target_os = "linux")]
            xdp_enabled: config.xdp.enabled,
        }
    }

    pub fn hydrate_from_advanced_config(&self, config: &AdvancedConfig) -> Result<()> {
        self.apply_sub_configs(config);
        *self.config.write() = Self::config_from_advanced(config);
        Ok(())
    }

    fn load_persisted_advanced_config(&self) -> Result<()> {
        let path = crate::utils::dirs::app_home_dir()?.join("advanced.yaml");
        let config = AdvancedConfig::load(&path)?;
        self.hydrate_from_advanced_config(&config)
    }

    pub fn apply_advanced_config(&self, config: &AdvancedConfig) -> Result<()> {
        self.apply_sub_configs(config);
        self.update_config(Self::config_from_advanced(config))
    }

    /// 将 AdvancedConfig 中的子配置分发到各管理器
    fn apply_sub_configs(&self, config: &AdvancedConfig) {
        self.anti_probe.update_config(config.security.anti_probe.clone());
        self.multipath_manager.update_config(config.multipath.clone());
        if let Err(e) = self.egress_identity_manager.update_config(config.egress_identity.clone()) {
            log::warn!("[Coordinator] 更新 egress_identity 配置失败: {}", e);
        }
        if let Err(e) = self.egress_monitor.update_config(config.egress_monitor.clone()) {
            log::warn!("[Coordinator] 更新 egress_monitor 配置失败: {}", e);
        }
        #[cfg(target_os = "linux")]
        self.xdp_manager.update_config(config.xdp.clone());
    }

    pub fn initialize(&self) -> Result<()> {
        self.load_persisted_advanced_config()?;
        let config = self.config.read();

        // 1. 启动安全监控
        if config.security_enabled {
            log::info!("[Coordinator] 启动安全监控");
            self.security_monitor.start();
        }

        // 2. 配置反探测
        if config.anti_probe_enabled {
            log::info!("[Coordinator] 启用反探测");
            // 反探测服务已在创建时初始化
        }

        // 3. 设置 TLS 指纹
        if let Some(ref fingerprint_name) = config.tls_fingerprint {
            log::info!("[Coordinator] 设置 TLS 指纹: {}", fingerprint_name);
            if let Err(e) = self.tls_fingerprint.set_by_name(fingerprint_name) {
                log::warn!("[Coordinator] 设置 TLS 指纹失败: {}", e);
            }
        }

        // 4. 启动出口 IP 监控
        if config.egress_monitor_enabled {
            log::info!("[Coordinator] 启动出口 IP 监控");
            self.egress_monitor.start();
        }

        // 5. 启动多路径路由
        if config.multipath_enabled {
            log::info!("[Coordinator] 启用多路径路由");
            // 多路径管理器已在创建时初始化
        }

        // 6. 启动 XDP（Linux）
        #[cfg(target_os = "linux")]
        if config.xdp_enabled {
            log::info!("[Coordinator] 启动 XDP 代理");
            if let Err(e) = self.xdp_manager.start() {
                log::error!("[Coordinator] XDP 启动失败: {}", e);
                return Err(e);
            }
        }

        log::info!("[Coordinator] 初始化完成");
        Ok(())
    }


    /// 更新配置
    pub fn update_config(&self, new_config: CoordinatorConfig) -> Result<()> {
        let mut config = self.config.write();
        
        // 检查变化并应用
        let security_changed = config.security_enabled != new_config.security_enabled;
        let _anti_probe_changed = config.anti_probe_enabled != new_config.anti_probe_enabled;
        let tls_changed = config.tls_fingerprint != new_config.tls_fingerprint;
        let _multipath_changed = config.multipath_enabled != new_config.multipath_enabled;
        let egress_monitor_changed = config.egress_monitor_enabled != new_config.egress_monitor_enabled;

        #[cfg(target_os = "linux")]
        let xdp_changed = config.xdp_enabled != new_config.xdp_enabled;

        *config = new_config.clone();
        drop(config);

        // 应用变化
        if security_changed {
            if new_config.security_enabled {
                self.security_monitor.start();
            } else {
                self.security_monitor.stop();
            }
        }

        if tls_changed {
            if let Some(ref fingerprint_name) = new_config.tls_fingerprint {
                if let Err(e) = self.tls_fingerprint.set_by_name(fingerprint_name) {
                    return Err(anyhow::anyhow!(e));
                }
            } else {
                self.tls_fingerprint.clear();
            }
        }

        if egress_monitor_changed {
            if new_config.egress_monitor_enabled {
                log::info!("[Coordinator] 启动出口 IP 监控");
                self.egress_monitor.start();
            } else {
                log::info!("[Coordinator] 停止出口 IP 监控");
                self.egress_monitor.stop();
            }
        }

        #[cfg(target_os = "linux")]
        if xdp_changed {
            if new_config.xdp_enabled {
                self.xdp_manager.start()?;
            } else {
                self.xdp_manager.stop()?;
            }
        }

        log::info!("[Coordinator] 配置已更新");
        Ok(())
    }

    /// 获取配置
    pub fn get_config(&self) -> CoordinatorConfig {
        self.config.read().clone()
    }

    /// 获取安全监控器
    #[allow(dead_code)]
    pub fn security_monitor(&self) -> Arc<SecurityMonitor> {
        self.security_monitor.clone()
    }

    /// 获取反探测服务
    pub fn anti_probe(&self) -> Arc<AntiProbeService> {
        self.anti_probe.clone()
    }

    /// 获取 TLS 指纹服务
    #[allow(dead_code)]
    pub fn tls_fingerprint(&self) -> Arc<TlsFingerprintService> {
        self.tls_fingerprint.clone()
    }

    /// 获取多路径管理器
    pub fn multipath_manager(&self) -> Arc<MultipathManager> {
        self.multipath_manager.clone()
    }

    pub fn egress_identity_manager(&self) -> Arc<EgressIdentityManager> {
        self.egress_identity_manager.clone()
    }

    #[allow(dead_code)]
    pub fn egress_monitor(&self) -> Arc<EgressMonitor> {
        self.egress_monitor.clone()
    }

    /// 获取 XDP 管理器（Linux）
    #[cfg(target_os = "linux")]
    pub fn xdp_manager(&self) -> Arc<XdpManager> {
        self.xdp_manager.clone()
    }

    /// 关闭协调器
    pub fn shutdown(&self) -> Result<()> {
        log::info!("[Coordinator] 开始关闭");

        // 停止出口 IP 监控
        self.egress_monitor.stop();

        // 停止安全监控
        self.security_monitor.stop();

        // 停止 XDP（Linux）
        #[cfg(target_os = "linux")]
        if self.config.read().xdp_enabled {
            self.xdp_manager.stop()?;
        }

        log::info!("[Coordinator] 关闭完成");
        Ok(())
    }
}

impl Default for CoreCoordinator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_coordinator_creation() {
        let coordinator = CoreCoordinator::new();
        let config = coordinator.get_config();
        assert!(!config.security_enabled);
        assert!(!config.anti_probe_enabled);
    }

    #[test]
    fn test_config_update() {
        let coordinator = CoreCoordinator::new();
        
        let mut config = CoordinatorConfig::default();
        config.security_enabled = true;
        config.anti_probe_enabled = true;
        
        coordinator.update_config(config.clone()).unwrap();
        
        let updated = coordinator.get_config();
        assert!(updated.security_enabled);
        assert!(updated.anti_probe_enabled);
    }
}
