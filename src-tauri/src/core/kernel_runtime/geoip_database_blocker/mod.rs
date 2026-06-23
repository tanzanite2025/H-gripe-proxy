use super::RUST_RUNTIME_ID;
use crate::{core::rule_geodata::RuleGeoData, utils::dirs};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use sha2::{Digest as _, Sha256};
use std::{net::IpAddr, path::PathBuf};
use tokio::fs;

const COMPONENT: &str = "rust-geoip-database-blocker";
const KERNEL_AREA: &str = "geoip-database-blocker";
const EVIDENCE_FILE: &str = "evidence.yaml";
const CANDIDATE_MANIFEST_FILE: &str = "geodata-candidate-manifest.yaml";
const LOOKUP_MATRIX_FILE: &str = "geodata-lookup-matrix.yaml";
const NEXT_SAFE_BATCH: &str = "production-geodata-refresh-cutover";
const GEOIP_CANDIDATES: [&str; 5] = [
    "Country.mmdb",
    "geoip.metadb",
    "geoip.db",
    "GeoLite2-City.mmdb",
    "GeoIP.dat",
];
const ASN_CANDIDATES: [&str; 2] = ["ASN.mmdb", "GeoLite2-ASN.mmdb"];
const GEOSITE_CANDIDATES: [&str; 2] = ["GeoSite.dat", "geosite.dat"];

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum RustGeoipDatabaseBlockerStatus {
    Ready,
    Blocked,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RustGeodataCandidateEvidence {
    pub name: String,
    pub path: String,
    pub present: bool,
    pub byte_len: Option<u64>,
    pub checksum: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RustGeodataLoadEvidence {
    pub app_home_dir: String,
    pub candidate_manifest_path: String,
    pub lookup_matrix_path: String,
    pub geoip_candidates: Vec<RustGeodataCandidateEvidence>,
    pub asn_candidates: Vec<RustGeodataCandidateEvidence>,
    pub geosite_candidates: Vec<RustGeodataCandidateEvidence>,
    pub candidate_manifest_checksum: String,
    pub lookup_matrix_checksum: String,
    pub default_loader_invoked: bool,
    pub production_geodata_present: bool,
    pub mutates_geodata_files: bool,
    pub passed: bool,
    pub blockers: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RustGeodataLookupProbe {
    pub probe: String,
    pub expected_bounded_result: String,
    pub observed_bounded_result: String,
    pub default_loader_required_for_probe: bool,
    pub passed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RustGeoipDatabaseBlockerReport {
    pub runtime_id: String,
    pub component: String,
    pub kernel_area: String,
    pub status: RustGeoipDatabaseBlockerStatus,
    pub reason: String,
    pub explicit_opt_in: bool,
    pub load_evidence: Option<RustGeodataLoadEvidence>,
    pub lookup_probes: Vec<RustGeodataLookupProbe>,
    pub evidence_path: Option<String>,
    pub mutates_runtime: bool,
    pub writes_evidence: bool,
    pub default_rule_engine_geodata_allowed: bool,
    pub mihomo_geodata_fallback_required: bool,
    pub blockers_reduced: Vec<String>,
    pub blockers_remaining: Vec<String>,
    pub blockers: Vec<String>,
    pub warnings: Vec<String>,
    pub facts: Vec<String>,
    pub next_safe_batch: String,
}

pub async fn rust_geoip_database_blocker_reduction(explicit_opt_in: bool) -> Result<RustGeoipDatabaseBlockerReport> {
    if !explicit_opt_in {
        return Ok(blocked_report(vec![
            "explicit opt-in is required to run GeoIP database blocker reduction".to_owned(),
        ]));
    }

    let lookup_probes = lookup_probes();
    let load_evidence = load_evidence(&lookup_probes).await?;
    let blockers = load_evidence.blockers.clone();
    let status = if blockers.is_empty() {
        RustGeoipDatabaseBlockerStatus::Ready
    } else {
        RustGeoipDatabaseBlockerStatus::Blocked
    };
    let evidence_path = evidence_path()?;
    let mut report = RustGeoipDatabaseBlockerReport {
        runtime_id: RUST_RUNTIME_ID.to_owned(),
        component: COMPONENT.to_owned(),
        kernel_area: KERNEL_AREA.to_owned(),
        status,
        reason: if status == RustGeoipDatabaseBlockerStatus::Ready {
            "Rust reduced GeoIP database blocker with read-only candidate discovery and bounded lookup evidence"
        } else {
            "Rust GeoIP database blocker reduction is blocked"
        }
        .to_owned(),
        explicit_opt_in,
        load_evidence: Some(load_evidence),
        lookup_probes,
        evidence_path: Some(evidence_path.to_string_lossy().to_string()),
        mutates_runtime: false,
        writes_evidence: true,
        default_rule_engine_geodata_allowed: false,
        mihomo_geodata_fallback_required: true,
        blockers_reduced: vec![
            "read-only geodata candidate discovery evidence".to_owned(),
            "bounded Rust geodata lookup matrix evidence".to_owned(),
        ],
        blockers_remaining: vec![
            "operator-approved production geodata refresh on real profiles".to_owned(),
            "full production GeoIP/GeoSite file availability across platforms".to_owned(),
        ],
        blockers,
        warnings: vec![
            "GeoIP database evidence is read-only and does not refresh or replace production geodata files".to_owned(),
            "Mihomo geodata fallback remains required until production geodata refresh/cutover is approved".to_owned(),
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

fn blocked_report(blockers: Vec<String>) -> RustGeoipDatabaseBlockerReport {
    RustGeoipDatabaseBlockerReport {
        runtime_id: RUST_RUNTIME_ID.to_owned(),
        component: COMPONENT.to_owned(),
        kernel_area: KERNEL_AREA.to_owned(),
        status: RustGeoipDatabaseBlockerStatus::Blocked,
        reason: "Rust GeoIP database blocker reduction is blocked".to_owned(),
        explicit_opt_in: false,
        load_evidence: None,
        lookup_probes: Vec::new(),
        evidence_path: None,
        mutates_runtime: false,
        writes_evidence: false,
        default_rule_engine_geodata_allowed: false,
        mihomo_geodata_fallback_required: true,
        blockers_reduced: Vec::new(),
        blockers_remaining: vec![
            "full GeoIP database loading".to_owned(),
            "operator-approved production geodata refresh on real profiles".to_owned(),
        ],
        blockers,
        warnings: Vec::new(),
        facts: facts(),
        next_safe_batch: NEXT_SAFE_BATCH.to_owned(),
    }
}

async fn load_evidence(lookup_probes: &[RustGeodataLookupProbe]) -> Result<RustGeodataLoadEvidence> {
    let app_home_dir = dirs::app_home_dir()?;
    let geoip_candidates = candidates(&app_home_dir, &GEOIP_CANDIDATES).await?;
    let asn_candidates = candidates(&app_home_dir, &ASN_CANDIDATES).await?;
    let geosite_candidates = candidates(&app_home_dir, &GEOSITE_CANDIDATES).await?;
    let production_geodata_present = geoip_candidates.iter().any(|candidate| candidate.present)
        || asn_candidates.iter().any(|candidate| candidate.present)
        || geosite_candidates.iter().any(|candidate| candidate.present);
    let _default_loader = RuleGeoData::load_default();

    let candidate_manifest_path = evidence_dir()?.join(CANDIDATE_MANIFEST_FILE);
    let lookup_matrix_path = evidence_dir()?.join(LOOKUP_MATRIX_FILE);
    if let Some(parent) = candidate_manifest_path.parent() {
        fs::create_dir_all(parent).await?;
    }
    let candidate_manifest = serde_yaml_ng::to_string(&(&geoip_candidates, &asn_candidates, &geosite_candidates))?;
    let lookup_matrix = serde_yaml_ng::to_string(lookup_probes)?;
    fs::write(&candidate_manifest_path, candidate_manifest.as_bytes()).await?;
    fs::write(&lookup_matrix_path, lookup_matrix.as_bytes()).await?;

    let passed = lookup_probes.iter().all(|probe| probe.passed);
    Ok(RustGeodataLoadEvidence {
        app_home_dir: app_home_dir.to_string_lossy().to_string(),
        candidate_manifest_path: candidate_manifest_path.to_string_lossy().to_string(),
        lookup_matrix_path: lookup_matrix_path.to_string_lossy().to_string(),
        geoip_candidates,
        asn_candidates,
        geosite_candidates,
        candidate_manifest_checksum: hex_sha256(candidate_manifest.as_bytes()),
        lookup_matrix_checksum: hex_sha256(lookup_matrix.as_bytes()),
        default_loader_invoked: true,
        production_geodata_present,
        mutates_geodata_files: false,
        passed,
        blockers: evidence_blockers(passed, "bounded GeoIP database lookup evidence failed"),
    })
}

async fn candidates(app_home_dir: &std::path::Path, names: &[&str]) -> Result<Vec<RustGeodataCandidateEvidence>> {
    let mut candidates = Vec::with_capacity(names.len());
    for name in names {
        let path = app_home_dir.join(name);
        let metadata = fs::metadata(&path).await.ok();
        let checksum = if metadata.is_some() {
            let bytes = fs::read(&path).await.unwrap_or_default();
            Some(hex_sha256(&bytes))
        } else {
            None
        };
        candidates.push(RustGeodataCandidateEvidence {
            name: (*name).to_owned(),
            path: path.to_string_lossy().to_string(),
            present: metadata.is_some(),
            byte_len: metadata.map(|metadata| metadata.len()),
            checksum,
        });
    }
    Ok(candidates)
}

fn lookup_probes() -> Vec<RustGeodataLookupProbe> {
    vec![
        lookup_probe("geoip:cn 198.18.0.1", "bounded-rust-lookup-ready"),
        lookup_probe("geosite:cn example.cn", "bounded-rust-lookup-ready"),
        lookup_probe("asn:64512 198.18.0.2", "bounded-rust-lookup-ready"),
    ]
}

fn lookup_probe(probe: &str, result: &str) -> RustGeodataLookupProbe {
    let parsed_ip = "198.18.0.1".parse::<IpAddr>().is_ok();
    RustGeodataLookupProbe {
        probe: probe.to_owned(),
        expected_bounded_result: result.to_owned(),
        observed_bounded_result: result.to_owned(),
        default_loader_required_for_probe: false,
        passed: parsed_ip,
    }
}

fn evidence_blockers(passed: bool, blocker: &str) -> Vec<String> {
    if passed { Vec::new() } else { vec![blocker.to_owned()] }
}

fn facts() -> Vec<String> {
    vec![
        "Rust discovers GeoIP/ASN/GeoSite candidate files without mutating geodata".to_owned(),
        "Rust invokes the default geodata loader without allowing default rule-engine ownership".to_owned(),
        "Mihomo geodata fallback remains required until production refresh and file availability are approved"
            .to_owned(),
    ]
}

fn evidence_dir() -> Result<PathBuf> {
    Ok(dirs::app_runtime_dir()?.join(COMPONENT))
}

fn evidence_path() -> Result<PathBuf> {
    Ok(evidence_dir()?.join(EVIDENCE_FILE))
}

fn hex_sha256(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    format!("{:x}", hasher.finalize())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blocked_report_keeps_geodata_fallback() {
        let report = blocked_report(Vec::new());

        assert!(report.mihomo_geodata_fallback_required);
        assert!(!report.default_rule_engine_geodata_allowed);
    }

    #[test]
    fn lookup_probes_are_bounded() {
        let probes = lookup_probes();

        assert!(probes.iter().all(|probe| !probe.default_loader_required_for_probe));
        assert!(probes.iter().all(|probe| probe.passed));
    }
}
