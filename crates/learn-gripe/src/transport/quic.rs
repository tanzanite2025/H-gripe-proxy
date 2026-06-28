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
use quinn::{ClientConfig, Connection, Endpoint, EndpointConfig, TransportConfig, ZeroRttAccepted};

use crate::transport::quic_obfs::{ObfsHopSocket, PacketObfs, PortHopConfig};
use crate::transport::tls;

/// QUIC congestion controller selection (`congestion-controller` in clash
/// configs). This is a purely local send-rate choice; it does not affect wire
/// interop, so any value dials successfully against any server.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
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
    /// Packet obfuscation applied to every QUIC datagram (Salamander for
    /// Hysteria2, XPlus for Hysteria v1), or `None` for a plain QUIC socket.
    pub obfs: Option<PacketObfs>,
    /// Port hopping: spread datagrams across a range of server ports, or `None`
    /// to always dial the configured port.
    pub port_hop: Option<PortHopConfig>,
    /// Attempt a 0-RTT handshake: when a resumption ticket is cached for this
    /// server, return the connection before the handshake completes so the
    /// caller can send early data. Falls back to a full 1-RTT handshake when no
    /// ticket is available (e.g. the first dial).
    pub zero_rtt: bool,
}

/// A live QUIC connection plus the endpoint that owns its UDP socket and driver
/// task. Both are held together because dropping the [`Endpoint`] tears down the
/// background driver the [`Connection`] depends on, so callers that keep a
/// connection alive must keep the endpoint alive too.
pub struct QuicConnection {
    pub endpoint: Endpoint,
    pub connection: Connection,
    /// `Some` when the connection was returned in 0-RTT state (handshake still
    /// in flight). Early data sent now is replay-vulnerable; awaiting the future
    /// yields `true` once the handshake completes if the server accepted the
    /// 0-RTT data and `false` if it rejected it (streams opened before
    /// completion then error and must be retried). `None` for a full handshake.
    pub zero_rtt: Option<ZeroRttAccepted>,
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
    let mut endpoint = build_endpoint(bind, addr, params)?;
    endpoint.set_default_client_config(client_config);

    let connecting = endpoint
        .connect(addr, &params.server_name)
        .with_context(|| format!("start QUIC connect to {addr}"))?;

    // With 0-RTT requested, `into_0rtt` hands back a usable connection before the
    // handshake finishes whenever a resumption ticket is cached; it only fails
    // (returning the pending `Connecting`) when no 0-RTT attempt is possible, in
    // which case we await the normal 1-RTT handshake. 0-RTT data is replayable,
    // so callers must only send it for idempotent-safe requests.
    let (connection, zero_rtt) = if params.zero_rtt {
        match connecting.into_0rtt() {
            Ok((connection, accepted)) => (connection, Some(accepted)),
            Err(connecting) => {
                let connection = connecting
                    .await
                    .with_context(|| format!("QUIC handshake with {} ({addr})", params.server_name))?;
                (connection, None)
            }
        }
    } else {
        let connection = connecting
            .await
            .with_context(|| format!("QUIC handshake with {} ({addr})", params.server_name))?;
        (connection, None)
    };

    Ok(QuicConnection {
        endpoint,
        connection,
        zero_rtt,
    })
}

/// Build the client [`Endpoint`] bound at `bind`. When the params request
/// Salamander obfuscation or port hopping, the runtime's UDP socket is wrapped
/// in an [`ObfsHopSocket`] that transforms every datagram below QUIC; otherwise
/// the plain `Endpoint::client` path is used. `canonical` is the resolved server
/// address quinn associates with the connection.
fn build_endpoint(bind: SocketAddr, canonical: SocketAddr, params: &QuicClientParams) -> Result<Endpoint> {
    if params.obfs.is_none() && params.port_hop.is_none() {
        return Endpoint::client(bind).context("bind QUIC client endpoint");
    }

    let runtime = quinn::default_runtime().ok_or_else(|| anyhow!("no async runtime found for QUIC endpoint"))?;
    let socket = std::net::UdpSocket::bind(bind).context("bind QUIC client socket")?;
    let inner = runtime.wrap_udp_socket(socket).context("wrap QUIC client socket")?;
    let wrapped = Arc::new(ObfsHopSocket::new(
        inner,
        canonical,
        params.obfs.clone(),
        params.port_hop.clone(),
    ));
    Endpoint::new_with_abstract_socket(EndpointConfig::default(), None, wrapped, runtime)
        .context("build QUIC endpoint with obfuscated socket")
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

    const TEST_CERT: &str = include_str!("../../tests/data/vless_tls_cert.pem");
    const TEST_KEY: &str = include_str!("../../tests/data/vless_tls_key.pem");

    /// A quinn echo server that issues TLS 1.3 session tickets and accepts 0-RTT
    /// early data (`max_early_data_size = 0xffff_ffff`, the only non-zero value
    /// QUIC permits, with the default stateful session cache).
    fn zero_rtt_server_config() -> quinn::ServerConfig {
        let certs = rustls_pemfile::certs(&mut TEST_CERT.as_bytes())
            .collect::<Result<Vec<_>, _>>()
            .unwrap();
        let key = rustls_pemfile::private_key(&mut TEST_KEY.as_bytes()).unwrap().unwrap();
        let mut crypto =
            rustls::ServerConfig::builder_with_provider(Arc::new(rustls::crypto::ring::default_provider()))
                .with_protocol_versions(&[&rustls::version::TLS13])
                .unwrap()
                .with_no_client_auth()
                .with_single_cert(certs, key)
                .unwrap();
        crypto.alpn_protocols = vec![b"h3".to_vec()];
        crypto.max_early_data_size = u32::MAX;
        let quic = quinn::crypto::rustls::QuicServerConfig::try_from(crypto).unwrap();
        quinn::ServerConfig::with_crypto(Arc::new(quic))
    }

    /// Accept connections and echo back each bidirectional stream's bytes.
    async fn run_echo_server(endpoint: Endpoint) {
        while let Some(incoming) = endpoint.accept().await {
            tokio::spawn(async move {
                let Ok(conn) = incoming.await else { return };
                while let Ok((mut send, mut recv)) = conn.accept_bi().await {
                    tokio::spawn(async move {
                        if let Ok(data) = recv.read_to_end(64 * 1024).await {
                            let _ = send.write_all(&data).await;
                            let _ = send.finish();
                        }
                    });
                }
            });
        }
    }

    fn params(port: u16, zero_rtt: bool, server_name: &str) -> QuicClientParams {
        QuicClientParams {
            server: "127.0.0.1".to_string(),
            port,
            server_name: server_name.to_string(),
            alpn: vec!["h3".to_string()],
            skip_cert_verify: true,
            congestion: Congestion::Bbr,
            obfs: None,
            port_hop: None,
            zero_rtt,
        }
    }

    async fn echo_round_trip(conn: &Connection, msg: &[u8]) -> Vec<u8> {
        let (mut send, mut recv) = conn.open_bi().await.unwrap();
        send.write_all(msg).await.unwrap();
        send.finish().unwrap();
        recv.read_to_end(64 * 1024).await.unwrap()
    }

    // First dial has no cached ticket, so it cannot attempt 0-RTT; a later dial
    // resumes the session and returns a 0-RTT connection that carries early data.
    #[tokio::test]
    async fn zero_rtt_resumes_after_a_prior_session() {
        let endpoint = Endpoint::server(zero_rtt_server_config(), (Ipv4Addr::LOCALHOST, 0).into()).unwrap();
        let port = endpoint.local_addr().unwrap().port();
        let server = tokio::spawn(run_echo_server(endpoint));

        // A per-test SNI keeps the process-wide session cache (keyed by server
        // name) from colliding with tickets banked by other tests' servers,
        // whose distinct ticket keys would make the server reject resumption.
        let sni = "resume.example";

        // First dial: no ticket yet, so 0-RTT is impossible even when requested.
        let warm = connect(&params(port, true, sni)).await.unwrap();
        assert!(warm.zero_rtt.is_none(), "first dial cannot resume");
        assert_eq!(echo_round_trip(&warm.connection, b"warmup").await, b"warmup");

        // A NewSessionTicket arrives shortly after the handshake; retry until the
        // client has cached it and `into_0rtt` succeeds.
        let mut zero_rtt = None;
        for _ in 0..20 {
            tokio::time::sleep(Duration::from_millis(20)).await;
            let quic = connect(&params(port, true, sni)).await.unwrap();
            if quic.zero_rtt.is_some() {
                zero_rtt = Some(quic);
                break;
            }
            // Fallback 1-RTT dial; finish it so it, too, banks a fresh ticket.
            let _ = echo_round_trip(&quic.connection, b"probe").await;
        }
        let quic = zero_rtt.expect("a resumed dial should yield a 0-RTT connection");

        // Early data sent on the 0-RTT connection round-trips through the server.
        assert_eq!(echo_round_trip(&quic.connection, b"early data").await, b"early data");

        drop(warm);
        drop(quic);
        server.abort();
    }

    // Without `zero_rtt`, even a resumable session always awaits the full
    // handshake and never returns a 0-RTT connection.
    #[tokio::test]
    async fn full_handshake_when_zero_rtt_disabled() {
        let endpoint = Endpoint::server(zero_rtt_server_config(), (Ipv4Addr::LOCALHOST, 0).into()).unwrap();
        let port = endpoint.local_addr().unwrap().port();
        let server = tokio::spawn(run_echo_server(endpoint));

        for _ in 0..3 {
            let quic = connect(&params(port, false, "no-rtt.example")).await.unwrap();
            assert!(
                quic.zero_rtt.is_none(),
                "0-RTT disabled must always complete the handshake"
            );
            assert_eq!(echo_round_trip(&quic.connection, b"hello").await, b"hello");
            tokio::time::sleep(Duration::from_millis(20)).await;
        }

        server.abort();
    }
}
