//! rustls client configuration for the OpenVPN control-channel TLS handshake.
//!
//! OpenVPN pins the server certificate to an inline CA (`ca`) rather than the
//! web PKI, and it does *not* verify the dial hostname (there is no SNI-style
//! name check by default — `remote-cert-tls` checks key usage, not the name).
//! This verifier therefore validates the chain against the configured CA and
//! deliberately skips the hostname check, but never disables signature/chain
//! verification. Optional client certificate auth is wired when both `cert` and
//! `key` are supplied.

use std::io::Cursor;
use std::sync::Arc;

use anyhow::{Result, bail};
use rustls::ClientConfig;
use rustls::client::danger::{HandshakeSignatureValid, ServerCertVerified, ServerCertVerifier};
use rustls::client::verify_server_cert_signed_by_trust_anchor;
use rustls::crypto::{CryptoProvider, ring, verify_tls12_signature, verify_tls13_signature};
use rustls::pki_types::{CertificateDer, PrivateKeyDer, ServerName, UnixTime};
use rustls::server::ParsedCertificate;
use rustls::{DigitallySignedStruct, RootCertStore, SignatureScheme};

/// Build a rustls [`ClientConfig`] that trusts only the inline `ca_pem` and
/// optionally presents a client certificate.
pub(super) fn build_client_config(
    ca_pem: &str,
    client_cert_pem: Option<&str>,
    client_key_pem: Option<&str>,
) -> Result<Arc<ClientConfig>> {
    let provider = Arc::new(ring::default_provider());

    let mut roots = RootCertStore::empty();
    for cert in parse_certs(ca_pem)? {
        roots
            .add(cert)
            .map_err(|e| anyhow::anyhow!("openvpn: invalid ca certificate: {e}"))?;
    }
    if roots.is_empty() {
        bail!("openvpn: ca certificate contained no certificates");
    }

    let verifier = Arc::new(InlineCaVerifier {
        roots,
        provider: provider.clone(),
    });

    let builder = ClientConfig::builder_with_provider(provider)
        .with_safe_default_protocol_versions()
        .map_err(|e| anyhow::anyhow!("openvpn: tls versions: {e}"))?
        .dangerous()
        .with_custom_certificate_verifier(verifier);

    let config = match (client_cert_pem, client_key_pem) {
        (Some(cert_pem), Some(key_pem)) => {
            let certs = parse_certs(cert_pem)?;
            if certs.is_empty() {
                bail!("openvpn: client certificate contained no certificates");
            }
            let key = parse_private_key(key_pem)?;
            builder
                .with_client_auth_cert(certs, key)
                .map_err(|e| anyhow::anyhow!("openvpn: client certificate: {e}"))?
        }
        (None, None) => builder.with_no_client_auth(),
        _ => bail!("openvpn: both `cert` and `key` are required for client certificate auth"),
    };

    Ok(Arc::new(config))
}

/// Build a rustls [`ServerName`] for SNI. The name is not verified (OpenVPN does
/// not check it), so a non-DNS server host falls back to a fixed placeholder.
pub(super) fn server_name(host: &str) -> ServerName<'static> {
    ServerName::try_from(host.to_owned())
        .or_else(|_| ServerName::try_from("openvpn"))
        .expect("static placeholder server name is valid")
}

fn parse_certs(pem: &str) -> Result<Vec<CertificateDer<'static>>> {
    let mut reader = Cursor::new(pem.as_bytes());
    rustls_pemfile::certs(&mut reader)
        .collect::<std::result::Result<Vec<_>, _>>()
        .map_err(|e| anyhow::anyhow!("openvpn: parse certificates: {e}"))
}

fn parse_private_key(pem: &str) -> Result<PrivateKeyDer<'static>> {
    let mut reader = Cursor::new(pem.as_bytes());
    rustls_pemfile::private_key(&mut reader)
        .map_err(|e| anyhow::anyhow!("openvpn: parse private key: {e}"))?
        .ok_or_else(|| anyhow::anyhow!("openvpn: no private key found"))
}

/// Certificate verifier pinned to an inline CA, skipping the hostname check.
#[derive(Debug)]
struct InlineCaVerifier {
    roots: RootCertStore,
    provider: Arc<CryptoProvider>,
}

impl ServerCertVerifier for InlineCaVerifier {
    fn verify_server_cert(
        &self,
        end_entity: &CertificateDer<'_>,
        intermediates: &[CertificateDer<'_>],
        _server_name: &ServerName<'_>,
        _ocsp_response: &[u8],
        now: UnixTime,
    ) -> Result<ServerCertVerified, rustls::Error> {
        let cert = ParsedCertificate::try_from(end_entity)?;
        verify_server_cert_signed_by_trust_anchor(
            &cert,
            &self.roots,
            intermediates,
            now,
            self.provider.signature_verification_algorithms.all,
        )?;
        Ok(ServerCertVerified::assertion())
    }

    fn verify_tls12_signature(
        &self,
        message: &[u8],
        cert: &CertificateDer<'_>,
        dss: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, rustls::Error> {
        verify_tls12_signature(message, cert, dss, &self.provider.signature_verification_algorithms)
    }

    fn verify_tls13_signature(
        &self,
        message: &[u8],
        cert: &CertificateDer<'_>,
        dss: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, rustls::Error> {
        verify_tls13_signature(message, cert, dss, &self.provider.signature_verification_algorithms)
    }

    fn supported_verify_schemes(&self) -> Vec<SignatureScheme> {
        self.provider.signature_verification_algorithms.supported_schemes()
    }
}
