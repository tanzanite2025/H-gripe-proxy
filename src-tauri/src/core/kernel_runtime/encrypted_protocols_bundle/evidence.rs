use super::{
    RustEncryptedProtocolsBundleRollbackEvidence,
    constants::{COMPONENT, EVIDENCE_FILE, ROLLBACK_FILE},
};
use crate::utils::dirs;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::fs;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RustEncryptedProtocolsBundleRollbackCheckpoint {
    component: String,
    fallback_retained_for: Vec<String>,
    created_at_epoch_seconds: u64,
}

pub(super) async fn write_rollback_checkpoint(
    rollback_path: &std::path::Path,
) -> Result<RustEncryptedProtocolsBundleRollbackEvidence> {
    let created_at_epoch_seconds = epoch_seconds();
    let checkpoint = RustEncryptedProtocolsBundleRollbackCheckpoint {
        component: COMPONENT.into(),
        fallback_retained_for: retained_fallback_scope(),
        created_at_epoch_seconds,
    };
    if let Some(parent) = rollback_path.parent() {
        fs::create_dir_all(parent).await?;
    }
    fs::write(rollback_path, serde_yaml_ng::to_string(&checkpoint)?.as_bytes()).await?;

    Ok(RustEncryptedProtocolsBundleRollbackEvidence {
        checkpoint_path: rollback_path.to_string_lossy().to_string(),
        fallback_retained_for: checkpoint.fallback_retained_for,
        created_at_epoch_seconds,
    })
}

pub(super) fn retained_fallback_scope() -> Vec<String> {
    vec![
        "non-loopback encrypted protocol forwarding".into(),
        "VMess/VLESS QUIC and UDP variants".into(),
        "Trojan TLS production transport".into(),
        "multiplexed encrypted sessions".into(),
        "plugin transports and default forwarding".into(),
    ]
}

pub(super) fn facts() -> Vec<String> {
    vec![
        "Rust executes bounded VMess, VLESS, and Trojan loopback TCP canary sessions".into(),
        "Each encrypted protocol canary validates request framing and forwards one TCP payload".into(),
        "Shared framing and byte-accounting code covers all encrypted protocol canaries".into(),
        "Mihomo fallback remains retained for non-loopback, UDP/QUIC, multiplexing, plugins, and defaults".into(),
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
