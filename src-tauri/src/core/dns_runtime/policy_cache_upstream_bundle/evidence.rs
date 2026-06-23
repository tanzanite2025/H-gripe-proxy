use super::RustDnsPolicyCacheUpstreamRollbackEvidence;
use super::constants::{COMPONENT, EVIDENCE_FILE, ROLLBACK_FILE, RUST_OWNED_SCOPE};
use crate::utils::dirs;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use smartstring::alias::String;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::fs;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RustDnsPolicyCacheUpstreamRollbackCheckpoint {
    component: String,
    rust_owned_scope: String,
    fallback_retained_for: Vec<String>,
    created_at_epoch_seconds: u64,
}

pub(super) async fn write_rollback_checkpoint(
    rollback_path: &std::path::Path,
) -> Result<RustDnsPolicyCacheUpstreamRollbackEvidence> {
    let created_at_epoch_seconds = epoch_seconds();
    let checkpoint = RustDnsPolicyCacheUpstreamRollbackCheckpoint {
        component: COMPONENT.into(),
        rust_owned_scope: RUST_OWNED_SCOPE.into(),
        fallback_retained_for: retained_fallback_scope(),
        created_at_epoch_seconds,
    };
    if let Some(parent) = rollback_path.parent() {
        fs::create_dir_all(parent).await?;
    }
    fs::write(rollback_path, serde_yaml_ng::to_string(&checkpoint)?.as_bytes()).await?;

    Ok(RustDnsPolicyCacheUpstreamRollbackEvidence {
        checkpoint_path: rollback_path.to_string_lossy().to_string().into(),
        fallback_retained_for: checkpoint.fallback_retained_for,
        created_at_epoch_seconds,
    })
}

pub(super) fn retained_fallback_scope() -> Vec<String> {
    vec![
        "default DNS runtime ownership".into(),
        "live resolver replacement and health racing".into(),
        "full GeoIP database loading".into(),
        "production persistent fake-ip cache storage".into(),
        "geosite/rule-provider database refresh".into(),
    ]
}

pub(super) fn facts() -> Vec<String> {
    vec![
        "Rust runs bounded fake-ip cache lifecycle and reverse lookup canaries".into(),
        "Rust evaluates fake-ip-filter wildcard matching for one domain".into(),
        "Rust selects and executes only loopback fallback upstream canaries".into(),
        "Rust evaluates nameserver-policy geosite/rule-provider/wildcard canaries".into(),
        "Mihomo fallback remains retained for default DNS and broad geodata/runtime ownership".into(),
    ]
}

pub(super) fn evidence_path() -> Result<std::path::PathBuf> {
    Ok(runtime_dir()?.join(EVIDENCE_FILE))
}

pub(super) fn rollback_path() -> Result<std::path::PathBuf> {
    Ok(runtime_dir()?.join(ROLLBACK_FILE))
}

fn runtime_dir() -> Result<std::path::PathBuf> {
    Ok(dirs::app_runtime_dir()?.join(COMPONENT))
}

fn epoch_seconds() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or_default()
}
