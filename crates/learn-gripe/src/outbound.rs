use crate::address::TargetAddr;
use crate::config::OutboundMode;
use crate::conntrack::ConnNetwork;
use crate::shadowsocks::{self, ShadowsocksOutboundConfig};
use crate::socks5;
use crate::trojan::{self, TrojanOutboundConfig};
use crate::vless::{self, VlessOutboundConfig};
use crate::vmess::{self, VmessOutboundConfig};
use anyhow::{Context, Result, bail};
use std::future::Future;
use std::pin::Pin;
use tokio::io::{AsyncRead, AsyncWrite};
use tokio::net::TcpStream;

/// A relay-ready outbound stream. Different outbounds wrap the underlying
/// socket differently (raw TCP, TLS, protocol framing), so the data plane
/// works against this boxed trait object.
pub trait AsyncStream: AsyncRead + AsyncWrite + Unpin + Send {}
impl<T: AsyncRead + AsyncWrite + Unpin + Send> AsyncStream for T {}

/// Boxed outbound stream used by the relay loop.
pub type BoxedStream = Box<dyn AsyncStream>;

/// Establish an outbound connection to `target` according to `mode` and return
/// a stream that is ready for relaying.
///
/// Boxed future so a [`OutboundMode::Routed`] outbound can recurse into the
/// selected sub-outbound.
pub fn connect<'a>(
    mode: &'a OutboundMode,
    target: &'a TargetAddr,
) -> Pin<Box<dyn Future<Output = Result<BoxedStream>> + Send + 'a>> {
    Box::pin(async move {
        match mode {
            OutboundMode::Direct => Ok(Box::new(dial_direct(target).await?) as BoxedStream),
            OutboundMode::Reject => bail!("connection to {target} rejected by rule"),
            OutboundMode::Socks5Upstream { addr } => {
                let mut stream = TcpStream::connect(addr)
                    .await
                    .with_context(|| format!("connect upstream SOCKS5 {addr}"))?;
                socks5::client_connect(&mut stream, target)
                    .await
                    .with_context(|| format!("upstream CONNECT to {target}"))?;
                Ok(Box::new(stream) as BoxedStream)
            }
            OutboundMode::Vless(config) => vless::connect(config, target).await,
            OutboundMode::Trojan(config) => trojan::connect(config, target).await,
            OutboundMode::Vmess(config) => vmess::connect(config, target).await,
            OutboundMode::Shadowsocks(config) => shadowsocks::connect(config, target).await,
            OutboundMode::Routed(router) => connect(router.select_network(target, ConnNetwork::Tcp), target).await,
        }
    })
}

/// The concrete egress a UDP datagram takes once routing is resolved. `Direct`
/// uses a plain OS UDP socket; the proxy variants tunnel each datagram through
/// the protocol's UDP framing over the (TCP/TLS/REALITY) outbound stream.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UdpEgress {
    Direct,
    Trojan(Box<TrojanOutboundConfig>),
    Vless(Box<VlessOutboundConfig>),
    Vmess(Box<VmessOutboundConfig>),
    Shadowsocks(Box<ShadowsocksOutboundConfig>),
}

/// Whether `mode` can serve a SOCKS5 `UDP ASSOCIATE`. `Direct`, the UDP-capable
/// proxy outbounds (Trojan/VLESS/VMess/Shadowsocks), and `Routed` (which resolves
/// per datagram) accept the association; `Reject` and an upstream SOCKS5 proxy
/// (which has no UDP relay path here) make the inbound refuse it up front.
pub fn supports_udp_associate(mode: &OutboundMode) -> bool {
    matches!(
        mode,
        OutboundMode::Direct
            | OutboundMode::Trojan(_)
            | OutboundMode::Vless(_)
            | OutboundMode::Vmess(_)
            | OutboundMode::Shadowsocks(_)
            | OutboundMode::Routed(_)
    )
}

/// Resolve the UDP egress for a datagram to `target` under `mode`, recursing
/// through `Routed` per target. Returns `None` for destinations that cannot
/// carry UDP (`Reject`, an upstream SOCKS5 proxy, or a rule resolving to one),
/// so the relay drops them rather than leaking traffic.
pub fn resolve_udp_egress(mode: &OutboundMode, target: &TargetAddr) -> Option<UdpEgress> {
    match mode {
        OutboundMode::Direct => Some(UdpEgress::Direct),
        OutboundMode::Trojan(config) => Some(UdpEgress::Trojan(config.clone())),
        OutboundMode::Vless(config) => Some(UdpEgress::Vless(config.clone())),
        OutboundMode::Vmess(config) => Some(UdpEgress::Vmess(config.clone())),
        OutboundMode::Shadowsocks(config) => Some(UdpEgress::Shadowsocks(config.clone())),
        OutboundMode::Routed(router) => resolve_udp_egress(router.select_network(target, ConnNetwork::Udp), target),
        // Reject blocks the datagram; an upstream SOCKS5 proxy has no UDP relay
        // path here, so its associations are refused rather than leaked.
        OutboundMode::Reject | OutboundMode::Socks5Upstream { .. } => None,
    }
}

/// Open the proxy-tunnel stream for a UDP datagram destined to `target`. Only
/// the proxy variants are valid here; `Direct` is handled by the relay with a
/// plain UDP socket and never reaches this path.
pub async fn connect_proxy_udp(egress: &UdpEgress, target: &TargetAddr) -> Result<BoxedStream> {
    match egress {
        UdpEgress::Trojan(config) => trojan::connect_udp(config, target).await,
        UdpEgress::Vless(config) => vless::connect_udp(config, target).await,
        UdpEgress::Vmess(config) => vmess::connect_udp(config, target).await,
        // Direct and Shadowsocks relay over a UDP socket, not a proxy stream.
        UdpEgress::Direct | UdpEgress::Shadowsocks(_) => bail!("egress has no proxy tunnel"),
    }
}

async fn dial_direct(target: &TargetAddr) -> Result<TcpStream> {
    match target {
        TargetAddr::Ip(addr) => TcpStream::connect(addr)
            .await
            .with_context(|| format!("direct connect {addr}")),
        TargetAddr::Domain(host, port) => TcpStream::connect((host.as_str(), *port))
            .await
            .with_context(|| format!("direct connect {host}:{port}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::router::{Router, Rule, RuleMatcher};
    use std::collections::HashMap;
    use std::net::{IpAddr, Ipv4Addr, SocketAddr};

    fn ip_target(a: u8, b: u8, c: u8, d: u8) -> TargetAddr {
        TargetAddr::Ip(SocketAddr::new(IpAddr::V4(Ipv4Addr::new(a, b, c, d)), 53))
    }

    #[test]
    fn udp_capable_outbounds_accept_associate() {
        assert!(supports_udp_associate(&OutboundMode::Direct));
        assert!(!supports_udp_associate(&OutboundMode::Reject));
        assert!(!supports_udp_associate(&OutboundMode::Socks5Upstream {
            addr: "127.0.0.1:1080".parse().unwrap(),
        }));
    }

    #[test]
    fn routed_udp_egress_resolves_per_target() {
        let mut outbounds = HashMap::new();
        outbounds.insert("blocked".to_string(), OutboundMode::Reject);
        let rules = vec![Rule {
            matcher: RuleMatcher::IpCidr(crate::router::IpCidr::parse("10.0.0.0/8").unwrap()),
            outbound: "blocked".to_string(),
        }];
        let router = Router::new(outbounds, rules, "DIRECT").unwrap();
        let routed = OutboundMode::Routed(Box::new(router));

        // Matches the reject rule -> no egress (dropped); falls through -> DIRECT.
        assert_eq!(resolve_udp_egress(&routed, &ip_target(10, 1, 2, 3)), None);
        assert_eq!(
            resolve_udp_egress(&routed, &ip_target(8, 8, 8, 8)),
            Some(UdpEgress::Direct)
        );
        // A bare Reject mode never produces an egress.
        assert_eq!(resolve_udp_egress(&OutboundMode::Reject, &ip_target(8, 8, 8, 8)), None);
    }
}
