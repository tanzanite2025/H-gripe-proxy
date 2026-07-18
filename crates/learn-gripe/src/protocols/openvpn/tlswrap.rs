//! `tls-auth` / `tls-crypt` control-channel protection.
//!
//! Both modes wrap every control-channel packet (`P_CONTROL_*`, `P_ACK_V1`)
//! with a static pre-shared key; `P_DATA_*` packets are never wrapped.
//!
//! `tls-auth` (HMAC authentication, packet stays in cleartext):
//! `opcode/key-id(1) || session-id(8) || HMAC(N) || packet-id(4) || net-time(4) || rest`,
//! where the HMAC is computed over the reordered pseudo-header
//! `packet-id || net-time || opcode/key-id || session-id || rest` (OpenVPN's
//! `swap_hmac`). The digest defaults to SHA1 (`auth`).
//!
//! `tls-crypt-v2` (per-client keys): the client key file carries a per-client
//! 2048-bit key `Kc` (used exactly like a `tls-crypt` key) plus `WKc`, an
//! opaque server-encrypted copy of `Kc` (the client cannot read it). Every
//! `P_CONTROL_HARD_RESET_CLIENT_V3` is `tls-crypt`-wrapped with `Kc` and then
//! has `WKc` appended in cleartext, letting the server recover `Kc` from the
//! first packet; all other control packets are plain `tls-crypt` with `Kc`.
//! `WKc = T || AES-256-CTR(Ke, IV=T[..16], Kc || metadata) || len`, with
//! `T = HMAC-SHA256(Ka, len || Kc || metadata)` under the server key — parsed
//! here only far enough to validate the trailing `len`.
//!
//! `tls-crypt` (encrypt + authenticate):
//! `opcode/key-id(1) || session-id(8) || packet-id(4) || net-time(4) || tag(32) || ciphertext`,
//! where `tag = HMAC-SHA256(Ka, header || plaintext)` (header = the 17
//! cleartext bytes), the AES-256-CTR IV is the first 16 bytes of the tag, and
//! `ciphertext = AES-256-CTR(Ke, IV, plaintext)` (SIV-style: deterministic IV
//! from the MAC).
//!
//! Key material comes from a 2048-bit "OpenVPN Static key V1" file: two
//! directional keys, each a 64-byte cipher half + 64-byte HMAC half. For
//! `tls-auth` the `key-direction` option picks which HMAC half each direction
//! uses (absent = bidirectional, both use key 0; `1` = client convention:
//! send with key 1, receive with key 0). `tls-crypt` always uses the client
//! (inverse) direction and takes the first 32 bytes of each half as the
//! AES-256 / HMAC-SHA256 keys.
//!
//! The wrap is (re)applied at write time, so each retransmission of a reliable
//! control packet carries a fresh packet id, mirroring upstream. On receive we
//! authenticate (and for `tls-crypt` decrypt) every control packet and fail
//! closed on a bad MAC; strict packet-id replay tracking is left to the
//! reliability layer, which already deduplicates by message id.

use std::sync::atomic::{AtomicU32, Ordering};

use aes::Aes256;
use anyhow::{Result, anyhow, bail};
use base64::Engine as _;
use ctr::cipher::{KeyIvInit, StreamCipher};
use hmac::{Hmac, Mac};
use sha1::Sha1;
use sha2::{Sha256, Sha512};

use super::packet::{
    CONTROL_HEADER_SIZE, P_CONTROL_HARD_RESET_CLIENT_V2, P_CONTROL_HARD_RESET_CLIENT_V3, SESSION_ID_SIZE,
    parse_opcode_key_id,
};

type Aes256Ctr = ctr::Ctr128BE<Aes256>;

/// Size of one directional key slot's cipher (and HMAC) half.
const KEY_HALF: usize = 64;
/// Total static key size: 2 keys x (cipher half + HMAC half).
pub(super) const STATIC_KEY_SIZE: usize = 4 * KEY_HALF;

/// `packet-id(4) || net-time(4)` replay id carried by both wrap modes.
const REPLAY_ID_SIZE: usize = 8;
/// `tls-crypt` tag size (HMAC-SHA256).
const TLS_CRYPT_TAG_SIZE: usize = 32;
/// `tls-crypt` cleartext header: opcode/key-id + session id + replay id.
const TLS_CRYPT_HEADER_SIZE: usize = CONTROL_HEADER_SIZE + REPLAY_ID_SIZE;
/// `tls-crypt` AES-256-CTR / HMAC-SHA256 key length (first half of each slot).
const TLS_CRYPT_KEY_LEN: usize = 32;

const PEM_BEGIN: &str = "-----BEGIN OpenVPN Static key V1-----";
const PEM_END: &str = "-----END OpenVPN Static key V1-----";

const PEM_V2_CLIENT_BEGIN: &str = "-----BEGIN OpenVPN tls-crypt-v2 client key-----";
const PEM_V2_CLIENT_END: &str = "-----END OpenVPN tls-crypt-v2 client key-----";

/// `WKc` trailing big-endian length field.
const WKC_LEN_SIZE: usize = 2;
/// Minimum `WKc`: HMAC-SHA256 tag + encrypted 256-byte client key + length.
const MIN_WKC_LEN: usize = TLS_CRYPT_TAG_SIZE + STATIC_KEY_SIZE + WKC_LEN_SIZE;
/// Upstream's `TLS_CRYPT_V2_MAX_WKC_LEN`.
const MAX_WKC_LEN: usize = 1024;

/// Parse an inline "OpenVPN Static key V1" block (hex body between the PEM-like
/// markers; `#`/`;` comment lines outside the block are ignored) into the raw
/// 256-byte key.
pub(super) fn parse_static_key(text: &str) -> Result<[u8; STATIC_KEY_SIZE]> {
    let start = text
        .find(PEM_BEGIN)
        .ok_or_else(|| anyhow!("openvpn: static key missing {PEM_BEGIN:?} marker"))?;
    let after_begin = start + PEM_BEGIN.len();
    let end = text[after_begin..]
        .find(PEM_END)
        .ok_or_else(|| anyhow!("openvpn: static key missing {PEM_END:?} marker"))?;
    let body = &text[after_begin..after_begin + end];

    let hex: String = body.chars().filter(|c| !c.is_whitespace()).collect();
    if hex.len() != STATIC_KEY_SIZE * 2 {
        bail!(
            "openvpn: static key must be {} hex chars, got {}",
            STATIC_KEY_SIZE * 2,
            hex.len()
        );
    }
    let mut key = [0u8; STATIC_KEY_SIZE];
    for (i, out) in key.iter_mut().enumerate() {
        let pair = &hex[i * 2..i * 2 + 2];
        *out = u8::from_str_radix(pair, 16).map_err(|_| anyhow!("openvpn: static key has non-hex byte {pair:?}"))?;
    }
    Ok(key)
}

/// Parse an inline "OpenVPN tls-crypt-v2 client key" block (base64 body
/// between the PEM-like markers) into the per-client 256-byte key `Kc` and the
/// opaque server-wrapped `WKc` (validated only against its trailing length
/// field; its contents are decryptable by the server alone).
pub(super) fn parse_tls_crypt_v2_client_key(text: &str) -> Result<([u8; STATIC_KEY_SIZE], Vec<u8>)> {
    let start = text
        .find(PEM_V2_CLIENT_BEGIN)
        .ok_or_else(|| anyhow!("openvpn: tls-crypt-v2 client key missing {PEM_V2_CLIENT_BEGIN:?} marker"))?;
    let after_begin = start + PEM_V2_CLIENT_BEGIN.len();
    let end = text[after_begin..]
        .find(PEM_V2_CLIENT_END)
        .ok_or_else(|| anyhow!("openvpn: tls-crypt-v2 client key missing {PEM_V2_CLIENT_END:?} marker"))?;
    let body: String = text[after_begin..after_begin + end]
        .chars()
        .filter(|c| !c.is_whitespace())
        .collect();
    let raw = base64::engine::general_purpose::STANDARD
        .decode(&body)
        .map_err(|e| anyhow!("openvpn: tls-crypt-v2 client key is not valid base64: {e}"))?;
    if raw.len() < STATIC_KEY_SIZE + MIN_WKC_LEN {
        bail!(
            "openvpn: tls-crypt-v2 client key too short: {} bytes (need at least {})",
            raw.len(),
            STATIC_KEY_SIZE + MIN_WKC_LEN
        );
    }
    let (kc, wkc) = raw.split_at(STATIC_KEY_SIZE);
    if wkc.len() > MAX_WKC_LEN {
        bail!(
            "openvpn: tls-crypt-v2 WKc too large: {} bytes (max {MAX_WKC_LEN})",
            wkc.len()
        );
    }
    let tail = u16::from_be_bytes(wkc[wkc.len() - WKC_LEN_SIZE..].try_into().unwrap()) as usize;
    if tail != wkc.len() {
        bail!(
            "openvpn: tls-crypt-v2 WKc length field {tail} does not match its {} bytes",
            wkc.len()
        );
    }
    Ok((
        kc.try_into().expect("split_at yields STATIC_KEY_SIZE bytes"),
        wkc.to_vec(),
    ))
}

/// `key-direction` semantics for the two directional key slots.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum KeyDirection {
    /// No `key-direction`: both directions use key slot 0.
    Bidirectional,
    /// `key-direction 0` (server convention): send with slot 0, receive slot 1.
    Normal,
    /// `key-direction 1` (client convention): send with slot 1, receive slot 0.
    Inverse,
}

impl KeyDirection {
    pub(super) fn from_option(value: Option<u8>) -> Result<Self> {
        match value {
            None => Ok(Self::Bidirectional),
            Some(0) => Ok(Self::Normal),
            Some(1) => Ok(Self::Inverse),
            Some(other) => bail!("openvpn: `key-direction` must be 0 or 1, got {other}"),
        }
    }

    /// (send slot, receive slot) into the static key's two key slots.
    fn slots(self) -> (usize, usize) {
        match self {
            Self::Bidirectional => (0, 0),
            Self::Normal => (0, 1),
            Self::Inverse => (1, 0),
        }
    }
}

/// One directional slot of the static key: 64-byte cipher half + HMAC half.
fn key_slot(key: &[u8; STATIC_KEY_SIZE], slot: usize) -> (&[u8], &[u8]) {
    let base = slot * 2 * KEY_HALF;
    (&key[base..base + KEY_HALF], &key[base + KEY_HALF..base + 2 * KEY_HALF])
}

/// HMAC digest for `tls-auth` (the OpenVPN `auth` option).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum AuthDigest {
    Sha1,
    Sha256,
    Sha512,
}

impl AuthDigest {
    pub(super) fn parse(name: &str) -> Result<Self> {
        match name.trim().to_ascii_uppercase().as_str() {
            "SHA1" => Ok(Self::Sha1),
            "SHA256" => Ok(Self::Sha256),
            "SHA512" => Ok(Self::Sha512),
            other => bail!("openvpn: unsupported `auth` digest {other:?} (SHA1, SHA256, SHA512)"),
        }
    }

    pub(super) fn name(self) -> &'static str {
        match self {
            Self::Sha1 => "SHA1",
            Self::Sha256 => "SHA256",
            Self::Sha512 => "SHA512",
        }
    }

    pub(super) fn size(self) -> usize {
        match self {
            Self::Sha1 => 20,
            Self::Sha256 => 32,
            Self::Sha512 => 64,
        }
    }

    pub(super) fn mac(self, key: &[u8], parts: &[&[u8]]) -> Vec<u8> {
        fn run<M: Mac + hmac::digest::KeyInit>(key: &[u8], parts: &[&[u8]]) -> Vec<u8> {
            let mut mac = <M as hmac::digest::KeyInit>::new_from_slice(key).expect("hmac accepts any key length");
            for part in parts {
                mac.update(part);
            }
            mac.finalize().into_bytes().to_vec()
        }
        match self {
            Self::Sha1 => run::<Hmac<Sha1>>(key, parts),
            Self::Sha256 => run::<Hmac<Sha256>>(key, parts),
            Self::Sha512 => run::<Hmac<Sha512>>(key, parts),
        }
    }
}

/// Monotonic `packet-id || net-time` replay id, starting at 1 like upstream.
struct ReplayId(AtomicU32);

impl ReplayId {
    fn new() -> Self {
        Self(AtomicU32::new(0))
    }

    fn next(&self) -> [u8; REPLAY_ID_SIZE] {
        let id = self.0.fetch_add(1, Ordering::Relaxed).wrapping_add(1);
        let time = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs() as u32)
            .unwrap_or(0);
        let mut out = [0u8; REPLAY_ID_SIZE];
        out[..4].copy_from_slice(&id.to_be_bytes());
        out[4..].copy_from_slice(&time.to_be_bytes());
        out
    }
}

/// The negotiated control-channel wrap mode.
pub(super) enum ControlWrap {
    TlsAuth(TlsAuth),
    TlsCrypt(TlsCrypt),
    TlsCryptV2(TlsCryptV2),
}

impl ControlWrap {
    pub(super) fn tls_auth(key_text: &str, direction: KeyDirection, digest: AuthDigest) -> Result<Self> {
        Ok(Self::TlsAuth(TlsAuth::new(
            &parse_static_key(key_text)?,
            direction,
            digest,
        )))
    }

    pub(super) fn tls_crypt(key_text: &str) -> Result<Self> {
        Ok(Self::TlsCrypt(TlsCrypt::new(&parse_static_key(key_text)?)))
    }

    pub(super) fn tls_crypt_v2(key_text: &str) -> Result<Self> {
        let (kc, wkc) = parse_tls_crypt_v2_client_key(key_text)?;
        Ok(Self::TlsCryptV2(TlsCryptV2 {
            crypt: TlsCrypt::new(&kc),
            wkc,
        }))
    }

    /// The client hard-reset opcode this wrap mode requires: `tls-crypt-v2`
    /// announces itself with `P_CONTROL_HARD_RESET_CLIENT_V3` (which carries
    /// the `WKc`), everything else uses the V2 reset.
    pub(super) fn reset_opcode(&self) -> u8 {
        match self {
            Self::TlsCryptV2(_) => P_CONTROL_HARD_RESET_CLIENT_V3,
            _ => P_CONTROL_HARD_RESET_CLIENT_V2,
        }
    }

    /// Wrap one plaintext control packet (`opcode/key-id || session || rest`)
    /// for the wire.
    pub(super) fn wrap(&self, plain: &[u8]) -> Result<Vec<u8>> {
        match self {
            Self::TlsAuth(w) => w.wrap(plain),
            Self::TlsCrypt(w) => w.wrap(plain),
            Self::TlsCryptV2(w) => w.wrap(plain),
        }
    }

    /// Authenticate (and for `tls-crypt` decrypt) one wire control packet back
    /// into its plaintext form.
    pub(super) fn unwrap(&self, wire: &[u8]) -> Result<Vec<u8>> {
        match self {
            Self::TlsAuth(w) => w.unwrap(wire),
            Self::TlsCrypt(w) => w.unwrap(wire),
            Self::TlsCryptV2(w) => w.unwrap(wire),
        }
    }
}

/// `tls-auth`: per-direction HMAC over every control packet.
pub(super) struct TlsAuth {
    send_key: Vec<u8>,
    recv_key: Vec<u8>,
    digest: AuthDigest,
    replay: ReplayId,
}

impl TlsAuth {
    fn new(key: &[u8; STATIC_KEY_SIZE], direction: KeyDirection, digest: AuthDigest) -> Self {
        let (send_slot, recv_slot) = direction.slots();
        // Only the HMAC half of each slot is used; the HMAC key is the first
        // `digest size` bytes of it, mirroring upstream.
        let (_, send_hmac) = key_slot(key, send_slot);
        let (_, recv_hmac) = key_slot(key, recv_slot);
        Self {
            send_key: send_hmac[..digest.size()].to_vec(),
            recv_key: recv_hmac[..digest.size()].to_vec(),
            digest,
            replay: ReplayId::new(),
        }
    }

    fn wrap(&self, plain: &[u8]) -> Result<Vec<u8>> {
        if plain.len() < CONTROL_HEADER_SIZE {
            bail!("openvpn: control packet too short to tls-auth wrap");
        }
        let (header, rest) = plain.split_at(CONTROL_HEADER_SIZE);
        let replay = self.replay.next();
        let hmac = self.digest.mac(&self.send_key, &[&replay, header, rest]);

        let mut out = Vec::with_capacity(plain.len() + hmac.len() + REPLAY_ID_SIZE);
        out.extend_from_slice(header);
        out.extend_from_slice(&hmac);
        out.extend_from_slice(&replay);
        out.extend_from_slice(rest);
        Ok(out)
    }

    fn unwrap(&self, wire: &[u8]) -> Result<Vec<u8>> {
        let hmac_size = self.digest.size();
        if wire.len() < CONTROL_HEADER_SIZE + hmac_size + REPLAY_ID_SIZE {
            bail!("openvpn: tls-auth control packet too short");
        }
        let header = &wire[..CONTROL_HEADER_SIZE];
        let hmac = &wire[CONTROL_HEADER_SIZE..CONTROL_HEADER_SIZE + hmac_size];
        let replay = &wire[CONTROL_HEADER_SIZE + hmac_size..CONTROL_HEADER_SIZE + hmac_size + REPLAY_ID_SIZE];
        let rest = &wire[CONTROL_HEADER_SIZE + hmac_size + REPLAY_ID_SIZE..];

        let expected = self.digest.mac(&self.recv_key, &[replay, header, rest]);
        if !constant_time_eq(&expected, hmac) {
            bail!("openvpn: tls-auth HMAC verification failed");
        }

        let mut out = Vec::with_capacity(CONTROL_HEADER_SIZE + rest.len());
        out.extend_from_slice(header);
        out.extend_from_slice(rest);
        Ok(out)
    }
}

/// `tls-crypt`: AES-256-CTR + HMAC-SHA256 (SIV-style) over every control packet.
pub(super) struct TlsCrypt {
    send_cipher_key: [u8; TLS_CRYPT_KEY_LEN],
    send_hmac_key: [u8; TLS_CRYPT_KEY_LEN],
    recv_cipher_key: [u8; TLS_CRYPT_KEY_LEN],
    recv_hmac_key: [u8; TLS_CRYPT_KEY_LEN],
    replay: ReplayId,
}

impl TlsCrypt {
    fn new(key: &[u8; STATIC_KEY_SIZE]) -> Self {
        // tls-crypt always uses the client (inverse) direction: the client
        // encrypts with key slot 1 and decrypts with slot 0; each slot yields a
        // 32-byte AES-256-CTR key + 32-byte HMAC-SHA256 key.
        let (send_slot, recv_slot) = KeyDirection::Inverse.slots();
        let (send_cipher, send_hmac) = key_slot(key, send_slot);
        let (recv_cipher, recv_hmac) = key_slot(key, recv_slot);
        let take = |half: &[u8]| -> [u8; TLS_CRYPT_KEY_LEN] { half[..TLS_CRYPT_KEY_LEN].try_into().unwrap() };
        Self {
            send_cipher_key: take(send_cipher),
            send_hmac_key: take(send_hmac),
            recv_cipher_key: take(recv_cipher),
            recv_hmac_key: take(recv_hmac),
            replay: ReplayId::new(),
        }
    }

    fn wrap(&self, plain: &[u8]) -> Result<Vec<u8>> {
        if plain.len() < CONTROL_HEADER_SIZE {
            bail!("openvpn: control packet too short to tls-crypt wrap");
        }
        let (control_header, rest) = plain.split_at(CONTROL_HEADER_SIZE);
        let replay = self.replay.next();

        let tag = AuthDigest::Sha256.mac(&self.send_hmac_key, &[control_header, &replay, rest]);
        let mut ciphertext = rest.to_vec();
        aes256_ctr(&self.send_cipher_key, &tag[..16], &mut ciphertext);

        let mut out = Vec::with_capacity(TLS_CRYPT_HEADER_SIZE + tag.len() + ciphertext.len());
        out.extend_from_slice(control_header);
        out.extend_from_slice(&replay);
        out.extend_from_slice(&tag);
        out.extend_from_slice(&ciphertext);
        Ok(out)
    }

    fn unwrap(&self, wire: &[u8]) -> Result<Vec<u8>> {
        if wire.len() < TLS_CRYPT_HEADER_SIZE + TLS_CRYPT_TAG_SIZE {
            bail!("openvpn: tls-crypt control packet too short");
        }
        let control_header = &wire[..CONTROL_HEADER_SIZE];
        let replay = &wire[CONTROL_HEADER_SIZE..TLS_CRYPT_HEADER_SIZE];
        let tag = &wire[TLS_CRYPT_HEADER_SIZE..TLS_CRYPT_HEADER_SIZE + TLS_CRYPT_TAG_SIZE];
        let mut plaintext = wire[TLS_CRYPT_HEADER_SIZE + TLS_CRYPT_TAG_SIZE..].to_vec();
        aes256_ctr(&self.recv_cipher_key, &tag[..16], &mut plaintext);

        let expected = AuthDigest::Sha256.mac(&self.recv_hmac_key, &[control_header, replay, &plaintext]);
        if !constant_time_eq(&expected, tag) {
            bail!("openvpn: tls-crypt authentication failed");
        }

        let mut out = Vec::with_capacity(CONTROL_HEADER_SIZE + plaintext.len());
        out.extend_from_slice(control_header);
        out.extend_from_slice(&plaintext);
        Ok(out)
    }
}

/// `tls-crypt-v2`: the per-client `tls-crypt` key plus the opaque `WKc`
/// appended to every client V3 hard reset.
pub(super) struct TlsCryptV2 {
    crypt: TlsCrypt,
    wkc: Vec<u8>,
}

impl TlsCryptV2 {
    fn wrap(&self, plain: &[u8]) -> Result<Vec<u8>> {
        let mut out = self.crypt.wrap(plain)?;
        let (opcode, _) = parse_opcode_key_id(plain[0]);
        if opcode == P_CONTROL_HARD_RESET_CLIENT_V3 {
            out.extend_from_slice(&self.wkc);
        }
        Ok(out)
    }

    fn unwrap(&self, wire: &[u8]) -> Result<Vec<u8>> {
        self.crypt.unwrap(wire)
    }
}

fn aes256_ctr(key: &[u8; TLS_CRYPT_KEY_LEN], iv: &[u8], data: &mut [u8]) {
    let mut cipher = Aes256Ctr::new(key.into(), iv.into());
    cipher.apply_keystream(data);
}

pub(super) fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut diff = 0u8;
    for (x, y) in a.iter().zip(b) {
        diff |= x ^ y;
    }
    diff == 0
}

/// Session-id/opcode-preserving check used by the control channel: whether the
/// packet body still starts with the same cleartext control header after
/// (un)wrapping. Exposed for tests.
#[cfg(test)]
fn header_of(packet: &[u8]) -> &[u8] {
    &packet[..CONTROL_HEADER_SIZE]
}

// Keep the session-id size assumption in one place.
const _: () = assert!(CONTROL_HEADER_SIZE == 1 + SESSION_ID_SIZE);

#[cfg(test)]
mod tests {
    use super::*;

    fn test_key_text() -> String {
        let mut body = String::new();
        for i in 0..STATIC_KEY_SIZE {
            body.push_str(&format!("{:02x}", (i * 7 + 3) as u8));
            if i % 16 == 15 {
                body.push('\n');
            }
        }
        format!("#\n# comment\n{PEM_BEGIN}\n{body}{PEM_END}\n")
    }

    fn plain_packet() -> Vec<u8> {
        // opcode/key-id + session id + reliability body.
        let mut p = vec![7u8 << 3];
        p.extend_from_slice(&[1, 2, 3, 4, 5, 6, 7, 8]);
        p.extend_from_slice(&[0, 0, 0, 0, 42, b'h', b'i']);
        p
    }

    #[test]
    fn static_key_parses_and_rejects_bad_input() {
        let key = parse_static_key(&test_key_text()).unwrap();
        assert_eq!(key[0], 3);
        assert_eq!(key[1], 10);
        assert!(parse_static_key("no markers").is_err());
        let short = format!("{PEM_BEGIN}\nabcd\n{PEM_END}");
        assert!(parse_static_key(&short).is_err());
    }

    /// A peer with mirrored direction must accept what we wrap (tls-auth).
    #[test]
    fn tls_auth_round_trips_between_mirrored_directions() {
        let key = parse_static_key(&test_key_text()).unwrap();
        let client = TlsAuth::new(&key, KeyDirection::Inverse, AuthDigest::Sha1);
        let server = TlsAuth::new(&key, KeyDirection::Normal, AuthDigest::Sha1);

        let plain = plain_packet();
        let wire = client.wrap(&plain).unwrap();
        assert_eq!(header_of(&wire), header_of(&plain), "header stays cleartext");
        assert_eq!(server.unwrap(&wire).unwrap(), plain);

        // And the reverse direction.
        let wire = server.wrap(&plain).unwrap();
        assert_eq!(client.unwrap(&wire).unwrap(), plain);
    }

    #[test]
    fn tls_auth_bidirectional_uses_one_key() {
        let key = parse_static_key(&test_key_text()).unwrap();
        let a = TlsAuth::new(&key, KeyDirection::Bidirectional, AuthDigest::Sha256);
        let b = TlsAuth::new(&key, KeyDirection::Bidirectional, AuthDigest::Sha256);
        let plain = plain_packet();
        assert_eq!(b.unwrap(&a.wrap(&plain).unwrap()).unwrap(), plain);
    }

    #[test]
    fn tls_auth_rejects_tampered_packets() {
        let key = parse_static_key(&test_key_text()).unwrap();
        let client = TlsAuth::new(&key, KeyDirection::Inverse, AuthDigest::Sha512);
        let server = TlsAuth::new(&key, KeyDirection::Normal, AuthDigest::Sha512);
        let mut wire = client.wrap(&plain_packet()).unwrap();
        let last = wire.len() - 1;
        wire[last] ^= 0xff;
        assert!(server.unwrap(&wire).is_err());
    }

    /// The server side of tls-crypt mirrors the client's key slots.
    struct ServerTlsCrypt(TlsCrypt);
    impl ServerTlsCrypt {
        fn new(key: &[u8; STATIC_KEY_SIZE]) -> Self {
            let client = TlsCrypt::new(key);
            Self(TlsCrypt {
                send_cipher_key: client.recv_cipher_key,
                send_hmac_key: client.recv_hmac_key,
                recv_cipher_key: client.send_cipher_key,
                recv_hmac_key: client.send_hmac_key,
                replay: ReplayId::new(),
            })
        }
    }

    #[test]
    fn tls_crypt_round_trips_and_hides_the_payload() {
        let key = parse_static_key(&test_key_text()).unwrap();
        let client = TlsCrypt::new(&key);
        let server = ServerTlsCrypt::new(&key);

        let plain = plain_packet();
        let wire = client.wrap(&plain).unwrap();
        assert_eq!(header_of(&wire), header_of(&plain), "header stays cleartext");
        // The reliability body must not appear in the wire packet.
        assert!(!wire.windows(7).any(|w| w == &plain[CONTROL_HEADER_SIZE..]));
        assert_eq!(server.0.unwrap(&wire).unwrap(), plain);

        let wire = server.0.wrap(&plain).unwrap();
        assert_eq!(client.unwrap(&wire).unwrap(), plain);
    }

    #[test]
    fn tls_crypt_rejects_tampered_packets() {
        let key = parse_static_key(&test_key_text()).unwrap();
        let client = TlsCrypt::new(&key);
        let server = ServerTlsCrypt::new(&key);
        let mut wire = client.wrap(&plain_packet()).unwrap();
        let last = wire.len() - 1;
        wire[last] ^= 1;
        assert!(server.0.unwrap(&wire).is_err());
    }

    fn v2_client_key_text(kc: &[u8; STATIC_KEY_SIZE], wkc: &[u8]) -> String {
        let mut raw = kc.to_vec();
        raw.extend_from_slice(wkc);
        let b64 = base64::engine::general_purpose::STANDARD.encode(&raw);
        format!("{PEM_V2_CLIENT_BEGIN}\n{b64}\n{PEM_V2_CLIENT_END}\n")
    }

    fn test_wkc() -> Vec<u8> {
        // Opaque to the client: tag + encrypted Kc + trailing length.
        let mut wkc = vec![0xabu8; MIN_WKC_LEN - WKC_LEN_SIZE];
        wkc.extend_from_slice(&(MIN_WKC_LEN as u16).to_be_bytes());
        wkc
    }

    #[test]
    fn tls_crypt_v2_client_key_parses_and_rejects_bad_input() {
        let kc = parse_static_key(&test_key_text()).unwrap();
        let wkc = test_wkc();
        let (parsed_kc, parsed_wkc) = parse_tls_crypt_v2_client_key(&v2_client_key_text(&kc, &wkc)).unwrap();
        assert_eq!(parsed_kc, kc);
        assert_eq!(parsed_wkc, wkc);

        assert!(parse_tls_crypt_v2_client_key("no markers").is_err());
        assert!(parse_tls_crypt_v2_client_key(&format!("{PEM_V2_CLIENT_BEGIN}\n!!!\n{PEM_V2_CLIENT_END}")).is_err());
        // Too short to hold Kc + a minimal WKc.
        assert!(parse_tls_crypt_v2_client_key(&v2_client_key_text(&kc, &[0u8; 4])).is_err());
        // Trailing length field must match the WKc size.
        let mut bad = test_wkc();
        let last = bad.len() - 1;
        bad[last] ^= 1;
        assert!(parse_tls_crypt_v2_client_key(&v2_client_key_text(&kc, &bad)).is_err());
    }

    #[test]
    fn tls_crypt_v2_appends_wkc_to_the_v3_hard_reset_only() {
        let kc = parse_static_key(&test_key_text()).unwrap();
        let wkc = test_wkc();
        let wrap = ControlWrap::tls_crypt_v2(&v2_client_key_text(&kc, &wkc)).unwrap();
        assert_eq!(wrap.reset_opcode(), P_CONTROL_HARD_RESET_CLIENT_V3);

        let mut reset = plain_packet();
        reset[0] = P_CONTROL_HARD_RESET_CLIENT_V3 << 3;
        let wire = wrap.wrap(&reset).unwrap();
        assert_eq!(&wire[wire.len() - wkc.len()..], &wkc[..], "WKc appended in cleartext");

        // The server (once it has Kc) reads the packet as plain tls-crypt.
        let server = ServerTlsCrypt::new(&kc);
        assert_eq!(server.0.unwrap(&wire[..wire.len() - wkc.len()]).unwrap(), reset);

        // Other control packets are plain tls-crypt: no WKc, round-trips.
        let ctrl = plain_packet();
        let wire = wrap.wrap(&ctrl).unwrap();
        assert_eq!(server.0.unwrap(&wire).unwrap(), ctrl);
        assert_eq!(wrap.unwrap(&server.0.wrap(&ctrl).unwrap()).unwrap(), ctrl);
    }

    #[test]
    fn key_direction_parses() {
        assert_eq!(KeyDirection::from_option(None).unwrap(), KeyDirection::Bidirectional);
        assert_eq!(KeyDirection::from_option(Some(0)).unwrap(), KeyDirection::Normal);
        assert_eq!(KeyDirection::from_option(Some(1)).unwrap(), KeyDirection::Inverse);
        assert!(KeyDirection::from_option(Some(2)).is_err());
    }

    #[test]
    fn auth_digest_parses() {
        assert_eq!(AuthDigest::parse("sha1").unwrap(), AuthDigest::Sha1);
        assert_eq!(AuthDigest::parse("SHA256").unwrap(), AuthDigest::Sha256);
        assert_eq!(AuthDigest::parse("Sha512").unwrap(), AuthDigest::Sha512);
        assert!(AuthDigest::parse("MD5").is_err());
    }
}
