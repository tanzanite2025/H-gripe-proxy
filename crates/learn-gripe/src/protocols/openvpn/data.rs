//! OpenVPN data channel (`P_DATA_V2`), AEAD and classic CBC+HMAC.
//!
//! Two data-cipher families are implemented:
//!
//! - **AEAD** (`AES-128/256-GCM`, `CHACHA20-POLY1305`). Wire layout of an
//!   encrypted packet:
//!   `opcode/key-id || 24-bit peer-id || 32-bit packet-id || 16-byte tag || ciphertext`.
//!   The nonce is the per-direction implicit IV (`0x00000000 || first 8 bytes of
//!   the direction's HMAC-key area`) with the packet id XORed into its first four
//!   bytes; the AAD is `header || packet-id`.
//!
//! - **CBC + HMAC** (`AES-128/192/256-CBC` with an `SHA1`/`SHA256`/`SHA512`
//!   `auth` digest). Wire layout of an encrypted packet:
//!   `opcode/key-id || 24-bit peer-id || HMAC || IV || CBC(packet-id || plaintext)`.
//!   The HMAC (of the negotiated digest, keyed by the direction's HMAC key) is
//!   computed over `IV || ciphertext`; the 32-bit packet id lives *inside* the
//!   encrypted plaintext (PKCS#7-padded), unlike the AEAD framing where it is in
//!   the clear. Both families share the 64-entry sliding replay window.

use aes::cipher::block_padding::Pkcs7;
use aes::cipher::{BlockDecryptMut, BlockEncryptMut, KeyIvInit};
use aes::{Aes128, Aes192, Aes256};
use aes_gcm::aead::{Aead as AeadTrait, KeyInit, Payload};
use anyhow::{Result, bail};
use chacha20poly1305::ChaCha20Poly1305;

use super::keymethod::KeyMaterial;
use super::packet::{P_DATA_V1, P_DATA_V2, opcode_key_id, parse_opcode_key_id};
use super::tlswrap::{AuthDigest, constant_time_eq};

const TAG_SIZE: usize = 16;
const IV_SIZE: usize = 12;
const REPLAY_WINDOW: u32 = 64;
/// AES block size — the CBC IV length and ciphertext block granularity.
const CBC_IV_SIZE: usize = 16;

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

/// One direction of a classic CBC data channel: the AES key (with its variant)
/// plus the HMAC key. A fresh CBC cipher is instantiated per packet because the
/// IV is random per packet.
struct CbcKey {
    variant: CbcVariant,
    cipher_key: Vec<u8>,
    hmac_key: Vec<u8>,
}

#[derive(Clone, Copy)]
enum CbcVariant {
    Aes128,
    Aes192,
    Aes256,
}

impl CbcVariant {
    fn from_cipher(cipher: &str) -> Option<Self> {
        match cipher {
            "AES-128-CBC" => Some(Self::Aes128),
            "AES-192-CBC" => Some(Self::Aes192),
            "AES-256-CBC" => Some(Self::Aes256),
            _ => None,
        }
    }

    fn key_len(self) -> usize {
        match self {
            Self::Aes128 => 16,
            Self::Aes192 => 24,
            Self::Aes256 => 32,
        }
    }
}

impl CbcKey {
    fn new(variant: CbcVariant, cipher_key: &[u8], hmac_key: &[u8], digest: AuthDigest) -> Result<Self> {
        if cipher_key.len() < variant.key_len() {
            bail!("openvpn: CBC cipher key material too short");
        }
        if hmac_key.len() < digest.size() {
            bail!("openvpn: CBC HMAC key material too short");
        }
        Ok(Self {
            variant,
            cipher_key: cipher_key[..variant.key_len()].to_vec(),
            hmac_key: hmac_key[..digest.size()].to_vec(),
        })
    }

    fn encrypt_blocks(&self, iv: &[u8; CBC_IV_SIZE], plaintext: &[u8]) -> Vec<u8> {
        match self.variant {
            CbcVariant::Aes128 => cbc::Encryptor::<Aes128>::new(self.cipher_key.as_slice().into(), iv.into())
                .encrypt_padded_vec_mut::<Pkcs7>(plaintext),
            CbcVariant::Aes192 => cbc::Encryptor::<Aes192>::new(self.cipher_key.as_slice().into(), iv.into())
                .encrypt_padded_vec_mut::<Pkcs7>(plaintext),
            CbcVariant::Aes256 => cbc::Encryptor::<Aes256>::new(self.cipher_key.as_slice().into(), iv.into())
                .encrypt_padded_vec_mut::<Pkcs7>(plaintext),
        }
    }

    fn decrypt_blocks(&self, iv: &[u8; CBC_IV_SIZE], ciphertext: &[u8]) -> Result<Vec<u8>> {
        let out = match self.variant {
            CbcVariant::Aes128 => cbc::Decryptor::<Aes128>::new(self.cipher_key.as_slice().into(), iv.into())
                .decrypt_padded_vec_mut::<Pkcs7>(ciphertext),
            CbcVariant::Aes192 => cbc::Decryptor::<Aes192>::new(self.cipher_key.as_slice().into(), iv.into())
                .decrypt_padded_vec_mut::<Pkcs7>(ciphertext),
            CbcVariant::Aes256 => cbc::Decryptor::<Aes256>::new(self.cipher_key.as_slice().into(), iv.into())
                .decrypt_padded_vec_mut::<Pkcs7>(ciphertext),
        };
        out.map_err(|_| anyhow::anyhow!("openvpn: CBC padding/decrypt failed"))
    }
}

/// The negotiated data-cipher family for one epoch.
enum DataCrypto {
    Aead {
        send: Aead,
        recv: Aead,
        send_iv: [u8; IV_SIZE],
        recv_iv: [u8; IV_SIZE],
    },
    Cbc {
        send: CbcKey,
        recv: CbcKey,
        digest: AuthDigest,
    },
}

/// A bidirectional data channel bound to one negotiated key epoch.
pub(super) struct DataChannel {
    crypto: DataCrypto,
    header: Vec<u8>,
    key_id: u8,
    send_packet_id: u32,
    recv_highest: u32,
    recv_window: u64,
    recv_seen: bool,
}

impl DataChannel {
    pub(super) fn new(keys: &KeyMaterial, cipher: &str, auth: AuthDigest, peer_id: u32, key_id: u8) -> Result<Self> {
        let crypto = if let Some(variant) = CbcVariant::from_cipher(cipher) {
            DataCrypto::Cbc {
                send: CbcKey::new(variant, &keys.send_cipher_key, &keys.send_hmac_key, auth)?,
                recv: CbcKey::new(variant, &keys.recv_cipher_key, &keys.recv_hmac_key, auth)?,
                digest: auth,
            }
        } else {
            if keys.send_hmac_key.len() < IV_SIZE - 4 || keys.recv_hmac_key.len() < IV_SIZE - 4 {
                bail!("openvpn: implicit IV key material too short");
            }
            let mut send_iv = [0u8; IV_SIZE];
            let mut recv_iv = [0u8; IV_SIZE];
            send_iv[4..].copy_from_slice(&keys.send_hmac_key[..IV_SIZE - 4]);
            recv_iv[4..].copy_from_slice(&keys.recv_hmac_key[..IV_SIZE - 4]);
            DataCrypto::Aead {
                send: Aead::new(cipher, &keys.send_cipher_key)?,
                recv: Aead::new(cipher, &keys.recv_cipher_key)?,
                send_iv,
                recv_iv,
            }
        };
        Ok(Self {
            crypto,
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

        match &self.crypto {
            DataCrypto::Aead { send, send_iv, .. } => {
                let mut aad = Vec::with_capacity(self.header.len() + 4);
                aad.extend_from_slice(&self.header);
                aad.extend_from_slice(&packet_id_bytes);

                let nonce = nonce(send_iv, packet_id);
                let sealed = send.seal(&nonce, &aad, packet)?;
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
            DataCrypto::Cbc { send, digest, .. } => {
                let mut iv = [0u8; CBC_IV_SIZE];
                getrandom::fill(&mut iv).map_err(|_| anyhow::anyhow!("openvpn: system RNG unavailable"))?;

                let mut plaintext = Vec::with_capacity(4 + packet.len());
                plaintext.extend_from_slice(&packet_id_bytes);
                plaintext.extend_from_slice(packet);
                let ciphertext = send.encrypt_blocks(&iv, &plaintext);
                let hmac = digest.mac(&send.hmac_key, &[&iv, &ciphertext]);

                let mut out = Vec::with_capacity(self.header.len() + hmac.len() + CBC_IV_SIZE + ciphertext.len());
                out.extend_from_slice(&self.header);
                out.extend_from_slice(&hmac);
                out.extend_from_slice(&iv);
                out.extend_from_slice(&ciphertext);
                Ok(out)
            }
        }
    }

    /// Decrypt a framed data packet, returning the inner IP packet. Rejects
    /// replayed packet ids within the sliding window.
    pub(super) fn decrypt(&mut self, packet: &[u8]) -> Result<Vec<u8>> {
        let header_size = data_packet_header_size(packet)?;
        let (plain, packet_id) = match &self.crypto {
            DataCrypto::Aead { recv, recv_iv, .. } => {
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

                let nonce = nonce(recv_iv, packet_id);
                (recv.open(&nonce, &aad, &combined)?, packet_id)
            }
            DataCrypto::Cbc { recv, digest, .. } => {
                let hmac_len = digest.size();
                if packet.len() < header_size + hmac_len + CBC_IV_SIZE + CBC_IV_SIZE {
                    bail!("openvpn: data packet too short");
                }
                let hmac = &packet[header_size..header_size + hmac_len];
                let iv: [u8; CBC_IV_SIZE] = packet[header_size + hmac_len..header_size + hmac_len + CBC_IV_SIZE]
                    .try_into()
                    .unwrap();
                let ciphertext = &packet[header_size + hmac_len + CBC_IV_SIZE..];
                if ciphertext.len() % CBC_IV_SIZE != 0 {
                    bail!("openvpn: CBC ciphertext not block-aligned");
                }

                let expected = digest.mac(&recv.hmac_key, &[&iv, ciphertext]);
                if !constant_time_eq(&expected, hmac) {
                    bail!("openvpn: data channel HMAC authentication failed");
                }

                let plaintext = recv.decrypt_blocks(&iv, ciphertext)?;
                if plaintext.len() < 4 {
                    bail!("openvpn: CBC plaintext missing packet id");
                }
                let packet_id = u32::from_be_bytes(plaintext[..4].try_into().unwrap());
                (plaintext[4..].to_vec(), packet_id)
            }
        };
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
            send_hmac_key: vec![2u8; 64],
            recv_cipher_key: vec![3u8; 32],
            recv_hmac_key: vec![4u8; 64],
        }
    }

    /// The peer's channel mirrors this side's directions (send<->recv swapped).
    fn mirror(k: &KeyMaterial) -> KeyMaterial {
        KeyMaterial {
            send_cipher_key: k.recv_cipher_key.clone(),
            send_hmac_key: k.recv_hmac_key.clone(),
            recv_cipher_key: k.send_cipher_key.clone(),
            recv_hmac_key: k.send_hmac_key.clone(),
        }
    }

    #[test]
    fn encrypt_decrypt_round_trip() {
        let client_keys = keys();
        let server_keys = mirror(&client_keys);
        let mut client = DataChannel::new(&client_keys, "AES-256-GCM", AuthDigest::Sha1, 3, 0).unwrap();
        let mut server = DataChannel::new(&server_keys, "AES-256-GCM", AuthDigest::Sha1, 3, 0).unwrap();

        let msg = b"an inner ip packet payload";
        let sealed = client.encrypt(msg).unwrap();
        assert_eq!(sealed[0] >> 3, P_DATA_V2);
        let plain = server.decrypt(&sealed).unwrap();
        assert_eq!(plain, msg);
    }

    #[test]
    fn replayed_packet_is_rejected() {
        let client_keys = keys();
        let server_keys = mirror(&client_keys);
        let mut client = DataChannel::new(&client_keys, "AES-256-GCM", AuthDigest::Sha1, 3, 0).unwrap();
        let mut server = DataChannel::new(&server_keys, "AES-256-GCM", AuthDigest::Sha1, 3, 0).unwrap();
        let sealed = client.encrypt(b"hello").unwrap();
        assert!(server.decrypt(&sealed).is_ok());
        assert!(server.decrypt(&sealed).is_err(), "replay must be rejected");
    }

    #[test]
    fn chacha_round_trip() {
        let client_keys = keys();
        let server_keys = mirror(&client_keys);
        let mut client = DataChannel::new(&client_keys, "CHACHA20-POLY1305", AuthDigest::Sha1, 7, 1).unwrap();
        let mut server = DataChannel::new(&server_keys, "CHACHA20-POLY1305", AuthDigest::Sha1, 7, 1).unwrap();
        let sealed = client.encrypt(b"chacha payload").unwrap();
        assert_eq!(sealed[0] & 0x07, 1, "key id carried in the header");
        assert_eq!(server.decrypt(&sealed).unwrap(), b"chacha payload");
    }

    #[test]
    fn cbc_round_trip_all_variants_and_digests() {
        for cipher in ["AES-128-CBC", "AES-192-CBC", "AES-256-CBC"] {
            for digest in [AuthDigest::Sha1, AuthDigest::Sha256, AuthDigest::Sha512] {
                let client_keys = keys();
                let server_keys = mirror(&client_keys);
                let mut client = DataChannel::new(&client_keys, cipher, digest, 5, 0).unwrap();
                let mut server = DataChannel::new(&server_keys, cipher, digest, 5, 0).unwrap();

                // Exercise a payload that is not block-aligned so PKCS#7 padding
                // and the in-ciphertext packet id are both covered.
                let msg = b"cbc inner ip packet payload of odd length";
                let sealed = client.encrypt(msg).unwrap();
                assert_eq!(sealed[0] >> 3, P_DATA_V2);
                assert_eq!(server.decrypt(&sealed).unwrap(), msg, "{cipher} {}", digest.name());
            }
        }
    }

    #[test]
    fn cbc_replay_and_tamper_are_rejected() {
        let client_keys = keys();
        let server_keys = mirror(&client_keys);
        let mut client = DataChannel::new(&client_keys, "AES-256-CBC", AuthDigest::Sha256, 5, 0).unwrap();
        let mut server = DataChannel::new(&server_keys, "AES-256-CBC", AuthDigest::Sha256, 5, 0).unwrap();

        let sealed = client.encrypt(b"hello cbc").unwrap();
        assert_eq!(server.decrypt(&sealed).unwrap(), b"hello cbc");
        assert!(server.decrypt(&sealed).is_err(), "replay must be rejected");

        // Flipping a ciphertext bit must fail the HMAC before decryption.
        let mut tampered = client.encrypt(b"tamper me").unwrap();
        let last = tampered.len() - 1;
        tampered[last] ^= 0x01;
        assert!(server.decrypt(&tampered).is_err(), "HMAC must reject tampering");
    }
}
