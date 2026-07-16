//! Per-server pool of idle reusable Snell sessions (v2 shadowaead and v4/v5
//! frame sessions), enabling sequential session reuse across logical streams.

use std::collections::HashMap;
use std::sync::Mutex as StdMutex;
use std::time::{Duration, Instant};

use crate::outbound::BoxedStream;

use super::crypto::{AeadCipher, SnellCipher};
use super::{SnellObfs, SnellOutboundConfig};

/// How long an idle reusable Snell session stays in the pool before it is
/// evicted (and its TCP closed) on the next access. Matches mihomo's snell pool
/// connection age (15s).
pub(super) const SESSION_IDLE_TTL: Duration = Duration::from_secs(15);
/// Cap on idle reusable sessions kept per server endpoint, bounding fd/memory
/// use (mihomo's snell pool size).
const SESSION_POOL_MAX: usize = 10;

/// Identifies a Snell server endpoint for the reuse pool. A session is only
/// reusable for an identical endpoint *and* crypto/transport config (version,
/// psk and obfs all change the bytes on the wire), so all are part of the key.
#[derive(Clone, PartialEq, Eq, Hash)]
pub(super) struct SnellServerKey {
    pub(super) server: String,
    pub(super) port: u16,
    pub(super) version: u8,
    pub(super) psk: Vec<u8>,
    pub(super) obfs: Option<SnellObfs>,
}

impl SnellServerKey {
    pub(super) fn from_config(config: &SnellOutboundConfig) -> Self {
        Self {
            server: config.server.clone(),
            port: config.port,
            version: config.version,
            psk: config.psk.clone(),
            obfs: config.obfs.clone(),
        }
    }
}

/// A live v1-v3 shadowaead session parked for sequential reuse: the established
/// transport plus the *continuous* shadowaead state (both ciphers and their
/// counter nonces keep advancing across logical streams). `read_cipher` is
/// always set because a session is only parked after its first stream consumed
/// the server salt.
pub(super) struct PooledSnell {
    pub(super) inner: BoxedStream,
    pub(super) cipher: SnellCipher,
    pub(super) psk: Vec<u8>,
    pub(super) write_cipher: AeadCipher,
    pub(super) write_nonce: [u8; 12],
    pub(super) read_cipher: AeadCipher,
    pub(super) read_nonce: [u8; 12],
    pub(super) idle_since: Instant,
}

/// A live v4/v5 frame session parked for sequential reuse: the established
/// transport plus the continuous v4 frame state. The salt was already sent /
/// consumed on the first stream, so reads resume at a frame-header boundary and
/// no further salt or initial padding is emitted. v4 is always AES-128-GCM, so
/// (unlike [`PooledSnell`]) the cipher family is implied.
pub(super) struct PooledSnellV4 {
    pub(super) inner: BoxedStream,
    pub(super) psk: Vec<u8>,
    pub(super) write_cipher: AeadCipher,
    pub(super) write_nonce: [u8; 12],
    pub(super) read_cipher: AeadCipher,
    pub(super) read_nonce: [u8; 12],
    pub(super) idle_since: Instant,
}

/// A pooled idle session: a v1-v3 shadowaead session or a v4/v5 frame session.
/// The server key embeds the protocol version, so the two never share a bucket.
pub(super) enum PooledSession {
    Shadowaead(PooledSnell),
    V4(PooledSnellV4),
}

impl PooledSession {
    fn idle_since(&self) -> Instant {
        match self {
            Self::Shadowaead(s) => s.idle_since,
            Self::V4(s) => s.idle_since,
        }
    }
}

/// Process-wide pool of idle reusable sessions, keyed by server endpoint, using
/// the same lazily-initialised `Mutex<Option<HashMap>>` idiom as the AnyTLS
/// session registry.
pub(super) static SESSION_POOL: StdMutex<Option<HashMap<SnellServerKey, Vec<PooledSession>>>> = StdMutex::new(None);

/// Take a still-fresh idle session for `key`, dropping any that have outlived
/// [`SESSION_IDLE_TTL`]. `None` means a new connection must be dialled.
pub(super) fn pool_take(key: &SnellServerKey) -> Option<PooledSession> {
    let mut guard = SESSION_POOL.lock().expect("snell session pool");
    let map = guard.as_mut()?;
    let list = map.get_mut(key)?;
    list.retain(|s| s.idle_since().elapsed() <= SESSION_IDLE_TTL);
    let taken = list.pop();
    if list.is_empty() {
        map.remove(key);
    }
    taken
}

/// Park a cleanly half-closed session for later reuse, bounded by
/// [`SESSION_POOL_MAX`]; over-capacity sessions are dropped (TCP closed).
pub(super) fn pool_put(key: SnellServerKey, session: PooledSession) {
    let mut guard = SESSION_POOL.lock().expect("snell session pool");
    let map = guard.get_or_insert_with(HashMap::new);
    let list = map.entry(key).or_default();
    list.retain(|s| s.idle_since().elapsed() <= SESSION_IDLE_TTL);
    if list.len() < SESSION_POOL_MAX {
        list.push(session);
    }
}
