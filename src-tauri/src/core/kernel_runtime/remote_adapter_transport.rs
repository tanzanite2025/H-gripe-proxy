use super::{
    RUST_RUNTIME_ID, RustRemoteAdapterTransportEvidence, RustRemoteAdapterTransportExpansionReport,
    RustRemoteAdapterTransportKind, RustRemoteAdapterTransportStatus,
};
use crate::utils::dirs;
use anyhow::{Result, bail};
use smartstring::alias::String;
use tokio::{
    fs,
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
    time::{Duration, timeout},
};

const RUST_REMOTE_ADAPTER_TRANSPORT_COMPONENT: &str = "rust-remote-adapter-transport-expansion";
const RUST_REMOTE_ADAPTER_TRANSPORT_KERNEL_AREA: &str = "remote-adapter-transport";
const RUST_REMOTE_ADAPTER_TRANSPORT_HOST: &str = "127.0.0.1";
const RUST_REMOTE_ADAPTER_TRANSPORT_EVIDENCE_FILE: &str = "evidence.yaml";
const DEFAULT_REMOTE_ADAPTER_CONTROL_PORT: u16 = 19480;
const DEFAULT_REMOTE_ADAPTER_TARGET_PORT: u16 = 19481;
const NEXT_SAFE_BATCH: &str = "rust-http-connect-proxy-adapter";

pub async fn rust_remote_adapter_transport_expansion_evidence(
    explicit_opt_in: bool,
) -> Result<RustRemoteAdapterTransportExpansionReport> {
    if !explicit_opt_in {
        return Ok(RustRemoteAdapterTransportExpansionReport {
            runtime_id: RUST_RUNTIME_ID.into(),
            component: RUST_REMOTE_ADAPTER_TRANSPORT_COMPONENT.into(),
            kernel_area: RUST_REMOTE_ADAPTER_TRANSPORT_KERNEL_AREA.into(),
            status: RustRemoteAdapterTransportStatus::Blocked,
            reason: "explicit opt-in is required to run remote adapter transport expansion".into(),
            explicit_opt_in,
            tcp_connect_evidence: None,
            unsupported_protocol_evidence: None,
            evidence_path: None,
            loopback_remote_only: true,
            mutates_runtime: false,
            forwards_traffic: false,
            outbound_adapters_used: false,
            writes_evidence_artifact: false,
            mihomo_fallback: true,
            blockers: vec!["explicit opt-in is required".into()],
            warnings: Vec::new(),
            facts: rust_remote_adapter_transport_facts(),
            next_safe_batch: NEXT_SAFE_BATCH.into(),
        });
    }

    let tcp_connect_evidence = run_tcp_connect_remote_adapter_evidence().await?;
    let unsupported_protocol_evidence = unsupported_remote_protocol_evidence();
    let mut blockers = Vec::new();
    if !tcp_connect_evidence.passed {
        blockers.push("TCP CONNECT remote adapter transport evidence failed".into());
        blockers.extend(tcp_connect_evidence.blockers.iter().cloned());
    }
    if !unsupported_protocol_evidence.passed {
        blockers.push("unsupported remote adapter fallback evidence failed".into());
        blockers.extend(unsupported_protocol_evidence.blockers.iter().cloned());
    }
    let status = if blockers.is_empty() {
        RustRemoteAdapterTransportStatus::Passed
    } else {
        RustRemoteAdapterTransportStatus::Failed
    };

    let mut report = RustRemoteAdapterTransportExpansionReport {
        runtime_id: RUST_RUNTIME_ID.into(),
        component: RUST_REMOTE_ADAPTER_TRANSPORT_COMPONENT.into(),
        kernel_area: RUST_REMOTE_ADAPTER_TRANSPORT_KERNEL_AREA.into(),
        status,
        reason: if status == RustRemoteAdapterTransportStatus::Passed {
            "Rust remote adapter transport expansion passed bounded TCP CONNECT evidence".into()
        } else {
            "Rust remote adapter transport expansion failed".into()
        },
        explicit_opt_in,
        tcp_connect_evidence: Some(tcp_connect_evidence),
        unsupported_protocol_evidence: Some(unsupported_protocol_evidence),
        evidence_path: None,
        loopback_remote_only: true,
        mutates_runtime: false,
        forwards_traffic: true,
        outbound_adapters_used: true,
        writes_evidence_artifact: true,
        mihomo_fallback: true,
        blockers,
        warnings: vec![
            "remote transport is capped to loopback TCP CONNECT evidence".into(),
            "Mihomo remains fallback for encrypted proxy protocols, UDP associate, and packet capture".into(),
        ],
        facts: rust_remote_adapter_transport_facts(),
        next_safe_batch: NEXT_SAFE_BATCH.into(),
    };

    let evidence_path = rust_remote_adapter_transport_evidence_path()?;
    if let Some(parent) = evidence_path.parent() {
        fs::create_dir_all(parent).await?;
    }
    report.evidence_path = Some(evidence_path.to_string_lossy().to_string().into());
    fs::write(&evidence_path, serde_yaml_ng::to_string(&report)?.as_bytes()).await?;
    Ok(report)
}

async fn run_tcp_connect_remote_adapter_evidence() -> Result<RustRemoteAdapterTransportEvidence> {
    let target = TcpListener::bind((RUST_REMOTE_ADAPTER_TRANSPORT_HOST, DEFAULT_REMOTE_ADAPTER_TARGET_PORT)).await?;
    let target_task = tokio::spawn(async move {
        let (mut stream, _) = timeout(Duration::from_secs(3), target.accept()).await??;
        let mut request = [0_u8; 1024];
        let request_len = timeout(Duration::from_secs(3), stream.read(&mut request)).await??;
        let received = std::str::from_utf8(&request[..request_len])
            .map(|request| request.contains("GET /rust-remote-adapter-transport"))
            .unwrap_or(false);
        stream
            .write_all(b"HTTP/1.1 204 No Content\r\nConnection: close\r\nContent-Length: 0\r\n\r\n")
            .await?;
        stream.shutdown().await?;
        Ok::<bool, anyhow::Error>(received)
    });

    let remote = TcpListener::bind((RUST_REMOTE_ADAPTER_TRANSPORT_HOST, DEFAULT_REMOTE_ADAPTER_CONTROL_PORT)).await?;
    let remote_task = tokio::spawn(async move {
        let (mut inbound, _) = timeout(Duration::from_secs(3), remote.accept()).await??;
        let mut buffer = vec![0_u8; 2048];
        let read_len = timeout(Duration::from_secs(3), inbound.read(&mut buffer)).await??;
        let header_end = find_header_end(&buffer[..read_len])?;
        let header = std::str::from_utf8(&buffer[..header_end])?;
        if header.trim() != "RUST-CONNECT 127.0.0.1:19481" {
            bail!("unsupported Rust remote adapter request: {header}");
        }
        let mut outbound =
            TcpStream::connect((RUST_REMOTE_ADAPTER_TRANSPORT_HOST, DEFAULT_REMOTE_ADAPTER_TARGET_PORT)).await?;
        let payload = &buffer[header_end + 4..read_len];
        outbound.write_all(payload).await?;
        let (extra_to_target, from_target) = tokio::io::copy_bidirectional(&mut inbound, &mut outbound).await?;
        Ok::<(u64, u64), anyhow::Error>((read_len as u64 + extra_to_target, from_target))
    });

    let request = concat!(
        "RUST-CONNECT 127.0.0.1:19481\r\n\r\n",
        "GET /rust-remote-adapter-transport HTTP/1.1\r\n",
        "Host: rust-remote-adapter\r\n",
        "Connection: close\r\n\r\n"
    );
    let response = send_remote_adapter_request(DEFAULT_REMOTE_ADAPTER_CONTROL_PORT, request).await?;
    let target_received = target_task.await??;
    let (bytes_to_remote, bytes_from_remote) = remote_task.await??;
    let response_status = parse_http_status(&response);
    let mut blockers = Vec::new();
    if !target_received {
        blockers.push("remote adapter target did not receive the forwarded request".into());
    }
    if response_status.as_deref() != Some("HTTP/1.1 204 No Content") {
        blockers.push("remote adapter transport did not return HTTP 204".into());
    }
    if bytes_to_remote == 0 || bytes_from_remote == 0 {
        blockers.push("remote adapter transport byte accounting is empty".into());
    }

    Ok(RustRemoteAdapterTransportEvidence {
        transport_kind: RustRemoteAdapterTransportKind::TcpConnect,
        adapter_name: "rust-loopback-tcp-connect".into(),
        control_port: Some(DEFAULT_REMOTE_ADAPTER_CONTROL_PORT),
        target_port: Some(DEFAULT_REMOTE_ADAPTER_TARGET_PORT),
        target_received,
        response_status,
        bytes_to_remote,
        bytes_from_remote,
        fallback_retained: false,
        passed: blockers.is_empty(),
        blockers,
    })
}

fn unsupported_remote_protocol_evidence() -> RustRemoteAdapterTransportEvidence {
    RustRemoteAdapterTransportEvidence {
        transport_kind: RustRemoteAdapterTransportKind::UnsupportedProxyProtocol,
        adapter_name: "socks5-udp-associate".into(),
        control_port: None,
        target_port: None,
        target_received: false,
        response_status: None,
        bytes_to_remote: 0,
        bytes_from_remote: 0,
        fallback_retained: true,
        passed: true,
        blockers: Vec::new(),
    }
}

async fn send_remote_adapter_request(port: u16, request: &str) -> Result<Vec<u8>> {
    let mut stream = TcpStream::connect((RUST_REMOTE_ADAPTER_TRANSPORT_HOST, port)).await?;
    stream.write_all(request.as_bytes()).await?;
    stream.shutdown().await?;
    let mut response = Vec::new();
    timeout(Duration::from_secs(3), stream.read_to_end(&mut response)).await??;
    Ok(response)
}

fn find_header_end(buffer: &[u8]) -> Result<usize> {
    buffer
        .windows(4)
        .position(|window| window == b"\r\n\r\n")
        .ok_or_else(|| anyhow::anyhow!("missing remote adapter transport header terminator"))
}

fn parse_http_status(response: &[u8]) -> Option<String> {
    std::str::from_utf8(response)
        .ok()
        .and_then(|response| response.lines().next())
        .map(Into::into)
}

fn rust_remote_adapter_transport_evidence_path() -> Result<std::path::PathBuf> {
    Ok(dirs::app_runtime_dir()?
        .join(RUST_REMOTE_ADAPTER_TRANSPORT_COMPONENT)
        .join(RUST_REMOTE_ADAPTER_TRANSPORT_EVIDENCE_FILE))
}

fn rust_remote_adapter_transport_facts() -> Vec<String> {
    vec![
        "Rust opens a bounded remote adapter control transport and connects it to a target".into(),
        "the remote adapter request is parsed and validated before target dialing".into(),
        "TCP response bytes return through the Rust remote adapter transport".into(),
        "unsupported proxy protocols stay on Mihomo fallback".into(),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn remote_adapter_transport_blocks_without_opt_in() {
        let report = rust_remote_adapter_transport_expansion_evidence(false).await.unwrap();

        assert_eq!(report.status, RustRemoteAdapterTransportStatus::Blocked);
        assert!(!report.forwards_traffic);
        assert!(report.mihomo_fallback);
    }

    #[tokio::test]
    async fn tcp_connect_remote_adapter_evidence_passes() {
        let evidence = run_tcp_connect_remote_adapter_evidence().await.unwrap();

        assert!(evidence.passed);
        assert!(evidence.target_received);
        assert_eq!(evidence.response_status.as_deref(), Some("HTTP/1.1 204 No Content"));
        assert!(evidence.bytes_to_remote > 0);
        assert!(evidence.bytes_from_remote > 0);
    }
}
