/**
 * 核心协调器 Tauri 命令
 */

use crate::config::AdvancedConfig;
use crate::core::coordinator::{CoreCoordinator, CoordinatorConfig};
use crate::core::{
    coordinator_status::CoordinatorStatus,
    stable_egress::project_runtime_status,
};
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

pub fn sync_coordinator_from_advanced_config() -> Result<(), String> {
    use crate::utils::dirs;

    let path = dirs::app_home_dir()
        .map_err(|e| e.to_string())?
        .join("advanced.yaml");
    let config = AdvancedConfig::load(&path).map_err(|e| e.to_string())?;
    COORDINATOR
        .hydrate_from_advanced_config(&config)
        .map_err(|e| e.to_string())
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
pub async fn save_advanced_config(config: AdvancedConfig) -> Result<(), String> {
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

    crate::cmd::clash::save_dns_config_mapping(&config.dns.to_dns_config_mapping()).await?;

    // 同步流量混淆配置：优先使用 traffic_obfuscation，兼容旧 traffic_padding
    let obf_config = if config.traffic_obfuscation.enabled {
        config.traffic_obfuscation.clone()
    } else if config.traffic_padding.enabled {
        crate::traffic::TrafficObfuscationConfig::from_legacy_padding(&config.traffic_padding)
    } else {
        config.traffic_obfuscation.clone()
    };
    crate::cmd::traffic::apply_traffic_obfuscation_config(obf_config).await?;
    
    // 应用到协调器
    COORDINATOR.apply_advanced_config(&config)
        .map_err(|e| e.to_string())?;

    crate::cmd::session_affinity::get_session_affinity_manager()
        .update_config(config.session_affinity)
        .await
        .map_err(|e| e.to_string())?;
    
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
pub async fn coordinator_get_status() -> Result<CoordinatorStatus, String> {
    let _ = sync_coordinator_from_advanced_config();
    let config = COORDINATOR.get_config();
    // 读取反探测与出站身份配置以保持同步
    let _anti_probe_cfg = COORDINATOR.anti_probe().get_config();
    let _egress_cfg = COORDINATOR.egress_identity_manager().get_config();
    let security_compromised = crate::security::is_security_compromised();
    let runtime_state = project_runtime_status(
        COORDINATOR.egress_identity_manager().get_active_assignments(),
        crate::cmd::session_affinity::get_session_affinity_manager()
            .get_all_bindings()
            .await
            .map_err(|e| e.to_string())?,
    )
    .await;
    let egress_identity_active_assignments = runtime_state.egress_identity_assignments.len();
    let session_affinity_active_bindings = runtime_state.session_affinity_bindings.len();
    
    Ok(CoordinatorStatus {
        initialized: true,
        security_enabled: config.security_enabled,
        security_compromised,
        anti_probe_enabled: config.anti_probe_enabled,
        tls_fingerprint: config.tls_fingerprint.clone(),
        egress_identity_enabled: config.egress_identity_enabled,
        session_affinity_enabled: config.session_affinity_enabled,
        egress_identity_active_assignments,
        session_affinity_active_bindings,
        runtime_state,
        multipath_enabled: config.multipath_enabled,
        #[cfg(target_os = "linux")]
        xdp_enabled: config.xdp_enabled,
        #[cfg(target_os = "linux")]
        xdp_running: COORDINATOR.xdp_manager().is_running(),
    })
}

