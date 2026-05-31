/**
 * 流量填充配置模块
 *
 * 注意：真正的流量填充由 Go 内核（mihomo）的 ObfuscatedConn 在连接层执行。
 * 此模块仅定义配置结构，供前端面板和 profile 派生使用。
 */

use serde::{Deserialize, Serialize};

/// 流量填充配置
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TrafficPaddingConfig {
    /// 启用填充
    pub enabled: bool,
    /// 最小填充大小（字节）
    pub min_size: usize,
    /// 最大填充大小（字节）
    pub max_size: usize,
    /// 加密填充数据
    pub encrypt: bool,
    /// 填充强度
    pub intensity: PaddingIntensity,
    /// 填充频率
    pub frequency: PaddingFrequency,
    /// 填充时机
    pub timing: PaddingTiming,
    /// 智能填充
    pub smart_padding: bool,
    /// 性能控制
    pub performance_control: PerformanceControl,
}

impl Default for TrafficPaddingConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            min_size: 512,
            max_size: 4096,
            encrypt: true,
            intensity: PaddingIntensity::Medium,
            frequency: PaddingFrequency {
                freq_type: FrequencyType::Time,
                interval: 10,
            },
            timing: PaddingTiming::Random,
            smart_padding: true,
            performance_control: PerformanceControl::default(),
        }
    }
}

/// 填充强度
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PaddingIntensity {
    Low,
    Medium,
    High,
    Custom(f32),
}

impl PaddingIntensity {
    pub fn as_multiplier(&self) -> f32 {
        match self {
            PaddingIntensity::Low => 0.5,
            PaddingIntensity::Medium => 1.0,
            PaddingIntensity::High => 2.0,
            PaddingIntensity::Custom(m) => *m,
        }
    }
}

/// 填充频率
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PaddingFrequency {
    pub freq_type: FrequencyType,
    pub interval: u64,
}

/// 频率类型
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FrequencyType {
    Time,      // 每 N 秒
    Request,   // 每 N 请求
    Random,    // 随机
}

/// 填充时机
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PaddingTiming {
    Before,    // 请求前
    After,     // 请求后
    Random,    // 随机
}

/// 性能控制
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PerformanceControl {
    /// 最大带宽（字节/秒）
    pub max_bandwidth: usize,
    /// 最大 CPU 使用率（%）
    pub max_cpu_usage: f32,
    /// 最大内存（字节）
    pub max_memory: usize,
    /// 自动降级
    pub auto_downgrade: bool,
}

impl Default for PerformanceControl {
    fn default() -> Self {
        Self {
            max_bandwidth: 1024 * 1024, // 1 MB/s
            max_cpu_usage: 5.0,          // 5%
            max_memory: 10 * 1024 * 1024, // 10 MB
            auto_downgrade: true,
        }
    }
}
