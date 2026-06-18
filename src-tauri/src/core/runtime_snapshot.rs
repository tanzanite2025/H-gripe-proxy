use std::{collections::HashMap, fs, path::PathBuf, sync::RwLock};

use crate::{
    config::Config,
    core::{CoreManager, handle::Handle, manager::RunningMode},
};
use anyhow::Result;
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use serde_yaml_ng::Value;
use tauri_plugin_mihomo::models::{
    DelayHistory, DnsMetrics, Extra, ProviderType, Proxies, Proxy, ProxyProvider, ProxyProviders, ProxyType, Rule,
    RuleBehavior, RuleFormat, RuleProvider, RuleProviders, RuleType, Rules, SubScriptionInfo, VehicleType,
};

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
static RUNTIME_PROXY_SELECTION_STATE: Lazy<RwLock<HashMap<String, String>>> = Lazy::new(|| RwLock::new(HashMap::new()));
const RUNTIME_PROXY_SELECTIONS_FILE: &str = "proxy-selections.yaml";

#[derive(Debug, Default, Deserialize, Serialize)]
pub struct RuntimeProxySelectionState {
    pub groups: HashMap<String, String>,
}

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

    apply_proxy_selection_state(&mut proxies);

    Proxies { proxies }
}

pub fn runtime_proxy_selection_state() -> HashMap<String, String> {
    RUNTIME_PROXY_SELECTION_STATE
        .read()
        .map(|state| state.clone())
        .unwrap_or_default()
}

pub fn record_runtime_proxy_selection(group_name: &str, proxy_name: &str) {
    if let Ok(mut state) = RUNTIME_PROXY_SELECTION_STATE.write() {
        state.insert(group_name.to_string(), proxy_name.to_string());
    }
}

pub fn record_and_persist_runtime_proxy_selection(group_name: &str, proxy_name: &str) {
    record_runtime_proxy_selection(group_name, proxy_name);
    if let Err(error) = persist_runtime_proxy_selection_state() {
        log::warn!("failed to persist runtime proxy selection state: {error}");
    }
}

pub fn load_runtime_proxy_selection_state_from_disk() -> Result<()> {
    let path = runtime_proxy_selection_state_path()?;
    if !path.exists() {
        return Ok(());
    }
    let content = match fs::read_to_string(path) {
        Ok(content) => content,
        Err(error) => {
            log::warn!("failed to read runtime proxy selection state: {error}");
            return Ok(());
        }
    };
    let document = match serde_yaml_ng::from_str::<RuntimeProxySelectionState>(&content) {
        Ok(document) => document,
        Err(error) => {
            log::warn!("failed to parse runtime proxy selection state: {error}");
            return Ok(());
        }
    };
    if let Ok(mut state) = RUNTIME_PROXY_SELECTION_STATE.write() {
        *state = document.groups;
    }
    Ok(())
}

fn persist_runtime_proxy_selection_state() -> Result<()> {
    let path = runtime_proxy_selection_state_path()?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let document = RuntimeProxySelectionState {
        groups: runtime_proxy_selection_state(),
    };
    fs::write(path, serde_yaml_ng::to_string(&document)?)?;
    Ok(())
}

fn runtime_proxy_selection_state_path() -> Result<PathBuf> {
    Ok(crate::utils::dirs::app_runtime_dir()?.join(RUNTIME_PROXY_SELECTIONS_FILE))
}

fn apply_proxy_selection_state(proxies: &mut HashMap<String, Proxy>) {
    let state = runtime_proxy_selection_state();
    for (group_name, proxy_name) in state {
        let Some(group) = proxies.get_mut(&group_name) else {
            continue;
        };
        let Some(all) = group.all.as_ref() else {
            continue;
        };
        if all.iter().any(|candidate| candidate == &proxy_name) {
            group.now = Some(proxy_name);
        }
    }
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

fn i64_field(item: &Value, field: &str) -> Option<i64> {
    item.get(field).and_then(Value::as_i64)
}

/// Build proxy providers from runtime config YAML and provider files on disk.
pub fn build_proxy_providers_from_runtime_config(config: &serde_yaml_ng::Mapping) -> ProxyProviders {
    let mut providers = HashMap::new();

    let Some(provider_map) = config.get("proxy-providers").and_then(Value::as_mapping) else {
        return ProxyProviders { providers };
    };

    let app_home = crate::utils::dirs::app_home_dir().unwrap_or_default();

    for (key, value) in provider_map {
        let Some(name) = key.as_str() else { continue };
        let Some(provider) = build_single_provider(name, value, &app_home) else {
            continue;
        };
        providers.insert(name.to_string(), provider);
    }

    ProxyProviders { providers }
}

fn build_single_provider(name: &str, value: &Value, app_home: &std::path::Path) -> Option<ProxyProvider> {
    let vehicle_type = match string_field(value, "type").as_deref() {
        Some("http") => VehicleType::HTTP,
        Some("file") => VehicleType::File,
        Some("inline") => VehicleType::Inline,
        _ => VehicleType::Compatible,
    };

    let test_url = value
        .get("health-check")
        .and_then(|hc| hc.get("url"))
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();

    let expected_status = value
        .get("health-check")
        .and_then(|hc| hc.get("expected-status"))
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();

    let proxies = load_provider_proxies(value, app_home, name);

    let subscription_info = load_subscription_info(value);

    Some(ProxyProvider {
        name: name.to_string(),
        provider_type: ProviderType::Proxy,
        vehicle_type,
        proxies,
        test_url,
        expected_status,
        updated_at: None,
        subscription_info,
    })
}

/// Load proxy nodes from provider file on disk.
fn load_provider_proxies(provider_config: &Value, app_home: &std::path::Path, provider_name: &str) -> Vec<Proxy> {
    // Inline providers have proxies embedded in the config
    if let Some(payload) = provider_config.get("payload").and_then(Value::as_sequence) {
        return payload
            .iter()
            .filter_map(|item| {
                let mut proxy = proxy_from_config_item(item)?;
                proxy.provider_name = Some(provider_name.to_string());
                Some(proxy)
            })
            .collect();
    }

    // File/HTTP providers store proxies in a file on disk
    let path_str = match string_field(provider_config, "path") {
        Some(p) => p,
        None => return Vec::new(),
    };

    let file_path = if std::path::Path::new(&path_str).is_absolute() {
        std::path::PathBuf::from(&path_str)
    } else {
        app_home.join(&path_str)
    };

    let content = match std::fs::read_to_string(&file_path) {
        Ok(c) => c,
        Err(_) => return Vec::new(),
    };

    parse_provider_file_content(&content, provider_name)
}

/// Parse provider file content (supports both proxies key and bare sequence).
fn parse_provider_file_content(content: &str, provider_name: &str) -> Vec<Proxy> {
    let value: Value = match serde_yaml_ng::from_str(content) {
        Ok(v) => v,
        Err(_) => return Vec::new(),
    };

    let sequence = if let Some(seq) = value.get("proxies").and_then(Value::as_sequence) {
        seq.clone()
    } else if let Some(seq) = value.as_sequence() {
        seq.clone()
    } else {
        return Vec::new();
    };

    sequence
        .iter()
        .filter_map(|item| {
            let mut proxy = proxy_from_config_item(item)?;
            proxy.provider_name = Some(provider_name.to_string());
            Some(proxy)
        })
        .collect()
}

/// Try to load subscription info from the provider config.
fn load_subscription_info(provider_config: &Value) -> Option<SubScriptionInfo> {
    let sub_info = provider_config.get("subscription-info")?;
    Some(SubScriptionInfo {
        upload: i64_field(sub_info, "Upload")
            .or_else(|| i64_field(sub_info, "upload"))
            .unwrap_or(0),
        download: i64_field(sub_info, "Download")
            .or_else(|| i64_field(sub_info, "download"))
            .unwrap_or(0),
        total: i64_field(sub_info, "Total")
            .or_else(|| i64_field(sub_info, "total"))
            .unwrap_or(0),
        expire: i64_field(sub_info, "Expire")
            .or_else(|| i64_field(sub_info, "expire"))
            .unwrap_or(0),
    })
}

pub fn build_rules_from_runtime_config(config: &serde_yaml_ng::Mapping) -> Rules {
    let mut rules = Vec::new();
    let mut rule_set_targets = HashMap::new();

    if let Some(items) = config.get("rules").and_then(Value::as_sequence) {
        for item in items {
            let Some(rule) = rule_from_value(item, rules.len() as i32, "profile", None) else {
                continue;
            };
            if matches!(rule.rule_type, RuleType::RuleSet) && !rule.payload.is_empty() && !rule.proxy.is_empty() {
                rule_set_targets.insert(rule.payload.clone(), rule.proxy.clone());
            }
            rules.push(rule);
        }
    }

    append_rule_provider_rules(config, &mut rules, &rule_set_targets);

    let total = i32::try_from(rules.len()).unwrap_or(i32::MAX);
    Rules {
        rules,
        total: Some(total),
        page: Some(1),
        page_size: Some(total),
    }
}

pub fn build_rule_providers_from_runtime_config(config: &serde_yaml_ng::Mapping) -> RuleProviders {
    let mut providers = HashMap::new();

    let Some(provider_map) = config.get("rule-providers").and_then(Value::as_mapping) else {
        return RuleProviders { providers };
    };

    let app_home = crate::utils::dirs::app_home_dir().unwrap_or_default();

    for (key, value) in provider_map {
        let Some(name) = key.as_str() else { continue };
        let provider = build_single_rule_provider(name, value, &app_home);
        providers.insert(name.to_string(), provider);
    }

    RuleProviders { providers }
}

fn append_rule_provider_rules(
    config: &serde_yaml_ng::Mapping,
    rules: &mut Vec<Rule>,
    targets: &HashMap<String, String>,
) {
    let Some(provider_map) = config.get("rule-providers").and_then(Value::as_mapping) else {
        return;
    };

    let app_home = crate::utils::dirs::app_home_dir().unwrap_or_default();

    for (key, value) in provider_map {
        let Some(name) = key.as_str() else { continue };
        let behavior = rule_behavior_from_str(string_field(value, "behavior").as_deref());
        let target = targets.get(name).map(std::string::String::as_str);
        let source = format!("provider:{name}");

        for payload in load_rule_provider_payloads(value, &app_home) {
            let index = i32::try_from(rules.len()).unwrap_or(i32::MAX);
            let rule = match behavior {
                RuleBehavior::Classical => rule_from_line(&payload, index, &source, target),
                RuleBehavior::Domain => Some(rule_from_provider_payload(
                    index,
                    RuleType::Domain,
                    payload,
                    target.unwrap_or_default().to_string(),
                    source.clone(),
                )),
                RuleBehavior::IpCidr => Some(rule_from_provider_payload(
                    index,
                    RuleType::IPCIDR,
                    payload,
                    target.unwrap_or_default().to_string(),
                    source.clone(),
                )),
            };

            if let Some(rule) = rule {
                rules.push(rule);
            }
        }
    }
}

fn build_single_rule_provider(name: &str, value: &Value, app_home: &std::path::Path) -> RuleProvider {
    let payloads = load_rule_provider_payloads(value, app_home);
    RuleProvider {
        behavior: rule_behavior_from_str(string_field(value, "behavior").as_deref()),
        format: rule_format_from_str(string_field(value, "format").as_deref()),
        name: name.to_string(),
        rule_count: u32::try_from(payloads.len()).unwrap_or(u32::MAX),
        provider_type: ProviderType::Rule,
        updated_at: provider_file_updated_at(value, app_home),
        vehicle_type: vehicle_type_from_str(string_field(value, "type").as_deref()),
    }
}

fn rule_from_value(item: &Value, index: i32, source: &str, fallback_proxy: Option<&str>) -> Option<Rule> {
    let line = item.as_str()?;
    rule_from_line(line, index, source, fallback_proxy)
}

fn rule_from_line(line: &str, index: i32, source: &str, fallback_proxy: Option<&str>) -> Option<Rule> {
    let fields = split_rule_fields(line);
    let rule_type_field = fields.first()?.trim();
    let rule_type = rule_type_from_str(Some(rule_type_field));
    let payload = if matches!(rule_type, RuleType::Match) {
        String::new()
    } else {
        fields.get(1).cloned().unwrap_or_default()
    };
    let proxy = if matches!(rule_type, RuleType::Match) {
        fields
            .get(1)
            .cloned()
            .or_else(|| fallback_proxy.map(std::string::String::from))
            .unwrap_or_default()
    } else {
        fields
            .get(2)
            .cloned()
            .or_else(|| fallback_proxy.map(std::string::String::from))
            .unwrap_or_default()
    };

    Some(Rule {
        index,
        rule_type,
        payload,
        proxy,
        size: i32::try_from(line.len()).unwrap_or(i32::MAX),
        source: source.to_string(),
        extra: None,
    })
}

fn rule_from_provider_payload(index: i32, rule_type: RuleType, payload: String, proxy: String, source: String) -> Rule {
    Rule {
        index,
        rule_type,
        size: i32::try_from(payload.len()).unwrap_or(i32::MAX),
        payload,
        proxy,
        source,
        extra: None,
    }
}

fn load_rule_provider_payloads(provider_config: &Value, app_home: &std::path::Path) -> Vec<String> {
    if let Some(payload) = provider_config.get("payload").and_then(Value::as_sequence) {
        return collect_payload_entries(payload);
    }

    let Some(file_path) = provider_file_path(provider_config, app_home) else {
        return Vec::new();
    };
    let content = match std::fs::read_to_string(&file_path) {
        Ok(content) => content,
        Err(_) => return Vec::new(),
    };

    parse_rule_provider_file_content(&content)
}

fn parse_rule_provider_file_content(content: &str) -> Vec<String> {
    let value: Result<Value, _> = serde_yaml_ng::from_str(content);
    let Ok(value) = value else {
        return content_lines(content);
    };

    if let Some(payload) = value.get("payload").and_then(Value::as_sequence) {
        return collect_payload_entries(payload);
    }
    if let Some(rules) = value.get("rules").and_then(Value::as_sequence) {
        return collect_payload_entries(rules);
    }
    if let Some(sequence) = value.as_sequence() {
        return collect_payload_entries(sequence);
    }
    if let Some(text) = value.as_str() {
        return content_lines(text);
    }

    Vec::new()
}

fn content_lines(content: &str) -> Vec<String> {
    content
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .map(std::string::String::from)
        .collect()
}

fn collect_payload_entries(items: &[Value]) -> Vec<String> {
    items
        .iter()
        .filter_map(|item| {
            item.as_str()
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(std::string::String::from)
        })
        .collect()
}

fn provider_file_path(provider_config: &Value, app_home: &std::path::Path) -> Option<std::path::PathBuf> {
    let path_str = string_field(provider_config, "path")?;
    if std::path::Path::new(&path_str).is_absolute() {
        Some(std::path::PathBuf::from(&path_str))
    } else {
        Some(app_home.join(&path_str))
    }
}

fn provider_file_updated_at(provider_config: &Value, app_home: &std::path::Path) -> String {
    let Some(file_path) = provider_file_path(provider_config, app_home) else {
        return String::new();
    };
    let Ok(metadata) = std::fs::metadata(&file_path) else {
        return String::new();
    };
    let Ok(modified) = metadata.modified() else {
        return String::new();
    };
    chrono::DateTime::<chrono::Utc>::from(modified).to_rfc3339()
}

fn split_rule_fields(line: &str) -> Vec<String> {
    let mut fields = Vec::new();
    let mut current = String::new();
    let mut depth = 0_i32;
    let mut in_single_quote = false;
    let mut in_double_quote = false;
    let mut escaped = false;

    for ch in line.chars() {
        if escaped {
            current.push(ch);
            escaped = false;
            continue;
        }

        if ch == '\\' {
            current.push(ch);
            escaped = true;
            continue;
        }

        match ch {
            '\'' if !in_double_quote => {
                in_single_quote = !in_single_quote;
                current.push(ch);
            }
            '"' if !in_single_quote => {
                in_double_quote = !in_double_quote;
                current.push(ch);
            }
            '(' | '[' | '{' if !in_single_quote && !in_double_quote => {
                depth += 1;
                current.push(ch);
            }
            ')' | ']' | '}' if !in_single_quote && !in_double_quote => {
                depth = (depth - 1).max(0);
                current.push(ch);
            }
            ',' if depth == 0 && !in_single_quote && !in_double_quote => {
                fields.push(current.trim().to_string());
                current.clear();
            }
            _ => current.push(ch),
        }
    }

    fields.push(current.trim().to_string());
    fields
}

fn rule_type_from_str(value: Option<&str>) -> RuleType {
    let Some(raw) = value else {
        return RuleType::Unknown("unknown".into());
    };
    match raw.replace(['-', '_'], "").to_ascii_uppercase().as_str() {
        "DOMAIN" => RuleType::Domain,
        "DOMAINSUFFIX" => RuleType::DomainSuffix,
        "DOMAINKEYWORD" => RuleType::DomainKeyword,
        "DOMAINREGEX" => RuleType::DomainRegex,
        "GEOSITE" => RuleType::GeoSite,
        "GEOIP" => RuleType::GeoIP,
        "SRCGEOIP" => RuleType::SrcGeoIP,
        "IPASN" => RuleType::IPASN,
        "SRCIPASN" => RuleType::SrcIPASN,
        "IPCIDR" => RuleType::IPCIDR,
        "SRCIPCIDR" => RuleType::SrcIPCIDR,
        "IPSUFFIX" => RuleType::IPSuffix,
        "SRCIPSUFFIX" => RuleType::SrcIPSuffix,
        "SRCPORT" => RuleType::SrcPort,
        "DSTPORT" => RuleType::DstPort,
        // spellchecker:disable-next-line
        "INPORT" => RuleType::InPort,
        "INUSER" => RuleType::InUser,
        "INNAME" => RuleType::InName,
        "INTYPE" => RuleType::InType,
        "PROCESSNAME" => RuleType::ProcessName,
        "PROCESSPATH" => RuleType::ProcessPath,
        "PROCESSNAMEREGEX" => RuleType::ProcessNameRegex,
        "PROCESSPATHREGEX" => RuleType::ProcessPathRegex,
        "MATCH" => RuleType::Match,
        "RULESET" => RuleType::RuleSet,
        "NETWORK" => RuleType::Network,
        "DSCP" => RuleType::DSCP,
        "UID" => RuleType::Uid,
        "SUBRULES" => RuleType::SubRules,
        "AND" => RuleType::AND,
        "OR" => RuleType::OR,
        "NOT" => RuleType::NOT,
        _ => RuleType::Unknown(raw.to_string()),
    }
}

fn rule_behavior_from_str(value: Option<&str>) -> RuleBehavior {
    match value
        .unwrap_or_default()
        .replace(['-', '_'], "")
        .to_ascii_lowercase()
        .as_str()
    {
        "domain" => RuleBehavior::Domain,
        "ipcidr" => RuleBehavior::IpCidr,
        _ => RuleBehavior::Classical,
    }
}

fn rule_format_from_str(value: Option<&str>) -> RuleFormat {
    match value.unwrap_or_default().to_ascii_lowercase().as_str() {
        "text" => RuleFormat::Text,
        "mrs" => RuleFormat::Mrs,
        _ => RuleFormat::Yaml,
    }
}

fn vehicle_type_from_str(value: Option<&str>) -> VehicleType {
    match value.unwrap_or_default().to_ascii_lowercase().as_str() {
        "http" => VehicleType::HTTP,
        "file" => VehicleType::File,
        "inline" => VehicleType::Inline,
        _ => VehicleType::Compatible,
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

    #[test]
    fn runtime_proxy_topology_applies_selection_state_cache() {
        let config: serde_yaml_ng::Mapping = serde_yaml_ng::from_str(
            r#"
proxies:
  - name: cache-node-a
    type: ss
  - name: cache-node-b
    type: ss
proxy-groups:
  - name: CacheSelector
    type: select
    proxies:
      - cache-node-a
      - cache-node-b
"#,
        )
        .unwrap();

        record_runtime_proxy_selection("CacheSelector", "cache-node-b");

        let topology = build_proxies_from_runtime_config(&config);

        assert_eq!(
            topology
                .proxies
                .get("CacheSelector")
                .and_then(|group| group.now.as_deref()),
            Some("cache-node-b")
        );
    }

    #[test]
    fn builds_rules_from_runtime_config_and_inline_rule_provider() {
        let runtime_yaml = r#"
rules:
  - DOMAIN-SUFFIX,example.com,DIRECT
  - RULE-SET,ads,REJECT
  - MATCH,DIRECT
rule-providers:
  ads:
    type: http
    behavior: domain
    format: yaml
    payload:
      - ads.example
"#;
        let value = serde_yaml_ng::from_str::<Value>(runtime_yaml).unwrap();
        let config = value.as_mapping().unwrap();

        let rules = build_rules_from_runtime_config(config);

        assert_eq!(rules.rules.len(), 4);
        assert_eq!(rules.rules[0].rule_type, RuleType::DomainSuffix);
        assert_eq!(rules.rules[0].payload, "example.com");
        assert_eq!(rules.rules[0].proxy, "DIRECT");
        assert_eq!(rules.rules[2].rule_type, RuleType::Match);
        assert_eq!(rules.rules[2].payload, "");
        assert_eq!(rules.rules[2].proxy, "DIRECT");
        assert_eq!(rules.rules[3].source, "provider:ads");
        assert_eq!(rules.rules[3].rule_type, RuleType::Domain);
        assert_eq!(rules.rules[3].payload, "ads.example");
        assert_eq!(rules.rules[3].proxy, "REJECT");
    }

    #[test]
    fn builds_rule_providers_from_runtime_config() {
        let runtime_yaml = r#"
rule-providers:
  cn:
    type: file
    behavior: ipcidr
    format: text
    payload:
      - 10.0.0.0/8
      - 192.168.0.0/16
"#;
        let value = serde_yaml_ng::from_str::<Value>(runtime_yaml).unwrap();
        let config = value.as_mapping().unwrap();

        let providers = build_rule_providers_from_runtime_config(config);
        let provider = providers.providers.get("cn").unwrap();

        assert_eq!(provider.name, "cn");
        assert_eq!(provider.rule_count, 2);
        assert_eq!(provider.behavior, RuleBehavior::IpCidr);
        assert_eq!(provider.format, RuleFormat::Text);
        assert_eq!(provider.provider_type, ProviderType::Rule);
        assert_eq!(provider.vehicle_type, VehicleType::File);
    }
}
