use crate::core::security_policy::*;

use super::{CmdResult, StringifyErr as _};

/// Get all security policy definitions
#[tauri::command]
pub async fn security_policy_get_policies() -> CmdResult<Vec<SecurityPolicy>> {
    crate::feat::security_policy_get_policies()
        .await
        .stringify_err()
}

/// Get a single security policy by name
#[tauri::command]
pub async fn security_policy_get(name: String) -> CmdResult<Option<SecurityPolicy>> {
    crate::feat::security_policy_get(&name)
        .await
        .stringify_err()
}

/// Create or update a security policy definition
#[tauri::command]
pub async fn security_policy_upsert(policy: SecurityPolicy) -> CmdResult<()> {
    crate::feat::security_policy_upsert(policy)
        .await
        .stringify_err()
}

/// Remove a security policy and revoke it if applied
#[tauri::command]
pub async fn security_policy_remove(name: String) -> CmdResult<()> {
    crate::feat::security_policy_remove(&name)
        .await
        .stringify_err()
}

/// Apply a single policy to Mihomo (create rules with source=security:<name>)
#[tauri::command]
pub async fn security_policy_apply(name: String) -> CmdResult<Vec<i32>> {
    crate::feat::security_policy_apply(&name)
        .await
        .stringify_err()
}

/// Revoke a single policy from Mihomo (soft-delete its rules)
#[tauri::command]
pub async fn security_policy_revoke(name: String) -> CmdResult<()> {
    crate::feat::security_policy_revoke(&name)
        .await
        .stringify_err()
}

/// Apply all enabled policies to Mihomo
#[tauri::command]
pub async fn security_policy_apply_all() -> CmdResult<Vec<String>> {
    crate::feat::security_policy_apply_all()
        .await
        .stringify_err()
}

/// Revoke all applied policies from Mihomo
#[tauri::command]
pub async fn security_policy_revoke_all() -> CmdResult<Vec<String>> {
    crate::feat::security_policy_revoke_all()
        .await
        .stringify_err()
}

/// Get runtime state of all applied policies
#[tauri::command]
pub async fn security_policy_get_states() -> CmdResult<Vec<AppliedPolicyState>> {
    crate::feat::security_policy_get_states()
        .await
        .stringify_err()
}

/// Get runtime state of a specific policy
#[tauri::command]
pub async fn security_policy_get_state(name: String) -> CmdResult<Option<AppliedPolicyState>> {
    crate::feat::security_policy_get_state(&name)
        .await
        .stringify_err()
}

/// Reload policies from config: revoke all, load new definitions, apply enabled
#[tauri::command]
pub async fn security_policy_reload(policies: Vec<SecurityPolicy>) -> CmdResult<Vec<String>> {
    crate::feat::security_policy_reload(policies)
        .await
        .stringify_err()
}
