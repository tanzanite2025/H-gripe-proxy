use std::collections::HashMap;

use crate::core::{CoreManager, handle::Handle, manager::RunningMode};
use anyhow::Result;
use once_cell::sync::Lazy;
use tauri_plugin_mihomo::models::{Connections, DnsMetrics, Proxies};

#[derive(Debug, Default)]
pub struct RuntimeSnapshot {
    pub core_running: bool,
    pub proxies: Option<Proxies>,
    pub connections: Option<Connections>,
    pub dns_metrics: Option<DnsMetrics>,
}

impl RuntimeSnapshot {
    pub fn stable_group_selected_nodes(&self) -> HashMap<String, String> {
        self.proxies
            .as_ref()
            .map(|proxies| {
                proxies
                    .proxies
                    .iter()
                    .filter_map(|(group_name, group_data)| {
                        if !group_name.starts_with("VERGE-STABLE-") {
                            return None;
                        }

                        group_data
                            .now
                            .as_ref()
                            .map(|value| value.trim())
                            .filter(|value| !value.is_empty())
                            .map(|value| (group_name.clone(), value.to_string()))
                    })
                    .collect()
            })
            .unwrap_or_default()
    }
}

static RUNTIME_SNAPSHOT_SERVICE: Lazy<RuntimeSnapshotService> = Lazy::new(RuntimeSnapshotService::new);

#[derive(Debug, Default)]
pub struct RuntimeSnapshotService;

impl RuntimeSnapshotService {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn global() -> &'static Self {
        &RUNTIME_SNAPSHOT_SERVICE
    }

    pub async fn refresh_dns_metrics(&self) -> RuntimeSnapshot {
        let core_running = *CoreManager::global().get_running_mode() != RunningMode::NotRunning;
        let mut snapshot = RuntimeSnapshot {
            core_running,
            ..RuntimeSnapshot::default()
        };

        if core_running {
            let mihomo = Handle::mihomo().await;
            snapshot.dns_metrics = mihomo.get_dns_metrics().await.ok();
        }

        snapshot
    }

    pub async fn refresh_proxies(&self) -> RuntimeSnapshot {
        let core_running = *CoreManager::global().get_running_mode() != RunningMode::NotRunning;
        let mut snapshot = RuntimeSnapshot {
            core_running,
            ..RuntimeSnapshot::default()
        };

        if core_running {
            let mihomo = Handle::mihomo().await;
            snapshot.proxies = mihomo.get_proxies().await.ok();
        }

        snapshot
    }

    pub async fn refresh_proxies_result(&self) -> Result<RuntimeSnapshot> {
        let core_running = *CoreManager::global().get_running_mode() != RunningMode::NotRunning;
        let mut snapshot = RuntimeSnapshot {
            core_running,
            ..RuntimeSnapshot::default()
        };

        if core_running {
            let mihomo = Handle::mihomo().await;
            snapshot.proxies = Some(mihomo.get_proxies().await?);
        }

        Ok(snapshot)
    }

    pub async fn refresh_connections_result(&self) -> Result<RuntimeSnapshot> {
        let core_running = *CoreManager::global().get_running_mode() != RunningMode::NotRunning;
        let mut snapshot = RuntimeSnapshot {
            core_running,
            ..RuntimeSnapshot::default()
        };

        if core_running {
            let mihomo = Handle::mihomo().await;
            snapshot.connections = Some(mihomo.get_connections().await?);
        }

        Ok(snapshot)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use tauri_plugin_mihomo::models::{Proxies, Proxy, ProxyType};

    fn proxy_group(name: &str, now: &str) -> Proxy {
        Proxy {
            all: Some(vec!["node-a".into(), "node-b".into()]),
            expected_status: None,
            fixed: None,
            hidden: None,
            icon: None,
            now: Some(now.into()),
            test_url: None,
            id: None,
            alive: true,
            history: Vec::new(),
            extra: HashMap::new(),
            name: name.into(),
            udp: true,
            uot: false,
            proxy_type: ProxyType::Selector,
            xudp: false,
            tfo: false,
            mptcp: false,
            smux: false,
            interface: String::new(),
            dialer_proxy: String::new(),
            routing_mark: 0,
            provider_name: None,
        }
    }

    #[test]
    fn snapshot_collects_stable_group_selections() {
        let snapshot = RuntimeSnapshot {
            core_running: true,
            proxies: Some(Proxies {
                proxies: HashMap::from([
                    (
                        "VERGE-STABLE-example".into(),
                        proxy_group("VERGE-STABLE-example", "node-a"),
                    ),
                    ("GLOBAL".into(), proxy_group("GLOBAL", "node-b")),
                ]),
            }),
            connections: None,
            dns_metrics: None,
        };

        let selections = snapshot.stable_group_selected_nodes();

        assert_eq!(
            selections.get("VERGE-STABLE-example").map(std::string::String::as_str),
            Some("node-a")
        );
        assert_eq!(selections.get("GLOBAL"), None);
    }

    #[test]
    fn snapshot_without_proxies_has_no_stable_group_selections() {
        let snapshot = RuntimeSnapshot {
            core_running: false,
            proxies: None,
            connections: None,
            dns_metrics: None,
        };

        assert!(snapshot.stable_group_selected_nodes().is_empty());
    }

    #[test]
    fn global_snapshot_service_is_available() {
        let service = RuntimeSnapshotService::global();

        assert!(std::ptr::eq(service, RuntimeSnapshotService::global()));
    }
}
