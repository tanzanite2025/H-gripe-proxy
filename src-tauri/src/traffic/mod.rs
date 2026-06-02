pub mod direction;
/**
 * 流量模块
 *
 * 真正的流量混淆由 Go 内核（mihomo）的 ObfuscatedConn 在连接层执行。
 * Rust 端仅持有配置并通过 Mihomo API 获取真实统计。
 *
 * 包含：
 * - padding: 流量填充配置（仅配置，不执行）
 * - timing_jitter: 时序混淆配置（仅配置，不执行）
 * - direction: 方向混淆配置（仅配置，不执行）
 * - scheduler: 流量混淆薄代理层
 */
pub mod padding;
pub mod scheduler;
pub mod timing_jitter;

pub use padding::TrafficPaddingConfig;

pub use scheduler::{ObfuscationProfile, ObfuscationScheduler, ObfuscationStats, TrafficObfuscationConfig};
