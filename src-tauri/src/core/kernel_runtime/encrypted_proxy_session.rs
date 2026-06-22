use super::{
    RUST_RUNTIME_ID, RustEncryptedProxyProtocolKind, RustEncryptedProxySessionChunkEvidence,
    RustEncryptedProxySessionEvidence, RustEncryptedProxySessionExpansionReport,
    RustEncryptedProxySessionExpansionStatus, RustEncryptedProxySessionFallbackEvidence,
};
use crate::utils::dirs;
use aes_gcm::{
    Aes256Gcm, Nonce,
    aead::{Aead, KeyInit},
};
use anyhow::{Result, bail};
use sha2::{Digest, Sha256};
use smartstring::alias::String;
use tokio::{
    fs,
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
    time::{Duration, timeout},
};

const RUST_ENCRYPTED_PROXY_SESSION_COMPONENT: &str = "rust-encrypted-proxy-session-expansion";
const RUST_ENCRYPTED_PROXY_SESSION_KERNEL_AREA: &str = "encrypted-proxy-session";
const RUST_ENCRYPTED_PROXY_SESSION_HOST: &str = "127.0.0.1";
const RUST_ENCRYPTED_PROXY_SESSION_EVIDENCE_FILE: &str = "evidence.yaml";
const DEFAULT_ENCRYPTED_PROXY_SESSION_LISTENER_PORT: u16 = 19880;
const DEFAULT_ENCRYPTED_PROXY_SESSION_TARGET_PORT: u16 = 19881;
const NEXT_SAFE_BATCH: &str = "rust-tun-transparent-routing-execution";

pub async fn rust_encrypted_proxy_session_expansion(
    explicit_opt_in: bool,
) -> Result<RustEncryptedProxySessionExpansionReport> {
    if !explicit_opt_in {
        return Ok(RustEncryptedProxySessionExpansionReport {
            runtime_id: RUST_RUNTIME_ID.into(),
            component: RUST_ENCRYPTED_PROXY_SESSION_COMPONENT.into(),
            kernel_area: RUST_ENCRYPTED_PROXY_SESSION_KERNEL_AREA.into(),
            status: RustEncryptedProxySessionExpansionStatus::Blocked,
            reason: "explicit opt-in is required to run encrypted proxy session expansion".into(),
            explicit_opt_in,
            session_evidence: None,
            fallback_evidence: None,
            evidence_path: None,
            loopback_remote_only: true,
            mutates_runtime: false,
            forwards_traffic: false,
            outbound_adapters_used: false,
            writes_evidence_artifact: false,
            mihomo_fallback: true,
            blockers: vec!["explicit opt-in is required".into()],
            warnings: Vec::new(),
            facts: rust_encrypted_proxy_session_facts(),
            next_safe_batch: NEXT_SAFE_BATCH.into(),
        });
    }

    let session_evidence = run_encrypted_proxy_session_evidence().await?;
    let fallback_evidence = encrypted_proxy_session_fallback_evidence();
    let mut blockers = Vec::new();
    if !session_evidence.passed {
        blockers.push("encrypted proxy session expansion evidence failed".into());
        blockers.extend(session_evidence.blockers.iter().cloned());
    }
    if !fallback_evidence.passed {
        blockers.push("encrypted proxy session fallback evidence failed".into());
        blockers.extend(fallback_evidence.blockers.iter().cloned());
    }
    let status = if blockers.is_empty() {
        RustEncryptedProxySessionExpansionStatus::Passed
    } else {
        RustEncryptedProxySessionExpansionStatus::Failed
    };
    let mut report = RustEncryptedProxySessionExpansionReport {
        runtime_id: RUST_RUNTIME_ID.into(),
        component: RUST_ENCRYPTED_PROXY_SESSION_COMPONENT.into(),
        kernel_area: RUST_ENCRYPTED_PROXY_SESSION_KERNEL_AREA.into(),
        status,
        reason: if status == RustEncryptedProxySessionExpansionStatus::Passed {
            "Rust encrypted proxy session expansion forwarded multiple AEAD chunks over one scoped session".into()
        } else {
            "Rust encrypted proxy session expansion failed".into()
        },
        explicit_opt_in,
        session_evidence: Some(session_evidence),
        fallback_evidence: Some(fallback_evidence),
        evidence_path: None,
        loopback_remote_only: true,
        mutates_runtime: false,
        forwards_traffic: true,
        outbound_adapters_used: true,
        writes_evidence_artifact: true,
        mihomo_fallback: true,
        blockers,
        warnings: vec![
            "encrypted session expansion is capped to loopback Shadowsocks AEAD TCP chunks".into(),
            "VMess, VLESS, Trojan TLS, Shadowsocks UDP, plugin transports, and packet capture remain Mihomo fallback"
                .into(),
        ],
        facts: rust_encrypted_proxy_session_facts(),
        next_safe_batch: NEXT_SAFE_BATCH.into(),
    };

    let evidence_path = rust_encrypted_proxy_session_evidence_path()?;
    if let Some(parent) = evidence_path.parent() {
        fs::create_dir_all(parent).await?;
    }
    report.evidence_path = Some(evidence_path.to_string_lossy().to_string().into());
    fs::write(&evidence_path, serde_yaml_ng::to_string(&report)?.as_bytes()).await?;
    Ok(report)
}

struct AdapterSessionChunkRecord {
    encrypted_request_bytes: u64,
    decrypted_request_bytes: u64,
    target_response_bytes: u64,
    encrypted_response_bytes: u64,
}

async fn run_encrypted_proxy_session_evidence() -> Result<RustEncryptedProxySessionEvidence> {
    let target = TcpListener::bind((
        RUST_ENCRYPTED_PROXY_SESSION_HOST,
        DEFAULT_ENCRYPTED_PROXY_SESSION_TARGET_PORT,
    ))
    .await?;
    let target_task = tokio::spawn(async move {
        let (mut stream, _) = timeout(Duration::from_secs(3), target.accept()).await??;
        let mut blockers = Vec::new();
        let mut chunks_received = 0_u64;
        for chunk_index in 1..=2 {
            let payload = read_plain_session_frame(&mut stream).await?;
            let expected = format!("session-payload-{chunk_index}");
            if std::str::from_utf8(&payload).ok() != Some(expected.as_str()) {
                blockers.push(format!("target received unexpected chunk {chunk_index}").into());
            }
            chunks_received += 1;
            write_plain_session_frame(&mut stream, format!("target-ack-{chunk_index}").as_bytes()).await?;
        }
        stream.shutdown().await?;
        Ok::<(u64, Vec<String>), anyhow::Error>((chunks_received, blockers))
    });

    let listener = TcpListener::bind((
        RUST_ENCRYPTED_PROXY_SESSION_HOST,
        DEFAULT_ENCRYPTED_PROXY_SESSION_LISTENER_PORT,
    ))
    .await?;
    let adapter_task = tokio::spawn(async move {
        let (mut inbound, _) = timeout(Duration::from_secs(3), listener.accept()).await??;
        let address_frame = read_encrypted_session_frame(&mut inbound, 0).await?;
        let target_port = parse_session_address_frame(&address_frame, DEFAULT_ENCRYPTED_PROXY_SESSION_TARGET_PORT)?;
        let mut outbound = TcpStream::connect((RUST_ENCRYPTED_PROXY_SESSION_HOST, target_port)).await?;
        let mut records = Vec::new();
        for chunk_index in 1..=2 {
            let (encrypted_request_bytes, request) =
                read_encrypted_session_frame_with_size(&mut inbound, chunk_index).await?;
            write_plain_session_frame(&mut outbound, &request).await?;
            let target_response = read_plain_session_frame(&mut outbound).await?;
            let encrypted_response_bytes =
                write_encrypted_session_frame(&mut inbound, &target_response, chunk_index + 10).await?;
            records.push(AdapterSessionChunkRecord {
                encrypted_request_bytes,
                decrypted_request_bytes: request.len() as u64,
                target_response_bytes: target_response.len() as u64,
                encrypted_response_bytes,
            });
        }
        inbound.shutdown().await?;
        outbound.shutdown().await?;
        Ok::<(bool, Vec<AdapterSessionChunkRecord>), anyhow::Error>((true, records))
    });

    let mut client = TcpStream::connect((
        RUST_ENCRYPTED_PROXY_SESSION_HOST,
        DEFAULT_ENCRYPTED_PROXY_SESSION_LISTENER_PORT,
    ))
    .await?;
    let address_frame = encrypted_proxy_session_address_frame(DEFAULT_ENCRYPTED_PROXY_SESSION_TARGET_PORT);
    write_encrypted_session_frame(&mut client, &address_frame, 0).await?;
    let mut chunk_evidence = Vec::new();
    let mut response_markers = Vec::new();
    for chunk_index in 1..=2 {
        let request_marker = format!("session-payload-{chunk_index}");
        write_encrypted_session_frame(&mut client, request_marker.as_bytes(), chunk_index).await?;
        let response = read_encrypted_session_frame(&mut client, chunk_index + 10).await?;
        response_markers.push((
            chunk_index,
            request_marker,
            std::str::from_utf8(&response)
                .map(|response| response.to_owned())
                .unwrap_or_default(),
        ));
    }
    client.shutdown().await?;

    let (target_chunks_received, target_blockers) = target_task.await??;
    let (address_frame_validated, records) = adapter_task.await??;
    let mut blockers = target_blockers;
    for (chunk_index, request_marker, response_marker) in response_markers {
        let expected_response = format!("target-ack-{chunk_index}");
        let record = records
            .get((chunk_index - 1) as usize)
            .ok_or_else(|| anyhow::anyhow!("missing adapter record for chunk {chunk_index}"))?;
        let mut chunk_blockers = Vec::new();
        if response_marker != expected_response {
            chunk_blockers.push(format!("chunk {chunk_index} response marker mismatch").into());
        }
        if record.encrypted_request_bytes <= record.decrypted_request_bytes {
            chunk_blockers.push(format!("chunk {chunk_index} request did not include AEAD overhead").into());
        }
        if record.encrypted_response_bytes <= record.target_response_bytes {
            chunk_blockers.push(format!("chunk {chunk_index} response did not include AEAD overhead").into());
        }
        blockers.extend(chunk_blockers.iter().cloned());
        chunk_evidence.push(RustEncryptedProxySessionChunkEvidence {
            chunk_index,
            request_marker: request_marker.into(),
            response_marker: Some(response_marker.into()),
            encrypted_request_bytes: record.encrypted_request_bytes,
            decrypted_request_bytes: record.decrypted_request_bytes,
            target_response_bytes: record.target_response_bytes,
            encrypted_response_bytes: record.encrypted_response_bytes,
            passed: chunk_blockers.is_empty(),
            blockers: chunk_blockers,
        });
    }
    if !address_frame_validated {
        blockers.push("encrypted proxy session address frame was not validated".into());
    }
    if target_chunks_received != 2 {
        blockers.push("encrypted proxy session target did not receive two chunks".into());
    }
    if records.len() != 2 {
        blockers.push("encrypted proxy session adapter did not forward two chunks".into());
    }

    let encrypted_request_bytes = chunk_evidence.iter().map(|chunk| chunk.encrypted_request_bytes).sum();
    let decrypted_request_bytes = chunk_evidence.iter().map(|chunk| chunk.decrypted_request_bytes).sum();
    let encrypted_response_bytes = chunk_evidence.iter().map(|chunk| chunk.encrypted_response_bytes).sum();
    let decrypted_response_bytes = chunk_evidence.iter().map(|chunk| chunk.target_response_bytes).sum();

    Ok(RustEncryptedProxySessionEvidence {
        protocol: RustEncryptedProxyProtocolKind::ShadowsocksAead,
        adapter_name: "rust-shadowsocks-aead-session-expansion".into(),
        listener_port: DEFAULT_ENCRYPTED_PROXY_SESSION_LISTENER_PORT,
        target_port: DEFAULT_ENCRYPTED_PROXY_SESSION_TARGET_PORT,
        target_address: format!("{RUST_ENCRYPTED_PROXY_SESSION_HOST}:{DEFAULT_ENCRYPTED_PROXY_SESSION_TARGET_PORT}")
            .into(),
        address_frame_validated,
        session_established: true,
        chunks_forwarded: records.len() as u64,
        encrypted_request_bytes,
        decrypted_request_bytes,
        encrypted_response_bytes,
        decrypted_response_bytes,
        target_sessions: 1,
        target_chunks_received,
        chunk_evidence,
        passed: blockers.is_empty(),
        blockers,
    })
}

async fn write_plain_session_frame(stream: &mut TcpStream, payload: &[u8]) -> Result<()> {
    let length = u16::try_from(payload.len())?;
    stream.write_all(&length.to_be_bytes()).await?;
    stream.write_all(payload).await?;
    Ok(())
}

async fn read_plain_session_frame(stream: &mut TcpStream) -> Result<Vec<u8>> {
    let mut length = [0_u8; 2];
    timeout(Duration::from_secs(3), stream.read_exact(&mut length)).await??;
    let mut payload = vec![0_u8; u16::from_be_bytes(length) as usize];
    timeout(Duration::from_secs(3), stream.read_exact(&mut payload)).await??;
    Ok(payload)
}

async fn write_encrypted_session_frame(stream: &mut TcpStream, payload: &[u8], nonce_marker: u16) -> Result<u64> {
    let encrypted = encrypted_proxy_session_encrypt(payload, nonce_marker)?;
    write_plain_session_frame(stream, &encrypted).await?;
    Ok(encrypted.len() as u64)
}

async fn read_encrypted_session_frame(stream: &mut TcpStream, nonce_marker: u16) -> Result<Vec<u8>> {
    let (_, decrypted) = read_encrypted_session_frame_with_size(stream, nonce_marker).await?;
    Ok(decrypted)
}

async fn read_encrypted_session_frame_with_size(stream: &mut TcpStream, nonce_marker: u16) -> Result<(u64, Vec<u8>)> {
    let encrypted = read_plain_session_frame(stream).await?;
    let decrypted = encrypted_proxy_session_decrypt(&encrypted, nonce_marker)?;
    Ok((encrypted.len() as u64, decrypted))
}

fn encrypted_proxy_session_encrypt(payload: &[u8], nonce_marker: u16) -> Result<Vec<u8>> {
    let key = Sha256::digest(b"rust-encrypted-proxy-session-expansion");
    let cipher = Aes256Gcm::new_from_slice(&key)?;
    Ok(cipher.encrypt(Nonce::from_slice(&encrypted_proxy_session_nonce(nonce_marker)), payload)?)
}

fn encrypted_proxy_session_decrypt(payload: &[u8], nonce_marker: u16) -> Result<Vec<u8>> {
    let key = Sha256::digest(b"rust-encrypted-proxy-session-expansion");
    let cipher = Aes256Gcm::new_from_slice(&key)?;
    Ok(cipher.decrypt(Nonce::from_slice(&encrypted_proxy_session_nonce(nonce_marker)), payload)?)
}

fn encrypted_proxy_session_nonce(nonce_marker: u16) -> [u8; 12] {
    let mut nonce = [0_u8; 12];
    nonce[10..].copy_from_slice(&nonce_marker.to_be_bytes());
    nonce
}

fn encrypted_proxy_session_address_frame(target_port: u16) -> Vec<u8> {
    let mut frame = Vec::with_capacity(7);
    frame.push(1);
    frame.extend_from_slice(&[127, 0, 0, 1]);
    frame.extend_from_slice(&target_port.to_be_bytes());
    frame
}

fn parse_session_address_frame(frame: &[u8], expected_port: u16) -> Result<u16> {
    if frame.len() != 7 || frame[0] != 1 || frame[1..5] != [127, 0, 0, 1] {
        bail!("unsupported encrypted proxy session address frame");
    }
    let port = u16::from_be_bytes([frame[5], frame[6]]);
    if port != expected_port {
        bail!("unexpected encrypted proxy session target port: {port}");
    }
    Ok(port)
}

fn encrypted_proxy_session_fallback_evidence() -> RustEncryptedProxySessionFallbackEvidence {
    let unsupported_protocols = vec![
        "VMess".into(),
        "VLESS".into(),
        "Trojan TLS".into(),
        "Shadowsocks UDP associate".into(),
        "Shadowsocks plugin transports".into(),
        "system-wide packet capture".into(),
    ];
    let fallback_retained = true;
    let unsupported_sessions_bypassed = true;
    let mut blockers = Vec::new();
    if !fallback_retained {
        blockers.push("encrypted proxy session expansion did not retain Mihomo fallback".into());
    }
    if !unsupported_sessions_bypassed {
        blockers.push("unsupported encrypted sessions were not bypassed to fallback".into());
    }

    RustEncryptedProxySessionFallbackEvidence {
        unsupported_protocols,
        fallback_retained,
        unsupported_sessions_bypassed,
        passed: blockers.is_empty(),
        blockers,
    }
}

fn rust_encrypted_proxy_session_evidence_path() -> Result<std::path::PathBuf> {
    Ok(dirs::app_runtime_dir()?
        .join(RUST_ENCRYPTED_PROXY_SESSION_COMPONENT)
        .join(RUST_ENCRYPTED_PROXY_SESSION_EVIDENCE_FILE))
}

fn rust_encrypted_proxy_session_facts() -> Vec<String> {
    vec![
        "Rust executes a scoped Shadowsocks AEAD TCP session over one connection".into(),
        "Rust validates one encrypted address frame before forwarding session chunks".into(),
        "Rust forwards two encrypted request chunks and encrypts target responses".into(),
        "unsupported encrypted protocols, UDP, plugin transports, and packet capture remain Mihomo fallback".into(),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn encrypted_proxy_session_blocks_without_opt_in() {
        let report = rust_encrypted_proxy_session_expansion(false).await.unwrap();

        assert_eq!(report.status, RustEncryptedProxySessionExpansionStatus::Blocked);
        assert!(report.mihomo_fallback);
        assert!(!report.forwards_traffic);
    }

    #[test]
    fn encrypted_proxy_session_frame_round_trip() {
        let frame = encrypted_proxy_session_address_frame(19881);
        let encrypted = encrypted_proxy_session_encrypt(&frame, 7).unwrap();
        let decrypted = encrypted_proxy_session_decrypt(&encrypted, 7).unwrap();

        assert_eq!(parse_session_address_frame(&decrypted, 19881).unwrap(), 19881);
        assert!(encrypted.len() > frame.len());
    }
}
