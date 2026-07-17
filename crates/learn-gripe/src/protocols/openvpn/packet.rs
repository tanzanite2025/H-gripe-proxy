//! OpenVPN packet opcodes, session IDs, control-packet codec, and the TCP
//! length-prefixed framing.
//!
//! Only the plaintext control channel is implemented (no `tls-auth` /
//! `tls-crypt` static wrapping): each control packet on the wire is
//! `opcode/key-id byte || 8-byte session id || plaintext body`.

use anyhow::{Result, bail};

/// Low 3 bits of the first packet byte carry the key id.
pub(super) const KEY_ID_MASK: u8 = 0x07;
/// The opcode occupies the high 5 bits of the first packet byte.
pub(super) const OPCODE_SHIFT: u8 = 3;

pub(super) const P_CONTROL_HARD_RESET_CLIENT_V2: u8 = 7;
pub(super) const P_CONTROL_HARD_RESET_CLIENT_V3: u8 = 10;
pub(super) const P_CONTROL_HARD_RESET_SERVER_V2: u8 = 8;
pub(super) const P_CONTROL_SOFT_RESET_V1: u8 = 3;
pub(super) const P_CONTROL_V1: u8 = 4;
pub(super) const P_ACK_V1: u8 = 5;
pub(super) const P_DATA_V1: u8 = 6;
pub(super) const P_DATA_V2: u8 = 9;

/// Session id length in bytes.
pub(super) const SESSION_ID_SIZE: usize = 8;
/// Plaintext control header: opcode/key-id byte + session id.
pub(super) const CONTROL_HEADER_SIZE: usize = 1 + SESSION_ID_SIZE;

/// An OpenVPN session id (random per side, echoed for ack routing).
pub(super) type SessionId = [u8; SESSION_ID_SIZE];

pub(super) fn opcode_key_id(opcode: u8, key_id: u8) -> u8 {
    (opcode << OPCODE_SHIFT) | (key_id & KEY_ID_MASK)
}

pub(super) fn parse_opcode_key_id(b: u8) -> (u8, u8) {
    (b >> OPCODE_SHIFT, b & KEY_ID_MASK)
}

pub(super) fn is_control(opcode: u8) -> bool {
    matches!(
        opcode,
        1 | 2
            | P_CONTROL_SOFT_RESET_V1
            | P_CONTROL_V1
            | P_ACK_V1
            | P_CONTROL_HARD_RESET_CLIENT_V2
            | P_CONTROL_HARD_RESET_SERVER_V2
            | P_CONTROL_HARD_RESET_CLIENT_V3
            | 11
    )
}

/// Whether a control opcode carries a reliable (acked) message id + payload.
/// Everything except a bare `P_ACK_V1` does.
pub(super) fn has_message_id(opcode: u8) -> bool {
    is_control(opcode) && opcode != P_ACK_V1
}

/// A decoded (or to-be-encoded) OpenVPN control packet.
#[derive(Debug, Clone)]
pub(super) struct ControlPacket {
    pub(super) opcode: u8,
    pub(super) key_id: u8,
    pub(super) local_session: SessionId,
    pub(super) ack_ids: Vec<u32>,
    pub(super) ack_remote_session: SessionId,
    pub(super) message_id: u32,
    pub(super) payload: Vec<u8>,
}

impl ControlPacket {
    /// Encode a plaintext control packet: header + ack array + optional message
    /// id + payload. Mirrors OpenVPN's reliability-layer wire order.
    pub(super) fn encode(&self) -> Result<Vec<u8>> {
        if !is_control(self.opcode) {
            bail!("openvpn: opcode {} is not a control opcode", self.opcode);
        }
        if self.ack_ids.len() > 255 {
            bail!("openvpn: too many ack ids: {}", self.ack_ids.len());
        }
        let mut out = Vec::with_capacity(CONTROL_HEADER_SIZE + 1 + self.ack_ids.len() * 4 + 4 + self.payload.len());
        out.push(opcode_key_id(self.opcode, self.key_id));
        out.extend_from_slice(&self.local_session);
        out.push(self.ack_ids.len() as u8);
        for id in &self.ack_ids {
            out.extend_from_slice(&id.to_be_bytes());
        }
        if !self.ack_ids.is_empty() {
            out.extend_from_slice(&self.ack_remote_session);
        }
        if has_message_id(self.opcode) {
            out.extend_from_slice(&self.message_id.to_be_bytes());
            out.extend_from_slice(&self.payload);
        }
        Ok(out)
    }

    /// Decode a plaintext control packet from a full framed packet (header
    /// included).
    pub(super) fn decode(packet: &[u8]) -> Result<Self> {
        if packet.len() < CONTROL_HEADER_SIZE + 1 {
            bail!("openvpn: control packet too short");
        }
        let (opcode, key_id) = parse_opcode_key_id(packet[0]);
        if !is_control(opcode) {
            bail!("openvpn: opcode {opcode} is not a control opcode");
        }
        let mut local_session = [0u8; SESSION_ID_SIZE];
        local_session.copy_from_slice(&packet[1..CONTROL_HEADER_SIZE]);

        let body = &packet[CONTROL_HEADER_SIZE..];
        let ack_len = body[0] as usize;
        let mut offset = 1;
        if body.len() < offset + ack_len * 4 {
            bail!("openvpn: control ack array truncated");
        }
        let mut ack_ids = Vec::with_capacity(ack_len);
        for _ in 0..ack_len {
            ack_ids.push(u32::from_be_bytes(body[offset..offset + 4].try_into().unwrap()));
            offset += 4;
        }
        let mut ack_remote_session = [0u8; SESSION_ID_SIZE];
        if ack_len > 0 {
            if body.len() < offset + SESSION_ID_SIZE {
                bail!("openvpn: control ack remote session truncated");
            }
            ack_remote_session.copy_from_slice(&body[offset..offset + SESSION_ID_SIZE]);
            offset += SESSION_ID_SIZE;
        }
        let mut message_id = 0u32;
        let mut payload = Vec::new();
        if has_message_id(opcode) {
            if body.len() < offset + 4 {
                bail!("openvpn: control message id truncated");
            }
            message_id = u32::from_be_bytes(body[offset..offset + 4].try_into().unwrap());
            offset += 4;
            payload = body[offset..].to_vec();
        } else if body.len() != offset {
            bail!("openvpn: ack packet has trailing payload");
        }

        Ok(Self {
            opcode,
            key_id,
            local_session,
            ack_ids,
            ack_remote_session,
            message_id,
            payload,
        })
    }
}

/// Generate a random session id from the OS RNG.
pub(super) fn new_session_id() -> Result<SessionId> {
    let mut id = [0u8; SESSION_ID_SIZE];
    getrandom::fill(&mut id).map_err(|_| anyhow::anyhow!("openvpn: system RNG unavailable"))?;
    Ok(id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn control_packet_round_trip() {
        let pkt = ControlPacket {
            opcode: P_CONTROL_V1,
            key_id: 0,
            local_session: [1, 2, 3, 4, 5, 6, 7, 8],
            ack_ids: vec![1, 2, 3],
            ack_remote_session: [9, 10, 11, 12, 13, 14, 15, 16],
            message_id: 42,
            payload: b"hello".to_vec(),
        };
        let encoded = pkt.encode().unwrap();
        let decoded = ControlPacket::decode(&encoded).unwrap();
        assert_eq!(decoded.opcode, P_CONTROL_V1);
        assert_eq!(decoded.local_session, pkt.local_session);
        assert_eq!(decoded.ack_ids, pkt.ack_ids);
        assert_eq!(decoded.ack_remote_session, pkt.ack_remote_session);
        assert_eq!(decoded.message_id, 42);
        assert_eq!(decoded.payload, b"hello");
    }

    #[test]
    fn ack_packet_has_no_message_id() {
        let pkt = ControlPacket {
            opcode: P_ACK_V1,
            key_id: 0,
            local_session: [1; 8],
            ack_ids: vec![7],
            ack_remote_session: [2; 8],
            message_id: 0,
            payload: Vec::new(),
        };
        let encoded = pkt.encode().unwrap();
        let decoded = ControlPacket::decode(&encoded).unwrap();
        assert_eq!(decoded.opcode, P_ACK_V1);
        assert_eq!(decoded.ack_ids, vec![7]);
        assert!(decoded.payload.is_empty());
    }

    #[test]
    fn opcode_key_id_packing() {
        let b = opcode_key_id(P_CONTROL_HARD_RESET_CLIENT_V2, 0);
        let (op, kid) = parse_opcode_key_id(b);
        assert_eq!(op, P_CONTROL_HARD_RESET_CLIENT_V2);
        assert_eq!(kid, 0);
        assert!(has_message_id(P_CONTROL_HARD_RESET_CLIENT_V2));
        assert!(!has_message_id(P_ACK_V1));
    }
}
