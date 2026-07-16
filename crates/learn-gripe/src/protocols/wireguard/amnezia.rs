use anyhow::{Result, bail};
use tokio::net::UdpSocket;

use crate::config::outbound_opts::AmneziaWgOption;

/// Standard WireGuard message sizes (boringtun emits these). Used on the RX
/// side to identify an obfuscated message by its `(padding + size, header)`
/// signature, since AmneziaWG hides the message type behind `H1`-`H4`.
const MSG_INIT_SIZE: usize = 148;
const MSG_RESP_SIZE: usize = 92;
const MSG_COOKIE_SIZE: usize = 64;
/// Smallest transport message (16-byte header + 16-byte Poly1305 tag of an
/// empty keepalive). Transport packets are variable length, so they are matched
/// by `H4` header + this minimum rather than an exact size.
const MSG_TRANSPORT_MIN: usize = 32;

/// Standard WireGuard message-type bytes (byte 0 of a boringtun message).
const TYPE_INIT: u8 = 1;
const TYPE_RESPONSE: u8 = 2;
const TYPE_COOKIE: u8 = 3;
const TYPE_TRANSPORT: u8 = 4;

/// Parsed AmneziaWG obfuscation parameters, shared by every peer in the device.
/// `s1`/`s2` are byte counts prepended to the handshake initiation / response;
/// `h1`-`h4` are the `u32` values written over the 4-byte message-type header of
/// the initiation / response / cookie / transport messages.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct Amnezia {
    jc: u32,
    jmin: u32,
    jmax: u32,
    s1: usize,
    s2: usize,
    h1: u32,
    h2: u32,
    h3: u32,
    h4: u32,
}

impl Amnezia {
    /// Parse and validate the `amnezia-wg-option` block. Unset numeric fields
    /// default to 0 (that obfuscation is skipped). `h1`-`h4` must all be set and
    /// mutually distinct so the RX side can recover the message type, and must
    /// not collide with the standard type bytes (1-4). `jmin <= jmax` when junk
    /// packets are enabled.
    pub(super) fn from_opts(o: &AmneziaWgOption) -> Result<Self> {
        let am = Amnezia {
            jc: o.jc.unwrap_or(0),
            jmin: o.jmin.unwrap_or(0),
            jmax: o.jmax.unwrap_or(0),
            s1: o.s1.unwrap_or(0) as usize,
            s2: o.s2.unwrap_or(0) as usize,
            h1: o.h1.unwrap_or(0),
            h2: o.h2.unwrap_or(0),
            h3: o.h3.unwrap_or(0),
            h4: o.h4.unwrap_or(0),
        };
        let headers = [am.h1, am.h2, am.h3, am.h4];
        if headers.contains(&0) {
            bail!("wireguard: `amnezia-wg-option` requires h1, h2, h3 and h4 to all be set");
        }
        for (i, h) in headers.iter().enumerate() {
            if *h <= TYPE_TRANSPORT as u32 {
                bail!(
                    "wireguard: `amnezia-wg-option` h{} must be > 4 (avoid the standard message types)",
                    i + 1
                );
            }
            if headers[i + 1..].contains(h) {
                bail!("wireguard: `amnezia-wg-option` h1-h4 must be distinct");
            }
        }
        if am.jc > 0 && am.jmin > am.jmax {
            bail!("wireguard: `amnezia-wg-option` requires jmin <= jmax");
        }
        Ok(am)
    }

    /// A fingerprint folded into the device registry key so configs differing
    /// only in obfuscation get their own device.
    pub(super) fn fingerprint(&self) -> String {
        format!(
            "{}:{}:{}:{}:{}:{:x}:{:x}:{:x}:{:x}",
            self.jc, self.jmin, self.jmax, self.s1, self.s2, self.h1, self.h2, self.h3, self.h4
        )
    }

    /// A random junk-packet length in `[jmin, jmax]`.
    fn junk_len(&self) -> usize {
        if self.jmax <= self.jmin {
            return self.jmin as usize;
        }
        let span = (self.jmax - self.jmin + 1) as u64;
        (self.jmin as u64 + random_u64() % span) as usize
    }

    /// The `(header, padding)` pair applied to a message of standard `type`.
    fn transform(&self, std_type: u8) -> Option<(u32, usize)> {
        match std_type {
            TYPE_INIT => Some((self.h1, self.s1)),
            TYPE_RESPONSE => Some((self.h2, self.s2)),
            TYPE_COOKIE => Some((self.h3, 0)),
            TYPE_TRANSPORT => Some((self.h4, 0)),
            _ => None,
        }
    }
}

/// Fill `buf` with random bytes (used for junk packets and S1/S2 padding).
fn fill_random(buf: &mut [u8]) {
    if getrandom::fill(buf).is_err() {
        // The system RNG being unavailable is fatal elsewhere (key/index gen);
        // here we degrade to a cheap fallback so obfuscation never panics.
        for b in buf.iter_mut() {
            *b = 0;
        }
    }
}

/// A random `u64` from the system RNG (falls back to 0 if unavailable).
fn random_u64() -> u64 {
    let mut b = [0u8; 8];
    let _ = getrandom::fill(&mut b);
    u64::from_le_bytes(b)
}

/// Send a boringtun-produced datagram to the peer. With AmneziaWG configured
/// this sends `jc` random junk packets ahead of a handshake initiation, prepends
/// the `S1`/`S2` random padding, and rewrites the 4-byte message-type header
/// (`H1`-`H4`); otherwise it just stamps the `reserved` tag. `out` is the raw
/// WireGuard message from boringtun (byte 0 is the standard type 1-4).
pub(super) async fn send_obfuscated(amnezia: &Option<Amnezia>, udp: &UdpSocket, reserved: [u8; 3], out: &mut [u8]) {
    let Some(am) = amnezia else {
        apply_reserved(out, reserved);
        let _ = udp.send(out).await;
        return;
    };
    if out.len() < 4 {
        let _ = udp.send(out).await;
        return;
    }
    let std_type = out[0];
    // Junk packets precede every handshake initiation.
    if std_type == TYPE_INIT {
        for _ in 0..am.jc {
            let mut junk = vec![0u8; am.junk_len()];
            fill_random(&mut junk);
            let _ = udp.send(&junk).await;
        }
    }
    let Some((header, pad)) = am.transform(std_type) else {
        let _ = udp.send(out).await;
        return;
    };
    let mut buf = vec![0u8; pad + out.len()];
    if pad > 0 {
        fill_random(&mut buf[..pad]);
    }
    buf[pad..].copy_from_slice(out);
    buf[pad..pad + 4].copy_from_slice(&header.to_le_bytes());
    let _ = udp.send(&buf).await;
}

/// Reverse AmneziaWG obfuscation on a received datagram in place: identify the
/// message by its `(padding, size, H-header)` signature, drop the `S1`/`S2`
/// prefix, and restore the standard 4-byte message-type header so boringtun can
/// parse it. Returns the deobfuscated length, or `None` to drop the datagram
/// (a junk packet or anything unrecognised).
pub(super) fn deobfuscate(am: &Amnezia, buf: &mut [u8]) -> Option<usize> {
    let size = buf.len();
    let header_at = |off: usize| {
        buf.get(off..off + 4)
            .map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
    };

    let (pad, std_type) = if size == am.s1 + MSG_INIT_SIZE && header_at(am.s1) == Some(am.h1) {
        (am.s1, TYPE_INIT)
    } else if size == am.s2 + MSG_RESP_SIZE && header_at(am.s2) == Some(am.h2) {
        (am.s2, TYPE_RESPONSE)
    } else if size == MSG_COOKIE_SIZE && header_at(0) == Some(am.h3) {
        (0, TYPE_COOKIE)
    } else if size >= MSG_TRANSPORT_MIN && header_at(0) == Some(am.h4) {
        (0, TYPE_TRANSPORT)
    } else {
        return None;
    };

    if pad > 0 {
        buf.copy_within(pad.., 0);
    }
    let n = size - pad;
    buf[0] = std_type;
    buf[1] = 0;
    buf[2] = 0;
    buf[3] = 0;
    Some(n)
}

/// Stamp the 3-byte WireGuard `reserved` field (bytes 1..4 of the message
/// header) on an outgoing datagram. A no-op for the default all-zero value.
fn apply_reserved(datagram: &mut [u8], reserved: [u8; 3]) {
    if reserved != [0u8; 3] && datagram.len() >= 4 {
        datagram[1..4].copy_from_slice(&reserved);
    }
}

/// Zero the `reserved` field before handing a received datagram to boringtun
/// (which validates those bytes are zero).
pub(super) fn clear_reserved(datagram: &mut [u8]) {
    if datagram.len() >= 4 {
        datagram[1] = 0;
        datagram[2] = 0;
        datagram[3] = 0;
    }
}
