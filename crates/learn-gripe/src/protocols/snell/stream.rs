//! The v1-v3 shadowaead chunk stream: [`SnellStream`] wraps the raw transport
//! in Snell's AEAD chunk framing and implements `AsyncRead`/`AsyncWrite`.

use std::io;
use std::pin::Pin;
use std::task::{Context as TaskContext, Poll, ready};
use std::time::Instant;

use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};

use crate::outbound::BoxedStream;

use super::crypto::{AeadCipher, SnellCipher, decrypt_err, increment_nonce, read_cipher_unset, snell_kdf};
use super::pool::{PooledSession, PooledSnell, SnellServerKey, pool_put};
use super::{MAX_CHUNK, RESP_ERROR, RESP_TUNNEL, SALT_LEN, TAG_LEN};

/// Read-side framing state machine.
enum ReadState {
    /// Waiting for the server's 16-byte salt (derives the read cipher).
    Salt,
    /// Waiting for the 18-byte AEAD-sealed chunk length.
    Len,
    /// Waiting for a `clen + 16`-byte sealed payload chunk.
    Data(usize),
    /// Clean EOF (the peer closed the connection).
    Eof,
}

/// Wraps the raw TCP transport in the Snell AEAD chunk stream. Writes seal
/// application data into chunks; reads strip the server salt, derive the read
/// cipher, consume the one-byte command response, then decrypt length-prefixed
/// chunks. The client salt and sealed request header are sent at connect time.
pub(super) struct SnellStream {
    /// `Option` only so [`Drop`] can move it out when parking the session for
    /// reuse; it is always `Some` during normal operation.
    inner: Option<BoxedStream>,
    cipher: SnellCipher,
    psk: Vec<u8>,
    // Write side.
    write_cipher: Option<AeadCipher>,
    write_nonce: [u8; 12],
    write_buf: Vec<u8>,
    write_pos: usize,
    // Read side.
    read_cipher: Option<AeadCipher>,
    read_nonce: [u8; 12],
    read_state: ReadState,
    read_raw: Vec<u8>,
    /// Whether the server's leading command-response byte has been consumed.
    reply_done: bool,
    plain: Vec<u8>,
    plain_pos: usize,
    /// Pool key if this stream may be parked for reuse (v2); `None` = one-shot.
    reuse_key: Option<SnellServerKey>,
    /// The server sent its zero-length chunk (a clean logical EOF, distinct from
    /// a transport close), so the session can be reused.
    read_saw_zero: bool,
    /// We sent our zero-length chunk (half-close) on shutdown.
    write_closed: bool,
}

impl SnellStream {
    pub(super) fn new(
        inner: BoxedStream,
        cipher: SnellCipher,
        psk: Vec<u8>,
        write_cipher: AeadCipher,
        reuse_key: Option<SnellServerKey>,
    ) -> Self {
        Self {
            inner: Some(inner),
            cipher,
            psk,
            write_cipher: Some(write_cipher),
            write_nonce: [0u8; 12],
            write_buf: Vec::new(),
            write_pos: 0,
            read_cipher: None,
            read_nonce: [0u8; 12],
            read_state: ReadState::Salt,
            read_raw: Vec::new(),
            reply_done: false,
            plain: Vec::new(),
            plain_pos: 0,
            reuse_key,
            read_saw_zero: false,
            write_closed: false,
        }
    }

    /// Rebuild a stream on a pooled (reused) session: the salt was consumed on
    /// the first stream so reads resume at a chunk length boundary with the
    /// session's continuous ciphers/nonces, and the server sends a fresh
    /// command-response byte for this request (`reply_done` reset).
    pub(super) fn from_pooled(pooled: PooledSnell, key: SnellServerKey) -> Self {
        Self {
            inner: Some(pooled.inner),
            cipher: pooled.cipher,
            psk: pooled.psk,
            write_cipher: Some(pooled.write_cipher),
            write_nonce: pooled.write_nonce,
            write_buf: Vec::new(),
            write_pos: 0,
            read_cipher: Some(pooled.read_cipher),
            read_nonce: pooled.read_nonce,
            read_state: ReadState::Len,
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
        let inner = self.inner.as_mut().expect("snell stream inner");
        while self.write_pos < self.write_buf.len() {
            let n = ready!(Pin::new(&mut *inner).poll_write(cx, &self.write_buf[self.write_pos..]))?;
            if n == 0 {
                return Poll::Ready(Err(io::Error::new(io::ErrorKind::WriteZero, "snell: write zero")));
            }
            self.write_pos += n;
        }
        self.write_buf.clear();
        self.write_pos = 0;
        Poll::Ready(Ok(()))
    }

    /// Seal `plaintext` (at most [`MAX_CHUNK`] bytes) into a length-prefixed AEAD
    /// chunk queued for writing.
    fn queue_chunk(&mut self, plaintext: &[u8]) -> io::Result<()> {
        let len = u16::try_from(plaintext.len())
            .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "snell: chunk too large"))?;
        let cipher = self.write_cipher.as_ref().expect("snell write cipher");
        let sealed_len = cipher
            .seal(&self.write_nonce, &len.to_be_bytes())
            .map_err(|e| io::Error::other(e.to_string()))?;
        increment_nonce(&mut self.write_nonce);
        let cipher = self.write_cipher.as_ref().expect("snell write cipher");
        let sealed_payload = cipher
            .seal(&self.write_nonce, plaintext)
            .map_err(|e| io::Error::other(e.to_string()))?;
        increment_nonce(&mut self.write_nonce);

        self.write_buf.clear();
        self.write_pos = 0;
        self.write_buf.extend_from_slice(&sealed_len);
        self.write_buf.extend_from_slice(&sealed_payload);
        Ok(())
    }

    /// Queue Snell's half-close: a single zero-length AEAD chunk — only the
    /// sealed length field `0x0000` (one nonce step), no payload block — matching
    /// mihomo's `writeZeroChunk` / shadowaead empty write. The peer decrypts the
    /// length, sees `0`, and treats it as a logical EOF (its `ErrZeroChunk`).
    fn queue_zero_chunk(&mut self) -> io::Result<()> {
        let cipher = self.write_cipher.as_ref().expect("snell write cipher");
        let sealed_len = cipher
            .seal(&self.write_nonce, &[0u8, 0u8])
            .map_err(|e| io::Error::other(e.to_string()))?;
        increment_nonce(&mut self.write_nonce);
        self.write_buf.clear();
        self.write_pos = 0;
        self.write_buf.extend_from_slice(&sealed_len);
        Ok(())
    }

    /// Strip the leading command-response byte from freshly-decrypted plaintext,
    /// returning an error if the server reported `RESP_ERROR`.
    fn consume_reply(&mut self) -> io::Result<()> {
        // `plain` holds the just-decrypted chunk starting at `plain_pos`.
        if self.plain_pos >= self.plain.len() {
            return Ok(());
        }
        self.plain_pos += consume_command_reply(&self.plain[self.plain_pos..])?;
        self.reply_done = true;
        Ok(())
    }
}

/// Inspect the server's leading command-response byte and return the offset of
/// the application data that follows it (`Tunnel` = 1), or an error if the
/// server reported `RESP_ERROR` / an unknown command. Shared by the shadowaead
/// ([`SnellStream`]) and v4 ([`super::v4::SnellV4Stream`]) read paths.
pub(super) fn consume_command_reply(plain: &[u8]) -> io::Result<usize> {
    match plain.first() {
        None => Ok(0),
        Some(&RESP_TUNNEL) => Ok(1),
        Some(&RESP_ERROR) => {
            // `code | msg-len | msg`; best-effort decode for diagnostics
            // (the connection fails regardless of how much is buffered).
            let rest = &plain[1..];
            let code = rest.first().copied().unwrap_or(0);
            let msg = rest
                .get(2..)
                .map(|m| String::from_utf8_lossy(m).into_owned())
                .unwrap_or_default();
            Err(io::Error::new(
                io::ErrorKind::ConnectionRefused,
                format!("snell: server error code {code}: {msg}"),
            ))
        }
        Some(&other) => Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("snell: unexpected command response {other}"),
        )),
    }
}

impl AsyncRead for SnellStream {
    fn poll_read(self: Pin<&mut Self>, cx: &mut TaskContext<'_>, buf: &mut ReadBuf<'_>) -> Poll<io::Result<()>> {
        let this = self.get_mut();
        loop {
            if this.plain_pos < this.plain.len() {
                let n = buf.remaining().min(this.plain.len() - this.plain_pos);
                buf.put_slice(&this.plain[this.plain_pos..this.plain_pos + n]);
                this.plain_pos += n;
                return Poll::Ready(Ok(()));
            }
            if matches!(this.read_state, ReadState::Eof) {
                return Poll::Ready(Ok(()));
            }

            let need = match this.read_state {
                ReadState::Salt => SALT_LEN,
                ReadState::Len => 2 + TAG_LEN,
                ReadState::Data(clen) => clen + TAG_LEN,
                ReadState::Eof => unreachable!(),
            };

            if this.read_raw.len() < need {
                let mut scratch = [0u8; 4096];
                let mut read_buf = ReadBuf::new(&mut scratch);
                let inner = this.inner.as_mut().expect("snell stream inner");
                ready!(Pin::new(inner).poll_read(cx, &mut read_buf))?;
                let filled = read_buf.filled();
                if filled.is_empty() {
                    // Snell (like Shadowsocks) signals end-of-stream by closing
                    // the TCP connection, with no terminating chunk.
                    this.read_state = ReadState::Eof;
                    return Poll::Ready(Ok(()));
                }
                this.read_raw.extend_from_slice(filled);
                continue;
            }

            match this.read_state {
                ReadState::Salt => {
                    let salt: Vec<u8> = this.read_raw.drain(..SALT_LEN).collect();
                    let subkey = snell_kdf(&this.psk, &salt, this.cipher.key_size());
                    let cipher = AeadCipher::new(this.cipher, &subkey).map_err(decrypt_err)?;
                    this.read_cipher = Some(cipher);
                    this.read_state = ReadState::Len;
                }
                ReadState::Len => {
                    let sealed: Vec<u8> = this.read_raw.drain(..2 + TAG_LEN).collect();
                    let Some(cipher) = this.read_cipher.as_ref() else {
                        return Poll::Ready(Err(read_cipher_unset()));
                    };
                    let plain = cipher.open(&this.read_nonce, &sealed).map_err(decrypt_err)?;
                    increment_nonce(&mut this.read_nonce);
                    let clen = u16::from_be_bytes([plain[0], plain[1]]) as usize;
                    if clen == 0 {
                        // Zero-length chunk = Snell half-close: a clean logical
                        // EOF on a (reusable) stream, not a transport close.
                        this.read_saw_zero = true;
                        this.read_state = ReadState::Eof;
                        return Poll::Ready(Ok(()));
                    }
                    if clen > MAX_CHUNK {
                        return Poll::Ready(Err(io::Error::new(
                            io::ErrorKind::InvalidData,
                            "snell: invalid chunk length",
                        )));
                    }
                    this.read_state = ReadState::Data(clen);
                }
                ReadState::Data(clen) => {
                    let sealed: Vec<u8> = this.read_raw.drain(..clen + TAG_LEN).collect();
                    let Some(cipher) = this.read_cipher.as_ref() else {
                        return Poll::Ready(Err(read_cipher_unset()));
                    };
                    let plain = cipher.open(&this.read_nonce, &sealed).map_err(decrypt_err)?;
                    increment_nonce(&mut this.read_nonce);
                    this.plain = plain;
                    this.plain_pos = 0;
                    this.read_state = ReadState::Len;
                    if !this.reply_done {
                        this.consume_reply()?;
                    }
                }
                ReadState::Eof => unreachable!(),
            }
        }
    }
}

impl AsyncWrite for SnellStream {
    fn poll_write(self: Pin<&mut Self>, cx: &mut TaskContext<'_>, buf: &[u8]) -> Poll<io::Result<usize>> {
        let this = self.get_mut();
        ready!(this.poll_drain(cx))?;
        if buf.is_empty() {
            return Poll::Ready(Ok(0));
        }
        let take = buf.len().min(MAX_CHUNK);
        this.queue_chunk(&buf[..take])?;
        if let Poll::Ready(Err(e)) = this.poll_drain(cx) {
            return Poll::Ready(Err(e));
        }
        Poll::Ready(Ok(take))
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut TaskContext<'_>) -> Poll<io::Result<()>> {
        let this = self.get_mut();
        ready!(this.poll_drain(cx))?;
        Pin::new(this.inner.as_mut().expect("snell stream inner")).poll_flush(cx)
    }

    fn poll_shutdown(self: Pin<&mut Self>, cx: &mut TaskContext<'_>) -> Poll<io::Result<()>> {
        let this = self.get_mut();
        // On a reuse-capable stream, shutdown means *half-close*: send our
        // zero-length chunk so the peer ends this logical stream, then flush but
        // keep the TCP open — the session returns to the pool on drop.
        if this.reuse_key.is_some() {
            ready!(this.poll_drain(cx))?;
            if !this.write_closed {
                this.queue_zero_chunk()?;
                this.write_closed = true;
            }
            ready!(this.poll_drain(cx))?;
            return Pin::new(this.inner.as_mut().expect("snell stream inner")).poll_flush(cx);
        }
        ready!(this.poll_drain(cx))?;
        Pin::new(this.inner.as_mut().expect("snell stream inner")).poll_shutdown(cx)
    }
}

impl Drop for SnellStream {
    fn drop(&mut self) {
        let Some(key) = self.reuse_key.take() else {
            return;
        };
        // Only park a session that half-closed cleanly in both directions and
        // carries no buffered/leftover bytes, so the next stream starts on a
        // clean chunk boundary with continuous nonces; otherwise the TCP is
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
            PooledSession::Shadowaead(PooledSnell {
                inner,
                cipher: self.cipher,
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
