//! learn-gripe: a self-built, pure-Rust proxy kernel for clash-verge-optimized.
//!
//! Brand name: `learn.gripe`. Crate/module identifiers use `learn-gripe` /
//! `learn_gripe` because Rust identifiers cannot contain a dot.
//!
//! The inbound is a mixed local listener that speaks both SOCKS5 (RFC 1928) and
//! HTTP proxy (`CONNECT` + plain absolute-form requests) on the same port, and
//! relays connections through a configurable outbound (direct, upstream SOCKS5,
//! or a proxy protocol). Protocol coverage (Shadowsocks / VMess / VLESS /
//! Trojan), TUN, DNS and the rule engine are layered on top of this core.

mod address;
mod config;
mod conntrack;
mod dns;
mod inbound;
mod outbound;
mod protocols;
mod proxy;
mod routing;
mod transport;
mod tun;
mod udp;

pub use address::TargetAddr;
pub use config::{GripeConfig, OutboundMode};
pub use conntrack::{ConnMeta, ConnNetwork, ConnRegistry, ConnSnapshot, ConnTableSnapshot, TrackedConn};
pub use dns::{
    DnsConfig, DnsHandle, DnsMode, DnsRecentQuery, DnsServer, DnsStats, DnsStatsSnapshot, FakeIpConfig, FakeIpPool,
    answer_query, unmap_fake_ip,
};
pub use inbound::{GripeHandle, GripeKernel};
pub use protocols::shadowsocks::{ShadowsocksCipher, ShadowsocksOutboundConfig};
pub use protocols::trojan::TrojanOutboundConfig;
pub use protocols::vless::VlessOutboundConfig;
pub use protocols::vmess::{VmessCipher, VmessOutboundConfig};
pub use proxy::{
    AntiDpiOpts, EchOpts, GrpcOpts, H2Opts, HttpOpts, Network, PluginOpts, ProtocolSupport, ProxyEntry, ProxyOptions,
    ProxyType, RealityOpts, WsOpts, XHttpOpts,
};
pub use routing::delay::measure_delay;
pub use routing::{
    DIRECT, GeoLookup, IpCidr, LogicalOp, PortRange, ProcessInfo, ProcessLookup, REJECT, Router, Rule, RuleMatcher,
    RuleSetLookup, UidRange,
};
pub use transport::grpc::GrpcTransportConfig;
pub use transport::http2::H2TransportConfig;
pub use transport::httpupgrade::HttpUpgradeTransportConfig;
pub use transport::obfuscation::{
    ObfuscationSnapshot, force_rotation as force_obfuscation_tls_rotation, reset as reset_obfuscation_stats,
    snapshot as snapshot_obfuscation_stats,
};
pub use transport::tls::{ClientFingerprint, RealityClientConfig, TlsClientConfig};
pub use transport::ws::WsTransportConfig;
pub use transport::xhttp::{XhttpMode, XhttpTransportConfig};
pub use transport::{Security, Transport};
pub use tun::{DEFAULT_MTU, serve_tun, serve_tun_device};
