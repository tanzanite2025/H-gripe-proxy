use super::{RustEncryptedProtocolBundleProtocol, constants::CANARY_PASSWORD};
use anyhow::{Result, anyhow, bail};
use sha2::{Digest, Sha224};

#[derive(Debug, Clone)]
pub(super) struct ProtocolRequest {
    pub(super) target_port: u16,
    pub(super) payload: Vec<u8>,
}

pub(super) fn adapter_name(protocol: RustEncryptedProtocolBundleProtocol) -> &'static str {
    match protocol {
        RustEncryptedProtocolBundleProtocol::VmessTcp => "vmess-loopback-tcp-canary",
        RustEncryptedProtocolBundleProtocol::VlessTcp => "vless-loopback-tcp-canary",
        RustEncryptedProtocolBundleProtocol::TrojanTcp => "trojan-loopback-tcp-canary",
    }
}

pub(super) fn request_marker(protocol: RustEncryptedProtocolBundleProtocol) -> String {
    format!("{}-request-marker", protocol_token(protocol))
}

pub(super) fn response_marker(protocol: RustEncryptedProtocolBundleProtocol) -> String {
    format!("{}-response-marker", protocol_token(protocol))
}

pub(super) fn encode_request_frame(
    protocol: RustEncryptedProtocolBundleProtocol,
    target_port: u16,
    payload: &[u8],
) -> Result<Vec<u8>> {
    let mut frame = Vec::new();
    match protocol {
        RustEncryptedProtocolBundleProtocol::VmessTcp => {
            frame.extend_from_slice(b"VMESS1");
            frame.extend_from_slice(&target_port.to_be_bytes());
            push_payload(&mut frame, payload)?;
        }
        RustEncryptedProtocolBundleProtocol::VlessTcp => {
            frame.extend_from_slice(b"VLESS1");
            frame.push(0);
            frame.extend_from_slice(&target_port.to_be_bytes());
            push_payload(&mut frame, payload)?;
        }
        RustEncryptedProtocolBundleProtocol::TrojanTcp => {
            frame.extend_from_slice(b"TROJAN1");
            frame.extend_from_slice(trojan_password_hash().as_bytes());
            frame.extend_from_slice(b"\r\n");
            frame.extend_from_slice(&target_port.to_be_bytes());
            push_payload(&mut frame, payload)?;
        }
    }
    Ok(frame)
}

pub(super) fn decode_request_frame(
    protocol: RustEncryptedProtocolBundleProtocol,
    frame: &[u8],
) -> Result<ProtocolRequest> {
    match protocol {
        RustEncryptedProtocolBundleProtocol::VmessTcp => decode_vmess(frame),
        RustEncryptedProtocolBundleProtocol::VlessTcp => decode_vless(frame),
        RustEncryptedProtocolBundleProtocol::TrojanTcp => decode_trojan(frame),
    }
}

pub(super) fn encode_response_frame(protocol: RustEncryptedProtocolBundleProtocol, payload: &[u8]) -> Result<Vec<u8>> {
    let mut frame = Vec::new();
    frame.extend_from_slice(protocol_token(protocol).as_bytes());
    frame.push(b':');
    push_payload(&mut frame, payload)?;
    Ok(frame)
}

pub(super) fn decode_response_frame(protocol: RustEncryptedProtocolBundleProtocol, frame: &[u8]) -> Result<Vec<u8>> {
    let prefix = format!("{}:", protocol_token(protocol));
    let payload = frame
        .strip_prefix(prefix.as_bytes())
        .ok_or_else(|| anyhow!("encrypted protocol response prefix mismatch"))?;
    pop_payload(payload)
}

fn decode_vmess(frame: &[u8]) -> Result<ProtocolRequest> {
    let body = frame
        .strip_prefix(b"VMESS1")
        .ok_or_else(|| anyhow!("VMess canary frame marker mismatch"))?;
    decode_port_payload(body)
}

fn decode_vless(frame: &[u8]) -> Result<ProtocolRequest> {
    let body = frame
        .strip_prefix(b"VLESS1")
        .ok_or_else(|| anyhow!("VLESS canary frame marker mismatch"))?;
    if body.first() != Some(&0) {
        bail!("VLESS canary command is not TCP connect");
    }
    decode_port_payload(&body[1..])
}

fn decode_trojan(frame: &[u8]) -> Result<ProtocolRequest> {
    let body = frame
        .strip_prefix(b"TROJAN1")
        .ok_or_else(|| anyhow!("Trojan canary frame marker mismatch"))?;
    let expected_hash = trojan_password_hash();
    let body = body
        .strip_prefix(expected_hash.as_bytes())
        .ok_or_else(|| anyhow!("Trojan canary password hash mismatch"))?;
    let body = body
        .strip_prefix(b"\r\n")
        .ok_or_else(|| anyhow!("Trojan canary CRLF missing"))?;
    decode_port_payload(body)
}

fn decode_port_payload(body: &[u8]) -> Result<ProtocolRequest> {
    if body.len() < 4 {
        bail!("encrypted protocol canary frame is too short");
    }
    let target_port = u16::from_be_bytes([body[0], body[1]]);
    let payload = pop_payload(&body[2..])?;
    Ok(ProtocolRequest { target_port, payload })
}

fn push_payload(frame: &mut Vec<u8>, payload: &[u8]) -> Result<()> {
    let length = u16::try_from(payload.len())?;
    frame.extend_from_slice(&length.to_be_bytes());
    frame.extend_from_slice(payload);
    Ok(())
}

fn pop_payload(body: &[u8]) -> Result<Vec<u8>> {
    if body.len() < 2 {
        bail!("encrypted protocol payload length is missing");
    }
    let payload_len = usize::from(u16::from_be_bytes([body[0], body[1]]));
    let payload = body
        .get(2..2 + payload_len)
        .ok_or_else(|| anyhow!("encrypted protocol payload is truncated"))?;
    Ok(payload.to_vec())
}

fn trojan_password_hash() -> String {
    let mut hasher = Sha224::new();
    hasher.update(CANARY_PASSWORD.as_bytes());
    format!("{:x}", hasher.finalize())
}

fn protocol_token(protocol: RustEncryptedProtocolBundleProtocol) -> &'static str {
    match protocol {
        RustEncryptedProtocolBundleProtocol::VmessTcp => "vmess",
        RustEncryptedProtocolBundleProtocol::VlessTcp => "vless",
        RustEncryptedProtocolBundleProtocol::TrojanTcp => "trojan",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decodes_all_canary_request_frames() {
        for protocol in [
            RustEncryptedProtocolBundleProtocol::VmessTcp,
            RustEncryptedProtocolBundleProtocol::VlessTcp,
            RustEncryptedProtocolBundleProtocol::TrojanTcp,
        ] {
            let frame = encode_request_frame(protocol, 18080, b"hello").unwrap();
            let request = decode_request_frame(protocol, &frame).unwrap();

            assert_eq!(request.target_port, 18080);
            assert_eq!(request.payload, b"hello");
        }
    }
}
