//! Snell's session crypto primitives: the version-selected AEAD cipher family,
//! the Argon2id session-subkey KDF and the counter-nonce helpers shared by the
//! shadowaead chunk stream, the v4 frame stream and both UDP paths.

use std::io;

use aes_gcm::Aes128Gcm;
use aes_gcm::aead::generic_array::GenericArray;
use aes_gcm::aead::{Aead, KeyInit, Payload};
use anyhow::{Result, anyhow};
use argon2::{Algorithm, Argon2, Params, Version};
use chacha20poly1305::ChaCha20Poly1305;

/// AEAD cipher selected by Snell protocol version.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum SnellCipher {
    /// v1: ChaCha20-Poly1305 with a 32-byte key.
    Chacha20Poly1305,
    /// v2/v3: AES-128-GCM with a 16-byte key.
    Aes128Gcm,
}

impl SnellCipher {
    pub(super) fn key_size(self) -> usize {
        match self {
            SnellCipher::Chacha20Poly1305 => 32,
            SnellCipher::Aes128Gcm => 16,
        }
    }
}

/// Snell's session-subkey KDF: `argon2id(psk, salt, t=3, m=8 KiB, p=1, 32)`
/// truncated to the cipher key length.
pub(super) fn snell_kdf(psk: &[u8], salt: &[u8], key_size: usize) -> Vec<u8> {
    let params = Params::new(8, 3, 1, Some(32)).expect("valid snell argon2 params");
    let argon2 = Argon2::new(Algorithm::Argon2id, Version::V0x13, params);
    let mut out = [0u8; 32];
    argon2
        .hash_password_into(psk, salt, &mut out)
        .expect("snell argon2 kdf");
    out[..key_size].to_vec()
}

/// Fill `buf` with cryptographically secure random bytes from the OS.
pub(super) fn random_bytes(buf: &mut [u8]) {
    if getrandom::fill(buf).is_err() {
        panic!("snell: system RNG unavailable");
    }
}

/// Increment a 12-byte little-endian counter nonce.
pub(super) fn increment_nonce(nonce: &mut [u8; 12]) {
    for byte in nonce.iter_mut() {
        *byte = byte.wrapping_add(1);
        if *byte != 0 {
            break;
        }
    }
}

/// An AEAD cipher instance keyed with a per-session subkey.
pub(super) enum AeadCipher {
    Aes128(Box<Aes128Gcm>),
    Chacha(Box<ChaCha20Poly1305>),
}

impl AeadCipher {
    pub(super) fn new(cipher: SnellCipher, subkey: &[u8]) -> Result<Self> {
        match cipher {
            SnellCipher::Aes128Gcm => Ok(AeadCipher::Aes128(Box::new(
                Aes128Gcm::new_from_slice(subkey).map_err(|_| anyhow!("snell: invalid aes-128 key"))?,
            ))),
            SnellCipher::Chacha20Poly1305 => Ok(AeadCipher::Chacha(Box::new(
                ChaCha20Poly1305::new_from_slice(subkey).map_err(|_| anyhow!("snell: invalid chacha key"))?,
            ))),
        }
    }

    pub(super) fn seal(&self, nonce: &[u8; 12], plaintext: &[u8]) -> Result<Vec<u8>> {
        let payload = Payload {
            msg: plaintext,
            aad: &[],
        };
        let result = match self {
            AeadCipher::Aes128(c) => c.encrypt(GenericArray::from_slice(nonce), payload),
            AeadCipher::Chacha(c) => c.encrypt(GenericArray::from_slice(nonce), payload),
        };
        result.map_err(|_| anyhow!("snell: AEAD seal failed"))
    }

    pub(super) fn open(&self, nonce: &[u8; 12], ciphertext: &[u8]) -> Result<Vec<u8>> {
        let payload = Payload {
            msg: ciphertext,
            aad: &[],
        };
        let result = match self {
            AeadCipher::Aes128(c) => c.decrypt(GenericArray::from_slice(nonce), payload),
            AeadCipher::Chacha(c) => c.decrypt(GenericArray::from_slice(nonce), payload),
        };
        result.map_err(|_| anyhow!("snell: AEAD open failed"))
    }
}

pub(super) fn decrypt_err(e: anyhow::Error) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, e.to_string())
}

pub(super) fn read_cipher_unset() -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, "snell: read cipher unset")
}
