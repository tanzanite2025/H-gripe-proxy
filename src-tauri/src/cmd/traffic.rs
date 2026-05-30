/**
 * 流量功能 Tauri 命令
 */

use crate::traffic::{
    ObfuscationProfile, ObfuscationScheduler, ObfuscationStats,
    PaddingStats, TrafficObfuscationConfig, TrafficPaddingConfig,
};
use once_cell::sync::Lazy;
use std::sync::Arc;
use tokio::sync::RwLock;

// ── 新：统一混淆调度器 ──
static OBFUSCATION_SCHEDULER: Lazy<Arc<RwLock<Option<ObfuscationScheduler>>>> =
    Lazy::new(|| Arc::new(RwLock::new(None)));

// ── 旧：独立 padding 调度器（向后兼容，委托到新调度器） ──
static PADDING_SCHEDULER: Lazy<Arc<RwLock<Option<crate::traffic::PaddingScheduler>>>> =
    Lazy::new(|| Arc::new(RwLock::new(None)));

// ============================================================
// 新：流量混淆命令
// ============================================================

/// 获取流量混淆配置
#[tauri::command]
pub async fn traffic_obfuscation_get_config() -> Result<TrafficObfuscationConfig, String> {
    let scheduler_guard = OBFUSCATION_SCHEDULER.read().await;

    if let Some(scheduler) = scheduler_guard.as_ref() {
        Ok(scheduler.get_config().await)
    } else {
        Ok(TrafficObfuscationConfig::default())
    }
}

/// 应用混淆配置（供内部调用，例如高级配置落盘后同步）
pub async fn apply_traffic_obfuscation_config(config: TrafficObfuscationConfig) -> Result<(), String> {
    let mut scheduler_guard = OBFUSCATION_SCHEDULER.write().await;

    if config.enabled {
        if let Some(scheduler) = scheduler_guard.as_ref() {
            scheduler.update_config(config.clone()).await;
        } else {
            *scheduler_guard = Some(ObfuscationScheduler::new(config.clone()));
        }

        if let Some(scheduler) = scheduler_guard.as_ref() {
            scheduler
                .start()
                .await
                .map_err(|e| format!("启动流量混淆失败: {}", e))?;
        }
    } else if let Some(scheduler) = scheduler_guard.as_ref() {
        scheduler.stop().await;
        *scheduler_guard = None;
    }

    Ok(())
}

/// 更新流量混淆配置
#[tauri::command]
pub async fn traffic_obfuscation_update_config(
    config: TrafficObfuscationConfig,
) -> Result<(), String> {
    config.validate().map_err(|e| format!("配置验证失败: {}", e))?;

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
#[tauri::command]
pub async fn traffic_obfuscation_start() -> Result<(), String> {
    let mut scheduler_guard = OBFUSCATION_SCHEDULER.write().await;

    if scheduler_guard.is_none() {
        *scheduler_guard = Some(ObfuscationScheduler::new(TrafficObfuscationConfig::default()));
    }

    if let Some(scheduler) = scheduler_guard.as_ref() {
        scheduler
            .start()
            .await
            .map_err(|e| format!("启动流量混淆失败: {}", e))?;
        log::info!("✅ 流量混淆已启动");
        Ok(())
    } else {
        Err("流量混淆调度器未初始化".to_string())
    }
}

/// 停止流量混淆
#[tauri::command]
pub async fn traffic_obfuscation_stop() -> Result<(), String> {
    let mut scheduler_guard = OBFUSCATION_SCHEDULER.write().await;

    if let Some(scheduler) = scheduler_guard.as_ref() {
        scheduler.stop().await;
        *scheduler_guard = None;
        log::info!("✅ 流量混淆已停止");
        Ok(())
    } else {
        Err("流量混淆调度器未运行".to_string())
    }
}

/// 获取流量混淆统计
#[tauri::command]
pub async fn traffic_obfuscation_get_stats() -> Result<ObfuscationStats, String> {
    let scheduler_guard = OBFUSCATION_SCHEDULER.read().await;

    if let Some(scheduler) = scheduler_guard.as_ref() {
        Ok(scheduler.get_stats().await)
    } else {
        Ok(ObfuscationStats::default())
    }
}

/// 重置流量混淆统计
#[tauri::command]
pub async fn traffic_obfuscation_reset_stats() -> Result<(), String> {
    let scheduler_guard = OBFUSCATION_SCHEDULER.read().await;

    if let Some(scheduler) = scheduler_guard.as_ref() {
        scheduler.reset_stats().await;
        log::info!("✅ 流量混淆统计已重置");
        Ok(())
    } else {
        Err("流量混淆调度器未运行".to_string())
    }
}

/// 检查流量混淆是否正在运行
#[tauri::command]
pub async fn traffic_obfuscation_is_running() -> Result<bool, String> {
    let scheduler_guard = OBFUSCATION_SCHEDULER.read().await;

    Ok(scheduler_guard
        .as_ref()
        .map(|s| s.is_running())
        .unwrap_or(false))
}

/// 应用预设 Profile，返回生成的配置
#[tauri::command]
pub async fn traffic_obfuscation_apply_profile(
    profile: ObfuscationProfile,
) -> Result<TrafficObfuscationConfig, String> {
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

// ============================================================
// 旧：流量填充命令（向后兼容，委托到新调度器或独立调度器）
// ============================================================

/// 获取流量填充配置
#[tauri::command]
pub async fn traffic_padding_get_config() -> Result<TrafficPaddingConfig, String> {
    // 优先从新调度器获取
    let obf_guard = OBFUSCATION_SCHEDULER.read().await;
    if let Some(scheduler) = obf_guard.as_ref() {
        return Ok(scheduler.get_config().await.padding);
    }

    let scheduler_guard = PADDING_SCHEDULER.read().await;
    if let Some(scheduler) = scheduler_guard.as_ref() {
        Ok(scheduler.get_config().await)
    } else {
        Ok(TrafficPaddingConfig::default())
    }
}

/// 应用填充配置（供内部调用，例如高级配置落盘后同步）
#[allow(dead_code)]
pub async fn apply_traffic_padding_config(config: TrafficPaddingConfig) -> Result<(), String> {
    // 如果新调度器已存在，委托到新调度器
    {
        let obf_guard = OBFUSCATION_SCHEDULER.read().await;
        if obf_guard.is_some() {
            drop(obf_guard);
            let obf_config = TrafficObfuscationConfig::from_legacy_padding(&config);
            return apply_traffic_obfuscation_config(obf_config).await;
        }
    }

    // 否则使用旧调度器
    let mut scheduler_guard = PADDING_SCHEDULER.write().await;

    if config.enabled {
        if let Some(scheduler) = scheduler_guard.as_ref() {
            scheduler.update_config(config.clone()).await;
        } else {
            *scheduler_guard = Some(crate::traffic::PaddingScheduler::new(config.clone()));
        }

        if let Some(scheduler) = scheduler_guard.as_ref() {
            scheduler
                .start()
                .await
                .map_err(|e| format!("启动流量填充失败: {}", e))?;
        }
    } else if let Some(scheduler) = scheduler_guard.as_ref() {
        scheduler.stop().await;
        *scheduler_guard = None;
    }

    Ok(())
}

/// 更新流量填充配置
#[tauri::command]
pub async fn traffic_padding_update_config(
    config: TrafficPaddingConfig,
) -> Result<(), String> {
    // 如果新调度器已存在，委托到新调度器器
    {
        let obf_guard = OBFUSCATION_SCHEDULER.read().await;
        if obf_guard.is_some() {
            drop(obf_guard);
            let mut obf_config = {
                let obf_guard = OBFUSCATION_SCHEDULER.read().await;
                obf_guard.as_ref().unwrap().get_config().await
            };
            obf_config.profile = ObfuscationProfile::Custom;
            obf_config.padding = config;
            return traffic_obfuscation_update_config(obf_config).await;
        }
    }

    let mut scheduler_guard = PADDING_SCHEDULER.write().await;

    if let Some(scheduler) = scheduler_guard.as_ref() {
        scheduler.update_config(config.clone()).await;
    } else {
        *scheduler_guard = Some(crate::traffic::PaddingScheduler::new(config));
    }

    log::info!("✅ 流量填充配置已更新");
    Ok(())
}

/// 启动流量填充
#[tauri::command]
pub async fn traffic_padding_start() -> Result<(), String> {
    let obf_guard = OBFUSCATION_SCHEDULER.write().await;
    if obf_guard.is_some() {
        return obf_guard.as_ref().unwrap()
            .start()
            .await
            .map_err(|e| format!("启动流量填充失败: {}", e));
    }
    drop(obf_guard);

    let mut scheduler_guard = PADDING_SCHEDULER.write().await;

    if scheduler_guard.is_none() {
        *scheduler_guard = Some(crate::traffic::PaddingScheduler::new(TrafficPaddingConfig::default()));
    }

    if let Some(scheduler) = scheduler_guard.as_ref() {
        scheduler
            .start()
            .await
            .map_err(|e| format!("启动流量填充失败: {}", e))?;
        log::info!("✅ 流量填充已启动");
        Ok(())
    } else {
        Err("流量填充调度器未初始化".to_string())
    }
}

/// 停止流量填充
#[tauri::command]
pub async fn traffic_padding_stop() -> Result<(), String> {
    let mut obf_guard = OBFUSCATION_SCHEDULER.write().await;
    if obf_guard.is_some() {
        obf_guard.as_ref().unwrap().stop().await;
        *obf_guard = None;
        log::info!("✅ 流量填充已停止");
        return Ok(());
    }
    drop(obf_guard);

    let mut scheduler_guard = PADDING_SCHEDULER.write().await;

    if let Some(scheduler) = scheduler_guard.as_ref() {
        scheduler.stop().await;
        *scheduler_guard = None;
        log::info!("✅ 流量填充已停止");
        Ok(())
    } else {
        Err("流量填充调度器未运行".to_string())
    }
}

/// 获取流量填充统计
#[tauri::command]
pub async fn traffic_padding_get_stats() -> Result<PaddingStats, String> {
    let obf_guard = OBFUSCATION_SCHEDULER.read().await;
    if let Some(scheduler) = obf_guard.as_ref() {
        return Ok(scheduler.get_stats().await.padding);
    }

    let scheduler_guard = PADDING_SCHEDULER.read().await;

    if let Some(scheduler) = scheduler_guard.as_ref() {
        Ok(scheduler.get_stats().await)
    } else {
        Ok(PaddingStats::default())
    }
}

/// 重置流量填充统计
#[tauri::command]
pub async fn traffic_padding_reset_stats() -> Result<(), String> {
    let obf_guard = OBFUSCATION_SCHEDULER.read().await;
    if let Some(scheduler) = obf_guard.as_ref() {
        scheduler.reset_stats().await;
        log::info!("✅ 流量填充统计已重置");
        return Ok(());
    }

    let scheduler_guard = PADDING_SCHEDULER.read().await;

    if let Some(scheduler) = scheduler_guard.as_ref() {
        scheduler.reset_stats().await;
        log::info!("✅ 流量填充统计已重置");
        Ok(())
    } else {
        Err("流量填充调度器未运行".to_string())
    }
}

/// 检查流量填充是否正在运行
#[tauri::command]
pub async fn traffic_padding_is_running() -> Result<bool, String> {
    let obf_guard = OBFUSCATION_SCHEDULER.read().await;
    if let Some(scheduler) = obf_guard.as_ref() {
        return Ok(scheduler.is_running());
    }

    let scheduler_guard = PADDING_SCHEDULER.read().await;

    Ok(scheduler_guard
        .as_ref()
        .map(|s| s.is_running())
        .unwrap_or(false))
}
