use crate::config::AdvancedConfig;
use crate::core::coordinator_status::CoordinatorStatus;
use crate::core::stable_egress::project_runtime_status;
use crate::core::{CoreManager, coordinator::CoreCoordinator};
use crate::traffic::TrafficObfuscationConfig;
use anyhow::Result;
use once_cell::sync::Lazy;
use std::sync::Arc;

/// 全局协调器实例
static COORDINATOR: Lazy<Arc<CoreCoordinator>> = Lazy::new(|| Arc::new(CoreCoordinator::new()));

/// 获取协调器实例
pub fn get_coordinator() -> Arc<CoreCoordinator> {
    COORDINATOR.clone()
}

/// 从磁盘重新加载 AdvancedConfig 并同步到 coordinator 内存
pub fn sync_coordinator_from_advanced_config() -> Result<()> {
    let path = AdvancedConfig::default_path()?;
    let config = AdvancedConfig::load(&path)?;
    COORDINATOR.hydrate_from_advanced_config(&config)
}

pub async fn sync_coordinator_from_advanced_config_async() -> Result<()> {
    let path = AdvancedConfig::default_path()?;
    let config = AdvancedConfig::load(&path)?;
    COORDINATOR.hydrate_from_advanced_config(&config)?;
    crate::feat::apply_egress_monitor_config(config.egress_monitor.clone()).await?;
    crate::feat::apply_ip_reputation_config(config.ip_reputation.clone()).await?;
    crate::feat::apply_blackhole_breaker_config(config.blackhole_breaker.clone()).await;
    crate::core::security_runtime::apply_local_stealth_config(config.local_stealth.clone()).await;
    let traffic_obfuscation = if config.traffic_obfuscation.enabled {
        config.traffic_obfuscation.clone()
    } else if config.traffic_padding.enabled {
        TrafficObfuscationConfig::from_legacy_padding(&config.traffic_padding)
    } else {
        config.traffic_obfuscation.clone()
    };
    crate::feat::apply_traffic_obfuscation_config(traffic_obfuscation).await?;
    crate::feat::get_session_affinity_manager()
        .update_config(config.session_affinity)
        .await?;
    Ok(())
}

/// 保存高级配置（业务逻辑）
pub async fn save_advanced_config(config: &AdvancedConfig) -> Result<()> {
    config.validate()?;

    let path = AdvancedConfig::default_path()?;
    config.save(&path)?;

    crate::feat::save_dns_config_mapping(&config.dns.to_dns_config_mapping()).await?;
    crate::feat::apply_egress_monitor_config(config.egress_monitor.clone()).await?;
    crate::feat::apply_ip_reputation_config(config.ip_reputation.clone()).await?;
    crate::feat::apply_blackhole_breaker_config(config.blackhole_breaker.clone()).await;
    crate::core::security_runtime::apply_local_stealth_config(config.local_stealth.clone()).await;
    #[cfg(target_os = "linux")]
    get_coordinator().xdp_manager().update_config(config.xdp.clone());

    // 同步流量混淆配置：优先使用 traffic_obfuscation，兼容旧 traffic_padding
    let obf_config = if config.traffic_obfuscation.enabled {
        config.traffic_obfuscation.clone()
    } else if config.traffic_padding.enabled {
        TrafficObfuscationConfig::from_legacy_padding(&config.traffic_padding)
    } else {
        config.traffic_obfuscation.clone()
    };
    crate::feat::apply_traffic_obfuscation_config(obf_config).await?;

    // 应用到协调器
    COORDINATOR.apply_advanced_config(config)?;

    crate::feat::get_session_affinity_manager()
        .update_config(config.session_affinity.clone())
        .await?;

    CoreManager::global().update_config_checked().await?;

    Ok(())
}

/// 获取协调器状态（业务逻辑）
pub async fn coordinator_get_status() -> Result<CoordinatorStatus> {
    let _ = sync_coordinator_from_advanced_config_async().await;
    let config = COORDINATOR.get_advanced_config();
    let security_compromised = crate::security::is_security_compromised();
    let runtime_state = project_runtime_status(
        COORDINATOR.egress_identity_manager().get_active_assignments(),
        crate::feat::get_session_affinity_manager().get_all_bindings().await?,
    )
    .await;
    let egress_identity_active_assignments = runtime_state.egress_identity_assignments.len();
    let session_affinity_active_bindings = runtime_state.session_affinity_bindings.len();

    Ok(CoordinatorStatus {
        initialized: true,
        security_enabled: config.security.enabled,
        security_compromised,
        anti_probe_enabled: config.security.anti_probe.enabled,
        tls_fingerprint: config.security.tls_fingerprint.clone(),
        egress_identity_enabled: config.egress_identity.enabled,
        session_affinity_enabled: config.session_affinity.enabled,
        egress_identity_active_assignments,
        session_affinity_active_bindings,
        runtime_state,
        multipath_enabled: config.multipath.enabled,
        traffic_obfuscation_enabled: config.traffic_obfuscation.enabled,
        honeypot_enabled: config.security.honeypot.enabled,
        self_destruct_enabled: config.security.self_destruct.enabled,
        #[cfg(target_os = "linux")]
        xdp_enabled: config.xdp.enabled,
        #[cfg(target_os = "linux")]
        xdp_running: COORDINATOR.xdp_manager().is_running(),
    })
}
