use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;

use parking_lot::RwLock;
use tokio::sync::Notify;
use tokio::time::Duration;

use crate::process::AsyncHandler;

pub struct ProxyGroupWatcher {
    snapshot: Arc<RwLock<HashMap<String, String>>>,
    poll_interval_secs: Arc<RwLock<u64>>,
    debounce_secs: Arc<RwLock<u64>>,
    last_backwrite: Arc<RwLock<Option<Instant>>>,
    pending_changes: Arc<RwLock<bool>>,
    cancel_notify: Arc<Notify>,
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

    pub fn set_debounce_secs(&self, secs: u64) {
        *self.debounce_secs.write() = secs.max(5);
    }

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
            log::info!("[ProxyGroupWatcher] started");
            let snapshot_service = crate::core::runtime_snapshot::RuntimeSnapshotService::global();

            *snapshot.write() = snapshot_service.refresh_proxies().await.stable_group_selected_nodes();

            loop {
                let interval = *poll_interval_secs.read();
                let sleep = tokio::time::sleep(Duration::from_secs(interval));
                tokio::pin!(sleep);

                tokio::select! {
                    _ = &mut sleep => {},
                    _ = cancel_notify.notified() => {
                        log::info!("[ProxyGroupWatcher] stop requested");
                        break;
                    }
                }

                let current = snapshot_service.refresh_proxies().await.stable_group_selected_nodes();

                let prev = snapshot.read().clone();
                let changes = detect_changes(&prev, &current);

                if !changes.is_empty() {
                    for (group, old_node, new_node) in &changes {
                        log::info!(
                            "[ProxyGroupWatcher] stable group changed: {} {} -> {}",
                            group,
                            old_node,
                            new_node,
                        );
                    }

                    let debounce = Duration::from_secs(*debounce_secs.read());
                    let now = Instant::now();
                    let last = *last_backwrite.read();
                    let can_write = last.map_or(true, |t| now.duration_since(t) >= debounce);

                    if can_write {
                        trigger_backwrite().await;
                        *last_backwrite.write() = Some(now);
                        *pending_changes.write() = false;
                    } else {
                        *pending_changes.write() = true;
                        log::info!(
                            "[ProxyGroupWatcher] debounce active, delayed backwrite: {}s",
                            debounce.as_secs(),
                        );
                    }
                } else if *pending_changes.read() {
                    let debounce = Duration::from_secs(*debounce_secs.read());
                    let now = Instant::now();
                    let last = *last_backwrite.read();
                    let can_write = last.map_or(true, |t| now.duration_since(t) >= debounce);

                    if can_write {
                        log::info!("[ProxyGroupWatcher] debounce expired, running delayed backwrite");
                        trigger_backwrite().await;
                        *last_backwrite.write() = Some(now);
                        *pending_changes.write() = false;
                    }
                }

                *snapshot.write() = current;
            }

            *running.write() = false;
            log::info!("[ProxyGroupWatcher] stopped");
        });
    }

    pub fn stop(&self) {
        if !*self.running.read() {
            return;
        }
        self.cancel_notify.notify_one();
    }
}

fn detect_changes(prev: &HashMap<String, String>, current: &HashMap<String, String>) -> Vec<(String, String, String)> {
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

async fn trigger_backwrite() {
    if let Some(runtime_config) = crate::config::Config::runtime().await.latest_arc().config.clone() {
        let coordinator = crate::core::coordinator::get_coordinator();
        let session_affinity = crate::core::session_affinity::get_session_affinity_manager();
        let ip_reputation = crate::core::ip_reputation::get_ip_reputation_manager();
        if let Err(e) = crate::core::stable_egress::sync_runtime_stable_egress_selection(
            &coordinator,
            &session_affinity,
            &ip_reputation,
            &runtime_config,
        )
        .await
        {
            log::warn!("[ProxyGroupWatcher] backwrite failed: {}", e);
        }
    }
}

impl Default for ProxyGroupWatcher {
    fn default() -> Self {
        Self::new()
    }
}
