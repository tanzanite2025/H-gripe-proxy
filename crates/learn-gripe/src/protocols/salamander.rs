//! Salamander packet obfuscation for Hysteria2 (`obfs: salamander`).
//!
//! Salamander is a per-datagram, stateless XOR cipher applied *underneath* QUIC:
//! every UDP datagram leaving the socket is obfuscated and every datagram
//! arriving is deobfuscated, so a passive observer sees uniformly random bytes
//! instead of a recognizable QUIC long header. It is keyed by a pre-shared
//! string (`obfs-password`) and a fresh random salt per packet:
//!
//! ```text
//! key    = BLAKE2b-256(PSK || salt)            ; 32-byte digest
//! packet = salt(8) || ( payload[i] XOR key[i % 32] )
//! ```
//!
//! Deobfuscation reads the 8-byte salt, re-derives the same key, and XORs the
//! remainder back. This matches the reference implementation in
//! `apernet/hysteria` (`extras/obfs/salamander.go`), so a kernel client with
//! `obfs: salamander` interoperates with a stock Hysteria2 server using the same
//! `obfs-password`.
//!
//! The XOR keystream is just the 32-byte digest repeated, which is weak as a
//! cipher — that is by design: Salamander only aims to defeat *protocol
//! fingerprinting*, not to provide confidentiality (QUIC/TLS already encrypts
//! the payload). The wire codec is owned here; the BLAKE2b-256 primitive is
//! delegated to the vetted `blake2` crate.

use blake2::Blake2b;
use blake2::digest::Digest;
use blake2::digest::consts::U32;

/// Length of the random per-packet salt prefix (bytes).
pub const SALT_LEN: usize = 8;
/// Length of the BLAKE2b-256 key digest (bytes).
const KEY_LEN: usize = 32;

/// BLAKE2b with a 256-bit (32-byte) digest, matching Go's `blake2b.Sum256`.
type Blake2b256 = Blake2b<U32>;

/// A Salamander obfuscator bound to a pre-shared key (the `obfs-password`).
#[derive(Clone, PartialEq, Eq)]
pub struct Salamander {
    psk: Vec<u8>,
}

impl std::fmt::Debug for Salamander {
    /// Redacts the pre-shared key so it never leaks into logs.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Salamander").field("psk", &"<redacted>").finish()
    }
}

impl Salamander {
    /// Build an obfuscator from the `obfs-password` bytes.
    pub fn new(psk: impl Into<Vec<u8>>) -> Self {
        Self { psk: psk.into() }
    }

    /// Derive the 32-byte XOR key for `salt`: `BLAKE2b-256(PSK || salt)`.
    fn key(&self, salt: &[u8]) -> [u8; KEY_LEN] {
        let mut hasher = Blake2b256::new();
        hasher.update(&self.psk);
        hasher.update(salt);
        let digest = hasher.finalize();
        let mut key = [0u8; KEY_LEN];
        key.copy_from_slice(&digest);
        key
    }

    /// Obfuscate `payload` into a fresh `salt(8) || XOR(payload)` datagram. The
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

    /// Deobfuscate a `salt(8) || ciphertext` datagram, returning the recovered
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

    /// The kernel's BLAKE2b-256 must match Go's `blake2b.Sum256` (digest length
    /// 32, not a truncated 512). The empty-input digest is a stable, widely
    /// published BLAKE2b-256 vector.
    #[test]
    fn blake2b_256_empty_vector() {
        let key = Salamander::new(Vec::<u8>::new()).key(&[]);
        assert_eq!(
            hex(&key),
            "0e5751c026e543b2e8ab2eb06099daa1d1e5df47778f7787faab45cdf12fe3a8"
        );
    }

    /// A fixed-salt interop vector computed independently (Python `hashlib`)
    /// against the Salamander wire format. Deobfuscating it with the matching
    /// PSK must recover the plaintext, proving byte-for-byte compatibility with
    /// a stock Hysteria2 peer.
    #[test]
    fn deobfuscates_reference_vector() {
        let salamander = Salamander::new(b"test-psk".to_vec());
        let packet = unhex("000102030405060794f40315ffac1aac07136a");
        let payload = salamander.deobfuscate(&packet).unwrap();
        assert_eq!(payload, b"hello world");
    }

    #[test]
    fn obfuscate_with_fixed_salt_matches_vector() {
        let salamander = Salamander::new(b"test-psk".to_vec());
        let salt: [u8; SALT_LEN] = [0, 1, 2, 3, 4, 5, 6, 7];
        let packet = salamander.obfuscate_with_salt(&salt, b"hello world");
        assert_eq!(hex(&packet), "000102030405060794f40315ffac1aac07136a");
    }

    #[test]
    fn round_trips_with_random_salt() {
        let salamander = Salamander::new(b"another secret".to_vec());
        for len in [1usize, 8, 31, 32, 33, 1500] {
            let payload: Vec<u8> = (0..len).map(|i| (i * 7 + 3) as u8).collect();
            let obf = salamander.obfuscate(&payload);
            assert_eq!(obf.len(), payload.len() + SALT_LEN);
            assert_ne!(&obf[SALT_LEN..], &payload[..], "payload should be masked");
            assert_eq!(salamander.deobfuscate(&obf).unwrap(), payload);
        }
    }

    #[test]
    fn deobfuscate_in_place_matches_allocating_variant() {
        let salamander = Salamander::new(b"pw".to_vec());
        let payload: Vec<u8> = (0..200u32).map(|i| i as u8).collect();
        let obf = salamander.obfuscate(&payload);
        let mut buf = obf.clone();
        let len = salamander.deobfuscate_in_place(&mut buf).unwrap();
        assert_eq!(&buf[..len], &payload[..]);
        assert_eq!(salamander.deobfuscate(&obf).unwrap(), payload);
    }

    #[test]
    fn rejects_short_datagrams() {
        let salamander = Salamander::new(b"pw".to_vec());
        assert!(salamander.deobfuscate(&[0u8; SALT_LEN]).is_none());
        let mut buf = [0u8; SALT_LEN];
        assert!(salamander.deobfuscate_in_place(&mut buf).is_none());
    }

    fn hex(bytes: &[u8]) -> String {
        bytes.iter().map(|b| format!("{b:02x}")).collect()
    }

    fn unhex(s: &str) -> Vec<u8> {
        (0..s.len())
            .step_by(2)
            .map(|i| u8::from_str_radix(&s[i..i + 2], 16).unwrap())
            .collect()
    }
}
