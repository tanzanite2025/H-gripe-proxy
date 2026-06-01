use crate::tls_fingerprint::{TlsFingerprint, TlsFingerprintLibrary};

pub fn tls_fingerprint_get_all() -> Vec<TlsFingerprint> {
    TlsFingerprintLibrary::get_all()
}

pub fn tls_fingerprint_get_by_name(name: &str) -> Option<TlsFingerprint> {
    TlsFingerprintLibrary::get_by_name(name)
}

pub fn tls_fingerprint_get_current() -> Option<TlsFingerprint> {
    let coordinator = crate::feat::get_coordinator();
    let service = coordinator.tls_fingerprint();
    service.get_fingerprint()
}

pub fn tls_fingerprint_generate_config() -> Option<serde_json::Value> {
    let coordinator = crate::feat::get_coordinator();
    let service = coordinator.tls_fingerprint();
    service.generate_clash_config()
}

pub fn tls_fingerprint_clear() {
    let coordinator = crate::feat::get_coordinator();
    let service = coordinator.tls_fingerprint();
    service.clear();
}
