use anyhow::Result;
use once_cell::sync::Lazy;
use parking_lot::RwLock;
use std::collections::HashSet;
use std::sync::Arc;

use crate::anti_probe::{AntiProbeConfig, AntiProbeService};
use crate::config::AdvancedConfig;
use crate::core::coordinator_status::CoordinatorStatus;
use crate::core::dns_runtime::save_dns_config_mapping;
use crate::core::egress_identity::EgressIdentityManager;
use crate::core::egress_monitor::egress_monitor;
use crate::core::ip_reputation::{get_ip_reputation_manager, normalize_ip_reputation_config};
use crate::core::security_policy::{get_security_policy_manager, revoke_policy};
use crate::core::session_affinity::get_session_affinity_manager;
use crate::core::stable_egress::project_runtime_status;
use crate::core::traffic_runtime::{apply_traffic_obfuscation_config, effective_traffic_obfuscation_config};
use crate::multipath::MultipathManager;
use crate::process::AsyncHandler;
use crate::security::SecurityMonitor;
use crate::security::ingress_countermeasure::IngressCountermeasureRuntime;
use crate::tls_fingerprint::TlsFingerprintService;
use crate::traffic::TrafficObfuscationConfig;

#[cfg(target_os = "linux")]
use crate::xdp::XdpManager;

static COORDINATOR: Lazy<Arc<CoreCoordinator>> = Lazy::new(|| Arc::new(CoreCoordinator::new()));

pub fn get_coordinator() -> Arc<CoreCoordinator> {
    COORDINATOR.clone()
}

fn normalize_advanced_config(mut config: AdvancedConfig) -> AdvancedConfig {
    config.ip_reputation = normalize_ip_reputation_config(config.ip_reputation);
    config
}

pub struct CoreCoordinator {
    anti_probe: Arc<AntiProbeService>,
    tls_fingerprint: Arc<TlsFingerprintService>,
    multipath_manager: Arc<MultipathManager>,
    ingress_countermeasure: Arc<IngressCountermeasureRuntime>,
    egress_identity_manager: Arc<EgressIdentityManager>,
    #[cfg(target_os = "linux")]
    xdp_manager: Arc<XdpManager>,
    advanced_config: Arc<RwLock<AdvancedConfig>>,
}

impl CoreCoordinator {
    pub fn new() -> Self {
        Self {
            anti_probe: Arc::new(AntiProbeService::new(AntiProbeConfig::default())),
            tls_fingerprint: Arc::new(TlsFingerprintService::new()),
            multipath_manager: Arc::new(MultipathManager::new()),
            ingress_countermeasure: Arc::new(IngressCountermeasureRuntime::new(
                AdvancedConfig::default().ingress_countermeasure,
            )),
            egress_identity_manager: Arc::new(EgressIdentityManager::new()),
            #[cfg(target_os = "linux")]
            xdp_manager: Arc::new(XdpManager::new()),
            advanced_config: Arc::new(RwLock::new(AdvancedConfig::default())),
        }
    }

    pub fn hydrate_from_advanced_config(&self, config: &AdvancedConfig) -> Result<()> {
        let config = normalize_advanced_config(config.clone());
        self.sync_security_policies_from_advanced_config(&config);
        self.apply_sub_configs(&config);
        *self.advanced_config.write() = config;
        Ok(())
    }

    pub fn get_advanced_config(&self) -> AdvancedConfig {
        self.advanced_config.read().clone()
    }

    fn load_persisted_advanced_config(&self) -> Result<()> {
        let path = AdvancedConfig::default_path()?;
        let config = AdvancedConfig::load(&path)?;
        self.hydrate_from_advanced_config(&config)
    }

    pub fn apply_advanced_config(&self, config: &AdvancedConfig) -> Result<()> {
        let config = normalize_advanced_config(config.clone());
        let old = self.advanced_config.read().clone();
        self.sync_security_policies_from_advanced_config(&config);
        self.apply_sub_configs(&config);
        *self.advanced_config.write() = config.clone();
        self.apply_runtime_changes(&old, &config)
    }

    fn sync_security_policies_from_advanced_config(&self, config: &AdvancedConfig) {
        get_security_policy_manager().sync_policies_from_config(config.security_policies.clone());
    }

    async fn revoke_removed_security_policies_async(config: &AdvancedConfig) -> Result<()> {
        let expected_names: HashSet<&str> = config
            .security_policies
            .iter()
            .map(|policy| policy.name.as_str())
            .collect();
        let manager = get_security_policy_manager();

        for state in manager.get_applied_states().await {
            if expected_names.contains(state.name.as_str()) {
                continue;
            }

            if state.applied {
                revoke_policy(&state.name).await?;
            }

            manager.remove_policy(&state.name).await;
        }

        Ok(())
    }

    fn revoke_removed_security_policies_blocking(config: &AdvancedConfig) -> Result<()> {
        let config = config.clone();
        AsyncHandler::block_on(async move { Self::revoke_removed_security_policies_async(&config).await })
    }

    fn apply_sub_configs(&self, config: &AdvancedConfig) {
        self.anti_probe.update_config(config.security.anti_probe.clone());
        self.multipath_manager.update_config(config.multipath.clone());
        self.ingress_countermeasure
            .update_config(config.ingress_countermeasure.clone());
        if let Err(error) = self
            .egress_identity_manager
            .update_config(config.egress_identity.clone())
        {
            log::warn!("[Coordinator] failed to update egress identity config: {}", error);
        }
        if let Err(error) = egress_monitor().update_config(config.egress_monitor.clone()) {
            log::warn!("[Coordinator] failed to update egress monitor config: {}", error);
        }
        #[cfg(target_os = "linux")]
        self.xdp_manager.update_config(config.xdp.clone());
    }

    fn apply_runtime_changes(&self, old: &AdvancedConfig, new: &AdvancedConfig) -> Result<()> {
        if old.security.enabled != new.security.enabled {
            if new.security.enabled {
                SecurityMonitor::global().start();
            } else {
                SecurityMonitor::global().stop();
            }
        }

        if old.security.tls_fingerprint != new.security.tls_fingerprint {
            if let Some(name) = new.security.tls_fingerprint.as_ref() {
                self.tls_fingerprint.set_by_name(name).map_err(anyhow::Error::msg)?;
            } else {
                self.tls_fingerprint.clear();
            }
        }

        if old.egress_monitor.enabled != new.egress_monitor.enabled {
            if new.egress_monitor.enabled {
                log::info!("[Coordinator] starting egress monitor");
                egress_monitor().start();
            } else {
                log::info!("[Coordinator] stopping egress monitor");
                egress_monitor().stop();
            }
        }

        #[cfg(target_os = "linux")]
        if old.xdp.enabled != new.xdp.enabled {
            if new.xdp.enabled {
                self.xdp_manager.start()?;
            } else {
                self.xdp_manager.stop()?;
            }
        }

        log::info!("[Coordinator] advanced config updated");
        Ok(())
    }

    pub fn initialize(&self) -> Result<()> {
        self.load_persisted_advanced_config()?;
        let config = self.advanced_config.read();

        if config.security.enabled {
            log::info!("[Coordinator] starting security monitor");
            SecurityMonitor::global().start();
        }

        if let Some(fingerprint_name) = config.security.tls_fingerprint.as_ref() {
            log::info!("[Coordinator] applying tls fingerprint: {}", fingerprint_name);
            if let Err(error) = self.tls_fingerprint.set_by_name(fingerprint_name) {
                log::warn!("[Coordinator] failed to apply tls fingerprint: {}", error);
            }
        }

        if config.egress_monitor.enabled {
            log::info!("[Coordinator] starting egress monitor");
            egress_monitor().start();
        }

        #[cfg(target_os = "linux")]
        if config.xdp.enabled {
            log::info!("[Coordinator] starting xdp manager");
            self.xdp_manager.start()?;
        }

        log::info!("[Coordinator] initialized");
        Ok(())
    }

    pub fn anti_probe(&self) -> Arc<AntiProbeService> {
        self.anti_probe.clone()
    }

    pub fn multipath_manager(&self) -> Arc<MultipathManager> {
        self.multipath_manager.clone()
    }

    pub fn ingress_countermeasure(&self) -> Arc<IngressCountermeasureRuntime> {
        self.ingress_countermeasure.clone()
    }

    pub fn egress_identity_manager(&self) -> Arc<EgressIdentityManager> {
        self.egress_identity_manager.clone()
    }

    #[cfg(target_os = "linux")]
    pub fn xdp_manager(&self) -> Arc<XdpManager> {
        self.xdp_manager.clone()
    }

    pub fn shutdown(&self) -> Result<()> {
        log::info!("[Coordinator] shutting down");
        egress_monitor().stop();
        SecurityMonitor::global().stop();

        #[cfg(target_os = "linux")]
        if self.advanced_config.read().xdp.enabled {
            self.xdp_manager.stop()?;
        }

        log::info!("[Coordinator] shutdown complete");
        Ok(())
    }
}

impl Default for CoreCoordinator {
    fn default() -> Self {
        Self::new()
    }
}

pub fn sync_coordinator_from_advanced_config() -> Result<()> {
    let path = AdvancedConfig::default_path()?;
    let config = normalize_advanced_config(AdvancedConfig::load(&path)?);
    CoreCoordinator::revoke_removed_security_policies_blocking(&config)?;
    get_coordinator().hydrate_from_advanced_config(&config)
}

pub async fn sync_coordinator_from_advanced_config_async() -> Result<()> {
    let path = AdvancedConfig::default_path()?;
    let config = normalize_advanced_config(AdvancedConfig::load(&path)?);
    CoreCoordinator::revoke_removed_security_policies_async(&config).await?;
    get_coordinator().hydrate_from_advanced_config(&config)?;
    get_ip_reputation_manager()
        .update_config(config.ip_reputation.clone())
        .await?;
    crate::core::blackhole_breaker::get_blackhole_breaker_manager()
        .update_config(config.blackhole_breaker.clone())
        .await;
    crate::core::security_runtime::apply_local_stealth_config(config.local_stealth.clone()).await;
    apply_traffic_obfuscation_config(effective_traffic_obfuscation_config(&config)).await?;
    get_session_affinity_manager()
        .update_config(config.session_affinity)
        .await?;
    Ok(())
}

pub async fn save_advanced_config(config: &AdvancedConfig) -> Result<()> {
    let config = normalize_advanced_config(config.clone());
    config.validate()?;

    let path = AdvancedConfig::default_path()?;
    config.save(&path)?;
    CoreCoordinator::revoke_removed_security_policies_async(&config).await?;

    save_dns_config_mapping(&config.dns.to_dns_config_mapping()).await?;
    get_ip_reputation_manager()
        .update_config(config.ip_reputation.clone())
        .await?;
    crate::core::blackhole_breaker::get_blackhole_breaker_manager()
        .update_config(config.blackhole_breaker.clone())
        .await;
    crate::core::security_runtime::apply_local_stealth_config(config.local_stealth.clone()).await;
    #[cfg(target_os = "linux")]
    get_coordinator().xdp_manager().update_config(config.xdp.clone());

    let obf_config = if config.traffic_obfuscation.enabled {
        config.traffic_obfuscation.clone()
    } else if config.traffic_padding.enabled {
        TrafficObfuscationConfig::from_legacy_padding(&config.traffic_padding)
    } else {
        config.traffic_obfuscation.clone()
    };
    apply_traffic_obfuscation_config(obf_config).await?;

    get_coordinator().apply_advanced_config(&config)?;

    get_session_affinity_manager()
        .update_config(config.session_affinity.clone())
        .await?;

    crate::core::runtime_lifecycle::update_runtime_config_checked("coordinator-sync").await?;

    Ok(())
}

pub fn save_advanced_config_blocking(config: AdvancedConfig) -> Result<()> {
    AsyncHandler::block_on(async move { save_advanced_config(&config).await })
}

pub async fn update_advanced_config<F>(mutator: F) -> Result<()>
where
    F: FnOnce(&mut AdvancedConfig),
{
    let mut config = get_coordinator().get_advanced_config();
    mutator(&mut config);
    save_advanced_config(&config).await
}

pub fn update_advanced_config_blocking<F>(mutator: F) -> Result<()>
where
    F: FnOnce(&mut AdvancedConfig),
{
    let mut config = get_coordinator().get_advanced_config();
    mutator(&mut config);
    save_advanced_config_blocking(config)
}

pub async fn coordinator_get_status() -> Result<CoordinatorStatus> {
    let _ = sync_coordinator_from_advanced_config_async().await;
    let coordinator = get_coordinator();
    let config = coordinator.get_advanced_config();
    let runtime_state = project_runtime_status(
        coordinator.egress_identity_manager().get_active_assignments(),
        get_session_affinity_manager().get_all_bindings().await?,
    )
    .await;

    Ok(CoordinatorStatus {
        initialized: true,
        security_enabled: config.security.enabled,
        security_compromised: crate::security::is_security_compromised(),
        anti_probe_enabled: config.security.anti_probe.enabled,
        tls_fingerprint: config.security.tls_fingerprint.clone(),
        egress_identity_enabled: config.egress_identity.enabled,
        session_affinity_enabled: config.session_affinity.enabled,
        egress_identity_active_assignments: runtime_state.egress_identity_assignments.len(),
        session_affinity_active_bindings: runtime_state.session_affinity_bindings.len(),
        runtime_state,
        multipath_enabled: config.multipath.enabled,
        traffic_obfuscation_enabled: config.traffic_obfuscation.enabled,
        honeypot_enabled: config.security.honeypot.enabled,
        self_destruct_enabled: config.security.self_destruct.enabled,
        #[cfg(target_os = "linux")]
        xdp_enabled: config.xdp.enabled,
        #[cfg(target_os = "linux")]
        xdp_running: coordinator.xdp_manager().is_running(),
    })
}
