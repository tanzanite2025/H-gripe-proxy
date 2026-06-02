/**
 * 稳定出口策略注入
 *
 * 读取 advanced.yaml 中的 session_affinity + egress_identity 配置，
 * 为高风险域名生成 VERGE-STABLE-* select 组 + DOMAIN-SUFFIX 规则 + DNS nameserver-policy。
 */

use crate::core::egress_identity::{
    DnsMode, EgressIdentityManager, EgressNodeMetadata, EgressSelectionContext,
    ResolvedEgressIdentity,
};
use crate::core::session_affinity::{DomainBindingRule, FallbackPolicy};
use crate::core::stable_egress::{
    STABLE_EGRESS_GROUP_PREFIX, domain_probe_for_pattern, stable_egress_group_name,
    stable_egress_rule_line,
};
use crate::config::AdvancedConfig;
use serde_yaml_ng::{Mapping, Sequence, Value};
use std::collections::HashSet;

#[derive(Debug, Clone)]
struct StaticProxySpec {
    name: std::string::String,
    server: Option<std::string::String>,
}

fn load_advanced_config_for_stable_egress() -> AdvancedConfig {
    crate::feat::get_coordinator().get_advanced_config()
}

pub(crate) fn apply_stable_egress_policy(config: Mapping) -> Mapping {
    let advanced_config = load_advanced_config_for_stable_egress();
    apply_stable_egress_policy_with_advanced(config, &advanced_config)
}

pub(crate) fn apply_stable_egress_policy_with_advanced(
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

        let (ordered_nodes, resolved_identity) = resolve_stable_egress_ordered_nodes(
            &egress_manager,
            &rule,
            &static_proxy_names,
            &metadata,
            &provider_names,
        );

        if generated_group_names.insert(group_name.clone()) {
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
            && let Some(identity) = resolved_identity.as_ref()
            && let Some(nameservers) =
                stable_dns_server_override(&config, advanced_config, identity)
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

    // 注入住宅链式代理
    config = apply_residential_chain_proxies(config, advanced_config);
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

fn resolve_stable_egress_ordered_nodes(
    manager: &EgressIdentityManager,
    rule: &DomainBindingRule,
    static_proxy_names: &[std::string::String],
    metadata: &[EgressNodeMetadata],
    provider_names: &[std::string::String],
) -> (Vec<std::string::String>, Option<ResolvedEgressIdentity>) {
    let resolved_identity =
        preview_stable_egress_identity(manager, rule, static_proxy_names, metadata);

    let mut ordered_nodes = static_proxy_names.to_vec();

    if let Some(identity) = resolved_identity.as_ref() {
        ordered_nodes = prioritize_node_names(
            ordered_nodes,
            &identity.selected_node,
            !provider_names.is_empty(),
        );
    }

    if let Some(bound_node) = rule.bound_node.as_ref() {
        ordered_nodes = prioritize_node_names(ordered_nodes, bound_node, !provider_names.is_empty());
    }

    ordered_nodes = dedupe_node_names(ordered_nodes);

    (ordered_nodes, resolved_identity)
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

/// 住宅链式代理前缀
const RESIDENTIAL_PROXY_PREFIX: &str = "VERGE-RES-";
/// 链式代理组前缀
const CHAIN_GROUP_PREFIX: &str = "VERGE-CHAIN-";
/// 住宅代理验证专用组
const RESIDENTIAL_VERIFY_GROUP: &str = "VERGE-RES-VERIFY";

/// 注入住宅链式代理
///
/// 当 egress_identity profile 设置了 use_residential_chain=true 且
/// residential_pool 有可用节点时，为对应的 VERGE-STABLE-* 组中的
/// 前置节点注入 dialer-proxy，构建 VPS→住宅 链式代理。
fn apply_residential_chain_proxies(mut config: Mapping, advanced_config: &AdvancedConfig) -> Mapping {
    let pool = &advanced_config.residential_pool;
    if !pool.enabled {
        return config;
    }

    let enabled_proxies = pool.enabled_proxies();
    if enabled_proxies.is_empty() {
        return config;
    }

    // 1. 注入住宅代理节点到 proxies 段
    let residential_mappings = pool.to_mihomo_proxy_mappings();
    let mut existing_proxies = config
        .get("proxies")
        .and_then(Value::as_sequence)
        .cloned()
        .unwrap_or_default();

    // 移除旧的住宅代理定义（避免重复）
    existing_proxies.retain(|proxy| {
        proxy
            .get("name")
            .and_then(Value::as_str)
            .map(|name| !name.starts_with(RESIDENTIAL_PROXY_PREFIX))
            .unwrap_or(true)
    });

    for (_, mapping) in &residential_mappings {
        existing_proxies.push(Value::Mapping(mapping.clone()));
    }
    config.insert("proxies".into(), Value::Sequence(existing_proxies));

    // 2. 维护住宅验证专用组，避免验证时借用 GLOBAL 影响用户当前选择
    let residential_names: Vec<Value> = residential_mappings
        .iter()
        .map(|(name, _)| Value::from(name.as_str()))
        .collect();

    // 3. 为每个需要链式路由的 profile，在 VERGE-STABLE-* 组中
    //    给前置节点添加 dialer-proxy 指向住宅出口
    let mut proxy_groups = config
        .get("proxy-groups")
        .and_then(Value::as_sequence)
        .cloned()
        .unwrap_or_default();

    // 移除旧的链式代理组
    proxy_groups.retain(|group| {
        group
            .get("name")
            .and_then(Value::as_str)
            .map(|name| !name.starts_with(CHAIN_GROUP_PREFIX) && name != RESIDENTIAL_VERIFY_GROUP)
            .unwrap_or(true)
    });

    let mut verify_group = Mapping::new();
    verify_group.insert("name".into(), Value::from(RESIDENTIAL_VERIFY_GROUP));
    verify_group.insert("type".into(), Value::from("select"));
    verify_group.insert("proxies".into(), Value::Sequence(residential_names));
    proxy_groups.push(Value::Mapping(verify_group));

    // 收集需要链式住宅路由的 profile
    let chain_profiles: Vec<_> = advanced_config
        .egress_identity
        .profiles
        .iter()
        .filter(|p| p.enabled && p.use_residential_chain)
        .collect();

    if chain_profiles.is_empty() {
        config.insert("proxy-groups".into(), Value::Sequence(proxy_groups));
        return config;
    }

    // 为每个 chain profile 创建链式代理组
    let mut new_chain_groups = Sequence::new();
    for profile in &chain_profiles {
        // 选择住宅代理：优先使用指定的，否则选第一个
        let residential_name = profile
            .residential_proxy_name
            .as_deref()
            .and_then(|name| pool.get_by_name(name))
            .map(|p| format!("{}{}", RESIDENTIAL_PROXY_PREFIX, p.name))
            .or_else(|| {
                enabled_proxies.first().map(|p| {
                    format!("{}{}", RESIDENTIAL_PROXY_PREFIX, p.name)
                })
            });

        let Some(res_name) = residential_name else {
            continue;
        };

        // 查找此 profile 绑定的 stable group
        let app_rules: Vec<_> = advanced_config
            .egress_identity
            .app_rules
            .iter()
            .filter(|r| r.enabled && r.profile_id == profile.id)
            .collect();

        for rule in app_rules {
            for domain in &rule.domains {
                let stable_group = stable_egress_group_name(domain);

                // 在 proxy-groups 中找到该 stable group
                if let Some(group) = proxy_groups.iter_mut().find(|g| {
                    g.get("name").and_then(Value::as_str) == Some(stable_group.as_str())
                }) {
                    // 给组中的每个前置节点创建链式代理组
                    if let Some(proxies_seq) = group.get_mut("proxies").and_then(Value::as_sequence_mut) {
                        let mut chain_proxies = Sequence::new();
                        for proxy_val in proxies_seq.iter() {
                            if let Some(node_name) = proxy_val.as_str() {
                                let chain_group_name = format!(
                                    "{}{}-via-{}",
                                    CHAIN_GROUP_PREFIX,
                                    node_name.replace(' ', "-"),
                                    res_name.trim_start_matches(RESIDENTIAL_PROXY_PREFIX)
                                );
                                chain_proxies.push(Value::from(chain_group_name.as_str()));

                                // 创建链式代理组：包含原节点，但设置 dialer-proxy
                                let mut chain_group = Mapping::new();
                                chain_group.insert("name".into(), Value::from(chain_group_name.as_str()));
                                chain_group.insert("type".into(), Value::from("select"));
                                chain_group.insert(
                                    "proxies".into(),
                                    Value::Sequence(vec![Value::from(node_name)]),
                                );

                                // 注入 dialer-proxy 到原节点
                                // Mihomo 支持 per-proxy dialer-proxy 字段
                                inject_dialer_proxy_to_node(
                                    &mut config,
                                    node_name,
                                    &res_name,
                                );

                                new_chain_groups.push(Value::Mapping(chain_group));
                            }
                        }
                        // 替换原组的 proxies 为链式代理组名
                        *proxies_seq = chain_proxies;
                    }
                }
            }
        }
    }

    // 追加链式代理组
    proxy_groups.extend(new_chain_groups);
    config.insert("proxy-groups".into(), Value::Sequence(proxy_groups));
    config
}

/// 给 proxies 段中的指定节点注入 dialer-proxy 字段
fn inject_dialer_proxy_to_node(config: &mut Mapping, node_name: &str, dialer_proxy: &str) {
    if let Some(Value::Sequence(proxies)) = config.get_mut("proxies") {
        for proxy in proxies.iter_mut() {
            if let Value::Mapping(mapping) = proxy {
                if mapping.get("name").and_then(Value::as_str) == Some(node_name) {
                    mapping.insert("dialer-proxy".into(), Value::from(dialer_proxy));
                    break;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stable_dns_policy_key_wildcard_suffix() {
        assert_eq!(
            stable_dns_policy_key("*.openai.com"),
            Some("+.openai.com".to_string())
        );
    }

    #[test]
    fn test_stable_dns_policy_key_wildcard_prefix() {
        assert_eq!(
            stable_dns_policy_key("*openai.com"),
            Some("+.openai.com".to_string())
        );
    }

    #[test]
    fn test_stable_dns_policy_key_exact_domain() {
        assert_eq!(
            stable_dns_policy_key("example.com"),
            Some("example.com".to_string())
        );
    }

    #[test]
    fn test_residential_verify_group_is_injected_without_chain_profiles() {
        let mut advanced = AdvancedConfig::default();
        advanced.residential_pool.enabled = true;
        advanced.residential_pool.proxies.push(crate::config::ResidentialProxy {
            name: "US-1".to_string(),
            proxy_type: crate::config::ResidentialProxyType::Vmess,
            server: "127.0.0.1".to_string(),
            port: 10000,
            username: None,
            password: None,
            cipher: None,
            uuid: Some("00000000-0000-0000-0000-000000000000".to_string()),
            trojan_password: None,
            tls: None,
            sni: None,
            skip_cert_verify: None,
            region: Some("US".to_string()),
            enabled: true,
        });

        let config = apply_residential_chain_proxies(Mapping::new(), &advanced);
        let groups = config
            .get("proxy-groups")
            .and_then(Value::as_sequence)
            .expect("proxy-groups should exist");
        let verify_group = groups
            .iter()
            .find(|group| group.get("name").and_then(Value::as_str) == Some(RESIDENTIAL_VERIFY_GROUP))
            .expect("verify group should be injected");

        assert_eq!(
            verify_group.get("proxies").and_then(Value::as_sequence).unwrap(),
            &vec![Value::from("VERGE-RES-US-1")]
        );
    }

    #[test]
    fn test_stable_dns_policy_key_complex_wildcard() {
        assert_eq!(stable_dns_policy_key("*.sub.*.com"), None);
    }

    #[test]
    fn test_stable_dns_policy_key_empty() {
        assert_eq!(stable_dns_policy_key(""), None);
        assert_eq!(stable_dns_policy_key("*"), None);
        assert_eq!(stable_dns_policy_key("*."), None);
    }

    #[test]
    fn test_prioritize_node_names_existing() {
        let nodes = vec![
            "node-c".to_string(),
            "node-a".to_string(),
            "node-b".to_string(),
        ];
        let result = prioritize_node_names(nodes, "node-b", false);
        assert_eq!(result[0], "node-b");
        assert_eq!(result.len(), 3);
    }

    #[test]
    fn test_prioritize_node_names_missing_with_allow_insert() {
        let nodes = vec!["node-a".to_string(), "node-b".to_string()];
        let result = prioritize_node_names(nodes, "node-x", true);
        assert_eq!(result[0], "node-x");
        assert_eq!(result.len(), 3);
    }

    #[test]
    fn test_prioritize_node_names_missing_without_allow_insert() {
        let nodes = vec!["node-a".to_string(), "node-b".to_string()];
        let result = prioritize_node_names(nodes, "node-x", false);
        assert_eq!(result.len(), 2);
        assert!(!result.iter().any(|n| n == "node-x"));
    }

    #[test]
    fn test_prioritize_node_names_empty_preferred() {
        let nodes = vec!["node-a".to_string()];
        let result = prioritize_node_names(nodes, "", false);
        assert_eq!(result, vec!["node-a".to_string()]);
    }

    #[test]
    fn test_dedupe_node_names() {
        let nodes = vec![
            "Node-A".to_string(),
            "node-a".to_string(),
            "Node-B".to_string(),
        ];
        let result = dedupe_node_names(nodes);
        assert_eq!(result.len(), 2);
    }

    #[test]
    fn test_dedupe_string_sequence() {
        let seq = vec![
            "a".to_string(),
            "b".to_string(),
            "a".to_string(),
            "c".to_string(),
        ];
        let result = dedupe_string_sequence(seq);
        assert_eq!(result, vec!["a", "b", "c"]);
    }

    #[test]
    fn test_first_non_empty_string_sequence() {
        let empty: Vec<std::string::String> = vec![];
        let non_empty = vec!["x".to_string()];
        assert_eq!(
            first_non_empty_string_sequence([empty.clone(), non_empty.clone()]),
            Some(vec!["x".to_string()])
        );
        assert_eq!(
            first_non_empty_string_sequence([empty.clone(), empty]),
            None
        );
    }

    #[test]
    fn test_collect_static_proxy_specs() {
        let yaml = r#"
proxies:
  - name: "node-a"
    type: ss
    server: 1.2.3.4
  - name: "node-b"
    type: vmess
"#;
        let config: Mapping = serde_yaml_ng::from_str(yaml).unwrap();
        let specs = collect_static_proxy_specs(&config);
        assert_eq!(specs.len(), 2);
        assert_eq!(specs[0].name, "node-a");
        assert_eq!(specs[0].server, Some("1.2.3.4".to_string()));
        assert_eq!(specs[1].name, "node-b");
        assert_eq!(specs[1].server, None);
    }

    #[test]
    fn test_collect_provider_names() {
        let yaml = r#"
proxy-providers:
  providerA:
    type: http
  providerB:
    type: file
"#;
        let config: Mapping = serde_yaml_ng::from_str(yaml).unwrap();
        let names = collect_provider_names(&config);
        assert_eq!(names.len(), 2);
        assert!(names.contains(&"providerA".to_string()));
        assert!(names.contains(&"providerB".to_string()));
    }
}
