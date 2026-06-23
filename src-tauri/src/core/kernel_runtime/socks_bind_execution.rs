use super::{
    RUST_RUNTIME_ID, RustDefaultDataPlaneCloseoutGateEvidence, RustSocksBindExecutionReport,
    RustSocksBindExecutionStatus, RustSocksBindForwardEvidence, RustSocksBindLeakEvidence,
    RustSocksBindRollbackEvidence, rust_default_data_plane_closeout_gate_evidence,
};
use crate::utils::dirs;
use anyhow::{Context as _, Result, anyhow};
use serde::{Deserialize, Serialize};
use smartstring::alias::String;
use std::{
    io::{Read, Write},
    net::{Ipv4Addr, SocketAddr, TcpListener, TcpStream},
    thread,
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use tokio::fs;

const RUST_SOCKS_BIND_COMPONENT: &str = "rust-socks-bind-execution";
const RUST_SOCKS_BIND_KERNEL_AREA: &str = "socks-bind";
const RUST_SOCKS_BIND_EVIDENCE_FILE: &str = "evidence.yaml";
const RUST_SOCKS_BIND_ROLLBACK_FILE: &str = "rollback-checkpoint.yaml";
const NEXT_SAFE_BATCH: &str = "route-packet-capture-privileged-hold";
const SOCKS_VERSION: u8 = 0x05;
const SOCKS_USERPASS_METHOD: u8 = 0x02;
const SOCKS_AUTH_VERSION: u8 = 0x01;
const SOCKS_SUCCESS: u8 = 0x00;
const SOCKS_CMD_BIND: u8 = 0x02;
const SOCKS_ATYP_IPV4: u8 = 0x01;
const TEST_USERNAME: &[u8] = b"rust-user";
const TEST_PASSWORD: &[u8] = b"rust-pass";
const TEST_REQUEST: &[u8] = b"GET /socks-bind HTTP/1.1\r\nHost: loopback.test\r\n\r\n";
const TEST_RESPONSE: &[u8] = b"HTTP/1.1 200 OK\r\nContent-Length: 16\r\n\r\nsocks-bind-ok:42";

pub async fn rust_socks_bind_execution(explicit_opt_in: bool) -> Result<RustSocksBindExecutionReport> {
    let default_data_plane_closeout_gate = rust_default_data_plane_closeout_gate_evidence().await?;

    if !explicit_opt_in {
        let mut blockers = vec!["SOCKS BIND execution requires explicit opt-in".into()];
        blockers.extend(default_data_plane_closeout_gate.blockers.clone());
        return Ok(blocked_report(
            explicit_opt_in,
            default_data_plane_closeout_gate,
            blockers,
        ));
    }
    if !default_data_plane_closeout_gate.blockers.is_empty() {
        return Ok(blocked_report(
            explicit_opt_in,
            default_data_plane_closeout_gate.clone(),
            default_data_plane_closeout_gate.blockers.clone(),
        ));
    }

    let rollback_path = rust_socks_bind_rollback_path()?;
    let rollback_evidence = write_rollback_checkpoint(&rollback_path).await?;
    let forward_evidence = match run_bounded_socks_bind_forwarding() {
        Ok(evidence) => evidence,
        Err(error) => {
            return Ok(blocked_report(
                explicit_opt_in,
                default_data_plane_closeout_gate,
                vec![format!("bounded SOCKS BIND execution failed: {error}").into()],
            ));
        }
    };
    let leak_evidence = RustSocksBindLeakEvidence {
        passed: forward_evidence.loopback_only && forward_evidence.data_forwarded,
        no_system_packet_capture: true,
        no_non_loopback_peer: forward_evidence.loopback_only,
        no_mihomo_binary_removal: true,
    };
    let evidence_path = rust_socks_bind_evidence_path()?;
    let mut report = RustSocksBindExecutionReport {
        runtime_id: RUST_RUNTIME_ID.into(),
        component: RUST_SOCKS_BIND_COMPONENT.into(),
        kernel_area: RUST_SOCKS_BIND_KERNEL_AREA.into(),
        status: RustSocksBindExecutionStatus::Executed,
        reason: "Rust executed bounded SOCKS5 BIND forwarding over loopback".into(),
        explicit_opt_in,
        rust_owned_scope: "SOCKS5 username/password BIND handshake and loopback peer request/response forwarding".into(),
        default_data_plane_closeout_gate,
        mutates_runtime: false,
        writes_evidence: true,
        evidence_path: Some(evidence_path.to_string_lossy().to_string().into()),
        forward_evidence: Some(forward_evidence),
        rollback_evidence: Some(rollback_evidence),
        leak_evidence: Some(leak_evidence),
        mihomo_fallback_retained_for: retained_socks_bind_fallback_scope(),
        blockers: Vec::new(),
        warnings: vec![
            "SOCKS non-loopback UDP, broad fragment queues/timeouts, Shadowsocks UDP/plugin transports, and packet capture remain Mihomo-owned".into(),
        ],
        facts: rust_socks_bind_facts(),
        next_safe_batch: NEXT_SAFE_BATCH.into(),
    };

    if let Some(parent) = evidence_path.parent() {
        fs::create_dir_all(parent).await?;
    }
    fs::write(&evidence_path, serde_yaml_ng::to_string(&report)?.as_bytes()).await?;
    report.evidence_path = Some(evidence_path.to_string_lossy().to_string().into());

    Ok(report)
}

fn blocked_report(
    explicit_opt_in: bool,
    default_data_plane_closeout_gate: RustDefaultDataPlaneCloseoutGateEvidence,
    blockers: Vec<String>,
) -> RustSocksBindExecutionReport {
    RustSocksBindExecutionReport {
        runtime_id: RUST_RUNTIME_ID.into(),
        component: RUST_SOCKS_BIND_COMPONENT.into(),
        kernel_area: RUST_SOCKS_BIND_KERNEL_AREA.into(),
        status: RustSocksBindExecutionStatus::Blocked,
        reason: "Rust SOCKS BIND execution is blocked".into(),
        explicit_opt_in,
        rust_owned_scope: "SOCKS5 username/password BIND handshake and loopback peer request/response forwarding"
            .into(),
        default_data_plane_closeout_gate,
        mutates_runtime: false,
        writes_evidence: false,
        evidence_path: None,
        forward_evidence: None,
        rollback_evidence: None,
        leak_evidence: None,
        mihomo_fallback_retained_for: retained_socks_bind_fallback_scope(),
        blockers,
        warnings: Vec::new(),
        facts: rust_socks_bind_facts(),
        next_safe_batch: NEXT_SAFE_BATCH.into(),
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RustSocksBindRollbackCheckpoint {
    component: String,
    rust_owned_scope: String,
    fallback_retained_for: Vec<String>,
    created_at_epoch_seconds: u64,
}

struct PeerEvidence {
    peer_addr: SocketAddr,
    peer_received_bytes: usize,
}

struct ProxyEvidence {
    proxy_addr: SocketAddr,
    bind_addr: SocketAddr,
    peer_addr: SocketAddr,
    selected_method: u8,
    auth_negotiated: bool,
    bind_command: u8,
    bind_atyp: u8,
    request_bytes: usize,
    response_bytes: usize,
    peer_received_bytes: usize,
}

fn run_bounded_socks_bind_forwarding() -> Result<RustSocksBindForwardEvidence> {
    let proxy_listener =
        TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).context("failed to bind SOCKS BIND loopback proxy")?;
    let proxy_addr = proxy_listener.local_addr()?;
    let proxy_handle = thread::spawn(move || run_socks_bind_proxy(proxy_listener));

    let (bind_addr, bound_peer_addr) = run_socks_bind_client(proxy_addr)?;
    let peer_handle = thread::spawn(move || run_loopback_bound_peer(bind_addr));
    let (response, second_reply_peer_addr) = read_second_bind_reply_and_exchange(bound_peer_addr)?;
    let peer_evidence = peer_handle
        .join()
        .map_err(|_| anyhow!("SOCKS BIND peer thread panicked"))??;
    let proxy_evidence = proxy_handle
        .join()
        .map_err(|_| anyhow!("SOCKS BIND proxy thread panicked"))??;
    let data_forwarded = response == TEST_RESPONSE
        && proxy_evidence.request_bytes == TEST_REQUEST.len()
        && proxy_evidence.peer_received_bytes == TEST_REQUEST.len()
        && peer_evidence.peer_received_bytes == TEST_REQUEST.len()
        && proxy_evidence.response_bytes == TEST_RESPONSE.len();

    Ok(RustSocksBindForwardEvidence {
        proxy_listener_addr: proxy_evidence.proxy_addr.to_string().into(),
        bind_addr: proxy_evidence.bind_addr.to_string().into(),
        peer_addr: proxy_evidence.peer_addr.to_string().into(),
        selected_method: format!("0x{:02x}", proxy_evidence.selected_method).into(),
        auth_negotiated: proxy_evidence.auth_negotiated,
        bind_command: format!("0x{:02x}", proxy_evidence.bind_command).into(),
        bind_atyp: format!("0x{:02x}", proxy_evidence.bind_atyp).into(),
        first_reply_sent: proxy_evidence.bind_addr == bind_addr,
        second_reply_sent: proxy_evidence.peer_addr == second_reply_peer_addr,
        request_bytes: TEST_REQUEST.len(),
        peer_received_bytes: peer_evidence.peer_received_bytes,
        response_bytes: response.len(),
        response_prefix: std::string::String::from_utf8_lossy(&response[..response.len().min(15)])
            .to_string()
            .into(),
        data_forwarded,
        loopback_only: proxy_evidence.proxy_addr.ip().is_loopback()
            && proxy_evidence.bind_addr.ip().is_loopback()
            && proxy_evidence.peer_addr.ip().is_loopback()
            && peer_evidence.peer_addr.ip().is_loopback(),
    })
}

fn run_socks_bind_proxy(listener: TcpListener) -> Result<ProxyEvidence> {
    let proxy_addr = listener.local_addr()?;
    let (mut client, peer_addr) = listener.accept()?;
    if !peer_addr.ip().is_loopback() {
        return Err(anyhow!("SOCKS BIND client peer was not loopback"));
    }
    client.set_read_timeout(Some(Duration::from_secs(2)))?;
    client.set_write_timeout(Some(Duration::from_secs(2)))?;
    let selected_method = negotiate_userpass_method(&mut client)?;
    authenticate_userpass(&mut client)?;
    let (bind_command, bind_atyp, requested_target) = read_bind_request(&mut client)?;
    if !requested_target.ip().is_loopback() {
        return Err(anyhow!("SOCKS BIND requested unsupported non-loopback target"));
    }

    let bind_listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0))?;
    let bind_addr = bind_listener.local_addr()?;
    client.write_all(&bind_success_response(bind_addr))?;
    let (mut bound_peer, bound_peer_addr) = bind_listener.accept()?;
    if !bound_peer_addr.ip().is_loopback() {
        return Err(anyhow!("SOCKS BIND bound peer was not loopback"));
    }
    bound_peer.set_read_timeout(Some(Duration::from_secs(2)))?;
    bound_peer.set_write_timeout(Some(Duration::from_secs(2)))?;
    client.write_all(&bind_success_response(bound_peer_addr))?;

    let request = read_until_http_headers(&mut client)?;
    bound_peer.write_all(&request)?;
    let response = read_http_response(&mut bound_peer)?;
    client.write_all(&response)?;

    Ok(ProxyEvidence {
        proxy_addr,
        bind_addr,
        peer_addr: bound_peer_addr,
        selected_method,
        auth_negotiated: true,
        bind_command,
        bind_atyp,
        request_bytes: request.len(),
        response_bytes: response.len(),
        peer_received_bytes: request.len(),
    })
}

fn run_loopback_bound_peer(bind_addr: SocketAddr) -> Result<PeerEvidence> {
    let mut stream = TcpStream::connect(bind_addr).context("failed to connect SOCKS BIND peer")?;
    let peer_addr = stream.local_addr()?;
    if !peer_addr.ip().is_loopback() {
        return Err(anyhow!("SOCKS BIND peer local address was not loopback"));
    }
    stream.set_read_timeout(Some(Duration::from_secs(2)))?;
    stream.set_write_timeout(Some(Duration::from_secs(2)))?;
    let request = read_until_http_headers(&mut stream)?;
    stream.write_all(TEST_RESPONSE)?;
    Ok(PeerEvidence {
        peer_addr,
        peer_received_bytes: request.len(),
    })
}

fn run_socks_bind_client(proxy_addr: SocketAddr) -> Result<(SocketAddr, TcpStream)> {
    let mut client = TcpStream::connect(proxy_addr).context("failed to connect SOCKS BIND client")?;
    client.set_read_timeout(Some(Duration::from_secs(2)))?;
    client.set_write_timeout(Some(Duration::from_secs(2)))?;
    client.write_all(&[SOCKS_VERSION, 0x01, SOCKS_USERPASS_METHOD])?;
    let mut method_response = [0_u8; 2];
    client.read_exact(&mut method_response)?;
    if method_response != [SOCKS_VERSION, SOCKS_USERPASS_METHOD] {
        return Err(anyhow!("SOCKS BIND method negotiation failed"));
    }
    client.write_all(&build_auth_request(TEST_USERNAME, TEST_PASSWORD)?)?;
    let mut auth_response = [0_u8; 2];
    client.read_exact(&mut auth_response)?;
    if auth_response != [SOCKS_AUTH_VERSION, SOCKS_SUCCESS] {
        return Err(anyhow!("SOCKS BIND authentication failed"));
    }
    client.write_all(&build_bind_request(SocketAddr::from((Ipv4Addr::LOCALHOST, 0)))?)?;
    let bind_addr = read_socks_reply_addr(&mut client)?;
    Ok((bind_addr, client))
}

fn read_second_bind_reply_and_exchange(mut client: TcpStream) -> Result<(Vec<u8>, SocketAddr)> {
    let peer_addr = read_socks_reply_addr(&mut client)?;
    client.write_all(TEST_REQUEST)?;
    let response = read_http_response(&mut client)?;
    if response != TEST_RESPONSE {
        return Err(anyhow!("SOCKS BIND response mismatch"));
    }
    Ok((response, peer_addr))
}

fn negotiate_userpass_method(stream: &mut TcpStream) -> Result<u8> {
    let mut greeting_header = [0_u8; 2];
    stream.read_exact(&mut greeting_header)?;
    if greeting_header[0] != SOCKS_VERSION {
        return Err(anyhow!("SOCKS version mismatch"));
    }
    let method_count = usize::from(greeting_header[1]);
    let mut methods = vec![0_u8; method_count];
    stream.read_exact(&mut methods)?;
    if !methods.contains(&SOCKS_USERPASS_METHOD) {
        return Err(anyhow!("SOCKS user/pass method was not offered"));
    }
    stream.write_all(&[SOCKS_VERSION, SOCKS_USERPASS_METHOD])?;
    Ok(SOCKS_USERPASS_METHOD)
}

fn authenticate_userpass(stream: &mut TcpStream) -> Result<()> {
    let mut version = [0_u8; 1];
    stream.read_exact(&mut version)?;
    if version[0] != SOCKS_AUTH_VERSION {
        return Err(anyhow!("SOCKS auth version mismatch"));
    }
    let username = read_len_prefixed_field(stream)?;
    let password = read_len_prefixed_field(stream)?;
    if username == TEST_USERNAME && password == TEST_PASSWORD {
        stream.write_all(&[SOCKS_AUTH_VERSION, SOCKS_SUCCESS])?;
        Ok(())
    } else {
        stream.write_all(&[SOCKS_AUTH_VERSION, 0x01])?;
        Err(anyhow!("SOCKS auth credentials mismatch"))
    }
}

fn read_bind_request(stream: &mut TcpStream) -> Result<(u8, u8, SocketAddr)> {
    let mut header = [0_u8; 4];
    stream.read_exact(&mut header)?;
    if header[0] != SOCKS_VERSION || header[2] != 0x00 || header[3] != SOCKS_ATYP_IPV4 {
        return Err(anyhow!("unsupported bounded SOCKS BIND request"));
    }
    let mut addr = [0_u8; 4];
    let mut port = [0_u8; 2];
    stream.read_exact(&mut addr)?;
    stream.read_exact(&mut port)?;
    let target = SocketAddr::from((
        Ipv4Addr::new(addr[0], addr[1], addr[2], addr[3]),
        u16::from_be_bytes(port),
    ));
    if header[1] != SOCKS_CMD_BIND {
        return Err(anyhow!("SOCKS request was not BIND"));
    }
    Ok((header[1], header[3], target))
}

fn read_len_prefixed_field(stream: &mut TcpStream) -> Result<Vec<u8>> {
    let mut len = [0_u8; 1];
    stream.read_exact(&mut len)?;
    let mut field = vec![0_u8; usize::from(len[0])];
    stream.read_exact(&mut field)?;
    Ok(field)
}

fn build_auth_request(username: &[u8], password: &[u8]) -> Result<Vec<u8>> {
    if username.len() > u8::MAX as usize || password.len() > u8::MAX as usize {
        return Err(anyhow!("SOCKS auth fields exceed RFC1929 one-byte length"));
    }
    let mut request = Vec::with_capacity(3 + username.len() + password.len());
    request.push(SOCKS_AUTH_VERSION);
    request.push(username.len() as u8);
    request.extend_from_slice(username);
    request.push(password.len() as u8);
    request.extend_from_slice(password);
    Ok(request)
}

fn build_bind_request(target_addr: SocketAddr) -> Result<[u8; 10]> {
    let SocketAddr::V4(target) = target_addr else {
        return Err(anyhow!("bounded SOCKS BIND target must be IPv4"));
    };
    let port = target.port().to_be_bytes();
    let octets = target.ip().octets();
    Ok([
        SOCKS_VERSION,
        SOCKS_CMD_BIND,
        0x00,
        SOCKS_ATYP_IPV4,
        octets[0],
        octets[1],
        octets[2],
        octets[3],
        port[0],
        port[1],
    ])
}

fn bind_success_response(bound_addr: SocketAddr) -> [u8; 10] {
    let SocketAddr::V4(bound) = bound_addr else {
        return [SOCKS_VERSION, 0x08, 0x00, SOCKS_ATYP_IPV4, 0, 0, 0, 0, 0, 0];
    };
    let port = bound.port().to_be_bytes();
    let octets = bound.ip().octets();
    [
        SOCKS_VERSION,
        SOCKS_SUCCESS,
        0x00,
        SOCKS_ATYP_IPV4,
        octets[0],
        octets[1],
        octets[2],
        octets[3],
        port[0],
        port[1],
    ]
}

fn read_socks_reply_addr(stream: &mut TcpStream) -> Result<SocketAddr> {
    let mut reply = [0_u8; 10];
    stream.read_exact(&mut reply)?;
    if reply[0] != SOCKS_VERSION || reply[1] != SOCKS_SUCCESS || reply[3] != SOCKS_ATYP_IPV4 {
        return Err(anyhow!("SOCKS BIND reply failed"));
    }
    Ok(SocketAddr::from((
        Ipv4Addr::new(reply[4], reply[5], reply[6], reply[7]),
        u16::from_be_bytes([reply[8], reply[9]]),
    )))
}

fn read_until_http_headers(stream: &mut TcpStream) -> Result<Vec<u8>> {
    let mut buffer = Vec::new();
    let mut byte = [0_u8; 1];
    while buffer.len() < 4096 {
        stream.read_exact(&mut byte)?;
        buffer.push(byte[0]);
        if buffer.ends_with(b"\r\n\r\n") {
            return Ok(buffer);
        }
    }
    Err(anyhow!("HTTP header read exceeded bounded size"))
}

fn read_http_response(stream: &mut TcpStream) -> Result<Vec<u8>> {
    let headers = read_until_http_headers(stream)?;
    let content_length = parse_content_length(&headers)?;
    let mut response = headers;
    let mut body = vec![0_u8; content_length];
    stream.read_exact(&mut body)?;
    response.extend_from_slice(&body);
    Ok(response)
}

fn parse_content_length(headers: &[u8]) -> Result<usize> {
    let text = std::str::from_utf8(headers)?;
    text.lines()
        .find_map(|line| {
            line.strip_prefix("Content-Length:")
                .and_then(|value| value.trim().parse::<usize>().ok())
        })
        .ok_or_else(|| anyhow!("bounded HTTP response missing Content-Length"))
}

async fn write_rollback_checkpoint(rollback_path: &std::path::Path) -> Result<RustSocksBindRollbackEvidence> {
    let created_at_epoch_seconds = rust_socks_bind_epoch_seconds();
    let checkpoint = RustSocksBindRollbackCheckpoint {
        component: RUST_SOCKS_BIND_COMPONENT.into(),
        rust_owned_scope: "SOCKS5 username/password BIND handshake and loopback peer request/response forwarding"
            .into(),
        fallback_retained_for: retained_socks_bind_fallback_scope(),
        created_at_epoch_seconds,
    };
    if let Some(parent) = rollback_path.parent() {
        fs::create_dir_all(parent).await?;
    }
    fs::write(rollback_path, serde_yaml_ng::to_string(&checkpoint)?.as_bytes()).await?;

    Ok(RustSocksBindRollbackEvidence {
        checkpoint_path: rollback_path.to_string_lossy().to_string().into(),
        fallback_retained_for: checkpoint.fallback_retained_for,
        created_at_epoch_seconds,
    })
}

fn retained_socks_bind_fallback_scope() -> Vec<String> {
    vec![
        "SOCKS non-loopback UDP forwarding and UDP fragments".into(),
        "Shadowsocks UDP/plugin transports".into(),
        "VMess, VLESS, and Trojan encrypted sessions".into(),
        "system-wide packet capture and transparent proxy defaults".into(),
    ]
}

fn rust_socks_bind_facts() -> Vec<String> {
    vec![
        "Rust negotiates SOCKS5 username/password method 0x02 before BIND".into(),
        "Rust validates a bounded IPv4 loopback BIND request".into(),
        "Rust sends both BIND success replies and forwards one loopback peer request/response".into(),
        "Mihomo fallback remains retained for non-loopback UDP, broad fragment queues/timeouts, plugin transports, and packet capture"
            .into(),
    ]
}

fn rust_socks_bind_dir() -> Result<std::path::PathBuf> {
    Ok(dirs::app_runtime_dir()?.join(RUST_SOCKS_BIND_COMPONENT))
}

fn rust_socks_bind_evidence_path() -> Result<std::path::PathBuf> {
    Ok(rust_socks_bind_dir()?.join(RUST_SOCKS_BIND_EVIDENCE_FILE))
}

fn rust_socks_bind_rollback_path() -> Result<std::path::PathBuf> {
    Ok(rust_socks_bind_dir()?.join(RUST_SOCKS_BIND_ROLLBACK_FILE))
}

fn rust_socks_bind_epoch_seconds() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_content_length() {
        let headers = b"HTTP/1.1 200 OK\r\nContent-Length: 16\r\n\r\n";

        assert_eq!(parse_content_length(headers).unwrap(), 16);
    }

    #[test]
    fn builds_loopback_bind_request() {
        let request = build_bind_request(SocketAddr::from((Ipv4Addr::LOCALHOST, 2081))).unwrap();

        assert_eq!(request[0], SOCKS_VERSION);
        assert_eq!(request[1], SOCKS_CMD_BIND);
        assert_eq!(request[3], SOCKS_ATYP_IPV4);
        assert_eq!(&request[4..8], &[127, 0, 0, 1]);
        assert_eq!(u16::from_be_bytes([request[8], request[9]]), 2081);
    }
}
