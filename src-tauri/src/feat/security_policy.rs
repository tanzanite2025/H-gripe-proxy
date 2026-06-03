use crate::core::security_policy::*;
use anyhow::{Result, bail};

/// Apply a single policy to Mihomo.
pub async fn security_policy_apply(name: &str) -> Result<Vec<i32>> {
    let manager = get_security_policy_manager();
    let policy = manager.get_policy(name).await;
    match policy {
        Some(policy) => apply_policy(&policy).await,
        None => bail!("policy '{}' not found", name),
    }
}

/// Revoke a single policy from Mihomo.
pub async fn security_policy_revoke(name: &str) -> Result<()> {
    revoke_policy(name).await
}

/// Apply all enabled policies to Mihomo.
pub async fn security_policy_apply_all() -> Result<Vec<String>> {
    apply_all_enabled_policies().await
}

/// Revoke all applied policies from Mihomo.
pub async fn security_policy_revoke_all() -> Result<Vec<String>> {
    revoke_all_policies().await
}

/// Get runtime state of all applied policies.
pub async fn security_policy_get_states() -> Result<Vec<AppliedPolicyState>> {
    let manager = get_security_policy_manager();
    Ok(manager.get_applied_states().await)
}

/// Get runtime state of a specific policy.
pub async fn security_policy_get_state(name: &str) -> Result<Option<AppliedPolicyState>> {
    let manager = get_security_policy_manager();
    Ok(manager.get_applied_state(name).await)
}
