use super::{
    RUST_RUNTIME_ID, RustShadowsocksAeadAdapterExecutionEvidence, RustShadowsocksAeadAdapterExecutionReport,
    RustShadowsocksAeadAdapterExecutionStatus,
};
use crate::utils::dirs;
use aes_gcm::{
    Aes256Gcm, Nonce,
    aead::{Aead, KeyInit},
};
use anyhow::{Result, bail};
use serde::Serialize;
use sha2::{Digest, Sha256};
use smartstring::alias::String;
use tokio::{
    fs,
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
    time::{Duration, timeout},
};

const RUST_SHADOWSOCKS_AEAD_ADAPTER_COMPONENT: &str = "rust-shadowsocks-aead-adapter-execution";
const RUST_SHADOWSOCKS_AEAD_ADAPTER_KERNEL_AREA: &str = "shadowsocks-aead-adapter";
const RUST_SHADOWSOCKS_AEAD_ADAPTER_HOST: &str = "127.0.0.1";
const RUST_SHADOWSOCKS_AEAD_ADAPTER_EVIDENCE_FILE: &str = "evidence.yaml";
const RUST_SHADOWSOCKS_AEAD_ADAPTER_ROLLBACK_FILE: &str = "rollback-checkpoint.yaml";
const DEFAULT_SHADOWSOCKS_AEAD_ADAPTER_LISTENER_PORT: u16 = 19780;
const DEFAULT_SHADOWSOCKS_AEAD_ADAPTER_TARGET_PORT: u16 = 19781;
const NEXT_SAFE_BATCH: &str = "rust-shadowsocks-aead-adapter-canary";

pub async fn rust_shadowsocks_aead_adapter_execution(
    explicit_opt_in: bool,
) -> Result<RustShadowsocksAeadAdapterExecutionReport> {
    if !explicit_opt_in {
        return Ok(RustShadowsocksAeadAdapterExecutionReport {
            runtime_id: RUST_RUNTIME_ID.into(),
            component: RUST_SHADOWSOCKS_AEAD_ADAPTER_COMPONENT.into(),
            kernel_area: RUST_SHADOWSOCKS_AEAD_ADAPTER_KERNEL_AREA.into(),
            status: RustShadowsocksAeadAdapterExecutionStatus::Blocked,
            reason: "explicit opt-in is required to run Shadowsocks AEAD adapter execution".into(),
            explicit_opt_in,
            execution_evidence: None,
            unsupported_protocols: unsupported_shadowsocks_aead_adapter_protocols(),
            evidence_path: None,
            rollback_checkpoint_path: None,
            loopback_remote_only: true,
            mutates_runtime: false,
            forwards_traffic: false,
            outbound_adapters_used: false,
            writes_evidence_artifact: false,
            mihomo_fallback: true,
            blockers: vec!["explicit opt-in is required".into()],
            warnings: Vec::new(),
            facts: rust_shadowsocks_aead_adapter_facts(),
            next_safe_batch: NEXT_SAFE_BATCH.into(),
        });
    }

    let rollback_checkpoint_path = rust_shadowsocks_aead_adapter_rollback_path()?;
    write_rollback_checkpoint(&rollback_checkpoint_path).await?;
    let rollback_checkpoint_path_string: String = rollback_checkpoint_path.to_string_lossy().to_string().into();
    let execution_evidence = run_shadowsocks_aead_adapter_execution(&rollback_checkpoint_path_string).await?;
    let blockers = if execution_evidence.passed {
        Vec::new()
    } else {
        let mut blockers = vec!["Shadowsocks AEAD adapter execution evidence failed".into()];
        blockers.extend(execution_evidence.blockers.iter().cloned());
        blockers
    };
    let status = if blockers.is_empty() {
        RustShadowsocksAeadAdapterExecutionStatus::Passed
    } else {
        RustShadowsocksAeadAdapterExecutionStatus::Failed
    };
    let mut report = RustShadowsocksAeadAdapterExecutionReport {
        runtime_id: RUST_RUNTIME_ID.into(),
        component: RUST_SHADOWSOCKS_AEAD_ADAPTER_COMPONENT.into(),
        kernel_area: RUST_SHADOWSOCKS_AEAD_ADAPTER_KERNEL_AREA.into(),
        status,
        reason: if status == RustShadowsocksAeadAdapterExecutionStatus::Passed {
            "Rust Shadowsocks AEAD adapter executed a scoped encrypted forwarding path".into()
        } else {
            "Rust Shadowsocks AEAD adapter execution failed".into()
        },
        explicit_opt_in,
        execution_evidence: Some(execution_evidence),
        unsupported_protocols: unsupported_shadowsocks_aead_adapter_protocols(),
        evidence_path: None,
        rollback_checkpoint_path: Some(rollback_checkpoint_path_string),
        loopback_remote_only: true,
        mutates_runtime: false,
        forwards_traffic: true,
        outbound_adapters_used: true,
        writes_evidence_artifact: true,
        mihomo_fallback: true,
        blockers,
        warnings: vec![
            "Shadowsocks AEAD execution is capped to loopback TCP address frames".into(),
            "Mihomo remains fallback for UDP associate, VMess/VLESS/Trojan, plugin transports, and packet capture"
                .into(),
        ],
        facts: rust_shadowsocks_aead_adapter_facts(),
        next_safe_batch: NEXT_SAFE_BATCH.into(),
    };

    let evidence_path = rust_shadowsocks_aead_adapter_evidence_path()?;
    if let Some(parent) = evidence_path.parent() {
        fs::create_dir_all(parent).await?;
    }
    report.evidence_path = Some(evidence_path.to_string_lossy().to_string().into());
    fs::write(&evidence_path, serde_yaml_ng::to_string(&report)?.as_bytes()).await?;
    Ok(report)
}

async fn run_shadowsocks_aead_adapter_execution(
    rollback_checkpoint_path: &str,
) -> Result<RustShadowsocksAeadAdapterExecutionEvidence> {
    let target = TcpListener::bind((
        RUST_SHADOWSOCKS_AEAD_ADAPTER_HOST,
        DEFAULT_SHADOWSOCKS_AEAD_ADAPTER_TARGET_PORT,
    ))
    .await?;
    let target_task = tokio::spawn(async move {
        let (mut stream, _) = timeout(Duration::from_secs(3), target.accept()).await??;
        let mut request = Vec::new();
        timeout(Duration::from_secs(3), stream.read_to_end(&mut request)).await??;
        let target_received = std::str::from_utf8(&request)
            .map(|request| request.contains("GET /rust-shadowsocks-aead-adapter-execution"))
            .unwrap_or(false);
        stream
            .write_all(b"HTTP/1.1 204 No Content\r\nConnection: close\r\nContent-Length: 0\r\n\r\n")
            .await?;
        stream.shutdown().await?;
        Ok::<bool, anyhow::Error>(target_received)
    });

    let listener = TcpListener::bind((
        RUST_SHADOWSOCKS_AEAD_ADAPTER_HOST,
        DEFAULT_SHADOWSOCKS_AEAD_ADAPTER_LISTENER_PORT,
    ))
    .await?;
    let adapter_task = tokio::spawn(async move {
        let (mut inbound, _) = timeout(Duration::from_secs(3), listener.accept()).await??;
        let mut encrypted_request = Vec::new();
        timeout(Duration::from_secs(3), inbound.read_to_end(&mut encrypted_request)).await??;
        let decrypted_request = shadowsocks_aead_adapter_decrypt(&encrypted_request, 0)?;
        let (target_port, payload) =
            parse_shadowsocks_loopback_frame(&decrypted_request, DEFAULT_SHADOWSOCKS_AEAD_ADAPTER_TARGET_PORT)?;
        let mut outbound = TcpStream::connect((RUST_SHADOWSOCKS_AEAD_ADAPTER_HOST, target_port)).await?;
        outbound.write_all(payload).await?;
        outbound.shutdown().await?;
        let mut response = Vec::new();
        timeout(Duration::from_secs(3), outbound.read_to_end(&mut response)).await??;
        let encrypted_response = shadowsocks_aead_adapter_encrypt(&response, 1)?;
        inbound.write_all(&encrypted_response).await?;
        inbound.shutdown().await?;
        Ok::<(u64, u64, u64, u64), anyhow::Error>((
            encrypted_request.len() as u64,
            decrypted_request.len() as u64,
            encrypted_response.len() as u64,
            response.len() as u64,
        ))
    });

    let request = shadowsocks_loopback_frame(
        DEFAULT_SHADOWSOCKS_AEAD_ADAPTER_TARGET_PORT,
        b"GET /rust-shadowsocks-aead-adapter-execution HTTP/1.1\r\nHost: rust-shadowsocks-aead-adapter\r\nConnection: close\r\n\r\n",
    );
    let encrypted_request = shadowsocks_aead_adapter_encrypt(&request, 0)?;
    let mut stream = TcpStream::connect((
        RUST_SHADOWSOCKS_AEAD_ADAPTER_HOST,
        DEFAULT_SHADOWSOCKS_AEAD_ADAPTER_LISTENER_PORT,
    ))
    .await?;
    stream.write_all(&encrypted_request).await?;
    stream.shutdown().await?;
    let mut encrypted_response = Vec::new();
    timeout(Duration::from_secs(3), stream.read_to_end(&mut encrypted_response)).await??;
    let response = shadowsocks_aead_adapter_decrypt(&encrypted_response, 1)?;
    let target_received = target_task.await??;
    let (encrypted_request_bytes, decrypted_request_bytes, encrypted_response_bytes, decrypted_response_bytes) =
        adapter_task.await??;
    let response_status = parse_http_status(&response);
    let address_frame_validated =
        parse_shadowsocks_loopback_frame(&request, DEFAULT_SHADOWSOCKS_AEAD_ADAPTER_TARGET_PORT).is_ok();
    let mut blockers = Vec::new();
    if !address_frame_validated {
        blockers.push("Shadowsocks AEAD adapter did not validate the loopback address frame".into());
    }
    if !target_received {
        blockers.push("Shadowsocks AEAD adapter target did not receive decrypted request".into());
    }
    if response_status.as_deref() != Some("HTTP/1.1 204 No Content") {
        blockers.push("Shadowsocks AEAD adapter response did not decrypt to HTTP 204".into());
    }
    if encrypted_request_bytes <= decrypted_request_bytes || encrypted_response_bytes <= decrypted_response_bytes {
        blockers.push("Shadowsocks AEAD adapter did not observe AEAD tag overhead".into());
    }

    Ok(RustShadowsocksAeadAdapterExecutionEvidence {
        adapter_name: "rust-shadowsocks-aead-loopback-execution".into(),
        cipher: "AES-256-GCM".into(),
        listener_port: DEFAULT_SHADOWSOCKS_AEAD_ADAPTER_LISTENER_PORT,
        target_port: DEFAULT_SHADOWSOCKS_AEAD_ADAPTER_TARGET_PORT,
        target_address: format!("{RUST_SHADOWSOCKS_AEAD_ADAPTER_HOST}:{DEFAULT_SHADOWSOCKS_AEAD_ADAPTER_TARGET_PORT}")
            .into(),
        accepted_connections: 1,
        target_received,
        response_status,
        encrypted_request_bytes,
        decrypted_request_bytes,
        encrypted_response_bytes,
        decrypted_response_bytes,
        address_frame_validated,
        rollback_checkpoint_path: Some(rollback_checkpoint_path.into()),
        fallback_retained_for_unsupported: true,
        passed: blockers.is_empty(),
        blockers,
    })
}

fn shadowsocks_aead_adapter_encrypt(payload: &[u8], nonce_marker: u8) -> Result<Vec<u8>> {
    let key = Sha256::digest(b"rust-shadowsocks-aead-adapter-execution");
    let cipher = Aes256Gcm::new_from_slice(&key)?;
    let nonce_bytes = [nonce_marker; 12];
    Ok(cipher.encrypt(Nonce::from_slice(&nonce_bytes), payload)?)
}

fn shadowsocks_aead_adapter_decrypt(payload: &[u8], nonce_marker: u8) -> Result<Vec<u8>> {
    let key = Sha256::digest(b"rust-shadowsocks-aead-adapter-execution");
    let cipher = Aes256Gcm::new_from_slice(&key)?;
    let nonce_bytes = [nonce_marker; 12];
    Ok(cipher.decrypt(Nonce::from_slice(&nonce_bytes), payload)?)
}

fn shadowsocks_loopback_frame(target_port: u16, payload: &[u8]) -> Vec<u8> {
    let mut frame = Vec::with_capacity(payload.len() + 7);
    frame.push(1);
    frame.extend_from_slice(&[127, 0, 0, 1]);
    frame.extend_from_slice(&target_port.to_be_bytes());
    frame.extend_from_slice(payload);
    frame
}

fn parse_shadowsocks_loopback_frame(frame: &[u8], expected_port: u16) -> Result<(u16, &[u8])> {
    if frame.len() < 7 || frame[0] != 1 || frame[1..5] != [127, 0, 0, 1] {
        bail!("unsupported Shadowsocks AEAD adapter target frame");
    }
    let port = u16::from_be_bytes([frame[5], frame[6]]);
    if port != expected_port {
        bail!("unexpected Shadowsocks AEAD adapter target port: {port}");
    }
    Ok((port, &frame[7..]))
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct RustShadowsocksAeadAdapterRollbackCheckpoint {
    component: String,
    kernel_area: String,
    adapter_name: String,
    fallback_retained_for_unsupported: bool,
    listener_port: u16,
    target_port: u16,
    rollback_action: String,
}

async fn write_rollback_checkpoint(path: &std::path::Path) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).await?;
    }
    let checkpoint = RustShadowsocksAeadAdapterRollbackCheckpoint {
        component: RUST_SHADOWSOCKS_AEAD_ADAPTER_COMPONENT.into(),
        kernel_area: RUST_SHADOWSOCKS_AEAD_ADAPTER_KERNEL_AREA.into(),
        adapter_name: "rust-shadowsocks-aead-loopback-execution".into(),
        fallback_retained_for_unsupported: true,
        listener_port: DEFAULT_SHADOWSOCKS_AEAD_ADAPTER_LISTENER_PORT,
        target_port: DEFAULT_SHADOWSOCKS_AEAD_ADAPTER_TARGET_PORT,
        rollback_action: "stop scoped Rust Shadowsocks AEAD listener and route encrypted protocols back to Mihomo"
            .into(),
    };
    fs::write(path, serde_yaml_ng::to_string(&checkpoint)?.as_bytes()).await?;
    Ok(())
}

fn unsupported_shadowsocks_aead_adapter_protocols() -> Vec<String> {
    vec![
        "Shadowsocks UDP associate".into(),
        "Shadowsocks plugin transports".into(),
        "VMess".into(),
        "VLESS".into(),
        "Trojan TLS".into(),
        "system-wide packet capture".into(),
    ]
}

fn parse_http_status(response: &[u8]) -> Option<String> {
    std::str::from_utf8(response)
        .ok()
        .and_then(|response| response.lines().next())
        .map(Into::into)
}

fn rust_shadowsocks_aead_adapter_evidence_path() -> Result<std::path::PathBuf> {
    Ok(dirs::app_runtime_dir()?
        .join(RUST_SHADOWSOCKS_AEAD_ADAPTER_COMPONENT)
        .join(RUST_SHADOWSOCKS_AEAD_ADAPTER_EVIDENCE_FILE))
}

fn rust_shadowsocks_aead_adapter_rollback_path() -> Result<std::path::PathBuf> {
    Ok(dirs::app_runtime_dir()?
        .join(RUST_SHADOWSOCKS_AEAD_ADAPTER_COMPONENT)
        .join(RUST_SHADOWSOCKS_AEAD_ADAPTER_ROLLBACK_FILE))
}

fn rust_shadowsocks_aead_adapter_facts() -> Vec<String> {
    vec![
        "Rust executes a scoped Shadowsocks AEAD adapter listener".into(),
        "Rust decrypts a loopback address frame and dials the requested TCP target".into(),
        "Rust encrypts target response bytes back to the client".into(),
        "unsupported encrypted protocols and UDP remain on Mihomo fallback".into(),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn shadowsocks_aead_adapter_blocks_without_opt_in() {
        let report = rust_shadowsocks_aead_adapter_execution(false).await.unwrap();

        assert_eq!(report.status, RustShadowsocksAeadAdapterExecutionStatus::Blocked);
        assert!(report.mihomo_fallback);
        assert!(!report.forwards_traffic);
    }

    #[test]
    fn shadowsocks_aead_adapter_frame_round_trip() {
        let frame = shadowsocks_loopback_frame(19781, b"GET / HTTP/1.1\r\n\r\n");
        let encrypted = shadowsocks_aead_adapter_encrypt(&frame, 3).unwrap();
        let decrypted = shadowsocks_aead_adapter_decrypt(&encrypted, 3).unwrap();
        let (port, payload) = parse_shadowsocks_loopback_frame(&decrypted, 19781).unwrap();

        assert_eq!(port, 19781);
        assert_eq!(payload, b"GET / HTTP/1.1\r\n\r\n");
        assert!(encrypted.len() > frame.len());
    }
}
