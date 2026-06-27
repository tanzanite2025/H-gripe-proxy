//! simple-obfs (obfs-local) client transports.
//!
//! `simple-obfs` is a SIP003 Shadowsocks plugin that disguises the proxy stream
//! as innocuous traffic. The Shadowsocks AEAD stream is wrapped in one of two
//! one-shot obfuscation framings before it reaches the socket:
//!
//! * [`http`] — a fake WebSocket-`Upgrade` exchange. The first bytes are sent as
//!   an HTTP request and the server replies `101 Switching Protocols`; after the
//!   headers are stripped the connection is a plain passthrough.
//! * [`tls`] — a fake TLS 1.2 handshake. The first bytes ride inside a
//!   `ClientHello`'s session-ticket extension, the server's fixed handshake
//!   response is skipped, and subsequent traffic is framed as TLS application
//!   data records.
//!
//! Only the client side is implemented (learn-gripe always dials outbound). The
//! framing is compatible with the `shadowsocks/simple-obfs` server and the
//! clash/mihomo client, so real obfs nodes interoperate.

pub mod http;
pub mod tls;

pub use http::connect_http;
pub use tls::connect_tls;

/// Find the first occurrence of `needle` in `haystack`.
pub(crate) fn find_subsequence(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    if needle.is_empty() || haystack.len() < needle.len() {
        return None;
    }
    haystack.windows(needle.len()).position(|w| w == needle)
}

/// Standard Base64 (RFC 4648) encoder. Only used to synthesise a plausible
/// `Sec-WebSocket-Key`, so it does not need to be fast.
pub(crate) fn base64_encode(input: &[u8]) -> String {
    const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(input.len().div_ceil(3) * 4);
    for chunk in input.chunks(3) {
        let b0 = chunk[0] as u32;
        let b1 = *chunk.get(1).unwrap_or(&0) as u32;
        let b2 = *chunk.get(2).unwrap_or(&0) as u32;
        let n = (b0 << 16) | (b1 << 8) | b2;
        out.push(ALPHABET[(n >> 18) as usize & 0x3f] as char);
        out.push(ALPHABET[(n >> 12) as usize & 0x3f] as char);
        out.push(if chunk.len() > 1 {
            ALPHABET[(n >> 6) as usize & 0x3f] as char
        } else {
            '='
        });
        out.push(if chunk.len() > 2 {
            ALPHABET[n as usize & 0x3f] as char
        } else {
            '='
        });
    }
    out
}

/// Fill `buf` with OS random bytes, mapping RNG failure to an error.
pub(crate) fn random_bytes(buf: &mut [u8]) -> anyhow::Result<()> {
    getrandom::fill(buf).map_err(|_| anyhow::anyhow!("simple-obfs: system RNG unavailable"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base64_encode_matches_rfc4648_vectors() {
        assert_eq!(base64_encode(b""), "");
        assert_eq!(base64_encode(b"f"), "Zg==");
        assert_eq!(base64_encode(b"fo"), "Zm8=");
        assert_eq!(base64_encode(b"foo"), "Zm9v");
        assert_eq!(base64_encode(b"foob"), "Zm9vYg==");
        assert_eq!(base64_encode(b"fooba"), "Zm9vYmE=");
        assert_eq!(base64_encode(b"foobar"), "Zm9vYmFy");
    }

    #[test]
    fn finds_subsequence() {
        assert_eq!(find_subsequence(b"abc\r\n\r\ndef", b"\r\n\r\n"), Some(3));
        assert_eq!(find_subsequence(b"no terminator", b"\r\n\r\n"), None);
    }
}
