use super::{
    RUST_RUNTIME_ID, RustSocksUdpFragmentsExecutionReport, RustSocksUdpFragmentsExecutionStatus,
    RustSocksUdpFragmentsLeakEvidence, RustSocksUdpFragmentsPacketEvidence, RustSocksUdpFragmentsRollbackEvidence,
};
use crate::utils::dirs;
use anyhow::{Context as _, Result, anyhow};
use serde::{Deserialize, Serialize};
use smartstring::alias::String;
use std::{
    net::{IpAddr, Ipv4Addr, SocketAddr, UdpSocket},
    thread,
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use tokio::fs;

const RUST_SOCKS_UDP_FRAGMENTS_COMPONENT: &str = "rust-socks-udp-fragments-execution";
const RUST_SOCKS_UDP_FRAGMENTS_KERNEL_AREA: &str = "socks-udp-fragments";
const RUST_SOCKS_UDP_FRAGMENTS_EVIDENCE_FILE: &str = "evidence.yaml";
const RUST_SOCKS_UDP_FRAGMENTS_ROLLBACK_FILE: &str = "rollback-checkpoint.yaml";
const NEXT_SAFE_BATCH: &str = "unsupported-protocol-and-packet-capture-implementation";
const SOCKS_UDP_ECHO_PREFIX: &[u8] = b"udp-fragments-ok:";
const SOCKS_UDP_FRAGMENT_ONE: u8 = 0x01;
const SOCKS_UDP_FRAGMENT_FINAL_TWO: u8 = 0x82;
const SOCKS_UDP_FINAL_MASK: u8 = 0x80;
const SOCKS_UDP_FRAGMENT_INDEX_MASK: u8 = 0x7f;
const TEST_FRAGMENT_ONE: &[u8] = b"bounded socks ";
const TEST_FRAGMENT_TWO: &[u8] = b"udp fragments payload";

pub async fn rust_socks_udp_fragments_execution(explicit_opt_in: bool) -> Result<RustSocksUdpFragmentsExecutionReport> {
    if !explicit_opt_in {
        return Ok(blocked_report(
            explicit_opt_in,
            vec!["SOCKS UDP fragment execution requires explicit opt-in".into()],
        ));
    }

    let rollback_path = rust_socks_udp_fragments_rollback_path()?;
    let rollback_evidence = write_rollback_checkpoint(&rollback_path).await?;
    let packet_evidence = match run_bounded_socks_udp_fragment_reassembly() {
        Ok(evidence) => evidence,
        Err(error) => {
            return Ok(blocked_report(
                explicit_opt_in,
                vec![format!("bounded SOCKS UDP fragment execution failed: {error}").into()],
            ));
        }
    };
    let leak_evidence = RustSocksUdpFragmentsLeakEvidence {
        passed: packet_evidence.loopback_only
            && packet_evidence.fragments_reassembled
            && packet_evidence.datagram_round_trip,
        no_system_packet_capture: true,
        no_non_loopback_target: packet_evidence.loopback_only,
        no_mihomo_binary_removal: true,
    };
    let evidence_path = rust_socks_udp_fragments_evidence_path()?;
    let mut report = RustSocksUdpFragmentsExecutionReport {
        runtime_id: RUST_RUNTIME_ID.into(),
        component: RUST_SOCKS_UDP_FRAGMENTS_COMPONENT.into(),
        kernel_area: RUST_SOCKS_UDP_FRAGMENTS_KERNEL_AREA.into(),
        status: RustSocksUdpFragmentsExecutionStatus::Executed,
        reason: "Rust executed bounded SOCKS5 UDP fragment reassembly over loopback".into(),
        explicit_opt_in,
        rust_owned_scope: "SOCKS5 UDP two-fragment loopback reassembly and forwarding".into(),
        mutates_runtime: false,
        writes_evidence: true,
        evidence_path: Some(evidence_path.to_string_lossy().to_string().into()),
        packet_evidence: Some(packet_evidence),
        rollback_evidence: Some(rollback_evidence),
        leak_evidence: Some(leak_evidence),
        mihomo_fallback_retained_for: retained_socks_udp_fragments_fallback_scope(),
        blockers: Vec::new(),
        warnings: vec![
            "SOCKS UDP non-loopback forwarding, fragment windows/timeouts, encrypted protocols, and packet capture remain Mihomo-owned".into(),
        ],
        facts: rust_socks_udp_fragments_facts(),
        next_safe_batch: NEXT_SAFE_BATCH.into(),
    };

    if let Some(parent) = evidence_path.parent() {
        fs::create_dir_all(parent).await?;
    }
    fs::write(&evidence_path, serde_yaml_ng::to_string(&report)?.as_bytes()).await?;
    report.evidence_path = Some(evidence_path.to_string_lossy().to_string().into());

    Ok(report)
}

fn blocked_report(explicit_opt_in: bool, blockers: Vec<String>) -> RustSocksUdpFragmentsExecutionReport {
    RustSocksUdpFragmentsExecutionReport {
        runtime_id: RUST_RUNTIME_ID.into(),
        component: RUST_SOCKS_UDP_FRAGMENTS_COMPONENT.into(),
        kernel_area: RUST_SOCKS_UDP_FRAGMENTS_KERNEL_AREA.into(),
        status: RustSocksUdpFragmentsExecutionStatus::Blocked,
        reason: "Rust SOCKS UDP fragment execution is blocked".into(),
        explicit_opt_in,
        rust_owned_scope: "SOCKS5 UDP two-fragment loopback reassembly and forwarding".into(),
        mutates_runtime: false,
        writes_evidence: false,
        evidence_path: None,
        packet_evidence: None,
        rollback_evidence: None,
        leak_evidence: None,
        mihomo_fallback_retained_for: retained_socks_udp_fragments_fallback_scope(),
        blockers,
        warnings: Vec::new(),
        facts: rust_socks_udp_fragments_facts(),
        next_safe_batch: NEXT_SAFE_BATCH.into(),
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RustSocksUdpFragmentsRollbackCheckpoint {
    component: String,
    rust_owned_scope: String,
    fallback_retained_for: Vec<String>,
    created_at_epoch_seconds: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct SocksUdpFragment {
    frag: u8,
    target: SocketAddr,
    payload: Vec<u8>,
}

impl SocksUdpFragment {
    fn index(&self) -> u8 {
        self.frag & SOCKS_UDP_FRAGMENT_INDEX_MASK
    }

    fn is_final(&self) -> bool {
        self.frag & SOCKS_UDP_FINAL_MASK != 0
    }
}

fn run_bounded_socks_udp_fragment_reassembly() -> Result<RustSocksUdpFragmentsPacketEvidence> {
    let echo = UdpSocket::bind((Ipv4Addr::LOCALHOST, 0)).context("failed to bind UDP target")?;
    echo.set_read_timeout(Some(Duration::from_secs(2)))?;
    echo.set_write_timeout(Some(Duration::from_secs(2)))?;
    let target = echo.local_addr()?;
    let echo_thread = thread::spawn(move || -> Result<usize> {
        let mut buffer = [0_u8; 512];
        let (received, peer) = echo.recv_from(&mut buffer)?;
        let mut response = SOCKS_UDP_ECHO_PREFIX.to_vec();
        response.extend_from_slice(&buffer[..received]);
        echo.send_to(&response, peer)?;
        Ok(received)
    });

    let fragments = [
        parse_socks_udp_fragment(&encode_socks_udp_fragment(
            target,
            SOCKS_UDP_FRAGMENT_ONE,
            TEST_FRAGMENT_ONE,
        ))?,
        parse_socks_udp_fragment(&encode_socks_udp_fragment(
            target,
            SOCKS_UDP_FRAGMENT_FINAL_TWO,
            TEST_FRAGMENT_TWO,
        ))?,
    ];
    let reassembled = reassemble_ordered_fragments(&fragments)?;
    ensure_loopback_target(target)?;

    let relay = UdpSocket::bind((Ipv4Addr::LOCALHOST, 0)).context("failed to bind UDP relay")?;
    relay.set_read_timeout(Some(Duration::from_secs(2)))?;
    relay.set_write_timeout(Some(Duration::from_secs(2)))?;
    relay.send_to(&reassembled, target)?;
    let mut response = [0_u8; 512];
    let (response_len, response_peer) = relay.recv_from(&mut response)?;
    let target_received_bytes = echo_thread
        .join()
        .map_err(|_| anyhow!("UDP target thread panicked"))??;
    ensure_loopback_target(response_peer)?;

    let response_payload = &response[..response_len];
    let response_payload_prefix =
        std::str::from_utf8(&response_payload[..SOCKS_UDP_ECHO_PREFIX.len().min(response_len)])
            .unwrap_or_default()
            .to_string();
    let expected_payload_bytes = TEST_FRAGMENT_ONE.len() + TEST_FRAGMENT_TWO.len();

    Ok(RustSocksUdpFragmentsPacketEvidence {
        target_addr: target.ip().to_string().into(),
        target_port: target.port(),
        fragment_count: fragments.len(),
        first_fragment: format!("0x{SOCKS_UDP_FRAGMENT_ONE:02x}").into(),
        final_fragment: format!("0x{SOCKS_UDP_FRAGMENT_FINAL_TWO:02x}").into(),
        request_payload_bytes: expected_payload_bytes,
        reassembled_payload_bytes: reassembled.len(),
        target_received_bytes,
        response_payload_bytes: response_len,
        response_payload_prefix: response_payload_prefix.into(),
        fragments_reassembled: reassembled.len() == expected_payload_bytes,
        datagram_round_trip: response_payload.starts_with(SOCKS_UDP_ECHO_PREFIX),
        loopback_only: target.ip().is_loopback() && response_peer.ip().is_loopback(),
    })
}

fn parse_socks_udp_fragment(packet: &[u8]) -> Result<SocksUdpFragment> {
    if packet.len() < 10 {
        return Err(anyhow!("SOCKS UDP fragment is truncated"));
    }
    if packet[0] != 0 || packet[1] != 0 {
        return Err(anyhow!("SOCKS UDP RSV bytes must be zero"));
    }
    if packet[2] == 0 {
        return Err(anyhow!("SOCKS UDP fragment path requires non-zero FRAG"));
    }
    if packet[3] != 0x01 {
        return Err(anyhow!("bounded SOCKS UDP fragment path only supports IPv4 ATYP"));
    }
    let ip = Ipv4Addr::new(packet[4], packet[5], packet[6], packet[7]);
    let port = u16::from_be_bytes([packet[8], packet[9]]);
    Ok(SocksUdpFragment {
        frag: packet[2],
        target: SocketAddr::new(IpAddr::V4(ip), port),
        payload: packet[10..].to_vec(),
    })
}

fn reassemble_ordered_fragments(fragments: &[SocksUdpFragment]) -> Result<Vec<u8>> {
    if fragments.len() != 2 {
        return Err(anyhow!("bounded SOCKS UDP fragment path requires two fragments"));
    }
    let target = fragments[0].target;
    let mut payload = Vec::new();
    for (offset, fragment) in fragments.iter().enumerate() {
        if fragment.target != target {
            return Err(anyhow!("SOCKS UDP fragments target different destinations"));
        }
        ensure_loopback_target(fragment.target)?;
        let expected_index = u8::try_from(offset + 1)?;
        if fragment.index() != expected_index {
            return Err(anyhow!("SOCKS UDP fragments arrived out of bounded order"));
        }
        if fragment.is_final() != (offset + 1 == fragments.len()) {
            return Err(anyhow!("SOCKS UDP final-fragment marker is invalid"));
        }
        payload.extend_from_slice(&fragment.payload);
    }
    Ok(payload)
}

fn encode_socks_udp_fragment(target: SocketAddr, frag: u8, payload: &[u8]) -> Vec<u8> {
    let mut packet = vec![0, 0, frag];
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

async fn write_rollback_checkpoint(rollback_path: &std::path::Path) -> Result<RustSocksUdpFragmentsRollbackEvidence> {
    let created_at_epoch_seconds = rust_socks_udp_fragments_epoch_seconds();
    let checkpoint = RustSocksUdpFragmentsRollbackCheckpoint {
        component: RUST_SOCKS_UDP_FRAGMENTS_COMPONENT.into(),
        rust_owned_scope: "bounded SOCKS5 UDP two-fragment loopback reassembly".into(),
        fallback_retained_for: retained_socks_udp_fragments_fallback_scope(),
        created_at_epoch_seconds,
    };
    if let Some(parent) = rollback_path.parent() {
        fs::create_dir_all(parent).await?;
    }
    fs::write(rollback_path, serde_yaml_ng::to_string(&checkpoint)?.as_bytes()).await?;

    Ok(RustSocksUdpFragmentsRollbackEvidence {
        checkpoint_path: rollback_path.to_string_lossy().to_string().into(),
        fallback_retained_for: checkpoint.fallback_retained_for,
        created_at_epoch_seconds,
    })
}

fn retained_socks_udp_fragments_fallback_scope() -> Vec<String> {
    vec![
        "SOCKS UDP non-loopback forwarding".into(),
        "SOCKS UDP multi-destination fragment queues, cache eviction, and timeout windows".into(),
        "Shadowsocks UDP/plugin transports".into(),
        "VMess, VLESS, and Trojan encrypted sessions".into(),
        "system-wide packet capture and transparent proxy defaults".into(),
    ]
}

fn rust_socks_udp_fragments_facts() -> Vec<String> {
    vec![
        "Rust parses two SOCKS5 UDP fragments with RFC1928 FRAG sequencing".into(),
        "Rust reassembles only a bounded IPv4 loopback target before forwarding".into(),
        "Rust forwards the reassembled payload to one loopback UDP target and records byte evidence".into(),
        "Mihomo fallback remains retained for non-loopback UDP, fragment queues/timeouts, plugin transports, and packet capture".into(),
    ]
}

fn rust_socks_udp_fragments_dir() -> Result<std::path::PathBuf> {
    Ok(dirs::app_runtime_dir()?.join(RUST_SOCKS_UDP_FRAGMENTS_COMPONENT))
}

fn rust_socks_udp_fragments_evidence_path() -> Result<std::path::PathBuf> {
    Ok(rust_socks_udp_fragments_dir()?.join(RUST_SOCKS_UDP_FRAGMENTS_EVIDENCE_FILE))
}

fn rust_socks_udp_fragments_rollback_path() -> Result<std::path::PathBuf> {
    Ok(rust_socks_udp_fragments_dir()?.join(RUST_SOCKS_UDP_FRAGMENTS_ROLLBACK_FILE))
}

fn rust_socks_udp_fragments_epoch_seconds() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reassembles_two_loopback_fragments() {
        let target = SocketAddr::from((Ipv4Addr::LOCALHOST, 2082));
        let fragments = [
            parse_socks_udp_fragment(&encode_socks_udp_fragment(target, SOCKS_UDP_FRAGMENT_ONE, b"one-")).unwrap(),
            parse_socks_udp_fragment(&encode_socks_udp_fragment(target, SOCKS_UDP_FRAGMENT_FINAL_TWO, b"two")).unwrap(),
        ];

        assert_eq!(reassemble_ordered_fragments(&fragments).unwrap(), b"one-two");
    }

    #[test]
    fn rejects_standalone_datagram_in_fragment_path() {
        let target = SocketAddr::from((Ipv4Addr::LOCALHOST, 2082));
        let packet = encode_socks_udp_fragment(target, 0, b"standalone");

        assert!(parse_socks_udp_fragment(&packet).is_err());
    }
}
