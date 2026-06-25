//! Proxy schema contract layer.
//!
//! This module deserializes the clash-style `proxies:` entries that the
//! application's control plane already generates. The frontend type union
//! `IProxyConfig` (see `src/types/global.d.ts`) is the source of truth for the
//! schema; the backend forwards those entries verbatim. learn-gripe therefore
//! must accept *every* shape the frontend can emit without ever failing to
//! parse — that is what "compatible with all" means here.
//!
//! Design rules enforced by this layer:
//! - Every protocol `type` in the frontend union maps to a [`ProxyType`]
//!   variant (lock-step is guarded by `tests/proxy-type-matrix.test.mjs`).
//! - An unknown / future `type` deserializes to [`ProxyType::Unknown`] instead
//!   of erroring, so a frontend that ships a new protocol before the kernel
//!   does never breaks config loading.
//! - Unknown fields are ignored (no `deny_unknown_fields`), so kernel-side or
//!   newer schema fields are tolerated.
//! - Transports (tcp/ws/http/h2/grpc/**xhttp**) and their option blocks are
//!   typed, including XHTTP, so protocol work in later phases reads typed data.
//!
//! Whether a parsed proxy can actually carry traffic *today* is a separate
//! question from whether it parses: see [`ProxyEntry::support`].

use std::collections::BTreeMap;

use anyhow::{Result, anyhow, bail};
use serde::Deserialize;

/// One entry of the clash `proxies:` array.
///
/// `name` and the protocol discriminant (`type`) are read directly; every other
/// field lives in the flattened [`ProxyOptions`] superset.
#[derive(Debug, Clone, Deserialize)]
pub struct ProxyEntry {
    pub name: String,
    #[serde(rename = "type")]
    pub kind: ProxyType,
    #[serde(flatten)]
    pub options: ProxyOptions,
}

impl ProxyEntry {
    /// Classify whether learn-gripe can route traffic through this proxy today.
    ///
    /// Parsing always succeeds for known protocols; this is the orthogonal
    /// "is the data plane implemented yet" axis. Unimplemented protocols are
    /// reported as [`ProtocolSupport::Unsupported`] rather than rejected at
    /// parse time.
    pub fn support(&self) -> ProtocolSupport {
        match self.kind {
            // Wired into an `OutboundMode` and reachable via `OutboundMode::from_proxy`.
            ProxyType::Direct
            | ProxyType::Reject
            | ProxyType::Socks5
            | ProxyType::Shadowsocks
            | ProxyType::Trojan
            | ProxyType::Vmess
            | ProxyType::Vless => ProtocolSupport::Implemented,
            // Parsed and type-checked, but no outbound data plane yet.
            _ => ProtocolSupport::Unsupported,
        }
    }
}

/// Routing capability of a parsed proxy in the current build.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProtocolSupport {
    /// learn-gripe can carry traffic through this proxy now.
    Implemented,
    /// The proxy parses and is type-checked, but no outbound is wired yet.
    Unsupported,
}

/// Every proxy `type` the frontend `IProxyConfig` union can emit.
///
/// Kept in lock-step with `src/types/global.d.ts` by
/// `tests/proxy-type-matrix.test.mjs`. [`ProxyType::Unknown`] is a forward
/// compatibility catch-all and must NOT be added to the frontend union.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub enum ProxyType {
    #[serde(rename = "ss")]
    Shadowsocks,
    #[serde(rename = "ssr")]
    ShadowsocksR,
    #[serde(rename = "direct")]
    Direct,
    #[serde(rename = "dns")]
    Dns,
    #[serde(rename = "snell")]
    Snell,
    #[serde(rename = "http")]
    Http,
    #[serde(rename = "trojan")]
    Trojan,
    #[serde(rename = "anytls")]
    AnyTls,
    #[serde(rename = "hysteria")]
    Hysteria,
    #[serde(rename = "hysteria2")]
    Hysteria2,
    #[serde(rename = "tuic")]
    Tuic,
    #[serde(rename = "wireguard")]
    WireGuard,
    #[serde(rename = "ssh")]
    Ssh,
    #[serde(rename = "socks5")]
    Socks5,
    #[serde(rename = "masque")]
    Masque,
    #[serde(rename = "gost-relay")]
    GostRelay,
    #[serde(rename = "trusttunnel")]
    TrustTunnel,
    #[serde(rename = "openvpn")]
    OpenVpn,
    #[serde(rename = "tailscale")]
    Tailscale,
    #[serde(rename = "reject")]
    Reject,
    #[serde(rename = "vmess")]
    Vmess,
    #[serde(rename = "vless")]
    Vless,
    #[serde(rename = "mieru")]
    Mieru,
    #[serde(rename = "sudoku")]
    Sudoku,
    /// Forward-compatibility catch-all for a `type` this build does not know.
    #[serde(other)]
    Unknown,
}

/// Transport carried under a protocol (`network` field).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Network {
    Tcp,
    Ws,
    Http,
    H2,
    Grpc,
    Xhttp,
}

/// Superset of all proxy fields across the `IProxyConfig` union.
///
/// Every field is optional so any single protocol's subset parses cleanly.
/// Protocol-specific readers in later phases pick the fields they need.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct ProxyOptions {
    // Dial / base options (IProxyBaseConfig).
    pub server: Option<String>,
    pub port: Option<u16>,
    pub tfo: Option<bool>,
    pub mptcp: Option<bool>,
    #[serde(rename = "interface-name")]
    pub interface_name: Option<String>,
    #[serde(rename = "routing-mark")]
    pub routing_mark: Option<u32>,
    #[serde(rename = "ip-version")]
    pub ip_version: Option<String>,
    #[serde(rename = "dialer-proxy")]
    pub dialer_proxy: Option<String>,
    pub udp: Option<bool>,

    // Credentials / identity shared across many protocols.
    pub username: Option<String>,
    pub password: Option<String>,
    pub uuid: Option<String>,
    pub token: Option<String>,
    pub psk: Option<String>,
    pub key: Option<String>,
    pub cipher: Option<String>,
    pub flow: Option<String>,
    pub encryption: Option<String>,
    #[serde(rename = "alterId")]
    pub alter_id: Option<u32>,

    // TLS / security.
    pub tls: Option<bool>,
    pub sni: Option<String>,
    pub servername: Option<String>,
    pub alpn: Option<Vec<String>>,
    #[serde(rename = "skip-cert-verify")]
    pub skip_cert_verify: Option<bool>,
    pub fingerprint: Option<String>,
    #[serde(rename = "client-fingerprint")]
    pub client_fingerprint: Option<String>,
    pub certificate: Option<String>,
    #[serde(rename = "private-key")]
    pub private_key: Option<String>,

    // Transport selection + typed option blocks.
    pub network: Option<Network>,
    #[serde(rename = "ws-opts")]
    pub ws_opts: Option<WsOpts>,
    #[serde(rename = "http-opts")]
    pub http_opts: Option<HttpOpts>,
    #[serde(rename = "h2-opts")]
    pub h2_opts: Option<H2Opts>,
    #[serde(rename = "grpc-opts")]
    pub grpc_opts: Option<GrpcOpts>,
    /// XHTTP transport options (`network: xhttp`).
    #[serde(rename = "xhttp-opts")]
    pub xhttp_opts: Option<XHttpOpts>,
    #[serde(rename = "reality-opts")]
    pub reality_opts: Option<RealityOpts>,
    #[serde(rename = "ech-opts")]
    pub ech_opts: Option<EchOpts>,
    #[serde(rename = "anti-dpi-opts")]
    pub anti_dpi_opts: Option<AntiDpiOpts>,
    pub smux: Option<bool>,

    // Shadowsocks plugin transport.
    pub plugin: Option<String>,
    #[serde(rename = "plugin-opts")]
    pub plugin_opts: Option<PluginOpts>,

    // Packet-encoding knobs shared by vmess/vless.
    #[serde(rename = "packet-addr")]
    pub packet_addr: Option<bool>,
    pub xudp: Option<bool>,
    #[serde(rename = "packet-encoding")]
    pub packet_encoding: Option<String>,

    // QUIC family (hysteria/hysteria2/tuic/masque) commonly-used knobs.
    pub ports: Option<String>,
    pub up: Option<String>,
    pub down: Option<String>,
    pub obfs: Option<String>,
    #[serde(rename = "obfs-password")]
    pub obfs_password: Option<String>,
    pub cwnd: Option<u32>,
}

/// WebSocket transport options (`ws-opts`).
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct WsOpts {
    pub path: Option<String>,
    pub headers: Option<BTreeMap<String, String>>,
    #[serde(rename = "max-early-data")]
    pub max_early_data: Option<u32>,
    #[serde(rename = "early-data-header-name")]
    pub early_data_header_name: Option<String>,
    #[serde(rename = "v2ray-http-upgrade")]
    pub v2ray_http_upgrade: Option<bool>,
    #[serde(rename = "v2ray-http-upgrade-fast-open")]
    pub v2ray_http_upgrade_fast_open: Option<bool>,
}

/// HTTP transport options (`http-opts`).
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct HttpOpts {
    pub method: Option<String>,
    pub path: Option<Vec<String>>,
    pub headers: Option<BTreeMap<String, Vec<String>>>,
}

/// HTTP/2 transport options (`h2-opts`).
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct H2Opts {
    pub path: Option<String>,
    pub host: Option<String>,
}

/// gRPC transport options (`grpc-opts`).
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct GrpcOpts {
    #[serde(rename = "grpc-service-name")]
    pub grpc_service_name: Option<String>,
}

/// XHTTP transport options (`xhttp-opts`).
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct XHttpOpts {
    pub path: Option<String>,
    pub host: Option<String>,
    pub mode: Option<String>,
    pub headers: Option<BTreeMap<String, String>>,
    #[serde(rename = "no-grpc-header")]
    pub no_grpc_header: Option<bool>,
}

/// REALITY options (`reality-opts`).
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct RealityOpts {
    #[serde(rename = "public-key")]
    pub public_key: Option<String>,
    #[serde(rename = "short-id")]
    pub short_id: Option<String>,
}

/// Encrypted Client Hello options (`ech-opts`).
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct EchOpts {
    pub enable: Option<bool>,
    pub config: Option<String>,
    #[serde(rename = "query-server-name")]
    pub query_server_name: Option<String>,
}

/// Anti-DPI options (`anti-dpi-opts`).
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct AntiDpiOpts {
    pub enabled: Option<bool>,
    #[serde(rename = "padding-mode")]
    pub padding_mode: Option<String>,
    #[serde(rename = "min-padding")]
    pub min_padding: Option<u32>,
    #[serde(rename = "max-padding")]
    pub max_padding: Option<u32>,
    #[serde(rename = "jitter-ms")]
    pub jitter_ms: Option<u32>,
    #[serde(rename = "burst-before")]
    pub burst_before: Option<u32>,
    #[serde(rename = "dummy-traffic")]
    pub dummy_traffic: Option<bool>,
}

/// Shadowsocks plugin options (`plugin-opts`).
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct PluginOpts {
    pub mode: Option<String>,
    pub host: Option<String>,
    pub password: Option<String>,
    pub path: Option<String>,
    pub tls: Option<String>,
    pub fingerprint: Option<String>,
    pub headers: Option<BTreeMap<String, String>>,
    #[serde(rename = "skip-cert-verify")]
    pub skip_cert_verify: Option<bool>,
    pub version: Option<u32>,
    pub mux: Option<bool>,
}

/// Parse a canonical hyphenated UUID (`xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx`)
/// into its 16 raw bytes. Hyphens are optional; any 32 hex digits are accepted.
///
/// Shared by the VLESS and VMess outbounds, both of which key their handshake
/// off the same 16-byte user id.
pub(crate) fn parse_uuid(value: &str) -> Result<[u8; 16]> {
    let hex: String = value.chars().filter(|c| *c != '-').collect();
    if hex.len() != 32 {
        bail!("uuid must be 32 hex digits, got {value:?}");
    }
    let mut out = [0u8; 16];
    for (i, byte) in out.iter_mut().enumerate() {
        let pair = &hex[i * 2..i * 2 + 2];
        *byte = u8::from_str_radix(pair, 16).map_err(|_| anyhow!("invalid uuid hex {pair:?}"))?;
    }
    Ok(out)
}
