//! Shared cryptographic helpers for the SSR layers (key derivation, RNG,
//! HMAC, RC4, base64).

use hmac::{Hmac, Mac};
use md5::Md5;
use sha1::Sha1;

use super::protocol::AuthHashKind;

/// One-shot RC4 (plain, key used directly — not the `MD5(key||iv)` of rc4-md5).
pub(super) fn rc4_apply(key: &[u8], data: &mut [u8]) {
    let mut s = [0u8; 256];
    for (i, b) in s.iter_mut().enumerate() {
        *b = i as u8;
    }
    let mut j: u8 = 0;
    for i in 0..256 {
        j = j.wrapping_add(s[i]).wrapping_add(key[i % key.len()]);
        s.swap(i, j as usize);
    }
    let (mut i, mut j) = (0u8, 0u8);
    for byte in data.iter_mut() {
        i = i.wrapping_add(1);
        j = j.wrapping_add(s[i as usize]);
        s.swap(i as usize, j as usize);
        let k = s[s[i as usize].wrapping_add(s[j as usize]) as usize];
        *byte ^= k;
    }
}

/// Standard base64 (RFC 4648, `+/` alphabet, `=` padding).
pub(super) fn base64_encode(data: &[u8]) -> Vec<u8> {
    const TABLE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = Vec::with_capacity(data.len().div_ceil(3) * 4);
    for chunk in data.chunks(3) {
        let b0 = chunk[0] as u32;
        let b1 = *chunk.get(1).unwrap_or(&0) as u32;
        let b2 = *chunk.get(2).unwrap_or(&0) as u32;
        let n = (b0 << 16) | (b1 << 8) | b2;
        out.push(TABLE[((n >> 18) & 0x3f) as usize]);
        out.push(TABLE[((n >> 12) & 0x3f) as usize]);
        out.push(if chunk.len() > 1 {
            TABLE[((n >> 6) & 0x3f) as usize]
        } else {
            b'='
        });
        out.push(if chunk.len() > 2 {
            TABLE[(n & 0x3f) as usize]
        } else {
            b'='
        });
    }
    out
}

/// HMAC over `msg` keyed by `key`, selecting SHA-1 or MD5.
pub(super) fn hmac_digest(hash: AuthHashKind, key: &[u8], msg: &[u8]) -> Vec<u8> {
    match hash {
        AuthHashKind::Sha1 => hmac_sha1(key, msg).to_vec(),
        AuthHashKind::Md5 => hmac_md5(key, msg).to_vec(),
    }
}

pub(super) fn hmac_sha1(key: &[u8], msg: &[u8]) -> [u8; 20] {
    let mut mac = <Hmac<Sha1> as Mac>::new_from_slice(key).expect("HMAC key length");
    mac.update(msg);
    mac.finalize().into_bytes().into()
}

pub(super) fn hmac_md5(key: &[u8], msg: &[u8]) -> [u8; 16] {
    let mut mac = <Hmac<Md5> as Mac>::new_from_slice(key).expect("HMAC key length");
    mac.update(msg);
    mac.finalize().into_bytes().into()
}

/// `EVP_BytesToKey` key derivation (MD5-based, shared with classic Shadowsocks).
pub(super) fn evp_bytes_to_key(password: &[u8], key_len: usize) -> Vec<u8> {
    use md5::Digest;
    let mut key = Vec::with_capacity(key_len);
    let mut prev = Vec::new();
    while key.len() < key_len {
        let mut hasher = Md5::new();
        hasher.update(&prev);
        hasher.update(password);
        let hash: [u8; 16] = hasher.finalize().into();
        key.extend_from_slice(&hash);
        prev = hash.to_vec();
    }
    key.truncate(key_len);
    key
}

/// Fill `buf` with cryptographically secure random bytes.
pub(super) fn random_bytes(buf: &mut [u8]) {
    if buf.is_empty() {
        return;
    }
    if getrandom::fill(buf).is_err() {
        panic!("ssr: system RNG unavailable");
    }
}

/// Return a random u16.
pub(super) fn random_u16() -> u16 {
    let mut buf = [0u8; 2];
    random_bytes(&mut buf);
    u16::from_le_bytes(buf)
}
