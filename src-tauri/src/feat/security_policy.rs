use crate::core::security_policy::*;
use anyhow::Result;

/// Get all security policy definitions from the manager
pub async fn security_policy_get_policies() -> Result<Vec<SecurityPolicy>> {
    let manager = get_security_policy_manager();
    Ok(manager.get_policies().await)
}

/// Get a single security policy by name
pub async fn security_policy_get(name: &str) -> Result<Option<SecurityPolicy>> {
    let manager = get_security_policy_manager();
    Ok(manager.get_policy(name).await)
}

/// Create or update a security policy definition
pub async fn security_policy_upsert(policy: SecurityPolicy) -> Result<()> {
    let manager = get_security_policy_manager();
    manager.upsert_policy(policy).await;
    Ok(())
}

/// Remove a security policy definition and revoke it if applied
pub async fn security_policy_remove(name: &str) -> Result<()> {
    let manager = get_security_policy_manager();
    // Revoke from Mihomo if currently applied
    let state = manager.get_applied_state(name).await;
    if let Some(state) = &state {
        if state.applied {
            revoke_policy(name).await?;
        }
    }
    manager.remove_policy(name).await;
    Ok(())
}

/// Apply a single policy to Mihomo
pub async fn security_policy_apply(name: &str) -> Result<Vec<i32>> {
    let manager = get_security_policy_manager();
    let policy = manager.get_policy(name).await;
    match policy {
        Some(policy) => apply_policy(&policy).await,
        None => anyhow::bail!("policy '{}' not found", name),
    }
}

/// Revoke a single policy from Mihomo
pub async fn security_policy_revoke(name: &str) -> Result<()> {
    revoke_policy(name).await
}

/// Apply all enabled policies to Mihomo
pub async fn security_policy_apply_all() -> Result<Vec<String>> {
    apply_all_enabled_policies().await
}

/// Revoke all applied policies from Mihomo
pub async fn security_policy_revoke_all() -> Result<Vec<String>> {
    revoke_all_policies().await
}

/// Get runtime state of all applied policies
pub async fn security_policy_get_states() -> Result<Vec<AppliedPolicyState>> {
    let manager = get_security_policy_manager();
    Ok(manager.get_applied_states().await)
}

/// Get runtime state of a specific policy
pub async fn security_policy_get_state(name: &str) -> Result<Option<AppliedPolicyState>> {
    let manager = get_security_policy_manager();
    Ok(manager.get_applied_state(name).await)
}

/// Load policies from advanced config into the manager (called on startup/config change)
pub async fn security_policy_load_from_config(policies: Vec<SecurityPolicy>) {
    let manager = get_security_policy_manager();
    manager.load_policies(policies).await;
}

/// Reload: revoke all, load new config, apply all enabled
pub async fn security_policy_reload(policies: Vec<SecurityPolicy>) -> Result<Vec<String>> {
    // Revoke existing
    let _ = revoke_all_policies().await;
    // Load new definitions
    let manager = get_security_policy_manager();
    manager.load_policies(policies).await;
    // Apply enabled
    apply_all_enabled_policies().await
}
