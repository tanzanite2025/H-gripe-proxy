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
            multipath_enabled: false,
            #[cfg(target_os = "linux")]
            xdp_enabled: false,
        }
    }
}

/// 连接请求
#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct ConnectionRequest {
    /// 客户端 IP
    pub client_ip: std::net::IpAddr,
    /// 握手 token
    pub token: String,
    /// 目标域名
    pub domain: String,
    /// 会话 ID
    pub session_id: u64,
}

/// 连接决策
#[allow(dead_code)]
#[derive(Debug, Clone)]
pub enum ConnectionDecision {
    /// 接受连接
    Accept {
        /// 路由节点
        route: Option<String>,
        /// TLS 指纹
        tls_fingerprint: Option<String>,
    },
    /// 拒绝连接
    Reject,
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
            #[cfg(target_os = "linux")]
            xdp_manager: Arc::new(XdpManager::new()),
            config: Arc::new(RwLock::new(CoordinatorConfig::default())),
        }
    }

    /// 初始化所有模块
    pub fn initialize(&self) -> Result<()> {
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

        // 4. 启动多路径路由
        if config.multipath_enabled {
            log::info!("[Coordinator] 启用多路径路由");
            // 多路径管理器已在创建时初始化
        }

        // 5. 启动 XDP（Linux）
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

    /// 处理连接请求
    #[allow(dead_code)]
    pub fn handle_connection(&self, request: ConnectionRequest) -> Result<ConnectionDecision> {
        let config = self.config.read();

        // 1. 安全检查
        if config.security_enabled && crate::security::is_security_compromised() {
            log::error!("[Coordinator] 安全状态已被破坏，拒绝连接");
            return Ok(ConnectionDecision::Reject);
        }

        // 2. 反探测验证
        if config.anti_probe_enabled {
            if !self.anti_probe.verify_handshake(&request.client_ip, &request.token) {
                log::warn!("[Coordinator] 反探测验证失败: {}", request.client_ip);
                return Ok(ConnectionDecision::Reject);
            }
            log::debug!("[Coordinator] 反探测验证通过: {}", request.client_ip);
        }

        // 3. 路由决策
        let route = if config.multipath_enabled {
            let node = self.multipath_manager.select_node(&request.domain, request.session_id);
            if let Some(ref node_name) = node {
                log::debug!("[Coordinator] 选择节点: {} for {}", node_name, request.domain);
            }
            node
        } else {
            None
        };

        // 4. 获取 TLS 指纹
        let tls_fingerprint = self.tls_fingerprint.get_current()
            .map(|f| f.name.clone());

        // 5. 返回决策
        Ok(ConnectionDecision::Accept {
            route,
            tls_fingerprint,
        })
    }

    /// 更新配置
    pub fn update_config(&self, new_config: CoordinatorConfig) -> Result<()> {
        let mut config = self.config.write();
        
        // 检查变化并应用
        let security_changed = config.security_enabled != new_config.security_enabled;
        let _anti_probe_changed = config.anti_probe_enabled != new_config.anti_probe_enabled;
        let tls_changed = config.tls_fingerprint != new_config.tls_fingerprint;
        let _multipath_changed = config.multipath_enabled != new_config.multipath_enabled;
        
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

    /// 获取 XDP 管理器（Linux）
    #[cfg(target_os = "linux")]
    pub fn xdp_manager(&self) -> Arc<XdpManager> {
        self.xdp_manager.clone()
    }

    /// 关闭协调器
    pub fn shutdown(&self) -> Result<()> {
        log::info!("[Coordinator] 开始关闭");

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
