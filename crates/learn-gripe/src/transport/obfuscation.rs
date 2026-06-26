//! In-process client-side obfuscation statistics.
//!
//! learn-gripe's only outbound traffic obfuscation is TLS ClientHello
//! fingerprint shaping (see [`crate::transport::tls::ClientFingerprint`]): an outbound
//! whose proxy carries a `client-fingerprint` reshapes its ClientHello
//! cipher-suite ordering to mimic a real browser. This module counts those
//! shaped handshakes in process, replacing the Mihomo controller
//! `/engine/obfuscation/stats` query that polled the external Go kernel.
//!
//! TLS handshakes happen deep in the dial pipeline (`tls::connect`), where no
//! per-kernel handle is in scope, so the counters are process-global — the same
//! shape as the in-process core-log tap. [`reset`] clears them and the kernel
//! resets them on start so the figures track the current run.
//!
//! The kernel performs no payload padding and never re-keys a live TLS session,
//! so the legacy padding-byte and mid-connection-rotation counters the Go
//! kernel exposed have no in-process equivalent; the bridge reports them as
//! zero. The `random` / `randomized` fingerprints — which re-select the
//! ClientHello cipher order on every dial — are counted as fingerprint
//! rotations, and an explicit operator-requested rotation ([`force_rotation`])
//! is counted alongside them.

use std::collections::HashMap;
use std::sync::Mutex;
use std::sync::atomic::{AtomicU64, Ordering};

use crate::transport::tls::ClientFingerprint;

/// A point-in-time snapshot of the process-global obfuscation counters.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ObfuscationSnapshot {
    /// Outbound handshakes whose ClientHello was fingerprint-shaped, cumulative
    /// over the current kernel run.
    pub total_obfuscated_conns: u64,
    /// Subset of shaped handshakes whose fingerprint re-selects or shuffles the
    /// ClientHello cipher order on every dial (`random` / `randomized`).
    pub tls_rotation_count: u64,
    /// Label of the most recently applied client-fingerprint (e.g. `"chrome"`),
    /// or empty when no shaped handshake has occurred.
    pub current_tls_fingerprint: String,
    /// Per-fingerprint shaped-handshake counts, keyed by clash/mihomo label
    /// (e.g. `"chrome"`). Empty until the first shaped handshake.
    pub fingerprint_usage: HashMap<String, u64>,
}

static TOTAL: AtomicU64 = AtomicU64::new(0);
static ROTATIONS: AtomicU64 = AtomicU64::new(0);
static CURRENT_FP: Mutex<String> = Mutex::new(String::new());
static USAGE: Mutex<Option<HashMap<String, u64>>> = Mutex::new(None);

/// Record that an outbound TLS / REALITY handshake shaped its ClientHello to
/// mimic `fingerprint`. Called from the TLS dial path on a successful
/// handshake.
pub(crate) fn record_shaped_handshake(fingerprint: ClientFingerprint) {
    TOTAL.fetch_add(1, Ordering::Relaxed);
    if fingerprint.rotates_per_dial() {
        ROTATIONS.fetch_add(1, Ordering::Relaxed);
    }
    if let Ok(mut current) = CURRENT_FP.lock() {
        current.clear();
        current.push_str(fingerprint.label());
    }
    if let Ok(mut usage) = USAGE.lock() {
        *usage
            .get_or_insert_with(HashMap::new)
            .entry(fingerprint.label().to_string())
            .or_insert(0) += 1;
    }
}

/// Snapshot the current counters.
pub fn snapshot() -> ObfuscationSnapshot {
    ObfuscationSnapshot {
        total_obfuscated_conns: TOTAL.load(Ordering::Relaxed),
        tls_rotation_count: ROTATIONS.load(Ordering::Relaxed),
        current_tls_fingerprint: CURRENT_FP.lock().map(|fp| fp.clone()).unwrap_or_default(),
        fingerprint_usage: USAGE.lock().ok().and_then(|u| u.clone()).unwrap_or_default(),
    }
}

/// Record an operator-requested TLS fingerprint rotation and report the
/// fingerprint that is currently active.
///
/// learn-gripe re-rolls the `random` / `randomized` ClientHello cipher order on
/// every dial and pins concrete fingerprints to per-proxy `client-fingerprint`
/// config, so there is no global live fingerprint to re-key mid-session: a
/// forced rotation has no on-the-wire effect. It is recorded as a rotation
/// event (bumping [`ObfuscationSnapshot::tls_rotation_count`]) for telemetry
/// parity with the former Mihomo controller, and returns the most recently
/// applied fingerprint label (empty when no shaped handshake has occurred).
/// Process-wide, so it does not require a running kernel.
pub fn force_rotation() -> String {
    ROTATIONS.fetch_add(1, Ordering::Relaxed);
    CURRENT_FP.lock().map(|fp| fp.clone()).unwrap_or_default()
}

/// Reset every counter to zero and clear the recorded fingerprint. Process-wide,
/// so it does not require a running kernel.
pub fn reset() {
    TOTAL.store(0, Ordering::Relaxed);
    ROTATIONS.store(0, Ordering::Relaxed);
    if let Ok(mut current) = CURRENT_FP.lock() {
        current.clear();
    }
    if let Ok(mut usage) = USAGE.lock() {
        *usage = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Mutex as StdMutex, MutexGuard};

    // The counters are process-global, so the tests that read them must not run
    // concurrently with one another.
    static SERIAL: StdMutex<()> = StdMutex::new(());

    fn serial() -> MutexGuard<'static, ()> {
        SERIAL.lock().unwrap_or_else(|e| e.into_inner())
    }

    #[test]
    fn records_total_rotations_and_current_fingerprint() {
        let _guard = serial();
        reset();

        record_shaped_handshake(ClientFingerprint::Chrome);
        record_shaped_handshake(ClientFingerprint::Firefox);
        record_shaped_handshake(ClientFingerprint::Chrome);
        let snap = snapshot();
        assert_eq!(snap.total_obfuscated_conns, 3);
        assert_eq!(snap.tls_rotation_count, 0, "fixed fingerprints do not rotate");
        assert_eq!(snap.current_tls_fingerprint, "chrome", "tracks the most recent");
        assert_eq!(snap.fingerprint_usage.get("chrome").copied(), Some(2));
        assert_eq!(snap.fingerprint_usage.get("firefox").copied(), Some(1));
    }

    #[test]
    fn force_rotation_counts_and_reports_current_fingerprint() {
        let _guard = serial();
        reset();

        assert_eq!(force_rotation(), "", "no shaped handshake yet");
        assert_eq!(snapshot().tls_rotation_count, 1, "forced rotation is counted");

        record_shaped_handshake(ClientFingerprint::Edge);
        assert_eq!(force_rotation(), "edge", "reports the active fingerprint");
        assert_eq!(snapshot().tls_rotation_count, 2);
    }

    #[test]
    fn random_and_randomized_count_as_rotations() {
        let _guard = serial();
        reset();

        record_shaped_handshake(ClientFingerprint::Random);
        record_shaped_handshake(ClientFingerprint::Randomized);
        record_shaped_handshake(ClientFingerprint::Chrome);
        let snap = snapshot();
        assert_eq!(snap.total_obfuscated_conns, 3);
        assert_eq!(snap.tls_rotation_count, 2, "only random/randomized rotate per dial");
    }

    #[test]
    fn reset_zeroes_every_counter() {
        let _guard = serial();
        reset();

        record_shaped_handshake(ClientFingerprint::Edge);
        reset();
        assert_eq!(snapshot(), ObfuscationSnapshot::default());
    }
}
