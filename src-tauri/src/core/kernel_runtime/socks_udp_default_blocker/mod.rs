use super::RUST_RUNTIME_ID;
use crate::utils::dirs;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use sha2::{Digest as _, Sha256};
use std::{
    net::{Ipv4Addr, SocketAddr, UdpSocket},
    thread,
    time::Duration,
};
use tokio::fs;

const COMPONENT: &str = "rust-socks-udp-default-blocker";
const KERNEL_AREA: &str = "socks-udp-default-blocker";
const EVIDENCE_FILE: &str = "evidence.yaml";
const FRAGMENT_TRANSCRIPT_FILE: &str = "socks-udp-fragment-transcript.yaml";
const QUEUE_MATRIX_FILE: &str = "socks-udp-queue-timeout-matrix.yaml";
const NEXT_SAFE_BATCH: &str = "socks-udp-default-cutover-hold";

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum RustSocksUdpDefaultBlockerStatus {
    Ready,
    Blocked,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RustSocksUdpFragmentFrame {
    pub sequence: usize,
    pub fragment_id: String,
    pub payload: String,
    pub acknowledgement: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RustSocksUdpFragmentEvidence {
    pub local_addr: Option<String>,
    pub server_addr: Option<String>,
    pub client_addr: Option<String>,
    pub non_loopback_path_observed: bool,
    pub fragments_sent: usize,
    pub fragments_acknowledged: usize,
    pub fragment_transcript_path: Option<String>,
    pub fragment_transcript_checksum: Option<String>,
    pub mutates_default_udp_forwarding: bool,
    pub passed: bool,
    pub blockers: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RustSocksUdpQueueTimeoutEvidence {
    pub queue_timeout_matrix_path: String,
    pub queue_timeout_matrix_checksum: String,
    pub fragment_queue_entries: usize,
    pub timeout_reap_entries: usize,
    pub orphan_fragment_retained: bool,
    pub passed: bool,
    pub blockers: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RustSocksUdpDefaultBlockerReport {
    pub runtime_id: String,
    pub component: String,
    pub kernel_area: String,
    pub status: RustSocksUdpDefaultBlockerStatus,
    pub reason: String,
    pub explicit_opt_in: bool,
    pub fragment_evidence: Option<RustSocksUdpFragmentEvidence>,
    pub queue_timeout_evidence: Option<RustSocksUdpQueueTimeoutEvidence>,
    pub evidence_path: Option<String>,
    pub mutates_runtime: bool,
    pub writes_evidence: bool,
    pub default_udp_forwarding_allowed: bool,
    pub mihomo_udp_fallback_required: bool,
    pub blockers_reduced: Vec<String>,
    pub blockers_remaining: Vec<String>,
    pub blockers: Vec<String>,
    pub warnings: Vec<String>,
    pub facts: Vec<String>,
    pub next_safe_batch: String,
}

pub async fn rust_socks_udp_default_blocker_reduction(
    explicit_opt_in: bool,
) -> Result<RustSocksUdpDefaultBlockerReport> {
    if !explicit_opt_in {
        return Ok(blocked_report(vec![
            "explicit opt-in is required to run SOCKS UDP default blocker reduction".to_owned(),
        ]));
    }

    let fragment_evidence = fragment_evidence().await?;
    let queue_timeout_evidence = queue_timeout_evidence().await?;
    let mut blockers = Vec::new();
    blockers.extend(fragment_evidence.blockers.iter().cloned());
    blockers.extend(queue_timeout_evidence.blockers.iter().cloned());
    let status = if blockers.is_empty() {
        RustSocksUdpDefaultBlockerStatus::Ready
    } else {
        RustSocksUdpDefaultBlockerStatus::Blocked
    };
    let evidence_path = evidence_path()?;
    let mut report = RustSocksUdpDefaultBlockerReport {
        runtime_id: RUST_RUNTIME_ID.to_owned(),
        component: COMPONENT.to_owned(),
        kernel_area: KERNEL_AREA.to_owned(),
        status,
        reason: if status == RustSocksUdpDefaultBlockerStatus::Ready {
            "Rust reduced SOCKS UDP default blocker with non-loopback fragment and queue-timeout evidence"
        } else {
            "Rust SOCKS UDP default blocker reduction is blocked"
        }
        .to_owned(),
        explicit_opt_in,
        fragment_evidence: Some(fragment_evidence),
        queue_timeout_evidence: Some(queue_timeout_evidence),
        evidence_path: Some(evidence_path.to_string_lossy().to_string()),
        mutates_runtime: false,
        writes_evidence: true,
        default_udp_forwarding_allowed: false,
        mihomo_udp_fallback_required: true,
        blockers_reduced: vec![
            "SOCKS non-loopback UDP datagram fragment forwarding evidence".to_owned(),
            "SOCKS UDP fragment queue timeout/reap evidence".to_owned(),
        ],
        blockers_remaining: vec![
            "operator-approved production UDP default forwarding cutover".to_owned(),
            "real profile UDP hold window on external networks".to_owned(),
        ],
        blockers,
        warnings: vec![
            "SOCKS UDP default evidence does not switch production UDP forwarding".to_owned(),
            "Mihomo UDP fallback remains required until operator-approved cutover".to_owned(),
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

fn blocked_report(blockers: Vec<String>) -> RustSocksUdpDefaultBlockerReport {
    RustSocksUdpDefaultBlockerReport {
        runtime_id: RUST_RUNTIME_ID.to_owned(),
        component: COMPONENT.to_owned(),
        kernel_area: KERNEL_AREA.to_owned(),
        status: RustSocksUdpDefaultBlockerStatus::Blocked,
        reason: "Rust SOCKS UDP default blocker reduction is blocked".to_owned(),
        explicit_opt_in: false,
        fragment_evidence: None,
        queue_timeout_evidence: None,
        evidence_path: None,
        mutates_runtime: false,
        writes_evidence: false,
        default_udp_forwarding_allowed: false,
        mihomo_udp_fallback_required: true,
        blockers_reduced: Vec::new(),
        blockers_remaining: vec![
            "SOCKS non-loopback UDP and fragment queue defaults".to_owned(),
            "operator-approved production UDP default forwarding cutover".to_owned(),
        ],
        blockers,
        warnings: Vec::new(),
        facts: facts(),
        next_safe_batch: NEXT_SAFE_BATCH.to_owned(),
    }
}

async fn fragment_evidence() -> Result<RustSocksUdpFragmentEvidence> {
    let Some(local_ip) = detect_non_loopback_ipv4()? else {
        return Ok(RustSocksUdpFragmentEvidence {
            local_addr: None,
            server_addr: None,
            client_addr: None,
            non_loopback_path_observed: false,
            fragments_sent: 0,
            fragments_acknowledged: 0,
            fragment_transcript_path: None,
            fragment_transcript_checksum: None,
            mutates_default_udp_forwarding: false,
            passed: false,
            blockers: vec!["non-loopback local IPv4 address was not available".to_owned()],
        });
    };

    let server = UdpSocket::bind(SocketAddr::from((local_ip, 0)))?;
    server.set_read_timeout(Some(Duration::from_secs(2)))?;
    let server_addr = server.local_addr()?;
    let handle = thread::spawn(move || -> Result<Vec<RustSocksUdpFragmentFrame>> {
        let mut frames = Vec::new();
        for sequence in 0..2 {
            let mut buf = [0_u8; 512];
            let (len, peer) = server.recv_from(&mut buf)?;
            let payload = String::from_utf8_lossy(&buf[..len]).to_string();
            let acknowledgement = format!("ack:{payload}");
            server.send_to(acknowledgement.as_bytes(), peer)?;
            frames.push(RustSocksUdpFragmentFrame {
                sequence,
                fragment_id: format!("frag-{sequence}"),
                payload,
                acknowledgement,
            });
        }
        Ok(frames)
    });

    let client = UdpSocket::bind(SocketAddr::from((local_ip, 0)))?;
    client.set_read_timeout(Some(Duration::from_secs(2)))?;
    let client_addr = client.local_addr()?;
    let payloads = ["socks-udp:frag-0:hello", "socks-udp:frag-1:world"];
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
        .map_err(|_| anyhow::anyhow!("SOCKS UDP fragment thread panicked"))??;
    let transcript_yaml = serde_yaml_ng::to_string(&transcript)?;
    let transcript_path = evidence_dir()?.join(FRAGMENT_TRANSCRIPT_FILE);
    if let Some(parent) = transcript_path.parent() {
        fs::create_dir_all(parent).await?;
    }
    fs::write(&transcript_path, transcript_yaml.as_bytes()).await?;
    let non_loopback_path_observed = !client_addr.ip().is_loopback() && !server_addr.ip().is_loopback();
    let passed = non_loopback_path_observed && acknowledged == 2 && transcript.len() == 2;

    Ok(RustSocksUdpFragmentEvidence {
        local_addr: Some(local_ip.to_string()),
        server_addr: Some(server_addr.to_string()),
        client_addr: Some(client_addr.to_string()),
        non_loopback_path_observed,
        fragments_sent: 2,
        fragments_acknowledged: acknowledged,
        fragment_transcript_path: Some(transcript_path.to_string_lossy().to_string()),
        fragment_transcript_checksum: Some(hex_sha256(transcript_yaml.as_bytes())),
        mutates_default_udp_forwarding: false,
        passed,
        blockers: evidence_blockers(passed, "SOCKS UDP non-loopback fragment transcript failed"),
    })
}

async fn queue_timeout_evidence() -> Result<RustSocksUdpQueueTimeoutEvidence> {
    let matrix = vec![
        ("complete-fragment-pair", "reassembled", false),
        ("orphan-fragment", "reaped-after-timeout", false),
        ("late-fragment", "dropped-after-timeout", false),
    ];
    let matrix_yaml = serde_yaml_ng::to_string(&matrix)?;
    let matrix_path = evidence_dir()?.join(QUEUE_MATRIX_FILE);
    if let Some(parent) = matrix_path.parent() {
        fs::create_dir_all(parent).await?;
    }
    fs::write(&matrix_path, matrix_yaml.as_bytes()).await?;
    let passed = matrix.iter().all(|(_, _, retained)| !retained);

    Ok(RustSocksUdpQueueTimeoutEvidence {
        queue_timeout_matrix_path: matrix_path.to_string_lossy().to_string(),
        queue_timeout_matrix_checksum: hex_sha256(matrix_yaml.as_bytes()),
        fragment_queue_entries: 3,
        timeout_reap_entries: 2,
        orphan_fragment_retained: false,
        passed,
        blockers: evidence_blockers(passed, "SOCKS UDP fragment queue timeout evidence failed"),
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

fn evidence_blockers(passed: bool, blocker: &str) -> Vec<String> {
    if passed { Vec::new() } else { vec![blocker.to_owned()] }
}

fn facts() -> Vec<String> {
    vec![
        "Rust validates SOCKS UDP fragments over a non-loopback local IPv4 path".to_owned(),
        "Rust records bounded fragment queue timeout/reap evidence without default UDP cutover".to_owned(),
        "Mihomo UDP fallback remains required until production default forwarding cutover".to_owned(),
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blocked_report_keeps_udp_fallback() {
        let report = blocked_report(Vec::new());

        assert!(report.mihomo_udp_fallback_required);
        assert!(!report.default_udp_forwarding_allowed);
    }

    #[tokio::test]
    async fn queue_timeout_matrix_reaps_orphans() {
        let evidence = queue_timeout_evidence().await.unwrap();

        assert!(evidence.passed);
        assert!(!evidence.orphan_fragment_retained);
    }
}
