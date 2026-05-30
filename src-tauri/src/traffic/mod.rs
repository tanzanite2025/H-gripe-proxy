/**
 * 流量模块
 * 
 * 包含：
 * - padding: 流量填充
 * - timing_jitter: 时序混淆
 * - direction: 方向混淆
 * - scheduler: 流量混淆统一调度器
 */

pub mod padding;
pub mod timing_jitter;
pub mod direction;
pub mod scheduler;

#[allow(unused_imports)]
pub use padding::{
    TrafficPaddingConfig,
    PaddingIntensity,
    PaddingFrequency,
    FrequencyType,
    PaddingTiming,
    PerformanceControl,
    PaddingStats,
    PaddingScheduler,
};

#[allow(unused_imports)]
pub use timing_jitter::{
    TimingJitterConfig,
    JitterMode,
    TimingJitterStats,
    TimingJitterEngine,
};

#[allow(unused_imports)]
pub use direction::{
    DirectionObfuscationConfig,
    DirectionMode,
    DirectionStats,
    DirectionObfuscator,
};

#[allow(unused_imports)]
pub use scheduler::{
    ObfuscationProfile,
    TrafficObfuscationConfig,
    ObfuscationStats,
    ObfuscationScheduler,
};
