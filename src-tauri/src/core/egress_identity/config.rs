use std::collections::HashSet;

use anyhow::{Result, anyhow};
use serde::{Deserialize, Serialize};

use crate::core::ip_reputation::IpType;

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
                    allowed_nodes: Vec::new(),
                    strict_node_scope: false,
                    use_residential_chain: false,
                    residential_proxy_name: None,
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
                    tls_fingerprint: Some("chrome".to_string()),
                    session_policy: IdentitySessionPolicy {
                        strict_affinity: true,
                        ttl_override: Some(86400),
                    },
                    failover_policy: EgressFailoverPolicy::Manual,
                    allowed_nodes: Vec::new(),
                    strict_node_scope: false,
                    use_residential_chain: true,
                    residential_proxy_name: None,
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
    #[serde(default)]
    pub allowed_nodes: Vec<String>,
    #[serde(default)]
    pub strict_node_scope: bool,
    /// 启用链式住宅路由：当此画像匹配时，自动构建 VPS→住宅 链式代理
    #[serde(default)]
    pub use_residential_chain: bool,
    /// 指定住宅代理名称（来自 residential_pool），为空则自动选择
    #[serde(default)]
    pub residential_proxy_name: Option<String>,
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
