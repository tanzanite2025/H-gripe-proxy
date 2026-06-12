/**
 * 出口 IP 监控器
 *
 * 定时探测出口 IP，发现变化时：
 * 1. 记录 IP 变化事件
 * 2. 通知前端
 * 3. 若启用自动重绑定，通过 RebindStrategy 策略切换节点
 */
use std::sync::Arc;
use std::time::Duration;

use anyhow::{Result, anyhow};
use parking_lot::RwLock;
use tokio::sync::Notify;
use tokio::time::Instant;

use crate::core::handle;
use crate::core::timezone_spoof::remember_observed_egress_region;
use crate::process::AsyncHandler;

pub mod config;
pub mod probe;
pub mod rebind;
pub mod watch;

// 从子模块重新导出公共类型，保持外部 API 不变
pub use config::{
    EgressIpChangeEvent, EgressIpProbeResult, EgressMonitorConfig, EgressMonitorStats, RebindStrategyType,
};
pub use rebind::{RebindStrategy, RoundRobinRebind, SmartRebind};
pub use watch::ProxyGroupWatcher;

// ── 监控器 ─────────────────────────────────────────────────────────────

pub struct EgressMonitor {
    config: Arc<RwLock<EgressMonitorConfig>>,
    stats: Arc<RwLock<EgressMonitorStats>>,
    last_known_ip: Arc<RwLock<Option<String>>>,
    last_known_country: Arc<RwLock<Option<String>>>,
    started_at: Arc<RwLock<Option<Instant>>>,
    cancel_notify: Arc<Notify>,
    running: Arc<RwLock<bool>>,
    rebind_strategy: Arc<RwLock<Arc<dyn RebindStrategy>>>,
    watcher: ProxyGroupWatcher,
}

impl EgressMonitor {
    pub fn new() -> Self {
        Self::with_strategy(Arc::new(SmartRebind))
    }

    /// 使用自定义重绑定策略创建监控器
    pub fn with_strategy(strategy: Arc<dyn RebindStrategy>) -> Self {
        Self {
            config: Arc::new(RwLock::new(EgressMonitorConfig::default())),
            stats: Arc::new(RwLock::new(EgressMonitorStats::default())),
            last_known_ip: Arc::new(RwLock::new(None)),
            last_known_country: Arc::new(RwLock::new(None)),
            started_at: Arc::new(RwLock::new(None)),
            cancel_notify: Arc::new(Notify::new()),
            running: Arc::new(RwLock::new(false)),
            rebind_strategy: Arc::new(RwLock::new(strategy)),
            watcher: ProxyGroupWatcher::new(),
        }
    }

    pub fn update_config(&self, config: EgressMonitorConfig) -> Result<()> {
        config.validate()?;
        // 策略变化时重建 rebind_strategy
        let old_strategy = self.config.read().rebind_strategy;
        *self.config.write() = config;
        let new_strategy = self.config.read().rebind_strategy;
        if old_strategy != new_strategy {
            *self.rebind_strategy.write() = strategy_from_type(new_strategy);
        }
        Ok(())
    }

    pub fn get_stats(&self) -> EgressMonitorStats {
        let mut stats = self.stats.read().clone();
        if let Some(started_at) = *self.started_at.read() {
            stats.uptime_secs = started_at.elapsed().as_secs();
        }
        stats
    }

    pub fn is_running(&self) -> bool {
        *self.running.read()
    }

    #[cfg(test)]
    pub fn get_config(&self) -> EgressMonitorConfig {
        self.config.read().clone()
    }

    /// 启动监控循环
    pub fn start(&self) {
        if *self.running.read() {
            return;
        }
        *self.running.write() = true;
        *self.started_at.write() = Some(Instant::now());

        // 启动代理组变化检测
        let watch_interval = self.config.read().watch_poll_interval_secs;
        let watch_debounce = self.config.read().watch_debounce_secs;
        self.watcher.set_poll_interval(watch_interval);
        self.watcher.set_debounce_secs(watch_debounce);
        self.watcher.start();

        let config = self.config.clone();
        let stats = self.stats.clone();
        let last_known_ip = self.last_known_ip.clone();
        let last_known_country = self.last_known_country.clone();
        let cancel_notify = self.cancel_notify.clone();
        let running = self.running.clone();
        let rebind_strategy = self.rebind_strategy.clone();

        AsyncHandler::spawn(move || async move {
            loop {
                let interval_secs = config.read().probe_interval_secs;
                let timeout_secs = config.read().probe_timeout_secs;

                // 执行探测
                let probe_result =
                    tokio::time::timeout(Duration::from_secs(timeout_secs), probe::probe_egress_ip()).await;

                match probe_result {
                    Ok(Ok(probe)) => {
                        {
                            let mut s = (*stats).write();
                            s.total_probes += 1;
                            s.successful_probes += 1;
                            s.last_probe = Some(probe.clone());
                        }

                        // 检查 IP 是否变化
                        let prev_ip = last_known_ip.read().clone();
                        let prev_country = last_known_country.read().clone();

                        if let Some(ref prev) = prev_ip {
                            if prev != &probe.ip {
                                // IP 变化！
                                let auto_rebind = config.read().auto_rebind_on_change;
                                let notify = config.read().notify_on_change;
                                let coordinator = crate::core::coordinator::get_coordinator();
                                let drift_policy =
                                    crate::core::stable_egress::current_egress_support_policy(&coordinator);

                                let rebind_applied = if auto_rebind && !drift_policy.minimize_drift {
                                    let strategy = rebind_strategy.read().clone();
                                    strategy
                                        .rebind(rebind::RebindContext {
                                            previous_ip: prev.clone(),
                                            current_ip: probe.ip.clone(),
                                            previous_country: prev_country.clone(),
                                            current_country: probe.country_code.clone(),
                                        })
                                        .await
                                } else {
                                    if auto_rebind && drift_policy.minimize_drift {
                                        log::info!(
                                            "[EgressMonitor] 跳过自动重绑定，当前入口威胁级别为 {:?}，优先减少出口漂移",
                                            drift_policy.strongest_threat_level
                                        );
                                    }
                                    false
                                };

                                let change_event = EgressIpChangeEvent {
                                    previous_ip: prev.clone(),
                                    current_ip: probe.ip.clone(),
                                    previous_country: prev_country.clone(),
                                    current_country: probe.country_code.clone(),
                                    timestamp_ms: probe::now_ms(),
                                    auto_rebind_applied: rebind_applied,
                                };

                                log::warn!(
                                    "[EgressMonitor] 出口 IP 变化: {} -> {} ({} -> {}){}",
                                    prev,
                                    probe.ip,
                                    prev_country.as_deref().unwrap_or("??"),
                                    probe.country_code.as_deref().unwrap_or("??"),
                                    if rebind_applied { " [已自动重绑定]" } else { "" }
                                );

                                {
                                    let mut g = (*stats).write();
                                    g.ip_change_count += 1;
                                    if rebind_applied {
                                        g.auto_rebind_count += 1;
                                    }
                                    g.last_change = Some(change_event.clone());
                                }

                                if notify {
                                    let _ = handle::Handle::notice_message(
                                        "egress_ip_changed",
                                        format!("出口 IP 变化: {} → {}", prev, probe.ip),
                                    );
                                }
                            }
                        }

                        *last_known_ip.write() = Some(probe.ip.clone());
                        *last_known_country.write() = probe.country_code.clone();
                        remember_observed_egress_region(
                            probe.country_code.as_deref(),
                            probe.timezone.as_deref(),
                            "egressMonitor",
                        );
                    }
                    Ok(Err(err)) => {
                        {
                            let mut g = (*stats).write();
                            g.total_probes += 1;
                            g.failed_probes += 1;
                        }
                        log::warn!("[EgressMonitor] 探测失败: {}", err);
                    }
                    Err(_) => {
                        {
                            let mut g = (*stats).write();
                            g.total_probes += 1;
                            g.failed_probes += 1;
                        }
                        log::warn!("[EgressMonitor] 探测超时 ({}s)", timeout_secs);
                    }
                }

                // 等待下一次探测，或被取消
                let sleep = tokio::time::sleep(Duration::from_secs(interval_secs));
                tokio::pin!(sleep);

                tokio::select! {
                    _ = &mut sleep => {},
                    _ = cancel_notify.notified() => {
                        log::info!("[EgressMonitor] 收到停止信号");
                        break;
                    }
                }
            }

            *running.write() = false;
            log::info!("[EgressMonitor] 监控已停止");
        });
    }

    /// 停止监控循环
    pub fn stop(&self) {
        if !*self.running.read() {
            return;
        }
        self.watcher.stop();
        self.cancel_notify.notify_one();
    }

    /// 手动触发一次探测
    pub async fn probe_now(&self) -> Result<EgressIpProbeResult> {
        let timeout_secs = self.config.read().probe_timeout_secs;
        let start = Instant::now();

        let probe = tokio::time::timeout(Duration::from_secs(timeout_secs), probe::probe_egress_ip())
            .await
            .map_err(|_| anyhow!("探测超时 ({}s)", timeout_secs))??;

        let latency_ms = start.elapsed().as_millis() as u64;

        // 更新 last_known
        let prev_ip = self.last_known_ip.read().clone();
        let prev_country = self.last_known_country.read().clone();
        *self.last_known_ip.write() = Some(probe.ip.clone());
        *self.last_known_country.write() = probe.country_code.clone();
        remember_observed_egress_region(
            probe.country_code.as_deref(),
            probe.timezone.as_deref(),
            "egressMonitor",
        );

        // 检查是否有变化
        if let Some(ref prev) = prev_ip {
            if prev != &probe.ip {
                let mut stats = self.stats.write();
                stats.ip_change_count += 1;
                let change_event = EgressIpChangeEvent {
                    previous_ip: prev.clone(),
                    current_ip: probe.ip.clone(),
                    previous_country: prev_country,
                    current_country: probe.country_code.clone(),
                    timestamp_ms: probe::now_ms(),
                    auto_rebind_applied: false,
                };
                stats.last_change = Some(change_event);
            }
        }

        let result = EgressIpProbeResult {
            ip: probe.ip.clone(),
            country_code: probe.country_code.clone(),
            city: probe.city.clone(),
            timezone: probe.timezone.clone(),
            probed_at_ms: probe.probed_at_ms,
            latency_ms,
        };

        {
            let mut stats_w = self.stats.write();
            stats_w.total_probes += 1;
            stats_w.successful_probes += 1;
            stats_w.last_probe = Some(result.clone());
        }

        Ok(result)
    }

    /// 重置统计
    pub fn reset_stats(&self) {
        *self.stats.write() = EgressMonitorStats::default();
    }
}

// ── 策略工厂 ──────────────────────────────────────────────────────────

fn strategy_from_type(strategy_type: RebindStrategyType) -> Arc<dyn RebindStrategy> {
    match strategy_type {
        RebindStrategyType::Smart => Arc::new(SmartRebind),
        RebindStrategyType::RoundRobin => Arc::new(RoundRobinRebind),
    }
}

// ── 单例 ──────────────────────────────────────────────────────────────

use crate::singleton;

singleton!(EgressMonitor, EGRESS_MONITOR);

/// 获取全局 EgressMonitor 单例引用
pub fn egress_monitor() -> &'static EgressMonitor {
    EgressMonitor::global()
}

// ── 测试 ──────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_validation() {
        let config = EgressMonitorConfig::default();
        assert!(config.validate().is_ok());

        let bad_config = EgressMonitorConfig {
            probe_interval_secs: 0,
            ..Default::default()
        };
        assert!(bad_config.validate().is_err());
    }

    #[test]
    fn test_stats_default() {
        let stats = EgressMonitorStats::default();
        assert_eq!(stats.total_probes, 0);
        assert_eq!(stats.ip_change_count, 0);
        assert!(stats.last_probe.is_none());
    }

    #[test]
    fn test_monitor_creation() {
        let monitor = EgressMonitor::new();
        assert!(!monitor.is_running());
        let config = monitor.get_config();
        assert!(!config.enabled);
        assert_eq!(config.probe_interval_secs, 120);
    }
}
