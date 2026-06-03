use crate::{
    config::AdvancedConfig,
    traffic::{ObfuscationProfile, ObfuscationScheduler, ObfuscationStats, TrafficObfuscationConfig},
};
use anyhow::Result;
use once_cell::sync::Lazy;
use std::sync::Arc;
use tokio::sync::RwLock;

static OBFUSCATION_SCHEDULER: Lazy<Arc<RwLock<Option<ObfuscationScheduler>>>> =
    Lazy::new(|| Arc::new(RwLock::new(None)));

pub async fn traffic_obfuscation_get_config() -> TrafficObfuscationConfig {
    let scheduler_guard = OBFUSCATION_SCHEDULER.read().await;

    if let Some(scheduler) = scheduler_guard.as_ref() {
        scheduler.get_config().await
    } else {
        effective_traffic_obfuscation_config(&AdvancedConfig::load_default())
    }
}

pub async fn apply_traffic_obfuscation_config(config: TrafficObfuscationConfig) -> Result<()> {
    let mut scheduler_guard = OBFUSCATION_SCHEDULER.write().await;

    if config.enabled {
        if let Some(scheduler) = scheduler_guard.as_ref() {
            scheduler.update_config(config.clone()).await;
        } else {
            *scheduler_guard = Some(ObfuscationScheduler::new(config.clone()));
        }

        if let Some(scheduler) = scheduler_guard.as_ref() {
            scheduler.start().await?;
        }
    } else if let Some(scheduler) = scheduler_guard.as_ref() {
        scheduler.stop().await;
        *scheduler_guard = None;
    }

    Ok(())
}

pub async fn traffic_obfuscation_update_config(config: TrafficObfuscationConfig) -> Result<()> {
    config.validate().map_err(|e| anyhow::anyhow!("{}", e))?;

    persist_traffic_obfuscation_config(&config).await?;
    log::info!("traffic obfuscation config updated");
    Ok(())
}

pub async fn traffic_obfuscation_start() -> Result<()> {
    let mut config = effective_traffic_obfuscation_config(&AdvancedConfig::load_default());
    if !config.enabled {
        let (padding, timing, direction) = ObfuscationProfile::Conservative.derive_configs();
        config = TrafficObfuscationConfig {
            enabled: true,
            profile: ObfuscationProfile::Conservative,
            padding,
            timing,
            direction,
        };
    }

    traffic_obfuscation_update_config(config).await?;
    log::info!("traffic obfuscation started");
    Ok(())
}

pub async fn traffic_obfuscation_stop() -> Result<()> {
    let mut config = effective_traffic_obfuscation_config(&AdvancedConfig::load_default());
    config.enabled = false;
    config.profile = ObfuscationProfile::None;

    traffic_obfuscation_update_config(config).await?;
    log::info!("traffic obfuscation stopped");
    Ok(())
}

pub async fn traffic_obfuscation_get_stats() -> ObfuscationStats {
    let scheduler_guard = OBFUSCATION_SCHEDULER.read().await;

    if let Some(scheduler) = scheduler_guard.as_ref() {
        scheduler.get_stats().await
    } else {
        ObfuscationStats::default()
    }
}

pub async fn traffic_obfuscation_reset_stats() -> Result<()> {
    let scheduler_guard = OBFUSCATION_SCHEDULER.read().await;

    if let Some(scheduler) = scheduler_guard.as_ref() {
        scheduler.reset_stats().await;
        log::info!("traffic obfuscation stats reset");
        Ok(())
    } else {
        Err(anyhow::anyhow!("traffic obfuscation scheduler is not running"))
    }
}

pub async fn traffic_obfuscation_is_running() -> bool {
    let scheduler_guard = OBFUSCATION_SCHEDULER.read().await;

    scheduler_guard.as_ref().map(|s| s.is_running()).unwrap_or(false)
}

pub async fn traffic_obfuscation_apply_profile(profile: ObfuscationProfile) -> Result<TrafficObfuscationConfig> {
    let (padding, timing, direction) = profile.derive_configs();
    let config = TrafficObfuscationConfig {
        enabled: !matches!(profile, ObfuscationProfile::None),
        profile,
        padding,
        timing,
        direction,
    };

    persist_traffic_obfuscation_config(&config).await?;
    log::info!("traffic obfuscation profile applied: {:?}", config.profile);
    Ok(config)
}

async fn persist_traffic_obfuscation_config(config: &TrafficObfuscationConfig) -> Result<()> {
    let mut advanced = AdvancedConfig::load_default_strict()?;
    advanced.traffic_obfuscation = config.clone();
    advanced.traffic_padding.enabled = false;
    crate::feat::save_advanced_config(&advanced).await
}

fn effective_traffic_obfuscation_config(advanced: &AdvancedConfig) -> TrafficObfuscationConfig {
    if advanced.traffic_obfuscation.enabled {
        advanced.traffic_obfuscation.clone()
    } else if advanced.traffic_padding.enabled {
        TrafficObfuscationConfig::from_legacy_padding(&advanced.traffic_padding)
    } else {
        advanced.traffic_obfuscation.clone()
    }
}
