//! IP/UDP frame codec: parse the TCP 5-tuple and UDP datagrams off the wire and
//! build UDP reply frames. smoltcp does the byte-level work; this just adapts
//! between raw frames and the module's flow types.

use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};

use smoltcp::phy::ChecksumCapabilities;
use smoltcp::wire::{
    IpAddress, IpEndpoint, IpProtocol, Ipv4Packet, Ipv4Repr, Ipv6Packet, Ipv6Repr, TcpPacket, UdpPacket, UdpRepr,
};

use super::udp::{UdpDatagram, UdpFlow};

/// Parse just enough of an IP frame to extract the TCP 5-tuple and SYN flag.
pub(super) fn parse_tcp_endpoints(frame: &[u8]) -> Option<(IpEndpoint, IpEndpoint, bool)> {
    match frame.first().map(|b| b >> 4) {
        Some(4) => {
            let ip = Ipv4Packet::new_checked(frame).ok()?;
            if ip.next_header() != IpProtocol::Tcp {
                return None;
            }
            let tcp = TcpPacket::new_checked(ip.payload()).ok()?;
            let src = IpEndpoint::new(IpAddress::Ipv4(ip.src_addr()), tcp.src_port());
            let dst = IpEndpoint::new(IpAddress::Ipv4(ip.dst_addr()), tcp.dst_port());
            Some((src, dst, tcp.syn() && !tcp.ack()))
        }
        Some(6) => {
            let ip = Ipv6Packet::new_checked(frame).ok()?;
            if ip.next_header() != IpProtocol::Tcp {
                return None;
            }
            let tcp = TcpPacket::new_checked(ip.payload()).ok()?;
            let src = IpEndpoint::new(IpAddress::Ipv6(ip.src_addr()), tcp.src_port());
            let dst = IpEndpoint::new(IpAddress::Ipv6(ip.dst_addr()), tcp.dst_port());
            Some((src, dst, tcp.syn() && !tcp.ack()))
        }
        _ => None,
    }
}

pub(super) fn endpoint_socketaddr(endpoint: IpEndpoint) -> SocketAddr {
    let ip = match endpoint.addr {
        IpAddress::Ipv4(addr) => IpAddr::V4(Ipv4Addr::from(addr.octets())),
        IpAddress::Ipv6(addr) => IpAddr::V6(Ipv6Addr::from(addr.octets())),
    };
    SocketAddr::new(ip, endpoint.port)
}

/// Parse an IP frame as a UDP datagram, extracting the endpoints and payload.
/// Returns `None` for anything that is not a well-formed IPv4/IPv6 UDP packet.
pub(super) fn parse_udp_datagram(frame: &[u8]) -> Option<UdpDatagram> {
    match frame.first().map(|b| b >> 4) {
        Some(4) => {
            let ip = Ipv4Packet::new_checked(frame).ok()?;
            if ip.next_header() != IpProtocol::Udp {
                return None;
            }
            let udp = UdpPacket::new_checked(ip.payload()).ok()?;
            Some(UdpDatagram {
                flow: UdpFlow {
                    src_addr: IpAddress::Ipv4(ip.src_addr()),
                    dst_addr: IpAddress::Ipv4(ip.dst_addr()),
                    src_port: udp.src_port(),
                    dst_port: udp.dst_port(),
                },
                payload: udp.payload().to_vec(),
            })
        }
        Some(6) => {
            let ip = Ipv6Packet::new_checked(frame).ok()?;
            if ip.next_header() != IpProtocol::Udp {
                return None;
            }
            let udp = UdpPacket::new_checked(ip.payload()).ok()?;
            Some(UdpDatagram {
                flow: UdpFlow {
                    src_addr: IpAddress::Ipv6(ip.src_addr()),
                    dst_addr: IpAddress::Ipv6(ip.dst_addr()),
                    src_port: udp.src_port(),
                    dst_port: udp.dst_port(),
                },
                payload: udp.payload().to_vec(),
            })
        }
        _ => None,
    }
}

/// Build the IP+UDP reply frame for `flow`, carrying `payload`. The reply swaps
/// source/destination (so it appears to come from the host the client targeted)
/// and lets smoltcp compute the checksums.
pub(super) fn build_udp_reply_frame(flow: &UdpFlow, payload: &[u8]) -> Option<Vec<u8>> {
    let udp_repr = UdpRepr {
        src_port: flow.dst_port,
        dst_port: flow.src_port,
    };
    let caps = ChecksumCapabilities::default();

    match (flow.dst_addr, flow.src_addr) {
        // Reply source = original destination, reply destination = original source.
        (IpAddress::Ipv4(reply_src), IpAddress::Ipv4(reply_dst)) => {
            let ip_repr = Ipv4Repr {
                src_addr: reply_src,
                dst_addr: reply_dst,
                next_header: IpProtocol::Udp,
                payload_len: udp_repr.header_len() + payload.len(),
                hop_limit: 64,
            };
            let mut frame = vec![0u8; ip_repr.buffer_len() + ip_repr.payload_len];
            let mut packet = Ipv4Packet::new_unchecked(&mut frame);
            ip_repr.emit(&mut packet, &caps);
            let mut udp = UdpPacket::new_unchecked(packet.payload_mut());
            udp_repr.emit(
                &mut udp,
                &IpAddress::Ipv4(reply_src),
                &IpAddress::Ipv4(reply_dst),
                payload.len(),
                |buf| buf.copy_from_slice(payload),
                &caps,
            );
            Some(frame)
        }
        (IpAddress::Ipv6(reply_src), IpAddress::Ipv6(reply_dst)) => {
            let ip_repr = Ipv6Repr {
                src_addr: reply_src,
                dst_addr: reply_dst,
                next_header: IpProtocol::Udp,
                payload_len: udp_repr.header_len() + payload.len(),
                hop_limit: 64,
            };
            let mut frame = vec![0u8; ip_repr.buffer_len() + ip_repr.payload_len];
            let mut packet = Ipv6Packet::new_unchecked(&mut frame);
            ip_repr.emit(&mut packet);
            let mut udp = UdpPacket::new_unchecked(packet.payload_mut());
            udp_repr.emit(
                &mut udp,
                &IpAddress::Ipv6(reply_src),
                &IpAddress::Ipv6(reply_dst),
                payload.len(),
                |buf| buf.copy_from_slice(payload),
                &caps,
            );
            Some(frame)
        }
        // Mixed address families cannot occur within a single IP packet.
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use smoltcp::wire::{Ipv4Address, Ipv4Repr, TcpControl, TcpRepr};

    #[test]
    fn parses_ipv4_tcp_syn_endpoints() {
        // Build a minimal IPv4 + TCP SYN with smoltcp's own wire writers.
        let src = IpEndpoint::new(IpAddress::Ipv4(Ipv4Address::new(10, 0, 0, 1)), 40000);
        let dst = IpEndpoint::new(IpAddress::Ipv4(Ipv4Address::new(93, 184, 216, 34)), 443);

        let tcp_repr = TcpRepr {
            src_port: src.port,
            dst_port: dst.port,
            control: TcpControl::Syn,
            seq_number: smoltcp::wire::TcpSeqNumber(0),
            ack_number: None,
            window_len: 64240,
            window_scale: None,
            max_seg_size: None,
            sack_permitted: false,
            sack_ranges: [None, None, None],
            timestamp: None,
            payload: &[],
        };
        let ipv4_repr = Ipv4Repr {
            src_addr: Ipv4Address::new(10, 0, 0, 1),
            dst_addr: Ipv4Address::new(93, 184, 216, 34),
            next_header: IpProtocol::Tcp,
            payload_len: tcp_repr.buffer_len(),
            hop_limit: 64,
        };
        let mut frame = vec![0u8; ipv4_repr.buffer_len() + tcp_repr.buffer_len()];
        let mut ipv4_packet = Ipv4Packet::new_unchecked(&mut frame);
        ipv4_repr.emit(&mut ipv4_packet, &ChecksumCapabilities::default());
        let mut tcp_packet = TcpPacket::new_unchecked(ipv4_packet.payload_mut());
        tcp_repr.emit(
            &mut tcp_packet,
            &IpAddress::Ipv4(ipv4_repr.src_addr),
            &IpAddress::Ipv4(ipv4_repr.dst_addr),
            &ChecksumCapabilities::default(),
        );

        let (parsed_src, parsed_dst, is_syn) = parse_tcp_endpoints(&frame).expect("parse syn");
        assert_eq!(parsed_src, src);
        assert_eq!(parsed_dst, dst);
        assert!(is_syn);
        assert_eq!(endpoint_socketaddr(dst), "93.184.216.34:443".parse().unwrap());
    }

    #[test]
    fn ignores_non_tcp_and_garbage() {
        assert!(parse_tcp_endpoints(&[]).is_none());
        assert!(parse_tcp_endpoints(&[0x45]).is_none());
        // IPv4 header with UDP protocol -> ignored.
        let mut frame = vec![0u8; 28];
        frame[0] = 0x45;
        frame[9] = IpProtocol::Udp.into();
        assert!(parse_tcp_endpoints(&frame).is_none());
    }
}
