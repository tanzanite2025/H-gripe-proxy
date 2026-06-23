use super::constants::{SOCKS_UDP_FINAL_MASK, SOCKS_UDP_FRAGMENT_INDEX_MASK};
use anyhow::{Result, anyhow};
use std::net::{IpAddr, Ipv4Addr, SocketAddr};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct SocksUdpFragment {
    frag: u8,
    pub(super) target: SocketAddr,
    pub(super) payload: Vec<u8>,
}

impl SocksUdpFragment {
    pub(super) fn index(&self) -> u8 {
        self.frag & SOCKS_UDP_FRAGMENT_INDEX_MASK
    }

    pub(super) fn is_final(&self) -> bool {
        self.frag & SOCKS_UDP_FINAL_MASK != 0
    }
}

pub(super) fn parse_socks_udp_fragment(packet: &[u8]) -> Result<SocksUdpFragment> {
    if packet.len() < 10 {
        return Err(anyhow!("SOCKS UDP fragment is truncated"));
    }
    if packet[0] != 0 || packet[1] != 0 {
        return Err(anyhow!("SOCKS UDP RSV bytes must be zero"));
    }
    if packet[2] == 0 {
        return Err(anyhow!("SOCKS UDP fragment path requires non-zero FRAG"));
    }
    if packet[3] != 0x01 {
        return Err(anyhow!("bounded SOCKS UDP fragment path only supports IPv4 ATYP"));
    }
    let ip = Ipv4Addr::new(packet[4], packet[5], packet[6], packet[7]);
    let port = u16::from_be_bytes([packet[8], packet[9]]);
    Ok(SocksUdpFragment {
        frag: packet[2],
        target: SocketAddr::new(IpAddr::V4(ip), port),
        payload: packet[10..].to_vec(),
    })
}

pub(super) fn encode_socks_udp_fragment(target: SocketAddr, frag: u8, payload: &[u8]) -> Vec<u8> {
    let mut packet = vec![0, 0, frag];
    match target.ip() {
        IpAddr::V4(ip) => {
            packet.push(0x01);
            packet.extend_from_slice(&ip.octets());
        }
        IpAddr::V6(ip) => {
            packet.push(0x04);
            packet.extend_from_slice(&ip.octets());
        }
    }
    packet.extend_from_slice(&target.port().to_be_bytes());
    packet.extend_from_slice(payload);
    packet
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_standalone_datagram_in_fragment_path() {
        let target = SocketAddr::from((Ipv4Addr::LOCALHOST, 2082));
        let packet = encode_socks_udp_fragment(target, 0, b"standalone");

        assert!(parse_socks_udp_fragment(&packet).is_err());
    }
}
