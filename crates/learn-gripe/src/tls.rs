//! TLS client transport for outbound protocols.
//!
//! Uses `rustls` (ring crypto provider) — TLS and cryptography are exactly the
//! kind of "do not hand-roll" surface called out in the kernel build-vs-adopt
//! boundary, so learn-gripe stands on a vetted implementation here.

use std::sync::Arc;

use anyhow::{Context, Result, anyhow, bail};
use rustls::client::RealityConfig;
use rustls::client::danger::{HandshakeSignatureValid, ServerCertVerified, ServerCertVerifier};
use rustls::crypto::{CryptoProvider, ring, verify_tls12_signature, verify_tls13_signature};
use rustls::pki_types::{CertificateDer, ServerName, UnixTime};
use rustls::{DigitallySignedStruct, RootCertStore, SignatureScheme};
use tokio::io::{AsyncRead, AsyncWrite};
use tokio_rustls::TlsConnector;
use tokio_rustls::client::TlsStream;

/// Parameters for the outbound TLS handshake, distilled from a proxy's
/// TLS-related options.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TlsClientConfig {
    /// Server name to send in SNI / verify against. Falls back to the dial host
    /// when empty.
    pub server_name: Option<String>,
    /// ALPN protocols to offer, in preference order.
    pub alpn: Vec<String>,
    /// Disable certificate verification (maps to `skip-cert-verify: true`).
    pub skip_cert_verify: bool,
}

/// A uTLS-style client fingerprint to mimic in the TLS ClientHello.
///
/// REALITY relies on the client looking like a real browser, so configs carry a
/// `client-fingerprint` (e.g. `chrome`). This enum is the validated set clash /
/// mihomo accept; unknown values are rejected by the parser rather than silently
/// dropped.
///
/// NOTE: the vendored `rustls` fork does not expose a uTLS-style ClientHello
/// shaping hook, so the chosen fingerprint is currently recorded and threaded
/// through the config but does not reshape the handshake beyond what `rustls`
/// emits natively. Faithful per-browser ClientHello mimicry is tracked as
/// follow-up work; until then this field documents intent and keeps configs
/// parsing losslessly.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClientFingerprint {
    Chrome,
    Firefox,
    Safari,
    Ios,
    Android,
    Edge,
    Qq,
    Random,
    Randomized,
}

impl ClientFingerprint {
    /// Parse a clash/mihomo `client-fingerprint` value (case-insensitive).
    pub fn parse(value: &str) -> Result<Self> {
        let fp = match value.trim().to_ascii_lowercase().as_str() {
            "chrome" => Self::Chrome,
            "firefox" => Self::Firefox,
            "safari" => Self::Safari,
            "ios" => Self::Ios,
            "android" => Self::Android,
            "edge" => Self::Edge,
            "qq" => Self::Qq,
            "random" => Self::Random,
            "randomized" => Self::Randomized,
            other => bail!("unknown client-fingerprint {other:?}"),
        };
        Ok(fp)
    }
}

/// Parameters for an outbound REALITY handshake, distilled from a proxy's
/// `reality-opts` plus `servername` / `client-fingerprint`.
///
/// REALITY rides standard TLS 1.3: the client embeds an x25519-derived auth
/// token in the ClientHello `session_id` (via the vendored fork's
/// [`RealityConfig`]) and masquerades as `server_name` in SNI. Because security
/// and transport are orthogonal (see [`crate::transport`]), the same config
/// works under tcp / grpc / h2 / xhttp.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RealityClientConfig {
    /// Masquerade SNI sent in the ClientHello (`servername` / `sni`).
    pub server_name: String,
    /// Server's static x25519 public key (32 bytes), from
    /// `reality-opts.public-key` (base64).
    pub public_key: [u8; 32],
    /// Short id (0..=8 bytes), from `reality-opts.short-id` (hex).
    pub short_id: Vec<u8>,
    /// ALPN protocols to offer, in preference order.
    pub alpn: Vec<String>,
    /// Disable certificate verification for the fallback (non-REALITY) path
    /// (maps to `skip-cert-verify: true`). The REALITY HMAC check still runs
    /// first; this only loosens the inner verifier it falls back to.
    pub skip_cert_verify: bool,
    /// uTLS client fingerprint to mimic, if any (see [`ClientFingerprint`]).
    pub client_fingerprint: Option<ClientFingerprint>,
}

/// Perform a TLS client handshake over an already-connected stream.
pub async fn connect<S>(config: &TlsClientConfig, dial_host: &str, stream: S) -> Result<TlsStream<S>>
where
    S: AsyncRead + AsyncWrite + Unpin,
{
    let provider = Arc::new(ring::default_provider());

    let mut client_config = if config.skip_cert_verify {
        rustls::ClientConfig::builder_with_provider(provider.clone())
            .with_safe_default_protocol_versions()
            .context("configure TLS protocol versions")?
            .dangerous()
            .with_custom_certificate_verifier(Arc::new(NoCertVerification(provider)))
            .with_no_client_auth()
    } else {
        let mut roots = RootCertStore::empty();
        roots.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());
        rustls::ClientConfig::builder_with_provider(provider)
            .with_safe_default_protocol_versions()
            .context("configure TLS protocol versions")?
            .with_root_certificates(roots)
            .with_no_client_auth()
    };

    client_config.alpn_protocols = config.alpn.iter().map(|p| p.as_bytes().to_vec()).collect();

    let sni = config
        .server_name
        .as_deref()
        .filter(|s| !s.is_empty())
        .unwrap_or(dial_host);
    let server_name =
        ServerName::try_from(sni.to_owned()).with_context(|| format!("invalid TLS server name {sni:?}"))?;

    let connector = TlsConnector::from(Arc::new(client_config));
    connector
        .connect(server_name, stream)
        .await
        .with_context(|| format!("TLS handshake with {sni}"))
}

/// Perform a REALITY TLS client handshake over an already-connected stream.
///
/// REALITY mandates TLS 1.3, so only that version is offered. The vendored fork
/// wraps the chosen certificate verifier in a `RealityServerCertVerifier` via
/// [`with_reality`](rustls::ConfigBuilder::with_reality), which validates the
/// REALITY HMAC cert and otherwise falls back to the inner verifier.
pub async fn connect_reality<S>(config: &RealityClientConfig, dial_host: &str, stream: S) -> Result<TlsStream<S>>
where
    S: AsyncRead + AsyncWrite + Unpin,
{
    let provider = Arc::new(ring::default_provider());

    let reality = RealityConfig::new(config.public_key, config.short_id.clone())
        .map_err(|e| anyhow!("build REALITY config: {e}"))?;

    let builder = rustls::ClientConfig::builder_with_provider(provider.clone())
        .with_protocol_versions(&[&rustls::version::TLS13])
        .context("configure TLS 1.3 for REALITY")?;

    let mut client_config = if config.skip_cert_verify {
        builder
            .dangerous()
            .with_custom_certificate_verifier(Arc::new(NoCertVerification(provider)))
            .with_reality(reality)
            .with_no_client_auth()
    } else {
        let mut roots = RootCertStore::empty();
        roots.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());
        builder
            .with_root_certificates(roots)
            .with_reality(reality)
            .with_no_client_auth()
    };

    client_config.alpn_protocols = config.alpn.iter().map(|p| p.as_bytes().to_vec()).collect();

    let sni = config.server_name.as_str();
    if sni.is_empty() {
        bail!("REALITY requires a non-empty servername for SNI masquerade");
    }
    let server_name =
        ServerName::try_from(sni.to_owned()).with_context(|| format!("invalid REALITY server name {sni:?}"))?;

    let connector = TlsConnector::from(Arc::new(client_config));
    connector
        .connect(server_name, stream)
        .await
        .with_context(|| format!("REALITY handshake (masquerade SNI {sni}, dial {dial_host})"))
}

/// A certificate verifier that accepts any server certificate. Used only when
/// the proxy explicitly opts into `skip-cert-verify`. Signature verification is
/// still delegated to the crypto provider so the handshake stays well-formed.
#[derive(Debug)]
struct NoCertVerification(Arc<CryptoProvider>);

impl ServerCertVerifier for NoCertVerification {
    fn verify_server_cert(
        &self,
        _end_entity: &CertificateDer<'_>,
        _intermediates: &[CertificateDer<'_>],
        _server_name: &ServerName<'_>,
        _ocsp_response: &[u8],
        _now: UnixTime,
    ) -> Result<ServerCertVerified, rustls::Error> {
        Ok(ServerCertVerified::assertion())
    }

    fn verify_tls12_signature(
        &self,
        message: &[u8],
        cert: &CertificateDer<'_>,
        dss: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, rustls::Error> {
        verify_tls12_signature(message, cert, dss, &self.0.signature_verification_algorithms)
    }

    fn verify_tls13_signature(
        &self,
        message: &[u8],
        cert: &CertificateDer<'_>,
        dss: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, rustls::Error> {
        verify_tls13_signature(message, cert, dss, &self.0.signature_verification_algorithms)
    }

    fn supported_verify_schemes(&self) -> Vec<SignatureScheme> {
        self.0.signature_verification_algorithms.supported_schemes()
    }
}
