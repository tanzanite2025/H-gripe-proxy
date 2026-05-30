/**
 * 代理组变化检测
 *
 * 定期轮询 Mihomo 代理组选中节点，当检测到 VERGE-STABLE-* 组
 * 的选中节点发生变化时（如 url-test/fallback 自动切换），
 * 自动触发 sync_runtime_stable_egress_selection 回写，
 * 确保 egress_identity 和 session_affinity 与 Mihomo 运行态一致。
 */

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;

use parking_lot::RwLock;
use tokio::sync::Notify;
use tokio::time::Duration;

use crate::core::handle;
use crate::process::AsyncHandler;

/// 代理组变化检测器
pub struct ProxyGroupWatcher {
    /// 上次快照：group_name -> selected_node
    snapshot: Arc<RwLock<HashMap<String, String>>>,
    /// 轮询间隔（秒）
    poll_interval_secs: Arc<RwLock<u64>>,
    /// 防抖冷却窗口（秒）：两次回写之间的最小间隔
    debounce_secs: Arc<RwLock<u64>>,
    /// 上次回写时间
    last_backwrite: Arc<RwLock<Option<Instant>>>,
    /// 待处理的变更（debounce 期间累积）
    pending_changes: Arc<RwLock<bool>>,
    /// 取消通知
    cancel_notify: Arc<Notify>,
    /// 是否运行中
    running: Arc<RwLock<bool>>,
}

impl ProxyGroupWatcher {
    pub fn new() -> Self {
        Self {
            snapshot: Arc::new(RwLock::new(HashMap::new())),
            poll_interval_secs: Arc::new(RwLock::new(30)),
            debounce_secs: Arc::new(RwLock::new(10)),
            last_backwrite: Arc::new(RwLock::new(None)),
            pending_changes: Arc::new(RwLock::new(false)),
            cancel_notify: Arc::new(Notify::new()),
            running: Arc::new(RwLock::new(false)),
        }
    }

    pub fn set_poll_interval(&self, secs: u64) {
        *self.poll_interval_secs.write() = secs.max(5);
    }

    /// 设置防抖冷却窗口（秒），最小 5 秒
    pub fn set_debounce_secs(&self, secs: u64) {
        *self.debounce_secs.write() = secs.max(5);
    }

    /// 启动代理组变化检测循环
    pub fn start(&self) {
        if *self.running.read() {
            return;
        }
        *self.running.write() = true;

        let snapshot = self.snapshot.clone();
        let poll_interval_secs = self.poll_interval_secs.clone();
        let debounce_secs = self.debounce_secs.clone();
        let last_backwrite = self.last_backwrite.clone();
        let pending_changes = self.pending_changes.clone();
        let cancel_notify = self.cancel_notify.clone();
        let running = self.running.clone();

        AsyncHandler::spawn(move || async move {
            log::info!("[ProxyGroupWatcher] 代理组变化检测已启动");

            // 初始快照
            if let Ok(proxies) = handle::Handle::mihomo().await.get_proxies().await {
                let initial = collect_stable_group_selections(&proxies);
                *snapshot.write() = initial;
            }

            loop {
                let interval = *poll_interval_secs.read();
                let sleep = tokio::time::sleep(Duration::from_secs(interval));
                tokio::pin!(sleep);

                tokio::select! {
                    _ = &mut sleep => {},
                    _ = cancel_notify.notified() => {
                        log::info!("[ProxyGroupWatcher] 收到停止信号");
                        break;
                    }
                }

                // 轮询当前选中节点
                let current = match handle::Handle::mihomo().await.get_proxies().await {
                    Ok(p) => collect_stable_group_selections(&p),
                    Err(e) => {
                        log::warn!("[ProxyGroupWatcher] 获取代理组失败: {:?}", e);
                        continue;
                    }
                };

                // 对比变化
                let prev = snapshot.read().clone();
                let changes = detect_changes(&prev, &current);

                if !changes.is_empty() {
                    for (group, old_node, new_node) in &changes {
                        log::info!(
                            "[ProxyGroupWatcher] 检测到代理组切换: {} 从 {} -> {}",
                            group,
                            old_node,
                            new_node,
                        );
                    }

                    // 防抖：距离上次回写不足冷却窗口时，标记待处理但不立即回写
                    let debounce = Duration::from_secs(*debounce_secs.read());
                    let now = Instant::now();
                    let last = *last_backwrite.read();
                    let can_write = last.map_or(true, |t| now.duration_since(t) >= debounce);

                    if can_write {
                        trigger_backwrite().await;
                        *last_backwrite.write() = Some(now);
                        *pending_changes.write() = false;
                    } else {
                        // 标记有待处理的变更，下次轮询时如果冷却期已过则回写
                        *pending_changes.write() = true;
                        log::info!(
                            "[ProxyGroupWatcher] 防抖中，延迟回写（冷却窗口 {}s）",
                            debounce.as_secs(),
                        );
                    }
                } else if *pending_changes.read() {
                    // 无新变化但有待处理的变更，检查冷却窗口是否已过
                    let debounce = Duration::from_secs(*debounce_secs.read());
                    let now = Instant::now();
                    let last = *last_backwrite.read();
                    let can_write = last.map_or(true, |t| now.duration_since(t) >= debounce);

                    if can_write {
                        log::info!("[ProxyGroupWatcher] 防抖冷却结束，执行延迟回写");
                        trigger_backwrite().await;
                        *last_backwrite.write() = Some(now);
                        *pending_changes.write() = false;
                    }
                }

                // 更新快照
                *snapshot.write() = current;
            }

            *running.write() = false;
            log::info!("[ProxyGroupWatcher] 代理组变化检测已停止");
        });
    }

    /// 停止检测循环
    pub fn stop(&self) {
        if !*self.running.read() {
            return;
        }
        self.cancel_notify.notify_one();
    }
}

/// 从代理数据中收集 VERGE-STABLE-* 组的当前选中节点
fn collect_stable_group_selections(
    proxies: &tauri_plugin_mihomo::models::Proxies,
) -> HashMap<String, String> {
    let mut result = HashMap::new();
    for (name, data) in proxies.proxies.iter() {
        if name.starts_with("VERGE-STABLE-") {
            if let Some(ref selected) = data.now {
                result.insert(name.clone(), selected.clone());
            }
        }
    }
    result
}

/// 检测选中节点变化，返回 (group_name, old_node, new_node) 列表
fn detect_changes(
    prev: &HashMap<String, String>,
    current: &HashMap<String, String>,
) -> Vec<(String, String, String)> {
    let mut changes = Vec::new();

    for (group, new_node) in current {
        if let Some(old_node) = prev.get(group) {
            if old_node != new_node {
                changes.push((group.clone(), old_node.clone(), new_node.clone()));
            }
        }
    }

    changes
}

/// 触发 sync_runtime_stable_egress_selection 回写
async fn trigger_backwrite() {
    if let Some(runtime_config) = crate::config::Config::runtime().await.latest_arc().config.clone() {
        let coordinator = crate::feat::get_coordinator();
        let session_affinity = crate::feat::get_session_affinity_manager();
        let ip_reputation = crate::feat::get_ip_reputation_manager();
        if let Err(e) = crate::core::stable_egress::sync_runtime_stable_egress_selection(
            &coordinator,
            &session_affinity,
            &ip_reputation,
            &runtime_config,
        ).await {
            log::warn!("[ProxyGroupWatcher] 回写失败: {}", e);
        }
    }
}
