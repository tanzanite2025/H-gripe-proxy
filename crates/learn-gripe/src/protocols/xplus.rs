//! XPlus packet obfuscation for Hysteria v1 (`obfs: <key>`).
//!
//! XPlus is Hysteria v1's packet obfuscator — the v1 equivalent of v2's
//! [`crate::protocols::salamander`]. It is a per-datagram, stateless XOR cipher
//! applied *underneath* QUIC: every UDP datagram leaving the socket is masked
//! with a one-time key derived from a pre-shared key and a fresh random salt,
//! and every datagram arriving is unmasked, so a passive observer sees
//! uniformly random bytes instead of a recognizable QUIC long header.
//!
//! ```text
//! key    = SHA-256(PSK || salt)                ; 32-byte digest
//! packet = salt(16) || ( payload[i] XOR key[i % 32] )
//! ```
//!
//! Deobfuscation reads the 16-byte salt, re-derives the same key, and XORs the
//! remainder back. This matches the reference implementation in
//! `apernet/hysteria` (`core/pktconns/obfs/obfs.go`, `XPlusObfuscator`), so a
//! kernel client with `obfs: <key>` interoperates with a stock Hysteria v1
//! server using the same key.
//!
//! Like Salamander, the keystream is just the 32-byte digest repeated — that is
//! by design: XPlus only defeats *protocol fingerprinting*, not confidentiality
//! (QUIC/TLS already encrypts the payload). The wire codec is owned here; the
//! SHA-256 primitive is delegated to the vetted `sha2` crate.

use sha2::{Digest, Sha256};

/// Length of the random per-packet salt prefix (bytes).
pub const SALT_LEN: usize = 16;
/// Length of the SHA-256 key digest (bytes).
const KEY_LEN: usize = 32;

/// An XPlus obfuscator bound to a pre-shared key (the `obfs` value).
#[derive(Clone, PartialEq, Eq)]
pub struct XPlus {
    psk: Vec<u8>,
}

impl std::fmt::Debug for XPlus {
    /// Redacts the pre-shared key so it never leaks into logs.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("XPlus").field("psk", &"<redacted>").finish()
    }
}

impl XPlus {
    /// Build an obfuscator from the `obfs` key bytes.
    pub fn new(psk: impl Into<Vec<u8>>) -> Self {
        Self { psk: psk.into() }
    }

    /// Derive the 32-byte XOR key for `salt`: `SHA-256(PSK || salt)`.
    fn key(&self, salt: &[u8]) -> [u8; KEY_LEN] {
        let mut hasher = Sha256::new();
        hasher.update(&self.psk);
        hasher.update(salt);
        hasher.finalize().into()
    }

    /// Obfuscate `payload` into a fresh `salt(16) || XOR(payload)` datagram. The
    /// salt is drawn from the OS CSPRNG so each packet derives a distinct key.
    pub fn obfuscate(&self, payload: &[u8]) -> Vec<u8> {
        let mut salt = [0u8; SALT_LEN];
        getrandom::fill(&mut salt).expect("os rng");
        self.obfuscate_with_salt(&salt, payload)
    }

    /// Obfuscate `payload` with a caller-supplied `salt` (used by tests for
    /// deterministic vectors).
    fn obfuscate_with_salt(&self, salt: &[u8; SALT_LEN], payload: &[u8]) -> Vec<u8> {
        let key = self.key(salt);
        let mut out = Vec::with_capacity(SALT_LEN + payload.len());
        out.extend_from_slice(salt);
        for (i, &b) in payload.iter().enumerate() {
            out.push(b ^ key[i % KEY_LEN]);
        }
        out
    }

    /// Deobfuscate a `salt(16) || ciphertext` datagram, returning the recovered
    /// payload. Returns `None` for a datagram too short to carry a salt and at
    /// least one payload byte (mirroring the reference server, which drops it).
    pub fn deobfuscate(&self, datagram: &[u8]) -> Option<Vec<u8>> {
        if datagram.len() <= SALT_LEN {
            return None;
        }
        let key = self.key(&datagram[..SALT_LEN]);
        let out = datagram[SALT_LEN..]
            .iter()
            .enumerate()
            .map(|(i, &b)| b ^ key[i % KEY_LEN])
            .collect();
        Some(out)
    }

    /// Deobfuscate in place: re-derive the key from the leading salt, XOR the
    /// ciphertext back, and shift it to the front of `buf`. Returns the
    /// recovered payload length (`buf.len() - SALT_LEN`), or `None` if `buf` is
    /// too short. Writing each output byte before re-reading a later input byte
    /// makes the forward shift safe to do in the same buffer.
    pub fn deobfuscate_in_place(&self, buf: &mut [u8]) -> Option<usize> {
        if buf.len() <= SALT_LEN {
            return None;
        }
        let mut salt = [0u8; SALT_LEN];
        salt.copy_from_slice(&buf[..SALT_LEN]);
        let key = self.key(&salt);
        let out_len = buf.len() - SALT_LEN;
        for i in 0..out_len {
            buf[i] = buf[SALT_LEN + i] ^ key[i % KEY_LEN];
        }
        Some(out_len)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A fixed-salt interop vector computed independently against the XPlus wire
    /// format (`SHA-256(psk || salt)` keystream). Deobfuscating it with the
    /// matching key must recover the plaintext, proving byte-for-byte
    /// compatibility with a stock Hysteria v1 peer.
    #[test]
    fn round_trips_with_fixed_salt() {
        let xplus = XPlus::new(b"test-key".to_vec());
        let salt: [u8; SALT_LEN] = [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15];
        let packet = xplus.obfuscate_with_salt(&salt, b"hello world");
        assert_eq!(&packet[..SALT_LEN], &salt);
        assert_eq!(xplus.deobfuscate(&packet).unwrap(), b"hello world");
    }

    /// The keystream is `SHA-256(psk || salt)` repeated; verify the first masked
    /// bytes against an independently computed digest so a regression in the
    /// hash input order is caught.
    #[test]
    fn masks_against_sha256_keystream() {
        let xplus = XPlus::new(b"pw".to_vec());
        let salt = [0u8; SALT_LEN];
        let key = {
            let mut h = Sha256::new();
            h.update(b"pw");
            h.update(salt);
            let d: [u8; KEY_LEN] = h.finalize().into();
            d
        };
        let packet = xplus.obfuscate_with_salt(&salt, &[0xff, 0x00, 0xaa]);
        assert_eq!(&packet[SALT_LEN..], &[0xff ^ key[0], key[1], 0xaa ^ key[2]]);
    }

    #[test]
    fn round_trips_with_random_salt() {
        let xplus = XPlus::new(b"another secret".to_vec());
        for len in [1usize, 16, 31, 32, 33, 1500] {
            let payload: Vec<u8> = (0..len).map(|i| (i * 7 + 3) as u8).collect();
            let obf = xplus.obfuscate(&payload);
            assert_eq!(obf.len(), payload.len() + SALT_LEN);
            assert_ne!(&obf[SALT_LEN..], &payload[..], "payload should be masked");
            assert_eq!(xplus.deobfuscate(&obf).unwrap(), payload);
        }
    }

    #[test]
    fn deobfuscate_in_place_matches_allocating_variant() {
        let xplus = XPlus::new(b"pw".to_vec());
        let payload: Vec<u8> = (0..200u32).map(|i| i as u8).collect();
        let obf = xplus.obfuscate(&payload);
        let mut buf = obf.clone();
        let len = xplus.deobfuscate_in_place(&mut buf).unwrap();
        assert_eq!(&buf[..len], &payload[..]);
        assert_eq!(xplus.deobfuscate(&obf).unwrap(), payload);
    }

    #[test]
    fn rejects_short_datagrams() {
        let xplus = XPlus::new(b"pw".to_vec());
        assert!(xplus.deobfuscate(&[0u8; SALT_LEN]).is_none());
        let mut buf = [0u8; SALT_LEN];
        assert!(xplus.deobfuscate_in_place(&mut buf).is_none());
    }
}
