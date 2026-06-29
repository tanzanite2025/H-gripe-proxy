use crate::address::TargetAddr;
use crate::config::OutboundMode;
use crate::conntrack::ConnNetwork;
use crate::inbound::socks5;
use crate::protocols::anytls::{self, AnyTlsOutboundConfig};
use crate::protocols::gost_relay::{self, GostRelayOutboundConfig};
use crate::protocols::http::{self, HttpOutboundConfig};
use crate::protocols::hysteria::{self, HysteriaOutboundConfig};
use crate::protocols::hysteria2::{self, Hysteria2OutboundConfig};
use crate::protocols::masque::MasqueOutboundConfig;
use crate::protocols::mieru::{self, MieruOutboundConfig};
use crate::protocols::shadowsocks::{self, ShadowsocksOutboundConfig};
use crate::protocols::snell::{self, SnellOutboundConfig};
use crate::protocols::ssh::{self, SshOutboundConfig};
use crate::protocols::ssr::{self, SsrOutboundConfig};
use crate::protocols::sudoku::{self, SudokuOutboundConfig};
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

/// Boxed future returned by [`TcpOutbound::connect_tcp`].
pub type BoxConnectFuture<'a> = Pin<Box<dyn Future<Output = Result<BoxedStream>> + Send + 'a>>;

/// The per-protocol behavior of a single outbound: how to dial it, what to
/// label it, and the fixed upstream it talks to.
///
/// This is the convergence point for the proxy [`OutboundMode`] variants:
/// instead of every protocol being threaded through a separate `match` in
/// `connect` / `type_label` / `direct_dial_endpoints` /
/// `supports_global_capture`, each protocol implements this trait once and
/// [`OutboundMode::as_tcp_outbound`] is the single place that maps a variant to
/// its implementation. The non-protocol variants (`Direct` / `Reject` /
/// `Socks5Upstream` / `Routed`) are not `TcpOutbound`s and stay special-cased.
pub trait TcpOutbound: Send + Sync {
    /// Short outbound type label used for connection bookkeeping.
    fn type_label(&self) -> &'static str;

    /// The fixed upstream `(host, port)` this outbound dials directly over the
    /// host network — the endpoint a global TUN capture must route around.
    fn dial_endpoint(&self) -> (String, u16);

    /// Whether installing a global TUN default-route capture is sound for this
    /// outbound. True for the single-server proxy modes; overridden to false by
    /// the UDP-only outbounds that cannot carry the captured TCP traffic.
    fn supports_global_capture(&self) -> bool {
        true
    }

    /// Open a relay-ready TCP stream to `target`.
    fn connect_tcp<'a>(&'a self, target: &'a TargetAddr) -> BoxConnectFuture<'a>;
}

/// MASQUE CONNECT-UDP carries UDP only; there is no TCP relay path, so its
/// `connect_tcp` errors the same way the old `connect` match arm did.
async fn masque_no_tcp(_config: &MasqueOutboundConfig, target: &TargetAddr) -> Result<BoxedStream> {
    bail!("masque: CONNECT-UDP is UDP-only; no TCP relay for {target}")
}

/// Implement [`TcpOutbound`] for a protocol config that exposes `server`/`port`
/// fields and a free `connect(&Config, &TargetAddr)` entrypoint. The optional
/// `no_capture` form marks an outbound that cannot back a global TUN capture.
macro_rules! impl_tcp_outbound {
    ($cfg:ty, $label:literal, $connect:path) => {
        impl_tcp_outbound!(@inner $cfg, $label, $connect, true);
    };
    ($cfg:ty, $label:literal, $connect:path, no_capture) => {
        impl_tcp_outbound!(@inner $cfg, $label, $connect, false);
    };
    (@inner $cfg:ty, $label:literal, $connect:path, $capture:expr) => {
        impl TcpOutbound for $cfg {
            fn type_label(&self) -> &'static str {
                $label
            }
            fn dial_endpoint(&self) -> (String, u16) {
                (self.server.clone(), self.port)
            }
            fn supports_global_capture(&self) -> bool {
                $capture
            }
            fn connect_tcp<'a>(&'a self, target: &'a TargetAddr) -> BoxConnectFuture<'a> {
                Box::pin($connect(self, target))
            }
        }
    };
}

impl_tcp_outbound!(HttpOutboundConfig, "http", http::connect);
impl_tcp_outbound!(VlessOutboundConfig, "vless", vless::connect);
impl_tcp_outbound!(TrojanOutboundConfig, "trojan", trojan::connect);
impl_tcp_outbound!(VmessOutboundConfig, "vmess", vmess::connect);
impl_tcp_outbound!(ShadowsocksOutboundConfig, "shadowsocks", shadowsocks::connect);
impl_tcp_outbound!(TuicOutboundConfig, "tuic", tuic::connect);
impl_tcp_outbound!(HysteriaOutboundConfig, "hysteria", hysteria::connect);
impl_tcp_outbound!(Hysteria2OutboundConfig, "hysteria2", hysteria2::connect);
impl_tcp_outbound!(MasqueOutboundConfig, "masque", masque_no_tcp, no_capture);
impl_tcp_outbound!(AnyTlsOutboundConfig, "anytls", anytls::connect);
impl_tcp_outbound!(SnellOutboundConfig, "snell", snell::connect);
impl_tcp_outbound!(SshOutboundConfig, "ssh", ssh::connect);
impl_tcp_outbound!(GostRelayOutboundConfig, "gost-relay", gost_relay::connect);
impl_tcp_outbound!(MieruOutboundConfig, "mieru", mieru::connect);
impl_tcp_outbound!(SsrOutboundConfig, "ssr", ssr::connect);
impl_tcp_outbound!(SudokuOutboundConfig, "sudoku", sudoku::connect);
impl_tcp_outbound!(WireGuardOutboundConfig, "wireguard", wireguard::connect);

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
            OutboundMode::Routed(router) => {
                connect(router.select_conn(target, ConnNetwork::Tcp, source), target, source).await
            }
            // Every protocol variant is a `TcpOutbound`; dispatch through the
            // single mapping in `OutboundMode::as_tcp_outbound`.
            proxy => match proxy.as_tcp_outbound() {
                Some(outbound) => outbound.connect_tcp(target).await,
                None => bail!("connection to {target}: unsupported outbound mode"),
            },
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

/// Whether `mode` can serve a SOCKS5 `UDP ASSOCIATE`. `Routed` resolves per
/// datagram so it accepts the association up front; every other mode is
/// UDP-capable exactly when it yields a [`UdpEgress`] (see [`udp_egress_for`]),
/// so the list of UDP-capable protocols lives in a single place and cannot
/// drift from what [`resolve_udp_egress`] actually builds. `Reject` and an
/// upstream SOCKS5/HTTP proxy (which has no UDP relay path here) make the
/// inbound refuse the association.
pub fn supports_udp_associate(mode: &OutboundMode) -> bool {
    matches!(mode, OutboundMode::Routed(_)) || udp_egress_for(mode).is_some()
}

/// The UDP egress a single non-`Routed` outbound provides, if it can carry UDP.
///
/// This is the single source of truth for UDP capability: [`resolve_udp_egress`]
/// (after resolving `Routed`) and [`supports_udp_associate`] both derive from
/// it, so the set of UDP-capable protocols is enumerated exactly once. The match
/// is exhaustive (no wildcard) so a new `OutboundMode` variant must declare its
/// UDP behavior here.
fn udp_egress_for(mode: &OutboundMode) -> Option<UdpEgress> {
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
        // `Routed` is resolved per datagram by `resolve_udp_egress` before it
        // reaches here; it has no egress of its own.
        OutboundMode::Routed(_) => None,
        // No UDP relay path: `Reject` blocks the datagram; an upstream
        // SOCKS5/HTTP proxy has none here; SSH / GOST relay / mieru / sudoku are
        // TCP-only; Hysteria v1 here carries TCP only (no UDP relay yet). Their
        // associations are refused rather than leaked.
        OutboundMode::Reject
        | OutboundMode::Socks5Upstream { .. }
        | OutboundMode::Http(_)
        | OutboundMode::Ssh(_)
        | OutboundMode::GostRelay(_)
        | OutboundMode::Mieru(_)
        | OutboundMode::Sudoku(_)
        | OutboundMode::Hysteria(_) => None,
    }
}

/// Resolve the UDP egress for a datagram to `target` under `mode`, recursing
/// through `Routed` per target. `source` is the inbound client's address (when
/// known), used so `Routed` can evaluate `SRC-PORT` rules; pass `None` when
/// unknown. Returns `None` for destinations that cannot carry UDP (`Reject`, an
/// upstream SOCKS5 proxy, or a rule resolving to one), so the relay drops them
/// rather than leaking traffic.
pub fn resolve_udp_egress(mode: &OutboundMode, target: &TargetAddr, source: Option<SocketAddr>) -> Option<UdpEgress> {
    match mode {
        OutboundMode::Routed(router) => {
            resolve_udp_egress(router.select_conn(target, ConnNetwork::Udp, source), target, source)
        }
        // Egress selection for every concrete outbound is target-independent.
        other => udp_egress_for(other),
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

    /// `supports_udp_associate` must stay derived from the same source of truth
    /// as `resolve_udp_egress`: for any non-`Routed` mode it is true exactly when
    /// an egress resolves. This locks the two functions together so they cannot
    /// drift back into separately maintained per-protocol lists.
    #[test]
    fn supports_udp_associate_tracks_resolve_egress() {
        let target = ip_target(8, 8, 8, 8);
        let socks5 = OutboundMode::Socks5Upstream {
            addr: "127.0.0.1:1080".parse().unwrap(),
        };
        for mode in [OutboundMode::Direct, OutboundMode::Reject, socks5] {
            let resolves = resolve_udp_egress(&mode, &target, None).is_some();
            assert_eq!(
                supports_udp_associate(&mode),
                resolves,
                "supports_udp_associate/resolve_udp_egress drift for {mode:?}"
            );
        }

        // `Routed` accepts the association up front even though a given target
        // may resolve to a non-UDP egress (dropped per datagram).
        let mut outbounds = HashMap::new();
        outbounds.insert("blocked".to_string(), OutboundMode::Reject);
        let router = Router::new(outbounds, Vec::new(), "blocked").unwrap();
        let routed = OutboundMode::Routed(Box::new(router));
        assert!(supports_udp_associate(&routed));
        assert_eq!(resolve_udp_egress(&routed, &target, None), None);
    }
}
