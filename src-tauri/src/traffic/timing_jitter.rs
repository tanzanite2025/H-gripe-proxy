/**
 * 时序混淆配置模块
 *
 * 注意：真正的时序混淆由 Go 内核（mihomo）的 ObfuscatedConn 在连接层执行。
 * 此模块仅定义配置结构，供前端面板和 profile 派生使用。
 */
use serde::{Deserialize, Serialize};

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
