use std::collections::{HashMap, HashSet};

use anyhow::{anyhow, Result};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};

use crate::core::ip_reputation::{matches_ip_type, IpType};
use crate::core::session_affinity::domain_matches;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EgressIdentityConfig {
    pub enabled: bool,
    pub default_profile: Option<String>,
    pub profiles: Vec<EgressIdentityProfile>,
    pub app_rules: Vec<AppEgressRule>,
    pub shortcut_rules: Vec<ShortcutEgressRule>,
}

impl Default for EgressIdentityConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            default_profile: None,
            profiles: Vec::new(),
            app_rules: Vec::new(),
            shortcut_rules: Vec::new(),
        }
    }
}

impl EgressIdentityConfig {
    pub fn recommended() -> Self {
        Self {
            enabled: false,
            default_profile: Some("stable-default".to_string()),
            profiles: vec![
                EgressIdentityProfile {
                    id: "stable-default".to_string(),
                    name: "稳定默认画像".to_string(),
                    enabled: true,
                    preferred_nodes: Vec::new(),
                    preferred_pools: vec!["通用池".to_string()],
                    required_ip_type: None,
                    max_fraud_score: Some(70),
                    dns_policy: DnsPolicy::default(),
                    tls_fingerprint: None,
                    session_policy: IdentitySessionPolicy::default(),
                    failover_policy: EgressFailoverPolicy::Manual,
                    description: "默认的稳定出口身份骨架".to_string(),
                },
                EgressIdentityProfile {
                    id: "ai-strict".to_string(),
                    name: "AI 严格画像".to_string(),
                    enabled: true,
                    preferred_nodes: Vec::new(),
                    preferred_pools: vec!["通用池".to_string()],
                    required_ip_type: Some(IpType::Residential),
                    max_fraud_score: Some(30),
                    dns_policy: DnsPolicy {
                        mode: DnsMode::Remote,
                        force_remote_dns: true,
                    },
                    tls_fingerprint: Some("Chrome 120 (Windows)".to_string()),
                    session_policy: IdentitySessionPolicy {
                        strict_affinity: true,
                        ttl_override: Some(86400),
                    },
                    failover_policy: EgressFailoverPolicy::Manual,
                    description: "适用于高风控服务的严格身份骨架".to_string(),
                },
            ],
            app_rules: vec![
                AppEgressRule {
                    process_name: None,
                    exe_path: None,
                    domains: vec!["*.openai.com".to_string(), "*.anthropic.com".to_string()],
                    profile_id: "ai-strict".to_string(),
                    priority: 10,
                    enabled: true,
                },
                AppEgressRule {
                    process_name: Some("Steam.exe".to_string()),
                    exe_path: None,
                    domains: Vec::new(),
                    profile_id: "stable-default".to_string(),
                    priority: 100,
                    enabled: true,
                },
            ],
            shortcut_rules: vec![ShortcutEgressRule {
                shortcut_id: "chatgpt".to_string(),
                profile_id: "ai-strict".to_string(),
                enabled: true,
            }],
        }
    }

    pub fn validate(&self) -> Result<()> {
        let mut seen_ids = HashSet::new();

        for profile in &self.profiles {
            if profile.id.trim().is_empty() {
                return Err(anyhow!("出口身份画像 ID 不能为空"));
            }

            if !seen_ids.insert(profile.id.clone()) {
                return Err(anyhow!("出口身份画像 ID 重复: {}", profile.id));
            }
        }

        if let Some(default_profile) = &self.default_profile {
            if !self.profiles.iter().any(|profile| &profile.id == default_profile) {
                return Err(anyhow!("默认出口身份画像不存在: {}", default_profile));
            }
        }

        for rule in &self.app_rules {
            let has_process_name = rule
                .process_name
                .as_ref()
                .map(|value| !value.trim().is_empty())
                .unwrap_or(false);
            let has_exe_path = rule
                .exe_path
                .as_ref()
                .map(|value| !value.trim().is_empty())
                .unwrap_or(false);
            let has_domains = rule.domains.iter().any(|value| !value.trim().is_empty());

            if !has_process_name && !has_exe_path && !has_domains {
                return Err(anyhow!(
                    "应用规则至少需要 process_name、exe_path 或 domains 中的一个条件"
                ));
            }

            if !self.profiles.iter().any(|profile| profile.id == rule.profile_id) {
                return Err(anyhow!("应用规则引用了不存在的画像: {}", rule.profile_id));
            }
        }

        for rule in &self.shortcut_rules {
            if rule.shortcut_id.trim().is_empty() {
                return Err(anyhow!("快捷方式规则的 shortcut_id 不能为空"));
            }

            if !self.profiles.iter().any(|profile| profile.id == rule.profile_id) {
                return Err(anyhow!("快捷方式规则引用了不存在的画像: {}", rule.profile_id));
            }
        }

        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EgressIdentityProfile {
    pub id: String,
    pub name: String,
    pub enabled: bool,
    pub preferred_nodes: Vec<String>,
    pub preferred_pools: Vec<String>,
    pub required_ip_type: Option<IpType>,
    pub max_fraud_score: Option<u8>,
    pub dns_policy: DnsPolicy,
    pub tls_fingerprint: Option<String>,
    pub session_policy: IdentitySessionPolicy,
    pub failover_policy: EgressFailoverPolicy,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppEgressRule {
    pub process_name: Option<String>,
    pub exe_path: Option<String>,
    pub domains: Vec<String>,
    pub profile_id: String,
    pub priority: u32,
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShortcutEgressRule {
    pub shortcut_id: String,
    pub profile_id: String,
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DnsPolicy {
    pub mode: DnsMode,
    pub force_remote_dns: bool,
}

impl Default for DnsPolicy {
    fn default() -> Self {
        Self {
            mode: DnsMode::Inherit,
            force_remote_dns: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DnsMode {
    Inherit,
    Hijack,
    Remote,
}

impl Default for DnsMode {
    fn default() -> Self {
        Self::Inherit
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IdentitySessionPolicy {
    pub strict_affinity: bool,
    pub ttl_override: Option<u64>,
}

impl Default for IdentitySessionPolicy {
    fn default() -> Self {
        Self {
            strict_affinity: false,
            ttl_override: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EgressFailoverPolicy {
    Block,
    Manual,
    AutoSwitch,
}

impl Default for EgressFailoverPolicy {
    fn default() -> Self {
        Self::Manual
    }
}

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

        match &self.profile.failover_policy {
            EgressFailoverPolicy::Block => {
                Err(anyhow!("出口身份画像 {} 没有满足约束的候选节点", self.profile.id))
            }
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

        self.active_assignments.write().retain(|_, assignment| {
            valid_profile_ids.contains(&assignment.profile_id)
        });

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
            let node_is_available = ctx.available_nodes.is_empty()
                || ctx.available_nodes.contains(&existing.selected_node);
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
        self.active_assignments
            .write()
            .insert(assignment_key, resolved.clone());
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

        self.active_assignments
            .write()
            .insert(assignment_key, resolved.clone());

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

        self.active_assignments
            .write()
            .insert(assignment_key, resolved.clone());

        Ok(resolved)
    }

    pub fn get_active_assignments(&self) -> Vec<ResolvedEgressIdentity> {
        let mut assignments = self
            .active_assignments
            .read()
            .values()
            .cloned()
            .collect::<Vec<_>>();

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
        if let Some(index) = nodes
            .iter()
            .position(|node| node.eq_ignore_ascii_case(preferred_node))
        {
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
                    return Ok((
                        profile.clone(),
                        format!("shortcut_id:{}", shortcut_id),
                    ));
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
                    .map(|domain| {
                        rule.domains
                            .iter()
                            .any(|pattern| domain_matches(domain, pattern))
                    })
                    .unwrap_or(false)
            };

            if process_matches && path_matches && domain_matches_rule {
                if let Some(profile) = config
                    .profiles
                    .iter()
                    .find(|profile| profile.enabled && profile.id == rule.profile_id)
                {
                    return Ok((
                        profile.clone(),
                        format!("app_rule:{}", rule.profile_id),
                    ));
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
        if ctx.available_nodes.is_empty() {
            let preferred_candidates = profile
                .preferred_nodes
                .iter()
                .filter(|node| self.matches_hard_constraints(profile, node, metadata_index))
                .cloned()
                .collect::<Vec<_>>();

            return self.order_candidates(profile, preferred_candidates, metadata_index);
        }

        let candidates = ctx
            .available_nodes
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
                metadata
                    .and_then(|metadata| metadata.fraud_score)
                    .unwrap_or(u8::MAX),
                if metadata.is_some() { 0usize } else { 1usize },
                *original_index,
            )
        });

        indexed_nodes
            .into_iter()
            .map(|(_, node_name)| node_name)
            .collect::<Vec<_>>()
    }

    fn preferred_pool_rank(
        profile: &EgressIdentityProfile,
        metadata: Option<&EgressNodeMetadata>,
    ) -> usize {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_recommended_config() {
        let config = EgressIdentityConfig::recommended();
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_preview_match_uses_default_profile_with_matching_metadata() {
        let mut config = EgressIdentityConfig::recommended();
        config.enabled = true;

        let manager = EgressIdentityManager::new_with_config(config);
        let resolved = manager
            .preview_match(EgressSelectionContext {
                available_nodes: vec!["node-a".to_string()],
                available_node_metadata: vec![EgressNodeMetadata {
                    name: "node-a".to_string(),
                    pool_name: Some("通用池".to_string()),
                    ip_type: Some(IpType::Residential),
                    fraud_score: Some(15),
                    ..Default::default()
                }],
                ..Default::default()
            })
            .unwrap();

        assert_eq!(resolved.profile_id, "stable-default");
        assert_eq!(resolved.selected_node, "node-a");
    }

    #[test]
    fn test_preview_match_uses_domain_only_app_rule() {
        let mut config = EgressIdentityConfig::recommended();
        config.enabled = true;

        let manager = EgressIdentityManager::new_with_config(config);
        let resolved = manager
            .preview_match(EgressSelectionContext {
                domain: Some("chat.openai.com".to_string()),
                available_nodes: vec!["node-resi".to_string(), "node-dc".to_string()],
                available_node_metadata: vec![
                    EgressNodeMetadata {
                        name: "node-resi".to_string(),
                        pool_name: Some("通用池".to_string()),
                        ip_type: Some(IpType::Residential),
                        fraud_score: Some(15),
                        ..Default::default()
                    },
                    EgressNodeMetadata {
                        name: "node-dc".to_string(),
                        pool_name: Some("通用池".to_string()),
                        ip_type: Some(IpType::Datacenter),
                        fraud_score: Some(85),
                        ..Default::default()
                    },
                ],
                ..Default::default()
            })
            .unwrap();

        assert_eq!(resolved.profile_id, "ai-strict");
        assert_eq!(resolved.selected_node, "node-resi");
        assert!(matches!(resolved.dns_mode, DnsMode::Remote));
    }

    #[test]
    fn test_assign_records_active_assignment() {
        let mut config = EgressIdentityConfig::recommended();
        config.enabled = true;

        let manager = EgressIdentityManager::new_with_config(config);
        let resolved = manager
            .assign(EgressSelectionContext {
                shortcut_id: Some("chatgpt".to_string()),
                available_nodes: vec!["node-resi".to_string(), "node-dc".to_string()],
                available_node_metadata: vec![
                    EgressNodeMetadata {
                        name: "node-resi".to_string(),
                        pool_name: Some("通用池".to_string()),
                        ip_type: Some(IpType::Residential),
                        fraud_score: Some(15),
                        ..Default::default()
                    },
                    EgressNodeMetadata {
                        name: "node-dc".to_string(),
                        pool_name: Some("通用池".to_string()),
                        ip_type: Some(IpType::Datacenter),
                        fraud_score: Some(85),
                        ..Default::default()
                    },
                ],
                ..Default::default()
            })
            .unwrap();

        assert_eq!(resolved.profile_id, "ai-strict");
        assert_eq!(resolved.selected_node, "node-resi");
        assert_eq!(manager.get_active_assignments().len(), 1);
    }

    #[test]
    fn test_preferred_pool_and_score_constraints_filter_candidates() {
        let config = EgressIdentityConfig {
            enabled: true,
            default_profile: Some("pool-pref".to_string()),
            profiles: vec![EgressIdentityProfile {
                id: "pool-pref".to_string(),
                name: "池偏好画像".to_string(),
                enabled: true,
                preferred_nodes: Vec::new(),
                preferred_pools: vec!["ResidentialPool".to_string()],
                required_ip_type: Some(IpType::Residential),
                max_fraud_score: Some(30),
                dns_policy: DnsPolicy::default(),
                tls_fingerprint: None,
                session_policy: IdentitySessionPolicy::default(),
                failover_policy: EgressFailoverPolicy::Block,
                description: String::new(),
            }],
            app_rules: Vec::new(),
            shortcut_rules: Vec::new(),
        };

        let manager = EgressIdentityManager::new_with_config(config);
        let resolved = manager
            .preview_match(EgressSelectionContext {
                available_nodes: vec!["node-a".to_string(), "node-b".to_string()],
                available_node_metadata: vec![
                    EgressNodeMetadata {
                        name: "node-a".to_string(),
                        pool_name: Some("GeneralPool".to_string()),
                        ip_type: Some(IpType::Residential),
                        fraud_score: Some(20),
                        ..Default::default()
                    },
                    EgressNodeMetadata {
                        name: "node-b".to_string(),
                        pool_name: Some("ResidentialPool".to_string()),
                        ip_type: Some(IpType::Residential),
                        fraud_score: Some(10),
                        ..Default::default()
                    },
                ],
                ..Default::default()
            })
            .unwrap();

        assert_eq!(resolved.selected_node, "node-b");
    }

    #[test]
    fn test_domain_override_beats_existing_exact_domain_assignment() {
        let mut config = EgressIdentityConfig::recommended();
        config.enabled = true;

        let manager = EgressIdentityManager::new_with_config(config);
        let domain_ctx = EgressSelectionContext {
            domain: Some("chat.openai.com".to_string()),
            available_nodes: vec!["node-resi-a".to_string(), "node-resi-b".to_string()],
            available_node_metadata: vec![
                EgressNodeMetadata {
                    name: "node-resi-a".to_string(),
                    pool_name: Some("通用池".to_string()),
                    ip_type: Some(IpType::Residential),
                    fraud_score: Some(15),
                    ..Default::default()
                },
                EgressNodeMetadata {
                    name: "node-resi-b".to_string(),
                    pool_name: Some("通用池".to_string()),
                    ip_type: Some(IpType::Residential),
                    fraud_score: Some(20),
                    ..Default::default()
                },
            ],
            ..Default::default()
        };

        let first = manager.assign(domain_ctx.clone()).unwrap();
        assert_eq!(first.selected_node, "node-resi-a");

        manager
            .record_domain_override(
                "*.openai.com",
                EgressSelectionContext {
                    domain: Some("openai.com".to_string()),
                    available_nodes: domain_ctx.available_nodes.clone(),
                    available_node_metadata: domain_ctx.available_node_metadata.clone(),
                    ..Default::default()
                },
                "node-resi-b".to_string(),
            )
            .unwrap();

        let updated = manager.assign(domain_ctx).unwrap();
        assert_eq!(updated.profile_id, "ai-strict");
        assert_eq!(updated.selected_node, "node-resi-b");
        assert_eq!(updated.matched_by, "runtime_domain_override:*.openai.com");

        let assignments = manager.get_active_assignments();
        assert!(assignments.iter().any(|assignment| {
            assignment.assignment_key.as_deref() == Some("domain-pattern:*.openai.com")
                && assignment.selected_node == "node-resi-b"
        }));

        manager.clear_assignment("domain-pattern:*.openai.com");

        let reassigned = manager
            .assign(EgressSelectionContext {
                domain: Some("chat.openai.com".to_string()),
                available_nodes: vec!["node-resi-a".to_string(), "node-resi-b".to_string()],
                available_node_metadata: vec![
                    EgressNodeMetadata {
                        name: "node-resi-a".to_string(),
                        pool_name: Some("通用池".to_string()),
                        ip_type: Some(IpType::Residential),
                        fraud_score: Some(15),
                        ..Default::default()
                    },
                    EgressNodeMetadata {
                        name: "node-resi-b".to_string(),
                        pool_name: Some("通用池".to_string()),
                        ip_type: Some(IpType::Residential),
                        fraud_score: Some(20),
                        ..Default::default()
                    },
                ],
                ..Default::default()
            })
            .unwrap();
        assert_eq!(reassigned.selected_node, "node-resi-a");
    }
}
