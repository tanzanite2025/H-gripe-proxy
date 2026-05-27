/**
 * 核心协调器 Tauri 命令
 */

use crate::core::coordinator::{CoreCoordinator, CoordinatorConfig};
use crate::config::AdvancedConfig;
use once_cell::sync::Lazy;
use std::sync::Arc;

/// 全局协调器实例
static COORDINATOR: Lazy<Arc<CoreCoordinator>> = Lazy::new(|| {
    Arc::new(CoreCoordinator::new())
});

/// 获取协调器实例
pub fn get_coordinator() -> Arc<CoreCoordinator> {
    COORDINATOR.clone()
}

/// 初始化协调器
#[tauri::command]
pub fn coordinator_initialize() -> Result<(), String> {
    COORDINATOR.initialize()
        .map_err(|e| e.to_string())
}

/// 获取协调器配置
#[tauri::command]
pub fn coordinator_get_config() -> Result<CoordinatorConfig, String> {
    Ok(COORDINATOR.get_config())
}

/// 更新协调器配置
#[tauri::command]
pub fn coordinator_update_config(config: CoordinatorConfig) -> Result<(), String> {
    COORDINATOR.update_config(config)
        .map_err(|e| e.to_string())
}

/// 关闭协调器
#[tauri::command]
pub fn coordinator_shutdown() -> Result<(), String> {
    COORDINATOR.shutdown()
        .map_err(|e| e.to_string())
}

/// 获取高级配置
#[tauri::command]
pub fn get_advanced_config() -> Result<AdvancedConfig, String> {
    use crate::utils::dirs;
    let path = dirs::app_home_dir()
        .map_err(|e| e.to_string())?
        .join("advanced.yaml");
    
    AdvancedConfig::load(&path)
        .map_err(|e| e.to_string())
}

/// 保存高级配置
#[tauri::command]
pub fn save_advanced_config(config: AdvancedConfig) -> Result<(), String> {
    use crate::utils::dirs;
    
    // 验证配置
    config.validate()
        .map_err(|e| format!("配置验证失败: {}", e))?;
    
    // 保存到文件
    let path = dirs::app_home_dir()
        .map_err(|e| e.to_string())?
        .join("advanced.yaml");
    
    config.save(&path)
        .map_err(|e| e.to_string())?;
    
    // 应用到协调器
    let coordinator_config = CoordinatorConfig {
        security_enabled: config.security.enabled,
        anti_probe_enabled: config.security.anti_probe.enabled,
        tls_fingerprint: config.security.tls_fingerprint.clone(),
        multipath_enabled: config.multipath.enabled,
        #[cfg(target_os = "linux")]
        xdp_enabled: config.xdp.enabled,
    };
    
    COORDINATOR.update_config(coordinator_config)
        .map_err(|e| e.to_string())?;
    
    // 更新各个服务的配置
    if config.security.anti_probe.enabled {
        COORDINATOR.anti_probe().update_config(config.security.anti_probe);
    }
    
    if config.multipath.enabled {
        COORDINATOR.multipath_manager().update_config(config.multipath);
    }
    
    #[cfg(target_os = "linux")]
    if config.xdp.enabled {
        COORDINATOR.xdp_manager().update_config(config.xdp)
            .map_err(|e| e.to_string())?;
    }
    
    Ok(())
}

/// 获取推荐配置
#[tauri::command]
pub fn get_recommended_advanced_config() -> Result<AdvancedConfig, String> {
    Ok(AdvancedConfig::recommended())
}

/// 验证高级配置
#[tauri::command]
pub fn validate_advanced_config(config: AdvancedConfig) -> Result<(), String> {
    config.validate()
        .map_err(|e| e.to_string())
}

/// 获取协调器状态
#[tauri::command]
pub fn coordinator_get_status() -> Result<CoordinatorStatus, String> {
    let config = COORDINATOR.get_config();
    let security_compromised = crate::security::is_security_compromised();
    
    Ok(CoordinatorStatus {
        initialized: true,
        security_enabled: config.security_enabled,
        security_compromised,
        anti_probe_enabled: config.anti_probe_enabled,
        tls_fingerprint: config.tls_fingerprint.clone(),
        multipath_enabled: config.multipath_enabled,
        #[cfg(target_os = "linux")]
        xdp_enabled: config.xdp_enabled,
        #[cfg(target_os = "linux")]
        xdp_running: COORDINATOR.xdp_manager().is_running(),
    })
}

/// 协调器状态
#[derive(Debug, Clone, serde::Serialize)]
pub struct CoordinatorStatus {
    pub initialized: bool,
    pub security_enabled: bool,
    pub security_compromised: bool,
    pub anti_probe_enabled: bool,
    pub tls_fingerprint: Option<String>,
    pub multipath_enabled: bool,
    #[cfg(target_os = "linux")]
    pub xdp_enabled: bool,
    #[cfg(target_os = "linux")]
    pub xdp_running: bool,
}
