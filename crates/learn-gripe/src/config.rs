use std::net::{Ipv4Addr, SocketAddr};

use crate::vless::VlessOutboundConfig;

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
    /// Forward through an upstream SOCKS5 proxy.
    Socks5Upstream { addr: SocketAddr },
    /// Forward through a VLESS outbound.
    Vless(Box<VlessOutboundConfig>),
}

impl Default for GripeConfig {
    fn default() -> Self {
        Self {
            socks_listen: SocketAddr::from((Ipv4Addr::LOCALHOST, 7890)),
            outbound: OutboundMode::Direct,
        }
    }
}
