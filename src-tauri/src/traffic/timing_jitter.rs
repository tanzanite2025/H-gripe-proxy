/**
 * 时序混淆模块
 *
 * 功能：
 * 1. 注入随机延迟，打破流量时序特征
 * 2. 支持多种分布模式（均匀、高斯、帕累托）
 * 3. 批处理窗口，同一窗口内的包共享延迟值
 */

use rand::Rng;
use serde::{Deserialize, Serialize};
use std::time::Duration;

/// 时序混淆配置
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TimingJitterConfig {
    /// 启用时序混淆
    pub enabled: bool,
    /// 延迟分布模式
    pub mode: JitterMode,
    /// 最小延迟（毫秒）
    pub min_delay_ms: u64,
    /// 最大延迟（毫秒）
    pub max_delay_ms: u64,
    /// 批处理窗口（毫秒），同一窗口内的包统一延迟
    pub batch_window_ms: u64,
}

impl Default for TimingJitterConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            mode: JitterMode::Uniform,
            min_delay_ms: 50,
            max_delay_ms: 200,
            batch_window_ms: 100,
        }
    }
}

impl TimingJitterConfig {
    pub fn conservative() -> Self {
        Self {
            enabled: true,
            mode: JitterMode::Uniform,
            min_delay_ms: 50,
            max_delay_ms: 200,
            batch_window_ms: 100,
        }
    }

    pub fn aggressive() -> Self {
        Self {
            enabled: true,
            mode: JitterMode::Gaussian,
            min_delay_ms: 100,
            max_delay_ms: 800,
            batch_window_ms: 50,
        }
    }

    pub fn validate(&self) -> Result<(), String> {
        if !self.enabled {
            return Ok(());
        }
        if self.min_delay_ms == 0 && self.max_delay_ms == 0 {
            return Err("时序混淆延迟范围不能同时为 0".to_string());
        }
        if self.min_delay_ms > self.max_delay_ms {
            return Err("时序混淆最小延迟不能大于最大延迟".to_string());
        }
        if self.batch_window_ms == 0 {
            return Err("批处理窗口必须大于 0".to_string());
        }
        Ok(())
    }
}

/// 延迟分布模式
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum JitterMode {
    /// 均匀随机 [min, max]
    Uniform,
    /// 高斯分布，μ=(min+max)/2, σ=(max-min)/6
    Gaussian,
    /// 重尾分布，偶尔长延迟
    Pareto,
}

/// 时序混淆统计
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TimingJitterStats {
    /// 被延迟的包数量
    pub delayed_count: u64,
    /// 总延迟时间（毫秒）
    pub total_delay_ms: u64,
    /// 当前批处理窗口的延迟值（毫秒）
    pub current_batch_delay_ms: u64,
}

impl Default for TimingJitterStats {
    fn default() -> Self {
        Self {
            delayed_count: 0,
            total_delay_ms: 0,
            current_batch_delay_ms: 0,
        }
    }
}

/// 时序混淆引擎
#[allow(dead_code)]
pub struct TimingJitterEngine {
    config: TimingJitterConfig,
    stats: TimingJitterStats,
    /// 当前批处理窗口的起始时间戳（毫秒）
    #[allow(dead_code)]
    batch_start_ms: u64,
    /// 当前批处理窗口的延迟值
    batch_delay: Duration,
}

impl TimingJitterEngine {
    pub fn new(config: TimingJitterConfig) -> Self {
        let initial_delay = Self::calculate_delay(&config);
        Self {
            config,
            stats: TimingJitterStats::default(),
            batch_start_ms: 0,
            batch_delay: initial_delay,
        }
    }

    /// 更新配置
    pub fn update_config(&mut self, config: TimingJitterConfig) {
        self.config = config;
        self.batch_delay = Self::calculate_delay(&self.config);
    }

    /// 获取当前配置
    #[allow(dead_code)]
    pub fn config(&self) -> &TimingJitterConfig {
        &self.config
    }

    /// 获取统计
    pub fn stats(&self) -> &TimingJitterStats {
        &self.stats
    }

    /// 重置统计
    #[allow(dead_code)]
    pub fn reset_stats(&mut self) {
        self.stats = TimingJitterStats::default();
        log::info!("📊 Timing jitter stats reset");
    }

    /// 计算一个包的延迟时间
    /// `now_ms`: 当前时间戳（毫秒）
    #[allow(dead_code)]
    pub fn get_delay(&mut self, now_ms: u64) -> Duration {
        if !self.config.enabled {
            return Duration::ZERO;
        }

        // 检查是否需要刷新批处理窗口
        if now_ms - self.batch_start_ms >= self.config.batch_window_ms {
            self.batch_start_ms = now_ms;
            self.batch_delay = Self::calculate_delay(&self.config);
            self.stats.current_batch_delay_ms = self.batch_delay.as_millis() as u64;
        }

        self.stats.delayed_count += 1;
        self.stats.total_delay_ms += self.batch_delay.as_millis() as u64;

        self.batch_delay
    }

    /// 根据配置和分布模式计算延迟
    fn calculate_delay(config: &TimingJitterConfig) -> Duration {
        if config.min_delay_ms == 0 && config.max_delay_ms == 0 {
            return Duration::ZERO;
        }

        let delay_ms = match config.mode {
            JitterMode::Uniform => {
                let mut rng = rand::thread_rng();
                rng.gen_range(config.min_delay_ms..=config.max_delay_ms)
            }
            JitterMode::Gaussian => {
                let mu = (config.min_delay_ms + config.max_delay_ms) as f64 / 2.0;
                let sigma = (config.max_delay_ms - config.min_delay_ms) as f64 / 6.0;
                // Box-Muller 变换
                let mut rng = rand::thread_rng();
                let u1: f64 = rng.r#gen::<f64>().max(1e-10);
                let u2: f64 = rng.r#gen::<f64>();
                let z = (-2.0 * u1.ln()).sqrt() * (2.0 * std::f64::consts::PI * u2).cos();
                let sample = mu + sigma * z;
                sample.round().clamp(config.min_delay_ms as f64, config.max_delay_ms as f64) as u64
            }
            JitterMode::Pareto => {
                // 简化 Pareto：大部分包短延迟，偶尔长延迟
                let mut rng = rand::thread_rng();
                let r: f64 = rng.r#gen::<f64>();
                let alpha = 1.5; // 形状参数
                let x_m = config.min_delay_ms as f64;
                // Pareto CDF 反函数: x = x_m / (1 - u)^(1/alpha)
                let sample = x_m / (1.0 - r).max(1e-10).powf(1.0 / alpha);
                sample.round().clamp(config.min_delay_ms as f64, config.max_delay_ms as f64) as u64
            }
        };

        Duration::from_millis(delay_ms)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = TimingJitterConfig::default();
        assert!(!config.enabled);
        assert!(matches!(config.mode, JitterMode::Uniform));
    }

    #[test]
    fn test_conservative_config() {
        let config = TimingJitterConfig::conservative();
        assert!(config.enabled);
        assert!(matches!(config.mode, JitterMode::Uniform));
    }

    #[test]
    fn test_aggressive_config() {
        let config = TimingJitterConfig::aggressive();
        assert!(config.enabled);
        assert!(matches!(config.mode, JitterMode::Gaussian));
    }

    #[test]
    fn test_validate_ok() {
        assert!(TimingJitterConfig::default().validate().is_ok());
        assert!(TimingJitterConfig::conservative().validate().is_ok());
        assert!(TimingJitterConfig::aggressive().validate().is_ok());
    }

    #[test]
    fn test_validate_min_greater_than_max() {
        let config = TimingJitterConfig {
            enabled: true,
            min_delay_ms: 300,
            max_delay_ms: 100,
            ..Default::default()
        };
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_calculate_delay_uniform() {
        let config = TimingJitterConfig {
            enabled: true,
            mode: JitterMode::Uniform,
            min_delay_ms: 50,
            max_delay_ms: 200,
            ..Default::default()
        };
        for _ in 0..100 {
            let delay = TimingJitterEngine::calculate_delay(&config);
            let ms = delay.as_millis() as u64;
            assert!(ms >= 50 && ms <= 200, "delay {} out of range", ms);
        }
    }

    #[test]
    fn test_calculate_delay_gaussian() {
        let config = TimingJitterConfig {
            enabled: true,
            mode: JitterMode::Gaussian,
            min_delay_ms: 100,
            max_delay_ms: 800,
            ..Default::default()
        };
        for _ in 0..100 {
            let delay = TimingJitterEngine::calculate_delay(&config);
            let ms = delay.as_millis() as u64;
            assert!(ms >= 100 && ms <= 800, "delay {} out of range", ms);
        }
    }

    #[test]
    fn test_calculate_delay_pareto() {
        let config = TimingJitterConfig {
            enabled: true,
            mode: JitterMode::Pareto,
            min_delay_ms: 50,
            max_delay_ms: 500,
            ..Default::default()
        };
        for _ in 0..100 {
            let delay = TimingJitterEngine::calculate_delay(&config);
            let ms = delay.as_millis() as u64;
            assert!(ms >= 50 && ms <= 500, "delay {} out of range", ms);
        }
    }

    #[test]
    fn test_engine_get_delay_disabled() {
        let config = TimingJitterConfig::default();
        let mut engine = TimingJitterEngine::new(config);
        let delay = engine.get_delay(1000);
        assert_eq!(delay, Duration::ZERO);
    }

    #[test]
    fn test_engine_get_delay_batch_window() {
        let config = TimingJitterConfig {
            enabled: true,
            mode: JitterMode::Uniform,
            min_delay_ms: 100,
            max_delay_ms: 100,
            batch_window_ms: 50,
            ..Default::default()
        };
        let mut engine = TimingJitterEngine::new(config);

        // 同一批窗口内，延迟应相同
        let d1 = engine.get_delay(1000);
        let d2 = engine.get_delay(1020);
        assert_eq!(d1, d2);

        // 超过窗口后，重新计算（固定 100ms，所以仍然 100）
        let d3 = engine.get_delay(1060);
        assert_eq!(d3, Duration::from_millis(100));
    }

    #[test]
    fn test_engine_stats() {
        let config = TimingJitterConfig {
            enabled: true,
            mode: JitterMode::Uniform,
            min_delay_ms: 50,
            max_delay_ms: 50,
            batch_window_ms: 100,
            ..Default::default()
        };
        let mut engine = TimingJitterEngine::new(config);
        engine.get_delay(1000);
        engine.get_delay(1010);

        let stats = engine.stats();
        assert_eq!(stats.delayed_count, 2);
        assert_eq!(stats.total_delay_ms, 100);
    }

    #[test]
    fn test_engine_reset_stats() {
        let config = TimingJitterConfig {
            enabled: true,
            mode: JitterMode::Uniform,
            min_delay_ms: 50,
            max_delay_ms: 50,
            batch_window_ms: 100,
            ..Default::default()
        };
        let mut engine = TimingJitterEngine::new(config);
        engine.get_delay(1000);
        engine.reset_stats();

        let stats = engine.stats();
        assert_eq!(stats.delayed_count, 0);
        assert_eq!(stats.total_delay_ms, 0);
    }

    #[test]
    fn test_engine_update_config() {
        let config = TimingJitterConfig::conservative();
        let mut engine = TimingJitterEngine::new(config);
        engine.update_config(TimingJitterConfig::aggressive());
        assert!(matches!(engine.config().mode, JitterMode::Gaussian));
    }
}
