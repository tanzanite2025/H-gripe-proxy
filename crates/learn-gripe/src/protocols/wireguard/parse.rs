use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};

use anyhow::{Context, Result, anyhow, bail};

use super::AllowedIp;

/// Lowercase hex of a byte slice, used to fingerprint keys in the registry key.
pub(super) fn hex(bytes: &[u8]) -> String {
    let mut s = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        s.push_str(&format!("{b:02x}"));
    }
    s
}

/// Parse the optional 3-byte `reserved` field (defaults to all-zero).
pub(super) fn parse_reserved(bytes: Option<&[u8]>) -> Result<[u8; 3]> {
    match bytes {
        Some(bytes) => {
            if bytes.len() != 3 {
                bail!("wireguard: `reserved` must be exactly 3 bytes, got {}", bytes.len());
            }
            Ok([bytes[0], bytes[1], bytes[2]])
        }
        None => Ok([0u8; 3]),
    }
}

/// The default `allowed-ips` for a lone peer: route every inner destination.
pub(super) fn catch_all() -> Vec<AllowedIp> {
    vec![
        AllowedIp::V4(Ipv4Addr::UNSPECIFIED, 0),
        AllowedIp::V6(Ipv6Addr::UNSPECIFIED, 0),
    ]
}

/// Parse a list of `allowed-ips` CIDR entries (`10.0.0.0/24`, `::/0`, or a bare
/// address meaning a host route).
pub(super) fn parse_allowed_ips(list: &[String]) -> Result<Vec<AllowedIp>> {
    let mut out = Vec::with_capacity(list.len());
    for entry in list {
        out.push(parse_allowed_ip(entry).with_context(|| format!("wireguard: invalid `allowed-ips` entry {entry:?}"))?);
    }
    Ok(out)
}

fn parse_allowed_ip(entry: &str) -> Result<AllowedIp> {
    let entry = entry.trim();
    let (addr, prefix) = match entry.split_once('/') {
        Some((addr, prefix)) => (addr, Some(prefix)),
        None => (entry, None),
    };
    let ip = addr.parse::<IpAddr>().map_err(|_| anyhow!("not an IP/CIDR"))?;
    match ip {
        IpAddr::V4(v4) => {
            let p = match prefix {
                Some(p) => p.parse::<u8>().map_err(|_| anyhow!("bad prefix"))?,
                None => 32,
            };
            if p > 32 {
                bail!("IPv4 prefix {p} > 32");
            }
            Ok(AllowedIp::V4(v4, p))
        }
        IpAddr::V6(v6) => {
            let p = match prefix {
                Some(p) => p.parse::<u8>().map_err(|_| anyhow!("bad prefix"))?,
                None => 128,
            };
            if p > 128 {
                bail!("IPv6 prefix {p} > 128");
            }
            Ok(AllowedIp::V6(v6, p))
        }
    }
}

/// Parse a base64-encoded 32-byte WireGuard key.
pub(super) fn parse_key(value: &str) -> Result<[u8; 32]> {
    let bytes = base64_decode(value.trim())?;
    if bytes.len() != 32 {
        bail!("expected a 32-byte key, decoded {} bytes", bytes.len());
    }
    let mut key = [0u8; 32];
    key.copy_from_slice(&bytes);
    Ok(key)
}

/// Parse the assigned IPv4 tunnel address (`ip`), accepting an optional CIDR
/// suffix (`10.0.0.2/32`).
pub(super) fn parse_local_v4(value: &str) -> Result<Ipv4Addr> {
    value
        .trim()
        .split('/')
        .next()
        .unwrap_or("")
        .parse::<Ipv4Addr>()
        .map_err(|_| anyhow!("not an IPv4 address"))
}

/// Parse a `dns` resolver entry: a bare IP (port defaults to 53) or `ip:port`
/// (bracketed for IPv6).
pub(super) fn parse_dns_server(value: &str) -> Result<SocketAddr> {
    let value = value.trim();
    if let Ok(addr) = value.parse::<SocketAddr>() {
        return Ok(addr);
    }
    let ip = value
        .parse::<IpAddr>()
        .map_err(|_| anyhow!("not an IP address or `ip:port`"))?;
    Ok(SocketAddr::new(ip, 53))
}

/// Decode standard or URL-safe Base64 (padding / whitespace ignored).
fn base64_decode(input: &str) -> Result<Vec<u8>> {
    fn sextet(c: u8) -> Option<u8> {
        match c {
            b'A'..=b'Z' => Some(c - b'A'),
            b'a'..=b'z' => Some(c - b'a' + 26),
            b'0'..=b'9' => Some(c - b'0' + 52),
            b'+' | b'-' => Some(62),
            b'/' | b'_' => Some(63),
            _ => None,
        }
    }
    let mut out = Vec::with_capacity(input.len() / 4 * 3);
    let mut acc = 0u32;
    let mut bits = 0u32;
    for &c in input.as_bytes() {
        if c == b'=' || c.is_ascii_whitespace() {
            continue;
        }
        let v = sextet(c).ok_or_else(|| anyhow!("invalid base64 character {:?}", c as char))?;
        acc = (acc << 6) | u32::from(v);
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push((acc >> bits) as u8);
        }
    }
    Ok(out)
}
