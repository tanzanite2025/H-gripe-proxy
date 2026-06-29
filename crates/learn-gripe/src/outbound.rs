use crate::address::TargetAddr;
use crate::config::OutboundMode;
use crate::conntrack::ConnNetwork;
use crate::inbound::socks5;
use crate::protocols::anytls::{self, AnyTlsOutboundConfig};
use crate::protocols::gost_relay;
use crate::protocols::http;
use crate::protocols::hysteria;
use crate::protocols::hysteria2::{self, Hysteria2OutboundConfig};
use crate::protocols::masque::MasqueOutboundConfig;
use crate::protocols::mieru;
use crate::protocols::shadowsocks::{self, ShadowsocksOutboundConfig};
use crate::protocols::snell::{self, SnellOutboundConfig};
use crate::protocols::ssh;
use crate::protocols::ssr::{self, SsrOutboundConfig};
use crate::protocols::trojan::{self, TrojanOutboundConfig};
use crate::protocols::tuic::{self, TuicOutboundConfig};
use crate::protocols::vless::{self, VlessOutboundConfig};
use crate::protocols::vmess::{self, VmessOutboundConfig};
use crate::protocols::wireguard::{self, WireGuardOutboundConfig};
use anyhow::{Context, Result, bail};
use std::future::Future;
use std::net::SocketAddr;
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
/// a stream that is ready for relaying. `source` is the inbound peer's address
/// (when the embedder can supply it), used so a [`OutboundMode::Routed`]
/// outbound can evaluate `SRC-PORT` rules; pass `None` when unknown.
///
/// Boxed future so a [`OutboundMode::Routed`] outbound can recurse into the
/// selected sub-outbound.
pub fn connect<'a>(
    mode: &'a OutboundMode,
    target: &'a TargetAddr,
    source: Option<SocketAddr>,
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
            OutboundMode::Http(config) => http::connect(config, target).await,
            OutboundMode::Vless(config) => vless::connect(config, target).await,
            OutboundMode::Trojan(config) => trojan::connect(config, target).await,
            OutboundMode::Vmess(config) => vmess::connect(config, target).await,
            OutboundMode::Shadowsocks(config) => shadowsocks::connect(config, target).await,
            OutboundMode::Tuic(config) => tuic::connect(config, target).await,
            OutboundMode::Hysteria(config) => hysteria::connect(config, target).await,
            OutboundMode::Hysteria2(config) => hysteria2::connect(config, target).await,
            // MASQUE CONNECT-UDP carries UDP only; there is no TCP relay path.
            OutboundMode::Masque(_) => bail!("masque: CONNECT-UDP is UDP-only; no TCP relay for {target}"),
            OutboundMode::AnyTls(config) => anytls::connect(config, target).await,
            OutboundMode::Snell(config) => snell::connect(config, target).await,
            OutboundMode::Ssh(config) => ssh::connect(config, target).await,
            OutboundMode::GostRelay(config) => gost_relay::connect(config, target).await,
            OutboundMode::Mieru(config) => mieru::connect(config, target).await,
            OutboundMode::Ssr(config) => ssr::connect(config, target).await,
            OutboundMode::WireGuard(config) => wireguard::connect(config, target).await,
            OutboundMode::Routed(router) => {
                connect(router.select_conn(target, ConnNetwork::Tcp, source), target, source).await
            }
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
    /// Hysteria2 carries datagrams over QUIC datagram frames, not a proxy stream.
    Hysteria2(Box<Hysteria2OutboundConfig>),
    /// MASQUE carries datagrams as HTTP Datagrams over QUIC datagram frames
    /// (CONNECT-UDP), not a proxy stream.
    Masque(Box<MasqueOutboundConfig>),
    /// TUIC carries datagrams over QUIC `Packet` datagram frames.
    Tuic(Box<TuicOutboundConfig>),
    /// AnyTLS carries datagrams over a udp-over-tcp v2 session stream.
    AnyTls(Box<AnyTlsOutboundConfig>),
    /// SSR seals each datagram with a per-packet stream cipher + protocol
    /// framing over a plain UDP socket (no obfs layer for UDP).
    Ssr(Box<SsrOutboundConfig>),
    /// Snell carries datagrams over a `CommandUDP` UDP-over-TCP session (v3 +
    /// v4/v5), one transport unit per packet (a shadowaead chunk for v3, a v4
    /// frame for v4/v5).
    Snell(Box<SnellOutboundConfig>),
    /// WireGuard relays each datagram through a userspace smoltcp UDP socket
    /// whose IP packets ride the Noise tunnel (no proxy stream).
    WireGuard(Box<WireGuardOutboundConfig>),
}

/// Whether `mode` can serve a SOCKS5 `UDP ASSOCIATE`. `Direct`, the UDP-capable
/// proxy outbounds (Trojan/VLESS/VMess/Shadowsocks, the QUIC Hysteria2/TUIC
/// datagram relays, and AnyTLS over udp-over-tcp v2), and `Routed` (which
/// resolves per datagram) accept the
/// association; `Reject` and an upstream SOCKS5 proxy (which has no UDP relay
/// path here) make the inbound refuse it up front.
pub fn supports_udp_associate(mode: &OutboundMode) -> bool {
    matches!(
        mode,
        OutboundMode::Direct
            | OutboundMode::Trojan(_)
            | OutboundMode::Vless(_)
            | OutboundMode::Vmess(_)
            | OutboundMode::Shadowsocks(_)
            | OutboundMode::Tuic(_)
            | OutboundMode::Hysteria2(_)
            | OutboundMode::Masque(_)
            | OutboundMode::AnyTls(_)
            | OutboundMode::Ssr(_)
            | OutboundMode::WireGuard(_)
            | OutboundMode::Routed(_)
    ) || matches!(mode, OutboundMode::Snell(config) if config.supports_udp())
}

/// Resolve the UDP egress for a datagram to `target` under `mode`, recursing
/// through `Routed` per target. `source` is the inbound client's address (when
/// known), used so `Routed` can evaluate `SRC-PORT` rules; pass `None` when
/// unknown. Returns `None` for destinations that cannot carry UDP (`Reject`, an
/// upstream SOCKS5 proxy, or a rule resolving to one), so the relay drops them
/// rather than leaking traffic.
pub fn resolve_udp_egress(mode: &OutboundMode, target: &TargetAddr, source: Option<SocketAddr>) -> Option<UdpEgress> {
    match mode {
        OutboundMode::Direct => Some(UdpEgress::Direct),
        OutboundMode::Trojan(config) => Some(UdpEgress::Trojan(config.clone())),
        OutboundMode::Vless(config) => Some(UdpEgress::Vless(config.clone())),
        OutboundMode::Vmess(config) => Some(UdpEgress::Vmess(config.clone())),
        OutboundMode::Shadowsocks(config) => Some(UdpEgress::Shadowsocks(config.clone())),
        OutboundMode::Tuic(config) => Some(UdpEgress::Tuic(config.clone())),
        OutboundMode::Hysteria2(config) => Some(UdpEgress::Hysteria2(config.clone())),
        OutboundMode::Masque(config) => Some(UdpEgress::Masque(config.clone())),
        OutboundMode::AnyTls(config) => Some(UdpEgress::AnyTls(config.clone())),
        // Snell UDP (CommandUDP) is supported on v3 + v4/v5; v1/v2 carry TCP
        // only, so drop UDP rather than mis-frame it.
        OutboundMode::Snell(config) if config.supports_udp() => Some(UdpEgress::Snell(config.clone())),
        OutboundMode::Snell(_) => None,
        OutboundMode::Ssr(config) => Some(UdpEgress::Ssr(config.clone())),
        OutboundMode::WireGuard(config) => Some(UdpEgress::WireGuard(config.clone())),
        OutboundMode::Routed(router) => {
            resolve_udp_egress(router.select_conn(target, ConnNetwork::Udp, source), target, source)
        }
        // Reject blocks the datagram; an upstream SOCKS5/HTTP proxy has no UDP
        // relay path here, so its associations are refused rather than leaked.
        // Hysteria v1 here carries TCP only (no UDP relay yet), so its
        // associations are refused rather than leaked.
        OutboundMode::Reject
        | OutboundMode::Socks5Upstream { .. }
        | OutboundMode::Http(_)
        | OutboundMode::Ssh(_)
        | OutboundMode::GostRelay(_)
        | OutboundMode::Mieru(_)
        | OutboundMode::Hysteria(_) => None,
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
        UdpEgress::AnyTls(config) => anytls::connect_udp(config, target).await,
        // Direct/Shadowsocks/SSR relay over a UDP socket and Hysteria2/TUIC over
        // QUIC datagrams, none of which is a proxy stream.
        UdpEgress::Direct
        | UdpEgress::Shadowsocks(_)
        | UdpEgress::Ssr(_)
        | UdpEgress::Snell(_)
        | UdpEgress::Hysteria2(_)
        | UdpEgress::Masque(_)
        | UdpEgress::Tuic(_)
        | UdpEgress::WireGuard(_) => {
            bail!("egress has no proxy tunnel")
        }
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
    use crate::routing::{Router, Rule, RuleMatcher};
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
            matcher: RuleMatcher::IpCidr(crate::routing::IpCidr::parse("10.0.0.0/8").unwrap()),
            outbound: "blocked".to_string(),
        }];
        let router = Router::new(outbounds, rules, "DIRECT").unwrap();
        let routed = OutboundMode::Routed(Box::new(router));

        // Matches the reject rule -> no egress (dropped); falls through -> DIRECT.
        assert_eq!(resolve_udp_egress(&routed, &ip_target(10, 1, 2, 3), None), None);
        assert_eq!(
            resolve_udp_egress(&routed, &ip_target(8, 8, 8, 8), None),
            Some(UdpEgress::Direct)
        );
        // A bare Reject mode never produces an egress.
        assert_eq!(
            resolve_udp_egress(&OutboundMode::Reject, &ip_target(8, 8, 8, 8), None),
            None
        );
    }
}
