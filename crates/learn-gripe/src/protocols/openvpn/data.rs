//! OpenVPN AEAD data channel (`P_DATA_V2`).
//!
//! Only AEAD ciphers are implemented (AES-128/256-GCM and CHACHA20-POLY1305):
//! CBC + HMAC data ciphers are intentionally out of scope for this slice.
//!
//! Wire layout of an encrypted `P_DATA_V2` packet:
//! `opcode/key-id || 24-bit peer-id || 32-bit packet-id || 16-byte tag || ciphertext`.
//! The nonce is the per-direction implicit IV (`0x00000000 || first 8 bytes of
//! the direction's HMAC-key area`) with the packet id XORed into its first four
//! bytes; the AAD is `header || packet-id`.

use aes_gcm::aead::{Aead as AeadTrait, KeyInit, Payload};
use anyhow::{Result, bail};
use chacha20poly1305::ChaCha20Poly1305;

use super::keymethod::KeyMaterial;
use super::packet::{P_DATA_V1, P_DATA_V2, opcode_key_id, parse_opcode_key_id};

const TAG_SIZE: usize = 16;
const IV_SIZE: usize = 12;
const REPLAY_WINDOW: u32 = 64;

/// Peer id sentinel meaning "unset" (falls back to `P_DATA_V1` framing).
pub(super) const PEER_ID_UNSET: u32 = 0x00ff_ffff;

/// OpenVPN's fixed data-channel keepalive ping payload (`PING_STRING`).
pub(super) const PING_PACKET: [u8; 16] = [
    0x2a, 0x18, 0x7b, 0xf3, 0x64, 0x1e, 0xb4, 0xcb, 0x07, 0xed, 0x2d, 0x0a, 0x98, 0x1f, 0xc7, 0x48,
];

pub(super) fn is_ping(packet: &[u8]) -> bool {
    packet == PING_PACKET
}

enum Aead {
    Aes128(Box<aes_gcm::Aes128Gcm>),
    Aes256(Box<aes_gcm::Aes256Gcm>),
    Chacha(Box<ChaCha20Poly1305>),
}

impl Aead {
    fn new(cipher: &str, key: &[u8]) -> Result<Self> {
        Ok(match cipher {
            "AES-128-GCM" => Aead::Aes128(Box::new(
                aes_gcm::Aes128Gcm::new_from_slice(key).map_err(|_| anyhow::anyhow!("openvpn: bad AES-128 key"))?,
            )),
            "AES-256-GCM" => Aead::Aes256(Box::new(
                aes_gcm::Aes256Gcm::new_from_slice(key).map_err(|_| anyhow::anyhow!("openvpn: bad AES-256 key"))?,
            )),
            "CHACHA20-POLY1305" => Aead::Chacha(Box::new(
                ChaCha20Poly1305::new_from_slice(key).map_err(|_| anyhow::anyhow!("openvpn: bad ChaCha20 key"))?,
            )),
            other => bail!("openvpn: unsupported AEAD cipher {other:?}"),
        })
    }

    fn seal(&self, nonce: &[u8; IV_SIZE], aad: &[u8], plaintext: &[u8]) -> Result<Vec<u8>> {
        let payload = Payload { msg: plaintext, aad };
        let out = match self {
            Aead::Aes128(c) => c.encrypt(nonce.into(), payload),
            Aead::Aes256(c) => c.encrypt(nonce.into(), payload),
            Aead::Chacha(c) => c.encrypt(nonce.into(), payload),
        };
        out.map_err(|_| anyhow::anyhow!("openvpn: data channel encryption failed"))
    }

    fn open(&self, nonce: &[u8; IV_SIZE], aad: &[u8], ciphertext: &[u8]) -> Result<Vec<u8>> {
        let payload = Payload { msg: ciphertext, aad };
        let out = match self {
            Aead::Aes128(c) => c.decrypt(nonce.into(), payload),
            Aead::Aes256(c) => c.decrypt(nonce.into(), payload),
            Aead::Chacha(c) => c.decrypt(nonce.into(), payload),
        };
        out.map_err(|_| anyhow::anyhow!("openvpn: data channel decryption failed"))
    }
}

/// A bidirectional AEAD data channel bound to one negotiated key epoch.
pub(super) struct DataChannel {
    send: Aead,
    recv: Aead,
    send_iv: [u8; IV_SIZE],
    recv_iv: [u8; IV_SIZE],
    header: Vec<u8>,
    key_id: u8,
    send_packet_id: u32,
    recv_highest: u32,
    recv_window: u64,
    recv_seen: bool,
}

impl DataChannel {
    pub(super) fn new(keys: &KeyMaterial, cipher: &str, peer_id: u32, key_id: u8) -> Result<Self> {
        if keys.send_hmac_key.len() < IV_SIZE - 4 || keys.recv_hmac_key.len() < IV_SIZE - 4 {
            bail!("openvpn: implicit IV key material too short");
        }
        let mut send_iv = [0u8; IV_SIZE];
        let mut recv_iv = [0u8; IV_SIZE];
        send_iv[4..].copy_from_slice(&keys.send_hmac_key[..IV_SIZE - 4]);
        recv_iv[4..].copy_from_slice(&keys.recv_hmac_key[..IV_SIZE - 4]);
        Ok(Self {
            send: Aead::new(cipher, &keys.send_cipher_key)?,
            recv: Aead::new(cipher, &keys.recv_cipher_key)?,
            send_iv,
            recv_iv,
            header: data_header(peer_id, key_id),
            key_id,
            send_packet_id: 0,
            recv_highest: 0,
            recv_window: 0,
            recv_seen: false,
        })
    }

    /// The key id this channel's epoch was negotiated under.
    pub(super) fn key_id(&self) -> u8 {
        self.key_id
    }

    /// Encrypt one inner IP packet into a framed `P_DATA_V2` packet.
    pub(super) fn encrypt(&mut self, packet: &[u8]) -> Result<Vec<u8>> {
        self.send_packet_id = self.send_packet_id.wrapping_add(1);
        let packet_id = self.send_packet_id;
        let packet_id_bytes = packet_id.to_be_bytes();

        let mut aad = Vec::with_capacity(self.header.len() + 4);
        aad.extend_from_slice(&self.header);
        aad.extend_from_slice(&packet_id_bytes);

        let nonce = nonce(&self.send_iv, packet_id);
        let sealed = self.send.seal(&nonce, &aad, packet)?;
        if sealed.len() < TAG_SIZE {
            bail!("openvpn: sealed data too short");
        }
        let (ciphertext, tag) = sealed.split_at(sealed.len() - TAG_SIZE);

        let mut out = Vec::with_capacity(self.header.len() + 4 + TAG_SIZE + ciphertext.len());
        out.extend_from_slice(&self.header);
        out.extend_from_slice(&packet_id_bytes);
        out.extend_from_slice(tag);
        out.extend_from_slice(ciphertext);
        Ok(out)
    }

    /// Decrypt a framed data packet, returning the inner IP packet. Rejects
    /// replayed packet ids within the sliding window.
    pub(super) fn decrypt(&mut self, packet: &[u8]) -> Result<Vec<u8>> {
        let header_size = data_packet_header_size(packet)?;
        if packet.len() < header_size + 4 + TAG_SIZE + 1 {
            bail!("openvpn: data packet too short");
        }
        let header = &packet[..header_size];
        let packet_id_bytes = &packet[header_size..header_size + 4];
        let packet_id = u32::from_be_bytes(packet_id_bytes.try_into().unwrap());
        let tag = &packet[header_size + 4..header_size + 4 + TAG_SIZE];
        let ciphertext = &packet[header_size + 4 + TAG_SIZE..];

        let mut combined = Vec::with_capacity(ciphertext.len() + TAG_SIZE);
        combined.extend_from_slice(ciphertext);
        combined.extend_from_slice(tag);

        let mut aad = Vec::with_capacity(header.len() + 4);
        aad.extend_from_slice(header);
        aad.extend_from_slice(packet_id_bytes);

        let nonce = nonce(&self.recv_iv, packet_id);
        let plain = self.recv.open(&nonce, &aad, &combined)?;
        self.accept_packet_id(packet_id)?;
        Ok(plain)
    }

    /// Sliding-window replay check mirroring OpenVPN's 64-entry bitmap.
    fn accept_packet_id(&mut self, packet_id: u32) -> Result<()> {
        if !self.recv_seen {
            self.recv_highest = packet_id;
            self.recv_window = 1;
            self.recv_seen = true;
            return Ok(());
        }
        if packet_id > self.recv_highest {
            let shift = packet_id - self.recv_highest;
            self.recv_window = if shift >= REPLAY_WINDOW {
                1
            } else {
                (self.recv_window << shift) | 1
            };
            self.recv_highest = packet_id;
            return Ok(());
        }
        let diff = self.recv_highest - packet_id;
        if diff >= REPLAY_WINDOW {
            bail!("openvpn: replayed data packet id {packet_id}");
        }
        let mask = 1u64 << diff;
        if self.recv_window & mask != 0 {
            bail!("openvpn: replayed data packet id {packet_id}");
        }
        self.recv_window |= mask;
        Ok(())
    }
}

fn data_header(peer_id: u32, key_id: u8) -> Vec<u8> {
    if peer_id != PEER_ID_UNSET {
        vec![
            opcode_key_id(P_DATA_V2, key_id),
            (peer_id >> 16) as u8,
            (peer_id >> 8) as u8,
            peer_id as u8,
        ]
    } else {
        vec![opcode_key_id(P_DATA_V1, key_id)]
    }
}

fn data_packet_header_size(packet: &[u8]) -> Result<usize> {
    if packet.is_empty() {
        bail!("openvpn: empty data packet");
    }
    let (opcode, _) = parse_opcode_key_id(packet[0]);
    match opcode {
        P_DATA_V1 => Ok(1),
        P_DATA_V2 => {
            if packet.len() < 4 {
                bail!("openvpn: P_DATA_V2 packet missing peer id");
            }
            Ok(4)
        }
        _ => bail!("openvpn: not a data packet (opcode {opcode})"),
    }
}

fn nonce(implicit: &[u8; IV_SIZE], packet_id: u32) -> [u8; IV_SIZE] {
    let mut nonce = *implicit;
    let head = u32::from_be_bytes(nonce[..4].try_into().unwrap()) ^ packet_id;
    nonce[..4].copy_from_slice(&head.to_be_bytes());
    nonce
}

#[cfg(test)]
mod tests {
    use super::*;

    fn keys() -> KeyMaterial {
        // Symmetric key material where send/recv are swapped between two peers.
        KeyMaterial {
            send_cipher_key: vec![1u8; 32],
            send_hmac_key: vec![2u8; 32],
            recv_cipher_key: vec![3u8; 32],
            recv_hmac_key: vec![4u8; 32],
        }
    }

    #[test]
    fn encrypt_decrypt_round_trip() {
        let client_keys = keys();
        // The server's channel mirrors the client's directions.
        let server_keys = KeyMaterial {
            send_cipher_key: client_keys.recv_cipher_key.clone(),
            send_hmac_key: client_keys.recv_hmac_key.clone(),
            recv_cipher_key: client_keys.send_cipher_key.clone(),
            recv_hmac_key: client_keys.send_hmac_key.clone(),
        };
        let mut client = DataChannel::new(&client_keys, "AES-256-GCM", 3, 0).unwrap();
        let mut server = DataChannel::new(&server_keys, "AES-256-GCM", 3, 0).unwrap();

        let msg = b"an inner ip packet payload";
        let sealed = client.encrypt(msg).unwrap();
        assert_eq!(sealed[0] >> 3, P_DATA_V2);
        let plain = server.decrypt(&sealed).unwrap();
        assert_eq!(plain, msg);
    }

    #[test]
    fn replayed_packet_is_rejected() {
        let client_keys = keys();
        let server_keys = KeyMaterial {
            send_cipher_key: client_keys.recv_cipher_key.clone(),
            send_hmac_key: client_keys.recv_hmac_key.clone(),
            recv_cipher_key: client_keys.send_cipher_key.clone(),
            recv_hmac_key: client_keys.send_hmac_key.clone(),
        };
        let mut client = DataChannel::new(&client_keys, "AES-256-GCM", 3, 0).unwrap();
        let mut server = DataChannel::new(&server_keys, "AES-256-GCM", 3, 0).unwrap();
        let sealed = client.encrypt(b"hello").unwrap();
        assert!(server.decrypt(&sealed).is_ok());
        assert!(server.decrypt(&sealed).is_err(), "replay must be rejected");
    }

    #[test]
    fn chacha_round_trip() {
        let client_keys = keys();
        let server_keys = KeyMaterial {
            send_cipher_key: client_keys.recv_cipher_key.clone(),
            send_hmac_key: client_keys.recv_hmac_key.clone(),
            recv_cipher_key: client_keys.send_cipher_key.clone(),
            recv_hmac_key: client_keys.send_hmac_key.clone(),
        };
        let mut client = DataChannel::new(&client_keys, "CHACHA20-POLY1305", 7, 1).unwrap();
        let mut server = DataChannel::new(&server_keys, "CHACHA20-POLY1305", 7, 1).unwrap();
        let sealed = client.encrypt(b"chacha payload").unwrap();
        assert_eq!(sealed[0] & 0x07, 1, "key id carried in the header");
        assert_eq!(server.decrypt(&sealed).unwrap(), b"chacha payload");
    }
}
