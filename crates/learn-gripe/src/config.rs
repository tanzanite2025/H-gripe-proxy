use std::net::{IpAddr, Ipv4Addr, SocketAddr};

use anyhow::{Context, Result, bail};

use crate::proxy::{ProxyEntry, ProxyType};
use crate::router::Router;
use crate::shadowsocks::ShadowsocksOutboundConfig;
use crate::trojan::TrojanOutboundConfig;
use crate::vless::VlessOutboundConfig;
use crate::vmess::VmessOutboundConfig;

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
    /// Forward through a VLESS outbound.
    Vless(Box<VlessOutboundConfig>),
    /// Forward through a Trojan outbound.
    Trojan(Box<TrojanOutboundConfig>),
    /// Forward through a VMess outbound.
    Vmess(Box<VmessOutboundConfig>),
    /// Forward through a Shadowsocks (AEAD) outbound.
    Shadowsocks(Box<ShadowsocksOutboundConfig>),
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
    /// on. Protocols without a data plane yet (Hysteria, TUIC, …) and the
    /// `select`/`url-test`/… proxy *groups* are reported as errors rather than
    /// silently mis-routed.
    pub fn from_proxy(entry: &ProxyEntry) -> Result<Self> {
        match entry.kind {
            ProxyType::Direct => Ok(OutboundMode::Direct),
            ProxyType::Reject => Ok(OutboundMode::Reject),
            ProxyType::Socks5 => Ok(OutboundMode::Socks5Upstream {
                addr: socks5_upstream_addr(entry)?,
            }),
            ProxyType::Shadowsocks => Ok(OutboundMode::Shadowsocks(Box::new(
                ShadowsocksOutboundConfig::from_proxy(entry)?,
            ))),
            ProxyType::Trojan => Ok(OutboundMode::Trojan(Box::new(TrojanOutboundConfig::from_proxy(entry)?))),
            ProxyType::Vmess => Ok(OutboundMode::Vmess(Box::new(VmessOutboundConfig::from_proxy(entry)?))),
            ProxyType::Vless => Ok(OutboundMode::Vless(Box::new(VlessOutboundConfig::from_proxy(entry)?))),
            other => bail!("proxy type {other:?} has no learn-gripe outbound yet"),
        }
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
    fn unimplemented_protocol_is_rejected() {
        let err = OutboundMode::from_proxy(&entry("name: h\ntype: hysteria2\nserver: a\nport: 1\n")).unwrap_err();
        assert!(err.to_string().contains("no learn-gripe outbound"), "{err}");
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
