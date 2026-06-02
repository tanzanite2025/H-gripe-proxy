use crate::{
    config::AdvancedConfig,
    tls_fingerprint::{TlsFingerprint, TlsFingerprintLibrary},
};
use anyhow::Result;

pub fn tls_fingerprint_get_all() -> Vec<TlsFingerprint> {
    TlsFingerprintLibrary::get_all()
}

pub fn tls_fingerprint_get_by_name(name: &str) -> Option<TlsFingerprint> {
    TlsFingerprintLibrary::get_by_name(name)
}

pub fn tls_fingerprint_get_current() -> Option<TlsFingerprint> {
    let coordinator = crate::feat::get_coordinator();
    coordinator
        .get_advanced_config()
        .security
        .tls_fingerprint
        .as_deref()
        .and_then(TlsFingerprintLibrary::get_by_name)
}

pub fn tls_fingerprint_generate_config() -> Option<serde_json::Value> {
    tls_fingerprint_get_current().map(|fp| {
        serde_json::json!({
            "global-client-fingerprint": fp.name,
        })
    })
}

pub fn tls_fingerprint_clear() -> Result<()> {
    let mut advanced = AdvancedConfig::load_default();
    advanced.security.tls_fingerprint = None;
    advanced.validate()?;
    advanced.save_default()?;
    crate::feat::get_coordinator().apply_advanced_config(&advanced)?;
    Ok(())
}
