use super::RustDnsFallbackFilterGeoipRollbackEvidence;
use super::constants::{
    RUST_DNS_FALLBACK_FILTER_GEOIP_COMPONENT, RUST_DNS_FALLBACK_FILTER_GEOIP_EVIDENCE_FILE,
    RUST_DNS_FALLBACK_FILTER_GEOIP_OWNED_SCOPE, RUST_DNS_FALLBACK_FILTER_GEOIP_ROLLBACK_FILE,
};
use crate::utils::dirs;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use smartstring::alias::String;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::fs;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RustDnsFallbackFilterGeoipRollbackCheckpoint {
    component: String,
    rust_owned_scope: String,
    fallback_retained_for: Vec<String>,
    created_at_epoch_seconds: u64,
}

pub(super) async fn write_rollback_checkpoint(
    rollback_path: &std::path::Path,
) -> Result<RustDnsFallbackFilterGeoipRollbackEvidence> {
    let created_at_epoch_seconds = rust_dns_fallback_filter_geoip_epoch_seconds();
    let checkpoint = RustDnsFallbackFilterGeoipRollbackCheckpoint {
        component: RUST_DNS_FALLBACK_FILTER_GEOIP_COMPONENT.into(),
        rust_owned_scope: RUST_DNS_FALLBACK_FILTER_GEOIP_OWNED_SCOPE.into(),
        fallback_retained_for: retained_fallback_filter_geoip_scope(),
        created_at_epoch_seconds,
    };
    if let Some(parent) = rollback_path.parent() {
        fs::create_dir_all(parent).await?;
    }
    fs::write(rollback_path, serde_yaml_ng::to_string(&checkpoint)?.as_bytes()).await?;

    Ok(RustDnsFallbackFilterGeoipRollbackEvidence {
        checkpoint_path: rollback_path.to_string_lossy().to_string().into(),
        fallback_retained_for: checkpoint.fallback_retained_for,
        created_at_epoch_seconds,
    })
}

pub(super) fn retained_fallback_filter_geoip_scope() -> Vec<String> {
    vec![
        "fallback-filter upstream fallback DNS execution".into(),
        "full GeoIP database loading and country-code coverage beyond bounded canary CIDRs".into(),
        "fallback-filter wildcard/default DNS integration".into(),
        "nameserver-policy geosite/rule-provider/wildcard execution".into(),
        "fake-ip cache and reverse mapping".into(),
    ]
}

pub(super) fn rust_dns_fallback_filter_geoip_facts() -> Vec<String> {
    vec![
        "Rust parses fallback-filter geoip and geoip-code settings from dns.fallback-filter".into(),
        "Rust evaluates one candidate answer against a bounded built-in GeoIP canary CIDR set".into(),
        "Rust marks fallback required when geoip is enabled and the candidate does not match geoip-code".into(),
        "Rust writes rollback/evidence artifacts without upstream DNS or system resolver mutation".into(),
        "Mihomo fallback remains retained for full geodata coverage, upstream fallback execution, policy cache, and default DNS".into(),
    ]
}

pub(super) fn rust_dns_fallback_filter_geoip_evidence_path() -> Result<std::path::PathBuf> {
    Ok(rust_dns_fallback_filter_geoip_dir()?.join(RUST_DNS_FALLBACK_FILTER_GEOIP_EVIDENCE_FILE))
}

pub(super) fn rust_dns_fallback_filter_geoip_rollback_path() -> Result<std::path::PathBuf> {
    Ok(rust_dns_fallback_filter_geoip_dir()?.join(RUST_DNS_FALLBACK_FILTER_GEOIP_ROLLBACK_FILE))
}

fn rust_dns_fallback_filter_geoip_dir() -> Result<std::path::PathBuf> {
    Ok(dirs::app_runtime_dir()?.join(RUST_DNS_FALLBACK_FILTER_GEOIP_COMPONENT))
}

fn rust_dns_fallback_filter_geoip_epoch_seconds() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or_default()
}
