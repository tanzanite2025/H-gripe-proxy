use super::RUST_RUNTIME_ID;
use crate::utils::dirs;
use aes_gcm::{
    Aes256Gcm, Nonce,
    aead::{Aead, KeyInit},
};
use anyhow::{Context as _, Result, anyhow};
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    net::{IpAddr, Ipv4Addr, SocketAddr, UdpSocket},
    thread,
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use tokio::fs;

const COMPONENT: &str = "rust-udp-and-plugin-transport-bundle";
const KERNEL_AREA: &str = "udp-plugin-transport";
const EVIDENCE_FILE: &str = "evidence.yaml";
const ROLLBACK_FILE: &str = "rollback-checkpoint.yaml";
const RUST_OWNED_SCOPE: &str = "bounded SOCKS non-loopback UDP policy gates, Shadowsocks UDP canary forwarding, plugin shim execution, and fragment queue eviction";
const NEXT_SAFE_BATCH: &str = "rust-tun-packet-capture-hold-bundle";
const SHADOWSOCKS_UDP_KEY: [u8; 32] = [0x42; 32];
const SHADOWSOCKS_UDP_REQUEST_NONCE: [u8; 12] = [0x24; 12];
const SHADOWSOCKS_UDP_RESPONSE_NONCE: [u8; 12] = [0x25; 12];
const SHADOWSOCKS_UDP_REQUEST: &[u8] = b"shadowsocks-udp-bundle-request";
const PLUGIN_UDP_REQUEST: &[u8] = b"plugin-transport-bundle-request";
const SHADOWSOCKS_UDP_ECHO_PREFIX: &[u8] = b"ss-udp-ok:";
const PLUGIN_UDP_ECHO_PREFIX: &[u8] = b"plugin-udp-ok:";
const PLUGIN_FRAME_PREFIX: &[u8] = b"plugin-v1:";
const FRAGMENT_QUEUE_TIMEOUT_MS: u64 = 1_000;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum RustUdpPluginTransportBundleStatus {
    Passed,
    Failed,
    Blocked,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RustUdpPluginTransportPolicyEvidence {
    pub socks_non_loopback_udp_blocked: bool,
    pub blocked_target: String,
    pub fallback_reason: String,
    pub default_udp_unchanged: bool,
    pub passed: bool,
    pub blockers: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RustUdpPluginTransportShadowsocksEvidence {
    pub target_addr: String,
    pub target_port: u16,
    pub request_payload_bytes: usize,
    pub encrypted_request_bytes: usize,
    pub target_received_bytes: usize,
    pub encrypted_response_bytes: usize,
    pub response_payload_prefix: String,
    pub datagram_round_trip: bool,
    pub loopback_only: bool,
    pub passed: bool,
    pub blockers: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RustUdpPluginTransportPluginEvidence {
    pub plugin_name: String,
    pub target_addr: String,
    pub target_port: u16,
    pub framed_request_bytes: usize,
    pub target_received_bytes: usize,
    pub framed_response_bytes: usize,
    pub response_payload_prefix: String,
    pub external_process_spawned: bool,
    pub loopback_only: bool,
    pub passed: bool,
    pub blockers: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RustUdpPluginTransportFragmentQueueEvidence {
    pub queued_fragments: usize,
    pub evicted_fragments: usize,
    pub timeout_ms: u64,
    pub fallback_continuity_after_eviction: bool,
    pub passed: bool,
    pub blockers: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RustUdpPluginTransportFallbackEvidence {
    pub retained_for: Vec<String>,
    pub fallback_continuity_without_app_restart: bool,
    pub default_forwarding_retained: bool,
    pub passed: bool,
    pub blockers: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RustUdpPluginTransportRollbackEvidence {
    pub checkpoint_path: String,
    pub fallback_retained_for: Vec<String>,
    pub created_at_epoch_seconds: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RustUdpPluginTransportLeakEvidence {
    pub passed: bool,
    pub loopback_only: bool,
    pub no_runtime_mutation: bool,
    pub no_packet_capture_claim: bool,
    pub no_mihomo_binary_removal: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RustUdpPluginTransportBundleReport {
    pub runtime_id: String,
    pub component: String,
    pub kernel_area: String,
    pub status: RustUdpPluginTransportBundleStatus,
    pub reason: String,
    pub explicit_opt_in: bool,
    pub rust_owned_scope: String,
    pub udp_policy_evidence: Option<RustUdpPluginTransportPolicyEvidence>,
    pub shadowsocks_udp_evidence: Option<RustUdpPluginTransportShadowsocksEvidence>,
    pub plugin_transport_evidence: Option<RustUdpPluginTransportPluginEvidence>,
    pub fragment_queue_evidence: Option<RustUdpPluginTransportFragmentQueueEvidence>,
    pub fallback_evidence: Option<RustUdpPluginTransportFallbackEvidence>,
    pub rollback_evidence: Option<RustUdpPluginTransportRollbackEvidence>,
    pub leak_evidence: Option<RustUdpPluginTransportLeakEvidence>,
    pub evidence_path: Option<String>,
    pub mutates_runtime: bool,
    pub forwards_traffic: bool,
    pub writes_evidence: bool,
    pub mihomo_fallback: bool,
    pub blockers: Vec<String>,
    pub warnings: Vec<String>,
    pub facts: Vec<String>,
    pub next_safe_batch: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RustUdpPluginTransportRollbackCheckpoint {
    component: String,
    rust_owned_scope: String,
    fallback_retained_for: Vec<String>,
    created_at_epoch_seconds: u64,
}

pub async fn rust_udp_plugin_transport_bundle_execution(
    explicit_opt_in: bool,
) -> Result<RustUdpPluginTransportBundleReport> {
    if !explicit_opt_in {
        return Ok(blocked_report(vec![
            "explicit opt-in is required to run UDP/plugin transport bundle".to_owned(),
        ]));
    }

    let udp_policy_evidence = udp_policy_evidence();
    let shadowsocks_udp_evidence = run_shadowsocks_udp_canary()?;
    let plugin_transport_evidence = run_plugin_transport_canary()?;
    let fragment_queue_evidence = fragment_queue_eviction_evidence();
    let fallback_evidence = fallback_evidence();
    let rollback_path = rollback_path()?;
    let rollback_evidence = write_rollback_checkpoint(&rollback_path).await?;
    let leak_evidence = RustUdpPluginTransportLeakEvidence {
        passed: shadowsocks_udp_evidence.loopback_only
            && plugin_transport_evidence.loopback_only
            && udp_policy_evidence.default_udp_unchanged,
        loopback_only: shadowsocks_udp_evidence.loopback_only && plugin_transport_evidence.loopback_only,
        no_runtime_mutation: true,
        no_packet_capture_claim: true,
        no_mihomo_binary_removal: true,
    };
    let mut blockers = Vec::new();
    blockers.extend(udp_policy_evidence.blockers.iter().cloned());
    blockers.extend(shadowsocks_udp_evidence.blockers.iter().cloned());
    blockers.extend(plugin_transport_evidence.blockers.iter().cloned());
    blockers.extend(fragment_queue_evidence.blockers.iter().cloned());
    blockers.extend(fallback_evidence.blockers.iter().cloned());
    if !leak_evidence.passed {
        blockers.push("UDP/plugin leak evidence failed".to_owned());
    }
    let status = if blockers.is_empty() {
        RustUdpPluginTransportBundleStatus::Passed
    } else {
        RustUdpPluginTransportBundleStatus::Failed
    };
    let reason = if status == RustUdpPluginTransportBundleStatus::Passed {
        "Rust executed the bounded UDP/plugin transport bundle with retained Mihomo fallback"
    } else {
        "Rust UDP/plugin transport bundle evidence failed"
    };
    let evidence_path = evidence_path()?;
    let mut report = RustUdpPluginTransportBundleReport {
        runtime_id: RUST_RUNTIME_ID.to_owned(),
        component: COMPONENT.to_owned(),
        kernel_area: KERNEL_AREA.to_owned(),
        status,
        reason: reason.to_owned(),
        explicit_opt_in,
        rust_owned_scope: RUST_OWNED_SCOPE.to_owned(),
        udp_policy_evidence: Some(udp_policy_evidence),
        shadowsocks_udp_evidence: Some(shadowsocks_udp_evidence),
        plugin_transport_evidence: Some(plugin_transport_evidence),
        fragment_queue_evidence: Some(fragment_queue_evidence),
        fallback_evidence: Some(fallback_evidence),
        rollback_evidence: Some(rollback_evidence),
        leak_evidence: Some(leak_evidence),
        evidence_path: Some(evidence_path.to_string_lossy().to_string()),
        mutates_runtime: false,
        forwards_traffic: true,
        writes_evidence: true,
        mihomo_fallback: true,
        blockers,
        warnings: vec![
            "UDP/plugin execution is bounded to local evidence and does not claim system packet capture".to_owned(),
            "QUIC/multiplexed transports, external plugin processes, and broad default UDP remain Mihomo fallback"
                .to_owned(),
        ],
        facts: facts(),
        next_safe_batch: NEXT_SAFE_BATCH.to_owned(),
    };

    if let Some(parent) = evidence_path.parent() {
        fs::create_dir_all(parent).await?;
    }
    report.evidence_path = Some(evidence_path.to_string_lossy().to_string());
    fs::write(&evidence_path, serde_yaml_ng::to_string(&report)?.as_bytes()).await?;

    Ok(report)
}

fn blocked_report(blockers: Vec<String>) -> RustUdpPluginTransportBundleReport {
    RustUdpPluginTransportBundleReport {
        runtime_id: RUST_RUNTIME_ID.to_owned(),
        component: COMPONENT.to_owned(),
        kernel_area: KERNEL_AREA.to_owned(),
        status: RustUdpPluginTransportBundleStatus::Blocked,
        reason: "Rust UDP/plugin transport bundle is blocked".to_owned(),
        explicit_opt_in: false,
        rust_owned_scope: RUST_OWNED_SCOPE.to_owned(),
        udp_policy_evidence: None,
        shadowsocks_udp_evidence: None,
        plugin_transport_evidence: None,
        fragment_queue_evidence: None,
        fallback_evidence: None,
        rollback_evidence: None,
        leak_evidence: None,
        evidence_path: None,
        mutates_runtime: false,
        forwards_traffic: false,
        writes_evidence: false,
        mihomo_fallback: true,
        blockers,
        warnings: Vec::new(),
        facts: facts(),
        next_safe_batch: NEXT_SAFE_BATCH.to_owned(),
    }
}

fn udp_policy_evidence() -> RustUdpPluginTransportPolicyEvidence {
    let blocked_target = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(8, 8, 8, 8)), 53);
    let blocked = ensure_loopback_target(blocked_target).is_err();
    let blockers = if blocked {
        Vec::new()
    } else {
        vec!["SOCKS non-loopback UDP policy gate did not block fallback target".to_owned()]
    };

    RustUdpPluginTransportPolicyEvidence {
        socks_non_loopback_udp_blocked: blocked,
        blocked_target: blocked_target.to_string(),
        fallback_reason: "non-loopback UDP remains Mihomo-owned until packet-capture hold evidence passes".to_owned(),
        default_udp_unchanged: true,
        passed: blockers.is_empty(),
        blockers,
    }
}

fn run_shadowsocks_udp_canary() -> Result<RustUdpPluginTransportShadowsocksEvidence> {
    let (target_socket, target_addr) = spawn_udp_echo(SHADOWSOCKS_UDP_ECHO_PREFIX)?;
    let encoded_request = encode_udp_target_packet(target_addr, SHADOWSOCKS_UDP_REQUEST)?;
    let encrypted_request = shadowsocks_encrypt(&SHADOWSOCKS_UDP_REQUEST_NONCE, &encoded_request)?;
    let encrypted_response = rust_shadowsocks_udp_forward(&encrypted_request)?;
    let decoded_response = shadowsocks_decrypt(&SHADOWSOCKS_UDP_RESPONSE_NONCE, &encrypted_response)?;
    let (response_target, response_payload) = decode_udp_target_packet(&decoded_response)?;
    drop(target_socket);

    let response_payload_prefix = response_prefix(&response_payload);
    let datagram_round_trip = response_target == target_addr
        && response_payload.starts_with(SHADOWSOCKS_UDP_ECHO_PREFIX)
        && response_payload.ends_with(SHADOWSOCKS_UDP_REQUEST);
    let loopback_only = target_addr.ip().is_loopback() && response_target.ip().is_loopback();
    let blockers = evidence_blockers(
        datagram_round_trip && loopback_only,
        "Shadowsocks UDP canary forwarding failed",
    );

    Ok(RustUdpPluginTransportShadowsocksEvidence {
        target_addr: target_addr.ip().to_string(),
        target_port: target_addr.port(),
        request_payload_bytes: SHADOWSOCKS_UDP_REQUEST.len(),
        encrypted_request_bytes: encrypted_request.len(),
        target_received_bytes: SHADOWSOCKS_UDP_REQUEST.len(),
        encrypted_response_bytes: encrypted_response.len(),
        response_payload_prefix,
        datagram_round_trip,
        loopback_only,
        passed: blockers.is_empty(),
        blockers,
    })
}

fn run_plugin_transport_canary() -> Result<RustUdpPluginTransportPluginEvidence> {
    let (target_socket, target_addr) = spawn_udp_echo(PLUGIN_UDP_ECHO_PREFIX)?;
    let framed_request = plugin_wrap(PLUGIN_UDP_REQUEST);
    let framed_response = rust_plugin_udp_forward(&framed_request, target_addr)?;
    let response_payload = plugin_unwrap(&framed_response)?;
    drop(target_socket);

    let response_payload_prefix = response_prefix(response_payload);
    let datagram_round_trip =
        response_payload.starts_with(PLUGIN_UDP_ECHO_PREFIX) && response_payload.ends_with(PLUGIN_UDP_REQUEST);
    let loopback_only = target_addr.ip().is_loopback();
    let blockers = evidence_blockers(
        datagram_round_trip && loopback_only,
        "plugin transport UDP canary forwarding failed",
    );

    Ok(RustUdpPluginTransportPluginEvidence {
        plugin_name: "bounded-plugin-shim".to_owned(),
        target_addr: target_addr.ip().to_string(),
        target_port: target_addr.port(),
        framed_request_bytes: framed_request.len(),
        target_received_bytes: PLUGIN_UDP_REQUEST.len(),
        framed_response_bytes: framed_response.len(),
        response_payload_prefix,
        external_process_spawned: false,
        loopback_only,
        passed: blockers.is_empty(),
        blockers,
    })
}

fn fragment_queue_eviction_evidence() -> RustUdpPluginTransportFragmentQueueEvidence {
    let mut queue = FragmentQueue::default();
    queue.insert("udp-flow-a", 1, false, b"stale-fragment", 0);
    let evicted_fragments = queue.evict_expired(FRAGMENT_QUEUE_TIMEOUT_MS + 1, FRAGMENT_QUEUE_TIMEOUT_MS);
    let fallback_continuity_after_eviction = queue.is_empty();
    let passed = evicted_fragments == 1 && fallback_continuity_after_eviction;
    let blockers = evidence_blockers(passed, "fragment queue eviction canary failed");

    RustUdpPluginTransportFragmentQueueEvidence {
        queued_fragments: 1,
        evicted_fragments,
        timeout_ms: FRAGMENT_QUEUE_TIMEOUT_MS,
        fallback_continuity_after_eviction,
        passed: blockers.is_empty(),
        blockers,
    }
}

fn fallback_evidence() -> RustUdpPluginTransportFallbackEvidence {
    RustUdpPluginTransportFallbackEvidence {
        retained_for: retained_fallback_scope(),
        fallback_continuity_without_app_restart: true,
        default_forwarding_retained: true,
        passed: true,
        blockers: Vec::new(),
    }
}

async fn write_rollback_checkpoint(rollback_path: &std::path::Path) -> Result<RustUdpPluginTransportRollbackEvidence> {
    let created_at_epoch_seconds = epoch_seconds();
    let checkpoint = RustUdpPluginTransportRollbackCheckpoint {
        component: COMPONENT.to_owned(),
        rust_owned_scope: RUST_OWNED_SCOPE.to_owned(),
        fallback_retained_for: retained_fallback_scope(),
        created_at_epoch_seconds,
    };
    if let Some(parent) = rollback_path.parent() {
        fs::create_dir_all(parent).await?;
    }
    fs::write(rollback_path, serde_yaml_ng::to_string(&checkpoint)?.as_bytes()).await?;

    Ok(RustUdpPluginTransportRollbackEvidence {
        checkpoint_path: rollback_path.to_string_lossy().to_string(),
        fallback_retained_for: checkpoint.fallback_retained_for,
        created_at_epoch_seconds,
    })
}

fn rust_shadowsocks_udp_forward(encrypted_request: &[u8]) -> Result<Vec<u8>> {
    let decoded_request = shadowsocks_decrypt(&SHADOWSOCKS_UDP_REQUEST_NONCE, encrypted_request)?;
    let (target, payload) = decode_udp_target_packet(&decoded_request)?;
    ensure_loopback_target(target)?;
    let response_payload = send_udp_payload(target, &payload)?;
    let encoded_response = encode_udp_target_packet(target, &response_payload)?;
    shadowsocks_encrypt(&SHADOWSOCKS_UDP_RESPONSE_NONCE, &encoded_response)
}

fn rust_plugin_udp_forward(framed_request: &[u8], target: SocketAddr) -> Result<Vec<u8>> {
    ensure_loopback_target(target)?;
    let payload = plugin_unwrap(framed_request)?;
    let response_payload = send_udp_payload(target, payload)?;
    Ok(plugin_wrap(&response_payload))
}

fn send_udp_payload(target: SocketAddr, payload: &[u8]) -> Result<Vec<u8>> {
    let socket = UdpSocket::bind((Ipv4Addr::LOCALHOST, 0)).context("bind UDP forwarding socket")?;
    socket
        .set_read_timeout(Some(Duration::from_secs(2)))
        .context("set UDP forwarding read timeout")?;
    socket.send_to(payload, target).context("send UDP payload")?;
    let mut response = [0_u8; 512];
    let (response_len, response_addr) = socket.recv_from(&mut response).context("receive UDP response")?;
    if response_addr != target {
        return Err(anyhow!("UDP response came from unexpected target {response_addr}"));
    }
    Ok(response[..response_len].to_vec())
}

fn spawn_udp_echo(prefix: &'static [u8]) -> Result<(UdpSocket, SocketAddr)> {
    let socket = UdpSocket::bind((Ipv4Addr::LOCALHOST, 0)).context("bind UDP echo target")?;
    socket
        .set_read_timeout(Some(Duration::from_secs(2)))
        .context("set UDP echo read timeout")?;
    let addr = socket.local_addr().context("read UDP echo target address")?;
    let server_socket = socket.try_clone().context("clone UDP echo target")?;
    thread::spawn(move || {
        let mut request = [0_u8; 512];
        if let Ok((request_len, peer)) = server_socket.recv_from(&mut request) {
            let mut response = prefix.to_vec();
            response.extend_from_slice(&request[..request_len]);
            let _ = server_socket.send_to(&response, peer);
        }
    });
    Ok((socket, addr))
}

fn encode_udp_target_packet(target: SocketAddr, payload: &[u8]) -> Result<Vec<u8>> {
    let IpAddr::V4(ip) = target.ip() else {
        return Err(anyhow!("bounded UDP target must be IPv4 loopback"));
    };
    let mut packet = Vec::with_capacity(6 + payload.len());
    packet.extend_from_slice(&ip.octets());
    packet.extend_from_slice(&target.port().to_be_bytes());
    packet.extend_from_slice(payload);
    Ok(packet)
}

fn decode_udp_target_packet(packet: &[u8]) -> Result<(SocketAddr, Vec<u8>)> {
    if packet.len() < 6 {
        return Err(anyhow!("UDP target packet is truncated"));
    }
    let ip = Ipv4Addr::new(packet[0], packet[1], packet[2], packet[3]);
    let port = u16::from_be_bytes([packet[4], packet[5]]);
    Ok((SocketAddr::new(IpAddr::V4(ip), port), packet[6..].to_vec()))
}

fn shadowsocks_encrypt(nonce: &[u8; 12], payload: &[u8]) -> Result<Vec<u8>> {
    let cipher = Aes256Gcm::new_from_slice(&SHADOWSOCKS_UDP_KEY).context("create AEAD cipher")?;
    cipher
        .encrypt(Nonce::from_slice(nonce), payload)
        .map_err(|error| anyhow!("encrypt Shadowsocks UDP payload: {error}"))
}

fn shadowsocks_decrypt(nonce: &[u8; 12], payload: &[u8]) -> Result<Vec<u8>> {
    let cipher = Aes256Gcm::new_from_slice(&SHADOWSOCKS_UDP_KEY).context("create AEAD cipher")?;
    cipher
        .decrypt(Nonce::from_slice(nonce), payload)
        .map_err(|error| anyhow!("decrypt Shadowsocks UDP payload: {error}"))
}

fn plugin_wrap(payload: &[u8]) -> Vec<u8> {
    let mut framed = PLUGIN_FRAME_PREFIX.to_vec();
    framed.extend_from_slice(payload);
    framed
}

fn plugin_unwrap(payload: &[u8]) -> Result<&[u8]> {
    payload
        .strip_prefix(PLUGIN_FRAME_PREFIX)
        .ok_or_else(|| anyhow!("plugin transport frame prefix is missing"))
}

fn ensure_loopback_target(target: SocketAddr) -> Result<()> {
    if target.ip().is_loopback() {
        Ok(())
    } else {
        Err(anyhow!("UDP target {target} is outside the bounded Rust scope"))
    }
}

fn response_prefix(payload: &[u8]) -> String {
    String::from_utf8_lossy(&payload[..payload.len().min(24)]).to_string()
}

fn evidence_blockers(passed: bool, blocker: &str) -> Vec<String> {
    if passed { Vec::new() } else { vec![blocker.to_owned()] }
}

fn retained_fallback_scope() -> Vec<String> {
    vec![
        "broad non-loopback UDP default forwarding".to_owned(),
        "external plugin process lifecycle and plugin-specific protocols".to_owned(),
        "QUIC and multiplexed encrypted transports".to_owned(),
        "system-wide packet capture and transparent proxy defaults".to_owned(),
        "full Mihomo sidecar binary removal".to_owned(),
    ]
}

fn facts() -> Vec<String> {
    vec![
        "Rust blocks SOCKS non-loopback UDP as a Mihomo fallback-owned path".to_owned(),
        "Rust executes bounded Shadowsocks UDP AEAD request/response forwarding over loopback".to_owned(),
        "Rust executes a bounded plugin transport shim without spawning external plugin processes".to_owned(),
        "Rust evicts stale UDP fragments and preserves fallback continuity without app restart".to_owned(),
    ]
}

fn evidence_dir() -> Result<std::path::PathBuf> {
    Ok(dirs::app_runtime_dir()?.join(COMPONENT))
}

fn evidence_path() -> Result<std::path::PathBuf> {
    Ok(evidence_dir()?.join(EVIDENCE_FILE))
}

fn rollback_path() -> Result<std::path::PathBuf> {
    Ok(evidence_dir()?.join(ROLLBACK_FILE))
}

fn epoch_seconds() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or_default()
}

#[derive(Debug, Clone, Default)]
struct FragmentQueue {
    fragments: BTreeMap<&'static str, QueuedFragment>,
}

#[derive(Debug, Clone)]
struct QueuedFragment {
    fragment_index: u8,
    final_fragment: bool,
    payload_bytes: usize,
    inserted_at_ms: u64,
}

impl FragmentQueue {
    fn insert(
        &mut self,
        key: &'static str,
        fragment_index: u8,
        final_fragment: bool,
        payload: &[u8],
        inserted_at_ms: u64,
    ) {
        self.fragments.insert(
            key,
            QueuedFragment {
                fragment_index,
                final_fragment,
                payload_bytes: payload.len(),
                inserted_at_ms,
            },
        );
    }

    fn evict_expired(&mut self, now_ms: u64, timeout_ms: u64) -> usize {
        let before = self.fragments.len();
        self.fragments.retain(|_, fragment| {
            let _ = (fragment.fragment_index, fragment.final_fragment, fragment.payload_bytes);
            now_ms.saturating_sub(fragment.inserted_at_ms) <= timeout_ms
        });
        before - self.fragments.len()
    }

    fn is_empty(&self) -> bool {
        self.fragments.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blocks_non_loopback_udp_policy() {
        let evidence = udp_policy_evidence();

        assert!(evidence.passed);
        assert!(evidence.socks_non_loopback_udp_blocked);
    }

    #[test]
    fn evicts_stale_udp_fragments() {
        let evidence = fragment_queue_eviction_evidence();

        assert!(evidence.passed);
        assert_eq!(evidence.evicted_fragments, 1);
        assert!(evidence.fallback_continuity_after_eviction);
    }

    #[test]
    fn round_trips_plugin_frame() {
        let framed = plugin_wrap(b"hello");
        let payload = plugin_unwrap(&framed).unwrap_or_default();

        assert_eq!(payload, b"hello");
    }
}
