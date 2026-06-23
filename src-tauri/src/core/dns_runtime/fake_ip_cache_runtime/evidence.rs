use super::RustDnsFakeIpCacheRollbackEvidence;
use super::constants::{
    RUST_DNS_FAKE_IP_CACHE_COMPONENT, RUST_DNS_FAKE_IP_CACHE_EVIDENCE_FILE, RUST_DNS_FAKE_IP_CACHE_OWNED_SCOPE,
    RUST_DNS_FAKE_IP_CACHE_ROLLBACK_FILE,
};
use crate::utils::dirs;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use smartstring::alias::String;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::fs;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RustDnsFakeIpCacheRollbackCheckpoint {
    component: String,
    rust_owned_scope: String,
    fallback_retained_for: Vec<String>,
    created_at_epoch_seconds: u64,
}

pub(super) async fn write_rollback_checkpoint(
    rollback_path: &std::path::Path,
) -> Result<RustDnsFakeIpCacheRollbackEvidence> {
    let created_at_epoch_seconds = rust_dns_fake_ip_cache_epoch_seconds();
    let checkpoint = RustDnsFakeIpCacheRollbackCheckpoint {
        component: RUST_DNS_FAKE_IP_CACHE_COMPONENT.into(),
        rust_owned_scope: RUST_DNS_FAKE_IP_CACHE_OWNED_SCOPE.into(),
        fallback_retained_for: retained_fake_ip_cache_scope(),
        created_at_epoch_seconds,
    };
    if let Some(parent) = rollback_path.parent() {
        fs::create_dir_all(parent).await?;
    }
    fs::write(rollback_path, serde_yaml_ng::to_string(&checkpoint)?.as_bytes()).await?;

    Ok(RustDnsFakeIpCacheRollbackEvidence {
        checkpoint_path: rollback_path.to_string_lossy().to_string().into(),
        fallback_retained_for: checkpoint.fallback_retained_for,
        created_at_epoch_seconds,
    })
}

pub(super) fn retained_fake_ip_cache_scope() -> Vec<String> {
    vec![
        "persistent fake-ip cache lifecycle and eviction".into(),
        "fake-ip-filter wildcard semantics".into(),
        "fallback-filter upstream execution and policy cache".into(),
        "nameserver-policy dispatch".into(),
        "default DNS runtime ownership".into(),
    ]
}

pub(super) fn rust_dns_fake_ip_cache_facts() -> Vec<String> {
    vec![
        "Rust inserts one fake-ip forward cache entry for one normalized domain".into(),
        "Rust resolves one reverse lookup from fake-ip back to the original domain".into(),
        "Rust verifies the cached fake-ip stays inside the configured fake-ip-range".into(),
        "Rust writes rollback/evidence artifacts without upstream DNS or system resolver mutation".into(),
        "Mihomo fallback remains retained for persistent cache lifecycle, filters, policy dispatch, and default DNS"
            .into(),
    ]
}

pub(super) fn rust_dns_fake_ip_cache_evidence_path() -> Result<std::path::PathBuf> {
    Ok(rust_dns_fake_ip_cache_dir()?.join(RUST_DNS_FAKE_IP_CACHE_EVIDENCE_FILE))
}

pub(super) fn rust_dns_fake_ip_cache_rollback_path() -> Result<std::path::PathBuf> {
    Ok(rust_dns_fake_ip_cache_dir()?.join(RUST_DNS_FAKE_IP_CACHE_ROLLBACK_FILE))
}

fn rust_dns_fake_ip_cache_dir() -> Result<std::path::PathBuf> {
    Ok(dirs::app_runtime_dir()?.join(RUST_DNS_FAKE_IP_CACHE_COMPONENT))
}

fn rust_dns_fake_ip_cache_epoch_seconds() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or_default()
}
