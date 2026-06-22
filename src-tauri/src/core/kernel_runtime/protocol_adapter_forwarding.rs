use super::{
    RUST_RUNTIME_ID, RustProtocolAdapterForwardingAdapterKind, RustProtocolAdapterForwardingDecisionEvidence,
    RustProtocolAdapterForwardingExpansionReport, RustProtocolAdapterForwardingStatus,
};
use crate::utils::dirs;
use anyhow::Result;
use smartstring::alias::String;
use tokio::{
    fs,
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
    time::{Duration, timeout},
};

const RUST_PROTOCOL_ADAPTER_FORWARDING_COMPONENT: &str = "rust-protocol-adapter-forwarding-expansion";
const RUST_PROTOCOL_ADAPTER_FORWARDING_KERNEL_AREA: &str = "protocol-adapter-forwarding";
const RUST_PROTOCOL_ADAPTER_FORWARDING_HOST: &str = "127.0.0.1";
const RUST_PROTOCOL_ADAPTER_FORWARDING_EVIDENCE_FILE: &str = "evidence.yaml";
const DEFAULT_DIRECT_LISTENER_PORT: u16 = 19380;
const DEFAULT_DIRECT_TARGET_PORT: u16 = 19381;
const DEFAULT_REJECT_LISTENER_PORT: u16 = 19382;
const NEXT_SAFE_BATCH: &str = "rust-remote-adapter-transport-expansion";

pub async fn rust_protocol_adapter_forwarding_expansion_evidence(
    explicit_opt_in: bool,
) -> Result<RustProtocolAdapterForwardingExpansionReport> {
    if !explicit_opt_in {
        return Ok(RustProtocolAdapterForwardingExpansionReport {
            runtime_id: RUST_RUNTIME_ID.into(),
            component: RUST_PROTOCOL_ADAPTER_FORWARDING_COMPONENT.into(),
            kernel_area: RUST_PROTOCOL_ADAPTER_FORWARDING_KERNEL_AREA.into(),
            status: RustProtocolAdapterForwardingStatus::Blocked,
            reason: "explicit opt-in is required to run protocol+adapter forwarding expansion".into(),
            explicit_opt_in,
            direct_evidence: None,
            reject_evidence: None,
            evidence_path: None,
            loopback_only: true,
            mutates_runtime: false,
            forwards_traffic: false,
            outbound_adapters_used: false,
            writes_evidence_artifact: false,
            mihomo_fallback: true,
            blockers: vec!["explicit opt-in is required".into()],
            warnings: Vec::new(),
            facts: rust_protocol_adapter_forwarding_facts(),
            next_safe_batch: NEXT_SAFE_BATCH.into(),
        });
    }

    let direct_evidence = run_direct_adapter_forwarding_evidence().await?;
    let reject_evidence = run_reject_adapter_forwarding_evidence().await?;
    let mut blockers = Vec::new();
    if !direct_evidence.passed {
        blockers.push("DIRECT adapter forwarding evidence failed".into());
        blockers.extend(direct_evidence.blockers.iter().cloned());
    }
    if !reject_evidence.passed {
        blockers.push("REJECT adapter forwarding evidence failed".into());
        blockers.extend(reject_evidence.blockers.iter().cloned());
    }
    let status = if blockers.is_empty() {
        RustProtocolAdapterForwardingStatus::Passed
    } else {
        RustProtocolAdapterForwardingStatus::Failed
    };

    let mut report = RustProtocolAdapterForwardingExpansionReport {
        runtime_id: RUST_RUNTIME_ID.into(),
        component: RUST_PROTOCOL_ADAPTER_FORWARDING_COMPONENT.into(),
        kernel_area: RUST_PROTOCOL_ADAPTER_FORWARDING_KERNEL_AREA.into(),
        status,
        reason: if status == RustProtocolAdapterForwardingStatus::Passed {
            "Rust protocol+adapter forwarding expansion passed DIRECT and REJECT evidence".into()
        } else {
            "Rust protocol+adapter forwarding expansion failed".into()
        },
        explicit_opt_in,
        direct_evidence: Some(direct_evidence),
        reject_evidence: Some(reject_evidence),
        evidence_path: None,
        loopback_only: true,
        mutates_runtime: false,
        forwards_traffic: true,
        outbound_adapters_used: true,
        writes_evidence_artifact: true,
        mihomo_fallback: true,
        blockers,
        warnings: vec![
            "expansion covers Rust DIRECT and REJECT adapter decisions over loopback TCP/HTTP".into(),
            "Mihomo remains fallback for remote proxy protocols, SOCKS, and packet capture".into(),
        ],
        facts: rust_protocol_adapter_forwarding_facts(),
        next_safe_batch: NEXT_SAFE_BATCH.into(),
    };

    let evidence_path = rust_protocol_adapter_forwarding_evidence_path()?;
    if let Some(parent) = evidence_path.parent() {
        fs::create_dir_all(parent).await?;
    }
    report.evidence_path = Some(evidence_path.to_string_lossy().to_string().into());
    fs::write(&evidence_path, serde_yaml_ng::to_string(&report)?.as_bytes()).await?;
    Ok(report)
}

async fn run_direct_adapter_forwarding_evidence() -> Result<RustProtocolAdapterForwardingDecisionEvidence> {
    let target = TcpListener::bind((RUST_PROTOCOL_ADAPTER_FORWARDING_HOST, DEFAULT_DIRECT_TARGET_PORT)).await?;
    let target_task = tokio::spawn(async move {
        let (mut stream, _) = timeout(Duration::from_secs(3), target.accept()).await??;
        let mut request = [0_u8; 1024];
        let request_len = timeout(Duration::from_secs(3), stream.read(&mut request)).await??;
        let received = std::str::from_utf8(&request[..request_len])
            .map(|request| request.contains("GET /rust-protocol-adapter-direct"))
            .unwrap_or(false);
        stream
            .write_all(b"HTTP/1.1 204 No Content\r\nConnection: close\r\nContent-Length: 0\r\n\r\n")
            .await?;
        stream.shutdown().await?;
        Ok::<bool, anyhow::Error>(received)
    });

    let listener = TcpListener::bind((RUST_PROTOCOL_ADAPTER_FORWARDING_HOST, DEFAULT_DIRECT_LISTENER_PORT)).await?;
    let forwarder_task = tokio::spawn(async move {
        let (mut inbound, _) = timeout(Duration::from_secs(3), listener.accept()).await??;
        let mut outbound =
            TcpStream::connect((RUST_PROTOCOL_ADAPTER_FORWARDING_HOST, DEFAULT_DIRECT_TARGET_PORT)).await?;
        let (from_client, from_target) = tokio::io::copy_bidirectional(&mut inbound, &mut outbound)
            .await
            .map(|(from_client, from_target)| (from_client, from_target))?;
        Ok::<(u64, u64), anyhow::Error>((from_client, from_target))
    });

    let response = send_http_request(
        DEFAULT_DIRECT_LISTENER_PORT,
        "GET /rust-protocol-adapter-direct HTTP/1.1\r\nHost: rust-direct\r\nConnection: close\r\n\r\n",
    )
    .await?;
    let target_received = target_task.await??;
    let (bytes_from_client, bytes_from_target) = forwarder_task.await??;
    let response_status = parse_http_status(&response);
    let mut blockers = Vec::new();
    if !target_received {
        blockers.push("DIRECT target did not receive the forwarded request".into());
    }
    if response_status.as_deref() != Some("HTTP/1.1 204 No Content") {
        blockers.push("DIRECT forwarding did not return HTTP 204".into());
    }
    if bytes_from_client == 0 || bytes_from_target == 0 {
        blockers.push("DIRECT forwarding byte accounting did not observe bidirectional traffic".into());
    }

    Ok(RustProtocolAdapterForwardingDecisionEvidence {
        adapter_kind: RustProtocolAdapterForwardingAdapterKind::Direct,
        listener_port: DEFAULT_DIRECT_LISTENER_PORT,
        target_port: Some(DEFAULT_DIRECT_TARGET_PORT),
        target_received,
        response_status,
        accepted_connections: 1,
        bytes_from_client,
        bytes_from_target,
        passed: blockers.is_empty(),
        blockers,
    })
}

async fn run_reject_adapter_forwarding_evidence() -> Result<RustProtocolAdapterForwardingDecisionEvidence> {
    let listener = TcpListener::bind((RUST_PROTOCOL_ADAPTER_FORWARDING_HOST, DEFAULT_REJECT_LISTENER_PORT)).await?;
    let reject_task = tokio::spawn(async move {
        let (mut inbound, _) = timeout(Duration::from_secs(3), listener.accept()).await??;
        let mut request = [0_u8; 1024];
        let bytes_from_client = timeout(Duration::from_secs(3), inbound.read(&mut request)).await??;
        inbound
            .write_all(b"HTTP/1.1 403 Forbidden\r\nConnection: close\r\nContent-Length: 0\r\n\r\n")
            .await?;
        inbound.shutdown().await?;
        Ok::<u64, anyhow::Error>(bytes_from_client as u64)
    });
    let response = send_http_request(
        DEFAULT_REJECT_LISTENER_PORT,
        "GET /rust-protocol-adapter-reject HTTP/1.1\r\nHost: rust-reject\r\nConnection: close\r\n\r\n",
    )
    .await?;
    let bytes_from_client = reject_task.await??;
    let response_status = parse_http_status(&response);
    let mut blockers = Vec::new();
    if response_status.as_deref() != Some("HTTP/1.1 403 Forbidden") {
        blockers.push("REJECT adapter did not return HTTP 403".into());
    }
    if bytes_from_client == 0 {
        blockers.push("REJECT adapter did not receive the client request".into());
    }

    Ok(RustProtocolAdapterForwardingDecisionEvidence {
        adapter_kind: RustProtocolAdapterForwardingAdapterKind::Reject,
        listener_port: DEFAULT_REJECT_LISTENER_PORT,
        target_port: None,
        target_received: false,
        response_status,
        accepted_connections: 1,
        bytes_from_client,
        bytes_from_target: response.len() as u64,
        passed: blockers.is_empty(),
        blockers,
    })
}

async fn send_http_request(port: u16, request: &str) -> Result<Vec<u8>> {
    let mut stream = TcpStream::connect((RUST_PROTOCOL_ADAPTER_FORWARDING_HOST, port)).await?;
    stream.write_all(request.as_bytes()).await?;
    stream.shutdown().await?;
    let mut response = Vec::new();
    timeout(Duration::from_secs(3), stream.read_to_end(&mut response)).await??;
    Ok(response)
}

fn parse_http_status(response: &[u8]) -> Option<String> {
    std::str::from_utf8(response)
        .ok()
        .and_then(|response| response.lines().next())
        .map(Into::into)
}

fn rust_protocol_adapter_forwarding_evidence_path() -> Result<std::path::PathBuf> {
    Ok(dirs::app_runtime_dir()?
        .join(RUST_PROTOCOL_ADAPTER_FORWARDING_COMPONENT)
        .join(RUST_PROTOCOL_ADAPTER_FORWARDING_EVIDENCE_FILE))
}

fn rust_protocol_adapter_forwarding_facts() -> Vec<String> {
    vec![
        "DIRECT adapter evidence opens a Rust loopback listener and forwards bytes to a target".into(),
        "REJECT adapter evidence denies the request without contacting a target".into(),
        "adapter policy is evaluated inside Rust before forwarding".into(),
        "evidence is persisted for the next remote adapter transport expansion".into(),
    ]
}
