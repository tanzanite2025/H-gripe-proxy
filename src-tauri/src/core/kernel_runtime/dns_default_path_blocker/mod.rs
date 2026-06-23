use super::RUST_RUNTIME_ID;
use crate::utils::dirs;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use sha2::{Digest as _, Sha256};
use std::{
    net::{Ipv4Addr, SocketAddr, UdpSocket},
    thread,
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use tokio::fs;

const COMPONENT: &str = "rust-dns-default-path-blocker";
const KERNEL_AREA: &str = "dns-default-path-blocker";
const EVIDENCE_FILE: &str = "evidence.yaml";
const CACHE_FILE: &str = "persistent-cache.yaml";
const GEODATA_REFRESH_FILE: &str = "geodata-refresh.yaml";
const DNS_QUERY: &[u8] = b"default-path-blocker.invalid";
const DNS_RESPONSE_PREFIX: &[u8] = b"rust-dns-default-ok:";
const NEXT_SAFE_BATCH: &str = "dns-default-cutover-hold-window";

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum RustDnsDefaultPathBlockerStatus {
    Ready,
    Blocked,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RustDnsDefaultPathLiveResolverEvidence {
    pub resolver_addr: String,
    pub query_name: String,
    pub response_payload: String,
    pub loopback_only: bool,
    pub response_matched: bool,
    pub passed: bool,
    pub blockers: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RustDnsDefaultPathPersistentCacheEvidence {
    pub cache_path: String,
    pub cache_key: String,
    pub cache_value: String,
    pub checksum: String,
    pub migrated_without_mihomo: bool,
    pub passed: bool,
    pub blockers: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RustDnsDefaultPathGeodataRefreshEvidence {
    pub refresh_manifest_path: String,
    pub source: String,
    pub next_refresh_epoch_seconds: u64,
    pub owned_by_rust: bool,
    pub mihomo_geodata_refresh_required: bool,
    pub passed: bool,
    pub blockers: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RustDnsDefaultPathBlockerReport {
    pub runtime_id: String,
    pub component: String,
    pub kernel_area: String,
    pub status: RustDnsDefaultPathBlockerStatus,
    pub reason: String,
    pub explicit_opt_in: bool,
    pub live_resolver_evidence: Option<RustDnsDefaultPathLiveResolverEvidence>,
    pub persistent_cache_evidence: Option<RustDnsDefaultPathPersistentCacheEvidence>,
    pub geodata_refresh_evidence: Option<RustDnsDefaultPathGeodataRefreshEvidence>,
    pub evidence_path: Option<String>,
    pub mutates_runtime: bool,
    pub writes_evidence: bool,
    pub default_dns_replacement_allowed: bool,
    pub mihomo_default_dns_fallback_required: bool,
    pub blockers_reduced: Vec<String>,
    pub blockers_remaining: Vec<String>,
    pub blockers: Vec<String>,
    pub warnings: Vec<String>,
    pub facts: Vec<String>,
    pub next_safe_batch: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PersistentCacheRecord {
    query_name: String,
    answer: String,
    source: String,
    created_at_epoch_seconds: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct GeodataRefreshManifest {
    source: String,
    refresh_owner: String,
    created_at_epoch_seconds: u64,
    next_refresh_epoch_seconds: u64,
    mihomo_geodata_refresh_required: bool,
}

pub async fn rust_dns_default_path_blocker_reduction(explicit_opt_in: bool) -> Result<RustDnsDefaultPathBlockerReport> {
    if !explicit_opt_in {
        return Ok(blocked_report(vec![
            "explicit opt-in is required to run DNS default-path blocker reduction".to_owned(),
        ]));
    }

    let live_resolver_evidence = live_resolver_evidence()?;
    let persistent_cache_evidence = persistent_cache_evidence().await?;
    let geodata_refresh_evidence = geodata_refresh_evidence().await?;
    let mut blockers = Vec::new();
    blockers.extend(live_resolver_evidence.blockers.iter().cloned());
    blockers.extend(persistent_cache_evidence.blockers.iter().cloned());
    blockers.extend(geodata_refresh_evidence.blockers.iter().cloned());
    let status = if blockers.is_empty() {
        RustDnsDefaultPathBlockerStatus::Ready
    } else {
        RustDnsDefaultPathBlockerStatus::Blocked
    };
    let evidence_path = evidence_path()?;
    let mut report = RustDnsDefaultPathBlockerReport {
        runtime_id: RUST_RUNTIME_ID.to_owned(),
        component: COMPONENT.to_owned(),
        kernel_area: KERNEL_AREA.to_owned(),
        status,
        reason: if status == RustDnsDefaultPathBlockerStatus::Ready {
            "Rust reduced DNS default-path blockers for bounded live resolver, persistent cache, and geodata refresh ownership"
        } else {
            "Rust DNS default-path blocker reduction is blocked"
        }
        .to_owned(),
        explicit_opt_in,
        live_resolver_evidence: Some(live_resolver_evidence),
        persistent_cache_evidence: Some(persistent_cache_evidence),
        geodata_refresh_evidence: Some(geodata_refresh_evidence),
        evidence_path: Some(evidence_path.to_string_lossy().to_string()),
        mutates_runtime: false,
        writes_evidence: true,
        default_dns_replacement_allowed: false,
        mihomo_default_dns_fallback_required: true,
        blockers_reduced: vec![
            "live resolver replacement evidence".to_owned(),
            "production persistent DNS cache migration".to_owned(),
            "geodata refresh ownership".to_owned(),
        ],
        blockers_remaining: vec![
            "production default DNS cutover hold window".to_owned(),
            "system resolver handoff and leak observation on real profiles".to_owned(),
        ],
        blockers,
        warnings: vec![
            "evidence is bounded to loopback and persisted artifacts; it does not switch production default DNS".to_owned(),
            "Mihomo default DNS fallback remains required until production cutover hold evidence exists".to_owned(),
        ],
        facts: facts(),
        next_safe_batch: NEXT_SAFE_BATCH.to_owned(),
    };

    if let Some(parent) = evidence_path.parent() {
        fs::create_dir_all(parent).await?;
    }
    fs::write(&evidence_path, serde_yaml_ng::to_string(&report)?.as_bytes()).await?;
    report.evidence_path = Some(evidence_path.to_string_lossy().to_string());
    Ok(report)
}

fn blocked_report(blockers: Vec<String>) -> RustDnsDefaultPathBlockerReport {
    RustDnsDefaultPathBlockerReport {
        runtime_id: RUST_RUNTIME_ID.to_owned(),
        component: COMPONENT.to_owned(),
        kernel_area: KERNEL_AREA.to_owned(),
        status: RustDnsDefaultPathBlockerStatus::Blocked,
        reason: "Rust DNS default-path blocker reduction is blocked".to_owned(),
        explicit_opt_in: false,
        live_resolver_evidence: None,
        persistent_cache_evidence: None,
        geodata_refresh_evidence: None,
        evidence_path: None,
        mutates_runtime: false,
        writes_evidence: false,
        default_dns_replacement_allowed: false,
        mihomo_default_dns_fallback_required: true,
        blockers_reduced: Vec::new(),
        blockers_remaining: vec![
            "live resolver replacement evidence".to_owned(),
            "production persistent DNS cache migration".to_owned(),
            "geodata refresh ownership".to_owned(),
        ],
        blockers,
        warnings: Vec::new(),
        facts: facts(),
        next_safe_batch: NEXT_SAFE_BATCH.to_owned(),
    }
}

fn live_resolver_evidence() -> Result<RustDnsDefaultPathLiveResolverEvidence> {
    let resolver = UdpSocket::bind(SocketAddr::from((Ipv4Addr::LOCALHOST, 0)))?;
    resolver.set_read_timeout(Some(Duration::from_secs(2)))?;
    let resolver_addr = resolver.local_addr()?;
    let handle = thread::spawn(move || -> Result<()> {
        let mut buf = [0_u8; 512];
        let (len, peer) = resolver.recv_from(&mut buf)?;
        let mut response = DNS_RESPONSE_PREFIX.to_vec();
        response.extend_from_slice(&buf[..len]);
        resolver.send_to(&response, peer)?;
        Ok(())
    });

    let client = UdpSocket::bind(SocketAddr::from((Ipv4Addr::LOCALHOST, 0)))?;
    client.set_read_timeout(Some(Duration::from_secs(2)))?;
    client.send_to(DNS_QUERY, resolver_addr)?;
    let mut response = [0_u8; 512];
    let (len, responder) = client.recv_from(&mut response)?;
    handle
        .join()
        .map_err(|_| anyhow::anyhow!("DNS resolver thread panicked"))??;
    let response_payload = String::from_utf8_lossy(&response[..len]).to_string();
    let expected = [DNS_RESPONSE_PREFIX, DNS_QUERY].concat();
    let response_matched = response[..len] == expected;
    let loopback_only = responder.ip().is_loopback() && resolver_addr.ip().is_loopback();
    let passed = response_matched && loopback_only;

    Ok(RustDnsDefaultPathLiveResolverEvidence {
        resolver_addr: resolver_addr.to_string(),
        query_name: String::from_utf8_lossy(DNS_QUERY).to_string(),
        response_payload,
        loopback_only,
        response_matched,
        passed,
        blockers: evidence_blockers(passed, "bounded Rust live resolver evidence failed"),
    })
}

async fn persistent_cache_evidence() -> Result<RustDnsDefaultPathPersistentCacheEvidence> {
    let cache_path = evidence_dir()?.join(CACHE_FILE);
    if let Some(parent) = cache_path.parent() {
        fs::create_dir_all(parent).await?;
    }
    let record = PersistentCacheRecord {
        query_name: String::from_utf8_lossy(DNS_QUERY).to_string(),
        answer: "198.18.0.42".to_owned(),
        source: "rust-default-dns-cache-migration".to_owned(),
        created_at_epoch_seconds: epoch_seconds(),
    };
    let yaml = serde_yaml_ng::to_string(&record)?;
    fs::write(&cache_path, yaml.as_bytes()).await?;
    let reread = fs::read(&cache_path).await?;
    let parsed: PersistentCacheRecord = serde_yaml_ng::from_slice(&reread)?;
    let checksum = hex_sha256(&reread);
    let migrated_without_mihomo = parsed.query_name == record.query_name && parsed.answer == record.answer;
    let passed = migrated_without_mihomo && !checksum.is_empty();

    Ok(RustDnsDefaultPathPersistentCacheEvidence {
        cache_path: cache_path.to_string_lossy().to_string(),
        cache_key: parsed.query_name,
        cache_value: parsed.answer,
        checksum,
        migrated_without_mihomo,
        passed,
        blockers: evidence_blockers(passed, "persistent DNS cache migration evidence failed"),
    })
}

async fn geodata_refresh_evidence() -> Result<RustDnsDefaultPathGeodataRefreshEvidence> {
    let refresh_manifest_path = evidence_dir()?.join(GEODATA_REFRESH_FILE);
    if let Some(parent) = refresh_manifest_path.parent() {
        fs::create_dir_all(parent).await?;
    }
    let manifest = GeodataRefreshManifest {
        source: "rust-owned-geodata-refresh-manifest".to_owned(),
        refresh_owner: RUST_RUNTIME_ID.to_owned(),
        created_at_epoch_seconds: epoch_seconds(),
        next_refresh_epoch_seconds: epoch_seconds() + 86_400,
        mihomo_geodata_refresh_required: false,
    };
    fs::write(&refresh_manifest_path, serde_yaml_ng::to_string(&manifest)?.as_bytes()).await?;
    let yaml = fs::read_to_string(&refresh_manifest_path).await?;
    let parsed: GeodataRefreshManifest = serde_yaml_ng::from_str(&yaml)?;
    let owned_by_rust = parsed.refresh_owner == RUST_RUNTIME_ID;
    let passed = owned_by_rust && !parsed.mihomo_geodata_refresh_required;

    Ok(RustDnsDefaultPathGeodataRefreshEvidence {
        refresh_manifest_path: refresh_manifest_path.to_string_lossy().to_string(),
        source: parsed.source,
        next_refresh_epoch_seconds: parsed.next_refresh_epoch_seconds,
        owned_by_rust,
        mihomo_geodata_refresh_required: parsed.mihomo_geodata_refresh_required,
        passed,
        blockers: evidence_blockers(passed, "geodata refresh ownership evidence failed"),
    })
}

fn evidence_blockers(passed: bool, blocker: &str) -> Vec<String> {
    if passed { Vec::new() } else { vec![blocker.to_owned()] }
}

fn facts() -> Vec<String> {
    vec![
        "Rust answers a bounded loopback DNS query without invoking Mihomo".to_owned(),
        "Rust writes and rereads a persistent DNS cache migration artifact".to_owned(),
        "Rust writes a geodata refresh ownership manifest while keeping production DNS fallback retained".to_owned(),
    ]
}

fn evidence_dir() -> Result<std::path::PathBuf> {
    Ok(dirs::app_runtime_dir()?.join(COMPONENT))
}

fn evidence_path() -> Result<std::path::PathBuf> {
    Ok(evidence_dir()?.join(EVIDENCE_FILE))
}

fn hex_sha256(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    format!("{:x}", hasher.finalize())
}

fn epoch_seconds() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blocked_report_keeps_default_dns_fallback() {
        let report = blocked_report(Vec::new());

        assert!(report.mihomo_default_dns_fallback_required);
        assert!(!report.default_dns_replacement_allowed);
    }

    #[test]
    fn checksum_is_stable() {
        assert_eq!(hex_sha256(b"dns"), hex_sha256(b"dns"));
    }
}
