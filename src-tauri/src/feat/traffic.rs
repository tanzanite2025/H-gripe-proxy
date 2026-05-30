use crate::traffic::{
    ObfuscationProfile, ObfuscationScheduler, ObfuscationStats,
    TrafficObfuscationConfig,
};
use anyhow::Result;
use once_cell::sync::Lazy;
use std::sync::Arc;
use tokio::sync::RwLock;

/// 全局混淆调度器实例
static OBFUSCATION_SCHEDULER: Lazy<Arc<RwLock<Option<ObfuscationScheduler>>>> =
    Lazy::new(|| Arc::new(RwLock::new(None)));

/// 获取流量混淆配置
pub async fn traffic_obfuscation_get_config() -> TrafficObfuscationConfig {
    let scheduler_guard = OBFUSCATION_SCHEDULER.read().await;

    if let Some(scheduler) = scheduler_guard.as_ref() {
        scheduler.get_config().await
    } else {
        TrafficObfuscationConfig::default()
    }
}

/// 应用混淆配置（供内部调用，例如高级配置落盘后同步）
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

/// 更新流量混淆配置
pub async fn traffic_obfuscation_update_config(config: TrafficObfuscationConfig) -> Result<()> {
    config.validate().map_err(|e| anyhow::anyhow!("{}", e))?;

    let mut scheduler_guard = OBFUSCATION_SCHEDULER.write().await;

    if let Some(scheduler) = scheduler_guard.as_ref() {
        scheduler.update_config(config).await;
    } else {
        *scheduler_guard = Some(ObfuscationScheduler::new(config));
    }

    log::info!("✅ 流量混淆配置已更新");
    Ok(())
}

/// 启动流量混淆
pub async fn traffic_obfuscation_start() -> Result<()> {
    let mut scheduler_guard = OBFUSCATION_SCHEDULER.write().await;

    if scheduler_guard.is_none() {
        *scheduler_guard = Some(ObfuscationScheduler::new(TrafficObfuscationConfig::default()));
    }

    if let Some(scheduler) = scheduler_guard.as_ref() {
        scheduler.start().await?;
        log::info!("✅ 流量混淆已启动");
        Ok(())
    } else {
        Err(anyhow::anyhow!("流量混淆调度器未初始化"))
    }
}

/// 停止流量混淆
pub async fn traffic_obfuscation_stop() -> Result<()> {
    let mut scheduler_guard = OBFUSCATION_SCHEDULER.write().await;

    if let Some(scheduler) = scheduler_guard.as_ref() {
        scheduler.stop().await;
        *scheduler_guard = None;
        log::info!("✅ 流量混淆已停止");
        Ok(())
    } else {
        Err(anyhow::anyhow!("流量混淆调度器未运行"))
    }
}

/// 获取流量混淆统计
pub async fn traffic_obfuscation_get_stats() -> ObfuscationStats {
    let scheduler_guard = OBFUSCATION_SCHEDULER.read().await;

    if let Some(scheduler) = scheduler_guard.as_ref() {
        scheduler.get_stats().await
    } else {
        ObfuscationStats::default()
    }
}

/// 重置流量混淆统计
pub async fn traffic_obfuscation_reset_stats() -> Result<()> {
    let scheduler_guard = OBFUSCATION_SCHEDULER.read().await;

    if let Some(scheduler) = scheduler_guard.as_ref() {
        scheduler.reset_stats().await;
        log::info!("✅ 流量混淆统计已重置");
        Ok(())
    } else {
        Err(anyhow::anyhow!("流量混淆调度器未运行"))
    }
}

/// 检查流量混淆是否正在运行
pub async fn traffic_obfuscation_is_running() -> bool {
    let scheduler_guard = OBFUSCATION_SCHEDULER.read().await;

    scheduler_guard
        .as_ref()
        .map(|s| s.is_running())
        .unwrap_or(false)
}

/// 应用预设 Profile，返回生成的配置
pub async fn traffic_obfuscation_apply_profile(
    profile: ObfuscationProfile,
) -> Result<TrafficObfuscationConfig> {
    let (p_cfg, t_cfg, d_cfg) = profile.derive_configs();
    let config = TrafficObfuscationConfig {
        enabled: !matches!(profile, ObfuscationProfile::None),
        profile,
        padding: p_cfg,
        timing: t_cfg,
        direction: d_cfg,
    };

    // 同步到调度器
    let mut scheduler_guard = OBFUSCATION_SCHEDULER.write().await;

    if config.enabled {
        if let Some(scheduler) = scheduler_guard.as_ref() {
            scheduler.update_config(config.clone()).await;
        } else {
            *scheduler_guard = Some(ObfuscationScheduler::new(config.clone()));
        }
    } else if let Some(scheduler) = scheduler_guard.as_ref() {
        scheduler.stop().await;
        *scheduler_guard = None;
    }

    log::info!("✅ 流量混淆 Profile 已应用: {:?}", config.profile);
    Ok(config)
}
