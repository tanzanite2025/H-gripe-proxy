use super::super::RustSocksUdpFragmentsRollbackEvidence;
use super::constants::{
    RUST_SOCKS_UDP_FRAGMENTS_COMPONENT, RUST_SOCKS_UDP_FRAGMENTS_EVIDENCE_FILE, RUST_SOCKS_UDP_FRAGMENTS_OWNED_SCOPE,
    RUST_SOCKS_UDP_FRAGMENTS_ROLLBACK_FILE,
};
use crate::utils::dirs;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use smartstring::alias::String;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::fs;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RustSocksUdpFragmentsRollbackCheckpoint {
    component: String,
    rust_owned_scope: String,
    fallback_retained_for: Vec<String>,
    created_at_epoch_seconds: u64,
}

pub(super) async fn write_rollback_checkpoint(
    rollback_path: &std::path::Path,
) -> Result<RustSocksUdpFragmentsRollbackEvidence> {
    let created_at_epoch_seconds = rust_socks_udp_fragments_epoch_seconds();
    let checkpoint = RustSocksUdpFragmentsRollbackCheckpoint {
        component: RUST_SOCKS_UDP_FRAGMENTS_COMPONENT.into(),
        rust_owned_scope: RUST_SOCKS_UDP_FRAGMENTS_OWNED_SCOPE.into(),
        fallback_retained_for: retained_socks_udp_fragments_fallback_scope(),
        created_at_epoch_seconds,
    };
    if let Some(parent) = rollback_path.parent() {
        fs::create_dir_all(parent).await?;
    }
    fs::write(rollback_path, serde_yaml_ng::to_string(&checkpoint)?.as_bytes()).await?;

    Ok(RustSocksUdpFragmentsRollbackEvidence {
        checkpoint_path: rollback_path.to_string_lossy().to_string().into(),
        fallback_retained_for: checkpoint.fallback_retained_for,
        created_at_epoch_seconds,
    })
}

pub(super) fn retained_socks_udp_fragments_fallback_scope() -> Vec<String> {
    vec![
        "SOCKS UDP non-loopback forwarding".into(),
        "SOCKS UDP multi-destination fragment queues, cache eviction, and timeout windows".into(),
        "Shadowsocks UDP/plugin transports".into(),
        "VMess, VLESS, and Trojan encrypted sessions".into(),
        "system-wide packet capture and transparent proxy defaults".into(),
    ]
}

pub(super) fn rust_socks_udp_fragments_facts() -> Vec<String> {
    vec![
        "Rust parses two SOCKS5 UDP fragments with RFC1928 FRAG sequencing".into(),
        "Rust reassembles only a bounded IPv4 loopback target before forwarding".into(),
        "Rust forwards the reassembled payload to one loopback UDP target and records byte evidence".into(),
        "Mihomo fallback remains retained for non-loopback UDP, fragment queues/timeouts, plugin transports, and packet capture".into(),
    ]
}

pub(super) fn rust_socks_udp_fragments_evidence_path() -> Result<std::path::PathBuf> {
    Ok(rust_socks_udp_fragments_dir()?.join(RUST_SOCKS_UDP_FRAGMENTS_EVIDENCE_FILE))
}

pub(super) fn rust_socks_udp_fragments_rollback_path() -> Result<std::path::PathBuf> {
    Ok(rust_socks_udp_fragments_dir()?.join(RUST_SOCKS_UDP_FRAGMENTS_ROLLBACK_FILE))
}

fn rust_socks_udp_fragments_dir() -> Result<std::path::PathBuf> {
    Ok(dirs::app_runtime_dir()?.join(RUST_SOCKS_UDP_FRAGMENTS_COMPONENT))
}

fn rust_socks_udp_fragments_epoch_seconds() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or_default()
}
