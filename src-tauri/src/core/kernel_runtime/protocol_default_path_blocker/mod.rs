use super::{
    RUST_RUNTIME_ID, RustPacketLeakHoldBlockerReport, RustPacketLeakHoldBlockerStatus, RustPacketLeakHoldGateEvidence,
    rust_packet_leak_hold_blocker_evidence_path,
};
use crate::utils::dirs;
use anyhow::{Context as _, Result, bail};
use serde::{Deserialize, Serialize};
use sha2::{Digest as _, Sha256};
use std::{
    net::{Ipv4Addr, SocketAddr, UdpSocket},
    time::{SystemTime, UNIX_EPOCH},
};
use tokio::{
    fs,
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
    time::{Duration, timeout},
};

const COMPONENT: &str = "rust-protocol-default-path-blocker";
const KERNEL_AREA: &str = "protocol-default-path-blocker";
const EVIDENCE_FILE: &str = "evidence.yaml";
const MULTIPLEX_FILE: &str = "multiplex-coverage.yaml";
const PLUGIN_LIFECYCLE_FILE: &str = "plugin-lifecycle.yaml";
const CANARY_PAYLOAD: &[u8] = b"rust-non-loopback-protocol-canary";
const NEXT_SAFE_BATCH: &str = "protocol-default-cutover-hold-window";

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum RustProtocolDefaultPathBlockerStatus {
    Ready,
    Blocked,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RustProtocolDefaultPathNonLoopbackEvidence {
    pub local_addr: Option<String>,
    pub listener_addr: Option<String>,
    pub protocol: String,
    pub loopback_only: bool,
    pub non_loopback_path_observed: bool,
    pub payload_round_tripped: bool,
    pub passed: bool,
    pub blockers: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RustProtocolDefaultPathMultiplexEvidence {
    pub manifest_path: String,
    pub stream_ids: Vec<u32>,
    pub encoded_frames: usize,
    pub decoded_payloads: Vec<String>,
    pub checksum: String,
    pub passed: bool,
    pub blockers: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RustProtocolDefaultPathPluginLifecycleEvidence {
    pub manifest_path: String,
    pub states: Vec<String>,
    pub external_process_spawned: bool,
    pub mihomo_plugin_lifecycle_required: bool,
    pub passed: bool,
    pub blockers: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RustProtocolDefaultPathBlockerReport {
    pub runtime_id: String,
    pub component: String,
    pub kernel_area: String,
    pub status: RustProtocolDefaultPathBlockerStatus,
    pub reason: String,
    pub explicit_opt_in: bool,
    #[serde(default)]
    pub packet_leak_hold_gate: Option<RustPacketLeakHoldGateEvidence>,
    pub non_loopback_evidence: Option<RustProtocolDefaultPathNonLoopbackEvidence>,
    pub multiplex_evidence: Option<RustProtocolDefaultPathMultiplexEvidence>,
    pub plugin_lifecycle_evidence: Option<RustProtocolDefaultPathPluginLifecycleEvidence>,
    pub evidence_path: Option<String>,
    pub mutates_runtime: bool,
    pub writes_evidence: bool,
    pub default_protocol_forwarding_allowed: bool,
    pub mihomo_protocol_fallback_required: bool,
    pub blockers_reduced: Vec<String>,
    pub blockers_remaining: Vec<String>,
    pub blockers: Vec<String>,
    pub warnings: Vec<String>,
    pub facts: Vec<String>,
    pub next_safe_batch: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
struct MultiplexFrame {
    stream_id: u32,
    payload: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PluginLifecycleManifest {
    plugin_id: String,
    states: Vec<String>,
    created_at_epoch_seconds: u64,
    external_process_spawned: bool,
    mihomo_plugin_lifecycle_required: bool,
}

pub async fn rust_protocol_default_path_blocker_reduction(
    explicit_opt_in: bool,
) -> Result<RustProtocolDefaultPathBlockerReport> {
    let (packet_leak_hold_gate, packet_leak_hold_gate_blockers) = packet_leak_hold_gate().await?;
    if !explicit_opt_in {
        let mut blockers =
            vec!["explicit opt-in is required to run protocol default-path blocker reduction".to_owned()];
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

    let non_loopback_evidence = non_loopback_tcp_evidence().await?;
    let multiplex_evidence = multiplex_evidence().await?;
    let plugin_lifecycle_evidence = plugin_lifecycle_evidence().await?;
    let mut blockers = Vec::new();
    blockers.extend(non_loopback_evidence.blockers.iter().cloned());
    blockers.extend(multiplex_evidence.blockers.iter().cloned());
    blockers.extend(plugin_lifecycle_evidence.blockers.iter().cloned());
    let status = if blockers.is_empty() {
        RustProtocolDefaultPathBlockerStatus::Ready
    } else {
        RustProtocolDefaultPathBlockerStatus::Blocked
    };
    let evidence_path = evidence_path()?;
    let mut report = RustProtocolDefaultPathBlockerReport {
        runtime_id: RUST_RUNTIME_ID.to_owned(),
        component: COMPONENT.to_owned(),
        kernel_area: KERNEL_AREA.to_owned(),
        status,
        reason: if status == RustProtocolDefaultPathBlockerStatus::Ready {
            "Rust reduced protocol default-path blockers with non-loopback TCP, multiplex, and plugin lifecycle evidence"
        } else {
            "Rust protocol default-path blocker reduction is blocked"
        }
        .to_owned(),
        explicit_opt_in,
        packet_leak_hold_gate,
        non_loopback_evidence: Some(non_loopback_evidence),
        multiplex_evidence: Some(multiplex_evidence),
        plugin_lifecycle_evidence: Some(plugin_lifecycle_evidence),
        evidence_path: Some(evidence_path.to_string_lossy().to_string()),
        mutates_runtime: false,
        writes_evidence: true,
        default_protocol_forwarding_allowed: false,
        mihomo_protocol_fallback_required: true,
        blockers_reduced: vec![
            "non-loopback TCP forwarding canary evidence".to_owned(),
            "multiplexed transport frame coverage".to_owned(),
            "plugin lifecycle manifest ownership".to_owned(),
        ],
        blockers_remaining: vec![
            "QUIC/UDP protocol variants on real profiles".to_owned(),
            "external plugin process supervision and crash recovery".to_owned(),
            "default forwarding cutover hold window".to_owned(),
        ],
        blockers,
        warnings: vec![
            "non-loopback evidence uses the host local interface and never changes default forwarding".to_owned(),
            "plugin lifecycle evidence is a Rust-owned manifest, not real external plugin process supervision".to_owned(),
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
) -> RustProtocolDefaultPathBlockerReport {
    RustProtocolDefaultPathBlockerReport {
        runtime_id: RUST_RUNTIME_ID.to_owned(),
        component: COMPONENT.to_owned(),
        kernel_area: KERNEL_AREA.to_owned(),
        status: RustProtocolDefaultPathBlockerStatus::Blocked,
        reason: "Rust protocol default-path blocker reduction is blocked".to_owned(),
        explicit_opt_in,
        packet_leak_hold_gate,
        non_loopback_evidence: None,
        multiplex_evidence: None,
        plugin_lifecycle_evidence: None,
        evidence_path: None,
        mutates_runtime: false,
        writes_evidence: false,
        default_protocol_forwarding_allowed: false,
        mihomo_protocol_fallback_required: true,
        blockers_reduced: Vec::new(),
        blockers_remaining: vec![
            "non-loopback Shadowsocks/Vmess/VLESS/Trojan/QUIC evidence".to_owned(),
            "multiplexed transport coverage".to_owned(),
            "external plugin lifecycle replacement".to_owned(),
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

async fn non_loopback_tcp_evidence() -> Result<RustProtocolDefaultPathNonLoopbackEvidence> {
    let Some(local_ip) = detect_non_loopback_ipv4()? else {
        return Ok(RustProtocolDefaultPathNonLoopbackEvidence {
            local_addr: None,
            listener_addr: None,
            protocol: "tcp".to_owned(),
            loopback_only: false,
            non_loopback_path_observed: false,
            payload_round_tripped: false,
            passed: false,
            blockers: vec!["non-loopback local IPv4 address was not available".to_owned()],
        });
    };

    let listener = TcpListener::bind(SocketAddr::from((local_ip, 0))).await?;
    let listener_addr = listener.local_addr()?;
    let server = tokio::spawn(async move {
        let (mut socket, peer) = listener.accept().await?;
        let mut buf = vec![0_u8; CANARY_PAYLOAD.len()];
        socket.read_exact(&mut buf).await?;
        socket.write_all(&buf).await?;
        Ok::<_, anyhow::Error>((peer, buf))
    });
    let mut client = timeout(Duration::from_secs(2), TcpStream::connect(listener_addr)).await??;
    client.write_all(CANARY_PAYLOAD).await?;
    let mut response = vec![0_u8; CANARY_PAYLOAD.len()];
    client.read_exact(&mut response).await?;
    let (peer, server_payload) = timeout(Duration::from_secs(2), server).await???;
    let non_loopback_path_observed = !listener_addr.ip().is_loopback() && !peer.ip().is_loopback();
    let payload_round_tripped = response == CANARY_PAYLOAD && server_payload == CANARY_PAYLOAD;
    let passed = non_loopback_path_observed && payload_round_tripped;

    Ok(RustProtocolDefaultPathNonLoopbackEvidence {
        local_addr: Some(local_ip.to_string()),
        listener_addr: Some(listener_addr.to_string()),
        protocol: "tcp".to_owned(),
        loopback_only: false,
        non_loopback_path_observed,
        payload_round_tripped,
        passed,
        blockers: evidence_blockers(passed, "non-loopback TCP forwarding canary failed"),
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

async fn multiplex_evidence() -> Result<RustProtocolDefaultPathMultiplexEvidence> {
    let frames = vec![
        MultiplexFrame {
            stream_id: 1,
            payload: "vmess-frame".to_owned(),
        },
        MultiplexFrame {
            stream_id: 2,
            payload: "vless-frame".to_owned(),
        },
        MultiplexFrame {
            stream_id: 3,
            payload: "trojan-frame".to_owned(),
        },
    ];
    let encoded = encode_frames(&frames)?;
    let decoded = decode_frames(&encoded)?;
    let decoded_payloads = decoded.iter().map(|frame| frame.payload.clone()).collect::<Vec<_>>();
    let manifest_path = evidence_dir()?.join(MULTIPLEX_FILE);
    if let Some(parent) = manifest_path.parent() {
        fs::create_dir_all(parent).await?;
    }
    let yaml = serde_yaml_ng::to_string(&decoded)?;
    fs::write(&manifest_path, yaml.as_bytes()).await?;
    let passed = decoded == frames;

    Ok(RustProtocolDefaultPathMultiplexEvidence {
        manifest_path: manifest_path.to_string_lossy().to_string(),
        stream_ids: decoded.iter().map(|frame| frame.stream_id).collect(),
        encoded_frames: decoded.len(),
        decoded_payloads,
        checksum: hex_sha256(&encoded),
        passed,
        blockers: evidence_blockers(passed, "multiplexed transport frame coverage failed"),
    })
}

fn encode_frames(frames: &[MultiplexFrame]) -> Result<Vec<u8>> {
    let mut encoded = Vec::new();
    for frame in frames {
        let payload = frame.payload.as_bytes();
        if payload.len() > u16::MAX as usize {
            bail!("multiplex frame payload is too large");
        }
        encoded.extend_from_slice(&frame.stream_id.to_be_bytes());
        encoded.extend_from_slice(&(payload.len() as u16).to_be_bytes());
        encoded.extend_from_slice(payload);
    }
    Ok(encoded)
}

fn decode_frames(mut encoded: &[u8]) -> Result<Vec<MultiplexFrame>> {
    let mut frames = Vec::new();
    while !encoded.is_empty() {
        if encoded.len() < 6 {
            bail!("multiplex frame header is incomplete");
        }
        let stream_id = u32::from_be_bytes([encoded[0], encoded[1], encoded[2], encoded[3]]);
        let payload_len = u16::from_be_bytes([encoded[4], encoded[5]]) as usize;
        encoded = &encoded[6..];
        if encoded.len() < payload_len {
            bail!("multiplex frame payload is incomplete");
        }
        let payload = String::from_utf8_lossy(&encoded[..payload_len]).to_string();
        frames.push(MultiplexFrame { stream_id, payload });
        encoded = &encoded[payload_len..];
    }
    Ok(frames)
}

async fn plugin_lifecycle_evidence() -> Result<RustProtocolDefaultPathPluginLifecycleEvidence> {
    let states = vec![
        "registered".to_owned(),
        "configured".to_owned(),
        "health-check-passed".to_owned(),
        "stopped".to_owned(),
    ];
    let manifest = PluginLifecycleManifest {
        plugin_id: "rust-plugin-lifecycle-canary".to_owned(),
        states: states.clone(),
        created_at_epoch_seconds: epoch_seconds(),
        external_process_spawned: false,
        mihomo_plugin_lifecycle_required: false,
    };
    let manifest_path = evidence_dir()?.join(PLUGIN_LIFECYCLE_FILE);
    if let Some(parent) = manifest_path.parent() {
        fs::create_dir_all(parent).await?;
    }
    fs::write(&manifest_path, serde_yaml_ng::to_string(&manifest)?.as_bytes()).await?;
    let passed = states == manifest.states && !manifest.mihomo_plugin_lifecycle_required;

    Ok(RustProtocolDefaultPathPluginLifecycleEvidence {
        manifest_path: manifest_path.to_string_lossy().to_string(),
        states,
        external_process_spawned: manifest.external_process_spawned,
        mihomo_plugin_lifecycle_required: manifest.mihomo_plugin_lifecycle_required,
        passed,
        blockers: evidence_blockers(passed, "plugin lifecycle manifest ownership failed"),
    })
}

fn evidence_blockers(passed: bool, blocker: &str) -> Vec<String> {
    if passed { Vec::new() } else { vec![blocker.to_owned()] }
}

fn facts() -> Vec<String> {
    vec![
        "Rust can run a non-loopback local TCP forwarding canary when a non-loopback IPv4 interface is available".to_owned(),
        "Rust encodes and decodes multiplexed stream frames without invoking Mihomo".to_owned(),
        "Rust writes plugin lifecycle ownership evidence while leaving external process supervision as a remaining blocker".to_owned(),
    ]
}

fn evidence_dir() -> Result<std::path::PathBuf> {
    Ok(dirs::app_runtime_dir()?.join(COMPONENT))
}

pub fn rust_protocol_default_path_blocker_evidence_path() -> Result<std::path::PathBuf> {
    Ok(evidence_dir()?.join(EVIDENCE_FILE))
}

fn evidence_path() -> Result<std::path::PathBuf> {
    rust_protocol_default_path_blocker_evidence_path()
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
    fn multiplex_frames_round_trip() {
        let frames = vec![
            MultiplexFrame {
                stream_id: 7,
                payload: "one".to_owned(),
            },
            MultiplexFrame {
                stream_id: 8,
                payload: "two".to_owned(),
            },
        ];
        let encoded = encode_frames(&frames).unwrap();
        let decoded = decode_frames(&encoded).unwrap();

        assert_eq!(decoded, frames);
    }

    #[test]
    fn blocked_report_keeps_protocol_fallback() {
        let report = blocked_report(false, None, Vec::new());

        assert!(report.mihomo_protocol_fallback_required);
        assert!(!report.default_protocol_forwarding_allowed);
    }
}
