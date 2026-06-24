//! Minimal SOCKS5 wire helpers shared by the inbound server and the upstream
//! outbound client. Implements the no-authentication CONNECT subset of
//! RFC 1928, which is sufficient for the MVP data plane.

use crate::address::TargetAddr;
use anyhow::{Result, anyhow, bail};
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

pub const VERSION: u8 = 0x05;
pub const CMD_CONNECT: u8 = 0x01;
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

/// Read a CONNECT request and return the requested target address.
pub async fn read_connect_request<S>(stream: &mut S) -> Result<TargetAddr>
where
    S: AsyncRead + AsyncWrite + Unpin,
{
    let mut header = [0u8; 4];
    stream.read_exact(&mut header).await?;
    if header[0] != VERSION {
        bail!("unsupported SOCKS version in request: {}", header[0]);
    }
    if header[1] != CMD_CONNECT {
        write_reply(stream, REP_CMD_NOT_SUPPORTED).await?;
        bail!("unsupported SOCKS command: {}", header[1]);
    }

    let target = read_address(stream, header[3]).await?;
    Ok(target)
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

/// Perform the client side of a no-auth handshake against an upstream proxy and
/// issue a CONNECT for `target`.
pub async fn client_connect<S>(stream: &mut S, target: &TargetAddr) -> Result<()>
where
    S: AsyncRead + AsyncWrite + Unpin,
{
    stream
        .write_all(&[VERSION, 0x01, METHOD_NO_AUTH])
        .await?;
    let mut selection = [0u8; 2];
    stream.read_exact(&mut selection).await?;
    if selection[0] != VERSION || selection[1] != METHOD_NO_AUTH {
        bail!(
            "upstream rejected no-auth handshake: {:?}",
            selection
        );
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

async fn read_address<S>(stream: &mut S, atyp: u8) -> Result<TargetAddr>
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
            let host =
                String::from_utf8(host).map_err(|_| anyhow!("domain is not valid UTF-8"))?;
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

fn encode_address(buf: &mut Vec<u8>, target: &TargetAddr) {
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
