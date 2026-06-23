use super::{
    RUST_RUNTIME_ID, RustSocksUdpAssociateExecutionReport, RustSocksUdpAssociateExecutionStatus,
    RustSocksUdpAssociateLeakEvidence, RustSocksUdpAssociatePacketEvidence, RustSocksUdpAssociateRollbackEvidence,
};
use crate::utils::dirs;
use anyhow::{Context as _, Result, anyhow};
use serde::{Deserialize, Serialize};
use smartstring::alias::String;
use std::{
    net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr, UdpSocket},
    thread,
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use tokio::fs;

const RUST_SOCKS_UDP_ASSOCIATE_COMPONENT: &str = "rust-socks-udp-associate-execution";
const RUST_SOCKS_UDP_ASSOCIATE_KERNEL_AREA: &str = "socks-udp-associate";
const RUST_SOCKS_UDP_ASSOCIATE_EVIDENCE_FILE: &str = "evidence.yaml";
const RUST_SOCKS_UDP_ASSOCIATE_ROLLBACK_FILE: &str = "rollback-checkpoint.yaml";
const NEXT_SAFE_BATCH: &str = "unsupported-protocol-and-packet-capture-implementation";
const SOCKS_UDP_ECHO_PREFIX: &[u8] = b"udp-associate-ok:";

pub async fn rust_socks_udp_associate_execution(explicit_opt_in: bool) -> Result<RustSocksUdpAssociateExecutionReport> {
    if !explicit_opt_in {
        return Ok(blocked_report(
            explicit_opt_in,
            vec!["SOCKS UDP associate execution requires explicit opt-in".into()],
        ));
    }

    let rollback_path = rust_socks_udp_associate_rollback_path()?;
    let rollback_evidence = write_rollback_checkpoint(&rollback_path).await?;
    let packet_evidence = match run_bounded_socks_udp_associate_datagram() {
        Ok(evidence) => evidence,
        Err(error) => {
            return Ok(blocked_report(
                explicit_opt_in,
                vec![format!("bounded SOCKS UDP associate execution failed: {error}").into()],
            ));
        }
    };
    let leak_evidence = RustSocksUdpAssociateLeakEvidence {
        passed: packet_evidence.loopback_only && packet_evidence.datagram_round_trip && !packet_evidence.frag_supported,
        no_system_packet_capture: true,
        no_non_loopback_target: packet_evidence.loopback_only,
        no_mihomo_binary_removal: true,
    };
    let evidence_path = rust_socks_udp_associate_evidence_path()?;
    let mut report = RustSocksUdpAssociateExecutionReport {
        runtime_id: RUST_RUNTIME_ID.into(),
        component: RUST_SOCKS_UDP_ASSOCIATE_COMPONENT.into(),
        kernel_area: RUST_SOCKS_UDP_ASSOCIATE_KERNEL_AREA.into(),
        status: RustSocksUdpAssociateExecutionStatus::Executed,
        reason: "Rust executed a bounded SOCKS5 UDP ASSOCIATE datagram over loopback UDP".into(),
        explicit_opt_in,
        rust_owned_scope: "SOCKS5 UDP ASSOCIATE datagram framing and loopback UDP forwarding".into(),
        mutates_runtime: false,
        writes_evidence: true,
        evidence_path: Some(evidence_path.to_string_lossy().to_string().into()),
        packet_evidence: Some(packet_evidence),
        rollback_evidence: Some(rollback_evidence),
        leak_evidence: Some(leak_evidence),
        mihomo_fallback_retained_for: retained_socks_udp_fallback_scope(),
        blockers: Vec::new(),
        warnings: vec![
            "SOCKS TCP CONNECT/BIND data handling, broad fragment queues/timeouts, and non-loopback UDP remain Mihomo-owned".into(),
        ],
        facts: rust_socks_udp_associate_facts(),
        next_safe_batch: NEXT_SAFE_BATCH.into(),
    };

    if let Some(parent) = evidence_path.parent() {
        fs::create_dir_all(parent).await?;
    }
    fs::write(&evidence_path, serde_yaml_ng::to_string(&report)?.as_bytes()).await?;
    report.evidence_path = Some(evidence_path.to_string_lossy().to_string().into());

    Ok(report)
}

fn blocked_report(explicit_opt_in: bool, blockers: Vec<String>) -> RustSocksUdpAssociateExecutionReport {
    RustSocksUdpAssociateExecutionReport {
        runtime_id: RUST_RUNTIME_ID.into(),
        component: RUST_SOCKS_UDP_ASSOCIATE_COMPONENT.into(),
        kernel_area: RUST_SOCKS_UDP_ASSOCIATE_KERNEL_AREA.into(),
        status: RustSocksUdpAssociateExecutionStatus::Blocked,
        reason: "Rust SOCKS UDP associate execution is blocked".into(),
        explicit_opt_in,
        rust_owned_scope: "SOCKS5 UDP ASSOCIATE datagram framing and loopback UDP forwarding".into(),
        mutates_runtime: false,
        writes_evidence: false,
        evidence_path: None,
        packet_evidence: None,
        rollback_evidence: None,
        leak_evidence: None,
        mihomo_fallback_retained_for: retained_socks_udp_fallback_scope(),
        blockers,
        warnings: Vec::new(),
        facts: rust_socks_udp_associate_facts(),
        next_safe_batch: NEXT_SAFE_BATCH.into(),
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RustSocksUdpAssociateRollbackCheckpoint {
    component: String,
    rust_owned_scope: String,
    fallback_retained_for: Vec<String>,
    created_at_epoch_seconds: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct SocksUdpDatagram {
    request_atyp: SocksUdpAddressType,
    target: SocketAddr,
    payload: Vec<u8>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SocksUdpAddressType {
    Ipv4,
    Domain,
    Ipv6,
}

impl SocksUdpAddressType {
    fn label(self) -> &'static str {
        match self {
            SocksUdpAddressType::Ipv4 => "ipv4",
            SocksUdpAddressType::Domain => "domain",
            SocksUdpAddressType::Ipv6 => "ipv6",
        }
    }
}

async fn write_rollback_checkpoint(rollback_path: &std::path::Path) -> Result<RustSocksUdpAssociateRollbackEvidence> {
    let created_at_epoch_seconds = rust_socks_udp_associate_epoch_seconds();
    let checkpoint = RustSocksUdpAssociateRollbackCheckpoint {
        component: RUST_SOCKS_UDP_ASSOCIATE_COMPONENT.into(),
        rust_owned_scope: "bounded SOCKS5 UDP ASSOCIATE loopback datagram".into(),
        fallback_retained_for: retained_socks_udp_fallback_scope(),
        created_at_epoch_seconds,
    };
    if let Some(parent) = rollback_path.parent() {
        fs::create_dir_all(parent).await?;
    }
    fs::write(rollback_path, serde_yaml_ng::to_string(&checkpoint)?.as_bytes()).await?;

    Ok(RustSocksUdpAssociateRollbackEvidence {
        checkpoint_path: rollback_path.to_string_lossy().to_string().into(),
        fallback_retained_for: checkpoint.fallback_retained_for,
        created_at_epoch_seconds,
    })
}

fn run_bounded_socks_udp_associate_datagram() -> Result<RustSocksUdpAssociatePacketEvidence> {
    let echo = UdpSocket::bind((Ipv4Addr::LOCALHOST, 0)).context("failed to bind UDP target")?;
    echo.set_read_timeout(Some(Duration::from_secs(2)))?;
    echo.set_write_timeout(Some(Duration::from_secs(2)))?;
    let target = echo.local_addr()?;
    let echo_thread = thread::spawn(move || -> Result<()> {
        let mut buffer = [0_u8; 512];
        let (received, peer) = echo.recv_from(&mut buffer)?;
        let mut response = SOCKS_UDP_ECHO_PREFIX.to_vec();
        response.extend_from_slice(&buffer[..received]);
        echo.send_to(&response, peer)?;
        Ok(())
    });

    let payload = b"bounded socks udp associate payload";
    let request = encode_socks_udp_datagram(target, payload);
    let parsed = parse_socks_udp_datagram(&request)?;
    ensure_loopback_target(parsed.target)?;

    let relay = UdpSocket::bind((Ipv4Addr::LOCALHOST, 0)).context("failed to bind UDP relay")?;
    relay.set_read_timeout(Some(Duration::from_secs(2)))?;
    relay.set_write_timeout(Some(Duration::from_secs(2)))?;
    relay.send_to(&parsed.payload, parsed.target)?;
    let mut response = [0_u8; 512];
    let (response_len, response_peer) = relay.recv_from(&mut response)?;
    echo_thread
        .join()
        .map_err(|_| anyhow!("UDP target thread panicked"))??;

    let response_frame = encode_socks_udp_datagram(response_peer, &response[..response_len]);
    let parsed_response = parse_socks_udp_datagram(&response_frame)?;
    let response_payload_prefix =
        std::str::from_utf8(&parsed_response.payload[..SOCKS_UDP_ECHO_PREFIX.len().min(parsed_response.payload.len())])
            .unwrap_or_default()
            .to_string();

    Ok(RustSocksUdpAssociatePacketEvidence {
        request_atyp: parsed.request_atyp.label().into(),
        target_addr: parsed.target.ip().to_string().into(),
        target_port: parsed.target.port(),
        request_payload_bytes: parsed.payload.len(),
        response_payload_bytes: parsed_response.payload.len(),
        response_payload_prefix: response_payload_prefix.into(),
        datagram_round_trip: parsed_response.payload.starts_with(SOCKS_UDP_ECHO_PREFIX),
        frag_supported: false,
        loopback_only: parsed.target.ip().is_loopback() && response_peer.ip().is_loopback(),
    })
}

fn parse_socks_udp_datagram(packet: &[u8]) -> Result<SocksUdpDatagram> {
    if packet.len() < 4 {
        return Err(anyhow!("SOCKS UDP packet is shorter than RSV/FRAG/ATYP"));
    }
    if packet[0] != 0 || packet[1] != 0 {
        return Err(anyhow!("SOCKS UDP RSV bytes must be zero"));
    }
    if packet[2] != 0 {
        return Err(anyhow!("fragmented SOCKS UDP packets remain Mihomo-owned"));
    }

    match packet[3] {
        0x01 => parse_socks_udp_ipv4(packet),
        0x03 => parse_socks_udp_domain(packet),
        0x04 => parse_socks_udp_ipv6(packet),
        atyp => Err(anyhow!("unsupported SOCKS UDP ATYP {atyp:#04x}")),
    }
}

fn parse_socks_udp_ipv4(packet: &[u8]) -> Result<SocksUdpDatagram> {
    if packet.len() < 10 {
        return Err(anyhow!("SOCKS UDP IPv4 packet is truncated"));
    }
    let ip = Ipv4Addr::new(packet[4], packet[5], packet[6], packet[7]);
    let port = u16::from_be_bytes([packet[8], packet[9]]);
    Ok(SocksUdpDatagram {
        request_atyp: SocksUdpAddressType::Ipv4,
        target: SocketAddr::new(IpAddr::V4(ip), port),
        payload: packet[10..].to_vec(),
    })
}

fn parse_socks_udp_domain(packet: &[u8]) -> Result<SocksUdpDatagram> {
    if packet.len() < 5 {
        return Err(anyhow!("SOCKS UDP domain packet is truncated"));
    }
    let domain_len = usize::from(packet[4]);
    let port_offset = 5 + domain_len;
    if packet.len() < port_offset + 2 {
        return Err(anyhow!("SOCKS UDP domain packet is missing port"));
    }
    let domain = std::str::from_utf8(&packet[5..port_offset]).context("SOCKS UDP domain is not valid UTF-8")?;
    let ip = match domain {
        "localhost" => IpAddr::V4(Ipv4Addr::LOCALHOST),
        "::1" => IpAddr::V6(Ipv6Addr::LOCALHOST),
        _ => return Err(anyhow!("SOCKS UDP domain target remains Mihomo-owned: {domain}")),
    };
    let port = u16::from_be_bytes([packet[port_offset], packet[port_offset + 1]]);
    Ok(SocksUdpDatagram {
        request_atyp: SocksUdpAddressType::Domain,
        target: SocketAddr::new(ip, port),
        payload: packet[port_offset + 2..].to_vec(),
    })
}

fn parse_socks_udp_ipv6(packet: &[u8]) -> Result<SocksUdpDatagram> {
    if packet.len() < 22 {
        return Err(anyhow!("SOCKS UDP IPv6 packet is truncated"));
    }
    let mut octets = [0_u8; 16];
    octets.copy_from_slice(&packet[4..20]);
    let port = u16::from_be_bytes([packet[20], packet[21]]);
    Ok(SocksUdpDatagram {
        request_atyp: SocksUdpAddressType::Ipv6,
        target: SocketAddr::new(IpAddr::V6(Ipv6Addr::from(octets)), port),
        payload: packet[22..].to_vec(),
    })
}

fn encode_socks_udp_datagram(target: SocketAddr, payload: &[u8]) -> Vec<u8> {
    let mut packet = vec![0, 0, 0];
    match target.ip() {
        IpAddr::V4(ip) => {
            packet.push(0x01);
            packet.extend_from_slice(&ip.octets());
        }
        IpAddr::V6(ip) => {
            packet.push(0x04);
            packet.extend_from_slice(&ip.octets());
        }
    }
    packet.extend_from_slice(&target.port().to_be_bytes());
    packet.extend_from_slice(payload);
    packet
}

fn ensure_loopback_target(target: SocketAddr) -> Result<()> {
    if target.ip().is_loopback() {
        Ok(())
    } else {
        Err(anyhow!(
            "SOCKS UDP target {target} is outside the bounded loopback scope"
        ))
    }
}

fn retained_socks_udp_fallback_scope() -> Vec<String> {
    vec![
        "SOCKS authentication and TCP command negotiation".into(),
        "SOCKS UDP broad fragment queues/timeouts".into(),
        "SOCKS UDP non-loopback forwarding".into(),
        "system-wide packet capture and transparent proxy defaults".into(),
        "VMess, VLESS, Trojan TLS, Shadowsocks UDP/plugin transports".into(),
    ]
}

fn rust_socks_udp_associate_facts() -> Vec<String> {
    vec![
        "Rust parses SOCKS5 UDP ASSOCIATE RSV/FRAG/ATYP/DST.PORT datagrams".into(),
        "Rust leaves fragmented UDP datagrams to the separate bounded fragment path or Mihomo fallback".into(),
        "Rust forwards only bounded loopback UDP targets and writes rollback/evidence artifacts".into(),
        "Mihomo fallback remains retained for authentication, non-loopback UDP, broad fragment queues/timeouts, and packet capture".into(),
    ]
}

fn rust_socks_udp_associate_dir() -> Result<std::path::PathBuf> {
    Ok(dirs::app_runtime_dir()?.join(RUST_SOCKS_UDP_ASSOCIATE_COMPONENT))
}

fn rust_socks_udp_associate_evidence_path() -> Result<std::path::PathBuf> {
    Ok(rust_socks_udp_associate_dir()?.join(RUST_SOCKS_UDP_ASSOCIATE_EVIDENCE_FILE))
}

fn rust_socks_udp_associate_rollback_path() -> Result<std::path::PathBuf> {
    Ok(rust_socks_udp_associate_dir()?.join(RUST_SOCKS_UDP_ASSOCIATE_ROLLBACK_FILE))
}

fn rust_socks_udp_associate_epoch_seconds() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_ipv4_loopback_datagram() {
        let target = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 5353);
        let packet = encode_socks_udp_datagram(target, b"hello");
        let parsed = parse_socks_udp_datagram(&packet).unwrap();

        assert_eq!(parsed.request_atyp, SocksUdpAddressType::Ipv4);
        assert_eq!(parsed.target, target);
        assert_eq!(parsed.payload, b"hello");
        assert!(ensure_loopback_target(parsed.target).is_ok());
    }

    #[test]
    fn rejects_fragmented_datagram() {
        let target = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 5353);
        let mut packet = encode_socks_udp_datagram(target, b"hello");
        packet[2] = 1;

        let error = parse_socks_udp_datagram(&packet).unwrap_err().to_string();

        assert!(error.contains("fragmented SOCKS UDP packets remain Mihomo-owned"));
    }

    #[test]
    fn rejects_non_loopback_target() {
        let target = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(8, 8, 8, 8)), 53);

        let error = ensure_loopback_target(target).unwrap_err().to_string();

        assert!(error.contains("outside the bounded loopback scope"));
    }
}
