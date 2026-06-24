//! TLS client transport for outbound protocols.
//!
//! Uses `rustls` (ring crypto provider) — TLS and cryptography are exactly the
//! kind of "do not hand-roll" surface called out in the kernel build-vs-adopt
//! boundary, so learn-gripe stands on a vetted implementation here.

use std::sync::Arc;

use anyhow::{Context, Result};
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
