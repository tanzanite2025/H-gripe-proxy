use super::{
    RustEncryptedProtocolBundleProtocol, RustEncryptedProtocolsBundleSessionEvidence,
    constants::{HOST, IO_TIMEOUT_SECONDS},
    framing::{read_frame, write_frame},
    protocol::{
        adapter_name, decode_request_frame, decode_response_frame, encode_request_frame, encode_response_frame,
        request_marker, response_marker,
    },
};
use anyhow::Result;
use tokio::{
    io::AsyncWriteExt,
    net::{TcpListener, TcpStream},
    time::{Duration, timeout},
};

struct AdapterRecord {
    handshake_validated: bool,
    payload_bytes_to_target: u64,
    response_bytes_to_client: u64,
}

pub(super) async fn run_protocol_session(
    protocol: RustEncryptedProtocolBundleProtocol,
) -> Result<RustEncryptedProtocolsBundleSessionEvidence> {
    let target = TcpListener::bind((HOST, 0)).await?;
    let target_port = target.local_addr()?.port();
    let expected_request = request_marker(protocol);
    let expected_response = response_marker(protocol);
    let target_task = tokio::spawn({
        let expected_request = expected_request.clone();
        let expected_response = expected_response.clone();
        async move {
            let (mut stream, _) = timeout(Duration::from_secs(IO_TIMEOUT_SECONDS), target.accept()).await??;
            let request = read_frame(&mut stream).await?;
            let mut blockers = Vec::new();
            if std::str::from_utf8(&request).ok() != Some(expected_request.as_str()) {
                blockers.push("target received unexpected encrypted protocol payload".to_owned());
            }
            let response_bytes = write_frame(&mut stream, expected_response.as_bytes()).await?;
            stream.shutdown().await?;
            Ok::<(u64, u64, Vec<std::string::String>), anyhow::Error>((request.len() as u64, response_bytes, blockers))
        }
    });

    let listener = TcpListener::bind((HOST, 0)).await?;
    let listener_port = listener.local_addr()?.port();
    let adapter_task = tokio::spawn(async move {
        let (mut inbound, _) = timeout(Duration::from_secs(IO_TIMEOUT_SECONDS), listener.accept()).await??;
        let encrypted_request = read_frame(&mut inbound).await?;
        let request = decode_request_frame(protocol, &encrypted_request)?;
        let mut outbound = TcpStream::connect((HOST, request.target_port)).await?;
        let payload_bytes_to_target = write_frame(&mut outbound, &request.payload).await?;
        let target_response = read_frame(&mut outbound).await?;
        let encrypted_response = encode_response_frame(protocol, &target_response)?;
        let response_bytes_to_client = write_frame(&mut inbound, &encrypted_response).await?;
        inbound.shutdown().await?;
        outbound.shutdown().await?;
        Ok::<AdapterRecord, anyhow::Error>(AdapterRecord {
            handshake_validated: true,
            payload_bytes_to_target,
            response_bytes_to_client,
        })
    });

    let mut client = TcpStream::connect((HOST, listener_port)).await?;
    let request_frame = encode_request_frame(protocol, target_port, expected_request.as_bytes())?;
    let request_bytes_from_client = write_frame(&mut client, &request_frame).await?;
    let response_frame = read_frame(&mut client).await?;
    let response = decode_response_frame(protocol, &response_frame)?;
    client.shutdown().await?;

    let (payload_bytes_to_target, target_response_bytes, target_blockers) = target_task.await??;
    let adapter_record = adapter_task.await??;
    let response_marker = std::str::from_utf8(&response)
        .map(ToOwned::to_owned)
        .unwrap_or_default();
    let mut blockers = target_blockers;
    if response_marker != expected_response {
        blockers.push("client received unexpected encrypted protocol response".to_owned());
    }
    if adapter_record.payload_bytes_to_target == 0 || adapter_record.response_bytes_to_client == 0 {
        blockers.push("encrypted protocol byte accounting is empty".to_owned());
    }

    Ok(RustEncryptedProtocolsBundleSessionEvidence {
        protocol,
        adapter_name: adapter_name(protocol).to_owned(),
        listener_port,
        target_port,
        target_address: format!("{HOST}:{target_port}"),
        handshake_validated: adapter_record.handshake_validated,
        session_established: blockers.is_empty(),
        request_marker: expected_request,
        response_marker: Some(response_marker),
        request_bytes_from_client,
        payload_bytes_to_target,
        target_response_bytes,
        response_bytes_to_client: adapter_record.response_bytes_to_client,
        fallback_triggered: false,
        passed: blockers.is_empty()
            && payload_bytes_to_target > 0
            && target_response_bytes > 0
            && adapter_record.handshake_validated,
        blockers,
    })
}
