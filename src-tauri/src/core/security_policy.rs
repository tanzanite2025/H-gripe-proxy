use anyhow::{Result, anyhow};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;

/// Source prefix for security policy rules in Mihomo
pub const SECURITY_SOURCE_PREFIX: &str = "security:";

/// Single rule within a security policy
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PolicyRule {
    /// Rule type (e.g. AND, OR, NOT, PROCESS-NAME, DOMAIN-SUFFIX, IN-TYPE, etc.)
    pub rule_type: String,
    /// Rule payload
    pub payload: String,
    /// Target proxy/group name
    pub proxy: String,
}

/// A security policy definition
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SecurityPolicy {
    /// Unique policy name
    pub name: String,
    /// Whether the policy is enabled
    #[serde(default = "default_true")]
    pub enabled: bool,
    /// Description of the policy
    #[serde(default)]
    pub description: String,
    /// Rules that make up this policy
    pub rules: Vec<PolicyRule>,
    /// Legacy compatibility field for old persisted configs; no longer changes rule path.
    #[serde(default, skip_serializing)]
    pub tun_only: bool,
}

fn default_true() -> bool {
    true
}

impl Default for SecurityPolicy {
    fn default() -> Self {
        Self {
            name: String::new(),
            enabled: true,
            description: String::new(),
            rules: Vec::new(),
            tun_only: false,
        }
    }
}

/// Runtime state tracking for an applied policy
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppliedPolicyState {
    /// Policy name
    pub name: String,
    /// Whether the policy is currently enabled
    pub enabled: bool,
    /// Mihomo rule indices that were created by this policy
    pub rule_indices: Vec<i32>,
    /// Whether the policy is currently applied to Mihomo
    pub applied: bool,
}

/// Security Policy Manager owns policy definitions and their runtime state
pub struct SecurityPolicyManager {
    /// Policy definitions (name -> policy)
    policies: RwLock<HashMap<String, SecurityPolicy>>,
    /// Runtime state (name -> applied state)
    applied_states: RwLock<HashMap<String, AppliedPolicyState>>,
}

impl SecurityPolicyManager {
    pub fn new() -> Self {
        Self {
            policies: RwLock::new(HashMap::new()),
            applied_states: RwLock::new(HashMap::new()),
        }
    }

    /// Load policies from config (called on startup/config change)
    pub fn sync_policies_from_config(&self, policies: Vec<SecurityPolicy>) {
        let mut guard = self.policies.write();
        guard.clear();
        for policy in policies {
            guard.insert(policy.name.clone(), policy);
        }
    }

    /// Get all policy definitions
    pub async fn get_policies(&self) -> Vec<SecurityPolicy> {
        let guard = self.policies.read();
        guard.values().cloned().collect()
    }

    /// Get a single policy by name
    pub async fn get_policy(&self, name: &str) -> Option<SecurityPolicy> {
        let guard = self.policies.read();
        guard.get(name).cloned()
    }

    /// Remove a policy definition (also removes runtime state)
    pub async fn remove_policy(&self, name: &str) -> Option<SecurityPolicy> {
        let mut guard = self.policies.write();
        let removed = guard.remove(name);
        drop(guard);
        let mut states = self.applied_states.write();
        states.remove(name);
        removed
    }

    /// Get all applied policy states
    pub async fn get_applied_states(&self) -> Vec<AppliedPolicyState> {
        let guard = self.applied_states.read();
        guard.values().cloned().collect()
    }

    /// Get applied state for a specific policy
    pub async fn get_applied_state(&self, name: &str) -> Option<AppliedPolicyState> {
        let guard = self.applied_states.read();
        guard.get(name).cloned()
    }

    /// Record that a policy has been applied with the given rule indices
    pub async fn mark_applied(&self, name: &str, enabled: bool, rule_indices: Vec<i32>) {
        let mut guard = self.applied_states.write();
        guard.insert(
            name.to_string(),
            AppliedPolicyState {
                name: name.to_string(),
                enabled,
                rule_indices,
                applied: true,
            },
        );
    }

    /// Mark a policy as revoked (no longer applied)
    pub async fn mark_revoked(&self, name: &str) {
        let mut guard = self.applied_states.write();
        if let Some(state) = guard.get_mut(name) {
            state.applied = false;
            state.rule_indices.clear();
        }
    }

    /// Build the source tag for a policy
    pub fn source_for_policy(policy_name: &str) -> String {
        format!("{}{}", SECURITY_SOURCE_PREFIX, policy_name)
    }
}

/// Global SecurityPolicyManager instance
static SECURITY_POLICY_MANAGER: once_cell::sync::Lazy<Arc<SecurityPolicyManager>> =
    once_cell::sync::Lazy::new(|| Arc::new(SecurityPolicyManager::new()));

/// Get the global SecurityPolicyManager instance
pub fn get_security_policy_manager() -> Arc<SecurityPolicyManager> {
    SECURITY_POLICY_MANAGER.clone()
}

/// Apply a single policy to Mihomo by creating rules with the policy's source tag
pub async fn apply_policy(policy: &SecurityPolicy) -> Result<Vec<i32>> {
    for rule in &policy.rules {
        let v = crate::core::rule_engine::validate_rule_spec(&rule.rule_type, &rule.payload, &rule.proxy);
        if !v.valid {
            return Err(anyhow!(
                "invalid rule ({},{},{}): {}",
                rule.rule_type,
                rule.payload,
                rule.proxy,
                v.error.unwrap_or_else(|| "invalid rule".into())
            ));
        }
    }

    Err(anyhow!(
        "security policy rule mutation through the Go/Mihomo plugin API is retired; use the Rust runtime policy path"
    ))
}

/// Revoke a single policy from Mihomo by soft-deleting its rules
pub async fn revoke_policy(policy_name: &str) -> Result<()> {
    let manager = get_security_policy_manager();
    manager.mark_revoked(policy_name).await;
    Ok(())
}

/// Apply all enabled policies (used after config reload)
pub async fn apply_all_enabled_policies() -> Result<Vec<String>> {
    let manager = get_security_policy_manager();
    let policies = manager.get_policies().await;
    let mut applied = Vec::new();

    for policy in &policies {
        if policy.enabled {
            match apply_policy(policy).await {
                Ok(_) => {
                    log::info!(
                        "[SecurityPolicy] applied policy '{}' ({} rules)",
                        policy.name,
                        policy.rules.len()
                    );
                    applied.push(policy.name.clone());
                }
                Err(e) => {
                    log::error!("[SecurityPolicy] failed to apply policy '{}': {}", policy.name, e);
                }
            }
        }
    }

    Ok(applied)
}

/// Revoke all applied policies (used before config reload)
pub async fn revoke_all_policies() -> Result<Vec<String>> {
    let manager = get_security_policy_manager();
    let states = manager.get_applied_states().await;
    let mut revoked = Vec::new();

    for state in &states {
        if state.applied {
            match revoke_policy(&state.name).await {
                Ok(_) => {
                    revoked.push(state.name.clone());
                }
                Err(e) => {
                    log::error!("[SecurityPolicy] failed to revoke policy '{}': {}", state.name, e);
                }
            }
        }
    }

    Ok(revoked)
}
