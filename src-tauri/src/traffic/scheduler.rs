use crate::traffic::direction::DirectionObfuscationConfig;
use crate::traffic::padding::TrafficPaddingConfig;
use crate::traffic::timing_jitter::TimingJitterConfig;
/**
 * 流量混淆调度器（薄代理层）
 *
 * 设计变更：真正的流量混淆由 Go 内核（mihomo）的 ObfuscatedConn 在连接层执行，
 * Rust 端不再运行伪 padding/timing/direction 引擎。
 *
 * 当前职责：
 * 1. 持有混淆配置（Profile + 子配置），供前端面板读写
 * 2. 通过 Mihomo API 获取 Go 端的真实混淆统计
 * 3. 向后兼容旧 TrafficPaddingConfig
 */
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use tokio::sync::RwLock;

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
                TrafficPaddingConfig {
                    enabled: false,
                    ..TrafficPaddingConfig::default()
                },
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
                (
                    TrafficPaddingConfig::default(),
                    TimingJitterConfig::default(),
                    DirectionObfuscationConfig::default(),
                )
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
        // padding 的 validate 由配置自身保证
        let _ = p;
        Ok(())
    }
}

/// 混淆统计（来自 Go 内核真实数据）
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ObfuscationStats {
    /// Go 端已创建的混淆连接总数
    pub total_obfuscated_conns: i64,
    /// 当前活跃混淆连接数
    pub active_conns: i64,
    /// 混淆连接写入总字节数
    pub total_write_bytes: i64,
    /// 混淆连接写入总次数
    pub total_write_count: i64,
    /// 添加的 padding 总字节数
    pub total_padding_bytes: i64,
    /// TLS 指纹轮换次数
    pub tls_rotation_count: i64,
    /// 当前 TLS 指纹
    pub current_tls_fingerprint: String,
}

impl Default for ObfuscationStats {
    fn default() -> Self {
        Self {
            total_obfuscated_conns: 0,
            active_conns: 0,
            total_write_bytes: 0,
            total_write_count: 0,
            total_padding_bytes: 0,
            tls_rotation_count: 0,
            current_tls_fingerprint: String::new(),
        }
    }
}

/// 混淆调度器（薄代理层）
///
/// 真正的混淆工作由 Go 内核完成，此结构仅持有配置并通过 API 获取真实统计。
pub struct ObfuscationScheduler {
    config: Arc<RwLock<TrafficObfuscationConfig>>,
    running: Arc<AtomicBool>,
}

impl ObfuscationScheduler {
    /// 创建新的混淆调度器
    pub fn new(config: TrafficObfuscationConfig) -> Self {
        Self {
            config: Arc::new(RwLock::new(config)),
            running: Arc::new(AtomicBool::new(false)),
        }
    }

    /// 启动混淆（标记状态，真实混淆由 Go 内核执行）
    pub async fn start(&self) -> Result<()> {
        if self.running.load(Ordering::SeqCst) {
            log::warn!("Obfuscation scheduler is already running");
            return Ok(());
        }
        self.running.store(true, Ordering::SeqCst);
        log::info!("Obfuscation scheduler started (Go kernel handles real obfuscation)");
        Ok(())
    }

    /// 停止混淆
    pub async fn stop(&self) {
        self.running.store(false, Ordering::SeqCst);
        log::info!("Obfuscation scheduler stopped");
    }

    /// 获取统计信息（从 learn-gripe 内核的进程内计数器获取真实数据）
    pub async fn get_stats(&self) -> ObfuscationStats {
        match Self::fetch_stats_from_kernel().await {
            Ok(stats) => stats,
            Err(e) => {
                log::warn!("Failed to fetch obfuscation stats from kernel: {}", e);
                ObfuscationStats::default()
            }
        }
    }

    /// 重置统计信息（重置内核进程内计数器）
    pub async fn reset_stats(&self) {
        if let Err(e) = Self::reset_stats_via_kernel().await {
            log::warn!("Failed to reset obfuscation stats via kernel: {}", e);
        } else {
            log::info!("Obfuscation stats reset (via kernel)");
        }
    }

    /// 更新配置
    pub async fn update_config(&self, config: TrafficObfuscationConfig) {
        {
            let mut cfg = self.config.write().await;
            *cfg = config;
        }
        log::info!("Obfuscation config updated");
    }

    /// 检查是否正在运行
    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::SeqCst)
    }

    /// 获取当前配置
    pub async fn get_config(&self) -> TrafficObfuscationConfig {
        self.config.read().await.clone()
    }

    /// 从运行时桥接层获取真实混淆统计
    async fn fetch_stats_from_kernel() -> Result<ObfuscationStats> {
        let raw = crate::core::runtime_bridge::read_runtime_obfuscation_stats().await?;
        let obf = &raw["obfuscation"];
        let tls = &raw["tls"];
        Ok(ObfuscationStats {
            total_obfuscated_conns: obf["totalObfuscatedConns"].as_i64().unwrap_or(0),
            active_conns: obf["activeConns"].as_i64().unwrap_or(0),
            total_write_bytes: obf["totalWriteBytes"].as_i64().unwrap_or(0),
            total_write_count: obf["totalWriteCount"].as_i64().unwrap_or(0),
            total_padding_bytes: obf["totalPaddingBytes"].as_i64().unwrap_or(0),
            tls_rotation_count: obf["tlsRotationCount"].as_i64().unwrap_or(0),
            current_tls_fingerprint: tls["currentFingerprint"].as_str().unwrap_or("").to_string(),
        })
    }

    /// 通过运行时桥接层重置统计
    async fn reset_stats_via_kernel() -> Result<()> {
        crate::core::runtime_bridge::reset_runtime_obfuscation_stats().await
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
        assert!(!d.enabled);
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
    fn test_default_stats() {
        let stats = ObfuscationStats::default();
        assert_eq!(stats.total_obfuscated_conns, 0);
        assert_eq!(stats.active_conns, 0);
        assert_eq!(stats.total_padding_bytes, 0);
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
}
