/**
 * 安全防御模块
 * 
 * 包含：
 * 1. 反调试检测
 * 2. 内存蜜罐
 * 3. 配置文件欺骗
 * 4. 自毁机制
 */

pub mod anti_debug;
pub mod memory_honeypot;
pub mod config_decoy;
pub mod self_destruct;

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

/// 安全状态
static SECURITY_COMPROMISED: AtomicBool = AtomicBool::new(false);

/// 检查安全状态是否被破坏
pub fn is_security_compromised() -> bool {
    SECURITY_COMPROMISED.load(Ordering::Relaxed)
}

/// 标记安全状态为已破坏
pub fn mark_security_compromised() {
    SECURITY_COMPROMISED.store(true, Ordering::Relaxed);
}

/// 安全监控服务
pub struct SecurityMonitor {
    enabled: Arc<AtomicBool>,
}

impl SecurityMonitor {
    pub fn new() -> Self {
        Self {
            enabled: Arc::new(AtomicBool::new(false)),
        }
    }

    /// 启动安全监控
    pub fn start(&self) {
        self.enabled.store(true, Ordering::Relaxed);
        
        // 启动反调试监控
        let enabled = self.enabled.clone();
        std::thread::spawn(move || {
            anti_debug::monitor_loop(enabled);
        });

        // 启动内存蜜罐监控
        let enabled = self.enabled.clone();
        std::thread::spawn(move || {
            memory_honeypot::monitor_loop(enabled);
        });
    }

    /// 停止安全监控
    pub fn stop(&self) {
        self.enabled.store(false, Ordering::Relaxed);
    }
}

impl Default for SecurityMonitor {
    fn default() -> Self {
        Self::new()
    }
}
