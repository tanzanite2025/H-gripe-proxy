//! Composable outbound dial pipeline.
//!
//! Proxy outbounds in the wild are an orthogonal product of three independent
//! layers, mirroring Xray/V2Ray (e.g. `VLESS-WS-TLS`, `VLESS-gRPC-REALITY`):
//!
//! ```text
//! protocol (VLESS / VMess / Trojan)   <- writes its own framing on top
//! ─────────────────────────────────
//! transport (tcp / ws / grpc / xhttp) <- this module
//! ─────────────────────────────────
//! security  (none / tls / reality)    <- this module
//! ─────────────────────────────────
//! raw TCP socket
//! ```
//!
//! [`establish`] dials the socket, applies the [`Security`] layer, then the
//! [`Transport`] layer, and hands back a [`BoxedStream`] of plain application
//! bytes. The protocol layer (e.g. `vless`) is the only thing that sits above
//! it, so adding a protocol never touches transport code and adding a transport
//! never touches protocol code.
//!
//! This slice implements `tcp`, `ws`, `grpc`, `xhttp`, `httpupgrade` and `h2`
//! transports over `none` / `tls` / `reality` security. Because REALITY slots
//! into the same `Security` enum, VLESS-REALITY works under every transport
//! automatically.

use anyhow::{Context, Result, anyhow, bail};
use tokio::net::TcpStream;

use crate::grpc::GrpcTransportConfig;
use crate::http2::H2TransportConfig;
use crate::httpupgrade::HttpUpgradeTransportConfig;
use crate::outbound::BoxedStream;
use crate::proxy::{Network, ProxyOptions, RealityOpts};
use crate::tls::{ClientFingerprint, RealityClientConfig, TlsClientConfig};
use crate::ws::WsTransportConfig;
use crate::xhttp::{XhttpMode, XhttpTransportConfig};

/// The security layer wrapping the raw TCP socket.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Security {
    /// Plaintext — no security wrapper.
    None,
    /// Standard TLS (rustls).
    Tls(TlsClientConfig),
    /// REALITY over TLS 1.3 (rustls `with_reality`).
    Reality(RealityClientConfig),
}

impl Security {
    /// Mutable access to the offered ALPN list, regardless of which secured
    /// variant this is. Used by protocol layers (e.g. VLESS) to force `h2`
    /// ALPN for HTTP/2-based transports without caring about TLS vs REALITY.
    pub(crate) fn alpn_mut(&mut self) -> Option<&mut Vec<String>> {
        match self {
            Security::None => None,
            Security::Tls(tls) => Some(&mut tls.alpn),
            Security::Reality(reality) => Some(&mut reality.alpn),
        }
    }
}

/// The transport layer carrying the protocol's bytes over the secured socket.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Transport {
    /// Raw stream — the protocol bytes flow directly over the secured socket.
    Tcp,
    /// WebSocket transport (`network: ws`).
    Ws(WsTransportConfig),
    /// gRPC (HTTP/2) transport (`network: grpc`).
    Grpc(GrpcTransportConfig),
    /// XHTTP (HTTP/2, stream-one) transport (`network: xhttp`).
    Xhttp(XhttpTransportConfig),
    /// HTTP Upgrade transport (`network: ws` + `v2ray-http-upgrade`).
    HttpUpgrade(HttpUpgradeTransportConfig),
    /// HTTP/2 transport (`network: h2`); always over TLS.
    H2(H2TransportConfig),
}

/// Dial `server:port`, apply `security`, then `transport`, returning a
/// relay-ready byte stream onto which a protocol layer can write its framing.
pub async fn establish(server: &str, port: u16, security: &Security, transport: &Transport) -> Result<BoxedStream> {
    let tcp = TcpStream::connect((server, port))
        .await
        .map_err(|e| anyhow::anyhow!("dial {server}:{port}: {e}"))?;

    let over_tls = matches!(security, Security::Tls(_) | Security::Reality(_));
    let secured: BoxedStream = match security {
        Security::None => Box::new(tcp),
        Security::Tls(cfg) => Box::new(crate::tls::connect(cfg, server, tcp).await?),
        Security::Reality(cfg) => Box::new(crate::tls::connect_reality(cfg, server, tcp).await?),
    };

    let transported: BoxedStream = match transport {
        Transport::Tcp => secured,
        Transport::Ws(cfg) => Box::new(crate::ws::connect(secured, server, cfg).await?),
        Transport::Grpc(cfg) => Box::new(crate::grpc::connect(secured, server, over_tls, cfg).await?),
        Transport::Xhttp(cfg) => Box::new(crate::xhttp::connect(secured, server, over_tls, cfg).await?),
        Transport::HttpUpgrade(cfg) => Box::new(crate::httpupgrade::connect(secured, server, cfg).await?),
        Transport::H2(cfg) => Box::new(crate::http2::connect(secured, server, cfg).await?),
    };

    Ok(transported)
}

/// Build the orthogonal `(Security, Transport)` pair shared by every outbound
/// protocol (VLESS, Trojan, ...) from the flattened proxy options.
///
/// The two layers are independent of the protocol framing, so this is the one
/// place that maps clash/mihomo `tls` / `reality-opts` / `network` / `*-opts`
/// fields onto the kernel's [`Security`] and [`Transport`]. `proto` only labels
/// error messages; `default_tls` is whether security defaults to TLS when `tls`
/// is unset (Trojan defaults on, VLESS off). Unimplemented sub-features are
/// rejected so traffic is never silently mis-framed.
pub(crate) fn build_layers(opts: &ProxyOptions, proto: &str, default_tls: bool) -> Result<(Security, Transport)> {
    if let Some(flow) = opts.flow.as_deref()
        && !flow.is_empty()
    {
        bail!("{proto}: flow {flow:?} not implemented yet");
    }

    let client_fingerprint = match opts.client_fingerprint.as_deref() {
        None | Some("") => None,
        Some(value) => Some(ClientFingerprint::parse(value).map_err(|e| anyhow!("{proto}: {e}"))?),
    };

    let mut security = if let Some(reality_opts) = &opts.reality_opts {
        Security::Reality(build_reality(opts, reality_opts, client_fingerprint, proto)?)
    } else if opts.tls.unwrap_or(default_tls) {
        Security::Tls(TlsClientConfig {
            server_name: opts.servername.clone().or_else(|| opts.sni.clone()),
            alpn: opts.alpn.clone().unwrap_or_default(),
            skip_cert_verify: opts.skip_cert_verify.unwrap_or(false),
        })
    } else {
        Security::None
    };

    let transport = match opts.network {
        None | Some(Network::Tcp) => Transport::Tcp,
        Some(Network::Ws) => {
            let ws = opts.ws_opts.clone().unwrap_or_default();
            let mut headers = ws.headers.unwrap_or_default();
            // The camouflage Host drives the handshake authority; keep the
            // remaining headers for the request as-is.
            let host = headers.remove("Host").or_else(|| headers.remove("host"));
            let path = ws.path.unwrap_or_default();
            // `v2ray-http-upgrade` selects the leaner HTTP-Upgrade transport
            // (raw stream after `101`), not a WebSocket-framed one.
            if ws.v2ray_http_upgrade.unwrap_or(false) {
                Transport::HttpUpgrade(HttpUpgradeTransportConfig { path, host, headers })
            } else {
                Transport::Ws(WsTransportConfig { path, host, headers })
            }
        }
        Some(Network::Grpc) => {
            let grpc = opts.grpc_opts.clone().unwrap_or_default();
            Transport::Grpc(GrpcTransportConfig {
                service_name: grpc.grpc_service_name.unwrap_or_default(),
                host: opts.servername.clone().or_else(|| opts.sni.clone()),
            })
        }
        Some(Network::Xhttp) => {
            let xhttp = opts.xhttp_opts.clone().unwrap_or_default();
            let mode = match xhttp.mode.as_deref() {
                None | Some("") | Some("auto") | Some("stream-one") => XhttpMode::StreamOne,
                Some(other) => bail!("{proto}: xhttp mode {other:?} not implemented yet (only stream-one)"),
            };
            Transport::Xhttp(XhttpTransportConfig {
                path: xhttp.path.unwrap_or_default(),
                host: xhttp
                    .host
                    .clone()
                    .or_else(|| opts.servername.clone())
                    .or_else(|| opts.sni.clone()),
                mode,
            })
        }
        Some(Network::H2) => {
            // The `h2` transport runs HTTP/2 in the clear-of-framing sense but
            // is only defined over TLS (ALPN selects `h2`); REALITY rides TLS
            // 1.3, so it qualifies too.
            if !matches!(security, Security::Tls(_) | Security::Reality(_)) {
                bail!("{proto}: h2 transport requires TLS");
            }
            let h2 = opts.h2_opts.clone().unwrap_or_default();
            Transport::H2(H2TransportConfig {
                path: h2.path.unwrap_or_default(),
                host: h2
                    .host
                    .clone()
                    .or_else(|| opts.servername.clone())
                    .or_else(|| opts.sni.clone()),
            })
        }
        Some(other) => bail!("{proto}: transport {other:?} not implemented yet"),
    };

    // gRPC, XHTTP and h2 all run over HTTP/2; make sure the TLS/REALITY
    // handshake advertises `h2` so the server selects the right protocol.
    if matches!(transport, Transport::Grpc(_) | Transport::Xhttp(_) | Transport::H2(_))
        && let Some(alpn) = security.alpn_mut()
        && !alpn.iter().any(|p| p == "h2")
    {
        *alpn = vec!["h2".to_string()];
    }

    Ok((security, transport))
}

/// Assemble a [`RealityClientConfig`] from a proxy's `reality-opts` plus the
/// shared `servername` / `client-fingerprint` fields. REALITY needs a masquerade
/// SNI and the server's static x25519 public key; both are hard requirements and
/// are rejected here rather than producing a handshake that cannot authenticate.
fn build_reality(
    opts: &ProxyOptions,
    reality_opts: &RealityOpts,
    client_fingerprint: Option<ClientFingerprint>,
    proto: &str,
) -> Result<RealityClientConfig> {
    let server_name = opts
        .servername
        .clone()
        .or_else(|| opts.sni.clone())
        .filter(|s| !s.is_empty())
        .with_context(|| format!("{proto}: REALITY requires a servername (masquerade SNI)"))?;

    let public_key = reality_opts
        .public_key
        .as_deref()
        .filter(|s| !s.is_empty())
        .with_context(|| format!("{proto}: REALITY requires reality-opts.public-key"))?;
    let public_key = decode_reality_public_key(public_key, proto)?;

    let short_id = match reality_opts.short_id.as_deref() {
        None | Some("") => Vec::new(),
        Some(hex) => decode_short_id(hex, proto)?,
    };

    Ok(RealityClientConfig {
        server_name,
        public_key,
        short_id,
        alpn: opts.alpn.clone().unwrap_or_default(),
        skip_cert_verify: opts.skip_cert_verify.unwrap_or(false),
        client_fingerprint,
    })
}

/// Decode a REALITY `public-key`: an x25519 public key in base64 (clash/mihomo
/// and Xray use URL-safe RawStdEncoding, but standard base64 with padding is
/// accepted too). Must decode to exactly 32 bytes.
fn decode_reality_public_key(value: &str, proto: &str) -> Result<[u8; 32]> {
    let bytes = base64_decode(value.trim())
        .with_context(|| format!("{proto}: invalid REALITY public-key (expected base64)"))?;
    let len = bytes.len();
    let key: [u8; 32] = bytes
        .try_into()
        .map_err(|_| anyhow!("{proto}: REALITY public-key must decode to 32 bytes, got {len}"))?;
    Ok(key)
}

/// Decode a REALITY `short-id`: a hex string of even length, at most 16 chars
/// (8 bytes), matching one of the server's configured short ids.
fn decode_short_id(value: &str, proto: &str) -> Result<Vec<u8>> {
    let value = value.trim();
    if value.len() > 16 {
        bail!(
            "{proto}: REALITY short-id must be at most 16 hex digits (8 bytes), got {}",
            value.len()
        );
    }
    if !value.len().is_multiple_of(2) {
        bail!(
            "{proto}: REALITY short-id must have an even number of hex digits, got {}",
            value.len()
        );
    }
    value
        .as_bytes()
        .chunks(2)
        .map(|pair| Ok((hex_val(pair[0], proto)? << 4) | hex_val(pair[1], proto)?))
        .collect()
}

/// Map a single ASCII hex digit to its 4-bit value.
fn hex_val(c: u8, proto: &str) -> Result<u8> {
    match c {
        b'0'..=b'9' => Ok(c - b'0'),
        b'a'..=b'f' => Ok(c - b'a' + 10),
        b'A'..=b'F' => Ok(c - b'A' + 10),
        other => bail!("{proto}: invalid hex digit {:?} in REALITY short-id", other as char),
    }
}

/// Minimal base64 decoder accepting both the standard (`+`/`/`) and URL-safe
/// (`-`/`_`) alphabets, with or without `=` padding. Encoding choice (not a
/// security primitive) is the only thing kept in-house; the cryptography stays
/// in the vendored rustls fork.
fn base64_decode(input: &str) -> Result<Vec<u8>> {
    fn sextet(c: u8) -> Option<u8> {
        match c {
            b'A'..=b'Z' => Some(c - b'A'),
            b'a'..=b'z' => Some(c - b'a' + 26),
            b'0'..=b'9' => Some(c - b'0' + 52),
            b'+' | b'-' => Some(62),
            b'/' | b'_' => Some(63),
            _ => None,
        }
    }

    let mut acc: u32 = 0;
    let mut bits = 0u32;
    let mut out = Vec::with_capacity(input.len() * 3 / 4);
    for &c in input.as_bytes() {
        if matches!(c, b'=' | b'\r' | b'\n') {
            continue;
        }
        let value = sextet(c).ok_or_else(|| anyhow!("invalid base64 character {:?}", c as char))?;
        acc = (acc << 6) | u32::from(value);
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push((acc >> bits) as u8);
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base64_decode_handles_both_alphabets_and_padding() {
        assert_eq!(base64_decode("AAAA").unwrap(), vec![0, 0, 0]);
        assert_eq!(base64_decode("AA==").unwrap(), vec![0]);
        // `////` (standard) and `____` (URL-safe) both decode to 0xFF bytes.
        assert_eq!(base64_decode("////").unwrap(), vec![0xff, 0xff, 0xff]);
        assert_eq!(base64_decode("____").unwrap(), vec![0xff, 0xff, 0xff]);
        assert!(base64_decode("**bad**").is_err());
    }

    #[test]
    fn decode_short_id_roundtrips_and_validates() {
        assert_eq!(decode_short_id("0123abCD", "t").unwrap(), vec![0x01, 0x23, 0xab, 0xcd]);
        assert!(decode_short_id("abc", "t").is_err(), "odd length");
        assert!(decode_short_id("0123456789abcdef01", "t").is_err(), "too long");
        assert!(decode_short_id("zz", "t").is_err(), "non-hex");
    }

    #[test]
    fn decode_public_key_requires_32_bytes() {
        assert_eq!(decode_reality_public_key(&"A".repeat(43), "t").unwrap(), [0u8; 32]);
        assert!(decode_reality_public_key("AAAA", "t").is_err());
    }
}
