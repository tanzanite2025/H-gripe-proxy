/**
 * 流量混淆调度器
 *
 * 功能：
 * 1. 统一协调 padding / timing_jitter / direction 三个子引擎
 * 2. 单一 tokio task 内运行，避免多 task 竞争资源
 * 3. 根据 Profile 自动生成子配置
 * 4. 向后兼容旧 TrafficPaddingConfig
 */

use anyhow::Result;
use crate::traffic::padding::{PaddingScheduler, TrafficPaddingConfig, PaddingStats};
use crate::traffic::timing_jitter::{TimingJitterConfig, TimingJitterStats, TimingJitterEngine};
use crate::traffic::direction::{DirectionObfuscationConfig, DirectionStats, DirectionObfuscator};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;
use tokio::sync::{Mutex, Notify, RwLock};
use tokio::task::JoinHandle;
use tokio::time;

/// 混淆预设 Profile
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ObfuscationProfile {
    None,
    Conservative,
    Aggressive,
    Custom,
}

impl Default for ObfuscationProfile {
    fn default() -> Self {
        Self::None
    }
}

impl ObfuscationProfile {
    /// 根据 Profile 生成子配置
    pub fn derive_configs(&self) -> (TrafficPaddingConfig, TimingJitterConfig, DirectionObfuscationConfig) {
        match self {
            ObfuscationProfile::None => (
                TrafficPaddingConfig { enabled: false, ..TrafficPaddingConfig::default() },
                TimingJitterConfig::default(),
                DirectionObfuscationConfig::default(),
            ),
            ObfuscationProfile::Conservative => (
                TrafficPaddingConfig {
                    enabled: true,
                    intensity: crate::traffic::padding::PaddingIntensity::Low,
                    frequency: crate::traffic::padding::PaddingFrequency {
                        freq_type: crate::traffic::padding::FrequencyType::Time,
                        interval: 10,
                    },
                    ..TrafficPaddingConfig::default()
                },
                TimingJitterConfig::conservative(),
                DirectionObfuscationConfig::conservative(),
            ),
            ObfuscationProfile::Aggressive => (
                TrafficPaddingConfig {
                    enabled: true,
                    intensity: crate::traffic::padding::PaddingIntensity::High,
                    frequency: crate::traffic::padding::PaddingFrequency {
                        freq_type: crate::traffic::padding::FrequencyType::Time,
                        interval: 5,
                    },
                    ..TrafficPaddingConfig::default()
                },
                TimingJitterConfig::aggressive(),
                DirectionObfuscationConfig::aggressive(),
            ),
            ObfuscationProfile::Custom => {
                // Custom 模式下使用用户提供的子配置，这里返回 default 作为占位
                (TrafficPaddingConfig::default(), TimingJitterConfig::default(), DirectionObfuscationConfig::default())
            }
        }
    }
}

/// 流量混淆总配置（伞形）
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TrafficObfuscationConfig {
    /// 启用流量混淆
    pub enabled: bool,
    /// 预设 Profile
    pub profile: ObfuscationProfile,
    /// 填充子配置
    pub padding: TrafficPaddingConfig,
    /// 时序混淆子配置
    pub timing: TimingJitterConfig,
    /// 方向混淆子配置
    pub direction: DirectionObfuscationConfig,
}

impl Default for TrafficObfuscationConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            profile: ObfuscationProfile::None,
            padding: TrafficPaddingConfig::default(),
            timing: TimingJitterConfig::default(),
            direction: DirectionObfuscationConfig::default(),
        }
    }
}

impl TrafficObfuscationConfig {
    /// 获取有效的子配置：Custom 使用用户值，其他 Profile 使用派生值
    pub fn effective_configs(&self) -> (TrafficPaddingConfig, TimingJitterConfig, DirectionObfuscationConfig) {
        if matches!(self.profile, ObfuscationProfile::Custom) {
            (self.padding.clone(), self.timing.clone(), self.direction.clone())
        } else {
            self.profile.derive_configs()
        }
    }

    /// 从旧 TrafficPaddingConfig 迁移
    pub fn from_legacy_padding(padding: &TrafficPaddingConfig) -> Self {
        Self {
            enabled: padding.enabled,
            profile: ObfuscationProfile::Custom,
            padding: padding.clone(),
            timing: TimingJitterConfig::default(),
            direction: DirectionObfuscationConfig::default(),
        }
    }

    pub fn validate(&self) -> Result<(), String> {
        if !self.enabled {
            return Ok(());
        }
        let (p, t, d) = self.effective_configs();
        t.validate()?;
        d.validate()?;
        // padding 的 validate 由 PaddingScheduler 内部保证
        let _ = p;
        Ok(())
    }
}

/// 混淆统计（聚合三个子引擎）
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ObfuscationStats {
    pub padding: PaddingStats,
    pub timing: TimingJitterStats,
    pub direction: DirectionStats,
}

impl Default for ObfuscationStats {
    fn default() -> Self {
        Self {
            padding: PaddingStats::default(),
            timing: TimingJitterStats::default(),
            direction: DirectionStats::default(),
        }
    }
}

/// 混淆调度器
pub struct ObfuscationScheduler {
    config: Arc<RwLock<TrafficObfuscationConfig>>,
    stats: Arc<RwLock<ObfuscationStats>>,
    padding: Arc<RwLock<PaddingScheduler>>,
    running: Arc<AtomicBool>,
    shutdown: Arc<Mutex<Option<Arc<Notify>>>>,
    handle: Arc<Mutex<Option<JoinHandle<()>>>>,
}

impl ObfuscationScheduler {
    /// 创建新的混淆调度器
    pub fn new(config: TrafficObfuscationConfig) -> Self {
        let (p_cfg, _t_cfg, _d_cfg) = config.effective_configs();
        Self {
            config: Arc::new(RwLock::new(config)),
            stats: Arc::new(RwLock::new(ObfuscationStats::default())),
            padding: Arc::new(RwLock::new(PaddingScheduler::new(p_cfg))),
            running: Arc::new(AtomicBool::new(false)),
            shutdown: Arc::new(Mutex::new(None)),
            handle: Arc::new(Mutex::new(None)),
        }
    }

    /// 启动混淆调度
    pub async fn start(&self) -> Result<()> {
        if self.running.load(Ordering::SeqCst) {
            log::warn!("Obfuscation scheduler is already running");
            return Ok(());
        }

        self.running.store(true, Ordering::SeqCst);

        // 启动 padding 子调度器
        let cfg = self.config.read().await;
        let (p_cfg, _, _) = cfg.effective_configs();
        drop(cfg);

        {
            let padding_guard = self.padding.write().await;
            padding_guard.update_config(p_cfg).await;
            padding_guard.start().await?;
        }

        // 启动主调度循环（负责 timing / direction 及统计聚合）
        let config = self.config.clone();
        let stats = self.stats.clone();
        let padding = self.padding.clone();
        let running = self.running.clone();

        let shutdown_token = Arc::new(Notify::new());
        {
            let mut guard = self.shutdown.lock().await;
            *guard = Some(shutdown_token.clone());
        }

        let handle_slot = self.handle.clone();

        let join = tokio::spawn(async move {
            Self::schedule_loop(config, stats, padding, running, shutdown_token).await;
        });

        let mut handle_guard = handle_slot.lock().await;
        *handle_guard = Some(join);

        log::info!("🎯 Starting traffic obfuscation scheduler");
        Ok(())
    }

    /// 停止混淆调度
    pub async fn stop(&self) {
        self.running.store(false, Ordering::SeqCst);

        // 停止 padding 子调度器
        {
            let padding_guard = self.padding.read().await;
            padding_guard.stop().await;
        }

        // 通知主循环退出
        if let Some(token) = self.shutdown.lock().await.take() {
            token.notify_waiters();
        }

        if let Some(handle) = self.handle.lock().await.take() {
            if let Err(e) = handle.await {
                log::warn!("Obfuscation scheduler task join failed: {}", e);
            }
        }

        // 重置统计
        {
            let mut stats_guard = self.stats.write().await;
            *stats_guard = ObfuscationStats::default();
        }

        log::info!("🛑 Stopping traffic obfuscation scheduler");
    }

    /// 主调度循环
    async fn schedule_loop(
        config: Arc<RwLock<TrafficObfuscationConfig>>,
        stats: Arc<RwLock<ObfuscationStats>>,
        padding: Arc<RwLock<PaddingScheduler>>,
        running: Arc<AtomicBool>,
        shutdown: Arc<Notify>,
    ) {
        // 内部引擎（非 async，纯计算）
        let mut timing_engine = TimingJitterEngine::new(TimingJitterConfig::default());
        let mut direction_obfuscator = DirectionObfuscator::new(DirectionObfuscationConfig::default());

        // 初始化引擎配置
        {
            let cfg = config.read().await;
            let (_, t_cfg, d_cfg) = cfg.effective_configs();
            timing_engine.update_config(t_cfg);
            direction_obfuscator.update_config(d_cfg);
        }

        // 统计聚合间隔
        let mut aggregate_interval = time::interval(Duration::from_secs(5));

        while running.load(Ordering::SeqCst) {
            tokio::select! {
                _ = shutdown.notified() => break,
                _ = aggregate_interval.tick() => {
                    // 聚合 padding 统计
                    let padding_stats = {
                        let padding_guard = padding.read().await;
                        padding_guard.get_stats().await
                    };

                    // 聚合到总统计
                    let mut stats_guard = stats.write().await;
                    stats_guard.padding = padding_stats;
                    stats_guard.timing = timing_engine.stats().clone();
                    stats_guard.direction = direction_obfuscator.stats().clone();
                }
            }
        }

        log::info!("Obfuscation scheduler loop stopped");
    }

    /// 获取统计信息
    pub async fn get_stats(&self) -> ObfuscationStats {
        self.stats.read().await.clone()
    }

    /// 重置统计信息
    pub async fn reset_stats(&self) {
        {
            let padding_guard = self.padding.read().await;
            padding_guard.reset_stats().await;
        }
        {
            let mut stats_guard = self.stats.write().await;
            *stats_guard = ObfuscationStats::default();
        }
        log::info!("📊 Obfuscation stats reset");
    }

    /// 更新配置
    pub async fn update_config(&self, config: TrafficObfuscationConfig) {
        let (p_cfg, _t_cfg, _d_cfg) = config.effective_configs();

        // 更新 padding 子调度器
        {
            let padding_guard = self.padding.write().await;
            padding_guard.update_config(p_cfg).await;
        }

        // 更新总配置
        {
            let mut cfg = self.config.write().await;
            *cfg = config;
        }

        log::info!("📝 Obfuscation config updated");
    }

    /// 检查是否正在运行
    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::SeqCst)
    }

    /// 获取当前配置
    pub async fn get_config(&self) -> TrafficObfuscationConfig {
        self.config.read().await.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_obfuscation_profile_none() {
        let (p, t, d) = ObfuscationProfile::None.derive_configs();
        assert!(!p.enabled);
        assert!(!t.enabled);
        assert!(!d.enabled);
    }

    #[test]
    fn test_obfuscation_profile_conservative() {
        let (p, t, d) = ObfuscationProfile::Conservative.derive_configs();
        assert!(p.enabled);
        assert!(t.enabled);
        assert!(!d.enabled); // conservative 不启用方向混淆
    }

    #[test]
    fn test_obfuscation_profile_aggressive() {
        let (p, t, d) = ObfuscationProfile::Aggressive.derive_configs();
        assert!(p.enabled);
        assert!(t.enabled);
        assert!(d.enabled);
    }

    #[test]
    fn test_default_config() {
        let config = TrafficObfuscationConfig::default();
        assert!(!config.enabled);
        assert!(matches!(config.profile, ObfuscationProfile::None));
    }

    #[test]
    fn test_effective_configs_custom() {
        let config = TrafficObfuscationConfig {
            enabled: true,
            profile: ObfuscationProfile::Custom,
            padding: TrafficPaddingConfig {
                enabled: true,
                min_size: 1024,
                ..TrafficPaddingConfig::default()
            },
            timing: TimingJitterConfig::aggressive(),
            direction: DirectionObfuscationConfig::aggressive(),
        };
        let (p, t, d) = config.effective_configs();
        assert_eq!(p.min_size, 1024);
        assert!(matches!(t.mode, crate::traffic::timing_jitter::JitterMode::Gaussian));
        assert!(d.enabled);
    }

    #[test]
    fn test_effective_configs_non_custom_ignores_user_values() {
        let config = TrafficObfuscationConfig {
            enabled: true,
            profile: ObfuscationProfile::Conservative,
            padding: TrafficPaddingConfig {
                enabled: true,
                min_size: 9999, // 用户值，但 Conservative 会覆盖
                ..TrafficPaddingConfig::default()
            },
            timing: TimingJitterConfig::default(),
            direction: DirectionObfuscationConfig::default(),
        };
        let (p, _, _) = config.effective_configs();
        // Conservative profile 的 padding min_size 是 default 512
        assert_eq!(p.min_size, 512);
    }

    #[test]
    fn test_from_legacy_padding() {
        let legacy = TrafficPaddingConfig {
            enabled: true,
            min_size: 2048,
            ..TrafficPaddingConfig::default()
        };
        let config = TrafficObfuscationConfig::from_legacy_padding(&legacy);
        assert!(config.enabled);
        assert!(matches!(config.profile, ObfuscationProfile::Custom));
        assert_eq!(config.padding.min_size, 2048);
        assert!(!config.timing.enabled);
        assert!(!config.direction.enabled);
    }

    #[test]
    fn test_validate_ok() {
        let config = TrafficObfuscationConfig::default();
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_validate_timing_error() {
        let config = TrafficObfuscationConfig {
            enabled: true,
            profile: ObfuscationProfile::Custom,
            timing: TimingJitterConfig {
                enabled: true,
                min_delay_ms: 300,
                max_delay_ms: 100,
                ..TimingJitterConfig::default()
            },
            ..TrafficObfuscationConfig::default()
        };
        assert!(config.validate().is_err());
    }

    #[tokio::test]
    async fn test_scheduler_creation() {
        let config = TrafficObfuscationConfig::default();
        let scheduler = ObfuscationScheduler::new(config);
        assert!(!scheduler.is_running());
    }

    #[tokio::test]
    async fn test_scheduler_get_config() {
        let config = TrafficObfuscationConfig {
            enabled: true,
            profile: ObfuscationProfile::Conservative,
            ..TrafficObfuscationConfig::default()
        };
        let scheduler = ObfuscationScheduler::new(config);
        let retrieved = scheduler.get_config().await;
        assert!(retrieved.enabled);
        assert!(matches!(retrieved.profile, ObfuscationProfile::Conservative));
    }

    #[tokio::test]
    async fn test_scheduler_update_config() {
        let config = TrafficObfuscationConfig::default();
        let scheduler = ObfuscationScheduler::new(config);

        let new_config = TrafficObfuscationConfig {
            enabled: true,
            profile: ObfuscationProfile::Aggressive,
            ..TrafficObfuscationConfig::default()
        };
        scheduler.update_config(new_config).await;

        let retrieved = scheduler.get_config().await;
        assert!(matches!(retrieved.profile, ObfuscationProfile::Aggressive));
    }

    #[tokio::test]
    async fn test_scheduler_get_stats() {
        let config = TrafficObfuscationConfig::default();
        let scheduler = ObfuscationScheduler::new(config);
        let stats = scheduler.get_stats().await;
        assert_eq!(stats.padding.padding_count, 0);
    }

    #[tokio::test]
    async fn test_scheduler_reset_stats() {
        let config = TrafficObfuscationConfig::default();
        let scheduler = ObfuscationScheduler::new(config);
        scheduler.reset_stats().await;
        let stats = scheduler.get_stats().await;
        assert_eq!(stats.padding.padding_count, 0);
    }
}
