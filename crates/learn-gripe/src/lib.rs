//! learn-gripe: a self-built, pure-Rust proxy kernel for clash-verge-optimized.
//!
//! Brand name: `learn.gripe`. Crate/module identifiers use `learn-gripe` /
//! `learn_gripe` because Rust identifiers cannot contain a dot.
//!
//! This is the MVP data-plane slice: a local SOCKS5 inbound listener that
//! relays connections through a configurable outbound (direct, or via an
//! upstream SOCKS5 proxy). Protocol coverage (Shadowsocks / VMess / VLESS /
//! Trojan), TUN, DNS and the rule engine are layered on top of this core in
//! later phases.

mod address;
mod config;
mod grpc;
mod h2stream;
mod http2;
mod httpupgrade;
mod outbound;
mod proxy;
mod server;
mod socks5;
mod tls;
mod transport;
mod trojan;
mod vless;
mod ws;
mod xhttp;

pub use address::TargetAddr;
pub use config::{GripeConfig, OutboundMode};
pub use grpc::GrpcTransportConfig;
pub use http2::H2TransportConfig;
pub use httpupgrade::HttpUpgradeTransportConfig;
pub use proxy::{
    AntiDpiOpts, EchOpts, GrpcOpts, H2Opts, HttpOpts, Network, PluginOpts, ProtocolSupport, ProxyEntry, ProxyOptions,
    ProxyType, RealityOpts, WsOpts, XHttpOpts,
};
pub use server::{GripeHandle, GripeKernel};
pub use tls::{ClientFingerprint, RealityClientConfig, TlsClientConfig};
pub use transport::{Security, Transport};
pub use trojan::TrojanOutboundConfig;
pub use vless::VlessOutboundConfig;
pub use ws::WsTransportConfig;
pub use xhttp::{XhttpMode, XhttpTransportConfig};
