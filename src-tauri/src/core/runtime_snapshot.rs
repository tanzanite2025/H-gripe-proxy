use std::collections::HashMap;

use crate::{
    config::Config,
    core::{CoreManager, handle::Handle, manager::RunningMode},
};
use anyhow::Result;
use once_cell::sync::Lazy;
use serde_yaml_ng::Value;
use tauri_plugin_mihomo::models::{DelayHistory, DnsMetrics, Extra, Proxies, Proxy, ProxyType};

#[derive(Debug, Default)]
pub struct RuntimeSnapshot {
    pub core_running: bool,
    pub proxies: Option<Proxies>,
    pub dns_metrics: Option<DnsMetrics>,
    pub proxies_from_runtime_config: bool,
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

    pub async fn refresh_proxy_topology_from_runtime_config(&self) -> Result<RuntimeSnapshot> {
        let core_running = *CoreManager::global().get_running_mode() != RunningMode::NotRunning;
        let runtime = Config::runtime().await;
        let runtime = runtime.latest_arc();
        let config = runtime
            .config
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("runtime config is not available"))?;
        Ok(RuntimeSnapshot {
            core_running,
            proxies: Some(build_proxies_from_runtime_config(config)),
            dns_metrics: None,
            proxies_from_runtime_config: true,
        })
    }
}

pub fn build_proxies_from_runtime_config(config: &serde_yaml_ng::Mapping) -> Proxies {
    let mut proxies = HashMap::new();

    if let Some(items) = config.get("proxies").and_then(Value::as_sequence) {
        for item in items {
            if let Some(proxy) = proxy_from_config_item(item) {
                proxies.insert(proxy.name.clone(), proxy);
            }
        }
    }

    let mut group_names = Vec::new();
    if let Some(groups) = config.get("proxy-groups").and_then(Value::as_sequence) {
        for item in groups {
            if let Some(group) = proxy_group_from_config_item(item) {
                group_names.push(group.name.clone());
                proxies.insert(group.name.clone(), group);
            }
        }
    }

    for builtin in [
        builtin_proxy("DIRECT", ProxyType::Direct),
        builtin_proxy("REJECT", ProxyType::Reject),
        builtin_proxy("REJECT-DROP", ProxyType::RejectDrop),
    ] {
        proxies.entry(builtin.name.clone()).or_insert(builtin);
    }

    if !proxies.contains_key("GLOBAL") {
        let global_all = if group_names.is_empty() {
            proxies
                .keys()
                .filter(|name| !matches!(name.as_str(), "GLOBAL" | "DIRECT" | "REJECT" | "REJECT-DROP"))
                .cloned()
                .collect::<Vec<_>>()
        } else {
            group_names
        };
        proxies.insert(
            "GLOBAL".into(),
            proxy_group("GLOBAL", ProxyType::Selector, global_all, None, None, None, None),
        );
    }

    Proxies { proxies }
}

fn proxy_from_config_item(item: &Value) -> Option<Proxy> {
    let name = string_field(item, "name")?;
    let proxy_type = proxy_type_from_str(string_field(item, "type").as_deref());
    Some(Proxy {
        name,
        proxy_type,
        alive: true,
        udp: bool_field(item, "udp").unwrap_or(false),
        uot: bool_field(item, "uot").unwrap_or(false),
        xudp: bool_field(item, "xudp").unwrap_or(false),
        tfo: bool_field(item, "tfo").unwrap_or(false),
        mptcp: bool_field(item, "mptcp").unwrap_or(false),
        smux: bool_field(item, "smux").unwrap_or(false),
        interface: string_field(item, "interface-name").unwrap_or_default(),
        dialer_proxy: string_field(item, "dialer-proxy").unwrap_or_default(),
        routing_mark: i32_field(item, "routing-mark").unwrap_or_default(),
        provider_name: string_field(item, "provider"),
        all: None,
        expected_status: None,
        fixed: None,
        hidden: bool_field(item, "hidden"),
        icon: string_field(item, "icon"),
        now: None,
        test_url: None,
        id: None,
        history: Vec::new(),
        extra: HashMap::new(),
    })
}

fn proxy_group_from_config_item(item: &Value) -> Option<Proxy> {
    let name = string_field(item, "name")?;
    let all = item
        .get("proxies")
        .and_then(Value::as_sequence)
        .map(|items| {
            items
                .iter()
                .filter_map(Value::as_str)
                .map(std::string::String::from)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    Some(proxy_group(
        &name,
        proxy_type_from_str(string_field(item, "type").as_deref()),
        all,
        string_field(item, "test-url"),
        bool_field(item, "hidden"),
        string_field(item, "icon"),
        string_field(item, "fixed"),
    ))
}

fn proxy_group(
    name: &str,
    proxy_type: ProxyType,
    all: Vec<String>,
    test_url: Option<String>,
    hidden: Option<bool>,
    icon: Option<String>,
    fixed: Option<String>,
) -> Proxy {
    Proxy {
        name: name.into(),
        proxy_type,
        alive: true,
        udp: true,
        uot: false,
        xudp: false,
        tfo: false,
        mptcp: false,
        smux: false,
        interface: String::new(),
        dialer_proxy: String::new(),
        routing_mark: 0,
        provider_name: None,
        now: all.first().cloned(),
        all: Some(all),
        expected_status: None,
        fixed,
        hidden,
        icon,
        test_url,
        id: None,
        history: Vec::new(),
        extra: HashMap::<String, Extra>::new(),
    }
}

fn builtin_proxy(name: &str, proxy_type: ProxyType) -> Proxy {
    Proxy {
        name: name.into(),
        proxy_type,
        alive: true,
        udp: true,
        uot: false,
        xudp: false,
        tfo: false,
        mptcp: false,
        smux: false,
        interface: String::new(),
        dialer_proxy: String::new(),
        routing_mark: 0,
        provider_name: None,
        all: None,
        expected_status: None,
        fixed: None,
        hidden: None,
        icon: None,
        now: None,
        test_url: None,
        id: None,
        history: Vec::<DelayHistory>::new(),
        extra: HashMap::new(),
    }
}

fn proxy_type_from_str(value: Option<&str>) -> ProxyType {
    match value.unwrap_or_default().to_ascii_lowercase().as_str() {
        "direct" => ProxyType::Direct,
        "reject" => ProxyType::Reject,
        "reject-drop" => ProxyType::RejectDrop,
        "compatible" => ProxyType::Compatible,
        "pass" => ProxyType::Pass,
        "dns" => ProxyType::Dns,
        "ss" | "shadowsocks" => ProxyType::Shadowsocks,
        "ssr" | "shadowsocksr" => ProxyType::ShadowsocksR,
        "snell" => ProxyType::Snell,
        "socks" | "socks5" => ProxyType::Socks5,
        "http" => ProxyType::Http,
        "vmess" => ProxyType::Vmess,
        "vless" => ProxyType::Vless,
        "trojan" => ProxyType::Trojan,
        "hysteria" => ProxyType::Hysteria,
        "hysteria2" | "hy2" => ProxyType::Hysteria2,
        "wireguard" | "wg" => ProxyType::WireGuard,
        "tuic" => ProxyType::Tuic,
        "ssh" => ProxyType::Ssh,
        "mieru" => ProxyType::Mieru,
        "masque" => ProxyType::Masque,
        "anytls" => ProxyType::AnyTLS,
        "relay" => ProxyType::Relay,
        "select" | "selector" => ProxyType::Selector,
        "fallback" => ProxyType::Fallback,
        "url-test" => ProxyType::URLTest,
        "load-balance" | "loadbalance" => ProxyType::LoadBalance,
        other if other.is_empty() => ProxyType::Unknown("unknown".into()),
        other => ProxyType::Unknown(other.into()),
    }
}

fn string_field(item: &Value, field: &str) -> Option<String> {
    item.get(field)
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .map(std::string::String::from)
}

fn bool_field(item: &Value, field: &str) -> Option<bool> {
    item.get(field).and_then(Value::as_bool)
}

fn i32_field(item: &Value, field: &str) -> Option<i32> {
    item.get(field)
        .and_then(Value::as_i64)
        .and_then(|value| i32::try_from(value).ok())
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
            dns_metrics: None,
            proxies_from_runtime_config: false,
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
            dns_metrics: None,
            proxies_from_runtime_config: false,
        };

        assert!(snapshot.stable_group_selected_nodes().is_empty());
    }

    #[test]
    fn global_snapshot_service_is_available() {
        let service = RuntimeSnapshotService::global();

        assert!(std::ptr::eq(service, RuntimeSnapshotService::global()));
    }

    #[test]
    fn runtime_config_topology_builds_proxies_groups_and_global() {
        let config: serde_yaml_ng::Mapping = serde_yaml_ng::from_str(
            r#"
proxies:
  - name: node-a
    type: ss
    udp: true
    dialer-proxy: relay-a
  - name: node-b
    type: vmess
proxy-groups:
  - name: Auto
    type: url-test
    proxies:
      - node-a
      - node-b
    test-url: https://example.com/generate_204
"#,
        )
        .unwrap();

        let topology = build_proxies_from_runtime_config(&config);

        let node_a = topology.proxies.get("node-a").unwrap();
        assert_eq!(node_a.proxy_type, ProxyType::Shadowsocks);
        assert_eq!(node_a.dialer_proxy, "relay-a");
        let auto = topology.proxies.get("Auto").unwrap();
        assert_eq!(auto.proxy_type, ProxyType::URLTest);
        assert_eq!(auto.now.as_deref(), Some("node-a"));
        assert_eq!(
            auto.all.as_ref().unwrap(),
            &vec!["node-a".to_string(), "node-b".to_string()]
        );
        let global = topology.proxies.get("GLOBAL").unwrap();
        assert_eq!(global.proxy_type, ProxyType::Selector);
        assert_eq!(global.all.as_ref().unwrap(), &vec!["Auto".to_string()]);
        assert!(topology.proxies.contains_key("DIRECT"));
        assert!(topology.proxies.contains_key("REJECT"));
    }
}
