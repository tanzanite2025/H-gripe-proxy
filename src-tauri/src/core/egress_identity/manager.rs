use std::collections::{HashMap, HashSet};

use anyhow::{Result, anyhow};
use parking_lot::RwLock;
use serde::Serialize;

use crate::core::ip_reputation::{IpType, matches_ip_type};
use crate::core::session_affinity::domain_matches;

use super::config::*;

#[derive(Debug, Clone, Default)]
pub struct EgressSelectionContext {
    pub shortcut_id: Option<String>,
    pub process_name: Option<String>,
    pub exe_path: Option<String>,
    pub domain: Option<String>,
    pub source_ip: Option<String>,
    pub source_port: Option<u16>,
    pub available_nodes: Vec<String>,
    pub available_node_metadata: Vec<EgressNodeMetadata>,
}

#[derive(Debug, Clone, Default)]
pub struct EgressNodeMetadata {
    pub name: String,
    pub server: Option<String>,
    pub pool_name: Option<String>,
    pub pool_type: Option<String>,
    pub ip_type: Option<IpType>,
    pub fraud_score: Option<u8>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ResolvedEgressIdentity {
    pub assignment_key: Option<String>,
    pub profile_id: String,
    pub selected_node: String,
    pub dns_mode: DnsMode,
    pub tls_fingerprint: Option<String>,
    pub matched_by: String,
}

#[derive(Debug, Clone)]
struct SelectionPlan {
    profile: EgressIdentityProfile,
    matched_by: String,
    candidate_nodes: Vec<String>,
    fallback_nodes: Vec<String>,
    domain_override_node: Option<String>,
}

impl SelectionPlan {
    fn select_node(&self) -> Result<String> {
        if let Some(candidate) = self.candidate_nodes.first() {
            return Ok(candidate.clone());
        }

        // 当启用 strict_node_scope 时，严格限制在候选集合内选择，不再允许回退到其他节点
        if self.profile.strict_node_scope {
            return Err(anyhow!(
                "出口身份画像 {} 没有满足 strict_node_scope 约束的候选节点",
                self.profile.id
            ));
        }

        match &self.profile.failover_policy {
            EgressFailoverPolicy::Block => Err(anyhow!("出口身份画像 {} 没有满足约束的候选节点", self.profile.id)),
            EgressFailoverPolicy::Manual | EgressFailoverPolicy::AutoSwitch => self
                .fallback_nodes
                .first()
                .cloned()
                .ok_or_else(|| anyhow!("没有可用节点")),
        }
    }

    fn allows_node(&self, node: &str) -> bool {
        if self.candidate_nodes.iter().any(|candidate| candidate == node) {
            return true;
        }

        if matches!(&self.profile.failover_policy, EgressFailoverPolicy::Block) {
            return false;
        }

        self.fallback_nodes.iter().any(|candidate| candidate == node)
    }
}

pub struct EgressIdentityManager {
    config: RwLock<EgressIdentityConfig>,
    active_assignments: RwLock<HashMap<String, ResolvedEgressIdentity>>,
    domain_overrides: RwLock<HashMap<String, String>>,
}

impl EgressIdentityManager {
    pub fn new() -> Self {
        Self::new_with_config(EgressIdentityConfig::default())
    }

    pub fn new_with_config(config: EgressIdentityConfig) -> Self {
        Self {
            config: RwLock::new(config),
            active_assignments: RwLock::new(HashMap::new()),
            domain_overrides: RwLock::new(HashMap::new()),
        }
    }

    pub fn get_config(&self) -> EgressIdentityConfig {
        self.config.read().clone()
    }

    pub fn update_config(&self, config: EgressIdentityConfig) -> Result<()> {
        config.validate()?;
        *self.config.write() = config.clone();

        let valid_profile_ids = config
            .profiles
            .into_iter()
            .filter(|profile| profile.enabled)
            .map(|profile| profile.id)
            .collect::<HashSet<_>>();

        self.active_assignments
            .write()
            .retain(|_, assignment| valid_profile_ids.contains(&assignment.profile_id));

        Ok(())
    }

    pub fn preview_match(&self, ctx: EgressSelectionContext) -> Result<ResolvedEgressIdentity> {
        self.resolve_internal(&ctx, None)
    }

    pub fn assign(&self, ctx: EgressSelectionContext) -> Result<ResolvedEgressIdentity> {
        let assignment_key = Self::assignment_key_for(&ctx);
        let config = self.config.read().clone();

        if !config.enabled {
            return Err(anyhow!("出口身份管理未启用"));
        }

        let plan = self.build_selection_plan(&config, &ctx)?;

        if let Some(existing) = self.active_assignments.read().get(&assignment_key).cloned() {
            let node_is_available =
                ctx.available_nodes.is_empty() || ctx.available_nodes.contains(&existing.selected_node);
            let override_requires_switch = plan
                .domain_override_node
                .as_ref()
                .map(|override_node| !existing.selected_node.eq_ignore_ascii_case(override_node))
                .unwrap_or(false);

            if node_is_available && plan.allows_node(&existing.selected_node) && !override_requires_switch {
                return Ok(existing);
            }
        }

        let selected_node = plan.select_node()?;

        let resolved = ResolvedEgressIdentity {
            assignment_key: Some(assignment_key.clone()),
            profile_id: plan.profile.id,
            selected_node,
            dns_mode: plan.profile.dns_policy.mode,
            tls_fingerprint: plan.profile.tls_fingerprint,
            matched_by: plan.matched_by,
        };
        self.active_assignments.write().insert(assignment_key, resolved.clone());
        Ok(resolved)
    }

    pub fn record_assignment(
        &self,
        ctx: EgressSelectionContext,
        selected_node: String,
    ) -> Result<ResolvedEgressIdentity> {
        let config = self.config.read().clone();

        if !config.enabled {
            return Err(anyhow!("出口身份管理未启用"));
        }

        let assignment_key = Self::assignment_key_for(&ctx);
        let plan = self.build_selection_plan(&config, &ctx)?;

        if !plan.allows_node(&selected_node) {
            return Err(anyhow!(
                "节点 {} 不满足出口身份画像 {} 的当前约束",
                selected_node,
                plan.profile.id
            ));
        }

        let resolved = ResolvedEgressIdentity {
            assignment_key: Some(assignment_key.clone()),
            profile_id: plan.profile.id,
            selected_node,
            dns_mode: plan.profile.dns_policy.mode,
            tls_fingerprint: plan.profile.tls_fingerprint,
            matched_by: plan.matched_by,
        };

        self.active_assignments.write().insert(assignment_key, resolved.clone());

        Ok(resolved)
    }

    pub fn record_domain_override(
        &self,
        domain_pattern: &str,
        ctx: EgressSelectionContext,
        selected_node: String,
    ) -> Result<ResolvedEgressIdentity> {
        let config = self.config.read().clone();

        if !config.enabled {
            return Err(anyhow!("出口身份管理未启用"));
        }

        let plan = self.build_selection_plan(&config, &ctx)?;

        if !plan.allows_node(&selected_node) {
            return Err(anyhow!(
                "节点 {} 不满足出口身份画像 {} 的当前约束",
                selected_node,
                plan.profile.id
            ));
        }

        self.domain_overrides
            .write()
            .insert(domain_pattern.to_string(), selected_node.clone());

        let assignment_key = format!("domain-pattern:{domain_pattern}");
        let resolved = ResolvedEgressIdentity {
            assignment_key: Some(assignment_key.clone()),
            profile_id: plan.profile.id,
            selected_node,
            dns_mode: plan.profile.dns_policy.mode,
            tls_fingerprint: plan.profile.tls_fingerprint,
            matched_by: format!("manual_group_selection:{domain_pattern}"),
        };

        self.active_assignments.write().insert(assignment_key, resolved.clone());

        Ok(resolved)
    }

    pub fn get_active_assignments(&self) -> Vec<ResolvedEgressIdentity> {
        let mut assignments = self.active_assignments.read().values().cloned().collect::<Vec<_>>();

        assignments.sort_by(|left, right| {
            left.assignment_key
                .as_deref()
                .unwrap_or_default()
                .cmp(right.assignment_key.as_deref().unwrap_or_default())
        });

        assignments
    }

    pub fn clear_assignment(&self, key: &str) {
        if let Some(domain_pattern) = key.strip_prefix("domain-pattern:") {
            self.domain_overrides.write().remove(domain_pattern);
            self.active_assignments.write().retain(|assignment_key, _| {
                if assignment_key == key {
                    return false;
                }

                assignment_key
                    .strip_prefix("domain:")
                    .map(|domain| !domain_matches(domain, domain_pattern))
                    .unwrap_or(true)
            });
            return;
        }

        self.active_assignments.write().remove(key);
    }

    fn resolve_internal(
        &self,
        ctx: &EgressSelectionContext,
        assignment_key: Option<String>,
    ) -> Result<ResolvedEgressIdentity> {
        let config = self.config.read().clone();

        if !config.enabled {
            return Err(anyhow!("出口身份管理未启用"));
        }

        let plan = self.build_selection_plan(&config, ctx)?;
        let selected_node = plan.select_node()?;

        Ok(ResolvedEgressIdentity {
            assignment_key,
            profile_id: plan.profile.id,
            selected_node,
            dns_mode: plan.profile.dns_policy.mode,
            tls_fingerprint: plan.profile.tls_fingerprint,
            matched_by: plan.matched_by,
        })
    }

    fn build_selection_plan(
        &self,
        config: &EgressIdentityConfig,
        ctx: &EgressSelectionContext,
    ) -> Result<SelectionPlan> {
        let (profile, matched_by) = self.match_profile(config, ctx)?;
        let metadata_index = Self::build_metadata_index(ctx);
        let mut candidate_nodes = self.filter_candidates(&profile, ctx, &metadata_index);
        let mut fallback_nodes = self.fallback_candidates(&profile, ctx, &metadata_index);
        let domain_override = self.resolve_domain_override(ctx, &candidate_nodes, &fallback_nodes);
        let matched_by = if let Some((pattern, node)) = domain_override.as_ref() {
            candidate_nodes = Self::prioritize_node(candidate_nodes, node);
            fallback_nodes = Self::prioritize_node(fallback_nodes, node);
            format!("runtime_domain_override:{pattern}")
        } else {
            matched_by
        };

        Ok(SelectionPlan {
            profile,
            matched_by,
            candidate_nodes,
            fallback_nodes,
            domain_override_node: domain_override.map(|(_, node)| node),
        })
    }

    fn resolve_domain_override(
        &self,
        ctx: &EgressSelectionContext,
        candidate_nodes: &[String],
        fallback_nodes: &[String],
    ) -> Option<(String, String)> {
        let domain = ctx.domain.as_ref()?;
        let overrides = self.domain_overrides.read();

        overrides
            .iter()
            .filter(|(pattern, selected_node)| {
                domain_matches(domain, pattern)
                    && (candidate_nodes.contains(selected_node) || fallback_nodes.contains(selected_node))
            })
            .max_by_key(|(pattern, _)| pattern.len())
            .map(|(pattern, selected_node)| (pattern.clone(), selected_node.clone()))
    }

    fn prioritize_node(mut nodes: Vec<String>, preferred_node: &str) -> Vec<String> {
        if let Some(index) = nodes.iter().position(|node| node.eq_ignore_ascii_case(preferred_node)) {
            let preferred = nodes.remove(index);
            nodes.insert(0, preferred);
        }

        nodes
    }

    fn match_profile(
        &self,
        config: &EgressIdentityConfig,
        ctx: &EgressSelectionContext,
    ) -> Result<(EgressIdentityProfile, String)> {
        if let Some(shortcut_id) = ctx.shortcut_id.as_ref() {
            if let Some(rule) = config
                .shortcut_rules
                .iter()
                .find(|rule| rule.enabled && rule.shortcut_id == *shortcut_id)
            {
                if let Some(profile) = config
                    .profiles
                    .iter()
                    .find(|profile| profile.enabled && profile.id == rule.profile_id)
                {
                    return Ok((profile.clone(), format!("shortcut_id:{}", shortcut_id)));
                }
            }
        }

        let mut app_rules = config
            .app_rules
            .iter()
            .filter(|rule| rule.enabled)
            .cloned()
            .collect::<Vec<_>>();
        app_rules.sort_by_key(|rule| rule.priority);

        for rule in app_rules {
            let process_matches = rule
                .process_name
                .as_ref()
                .map(|value| {
                    ctx.process_name
                        .as_ref()
                        .map(|process_name| process_name.eq_ignore_ascii_case(value))
                        .unwrap_or(false)
                })
                .unwrap_or(true);
            let path_matches = rule
                .exe_path
                .as_ref()
                .map(|value| {
                    ctx.exe_path
                        .as_ref()
                        .map(|exe_path| exe_path.eq_ignore_ascii_case(value))
                        .unwrap_or(false)
                })
                .unwrap_or(true);
            let domain_matches_rule = if rule.domains.is_empty() {
                true
            } else {
                ctx.domain
                    .as_ref()
                    .map(|domain| rule.domains.iter().any(|pattern| domain_matches(domain, pattern)))
                    .unwrap_or(false)
            };

            if process_matches && path_matches && domain_matches_rule {
                if let Some(profile) = config
                    .profiles
                    .iter()
                    .find(|profile| profile.enabled && profile.id == rule.profile_id)
                {
                    return Ok((profile.clone(), format!("app_rule:{}", rule.profile_id)));
                }
            }
        }

        if let Some(default_profile_id) = config.default_profile.as_ref() {
            if let Some(profile) = config
                .profiles
                .iter()
                .find(|profile| profile.enabled && &profile.id == default_profile_id)
            {
                return Ok((profile.clone(), format!("default_profile:{}", default_profile_id)));
            }
        }

        if let Some(profile) = config.profiles.iter().find(|profile| profile.enabled) {
            return Ok((profile.clone(), format!("first_enabled:{}", profile.id)));
        }

        Err(anyhow!("没有可用的出口身份画像"))
    }

    fn filter_candidates(
        &self,
        profile: &EgressIdentityProfile,
        ctx: &EgressSelectionContext,
        metadata_index: &HashMap<String, EgressNodeMetadata>,
    ) -> Vec<String> {
        // 如果配置了 allowed_nodes，则只在该集合与可用节点交集中选择
        let scoped_available = if !profile.allowed_nodes.is_empty() {
            if ctx.available_nodes.is_empty() {
                profile.allowed_nodes.clone()
            } else {
                ctx.available_nodes
                    .iter()
                    .filter(|node| profile.allowed_nodes.contains(node))
                    .cloned()
                    .collect::<Vec<_>>()
            }
        } else {
            ctx.available_nodes.clone()
        };

        if ctx.available_nodes.is_empty() {
            let preferred_candidates = profile
                .preferred_nodes
                .iter()
                .filter(|node| self.matches_hard_constraints(profile, node, metadata_index))
                .cloned()
                .collect::<Vec<_>>();

            return self.order_candidates(profile, preferred_candidates, metadata_index);
        }

        let candidates = scoped_available
            .iter()
            .filter(|node| self.matches_hard_constraints(profile, node, metadata_index))
            .cloned()
            .collect::<Vec<_>>();

        self.order_candidates(profile, candidates, metadata_index)
    }

    fn fallback_candidates(
        &self,
        profile: &EgressIdentityProfile,
        ctx: &EgressSelectionContext,
        metadata_index: &HashMap<String, EgressNodeMetadata>,
    ) -> Vec<String> {
        // 当开启 strict_node_scope 时，不再提供额外回退节点，统一交给 select_node 中的严格约束处理
        if profile.strict_node_scope {
            return Vec::new();
        }

        if ctx.available_nodes.is_empty() {
            return profile.preferred_nodes.clone();
        }

        self.order_candidates(profile, ctx.available_nodes.clone(), metadata_index)
    }

    fn build_metadata_index(ctx: &EgressSelectionContext) -> HashMap<String, EgressNodeMetadata> {
        ctx.available_node_metadata
            .iter()
            .cloned()
            .map(|metadata| (metadata.name.clone(), metadata))
            .collect::<HashMap<_, _>>()
    }

    fn matches_hard_constraints(
        &self,
        profile: &EgressIdentityProfile,
        node: &str,
        metadata_index: &HashMap<String, EgressNodeMetadata>,
    ) -> bool {
        let metadata = metadata_index.get(node);

        let type_match = match profile.required_ip_type.as_ref() {
            Some(required_ip_type) => metadata
                .and_then(|metadata| metadata.ip_type.as_ref())
                .map(|actual_ip_type| matches_ip_type(actual_ip_type, required_ip_type))
                .unwrap_or(false),
            None => true,
        };

        let score_match = match profile.max_fraud_score {
            Some(max_fraud_score) => metadata
                .and_then(|metadata| metadata.fraud_score)
                .map(|fraud_score| fraud_score <= max_fraud_score)
                .unwrap_or(false),
            None => true,
        };

        type_match && score_match
    }

    fn order_candidates(
        &self,
        profile: &EgressIdentityProfile,
        nodes: Vec<String>,
        metadata_index: &HashMap<String, EgressNodeMetadata>,
    ) -> Vec<String> {
        let mut indexed_nodes = nodes.into_iter().enumerate().collect::<Vec<_>>();

        indexed_nodes.sort_by_key(|(original_index, node_name)| {
            let metadata = metadata_index.get(node_name);

            (
                profile
                    .preferred_nodes
                    .iter()
                    .position(|preferred_node| preferred_node.eq_ignore_ascii_case(node_name))
                    .unwrap_or(usize::MAX),
                Self::preferred_pool_rank(profile, metadata),
                metadata.and_then(|metadata| metadata.fraud_score).unwrap_or(u8::MAX),
                if metadata.is_some() { 0usize } else { 1usize },
                *original_index,
            )
        });

        indexed_nodes
            .into_iter()
            .map(|(_, node_name)| node_name)
            .collect::<Vec<_>>()
    }

    fn preferred_pool_rank(profile: &EgressIdentityProfile, metadata: Option<&EgressNodeMetadata>) -> usize {
        if profile.preferred_pools.is_empty() {
            return usize::MAX;
        }

        let Some(metadata) = metadata else {
            return usize::MAX;
        };

        profile
            .preferred_pools
            .iter()
            .position(|preferred_pool| {
                metadata
                    .pool_name
                    .as_ref()
                    .map(|pool_name| pool_name == preferred_pool || pool_name.eq_ignore_ascii_case(preferred_pool))
                    .unwrap_or(false)
                    || metadata
                        .pool_type
                        .as_ref()
                        .map(|pool_type| pool_type == preferred_pool || pool_type.eq_ignore_ascii_case(preferred_pool))
                        .unwrap_or(false)
            })
            .unwrap_or(usize::MAX)
    }

    fn assignment_key_for(ctx: &EgressSelectionContext) -> String {
        if let Some(shortcut_id) = ctx.shortcut_id.as_ref() {
            return format!("shortcut:{}", shortcut_id);
        }

        if let Some(exe_path) = ctx.exe_path.as_ref() {
            return format!("exe:{}", exe_path);
        }

        if let Some(process_name) = ctx.process_name.as_ref() {
            return format!("process:{}", process_name);
        }

        if let Some(domain) = ctx.domain.as_ref() {
            return format!("domain:{}", domain);
        }

        if let (Some(source_ip), Some(source_port)) = (ctx.source_ip.as_ref(), ctx.source_port) {
            return format!("connection:{}:{}", source_ip, source_port);
        }

        "default".to_string()
    }
}

impl Default for EgressIdentityManager {
    fn default() -> Self {
        Self::new()
    }
}
