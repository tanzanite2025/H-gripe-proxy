use super::{
    RUST_RUNTIME_ID, RustHttpConnectProxyAdapterEvidence, RustHttpConnectProxyAdapterReport,
    RustHttpConnectProxyAdapterStatus,
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

const RUST_HTTP_CONNECT_PROXY_ADAPTER_COMPONENT: &str = "rust-http-connect-proxy-adapter";
const RUST_HTTP_CONNECT_PROXY_ADAPTER_KERNEL_AREA: &str = "http-connect-proxy-adapter";
const RUST_HTTP_CONNECT_PROXY_ADAPTER_HOST: &str = "127.0.0.1";
const RUST_HTTP_CONNECT_PROXY_ADAPTER_EVIDENCE_FILE: &str = "evidence.yaml";
const DEFAULT_HTTP_CONNECT_PROXY_LISTENER_PORT: u16 = 19580;
const DEFAULT_HTTP_CONNECT_PROXY_TARGET_PORT: u16 = 19581;
const NEXT_SAFE_BATCH: &str = "rust-encrypted-proxy-protocol-preflight";

pub async fn rust_http_connect_proxy_adapter_evidence(
    explicit_opt_in: bool,
) -> Result<RustHttpConnectProxyAdapterReport> {
    if !explicit_opt_in {
        return Ok(RustHttpConnectProxyAdapterReport {
            runtime_id: RUST_RUNTIME_ID.into(),
            component: RUST_HTTP_CONNECT_PROXY_ADAPTER_COMPONENT.into(),
            kernel_area: RUST_HTTP_CONNECT_PROXY_ADAPTER_KERNEL_AREA.into(),
            status: RustHttpConnectProxyAdapterStatus::Blocked,
            reason: "explicit opt-in is required to run HTTP CONNECT proxy adapter evidence".into(),
            explicit_opt_in,
            connect_evidence: None,
            unsupported_protocols: unsupported_http_connect_proxy_adapter_protocols(),
            evidence_path: None,
            loopback_remote_only: true,
            mutates_runtime: false,
            forwards_traffic: false,
            outbound_adapters_used: false,
            writes_evidence_artifact: false,
            mihomo_fallback: true,
            blockers: vec!["explicit opt-in is required".into()],
            warnings: Vec::new(),
            facts: rust_http_connect_proxy_adapter_facts(),
            next_safe_batch: NEXT_SAFE_BATCH.into(),
        });
    }

    let connect_evidence = run_http_connect_proxy_adapter_evidence().await?;
    let blockers = if connect_evidence.passed {
        Vec::new()
    } else {
        let mut blockers = vec!["HTTP CONNECT proxy adapter evidence failed".into()];
        blockers.extend(connect_evidence.blockers.iter().cloned());
        blockers
    };
    let status = if blockers.is_empty() {
        RustHttpConnectProxyAdapterStatus::Passed
    } else {
        RustHttpConnectProxyAdapterStatus::Failed
    };
    let mut report = RustHttpConnectProxyAdapterReport {
        runtime_id: RUST_RUNTIME_ID.into(),
        component: RUST_HTTP_CONNECT_PROXY_ADAPTER_COMPONENT.into(),
        kernel_area: RUST_HTTP_CONNECT_PROXY_ADAPTER_KERNEL_AREA.into(),
        status,
        reason: if status == RustHttpConnectProxyAdapterStatus::Passed {
            "Rust HTTP CONNECT proxy adapter established a tunnel and forwarded target bytes".into()
        } else {
            "Rust HTTP CONNECT proxy adapter evidence failed".into()
        },
        explicit_opt_in,
        connect_evidence: Some(connect_evidence),
        unsupported_protocols: unsupported_http_connect_proxy_adapter_protocols(),
        evidence_path: None,
        loopback_remote_only: true,
        mutates_runtime: false,
        forwards_traffic: true,
        outbound_adapters_used: true,
        writes_evidence_artifact: true,
        mihomo_fallback: true,
        blockers,
        warnings: vec![
            "HTTP CONNECT adapter is capped to loopback TCP targets".into(),
            "Mihomo remains fallback for TLS outbound protocols, SOCKS UDP, VMess/VLESS/Trojan/Shadowsocks, and packet capture".into(),
        ],
        facts: rust_http_connect_proxy_adapter_facts(),
        next_safe_batch: NEXT_SAFE_BATCH.into(),
    };

    let evidence_path = rust_http_connect_proxy_adapter_evidence_path()?;
    if let Some(parent) = evidence_path.parent() {
        fs::create_dir_all(parent).await?;
    }
    report.evidence_path = Some(evidence_path.to_string_lossy().to_string().into());
    fs::write(&evidence_path, serde_yaml_ng::to_string(&report)?.as_bytes()).await?;
    Ok(report)
}

async fn run_http_connect_proxy_adapter_evidence() -> Result<RustHttpConnectProxyAdapterEvidence> {
    let connect_authority = format!("{RUST_HTTP_CONNECT_PROXY_ADAPTER_HOST}:{DEFAULT_HTTP_CONNECT_PROXY_TARGET_PORT}");
    let target = TcpListener::bind((
        RUST_HTTP_CONNECT_PROXY_ADAPTER_HOST,
        DEFAULT_HTTP_CONNECT_PROXY_TARGET_PORT,
    ))
    .await?;
    let target_task = tokio::spawn(async move {
        let (mut stream, _) = timeout(Duration::from_secs(3), target.accept()).await??;
        let mut request = [0_u8; 1024];
        let request_len = timeout(Duration::from_secs(3), stream.read(&mut request)).await??;
        let target_received = std::str::from_utf8(&request[..request_len])
            .map(|request| request.contains("GET /rust-http-connect-proxy-adapter"))
            .unwrap_or(false);
        stream
            .write_all(b"HTTP/1.1 204 No Content\r\nConnection: close\r\nContent-Length: 0\r\n\r\n")
            .await?;
        stream.shutdown().await?;
        Ok::<bool, anyhow::Error>(target_received)
    });

    let listener = TcpListener::bind((
        RUST_HTTP_CONNECT_PROXY_ADAPTER_HOST,
        DEFAULT_HTTP_CONNECT_PROXY_LISTENER_PORT,
    ))
    .await?;
    let proxy_task = tokio::spawn({
        let connect_authority = connect_authority.clone();
        async move {
            let (mut inbound, _) = timeout(Duration::from_secs(3), listener.accept()).await??;
            let request = read_http_connect_request(&mut inbound).await?;
            if !valid_connect_request(&request, &connect_authority) {
                bail!("unsupported HTTP CONNECT request: {request}");
            }
            let mut outbound = TcpStream::connect((
                RUST_HTTP_CONNECT_PROXY_ADAPTER_HOST,
                DEFAULT_HTTP_CONNECT_PROXY_TARGET_PORT,
            ))
            .await?;
            inbound
                .write_all(b"HTTP/1.1 200 Connection Established\r\nProxy-Agent: RustConnect\r\n\r\n")
                .await?;
            let (bytes_from_client, bytes_from_target) =
                tokio::io::copy_bidirectional(&mut inbound, &mut outbound).await?;
            Ok::<(u64, u64), anyhow::Error>((bytes_from_client, bytes_from_target))
        }
    });

    let tunnel_response = run_http_connect_client(DEFAULT_HTTP_CONNECT_PROXY_LISTENER_PORT, &connect_authority).await?;
    let target_received = target_task.await??;
    let (bytes_from_client, bytes_from_target) = proxy_task.await??;
    let connect_established = tunnel_response.connect_established;
    let response_status = tunnel_response.response_status;
    let mut blockers = Vec::new();
    if !connect_established {
        blockers.push("HTTP CONNECT tunnel was not established".into());
    }
    if !target_received {
        blockers.push("HTTP CONNECT target did not receive tunneled request".into());
    }
    if response_status.as_deref() != Some("HTTP/1.1 204 No Content") {
        blockers.push("HTTP CONNECT tunnel did not return target HTTP 204".into());
    }
    if bytes_from_client == 0 || bytes_from_target == 0 {
        blockers.push("HTTP CONNECT byte accounting did not observe bidirectional traffic".into());
    }

    Ok(RustHttpConnectProxyAdapterEvidence {
        adapter_name: "rust-http-connect-loopback".into(),
        listener_port: DEFAULT_HTTP_CONNECT_PROXY_LISTENER_PORT,
        target_port: DEFAULT_HTTP_CONNECT_PROXY_TARGET_PORT,
        connect_authority: connect_authority.into(),
        connect_established,
        target_received,
        response_status,
        bytes_from_client,
        bytes_from_target,
        passed: blockers.is_empty(),
        blockers,
    })
}

struct HttpConnectClientResult {
    connect_established: bool,
    response_status: Option<String>,
}

async fn run_http_connect_client(port: u16, connect_authority: &str) -> Result<HttpConnectClientResult> {
    let mut stream = TcpStream::connect((RUST_HTTP_CONNECT_PROXY_ADAPTER_HOST, port)).await?;
    let connect_request = format!("CONNECT {connect_authority} HTTP/1.1\r\nHost: {connect_authority}\r\n\r\n");
    stream.write_all(connect_request.as_bytes()).await?;
    let connect_response = read_until_header_end(&mut stream).await?;
    let connect_established =
        parse_http_status(&connect_response).as_deref() == Some("HTTP/1.1 200 Connection Established");
    if !connect_established {
        return Ok(HttpConnectClientResult {
            connect_established,
            response_status: parse_http_status(&connect_response),
        });
    }
    stream
        .write_all(
            b"GET /rust-http-connect-proxy-adapter HTTP/1.1\r\nHost: rust-connect-target\r\nConnection: close\r\n\r\n",
        )
        .await?;
    stream.shutdown().await?;
    let mut response = Vec::new();
    timeout(Duration::from_secs(3), stream.read_to_end(&mut response)).await??;
    Ok(HttpConnectClientResult {
        connect_established,
        response_status: parse_http_status(&response),
    })
}

async fn read_http_connect_request(stream: &mut TcpStream) -> Result<String> {
    let header = read_until_header_end(stream).await?;
    Ok(std::str::from_utf8(&header)?.to_owned().into())
}

async fn read_until_header_end(stream: &mut TcpStream) -> Result<Vec<u8>> {
    let mut buffer = Vec::new();
    let mut byte = [0_u8; 1];
    while !buffer.ends_with(b"\r\n\r\n") {
        let read = timeout(Duration::from_secs(3), stream.read(&mut byte)).await??;
        if read == 0 {
            bail!("connection closed before HTTP headers completed");
        }
        buffer.push(byte[0]);
        if buffer.len() > 4096 {
            bail!("HTTP header exceeded 4096 bytes");
        }
    }
    Ok(buffer)
}

fn valid_connect_request(request: &str, connect_authority: &str) -> bool {
    let mut lines = request.lines();
    let Some(request_line) = lines.next() else {
        return false;
    };
    let Some(host_line) = lines.find(|line| line.to_ascii_lowercase().starts_with("host:")) else {
        return false;
    };
    request_line == format!("CONNECT {connect_authority} HTTP/1.1")
        && host_line
            .split_once(':')
            .map(|(_, authority)| authority.trim() == connect_authority)
            .unwrap_or(false)
}

fn parse_http_status(response: &[u8]) -> Option<String> {
    std::str::from_utf8(response)
        .ok()
        .and_then(|response| response.lines().next())
        .map(Into::into)
}

fn rust_http_connect_proxy_adapter_evidence_path() -> Result<std::path::PathBuf> {
    Ok(dirs::app_runtime_dir()?
        .join(RUST_HTTP_CONNECT_PROXY_ADAPTER_COMPONENT)
        .join(RUST_HTTP_CONNECT_PROXY_ADAPTER_EVIDENCE_FILE))
}

fn unsupported_http_connect_proxy_adapter_protocols() -> Vec<String> {
    vec![
        "SOCKS UDP ASSOCIATE".into(),
        "VMess".into(),
        "VLESS".into(),
        "Trojan".into(),
        "Shadowsocks".into(),
        "system-wide packet capture".into(),
    ]
}

fn rust_http_connect_proxy_adapter_facts() -> Vec<String> {
    vec![
        "Rust accepts an HTTP CONNECT request and validates the authority".into(),
        "Rust establishes the target TCP stream only after CONNECT validation".into(),
        "HTTP bytes are tunneled bidirectionally through the Rust adapter".into(),
        "unsupported encrypted proxy protocols remain on Mihomo fallback".into(),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn http_connect_proxy_adapter_blocks_without_opt_in() {
        let report = rust_http_connect_proxy_adapter_evidence(false).await.unwrap();

        assert_eq!(report.status, RustHttpConnectProxyAdapterStatus::Blocked);
        assert!(!report.forwards_traffic);
        assert!(report.mihomo_fallback);
    }

    #[test]
    fn validates_connect_authority_and_host_header() {
        assert!(valid_connect_request(
            "CONNECT 127.0.0.1:19581 HTTP/1.1\r\nHost: 127.0.0.1:19581\r\n\r\n",
            "127.0.0.1:19581",
        ));
        assert!(!valid_connect_request(
            "CONNECT 127.0.0.1:19581 HTTP/1.1\r\nHost: example.invalid:443\r\n\r\n",
            "127.0.0.1:19581",
        ));
    }
}
