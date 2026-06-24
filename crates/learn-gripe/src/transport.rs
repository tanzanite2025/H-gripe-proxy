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
//! This slice implements `tcp` and `ws` transports over `none`/`tls` security.
//! `grpc`/`h2`/`xhttp`/`httpupgrade` and `reality` are represented in the type
//! system as explicit "not yet" arms so the wiring is ready for follow-ups.

use anyhow::Result;
use tokio::net::TcpStream;

use crate::outbound::BoxedStream;
use crate::tls::TlsClientConfig;
use crate::ws::WsTransportConfig;

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
}

/// Dial `server:port`, apply `security`, then `transport`, returning a
/// relay-ready byte stream onto which a protocol layer can write its framing.
pub async fn establish(server: &str, port: u16, security: &Security, transport: &Transport) -> Result<BoxedStream> {
    let tcp = TcpStream::connect((server, port))
        .await
        .map_err(|e| anyhow::anyhow!("dial {server}:{port}: {e}"))?;

    let secured: BoxedStream = match security {
        Security::None => Box::new(tcp),
        Security::Tls(cfg) => Box::new(crate::tls::connect(cfg, server, tcp).await?),
    };

    let transported: BoxedStream = match transport {
        Transport::Tcp => secured,
        Transport::Ws(cfg) => Box::new(crate::ws::connect(secured, server, cfg).await?),
    };

    Ok(transported)
}
