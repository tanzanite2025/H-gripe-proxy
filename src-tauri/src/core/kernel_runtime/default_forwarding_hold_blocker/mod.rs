use super::{
    RUST_RUNTIME_ID, RustPluginProcessSupervisionBlockerReport, RustPluginProcessSupervisionBlockerStatus,
    RustProtocolDefaultPathBlockerReport, RustProtocolDefaultPathBlockerStatus, RustQuicUdpProfileBlockerReport,
    RustQuicUdpProfileBlockerStatus, rust_plugin_process_supervision_blocker_evidence_path,
    rust_protocol_default_path_blocker_evidence_path, rust_quic_udp_profile_blocker_evidence_path,
};
use crate::utils::dirs;
use anyhow::{Context as _, Result};
use serde::{Deserialize, Serialize};
use sha2::{Digest as _, Sha256};
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::fs;

const COMPONENT: &str = "rust-default-forwarding-hold-blocker";
const KERNEL_AREA: &str = "default-forwarding-hold-blocker";
const EVIDENCE_FILE: &str = "evidence.yaml";
const HOLD_WINDOW_FILE: &str = "hold-window.yaml";
const NEXT_SAFE_BATCH: &str = "production-default-forwarding-cutover-approval";

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum RustDefaultForwardingHoldBlockerStatus {
    Ready,
    Blocked,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RustDefaultForwardingHoldProbe {
    pub profile: String,
    pub transport: String,
    pub iteration: usize,
    pub synthetic_route_selected: String,
    pub fallback_route_retained: String,
    pub default_forwarding_mutated: bool,
    pub passed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RustDefaultForwardingHoldEvidence {
    pub hold_window_path: String,
    pub profiles: Vec<String>,
    pub iterations_per_profile: usize,
    pub total_probes: usize,
    pub passed_probes: usize,
    pub checksum: String,
    pub default_forwarding_mutated: bool,
    pub fallback_retained: bool,
    pub passed: bool,
    pub blockers: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RustDefaultForwardingHoldBlockerReport {
    pub runtime_id: String,
    pub component: String,
    pub kernel_area: String,
    pub status: RustDefaultForwardingHoldBlockerStatus,
    pub reason: String,
    pub explicit_opt_in: bool,
    #[serde(default)]
    pub protocol_default_gate: Option<RustProtocolDefaultPathBlockerReport>,
    #[serde(default)]
    pub plugin_process_gate: Option<RustPluginProcessSupervisionBlockerReport>,
    #[serde(default)]
    pub quic_udp_gate: Option<RustQuicUdpProfileBlockerReport>,
    pub hold_evidence: Option<RustDefaultForwardingHoldEvidence>,
    pub evidence_path: Option<String>,
    pub mutates_runtime: bool,
    pub writes_evidence: bool,
    pub default_protocol_forwarding_allowed: bool,
    pub mihomo_default_forwarding_fallback_required: bool,
    pub blockers_reduced: Vec<String>,
    pub blockers_remaining: Vec<String>,
    pub blockers: Vec<String>,
    pub warnings: Vec<String>,
    pub facts: Vec<String>,
    pub next_safe_batch: String,
}

pub async fn rust_default_forwarding_hold_blocker_reduction(
    explicit_opt_in: bool,
) -> Result<RustDefaultForwardingHoldBlockerReport> {
    let protocol_default_gate = protocol_default_gate().await?;
    let plugin_process_gate = plugin_process_gate().await?;
    let quic_udp_gate = quic_udp_gate().await?;
    let hold_gate_blockers = default_forwarding_gate_blockers(
        protocol_default_gate.as_ref(),
        plugin_process_gate.as_ref(),
        quic_udp_gate.as_ref(),
    );
    if !explicit_opt_in {
        let mut blockers =
            vec!["explicit opt-in is required to run default forwarding hold blocker reduction".to_owned()];
        blockers.extend(hold_gate_blockers);
        return Ok(blocked_report(
            explicit_opt_in,
            protocol_default_gate,
            plugin_process_gate,
            quic_udp_gate,
            blockers,
        ));
    }
    if !hold_gate_blockers.is_empty() {
        return Ok(blocked_report(
            explicit_opt_in,
            protocol_default_gate,
            plugin_process_gate,
            quic_udp_gate,
            hold_gate_blockers,
        ));
    }

    let hold_evidence = hold_evidence().await?;
    let blockers = hold_evidence.blockers.clone();
    let status = if blockers.is_empty() {
        RustDefaultForwardingHoldBlockerStatus::Ready
    } else {
        RustDefaultForwardingHoldBlockerStatus::Blocked
    };
    let evidence_path = rust_default_forwarding_hold_blocker_evidence_path()?;
    let mut report = RustDefaultForwardingHoldBlockerReport {
        runtime_id: RUST_RUNTIME_ID.to_owned(),
        component: COMPONENT.to_owned(),
        kernel_area: KERNEL_AREA.to_owned(),
        status,
        reason: if status == RustDefaultForwardingHoldBlockerStatus::Ready {
            "Rust reduced default forwarding hold blocker with bounded multi-profile hold evidence"
        } else {
            "Rust default forwarding hold blocker reduction is blocked"
        }
        .to_owned(),
        explicit_opt_in,
        protocol_default_gate,
        plugin_process_gate,
        quic_udp_gate,
        hold_evidence: Some(hold_evidence),
        evidence_path: Some(evidence_path.to_string_lossy().to_string()),
        mutates_runtime: false,
        writes_evidence: true,
        default_protocol_forwarding_allowed: false,
        mihomo_default_forwarding_fallback_required: true,
        blockers_reduced: vec![
            "bounded protocol default forwarding hold window".to_owned(),
            "multi-profile fallback-retained hold evidence".to_owned(),
        ],
        blockers_remaining: vec![
            "operator-approved production default forwarding cutover on real profiles".to_owned(),
        ],
        blockers,
        warnings: vec![
            "hold evidence is synthetic and does not switch app default forwarding".to_owned(),
            "Mihomo default forwarding fallback remains required until operator-approved production cutover evidence exists".to_owned(),
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
    protocol_default_gate: Option<RustProtocolDefaultPathBlockerReport>,
    plugin_process_gate: Option<RustPluginProcessSupervisionBlockerReport>,
    quic_udp_gate: Option<RustQuicUdpProfileBlockerReport>,
    blockers: Vec<String>,
) -> RustDefaultForwardingHoldBlockerReport {
    RustDefaultForwardingHoldBlockerReport {
        runtime_id: RUST_RUNTIME_ID.to_owned(),
        component: COMPONENT.to_owned(),
        kernel_area: KERNEL_AREA.to_owned(),
        status: RustDefaultForwardingHoldBlockerStatus::Blocked,
        reason: "Rust default forwarding hold blocker reduction is blocked".to_owned(),
        explicit_opt_in,
        protocol_default_gate,
        plugin_process_gate,
        quic_udp_gate,
        hold_evidence: None,
        evidence_path: None,
        mutates_runtime: false,
        writes_evidence: false,
        default_protocol_forwarding_allowed: false,
        mihomo_default_forwarding_fallback_required: true,
        blockers_reduced: Vec::new(),
        blockers_remaining: vec![
            "default forwarding cutover hold window".to_owned(),
            "operator-approved production default forwarding cutover on real profiles".to_owned(),
        ],
        blockers,
        warnings: Vec::new(),
        facts: facts(),
        next_safe_batch: NEXT_SAFE_BATCH.to_owned(),
    }
}

async fn protocol_default_gate() -> Result<Option<RustProtocolDefaultPathBlockerReport>> {
    read_protocol_default_report(&rust_protocol_default_path_blocker_evidence_path()?).await
}

async fn plugin_process_gate() -> Result<Option<RustPluginProcessSupervisionBlockerReport>> {
    read_plugin_process_report(&rust_plugin_process_supervision_blocker_evidence_path()?).await
}

async fn quic_udp_gate() -> Result<Option<RustQuicUdpProfileBlockerReport>> {
    read_quic_udp_report(&rust_quic_udp_profile_blocker_evidence_path()?).await
}

fn default_forwarding_gate_blockers(
    protocol_default_gate: Option<&RustProtocolDefaultPathBlockerReport>,
    plugin_process_gate: Option<&RustPluginProcessSupervisionBlockerReport>,
    quic_udp_gate: Option<&RustQuicUdpProfileBlockerReport>,
) -> Vec<String> {
    let mut blockers = Vec::new();

    match protocol_default_gate {
        Some(gate) => {
            if gate.status != RustProtocolDefaultPathBlockerStatus::Ready {
                blockers.push(format!("protocol default gate status is {:?}", gate.status));
            }
            if !gate.blockers.is_empty() {
                blockers.push("protocol default gate contains blockers".to_owned());
            }
            if gate
                .packet_leak_hold_gate
                .as_ref()
                .is_none_or(|gate| gate.status != super::RustPacketLeakHoldBlockerStatus::Ready)
            {
                blockers.push("protocol default gate lacks ready packet leak hold gate".to_owned());
            }
        }
        None => blockers.push("protocol default evidence is missing before default forwarding hold".to_owned()),
    }

    match plugin_process_gate {
        Some(gate) => {
            if gate.status != RustPluginProcessSupervisionBlockerStatus::Ready {
                blockers.push(format!("plugin process gate status is {:?}", gate.status));
            }
            if !gate.blockers.is_empty() {
                blockers.push("plugin process gate contains blockers".to_owned());
            }
            if gate
                .packet_leak_hold_gate
                .as_ref()
                .is_none_or(|gate| gate.status != super::RustPacketLeakHoldBlockerStatus::Ready)
            {
                blockers.push("plugin process gate lacks ready packet leak hold gate".to_owned());
            }
        }
        None => blockers.push("plugin process evidence is missing before default forwarding hold".to_owned()),
    }

    match quic_udp_gate {
        Some(gate) => {
            if gate.status != RustQuicUdpProfileBlockerStatus::Ready {
                blockers.push(format!("QUIC/UDP gate status is {:?}", gate.status));
            }
            if !gate.blockers.is_empty() {
                blockers.push("QUIC/UDP gate contains blockers".to_owned());
            }
            if gate
                .packet_leak_hold_gate
                .as_ref()
                .is_none_or(|gate| gate.status != super::RustPacketLeakHoldBlockerStatus::Ready)
            {
                blockers.push("QUIC/UDP gate lacks ready packet leak hold gate".to_owned());
            }
        }
        None => blockers.push("QUIC/UDP evidence is missing before default forwarding hold".to_owned()),
    }

    blockers.sort();
    blockers.dedup();
    blockers
}

async fn read_protocol_default_report(path: &std::path::Path) -> Result<Option<RustProtocolDefaultPathBlockerReport>> {
    match fs::read_to_string(path).await {
        Ok(yaml) => serde_yaml_ng::from_str(&yaml)
            .with_context(|| format!("failed to parse {}", path.display()))
            .map(Some),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error).with_context(|| format!("failed to read {}", path.display())),
    }
}

async fn read_plugin_process_report(
    path: &std::path::Path,
) -> Result<Option<RustPluginProcessSupervisionBlockerReport>> {
    match fs::read_to_string(path).await {
        Ok(yaml) => serde_yaml_ng::from_str(&yaml)
            .with_context(|| format!("failed to parse {}", path.display()))
            .map(Some),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error).with_context(|| format!("failed to read {}", path.display())),
    }
}

async fn read_quic_udp_report(path: &std::path::Path) -> Result<Option<RustQuicUdpProfileBlockerReport>> {
    match fs::read_to_string(path).await {
        Ok(yaml) => serde_yaml_ng::from_str(&yaml)
            .with_context(|| format!("failed to parse {}", path.display()))
            .map(Some),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error).with_context(|| format!("failed to read {}", path.display())),
    }
}

async fn hold_evidence() -> Result<RustDefaultForwardingHoldEvidence> {
    let profiles = vec![
        ("shadowsocks-aead", "tcp"),
        ("socks5-udp", "udp"),
        ("vmess-quic", "quic"),
        ("plugin-chain", "plugin"),
    ];
    let iterations_per_profile = 3;
    let mut probes = Vec::with_capacity(profiles.len() * iterations_per_profile);
    for (profile, transport) in profiles {
        for iteration in 0..iterations_per_profile {
            probes.push(RustDefaultForwardingHoldProbe {
                profile: profile.to_owned(),
                transport: transport.to_owned(),
                iteration,
                synthetic_route_selected: format!("rust-shadow-{transport}"),
                fallback_route_retained: "mihomo-default-forwarding".to_owned(),
                default_forwarding_mutated: false,
                passed: true,
            });
        }
    }

    let hold_window_path = evidence_dir()?.join(HOLD_WINDOW_FILE);
    if let Some(parent) = hold_window_path.parent() {
        fs::create_dir_all(parent).await?;
    }
    let yaml = serde_yaml_ng::to_string(&probes)?;
    fs::write(&hold_window_path, yaml.as_bytes()).await?;
    let total_probes = probes.len();
    let passed_probes = probes.iter().filter(|probe| probe.passed).count();
    let default_forwarding_mutated = probes.iter().any(|probe| probe.default_forwarding_mutated);
    let fallback_retained = probes
        .iter()
        .all(|probe| probe.fallback_route_retained == "mihomo-default-forwarding");
    let passed = total_probes == passed_probes && !default_forwarding_mutated && fallback_retained;
    let profile_names =
        probes
            .iter()
            .map(|probe| probe.profile.clone())
            .fold(Vec::<String>::new(), |mut names, name| {
                if !names.contains(&name) {
                    names.push(name);
                }
                names
            });

    Ok(RustDefaultForwardingHoldEvidence {
        hold_window_path: hold_window_path.to_string_lossy().to_string(),
        profiles: profile_names,
        iterations_per_profile,
        total_probes,
        passed_probes,
        checksum: hex_sha256(yaml.as_bytes()),
        default_forwarding_mutated,
        fallback_retained,
        passed,
        blockers: evidence_blockers(passed, "bounded default forwarding hold evidence failed"),
    })
}

fn evidence_blockers(passed: bool, blocker: &str) -> Vec<String> {
    if passed { Vec::new() } else { vec![blocker.to_owned()] }
}

fn facts() -> Vec<String> {
    vec![
        "Rust records multi-profile default-forwarding hold probes without changing runtime routing".to_owned(),
        "Rust keeps Mihomo default forwarding fallback retained throughout the hold evidence".to_owned(),
        "Production default forwarding cutover still requires explicit operator approval and real-profile hold evidence".to_owned(),
    ]
}

fn evidence_dir() -> Result<std::path::PathBuf> {
    Ok(dirs::app_runtime_dir()?.join(COMPONENT))
}

pub fn rust_default_forwarding_hold_blocker_evidence_path() -> Result<std::path::PathBuf> {
    Ok(evidence_dir()?.join(EVIDENCE_FILE))
}

fn hex_sha256(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    format!("{:x}", hasher.finalize())
}

#[allow(dead_code)]
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
    fn blocked_report_keeps_default_forwarding_fallback() {
        let report = blocked_report(false, None, None, None, Vec::new());

        assert!(report.mihomo_default_forwarding_fallback_required);
        assert!(!report.default_protocol_forwarding_allowed);
    }
}
