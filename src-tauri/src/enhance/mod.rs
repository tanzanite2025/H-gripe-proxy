mod chain;
pub mod field;
mod merge;
mod obfuscation;
mod script;
pub mod seq;
mod sniffer;
mod tls_fingerprint;
mod traffic_obfuscation;
mod tun;

mod blackhole_breaker;
mod connection_stability;
mod proxy_cleanup;
mod stable_egress;
mod timezone_spoof;

use self::{
    blackhole_breaker::apply_blackhole_breaker_config,
    chain::{AsyncChainItemFrom as _, ChainItem, ChainType},
    connection_stability::{apply_connection_stability, apply_multiplex},
    field::{use_keys, use_lowercase, use_sort},
    merge::use_merge,
    obfuscation::apply_obfuscation_config,
    proxy_cleanup::cleanup_proxy_groups,
    script::use_script,
    seq::{SeqMap, use_seq},
    sniffer::apply_sniffer_config,
    timezone_spoof::apply_timezone_spoof_config,
    tls_fingerprint::apply_tls_fingerprint_config,
    traffic_obfuscation::apply_traffic_obfuscation_config,
    tun::use_tun,
};

#[allow(unused_imports)]
pub(crate) use self::stable_egress::{apply_stable_egress_policy, apply_stable_egress_policy_with_advanced};
use crate::utils::dirs;
use crate::{
    config::{AUXILIARY_RULES_NAME, Config, IVerge},
    constants,
    utils::tmpl,
};
use anyhow::{Context as _, Result};
use clash_verge_logging::{Type, logging};
use serde_yaml_ng::{Mapping, Sequence, Value};
use smartstring::alias::String;
use std::collections::{HashMap, HashSet};
use tokio::fs;

type ResultLog = Vec<(String, String)>;

const SOFTWARE_STRATEGY_POOL_NAME: &str = "策略池";
const LEGACY_BLOCKED_GROUP_NAMES: [&str; 2] = ["自动选择", "故障转移"];

#[derive(Debug)]
struct ConfigValues {
    clash_config: Mapping,
    enable_tun: bool,
    socks_enabled: bool,
    http_enabled: bool,
    enable_dns_settings: bool,
    #[cfg(not(target_os = "windows"))]
    redir_enabled: bool,
    #[cfg(target_os = "linux")]
    tproxy_enabled: bool,
}

#[derive(Debug)]
struct ProfileItems {
    config: Mapping,
    merge_item: ChainItem,
    script_item: ChainItem,
    rules_item: ChainItem,
    proxies_item: ChainItem,
    groups_item: ChainItem,
    global_merge: ChainItem,
    global_script: ChainItem,
    profile_name: String,
}

fn normalize_group_type(value: &str) -> std::string::String {
    value.trim().to_ascii_lowercase()
}

fn is_manual_group_mapping(group: &Mapping) -> bool {
    group
        .get("type")
        .and_then(Value::as_str)
        .map(normalize_group_type)
        .is_some_and(|value| value == "select" || value == "selector")
}

fn is_strategy_group_mapping(group: &Mapping) -> bool {
    group
        .get("type")
        .and_then(Value::as_str)
        .map(normalize_group_type)
        .is_some_and(|value| {
            value == "url-test" || value == "urltest" || value == "load-balance" || value == "loadbalance"
        })
}

fn is_auxiliary_group_mapping(group: &Mapping) -> bool {
    group
        .get("type")
        .and_then(Value::as_str)
        .map(normalize_group_type)
        .is_some_and(|value| value == "fallback")
}

fn is_software_blocked_group_name(name: &str) -> bool {
    LEGACY_BLOCKED_GROUP_NAMES.contains(&name)
}

fn collect_proxy_names(config: &Mapping) -> Vec<String> {
    let mut names = Vec::new();
    let mut seen = HashSet::new();

    if let Some(Value::Sequence(proxies)) = config.get("proxies") {
        for proxy in proxies {
            let Some(name) = (match proxy {
                Value::Mapping(map) => map.get("name").and_then(Value::as_str),
                Value::String(name) => Some(name.as_str()),
                _ => None,
            }) else {
                continue;
            };

            let trimmed = name.trim();
            if trimmed.is_empty() || !seen.insert(trimmed.to_owned()) {
                continue;
            }

            names.push(trimmed.to_owned().into());
        }
    }

    names
}

fn preferred_root_group_name(config: &Mapping, profile_name: &String) -> String {
    if let Some(Value::Sequence(groups)) = config.get("proxy-groups") {
        for group in groups {
            let Some(map) = group.as_mapping() else {
                continue;
            };

            if !is_manual_group_mapping(map) {
                continue;
            }

            if let Some(name) = map.get("name").and_then(Value::as_str) {
                let trimmed = name.trim();
                if !trimmed.is_empty() {
                    return trimmed.to_owned().into();
                }
            }
        }
    }

    let trimmed = profile_name.trim();
    if trimmed.is_empty() {
        "订阅组".into()
    } else {
        trimmed.to_owned().into()
    }
}

fn remove_proxy_groups(mut config: Mapping) -> Mapping {
    config.remove("proxy-groups");
    config
}

fn build_group_value(
    name: &str,
    group_type: &str,
    proxies: &[String],
    url: Option<&str>,
    interval: Option<i64>,
) -> Value {
    let mut group = Mapping::new();
    let proxy_values = proxies
        .iter()
        .map(|proxy| Value::String(proxy.to_string()))
        .collect::<Sequence>();

    group.insert("name".into(), name.into());
    group.insert("type".into(), group_type.into());
    group.insert("proxies".into(), Value::Sequence(proxy_values));

    if let Some(url) = url {
        group.insert("url".into(), url.into());
    }

    if let Some(interval) = interval {
        group.insert("interval".into(), interval.into());
    }

    Value::Mapping(group)
}

fn normalize_software_owned_proxy_groups(mut config: Mapping, root_group_name: &String) -> Mapping {
    let proxy_names = collect_proxy_names(&config);

    if proxy_names.is_empty() {
        config.insert("proxy-groups".into(), Value::Sequence(Sequence::new()));
        return config;
    }

    let existing_groups = config
        .get("proxy-groups")
        .and_then(Value::as_sequence)
        .cloned()
        .unwrap_or_default();

    let mut manual_groups = Vec::new();
    let mut strategy_groups = Vec::new();

    for group in existing_groups {
        let Some(group_map) = group.as_mapping() else {
            continue;
        };

        let name = group_map.get("name").and_then(Value::as_str).unwrap_or("").trim();
        if name.is_empty() || is_software_blocked_group_name(name) {
            continue;
        }

        if is_auxiliary_group_mapping(group_map) {
            continue;
        }

        if is_strategy_group_mapping(group_map) {
            strategy_groups.push(Value::Mapping(group_map.clone()));
            continue;
        }

        if is_manual_group_mapping(group_map) {
            if name == root_group_name.as_str() {
                continue;
            }
            manual_groups.push(Value::Mapping(group_map.clone()));
        }
    }

    if strategy_groups.is_empty() {
        strategy_groups.push(build_group_value(
            SOFTWARE_STRATEGY_POOL_NAME,
            "url-test",
            &proxy_names,
            Some("http://www.gstatic.com/generate_204"),
            Some(86400),
        ));
    }

    let mut root_proxies = Vec::new();
    let mut seen = HashSet::new();

    for group in &strategy_groups {
        if let Some(name) = group
            .as_mapping()
            .and_then(|map| map.get("name"))
            .and_then(Value::as_str)
        {
            let trimmed = name.trim();
            if !trimmed.is_empty() && seen.insert(trimmed.to_owned()) {
                root_proxies.push(trimmed.to_owned().into());
            }
        }
    }

    for proxy_name in &proxy_names {
        if seen.insert(proxy_name.to_string()) {
            root_proxies.push(proxy_name.clone());
        }
    }

    let mut normalized_groups = Sequence::new();
    normalized_groups.push(build_group_value(
        root_group_name.as_str(),
        "select",
        &root_proxies,
        None,
        None,
    ));
    normalized_groups.extend(manual_groups);
    normalized_groups.extend(strategy_groups);

    config.insert("proxy-groups".into(), Value::Sequence(normalized_groups));
    config
}

impl Default for ProfileItems {
    fn default() -> Self {
        Self {
            config: Default::default(),
            profile_name: Default::default(),
            merge_item: ChainItem {
                uid: "".into(),
                data: ChainType::Merge(Mapping::new()),
            },
            script_item: ChainItem {
                uid: "".into(),
                data: ChainType::Script(tmpl::ITEM_SCRIPT.into()),
            },
            rules_item: ChainItem {
                uid: "".into(),
                data: ChainType::Rules(SeqMap::default()),
            },
            proxies_item: ChainItem {
                uid: "".into(),
                data: ChainType::Proxies(SeqMap::default()),
            },
            groups_item: ChainItem {
                uid: "".into(),
                data: ChainType::Groups(SeqMap::default()),
            },
            global_merge: ChainItem {
                uid: "Merge".into(),
                data: ChainType::Merge(Mapping::new()),
            },
            global_script: ChainItem {
                uid: "Script".into(),
                data: ChainType::Script(tmpl::ITEM_SCRIPT.into()),
            },
        }
    }
}

async fn get_config_values() -> ConfigValues {
    let clash = Config::clash().await;
    let clash_arc = clash.latest_arc();
    let clash_config = clash_arc.0.clone();
    drop(clash_arc);
    drop(clash);

    let verge = Config::verge().await;

    let verge_arc = verge.latest_arc();
    let IVerge {
        ref enable_tun_mode,
        ref verge_socks_enabled,
        ref verge_http_enabled,
        ref enable_dns_settings,
        ..
    } = *verge_arc;

    let (enable_tun, socks_enabled, http_enabled, enable_dns_settings) = (
        enable_tun_mode.unwrap_or(false),
        verge_socks_enabled.unwrap_or(false),
        verge_http_enabled.unwrap_or(false),
        enable_dns_settings.unwrap_or(false),
    );

    #[cfg(not(target_os = "windows"))]
    let redir_enabled = verge_arc.verge_redir_enabled.unwrap_or(false);

    #[cfg(target_os = "linux")]
    let tproxy_enabled = verge_arc.verge_tproxy_enabled.unwrap_or(false);

    drop(verge_arc);
    drop(verge);

    ConfigValues {
        clash_config,
        enable_tun,
        socks_enabled,
        http_enabled,
        enable_dns_settings,
        #[cfg(not(target_os = "windows"))]
        redir_enabled,
        #[cfg(target_os = "linux")]
        tproxy_enabled,
    }
}

#[allow(clippy::cognitive_complexity)]
async fn collect_profile_items() -> Result<ProfileItems> {
    let profiles = Config::profiles().await;
    let profiles_arc = profiles.latest_arc();
    drop(profiles);

    let current_profile_uid = match profiles_arc.get_current().cloned() {
        Some(uid) => uid,
        None => {
            drop(profiles_arc);
            return Ok(ProfileItems::default());
        }
    };

    let current = profiles_arc
        .current_mapping()
        .await
        .with_context(|| format!("failed to read current profile \"{current_profile_uid}\""))?;

    let current_item = match profiles_arc.get_item(&current_profile_uid) {
        Ok(item) => item,
        Err(err) => {
            return Err(err).with_context(|| format!("failed to get current profile \"{current_profile_uid}\""));
        }
    };

    let merge_uid = current_item.current_merge().cloned().unwrap_or_else(|| "Merge".into());
    let script_uid = current_item
        .current_script()
        .cloned()
        .unwrap_or_else(|| "Script".into());
    let rules_uid = current_item
        .current_rules()
        .cloned()
        .unwrap_or_else(|| AUXILIARY_RULES_NAME.into());
    let proxies_uid = current_item
        .current_proxies()
        .cloned()
        .unwrap_or_else(|| "Proxies".into());
    let groups_uid = current_item
        .current_groups()
        .cloned()
        .unwrap_or_else(|| "Groups".into());

    let name = current_item.name.clone().unwrap_or_default();

    let merge_item = {
        let item = profiles_arc.get_item(&merge_uid).ok().cloned();
        if let Some(item) = item {
            <Option<ChainItem>>::from_async(&item).await
        } else {
            None
        }
    }
    .unwrap_or_else(|| ChainItem {
        uid: "".into(),
        data: ChainType::Merge(Mapping::new()),
    });

    let script_item = {
        let item = profiles_arc.get_item(&script_uid).ok().cloned();
        if let Some(item) = item {
            <Option<ChainItem>>::from_async(&item).await
        } else {
            None
        }
    }
    .unwrap_or_else(|| ChainItem {
        uid: "".into(),
        data: ChainType::Script(tmpl::ITEM_SCRIPT.into()),
    });

    let rules_item = {
        let item = profiles_arc.get_item(&rules_uid).ok().cloned();
        if let Some(item) = item {
            <Option<ChainItem>>::from_async(&item).await
        } else {
            None
        }
    }
    .unwrap_or_else(|| ChainItem {
        uid: "".into(),
        data: ChainType::Rules(SeqMap::default()),
    });

    let proxies_item = {
        let item = profiles_arc.get_item(&proxies_uid).ok().cloned();
        if let Some(item) = item {
            <Option<ChainItem>>::from_async(&item).await
        } else {
            None
        }
    }
    .unwrap_or_else(|| ChainItem {
        uid: "".into(),
        data: ChainType::Proxies(SeqMap::default()),
    });

    let groups_item = {
        let item = profiles_arc.get_item(&groups_uid).ok().cloned();
        if let Some(item) = item {
            <Option<ChainItem>>::from_async(&item).await
        } else {
            None
        }
    }
    .unwrap_or_else(|| ChainItem {
        uid: "".into(),
        data: ChainType::Groups(SeqMap::default()),
    });

    let global_merge = {
        let item = profiles_arc.get_item("Merge").ok().cloned();
        if let Some(item) = item {
            <Option<ChainItem>>::from_async(&item).await
        } else {
            None
        }
    }
    .unwrap_or_else(|| ChainItem {
        uid: "Merge".into(),
        data: ChainType::Merge(Mapping::new()),
    });

    let global_script = {
        let item = profiles_arc.get_item("Script").ok().cloned();
        if let Some(item) = item {
            <Option<ChainItem>>::from_async(&item).await
        } else {
            None
        }
    }
    .unwrap_or_else(|| ChainItem {
        uid: "Script".into(),
        data: ChainType::Script(tmpl::ITEM_SCRIPT.into()),
    });

    drop(profiles_arc);

    Ok(ProfileItems {
        config: current,
        merge_item,
        script_item,
        rules_item,
        proxies_item,
        groups_item,
        global_merge,
        global_script,
        profile_name: name,
    })
}

async fn process_global_items(
    mut config: Mapping,
    global_merge: ChainItem,
    global_script: ChainItem,
    profile_name: &String,
) -> (Mapping, Vec<String>, HashMap<String, ResultLog>) {
    let mut result_map = HashMap::new();
    let mut exists_keys = use_keys(&config).collect::<Vec<_>>();

    if let ChainType::Merge(merge) = global_merge.data {
        exists_keys.extend(use_keys(&merge));
        config = use_merge(&merge, config.to_owned());
    }

    if let ChainType::Script(script) = global_script.data {
        let mut logs = vec![];
        match use_script(script, config.clone(), profile_name.clone()).await {
            Ok((res_config, res_logs)) => {
                exists_keys.extend(use_keys(&res_config));
                config = res_config;
                logs.extend(res_logs);
            }
            Err(err) => logs.push(("exception".into(), err.to_string().into())),
        }
        result_map.insert(global_script.uid, logs);
    }

    (config, exists_keys, result_map)
}

#[allow(clippy::too_many_arguments)]
async fn process_profile_items(
    mut config: Mapping,
    mut exists_keys: Vec<String>,
    mut result_map: HashMap<String, ResultLog>,
    rules_item: ChainItem,
    proxies_item: ChainItem,
    groups_item: ChainItem,
    merge_item: ChainItem,
    script_item: ChainItem,
    profile_name: &String,
) -> (Mapping, Vec<String>, HashMap<String, ResultLog>) {
    if let ChainType::Rules(rules) = rules_item.data {
        config = use_seq(rules, config.to_owned(), "rules");
    }

    if let ChainType::Proxies(proxies) = proxies_item.data {
        config = use_seq(proxies, config.to_owned(), "proxies");
    }
    config = remove_proxy_groups(config);

    if let ChainType::Merge(merge) = merge_item.data {
        exists_keys.extend(use_keys(&merge));
        config = use_merge(&merge, config.to_owned());
    }

    if let ChainType::Script(script) = script_item.data {
        let mut logs = vec![];
        match use_script(script, config.clone(), profile_name.clone()).await {
            Ok((res_config, res_logs)) => {
                exists_keys.extend(use_keys(&res_config));
                config = res_config;
                logs.extend(res_logs);
            }
            Err(err) => logs.push(("exception".into(), err.to_string().into())),
        }
        result_map.insert(script_item.uid, logs);
    }

    config = remove_proxy_groups(config);

    if let ChainType::Groups(groups) = groups_item.data {
        config = use_seq(groups, config.to_owned(), "proxy-groups");
    }

    (config, exists_keys, result_map)
}

async fn merge_default_config(
    mut config: Mapping,
    clash_config: Mapping,
    socks_enabled: bool,
    http_enabled: bool,
    #[cfg(not(target_os = "windows"))] redir_enabled: bool,
    #[cfg(target_os = "linux")] tproxy_enabled: bool,
) -> Mapping {
    for (key, value) in clash_config.into_iter() {
        if key.as_str() == Some("tun") {
            let mut tun = config.get_mut("tun").map_or_else(Mapping::new, |val| {
                val.as_mapping().cloned().unwrap_or_else(Mapping::new)
            });
            let patch_tun = value.as_mapping().cloned().unwrap_or_else(Mapping::new);
            for (key, value) in patch_tun.into_iter() {
                tun.insert(key, value);
            }
            config.insert("tun".into(), tun.into());
        } else {
            if key.as_str() == Some("socks-port") && !socks_enabled {
                config.remove("socks-port");
                continue;
            }
            if key.as_str() == Some("port") && !http_enabled {
                config.remove("port");
                continue;
            }
            #[cfg(target_os = "windows")]
            {
                if key.as_str() == Some("redir-port") {
                    continue;
                }
            }
            #[cfg(not(target_os = "windows"))]
            {
                if key.as_str() == Some("redir-port") && !redir_enabled {
                    config.remove("redir-port");
                    continue;
                }
            }
            #[cfg(target_os = "linux")]
            {
                if key.as_str() == Some("tproxy-port") && !tproxy_enabled {
                    config.remove("tproxy-port");
                    continue;
                }
            }
            #[cfg(not(target_os = "linux"))]
            {
                if key.as_str() == Some("tproxy-port") {
                    config.remove("tproxy-port");
                    continue;
                }
            }
            // 处理 external-controller 键的开关逻辑
            if key.as_str() == Some("external-controller") {
                let verge = Config::verge().await;
                let verge_arc = verge.latest_arc();
                let mut enable_external_controller = verge_arc.enable_external_controller.unwrap_or(false);
                #[cfg(target_os = "windows")]
                {
                    enable_external_controller |= verge_arc.enable_tun_mode.unwrap_or(false);
                }
                drop(verge_arc);
                drop(verge);

                if enable_external_controller {
                    config.insert(key, value);
                } else {
                    // 如果禁用了外部控制器，设置为空字符串
                    config.insert(key, "".into());
                }
            } else {
                config.insert(key, value);
            }
        }
    }

    config
}

async fn apply_dns_settings(mut config: Mapping, enable_dns_settings: bool) -> Mapping {
    if enable_dns_settings && let Ok(app_dir) = dirs::app_home_dir() {
        let dns_path = app_dir.join(constants::files::DNS_CONFIG);

        if dns_path.exists()
            && let Ok(dns_yaml) = fs::read_to_string(&dns_path).await
            && let Ok(dns_config) = serde_yaml_ng::from_str::<serde_yaml_ng::Mapping>(&dns_yaml)
        {
            if let Some(hosts_value) = dns_config.get("hosts")
                && hosts_value.is_mapping()
            {
                config.insert("hosts".into(), hosts_value.clone());
                logging!(info, Type::Core, "apply hosts configuration");
            }

            if let Some(dns_value) = dns_config.get("dns") {
                if let Some(dns_mapping) = dns_value.as_mapping() {
                    config.insert("dns".into(), dns_mapping.clone().into());
                    logging!(info, Type::Core, "apply dns_config.yaml (dns section)");
                }
            } else {
                config.insert("dns".into(), dns_config.into());
                logging!(info, Type::Core, "apply dns_config.yaml");
            }
        }
    }

    // Defensive: ensure proxy-server-nameserver is non-empty when respect-rules is enabled.
    // Mihomo rejects the config if respect-rules=true but proxy-server-nameserver is missing.
    if let Some(dns_value) = config.get_mut("dns") {
        if let Some(dns_mapping) = dns_value.as_mapping_mut() {
            let respect_rules = dns_mapping
                .get("respect-rules")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);

            if respect_rules {
                let has_proxy_ns = dns_mapping
                    .get("proxy-server-nameserver")
                    .and_then(|v| v.as_sequence())
                    .map(|s| !s.is_empty())
                    .unwrap_or(false);

                if !has_proxy_ns {
                    // Auto-fill with domestic plain IP nameservers
                    let fallback_ns: Vec<Value> = ["223.5.5.5", "119.29.29.29"].into_iter().map(Value::from).collect();
                    dns_mapping.insert("proxy-server-nameserver".into(), Value::Sequence(fallback_ns));
                    logging!(
                        warn,
                        Type::Core,
                        "respect-rules enabled but proxy-server-nameserver missing, auto-filled with domestic DNS"
                    );
                }
            }
        }
    }

    config
}

/// Enhance mode
/// 返回最终订阅、该订阅包含的键、和script执行的结果
pub async fn enhance() -> Result<(Mapping, HashSet<String>, HashMap<String, ResultLog>)> {
    // gather config values
    let cfg_vals = get_config_values().await;
    let ConfigValues {
        clash_config,
        enable_tun,
        socks_enabled,
        http_enabled,
        enable_dns_settings,
        #[cfg(not(target_os = "windows"))]
        redir_enabled,
        #[cfg(target_os = "linux")]
        tproxy_enabled,
    } = cfg_vals;

    // collect profile items
    let profile = collect_profile_items().await?;
    let config = profile.config;
    let merge_item = profile.merge_item;
    let script_item = profile.script_item;
    let rules_item = profile.rules_item;
    let proxies_item = profile.proxies_item;
    let groups_item = profile.groups_item;
    let global_merge = profile.global_merge;
    let global_script = profile.global_script;
    let profile_name = profile.profile_name;
    let root_group_name = preferred_root_group_name(&config, &profile_name);

    // process globals
    let (config, exists_keys, result_map) =
        process_global_items(config, global_merge, global_script, &profile_name).await;

    // process profile-specific items
    let (config, exists_keys, result_map) = process_profile_items(
        config,
        exists_keys,
        result_map,
        rules_item,
        proxies_item,
        groups_item,
        merge_item,
        script_item,
        &profile_name,
    )
    .await;
    let config = normalize_software_owned_proxy_groups(config, &root_group_name);

    // merge default clash config
    let config = merge_default_config(
        config,
        clash_config,
        socks_enabled,
        http_enabled,
        #[cfg(not(target_os = "windows"))]
        redir_enabled,
        #[cfg(target_os = "linux")]
        tproxy_enabled,
    )
    .await;

    let mut config = config;
    config = cleanup_proxy_groups(config);
    config = apply_connection_stability(config);
    config = apply_multiplex(config);

    config = use_tun(config, enable_tun);
    config = apply_sniffer_config(config);
    config = apply_tls_fingerprint_config(config);
    config = apply_obfuscation_config(config);
    config = apply_traffic_obfuscation_config(config);
    config = apply_blackhole_breaker_config(config).await;
    config = apply_timezone_spoof_config(config);
    config = use_sort(config);

    // dns settings
    config = apply_dns_settings(config, enable_dns_settings).await;
    config = apply_stable_egress_policy(config);
    config = use_sort(config);

    let mut exists_keys_set = HashSet::new();
    exists_keys_set.extend(exists_keys);

    Ok((config, exists_keys_set, result_map))
}

#[allow(clippy::expect_used)]
#[cfg(test)]
mod tests {
    pub(crate) fn parse_yaml(yaml: &str) -> serde_yaml_ng::Mapping {
        serde_yaml_ng::from_str(yaml).expect("Failed to parse test yaml")
    }

    use super::apply_stable_egress_policy_with_advanced;

    #[test]
    fn append_stable_egress_groups_for_high_risk_domains() {
        let config_str = r#"
proxies:
  - name: "node-a"
    type: ss
  - name: "node-b"
    type: ss
dns:
  nameserver:
    - https://dns.alidns.com/dns-query
    - https://dns.google/dns-query
  fallback:
    - https://cloudflare-dns.com/dns-query
  nameserver-policy:
    geosite:cn:
      - https://dns.alidns.com/dns-query
    geosite:geolocation-!cn:
      - https://dns.google/dns-query
proxy-groups: []
rules:
  - MATCH,Proxy
"#;

        let config: serde_yaml_ng::Mapping = serde_yaml_ng::from_str(config_str).expect("Failed to parse test yaml");
        let mut advanced = crate::config::AdvancedConfig::default();
        advanced.egress_identity = crate::core::egress_identity::EgressIdentityConfig::recommended();
        advanced.egress_identity.enabled = true;
        if let Some(profile) = advanced
            .egress_identity
            .profiles
            .iter_mut()
            .find(|profile| profile.id == "ai-strict")
        {
            profile.preferred_nodes = vec!["node-b".to_string()];
        }
        advanced.session_affinity.enabled = true;
        advanced.session_affinity.domain_rules = vec![crate::core::session_affinity::DomainBindingRule {
            domain_pattern: "*.openai.com".to_string(),
            enabled: true,
            bound_node: None,
            ttl: 86400,
            fallback_policy: crate::core::session_affinity::FallbackPolicy::Manual,
            description: "test".to_string(),
        }];

        let config = apply_stable_egress_policy_with_advanced(config, &advanced);

        let profile = config
            .get("profile")
            .and_then(serde_yaml_ng::Value::as_mapping)
            .expect("profile should exist");
        assert_eq!(
            profile.get("store-selected").and_then(serde_yaml_ng::Value::as_bool),
            Some(true)
        );

        let groups = config
            .get("proxy-groups")
            .and_then(|v| v.as_sequence())
            .cloned()
            .expect("proxy-groups should be a sequence");

        let stable_group = groups
            .iter()
            .find(|group| {
                group.get("name").and_then(serde_yaml_ng::Value::as_str) == Some("VERGE-STABLE-STAR-OPENAI-COM")
            })
            .and_then(|group| group.as_mapping())
            .expect("stable group should exist");

        let proxies = stable_group
            .get("proxies")
            .and_then(|v| v.as_sequence())
            .expect("stable group proxies should exist");
        assert_eq!(proxies[0].as_str(), Some("node-b"));

        let rules = config
            .get("rules")
            .and_then(|v| v.as_sequence())
            .cloned()
            .expect("rules should be a sequence");
        assert_eq!(
            rules[0].as_str(),
            Some("DOMAIN-SUFFIX,openai.com,VERGE-STABLE-STAR-OPENAI-COM")
        );
        assert_eq!(rules[1].as_str(), Some("MATCH,Proxy"));

        let dns = config
            .get("dns")
            .and_then(serde_yaml_ng::Value::as_mapping)
            .expect("dns should exist");
        let nameserver_policy = dns
            .get("nameserver-policy")
            .and_then(serde_yaml_ng::Value::as_mapping)
            .expect("nameserver-policy should exist");
        let high_risk_policy = nameserver_policy
            .get("+.openai.com")
            .and_then(serde_yaml_ng::Value::as_sequence)
            .expect("high-risk domain dns policy should exist");
        assert_eq!(high_risk_policy.len(), 1);
        assert_eq!(high_risk_policy[0].as_str(), Some("https://dns.google/dns-query"));
    }
}
