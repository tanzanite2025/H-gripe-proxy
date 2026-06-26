//! XTLS Vision (`flow: xtls-rprx-vision`) body framing for VLESS.
//!
//! Vision is *not* a security or transport layer — it is an extra framing
//! applied to the VLESS body, orthogonal to [`crate::transport`]. It only runs
//! over raw-TCP VLESS (the inner relayed bytes must be a clean stream), which
//! [`crate::protocols::vless`] enforces. The point of Vision is to hide the length
//! signature of the *tunneled* TLS handshake: while the inner connection is
//! still handshaking, each application write is wrapped in a small padding frame
//! so the on-wire record sizes don't fingerprint the proxy; once the inner TLS
//! handshake finishes (the first inner `application_data` record is seen) the
//! padding ends and bytes pass straight through.
//!
//! This is a faithful byte-level port of Xray-core's `proxy.VisionWriter` /
//! `proxy.VisionReader` (`XtlsPadding` / `XtlsUnpadding` / `XtlsFilterTls` /
//! `ReshapeMultiBuffer` / `IsCompleteRecord`) so it interoperates with a real
//! Xray/v2ray server. The one deliberate omission is the optional zero-copy
//! "direct splice" optimization (`CommandPaddingDirect` + raw `splice(2)`):
//! Vision's `direct` mode is purely a performance optimization over `end`, and
//! the protocol is identical on the wire (the receiver stops unwrapping either
//! way), so we treat an inbound `direct` command exactly like `end` (stop
//! unwrapping, pass through) and still emit `direct` when the inner stream is
//! TLS 1.3 so a peer that *does* splice can.
//!
//! Frame layout (the first frame in each direction is prefixed with the 16-byte
//! user UUID so the receiver can resynchronize):
//! ```text
//! [uuid(16)]? | command(1) | contentLen(2 BE) | paddingLen(2 BE) | content | padding(0x00 * paddingLen)
//! ```
//! `command` is `0x00` continue, `0x01` end, `0x02` direct.

use std::io;
use std::pin::Pin;
use std::task::{Context, Poll, ready};

use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};

/// The flow value selecting Vision; matches Xray's `vless.XRV`.
pub(crate) const VISION_FLOW: &str = "xtls-rprx-vision";

/// Xray's `buf.Size`; the padding frame reserves 21 bytes (16 uuid + 5 header)
/// of headroom inside one buffer.
const BUF_SIZE: usize = 8192;
/// Largest `content` a single frame carries (`buf.Size - 21`); larger writes are
/// reshaped into two frames so the padding length never goes negative.
const MAX_CONTENT: usize = BUF_SIZE - 21;
/// How many leading buffers (in either direction) to inspect for TLS detection,
/// matching Xray's `NumberOfPacketToFilter`.
const NUM_PACKET_TO_FILTER: i32 = 8;

const CMD_CONTINUE: u8 = 0x00;
const CMD_END: u8 = 0x01;
const CMD_DIRECT: u8 = 0x02;

/// Default Vision padding seeds (`{900, 500, 900, 256}` in Xray): long-padding
/// targets `~900 + rand(0..500) - contentLen` bytes, otherwise `rand(0..256)`.
const SEED_LONG_THRESHOLD: i32 = 900;
const SEED_LONG_RAND: u32 = 500;
const SEED_LONG_BASE: i32 = 900;
const SEED_SHORT_RAND: u32 = 256;

const TLS_APP_DATA_START: [u8; 3] = [0x17, 0x03, 0x03];
const TLS_SERVER_HS_START: [u8; 3] = [0x16, 0x03, 0x03];
const TLS_CLIENT_HS_START: [u8; 2] = [0x16, 0x03];
const TLS13_SUPPORTED_VERSIONS: [u8; 6] = [0x00, 0x2b, 0x00, 0x02, 0x03, 0x04];
const TLS_HS_TYPE_CLIENT_HELLO: u8 = 0x01;
const TLS_HS_TYPE_SERVER_HELLO: u8 = 0x02;
/// `TLS_AES_128_CCM_8_SHA256` — the one TLS 1.3 cipher Xray does *not* splice.
const TLS_AES_128_CCM_8: u16 = 0x1305;

/// Build the VLESS request-header addon bytes carrying the Vision flow: the
/// protobuf encoding of `Addons { string Flow = 1 }` for `xtls-rprx-vision`
/// (field 1, wire type 2: tag `0x0a`, len `0x10`, then the 16 ASCII bytes).
pub(crate) fn flow_addon() -> Vec<u8> {
    let mut addon = Vec::with_capacity(2 + VISION_FLOW.len());
    addon.push(0x0a);
    addon.push(VISION_FLOW.len() as u8);
    addon.extend_from_slice(VISION_FLOW.as_bytes());
    addon
}

/// Shared TLS-detection state, updated by inspecting the first few buffers in
/// both directions (mirrors Xray's `TrafficState` filter fields).
#[derive(Debug)]
struct TrafficState {
    uuid: [u8; 16],
    packets_to_filter: i32,
    is_tls: bool,
    is_tls12_or_above: bool,
    enable_xtls: bool,
    cipher: u16,
    remaining_server_hello: i32,
}

impl TrafficState {
    fn new(uuid: [u8; 16]) -> Self {
        Self {
            uuid,
            packets_to_filter: NUM_PACKET_TO_FILTER,
            is_tls: false,
            is_tls12_or_above: false,
            enable_xtls: false,
            cipher: 0,
            remaining_server_hello: -1,
        }
    }

    /// Port of `XtlsFilterTls`: recognize the tunneled TLS version/cipher from a
    /// content buffer so the writer knows when to stop padding and whether the
    /// inner stream is spliceable TLS 1.3.
    fn filter_tls(&mut self, b: &[u8]) {
        if self.packets_to_filter <= 0 {
            return;
        }
        self.packets_to_filter -= 1;
        if b.len() >= 6 {
            if b[..3] == TLS_SERVER_HS_START && b[5] == TLS_HS_TYPE_SERVER_HELLO {
                self.remaining_server_hello = ((i32::from(b[3]) << 8) | i32::from(b[4])) + 5;
                self.is_tls12_or_above = true;
                self.is_tls = true;
                if b.len() >= 79 && self.remaining_server_hello >= 79 {
                    let session_id_len = usize::from(b[43]);
                    let cipher_at = 43 + session_id_len + 1;
                    if cipher_at + 2 <= b.len() {
                        self.cipher = (u16::from(b[cipher_at]) << 8) | u16::from(b[cipher_at + 1]);
                    }
                }
            } else if b[..2] == TLS_CLIENT_HS_START && b[5] == TLS_HS_TYPE_CLIENT_HELLO {
                self.is_tls = true;
            }
        }
        if self.remaining_server_hello > 0 {
            let end = (self.remaining_server_hello as usize).min(b.len());
            self.remaining_server_hello -= b.len() as i32;
            if contains(&b[..end], &TLS13_SUPPORTED_VERSIONS) {
                let known = matches!(self.cipher, 0x1301..=0x1305);
                if known && self.cipher != TLS_AES_128_CCM_8 {
                    self.enable_xtls = true;
                }
                self.packets_to_filter = 0;
            } else if self.remaining_server_hello <= 0 {
                self.packets_to_filter = 0;
            }
        }
    }
}

/// Return the index of the last occurrence of `needle` in `haystack`, or -1.
fn last_index_of(haystack: &[u8], needle: &[u8]) -> i32 {
    if needle.is_empty() || haystack.len() < needle.len() {
        return -1;
    }
    for start in (0..=haystack.len() - needle.len()).rev() {
        if &haystack[start..start + needle.len()] == needle {
            return start as i32;
        }
    }
    -1
}

/// Whether `haystack` contains `needle`.
fn contains(haystack: &[u8], needle: &[u8]) -> bool {
    if needle.is_empty() {
        return true;
    }
    haystack.windows(needle.len()).any(|w| w == needle)
}

/// Port of `IsCompleteRecord`: true when `buf` is exactly a whole number of
/// complete `application_data` (`0x17 0x03 0x03`) TLS records.
fn is_complete_record(buf: &[u8]) -> bool {
    let mut i = 0usize;
    let total = buf.len();
    let mut header_len = 5i32;
    let mut record_len = 0usize;
    while i < total {
        if header_len > 0 {
            let data = buf[i];
            i += 1;
            match header_len {
                5 if data != 0x17 => return false,
                4 | 3 if data != 0x03 => return false,
                2 => record_len = usize::from(data) << 8,
                1 => record_len |= usize::from(data),
                _ => {}
            }
            header_len -= 1;
        } else if record_len > 0 {
            let remaining = total - i;
            if remaining < record_len {
                return false;
            }
            i += record_len;
            record_len = 0;
            header_len = 5;
        } else {
            return false;
        }
    }
    header_len == 5 && record_len == 0
}

/// Compute a Vision padding length for `content_len` (port of the length logic
/// in `XtlsPadding`).
fn padding_len(content_len: i32, long_padding: bool) -> usize {
    let mut pad = if content_len < SEED_LONG_THRESHOLD && long_padding {
        rand_below(SEED_LONG_RAND) as i32 + SEED_LONG_BASE - content_len
    } else {
        rand_below(SEED_SHORT_RAND) as i32
    };
    let cap = MAX_CONTENT as i32 - content_len;
    if pad > cap {
        pad = cap;
    }
    if pad < 0 { 0 } else { pad as usize }
}

/// A uniformly random integer in `[0, n)`; `0` when `n == 0`. Padding is traffic
/// obfuscation, not a cryptographic secret, so a CSPRNG read failure falls back
/// to `0` rather than aborting the connection.
fn rand_below(n: u32) -> u32 {
    if n == 0 {
        return 0;
    }
    let mut bytes = [0u8; 4];
    if getrandom::fill(&mut bytes).is_err() {
        return 0;
    }
    u32::from_le_bytes(bytes) % n
}

/// Write-side framing: wraps each application write in a Vision padding frame
/// until the inner TLS handshake completes, then passes bytes through.
#[derive(Debug)]
struct WriteSide {
    is_padding: bool,
    uuid_pending: Option<[u8; 16]>,
    pending: Vec<u8>,
    pending_pos: usize,
}

impl WriteSide {
    fn new(uuid: [u8; 16]) -> Self {
        Self {
            is_padding: true,
            uuid_pending: Some(uuid),
            pending: Vec::new(),
            pending_pos: 0,
        }
    }

    /// Append a single padding frame (`[uuid?] command clen plen content pad`)
    /// for `content` to the pending output buffer.
    fn push_frame(&mut self, content: &[u8], command: u8, long_padding: bool) {
        let content_len = content.len() as i32;
        let pad = padding_len(content_len, long_padding);
        if let Some(uuid) = self.uuid_pending.take() {
            self.pending.extend_from_slice(&uuid);
        }
        self.pending.extend_from_slice(&[
            command,
            (content_len >> 8) as u8,
            content_len as u8,
            (pad >> 8) as u8,
            pad as u8,
        ]);
        self.pending.extend_from_slice(content);
        self.pending.resize(self.pending.len() + pad, 0);
    }

    /// Frame one application write (`buf`, already capped to `<= BUF_SIZE`) into
    /// `pending`, advancing the padding state machine (port of
    /// `VisionWriter.WriteMultiBuffer`'s padding block).
    fn frame(&mut self, buf: &[u8], state: &TrafficState) {
        if !self.is_padding {
            self.pending.extend_from_slice(buf);
            return;
        }
        let is_complete = is_complete_record(buf);
        let subs = reshape(buf);
        let last = subs.len() - 1;
        let mut long_padding = state.is_tls;
        for (i, sub) in subs.iter().enumerate() {
            if state.is_tls && sub.len() >= 6 && sub[..3] == TLS_APP_DATA_START && is_complete {
                let command = if i == last {
                    if state.enable_xtls { CMD_DIRECT } else { CMD_END }
                } else {
                    CMD_CONTINUE
                };
                self.push_frame(sub, command, true);
                self.is_padding = false;
                long_padding = false;
                continue;
            } else if !state.is_tls12_or_above && state.packets_to_filter <= 1 {
                // Finish padding one packet early for older Vision receivers.
                self.is_padding = false;
                self.push_frame(sub, CMD_END, long_padding);
                for rest in &subs[i + 1..] {
                    self.pending.extend_from_slice(rest);
                }
                return;
            }
            let command = if i == last && !self.is_padding {
                if state.enable_xtls { CMD_DIRECT } else { CMD_END }
            } else {
                CMD_CONTINUE
            };
            self.push_frame(sub, command, long_padding);
        }
    }

    /// Drain `pending` to `inner`; `Ready(Ok(()))` once fully flushed.
    fn poll_drain<S: AsyncWrite + Unpin>(&mut self, inner: &mut S, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        while self.pending_pos < self.pending.len() {
            let n = ready!(Pin::new(&mut *inner).poll_write(cx, &self.pending[self.pending_pos..]))?;
            if n == 0 {
                return Poll::Ready(Err(io::ErrorKind::WriteZero.into()));
            }
            self.pending_pos += n;
        }
        self.pending.clear();
        self.pending_pos = 0;
        Poll::Ready(Ok(()))
    }
}

/// Split a write into Vision-frameable sub-buffers (port of
/// `ReshapeMultiBuffer`): only buffers at/above the frame headroom are split, at
/// the last inner `application_data` boundary so the end-of-padding trigger
/// keeps working.
fn reshape(buf: &[u8]) -> Vec<&[u8]> {
    if buf.len() < MAX_CONTENT {
        return vec![buf];
    }
    let mut index = last_index_of(buf, &TLS_APP_DATA_START);
    if index < 21 || index > MAX_CONTENT as i32 {
        index = (BUF_SIZE / 2) as i32;
    }
    let idx = index as usize;
    vec![&buf[..idx], &buf[idx..]]
}

/// Read-side decode: first strips the VLESS response header, then removes Vision
/// padding until a terminal command switches the stream to pass-through.
#[derive(Debug)]
enum HeaderPhase {
    NeedVersion,
    NeedAddonLen,
    SkipAddons(u8),
    Done,
}

#[derive(Debug)]
struct ReadSide {
    header: HeaderPhase,
    rem_command: i32,
    rem_content: i32,
    rem_padding: i32,
    cur_command: i32,
    initial_decided: bool,
    passthrough: bool,
    raw: Vec<u8>,
    raw_pos: usize,
    out: Vec<u8>,
    out_pos: usize,
    eof: bool,
}

impl ReadSide {
    fn new() -> Self {
        Self {
            header: HeaderPhase::NeedVersion,
            rem_command: -1,
            rem_content: -1,
            rem_padding: -1,
            cur_command: 0,
            initial_decided: false,
            passthrough: false,
            raw: Vec::new(),
            raw_pos: 0,
            out: Vec::new(),
            out_pos: 0,
            eof: false,
        }
    }

    fn raw_left(&self) -> usize {
        self.raw.len() - self.raw_pos
    }

    /// Compact consumed bytes, then append freshly-read inner bytes.
    fn push_raw(&mut self, data: &[u8]) {
        if self.raw_pos > 0 {
            self.raw.drain(..self.raw_pos);
            self.raw_pos = 0;
        }
        self.raw.extend_from_slice(data);
    }

    /// Strip the VLESS response header (`version | addonLen | addons`) from the
    /// front of `raw`; returns true once consumed, false if more bytes needed.
    fn strip_header(&mut self) -> bool {
        loop {
            match self.header {
                HeaderPhase::Done => return true,
                HeaderPhase::NeedVersion => {
                    if self.raw_left() < 1 {
                        return false;
                    }
                    self.raw_pos += 1;
                    self.header = HeaderPhase::NeedAddonLen;
                }
                HeaderPhase::NeedAddonLen => {
                    if self.raw_left() < 1 {
                        return false;
                    }
                    let len = self.raw[self.raw_pos];
                    self.raw_pos += 1;
                    self.header = if len == 0 {
                        HeaderPhase::Done
                    } else {
                        HeaderPhase::SkipAddons(len)
                    };
                }
                HeaderPhase::SkipAddons(len) => {
                    let len = usize::from(len);
                    if self.raw_left() < len {
                        return false;
                    }
                    self.raw_pos += len;
                    self.header = HeaderPhase::Done;
                }
            }
        }
    }

    /// Drive the unpadding state machine over buffered `raw`, appending decoded
    /// content to `out` and filtering decoded TLS records (port of
    /// `XtlsUnpadding` + the reader's `XtlsFilterTls`).
    fn decode(&mut self, state: &mut TrafficState) {
        if !self.strip_header() {
            return;
        }
        if self.passthrough {
            let from = self.raw_pos;
            self.out.extend_from_slice(&self.raw[from..]);
            self.raw_pos = self.raw.len();
            return;
        }
        let decoded_start = self.out.len();
        loop {
            if self.rem_command == -1 && self.rem_content == -1 && self.rem_padding == -1 {
                if self.initial_decided {
                    // A terminal block just completed; the rest is raw.
                    self.passthrough = true;
                    let from = self.raw_pos;
                    self.out.extend_from_slice(&self.raw[from..]);
                    self.raw_pos = self.raw.len();
                    break;
                }
                if self.raw_left() < 21 {
                    if self.eof {
                        self.passthrough = true;
                        self.initial_decided = true;
                        let from = self.raw_pos;
                        self.out.extend_from_slice(&self.raw[from..]);
                        self.raw_pos = self.raw.len();
                    }
                    break;
                }
                if self.raw[self.raw_pos..self.raw_pos + 16] == state.uuid {
                    self.raw_pos += 16;
                    self.rem_command = 5;
                    self.initial_decided = true;
                } else {
                    self.passthrough = true;
                    self.initial_decided = true;
                    let from = self.raw_pos;
                    self.out.extend_from_slice(&self.raw[from..]);
                    self.raw_pos = self.raw.len();
                    break;
                }
            }

            if self.rem_command > 0 {
                if self.raw_left() == 0 {
                    break;
                }
                let data = self.raw[self.raw_pos];
                self.raw_pos += 1;
                match self.rem_command {
                    5 => self.cur_command = i32::from(data),
                    4 => self.rem_content = i32::from(data) << 8,
                    3 => self.rem_content |= i32::from(data),
                    2 => self.rem_padding = i32::from(data) << 8,
                    1 => self.rem_padding |= i32::from(data),
                    _ => {}
                }
                self.rem_command -= 1;
            } else if self.rem_content > 0 {
                if self.raw_left() == 0 {
                    break;
                }
                let take = (self.rem_content as usize).min(self.raw_left());
                let from = self.raw_pos;
                self.out.extend_from_slice(&self.raw[from..from + take]);
                self.raw_pos += take;
                self.rem_content -= take as i32;
            } else if self.rem_padding > 0 {
                if self.raw_left() == 0 {
                    break;
                }
                let take = (self.rem_padding as usize).min(self.raw_left());
                self.raw_pos += take;
                self.rem_padding -= take as i32;
            }

            if self.rem_command <= 0 && self.rem_content <= 0 && self.rem_padding <= 0 {
                if self.cur_command == 0 {
                    self.rem_command = 5;
                } else {
                    self.rem_command = -1;
                    self.rem_content = -1;
                    self.rem_padding = -1;
                    self.passthrough = true;
                    let from = self.raw_pos;
                    self.out.extend_from_slice(&self.raw[from..]);
                    self.raw_pos = self.raw.len();
                    break;
                }
            }
        }
        if state.packets_to_filter > 0 && self.out.len() > decoded_start {
            let decoded = self.out[decoded_start..].to_vec();
            state.filter_tls(&decoded);
        }
    }
}

/// A VLESS body stream with XTLS Vision framing applied. Wraps the secured
/// transport stream after the VLESS request header (already carrying the Vision
/// flow addon) has been written.
#[derive(Debug)]
pub(crate) struct VisionStream<S> {
    inner: S,
    state: TrafficState,
    read: ReadSide,
    write: WriteSide,
}

impl<S> VisionStream<S> {
    pub(crate) fn new(inner: S, uuid: [u8; 16]) -> Self {
        Self {
            inner,
            state: TrafficState::new(uuid),
            read: ReadSide::new(),
            write: WriteSide::new(uuid),
        }
    }
}

impl<S: AsyncRead + Unpin> AsyncRead for VisionStream<S> {
    fn poll_read(self: Pin<&mut Self>, cx: &mut Context<'_>, buf: &mut ReadBuf<'_>) -> Poll<io::Result<()>> {
        let this = self.get_mut();
        loop {
            if this.read.out_pos < this.read.out.len() {
                let n = (this.read.out.len() - this.read.out_pos).min(buf.remaining());
                buf.put_slice(&this.read.out[this.read.out_pos..this.read.out_pos + n]);
                this.read.out_pos += n;
                if this.read.out_pos == this.read.out.len() {
                    this.read.out.clear();
                    this.read.out_pos = 0;
                }
                return Poll::Ready(Ok(()));
            }

            // Try to decode more from already-buffered raw bytes first.
            this.read.decode(&mut this.state);
            if this.read.out_pos < this.read.out.len() {
                continue;
            }
            if this.read.eof {
                return Poll::Ready(Ok(()));
            }

            let mut scratch = [0u8; 4096];
            let mut rb = ReadBuf::new(&mut scratch);
            ready!(Pin::new(&mut this.inner).poll_read(cx, &mut rb))?;
            let filled = rb.filled();
            if filled.is_empty() {
                this.read.eof = true;
            } else {
                this.read.push_raw(filled);
            }
        }
    }
}

impl<S: AsyncWrite + Unpin> AsyncWrite for VisionStream<S> {
    fn poll_write(self: Pin<&mut Self>, cx: &mut Context<'_>, buf: &[u8]) -> Poll<io::Result<usize>> {
        let this = self.get_mut();
        // Flush any framed-but-unwritten bytes before accepting more input so
        // frame ordering is preserved.
        ready!(this.write.poll_drain(&mut this.inner, cx))?;
        if buf.is_empty() {
            return Poll::Ready(Ok(0));
        }
        let chunk = &buf[..buf.len().min(BUF_SIZE)];
        if this.state.packets_to_filter > 0 {
            this.state.filter_tls(chunk);
        }
        this.write.frame(chunk, &this.state);
        // Opportunistically flush; partial writes are fine since the bytes are
        // already buffered and will be drained by the next call / poll_flush.
        let _ = this.write.poll_drain(&mut this.inner, cx)?;
        Poll::Ready(Ok(chunk.len()))
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        let this = self.get_mut();
        ready!(this.write.poll_drain(&mut this.inner, cx))?;
        Pin::new(&mut this.inner).poll_flush(cx)
    }

    fn poll_shutdown(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        let this = self.get_mut();
        ready!(this.write.poll_drain(&mut this.inner, cx))?;
        Pin::new(&mut this.inner).poll_shutdown(cx)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flow_addon_is_vision_protobuf() {
        let addon = flow_addon();
        assert_eq!(addon[0], 0x0a);
        assert_eq!(addon[1], 0x10);
        assert_eq!(addon.len(), 18);
        assert_eq!(&addon[2..], b"xtls-rprx-vision");
    }

    #[test]
    fn is_complete_record_recognizes_app_data() {
        // Two complete application_data records.
        let mut buf = vec![0x17, 0x03, 0x03, 0x00, 0x02, 0xaa, 0xbb];
        buf.extend_from_slice(&[0x17, 0x03, 0x03, 0x00, 0x01, 0xcc]);
        assert!(is_complete_record(&buf));
        // Truncated body.
        assert!(!is_complete_record(&[0x17, 0x03, 0x03, 0x00, 0x05, 0x01]));
        // Not application_data.
        assert!(!is_complete_record(&[0x16, 0x03, 0x03, 0x00, 0x00]));
    }

    #[test]
    fn last_index_of_and_contains() {
        assert_eq!(last_index_of(b"aXXbXX", b"XX"), 4);
        assert_eq!(last_index_of(b"abc", b"zz"), -1);
        assert!(contains(
            &[0, 0x2b, 0x00, 0x2b, 0x00, 0x02, 0x03, 0x04],
            &TLS13_SUPPORTED_VERSIONS
        ));
        assert!(!contains(b"abc", b"xyz"));
    }

    #[test]
    fn padding_len_within_bounds() {
        // Long padding for small TLS content stays inside the frame headroom.
        for _ in 0..64 {
            let pad = padding_len(100, true);
            assert!(pad <= MAX_CONTENT - 100);
        }
        // Large content gets little/no padding and never overflows.
        let pad = padding_len(MAX_CONTENT as i32 - 5, true);
        assert!(pad <= 5);
    }

    /// A padded frame produced by the writer must round-trip through the
    /// reader's unpadding state machine back to the original content.
    #[test]
    fn write_frames_round_trip_through_read_decode() {
        let uuid = [0x11u8; 16];
        let mut write = WriteSide::new(uuid);

        // A continue frame then a terminal end frame.
        write.push_frame(b"hello ", CMD_CONTINUE, false);
        write.push_frame(b"world", CMD_END, false);
        let framed = write.pending.clone();

        let mut read = ReadSide::new();
        read.header = HeaderPhase::Done; // header already stripped in this unit test
        let mut rstate = TrafficState::new(uuid);
        read.push_raw(&framed);
        read.decode(&mut rstate);
        assert_eq!(&read.out, b"hello world");
        assert!(read.passthrough, "terminal command should switch to passthrough");
    }

    /// Feeding the framed bytes one at a time must decode identically, proving
    /// the reader state machine survives arbitrary chunk boundaries.
    #[test]
    fn read_decode_is_resilient_to_chunk_boundaries() {
        let uuid = [0x22u8; 16];
        let mut write = WriteSide::new(uuid);
        write.push_frame(b"chunked-data-test", CMD_END, true);
        let framed = write.pending.clone();

        let mut read = ReadSide::new();
        read.header = HeaderPhase::Done;
        let mut rstate = TrafficState::new(uuid);
        for byte in &framed {
            read.push_raw(&[*byte]);
            read.decode(&mut rstate);
        }
        assert_eq!(&read.out, b"chunked-data-test");
    }

    #[test]
    fn filter_detects_client_and_server_hello() {
        let mut state = TrafficState::new([0u8; 16]);
        // A TLS client hello record header.
        state.filter_tls(&[0x16, 0x03, 0x01, 0x00, 0x10, TLS_HS_TYPE_CLIENT_HELLO]);
        assert!(state.is_tls);
        assert!(!state.is_tls12_or_above);
    }
}
