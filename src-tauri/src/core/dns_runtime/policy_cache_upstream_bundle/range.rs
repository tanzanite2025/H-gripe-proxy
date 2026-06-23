use anyhow::{Context as _, Result, anyhow};
use std::net::Ipv4Addr;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct FakeIpRange {
    network: u32,
    prefix: u8,
}

impl FakeIpRange {
    pub(super) fn parse(input: &str) -> Result<Self> {
        let (addr, prefix) = input
            .split_once('/')
            .ok_or_else(|| anyhow!("fake-ip-range must be CIDR notation"))?;
        let ip = addr
            .parse::<Ipv4Addr>()
            .with_context(|| format!("invalid fake-ip-range address: {addr}"))?;
        let prefix = prefix
            .parse::<u8>()
            .with_context(|| format!("invalid fake-ip-range prefix: {prefix}"))?;
        if prefix > 30 {
            return Err(anyhow!("fake-ip-range prefix must leave at least two host addresses"));
        }
        let mask = if prefix == 0 { 0 } else { u32::MAX << (32 - prefix) };
        Ok(Self {
            network: u32::from(ip) & mask,
            prefix,
        })
    }

    pub(super) fn allocate(self, domain: &str) -> Ipv4Addr {
        let host_bits = 32 - self.prefix;
        let usable = (1_u64 << host_bits) - 2;
        let host = (stable_domain_hash(domain) % usable) + 1;
        Ipv4Addr::from(self.network + host as u32)
    }

    pub(super) fn contains(self, ip: Ipv4Addr) -> bool {
        let mask = if self.prefix == 0 {
            0
        } else {
            u32::MAX << (32 - self.prefix)
        };
        (u32::from(ip) & mask) == self.network
    }
}

impl std::fmt::Display for FakeIpRange {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{}/{}", Ipv4Addr::from(self.network), self.prefix)
    }
}

fn stable_domain_hash(domain: &str) -> u64 {
    domain.bytes().fold(0xcbf29ce484222325, |hash, byte| {
        (hash ^ u64::from(byte)).wrapping_mul(0x100000001b3)
    })
}
