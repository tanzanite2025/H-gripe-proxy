/**
 * 泄漏监控循环模块
 *
 * 功能：
 * 1. 定时监控循环 - 每 30 秒检查一次
 * 2. 泄漏检测 - 检测本地绑定、防火墙、外部访问泄漏
 * 3. 自动修复 - 修复检测到的泄漏问题
 * 4. 事件发送 - 向前端发送状态更新
 */
use anyhow::Result;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;
use tokio::sync::RwLock;
use tokio::time;

use super::local_security::LocalSecurityConfig;
use super::local_security::{LeakMonitorStatus, LocalSecurityMonitor};

/// 泄漏监控器
pub struct LeakMonitor {
    monitor: Arc<LocalSecurityMonitor>,
    running: Arc<AtomicBool>,
    port: Arc<RwLock<u16>>,
}

impl LeakMonitor {
    /// 创建新的泄漏监控器
    pub fn new(monitor: Arc<LocalSecurityMonitor>, port: u16) -> Self {
        Self {
            monitor,
            running: Arc::new(AtomicBool::new(false)),
            port: Arc::new(RwLock::new(port)),
        }
    }

    /// 启动监控循环
    pub async fn start(&self) -> Result<()> {
        if self.running.load(Ordering::SeqCst) {
            log::warn!("Leak monitor is already running");
            return Ok(());
        }

        self.running.store(true, Ordering::SeqCst);
        log::info!("🔍 Starting leak monitor");

        let monitor = self.monitor.clone();
        let running = self.running.clone();
        let port = self.port.clone();

        let config = monitor.get_config().await;

        tokio::spawn(async move {
            Self::monitor_loop(monitor, running, port, config).await;
        });

        Ok(())
    }

    /// 停止监控循环
    pub async fn stop(&self) {
        self.running.store(false, Ordering::SeqCst);
        log::info!("🛑 Stopping leak monitor");
    }

    /// 更新监控端口
    pub async fn set_port(&self, new_port: u16) {
        let mut port = self.port.write().await;
        *port = new_port;
        log::info!("📝 Leak monitor port updated to {}", new_port);
    }

    /// 监控循环
    async fn monitor_loop(
        monitor: Arc<LocalSecurityMonitor>,
        running: Arc<AtomicBool>,
        port: Arc<RwLock<u16>>,
        config: LocalSecurityConfig,
    ) {
        let interval = Duration::from_secs(config.monitor_interval);

        while running.load(Ordering::SeqCst) {
            let current_port = *port.read().await;

            // 执行安全检查
            match monitor.perform_security_check(current_port).await {
                Ok(status) => {
                    // 检查是否检测到泄漏
                    if status.leak_detected {
                        let leak_types = super::leak_monitor::detect_leak_types(&status);
                        let leak_desc: Vec<&str> = leak_types.iter().map(|t| t.as_str()).collect();
                        log::warn!("🚨 Security leak detected: {:?}", leak_desc);

                        // 尝试自动修复
                        if config.auto_firewall {
                            if let Err(e) = Self::auto_fix_leak(&monitor, current_port, &status).await {
                                log::error!("Failed to auto-fix leak: {}", e);
                            }
                        }
                    } else {
                        log::trace!("✅ Security check passed (port: {})", current_port);

                        // 可选：周期性验证/刷新防火墙规则（这里简单地重新应用一次）
                        if config.auto_firewall {
                            // 使用当前端口刷新规则，若端口被外部更新则读取最新值
                            let refreshed_port = *port.read().await;
                            if let Err(e) = monitor.configure_firewall(refreshed_port).await {
                                log::warn!("⚠️ Firewall rule refresh failed: {}", e);
                            }
                        }
                    }

                    // 发送安全警报事件到前端
                    if status.leak_detected {
                        super::mark_security_compromised();
                    }
                }
                Err(e) => {
                    log::error!("Security check failed: {}", e);
                }
            }

            // 等待下一次检查
            time::sleep(interval).await;
        }

        log::info!("Leak monitor loop stopped");
    }

    /// 自动修复泄漏
    async fn auto_fix_leak(monitor: &Arc<LocalSecurityMonitor>, port: u16, status: &LeakMonitorStatus) -> Result<()> {
        log::info!("🔧 Attempting to auto-fix security leak");

        // 1. 如果本地绑定不安全，记录警告（无法自动修复）
        if !status.local_binding_secure {
            log::warn!("⚠️ Local binding is not secure - manual intervention required");
        }

        // 2. 如果防火墙规则未生效，尝试重新配置
        if !status.firewall_rules_active {
            log::info!("🔧 Reconfiguring firewall rules");
            monitor.configure_firewall(port).await?;
            log::info!("✅ Firewall rules reconfigured");
        }

        // 3. 如果外部访问未被阻止，尝试重新配置防火墙
        if !status.external_access_blocked {
            log::info!("🔧 Blocking external access");
            monitor.configure_firewall(port).await?;
            log::info!("✅ External access blocked");
        }

        Ok(())
    }

    /// 检查监控器是否正在运行
    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::SeqCst)
    }

    /// 获取当前监控端口
    pub async fn get_port(&self) -> u16 {
        *self.port.read().await
    }
}

/// 泄漏类型
#[derive(Debug, Clone)]
pub enum LeakType {
    /// 非本地绑定
    NonLocalBinding,
    /// 防火墙规则未生效
    FirewallInactive,
    /// 外部访问未被阻止
    ExternalAccessNotBlocked,
    /// 进程未隐蔽
    ProcessNotHidden,
}

impl LeakType {
    pub fn as_str(&self) -> &str {
        match self {
            LeakType::NonLocalBinding => "Non-localhost binding detected",
            LeakType::FirewallInactive => "Firewall rules not active",
            LeakType::ExternalAccessNotBlocked => "External access not blocked",
            LeakType::ProcessNotHidden => "Process not hidden",
        }
    }
}

/// 检测泄漏类型
pub fn detect_leak_types(status: &LeakMonitorStatus) -> Vec<LeakType> {
    let mut leaks = Vec::new();

    if !status.local_binding_secure {
        leaks.push(LeakType::NonLocalBinding);
    }

    if !status.firewall_rules_active {
        leaks.push(LeakType::FirewallInactive);
    }

    if !status.external_access_blocked {
        leaks.push(LeakType::ExternalAccessNotBlocked);
    }

    if !status.process_hidden {
        leaks.push(LeakType::ProcessNotHidden);
    }

    leaks
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_leak_monitor_creation() {
        let config = LocalSecurityConfig::default();
        let monitor = Arc::new(LocalSecurityMonitor::new(config));
        let leak_monitor = LeakMonitor::new(monitor, 10808);

        assert!(!leak_monitor.is_running());
        assert_eq!(leak_monitor.get_port().await, 10808);
    }

    #[tokio::test]
    async fn test_leak_monitor_start_stop() {
        let config = LocalSecurityConfig::default();
        let monitor = Arc::new(LocalSecurityMonitor::new(config));
        let leak_monitor = LeakMonitor::new(monitor, 10808);

        // 启动监控
        leak_monitor.start().await.unwrap();
        assert!(leak_monitor.is_running());

        // 停止监控
        leak_monitor.stop().await;
        tokio::time::sleep(Duration::from_millis(100)).await;
        assert!(!leak_monitor.is_running());
    }

    #[tokio::test]
    async fn test_leak_monitor_port_update() {
        let config = LocalSecurityConfig::default();
        let monitor = Arc::new(LocalSecurityMonitor::new(config));
        let leak_monitor = LeakMonitor::new(monitor, 10808);

        assert_eq!(leak_monitor.get_port().await, 10808);

        leak_monitor.set_port(10809).await;
        assert_eq!(leak_monitor.get_port().await, 10809);
    }

    #[test]
    fn test_detect_leak_types() {
        let status = LeakMonitorStatus {
            local_binding_secure: false,
            firewall_rules_active: true,
            process_hidden: false,
            external_access_blocked: true,
            last_check_time: 0,
            leak_detected: true,
            leak_type: None,
            auto_fix_applied: false,
        };

        let leaks = detect_leak_types(&status);
        assert_eq!(leaks.len(), 2);
        assert!(matches!(leaks[0], LeakType::NonLocalBinding));
        assert!(matches!(leaks[1], LeakType::ProcessNotHidden));
    }

    #[test]
    fn test_leak_type_as_str() {
        assert_eq!(LeakType::NonLocalBinding.as_str(), "Non-localhost binding detected");
        assert_eq!(LeakType::FirewallInactive.as_str(), "Firewall rules not active");
        assert_eq!(
            LeakType::ExternalAccessNotBlocked.as_str(),
            "External access not blocked"
        );
        assert_eq!(LeakType::ProcessNotHidden.as_str(), "Process not hidden");
    }

    #[tokio::test]
    async fn test_monitor_loop_short_run() {
        let config = LocalSecurityConfig {
            monitor_interval: 1, // 1秒间隔用于测试
            ..Default::default()
        };
        let monitor = Arc::new(LocalSecurityMonitor::new(config));
        let leak_monitor = LeakMonitor::new(monitor, 65450);

        // 启动监控
        leak_monitor.start().await.unwrap();
        assert!(leak_monitor.is_running());

        // 运行 2 秒
        tokio::time::sleep(Duration::from_secs(2)).await;

        // 停止监控
        leak_monitor.stop().await;
        tokio::time::sleep(Duration::from_millis(100)).await;
        assert!(!leak_monitor.is_running());
    }
}
