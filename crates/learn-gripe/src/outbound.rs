use crate::address::TargetAddr;
use crate::config::OutboundMode;
use crate::socks5;
use crate::trojan;
use crate::vless;
use crate::vmess;
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
            OutboundMode::Routed(router) => connect(router.select(target), target).await,
        }
    })
}

/// Whether `mode` can serve a SOCKS5 `UDP ASSOCIATE`. Only `Direct` (and
/// `Routed`, which may resolve to `Direct` per datagram) carry UDP today; the
/// proxy-tunnelled UDP framings are a follow-up, so pure proxy outbounds make
/// the inbound refuse the association up front.
pub fn supports_udp_associate(mode: &OutboundMode) -> bool {
    matches!(mode, OutboundMode::Direct | OutboundMode::Routed(_))
}

/// Decide whether a UDP datagram to `target` egresses directly under `mode`.
/// `Routed` is resolved per target so a rule can still send some destinations
/// `DIRECT` and refuse (drop) the rest.
pub fn udp_egress_is_direct(mode: &OutboundMode, target: &TargetAddr) -> bool {
    match mode {
        OutboundMode::Direct => true,
        OutboundMode::Routed(router) => udp_egress_is_direct(router.select(target), target),
        _ => false,
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
    fn only_direct_and_routed_accept_udp_associate() {
        assert!(supports_udp_associate(&OutboundMode::Direct));
        assert!(!supports_udp_associate(&OutboundMode::Reject));
        assert!(!supports_udp_associate(&OutboundMode::Socks5Upstream {
            addr: "127.0.0.1:1080".parse().unwrap(),
        }));
    }

    #[test]
    fn routed_udp_egress_is_direct_per_target() {
        let mut outbounds = HashMap::new();
        outbounds.insert("blocked".to_string(), OutboundMode::Reject);
        let rules = vec![Rule {
            matcher: RuleMatcher::IpCidr(crate::router::IpCidr::parse("10.0.0.0/8").unwrap()),
            outbound: "blocked".to_string(),
        }];
        let router = Router::new(outbounds, rules, "DIRECT").unwrap();
        let routed = OutboundMode::Routed(Box::new(router));

        // Matches the reject rule -> not direct; falls through -> DIRECT.
        assert!(!udp_egress_is_direct(&routed, &ip_target(10, 1, 2, 3)));
        assert!(udp_egress_is_direct(&routed, &ip_target(8, 8, 8, 8)));
        // A bare Reject mode never egresses directly.
        assert!(!udp_egress_is_direct(&OutboundMode::Reject, &ip_target(8, 8, 8, 8)));
    }
}
