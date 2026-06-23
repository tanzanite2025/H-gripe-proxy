use super::{
    RUST_RUNTIME_ID, RustDefaultDataPlaneCloseoutGateEvidence, RustSocksAuthExecutionReport,
    RustSocksAuthExecutionStatus, RustSocksBindExecutionReport, RustSocksBindExecutionStatus,
    RustSocksTcpConnectExecutionReport, RustSocksTcpConnectExecutionStatus, RustSocksUdpAssociateExecutionReport,
    RustSocksUdpAssociateExecutionStatus, RustSocksUdpFragmentsExecutionReport, RustSocksUdpFragmentsExecutionStatus,
};
use crate::utils::dirs;
use anyhow::{Context as _, Result, bail};
use serde::{Deserialize, Serialize};
use sha2::{Digest as _, Sha256};
use std::{
    net::Ipv4Addr,
    process::Command,
    time::{SystemTime, UNIX_EPOCH},
};
use tokio::fs;

const COMPONENT: &str = "rust-route-packet-capture-blocker";
const KERNEL_AREA: &str = "route-packet-capture-blocker";
const EVIDENCE_FILE: &str = "evidence.yaml";
const ROUTE_SNAPSHOT_FILE: &str = "route-snapshot.txt";
const ROUTE_RESTORE_PLAN_FILE: &str = "route-restore-plan.yaml";
const PACKET_CAPTURE_HOLD_FILE: &str = "packet-capture-hold.yaml";
const NEXT_SAFE_BATCH: &str = "privileged-tun-device-lifecycle-blocker";

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum RustRoutePacketCaptureBlockerStatus {
    Ready,
    Blocked,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RustRoutePacketCaptureRouteSnapshotEvidence {
    pub platform: String,
    pub command: Vec<String>,
    pub snapshot_path: String,
    pub route_restore_plan_path: String,
    pub snapshot_present: bool,
    pub snapshot_checksum: Option<String>,
    pub route_entries_observed: usize,
    pub mutates_routes: bool,
    pub passed: bool,
    pub blockers: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RustRoutePacketCaptureRestoreStep {
    pub order: usize,
    pub action: String,
    pub route_family: String,
    pub requires_privilege: bool,
    pub mutates_routes_in_this_step: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RustRoutePacketCapturePacketEvidence {
    pub packet_source: String,
    pub packet_destination: String,
    pub packet_destination_port: u16,
    pub packet_capture_hold_path: String,
    pub ipv4_packet_parsed: bool,
    pub tcp_destination_extracted: bool,
    pub packet_hold_iterations: usize,
    pub checksum: String,
    pub passed: bool,
    pub blockers: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RustUnsupportedProtocolExecutionGateEvidence {
    pub evidence_paths: Vec<String>,
    pub executed_components: Vec<String>,
    pub default_closeout_gate_confirmed: bool,
    pub blockers: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RustRoutePacketCaptureBlockerReport {
    pub runtime_id: String,
    pub component: String,
    pub kernel_area: String,
    pub status: RustRoutePacketCaptureBlockerStatus,
    pub reason: String,
    pub explicit_opt_in: bool,
    #[serde(default)]
    pub unsupported_protocol_execution_gate: Option<RustUnsupportedProtocolExecutionGateEvidence>,
    pub route_snapshot_evidence: Option<RustRoutePacketCaptureRouteSnapshotEvidence>,
    pub restore_steps: Vec<RustRoutePacketCaptureRestoreStep>,
    pub packet_capture_evidence: Option<RustRoutePacketCapturePacketEvidence>,
    pub evidence_path: Option<String>,
    pub mutates_runtime: bool,
    pub writes_evidence: bool,
    pub default_tun_replacement_allowed: bool,
    pub mihomo_system_packet_capture_fallback_required: bool,
    pub blockers_reduced: Vec<String>,
    pub blockers_remaining: Vec<String>,
    pub blockers: Vec<String>,
    pub warnings: Vec<String>,
    pub facts: Vec<String>,
    pub next_safe_batch: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PacketCaptureHoldRecord {
    source: String,
    destination: String,
    destination_port: u16,
    iterations: usize,
    created_at_epoch_seconds: u64,
}

pub async fn rust_route_packet_capture_blocker_reduction(
    explicit_opt_in: bool,
) -> Result<RustRoutePacketCaptureBlockerReport> {
    let unsupported_protocol_execution_gate = unsupported_protocol_execution_gate_evidence().await?;
    if !explicit_opt_in {
        let mut blockers = vec!["explicit opt-in is required to run route/packet-capture blocker reduction".to_owned()];
        blockers.extend(unsupported_protocol_execution_gate.blockers.clone());
        return Ok(blocked_report(
            explicit_opt_in,
            Some(unsupported_protocol_execution_gate),
            blockers,
        ));
    }
    if !unsupported_protocol_execution_gate.blockers.is_empty() {
        return Ok(blocked_report(
            explicit_opt_in,
            Some(unsupported_protocol_execution_gate.clone()),
            unsupported_protocol_execution_gate.blockers.clone(),
        ));
    }

    let route_snapshot_evidence = route_snapshot_evidence().await?;
    let restore_steps = restore_steps();
    write_restore_plan(&restore_steps).await?;
    let packet_capture_evidence = packet_capture_hold_evidence().await?;
    let mut blockers = Vec::new();
    blockers.extend(route_snapshot_evidence.blockers.iter().cloned());
    blockers.extend(packet_capture_evidence.blockers.iter().cloned());
    let status = if blockers.is_empty() {
        RustRoutePacketCaptureBlockerStatus::Ready
    } else {
        RustRoutePacketCaptureBlockerStatus::Blocked
    };
    let evidence_path = evidence_path()?;
    let mut report = RustRoutePacketCaptureBlockerReport {
        runtime_id: RUST_RUNTIME_ID.to_owned(),
        component: COMPONENT.to_owned(),
        kernel_area: KERNEL_AREA.to_owned(),
        status,
        reason: if status == RustRoutePacketCaptureBlockerStatus::Ready {
            "Rust reduced route/packet-capture blockers with route snapshot restore planning and synthetic packet-capture hold evidence"
        } else {
            "Rust route/packet-capture blocker reduction is blocked"
        }
        .to_owned(),
        explicit_opt_in,
        unsupported_protocol_execution_gate: Some(unsupported_protocol_execution_gate),
        route_snapshot_evidence: Some(route_snapshot_evidence),
        restore_steps,
        packet_capture_evidence: Some(packet_capture_evidence),
        evidence_path: Some(evidence_path.to_string_lossy().to_string()),
        mutates_runtime: false,
        writes_evidence: true,
        default_tun_replacement_allowed: false,
        mihomo_system_packet_capture_fallback_required: true,
        blockers_reduced: vec![
            "host route table snapshot and restore-plan evidence".to_owned(),
            "bounded packet-capture hold parser evidence".to_owned(),
        ],
        blockers_remaining: vec![
            "real TUN device lifecycle ownership".to_owned(),
            "privileged route table mutation apply/rollback on real interfaces".to_owned(),
            "post-cutover packet leak hold window".to_owned(),
        ],
        blockers,
        warnings: vec![
            "route snapshot is read-only and does not install or delete host routes".to_owned(),
            "packet-capture hold evidence is synthetic and does not attach to a real TUN device".to_owned(),
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
    unsupported_protocol_execution_gate: Option<RustUnsupportedProtocolExecutionGateEvidence>,
    blockers: Vec<String>,
) -> RustRoutePacketCaptureBlockerReport {
    RustRoutePacketCaptureBlockerReport {
        runtime_id: RUST_RUNTIME_ID.to_owned(),
        component: COMPONENT.to_owned(),
        kernel_area: KERNEL_AREA.to_owned(),
        status: RustRoutePacketCaptureBlockerStatus::Blocked,
        reason: "Rust route/packet-capture blocker reduction is blocked".to_owned(),
        explicit_opt_in,
        unsupported_protocol_execution_gate,
        route_snapshot_evidence: None,
        restore_steps: Vec::new(),
        packet_capture_evidence: None,
        evidence_path: None,
        mutates_runtime: false,
        writes_evidence: false,
        default_tun_replacement_allowed: false,
        mihomo_system_packet_capture_fallback_required: true,
        blockers_reduced: Vec::new(),
        blockers_remaining: vec![
            "real TUN device lifecycle ownership".to_owned(),
            "host route table mutation and rollback on all platforms".to_owned(),
            "post-cutover packet leak hold window".to_owned(),
        ],
        blockers,
        warnings: Vec::new(),
        facts: facts(),
        next_safe_batch: NEXT_SAFE_BATCH.to_owned(),
    }
}

async fn unsupported_protocol_execution_gate_evidence() -> Result<RustUnsupportedProtocolExecutionGateEvidence> {
    let mut evidence_paths = Vec::new();
    let mut executed_components = Vec::new();
    let mut closeout_gates = Vec::new();
    let mut blockers = Vec::new();

    let udp_associate_path = runtime_evidence_path("rust-socks-udp-associate-execution")?;
    match read_yaml::<RustSocksUdpAssociateExecutionReport>(&udp_associate_path).await? {
        Some(report) if report.status == RustSocksUdpAssociateExecutionStatus::Executed => {
            evidence_paths.push(udp_associate_path.to_string_lossy().to_string());
            executed_components.push(report.component.to_string());
            closeout_gates.push(report.default_data_plane_closeout_gate);
        }
        Some(report) => blockers.push(format!("SOCKS UDP associate evidence status is {:?}", report.status)),
        None => blockers.push("SOCKS UDP associate execution evidence is missing".to_owned()),
    }

    let udp_fragments_path = runtime_evidence_path("rust-socks-udp-fragments-execution")?;
    match read_yaml::<RustSocksUdpFragmentsExecutionReport>(&udp_fragments_path).await? {
        Some(report) if report.status == RustSocksUdpFragmentsExecutionStatus::Executed => {
            evidence_paths.push(udp_fragments_path.to_string_lossy().to_string());
            executed_components.push(report.component.to_string());
            closeout_gates.push(report.default_data_plane_closeout_gate);
        }
        Some(report) => blockers.push(format!("SOCKS UDP fragments evidence status is {:?}", report.status)),
        None => blockers.push("SOCKS UDP fragments execution evidence is missing".to_owned()),
    }

    let auth_path = runtime_evidence_path("rust-socks-auth-execution")?;
    match read_yaml::<RustSocksAuthExecutionReport>(&auth_path).await? {
        Some(report) if report.status == RustSocksAuthExecutionStatus::Executed => {
            evidence_paths.push(auth_path.to_string_lossy().to_string());
            executed_components.push(report.component.to_string());
            closeout_gates.push(report.default_data_plane_closeout_gate);
        }
        Some(report) => blockers.push(format!("SOCKS auth evidence status is {:?}", report.status)),
        None => blockers.push("SOCKS auth execution evidence is missing".to_owned()),
    }

    let tcp_connect_path = runtime_evidence_path("rust-socks-tcp-connect-execution")?;
    match read_yaml::<RustSocksTcpConnectExecutionReport>(&tcp_connect_path).await? {
        Some(report) if report.status == RustSocksTcpConnectExecutionStatus::Executed => {
            evidence_paths.push(tcp_connect_path.to_string_lossy().to_string());
            executed_components.push(report.component.to_string());
            closeout_gates.push(report.default_data_plane_closeout_gate);
        }
        Some(report) => blockers.push(format!("SOCKS TCP CONNECT evidence status is {:?}", report.status)),
        None => blockers.push("SOCKS TCP CONNECT execution evidence is missing".to_owned()),
    }

    let bind_path = runtime_evidence_path("rust-socks-bind-execution")?;
    match read_yaml::<RustSocksBindExecutionReport>(&bind_path).await? {
        Some(report) if report.status == RustSocksBindExecutionStatus::Executed => {
            evidence_paths.push(bind_path.to_string_lossy().to_string());
            executed_components.push(report.component.to_string());
            closeout_gates.push(report.default_data_plane_closeout_gate);
        }
        Some(report) => blockers.push(format!("SOCKS BIND evidence status is {:?}", report.status)),
        None => blockers.push("SOCKS BIND execution evidence is missing".to_owned()),
    }

    let default_closeout_gate_confirmed = closeout_gates
        .iter()
        .all(default_closeout_gate_allows_route_packet_capture);
    if !default_closeout_gate_confirmed {
        blockers
            .push("unsupported protocol evidence does not confirm closed-out default data-plane cutover".to_owned());
    }

    blockers.sort();
    blockers.dedup();
    Ok(RustUnsupportedProtocolExecutionGateEvidence {
        evidence_paths,
        executed_components,
        default_closeout_gate_confirmed,
        blockers,
    })
}

fn default_closeout_gate_allows_route_packet_capture(gate: &RustDefaultDataPlaneCloseoutGateEvidence) -> bool {
    gate.closeout_manifest_closed_out
        && gate.ownership_reconciled
        && gate.operator_default_path_cutover_committed
        && !gate.operator_default_path_cutover_fallback_scopes.is_empty()
        && gate.blockers.is_empty()
}

fn runtime_evidence_path(component: &str) -> Result<std::path::PathBuf> {
    Ok(dirs::app_runtime_dir()?.join(component).join(EVIDENCE_FILE))
}

async fn read_yaml<T>(path: &std::path::Path) -> Result<Option<T>>
where
    T: for<'de> Deserialize<'de>,
{
    match fs::read_to_string(path).await {
        Ok(yaml) => serde_yaml_ng::from_str(&yaml)
            .with_context(|| format!("failed to parse {}", path.display()))
            .map(Some),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error).with_context(|| format!("failed to read {}", path.display())),
    }
}

async fn route_snapshot_evidence() -> Result<RustRoutePacketCaptureRouteSnapshotEvidence> {
    let (program, args) = route_snapshot_command();
    let output = Command::new(program)
        .args(&args)
        .output()
        .with_context(|| format!("failed to run route snapshot command `{program}`"))?;
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let snapshot_path = evidence_dir()?.join(ROUTE_SNAPSHOT_FILE);
    if let Some(parent) = snapshot_path.parent() {
        fs::create_dir_all(parent).await?;
    }
    fs::write(&snapshot_path, stdout.as_bytes()).await?;
    let snapshot_present = output.status.success() && !stdout.trim().is_empty();
    let route_entries_observed = stdout
        .lines()
        .filter(|line| line.chars().any(|ch| ch.is_ascii_digit()))
        .count();
    let passed = snapshot_present && route_entries_observed > 0;
    let checksum = if stdout.is_empty() {
        None
    } else {
        Some(hex_sha256(stdout.as_bytes()))
    };

    Ok(RustRoutePacketCaptureRouteSnapshotEvidence {
        platform: std::env::consts::OS.to_owned(),
        command: std::iter::once(program.to_owned())
            .chain(args.iter().cloned())
            .collect(),
        snapshot_path: snapshot_path.to_string_lossy().to_string(),
        route_restore_plan_path: route_restore_plan_path()?.to_string_lossy().to_string(),
        snapshot_present,
        snapshot_checksum: checksum,
        route_entries_observed,
        mutates_routes: false,
        passed,
        blockers: evidence_blockers(passed, "route table snapshot evidence failed"),
    })
}

#[cfg(target_os = "windows")]
fn route_snapshot_command() -> (&'static str, Vec<String>) {
    ("route", vec!["print".to_owned(), "-4".to_owned()])
}

#[cfg(not(target_os = "windows"))]
fn route_snapshot_command() -> (&'static str, Vec<String>) {
    ("netstat", vec!["-rn".to_owned()])
}

fn restore_steps() -> Vec<RustRoutePacketCaptureRestoreStep> {
    vec![
        RustRoutePacketCaptureRestoreStep {
            order: 1,
            action: "capture pre-cutover route snapshot checksum".to_owned(),
            route_family: "ipv4".to_owned(),
            requires_privilege: false,
            mutates_routes_in_this_step: false,
        },
        RustRoutePacketCaptureRestoreStep {
            order: 2,
            action: "restore platform route table from approved rollback plan".to_owned(),
            route_family: "ipv4".to_owned(),
            requires_privilege: true,
            mutates_routes_in_this_step: false,
        },
        RustRoutePacketCaptureRestoreStep {
            order: 3,
            action: "verify route snapshot checksum after rollback".to_owned(),
            route_family: "ipv4".to_owned(),
            requires_privilege: false,
            mutates_routes_in_this_step: false,
        },
    ]
}

async fn write_restore_plan(restore_steps: &[RustRoutePacketCaptureRestoreStep]) -> Result<()> {
    let path = route_restore_plan_path()?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).await?;
    }
    fs::write(&path, serde_yaml_ng::to_string(restore_steps)?.as_bytes()).await?;
    Ok(())
}

async fn packet_capture_hold_evidence() -> Result<RustRoutePacketCapturePacketEvidence> {
    let packet = build_ipv4_tcp_packet(
        Ipv4Addr::new(10, 10, 0, 2),
        Ipv4Addr::new(198, 18, 0, 42),
        53122,
        443,
        b"hold-window-packet",
    );
    let parsed = parse_ipv4_tcp_packet(&packet)?;
    let record = PacketCaptureHoldRecord {
        source: format!("{}:{}", parsed.source, parsed.source_port),
        destination: parsed.destination.to_string(),
        destination_port: parsed.destination_port,
        iterations: 3,
        created_at_epoch_seconds: epoch_seconds(),
    };
    let hold_path = evidence_dir()?.join(PACKET_CAPTURE_HOLD_FILE);
    if let Some(parent) = hold_path.parent() {
        fs::create_dir_all(parent).await?;
    }
    let yaml = serde_yaml_ng::to_string(&record)?;
    fs::write(&hold_path, yaml.as_bytes()).await?;
    let passed =
        parsed.destination == Ipv4Addr::new(198, 18, 0, 42) && parsed.destination_port == 443 && record.iterations == 3;

    Ok(RustRoutePacketCapturePacketEvidence {
        packet_source: record.source,
        packet_destination: record.destination,
        packet_destination_port: record.destination_port,
        packet_capture_hold_path: hold_path.to_string_lossy().to_string(),
        ipv4_packet_parsed: true,
        tcp_destination_extracted: true,
        packet_hold_iterations: record.iterations,
        checksum: hex_sha256(yaml.as_bytes()),
        passed,
        blockers: evidence_blockers(passed, "bounded packet-capture hold parser evidence failed"),
    })
}

struct ParsedTcpPacket {
    source: Ipv4Addr,
    destination: Ipv4Addr,
    source_port: u16,
    destination_port: u16,
}

fn build_ipv4_tcp_packet(
    source: Ipv4Addr,
    destination: Ipv4Addr,
    source_port: u16,
    destination_port: u16,
    payload: &[u8],
) -> Vec<u8> {
    let ip_header_len = 20_u16;
    let tcp_header_len = 20_u16;
    let total_len = ip_header_len + tcp_header_len + payload.len() as u16;
    let mut packet = Vec::with_capacity(total_len as usize);
    packet.push(0x45);
    packet.push(0);
    packet.extend_from_slice(&total_len.to_be_bytes());
    packet.extend_from_slice(&0_u16.to_be_bytes());
    packet.extend_from_slice(&0_u16.to_be_bytes());
    packet.push(64);
    packet.push(6);
    packet.extend_from_slice(&0_u16.to_be_bytes());
    packet.extend_from_slice(&source.octets());
    packet.extend_from_slice(&destination.octets());
    packet.extend_from_slice(&source_port.to_be_bytes());
    packet.extend_from_slice(&destination_port.to_be_bytes());
    packet.extend_from_slice(&0_u32.to_be_bytes());
    packet.extend_from_slice(&0_u32.to_be_bytes());
    packet.push(0x50);
    packet.push(0x18);
    packet.extend_from_slice(&1024_u16.to_be_bytes());
    packet.extend_from_slice(&0_u16.to_be_bytes());
    packet.extend_from_slice(&0_u16.to_be_bytes());
    packet.extend_from_slice(payload);
    packet
}

fn parse_ipv4_tcp_packet(packet: &[u8]) -> Result<ParsedTcpPacket> {
    if packet.len() < 40 {
        bail!("IPv4/TCP packet too short");
    }
    if packet[0] >> 4 != 4 || packet[9] != 6 {
        bail!("packet is not IPv4/TCP");
    }
    let ip_header_len = usize::from(packet[0] & 0x0f) * 4;
    if ip_header_len < 20 || packet.len() < ip_header_len + 20 {
        bail!("invalid IPv4/TCP header length");
    }
    let source = Ipv4Addr::new(packet[12], packet[13], packet[14], packet[15]);
    let destination = Ipv4Addr::new(packet[16], packet[17], packet[18], packet[19]);
    let source_port = u16::from_be_bytes([packet[ip_header_len], packet[ip_header_len + 1]]);
    let destination_port = u16::from_be_bytes([packet[ip_header_len + 2], packet[ip_header_len + 3]]);
    Ok(ParsedTcpPacket {
        source,
        destination,
        source_port,
        destination_port,
    })
}

fn evidence_blockers(passed: bool, blocker: &str) -> Vec<String> {
    if passed { Vec::new() } else { vec![blocker.to_owned()] }
}

fn facts() -> Vec<String> {
    vec![
        "Rust captures a read-only platform route snapshot and checksum".to_owned(),
        "Rust writes a route restore plan without installing or deleting routes".to_owned(),
        "Rust parses synthetic IPv4/TCP packet-capture hold evidence while keeping Mihomo fallback".to_owned(),
    ]
}

fn evidence_dir() -> Result<std::path::PathBuf> {
    Ok(dirs::app_runtime_dir()?.join(COMPONENT))
}

pub fn rust_route_packet_capture_blocker_evidence_path() -> Result<std::path::PathBuf> {
    Ok(evidence_dir()?.join(EVIDENCE_FILE))
}

fn evidence_path() -> Result<std::path::PathBuf> {
    rust_route_packet_capture_blocker_evidence_path()
}

fn route_restore_plan_path() -> Result<std::path::PathBuf> {
    Ok(evidence_dir()?.join(ROUTE_RESTORE_PLAN_FILE))
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
    fn packet_round_trip_extracts_destination() {
        let packet = build_ipv4_tcp_packet(
            Ipv4Addr::new(10, 10, 0, 2),
            Ipv4Addr::new(198, 18, 0, 42),
            50000,
            443,
            b"payload",
        );
        let parsed = parse_ipv4_tcp_packet(&packet).unwrap();

        assert_eq!(parsed.destination, Ipv4Addr::new(198, 18, 0, 42));
        assert_eq!(parsed.destination_port, 443);
    }

    #[test]
    fn blocked_report_keeps_system_packet_capture_fallback() {
        let report = blocked_report(false, None, Vec::new());

        assert!(report.mihomo_system_packet_capture_fallback_required);
        assert!(!report.default_tun_replacement_allowed);
    }
}
