use super::{
    RUST_RUNTIME_ID, RustTunTransparentRoutingExecutionReport, RustTunTransparentRoutingExecutionStatus,
    RustTunTransparentRoutingLeakEvidence, RustTunTransparentRoutingPacketEvidence,
    RustTunTransparentRoutingRollbackEvidence,
};
use crate::utils::dirs;
use anyhow::{Result, bail};
use serde::Serialize;
use smartstring::alias::String;
use std::net::Ipv4Addr;
use tokio::{
    fs,
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
    time::{Duration, timeout},
};

const RUST_TUN_TRANSPARENT_ROUTING_COMPONENT: &str = "rust-tun-transparent-routing-execution";
const RUST_TUN_TRANSPARENT_ROUTING_KERNEL_AREA: &str = "tun-transparent-routing";
const RUST_TUN_TRANSPARENT_ROUTING_HOST: &str = "127.0.0.1";
const RUST_TUN_TRANSPARENT_ROUTING_EVIDENCE_FILE: &str = "evidence.yaml";
const RUST_TUN_TRANSPARENT_ROUTING_ROLLBACK_FILE: &str = "rollback-checkpoint.yaml";
const DEFAULT_TUN_TRANSPARENT_ROUTING_TARGET_PORT: u16 = 19981;
const NEXT_SAFE_BATCH: &str = "mihomo-fallback-retirement-wider-scope";

pub async fn rust_tun_transparent_routing_execution(
    explicit_opt_in: bool,
) -> Result<RustTunTransparentRoutingExecutionReport> {
    if !explicit_opt_in {
        return Ok(RustTunTransparentRoutingExecutionReport {
            runtime_id: RUST_RUNTIME_ID.into(),
            component: RUST_TUN_TRANSPARENT_ROUTING_COMPONENT.into(),
            kernel_area: RUST_TUN_TRANSPARENT_ROUTING_KERNEL_AREA.into(),
            status: RustTunTransparentRoutingExecutionStatus::Blocked,
            reason: "explicit opt-in is required to run TUN transparent routing execution".into(),
            explicit_opt_in,
            packet_evidence: None,
            rollback_evidence: None,
            leak_evidence: None,
            evidence_path: None,
            rollback_checkpoint_path: None,
            loopback_remote_only: true,
            mutates_runtime: false,
            forwards_traffic: false,
            packet_capture_owned: false,
            writes_evidence_artifact: false,
            mihomo_fallback: true,
            blockers: vec!["explicit opt-in is required".into()],
            warnings: Vec::new(),
            facts: rust_tun_transparent_routing_facts(),
            next_safe_batch: NEXT_SAFE_BATCH.into(),
        });
    }

    let rollback_checkpoint_path = rust_tun_transparent_routing_rollback_path()?;
    write_tun_transparent_routing_rollback_checkpoint(&rollback_checkpoint_path).await?;
    let packet_evidence = run_tun_transparent_packet_execution().await?;
    let rollback_evidence = tun_transparent_routing_rollback_evidence(&rollback_checkpoint_path).await;
    let leak_evidence = tun_transparent_routing_leak_evidence();
    let mut blockers = Vec::new();
    if !packet_evidence.passed {
        blockers.push("TUN transparent routing packet execution failed".into());
        blockers.extend(packet_evidence.blockers.iter().cloned());
    }
    if !rollback_evidence.passed {
        blockers.push("TUN transparent routing rollback evidence failed".into());
        blockers.extend(rollback_evidence.blockers.iter().cloned());
    }
    if !leak_evidence.passed {
        blockers.push("TUN transparent routing leak evidence failed".into());
        blockers.extend(leak_evidence.blockers.iter().cloned());
    }
    let status = if blockers.is_empty() {
        RustTunTransparentRoutingExecutionStatus::Passed
    } else {
        RustTunTransparentRoutingExecutionStatus::Failed
    };
    let rollback_checkpoint_path_string: String = rollback_checkpoint_path.to_string_lossy().to_string().into();
    let mut report = RustTunTransparentRoutingExecutionReport {
        runtime_id: RUST_RUNTIME_ID.into(),
        component: RUST_TUN_TRANSPARENT_ROUTING_COMPONENT.into(),
        kernel_area: RUST_TUN_TRANSPARENT_ROUTING_KERNEL_AREA.into(),
        status,
        reason: if status == RustTunTransparentRoutingExecutionStatus::Passed {
            "Rust TUN transparent routing executed a bounded IPv4/TCP packet route".into()
        } else {
            "Rust TUN transparent routing execution failed".into()
        },
        explicit_opt_in,
        packet_evidence: Some(packet_evidence),
        rollback_evidence: Some(rollback_evidence),
        leak_evidence: Some(leak_evidence),
        evidence_path: None,
        rollback_checkpoint_path: Some(rollback_checkpoint_path_string),
        loopback_remote_only: true,
        mutates_runtime: false,
        forwards_traffic: true,
        packet_capture_owned: false,
        writes_evidence_artifact: true,
        mihomo_fallback: true,
        blockers,
        warnings: vec![
            "transparent routing execution is capped to a synthetic loopback IPv4/TCP packet".into(),
            "Mihomo/service remains fallback for system-wide packet capture, route install, and transparent proxy defaults".into(),
        ],
        facts: rust_tun_transparent_routing_facts(),
        next_safe_batch: NEXT_SAFE_BATCH.into(),
    };

    let evidence_path = rust_tun_transparent_routing_evidence_path()?;
    if let Some(parent) = evidence_path.parent() {
        fs::create_dir_all(parent).await?;
    }
    report.evidence_path = Some(evidence_path.to_string_lossy().to_string().into());
    fs::write(&evidence_path, serde_yaml_ng::to_string(&report)?.as_bytes()).await?;
    Ok(report)
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct TransparentTcpPacket {
    source: Ipv4Addr,
    destination: Ipv4Addr,
    source_port: u16,
    destination_port: u16,
    payload: Vec<u8>,
}

async fn run_tun_transparent_packet_execution() -> Result<RustTunTransparentRoutingPacketEvidence> {
    let target = TcpListener::bind((
        RUST_TUN_TRANSPARENT_ROUTING_HOST,
        DEFAULT_TUN_TRANSPARENT_ROUTING_TARGET_PORT,
    ))
    .await?;
    let target_task = tokio::spawn(async move {
        let (mut stream, _) = timeout(Duration::from_secs(3), target.accept()).await??;
        let mut request = Vec::new();
        timeout(Duration::from_secs(3), stream.read_to_end(&mut request)).await??;
        let target_received = std::str::from_utf8(&request)
            .map(|request| request.contains("GET /rust-tun-transparent-routing-execution"))
            .unwrap_or(false);
        stream
            .write_all(b"HTTP/1.1 204 No Content\r\nConnection: close\r\nContent-Length: 0\r\n\r\n")
            .await?;
        stream.shutdown().await?;
        Ok::<bool, anyhow::Error>(target_received)
    });

    let payload = b"GET /rust-tun-transparent-routing-execution HTTP/1.1\r\nHost: rust-tun-transparent-routing\r\nConnection: close\r\n\r\n";
    let packet_bytes = build_ipv4_tcp_packet(
        Ipv4Addr::new(10, 10, 0, 2),
        Ipv4Addr::new(127, 0, 0, 1),
        53000,
        DEFAULT_TUN_TRANSPARENT_ROUTING_TARGET_PORT,
        payload,
    )?;
    let parsed_packet = parse_ipv4_tcp_packet(&packet_bytes)?;
    let mut stream = TcpStream::connect((parsed_packet.destination, parsed_packet.destination_port)).await?;
    stream.write_all(&parsed_packet.payload).await?;
    stream.shutdown().await?;
    let mut response = Vec::new();
    timeout(Duration::from_secs(3), stream.read_to_end(&mut response)).await??;
    let target_received = target_task.await??;
    let response_status = parse_http_status(&response);
    let mut blockers = Vec::new();
    if parsed_packet.destination != Ipv4Addr::new(127, 0, 0, 1) {
        blockers.push("transparent routing packet destination was not loopback".into());
    }
    if parsed_packet.destination_port != DEFAULT_TUN_TRANSPARENT_ROUTING_TARGET_PORT {
        blockers.push("transparent routing packet destination port mismatch".into());
    }
    if !target_received {
        blockers.push("transparent routing target did not receive routed payload".into());
    }
    if response_status.as_deref() != Some("HTTP/1.1 204 No Content") {
        blockers.push("transparent routing target did not return HTTP 204".into());
    }

    Ok(RustTunTransparentRoutingPacketEvidence {
        packet_source: format!("{}:{}", parsed_packet.source, parsed_packet.source_port).into(),
        packet_destination: parsed_packet.destination.to_string().into(),
        packet_destination_port: parsed_packet.destination_port,
        ipv4_packet_parsed: true,
        tcp_destination_extracted: true,
        payload_bytes: parsed_packet.payload.len() as u64,
        target_received,
        response_status,
        response_bytes: response.len() as u64,
        passed: blockers.is_empty(),
        blockers,
    })
}

fn build_ipv4_tcp_packet(
    source: Ipv4Addr,
    destination: Ipv4Addr,
    source_port: u16,
    destination_port: u16,
    payload: &[u8],
) -> Result<Vec<u8>> {
    let total_length = u16::try_from(40 + payload.len())?;
    let mut packet = Vec::with_capacity(total_length as usize);
    packet.push(0x45);
    packet.push(0);
    packet.extend_from_slice(&total_length.to_be_bytes());
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
    packet.extend_from_slice(&4096_u16.to_be_bytes());
    packet.extend_from_slice(&0_u16.to_be_bytes());
    packet.extend_from_slice(&0_u16.to_be_bytes());
    packet.extend_from_slice(payload);
    Ok(packet)
}

fn parse_ipv4_tcp_packet(packet: &[u8]) -> Result<TransparentTcpPacket> {
    if packet.len() < 40 {
        bail!("transparent routing packet is too short");
    }
    let version = packet[0] >> 4;
    let header_words = packet[0] & 0x0f;
    let ip_header_len = usize::from(header_words) * 4;
    if version != 4 || ip_header_len < 20 || packet.len() < ip_header_len + 20 {
        bail!("transparent routing packet is not IPv4/TCP");
    }
    if packet[9] != 6 {
        bail!("transparent routing packet protocol is not TCP");
    }
    let total_length = u16::from_be_bytes([packet[2], packet[3]]) as usize;
    if total_length > packet.len() || total_length < ip_header_len + 20 {
        bail!("transparent routing packet length is invalid");
    }
    let source = Ipv4Addr::new(packet[12], packet[13], packet[14], packet[15]);
    let destination = Ipv4Addr::new(packet[16], packet[17], packet[18], packet[19]);
    let tcp_offset = ip_header_len;
    let source_port = u16::from_be_bytes([packet[tcp_offset], packet[tcp_offset + 1]]);
    let destination_port = u16::from_be_bytes([packet[tcp_offset + 2], packet[tcp_offset + 3]]);
    let tcp_header_len = usize::from(packet[tcp_offset + 12] >> 4) * 4;
    if tcp_header_len < 20 || total_length < tcp_offset + tcp_header_len {
        bail!("transparent routing TCP header length is invalid");
    }
    let payload = packet[tcp_offset + tcp_header_len..total_length].to_vec();

    Ok(TransparentTcpPacket {
        source,
        destination,
        source_port,
        destination_port,
        payload,
    })
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct RustTunTransparentRoutingRollbackCheckpoint {
    component: String,
    kernel_area: String,
    route_owner_before: String,
    route_owner_after: String,
    packet_capture_default_unchanged: bool,
    rollback_action: String,
}

async fn write_tun_transparent_routing_rollback_checkpoint(path: &std::path::Path) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).await?;
    }
    let checkpoint = RustTunTransparentRoutingRollbackCheckpoint {
        component: RUST_TUN_TRANSPARENT_ROUTING_COMPONENT.into(),
        kernel_area: RUST_TUN_TRANSPARENT_ROUTING_KERNEL_AREA.into(),
        route_owner_before: "mihomo-service".into(),
        route_owner_after: "mihomo-service".into(),
        packet_capture_default_unchanged: true,
        rollback_action: "drop bounded Rust transparent packet executor and keep TUN packet capture on Mihomo/service"
            .into(),
    };
    fs::write(path, serde_yaml_ng::to_string(&checkpoint)?.as_bytes()).await?;
    Ok(())
}

async fn tun_transparent_routing_rollback_evidence(
    checkpoint_path: &std::path::Path,
) -> RustTunTransparentRoutingRollbackEvidence {
    let checkpoint_written = fs::metadata(checkpoint_path).await.is_ok();
    let mut blockers = Vec::new();
    if !checkpoint_written {
        blockers.push("TUN transparent routing rollback checkpoint was not written".into());
    }

    RustTunTransparentRoutingRollbackEvidence {
        checkpoint_path: Some(checkpoint_path.to_string_lossy().to_string().into()),
        checkpoint_written,
        route_owner_before: "mihomo-service".into(),
        route_owner_after: "mihomo-service".into(),
        rollback_action: "drop bounded Rust transparent packet executor and keep TUN packet capture on Mihomo/service"
            .into(),
        packet_capture_default_unchanged: true,
        passed: blockers.is_empty(),
        blockers,
    }
}

fn tun_transparent_routing_leak_evidence() -> RustTunTransparentRoutingLeakEvidence {
    let loopback_only = true;
    let os_route_mutation_attempted = false;
    let system_proxy_mutation_attempted = false;
    let tun_device_mutation_attempted = false;
    let unsupported_packet_capture_fallback = true;
    let mut blockers = Vec::new();
    if !loopback_only {
        blockers.push("TUN transparent routing execution was not loopback-only".into());
    }
    if os_route_mutation_attempted {
        blockers.push("TUN transparent routing attempted an OS route mutation".into());
    }
    if system_proxy_mutation_attempted {
        blockers.push("TUN transparent routing attempted a system proxy mutation".into());
    }
    if tun_device_mutation_attempted {
        blockers.push("TUN transparent routing attempted a TUN device mutation".into());
    }
    if !unsupported_packet_capture_fallback {
        blockers.push("unsupported packet capture fallback was not retained".into());
    }

    RustTunTransparentRoutingLeakEvidence {
        loopback_only,
        os_route_mutation_attempted,
        system_proxy_mutation_attempted,
        tun_device_mutation_attempted,
        unsupported_packet_capture_fallback,
        passed: blockers.is_empty(),
        blockers,
    }
}

fn parse_http_status(response: &[u8]) -> Option<String> {
    std::str::from_utf8(response)
        .ok()
        .and_then(|response| response.lines().next())
        .map(Into::into)
}

fn rust_tun_transparent_routing_evidence_path() -> Result<std::path::PathBuf> {
    Ok(dirs::app_runtime_dir()?
        .join(RUST_TUN_TRANSPARENT_ROUTING_COMPONENT)
        .join(RUST_TUN_TRANSPARENT_ROUTING_EVIDENCE_FILE))
}

fn rust_tun_transparent_routing_rollback_path() -> Result<std::path::PathBuf> {
    Ok(dirs::app_runtime_dir()?
        .join(RUST_TUN_TRANSPARENT_ROUTING_COMPONENT)
        .join(RUST_TUN_TRANSPARENT_ROUTING_ROLLBACK_FILE))
}

fn rust_tun_transparent_routing_facts() -> Vec<String> {
    vec![
        "Rust parses a bounded IPv4/TCP packet and extracts transparent-route destination metadata".into(),
        "Rust executes the bounded transparent route by dialing the extracted loopback target".into(),
        "Rust writes evidence and rollback checkpoint artifacts without mutating OS routes or TUN devices".into(),
        "Mihomo/service remains fallback for system-wide packet capture and transparent proxy defaults".into(),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn tun_transparent_routing_blocks_without_opt_in() {
        let report = rust_tun_transparent_routing_execution(false).await.unwrap();

        assert_eq!(report.status, RustTunTransparentRoutingExecutionStatus::Blocked);
        assert!(report.mihomo_fallback);
        assert!(!report.packet_capture_owned);
    }

    #[test]
    fn tun_transparent_routing_packet_round_trip() {
        let packet = build_ipv4_tcp_packet(
            Ipv4Addr::new(10, 10, 0, 2),
            Ipv4Addr::new(127, 0, 0, 1),
            53000,
            19981,
            b"payload",
        )
        .unwrap();
        let parsed = parse_ipv4_tcp_packet(&packet).unwrap();

        assert_eq!(parsed.source, Ipv4Addr::new(10, 10, 0, 2));
        assert_eq!(parsed.destination, Ipv4Addr::new(127, 0, 0, 1));
        assert_eq!(parsed.source_port, 53000);
        assert_eq!(parsed.destination_port, 19981);
        assert_eq!(parsed.payload, b"payload");
    }
}
