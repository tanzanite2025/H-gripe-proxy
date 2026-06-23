use super::{
    RUST_RUNTIME_ID, RustDefaultDataPlaneCloseoutGateEvidence, RustSocksAuthExecutionEvidence,
    RustSocksAuthExecutionReport, RustSocksAuthExecutionStatus, RustSocksAuthLeakEvidence,
    RustSocksAuthRollbackEvidence, rust_default_data_plane_closeout_gate_evidence,
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

const RUST_SOCKS_AUTH_COMPONENT: &str = "rust-socks-auth-execution";
const RUST_SOCKS_AUTH_KERNEL_AREA: &str = "socks-auth";
const RUST_SOCKS_AUTH_EVIDENCE_FILE: &str = "evidence.yaml";
const RUST_SOCKS_AUTH_ROLLBACK_FILE: &str = "rollback-checkpoint.yaml";
const NEXT_SAFE_BATCH: &str = "route-packet-capture-privileged-hold";
const SOCKS_VERSION: u8 = 0x05;
const SOCKS_USERPASS_METHOD: u8 = 0x02;
const SOCKS_NO_ACCEPTABLE_METHODS: u8 = 0xff;
const SOCKS_AUTH_VERSION: u8 = 0x01;
const SOCKS_AUTH_SUCCESS: u8 = 0x00;
const SOCKS_CMD_CONNECT: u8 = 0x01;
const SOCKS_ATYP_IPV4: u8 = 0x01;
const TEST_USERNAME: &[u8] = b"rust-user";
const TEST_PASSWORD: &[u8] = b"rust-pass";

pub async fn rust_socks_auth_execution(explicit_opt_in: bool) -> Result<RustSocksAuthExecutionReport> {
    let default_data_plane_closeout_gate = rust_default_data_plane_closeout_gate_evidence().await?;

    if !explicit_opt_in {
        let mut blockers = vec!["SOCKS username/password authentication execution requires explicit opt-in".into()];
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

    let rollback_path = rust_socks_auth_rollback_path()?;
    let rollback_evidence = write_rollback_checkpoint(&rollback_path).await?;
    let auth_evidence = match run_bounded_socks_auth_handshake() {
        Ok(evidence) => evidence,
        Err(error) => {
            return Ok(blocked_report(
                explicit_opt_in,
                default_data_plane_closeout_gate,
                vec![format!("bounded SOCKS auth execution failed: {error}").into()],
            ));
        }
    };
    let leak_evidence = RustSocksAuthLeakEvidence {
        passed: auth_evidence.loopback_only
            && auth_evidence.method_negotiated
            && auth_evidence.auth_accepted
            && auth_evidence.connect_request_validated,
        no_system_packet_capture: true,
        no_non_loopback_target: auth_evidence.loopback_only,
        no_mihomo_binary_removal: true,
    };
    let evidence_path = rust_socks_auth_evidence_path()?;
    let mut report = RustSocksAuthExecutionReport {
        runtime_id: RUST_RUNTIME_ID.into(),
        component: RUST_SOCKS_AUTH_COMPONENT.into(),
        kernel_area: RUST_SOCKS_AUTH_KERNEL_AREA.into(),
        status: RustSocksAuthExecutionStatus::Executed,
        reason: "Rust executed bounded SOCKS5 username/password negotiation over loopback TCP".into(),
        explicit_opt_in,
        rust_owned_scope: "SOCKS5 username/password method negotiation and loopback CONNECT preflight".into(),
        default_data_plane_closeout_gate,
        mutates_runtime: false,
        writes_evidence: true,
        evidence_path: Some(evidence_path.to_string_lossy().to_string().into()),
        auth_evidence: Some(auth_evidence),
        rollback_evidence: Some(rollback_evidence),
        leak_evidence: Some(leak_evidence),
        mihomo_fallback_retained_for: retained_socks_auth_fallback_scope(),
        blockers: Vec::new(),
        warnings: vec![
            "SOCKS BIND, non-loopback UDP, broad fragment queues/timeouts, and packet capture remain Mihomo-owned"
                .into(),
        ],
        facts: rust_socks_auth_facts(),
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
) -> RustSocksAuthExecutionReport {
    RustSocksAuthExecutionReport {
        runtime_id: RUST_RUNTIME_ID.into(),
        component: RUST_SOCKS_AUTH_COMPONENT.into(),
        kernel_area: RUST_SOCKS_AUTH_KERNEL_AREA.into(),
        status: RustSocksAuthExecutionStatus::Blocked,
        reason: "Rust SOCKS auth execution is blocked".into(),
        explicit_opt_in,
        rust_owned_scope: "SOCKS5 username/password method negotiation and loopback CONNECT preflight".into(),
        default_data_plane_closeout_gate,
        mutates_runtime: false,
        writes_evidence: false,
        evidence_path: None,
        auth_evidence: None,
        rollback_evidence: None,
        leak_evidence: None,
        mihomo_fallback_retained_for: retained_socks_auth_fallback_scope(),
        blockers,
        warnings: Vec::new(),
        facts: rust_socks_auth_facts(),
        next_safe_batch: NEXT_SAFE_BATCH.into(),
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RustSocksAuthRollbackCheckpoint {
    component: String,
    rust_owned_scope: String,
    fallback_retained_for: Vec<String>,
    created_at_epoch_seconds: u64,
}

struct ServerAuthEvidence {
    method_negotiated: bool,
    auth_accepted: bool,
    connect_request_validated: bool,
    selected_method: u8,
    command: u8,
    atyp: u8,
}

fn run_bounded_socks_auth_handshake() -> Result<RustSocksAuthExecutionEvidence> {
    let listener =
        TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).context("failed to bind SOCKS auth loopback listener")?;
    listener.set_nonblocking(false)?;
    let listener_addr = listener.local_addr()?;
    let handle = thread::spawn(move || run_socks_auth_server(listener));

    let mut client = TcpStream::connect(listener_addr).context("failed to connect SOCKS auth client")?;
    client.set_read_timeout(Some(Duration::from_secs(2)))?;
    client.set_write_timeout(Some(Duration::from_secs(2)))?;
    client.write_all(&[SOCKS_VERSION, 0x01, SOCKS_USERPASS_METHOD])?;
    let mut method_response = [0_u8; 2];
    client.read_exact(&mut method_response)?;
    if method_response != [SOCKS_VERSION, SOCKS_USERPASS_METHOD] {
        return Err(anyhow!("SOCKS auth method negotiation failed"));
    }

    let auth_request = build_auth_request(TEST_USERNAME, TEST_PASSWORD)?;
    client.write_all(&auth_request)?;
    let mut auth_response = [0_u8; 2];
    client.read_exact(&mut auth_response)?;
    if auth_response != [SOCKS_AUTH_VERSION, SOCKS_AUTH_SUCCESS] {
        return Err(anyhow!("SOCKS auth credentials were rejected"));
    }

    let target_port = listener_addr.port();
    client.write_all(&build_connect_request(target_port))?;
    let mut connect_response = [0_u8; 10];
    client.read_exact(&mut connect_response)?;
    if connect_response[0] != SOCKS_VERSION || connect_response[1] != SOCKS_AUTH_SUCCESS {
        return Err(anyhow!("SOCKS loopback CONNECT preflight was rejected"));
    }

    let server_evidence = handle
        .join()
        .map_err(|_| anyhow!("SOCKS auth server thread panicked"))??;
    Ok(RustSocksAuthExecutionEvidence {
        listener_addr: listener_addr.to_string().into(),
        selected_method: format!("0x{:02x}", server_evidence.selected_method).into(),
        username_bytes: TEST_USERNAME.len(),
        password_bytes: TEST_PASSWORD.len(),
        auth_version: SOCKS_AUTH_VERSION,
        method_negotiated: server_evidence.method_negotiated,
        auth_accepted: server_evidence.auth_accepted,
        connect_command: format!("0x{:02x}", server_evidence.command).into(),
        connect_atyp: format!("0x{:02x}", server_evidence.atyp).into(),
        connect_request_validated: server_evidence.connect_request_validated,
        loopback_only: listener_addr.ip().is_loopback(),
    })
}

fn run_socks_auth_server(listener: TcpListener) -> Result<ServerAuthEvidence> {
    let (mut stream, peer_addr) = listener.accept()?;
    if !peer_addr.ip().is_loopback() {
        return Err(anyhow!("SOCKS auth peer was not loopback"));
    }
    stream.set_read_timeout(Some(Duration::from_secs(2)))?;
    stream.set_write_timeout(Some(Duration::from_secs(2)))?;

    let mut greeting_header = [0_u8; 2];
    stream.read_exact(&mut greeting_header)?;
    if greeting_header[0] != SOCKS_VERSION {
        stream.write_all(&[SOCKS_VERSION, SOCKS_NO_ACCEPTABLE_METHODS])?;
        return Err(anyhow!("SOCKS version mismatch"));
    }
    let method_count = usize::from(greeting_header[1]);
    let mut methods = vec![0_u8; method_count];
    stream.read_exact(&mut methods)?;
    if !methods.contains(&SOCKS_USERPASS_METHOD) {
        stream.write_all(&[SOCKS_VERSION, SOCKS_NO_ACCEPTABLE_METHODS])?;
        return Err(anyhow!("SOCKS user/pass method was not offered"));
    }
    stream.write_all(&[SOCKS_VERSION, SOCKS_USERPASS_METHOD])?;

    let auth_accepted = read_and_validate_auth(&mut stream)?;
    let auth_status = if auth_accepted { SOCKS_AUTH_SUCCESS } else { 0x01 };
    stream.write_all(&[SOCKS_AUTH_VERSION, auth_status])?;
    if !auth_accepted {
        return Err(anyhow!("SOCKS auth credentials mismatch"));
    }
    let (command, atyp, target) = read_connect_request(&mut stream)?;
    if !target.ip().is_loopback() {
        return Err(anyhow!("SOCKS auth target was not loopback"));
    }
    stream.write_all(&[
        SOCKS_VERSION,
        SOCKS_AUTH_SUCCESS,
        0x00,
        SOCKS_ATYP_IPV4,
        127,
        0,
        0,
        1,
        (target.port() >> 8) as u8,
        (target.port() & 0xff) as u8,
    ])?;

    Ok(ServerAuthEvidence {
        method_negotiated: true,
        auth_accepted,
        connect_request_validated: command == SOCKS_CMD_CONNECT && atyp == SOCKS_ATYP_IPV4,
        selected_method: SOCKS_USERPASS_METHOD,
        command,
        atyp,
    })
}

fn read_and_validate_auth(stream: &mut TcpStream) -> Result<bool> {
    let mut version = [0_u8; 1];
    stream.read_exact(&mut version)?;
    if version[0] != SOCKS_AUTH_VERSION {
        return Err(anyhow!("SOCKS auth version mismatch"));
    }
    let username = read_len_prefixed_field(stream)?;
    let password = read_len_prefixed_field(stream)?;
    Ok(username == TEST_USERNAME && password == TEST_PASSWORD)
}

fn read_connect_request(stream: &mut TcpStream) -> Result<(u8, u8, SocketAddr)> {
    let mut header = [0_u8; 4];
    stream.read_exact(&mut header)?;
    if header[0] != SOCKS_VERSION || header[2] != 0x00 || header[3] != SOCKS_ATYP_IPV4 {
        return Err(anyhow!("unsupported bounded SOCKS CONNECT request"));
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

fn build_connect_request(target_port: u16) -> [u8; 10] {
    let port = target_port.to_be_bytes();
    [
        SOCKS_VERSION,
        SOCKS_CMD_CONNECT,
        0x00,
        SOCKS_ATYP_IPV4,
        127,
        0,
        0,
        1,
        port[0],
        port[1],
    ]
}

async fn write_rollback_checkpoint(rollback_path: &std::path::Path) -> Result<RustSocksAuthRollbackEvidence> {
    let created_at_epoch_seconds = rust_socks_auth_epoch_seconds();
    let checkpoint = RustSocksAuthRollbackCheckpoint {
        component: RUST_SOCKS_AUTH_COMPONENT.into(),
        rust_owned_scope: "SOCKS5 username/password method negotiation and loopback CONNECT preflight".into(),
        fallback_retained_for: retained_socks_auth_fallback_scope(),
        created_at_epoch_seconds,
    };
    if let Some(parent) = rollback_path.parent() {
        fs::create_dir_all(parent).await?;
    }
    fs::write(rollback_path, serde_yaml_ng::to_string(&checkpoint)?.as_bytes()).await?;

    Ok(RustSocksAuthRollbackEvidence {
        checkpoint_path: rollback_path.to_string_lossy().to_string().into(),
        fallback_retained_for: checkpoint.fallback_retained_for,
        created_at_epoch_seconds,
    })
}

fn retained_socks_auth_fallback_scope() -> Vec<String> {
    vec![
        "SOCKS unauthenticated and GSSAPI negotiation".into(),
        "SOCKS TCP CONNECT data forwarding and BIND command handling".into(),
        "SOCKS UDP non-loopback forwarding and broad fragment queues/timeouts".into(),
        "Shadowsocks UDP/plugin transports".into(),
        "system-wide packet capture and transparent proxy defaults".into(),
    ]
}

fn rust_socks_auth_facts() -> Vec<String> {
    vec![
        "Rust negotiates SOCKS5 username/password method 0x02 over loopback TCP".into(),
        "Rust validates an RFC1929 username/password frame without persisting credential values".into(),
        "Rust validates a loopback CONNECT preflight but does not claim TCP data forwarding at this batch boundary"
            .into(),
        "Mihomo fallback remains retained for non-loopback UDP, broad fragment queues/timeouts, plugin transports, and packet capture"
            .into(),
    ]
}

fn rust_socks_auth_dir() -> Result<std::path::PathBuf> {
    Ok(dirs::app_runtime_dir()?.join(RUST_SOCKS_AUTH_COMPONENT))
}

fn rust_socks_auth_evidence_path() -> Result<std::path::PathBuf> {
    Ok(rust_socks_auth_dir()?.join(RUST_SOCKS_AUTH_EVIDENCE_FILE))
}

fn rust_socks_auth_rollback_path() -> Result<std::path::PathBuf> {
    Ok(rust_socks_auth_dir()?.join(RUST_SOCKS_AUTH_ROLLBACK_FILE))
}

fn rust_socks_auth_epoch_seconds() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_rfc1929_auth_request() {
        let request = build_auth_request(b"user", b"pass").unwrap();

        assert_eq!(
            request,
            vec![0x01, 0x04, b'u', b's', b'e', b'r', 0x04, b'p', b'a', b's', b's']
        );
    }

    #[test]
    fn builds_loopback_connect_request() {
        let request = build_connect_request(1080);

        assert_eq!(request[0], SOCKS_VERSION);
        assert_eq!(request[1], SOCKS_CMD_CONNECT);
        assert_eq!(request[3], SOCKS_ATYP_IPV4);
        assert_eq!(&request[4..8], &[127, 0, 0, 1]);
        assert_eq!(u16::from_be_bytes([request[8], request[9]]), 1080);
    }
}
