use super::{
    RUST_RUNTIME_ID, RustProtocolForwardingSubsetAccounting, RustProtocolForwardingSubsetPreflightReport,
    RustProtocolForwardingSubsetSmokeEvidenceReport, RustProtocolForwardingSubsetStartReport,
    RustProtocolForwardingSubsetStatus, RustProtocolForwardingSubsetStatusReport,
    RustProtocolForwardingSubsetStopReport,
};
use anyhow::Result;
use once_cell::sync::Lazy;
use smartstring::alias::String;
use std::{
    net::{IpAddr, Ipv4Addr, Ipv6Addr, TcpListener as StdTcpListener},
    sync::{
        Arc, Mutex,
        atomic::{AtomicU64, Ordering},
    },
    time::{SystemTime, UNIX_EPOCH},
};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
    sync::oneshot,
    time::{Duration, timeout},
};

const RUST_PROTOCOL_FORWARDING_COMPONENT: &str = "rust-protocol-forwarding-subset";
const RUST_PROTOCOL_FORWARDING_KERNEL_AREA: &str = "protocol-forwarding";
const RUST_PROTOCOL_FORWARDING_HOST: &str = "127.0.0.1";
const DEFAULT_RUST_PROTOCOL_FORWARDING_LISTENER_PORT: u16 = 19280;
const DEFAULT_RUST_PROTOCOL_FORWARDING_TARGET_PORT: u16 = 19281;
const RUST_PROTOCOL_FORWARDING_NEXT_BATCH: &str = "rust-tun-system-proxy-parity";
const RUST_PROTOCOL_FORWARDING_SUPPORTED_PROTOCOLS: [&str; 2] = ["tcp-loopback-direct", "http/1.1-over-loopback-tcp"];

static RUST_PROTOCOL_FORWARDER: Lazy<Mutex<Option<RustProtocolForwardingState>>> = Lazy::new(|| Mutex::new(None));

struct RustProtocolForwardingState {
    listener_port: u16,
    target_host: String,
    target_port: u16,
    started_at_epoch_ms: u64,
    accounting: Arc<RustProtocolForwardingAccountingState>,
    stop_tx: oneshot::Sender<()>,
}

#[derive(Default)]
struct RustProtocolForwardingAccountingState {
    accepted_connections: AtomicU64,
    completed_connections: AtomicU64,
    failed_connections: AtomicU64,
    bytes_from_client: AtomicU64,
    bytes_from_target: AtomicU64,
    last_error: Mutex<Option<String>>,
}

pub async fn rust_protocol_forwarding_subset_preflight(
    listener_port: Option<u16>,
    target_host: Option<String>,
    target_port: Option<u16>,
) -> RustProtocolForwardingSubsetPreflightReport {
    let listener_port = listener_port.unwrap_or(DEFAULT_RUST_PROTOCOL_FORWARDING_LISTENER_PORT);
    let target_host = normalize_forwarding_target_host(target_host);
    let target_port = target_port.unwrap_or(DEFAULT_RUST_PROTOCOL_FORWARDING_TARGET_PORT);
    let mut blockers = Vec::new();

    if listener_port == 0 {
        blockers.push("listener port must be non-zero".into());
    }
    if target_port == 0 {
        blockers.push("target port must be non-zero".into());
    }
    if listener_port == target_port {
        blockers.push("listener port and target port must be different".into());
    }
    if !rust_protocol_forwarding_loopback_host(&target_host) {
        blockers.push("target host must be loopback-only for the Rust protocol subset".into());
    }
    if !rust_protocol_forwarding_port_available(listener_port) {
        blockers.push("listener port is already in use".into());
    }
    if rust_protocol_forwarding_status_snapshot().running {
        blockers.push("Rust protocol forwarding subset is already running".into());
    }

    let can_start_after_opt_in = blockers.is_empty();
    RustProtocolForwardingSubsetPreflightReport {
        runtime_id: RUST_RUNTIME_ID.into(),
        component: RUST_PROTOCOL_FORWARDING_COMPONENT.into(),
        kernel_area: RUST_PROTOCOL_FORWARDING_KERNEL_AREA.into(),
        status: if can_start_after_opt_in {
            RustProtocolForwardingSubsetStatus::Ready
        } else {
            RustProtocolForwardingSubsetStatus::Blocked
        },
        reason: if can_start_after_opt_in {
            "Rust protocol forwarding subset can start after explicit opt-in".into()
        } else {
            "Rust protocol forwarding subset preflight is blocked".into()
        },
        listener_host: RUST_PROTOCOL_FORWARDING_HOST.into(),
        listener_port,
        target_host: target_host.clone(),
        target_port,
        can_start_after_opt_in,
        explicit_opt_in_required: true,
        loopback_only: true,
        supported_protocols: rust_protocol_forwarding_supported_protocols(),
        mutates_runtime: false,
        live_execution_allowed: can_start_after_opt_in,
        default_route: false,
        forwards_traffic: false,
        outbound_adapters_used: false,
        mihomo_fallback: true,
        blockers,
        warnings: vec![
            "subset is limited to loopback TCP forwarding and does not own TUN/system proxy".into(),
            "Mihomo remains the fallback for non-loopback targets, SOCKS, TLS interception, and adapters".into(),
        ],
        facts: rust_protocol_forwarding_facts(),
        next_safe_batch: RUST_PROTOCOL_FORWARDING_NEXT_BATCH.into(),
    }
}

pub async fn start_rust_protocol_forwarding_subset(
    listener_port: Option<u16>,
    target_host: Option<String>,
    target_port: Option<u16>,
    explicit_opt_in: bool,
) -> Result<RustProtocolForwardingSubsetStartReport> {
    let preflight = rust_protocol_forwarding_subset_preflight(listener_port, target_host, target_port).await;
    let mut blockers = preflight.blockers.clone();
    if !explicit_opt_in {
        blockers.push("explicit opt-in is required to start Rust protocol forwarding".into());
    }
    if !blockers.is_empty() {
        let status = rust_protocol_forwarding_status_with_warnings(preflight.warnings.clone());
        return Ok(RustProtocolForwardingSubsetStartReport {
            preflight,
            status,
            explicit_opt_in,
            started: false,
            blockers,
            warnings: Vec::new(),
            facts: rust_protocol_forwarding_facts(),
        });
    }

    let listener = TcpListener::bind((RUST_PROTOCOL_FORWARDING_HOST, preflight.listener_port)).await?;
    let accounting = Arc::new(RustProtocolForwardingAccountingState::default());
    let (stop_tx, stop_rx) = oneshot::channel();
    let target_host = preflight.target_host.clone();
    let target_port = preflight.target_port;
    let listener_port = preflight.listener_port;
    let started_at_epoch_ms = rust_protocol_forwarding_epoch_ms();
    tokio::spawn(rust_protocol_forwarding_accept_loop(
        listener,
        target_host.clone(),
        target_port,
        accounting.clone(),
        stop_rx,
    ));

    {
        let mut guard = RUST_PROTOCOL_FORWARDER
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        *guard = Some(RustProtocolForwardingState {
            listener_port,
            target_host,
            target_port,
            started_at_epoch_ms,
            accounting,
            stop_tx,
        });
    }

    let status = rust_protocol_forwarding_status_snapshot();
    Ok(RustProtocolForwardingSubsetStartReport {
        preflight,
        status,
        explicit_opt_in,
        started: true,
        blockers: Vec::new(),
        warnings: vec![
            "started loopback-only Rust TCP forwarding; Mihomo remains fallback for all other traffic".into(),
        ],
        facts: rust_protocol_forwarding_facts(),
    })
}

pub async fn rust_protocol_forwarding_subset_status() -> RustProtocolForwardingSubsetStatusReport {
    rust_protocol_forwarding_status_snapshot()
}

pub async fn stop_rust_protocol_forwarding_subset() -> RustProtocolForwardingSubsetStopReport {
    let previous_status = rust_protocol_forwarding_status_snapshot();
    let stopped = {
        let mut guard = RUST_PROTOCOL_FORWARDER
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        guard
            .take()
            .map(|state| state.stop_tx.send(()).is_ok())
            .unwrap_or(false)
    };
    let after_status = rust_protocol_forwarding_status_snapshot();
    RustProtocolForwardingSubsetStopReport {
        status: if stopped {
            RustProtocolForwardingSubsetStatus::Stopped
        } else {
            RustProtocolForwardingSubsetStatus::Blocked
        },
        reason: if stopped {
            "Rust protocol forwarding subset stopped".into()
        } else {
            "Rust protocol forwarding subset was not running".into()
        },
        stopped,
        previous_status,
        after_status,
        blockers: if stopped {
            Vec::new()
        } else {
            vec!["Rust protocol forwarding subset was not running".into()]
        },
        warnings: Vec::new(),
        facts: rust_protocol_forwarding_facts(),
    }
}

pub async fn rust_protocol_forwarding_subset_smoke_evidence(
    listener_port: Option<u16>,
    target_port: Option<u16>,
) -> Result<RustProtocolForwardingSubsetSmokeEvidenceReport> {
    let listener_port = listener_port.unwrap_or(DEFAULT_RUST_PROTOCOL_FORWARDING_LISTENER_PORT);
    let target_port = target_port.unwrap_or(DEFAULT_RUST_PROTOCOL_FORWARDING_TARGET_PORT);
    let target = TcpListener::bind((RUST_PROTOCOL_FORWARDING_HOST, target_port)).await?;
    let target_task = tokio::spawn(async move {
        let (mut stream, _) = timeout(Duration::from_secs(3), target.accept()).await??;
        let mut request = [0_u8; 1024];
        let request_len = timeout(Duration::from_secs(3), stream.read(&mut request)).await??;
        let received = std::str::from_utf8(&request[..request_len])
            .map(|request| request.contains("GET /rust-protocol-forwarding-subset"))
            .unwrap_or(false);
        stream
            .write_all(b"HTTP/1.1 204 No Content\r\nConnection: close\r\nContent-Length: 0\r\n\r\n")
            .await?;
        stream.shutdown().await?;
        Ok::<bool, anyhow::Error>(received)
    });

    let start = start_rust_protocol_forwarding_subset(
        Some(listener_port),
        Some(RUST_PROTOCOL_FORWARDING_HOST.into()),
        Some(target_port),
        true,
    )
    .await?;
    let mut blockers = start.blockers.clone();
    let response_status = if start.started {
        let mut client = timeout(
            Duration::from_secs(3),
            TcpStream::connect((RUST_PROTOCOL_FORWARDING_HOST, listener_port)),
        )
        .await??;
        client
            .write_all(b"GET /rust-protocol-forwarding-subset HTTP/1.1\r\nHost: 127.0.0.1\r\nConnection: close\r\n\r\n")
            .await?;
        client.shutdown().await?;
        let mut response = [0_u8; 512];
        let response_len = timeout(Duration::from_secs(3), client.read(&mut response)).await??;
        let response = std::string::String::from_utf8_lossy(&response[..response_len]);
        response.lines().next().map(Into::into)
    } else {
        None
    };

    let target_received = target_task.await??;
    if response_status.as_deref() != Some("HTTP/1.1 204 No Content") {
        blockers.push("Rust protocol forwarding smoke response did not return HTTP 204".into());
    }
    if !target_received {
        blockers.push("target did not receive the forwarded Rust protocol smoke request".into());
    }
    let status_before_stop = rust_protocol_forwarding_wait_for_accounting().await;
    let stop_report = Some(stop_rust_protocol_forwarding_subset().await);
    let passed = blockers.is_empty()
        && status_before_stop.accounting.accepted_connections > 0
        && status_before_stop.accounting.bytes_from_client > 0
        && status_before_stop.accounting.bytes_from_target > 0;

    Ok(RustProtocolForwardingSubsetSmokeEvidenceReport {
        runtime_id: RUST_RUNTIME_ID.into(),
        component: "rust-protocol-forwarding-subset-smoke-evidence".into(),
        kernel_area: RUST_PROTOCOL_FORWARDING_KERNEL_AREA.into(),
        status: if passed {
            RustProtocolForwardingSubsetStatus::Stopped
        } else {
            RustProtocolForwardingSubsetStatus::Blocked
        },
        listener_port,
        target_port,
        target_received,
        response_status,
        accounting: status_before_stop.accounting,
        stop_report,
        passed,
        mutates_runtime: true,
        live_execution_allowed: true,
        default_route: false,
        forwards_traffic: true,
        outbound_adapters_used: false,
        mihomo_fallback: true,
        blockers,
        warnings: vec!["smoke evidence exercises real Rust TCP forwarding but only against loopback endpoints".into()],
        facts: rust_protocol_forwarding_facts(),
        next_safe_batch: RUST_PROTOCOL_FORWARDING_NEXT_BATCH.into(),
    })
}

async fn rust_protocol_forwarding_accept_loop(
    listener: TcpListener,
    target_host: String,
    target_port: u16,
    accounting: Arc<RustProtocolForwardingAccountingState>,
    mut stop_rx: oneshot::Receiver<()>,
) {
    loop {
        tokio::select! {
            _ = &mut stop_rx => break,
            accepted = listener.accept() => {
                match accepted {
                    Ok((inbound, _)) => {
                        accounting.accepted_connections.fetch_add(1, Ordering::Relaxed);
                        let target_host = target_host.clone();
                        let accounting = accounting.clone();
                        tokio::spawn(async move {
                            if let Err(error) = rust_protocol_forward_connection(inbound, &target_host, target_port, accounting.clone()).await {
                                accounting.failed_connections.fetch_add(1, Ordering::Relaxed);
                                rust_protocol_forwarding_set_last_error(&accounting, error.to_string().into());
                            }
                        });
                    }
                    Err(error) => {
                        accounting.failed_connections.fetch_add(1, Ordering::Relaxed);
                        rust_protocol_forwarding_set_last_error(&accounting, error.to_string().into());
                        break;
                    }
                }
            }
        }
    }
}

async fn rust_protocol_forward_connection(
    mut inbound: TcpStream,
    target_host: &str,
    target_port: u16,
    accounting: Arc<RustProtocolForwardingAccountingState>,
) -> Result<()> {
    let mut outbound = timeout(Duration::from_secs(5), TcpStream::connect((target_host, target_port))).await??;
    let (bytes_from_client, bytes_from_target) = tokio::io::copy_bidirectional(&mut inbound, &mut outbound).await?;
    accounting
        .bytes_from_client
        .fetch_add(bytes_from_client, Ordering::Relaxed);
    accounting
        .bytes_from_target
        .fetch_add(bytes_from_target, Ordering::Relaxed);
    accounting.completed_connections.fetch_add(1, Ordering::Relaxed);
    Ok(())
}

fn rust_protocol_forwarding_status_snapshot() -> RustProtocolForwardingSubsetStatusReport {
    let guard = RUST_PROTOCOL_FORWARDER
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    if let Some(state) = guard.as_ref() {
        RustProtocolForwardingSubsetStatusReport {
            runtime_id: RUST_RUNTIME_ID.into(),
            component: RUST_PROTOCOL_FORWARDING_COMPONENT.into(),
            kernel_area: RUST_PROTOCOL_FORWARDING_KERNEL_AREA.into(),
            status: RustProtocolForwardingSubsetStatus::Running,
            reason: "Rust protocol forwarding subset is running".into(),
            running: true,
            listener_host: RUST_PROTOCOL_FORWARDING_HOST.into(),
            listener_port: Some(state.listener_port),
            target_host: Some(state.target_host.clone()),
            target_port: Some(state.target_port),
            started_at_epoch_ms: Some(state.started_at_epoch_ms),
            accounting: rust_protocol_forwarding_accounting(&state.accounting),
            loopback_only: true,
            supported_protocols: rust_protocol_forwarding_supported_protocols(),
            mutates_runtime: true,
            live_execution_allowed: true,
            default_route: false,
            forwards_traffic: true,
            outbound_adapters_used: false,
            mihomo_fallback: true,
            blockers: Vec::new(),
            warnings: Vec::new(),
            facts: rust_protocol_forwarding_facts(),
            next_safe_batch: RUST_PROTOCOL_FORWARDING_NEXT_BATCH.into(),
        }
    } else {
        rust_protocol_forwarding_status_with_warnings(Vec::new())
    }
}

async fn rust_protocol_forwarding_wait_for_accounting() -> RustProtocolForwardingSubsetStatusReport {
    for _ in 0..20 {
        let status = rust_protocol_forwarding_status_snapshot();
        if status.accounting.completed_connections > 0 || status.accounting.failed_connections > 0 {
            return status;
        }
        tokio::time::sleep(Duration::from_millis(25)).await;
    }
    rust_protocol_forwarding_status_snapshot()
}

fn rust_protocol_forwarding_status_with_warnings(warnings: Vec<String>) -> RustProtocolForwardingSubsetStatusReport {
    RustProtocolForwardingSubsetStatusReport {
        runtime_id: RUST_RUNTIME_ID.into(),
        component: RUST_PROTOCOL_FORWARDING_COMPONENT.into(),
        kernel_area: RUST_PROTOCOL_FORWARDING_KERNEL_AREA.into(),
        status: RustProtocolForwardingSubsetStatus::Stopped,
        reason: "Rust protocol forwarding subset is stopped".into(),
        running: false,
        listener_host: RUST_PROTOCOL_FORWARDING_HOST.into(),
        listener_port: None,
        target_host: None,
        target_port: None,
        started_at_epoch_ms: None,
        accounting: RustProtocolForwardingSubsetAccounting {
            accepted_connections: 0,
            completed_connections: 0,
            failed_connections: 0,
            bytes_from_client: 0,
            bytes_from_target: 0,
            last_error: None,
        },
        loopback_only: true,
        supported_protocols: rust_protocol_forwarding_supported_protocols(),
        mutates_runtime: false,
        live_execution_allowed: false,
        default_route: false,
        forwards_traffic: false,
        outbound_adapters_used: false,
        mihomo_fallback: true,
        blockers: Vec::new(),
        warnings,
        facts: rust_protocol_forwarding_facts(),
        next_safe_batch: RUST_PROTOCOL_FORWARDING_NEXT_BATCH.into(),
    }
}

fn rust_protocol_forwarding_accounting(
    accounting: &RustProtocolForwardingAccountingState,
) -> RustProtocolForwardingSubsetAccounting {
    RustProtocolForwardingSubsetAccounting {
        accepted_connections: accounting.accepted_connections.load(Ordering::Relaxed),
        completed_connections: accounting.completed_connections.load(Ordering::Relaxed),
        failed_connections: accounting.failed_connections.load(Ordering::Relaxed),
        bytes_from_client: accounting.bytes_from_client.load(Ordering::Relaxed),
        bytes_from_target: accounting.bytes_from_target.load(Ordering::Relaxed),
        last_error: accounting
            .last_error
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .clone(),
    }
}

fn rust_protocol_forwarding_set_last_error(accounting: &RustProtocolForwardingAccountingState, error: String) {
    let mut guard = accounting.last_error.lock().unwrap_or_else(|error| error.into_inner());
    *guard = Some(error);
}

fn normalize_forwarding_target_host(target_host: Option<String>) -> String {
    target_host
        .as_deref()
        .map(str::trim)
        .filter(|host| !host.is_empty())
        .unwrap_or(RUST_PROTOCOL_FORWARDING_HOST)
        .into()
}

fn rust_protocol_forwarding_loopback_host(host: &str) -> bool {
    host.eq_ignore_ascii_case("localhost")
        || host
            .parse::<IpAddr>()
            .map(|addr| match addr {
                IpAddr::V4(addr) => addr.octets()[0] == 127,
                IpAddr::V6(addr) => addr == Ipv6Addr::LOCALHOST,
            })
            .unwrap_or(false)
        || host
            .parse::<Ipv4Addr>()
            .map(|addr| addr.octets()[0] == 127)
            .unwrap_or(false)
}

fn rust_protocol_forwarding_port_available(port: u16) -> bool {
    port > 0 && StdTcpListener::bind((RUST_PROTOCOL_FORWARDING_HOST, port)).is_ok()
}

fn rust_protocol_forwarding_supported_protocols() -> Vec<String> {
    RUST_PROTOCOL_FORWARDING_SUPPORTED_PROTOCOLS
        .iter()
        .map(|protocol| (*protocol).into())
        .collect()
}

fn rust_protocol_forwarding_facts() -> Vec<String> {
    vec![
        "Rust owns the listener accept loop and TCP byte forwarding for this subset".into(),
        "the subset is loopback-only and never installs a system proxy or TUN route".into(),
        "connection/session accounting is maintained in Rust atomics".into(),
        "Mihomo remains fallback for adapters, SOCKS, remote proxy protocols, and non-loopback traffic".into(),
    ]
}

fn rust_protocol_forwarding_epoch_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis().try_into().unwrap_or(u64::MAX))
        .unwrap_or_default()
}
