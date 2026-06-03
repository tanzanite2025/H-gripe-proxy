use anyhow::Result;
use parking_lot::RwLock;
/**
 * 核心协调器
 *
 * 统一管理所有高级功能模块：
 * - 安全防御层（反探测、TLS 指纹、内生欺骗）
 * - 路由决策层（多路径路由）
 * - 数据平面层（XDP 代理）
 */
use std::sync::Arc;

use crate::anti_probe::{AntiProbeConfig, AntiProbeService};
use crate::config::AdvancedConfig;
use crate::core::egress_identity::EgressIdentityManager;
use crate::core::egress_monitor::EgressMonitor;
use crate::core::security_policy::get_security_policy_manager;
use crate::multipath::MultipathManager;
use crate::security::SecurityMonitor;
use crate::security::ingress_countermeasure::IngressCountermeasureRuntime;
use crate::tls_fingerprint::TlsFingerprintService;

#[cfg(target_os = "linux")]
use crate::xdp::XdpManager;

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
    ingress_countermeasure: Arc<IngressCountermeasureRuntime>,
    egress_identity_manager: Arc<EgressIdentityManager>,
    egress_monitor: Arc<EgressMonitor>,
    /// XDP 管理器（Linux）
    #[cfg(target_os = "linux")]
    xdp_manager: Arc<XdpManager>,
    /// 完整高级配置（内存缓存，避免各模块各自 load_default 读磁盘）
    advanced_config: Arc<RwLock<AdvancedConfig>>,
}

impl CoreCoordinator {
    /// 创建新的协调器
    pub fn new() -> Self {
        Self {
            security_monitor: Arc::new(SecurityMonitor::new()),
            anti_probe: Arc::new(AntiProbeService::new(AntiProbeConfig::default())),
            tls_fingerprint: Arc::new(TlsFingerprintService::new()),
            multipath_manager: Arc::new(MultipathManager::new()),
            ingress_countermeasure: Arc::new(IngressCountermeasureRuntime::new(
                AdvancedConfig::default().ingress_countermeasure,
            )),
            egress_identity_manager: Arc::new(EgressIdentityManager::new()),
            egress_monitor: Arc::new(EgressMonitor::new()),
            #[cfg(target_os = "linux")]
            xdp_manager: Arc::new(XdpManager::new()),
            advanced_config: Arc::new(RwLock::new(AdvancedConfig::default())),
        }
    }

    pub fn hydrate_from_advanced_config(&self, config: &AdvancedConfig) -> Result<()> {
        self.sync_security_policies_from_advanced_config(config);
        self.apply_sub_configs(config);
        *self.advanced_config.write() = config.clone();
        Ok(())
    }

    /// 获取当前内存中的完整高级配置（只读快照）
    pub fn get_advanced_config(&self) -> AdvancedConfig {
        self.advanced_config.read().clone()
    }

    fn load_persisted_advanced_config(&self) -> Result<()> {
        let path = AdvancedConfig::default_path()?;
        let config = AdvancedConfig::load(&path)?;
        self.hydrate_from_advanced_config(&config)
    }

    pub fn apply_advanced_config(&self, config: &AdvancedConfig) -> Result<()> {
        let old = self.advanced_config.read().clone();
        self.sync_security_policies_from_advanced_config(config);
        self.apply_sub_configs(config);
        *self.advanced_config.write() = config.clone();
        self.apply_runtime_changes(&old, config)
    }

    fn sync_security_policies_from_advanced_config(&self, config: &AdvancedConfig) {
        get_security_policy_manager().sync_policies_from_config(config.security_policies.clone());
    }

    /// 将 AdvancedConfig 中的子配置分发到各管理器
    fn apply_sub_configs(&self, config: &AdvancedConfig) {
        self.anti_probe.update_config(config.security.anti_probe.clone());
        self.multipath_manager.update_config(config.multipath.clone());
        self.ingress_countermeasure
            .update_config(config.ingress_countermeasure.clone());
        if let Err(e) = self
            .egress_identity_manager
            .update_config(config.egress_identity.clone())
        {
            log::warn!("[Coordinator] 更新 egress_identity 配置失败: {}", e);
        }
        if let Err(e) = self.egress_monitor.update_config(config.egress_monitor.clone()) {
            log::warn!("[Coordinator] 更新 egress_monitor 配置失败: {}", e);
        }
        #[cfg(target_os = "linux")]
        self.xdp_manager.update_config(config.xdp.clone());
    }

    /// 对比新旧配置，应用运行时副作用（启停服务）
    fn apply_runtime_changes(&self, old: &AdvancedConfig, new: &AdvancedConfig) -> Result<()> {
        // 安全监控
        if old.security.enabled != new.security.enabled {
            if new.security.enabled {
                self.security_monitor.start();
            } else {
                self.security_monitor.stop();
            }
        }

        // TLS 指纹
        if old.security.tls_fingerprint != new.security.tls_fingerprint {
            if let Some(ref name) = new.security.tls_fingerprint {
                if let Err(e) = self.tls_fingerprint.set_by_name(name) {
                    return Err(anyhow::anyhow!(e));
                }
            } else {
                self.tls_fingerprint.clear();
            }
        }

        // 出口 IP 监控
        if old.egress_monitor.enabled != new.egress_monitor.enabled {
            if new.egress_monitor.enabled {
                log::info!("[Coordinator] 启动出口 IP 监控");
                self.egress_monitor.start();
            } else {
                log::info!("[Coordinator] 停止出口 IP 监控");
                self.egress_monitor.stop();
            }
        }

        // XDP（Linux）
        #[cfg(target_os = "linux")]
        if old.xdp.enabled != new.xdp.enabled {
            if new.xdp.enabled {
                self.xdp_manager.start()?;
            } else {
                self.xdp_manager.stop()?;
            }
        }

        log::info!("[Coordinator] 配置已更新");
        Ok(())
    }

    pub fn initialize(&self) -> Result<()> {
        self.load_persisted_advanced_config()?;
        let config = self.advanced_config.read();

        // 1. 启动安全监控
        if config.security.enabled {
            log::info!("[Coordinator] 启动安全监控");
            self.security_monitor.start();
        }

        // 2. 配置反探测
        if config.security.anti_probe.enabled {
            log::info!("[Coordinator] 启用反探测");
            // 反探测服务已在创建时初始化
        }

        // 3. 设置 TLS 指纹
        if let Some(ref fingerprint_name) = config.security.tls_fingerprint {
            log::info!("[Coordinator] 设置 TLS 指纹: {}", fingerprint_name);
            if let Err(e) = self.tls_fingerprint.set_by_name(fingerprint_name) {
                log::warn!("[Coordinator] 设置 TLS 指纹失败: {}", e);
            }
        }

        // 4. 启动出口 IP 监控
        if config.egress_monitor.enabled {
            log::info!("[Coordinator] 启动出口 IP 监控");
            self.egress_monitor.start();
        }

        // 5. 启动多路径路由
        if config.multipath.enabled {
            log::info!("[Coordinator] 启用多路径路由");
            // 多路径管理器已在创建时初始化
        }

        // 6. 启动 XDP（Linux）
        #[cfg(target_os = "linux")]
        if config.xdp.enabled {
            log::info!("[Coordinator] 启动 XDP 代理");
            if let Err(e) = self.xdp_manager.start() {
                log::error!("[Coordinator] XDP 启动失败: {}", e);
                return Err(e);
            }
        }

        log::info!("[Coordinator] 初始化完成");
        Ok(())
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

    pub fn ingress_countermeasure(&self) -> Arc<IngressCountermeasureRuntime> {
        self.ingress_countermeasure.clone()
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
        if self.advanced_config.read().xdp.enabled {
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
