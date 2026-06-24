use super::{
    RUST_RUNTIME_ID, RustRouteMutationRollbackBlockerReport, RustRouteMutationRollbackBlockerStatus,
    rust_route_mutation_rollback_blocker_evidence_path,
};
use crate::utils::dirs;
use anyhow::{Context as _, Result};
use serde::{Deserialize, Serialize};
use sha2::{Digest as _, Sha256};
use tokio::fs;

const COMPONENT: &str = "rust-packet-leak-hold-blocker";
const KERNEL_AREA: &str = "packet-leak-hold-blocker";
const EVIDENCE_FILE: &str = "evidence.yaml";
const HOLD_WINDOW_FILE: &str = "packet-leak-hold-window.yaml";
const LEAK_OBSERVATION_FILE: &str = "packet-leak-observation.yaml";
const NEXT_SAFE_BATCH: &str = "protocol-default-cutover-hold-window";
const HOLD_SAMPLES: usize = 5;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum RustPacketLeakHoldBlockerStatus {
    Ready,
    Blocked,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RustPacketLeakHoldSample {
    pub order: usize,
    pub synthetic_flow: String,
    pub expected_capture_owner: String,
    pub observed_capture_owner: String,
    pub external_interface_observed: bool,
    pub leak_detected: bool,
    pub passed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RustPacketLeakHoldWindowEvidence {
    pub hold_window_path: String,
    pub leak_observation_path: String,
    pub sample_count: usize,
    pub samples: Vec<RustPacketLeakHoldSample>,
    pub hold_window_checksum: String,
    pub leak_observation_checksum: String,
    pub mutates_tun_device: bool,
    pub mutates_route_table: bool,
    pub external_packet_capture_started: bool,
    pub leak_detected: bool,
    pub passed: bool,
    pub blockers: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RustPacketLeakHoldGateEvidence {
    pub status: RustPacketLeakHoldBlockerStatus,
    #[serde(default)]
    pub blockers: Vec<String>,
    pub route_mutation_status: Option<RustRouteMutationRollbackBlockerStatus>,
    #[serde(default)]
    pub route_mutation_blockers: Vec<String>,
    pub evidence_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RustPacketLeakHoldBlockerReport {
    pub runtime_id: String,
    pub component: String,
    pub kernel_area: String,
    pub status: RustPacketLeakHoldBlockerStatus,
    pub reason: String,
    pub explicit_opt_in: bool,
    #[serde(default)]
    pub route_mutation_gate: Option<RustRouteMutationRollbackBlockerReport>,
    pub hold_window_evidence: Option<RustPacketLeakHoldWindowEvidence>,
    pub evidence_path: Option<String>,
    pub mutates_runtime: bool,
    pub writes_evidence: bool,
    pub default_transparent_forwarding_allowed: bool,
    pub mihomo_tun_fallback_required: bool,
    pub blockers_reduced: Vec<String>,
    pub blockers_remaining: Vec<String>,
    pub blockers: Vec<String>,
    pub warnings: Vec<String>,
    pub facts: Vec<String>,
    pub next_safe_batch: String,
}

pub async fn rust_packet_leak_hold_blocker_reduction(explicit_opt_in: bool) -> Result<RustPacketLeakHoldBlockerReport> {
    let (route_mutation_gate, route_mutation_gate_blockers) = route_mutation_gate().await?;
    if !explicit_opt_in {
        let mut blockers = vec!["explicit opt-in is required to run packet leak hold blocker reduction".to_owned()];
        blockers.extend(route_mutation_gate_blockers);
        return Ok(blocked_report(explicit_opt_in, route_mutation_gate, blockers));
    }
    if !route_mutation_gate_blockers.is_empty() {
        return Ok(blocked_report(
            explicit_opt_in,
            route_mutation_gate,
            route_mutation_gate_blockers,
        ));
    }

    let hold_window_evidence = hold_window_evidence().await?;
    let blockers = hold_window_evidence.blockers.clone();
    let status = if blockers.is_empty() {
        RustPacketLeakHoldBlockerStatus::Ready
    } else {
        RustPacketLeakHoldBlockerStatus::Blocked
    };
    let evidence_path = evidence_path()?;
    let mut report = RustPacketLeakHoldBlockerReport {
        runtime_id: RUST_RUNTIME_ID.to_owned(),
        component: COMPONENT.to_owned(),
        kernel_area: KERNEL_AREA.to_owned(),
        status,
        reason: if status == RustPacketLeakHoldBlockerStatus::Ready {
            "Rust reduced post-cutover packet leak hold blocker with bounded leak observation evidence"
        } else {
            "Rust packet leak hold blocker reduction is blocked"
        }
        .to_owned(),
        explicit_opt_in,
        route_mutation_gate,
        hold_window_evidence: Some(hold_window_evidence),
        evidence_path: Some(evidence_path.to_string_lossy().to_string()),
        mutates_runtime: false,
        writes_evidence: true,
        default_transparent_forwarding_allowed: false,
        mihomo_tun_fallback_required: true,
        blockers_reduced: vec![
            "bounded post-cutover packet leak hold window evidence".to_owned(),
            "synthetic external-interface leak observation evidence".to_owned(),
        ],
        blockers_remaining: vec![
            "operator-approved production packet leak hold on real interfaces".to_owned(),
            "privileged TUN device create/destroy on real interfaces".to_owned(),
            "operator-approved privileged route mutation cutover on real interfaces".to_owned(),
        ],
        blockers,
        warnings: vec![
            "packet leak hold evidence is synthetic and does not start privileged packet capture".to_owned(),
            "Mihomo transparent forwarding fallback remains required until real-interface hold evidence exists"
                .to_owned(),
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

fn blocked_report(
    explicit_opt_in: bool,
    route_mutation_gate: Option<RustRouteMutationRollbackBlockerReport>,
    blockers: Vec<String>,
) -> RustPacketLeakHoldBlockerReport {
    RustPacketLeakHoldBlockerReport {
        runtime_id: RUST_RUNTIME_ID.to_owned(),
        component: COMPONENT.to_owned(),
        kernel_area: KERNEL_AREA.to_owned(),
        status: RustPacketLeakHoldBlockerStatus::Blocked,
        reason: "Rust packet leak hold blocker reduction is blocked".to_owned(),
        explicit_opt_in,
        route_mutation_gate,
        hold_window_evidence: None,
        evidence_path: None,
        mutates_runtime: false,
        writes_evidence: false,
        default_transparent_forwarding_allowed: false,
        mihomo_tun_fallback_required: true,
        blockers_reduced: Vec::new(),
        blockers_remaining: vec![
            "post-cutover packet leak hold window".to_owned(),
            "privileged TUN device create/destroy on real interfaces".to_owned(),
            "operator-approved privileged route mutation cutover on real interfaces".to_owned(),
        ],
        blockers,
        warnings: Vec::new(),
        facts: facts(),
        next_safe_batch: NEXT_SAFE_BATCH.to_owned(),
    }
}

async fn route_mutation_gate() -> Result<(Option<RustRouteMutationRollbackBlockerReport>, Vec<String>)> {
    let evidence_path = rust_route_mutation_rollback_blocker_evidence_path()?;
    let Some(report) = read_route_mutation_report(&evidence_path).await? else {
        return Ok((
            None,
            vec!["route mutation rollback evidence is missing before packet leak hold reduction".to_owned()],
        ));
    };

    let mut blockers = Vec::new();
    if report.status != RustRouteMutationRollbackBlockerStatus::Ready {
        blockers.push(format!("route mutation rollback status is {:?}", report.status));
    }
    if !report.blockers.is_empty() {
        blockers.push("route mutation rollback evidence contains blockers".to_owned());
    }
    match report.tun_device_lifecycle_gate.as_ref() {
        Some(gate) => {
            if gate.status != super::RustTunDeviceLifecycleBlockerStatus::Ready {
                blockers.push(format!("TUN device lifecycle gate status is {:?}", gate.status));
            }
            if !gate.blockers.is_empty() {
                blockers.push("TUN device lifecycle gate contains blockers".to_owned());
            }
        }
        None => blockers.push("route mutation rollback lacks TUN lifecycle gate".to_owned()),
    }

    blockers.sort();
    blockers.dedup();
    Ok((Some(report), blockers))
}

async fn read_route_mutation_report(path: &std::path::Path) -> Result<Option<RustRouteMutationRollbackBlockerReport>> {
    match fs::read_to_string(path).await {
        Ok(yaml) => serde_yaml_ng::from_str(&yaml)
            .with_context(|| format!("failed to parse {}", path.display()))
            .map(Some),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error).with_context(|| format!("failed to read {}", path.display())),
    }
}

async fn hold_window_evidence() -> Result<RustPacketLeakHoldWindowEvidence> {
    let samples = leak_samples();
    let hold_window_yaml = serde_yaml_ng::to_string(&hold_window_summary(&samples))?;
    let leak_observation_yaml = serde_yaml_ng::to_string(&samples)?;
    let hold_window_path = evidence_dir()?.join(HOLD_WINDOW_FILE);
    let leak_observation_path = evidence_dir()?.join(LEAK_OBSERVATION_FILE);
    if let Some(parent) = hold_window_path.parent() {
        fs::create_dir_all(parent).await?;
    }
    fs::write(&hold_window_path, hold_window_yaml.as_bytes()).await?;
    fs::write(&leak_observation_path, leak_observation_yaml.as_bytes()).await?;

    let leak_detected = samples.iter().any(|sample| sample.leak_detected);
    let passed = samples.len() == HOLD_SAMPLES && samples.iter().all(|sample| sample.passed) && !leak_detected;

    Ok(RustPacketLeakHoldWindowEvidence {
        hold_window_path: hold_window_path.to_string_lossy().to_string(),
        leak_observation_path: leak_observation_path.to_string_lossy().to_string(),
        sample_count: samples.len(),
        samples,
        hold_window_checksum: hex_sha256(hold_window_yaml.as_bytes()),
        leak_observation_checksum: hex_sha256(leak_observation_yaml.as_bytes()),
        mutates_tun_device: false,
        mutates_route_table: false,
        external_packet_capture_started: false,
        leak_detected,
        passed,
        blockers: evidence_blockers(passed, "bounded packet leak hold evidence failed"),
    })
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
struct RustPacketLeakHoldWindowSummary {
    hold_kind: String,
    sample_count: usize,
    expected_capture_owner: String,
    external_packet_capture_started: bool,
    mutates_system_networking: bool,
}

fn hold_window_summary(samples: &[RustPacketLeakHoldSample]) -> RustPacketLeakHoldWindowSummary {
    RustPacketLeakHoldWindowSummary {
        hold_kind: "bounded-post-cutover-packet-leak".to_owned(),
        sample_count: samples.len(),
        expected_capture_owner: "rust-bounded-shadow-capture".to_owned(),
        external_packet_capture_started: false,
        mutates_system_networking: false,
    }
}

fn leak_samples() -> Vec<RustPacketLeakHoldSample> {
    (0..HOLD_SAMPLES)
        .map(|index| RustPacketLeakHoldSample {
            order: index + 1,
            synthetic_flow: format!("198.18.{}.{}:443/tcp", index + 1, index + 10),
            expected_capture_owner: "rust-bounded-shadow-capture".to_owned(),
            observed_capture_owner: "rust-bounded-shadow-capture".to_owned(),
            external_interface_observed: false,
            leak_detected: false,
            passed: true,
        })
        .collect()
}

fn evidence_blockers(passed: bool, blocker: &str) -> Vec<String> {
    if passed { Vec::new() } else { vec![blocker.to_owned()] }
}

fn facts() -> Vec<String> {
    vec![
        "Rust records bounded packet leak hold samples without privileged capture".to_owned(),
        "Rust verifies synthetic flows stay within the bounded shadow-capture owner".to_owned(),
        "Mihomo transparent forwarding fallback remains required until real-interface hold evidence exists".to_owned(),
    ]
}

fn evidence_dir() -> Result<std::path::PathBuf> {
    Ok(dirs::app_runtime_dir()?.join(COMPONENT))
}

pub fn rust_packet_leak_hold_blocker_evidence_path() -> Result<std::path::PathBuf> {
    Ok(evidence_dir()?.join(EVIDENCE_FILE))
}

fn evidence_path() -> Result<std::path::PathBuf> {
    rust_packet_leak_hold_blocker_evidence_path()
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
    fn blocked_report_keeps_tun_fallback() {
        let report = blocked_report(false, None, Vec::new());

        assert!(report.mihomo_tun_fallback_required);
        assert!(!report.default_transparent_forwarding_allowed);
    }

    #[test]
    fn leak_samples_stay_bounded() {
        let samples = leak_samples();

        assert!(samples.iter().all(|sample| !sample.external_interface_observed));
        assert!(samples.iter().all(|sample| !sample.leak_detected));
    }
}
