//! QUIC client transport, built on `quinn`.
//!
//! This is the shared QUIC dialer used by the QUIC-family outbounds (TUIC
//! today; Hysteria2 / MASQUE later). It owns only the QUIC connection setup:
//! building a client [`Endpoint`], applying the TLS 1.3 crypto config (the same
//! vendored rustls fork + `ring` provider the TCP transport uses, via
//! [`crate::transport::tls::quic_client_config`]), tuning the transport
//! parameters (keep-alive, idle timeout, congestion controller), and dialing a
//! [`quinn::Connection`]. The protocol framing that runs *over* the connection
//! (authentication, stream commands) lives in the protocol modules.

use std::net::{Ipv4Addr, Ipv6Addr, SocketAddr};
use std::sync::Arc;
use std::time::Duration;

use anyhow::{Context, Result, anyhow, bail};
use quinn::crypto::rustls::QuicClientConfig;
use quinn::{ClientConfig, Connection, Endpoint, TransportConfig};

use crate::transport::tls;

/// QUIC congestion controller selection (`congestion-controller` in clash
/// configs). This is a purely local send-rate choice; it does not affect wire
/// interop, so any value dials successfully against any server.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Congestion {
    Cubic,
    NewReno,
    Bbr,
}

impl Congestion {
    /// Parse a clash `congestion-controller` value (case-insensitive). Unknown
    /// values fall back to BBR, the controller TUIC deployments commonly use.
    pub fn parse(value: &str) -> Self {
        match value.trim().to_ascii_lowercase().as_str() {
            "cubic" => Congestion::Cubic,
            "new_reno" | "new-reno" | "newreno" | "reno" => Congestion::NewReno,
            _ => Congestion::Bbr,
        }
    }
}

/// Parameters for dialing a QUIC server.
#[derive(Debug, Clone)]
pub struct QuicClientParams {
    /// Server host to resolve and dial (domain or IP literal).
    pub server: String,
    /// Server UDP port.
    pub port: u16,
    /// TLS SNI / certificate name to validate against.
    pub server_name: String,
    /// ALPN protocols to offer (QUIC requires at least one).
    pub alpn: Vec<String>,
    /// Accept any server certificate (`skip-cert-verify`).
    pub skip_cert_verify: bool,
    /// Send-side congestion controller.
    pub congestion: Congestion,
}

/// A live QUIC connection plus the endpoint that owns its UDP socket and driver
/// task. Both are held together because dropping the [`Endpoint`] tears down the
/// background driver the [`Connection`] depends on, so callers that keep a
/// connection alive must keep the endpoint alive too.
pub struct QuicConnection {
    pub endpoint: Endpoint,
    pub connection: Connection,
}

/// Resolve and dial `params`, completing the QUIC (TLS 1.3) handshake.
pub async fn connect(params: &QuicClientParams) -> Result<QuicConnection> {
    if params.alpn.is_empty() {
        bail!("QUIC requires at least one ALPN protocol");
    }

    let crypto = tls::quic_client_config(&params.alpn, params.skip_cert_verify)?;
    let quic_crypto = QuicClientConfig::try_from(crypto).map_err(|e| anyhow!("build QUIC crypto config: {e}"))?;
    let mut client_config = ClientConfig::new(Arc::new(quic_crypto));
    client_config.transport_config(Arc::new(transport_config(params.congestion)));

    let addr = resolve(&params.server, params.port).await?;
    // Bind the client socket on the address family of the resolved server so the
    // OS routes the datagrams correctly (an IPv4 socket cannot reach an IPv6
    // peer and vice versa).
    let bind: SocketAddr = if addr.is_ipv6() {
        (Ipv6Addr::UNSPECIFIED, 0).into()
    } else {
        (Ipv4Addr::UNSPECIFIED, 0).into()
    };
    let mut endpoint = Endpoint::client(bind).context("bind QUIC client endpoint")?;
    endpoint.set_default_client_config(client_config);

    let connection = endpoint
        .connect(addr, &params.server_name)
        .with_context(|| format!("start QUIC connect to {addr}"))?
        .await
        .with_context(|| format!("QUIC handshake with {} ({addr})", params.server_name))?;

    Ok(QuicConnection { endpoint, connection })
}

/// Transport tuning shared by all QUIC outbounds: a keep-alive PING below the
/// idle timeout so an idle relay connection is not reaped, plus the selected
/// congestion controller.
fn transport_config(congestion: Congestion) -> TransportConfig {
    let mut transport = TransportConfig::default();
    transport.keep_alive_interval(Some(Duration::from_secs(8)));
    transport.max_idle_timeout(Some(
        Duration::from_secs(30)
            .try_into()
            .expect("30s is a valid QUIC idle timeout"),
    ));
    match congestion {
        Congestion::Cubic => {
            transport.congestion_controller_factory(Arc::new(quinn::congestion::CubicConfig::default()));
        }
        Congestion::NewReno => {
            transport.congestion_controller_factory(Arc::new(quinn::congestion::NewRenoConfig::default()));
        }
        Congestion::Bbr => {
            transport.congestion_controller_factory(Arc::new(quinn::congestion::BbrConfig::default()));
        }
    }
    transport
}

/// Resolve `host:port` to a single socket address, preferring the first result.
async fn resolve(host: &str, port: u16) -> Result<SocketAddr> {
    tokio::net::lookup_host((host, port))
        .await
        .with_context(|| format!("resolve QUIC server {host}:{port}"))?
        .next()
        .ok_or_else(|| anyhow!("no addresses resolved for QUIC server {host}:{port}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_congestion_controllers() {
        assert_eq!(Congestion::parse("cubic"), Congestion::Cubic);
        assert_eq!(Congestion::parse("CUBIC"), Congestion::Cubic);
        assert_eq!(Congestion::parse("new_reno"), Congestion::NewReno);
        assert_eq!(Congestion::parse("new-reno"), Congestion::NewReno);
        assert_eq!(Congestion::parse("bbr"), Congestion::Bbr);
        // Unknown / empty falls back to BBR.
        assert_eq!(Congestion::parse("whatever"), Congestion::Bbr);
        assert_eq!(Congestion::parse(""), Congestion::Bbr);
    }
}
