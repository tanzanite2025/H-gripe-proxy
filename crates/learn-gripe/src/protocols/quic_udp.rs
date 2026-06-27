//! Shared fragmentation/reassembly for QUIC-datagram UDP relays.
//!
//! Both Hysteria2 and TUIC carry UDP datagrams over QUIC datagram frames, and
//! both fragment a UDP payload that does not fit in a single QUIC datagram into
//! several frames that share a packet id and carry a `(fragment id, fragment
//! count)` pair. The wire headers differ per protocol (and live in each
//! protocol module), but the *fragmentation maths* on send and the *reassembly*
//! on receive are identical, so they live here.
//!
//! Reassembly is best-effort, matching UDP's lossy semantics: a packet whose
//! fragments never all arrive is eventually evicted to bound memory, and a
//! malformed fragment count simply yields nothing.

use std::collections::HashMap;

/// Cap on simultaneously in-flight (partially received) fragmented packets.
/// Beyond this we drop the oldest incomplete packet — UDP is lossy, so a stuck
/// reassembly must never pin memory.
const MAX_PENDING_PACKETS: usize = 64;

/// Split `payload` into `chunk_size`-sized fragments. Always yields at least one
/// fragment, so a zero-length payload produces a single empty fragment (an empty
/// UDP datagram is valid). `chunk_size` must be non-zero. The fragment count is
/// `Vec::len`, which the caller checks fits in the protocol's `u8` count field.
pub(crate) fn fragments(payload: &[u8], chunk_size: usize) -> Vec<&[u8]> {
    debug_assert!(chunk_size > 0, "chunk size must be non-zero");
    if payload.is_empty() {
        return vec![&[]];
    }
    payload.chunks(chunk_size).collect()
}

/// Accumulates inbound fragments until each packet is whole.
pub(crate) struct Reassembler {
    pending: HashMap<u16, Pending>,
    /// Monotonic counter used to evict the oldest pending packet when full.
    tick: u64,
}

struct Pending {
    fragments: Vec<Option<Vec<u8>>>,
    remaining: usize,
    seen_at: u64,
}

impl Reassembler {
    pub(crate) fn new() -> Self {
        Self {
            pending: HashMap::new(),
            tick: 0,
        }
    }

    /// Feed one fragment. Returns the fully reassembled payload once every
    /// fragment of its packet has arrived, or `None` while the packet is still
    /// incomplete (or the fragment was malformed/duplicate).
    pub(crate) fn accept(&mut self, packet_id: u16, frag_id: u8, frag_count: u8, payload: Vec<u8>) -> Option<Vec<u8>> {
        // The common, unfragmented case: deliver immediately without bookkeeping.
        if frag_count <= 1 {
            return Some(payload);
        }
        if frag_id >= frag_count {
            return None;
        }

        self.tick += 1;
        let now = self.tick;
        let count = frag_count as usize;
        let entry = self.pending.entry(packet_id).or_insert_with(|| Pending {
            fragments: vec![None; count],
            remaining: count,
            seen_at: now,
        });
        // A reused packet id with a different fragment count is a fresh packet.
        if entry.fragments.len() != count {
            *entry = Pending {
                fragments: vec![None; count],
                remaining: count,
                seen_at: now,
            };
        }
        entry.seen_at = now;

        let slot = &mut entry.fragments[frag_id as usize];
        if slot.is_none() {
            *slot = Some(payload);
            entry.remaining -= 1;
        }

        if entry.remaining == 0 {
            let done = self.pending.remove(&packet_id)?;
            let mut out = Vec::new();
            for frag in done.fragments.into_iter().flatten() {
                out.extend_from_slice(&frag);
            }
            return Some(out);
        }

        self.evict_if_full();
        None
    }

    /// Keep the pending set bounded by dropping the oldest incomplete packet.
    fn evict_if_full(&mut self) {
        if self.pending.len() <= MAX_PENDING_PACKETS {
            return;
        }
        if let Some(oldest) = self.pending.iter().min_by_key(|(_, p)| p.seen_at).map(|(id, _)| *id) {
            self.pending.remove(&oldest);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn single_fragment_passes_through() {
        let mut r = Reassembler::new();
        assert_eq!(r.accept(7, 0, 1, b"hello".to_vec()), Some(b"hello".to_vec()));
    }

    #[test]
    fn reassembles_in_order_regardless_of_arrival() {
        let mut r = Reassembler::new();
        // Deliver fragment 1 before fragment 0.
        assert_eq!(r.accept(9, 1, 2, b"world".to_vec()), None);
        assert_eq!(r.accept(9, 0, 2, b"hello ".to_vec()), Some(b"hello world".to_vec()));
    }

    #[test]
    fn three_way_split_reassembles() {
        let mut r = Reassembler::new();
        assert_eq!(r.accept(1, 0, 3, b"aaa".to_vec()), None);
        assert_eq!(r.accept(1, 2, 3, b"ccc".to_vec()), None);
        assert_eq!(r.accept(1, 1, 3, b"bbb".to_vec()), Some(b"aaabbbccc".to_vec()));
    }

    #[test]
    fn out_of_range_fragment_is_ignored() {
        let mut r = Reassembler::new();
        assert_eq!(r.accept(1, 5, 3, b"x".to_vec()), None);
    }

    #[test]
    fn fragments_helper_splits_and_counts() {
        let chunks = fragments(b"abcde", 2);
        assert_eq!(chunks, vec![&b"ab"[..], &b"cd"[..], &b"e"[..]]);

        // A zero-length payload still yields one (empty) fragment.
        let chunks = fragments(b"", 4);
        assert_eq!(chunks, vec![&b""[..]]);
    }
}
