use super::{
    RUST_RUNTIME_ID, RustSocksTcpConnectExecutionReport, RustSocksTcpConnectExecutionStatus,
    RustSocksTcpConnectForwardEvidence, RustSocksTcpConnectLeakEvidence, RustSocksTcpConnectRollbackEvidence,
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

const RUST_SOCKS_TCP_CONNECT_COMPONENT: &str = "rust-socks-tcp-connect-execution";
const RUST_SOCKS_TCP_CONNECT_KERNEL_AREA: &str = "socks-tcp-connect";
const RUST_SOCKS_TCP_CONNECT_EVIDENCE_FILE: &str = "evidence.yaml";
const RUST_SOCKS_TCP_CONNECT_ROLLBACK_FILE: &str = "rollback-checkpoint.yaml";
const NEXT_SAFE_BATCH: &str = "unsupported-protocol-and-packet-capture-implementation";
const SOCKS_VERSION: u8 = 0x05;
const SOCKS_USERPASS_METHOD: u8 = 0x02;
const SOCKS_AUTH_VERSION: u8 = 0x01;
const SOCKS_SUCCESS: u8 = 0x00;
const SOCKS_CMD_CONNECT: u8 = 0x01;
const SOCKS_ATYP_IPV4: u8 = 0x01;
const TEST_USERNAME: &[u8] = b"rust-user";
const TEST_PASSWORD: &[u8] = b"rust-pass";
const TEST_REQUEST: &[u8] = b"GET /socks-tcp-connect HTTP/1.1\r\nHost: loopback.test\r\n\r\n";
const TEST_RESPONSE: &[u8] = b"HTTP/1.1 200 OK\r\nContent-Length: 15\r\n\r\nsocks-tcp-ok:42";

pub async fn rust_socks_tcp_connect_execution(explicit_opt_in: bool) -> Result<RustSocksTcpConnectExecutionReport> {
    if !explicit_opt_in {
        return Ok(blocked_report(
            explicit_opt_in,
            vec!["SOCKS TCP CONNECT forwarding execution requires explicit opt-in".into()],
        ));
    }

    let rollback_path = rust_socks_tcp_connect_rollback_path()?;
    let rollback_evidence = write_rollback_checkpoint(&rollback_path).await?;
    let forward_evidence = match run_bounded_socks_tcp_connect_forwarding() {
        Ok(evidence) => evidence,
        Err(error) => {
            return Ok(blocked_report(
                explicit_opt_in,
                vec![format!("bounded SOCKS TCP CONNECT execution failed: {error}").into()],
            ));
        }
    };
    let leak_evidence = RustSocksTcpConnectLeakEvidence {
        passed: forward_evidence.loopback_only && forward_evidence.data_forwarded,
        no_system_packet_capture: true,
        no_non_loopback_target: forward_evidence.loopback_only,
        no_mihomo_binary_removal: true,
    };
    let evidence_path = rust_socks_tcp_connect_evidence_path()?;
    let mut report = RustSocksTcpConnectExecutionReport {
        runtime_id: RUST_RUNTIME_ID.into(),
        component: RUST_SOCKS_TCP_CONNECT_COMPONENT.into(),
        kernel_area: RUST_SOCKS_TCP_CONNECT_KERNEL_AREA.into(),
        status: RustSocksTcpConnectExecutionStatus::Executed,
        reason: "Rust executed bounded SOCKS5 TCP CONNECT data forwarding over loopback".into(),
        explicit_opt_in,
        rust_owned_scope: "SOCKS5 username/password CONNECT handshake and loopback TCP request/response forwarding".into(),
        mutates_runtime: false,
        writes_evidence: true,
        evidence_path: Some(evidence_path.to_string_lossy().to_string().into()),
        forward_evidence: Some(forward_evidence),
        rollback_evidence: Some(rollback_evidence),
        leak_evidence: Some(leak_evidence),
        mihomo_fallback_retained_for: retained_socks_tcp_connect_fallback_scope(),
        blockers: Vec::new(),
        warnings: vec![
            "SOCKS BIND, UDP fragments, non-loopback UDP, Shadowsocks UDP/plugin transports, and packet capture remain Mihomo-owned".into(),
        ],
        facts: rust_socks_tcp_connect_facts(),
        next_safe_batch: NEXT_SAFE_BATCH.into(),
    };

    if let Some(parent) = evidence_path.parent() {
        fs::create_dir_all(parent).await?;
    }
    fs::write(&evidence_path, serde_yaml_ng::to_string(&report)?.as_bytes()).await?;
    report.evidence_path = Some(evidence_path.to_string_lossy().to_string().into());

    Ok(report)
}

fn blocked_report(explicit_opt_in: bool, blockers: Vec<String>) -> RustSocksTcpConnectExecutionReport {
    RustSocksTcpConnectExecutionReport {
        runtime_id: RUST_RUNTIME_ID.into(),
        component: RUST_SOCKS_TCP_CONNECT_COMPONENT.into(),
        kernel_area: RUST_SOCKS_TCP_CONNECT_KERNEL_AREA.into(),
        status: RustSocksTcpConnectExecutionStatus::Blocked,
        reason: "Rust SOCKS TCP CONNECT execution is blocked".into(),
        explicit_opt_in,
        rust_owned_scope: "SOCKS5 username/password CONNECT handshake and loopback TCP request/response forwarding"
            .into(),
        mutates_runtime: false,
        writes_evidence: false,
        evidence_path: None,
        forward_evidence: None,
        rollback_evidence: None,
        leak_evidence: None,
        mihomo_fallback_retained_for: retained_socks_tcp_connect_fallback_scope(),
        blockers,
        warnings: Vec::new(),
        facts: rust_socks_tcp_connect_facts(),
        next_safe_batch: NEXT_SAFE_BATCH.into(),
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RustSocksTcpConnectRollbackCheckpoint {
    component: String,
    rust_owned_scope: String,
    fallback_retained_for: Vec<String>,
    created_at_epoch_seconds: u64,
}

struct TargetEvidence {
    target_addr: SocketAddr,
    target_received_bytes: usize,
}

struct ProxyEvidence {
    proxy_addr: SocketAddr,
    selected_method: u8,
    auth_negotiated: bool,
    connect_command: u8,
    connect_atyp: u8,
    request_bytes: usize,
    response_bytes: usize,
    target_received_bytes: usize,
}

fn run_bounded_socks_tcp_connect_forwarding() -> Result<RustSocksTcpConnectForwardEvidence> {
    let target_listener =
        TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).context("failed to bind SOCKS TCP loopback target")?;
    let target_addr = target_listener.local_addr()?;
    let target_handle = thread::spawn(move || run_loopback_target(target_listener));

    let proxy_listener =
        TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).context("failed to bind SOCKS TCP loopback proxy")?;
    let proxy_addr = proxy_listener.local_addr()?;
    let proxy_handle = thread::spawn(move || run_socks_tcp_proxy(proxy_listener, target_addr));

    let response = run_socks_tcp_client(proxy_addr, target_addr)?;
    let proxy_evidence = proxy_handle
        .join()
        .map_err(|_| anyhow!("SOCKS TCP proxy thread panicked"))??;
    let target_evidence = target_handle
        .join()
        .map_err(|_| anyhow!("SOCKS TCP target thread panicked"))??;
    let data_forwarded = response == TEST_RESPONSE
        && proxy_evidence.request_bytes == TEST_REQUEST.len()
        && proxy_evidence.target_received_bytes == TEST_REQUEST.len()
        && target_evidence.target_received_bytes == TEST_REQUEST.len()
        && proxy_evidence.response_bytes == TEST_RESPONSE.len();

    Ok(RustSocksTcpConnectForwardEvidence {
        proxy_listener_addr: proxy_evidence.proxy_addr.to_string().into(),
        target_addr: target_evidence.target_addr.to_string().into(),
        selected_method: format!("0x{:02x}", proxy_evidence.selected_method).into(),
        auth_negotiated: proxy_evidence.auth_negotiated,
        connect_command: format!("0x{:02x}", proxy_evidence.connect_command).into(),
        connect_atyp: format!("0x{:02x}", proxy_evidence.connect_atyp).into(),
        request_bytes: TEST_REQUEST.len(),
        target_received_bytes: target_evidence.target_received_bytes,
        response_bytes: response.len(),
        response_prefix: std::string::String::from_utf8_lossy(&response[..response.len().min(15)])
            .to_string()
            .into(),
        data_forwarded,
        loopback_only: proxy_evidence.proxy_addr.ip().is_loopback() && target_evidence.target_addr.ip().is_loopback(),
    })
}

fn run_loopback_target(listener: TcpListener) -> Result<TargetEvidence> {
    let target_addr = listener.local_addr()?;
    let (mut stream, peer_addr) = listener.accept()?;
    if !peer_addr.ip().is_loopback() {
        return Err(anyhow!("SOCKS TCP target peer was not loopback"));
    }
    stream.set_read_timeout(Some(Duration::from_secs(2)))?;
    stream.set_write_timeout(Some(Duration::from_secs(2)))?;
    let request = read_until_http_headers(&mut stream)?;
    stream.write_all(TEST_RESPONSE)?;
    Ok(TargetEvidence {
        target_addr,
        target_received_bytes: request.len(),
    })
}

fn run_socks_tcp_proxy(listener: TcpListener, target_addr: SocketAddr) -> Result<ProxyEvidence> {
    let proxy_addr = listener.local_addr()?;
    let (mut client, peer_addr) = listener.accept()?;
    if !peer_addr.ip().is_loopback() {
        return Err(anyhow!("SOCKS TCP client peer was not loopback"));
    }
    client.set_read_timeout(Some(Duration::from_secs(2)))?;
    client.set_write_timeout(Some(Duration::from_secs(2)))?;
    let selected_method = negotiate_userpass_method(&mut client)?;
    authenticate_userpass(&mut client)?;
    let (connect_command, connect_atyp, requested_target) = read_connect_request(&mut client)?;
    if requested_target != target_addr || !requested_target.ip().is_loopback() {
        return Err(anyhow!("SOCKS TCP CONNECT requested unsupported target"));
    }
    client.write_all(&connect_success_response(target_addr))?;

    let mut target = TcpStream::connect(target_addr)?;
    target.set_read_timeout(Some(Duration::from_secs(2)))?;
    target.set_write_timeout(Some(Duration::from_secs(2)))?;
    let request = read_until_http_headers(&mut client)?;
    target.write_all(&request)?;
    let response = read_http_response(&mut target)?;
    client.write_all(&response)?;

    Ok(ProxyEvidence {
        proxy_addr,
        selected_method,
        auth_negotiated: true,
        connect_command,
        connect_atyp,
        request_bytes: request.len(),
        response_bytes: response.len(),
        target_received_bytes: request.len(),
    })
}

fn run_socks_tcp_client(proxy_addr: SocketAddr, target_addr: SocketAddr) -> Result<Vec<u8>> {
    let mut client = TcpStream::connect(proxy_addr).context("failed to connect SOCKS TCP client")?;
    client.set_read_timeout(Some(Duration::from_secs(2)))?;
    client.set_write_timeout(Some(Duration::from_secs(2)))?;
    client.write_all(&[SOCKS_VERSION, 0x01, SOCKS_USERPASS_METHOD])?;
    let mut method_response = [0_u8; 2];
    client.read_exact(&mut method_response)?;
    if method_response != [SOCKS_VERSION, SOCKS_USERPASS_METHOD] {
        return Err(anyhow!("SOCKS TCP method negotiation failed"));
    }
    client.write_all(&build_auth_request(TEST_USERNAME, TEST_PASSWORD)?)?;
    let mut auth_response = [0_u8; 2];
    client.read_exact(&mut auth_response)?;
    if auth_response != [SOCKS_AUTH_VERSION, SOCKS_SUCCESS] {
        return Err(anyhow!("SOCKS TCP authentication failed"));
    }
    client.write_all(&build_connect_request(target_addr)?)?;
    let mut connect_response = [0_u8; 10];
    client.read_exact(&mut connect_response)?;
    if connect_response[0] != SOCKS_VERSION || connect_response[1] != SOCKS_SUCCESS {
        return Err(anyhow!("SOCKS TCP CONNECT failed"));
    }
    client.write_all(TEST_REQUEST)?;
    read_http_response(&mut client)
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

fn read_connect_request(stream: &mut TcpStream) -> Result<(u8, u8, SocketAddr)> {
    let mut header = [0_u8; 4];
    stream.read_exact(&mut header)?;
    if header[0] != SOCKS_VERSION || header[2] != 0x00 || header[3] != SOCKS_ATYP_IPV4 {
        return Err(anyhow!("unsupported bounded SOCKS TCP CONNECT request"));
    }
    let mut addr = [0_u8; 4];
    let mut port = [0_u8; 2];
    stream.read_exact(&mut addr)?;
    stream.read_exact(&mut port)?;
    let target = SocketAddr::from((
        Ipv4Addr::new(addr[0], addr[1], addr[2], addr[3]),
        u16::from_be_bytes(port),
    ));
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

fn build_connect_request(target_addr: SocketAddr) -> Result<[u8; 10]> {
    let SocketAddr::V4(target) = target_addr else {
        return Err(anyhow!("bounded SOCKS TCP CONNECT target must be IPv4"));
    };
    let port = target.port().to_be_bytes();
    let octets = target.ip().octets();
    Ok([
        SOCKS_VERSION,
        SOCKS_CMD_CONNECT,
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

fn connect_success_response(bound_addr: SocketAddr) -> [u8; 10] {
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

async fn write_rollback_checkpoint(rollback_path: &std::path::Path) -> Result<RustSocksTcpConnectRollbackEvidence> {
    let created_at_epoch_seconds = rust_socks_tcp_connect_epoch_seconds();
    let checkpoint = RustSocksTcpConnectRollbackCheckpoint {
        component: RUST_SOCKS_TCP_CONNECT_COMPONENT.into(),
        rust_owned_scope: "SOCKS5 username/password CONNECT handshake and loopback TCP request/response forwarding"
            .into(),
        fallback_retained_for: retained_socks_tcp_connect_fallback_scope(),
        created_at_epoch_seconds,
    };
    if let Some(parent) = rollback_path.parent() {
        fs::create_dir_all(parent).await?;
    }
    fs::write(rollback_path, serde_yaml_ng::to_string(&checkpoint)?.as_bytes()).await?;

    Ok(RustSocksTcpConnectRollbackEvidence {
        checkpoint_path: rollback_path.to_string_lossy().to_string().into(),
        fallback_retained_for: checkpoint.fallback_retained_for,
        created_at_epoch_seconds,
    })
}

fn retained_socks_tcp_connect_fallback_scope() -> Vec<String> {
    vec![
        "SOCKS BIND command handling".into(),
        "SOCKS UDP fragments and non-loopback UDP forwarding".into(),
        "Shadowsocks UDP/plugin transports".into(),
        "VMess, VLESS, and Trojan encrypted sessions".into(),
        "system-wide packet capture and transparent proxy defaults".into(),
    ]
}

fn rust_socks_tcp_connect_facts() -> Vec<String> {
    vec![
        "Rust negotiates SOCKS5 username/password method 0x02 over loopback TCP".into(),
        "Rust validates a bounded IPv4 loopback CONNECT request".into(),
        "Rust forwards one bounded HTTP request/response between loopback client and target".into(),
        "Mihomo fallback remains retained for BIND, non-loopback UDP, fragments, plugin transports, and packet capture"
            .into(),
    ]
}

fn rust_socks_tcp_connect_dir() -> Result<std::path::PathBuf> {
    Ok(dirs::app_runtime_dir()?.join(RUST_SOCKS_TCP_CONNECT_COMPONENT))
}

fn rust_socks_tcp_connect_evidence_path() -> Result<std::path::PathBuf> {
    Ok(rust_socks_tcp_connect_dir()?.join(RUST_SOCKS_TCP_CONNECT_EVIDENCE_FILE))
}

fn rust_socks_tcp_connect_rollback_path() -> Result<std::path::PathBuf> {
    Ok(rust_socks_tcp_connect_dir()?.join(RUST_SOCKS_TCP_CONNECT_ROLLBACK_FILE))
}

fn rust_socks_tcp_connect_epoch_seconds() -> u64 {
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
        let headers = b"HTTP/1.1 200 OK\r\nContent-Length: 15\r\n\r\n";

        assert_eq!(parse_content_length(headers).unwrap(), 15);
    }

    #[test]
    fn builds_loopback_connect_request() {
        let request = build_connect_request(SocketAddr::from((Ipv4Addr::LOCALHOST, 2080))).unwrap();

        assert_eq!(request[0], SOCKS_VERSION);
        assert_eq!(request[1], SOCKS_CMD_CONNECT);
        assert_eq!(request[3], SOCKS_ATYP_IPV4);
        assert_eq!(&request[4..8], &[127, 0, 0, 1]);
        assert_eq!(u16::from_be_bytes([request[8], request[9]]), 2080);
    }
}
