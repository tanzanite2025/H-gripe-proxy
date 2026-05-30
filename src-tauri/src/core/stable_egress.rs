use std::collections::{HashMap, HashSet};
use std::net::IpAddr;

use anyhow::Result;
use serde_yaml_ng::{Mapping, Value};

use crate::core::{
    coordinator_status::{
        CoordinatorBindingInfo, CoordinatorResolvedEgressIdentity, CoordinatorRuntimeState,
        StableEgressBackwriteStatus,
    },
    egress_identity::{EgressNodeMetadata, EgressSelectionContext, ResolvedEgressIdentity},
    handle,
    ip_reputation::IpReputationManager,
    session_affinity::{BindingInfo, SessionAffinityManager},
};
use crate::core::coordinator::CoreCoordinator;
use crate::multipath::MultipathManager;

pub const STABLE_EGRESS_GROUP_PREFIX: &str = "VERGE-STABLE-";

pub fn domain_probe_for_pattern(pattern: &str) -> Option<String> {
    let normalized = pattern
        .strip_prefix("*.")
        .or_else(|| pattern.strip_prefix('*'))
        .unwrap_or(pattern)
        .trim_start_matches('.')
        .trim();

    if normalized.is_empty() {
        None
    } else {
        Some(normalized.to_string())
    }
}

pub fn stable_egress_group_name(pattern: &str) -> String {
    let slug = pattern
        .chars()
        .flat_map(|ch| {
            if ch.is_ascii_alphanumeric() {
                vec![ch.to_ascii_uppercase()]
            } else if ch == '*' {
                vec!['-', 'S', 'T', 'A', 'R', '-']
            } else {
                vec!['-']
            }
        })
        .collect::<String>();
    let slug = slug
        .split('-')
        .filter(|segment| !segment.is_empty())
        .collect::<Vec<_>>()
        .join("-");

    if slug.is_empty() {
        format!("{STABLE_EGRESS_GROUP_PREFIX}DOMAIN")
    } else {
        format!("{STABLE_EGRESS_GROUP_PREFIX}{slug}")
    }
}

pub fn stable_egress_rule_line(pattern: &str, group_name: &str) -> Option<String> {
    if let Some(suffix) = pattern.strip_prefix("*.").or_else(|| pattern.strip_prefix('*')) {
        let suffix = suffix.trim_start_matches('.').trim();
        if suffix.is_empty() {
            None
        } else {
            Some(format!("DOMAIN-SUFFIX,{suffix},{group_name}"))
        }
    } else if pattern.contains('*') {
        None
    } else {
        let domain = pattern.trim();
        if domain.is_empty() {
            None
        } else {
            Some(format!("DOMAIN,{domain},{group_name}"))
        }
    }
}

pub fn parse_stable_rule_line(rule: &str) -> Option<(String, String)> {
    let mut segments = rule.split(',');
    let rule_type = segments.next()?.trim();
    let raw_pattern = segments.next()?.trim();
    let group_name = segments.next()?.trim();

    if !group_name.starts_with(STABLE_EGRESS_GROUP_PREFIX) {
        return None;
    }

    match rule_type {
        "DOMAIN-SUFFIX" => Some((group_name.to_string(), format!("*.{}", raw_pattern))),
        "DOMAIN" => Some((group_name.to_string(), raw_pattern.to_string())),
        _ => None,
    }
}

pub fn collect_stable_group_patterns(config: &Mapping) -> HashMap<String, Vec<String>> {
    let mut group_patterns = HashMap::<String, Vec<String>>::new();

    let Some(rules) = config.get("rules").and_then(Value::as_sequence) else {
        return group_patterns;
    };

    for rule in rules.iter().filter_map(Value::as_str) {
        let Some((group_name, domain_pattern)) = parse_stable_rule_line(rule) else {
            continue;
        };

        let patterns = group_patterns.entry(group_name).or_default();
        if !patterns.iter().any(|pattern| pattern == &domain_pattern) {
            patterns.push(domain_pattern);
        }
    }

    group_patterns
}

pub async fn project_runtime_status(
    egress_identity_assignments: Vec<ResolvedEgressIdentity>,
    session_affinity_bindings: Vec<BindingInfo>,
) -> CoordinatorRuntimeState {
    let source_group_selected_nodes = source_group_selected_nodes().await;

    let egress_identity_assignments = egress_identity_assignments
        .into_iter()
        .map(|assignment| project_assignment(assignment, &source_group_selected_nodes))
        .collect::<Vec<_>>();
    let session_affinity_bindings = session_affinity_bindings
        .into_iter()
        .map(|binding| project_binding(binding, &source_group_selected_nodes))
        .collect::<Vec<_>>();

    let domain_pattern_assignments = egress_identity_assignments
        .iter()
        .filter(|assignment| is_domain_pattern_assignment(assignment))
        .cloned()
        .collect::<Vec<_>>();
    let domain_rule_bindings = session_affinity_bindings
        .iter()
        .filter(|binding| is_domain_rule_binding(binding))
        .cloned()
        .collect::<Vec<_>>();

    CoordinatorRuntimeState {
        egress_identity_assignments,
        session_affinity_bindings,
        stable_egress_backwrite: StableEgressBackwriteStatus {
            domain_pattern_assignments,
            domain_rule_bindings,
        },
    }
}

async fn source_group_selected_nodes() -> HashMap<String, String> {
    handle::Handle::mihomo()
        .await
        .get_proxies()
        .await
        .ok()
        .map(|proxies| {
            proxies
                .proxies
                .into_iter()
                .filter_map(|(group_name, group_data)| {
                    group_data
                        .now
                        .as_ref()
                        .map(|value| value.trim())
                        .filter(|value| !value.is_empty())
                        .map(|value| (group_name, value.to_string()))
                })
                .collect::<HashMap<_, _>>()
        })
        .unwrap_or_default()
}

fn project_assignment(
    assignment: ResolvedEgressIdentity,
    source_group_selected_nodes: &HashMap<String, String>,
) -> CoordinatorResolvedEgressIdentity {
    let source_group_name = source_group_name_for_assignment(&assignment);
    let source_group_selected_node = source_group_name
        .as_ref()
        .and_then(|group_name| source_group_selected_nodes.get(group_name).cloned());

    CoordinatorResolvedEgressIdentity {
        assignment_key: assignment.assignment_key,
        profile_id: assignment.profile_id,
        selected_node: assignment.selected_node,
        dns_mode: assignment.dns_mode,
        tls_fingerprint: assignment.tls_fingerprint,
        matched_by: assignment.matched_by,
        source_group_name,
        source_group_selected_node,
    }
}

fn project_binding(
    binding: BindingInfo,
    source_group_selected_nodes: &HashMap<String, String>,
) -> CoordinatorBindingInfo {
    let source_group_name = source_group_name_for_binding(&binding);
    let source_group_selected_node = source_group_name
        .as_ref()
        .and_then(|group_name| source_group_selected_nodes.get(group_name).cloned());

    CoordinatorBindingInfo {
        binding_type: binding.binding_type,
        key: binding.key,
        node_id: binding.node_id,
        bound_at: binding.bound_at,
        expires_at: binding.expires_at,
        remaining_seconds: binding.remaining_seconds,
        source_group_name,
        source_group_selected_node,
    }
}

fn source_group_name_for_assignment(assignment: &ResolvedEgressIdentity) -> Option<String> {
    let domain_pattern = assignment
        .assignment_key
        .as_deref()?
        .strip_prefix("domain-pattern:")?;
    Some(stable_egress_group_name(domain_pattern))
}

fn source_group_name_for_binding(binding: &BindingInfo) -> Option<String> {
    if !is_domain_rule_runtime_binding(binding) {
        return None;
    }

    let domain_pattern = binding.key.strip_prefix("rule:")?;
    Some(stable_egress_group_name(domain_pattern))
}

fn is_domain_rule_runtime_binding(binding: &BindingInfo) -> bool {
    binding.binding_type == "domain-rule"
}

fn is_domain_pattern_assignment(assignment: &CoordinatorResolvedEgressIdentity) -> bool {
    assignment
        .assignment_key
        .as_deref()
        .map(|key| key.starts_with("domain-pattern:"))
        .unwrap_or(false)
}

fn is_domain_rule_binding(binding: &CoordinatorBindingInfo) -> bool {
    binding.binding_type == "domain-rule"
}

// ── 出口选择上下文增强 ────────────────────────────────────────────────

pub async fn resolve_server_ip(server: &str) -> Option<String> {
    if let Ok(ip_addr) = server.parse::<IpAddr>() {
        return Some(ip_addr.to_string());
    }

    let resolved = tokio::net::lookup_host((server, 0)).await.ok()?;
    let resolved_addresses = resolved.collect::<Vec<_>>();

    resolved_addresses
        .iter()
        .find(|socket_addr| matches!(socket_addr.ip(), IpAddr::V4(_)))
        .or_else(|| resolved_addresses.first())
        .map(|socket_addr| socket_addr.ip().to_string())
}

/// 丰富出口选择上下文：注入多路径元数据 + IP 信誉度
pub async fn enrich_egress_selection_context(
    mut ctx: EgressSelectionContext,
    multipath_manager: &MultipathManager,
    ip_reputation_manager: &IpReputationManager,
) -> EgressSelectionContext {
    let multipath_config = multipath_manager.get_config();
    let requested_nodes = if ctx.available_nodes.is_empty() {
        None
    } else {
        Some(ctx.available_nodes.iter().cloned().collect::<HashSet<_>>())
    };

    let mut metadata_index = HashMap::<String, EgressNodeMetadata>::new();
    let mut ordered_nodes = Vec::<String>::new();

    for pool in multipath_config.node_pools.iter().filter(|pool| pool.enabled) {
        for node in pool.nodes.iter().filter(|node| node.enabled) {
            let should_include = requested_nodes
                .as_ref()
                .map(|nodes| nodes.contains(&node.name))
                .unwrap_or(true);

            if !should_include {
                continue;
            }

            if !metadata_index.contains_key(&node.name) {
                ordered_nodes.push(node.name.clone());
            }

            metadata_index.entry(node.name.clone()).or_insert_with(|| EgressNodeMetadata {
                name: node.name.clone(),
                server: Some(node.server.clone()),
                pool_name: Some(pool.name.clone()),
                pool_type: Some(format!("{:?}", pool.pool_type)),
                ip_type: None,
                fraud_score: None,
            });
        }
    }

    if ctx.available_nodes.is_empty() {
        ctx.available_nodes = ordered_nodes;
    }

    for node_name in &ctx.available_nodes {
        metadata_index
            .entry(node_name.clone())
            .or_insert_with(|| EgressNodeMetadata {
                name: node_name.clone(),
                ..Default::default()
            });
    }

    for node_name in &ctx.available_nodes {
        if let Some(metadata) = metadata_index.get_mut(node_name) {
            if let Some(server) = metadata.server.clone() {
                if let Some(server_ip) = resolve_server_ip(&server).await {
                    match ip_reputation_manager.inspect_ip_metadata(&server_ip).await {
                        Ok(reputation) => {
                            metadata.ip_type = Some(reputation.ip_type);
                            metadata.fraud_score = Some(reputation.fraud_score);
                        }
                        Err(error) => {
                            log::warn!(
                                "[EgressIdentity] 检测节点 {} 的 IP 元数据失败: {}",
                                node_name,
                                error
                            );
                        }
                    }
                } else {
                    log::warn!(
                        "[EgressIdentity] 无法解析节点 {} 的 server 地址 {}",
                        node_name,
                        server
                    );
                }
            }
        }
    }

    ctx.available_node_metadata = ctx
        .available_nodes
        .iter()
        .filter_map(|node_name| metadata_index.get(node_name).cloned())
        .collect::<Vec<_>>();

    ctx
}

// ── 稳定出口运行态同步 ────────────────────────────────────────────────

fn with_selected_node(mut available_nodes: Vec<String>, selected_node: &str) -> Vec<String> {
    if !available_nodes.iter().any(|node| node == selected_node) {
        available_nodes.insert(0, selected_node.to_string());
    }
    available_nodes
}

/// 同步 VERGE-STABLE-* 组的选中节点到 egress_identity 和 session_affinity 的运行态
pub async fn sync_runtime_stable_egress_selection(
    coordinator: &CoreCoordinator,
    session_affinity_manager: &SessionAffinityManager,
    ip_reputation_manager: &IpReputationManager,
    runtime_config: &Mapping,
) -> Result<()> {
    let stable_group_patterns = collect_stable_group_patterns(runtime_config);
    if stable_group_patterns.is_empty() {
        return Ok(());
    }

    let proxies = handle::Handle::mihomo()
        .await
        .get_proxies()
        .await
        .map_err(|e| anyhow::anyhow!("获取代理组失败: {}", e))?;

    let egress_identity_manager = coordinator.egress_identity_manager();
    let multipath_manager = coordinator.multipath_manager();

    for (group_name, domain_patterns) in &stable_group_patterns {
        let Some(group_data) = proxies.proxies.get(group_name.as_str()) else {
            continue;
        };

        let Some(selected_node) = group_data
            .now
            .as_ref()
            .map(|value| value.trim())
            .filter(|value| !value.is_empty())
            .map(|value| value.to_string())
        else {
            continue;
        };

        let available_nodes = with_selected_node(
            group_data.all.clone().unwrap_or_default(),
            &selected_node,
        );

        if available_nodes.is_empty() {
            continue;
        }

        for domain_pattern in domain_patterns {
            let Some(domain_probe) = domain_probe_for_pattern(domain_pattern) else {
                continue;
            };

            let egress_context = enrich_egress_selection_context(
                EgressSelectionContext {
                    domain: Some(domain_probe),
                    available_nodes: available_nodes.clone(),
                    ..Default::default()
                },
                &multipath_manager,
                ip_reputation_manager,
            )
            .await;

            if let Err(error) = egress_identity_manager.record_domain_override(
                domain_pattern,
                egress_context,
                selected_node.clone(),
            ) {
                log::warn!(
                    "Failed to backwrite stable egress selection into egress identity for {} -> {}: {}",
                    domain_pattern,
                    selected_node,
                    error
                );
            }

            if let Err(error) = session_affinity_manager
                .record_domain_rule_binding(domain_pattern, selected_node.clone())
                .await
            {
                log::warn!(
                    "Failed to backwrite stable egress selection into session affinity for {} -> {}: {}",
                    domain_pattern,
                    selected_node,
                    error
                );
            }
        }
    }

    Ok(())
}
