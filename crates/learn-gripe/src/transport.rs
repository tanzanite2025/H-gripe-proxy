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
//! transports over `none`/`tls` security. `reality` lands in a follow-up and
//! slots into the same `Transport`/`Security` enums without restructuring.

use anyhow::Result;
use tokio::net::TcpStream;

use crate::grpc::GrpcTransportConfig;
use crate::http2::H2TransportConfig;
use crate::httpupgrade::HttpUpgradeTransportConfig;
use crate::outbound::BoxedStream;
use crate::tls::TlsClientConfig;
use crate::ws::WsTransportConfig;
use crate::xhttp::XhttpTransportConfig;

/// The security layer wrapping the raw TCP socket.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Security {
    /// Plaintext — no security wrapper.
    None,
    /// Standard TLS (rustls).
    Tls(TlsClientConfig),
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

    let over_tls = matches!(security, Security::Tls(_));
    let secured: BoxedStream = match security {
        Security::None => Box::new(tcp),
        Security::Tls(cfg) => Box::new(crate::tls::connect(cfg, server, tcp).await?),
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
