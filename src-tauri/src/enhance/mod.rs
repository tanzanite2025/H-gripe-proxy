mod chain;
pub mod field;
mod merge;
mod script;
pub mod seq;
mod tun;

use self::{
    chain::{AsyncChainItemFrom as _, ChainItem, ChainType},
    field::{use_keys, use_lowercase, use_sort},
    merge::use_merge,
    script::use_script,
    seq::{SeqMap, use_seq},
    tun::use_tun,
};
use crate::utils::dirs;
use crate::core::{
    egress_identity::{
        DnsMode, EgressIdentityManager, EgressNodeMetadata, EgressSelectionContext,
        ResolvedEgressIdentity,
    },
    session_affinity::{DomainBindingRule, FallbackPolicy},
    stable_egress::{
        STABLE_EGRESS_GROUP_PREFIX, domain_probe_for_pattern, stable_egress_group_name,
        stable_egress_rule_line,
    },
};
use crate::{
    config::{AdvancedConfig, Config, IVerge},
    constants,
    utils::tmpl,
};
use anyhow::{Context as _, Result};
use clash_verge_logging::{Type, logging};
use regex::Regex;
use serde_yaml_ng::{Mapping, Sequence, Value};
use smartstring::alias::String;
use std::collections::{HashMap, HashSet};
use tokio::fs;

type ResultLog = Vec<(String, String)>;

const STANDARD_REGION_POOL_PREFIX: &str = "VERGE-REGION-";
const STANDARD_REGION_POOL_URL: &str = "https://www.gstatic.com/generate_204";
const STANDARD_REGION_POOL_INTERVAL: u64 = 300;
const STANDARD_REGION_POOL_TIMEOUT: u64 = 5_000;
const STANDARD_REGION_POOL_MAX_FAILED_TIMES: u64 = 3;

#[derive(Clone, Copy)]
struct StandardRegionPoolSpec {
    code: &'static str,
    filter: &'static str,
}

impl StandardRegionPoolSpec {
    fn group_name(self) -> std::string::String {
        format!("{STANDARD_REGION_POOL_PREFIX}{}-AUTO", self.code)
    }

    fn matches(self, value: &str) -> bool {
        Regex::new(self.filter).map(|re| re.is_match(value)).unwrap_or(false)
    }
}

#[derive(Debug, Clone)]
struct StaticProxySpec {
    name: std::string::String,
    server: Option<std::string::String>,
}

const STANDARD_REGION_POOL_SPECS: [StandardRegionPoolSpec; 6] = [
    StandardRegionPoolSpec {
        code: "TW",
        filter: r"(?i)(台湾|台灣|臺灣|taiwan|taipei|台北|新北|taichung|台中|tainan|台南|kaohsiung|高雄|\bTW\b)",
    },
    StandardRegionPoolSpec {
        code: "HK",
        filter: r"(?i)(香港|hong[\s_-]?kong|\bHK\b)",
    },
    StandardRegionPoolSpec {
        code: "JP",
        filter: r"(?i)(日本|japan|tokyo|osaka|sapporo|\bJP\b)",
    },
    StandardRegionPoolSpec {
        code: "SG",
        filter: r"(?i)(新加坡|singapore|\bSG\b)",
    },
    StandardRegionPoolSpec {
        code: "US",
        filter: r"(?i)(美国|美國|united[\s_-]?states|america|los[\s_-]?angeles|san[\s_-]?jose|seattle|new[\s_-]?york|ashburn|\bUS\b)",
    },
    StandardRegionPoolSpec {
        code: "KR",
        filter: r"(?i)(韩国|韓國|korea|seoul|busan|\bKR\b)",
    },
];

#[derive(Debug)]
struct ConfigValues {
    clash_config: Mapping,
    clash_core: Option<String>,
    enable_tun: bool,
    enable_builtin: bool,
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
        ref enable_builtin_enhanced,
        ref verge_socks_enabled,
        ref verge_http_enabled,
        ref enable_dns_settings,
        ..
    } = *verge_arc;

    let (clash_core, enable_tun, enable_builtin, socks_enabled, http_enabled, enable_dns_settings) = (
        Some(verge_arc.get_valid_clash_core()),
        enable_tun_mode.unwrap_or(false),
        enable_builtin_enhanced.unwrap_or(true),
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
        clash_core,
        enable_tun,
        enable_builtin,
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
    let rules_uid = current_item.current_rules().cloned().unwrap_or_else(|| "Rules".into());
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

    if let ChainType::Groups(groups) = groups_item.data {
        config = use_seq(groups, config.to_owned(), "proxy-groups");
    }

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
                let enable_external_controller = Config::verge()
                    .await
                    .latest_arc()
                    .enable_external_controller
                    .unwrap_or(false);

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

async fn apply_builtin_scripts(mut config: Mapping, clash_core: Option<String>, enable_builtin: bool) -> Mapping {
    if enable_builtin {
        let items: Vec<_> = ChainItem::builtin()
            .into_iter()
            .filter(|(s, _)| s.is_support(clash_core.as_ref()))
            .map(|(_, c)| c)
            .collect();
        for item in items {
            logging!(debug, Type::Core, "run builtin script {}", item.uid);
            if let ChainType::Script(script) = item.data {
                match use_script(script, config.clone(), String::from("")).await {
                    Ok((res_config, _)) => {
                        config = res_config;
                    }
                    Err(err) => {
                        logging!(error, Type::Core, "builtin script error `{err}`");
                    }
                }
            }
        }
    }

    config
}

fn cleanup_proxy_groups(mut config: Mapping) -> Mapping {
    const BUILTIN_POLICIES: &[&str] = &["DIRECT", "REJECT", "REJECT-DROP", "PASS"];

    let proxy_names = config
        .get("proxies")
        .and_then(|v| v.as_sequence())
        .map(|seq| {
            seq.iter()
                .filter_map(|item| match item {
                    Value::Mapping(map) => map
                        .get("name")
                        .and_then(Value::as_str)
                        .map(|name| name.to_owned().into()),
                    Value::String(name) => Some(name.to_owned().into()),
                    _ => None,
                })
                .collect::<HashSet<String>>()
        })
        .unwrap_or_default();

    let group_names = config
        .get("proxy-groups")
        .and_then(|v| v.as_sequence())
        .map(|seq| {
            seq.iter()
                .filter_map(|item| {
                    item.as_mapping()
                        .and_then(|map| map.get("name"))
                        .and_then(Value::as_str)
                        .map(std::convert::Into::into)
                })
                .collect::<HashSet<String>>()
        })
        .unwrap_or_default();

    let provider_names = config
        .get("proxy-providers")
        .and_then(Value::as_mapping)
        .map(|map| {
            map.keys()
                .filter_map(Value::as_str)
                .map(std::convert::Into::into)
                .collect::<HashSet<String>>()
        })
        .unwrap_or_default();

    let mut allowed_names = proxy_names;
    allowed_names.extend(group_names);
    allowed_names.extend(provider_names.iter().cloned());
    allowed_names.extend(BUILTIN_POLICIES.iter().map(|p| (*p).into()));

    if let Some(Value::Sequence(groups)) = config.get_mut("proxy-groups") {
        for group in groups {
            if let Some(group_map) = group.as_mapping_mut() {
                let mut has_valid_provider = false;

                if let Some(Value::Sequence(uses)) = group_map.get_mut("use") {
                    uses.retain(|provider| match provider {
                        Value::String(name) => {
                            let exists = provider_names.contains(name.as_str());
                            has_valid_provider = has_valid_provider || exists;
                            exists
                        }
                        _ => false,
                    });
                }

                if let Some(Value::Sequence(proxies)) = group_map.get_mut("proxies") {
                    proxies.retain(|proxy| match proxy {
                        Value::String(name) => allowed_names.contains(name.as_str()) || has_valid_provider,
                        _ => true,
                    });
                }
            }
        }
    }

    config
}

fn append_standard_region_pools(mut config: Mapping) -> Mapping {
    let proxy_names = config
        .get("proxies")
        .and_then(Value::as_sequence)
        .map(|seq| {
            seq.iter()
                .filter_map(|item| match item {
                    Value::Mapping(map) => map.get("name").and_then(Value::as_str).map(String::from),
                    Value::String(name) => Some(String::from(name.as_str())),
                    _ => None,
                })
                .collect::<Vec<String>>()
        })
        .unwrap_or_default();

    let provider_names = config
        .get("proxy-providers")
        .and_then(Value::as_mapping)
        .map(|map| {
            map.keys()
                .filter_map(Value::as_str)
                .map(String::from)
                .collect::<Vec<String>>()
        })
        .unwrap_or_default();

    if proxy_names.is_empty() && provider_names.is_empty() {
        return config;
    }

    let mut groups = config
        .get("proxy-groups")
        .and_then(Value::as_sequence)
        .cloned()
        .unwrap_or_default();

    groups.retain(|group| {
        group
            .get("name")
            .and_then(Value::as_str)
            .map(|name| !name.starts_with(STANDARD_REGION_POOL_PREFIX))
            .unwrap_or(true)
    });

    for spec in STANDARD_REGION_POOL_SPECS {
        let static_matches = proxy_names
            .iter()
            .filter(|name| spec.matches(name))
            .cloned()
            .collect::<Vec<String>>();

        if static_matches.is_empty() && provider_names.is_empty() {
            continue;
        }

        let mut group = Mapping::new();
        let group_name = spec.group_name();
        group.insert("name".into(), Value::from(group_name.as_str()));
        group.insert("type".into(), "fallback".into());
        group.insert("url".into(), STANDARD_REGION_POOL_URL.into());
        group.insert("interval".into(), STANDARD_REGION_POOL_INTERVAL.into());
        group.insert("timeout".into(), STANDARD_REGION_POOL_TIMEOUT.into());
        group.insert(
            "max-failed-times".into(),
            STANDARD_REGION_POOL_MAX_FAILED_TIMES.into(),
        );
        group.insert("lazy".into(), true.into());
        group.insert("hidden".into(), true.into());

        if !static_matches.is_empty() {
            group.insert(
                "proxies".into(),
                Value::Sequence(
                    static_matches
                        .into_iter()
                        .map(|name| Value::from(name.as_str()))
                        .collect::<Sequence>(),
                ),
            );
        }

        if !provider_names.is_empty() {
            group.insert(
                "use".into(),
                Value::Sequence(
                    provider_names
                        .iter()
                        .map(|name| Value::from(name.as_str()))
                        .collect::<Sequence>(),
                ),
            );
            group.insert("filter".into(), spec.filter.into());
        }

        groups.push(Value::Mapping(group));
    }

    config.insert("proxy-groups".into(), Value::Sequence(groups));
    config
}

fn load_advanced_config_for_stable_egress() -> AdvancedConfig {
    dirs::app_home_dir()
        .ok()
        .map(|dir| dir.join("advanced.yaml"))
        .and_then(|path| AdvancedConfig::load(&path).ok())
        .unwrap_or_default()
}

pub(crate) fn apply_stable_egress_policy(config: Mapping) -> Mapping {
    let advanced_config = load_advanced_config_for_stable_egress();
    apply_stable_egress_policy_with_advanced(config, &advanced_config)
}

fn apply_stable_egress_policy_with_advanced(
    mut config: Mapping,
    advanced_config: &AdvancedConfig,
) -> Mapping {
    if !advanced_config.session_affinity.enabled || !advanced_config.egress_identity.enabled {
        return config;
    }

    let domain_rules = advanced_config
        .session_affinity
        .domain_rules
        .iter()
        .filter(|rule| {
            rule.enabled && matches!(rule.fallback_policy.clone(), FallbackPolicy::Manual)
        })
        .cloned()
        .collect::<Vec<_>>();

    if domain_rules.is_empty() {
        return config;
    }

    let static_proxies = collect_static_proxy_specs(&config);
    let provider_names = collect_provider_names(&config);

    if static_proxies.is_empty() && provider_names.is_empty() {
        return config;
    }

    let egress_manager =
        EgressIdentityManager::new_with_config(advanced_config.egress_identity.clone());
    let metadata = build_static_egress_metadata(&static_proxies, advanced_config);
    let static_proxy_names = static_proxies
        .iter()
        .map(|proxy| proxy.name.clone())
        .collect::<Vec<_>>();

    let mut groups = config
        .get("proxy-groups")
        .and_then(Value::as_sequence)
        .cloned()
        .unwrap_or_default();
    groups.retain(|group| {
        group
            .get("name")
            .and_then(Value::as_str)
            .map(|name| !name.starts_with(STABLE_EGRESS_GROUP_PREFIX))
            .unwrap_or(true)
    });

    let mut generated_group_names = HashSet::<std::string::String>::new();
    let mut generated_rules = Sequence::new();
    let mut generated_dns_policy = Mapping::new();

    for rule in domain_rules {
        let group_name = stable_egress_group_name(&rule.domain_pattern);
        let Some(rule_line) = stable_egress_rule_line(&rule.domain_pattern, &group_name) else {
            continue;
        };
        let resolved_identity =
            preview_stable_egress_identity(&egress_manager, &rule, &static_proxy_names, &metadata);

        if generated_group_names.insert(group_name.clone()) {
            let mut ordered_nodes = static_proxy_names.clone();

            if let Some(resolved_identity) = resolved_identity.as_ref() {
                ordered_nodes = prioritize_node_names(
                    ordered_nodes,
                    &resolved_identity.selected_node,
                    !provider_names.is_empty(),
                );
            }

            if let Some(bound_node) = rule.bound_node.as_ref() {
                ordered_nodes =
                    prioritize_node_names(ordered_nodes, bound_node, !provider_names.is_empty());
            }

            ordered_nodes = dedupe_node_names(ordered_nodes);

            if ordered_nodes.is_empty() && provider_names.is_empty() {
                continue;
            }

            let mut group = Mapping::new();
            group.insert("name".into(), Value::from(group_name.as_str()));
            group.insert("type".into(), Value::from("select"));

            if !ordered_nodes.is_empty() {
                group.insert(
                    "proxies".into(),
                    Value::Sequence(
                        ordered_nodes
                            .iter()
                            .map(|name| Value::from(name.as_str()))
                            .collect(),
                    ),
                );
            }

            if !provider_names.is_empty() {
                group.insert(
                    "use".into(),
                    Value::Sequence(
                        provider_names
                            .iter()
                            .map(|name| Value::from(name.as_str()))
                            .collect(),
                    ),
                );
            }

            groups.push(Value::Mapping(group));
        }

        if let Some(policy_key) = stable_dns_policy_key(&rule.domain_pattern)
            && let Some(resolved_identity) = resolved_identity.as_ref()
            && let Some(nameservers) =
                stable_dns_server_override(&config, advanced_config, resolved_identity)
        {
            generated_dns_policy.insert(
                Value::from(policy_key.as_str()),
                Value::Sequence(
                    nameservers
                        .iter()
                        .map(|server| Value::from(server.as_str()))
                        .collect(),
                ),
            );
        }

        generated_rules.push(Value::from(rule_line.as_str()));
    }

    if generated_rules.is_empty() {
        return config;
    }

    let mut existing_rules = config
        .get("rules")
        .and_then(Value::as_sequence)
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .filter(|rule| {
            rule.as_str()
                .map(|line| !line.contains(STABLE_EGRESS_GROUP_PREFIX))
                .unwrap_or(true)
        })
        .collect::<Sequence>();

    generated_rules.append(&mut existing_rules);

    let mut profile = config
        .get("profile")
        .and_then(Value::as_mapping)
        .cloned()
        .unwrap_or_default();
    profile.insert("store-selected".into(), Value::Bool(true));

    config.insert("profile".into(), Value::Mapping(profile));
    config.insert("proxy-groups".into(), Value::Sequence(groups));
    config.insert("rules".into(), Value::Sequence(generated_rules));
    apply_stable_egress_dns_overrides(&mut config, generated_dns_policy);
    config
}

fn collect_static_proxy_specs(config: &Mapping) -> Vec<StaticProxySpec> {
    config
        .get("proxies")
        .and_then(Value::as_sequence)
        .map(|proxies| {
            proxies
                .iter()
                .filter_map(|proxy| match proxy {
                    Value::Mapping(mapping) => {
                        mapping.get("name").and_then(Value::as_str).map(|name| StaticProxySpec {
                            name: name.to_string(),
                            server: mapping
                                .get("server")
                                .and_then(Value::as_str)
                                .map(|server| server.to_string()),
                        })
                    }
                    Value::String(name) => Some(StaticProxySpec {
                        name: name.to_string(),
                        server: None,
                    }),
                    _ => None,
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default()
}

fn collect_provider_names(config: &Mapping) -> Vec<std::string::String> {
    config
        .get("proxy-providers")
        .and_then(Value::as_mapping)
        .map(|providers| {
            providers
                .keys()
                .filter_map(Value::as_str)
                .map(|name| name.to_string())
                .collect::<Vec<_>>()
        })
        .unwrap_or_default()
}

fn build_static_egress_metadata(
    static_proxies: &[StaticProxySpec],
    advanced_config: &AdvancedConfig,
) -> Vec<EgressNodeMetadata> {
    static_proxies
        .iter()
        .map(|proxy| {
            let mut metadata = EgressNodeMetadata {
                name: proxy.name.clone(),
                server: proxy.server.clone(),
                ..Default::default()
            };

            if let Some((pool_name, pool_type, server)) = advanced_config
                .multipath
                .node_pools
                .iter()
                .filter(|pool| pool.enabled)
                .find_map(|pool| {
                    pool.nodes
                        .iter()
                        .find(|node| node.enabled && node.name.eq_ignore_ascii_case(&proxy.name))
                        .map(|node| {
                            (
                                pool.name.clone(),
                                format!("{:?}", pool.pool_type),
                                node.server.clone(),
                            )
                        })
                })
            {
                metadata.pool_name = Some(pool_name);
                metadata.pool_type = Some(pool_type);
                if metadata.server.is_none() {
                    metadata.server = Some(server);
                }
            }

            metadata
        })
        .collect::<Vec<_>>()
}

fn preview_stable_egress_identity(
    manager: &EgressIdentityManager,
    rule: &DomainBindingRule,
    static_proxy_names: &[std::string::String],
    metadata: &[EgressNodeMetadata],
) -> Option<ResolvedEgressIdentity> {
    let domain = domain_probe_for_pattern(&rule.domain_pattern)?;
    manager
        .preview_match(EgressSelectionContext {
            domain: Some(domain),
            available_nodes: static_proxy_names.to_vec(),
            available_node_metadata: metadata.to_vec(),
            ..Default::default()
        })
        .ok()
}

fn stable_dns_policy_key(pattern: &str) -> Option<std::string::String> {
    if let Some(suffix) = pattern.strip_prefix("*.").or_else(|| pattern.strip_prefix('*')) {
        let suffix = suffix.trim_start_matches('.').trim();
        if suffix.is_empty() {
            None
        } else {
            Some(format!("+.{suffix}"))
        }
    } else if pattern.contains('*') {
        None
    } else {
        let domain = pattern.trim();
        if domain.is_empty() {
            None
        } else {
            Some(domain.to_string())
        }
    }
}

fn stable_dns_server_override(
    config: &Mapping,
    advanced_config: &AdvancedConfig,
    resolved_identity: &ResolvedEgressIdentity,
) -> Option<Vec<std::string::String>> {
    let dns_mapping = config.get("dns").and_then(Value::as_mapping)?;
    let profile = advanced_config
        .egress_identity
        .profiles
        .iter()
        .find(|profile| profile.id == resolved_identity.profile_id)?;

    let remote_dns = matches!(resolved_identity.dns_mode, DnsMode::Remote)
        || profile.dns_policy.force_remote_dns;
    let hijack_dns = matches!(resolved_identity.dns_mode, DnsMode::Hijack);

    if !remote_dns && !hijack_dns {
        return None;
    }

    let domestic_nameservers =
        mapping_nested_string_sequence(dns_mapping, "nameserver-policy", "geosite:cn");
    let foreign_nameservers = mapping_nested_string_sequence(
        dns_mapping,
        "nameserver-policy",
        "geosite:geolocation-!cn",
    );
    let nameserver = mapping_string_sequence(dns_mapping, "nameserver");
    let fallback = mapping_string_sequence(dns_mapping, "fallback");

    if remote_dns {
        first_non_empty_string_sequence([
            foreign_nameservers,
            fallback,
            nameserver,
            domestic_nameservers,
        ])
        .map(dedupe_string_sequence)
    } else {
        first_non_empty_string_sequence([
            nameserver,
            domestic_nameservers,
            fallback,
            foreign_nameservers,
        ])
        .map(dedupe_string_sequence)
    }
}

fn apply_stable_egress_dns_overrides(config: &mut Mapping, overrides: Mapping) {
    if overrides.is_empty() {
        return;
    }

    let Some(Value::Mapping(dns_mapping)) = config.get_mut("dns") else {
        return;
    };

    let mut nameserver_policy = dns_mapping
        .get("nameserver-policy")
        .and_then(Value::as_mapping)
        .cloned()
        .unwrap_or_default();

    for (key, value) in overrides {
        nameserver_policy.insert(key, value);
    }

    dns_mapping.insert("nameserver-policy".into(), Value::Mapping(nameserver_policy));
}

fn mapping_string_sequence(mapping: &Mapping, key: &str) -> Vec<std::string::String> {
    mapping
        .get(key)
        .and_then(Value::as_sequence)
        .map(|sequence| {
            sequence
                .iter()
                .filter_map(Value::as_str)
                .map(|value| value.to_string())
                .collect::<Vec<_>>()
        })
        .unwrap_or_default()
}

fn mapping_nested_string_sequence(
    mapping: &Mapping,
    key: &str,
    nested_key: &str,
) -> Vec<std::string::String> {
    mapping
        .get(key)
        .and_then(Value::as_mapping)
        .and_then(|nested_mapping| nested_mapping.get(nested_key))
        .and_then(Value::as_sequence)
        .map(|sequence| {
            sequence
                .iter()
                .filter_map(Value::as_str)
                .map(|value| value.to_string())
                .collect::<Vec<_>>()
        })
        .unwrap_or_default()
}

fn first_non_empty_string_sequence<const N: usize>(
    sequences: [Vec<std::string::String>; N],
) -> Option<Vec<std::string::String>> {
    sequences.into_iter().find(|sequence| !sequence.is_empty())
}

fn dedupe_string_sequence(values: Vec<std::string::String>) -> Vec<std::string::String> {
    let mut seen = HashSet::<std::string::String>::new();
    let mut deduped = Vec::with_capacity(values.len());

    for value in values {
        if seen.insert(value.clone()) {
            deduped.push(value);
        }
    }

    deduped
}

fn prioritize_node_names(
    mut available_nodes: Vec<std::string::String>,
    preferred_node: &str,
    allow_insert_missing: bool,
) -> Vec<std::string::String> {
    if preferred_node.trim().is_empty() {
        return available_nodes;
    }

    if let Some(index) = available_nodes
        .iter()
        .position(|node| node.eq_ignore_ascii_case(preferred_node))
    {
        let preferred = available_nodes.remove(index);
        available_nodes.insert(0, preferred);
    } else if allow_insert_missing {
        available_nodes.insert(0, preferred_node.to_string());
    }

    available_nodes
}

fn dedupe_node_names(nodes: Vec<std::string::String>) -> Vec<std::string::String> {
    let mut seen = HashSet::<std::string::String>::new();
    let mut deduped = Vec::with_capacity(nodes.len());

    for node in nodes {
        let key = node.to_ascii_lowercase();
        if seen.insert(key) {
            deduped.push(node);
        }
    }

    deduped
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

    config
}

/// Enhance mode
/// 返回最终订阅、该订阅包含的键、和script执行的结果
pub async fn enhance() -> Result<(Mapping, HashSet<String>, HashMap<String, ResultLog>)> {
    // gather config values
    let cfg_vals = get_config_values().await;
    let ConfigValues {
        clash_config,
        clash_core,
        enable_tun,
        enable_builtin,
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

    // builtin scripts
    let mut config = apply_builtin_scripts(config, clash_core, enable_builtin).await;

    config = cleanup_proxy_groups(config);
    config = append_standard_region_pools(config);

    config = use_tun(config, enable_tun);
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
    use super::{
        append_standard_region_pools, apply_stable_egress_policy_with_advanced,
        cleanup_proxy_groups,
    };

    #[test]
    fn remove_missing_proxies_from_groups() {
        let config_str = r#"
proxies:
  - name: "alive-node"
    type: ss
proxy-groups:
  - name: "manual"
    type: select
    proxies:
      - "alive-node"
      - "missing-node"
      - "DIRECT"
  - name: "nested"
    type: select
    proxies:
      - "manual"
      - "ghost"
"#;

        let mut config: serde_yaml_ng::Mapping =
            serde_yaml_ng::from_str(config_str).expect("Failed to parse test yaml");
        config = cleanup_proxy_groups(config);

        let groups = config
            .get("proxy-groups")
            .and_then(|v| v.as_sequence())
            .cloned()
            .expect("proxy-groups should be a sequence");

        let manual_group = groups
            .iter()
            .find(|group| group.get("name").and_then(serde_yaml_ng::Value::as_str) == Some("manual"))
            .and_then(|group| group.as_mapping())
            .expect("manual group should exist");

        let manual_proxies = manual_group
            .get("proxies")
            .and_then(|v| v.as_sequence())
            .expect("manual proxies should be a sequence");

        assert_eq!(manual_proxies.len(), 2);
        assert!(manual_proxies.iter().any(|p| p.as_str() == Some("alive-node")));
        assert!(manual_proxies.iter().any(|p| p.as_str() == Some("DIRECT")));

        let nested_group = groups
            .iter()
            .find(|group| group.get("name").and_then(serde_yaml_ng::Value::as_str) == Some("nested"))
            .and_then(|group| group.as_mapping())
            .expect("nested group should exist");

        let nested_proxies = nested_group
            .get("proxies")
            .and_then(|v| v.as_sequence())
            .expect("nested proxies should be a sequence");

        assert_eq!(nested_proxies.len(), 1);
        assert_eq!(nested_proxies[0].as_str(), Some("manual"));
    }

    #[test]
    fn keep_provider_backed_groups_intact() {
        let config_str = r#"
proxy-providers:
  providerA:
    type: http
    url: https://example.com
    path: ./providerA.yaml
proxies: []
proxy-groups:
  - name: "manual"
    type: select
    use:
      - "providerA"
      - "ghostProvider"
    proxies:
      - "dynamic-node"
      - "DIRECT"
"#;

        let mut config: serde_yaml_ng::Mapping =
            serde_yaml_ng::from_str(config_str).expect("Failed to parse test yaml");
        config = cleanup_proxy_groups(config);

        let groups = config
            .get("proxy-groups")
            .and_then(|v| v.as_sequence())
            .cloned()
            .expect("proxy-groups should be a sequence");

        let manual_group = groups
            .iter()
            .find(|group| group.get("name").and_then(serde_yaml_ng::Value::as_str) == Some("manual"))
            .and_then(|group| group.as_mapping())
            .expect("manual group should exist");

        let uses = manual_group
            .get("use")
            .and_then(|v| v.as_sequence())
            .expect("use should be a sequence");
        assert_eq!(uses.len(), 1);
        assert_eq!(uses[0].as_str(), Some("providerA"));

        let proxies = manual_group
            .get("proxies")
            .and_then(|v| v.as_sequence())
            .expect("proxies should be a sequence");
        assert_eq!(proxies.len(), 2);
        assert!(proxies.iter().any(|p| p.as_str() == Some("dynamic-node")));
        assert!(proxies.iter().any(|p| p.as_str() == Some("DIRECT")));
    }

    #[test]
    fn prune_invalid_provider_and_proxies_without_provider() {
        let config_str = r#"
proxy-groups:
  - name: "manual"
    type: select
    use:
      - "ghost-provider"
    proxies:
      - "ghost-node"
      - "DIRECT"
"#;

        let mut config: serde_yaml_ng::Mapping =
            serde_yaml_ng::from_str(config_str).expect("Failed to parse test yaml");
        config = cleanup_proxy_groups(config);

        let groups = config
            .get("proxy-groups")
            .and_then(|v| v.as_sequence())
            .cloned()
            .expect("proxy-groups should be a sequence");

        let manual_group = groups
            .iter()
            .find(|group| group.get("name").and_then(serde_yaml_ng::Value::as_str) == Some("manual"))
            .and_then(|group| group.as_mapping())
            .expect("manual group should exist");

        let uses = manual_group
            .get("use")
            .and_then(|v| v.as_sequence())
            .expect("use should be a sequence");
        assert_eq!(uses.len(), 0);

        let proxies = manual_group
            .get("proxies")
            .and_then(|v| v.as_sequence())
            .expect("proxies should be a sequence");
        assert_eq!(proxies.len(), 1);
        assert_eq!(proxies[0].as_str(), Some("DIRECT"));
    }

    #[test]
    fn append_standard_region_pool_from_static_proxies() {
        let config_str = r#"
proxies:
  - name: "台湾-01"
    type: ss
  - name: "香港-01"
    type: ss
proxy-groups: []
"#;

        let config: serde_yaml_ng::Mapping =
            serde_yaml_ng::from_str(config_str).expect("Failed to parse test yaml");
        let config = append_standard_region_pools(config);

        let groups = config
            .get("proxy-groups")
            .and_then(|v| v.as_sequence())
            .cloned()
            .expect("proxy-groups should be a sequence");

        let tw_group = groups
            .iter()
            .find(|group| {
                group.get("name").and_then(serde_yaml_ng::Value::as_str)
                    == Some("VERGE-REGION-TW-AUTO")
            })
            .and_then(|group| group.as_mapping())
            .expect("TW region pool should exist");

        let proxies = tw_group
            .get("proxies")
            .and_then(|v| v.as_sequence())
            .expect("TW region pool proxies should exist");

        assert_eq!(proxies.len(), 1);
        assert_eq!(proxies[0].as_str(), Some("台湾-01"));
        assert_eq!(tw_group.get("hidden").and_then(serde_yaml_ng::Value::as_bool), Some(true));
    }

    #[test]
    fn append_standard_region_pool_from_providers() {
        let config_str = r#"
proxy-providers:
  providerA:
    type: http
    url: https://example.com
    path: ./providerA.yaml
proxy-groups: []
"#;

        let config: serde_yaml_ng::Mapping =
            serde_yaml_ng::from_str(config_str).expect("Failed to parse test yaml");
        let config = append_standard_region_pools(config);

        let groups = config
            .get("proxy-groups")
            .and_then(|v| v.as_sequence())
            .cloned()
            .expect("proxy-groups should be a sequence");

        let tw_group = groups
            .iter()
            .find(|group| {
                group.get("name").and_then(serde_yaml_ng::Value::as_str)
                    == Some("VERGE-REGION-TW-AUTO")
            })
            .and_then(|group| group.as_mapping())
            .expect("TW region pool should exist");

        let uses = tw_group
            .get("use")
            .and_then(|v| v.as_sequence())
            .expect("TW region pool use should exist");

        assert_eq!(uses.len(), 1);
        assert_eq!(uses[0].as_str(), Some("providerA"));
        assert!(tw_group.get("filter").and_then(serde_yaml_ng::Value::as_str).is_some());
    }

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

        let config: serde_yaml_ng::Mapping =
            serde_yaml_ng::from_str(config_str).expect("Failed to parse test yaml");
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
            profile
                .get("store-selected")
                .and_then(serde_yaml_ng::Value::as_bool),
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
                group.get("name").and_then(serde_yaml_ng::Value::as_str)
                    == Some("VERGE-STABLE-STAR-OPENAI-COM")
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
        assert_eq!(
            high_risk_policy[0].as_str(),
            Some("https://dns.google/dns-query")
        );
    }
}
