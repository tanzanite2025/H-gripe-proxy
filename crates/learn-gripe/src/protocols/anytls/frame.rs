use crate::address::TargetAddr;
use crate::inbound::socks5;

use super::padding::PaddingScheme;

// Session-layer commands (anytls protocol, "since version 1" + "since version 2").
pub(super) const CMD_WASTE: u8 = 0;
pub(super) const CMD_SYN: u8 = 1;
pub(super) const CMD_PSH: u8 = 2;
pub(super) const CMD_FIN: u8 = 3;
pub(super) const CMD_SETTINGS: u8 = 4;
pub(super) const CMD_ALERT: u8 = 5;
pub(super) const CMD_UPDATE_PADDING_SCHEME: u8 = 6;
pub(super) const CMD_SYNACK: u8 = 7;
pub(super) const CMD_HEART_REQUEST: u8 = 8;
pub(super) const CMD_HEART_RESPONSE: u8 = 9;
pub(super) const CMD_SERVER_SETTINGS: u8 = 10;

pub(super) const FRAME_HEADER_LEN: usize = 7;
/// The first stream id opened on a fresh session. anytls stream ids are
/// monotonic within a session; reusing a pooled connection opens the next id,
/// so leftover frames from an earlier (closed) stream carry an older id and are
/// skipped by the new stream.
pub(super) const STREAM_ID: u32 = 1;
/// Cap on a single `cmdPSH` payload (the frame length field is a `u16`); a
/// comfortable margin keeps frames small without excessive overhead.
pub(super) const MAX_PSH_CHUNK: usize = 8192;

/// Implemented protocol version reported in `cmdSettings` (`v=2`).
pub(super) const PROTOCOL_VERSION: u8 = 2;
/// `client` identifier reported in `cmdSettings` (real name, per the spec —
/// spoofing it is pointless).
pub(super) const CLIENT_NAME: &str = concat!("learn-gripe/", env!("CARGO_PKG_VERSION"));

/// Append one session frame (`cmd | streamId | len | data`) to `buf`.
pub(super) fn push_frame(buf: &mut Vec<u8>, cmd: u8, stream_id: u32, data: &[u8]) {
    buf.push(cmd);
    buf.extend_from_slice(&stream_id.to_be_bytes());
    buf.extend_from_slice(&(data.len() as u16).to_be_bytes());
    buf.extend_from_slice(data);
}

/// Append a `cmdWaste` frame carrying `payload_len` zero bytes of padding.
pub(super) fn push_waste(buf: &mut Vec<u8>, payload_len: usize) {
    buf.push(CMD_WASTE);
    buf.extend_from_slice(&0u32.to_be_bytes());
    buf.extend_from_slice(&(payload_len as u16).to_be_bytes());
    buf.resize(buf.len() + payload_len, 0);
}

/// Build the auth header (packet 0): `SHA256(password)`, the `padding0` length,
/// then that many zero bytes. `padding0` is the scheme's packet-0 size (anytls
/// `GenerateRecordPayloadSizes(0)[0]`); the default scheme yields 30 bytes.
pub(super) fn build_auth_header(password_sha256: &[u8; 32], scheme: &PaddingScheme) -> Vec<u8> {
    let padding0 = match scheme.record_payload_sizes(0).first().copied() {
        Some(size) if size > 0 => size as usize,
        _ => 0,
    };
    let mut buf = Vec::with_capacity(32 + 2 + padding0);
    buf.extend_from_slice(password_sha256);
    buf.extend_from_slice(&(padding0 as u16).to_be_bytes());
    buf.resize(buf.len() + padding0, 0);
    buf
}

/// Build the packet-1 session bytes for the session's first stream:
/// `cmdSettings` (advertising the scheme md5), `cmdSYN` opening stream `sid`, and
/// the `cmdPSH` carrying the SOCKS5-encoded proxy target. The caller feeds the
/// whole blob through the padding shaper as a single `writeConn` unit.
pub(super) fn build_session_init(scheme: &PaddingScheme, sid: u32, target: &TargetAddr) -> Vec<u8> {
    let mut buf = Vec::with_capacity(64 + FRAME_HEADER_LEN * 2 + 64);
    let settings = format!(
        "v={PROTOCOL_VERSION}\nclient={CLIENT_NAME}\npadding-md5={}",
        scheme.md5_hex
    );
    push_frame(&mut buf, CMD_SETTINGS, 0, settings.as_bytes());
    push_frame(&mut buf, CMD_SYN, sid, &[]);
    let mut addr = Vec::with_capacity(1 + 256 + 2);
    socks5::encode_address(&mut addr, target);
    push_frame(&mut buf, CMD_PSH, sid, &addr);
    buf
}

/// Build the bytes opening a stream on an already-established (pooled) session:
/// `cmdSYN` with the next stream id, then the `cmdPSH` carrying the SOCKS5-encoded
/// proxy target. No `cmdSettings` — that is sent once at session creation. The
/// caller feeds the blob through the session's shaper as one `writeConn` unit.
pub(super) fn build_stream_open(stream_id: u32, target: &TargetAddr) -> Vec<u8> {
    let mut buf = Vec::with_capacity(FRAME_HEADER_LEN * 2 + 64);
    push_frame(&mut buf, CMD_SYN, stream_id, &[]);
    let mut addr = Vec::with_capacity(1 + 256 + 2);
    socks5::encode_address(&mut addr, target);
    push_frame(&mut buf, CMD_PSH, stream_id, &addr);
    buf
}
