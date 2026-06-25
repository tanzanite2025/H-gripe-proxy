//! SOCKS5 `UDP ASSOCIATE` relay.
//!
//! The inbound answers an associate request with a bound UDP socket; the client
//! then sends SOCKS5-wrapped datagrams to that socket. Each datagram names its
//! own destination, so a single association can talk to many remote hosts. We
//! keep one egress UDP socket per destination and a background reader per
//! socket that wraps replies back to the client.
//!
//! Per RFC 1928 the association lives exactly as long as the TCP control
//! connection that created it; when that connection closes we tear everything
//! down. Only `Direct` UDP egress is implemented here — proxy-tunnelled UDP is
//! a follow-up, so datagrams whose route is not direct are dropped.

use std::collections::HashMap;
use std::net::{Ipv4Addr, Ipv6Addr, SocketAddr};
use std::sync::Arc;

use anyhow::{Context, Result, anyhow};
use tokio::io::{AsyncRead, AsyncReadExt};
use tokio::net::UdpSocket;

use crate::address::TargetAddr;
use crate::config::OutboundMode;
use crate::outbound;
use crate::socks5;

/// Upper bound on a single UDP datagram (IPv4 total length limit).
const MAX_DATAGRAM: usize = 65_535;

/// Relay datagrams between the client and remote hosts until `control` (the TCP
/// connection that requested the association) closes.
pub async fn run_associate<C>(mut control: C, relay: UdpSocket, mode: Arc<OutboundMode>) -> Result<()>
where
    C: AsyncRead + Unpin,
{
    let relay = Arc::new(relay);
    let mut targets: HashMap<SocketAddr, Arc<UdpSocket>> = HashMap::new();
    let mut buf = vec![0u8; MAX_DATAGRAM];
    let mut control_buf = [0u8; 256];

    loop {
        tokio::select! {
            res = control.read(&mut control_buf) => {
                // EOF or error on the control connection ends the association.
                // Clients don't normally send payload here; ignore any bytes.
                match res {
                    Ok(0) | Err(_) => return Ok(()),
                    Ok(_) => {}
                }
            }
            res = relay.recv_from(&mut buf) => {
                let (n, client_addr) = match res {
                    Ok(v) => v,
                    Err(_) => continue,
                };
                if let Err(err) = forward_client_datagram(&buf[..n], client_addr, &relay, &mode, &mut targets).await {
                    log::debug!("learn-gripe udp: dropped client datagram: {err:#}");
                }
            }
        }
    }
}

/// Parse one client datagram and forward its payload to the destination via a
/// per-destination egress socket, spawning the reverse reader on first use.
async fn forward_client_datagram(
    datagram: &[u8],
    client_addr: SocketAddr,
    relay: &Arc<UdpSocket>,
    mode: &Arc<OutboundMode>,
    targets: &mut HashMap<SocketAddr, Arc<UdpSocket>>,
) -> Result<()> {
    let (target, offset) = socks5::parse_udp_datagram(datagram)?;
    if !outbound::udp_egress_is_direct(mode, &target) {
        bail_no_direct(&target)?;
    }
    let payload = &datagram[offset..];
    let dest = resolve(&target).await?;

    let socket = match targets.get(&dest) {
        Some(socket) => socket.clone(),
        None => {
            let egress = bind_egress(dest).await?;
            egress
                .connect(dest)
                .await
                .with_context(|| format!("udp connect {dest}"))?;
            let egress = Arc::new(egress);
            spawn_reverse(egress.clone(), relay.clone(), client_addr, target.clone());
            targets.insert(dest, egress.clone());
            egress
        }
    };
    socket
        .send(payload)
        .await
        .with_context(|| format!("udp send to {dest}"))?;
    Ok(())
}

fn bail_no_direct(target: &TargetAddr) -> Result<()> {
    Err(anyhow!("no direct UDP egress for {target}"))
}

/// Background task: read replies on `egress` and forward them, SOCKS5-wrapped
/// with `source` as the reported origin, to `client_addr` via `relay`.
fn spawn_reverse(egress: Arc<UdpSocket>, relay: Arc<UdpSocket>, client_addr: SocketAddr, source: TargetAddr) {
    tokio::spawn(async move {
        let mut buf = vec![0u8; MAX_DATAGRAM];
        loop {
            match egress.recv(&mut buf).await {
                Ok(n) => {
                    let packet = socks5::encode_udp_datagram(&source, &buf[..n]);
                    if relay.send_to(&packet, client_addr).await.is_err() {
                        return;
                    }
                }
                Err(_) => return,
            }
        }
    });
}

/// Bind an egress socket on the unspecified address of the destination family.
async fn bind_egress(dest: SocketAddr) -> Result<UdpSocket> {
    let bind: SocketAddr = match dest {
        SocketAddr::V4(_) => (Ipv4Addr::UNSPECIFIED, 0).into(),
        SocketAddr::V6(_) => (Ipv6Addr::UNSPECIFIED, 0).into(),
    };
    UdpSocket::bind(bind)
        .await
        .with_context(|| format!("bind udp egress for {dest}"))
}

/// Resolve a target to a concrete socket address (first DNS answer for a
/// domain).
async fn resolve(target: &TargetAddr) -> Result<SocketAddr> {
    match target {
        TargetAddr::Ip(addr) => Ok(*addr),
        TargetAddr::Domain(host, port) => tokio::net::lookup_host((host.as_str(), *port))
            .await
            .with_context(|| format!("resolve udp {host}:{port}"))?
            .next()
            .ok_or_else(|| anyhow!("no address for {host}:{port}")),
    }
}
