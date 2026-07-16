//! The v4/v5 frame stream: [`SnellV4Stream`] plus the shared frame
//! serialisation helpers ([`build_v4_frame`] / [`read_v4_frame`]) used by both
//! the TCP stream and the v4 UDP path.

use std::io;
use std::pin::Pin;
use std::task::{Context as TaskContext, Poll, ready};
use std::time::Instant;

use anyhow::{Context, Result, bail};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, ReadBuf};

use crate::outbound::BoxedStream;

use super::crypto::{
    AeadCipher, SnellCipher, decrypt_err, increment_nonce, random_bytes, read_cipher_unset, snell_kdf,
};
use super::pool::{PooledSession, PooledSnellV4, SnellServerKey, pool_put};
use super::stream::consume_command_reply;
use super::{MAX_CHUNK, SALT_LEN, TAG_LEN};

/// v4 frame header plaintext: `0x04 | 0 | 0 | padding-len(u16 BE) | payload-len(u16 BE)`.
const V4_HEADER_PLAIN: usize = 7;
/// v4 sealed header is the 7-byte plaintext plus the AEAD tag.
const V4_HEADER_CIPHER: usize = V4_HEADER_PLAIN + TAG_LEN;
/// Marker byte at the start of every v4 frame header.
const V4_FRAME_BYTE: u8 = 4;
/// Inclusive lower bound for the random initial-padding block on the first frame.
pub(super) const V4_INITIAL_PADDING_MIN: usize = 0x100;
/// Width of the initial-padding range; the length is `MIN + rand(0..SPAN)`.
pub(super) const V4_INITIAL_PADDING_SPAN: usize = 0x100;

/// v4's initial-padding obfuscation: swap every even-indexed byte between the
/// padding block and the payload ciphertext, up to the shorter of the two. It
/// is its own inverse, so the reader applies the same swap to recover the
/// payload ciphertext before decrypting.
fn swap_padding(padding: &mut [u8], payload_cipher: &mut [u8]) {
    let limit = padding.len().min(payload_cipher.len());
    let mut i = 0;
    while i < limit {
        std::mem::swap(&mut padding[i], &mut payload_cipher[i]);
        i += 2;
    }
}

/// Read-side framing state for the v4 frame stream.
enum V4ReadState {
    /// Waiting for the server's 16-byte salt (derives the read cipher).
    Salt,
    /// Waiting for the 23-byte AEAD-sealed frame header.
    Header,
    /// Waiting for `padding + payload + TAG_LEN` bytes of frame body.
    Body { padding: usize, payload: usize },
    /// Clean EOF (a zero-payload frame or a transport close).
    Eof,
}

/// Wraps the raw transport in the Snell **v4** frame stream (v4/v5). Like
/// [`super::stream::SnellStream`] it carries the request header / command
/// response and uses Argon2id + AES-128-GCM + a counter nonce, but each frame
/// is `AEAD(header) | [padding] | AEAD(payload)`; the first frame is prefixed
/// with the client salt and an initial random padding block (see the module
/// docs).
pub(super) struct SnellV4Stream {
    /// `Option` only so [`Drop`] can move it out when parking the session for
    /// reuse; always `Some` during normal operation.
    inner: Option<BoxedStream>,
    psk: Vec<u8>,
    // Write side.
    /// Same as `inner`: `Option` only so a reusable session can be parked.
    write_cipher: Option<AeadCipher>,
    write_salt: [u8; SALT_LEN],
    write_nonce: [u8; 12],
    /// Whether the salt (and, with it, the first frame's initial padding) has
    /// been emitted; gates the one-time salt prefix and initial padding.
    salt_sent: bool,
    initial_padding: usize,
    write_buf: Vec<u8>,
    write_pos: usize,
    // Read side.
    read_cipher: Option<AeadCipher>,
    read_nonce: [u8; 12],
    read_state: V4ReadState,
    read_raw: Vec<u8>,
    reply_done: bool,
    plain: Vec<u8>,
    plain_pos: usize,
    /// Pool key if this stream may be parked for reuse (v4/v5 + `reuse`);
    /// `None` = one-shot.
    reuse_key: Option<SnellServerKey>,
    /// The server sent its zero-payload frame (a clean logical EOF, distinct
    /// from a transport close), so the session can be reused.
    read_saw_zero: bool,
    /// We sent our zero-payload frame (half-close) on shutdown.
    write_closed: bool,
}

impl SnellV4Stream {
    pub(super) fn new(inner: BoxedStream, psk: Vec<u8>, reuse_key: Option<SnellServerKey>) -> Result<Self> {
        // v4 is always AES-128-GCM.
        let mut salt = [0u8; SALT_LEN];
        random_bytes(&mut salt);
        let subkey = snell_kdf(&psk, &salt, SnellCipher::Aes128Gcm.key_size());
        let write_cipher = AeadCipher::new(SnellCipher::Aes128Gcm, &subkey)?;

        let mut delta = [0u8; 2];
        random_bytes(&mut delta);
        let initial_padding = V4_INITIAL_PADDING_MIN + (u16::from_le_bytes(delta) as usize) % V4_INITIAL_PADDING_SPAN;

        Ok(Self {
            inner: Some(inner),
            psk,
            write_cipher: Some(write_cipher),
            write_salt: salt,
            write_nonce: [0u8; 12],
            salt_sent: false,
            initial_padding,
            write_buf: Vec::new(),
            write_pos: 0,
            read_cipher: None,
            read_nonce: [0u8; 12],
            read_state: V4ReadState::Salt,
            read_raw: Vec::new(),
            reply_done: false,
            plain: Vec::new(),
            plain_pos: 0,
            reuse_key,
            read_saw_zero: false,
            write_closed: false,
        })
    }

    /// Rebuild a stream on a pooled (reused) v4 session: the salt was already
    /// sent / consumed on the first stream, so writes emit no salt or initial
    /// padding and reads resume at a frame-header boundary with the session's
    /// continuous ciphers/nonces; the server sends a fresh command-response byte
    /// for this request (`reply_done` reset).
    pub(super) fn from_pooled(pooled: PooledSnellV4, key: SnellServerKey) -> Self {
        Self {
            inner: Some(pooled.inner),
            psk: pooled.psk,
            write_cipher: Some(pooled.write_cipher),
            write_salt: [0u8; SALT_LEN],
            write_nonce: pooled.write_nonce,
            salt_sent: true,
            initial_padding: 0,
            write_buf: Vec::new(),
            write_pos: 0,
            read_cipher: Some(pooled.read_cipher),
            read_nonce: pooled.read_nonce,
            read_state: V4ReadState::Header,
            read_raw: Vec::new(),
            reply_done: false,
            plain: Vec::new(),
            plain_pos: 0,
            reuse_key: Some(key),
            read_saw_zero: false,
            write_closed: false,
        }
    }

    /// Flush any pending sealed bytes to the inner stream.
    fn poll_drain(&mut self, cx: &mut TaskContext<'_>) -> Poll<io::Result<()>> {
        let inner = self.inner.as_mut().expect("snell v4 stream inner");
        while self.write_pos < self.write_buf.len() {
            let n = ready!(Pin::new(&mut *inner).poll_write(cx, &self.write_buf[self.write_pos..]))?;
            if n == 0 {
                return Poll::Ready(Err(io::Error::new(io::ErrorKind::WriteZero, "snell v4: write zero")));
            }
            self.write_pos += n;
        }
        self.write_buf.clear();
        self.write_pos = 0;
        Poll::Ready(Ok(()))
    }

    /// Seal `payload` (at most [`MAX_CHUNK`] bytes) into one v4 frame queued for
    /// writing. The first frame prepends the salt and an initial padding block
    /// (interleaved with the payload ciphertext via [`swap_padding`]).
    fn queue_frame(&mut self, payload: &[u8]) -> io::Result<()> {
        let first = !self.salt_sent;
        let padding_len = if first && !payload.is_empty() {
            self.initial_padding
        } else {
            0
        };
        let salt = if first { Some(&self.write_salt[..]) } else { None };
        let cipher = self.write_cipher.as_ref().expect("snell v4 write cipher");
        let frame = build_v4_frame(cipher, &mut self.write_nonce, payload, padding_len, salt)
            .map_err(|e| io::Error::other(e.to_string()))?;
        self.salt_sent = true;
        self.write_buf = frame;
        self.write_pos = 0;
        Ok(())
    }

    /// Queue Snell's v4 half-close: a single zero-payload frame (only the sealed
    /// header, `payLen == 0`, no padding, one nonce step). The peer decodes the
    /// header, sees a zero payload, and treats it as a logical EOF
    /// (`ErrZeroChunk`). Never prepends the salt — a reusable stream has already
    /// sent it.
    fn queue_zero_frame(&mut self) -> io::Result<()> {
        let cipher = self.write_cipher.as_ref().expect("snell v4 write cipher");
        let frame =
            build_v4_frame(cipher, &mut self.write_nonce, &[], 0, None).map_err(|e| io::Error::other(e.to_string()))?;
        self.salt_sent = true;
        self.write_buf = frame;
        self.write_pos = 0;
        Ok(())
    }
}

/// Serialise one v4 frame `AEAD(header) | [padding] | AEAD(payload)`, advancing
/// `nonce` once per AEAD seal. When `salt` is `Some` (the first frame on a
/// stream) it is prepended and `padding_len` random padding bytes are
/// interleaved with the payload ciphertext via [`swap_padding`]. Shared by the
/// v4 TCP stream ([`SnellV4Stream`]) and the v4 UDP path
/// ([`super::udp::SnellV4Udp`]).
pub(super) fn build_v4_frame(
    cipher: &AeadCipher,
    nonce: &mut [u8; 12],
    payload: &[u8],
    padding_len: usize,
    salt: Option<&[u8]>,
) -> Result<Vec<u8>> {
    let mut header = [0u8; V4_HEADER_PLAIN];
    header[0] = V4_FRAME_BYTE;
    header[3..5].copy_from_slice(&(padding_len as u16).to_be_bytes());
    header[5..7].copy_from_slice(&(payload.len() as u16).to_be_bytes());
    let sealed_header = cipher.seal(nonce, &header)?;
    increment_nonce(nonce);

    let mut payload_cipher = if payload.is_empty() {
        Vec::new()
    } else {
        let pc = cipher.seal(nonce, payload)?;
        increment_nonce(nonce);
        pc
    };

    let mut out = Vec::new();
    if let Some(salt) = salt {
        out.extend_from_slice(salt);
    }
    out.extend_from_slice(&sealed_header);
    if padding_len > 0 {
        let mut padding = vec![0u8; padding_len];
        random_bytes(&mut padding);
        swap_padding(&mut padding, &mut payload_cipher);
        out.extend_from_slice(&padding);
    }
    out.extend_from_slice(&payload_cipher);
    Ok(out)
}

/// Read exactly one v4 frame from `reader` and return its decrypted payload,
/// advancing `nonce` once per AEAD open. A zero-payload frame (Snell's logical
/// EOF) returns an empty `Vec`.
pub(super) async fn read_v4_frame<R: AsyncRead + Unpin>(
    reader: &mut R,
    cipher: &AeadCipher,
    nonce: &mut [u8; 12],
) -> Result<Vec<u8>> {
    let mut header_cipher = [0u8; V4_HEADER_CIPHER];
    reader
        .read_exact(&mut header_cipher)
        .await
        .context("snell v4 udp: read frame header")?;
    let header = cipher.open(nonce, &header_cipher)?;
    increment_nonce(nonce);
    if header.len() != V4_HEADER_PLAIN || header[0] != V4_FRAME_BYTE {
        bail!("snell v4 udp: invalid frame header");
    }
    let padding = u16::from_be_bytes([header[3], header[4]]) as usize;
    let payload = u16::from_be_bytes([header[5], header[6]]) as usize;
    if payload == 0 {
        return Ok(Vec::new());
    }
    if payload > MAX_CHUNK || padding > MAX_CHUNK {
        bail!("snell v4 udp: frame too large");
    }
    let mut frame = vec![0u8; padding + payload + TAG_LEN];
    reader
        .read_exact(&mut frame)
        .await
        .context("snell v4 udp: read frame body")?;
    if padding > 0 {
        let (pad_part, pay_part) = frame.split_at_mut(padding);
        swap_padding(pad_part, pay_part);
    }
    let plain = cipher.open(nonce, &frame[padding..])?;
    increment_nonce(nonce);
    Ok(plain)
}

impl AsyncRead for SnellV4Stream {
    fn poll_read(self: Pin<&mut Self>, cx: &mut TaskContext<'_>, buf: &mut ReadBuf<'_>) -> Poll<io::Result<()>> {
        let this = self.get_mut();
        loop {
            if this.plain_pos < this.plain.len() {
                let n = buf.remaining().min(this.plain.len() - this.plain_pos);
                buf.put_slice(&this.plain[this.plain_pos..this.plain_pos + n]);
                this.plain_pos += n;
                return Poll::Ready(Ok(()));
            }
            if matches!(this.read_state, V4ReadState::Eof) {
                return Poll::Ready(Ok(()));
            }

            let need = match this.read_state {
                V4ReadState::Salt => SALT_LEN,
                V4ReadState::Header => V4_HEADER_CIPHER,
                V4ReadState::Body { padding, payload } => padding + payload + TAG_LEN,
                V4ReadState::Eof => unreachable!(),
            };

            if this.read_raw.len() < need {
                let mut scratch = [0u8; 4096];
                let mut read_buf = ReadBuf::new(&mut scratch);
                let inner = this.inner.as_mut().expect("snell v4 stream inner");
                ready!(Pin::new(inner).poll_read(cx, &mut read_buf))?;
                let filled = read_buf.filled();
                if filled.is_empty() {
                    this.read_state = V4ReadState::Eof;
                    return Poll::Ready(Ok(()));
                }
                this.read_raw.extend_from_slice(filled);
                continue;
            }

            match this.read_state {
                V4ReadState::Salt => {
                    let salt: Vec<u8> = this.read_raw.drain(..SALT_LEN).collect();
                    let subkey = snell_kdf(&this.psk, &salt, SnellCipher::Aes128Gcm.key_size());
                    let cipher = AeadCipher::new(SnellCipher::Aes128Gcm, &subkey).map_err(decrypt_err)?;
                    this.read_cipher = Some(cipher);
                    this.read_state = V4ReadState::Header;
                }
                V4ReadState::Header => {
                    let sealed: Vec<u8> = this.read_raw.drain(..V4_HEADER_CIPHER).collect();
                    let Some(cipher) = this.read_cipher.as_ref() else {
                        return Poll::Ready(Err(read_cipher_unset()));
                    };
                    let header = cipher.open(&this.read_nonce, &sealed).map_err(decrypt_err)?;
                    increment_nonce(&mut this.read_nonce);
                    if header.len() != V4_HEADER_PLAIN || header[0] != V4_FRAME_BYTE {
                        return Poll::Ready(Err(io::Error::new(
                            io::ErrorKind::InvalidData,
                            "snell v4: invalid frame header",
                        )));
                    }
                    let padding = u16::from_be_bytes([header[3], header[4]]) as usize;
                    let payload = u16::from_be_bytes([header[5], header[6]]) as usize;
                    if payload == 0 {
                        if padding != 0 {
                            return Poll::Ready(Err(io::Error::new(
                                io::ErrorKind::InvalidData,
                                "snell v4: zero chunk with padding",
                            )));
                        }
                        // Zero-payload frame = Snell's logical EOF (ErrZeroChunk):
                        // a clean half-close on a (reusable) stream, distinct
                        // from a transport close.
                        this.read_saw_zero = true;
                        this.read_state = V4ReadState::Eof;
                        return Poll::Ready(Ok(()));
                    }
                    if payload > MAX_CHUNK || padding > MAX_CHUNK {
                        return Poll::Ready(Err(io::Error::new(
                            io::ErrorKind::InvalidData,
                            "snell v4: frame too large",
                        )));
                    }
                    this.read_state = V4ReadState::Body { padding, payload };
                }
                V4ReadState::Body { padding, payload } => {
                    let mut frame: Vec<u8> = this.read_raw.drain(..padding + payload + TAG_LEN).collect();
                    if padding > 0 {
                        let (pad_part, pay_part) = frame.split_at_mut(padding);
                        swap_padding(pad_part, pay_part);
                    }
                    let Some(cipher) = this.read_cipher.as_ref() else {
                        return Poll::Ready(Err(read_cipher_unset()));
                    };
                    let plain = cipher.open(&this.read_nonce, &frame[padding..]).map_err(decrypt_err)?;
                    increment_nonce(&mut this.read_nonce);
                    this.plain = plain;
                    this.plain_pos = 0;
                    this.read_state = V4ReadState::Header;
                    if !this.reply_done {
                        this.plain_pos = consume_command_reply(&this.plain)?;
                        this.reply_done = true;
                    }
                }
                V4ReadState::Eof => unreachable!(),
            }
        }
    }
}

impl AsyncWrite for SnellV4Stream {
    fn poll_write(self: Pin<&mut Self>, cx: &mut TaskContext<'_>, buf: &[u8]) -> Poll<io::Result<usize>> {
        let this = self.get_mut();
        ready!(this.poll_drain(cx))?;
        if buf.is_empty() {
            return Poll::Ready(Ok(0));
        }
        let take = buf.len().min(MAX_CHUNK);
        this.queue_frame(&buf[..take])?;
        if let Poll::Ready(Err(e)) = this.poll_drain(cx) {
            return Poll::Ready(Err(e));
        }
        Poll::Ready(Ok(take))
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut TaskContext<'_>) -> Poll<io::Result<()>> {
        let this = self.get_mut();
        ready!(this.poll_drain(cx))?;
        Pin::new(this.inner.as_mut().expect("snell v4 stream inner")).poll_flush(cx)
    }

    fn poll_shutdown(self: Pin<&mut Self>, cx: &mut TaskContext<'_>) -> Poll<io::Result<()>> {
        let this = self.get_mut();
        // On a reuse-capable stream, shutdown means *half-close*: send our
        // zero-payload frame so the peer ends this logical stream, then flush
        // but keep the TCP open — the session returns to the pool on drop.
        if this.reuse_key.is_some() {
            ready!(this.poll_drain(cx))?;
            if !this.write_closed {
                this.queue_zero_frame()?;
                this.write_closed = true;
            }
            ready!(this.poll_drain(cx))?;
            return Pin::new(this.inner.as_mut().expect("snell v4 stream inner")).poll_flush(cx);
        }
        ready!(this.poll_drain(cx))?;
        Pin::new(this.inner.as_mut().expect("snell v4 stream inner")).poll_shutdown(cx)
    }
}

impl Drop for SnellV4Stream {
    fn drop(&mut self) {
        let Some(key) = self.reuse_key.take() else {
            return;
        };
        // Only park a session that half-closed cleanly in both directions and
        // carries no buffered/leftover bytes, so the next stream starts on a
        // clean frame boundary with continuous nonces; otherwise the TCP is
        // closed by dropping the fields.
        if !(self.read_saw_zero && self.write_closed) {
            return;
        }
        if self.write_pos < self.write_buf.len() || !self.read_raw.is_empty() || self.plain_pos < self.plain.len() {
            return;
        }
        let (Some(inner), Some(write_cipher), Some(read_cipher)) =
            (self.inner.take(), self.write_cipher.take(), self.read_cipher.take())
        else {
            return;
        };
        pool_put(
            key,
            PooledSession::V4(PooledSnellV4 {
                inner,
                psk: std::mem::take(&mut self.psk),
                write_cipher,
                write_nonce: self.write_nonce,
                read_cipher,
                read_nonce: self.read_nonce,
                idle_since: Instant::now(),
            }),
        );
    }
}
