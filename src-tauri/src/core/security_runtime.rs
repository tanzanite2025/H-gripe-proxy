use crate::security::{
    SecurityMonitor, anti_debug, honeypot, leak_monitor::LeakMonitor, local_security::LocalSecurityMonitor,
    local_stealth::LocalStealthManager,
};
use anyhow::{Result, anyhow, bail};
use once_cell::sync::Lazy;
use parking_lot::RwLock;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SecurityStatus {
    pub compromised: bool,
    pub debugger_present: bool,
    pub memory_scanning: bool,
    pub leak_detected: bool,
    pub leak_type: Option<String>,
    pub anti_debug_enabled: bool,
    pub suspicious_parent: bool,
}

static SECURITY_MONITOR: Lazy<Arc<RwLock<SecurityMonitor>>> =
    Lazy::new(|| Arc::new(RwLock::new(SecurityMonitor::new())));

static LOCAL_SECURITY_MONITOR: Lazy<Arc<LocalSecurityMonitor>> =
    Lazy::new(|| Arc::new(LocalSecurityMonitor::new(LocalSecurityConfig::default())));

static LEAK_MONITOR: Lazy<Arc<tokio::sync::RwLock<Option<LeakMonitor>>>> =
    Lazy::new(|| Arc::new(tokio::sync::RwLock::new(None)));

static HONEYPOT_FLAG: Lazy<Arc<AtomicBool>> = Lazy::new(|| Arc::new(AtomicBool::new(false)));

static ANTI_DEBUG_FLAG: Lazy<Arc<AtomicBool>> = Lazy::new(|| Arc::new(AtomicBool::new(false)));

static LOCAL_STEALTH_MANAGER: Lazy<Arc<tokio::sync::RwLock<LocalStealthManager>>> = Lazy::new(|| {
    Arc::new(tokio::sync::RwLock::new(LocalStealthManager::new(
        LocalStealthConfig::default(),
    )))
});

pub use crate::security::local_security::{LeakMonitorStatus, LocalSecurityConfig};
pub use crate::security::local_stealth::{LocalStealthConfig, StealthApplyResult};

// ---------- Security monitor control ----------
pub async fn start_monitor() {
    {
        let monitor = SECURITY_MONITOR.read();
        monitor.start();
    }

    // 初始化内存蜜罐以检测内存扫描（从 coordinator 内存配置读取）
    let hp_cfg = crate::feat::get_coordinator().get_advanced_config().security.honeypot;
    if hp_cfg.enabled {
        honeypot::init_global_honeypot_with_count(hp_cfg.token_count);
    } else {
        honeypot::init_global_honeypot();
    }

    // 启动蜜罐监控线程（幂等）
    if !HONEYPOT_FLAG.load(Ordering::SeqCst) {
        HONEYPOT_FLAG.store(true, Ordering::SeqCst);
        let flag = HONEYPOT_FLAG.clone();
        std::thread::spawn(move || {
            crate::security::honeypot::monitor_loop(flag);
        });
    }

    // 启动反调试监控线程（幂等）
    if !ANTI_DEBUG_FLAG.load(Ordering::SeqCst) {
        ANTI_DEBUG_FLAG.store(true, Ordering::SeqCst);
        let flag = ANTI_DEBUG_FLAG.clone();
        std::thread::spawn(move || {
            crate::security::anti_debug::monitor_loop(flag);
        });
    }

    // 自动启动泄漏监控：选择可用端口
    let port = LOCAL_SECURITY_MONITOR.find_available_port().await.unwrap_or(10808);
    let start_result = leak_monitor_start(port).await;

    if let Err(e) = start_result {
        log::warn!("Failed to start leak monitor: {}", e);
    }
}

pub fn stop_monitor() {
    let monitor = SECURITY_MONITOR.read();
    monitor.stop();

    // 停止蜜罐监控线程
    HONEYPOT_FLAG.store(false, Ordering::SeqCst);

    // 停止反调试监控线程
    ANTI_DEBUG_FLAG.store(false, Ordering::SeqCst);
}

pub async fn check_status() -> SecurityStatus {
    let honeypot_triggered = honeypot::check_global_honeypot();
    if honeypot_triggered {
        crate::security::mark_security_compromised();
    }

    // 轻量级父进程检查
    let suspicious_parent = crate::security::anti_debug::check_parent_process();
    if suspicious_parent {
        crate::security::mark_security_compromised();
    }

    let leak_status = LOCAL_SECURITY_MONITOR.get_status().await;
    let anti_cfg = anti_debug::AntiDebugConfig::default();

    SecurityStatus {
        compromised: crate::security::is_security_compromised(),
        debugger_present: crate::security::anti_debug::is_debugger_present() || suspicious_parent,
        memory_scanning: honeypot_triggered || crate::security::honeypot::detect_memory_scanning(),
        leak_detected: leak_status.leak_detected,
        leak_type: leak_status.leak_type,
        anti_debug_enabled: anti_cfg.enabled,
        suspicious_parent,
    }
}

// ---------- Config decoy ----------
pub fn deploy_decoy(decoy_path: PathBuf) -> Result<()> {
    let decoy = honeypot::ConfigDecoy::new(decoy_path);
    decoy.deploy().map_err(|e| anyhow!(e))
}

pub fn cleanup_decoy(decoy_path: PathBuf) -> Result<()> {
    let decoy = honeypot::ConfigDecoy::new(decoy_path);
    decoy.cleanup().map_err(|e| anyhow!(e))
}

pub fn check_decoy_access(decoy_path: PathBuf) -> Result<bool> {
    let decoy = honeypot::ConfigDecoy::new(decoy_path);
    Ok(decoy.check_access())
}

pub fn deploy_decoy_plan(plan: honeypot::DecoyDeploymentPlan) -> Result<honeypot::DecoyBatchResult> {
    honeypot::deploy_decoy_plan(plan).map_err(|e| anyhow!(e))
}

pub fn cleanup_decoy_plan(plan: honeypot::DecoyDeploymentPlan) -> Result<honeypot::DecoyBatchResult> {
    honeypot::cleanup_decoy_plan(plan).map_err(|e| anyhow!(e))
}

pub fn check_decoy_plan_access(plan: honeypot::DecoyDeploymentPlan) -> Result<honeypot::DecoyBatchResult> {
    honeypot::check_decoy_plan_access(plan).map_err(|e| anyhow!(e))
}

// ---------- Encryption ----------
pub fn generate_key() -> String {
    honeypot::generate_encryption_key()
}

pub fn encrypt_data(data: Vec<u8>) -> Result<Vec<u8>> {
    let storage = honeypot::SecureConfigStorage::new();
    if !storage.is_key_available() {
        anyhow::bail!("加密密钥未设置，请设置环境变量 CLASH_VERGE_SECURE_KEY");
    }
    storage.encrypt(&data).map_err(|e| anyhow!(e))
}

pub fn decrypt_data(data: Vec<u8>) -> Result<Vec<u8>> {
    let storage = honeypot::SecureConfigStorage::new();
    if !storage.is_key_available() {
        anyhow::bail!("加密密钥未设置，请设置环境变量 CLASH_VERGE_SECURE_KEY");
    }
    storage.decrypt(&data).map_err(|e| anyhow!(e))
}

pub fn is_key_available() -> bool {
    let storage = honeypot::SecureConfigStorage::new();
    storage.is_key_available()
}

// ---------- Local security monitor ----------
pub async fn local_security_get_config() -> LocalSecurityConfig {
    let monitor = LOCAL_SECURITY_MONITOR.clone();
    monitor.get_config().await
}

pub async fn local_security_update_config(config: LocalSecurityConfig) {
    let monitor = LOCAL_SECURITY_MONITOR.clone();
    monitor.update_config(config).await;
}

pub async fn local_security_get_status() -> LeakMonitorStatus {
    let monitor = LOCAL_SECURITY_MONITOR.clone();
    monitor.get_status().await
}

pub async fn local_security_check_now(port: u16) -> Result<LeakMonitorStatus> {
    let monitor = LOCAL_SECURITY_MONITOR.clone();
    monitor.perform_security_check(port).await.map_err(Into::into)
}

pub async fn local_security_check_binding(port: u16) -> Result<bool> {
    let monitor = LOCAL_SECURITY_MONITOR.clone();
    monitor.check_local_binding(port).await.map_err(Into::into)
}

pub async fn local_security_check_port_conflict(port: u16) -> Result<bool> {
    let monitor = LOCAL_SECURITY_MONITOR.clone();
    monitor.check_port_conflict(port).await.map_err(Into::into)
}

pub async fn local_security_find_available_port() -> Result<u16> {
    let monitor = LOCAL_SECURITY_MONITOR.clone();
    monitor.find_available_port().await.map_err(Into::into)
}

pub async fn local_security_configure_firewall(port: u16) -> Result<()> {
    let monitor = LOCAL_SECURITY_MONITOR.clone();
    monitor.configure_firewall(port).await.map_err(Into::into)
}

pub async fn local_security_remove_firewall(port: u16) -> Result<()> {
    let monitor = LOCAL_SECURITY_MONITOR.clone();
    monitor.remove_firewall_rules(port).await.map_err(Into::into)
}

// ---------- Leak monitor loop ----------
pub async fn leak_monitor_start(port: u16) -> Result<()> {
    let mut leak_monitor_guard = LEAK_MONITOR.write().await;

    // 如果已经在运行，先停止
    if let Some(monitor) = leak_monitor_guard.as_ref() {
        if monitor.is_running() {
            monitor.stop().await;
        }
    }

    // 创建新的监控器
    let monitor = LOCAL_SECURITY_MONITOR.clone();
    let config = monitor.get_config().await;
    let leak_monitor = LeakMonitor::new(monitor, port);

    // 若开启自动防火墙，先配置一次
    if config.auto_firewall {
        LOCAL_SECURITY_MONITOR
            .configure_firewall(port)
            .await
            .map_err(|e| anyhow!(e))?;
    }

    leak_monitor.start().await?;

    *leak_monitor_guard = Some(leak_monitor);
    Ok(())
}

pub async fn leak_monitor_stop() -> Result<()> {
    let leak_monitor_guard = LEAK_MONITOR.read().await;

    if let Some(monitor) = leak_monitor_guard.as_ref() {
        monitor.stop().await;

        // 停止时移除防火墙规则（如果之前启用自动防火墙）
        let cfg = LOCAL_SECURITY_MONITOR.get_config().await;
        if cfg.auto_firewall {
            LOCAL_SECURITY_MONITOR
                .remove_firewall_rules(monitor.get_port().await)
                .await
                .map_err(|e| anyhow!(e))?;
        }
    }

    Ok(())
}

pub async fn leak_monitor_is_running() -> bool {
    let leak_monitor_guard = LEAK_MONITOR.read().await;
    leak_monitor_guard.as_ref().map(|m| m.is_running()).unwrap_or(false)
}

pub async fn leak_monitor_set_port(port: u16) -> Result<()> {
    let leak_monitor_guard = LEAK_MONITOR.read().await;

    if let Some(monitor) = leak_monitor_guard.as_ref() {
        monitor.set_port(port).await;
    }

    Ok(())
}

pub async fn leak_monitor_get_port() -> Result<u16> {
    let leak_monitor_guard = LEAK_MONITOR.read().await;

    if let Some(monitor) = leak_monitor_guard.as_ref() {
        Ok(monitor.get_port().await)
    } else {
        bail!("泄漏监控未运行")
    }
}

// ---------- Local stealth ----------

pub async fn local_stealth_get_config() -> LocalStealthConfig {
    crate::config::AdvancedConfig::load_default().local_stealth
}

pub async fn apply_local_stealth_config(config: LocalStealthConfig) {
    let mut manager = LOCAL_STEALTH_MANAGER.write().await;
    manager.update_config(config).await;
}

pub async fn local_stealth_update_config(config: LocalStealthConfig) -> Result<()> {
    let mut advanced = crate::config::AdvancedConfig::load_default();
    advanced.local_stealth = config.clone();
    advanced.validate()?;
    advanced.save_default()?;
    crate::feat::get_coordinator().hydrate_from_advanced_config(&advanced)?;
    apply_local_stealth_config(config).await;
    Ok(())
}

pub async fn local_stealth_apply() -> Result<StealthApplyResult> {
    let mut manager = LOCAL_STEALTH_MANAGER.write().await;
    manager.apply_all().await.map_err(|e| anyhow!(e))
}

pub async fn local_stealth_restore() {
    let manager = LOCAL_STEALTH_MANAGER.read().await;
    manager.restore_all().await;
}

pub async fn local_stealth_allocate_port() -> Result<u16> {
    let manager = LOCAL_STEALTH_MANAGER.read().await;
    manager
        .port_manager()
        .allocate_stealth_port()
        .await
        .map_err(|e| anyhow!(e))
}

pub async fn local_stealth_get_port() -> Result<Option<u16>> {
    let manager = LOCAL_STEALTH_MANAGER.read().await;
    Ok(manager.port_manager().get_current_port().await)
}
