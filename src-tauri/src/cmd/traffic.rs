/**
 * 流量功能 Tauri 命令
 */

use crate::traffic::{
    PaddingScheduler, PaddingStats, TrafficPaddingConfig,
};
use once_cell::sync::Lazy;
use std::sync::Arc;
use tokio::sync::RwLock;

static PADDING_SCHEDULER: Lazy<Arc<RwLock<Option<PaddingScheduler>>>> =
    Lazy::new(|| Arc::new(RwLock::new(None)));

/// 获取流量填充配置
#[tauri::command]
pub async fn traffic_padding_get_config() -> Result<TrafficPaddingConfig, String> {
    let scheduler_guard = PADDING_SCHEDULER.read().await;

    if let Some(scheduler) = scheduler_guard.as_ref() {
        Ok(scheduler.get_config().await)
    } else {
        Ok(TrafficPaddingConfig::default())
    }

}

/// 应用填充配置（供内部调用，例如高级配置落盘后同步）
pub async fn apply_traffic_padding_config(config: TrafficPaddingConfig) -> Result<(), String> {
    let mut scheduler_guard = PADDING_SCHEDULER.write().await;

    if config.enabled {
        if let Some(scheduler) = scheduler_guard.as_ref() {
            scheduler.update_config(config.clone()).await;
        } else {
            *scheduler_guard = Some(PaddingScheduler::new(config.clone()));
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
    let mut scheduler_guard = PADDING_SCHEDULER.write().await;

    if let Some(scheduler) = scheduler_guard.as_ref() {
        scheduler.update_config(config.clone()).await;
    } else {
        // 创建新的调度器（未启动，等待用户点击开始）
        *scheduler_guard = Some(PaddingScheduler::new(config));
    }

    log::info!("✅ 流量填充配置已更新");
    Ok(())
}

/// 启动流量填充
#[tauri::command]
pub async fn traffic_padding_start() -> Result<(), String> {
    let mut scheduler_guard = PADDING_SCHEDULER.write().await;

    if scheduler_guard.is_none() {
        *scheduler_guard = Some(PaddingScheduler::new(TrafficPaddingConfig::default()));
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
    let scheduler_guard = PADDING_SCHEDULER.read().await;
    
    Ok(scheduler_guard
        .as_ref()
        .map(|s| s.is_running())
        .unwrap_or(false))
}
