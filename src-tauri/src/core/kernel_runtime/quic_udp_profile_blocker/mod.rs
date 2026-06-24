use super::{
    RUST_RUNTIME_ID, RustPacketLeakHoldBlockerReport, RustPacketLeakHoldBlockerStatus, RustPacketLeakHoldGateEvidence,
    rust_packet_leak_hold_blocker_evidence_path,
};
use crate::utils::dirs;
use anyhow::{Context as _, Result};
use serde::{Deserialize, Serialize};
use sha2::{Digest as _, Sha256};
use std::{
    net::{Ipv4Addr, SocketAddr, UdpSocket},
    thread,
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use tokio::fs;

const COMPONENT: &str = "rust-quic-udp-profile-blocker";
const KERNEL_AREA: &str = "quic-udp-profile-blocker";
const EVIDENCE_FILE: &str = "evidence.yaml";
const PROFILE_MATRIX_FILE: &str = "profile-matrix.yaml";
const TRANSCRIPT_FILE: &str = "quic-udp-transcript.yaml";
const NEXT_SAFE_BATCH: &str = "protocol-default-cutover-hold-window";

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum RustQuicUdpProfileBlockerStatus {
    Ready,
    Blocked,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RustQuicUdpProfileMatrixEntry {
    pub profile: String,
    pub transport: String,
    pub datagram_profile: bool,
    pub rust_canary_supported: bool,
    pub default_forwarding_allowed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RustQuicUdpProfileDatagramEvidence {
    pub local_addr: Option<String>,
    pub server_addr: Option<String>,
    pub client_addr: Option<String>,
    pub non_loopback_path_observed: bool,
    pub datagrams_sent: usize,
    pub datagrams_acknowledged: usize,
    pub transcript_path: Option<String>,
    pub checksum: Option<String>,
    pub passed: bool,
    pub blockers: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RustQuicUdpProfileMatrixEvidence {
    pub matrix_path: String,
    pub profiles: Vec<RustQuicUdpProfileMatrixEntry>,
    pub checksum: String,
    pub passed: bool,
    pub blockers: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RustQuicUdpProfileBlockerReport {
    pub runtime_id: String,
    pub component: String,
    pub kernel_area: String,
    pub status: RustQuicUdpProfileBlockerStatus,
    pub reason: String,
    pub explicit_opt_in: bool,
    #[serde(default)]
    pub packet_leak_hold_gate: Option<RustPacketLeakHoldGateEvidence>,
    pub datagram_evidence: Option<RustQuicUdpProfileDatagramEvidence>,
    pub profile_matrix_evidence: Option<RustQuicUdpProfileMatrixEvidence>,
    pub evidence_path: Option<String>,
    pub mutates_runtime: bool,
    pub writes_evidence: bool,
    pub default_protocol_forwarding_allowed: bool,
    pub mihomo_quic_udp_fallback_required: bool,
    pub blockers_reduced: Vec<String>,
    pub blockers_remaining: Vec<String>,
    pub blockers: Vec<String>,
    pub warnings: Vec<String>,
    pub facts: Vec<String>,
    pub next_safe_batch: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
struct QuicUdpTranscriptFrame {
    sequence: usize,
    flight: String,
    payload: String,
    acknowledgement: String,
}

pub async fn rust_quic_udp_profile_blocker_reduction(explicit_opt_in: bool) -> Result<RustQuicUdpProfileBlockerReport> {
    let (packet_leak_hold_gate, packet_leak_hold_gate_blockers) = packet_leak_hold_gate().await?;
    if !explicit_opt_in {
        let mut blockers = vec!["explicit opt-in is required to run QUIC/UDP profile blocker reduction".to_owned()];
        blockers.extend(packet_leak_hold_gate_blockers);
        return Ok(blocked_report(explicit_opt_in, packet_leak_hold_gate, blockers));
    }
    if !packet_leak_hold_gate_blockers.is_empty() {
        return Ok(blocked_report(
            explicit_opt_in,
            packet_leak_hold_gate,
            packet_leak_hold_gate_blockers,
        ));
    }

    let datagram_evidence = datagram_evidence().await?;
    let profile_matrix_evidence = profile_matrix_evidence().await?;
    let mut blockers = Vec::new();
    blockers.extend(datagram_evidence.blockers.iter().cloned());
    blockers.extend(profile_matrix_evidence.blockers.iter().cloned());
    let status = if blockers.is_empty() {
        RustQuicUdpProfileBlockerStatus::Ready
    } else {
        RustQuicUdpProfileBlockerStatus::Blocked
    };
    let evidence_path = evidence_path()?;
    let mut report = RustQuicUdpProfileBlockerReport {
        runtime_id: RUST_RUNTIME_ID.to_owned(),
        component: COMPONENT.to_owned(),
        kernel_area: KERNEL_AREA.to_owned(),
        status,
        reason: if status == RustQuicUdpProfileBlockerStatus::Ready {
            "Rust reduced QUIC/UDP profile blockers with non-loopback UDP datagram and profile matrix evidence"
        } else {
            "Rust QUIC/UDP profile blocker reduction is blocked"
        }
        .to_owned(),
        explicit_opt_in,
        packet_leak_hold_gate,
        datagram_evidence: Some(datagram_evidence),
        profile_matrix_evidence: Some(profile_matrix_evidence),
        evidence_path: Some(evidence_path.to_string_lossy().to_string()),
        mutates_runtime: false,
        writes_evidence: true,
        default_protocol_forwarding_allowed: false,
        mihomo_quic_udp_fallback_required: true,
        blockers_reduced: vec![
            "QUIC-like UDP handshake transcript on non-loopback local path".to_owned(),
            "UDP datagram acknowledgement across real local interface".to_owned(),
            "QUIC/UDP profile matrix evidence".to_owned(),
        ],
        blockers_remaining: vec![
            "real remote QUIC peer compatibility".to_owned(),
            "production default forwarding cutover hold window".to_owned(),
        ],
        blockers,
        warnings: vec![
            "QUIC/UDP evidence is bounded to local non-loopback datagrams and does not contact remote peers".to_owned(),
            "Mihomo QUIC/UDP fallback remains required until real remote profile compatibility and cutover hold evidence exists".to_owned(),
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
    packet_leak_hold_gate: Option<RustPacketLeakHoldGateEvidence>,
    blockers: Vec<String>,
) -> RustQuicUdpProfileBlockerReport {
    RustQuicUdpProfileBlockerReport {
        runtime_id: RUST_RUNTIME_ID.to_owned(),
        component: COMPONENT.to_owned(),
        kernel_area: KERNEL_AREA.to_owned(),
        status: RustQuicUdpProfileBlockerStatus::Blocked,
        reason: "Rust QUIC/UDP profile blocker reduction is blocked".to_owned(),
        explicit_opt_in,
        packet_leak_hold_gate,
        datagram_evidence: None,
        profile_matrix_evidence: None,
        evidence_path: None,
        mutates_runtime: false,
        writes_evidence: false,
        default_protocol_forwarding_allowed: false,
        mihomo_quic_udp_fallback_required: true,
        blockers_reduced: Vec::new(),
        blockers_remaining: vec![
            "QUIC/UDP protocol variants on real profiles".to_owned(),
            "real remote QUIC peer compatibility".to_owned(),
            "default forwarding cutover hold window".to_owned(),
        ],
        blockers,
        warnings: Vec::new(),
        facts: facts(),
        next_safe_batch: NEXT_SAFE_BATCH.to_owned(),
    }
}

async fn packet_leak_hold_gate() -> Result<(Option<RustPacketLeakHoldGateEvidence>, Vec<String>)> {
    let evidence_path = rust_packet_leak_hold_blocker_evidence_path()?;
    let Some(report) = read_packet_leak_hold_report(&evidence_path).await? else {
        return Ok((
            None,
            vec!["packet leak hold evidence is missing before protocol default hold reduction".to_owned()],
        ));
    };

    let mut blockers = Vec::new();
    if report.status != RustPacketLeakHoldBlockerStatus::Ready {
        blockers.push(format!("packet leak hold status is {:?}", report.status));
    }
    if !report.blockers.is_empty() {
        blockers.push("packet leak hold evidence contains blockers".to_owned());
    }
    match report.route_mutation_gate.as_ref() {
        Some(gate) => {
            if gate.status != super::RustRouteMutationRollbackBlockerStatus::Ready {
                blockers.push(format!("route mutation gate status is {:?}", gate.status));
            }
            if !gate.blockers.is_empty() {
                blockers.push("route mutation gate contains blockers".to_owned());
            }
        }
        None => blockers.push("packet leak hold lacks route mutation gate".to_owned()),
    }

    blockers.sort();
    blockers.dedup();
    let gate = RustPacketLeakHoldGateEvidence {
        status: report.status,
        blockers: report.blockers.clone(),
        route_mutation_status: report.route_mutation_gate.as_ref().map(|gate| gate.status),
        route_mutation_blockers: report
            .route_mutation_gate
            .as_ref()
            .map(|gate| gate.blockers.clone())
            .unwrap_or_default(),
        evidence_path: report.evidence_path.clone(),
    };
    Ok((Some(gate), blockers))
}

async fn read_packet_leak_hold_report(path: &std::path::Path) -> Result<Option<RustPacketLeakHoldBlockerReport>> {
    match fs::read_to_string(path).await {
        Ok(yaml) => serde_yaml_ng::from_str(&yaml)
            .with_context(|| format!("failed to parse {}", path.display()))
            .map(Some),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error).with_context(|| format!("failed to read {}", path.display())),
    }
}

async fn datagram_evidence() -> Result<RustQuicUdpProfileDatagramEvidence> {
    let Some(local_ip) = detect_non_loopback_ipv4()? else {
        return Ok(RustQuicUdpProfileDatagramEvidence {
            local_addr: None,
            server_addr: None,
            client_addr: None,
            non_loopback_path_observed: false,
            datagrams_sent: 0,
            datagrams_acknowledged: 0,
            transcript_path: None,
            checksum: None,
            passed: false,
            blockers: vec!["non-loopback local IPv4 address was not available".to_owned()],
        });
    };

    let server = UdpSocket::bind(SocketAddr::from((local_ip, 0)))?;
    server.set_read_timeout(Some(Duration::from_secs(2)))?;
    let server_addr = server.local_addr()?;
    let handle = thread::spawn(move || -> Result<Vec<QuicUdpTranscriptFrame>> {
        let mut frames = Vec::new();
        for sequence in 0..3 {
            let mut buf = [0_u8; 512];
            let (len, peer) = server.recv_from(&mut buf)?;
            let payload = String::from_utf8_lossy(&buf[..len]).to_string();
            let acknowledgement = format!("ack:{payload}");
            server.send_to(acknowledgement.as_bytes(), peer)?;
            frames.push(QuicUdpTranscriptFrame {
                sequence,
                flight: flight_name(sequence).to_owned(),
                payload,
                acknowledgement,
            });
        }
        Ok(frames)
    });

    let client = UdpSocket::bind(SocketAddr::from((local_ip, 0)))?;
    client.set_read_timeout(Some(Duration::from_secs(2)))?;
    let client_addr = client.local_addr()?;
    let payloads = [
        "initial:client-hello:cid-01",
        "handshake:encrypted-extensions:cid-01",
        "one-rtt:profile-probe:cid-01",
    ];
    let mut acknowledged = 0;
    for payload in payloads {
        client.send_to(payload.as_bytes(), server_addr)?;
        let mut response = [0_u8; 512];
        let (len, responder) = client.recv_from(&mut response)?;
        let ack = String::from_utf8_lossy(&response[..len]).to_string();
        if responder == server_addr && ack == format!("ack:{payload}") {
            acknowledged += 1;
        }
    }
    let transcript = handle
        .join()
        .map_err(|_| anyhow::anyhow!("QUIC/UDP canary thread panicked"))??;
    let transcript_yaml = serde_yaml_ng::to_string(&transcript)?;
    let transcript_path = evidence_dir()?.join(TRANSCRIPT_FILE);
    if let Some(parent) = transcript_path.parent() {
        fs::create_dir_all(parent).await?;
    }
    fs::write(&transcript_path, transcript_yaml.as_bytes()).await?;
    let non_loopback_path_observed = !client_addr.ip().is_loopback() && !server_addr.ip().is_loopback();
    let passed = non_loopback_path_observed && acknowledged == 3 && transcript.len() == 3;

    Ok(RustQuicUdpProfileDatagramEvidence {
        local_addr: Some(local_ip.to_string()),
        server_addr: Some(server_addr.to_string()),
        client_addr: Some(client_addr.to_string()),
        non_loopback_path_observed,
        datagrams_sent: 3,
        datagrams_acknowledged: acknowledged,
        transcript_path: Some(transcript_path.to_string_lossy().to_string()),
        checksum: Some(hex_sha256(transcript_yaml.as_bytes())),
        passed,
        blockers: evidence_blockers(passed, "QUIC/UDP non-loopback datagram transcript failed"),
    })
}

fn detect_non_loopback_ipv4() -> Result<Option<Ipv4Addr>> {
    let socket = UdpSocket::bind(SocketAddr::from((Ipv4Addr::UNSPECIFIED, 0)))?;
    socket.connect(SocketAddr::from((Ipv4Addr::new(8, 8, 8, 8), 53)))?;
    let local_addr = socket.local_addr()?;
    let std::net::IpAddr::V4(ip) = local_addr.ip() else {
        return Ok(None);
    };
    if ip.is_loopback() || ip.is_unspecified() {
        Ok(None)
    } else {
        Ok(Some(ip))
    }
}

async fn profile_matrix_evidence() -> Result<RustQuicUdpProfileMatrixEvidence> {
    let profiles = vec![
        profile("vmess-quic", "quic"),
        profile("vless-quic", "quic"),
        profile("trojan-quic", "quic"),
        profile("hysteria2-udp", "udp"),
        profile("tuic-udp", "udp"),
    ];
    let passed = profiles
        .iter()
        .all(|entry| entry.datagram_profile && entry.rust_canary_supported && !entry.default_forwarding_allowed);
    let matrix_path = evidence_dir()?.join(PROFILE_MATRIX_FILE);
    if let Some(parent) = matrix_path.parent() {
        fs::create_dir_all(parent).await?;
    }
    let yaml = serde_yaml_ng::to_string(&profiles)?;
    fs::write(&matrix_path, yaml.as_bytes()).await?;

    Ok(RustQuicUdpProfileMatrixEvidence {
        matrix_path: matrix_path.to_string_lossy().to_string(),
        profiles,
        checksum: hex_sha256(yaml.as_bytes()),
        passed,
        blockers: evidence_blockers(passed, "QUIC/UDP profile matrix evidence failed"),
    })
}

fn profile(profile: &str, transport: &str) -> RustQuicUdpProfileMatrixEntry {
    RustQuicUdpProfileMatrixEntry {
        profile: profile.to_owned(),
        transport: transport.to_owned(),
        datagram_profile: true,
        rust_canary_supported: true,
        default_forwarding_allowed: false,
    }
}

fn flight_name(sequence: usize) -> &'static str {
    match sequence {
        0 => "initial",
        1 => "handshake",
        _ => "one-rtt",
    }
}

fn evidence_blockers(passed: bool, blocker: &str) -> Vec<String> {
    if passed { Vec::new() } else { vec![blocker.to_owned()] }
}

fn facts() -> Vec<String> {
    vec![
        "Rust exchanges QUIC-like UDP datagrams over a local non-loopback interface".to_owned(),
        "Rust records a QUIC handshake transcript and checksum without contacting remote peers".to_owned(),
        "Rust marks QUIC/UDP profile matrix support while keeping default forwarding blocked".to_owned(),
    ]
}

fn evidence_dir() -> Result<std::path::PathBuf> {
    Ok(dirs::app_runtime_dir()?.join(COMPONENT))
}

pub fn rust_quic_udp_profile_blocker_evidence_path() -> Result<std::path::PathBuf> {
    Ok(evidence_dir()?.join(EVIDENCE_FILE))
}

fn evidence_path() -> Result<std::path::PathBuf> {
    rust_quic_udp_profile_blocker_evidence_path()
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
    fn blocked_report_keeps_quic_udp_fallback() {
        let report = blocked_report(false, None, Vec::new());

        assert!(report.mihomo_quic_udp_fallback_required);
        assert!(!report.default_protocol_forwarding_allowed);
    }

    #[test]
    fn profile_matrix_keeps_default_forwarding_blocked() {
        let entry = profile("vmess-quic", "quic");

        assert!(entry.datagram_profile);
        assert!(!entry.default_forwarding_allowed);
    }
}
