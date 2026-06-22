use super::{
    RUST_RUNTIME_ID, RustEncryptedProxyProtocolEvidence, RustEncryptedProxyProtocolKind,
    RustEncryptedProxyProtocolPreflightReport, RustEncryptedProxyProtocolStatus,
};
use crate::utils::dirs;
use aes_gcm::{
    Aes256Gcm, Nonce,
    aead::{Aead, KeyInit},
};
use anyhow::{Result, bail};
use sha2::{Digest, Sha224, Sha256};
use smartstring::alias::String;
use tokio::{
    fs,
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
    time::{Duration, timeout},
};

const RUST_ENCRYPTED_PROXY_PROTOCOL_COMPONENT: &str = "rust-encrypted-proxy-protocol-preflight";
const RUST_ENCRYPTED_PROXY_PROTOCOL_KERNEL_AREA: &str = "encrypted-proxy-protocol";
const RUST_ENCRYPTED_PROXY_PROTOCOL_HOST: &str = "127.0.0.1";
const RUST_ENCRYPTED_PROXY_PROTOCOL_EVIDENCE_FILE: &str = "evidence.yaml";
const DEFAULT_SHADOWSOCKS_AEAD_LISTENER_PORT: u16 = 19680;
const DEFAULT_SHADOWSOCKS_AEAD_TARGET_PORT: u16 = 19681;
const DEFAULT_TROJAN_AUTH_LISTENER_PORT: u16 = 19682;
const DEFAULT_TROJAN_AUTH_TARGET_PORT: u16 = 19683;
const NEXT_SAFE_BATCH: &str = "rust-shadowsocks-aead-adapter-execution";

pub async fn rust_encrypted_proxy_protocol_preflight_evidence(
    explicit_opt_in: bool,
) -> Result<RustEncryptedProxyProtocolPreflightReport> {
    if !explicit_opt_in {
        return Ok(RustEncryptedProxyProtocolPreflightReport {
            runtime_id: RUST_RUNTIME_ID.into(),
            component: RUST_ENCRYPTED_PROXY_PROTOCOL_COMPONENT.into(),
            kernel_area: RUST_ENCRYPTED_PROXY_PROTOCOL_KERNEL_AREA.into(),
            status: RustEncryptedProxyProtocolStatus::Blocked,
            reason: "explicit opt-in is required to run encrypted proxy protocol preflight".into(),
            explicit_opt_in,
            shadowsocks_aead_evidence: None,
            trojan_auth_evidence: None,
            unsupported_protocol_evidence: unsupported_encrypted_protocol_evidence(),
            evidence_path: None,
            loopback_remote_only: true,
            mutates_runtime: false,
            forwards_traffic: false,
            outbound_adapters_used: false,
            writes_evidence_artifact: false,
            mihomo_fallback: true,
            blockers: vec!["explicit opt-in is required".into()],
            warnings: Vec::new(),
            facts: rust_encrypted_proxy_protocol_facts(),
            next_safe_batch: NEXT_SAFE_BATCH.into(),
        });
    }

    let shadowsocks_aead_evidence = run_shadowsocks_aead_evidence().await?;
    let trojan_auth_evidence = run_trojan_auth_evidence().await?;
    let unsupported_protocol_evidence = unsupported_encrypted_protocol_evidence();
    let mut blockers = Vec::new();
    if !shadowsocks_aead_evidence.passed {
        blockers.push("Shadowsocks AEAD encrypted framing evidence failed".into());
        blockers.extend(shadowsocks_aead_evidence.blockers.iter().cloned());
    }
    if !trojan_auth_evidence.passed {
        blockers.push("Trojan auth framing evidence failed".into());
        blockers.extend(trojan_auth_evidence.blockers.iter().cloned());
    }
    for evidence in &unsupported_protocol_evidence {
        if !evidence.passed {
            blockers.push(format!("{:?} fallback evidence failed", evidence.protocol).into());
            blockers.extend(evidence.blockers.iter().cloned());
        }
    }
    let status = if blockers.is_empty() {
        RustEncryptedProxyProtocolStatus::Passed
    } else {
        RustEncryptedProxyProtocolStatus::Failed
    };

    let mut report = RustEncryptedProxyProtocolPreflightReport {
        runtime_id: RUST_RUNTIME_ID.into(),
        component: RUST_ENCRYPTED_PROXY_PROTOCOL_COMPONENT.into(),
        kernel_area: RUST_ENCRYPTED_PROXY_PROTOCOL_KERNEL_AREA.into(),
        status,
        reason: if status == RustEncryptedProxyProtocolStatus::Passed {
            "Rust encrypted proxy protocol preflight passed bounded AEAD and auth framing evidence".into()
        } else {
            "Rust encrypted proxy protocol preflight failed".into()
        },
        explicit_opt_in,
        shadowsocks_aead_evidence: Some(shadowsocks_aead_evidence),
        trojan_auth_evidence: Some(trojan_auth_evidence),
        unsupported_protocol_evidence,
        evidence_path: None,
        loopback_remote_only: true,
        mutates_runtime: false,
        forwards_traffic: true,
        outbound_adapters_used: true,
        writes_evidence_artifact: true,
        mihomo_fallback: true,
        blockers,
        warnings: vec![
            "preflight is capped to loopback Shadowsocks AEAD framing and Trojan auth framing".into(),
            "Mihomo remains fallback for VMess/VLESS/Trojan TLS, Shadowsocks full sessions, UDP, and packet capture"
                .into(),
        ],
        facts: rust_encrypted_proxy_protocol_facts(),
        next_safe_batch: NEXT_SAFE_BATCH.into(),
    };

    let evidence_path = rust_encrypted_proxy_protocol_evidence_path()?;
    if let Some(parent) = evidence_path.parent() {
        fs::create_dir_all(parent).await?;
    }
    report.evidence_path = Some(evidence_path.to_string_lossy().to_string().into());
    fs::write(&evidence_path, serde_yaml_ng::to_string(&report)?.as_bytes()).await?;
    Ok(report)
}

async fn run_shadowsocks_aead_evidence() -> Result<RustEncryptedProxyProtocolEvidence> {
    let target = TcpListener::bind((RUST_ENCRYPTED_PROXY_PROTOCOL_HOST, DEFAULT_SHADOWSOCKS_AEAD_TARGET_PORT)).await?;
    let target_task = tokio::spawn(async move {
        let (mut stream, _) = timeout(Duration::from_secs(3), target.accept()).await??;
        let mut request = Vec::new();
        timeout(Duration::from_secs(3), stream.read_to_end(&mut request)).await??;
        let target_received = std::str::from_utf8(&request)
            .map(|request| request.contains("GET /rust-shadowsocks-aead"))
            .unwrap_or(false);
        stream
            .write_all(b"HTTP/1.1 204 No Content\r\nConnection: close\r\nContent-Length: 0\r\n\r\n")
            .await?;
        stream.shutdown().await?;
        Ok::<bool, anyhow::Error>(target_received)
    });

    let listener = TcpListener::bind((
        RUST_ENCRYPTED_PROXY_PROTOCOL_HOST,
        DEFAULT_SHADOWSOCKS_AEAD_LISTENER_PORT,
    ))
    .await?;
    let server_task = tokio::spawn(async move {
        let (mut inbound, _) = timeout(Duration::from_secs(3), listener.accept()).await??;
        let mut encrypted_request = Vec::new();
        timeout(Duration::from_secs(3), inbound.read_to_end(&mut encrypted_request)).await??;
        let decrypted_request = shadowsocks_aead_decrypt(&encrypted_request, 0)?;
        let (target_port, payload) =
            parse_loopback_target_payload(&decrypted_request, DEFAULT_SHADOWSOCKS_AEAD_TARGET_PORT)?;
        let mut outbound = TcpStream::connect((RUST_ENCRYPTED_PROXY_PROTOCOL_HOST, target_port)).await?;
        outbound.write_all(payload).await?;
        outbound.shutdown().await?;
        let mut response = Vec::new();
        timeout(Duration::from_secs(3), outbound.read_to_end(&mut response)).await??;
        let encrypted_response = shadowsocks_aead_encrypt(&response, 1)?;
        inbound.write_all(&encrypted_response).await?;
        inbound.shutdown().await?;
        Ok::<(u64, u64, u64, u64), anyhow::Error>((
            encrypted_request.len() as u64,
            decrypted_request.len() as u64,
            encrypted_response.len() as u64,
            response.len() as u64,
        ))
    });

    let request = loopback_target_payload(
        DEFAULT_SHADOWSOCKS_AEAD_TARGET_PORT,
        b"GET /rust-shadowsocks-aead HTTP/1.1\r\nHost: rust-aead\r\nConnection: close\r\n\r\n",
    );
    let encrypted_request = shadowsocks_aead_encrypt(&request, 0)?;
    let mut stream = TcpStream::connect((
        RUST_ENCRYPTED_PROXY_PROTOCOL_HOST,
        DEFAULT_SHADOWSOCKS_AEAD_LISTENER_PORT,
    ))
    .await?;
    stream.write_all(&encrypted_request).await?;
    stream.shutdown().await?;
    let mut encrypted_response = Vec::new();
    timeout(Duration::from_secs(3), stream.read_to_end(&mut encrypted_response)).await??;
    let response = shadowsocks_aead_decrypt(&encrypted_response, 1)?;
    let target_received = target_task.await??;
    let (encrypted_request_bytes, decrypted_request_bytes, encrypted_response_bytes, decrypted_response_bytes) =
        server_task.await??;
    let response_status = parse_http_status(&response);
    let mut blockers = Vec::new();
    if !target_received {
        blockers.push("Shadowsocks AEAD target did not receive decrypted request".into());
    }
    if response_status.as_deref() != Some("HTTP/1.1 204 No Content") {
        blockers.push("Shadowsocks AEAD response did not decrypt to HTTP 204".into());
    }
    if encrypted_request_bytes <= decrypted_request_bytes || encrypted_response_bytes <= decrypted_response_bytes {
        blockers.push("Shadowsocks AEAD evidence did not include authentication tag overhead".into());
    }

    Ok(RustEncryptedProxyProtocolEvidence {
        protocol: RustEncryptedProxyProtocolKind::ShadowsocksAead,
        adapter_name: "rust-shadowsocks-aead-loopback".into(),
        listener_port: Some(DEFAULT_SHADOWSOCKS_AEAD_LISTENER_PORT),
        target_port: Some(DEFAULT_SHADOWSOCKS_AEAD_TARGET_PORT),
        target_received,
        response_status,
        encrypted_request_bytes,
        decrypted_request_bytes,
        encrypted_response_bytes,
        decrypted_response_bytes,
        fallback_retained: false,
        passed: blockers.is_empty(),
        blockers,
    })
}

async fn run_trojan_auth_evidence() -> Result<RustEncryptedProxyProtocolEvidence> {
    let target = TcpListener::bind((RUST_ENCRYPTED_PROXY_PROTOCOL_HOST, DEFAULT_TROJAN_AUTH_TARGET_PORT)).await?;
    let target_task = tokio::spawn(async move {
        let (mut stream, _) = timeout(Duration::from_secs(3), target.accept()).await??;
        let mut request = Vec::new();
        timeout(Duration::from_secs(3), stream.read_to_end(&mut request)).await??;
        let target_received = std::str::from_utf8(&request)
            .map(|request| request.contains("GET /rust-trojan-auth"))
            .unwrap_or(false);
        stream
            .write_all(b"HTTP/1.1 204 No Content\r\nConnection: close\r\nContent-Length: 0\r\n\r\n")
            .await?;
        stream.shutdown().await?;
        Ok::<bool, anyhow::Error>(target_received)
    });

    let listener = TcpListener::bind((RUST_ENCRYPTED_PROXY_PROTOCOL_HOST, DEFAULT_TROJAN_AUTH_LISTENER_PORT)).await?;
    let server_task = tokio::spawn(async move {
        let (mut inbound, _) = timeout(Duration::from_secs(3), listener.accept()).await??;
        let mut request = Vec::new();
        timeout(Duration::from_secs(3), inbound.read_to_end(&mut request)).await??;
        let (target_port, payload) = parse_trojan_auth_payload(&request, DEFAULT_TROJAN_AUTH_TARGET_PORT)?;
        let mut outbound = TcpStream::connect((RUST_ENCRYPTED_PROXY_PROTOCOL_HOST, target_port)).await?;
        outbound.write_all(payload).await?;
        outbound.shutdown().await?;
        let mut response = Vec::new();
        timeout(Duration::from_secs(3), outbound.read_to_end(&mut response)).await??;
        inbound.write_all(&response).await?;
        inbound.shutdown().await?;
        Ok::<(u64, u64), anyhow::Error>((request.len() as u64, response.len() as u64))
    });

    let trojan_frame = trojan_auth_payload(
        DEFAULT_TROJAN_AUTH_TARGET_PORT,
        b"GET /rust-trojan-auth HTTP/1.1\r\nHost: rust-trojan\r\nConnection: close\r\n\r\n",
    );
    let mut stream =
        TcpStream::connect((RUST_ENCRYPTED_PROXY_PROTOCOL_HOST, DEFAULT_TROJAN_AUTH_LISTENER_PORT)).await?;
    stream.write_all(&trojan_frame).await?;
    stream.shutdown().await?;
    let mut response = Vec::new();
    timeout(Duration::from_secs(3), stream.read_to_end(&mut response)).await??;
    let target_received = target_task.await??;
    let (request_bytes, response_bytes) = server_task.await??;
    let response_status = parse_http_status(&response);
    let mut blockers = Vec::new();
    if !target_received {
        blockers.push("Trojan auth target did not receive framed request".into());
    }
    if response_status.as_deref() != Some("HTTP/1.1 204 No Content") {
        blockers.push("Trojan auth response did not return HTTP 204".into());
    }

    Ok(RustEncryptedProxyProtocolEvidence {
        protocol: RustEncryptedProxyProtocolKind::TrojanAuth,
        adapter_name: "rust-trojan-auth-loopback".into(),
        listener_port: Some(DEFAULT_TROJAN_AUTH_LISTENER_PORT),
        target_port: Some(DEFAULT_TROJAN_AUTH_TARGET_PORT),
        target_received,
        response_status,
        encrypted_request_bytes: request_bytes,
        decrypted_request_bytes: request_bytes.saturating_sub(58),
        encrypted_response_bytes: response_bytes,
        decrypted_response_bytes: response.len() as u64,
        fallback_retained: false,
        passed: blockers.is_empty(),
        blockers,
    })
}

fn shadowsocks_aead_encrypt(payload: &[u8], nonce_marker: u8) -> Result<Vec<u8>> {
    let key = Sha256::digest(b"rust-encrypted-proxy-protocol-preflight");
    let cipher = Aes256Gcm::new_from_slice(&key)?;
    let nonce_bytes = [nonce_marker; 12];
    Ok(cipher.encrypt(Nonce::from_slice(&nonce_bytes), payload)?)
}

fn shadowsocks_aead_decrypt(payload: &[u8], nonce_marker: u8) -> Result<Vec<u8>> {
    let key = Sha256::digest(b"rust-encrypted-proxy-protocol-preflight");
    let cipher = Aes256Gcm::new_from_slice(&key)?;
    let nonce_bytes = [nonce_marker; 12];
    Ok(cipher.decrypt(Nonce::from_slice(&nonce_bytes), payload)?)
}

fn loopback_target_payload(target_port: u16, payload: &[u8]) -> Vec<u8> {
    let mut frame = Vec::with_capacity(payload.len() + 7);
    frame.push(1);
    frame.extend_from_slice(&[127, 0, 0, 1]);
    frame.extend_from_slice(&target_port.to_be_bytes());
    frame.extend_from_slice(payload);
    frame
}

fn parse_loopback_target_payload(frame: &[u8], expected_port: u16) -> Result<(u16, &[u8])> {
    if frame.len() < 7 || frame[0] != 1 || frame[1..5] != [127, 0, 0, 1] {
        bail!("unsupported encrypted proxy target address frame");
    }
    let port = u16::from_be_bytes([frame[5], frame[6]]);
    if port != expected_port {
        bail!("unexpected encrypted proxy target port: {port}");
    }
    Ok((port, &frame[7..]))
}

fn trojan_auth_payload(target_port: u16, payload: &[u8]) -> Vec<u8> {
    let mut frame = Vec::new();
    frame.extend_from_slice(trojan_password_hash().as_bytes());
    frame.extend_from_slice(b"\r\n");
    frame.extend_from_slice(&loopback_target_payload(target_port, payload));
    frame
}

fn parse_trojan_auth_payload(frame: &[u8], expected_port: u16) -> Result<(u16, &[u8])> {
    if frame.len() < 58 {
        bail!("Trojan auth frame is too short");
    }
    let expected_hash = trojan_password_hash();
    if &frame[..56] != expected_hash.as_bytes() || &frame[56..58] != b"\r\n" {
        bail!("Trojan auth hash mismatch");
    }
    parse_loopback_target_payload(&frame[58..], expected_port)
}

fn trojan_password_hash() -> std::string::String {
    Sha224::digest(b"rust-trojan-password")
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn unsupported_encrypted_protocol_evidence() -> Vec<RustEncryptedProxyProtocolEvidence> {
    vec![
        unsupported_protocol("vmess-aead"),
        unsupported_protocol("vless-xtls"),
        unsupported_protocol("trojan-tls-session"),
        unsupported_protocol("shadowsocks-udp-associate"),
    ]
}

fn unsupported_protocol(adapter_name: &str) -> RustEncryptedProxyProtocolEvidence {
    RustEncryptedProxyProtocolEvidence {
        protocol: RustEncryptedProxyProtocolKind::UnsupportedEncryptedProtocol,
        adapter_name: adapter_name.into(),
        listener_port: None,
        target_port: None,
        target_received: false,
        response_status: None,
        encrypted_request_bytes: 0,
        decrypted_request_bytes: 0,
        encrypted_response_bytes: 0,
        decrypted_response_bytes: 0,
        fallback_retained: true,
        passed: true,
        blockers: Vec::new(),
    }
}

fn parse_http_status(response: &[u8]) -> Option<String> {
    std::str::from_utf8(response)
        .ok()
        .and_then(|response| response.lines().next())
        .map(Into::into)
}

fn rust_encrypted_proxy_protocol_evidence_path() -> Result<std::path::PathBuf> {
    Ok(dirs::app_runtime_dir()?
        .join(RUST_ENCRYPTED_PROXY_PROTOCOL_COMPONENT)
        .join(RUST_ENCRYPTED_PROXY_PROTOCOL_EVIDENCE_FILE))
}

fn rust_encrypted_proxy_protocol_facts() -> Vec<String> {
    vec![
        "Rust encrypts and decrypts a Shadowsocks-style AEAD address frame".into(),
        "Rust validates Trojan SHA224 auth framing before target dialing".into(),
        "both bounded encrypted protocol paths forward loopback HTTP bytes to a target".into(),
        "unsupported encrypted proxy sessions stay on Mihomo fallback".into(),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn encrypted_proxy_protocol_blocks_without_opt_in() {
        let report = rust_encrypted_proxy_protocol_preflight_evidence(false).await.unwrap();

        assert_eq!(report.status, RustEncryptedProxyProtocolStatus::Blocked);
        assert!(report.mihomo_fallback);
        assert!(!report.forwards_traffic);
    }

    #[test]
    fn shadowsocks_aead_round_trip_preserves_frame() {
        let payload = loopback_target_payload(19681, b"GET / HTTP/1.1\r\n\r\n");
        let encrypted = shadowsocks_aead_encrypt(&payload, 7).unwrap();
        let decrypted = shadowsocks_aead_decrypt(&encrypted, 7).unwrap();

        assert_eq!(decrypted, payload);
        assert!(encrypted.len() > payload.len());
    }

    #[test]
    fn trojan_auth_frame_round_trip_preserves_target_payload() {
        let payload = trojan_auth_payload(19683, b"GET / HTTP/1.1\r\n\r\n");
        let (port, parsed) = parse_trojan_auth_payload(&payload, 19683).unwrap();

        assert_eq!(port, 19683);
        assert_eq!(parsed, b"GET / HTTP/1.1\r\n\r\n");
    }
}
