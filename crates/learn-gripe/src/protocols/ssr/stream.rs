//! `SsrStream`: the SSR three-layer TCP relay stream (`AsyncRead` +
//! `AsyncWrite`).

use std::io;
use std::pin::Pin;
use std::task::{Context as TaskContext, Poll, ready};

use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};

use crate::outbound::BoxedStream;

use super::cipher::{SsrCipher, StreamCryptor};
use super::obfs::ObfsState;
use super::protocol::ProtocolState;

/// Maximum bytes we buffer before flushing to the inner stream.
const MAX_WRITE_BUF: usize = 0x4000;

/// Wraps the raw TCP transport in the SSR three-layer stack.
pub(super) struct SsrStream {
    inner: BoxedStream,
    cipher_kind: SsrCipher,
    key: Vec<u8>,
    // Write side.
    write_cipher: StreamCryptor,
    write_buf: Vec<u8>,
    write_pos: usize,
    /// Whether the client IV has been sent.
    iv_sent: bool,
    /// Client IV (prepended to the first write).
    client_iv: Vec<u8>,
    // Read side.
    read_cipher: Option<StreamCryptor>,
    read_raw: Vec<u8>,
    /// Whether the server IV has been read.
    iv_read: bool,
    plain: Vec<u8>,
    plain_pos: usize,
    // Protocol and obfs layers.
    protocol: ProtocolState,
    obfs: ObfsState,
    /// Initial payload (socks5 addr) queued for the first write.
    pending_addr: Option<Vec<u8>>,
}

impl SsrStream {
    #[allow(clippy::too_many_arguments)]
    pub(super) fn new(
        inner: BoxedStream,
        cipher_kind: SsrCipher,
        key: Vec<u8>,
        write_cipher: StreamCryptor,
        client_iv: Vec<u8>,
        protocol: ProtocolState,
        obfs: ObfsState,
        addr_payload: Vec<u8>,
    ) -> Self {
        Self {
            inner,
            cipher_kind,
            key,
            write_cipher,
            write_buf: Vec::new(),
            write_pos: 0,
            iv_sent: false,
            client_iv,
            read_cipher: None,
            read_raw: Vec::new(),
            iv_read: false,
            plain: Vec::new(),
            plain_pos: 0,
            protocol,
            obfs,
            pending_addr: Some(addr_payload),
        }
    }

    /// Flush pending sealed bytes to the inner stream.
    fn poll_drain(&mut self, cx: &mut TaskContext<'_>) -> Poll<io::Result<()>> {
        while self.write_pos < self.write_buf.len() {
            let n = ready!(Pin::new(&mut self.inner).poll_write(cx, &self.write_buf[self.write_pos..]))?;
            if n == 0 {
                return Poll::Ready(Err(io::Error::new(io::ErrorKind::WriteZero, "ssr: write zero")));
            }
            self.write_pos += n;
        }
        self.write_buf.clear();
        self.write_pos = 0;
        Poll::Ready(Ok(()))
    }

    /// Encrypt and queue data for writing through the SSR stack.
    fn queue_write(&mut self, data: &[u8]) -> io::Result<()> {
        // Protocol layer: wrap data with auth framing.
        let protocol_data = self.protocol.client_pre_encrypt(data);

        // Stream cipher: encrypt.
        let mut encrypted = protocol_data;
        self.write_cipher.update(&mut encrypted);

        // Prepend IV if this is the first write.
        let wire_data = if !self.iv_sent {
            self.iv_sent = true;
            let mut out = Vec::with_capacity(self.client_iv.len() + encrypted.len());
            out.extend_from_slice(&self.client_iv);
            out.extend_from_slice(&encrypted);
            out
        } else {
            encrypted
        };

        // Obfs layer: wrap.
        let obfs_data = self.obfs.client_encode(&wire_data);

        self.write_buf = obfs_data;
        self.write_pos = 0;
        Ok(())
    }
}

impl AsyncRead for SsrStream {
    fn poll_read(self: Pin<&mut Self>, cx: &mut TaskContext<'_>, buf: &mut ReadBuf<'_>) -> Poll<io::Result<()>> {
        let this = self.get_mut();
        loop {
            // Return buffered plaintext.
            if this.plain_pos < this.plain.len() {
                let n = buf.remaining().min(this.plain.len() - this.plain_pos);
                buf.put_slice(&this.plain[this.plain_pos..this.plain_pos + n]);
                this.plain_pos += n;
                return Poll::Ready(Ok(()));
            }

            // Read more raw data from the transport.
            let mut scratch = [0u8; 8192];
            let mut read_buf = ReadBuf::new(&mut scratch);
            ready!(Pin::new(&mut this.inner).poll_read(cx, &mut read_buf))?;
            let filled = read_buf.filled();
            if filled.is_empty() {
                return Poll::Ready(Ok(())); // EOF
            }

            // Obfs decode.
            let decoded = this
                .obfs
                .client_decode(filled)
                .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e.to_string()))?;

            if decoded.is_empty() {
                continue; // obfs needs more data (e.g., incomplete HTTP header)
            }

            this.read_raw.extend_from_slice(&decoded);

            // Extract server IV if not yet read.
            if !this.iv_read {
                let iv_len = this.cipher_kind.iv_size();
                if this.read_raw.len() < iv_len {
                    continue; // need more data for IV
                }
                let server_iv: Vec<u8> = this.read_raw.drain(..iv_len).collect();
                this.read_cipher = Some(StreamCryptor::new_decrypt(this.cipher_kind, &this.key, &server_iv));
                this.iv_read = true;
                if this.read_raw.is_empty() {
                    continue;
                }
            }

            // Decrypt.
            let Some(cipher) = this.read_cipher.as_mut() else {
                return Poll::Ready(Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "ssr: read cipher not initialized",
                )));
            };
            let mut decrypted = std::mem::take(&mut this.read_raw);
            cipher.update(&mut decrypted);

            // Protocol post-decrypt: strip framing.
            let payload = this
                .protocol
                .client_post_decrypt(&decrypted)
                .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e.to_string()))?;

            if payload.is_empty() {
                continue; // protocol layer buffering (need more data)
            }

            this.plain = payload;
            this.plain_pos = 0;
        }
    }
}

impl AsyncWrite for SsrStream {
    fn poll_write(self: Pin<&mut Self>, cx: &mut TaskContext<'_>, buf: &[u8]) -> Poll<io::Result<usize>> {
        let this = self.get_mut();
        ready!(this.poll_drain(cx))?;

        // On the first write, send the queued target address alongside user data.
        if let Some(addr) = this.pending_addr.take() {
            let mut combined = Vec::with_capacity(addr.len() + buf.len());
            combined.extend_from_slice(&addr);
            combined.extend_from_slice(buf);
            this.queue_write(&combined)?;
            // Eagerly start draining.
            if let Poll::Ready(Err(e)) = this.poll_drain(cx) {
                return Poll::Ready(Err(e));
            }
            return Poll::Ready(Ok(buf.len()));
        }

        if buf.is_empty() {
            return Poll::Ready(Ok(0));
        }
        let take = buf.len().min(MAX_WRITE_BUF);
        this.queue_write(&buf[..take])?;
        if let Poll::Ready(Err(e)) = this.poll_drain(cx) {
            return Poll::Ready(Err(e));
        }
        Poll::Ready(Ok(take))
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut TaskContext<'_>) -> Poll<io::Result<()>> {
        let this = self.get_mut();
        ready!(this.poll_drain(cx))?;
        Pin::new(&mut this.inner).poll_flush(cx)
    }

    fn poll_shutdown(self: Pin<&mut Self>, cx: &mut TaskContext<'_>) -> Poll<io::Result<()>> {
        let this = self.get_mut();
        ready!(this.poll_drain(cx))?;
        Pin::new(&mut this.inner).poll_shutdown(cx)
    }
}
