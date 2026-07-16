use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, Mutex};

use md5::Md5;
use sha2::Digest;

use super::frame::{FRAME_HEADER_LEN, push_waste};

/// The upstream default padding scheme (anytls-go `proxy/padding/padding.go`).
/// We both advertise its md5 and shape traffic by it. No trailing newline — it
/// must hash identically to the upstream bytes.
pub(super) const DEFAULT_PADDING_SCHEME: &str = "stop=8\n\
0=30-30\n\
1=100-400\n\
2=400-500,c,500-1000,c,500-1000,c,500-1000,c,500-1000\n\
3=9-9,500-1000\n\
4=500-1000\n\
5=500-1000\n\
6=500-1000\n\
7=500-1000";

/// Lowercase-hex md5 of [`DEFAULT_PADDING_SCHEME`] — what we advertise in
/// `cmdSettings`, and the baseline against which a pushed scheme is judged
/// "different" (so a stock server's matching scheme is a no-op).
pub(super) const DEFAULT_PADDING_MD5: &str = "75cff2ad89aadf5e257059ee571ebe11";

/// Sentinel returned by [`PaddingScheme::record_payload_sizes`] for the scheme's
/// `c` token (anytls `padding.CheckMark`): "if the user payload is exhausted,
/// stop emitting padding records for this packet; otherwise carry on".
pub(super) const CHECK_MARK: i64 = -1;

/// One token of a padding-scheme packet entry: either a `min-max` byte-size
/// range or the `c` check mark.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SizeToken {
    /// A `min-max` range; `record_payload_sizes` resolves it to a random size in
    /// `[min, max)` (upper-exclusive, matching anytls `rand.Int(max-min)+min`),
    /// or exactly `min` when `min == max`.
    Range(i64, i64),
    /// The `c` check mark.
    Check,
}

/// A parsed anytls padding scheme (anytls-go `proxy/padding`): the lowercase-hex
/// md5 of the raw bytes (advertised in `cmdSettings`), the `stop` packet count,
/// and the per-packet size-token lists keyed by packet index.
#[derive(Debug, Clone)]
pub(super) struct PaddingScheme {
    pub(super) md5_hex: String,
    pub(super) stop: u32,
    packets: HashMap<u32, Vec<SizeToken>>,
}

impl PaddingScheme {
    /// Parse a raw scheme (`key=value` lines, `\n`-separated, per
    /// `util.StringMapFromBytes`). Returns `None` if there is no usable `stop`
    /// line — matching anytls `NewPaddingFactory`, which rejects such schemes.
    pub(super) fn parse(raw: &[u8]) -> Option<Self> {
        let md5_hex = md5_hex(raw);
        let text = String::from_utf8_lossy(raw);
        let mut stop = None;
        let mut packets: HashMap<u32, Vec<SizeToken>> = HashMap::new();
        for line in text.split('\n') {
            let Some((key, value)) = line.split_once('=') else {
                continue;
            };
            // anytls `util.StringMapFromBytes` trims spaces on both key and
            // value of every line, so a scheme delimited with `\r\n` (trailing
            // `\r`) or written with spaces (`stop = 8`) parses identically.
            let (key, value) = (key.trim(), value.trim());
            if key == "stop" {
                stop = value.parse::<u32>().ok();
            } else if let Ok(pkt) = key.parse::<u32>() {
                let tokens = value
                    .split(',')
                    .filter_map(|tok| match tok {
                        "c" => Some(SizeToken::Check),
                        _ => {
                            let (lo, hi) = tok.split_once('-')?;
                            let (lo, hi) = (lo.parse::<i64>().ok()?, hi.parse::<i64>().ok()?);
                            let (lo, hi) = (lo.min(hi), lo.max(hi));
                            // anytls skips non-positive ranges.
                            (lo > 0 && hi > 0).then_some(SizeToken::Range(lo, hi))
                        }
                    })
                    .collect();
                packets.insert(pkt, tokens);
            }
        }
        stop.map(|stop| Self { md5_hex, stop, packets })
    }

    /// The built-in default scheme (always parses).
    pub(super) fn default_scheme() -> Self {
        Self::parse(DEFAULT_PADDING_SCHEME.as_bytes()).expect("default padding scheme parses")
    }

    /// Resolve packet `pkt`'s tokens to concrete record payload sizes, picking a
    /// fresh random length within each range and mapping `c` to [`CHECK_MARK`].
    /// An undefined packet yields an empty list (anytls sends it unshaped).
    pub(super) fn record_payload_sizes(&self, pkt: u32) -> Vec<i64> {
        let Some(tokens) = self.packets.get(&pkt) else {
            return Vec::new();
        };
        tokens
            .iter()
            .map(|tok| match *tok {
                SizeToken::Check => CHECK_MARK,
                SizeToken::Range(lo, hi) if lo == hi => lo,
                // Upper-exclusive, like anytls `rand.Int(big.NewInt(max-min))+min`.
                SizeToken::Range(lo, hi) => lo + random_below((hi - lo) as u64) as i64,
            })
            .collect()
    }
}

/// Lowercase-hex md5 of `data`.
pub(super) fn md5_hex(data: &[u8]) -> String {
    let digest = Md5::digest(data);
    let mut out = String::with_capacity(32);
    for byte in digest {
        out.push_str(&format!("{byte:02x}"));
    }
    out
}

/// A uniform-ish random integer in `[0, n)` (0 when `n == 0`). Modulo bias is
/// irrelevant for traffic-padding sizes (`n` is at most a few thousand).
fn random_below(n: u64) -> u64 {
    if n == 0 {
        return 0;
    }
    let mut bytes = [0u8; 8];
    getrandom::fill(&mut bytes).expect("os rng");
    u64::from_le_bytes(bytes) % n
}

/// Identifies an AnyTLS server endpoint for the per-server padding-scheme store.
/// A server-pushed `cmdUpdatePaddingScheme` applies only to connections to that
/// same server (anytls-go stores it on the per-server `Client`).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(super) struct ServerKey {
    pub(super) server: String,
    pub(super) port: u16,
}

/// Per-server override schemes learned from `cmdUpdatePaddingScheme`; a missing
/// entry means "use the built-in default". Process-wide because outbound
/// `connect`s are independent calls with no other shared state, matching
/// anytls-go's per-server `Client` storage. `None` is the lazily-initialised
/// empty map (the `static Mutex<Option<HashMap>>` idiom used elsewhere in the
/// crate avoids a separate lazy-init dependency).
static SCHEME_STORE: Mutex<Option<HashMap<ServerKey, Arc<PaddingScheme>>>> = Mutex::new(None);

/// The padding scheme a new connection to `key` should use: the server's pushed
/// scheme if one has been learned, else the built-in default.
pub(super) fn current_scheme(key: &ServerKey) -> Arc<PaddingScheme> {
    let store = SCHEME_STORE.lock().expect("anytls scheme store");
    store
        .as_ref()
        .and_then(|map| map.get(key).cloned())
        .unwrap_or_else(|| Arc::new(PaddingScheme::default_scheme()))
}

/// Apply a server-pushed `cmdUpdatePaddingScheme` for `key`: parse it and, when
/// it is valid and its md5 differs from the scheme currently in effect, store it
/// so subsequent connections to that server advertise and shape by it (anytls-go
/// `UpdatePaddingScheme`). The connection that received it keeps its own scheme.
pub(super) fn apply_scheme_update(key: &ServerKey, raw: &[u8]) {
    let Some(scheme) = PaddingScheme::parse(raw) else {
        return;
    };
    let mut store = SCHEME_STORE.lock().expect("anytls scheme store");
    let map = store.get_or_insert_with(HashMap::new);
    let current_md5 = map.get(key).map_or(DEFAULT_PADDING_MD5, |s| s.md5_hex.as_str());
    if scheme.md5_hex != current_md5 {
        map.insert(key.clone(), Arc::new(scheme));
    }
}

/// Drives the anytls padding scheme over the outgoing frame stream. Each call to
/// [`PaddingShaper::shape`] is one anytls "TLS packet" (`writeConn` flush): the
/// packet counter advances, and while it is below the scheme's `stop` the frame
/// bytes are split into records of the scheme's sizes — emitting `cmdWaste`
/// frames to fill short writes — exactly as anytls-go `Session.writeConn` does.
pub(super) struct PaddingShaper {
    pub(super) scheme: PaddingScheme,
    /// Number of packets (flushes) shaped so far; the next flush is `pkt + 1`.
    pkt: u32,
    /// Cleared once the packet counter reaches `stop`; thereafter frames pass
    /// through unshaped (matching anytls clearing `sendPadding`).
    pub(super) send_padding: bool,
}

impl PaddingShaper {
    pub(super) fn new(scheme: PaddingScheme) -> Self {
        Self {
            scheme,
            pkt: 0,
            send_padding: true,
        }
    }

    /// Shape one `writeConn` unit of complete frame bytes into the record queue
    /// `out`, appending `cmdWaste` padding per the scheme for the current packet.
    pub(super) fn shape(&mut self, out: &mut VecDeque<Vec<u8>>, frame_bytes: Vec<u8>) {
        if self.send_padding {
            self.pkt += 1;
            if self.pkt < self.scheme.stop {
                self.shape_packet(out, frame_bytes);
                return;
            }
            self.send_padding = false;
        }
        out.push_back(frame_bytes);
    }

    /// The padded-packet branch of anytls `writeConn`: walk the scheme's record
    /// sizes, chopping `frame_bytes` into records and inserting `cmdWaste`.
    fn shape_packet(&self, out: &mut VecDeque<Vec<u8>>, frame_bytes: Vec<u8>) {
        let mut pos = 0usize;
        for size in self.scheme.record_payload_sizes(self.pkt) {
            let remain = frame_bytes.len() - pos;
            if size == CHECK_MARK {
                // Stop padding once the payload is drained; else keep going.
                if remain == 0 {
                    break;
                }
                continue;
            }
            let size = size as usize;
            if remain > size {
                // Record is entirely real payload (a prefix; may cut mid-frame,
                // which is transparent once the peer reassembles by length).
                out.push_back(frame_bytes[pos..pos + size].to_vec());
                pos += size;
            } else if remain > 0 {
                // Last of the payload, padded up to `size` with one `cmdWaste`.
                let mut record = frame_bytes[pos..].to_vec();
                pos = frame_bytes.len();
                let pad = size as isize - remain as isize - FRAME_HEADER_LEN as isize;
                if pad > 0 {
                    push_waste(&mut record, pad as usize);
                }
                out.push_back(record);
            } else {
                // Payload exhausted: a standalone `cmdWaste` record of `size`.
                let mut record = Vec::with_capacity(FRAME_HEADER_LEN + size);
                push_waste(&mut record, size);
                out.push_back(record);
            }
        }
        // Any payload the scheme did not cover is sent as a final record.
        if pos < frame_bytes.len() {
            out.push_back(frame_bytes[pos..].to_vec());
        }
    }
}
