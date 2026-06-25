use std::fmt;
use std::net::SocketAddr;

/// A connection target requested by a client. A domain target is kept
/// unresolved so the outbound can decide how to resolve it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TargetAddr {
    Ip(SocketAddr),
    Domain(String, u16),
}

impl TargetAddr {
    pub fn port(&self) -> u16 {
        match self {
            TargetAddr::Ip(addr) => addr.port(),
            TargetAddr::Domain(_, port) => *port,
        }
    }

    /// Destination host as text: the domain, or the IP literal.
    pub fn host(&self) -> String {
        match self {
            TargetAddr::Ip(addr) => addr.ip().to_string(),
            TargetAddr::Domain(host, _) => host.clone(),
        }
    }

    /// The destination IP when the target is already an IP literal, else `None`
    /// (a domain target has not been resolved at this layer).
    pub fn ip(&self) -> Option<std::net::IpAddr> {
        match self {
            TargetAddr::Ip(addr) => Some(addr.ip()),
            TargetAddr::Domain(_, _) => None,
        }
    }
}

impl fmt::Display for TargetAddr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TargetAddr::Ip(addr) => write!(f, "{addr}"),
            TargetAddr::Domain(host, port) => write!(f, "{host}:{port}"),
        }
    }
}
