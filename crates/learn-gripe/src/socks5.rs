//! Minimal SOCKS5 wire helpers shared by the inbound server and the upstream
//! outbound client. Implements the no-authentication CONNECT subset of
//! RFC 1928, which is sufficient for the MVP data plane.

use crate::address::TargetAddr;
use anyhow::{Result, anyhow, bail};
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

pub const VERSION: u8 = 0x05;
pub const CMD_CONNECT: u8 = 0x01;
pub const CMD_UDP_ASSOCIATE: u8 = 0x03;
pub const RSV: u8 = 0x00;

pub const METHOD_NO_AUTH: u8 = 0x00;
pub const METHOD_NO_ACCEPTABLE: u8 = 0xFF;

pub const ATYP_IPV4: u8 = 0x01;
pub const ATYP_DOMAIN: u8 = 0x03;
pub const ATYP_IPV6: u8 = 0x04;

pub const REP_SUCCEEDED: u8 = 0x00;
pub const REP_GENERAL_FAILURE: u8 = 0x01;
pub const REP_CMD_NOT_SUPPORTED: u8 = 0x07;

/// Read the client method-selection greeting and reply that no authentication
/// is required. Returns an error if the client offers no compatible method.
pub async fn server_handshake<S>(stream: &mut S) -> Result<()>
where
    S: AsyncRead + AsyncWrite + Unpin,
{
    let mut header = [0u8; 2];
    stream.read_exact(&mut header).await?;
    if header[0] != VERSION {
        bail!("unsupported SOCKS version: {}", header[0]);
    }
    let n_methods = header[1] as usize;
    let mut methods = vec![0u8; n_methods];
    stream.read_exact(&mut methods).await?;

    if methods.contains(&METHOD_NO_AUTH) {
        stream.write_all(&[VERSION, METHOD_NO_AUTH]).await?;
        Ok(())
    } else {
        stream.write_all(&[VERSION, METHOD_NO_ACCEPTABLE]).await?;
        bail!("client offered no no-auth method");
    }
}

/// A SOCKS5 request command that the inbound supports.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Command {
    Connect,
    UdpAssociate,
}

/// Read a request header and return its command plus target address. Replies
/// with `REP_CMD_NOT_SUPPORTED` (and errors) for commands the kernel does not
/// implement, e.g. `BIND`.
pub async fn read_request<S>(stream: &mut S) -> Result<(Command, TargetAddr)>
where
    S: AsyncRead + AsyncWrite + Unpin,
{
    let mut header = [0u8; 4];
    stream.read_exact(&mut header).await?;
    if header[0] != VERSION {
        bail!("unsupported SOCKS version in request: {}", header[0]);
    }
    let command = match header[1] {
        CMD_CONNECT => Command::Connect,
        CMD_UDP_ASSOCIATE => Command::UdpAssociate,
        other => {
            write_reply(stream, REP_CMD_NOT_SUPPORTED).await?;
            bail!("unsupported SOCKS command: {other}");
        }
    };
    let target = read_address(stream, header[3]).await?;
    Ok((command, target))
}

/// Write a SOCKS5 reply with a zero bind address.
pub async fn write_reply<S>(stream: &mut S, rep: u8) -> Result<()>
where
    S: AsyncWrite + Unpin,
{
    stream
        .write_all(&[VERSION, rep, RSV, ATYP_IPV4, 0, 0, 0, 0, 0, 0])
        .await?;
    Ok(())
}

/// Write a SOCKS5 reply carrying a concrete bound address. Used to answer a
/// `UDP ASSOCIATE` with the relay socket the client should send datagrams to.
pub async fn write_reply_with_addr<S>(stream: &mut S, rep: u8, addr: SocketAddr) -> Result<()>
where
    S: AsyncWrite + Unpin,
{
    let mut reply = vec![VERSION, rep, RSV];
    encode_address(&mut reply, &TargetAddr::Ip(addr));
    stream.write_all(&reply).await?;
    Ok(())
}

/// Parse a SOCKS5 UDP relay datagram (`RSV RSV FRAG ATYP DST.ADDR DST.PORT
/// DATA`). Returns the destination and the offset where the payload begins.
/// Fragmented datagrams (`FRAG != 0`) are rejected since reassembly is not
/// supported.
pub fn parse_udp_datagram(buf: &[u8]) -> Result<(TargetAddr, usize)> {
    if buf.len() < 4 {
        bail!("UDP datagram too short");
    }
    if buf[2] != 0 {
        bail!("fragmented UDP datagrams are not supported");
    }
    let atyp = buf[3];
    let mut cursor = 4usize;
    let target = match atyp {
        ATYP_IPV4 => {
            if buf.len() < cursor + 6 {
                bail!("UDP datagram truncated (ipv4)");
            }
            let octets: [u8; 4] = [buf[cursor], buf[cursor + 1], buf[cursor + 2], buf[cursor + 3]];
            cursor += 4;
            let port = u16::from_be_bytes([buf[cursor], buf[cursor + 1]]);
            cursor += 2;
            TargetAddr::Ip(SocketAddr::new(IpAddr::V4(Ipv4Addr::from(octets)), port))
        }
        ATYP_IPV6 => {
            if buf.len() < cursor + 18 {
                bail!("UDP datagram truncated (ipv6)");
            }
            let mut octets = [0u8; 16];
            octets.copy_from_slice(&buf[cursor..cursor + 16]);
            cursor += 16;
            let port = u16::from_be_bytes([buf[cursor], buf[cursor + 1]]);
            cursor += 2;
            TargetAddr::Ip(SocketAddr::new(IpAddr::V6(Ipv6Addr::from(octets)), port))
        }
        ATYP_DOMAIN => {
            let len = *buf
                .get(cursor)
                .ok_or_else(|| anyhow!("UDP datagram truncated (domain len)"))? as usize;
            cursor += 1;
            if buf.len() < cursor + len + 2 {
                bail!("UDP datagram truncated (domain)");
            }
            let host = String::from_utf8(buf[cursor..cursor + len].to_vec())
                .map_err(|_| anyhow!("UDP domain is not valid UTF-8"))?;
            cursor += len;
            let port = u16::from_be_bytes([buf[cursor], buf[cursor + 1]]);
            cursor += 2;
            TargetAddr::Domain(host, port)
        }
        other => bail!("unsupported UDP address type: {other}"),
    };
    Ok((target, cursor))
}

/// Encode a SOCKS5 UDP relay datagram header for `source` followed by `payload`.
/// Used on the reverse path to wrap data coming back from the remote host.
pub fn encode_udp_datagram(source: &TargetAddr, payload: &[u8]) -> Vec<u8> {
    let mut out = vec![RSV, RSV, 0];
    encode_address(&mut out, source);
    out.extend_from_slice(payload);
    out
}

/// Perform the client side of a no-auth handshake against an upstream proxy and
/// issue a CONNECT for `target`.
pub async fn client_connect<S>(stream: &mut S, target: &TargetAddr) -> Result<()>
where
    S: AsyncRead + AsyncWrite + Unpin,
{
    stream.write_all(&[VERSION, 0x01, METHOD_NO_AUTH]).await?;
    let mut selection = [0u8; 2];
    stream.read_exact(&mut selection).await?;
    if selection[0] != VERSION || selection[1] != METHOD_NO_AUTH {
        bail!("upstream rejected no-auth handshake: {:?}", selection);
    }

    let mut request = vec![VERSION, CMD_CONNECT, RSV];
    encode_address(&mut request, target);
    stream.write_all(&request).await?;

    let mut reply_head = [0u8; 4];
    stream.read_exact(&mut reply_head).await?;
    if reply_head[0] != VERSION {
        bail!("upstream reply has bad version: {}", reply_head[0]);
    }
    if reply_head[1] != REP_SUCCEEDED {
        bail!("upstream CONNECT failed with reply code {}", reply_head[1]);
    }
    // Consume and discard the bound address echoed by the upstream.
    let _ = read_address(stream, reply_head[3]).await?;
    Ok(())
}

/// Read a SOCKS5 address (`atyp` already consumed by the caller) and its
/// trailing big-endian port, returning the [`TargetAddr`]. Shared by the
/// inbound request parser, the upstream client, and the Trojan UDP packet
/// codec, which all use the SOCKS5 address layout.
pub(crate) async fn read_address<S>(stream: &mut S, atyp: u8) -> Result<TargetAddr>
where
    S: AsyncRead + Unpin,
{
    match atyp {
        ATYP_IPV4 => {
            let mut octets = [0u8; 4];
            stream.read_exact(&mut octets).await?;
            let port = read_port(stream).await?;
            Ok(TargetAddr::Ip(SocketAddr::new(
                IpAddr::V4(Ipv4Addr::from(octets)),
                port,
            )))
        }
        ATYP_IPV6 => {
            let mut octets = [0u8; 16];
            stream.read_exact(&mut octets).await?;
            let port = read_port(stream).await?;
            Ok(TargetAddr::Ip(SocketAddr::new(
                IpAddr::V6(Ipv6Addr::from(octets)),
                port,
            )))
        }
        ATYP_DOMAIN => {
            let mut len = [0u8; 1];
            stream.read_exact(&mut len).await?;
            let mut host = vec![0u8; len[0] as usize];
            stream.read_exact(&mut host).await?;
            let host = String::from_utf8(host).map_err(|_| anyhow!("domain is not valid UTF-8"))?;
            let port = read_port(stream).await?;
            Ok(TargetAddr::Domain(host, port))
        }
        other => bail!("unsupported address type: {other}"),
    }
}

async fn read_port<S>(stream: &mut S) -> Result<u16>
where
    S: AsyncRead + Unpin,
{
    let mut port = [0u8; 2];
    stream.read_exact(&mut port).await?;
    Ok(u16::from_be_bytes(port))
}

/// Append the SOCKS5 address encoding of `target` (`atyp`, address, then a
/// big-endian port) to `buf`. Shared with the Trojan UDP packet codec.
pub(crate) fn encode_address(buf: &mut Vec<u8>, target: &TargetAddr) {
    match target {
        TargetAddr::Ip(SocketAddr::V4(addr)) => {
            buf.push(ATYP_IPV4);
            buf.extend_from_slice(&addr.ip().octets());
            buf.extend_from_slice(&addr.port().to_be_bytes());
        }
        TargetAddr::Ip(SocketAddr::V6(addr)) => {
            buf.push(ATYP_IPV6);
            buf.extend_from_slice(&addr.ip().octets());
            buf.extend_from_slice(&addr.port().to_be_bytes());
        }
        TargetAddr::Domain(host, port) => {
            buf.push(ATYP_DOMAIN);
            buf.push(host.len() as u8);
            buf.extend_from_slice(host.as_bytes());
            buf.extend_from_slice(&port.to_be_bytes());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn udp_datagram_roundtrip_ipv4() {
        let target = TargetAddr::Ip(SocketAddr::new(IpAddr::V4(Ipv4Addr::new(8, 8, 8, 8)), 53));
        let datagram = encode_udp_datagram(&target, b"query");
        // RSV RSV FRAG=0, then the address, then the payload.
        assert_eq!(&datagram[..3], &[RSV, RSV, 0]);
        let (parsed, offset) = parse_udp_datagram(&datagram).unwrap();
        assert_eq!(parsed, target);
        assert_eq!(&datagram[offset..], b"query");
    }

    #[test]
    fn udp_datagram_roundtrip_domain() {
        let target = TargetAddr::Domain("example.com".to_string(), 443);
        let datagram = encode_udp_datagram(&target, b"hello");
        let (parsed, offset) = parse_udp_datagram(&datagram).unwrap();
        assert_eq!(parsed, target);
        assert_eq!(&datagram[offset..], b"hello");
    }

    #[test]
    fn udp_datagram_rejects_fragments() {
        // Same as a valid ipv4 datagram but with FRAG set.
        let mut datagram = encode_udp_datagram(
            &TargetAddr::Ip(SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 80)),
            b"x",
        );
        datagram[2] = 1;
        assert!(parse_udp_datagram(&datagram).is_err());
    }

    #[test]
    fn udp_datagram_rejects_truncated() {
        assert!(parse_udp_datagram(&[RSV, RSV, 0]).is_err());
        // ATYP ipv4 but not enough address bytes.
        assert!(parse_udp_datagram(&[RSV, RSV, 0, ATYP_IPV4, 1, 2]).is_err());
    }

    #[test]
    fn reply_with_addr_encodes_bound_socket() {
        // Build the reply by hand against a known socket to lock the layout.
        let mut buf = vec![VERSION, REP_SUCCEEDED, RSV];
        encode_address(
            &mut buf,
            &TargetAddr::Ip(SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 0x1234)),
        );
        assert_eq!(
            buf,
            vec![VERSION, REP_SUCCEEDED, RSV, ATYP_IPV4, 127, 0, 0, 1, 0x12, 0x34]
        );
    }
}
