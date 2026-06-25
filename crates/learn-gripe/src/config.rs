use std::net::{Ipv4Addr, SocketAddr};

use crate::router::Router;
use crate::trojan::TrojanOutboundConfig;
use crate::vless::VlessOutboundConfig;
use crate::vmess::VmessOutboundConfig;

/// Runtime configuration for the learn-gripe kernel MVP.
#[derive(Debug, Clone)]
pub struct GripeConfig {
    /// Local address the SOCKS5 inbound listens on.
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
    /// Select the outbound per connection from a rule list.
    Routed(Box<Router>),
}

impl Default for GripeConfig {
    fn default() -> Self {
        Self {
            socks_listen: SocketAddr::from((Ipv4Addr::LOCALHOST, 7890)),
            outbound: OutboundMode::Direct,
        }
    }
}
