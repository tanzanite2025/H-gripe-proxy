//! TLS client transport for outbound protocols.
//!
//! Uses `rustls` (ring crypto provider) — TLS and cryptography are exactly the
//! kind of "do not hand-roll" surface called out in the kernel build-vs-adopt
//! boundary, so learn-gripe stands on a vetted implementation here.

use std::sync::Arc;

use anyhow::{Context, Result, anyhow, bail};
use rustls::client::RealityConfig;
use rustls::client::danger::{HandshakeSignatureValid, ServerCertVerified, ServerCertVerifier};
use rustls::crypto::ring::cipher_suite;
use rustls::crypto::{CryptoProvider, SecureRandom, ring, verify_tls12_signature, verify_tls13_signature};
use rustls::pki_types::{CertificateDer, ServerName, UnixTime};
use rustls::{DigitallySignedStruct, RootCertStore, SignatureScheme, SupportedCipherSuite};
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
    /// uTLS client fingerprint to mimic in the ClientHello, if any (see
    /// [`ClientFingerprint`]). `None` uses the rustls default ordering.
    pub client_fingerprint: Option<ClientFingerprint>,
}

/// A uTLS-style client fingerprint to mimic in the TLS ClientHello.
///
/// REALITY relies on the client looking like a real browser, so configs carry a
/// `client-fingerprint` (e.g. `chrome`). This enum is the validated set clash /
/// mihomo accept; unknown values are rejected by the parser rather than silently
/// dropped.
///
/// The fingerprint reshapes the ClientHello's **cipher-suite ordering** — the
/// most prominent JA3 list field — via a per-fingerprint [`CryptoProvider`]
/// (see [`ClientFingerprint::crypto_provider`]). Ordering is drawn from the
/// suites the ring provider implements; the full suite *set* is preserved so
/// interop is never reduced, only the order changes to match the browser.
///
/// NOTE: this shapes the cipher-suite order (and, for `random`/`randomized`,
/// chooses/shuffles it at connect time). The vendored `rustls` fork does not
/// expose hooks for the remaining JA3 surface — TLS extension ordering, GREASE
/// values, padding — so byte-perfect per-browser mimicry remains follow-up
/// work. The supported-groups order already matches every modern browser
/// (`x25519`, `secp256r1`, `secp384r1`) so it is left at the provider default.
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

    /// The fixed cipher-suite order this fingerprint presents in its
    /// ClientHello, or `None` for the randomizing fingerprints (`random` /
    /// `randomized`), whose order is decided at connect time.
    fn fixed_cipher_order(self) -> Option<&'static [SupportedCipherSuite]> {
        match self {
            // Chromium-family browsers share Chrome's suite ordering.
            Self::Chrome | Self::Edge | Self::Qq | Self::Android => Some(CHROME_CIPHER_ORDER),
            Self::Firefox => Some(FIREFOX_CIPHER_ORDER),
            // iOS Safari uses the desktop Safari ordering.
            Self::Safari | Self::Ios => Some(SAFARI_CIPHER_ORDER),
            Self::Random | Self::Randomized => None,
        }
    }

    /// The cipher-suite list to advertise for this fingerprint. Concrete
    /// browsers return their fixed order; `random` presents one real browser's
    /// order and `randomized` shuffles the full suite set — both drawing from
    /// `rng` so the choice differs per connection.
    fn cipher_suites(self, rng: &dyn SecureRandom) -> Vec<SupportedCipherSuite> {
        if let Some(order) = self.fixed_cipher_order() {
            return order.to_vec();
        }
        match self {
            Self::Random => {
                let orders = [CHROME_CIPHER_ORDER, FIREFOX_CIPHER_ORDER, SAFARI_CIPHER_ORDER];
                orders[random_below(rng, orders.len())].to_vec()
            }
            // Randomized: shuffle the full set so the order varies per dial.
            _ => {
                let mut suites = ring::ALL_CIPHER_SUITES.to_vec();
                shuffle(rng, &mut suites);
                suites
            }
        }
    }

    /// Build a [`CryptoProvider`] whose ClientHello cipher-suite ordering mimics
    /// this browser fingerprint. The rest of the provider (key exchange groups,
    /// signature algorithms, RNG) is the vetted ring default — only the
    /// preference order, which feeds the ClientHello, is reshaped.
    pub fn crypto_provider(self) -> CryptoProvider {
        let mut provider = ring::default_provider();
        provider.cipher_suites = self.cipher_suites(provider.secure_random);
        provider
    }
}

/// Chrome / Chromium (also Edge, QQ, Android WebView) cipher-suite order,
/// restricted to the suites the ring provider implements. Real Chrome leads
/// with a GREASE value and includes CBC suites rustls does not, which is why
/// this is an ordering approximation rather than a byte-for-byte match.
static CHROME_CIPHER_ORDER: &[SupportedCipherSuite] = &[
    cipher_suite::TLS13_AES_128_GCM_SHA256,
    cipher_suite::TLS13_AES_256_GCM_SHA384,
    cipher_suite::TLS13_CHACHA20_POLY1305_SHA256,
    cipher_suite::TLS_ECDHE_ECDSA_WITH_AES_128_GCM_SHA256,
    cipher_suite::TLS_ECDHE_RSA_WITH_AES_128_GCM_SHA256,
    cipher_suite::TLS_ECDHE_ECDSA_WITH_AES_256_GCM_SHA384,
    cipher_suite::TLS_ECDHE_RSA_WITH_AES_256_GCM_SHA384,
    cipher_suite::TLS_ECDHE_ECDSA_WITH_CHACHA20_POLY1305_SHA256,
    cipher_suite::TLS_ECDHE_RSA_WITH_CHACHA20_POLY1305_SHA256,
];

/// Firefox cipher-suite order (TLS 1.3 `AES-128 / CHACHA20 / AES-256`, then the
/// ECDHE suites grouped 128 / CHACHA20 / 256), restricted to ring's suites.
static FIREFOX_CIPHER_ORDER: &[SupportedCipherSuite] = &[
    cipher_suite::TLS13_AES_128_GCM_SHA256,
    cipher_suite::TLS13_CHACHA20_POLY1305_SHA256,
    cipher_suite::TLS13_AES_256_GCM_SHA384,
    cipher_suite::TLS_ECDHE_ECDSA_WITH_AES_128_GCM_SHA256,
    cipher_suite::TLS_ECDHE_RSA_WITH_AES_128_GCM_SHA256,
    cipher_suite::TLS_ECDHE_ECDSA_WITH_CHACHA20_POLY1305_SHA256,
    cipher_suite::TLS_ECDHE_RSA_WITH_CHACHA20_POLY1305_SHA256,
    cipher_suite::TLS_ECDHE_ECDSA_WITH_AES_256_GCM_SHA384,
    cipher_suite::TLS_ECDHE_RSA_WITH_AES_256_GCM_SHA384,
];

/// Safari / iOS cipher-suite order (TLS 1.3 `AES-256 / CHACHA20 / AES-128`,
/// ECDSA suites ahead of RSA), restricted to ring's suites.
static SAFARI_CIPHER_ORDER: &[SupportedCipherSuite] = &[
    cipher_suite::TLS13_AES_256_GCM_SHA384,
    cipher_suite::TLS13_CHACHA20_POLY1305_SHA256,
    cipher_suite::TLS13_AES_128_GCM_SHA256,
    cipher_suite::TLS_ECDHE_ECDSA_WITH_AES_256_GCM_SHA384,
    cipher_suite::TLS_ECDHE_ECDSA_WITH_AES_128_GCM_SHA256,
    cipher_suite::TLS_ECDHE_ECDSA_WITH_CHACHA20_POLY1305_SHA256,
    cipher_suite::TLS_ECDHE_RSA_WITH_AES_256_GCM_SHA384,
    cipher_suite::TLS_ECDHE_RSA_WITH_AES_128_GCM_SHA256,
    cipher_suite::TLS_ECDHE_RSA_WITH_CHACHA20_POLY1305_SHA256,
];

/// Draw an index in `0..len` from the crypto provider's RNG. This only chooses
/// which fingerprint/ordering to present, so a small modulo bias is irrelevant;
/// on the rare RNG failure it falls back to the first element instead of
/// panicking on the dial path.
fn random_below(rng: &dyn SecureRandom, len: usize) -> usize {
    let mut byte = [0u8; 1];
    if rng.fill(&mut byte).is_err() {
        return 0;
    }
    usize::from(byte[0]) % len.max(1)
}

/// In-place Fisher–Yates shuffle of `suites`, seeded from the crypto provider's
/// RNG. Used only for the `randomized` fingerprint's cipher ordering, so RNG
/// failure simply leaves the default order in place.
fn shuffle(rng: &dyn SecureRandom, suites: &mut [SupportedCipherSuite]) {
    let mut bytes = vec![0u8; suites.len()];
    if rng.fill(&mut bytes).is_err() {
        return;
    }
    for i in (1..suites.len()).rev() {
        let j = usize::from(bytes[i]) % (i + 1);
        suites.swap(i, j);
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
    let provider = Arc::new(
        config
            .client_fingerprint
            .map(ClientFingerprint::crypto_provider)
            .unwrap_or_else(ring::default_provider),
    );

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
    let provider = Arc::new(
        config
            .client_fingerprint
            .map(ClientFingerprint::crypto_provider)
            .unwrap_or_else(ring::default_provider),
    );

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

#[cfg(test)]
mod tests {
    use super::*;

    /// All fingerprints whose ClientHello cipher order is deterministic.
    const FIXED: &[ClientFingerprint] = &[
        ClientFingerprint::Chrome,
        ClientFingerprint::Firefox,
        ClientFingerprint::Safari,
        ClientFingerprint::Ios,
        ClientFingerprint::Android,
        ClientFingerprint::Edge,
        ClientFingerprint::Qq,
    ];

    fn ids(suites: &[SupportedCipherSuite]) -> Vec<u16> {
        suites.iter().map(|s| u16::from(s.suite())).collect()
    }

    fn sorted_ids(suites: &[SupportedCipherSuite]) -> Vec<u16> {
        let mut v = ids(suites);
        v.sort_unstable();
        v
    }

    fn rng() -> &'static dyn SecureRandom {
        // Reuse the ring provider's RNG for the order-selection helpers.
        ring::default_provider().secure_random
    }

    #[test]
    fn parse_round_trips_all_known_fingerprints() {
        for (name, fp) in [
            ("chrome", ClientFingerprint::Chrome),
            ("Firefox", ClientFingerprint::Firefox),
            ("SAFARI", ClientFingerprint::Safari),
            ("ios", ClientFingerprint::Ios),
            ("android", ClientFingerprint::Android),
            ("edge", ClientFingerprint::Edge),
            ("qq", ClientFingerprint::Qq),
            ("random", ClientFingerprint::Random),
            ("randomized", ClientFingerprint::Randomized),
        ] {
            assert_eq!(ClientFingerprint::parse(name).unwrap(), fp);
        }
        assert!(ClientFingerprint::parse("netscape").is_err());
    }

    #[test]
    fn fixed_fingerprints_preserve_the_full_suite_set() {
        // Reshaping only reorders; the advertised suite *set* must equal the
        // ring default so interop is never reduced.
        let default_set = sorted_ids(ring::ALL_CIPHER_SUITES);
        for &fp in FIXED {
            let order = fp.fixed_cipher_order().expect("fixed order");
            assert_eq!(sorted_ids(order), default_set, "{fp:?} dropped/added a suite");
        }
    }

    #[test]
    fn chromium_family_shares_chrome_order() {
        let chrome = ids(ClientFingerprint::Chrome.fixed_cipher_order().unwrap());
        for fp in [
            ClientFingerprint::Edge,
            ClientFingerprint::Qq,
            ClientFingerprint::Android,
        ] {
            assert_eq!(ids(fp.fixed_cipher_order().unwrap()), chrome, "{fp:?}");
        }
        assert_eq!(
            ids(ClientFingerprint::Ios.fixed_cipher_order().unwrap()),
            ids(ClientFingerprint::Safari.fixed_cipher_order().unwrap()),
        );
    }

    #[test]
    fn distinct_browser_families_have_distinct_orders() {
        let chrome = ids(ClientFingerprint::Chrome.fixed_cipher_order().unwrap());
        let firefox = ids(ClientFingerprint::Firefox.fixed_cipher_order().unwrap());
        let safari = ids(ClientFingerprint::Safari.fixed_cipher_order().unwrap());
        assert_ne!(chrome, firefox);
        assert_ne!(chrome, safari);
        assert_ne!(firefox, safari);
        // The leading TLS 1.3 suite is the most visible difference.
        assert_eq!(chrome[0], u16::from(cipher_suite::TLS13_AES_128_GCM_SHA256.suite()));
        assert_eq!(firefox[0], u16::from(cipher_suite::TLS13_AES_128_GCM_SHA256.suite()));
        assert_eq!(safari[0], u16::from(cipher_suite::TLS13_AES_256_GCM_SHA384.suite()));
    }

    #[test]
    fn random_returns_one_of_the_known_browser_orders() {
        let rng = rng();
        let known = [
            ids(CHROME_CIPHER_ORDER),
            ids(FIREFOX_CIPHER_ORDER),
            ids(SAFARI_CIPHER_ORDER),
        ];
        for _ in 0..32 {
            let got = ids(&ClientFingerprint::Random.cipher_suites(rng));
            assert!(known.contains(&got), "random produced an unknown order: {got:?}");
        }
    }

    #[test]
    fn randomized_keeps_the_full_set() {
        let rng = rng();
        let default_set = sorted_ids(ring::ALL_CIPHER_SUITES);
        for _ in 0..32 {
            let got = ClientFingerprint::Randomized.cipher_suites(rng);
            assert_eq!(sorted_ids(&got), default_set);
        }
    }

    #[test]
    fn crypto_provider_reshapes_order_without_changing_the_set() {
        let default_ids = ids(&ring::default_provider().cipher_suites);
        let chrome = ClientFingerprint::Chrome.crypto_provider();
        assert_eq!(
            sorted_ids(&chrome.cipher_suites),
            sorted_ids(&ring::default_provider().cipher_suites)
        );
        // Chrome leads with AES-128 while the ring default leads with AES-256,
        // so the provider order is actually reshaped.
        assert_ne!(ids(&chrome.cipher_suites), default_ids);
        assert_eq!(ids(&chrome.cipher_suites), ids(CHROME_CIPHER_ORDER));
    }
}
