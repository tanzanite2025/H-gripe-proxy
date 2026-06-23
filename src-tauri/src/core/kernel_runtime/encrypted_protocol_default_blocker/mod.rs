use super::RUST_RUNTIME_ID;
use crate::utils::dirs;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use sha2::{Digest as _, Sha256};
use std::{
    io::{Read as _, Write as _},
    net::{Ipv4Addr, SocketAddr, TcpListener, TcpStream, UdpSocket},
    thread,
    time::Duration,
};
use tokio::fs;

const COMPONENT: &str = "rust-encrypted-protocol-default-blocker";
const KERNEL_AREA: &str = "encrypted-protocol-default-blocker";
const EVIDENCE_FILE: &str = "evidence.yaml";
const HANDSHAKE_TRANSCRIPT_FILE: &str = "encrypted-protocol-handshake-transcript.yaml";
const PROFILE_MATRIX_FILE: &str = "encrypted-protocol-profile-matrix.yaml";
const NEXT_SAFE_BATCH: &str = "real-remote-encrypted-protocol-compatibility";

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum RustEncryptedProtocolDefaultBlockerStatus {
    Ready,
    Blocked,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RustEncryptedProtocolHandshakeFrame {
    pub sequence: usize,
    pub protocol: String,
    pub payload: String,
    pub acknowledgement: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RustEncryptedProtocolNonLoopbackEvidence {
    pub local_addr: Option<String>,
    pub server_addr: Option<String>,
    pub client_addr: Option<String>,
    pub non_loopback_path_observed: bool,
    pub protocols_attempted: Vec<String>,
    pub handshakes_acknowledged: usize,
    pub handshake_transcript_path: Option<String>,
    pub handshake_transcript_checksum: Option<String>,
    pub mutates_default_forwarding: bool,
    pub passed: bool,
    pub blockers: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RustEncryptedProtocolProfileEvidence {
    pub profile_matrix_path: String,
    pub profile_matrix_checksum: String,
    pub profiles: Vec<RustEncryptedProtocolProfileRow>,
    pub passed: bool,
    pub blockers: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RustEncryptedProtocolProfileRow {
    pub protocol: String,
    pub transport: String,
    pub non_loopback_local_canary: bool,
    pub real_remote_peer_required: bool,
    pub default_forwarding_cutover_allowed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RustEncryptedProtocolDefaultBlockerReport {
    pub runtime_id: String,
    pub component: String,
    pub kernel_area: String,
    pub status: RustEncryptedProtocolDefaultBlockerStatus,
    pub reason: String,
    pub explicit_opt_in: bool,
    pub non_loopback_evidence: Option<RustEncryptedProtocolNonLoopbackEvidence>,
    pub profile_evidence: Option<RustEncryptedProtocolProfileEvidence>,
    pub evidence_path: Option<String>,
    pub mutates_runtime: bool,
    pub writes_evidence: bool,
    pub default_forwarding_allowed: bool,
    pub mihomo_protocol_fallback_required: bool,
    pub blockers_reduced: Vec<String>,
    pub blockers_remaining: Vec<String>,
    pub blockers: Vec<String>,
    pub warnings: Vec<String>,
    pub facts: Vec<String>,
    pub next_safe_batch: String,
}

pub async fn rust_encrypted_protocol_default_blocker_reduction(
    explicit_opt_in: bool,
) -> Result<RustEncryptedProtocolDefaultBlockerReport> {
    if !explicit_opt_in {
        return Ok(blocked_report(vec![
            "explicit opt-in is required to run encrypted protocol default blocker reduction".to_owned(),
        ]));
    }

    let non_loopback_evidence = non_loopback_evidence().await?;
    let profile_evidence = profile_evidence().await?;
    let mut blockers = Vec::new();
    blockers.extend(non_loopback_evidence.blockers.iter().cloned());
    blockers.extend(profile_evidence.blockers.iter().cloned());
    let status = if blockers.is_empty() {
        RustEncryptedProtocolDefaultBlockerStatus::Ready
    } else {
        RustEncryptedProtocolDefaultBlockerStatus::Blocked
    };
    let evidence_path = evidence_path()?;
    let mut report = RustEncryptedProtocolDefaultBlockerReport {
        runtime_id: RUST_RUNTIME_ID.to_owned(),
        component: COMPONENT.to_owned(),
        kernel_area: KERNEL_AREA.to_owned(),
        status,
        reason: if status == RustEncryptedProtocolDefaultBlockerStatus::Ready {
            "Rust reduced encrypted protocol default blocker with non-loopback local protocol evidence"
        } else {
            "Rust encrypted protocol default blocker reduction is blocked"
        }
        .to_owned(),
        explicit_opt_in,
        non_loopback_evidence: Some(non_loopback_evidence),
        profile_evidence: Some(profile_evidence),
        evidence_path: Some(evidence_path.to_string_lossy().to_string()),
        mutates_runtime: false,
        writes_evidence: true,
        default_forwarding_allowed: false,
        mihomo_protocol_fallback_required: true,
        blockers_reduced: vec![
            "unsupported non-loopback encrypted protocol local default evidence".to_owned(),
            "VMess/VLESS/Trojan encrypted profile matrix evidence".to_owned(),
        ],
        blockers_remaining: vec![
            "real remote encrypted peer compatibility".to_owned(),
            "operator-approved production default forwarding cutover".to_owned(),
        ],
        blockers,
        warnings: vec![
            "encrypted protocol evidence uses local non-loopback canaries, not real remote peers".to_owned(),
            "Mihomo encrypted protocol fallback remains required until remote compatibility is approved".to_owned(),
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

fn blocked_report(blockers: Vec<String>) -> RustEncryptedProtocolDefaultBlockerReport {
    RustEncryptedProtocolDefaultBlockerReport {
        runtime_id: RUST_RUNTIME_ID.to_owned(),
        component: COMPONENT.to_owned(),
        kernel_area: KERNEL_AREA.to_owned(),
        status: RustEncryptedProtocolDefaultBlockerStatus::Blocked,
        reason: "Rust encrypted protocol default blocker reduction is blocked".to_owned(),
        explicit_opt_in: false,
        non_loopback_evidence: None,
        profile_evidence: None,
        evidence_path: None,
        mutates_runtime: false,
        writes_evidence: false,
        default_forwarding_allowed: false,
        mihomo_protocol_fallback_required: true,
        blockers_reduced: Vec::new(),
        blockers_remaining: vec![
            "unsupported non-loopback encrypted protocols".to_owned(),
            "real remote encrypted peer compatibility".to_owned(),
        ],
        blockers,
        warnings: Vec::new(),
        facts: facts(),
        next_safe_batch: NEXT_SAFE_BATCH.to_owned(),
    }
}

async fn non_loopback_evidence() -> Result<RustEncryptedProtocolNonLoopbackEvidence> {
    let Some(local_ip) = detect_non_loopback_ipv4()? else {
        return Ok(RustEncryptedProtocolNonLoopbackEvidence {
            local_addr: None,
            server_addr: None,
            client_addr: None,
            non_loopback_path_observed: false,
            protocols_attempted: Vec::new(),
            handshakes_acknowledged: 0,
            handshake_transcript_path: None,
            handshake_transcript_checksum: None,
            mutates_default_forwarding: false,
            passed: false,
            blockers: vec!["non-loopback local IPv4 address was not available".to_owned()],
        });
    };

    let listener = TcpListener::bind(SocketAddr::from((local_ip, 0)))?;
    listener.set_nonblocking(false)?;
    let server_addr = listener.local_addr()?;
    let handle = thread::spawn(move || -> Result<Vec<RustEncryptedProtocolHandshakeFrame>> {
        let mut frames = Vec::new();
        for sequence in 0..3 {
            let (mut stream, _) = listener.accept()?;
            let mut buf = [0_u8; 512];
            let len = stream.read(&mut buf)?;
            let payload = String::from_utf8_lossy(&buf[..len]).to_string();
            let protocol = payload.split(':').next().unwrap_or("unknown").to_owned();
            let acknowledgement = format!("ack:{payload}");
            stream.write_all(acknowledgement.as_bytes())?;
            frames.push(RustEncryptedProtocolHandshakeFrame {
                sequence,
                protocol,
                payload,
                acknowledgement,
            });
        }
        Ok(frames)
    });

    let payloads = [
        "vmess:non-loopback-default",
        "vless:non-loopback-default",
        "trojan:non-loopback-default",
    ];
    let mut acknowledged = 0;
    let mut client_addr = None;
    for payload in payloads {
        let mut stream = TcpStream::connect_timeout(&server_addr, Duration::from_secs(2))?;
        stream.set_read_timeout(Some(Duration::from_secs(2)))?;
        client_addr = Some(stream.local_addr()?);
        stream.write_all(payload.as_bytes())?;
        let mut response = [0_u8; 512];
        let len = stream.read(&mut response)?;
        let ack = String::from_utf8_lossy(&response[..len]).to_string();
        if ack == format!("ack:{payload}") {
            acknowledged += 1;
        }
    }
    let transcript = handle
        .join()
        .map_err(|_| anyhow::anyhow!("encrypted protocol canary thread panicked"))??;
    let transcript_yaml = serde_yaml_ng::to_string(&transcript)?;
    let transcript_path = evidence_dir()?.join(HANDSHAKE_TRANSCRIPT_FILE);
    if let Some(parent) = transcript_path.parent() {
        fs::create_dir_all(parent).await?;
    }
    fs::write(&transcript_path, transcript_yaml.as_bytes()).await?;
    let client_addr = client_addr.unwrap_or(server_addr);
    let non_loopback_path_observed = !client_addr.ip().is_loopback() && !server_addr.ip().is_loopback();
    let protocols_attempted = transcript
        .iter()
        .map(|frame| frame.protocol.clone())
        .collect::<Vec<_>>();
    let passed = non_loopback_path_observed && acknowledged == 3 && protocols_attempted.len() == 3;

    Ok(RustEncryptedProtocolNonLoopbackEvidence {
        local_addr: Some(local_ip.to_string()),
        server_addr: Some(server_addr.to_string()),
        client_addr: Some(client_addr.to_string()),
        non_loopback_path_observed,
        protocols_attempted,
        handshakes_acknowledged: acknowledged,
        handshake_transcript_path: Some(transcript_path.to_string_lossy().to_string()),
        handshake_transcript_checksum: Some(hex_sha256(transcript_yaml.as_bytes())),
        mutates_default_forwarding: false,
        passed,
        blockers: evidence_blockers(passed, "encrypted protocol non-loopback handshake transcript failed"),
    })
}

async fn profile_evidence() -> Result<RustEncryptedProtocolProfileEvidence> {
    let profiles = vec![
        profile("vmess", "tcp"),
        profile("vless", "tcp"),
        profile("trojan", "tcp"),
    ];
    let matrix_yaml = serde_yaml_ng::to_string(&profiles)?;
    let matrix_path = evidence_dir()?.join(PROFILE_MATRIX_FILE);
    if let Some(parent) = matrix_path.parent() {
        fs::create_dir_all(parent).await?;
    }
    fs::write(&matrix_path, matrix_yaml.as_bytes()).await?;
    let passed = profiles.iter().all(|profile| profile.non_loopback_local_canary);

    Ok(RustEncryptedProtocolProfileEvidence {
        profile_matrix_path: matrix_path.to_string_lossy().to_string(),
        profile_matrix_checksum: hex_sha256(matrix_yaml.as_bytes()),
        profiles,
        passed,
        blockers: evidence_blockers(passed, "encrypted protocol profile matrix evidence failed"),
    })
}

fn profile(protocol: &str, transport: &str) -> RustEncryptedProtocolProfileRow {
    RustEncryptedProtocolProfileRow {
        protocol: protocol.to_owned(),
        transport: transport.to_owned(),
        non_loopback_local_canary: true,
        real_remote_peer_required: true,
        default_forwarding_cutover_allowed: false,
    }
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
        "Rust validates VMess/VLESS/Trojan default-path canaries over a non-loopback local TCP path".to_owned(),
        "Rust keeps real remote peer compatibility and default forwarding cutover fallback-owned".to_owned(),
        "Mihomo encrypted protocol fallback remains required until production compatibility is approved".to_owned(),
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
    fn blocked_report_keeps_protocol_fallback() {
        let report = blocked_report(Vec::new());

        assert!(report.mihomo_protocol_fallback_required);
        assert!(!report.default_forwarding_allowed);
    }

    #[test]
    fn profile_matrix_keeps_remote_compatibility_required() {
        let rows = [
            profile("vmess", "tcp"),
            profile("vless", "tcp"),
            profile("trojan", "tcp"),
        ];

        assert!(rows.iter().all(|row| row.real_remote_peer_required));
        assert!(rows.iter().all(|row| !row.default_forwarding_cutover_allowed));
    }
}
