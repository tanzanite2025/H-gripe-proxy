/**
 * 出口 IP 监控器 — 配置与数据类型
 */

use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};

// ── 重绑定策略类型 ──────────────────────────────────────────────────

/// 重绑定策略选择
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum RebindStrategyType {
    /// 同画像优先：选择与变化前 IP 同国家/同类型的最优节点
    Smart,
    /// 简单轮转：切换到下一个节点
    RoundRobin,
}

impl Default for RebindStrategyType {
    fn default() -> Self {
        Self::Smart
    }
}

// ── 配置 ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EgressMonitorConfig {
    /// 是否启用出口监控
    pub enabled: bool,
    /// 探测间隔（秒），默认 120
    pub probe_interval_secs: u64,
    /// IP 变化时是否自动重绑定到同画像其他节点
    pub auto_rebind_on_change: bool,
    /// IP 变化时是否通知前端
    pub notify_on_change: bool,
    /// 探测超时（秒），默认 10
    pub probe_timeout_secs: u64,
    /// 代理组变化检测轮询间隔（秒），默认 30，最小 5
    pub watch_poll_interval_secs: u64,
    /// 重绑定策略，默认 smart
    pub rebind_strategy: RebindStrategyType,
}

impl Default for EgressMonitorConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            probe_interval_secs: 120,
            auto_rebind_on_change: false,
            notify_on_change: true,
            probe_timeout_secs: 10,
            watch_poll_interval_secs: 30,
            rebind_strategy: RebindStrategyType::default(),
        }
    }
}

impl EgressMonitorConfig {
    pub fn validate(&self) -> Result<()> {
        if self.probe_interval_secs == 0 {
            return Err(anyhow!("probe_interval_secs 必须 > 0"));
        }
        if self.probe_timeout_secs == 0 {
            return Err(anyhow!("probe_timeout_secs 必须 > 0"));
        }
        if self.watch_poll_interval_secs < 5 {
            return Err(anyhow!("watch_poll_interval_secs 必须 >= 5"));
        }
        Ok(())
    }
}

// ── IP 探测结果 ───────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EgressIpProbeResult {
    /// 探测到的出口 IP
    pub ip: String,
    /// IP 归属国家代码
    pub country_code: Option<String>,
    /// 探测时间戳 (ms)
    pub probed_at_ms: u64,
    /// 探测耗时 (ms)
    pub latency_ms: u64,
}

// ── IP 变化事件 ────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EgressIpChangeEvent {
    /// 变化前的 IP
    pub previous_ip: String,
    /// 变化后的 IP
    pub current_ip: String,
    /// 变化前的国家代码
    pub previous_country: Option<String>,
    /// 变化后的国家代码
    pub current_country: Option<String>,
    /// 事件时间戳 (ms)
    pub timestamp_ms: u64,
    /// 是否已自动重绑定
    pub auto_rebind_applied: bool,
}

// ── 监控统计 ──────────────────────────────────────────────────────────

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EgressMonitorStats {
    /// 总探测次数
    pub total_probes: u64,
    /// 探测成功次数
    pub successful_probes: u64,
    /// 探测失败次数
    pub failed_probes: u64,
    /// IP 变化次数
    pub ip_change_count: u64,
    /// 自动重绑定次数
    pub auto_rebind_count: u64,
    /// 最后一次成功探测结果
    pub last_probe: Option<EgressIpProbeResult>,
    /// 最后一次 IP 变化事件
    pub last_change: Option<EgressIpChangeEvent>,
    /// 监控运行时长 (秒)
    pub uptime_secs: u64,
}
