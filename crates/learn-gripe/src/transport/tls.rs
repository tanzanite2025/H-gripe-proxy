//! TLS client transport for outbound protocols.
//!
//! Uses `rustls` (ring crypto provider) — TLS and cryptography are exactly the
//! kind of "do not hand-roll" surface called out in the kernel build-vs-adopt
//! boundary, so learn-gripe stands on a vetted implementation here.

use std::sync::Arc;

use anyhow::{Context, Result, anyhow, bail};
use rustls::client::danger::{HandshakeSignatureValid, ServerCertVerified, ServerCertVerifier};
use rustls::client::{EchConfig, EchMode, RealityConfig};
use rustls::crypto::ring::cipher_suite;
use rustls::crypto::{CryptoProvider, SecureRandom, ring, verify_tls12_signature, verify_tls13_signature};
use rustls::pki_types::{CertificateDer, EchConfigListBytes, ServerName, UnixTime};
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
    /// Encrypted Client Hello (ECH) configuration list (`ech-opts.config`),
    /// already base64-decoded to the raw `ECHConfigList` bytes. When set, the
    /// real SNI is encrypted under one of these configs and the outer
    /// ClientHello advertises only the config's public name; ECH also forces
    /// TLS 1.3. `None` leaves ECH off.
    pub ech_config_list: Option<Vec<u8>>,
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
/// The fingerprint also shapes the ClientHello's **TLS extension ordering** —
/// the other JA3 list field — via the vendored fork's per-connection
/// `extension_order_seed` (see [`ClientFingerprint::extension_order_seed`]).
/// Non-randomizing browsers (Firefox, Safari/iOS) pin a stable seed so their
/// extension order stays fixed connection-to-connection, matching real
/// behaviour; Chromium-family fingerprints leave the seed unset so rustls keeps
/// reshuffling the order on every ClientHello, which is exactly what modern
/// Chrome/Edge do.
///
/// NOTE: GREASE values and record/extension padding are still not shaped, so
/// byte-perfect per-browser mimicry remains follow-up work. The supported-groups
/// order already matches every modern browser (`x25519`, `secp256r1`,
/// `secp384r1`) so it is left at the provider default.
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

    /// The clash/mihomo label for this fingerprint — the inverse of
    /// [`ClientFingerprint::parse`]. Used to report the active fingerprint in
    /// the obfuscation stats.
    pub fn label(self) -> &'static str {
        match self {
            Self::Chrome => "chrome",
            Self::Firefox => "firefox",
            Self::Safari => "safari",
            Self::Ios => "ios",
            Self::Android => "android",
            Self::Edge => "edge",
            Self::Qq => "qq",
            Self::Random => "random",
            Self::Randomized => "randomized",
        }
    }

    /// Whether this fingerprint re-selects or shuffles its ClientHello cipher
    /// order on every dial (`random` picks one real browser order per
    /// connection, `randomized` shuffles the full suite set), i.e. the TLS
    /// fingerprint rotates connection-to-connection instead of staying fixed.
    pub fn rotates_per_dial(self) -> bool {
        matches!(self, Self::Random | Self::Randomized)
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

    /// The fixed ClientHello extension-ordering seed this fingerprint pins, or
    /// `None` to let the vendored rustls fork keep drawing a fresh random seed
    /// per ClientHello (so the extension order reshuffles connection-to-
    /// connection).
    ///
    /// This maps to how real browsers behave: Firefox and Safari/iOS emit a
    /// stable extension order, so they pin a fixed (per-family) seed; modern
    /// Chromium browsers (Chrome/Edge/QQ/Android WebView) deliberately shuffle
    /// their extension order on every ClientHello, so they — like the explicit
    /// `random`/`randomized` fingerprints — keep the randomized default. The
    /// concrete seed values are arbitrary but stable: they only need to be
    /// fixed and distinct per family so each browser presents a consistent,
    /// recognisable extension order rather than a per-connection random one.
    pub fn extension_order_seed(self) -> Option<u16> {
        match self {
            // Chromium-family browsers randomize extension order per ClientHello.
            Self::Chrome | Self::Edge | Self::Qq | Self::Android => None,
            Self::Firefox => Some(FIREFOX_EXTENSION_ORDER_SEED),
            Self::Safari | Self::Ios => Some(SAFARI_EXTENSION_ORDER_SEED),
            // Explicitly rotating fingerprints keep rustls's per-dial randomization.
            Self::Random | Self::Randomized => None,
        }
    }
}

/// Stable ClientHello extension-ordering seed for the Firefox fingerprint.
/// Arbitrary but fixed so Firefox presents a consistent extension order.
const FIREFOX_EXTENSION_ORDER_SEED: u16 = 0xF1F0;

/// Stable ClientHello extension-ordering seed for the Safari / iOS fingerprint.
/// Arbitrary but fixed, and distinct from Firefox's, so Safari presents its own
/// consistent extension order.
const SAFARI_EXTENSION_ORDER_SEED: u16 = 0x5AF1;

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

    // ECH (when configured) selects TLS 1.3 and supplies an HPKE-sealed inner
    // ClientHello; otherwise offer the default protocol versions.
    let versions = rustls::ClientConfig::builder_with_provider(provider.clone());
    let verifier_stage = match build_ech_mode(config)? {
        Some(mode) => versions.with_ech(mode).context("configure ECH")?,
        None => versions
            .with_safe_default_protocol_versions()
            .context("configure TLS protocol versions")?,
    };

    let mut client_config = if config.skip_cert_verify {
        verifier_stage
            .dangerous()
            .with_custom_certificate_verifier(Arc::new(NoCertVerification(provider)))
            .with_no_client_auth()
    } else {
        let mut roots = RootCertStore::empty();
        roots.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());
        verifier_stage.with_root_certificates(roots).with_no_client_auth()
    };

    client_config.alpn_protocols = config.alpn.iter().map(|p| p.as_bytes().to_vec()).collect();
    client_config.extension_order_seed = config
        .client_fingerprint
        .and_then(ClientFingerprint::extension_order_seed);

    let sni = config
        .server_name
        .as_deref()
        .filter(|s| !s.is_empty())
        .unwrap_or(dial_host);
    let server_name =
        ServerName::try_from(sni.to_owned()).with_context(|| format!("invalid TLS server name {sni:?}"))?;

    let connector = TlsConnector::from(Arc::new(client_config));
    let tls = connector
        .connect(server_name, stream)
        .await
        .with_context(|| format!("TLS handshake with {sni}"))?;
    if let Some(fp) = config.client_fingerprint {
        crate::transport::obfuscation::record_shaped_handshake(fp);
    }
    Ok(tls)
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
    client_config.extension_order_seed = config
        .client_fingerprint
        .and_then(ClientFingerprint::extension_order_seed);

    let sni = config.server_name.as_str();
    if sni.is_empty() {
        bail!("REALITY requires a non-empty servername for SNI masquerade");
    }
    let server_name =
        ServerName::try_from(sni.to_owned()).with_context(|| format!("invalid REALITY server name {sni:?}"))?;

    let connector = TlsConnector::from(Arc::new(client_config));
    let tls = connector
        .connect(server_name, stream)
        .await
        .with_context(|| format!("REALITY handshake (masquerade SNI {sni}, dial {dial_host})"))?;
    if let Some(fp) = config.client_fingerprint {
        crate::transport::obfuscation::record_shaped_handshake(fp);
    }
    Ok(tls)
}

/// Build the rustls [`EchMode`] for a connection, if `ech_config_list` is set.
/// The raw `ECHConfigList` is parsed and matched against the kernel's HPKE
/// provider ([`crate::transport::hpke`]); an incompatible or malformed list is
/// rejected rather than silently falling back to a cleartext SNI.
fn build_ech_mode(config: &TlsClientConfig) -> Result<Option<EchMode>> {
    let Some(bytes) = &config.ech_config_list else {
        return Ok(None);
    };
    let list = EchConfigListBytes::from(bytes.clone());
    let ech = EchConfig::new(list, crate::transport::hpke::ALL_SUPPORTED_SUITES)
        .map_err(|e| anyhow!("ECH: no usable config in ech-opts.config ({e})"))?;
    Ok(Some(EchMode::Enable(ech)))
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

    #[test]
    fn stable_browsers_pin_a_deterministic_extension_seed() {
        // Firefox and Safari/iOS emit a stable extension order, so they pin a
        // fixed seed; the value must be deterministic across calls.
        for fp in [
            ClientFingerprint::Firefox,
            ClientFingerprint::Safari,
            ClientFingerprint::Ios,
        ] {
            let seed = fp.extension_order_seed();
            assert!(seed.is_some(), "{fp:?} should pin a seed");
            assert_eq!(seed, fp.extension_order_seed(), "{fp:?} seed not deterministic");
        }
        // iOS reuses the desktop Safari extension order.
        assert_eq!(
            ClientFingerprint::Ios.extension_order_seed(),
            ClientFingerprint::Safari.extension_order_seed(),
        );
    }

    #[test]
    fn distinct_stable_families_have_distinct_extension_seeds() {
        assert_ne!(
            ClientFingerprint::Firefox.extension_order_seed(),
            ClientFingerprint::Safari.extension_order_seed(),
        );
    }

    #[test]
    fn randomizing_fingerprints_leave_the_extension_seed_unset() {
        // Chromium-family browsers reshuffle extension order per ClientHello, as
        // do the explicit random/randomized fingerprints — all keep rustls's
        // per-connection randomized seed (None).
        for fp in [
            ClientFingerprint::Chrome,
            ClientFingerprint::Edge,
            ClientFingerprint::Qq,
            ClientFingerprint::Android,
            ClientFingerprint::Random,
            ClientFingerprint::Randomized,
        ] {
            assert_eq!(fp.extension_order_seed(), None, "{fp:?} should not pin a seed");
        }
    }
}

/// End-to-end wiring tests for Encrypted Client Hello: a real rustls handshake
/// is driven through an in-memory pipe and the outbound ClientHello is parsed to
/// prove the true SNI is hidden behind the ECH public name.
#[cfg(test)]
mod ech_tests {
    use tokio::io::{AsyncReadExt, duplex};

    use super::*;
    use crate::transport::hpke;

    const KEM_X25519: u16 = 0x0020;
    const KDF_SHA256: u16 = 0x0001;
    const AEAD_AES128: u16 = 0x0001;
    const ECH_VERSION_V18: u16 = 0xfe0d;
    const EXT_SNI: u16 = 0x0000;
    const EXT_ECH: u16 = 0xfe0d;

    /// Encode a single-config `ECHConfigList` (draft-ietf-tls-esni-18 wire
    /// format) advertising DHKEM(X25519)/HKDF-SHA256/AES-128-GCM and the given
    /// HPKE public key + public name.
    fn ech_config_list(public_key: &[u8], public_name: &str) -> Vec<u8> {
        let mut suites = Vec::new();
        suites.extend_from_slice(&KDF_SHA256.to_be_bytes());
        suites.extend_from_slice(&AEAD_AES128.to_be_bytes());

        let mut contents = Vec::new();
        contents.push(0x2a); // config_id
        contents.extend_from_slice(&KEM_X25519.to_be_bytes());
        contents.extend_from_slice(&(public_key.len() as u16).to_be_bytes());
        contents.extend_from_slice(public_key);
        contents.extend_from_slice(&(suites.len() as u16).to_be_bytes());
        contents.extend_from_slice(&suites);
        contents.push(0); // maximum_name_length
        contents.push(public_name.len() as u8);
        contents.extend_from_slice(public_name.as_bytes());
        contents.extend_from_slice(&0u16.to_be_bytes()); // no extensions

        let mut config = Vec::new();
        config.extend_from_slice(&ECH_VERSION_V18.to_be_bytes());
        config.extend_from_slice(&(contents.len() as u16).to_be_bytes());
        config.extend_from_slice(&contents);

        let mut list = Vec::new();
        list.extend_from_slice(&(config.len() as u16).to_be_bytes());
        list.extend_from_slice(&config);
        list
    }

    /// Extract `(outer SNI host, ECH extension present)` from a ClientHello
    /// handshake message (RFC 8446 §4.1.2).
    fn parse_client_hello(msg: &[u8]) -> (Option<String>, bool) {
        assert_eq!(msg[0], 0x01, "expected ClientHello handshake message");
        let mut p = 4; // skip msg_type(1) + length(3)
        p += 2 + 32; // legacy_version + random
        p += 1 + msg[p] as usize; // legacy_session_id
        let cs_len = u16::from_be_bytes([msg[p], msg[p + 1]]) as usize;
        p += 2 + cs_len; // cipher_suites
        p += 1 + msg[p] as usize; // legacy_compression_methods
        let ext_total = u16::from_be_bytes([msg[p], msg[p + 1]]) as usize;
        p += 2;
        let end = p + ext_total;

        let mut sni = None;
        let mut has_ech = false;
        while p + 4 <= end {
            let ext_type = u16::from_be_bytes([msg[p], msg[p + 1]]);
            let ext_len = u16::from_be_bytes([msg[p + 2], msg[p + 3]]) as usize;
            let data = &msg[p + 4..p + 4 + ext_len];
            p += 4 + ext_len;
            match ext_type {
                EXT_SNI => {
                    // ServerNameList: u16 list len, then {name_type u8, HostName u16}.
                    let mut q = 2;
                    while q + 3 <= data.len() {
                        let name_type = data[q];
                        let nlen = u16::from_be_bytes([data[q + 1], data[q + 2]]) as usize;
                        let name = &data[q + 3..q + 3 + nlen];
                        if name_type == 0 {
                            sni = Some(String::from_utf8_lossy(name).into_owned());
                        }
                        q += 3 + nlen;
                    }
                }
                EXT_ECH => has_ech = true,
                _ => {}
            }
        }
        (sni, has_ech)
    }

    /// Read the first TLS handshake record and return its fragment (the
    /// ClientHello message).
    async fn read_client_hello<R: AsyncReadExt + Unpin>(reader: &mut R) -> Vec<u8> {
        let mut header = [0u8; 5];
        reader.read_exact(&mut header).await.unwrap();
        assert_eq!(header[0], 0x16, "first record must be a handshake record");
        let len = u16::from_be_bytes([header[3], header[4]]) as usize;
        let mut body = vec![0u8; len];
        reader.read_exact(&mut body).await.unwrap();
        body
    }

    #[test]
    fn build_ech_mode_accepts_a_valid_config_and_rejects_garbage() {
        let (pk, _sk) = hpke::ALL_SUPPORTED_SUITES[0].generate_key_pair().unwrap();
        let list = ech_config_list(&pk.0, "public.example");

        let ok = TlsClientConfig {
            ech_config_list: Some(list),
            ..Default::default()
        };
        assert!(matches!(build_ech_mode(&ok), Ok(Some(_))));

        let none = TlsClientConfig::default();
        assert!(matches!(build_ech_mode(&none), Ok(None)));

        let bad = TlsClientConfig {
            ech_config_list: Some(vec![0xff, 0xff, 0xff]),
            ..Default::default()
        };
        assert!(build_ech_mode(&bad).is_err());
    }

    #[tokio::test]
    async fn ech_client_hello_hides_the_real_sni_behind_the_public_name() {
        let (pk, _sk) = hpke::ALL_SUPPORTED_SUITES[0].generate_key_pair().unwrap();
        let list = ech_config_list(&pk.0, "public.example");

        let config = TlsClientConfig {
            server_name: Some("secret.example".to_string()),
            skip_cert_verify: true,
            ech_config_list: Some(list),
            ..Default::default()
        };

        let (client_io, mut server_io) = duplex(16 * 1024);
        // Drive the client handshake; it will block reading our (absent) reply,
        // so run it detached and just inspect the ClientHello it emits.
        let client = tokio::spawn(async move {
            let _ = connect(&config, "secret.example", client_io).await;
        });

        let msg = read_client_hello(&mut server_io).await;
        let (outer_sni, has_ech) = parse_client_hello(&msg);

        assert!(has_ech, "ClientHello must carry the encrypted_client_hello extension");
        assert_eq!(
            outer_sni.as_deref(),
            Some("public.example"),
            "outer SNI must be the ECH public name, not the protected server name"
        );
        assert!(
            !msg.windows(b"secret.example".len()).any(|w| w == b"secret.example"),
            "the protected server name must not appear in cleartext in the ClientHello"
        );

        drop(server_io);
        client.abort();
    }
}
