use std::net::{IpAddr, Ipv4Addr, SocketAddr};

use anyhow::{Context, Result, bail};

use crate::config::outbound_opts::{ProxyEntry, ProxyType};
use crate::protocols::anytls::AnyTlsOutboundConfig;
use crate::protocols::gost_relay::GostRelayOutboundConfig;
use crate::protocols::http::HttpOutboundConfig;
use crate::protocols::hysteria::HysteriaOutboundConfig;
use crate::protocols::hysteria2::Hysteria2OutboundConfig;
use crate::protocols::mieru::MieruOutboundConfig;
use crate::protocols::shadowsocks::ShadowsocksOutboundConfig;
use crate::protocols::snell::SnellOutboundConfig;
use crate::protocols::ssh::SshOutboundConfig;
use crate::protocols::ssr::SsrOutboundConfig;
use crate::protocols::trojan::TrojanOutboundConfig;
use crate::protocols::tuic::TuicOutboundConfig;
use crate::protocols::vless::VlessOutboundConfig;
use crate::protocols::vmess::VmessOutboundConfig;
use crate::protocols::wireguard::WireGuardOutboundConfig;
use crate::routing::Router;

pub mod outbound_opts;

/// Runtime configuration for the learn-gripe kernel MVP.
#[derive(Debug, Clone)]
pub struct GripeConfig {
    /// Local address the mixed inbound listens on (SOCKS5 + HTTP proxy on one
    /// port, dispatched by peeking the first byte).
    pub socks_listen: SocketAddr,
    /// How accepted connections are forwarded.
    pub outbound: OutboundMode,
}

/// Outbound strategy for the MVP data plane.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OutboundMode {
    /// Connect straight to the requested target.
    Direct,
    /// Refuse the connection (the `REJECT` policy).
    Reject,
    /// Forward through an upstream SOCKS5 proxy.
    Socks5Upstream { addr: SocketAddr },
    /// Forward through an upstream HTTP(S) proxy (the `CONNECT` method).
    Http(Box<HttpOutboundConfig>),
    /// Forward through a VLESS outbound.
    Vless(Box<VlessOutboundConfig>),
    /// Forward through a Trojan outbound.
    Trojan(Box<TrojanOutboundConfig>),
    /// Forward through a VMess outbound.
    Vmess(Box<VmessOutboundConfig>),
    /// Forward through a Shadowsocks (AEAD) outbound.
    Shadowsocks(Box<ShadowsocksOutboundConfig>),
    /// Forward through a TUIC v5 (QUIC) outbound.
    Tuic(Box<TuicOutboundConfig>),
    /// Forward through a Hysteria v1 (QUIC) outbound.
    Hysteria(Box<HysteriaOutboundConfig>),
    /// Forward through a Hysteria2 (QUIC) outbound.
    Hysteria2(Box<Hysteria2OutboundConfig>),
    /// Forward through an AnyTLS outbound.
    AnyTls(Box<AnyTlsOutboundConfig>),
    /// Forward through a Snell outbound.
    Snell(Box<SnellOutboundConfig>),
    /// Forward through an SSH `direct-tcpip` tunnel.
    Ssh(Box<SshOutboundConfig>),
    /// Forward through a GOST relay (relay protocol v1) outbound.
    GostRelay(Box<GostRelayOutboundConfig>),
    /// Forward through a mieru outbound (TCP underlay, single logical stream).
    Mieru(Box<MieruOutboundConfig>),
    /// Forward through a ShadowsocksR (SSR) outbound.
    Ssr(Box<SsrOutboundConfig>),
    /// Forward through a WireGuard outbound (L3 tunnel + userspace netstack).
    WireGuard(Box<WireGuardOutboundConfig>),
    /// Select the outbound per connection from a rule list.
    Routed(Box<Router>),
}

impl OutboundMode {
    /// Build the outbound for a single selected node from its parsed clash
    /// `proxies:` entry.
    ///
    /// This is the bridge the control plane uses to turn the user's currently
    /// selected node into a concrete data-plane outbound (instead of always
    /// dialing [`OutboundMode::Direct`]). Each real protocol delegates to its
    /// own `from_proxy` parser, which rejects sub-features that are not
    /// implemented yet, so an entry either maps to an outbound that can
    /// actually carry its traffic or returns an error the caller can fall back
    /// on. Protocols without a data plane yet (MASQUE, …) and the
    /// `select`/`url-test`/… proxy *groups* are reported as errors rather than
    /// silently mis-routed. (TUIC, Hysteria v1/2, GOST relay, and mieru have a TCP data plane.)
    pub fn from_proxy(entry: &ProxyEntry) -> Result<Self> {
        match entry.kind {
            ProxyType::Direct => Ok(OutboundMode::Direct),
            ProxyType::Reject => Ok(OutboundMode::Reject),
            ProxyType::Socks5 => Ok(OutboundMode::Socks5Upstream {
                addr: socks5_upstream_addr(entry)?,
            }),
            ProxyType::Http => Ok(OutboundMode::Http(Box::new(HttpOutboundConfig::from_proxy(entry)?))),
            ProxyType::Shadowsocks => Ok(OutboundMode::Shadowsocks(Box::new(
                ShadowsocksOutboundConfig::from_proxy(entry)?,
            ))),
            ProxyType::Trojan => Ok(OutboundMode::Trojan(Box::new(TrojanOutboundConfig::from_proxy(entry)?))),
            ProxyType::Vmess => Ok(OutboundMode::Vmess(Box::new(VmessOutboundConfig::from_proxy(entry)?))),
            ProxyType::Vless => Ok(OutboundMode::Vless(Box::new(VlessOutboundConfig::from_proxy(entry)?))),
            ProxyType::Tuic => Ok(OutboundMode::Tuic(Box::new(TuicOutboundConfig::from_proxy(entry)?))),
            ProxyType::Hysteria => Ok(OutboundMode::Hysteria(Box::new(HysteriaOutboundConfig::from_proxy(
                entry,
            )?))),
            ProxyType::Hysteria2 => Ok(OutboundMode::Hysteria2(Box::new(Hysteria2OutboundConfig::from_proxy(
                entry,
            )?))),
            ProxyType::AnyTls => Ok(OutboundMode::AnyTls(Box::new(AnyTlsOutboundConfig::from_proxy(entry)?))),
            ProxyType::Snell => Ok(OutboundMode::Snell(Box::new(SnellOutboundConfig::from_proxy(entry)?))),
            ProxyType::Ssh => Ok(OutboundMode::Ssh(Box::new(SshOutboundConfig::from_proxy(entry)?))),
            ProxyType::GostRelay => Ok(OutboundMode::GostRelay(Box::new(GostRelayOutboundConfig::from_proxy(
                entry,
            )?))),
            ProxyType::Mieru => Ok(OutboundMode::Mieru(Box::new(MieruOutboundConfig::from_proxy(entry)?))),
            ProxyType::ShadowsocksR => Ok(OutboundMode::Ssr(Box::new(SsrOutboundConfig::from_proxy(entry)?))),
            ProxyType::WireGuard => Ok(OutboundMode::WireGuard(Box::new(WireGuardOutboundConfig::from_proxy(
                entry,
            )?))),
            other => bail!("proxy type {other:?} has no learn-gripe outbound yet"),
        }
    }

    /// The fixed upstream endpoints (`host`, `port`) this outbound dials
    /// directly over the host network — the set a global TUN default-route
    /// capture must route *around* (bypass) so the proxy's own traffic is not
    /// looped back into the tunnel.
    ///
    /// `Direct`/`Reject` dial no fixed upstream (`Direct` reaches arbitrary
    /// targets, so it cannot be globally captured without looping). `Routed`
    /// unions the endpoints of its named outbounds. Hosts may be domains; the
    /// caller resolves them to literal IPs before installing bypass routes.
    pub fn direct_dial_endpoints(&self) -> Vec<(String, u16)> {
        match self {
            OutboundMode::Direct | OutboundMode::Reject => Vec::new(),
            OutboundMode::Socks5Upstream { addr } => vec![(addr.ip().to_string(), addr.port())],
            OutboundMode::Http(c) => vec![(c.server.clone(), c.port)],
            OutboundMode::Vless(c) => vec![(c.server.clone(), c.port)],
            OutboundMode::Trojan(c) => vec![(c.server.clone(), c.port)],
            OutboundMode::Vmess(c) => vec![(c.server.clone(), c.port)],
            OutboundMode::Shadowsocks(c) => vec![(c.server.clone(), c.port)],
            OutboundMode::Tuic(c) => vec![(c.server.clone(), c.port)],
            OutboundMode::Hysteria(c) => vec![(c.server.clone(), c.port)],
            OutboundMode::Hysteria2(c) => vec![(c.server.clone(), c.port)],
            OutboundMode::AnyTls(c) => vec![(c.server.clone(), c.port)],
            OutboundMode::Snell(c) => vec![(c.server.clone(), c.port)],
            OutboundMode::Ssh(c) => vec![(c.server.clone(), c.port)],
            OutboundMode::GostRelay(c) => vec![(c.server.clone(), c.port)],
            OutboundMode::Mieru(c) => vec![(c.server.clone(), c.port)],
            OutboundMode::Ssr(c) => vec![(c.server.clone(), c.port)],
            OutboundMode::WireGuard(c) => vec![(c.server.clone(), c.port)],
            OutboundMode::Routed(router) => router
                .outbound_modes()
                .flat_map(OutboundMode::direct_dial_endpoints)
                .collect(),
        }
    }

    /// Short outbound type label used for connection bookkeeping (the chain
    /// entry when no named rule router selected the outbound).
    pub fn type_label(&self) -> &'static str {
        match self {
            OutboundMode::Direct => "DIRECT",
            OutboundMode::Reject => "REJECT",
            OutboundMode::Socks5Upstream { .. } => "socks5",
            OutboundMode::Http(_) => "http",
            OutboundMode::Vless(_) => "vless",
            OutboundMode::Trojan(_) => "trojan",
            OutboundMode::Vmess(_) => "vmess",
            OutboundMode::Shadowsocks(_) => "shadowsocks",
            OutboundMode::Tuic(_) => "tuic",
            OutboundMode::Hysteria(_) => "hysteria",
            OutboundMode::Hysteria2(_) => "hysteria2",
            OutboundMode::AnyTls(_) => "anytls",
            OutboundMode::Snell(_) => "snell",
            OutboundMode::Ssh(_) => "ssh",
            OutboundMode::GostRelay(_) => "gost-relay",
            OutboundMode::Mieru(_) => "mieru",
            OutboundMode::Ssr(_) => "ssr",
            OutboundMode::WireGuard(_) => "wireguard",
            OutboundMode::Routed(_) => "routed",
        }
    }

    /// Whether installing a global TUN default-route capture is sound for this
    /// outbound. It must dial a *fixed, bypassable* set of upstreams and tunnel
    /// everything else through them — true only for the single-server proxy
    /// modes. `Direct`/`Reject` would loop (arbitrary targets are dialed
    /// directly), and `Routed` may contain a `Direct` path, so both are
    /// excluded; capture stays off and the TUN serves only its on-link subnet.
    pub fn supports_global_capture(&self) -> bool {
        matches!(
            self,
            OutboundMode::Socks5Upstream { .. }
                | OutboundMode::Http(_)
                | OutboundMode::Vless(_)
                | OutboundMode::Trojan(_)
                | OutboundMode::Vmess(_)
                | OutboundMode::Shadowsocks(_)
                | OutboundMode::Tuic(_)
                | OutboundMode::Hysteria(_)
                | OutboundMode::Hysteria2(_)
                | OutboundMode::AnyTls(_)
                | OutboundMode::Snell(_)
                | OutboundMode::Ssh(_)
                | OutboundMode::GostRelay(_)
                | OutboundMode::Mieru(_)
                | OutboundMode::Ssr(_)
                | OutboundMode::WireGuard(_)
        )
    }
}

/// Resolve a `socks5` node's `server:port` into the literal [`SocketAddr`] the
/// upstream-SOCKS5 outbound dials. Hostnames are rejected (no DNS at config
/// build time) so the caller falls back rather than dialing a bad address.
fn socks5_upstream_addr(entry: &ProxyEntry) -> Result<SocketAddr> {
    let server = entry
        .options
        .server
        .as_deref()
        .filter(|s| !s.is_empty())
        .context("socks5 proxy missing server")?;
    let port = entry.options.port.context("socks5 proxy missing port")?;
    let ip: IpAddr = server
        .parse()
        .with_context(|| format!("socks5 outbound requires a literal IP server, got {server:?}"))?;
    Ok(SocketAddr::new(ip, port))
}

impl Default for GripeConfig {
    fn default() -> Self {
        Self {
            socks_listen: SocketAddr::from((Ipv4Addr::LOCALHOST, 7890)),
            outbound: OutboundMode::Direct,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(yaml: &str) -> ProxyEntry {
        serde_yaml_ng::from_str(yaml).expect("valid proxy entry")
    }

    fn mode(yaml: &str) -> OutboundMode {
        OutboundMode::from_proxy(&entry(yaml)).expect("supported proxy maps to an outbound")
    }

    #[test]
    fn direct_and_reject_map_to_their_modes() {
        assert_eq!(mode("name: d\ntype: direct\n"), OutboundMode::Direct);
        assert_eq!(mode("name: r\ntype: reject\n"), OutboundMode::Reject);
    }

    #[test]
    fn socks5_maps_to_upstream_with_literal_ip() {
        let m = mode("name: s\ntype: socks5\nserver: 10.0.0.1\nport: 1080\n");
        assert_eq!(
            m,
            OutboundMode::Socks5Upstream {
                addr: SocketAddr::from(([10, 0, 0, 1], 1080)),
            }
        );
    }

    #[test]
    fn socks5_hostname_server_is_rejected() {
        let err =
            OutboundMode::from_proxy(&entry("name: s\ntype: socks5\nserver: proxy.example\nport: 1080\n")).unwrap_err();
        assert!(err.to_string().contains("literal IP"), "{err}");
    }

    #[test]
    fn shadowsocks_entry_maps_to_shadowsocks_outbound() {
        let m = mode("name: s\ntype: ss\nserver: example.com\nport: 8388\ncipher: aes-256-gcm\npassword: secret\n");
        assert!(matches!(m, OutboundMode::Shadowsocks(_)));
    }

    #[test]
    fn trojan_entry_maps_to_trojan_outbound() {
        let m = mode("name: t\ntype: trojan\nserver: example.com\nport: 443\npassword: secret\n");
        assert!(matches!(m, OutboundMode::Trojan(_)));
    }

    #[test]
    fn anytls_entry_maps_to_anytls_outbound() {
        let m = mode("name: a\ntype: anytls\nserver: example.com\nport: 443\npassword: secret\n");
        assert!(matches!(m, OutboundMode::AnyTls(_)));
        assert_eq!(m.type_label(), "anytls");
        assert_eq!(m.direct_dial_endpoints(), vec![("example.com".to_string(), 443)]);
        assert!(m.supports_global_capture());
    }

    #[test]
    fn unimplemented_protocol_is_rejected() {
        let err = OutboundMode::from_proxy(&entry("name: m\ntype: masque\nserver: a\nport: 1\n")).unwrap_err();
        assert!(err.to_string().contains("no learn-gripe outbound"), "{err}");
    }

    #[test]
    fn endpoints_and_capture_for_each_mode() {
        // Proxy modes expose their fixed server:port and support global capture.
        let ss = mode("name: s\ntype: ss\nserver: ss.example\nport: 8388\ncipher: aes-256-gcm\npassword: secret\n");
        assert_eq!(ss.direct_dial_endpoints(), vec![("ss.example".to_string(), 8388)]);
        assert!(ss.supports_global_capture());

        let socks = mode("name: s\ntype: socks5\nserver: 10.0.0.1\nport: 1080\n");
        assert_eq!(socks.direct_dial_endpoints(), vec![("10.0.0.1".to_string(), 1080)]);
        assert!(socks.supports_global_capture());

        // Direct/Reject dial no fixed upstream and cannot be globally captured
        // (arbitrary targets would loop back into the tunnel).
        assert!(OutboundMode::Direct.direct_dial_endpoints().is_empty());
        assert!(!OutboundMode::Direct.supports_global_capture());
        assert!(!OutboundMode::Reject.supports_global_capture());
    }

    #[test]
    fn routed_unions_endpoints_but_is_not_capturable() {
        use crate::routing::Router;
        use std::collections::HashMap;

        let trojan = mode("name: t\ntype: trojan\nserver: t.example\nport: 443\npassword: secret\n");
        let ss = mode("name: s\ntype: ss\nserver: ss.example\nport: 8388\ncipher: aes-256-gcm\npassword: secret\n");
        let mut outbounds = HashMap::new();
        outbounds.insert("t".to_string(), trojan);
        outbounds.insert("s".to_string(), ss);
        let router = Router::new(outbounds, Vec::new(), "t").expect("router");
        let routed = OutboundMode::Routed(Box::new(router));

        let mut endpoints = routed.direct_dial_endpoints();
        endpoints.sort();
        assert_eq!(
            endpoints,
            vec![("ss.example".to_string(), 8388), ("t.example".to_string(), 443)]
        );
        // A router may include a Direct path, so capture stays off.
        assert!(!routed.supports_global_capture());
    }

    #[test]
    fn shadowsocks_unsupported_cipher_propagates_error() {
        // A `ss` node whose cipher is a legacy stream cipher must error (so the
        // caller falls back) rather than being mis-framed as AEAD.
        let err = OutboundMode::from_proxy(&entry(
            "name: s\ntype: ss\nserver: h\nport: 1\ncipher: aes-256-cfb\npassword: p\n",
        ))
        .unwrap_err();
        assert!(!err.to_string().is_empty());
    }
}
