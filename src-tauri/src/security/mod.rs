/**
 * 安全模块
 * 
 * 包含：
 * - local_security: 本地安全监控
 * - firewall: 防火墙管理
 * - leak_monitor: 泄漏监控循环
 */

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use tauri::Emitter;

pub mod anti_debug;
pub mod config_decoy;
pub mod local_security;
pub mod firewall;
pub mod leak_monitor;
pub mod memory_honeypot;
pub mod self_destruct;

static SECURITY_COMPROMISED: AtomicBool = AtomicBool::new(false);

#[derive(Debug)]
pub struct SecurityMonitor {
    running: Arc<AtomicBool>,
}

impl SecurityMonitor {
    pub fn new() -> Self {
        Self {
            running: Arc::new(AtomicBool::new(false)),
        }
    }

    pub fn start(&self) {
        self.running.store(true, Ordering::Relaxed);
    }

    pub fn stop(&self) {
        self.running.store(false, Ordering::Relaxed);
    }

    #[allow(dead_code)]
    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::Relaxed)
    }
}

pub fn mark_security_compromised() {
    SECURITY_COMPROMISED.store(true, Ordering::SeqCst);
    emit_security_alert();
}

/// Emit a security-alert event to the frontend so it can react without polling
fn emit_security_alert() {
    let compromised = SECURITY_COMPROMISED.load(Ordering::SeqCst);
    #[derive(serde::Serialize)]
    struct AlertPayload {
        compromised: bool,
    }
    if let Err(e) = crate::core::handle::Handle::app_handle().emit("security-alert", &AlertPayload { compromised }) {
        log::warn!("Failed to emit security-alert event: {}", e);
    }
}

pub fn is_security_compromised() -> bool {
    SECURITY_COMPROMISED.load(Ordering::SeqCst)
}

#[allow(dead_code)]
pub fn reset_security_compromised() {
    SECURITY_COMPROMISED.store(false, Ordering::SeqCst);
}

#[allow(unused_imports)]
pub use local_security::{LocalSecurityConfig, LocalSecurityMonitor, LeakMonitorStatus, SecurityError};
#[allow(unused_imports)]
pub use firewall::{FirewallManager, FirewallRule, Protocol, Action};
#[allow(unused_imports)]
pub use leak_monitor::{LeakMonitor, LeakType, detect_leak_types};
