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
mod outbound;
mod proxy;
mod server;
mod socks5;

pub use address::TargetAddr;
pub use config::{GripeConfig, OutboundMode};
pub use proxy::{
    AntiDpiOpts, EchOpts, GrpcOpts, H2Opts, HttpOpts, Network, PluginOpts, ProtocolSupport, ProxyEntry, ProxyOptions,
    ProxyType, RealityOpts, WsOpts, XHttpOpts,
};
pub use server::{GripeHandle, GripeKernel};
