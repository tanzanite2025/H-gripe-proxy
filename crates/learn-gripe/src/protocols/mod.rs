//! Outbound proxy protocol implementations.
//!
//! Each submodule implements one proxy protocol's client side (handshake,
//! header sealing, framing) and exposes an `*OutboundConfig` plus a `connect`
//! entrypoint. `vision` is the XTLS Vision flow filter that `vless` wraps the
//! stream in when the Vision flow is negotiated.

pub mod anytls;
pub mod hysteria2;
pub mod quic_udp;
pub mod salamander;
pub mod shadowsocks;
pub mod snell;
pub mod ss_plugin;
pub mod ssr;
pub mod trojan;
pub mod tuic;
pub mod vision;
pub mod vless;
pub mod vmess;
