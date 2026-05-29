use crate::core::{
    coordinator_status::{
        CoordinatorBindingInfo, CoordinatorResolvedEgressIdentity, CoordinatorRuntimeState,
        StableEgressBackwriteStatus,
    },
    egress_identity::ResolvedEgressIdentity,
    handle,
    session_affinity::BindingInfo,
};
use serde_yaml_ng::{Mapping, Value};
use std::collections::HashMap;

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
