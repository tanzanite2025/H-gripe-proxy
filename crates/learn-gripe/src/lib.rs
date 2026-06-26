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
mod delay;
mod dns;
mod grpc;
mod h2stream;
mod http;
mod http2;
mod httpupgrade;
mod obfuscation;
mod outbound;
mod proxy;
mod router;
mod server;
mod shadowsocks;
mod socks5;
mod tls;
mod transport;
mod trojan;
mod tun;
mod udp;
mod vision;
mod vless;
mod vmess;
mod ws;
mod xhttp;

pub use address::TargetAddr;
pub use config::{GripeConfig, OutboundMode};
pub use conntrack::{ConnMeta, ConnNetwork, ConnRegistry, ConnSnapshot, ConnTableSnapshot, TrackedConn};
pub use delay::measure_delay;
pub use dns::{
    DnsConfig, DnsHandle, DnsMode, DnsRecentQuery, DnsServer, DnsStats, DnsStatsSnapshot, FakeIpConfig, FakeIpPool,
    answer_query, unmap_fake_ip,
};
pub use grpc::GrpcTransportConfig;
pub use http2::H2TransportConfig;
pub use httpupgrade::HttpUpgradeTransportConfig;
pub use obfuscation::{
    ObfuscationSnapshot, force_rotation as force_obfuscation_tls_rotation, reset as reset_obfuscation_stats,
    snapshot as snapshot_obfuscation_stats,
};
pub use proxy::{
    AntiDpiOpts, EchOpts, GrpcOpts, H2Opts, HttpOpts, Network, PluginOpts, ProtocolSupport, ProxyEntry, ProxyOptions,
    ProxyType, RealityOpts, WsOpts, XHttpOpts,
};
pub use router::{
    DIRECT, GeoLookup, IpCidr, LogicalOp, PortRange, ProcessInfo, ProcessLookup, REJECT, Router, Rule, RuleMatcher,
    RuleSetLookup, UidRange,
};
pub use server::{GripeHandle, GripeKernel};
pub use shadowsocks::{ShadowsocksCipher, ShadowsocksOutboundConfig};
pub use tls::{ClientFingerprint, RealityClientConfig, TlsClientConfig};
pub use transport::{Security, Transport};
pub use trojan::TrojanOutboundConfig;
pub use tun::{DEFAULT_MTU, serve_tun, serve_tun_device};
pub use vless::VlessOutboundConfig;
pub use vmess::{VmessCipher, VmessOutboundConfig};
pub use ws::WsTransportConfig;
pub use xhttp::{XhttpMode, XhttpTransportConfig};
