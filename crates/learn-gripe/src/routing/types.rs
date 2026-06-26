//! Value types parsed from rule payloads: CIDR blocks, port ranges, uid ranges.

use std::fmt;
use std::net::IpAddr;

use anyhow::{Result, bail};

/// A CIDR block (`addr/prefix`) supporting both IPv4 and IPv6.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IpCidr {
    network: IpAddr,
    prefix: u8,
}

impl IpCidr {
    /// Parse a CIDR such as `10.0.0.0/8` or `2001:db8::/32`. A bare address
    /// (no `/prefix`) is treated as a host route (`/32` or `/128`).
    pub fn parse(s: &str) -> Result<Self> {
        let (addr_str, prefix) = match s.split_once('/') {
            Some((addr, prefix)) => {
                let prefix: u8 = prefix
                    .parse()
                    .map_err(|_| anyhow::anyhow!("invalid CIDR prefix in {s:?}"))?;
                (addr, Some(prefix))
            }
            None => (s, None),
        };
        let addr: IpAddr = addr_str
            .parse()
            .map_err(|_| anyhow::anyhow!("invalid CIDR address in {s:?}"))?;
        let max = match addr {
            IpAddr::V4(_) => 32,
            IpAddr::V6(_) => 128,
        };
        let prefix = prefix.unwrap_or(max);
        if prefix > max {
            bail!("CIDR prefix /{prefix} out of range for {addr}");
        }
        Ok(Self {
            network: masked(addr, prefix),
            prefix,
        })
    }

    /// Whether `ip` falls inside this block.
    pub fn contains(&self, ip: IpAddr) -> bool {
        match (self.network, ip) {
            (IpAddr::V4(_), IpAddr::V4(_)) | (IpAddr::V6(_), IpAddr::V6(_)) => masked(ip, self.prefix) == self.network,
            _ => false,
        }
    }
}

impl fmt::Display for IpCidr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}/{}", self.network, self.prefix)
    }
}

/// An inclusive destination-port range used by the `DST-PORT` matcher. A single
/// port (`443`) parses as a one-wide range (`start == end`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PortRange {
    start: u16,
    end: u16,
}

impl PortRange {
    /// Parse a single port (`443`) or an inclusive range (`8000-9000`). The
    /// bounds must be valid `u16`s and `start` must not exceed `end`.
    pub fn parse(s: &str) -> Result<Self> {
        let s = s.trim();
        let (start, end) = match s.split_once('-') {
            Some((a, b)) => {
                let start = a
                    .trim()
                    .parse()
                    .map_err(|_| anyhow::anyhow!("invalid port range start in {s:?}"))?;
                let end = b
                    .trim()
                    .parse()
                    .map_err(|_| anyhow::anyhow!("invalid port range end in {s:?}"))?;
                (start, end)
            }
            None => {
                let port = s.parse().map_err(|_| anyhow::anyhow!("invalid port in {s:?}"))?;
                (port, port)
            }
        };
        if start > end {
            bail!("port range start {start} exceeds end {end}");
        }
        Ok(Self { start, end })
    }

    /// Whether `port` falls inside this inclusive range.
    pub fn contains(&self, port: u16) -> bool {
        self.start <= port && port <= self.end
    }
}

impl fmt::Display for PortRange {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.start == self.end {
            write!(f, "{}", self.start)
        } else {
            write!(f, "{}-{}", self.start, self.end)
        }
    }
}

/// An inclusive user-id range used by the `UID` matcher. A single uid (`1000`)
/// parses as a one-wide range (`start == end`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UidRange {
    start: u32,
    end: u32,
}

impl UidRange {
    /// Parse a single uid (`1000`) or an inclusive range (`1000-2000`). The
    /// bounds must be valid `u32`s and `start` must not exceed `end`.
    pub fn parse(s: &str) -> Result<Self> {
        let s = s.trim();
        let (start, end) = match s.split_once('-') {
            Some((a, b)) => {
                let start = a
                    .trim()
                    .parse()
                    .map_err(|_| anyhow::anyhow!("invalid uid range start in {s:?}"))?;
                let end = b
                    .trim()
                    .parse()
                    .map_err(|_| anyhow::anyhow!("invalid uid range end in {s:?}"))?;
                (start, end)
            }
            None => {
                let uid = s.parse().map_err(|_| anyhow::anyhow!("invalid uid in {s:?}"))?;
                (uid, uid)
            }
        };
        if start > end {
            bail!("uid range start {start} exceeds end {end}");
        }
        Ok(Self { start, end })
    }

    /// Whether `uid` falls inside this inclusive range.
    pub fn contains(&self, uid: u32) -> bool {
        self.start <= uid && uid <= self.end
    }
}

impl fmt::Display for UidRange {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.start == self.end {
            write!(f, "{}", self.start)
        } else {
            write!(f, "{}-{}", self.start, self.end)
        }
    }
}

/// Zero out the host bits of `addr` below `prefix`.
fn masked(addr: IpAddr, prefix: u8) -> IpAddr {
    match addr {
        IpAddr::V4(v4) => {
            let bits = u32::from(v4);
            let mask = if prefix == 0 { 0 } else { u32::MAX << (32 - prefix) };
            IpAddr::V4((bits & mask).into())
        }
        IpAddr::V6(v6) => {
            let bits = u128::from(v6);
            let mask = if prefix == 0 { 0 } else { u128::MAX << (128 - prefix) };
            IpAddr::V6((bits & mask).into())
        }
    }
}
