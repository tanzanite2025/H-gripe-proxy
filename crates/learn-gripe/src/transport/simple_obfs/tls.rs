//! simple-obfs **tls** mode client (fake TLS 1.2).
//!
//! The TLS mode disguises the stream as a TLS 1.2 session without doing any real
//! cryptography. The framing matches `shadowsocks/simple-obfs` and the
//! clash/mihomo client, so real obfs-tls nodes interoperate:
//!
//! * **First client write** is wrapped in a fake `ClientHello`: the payload
//!   rides inside the `SessionTicket` extension (type `0x0023`) and the obfs
//!   host is sent as the SNI. See [`build_client_hello`].
//! * **First server response** is a fixed 105-byte fake handshake (ServerHello +
//!   ChangeCipherSpec + Finished) which the client skips wholesale.
//! * **Everything after** is framed as TLS *application data* records
//!   (`0x17 0x03 0x03 | len(2) | payload`), chunked to at most [`CHUNK_SIZE`]
//!   bytes per record.
//!
//! Only the client side is implemented. Like clash, the `ClientHello` is sent
//! lazily on the first write rather than at connect time, so no bytes hit the
//! wire until the Shadowsocks layer sends its salt.

use std::io;
use std::pin::Pin;
use std::task::{Context as TaskContext, Poll, ready};
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::Result;
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};

/// Largest payload carried in a single TLS application-data record.
const CHUNK_SIZE: usize = 1 << 14; // 16 KiB

/// Fixed size of the server's fake handshake response (ServerHello +
/// ChangeCipherSpec + Finished) that the client skips before reading data.
const SERVER_HANDSHAKE_LEN: usize = 105;

/// TLS application-data record header: content type `0x17`, version TLS 1.2.
const APP_DATA_HEADER: [u8; 3] = [0x17, 0x03, 0x03];

/// Begin a simple-obfs TLS session over `stream`. No bytes are written yet; the
/// `ClientHello` carrying the first payload is sent on the first write.
pub async fn connect_tls<S>(stream: S, host: &str) -> Result<ObfsTlsStream<S>>
where
    S: AsyncRead + AsyncWrite + Unpin,
{
    Ok(ObfsTlsStream::new(stream, host))
}

/// A simple-obfs fake-TLS client stream. Writes are framed as a `ClientHello`
/// (first write) then TLS application-data records; reads skip the server's
/// fixed handshake response then strip the record headers.
pub struct ObfsTlsStream<S> {
    inner: S,
    server: String,
    first_request: bool,

    /// Encoded bytes pending write to `inner`, and how many were already sent.
    write_buf: Vec<u8>,
    write_off: usize,

    /// Bytes of the server handshake response still to be skipped (starts at
    /// [`SERVER_HANDSHAKE_LEN`]).
    skip_remaining: usize,
    /// Raw bytes read from `inner` not yet parsed.
    raw: Vec<u8>,
    /// Payload bytes left in the application-data record currently being parsed.
    record_remaining: usize,
    /// Decoded application bytes ready to hand to the caller.
    out: Vec<u8>,
    out_pos: usize,
    saw_eof: bool,
}

impl<S> ObfsTlsStream<S> {
    fn new(inner: S, host: &str) -> Self {
        Self {
            inner,
            server: host.to_string(),
            first_request: true,
            write_buf: Vec::new(),
            write_off: 0,
            skip_remaining: SERVER_HANDSHAKE_LEN,
            raw: Vec::new(),
            record_remaining: 0,
            out: Vec::new(),
            out_pos: 0,
            saw_eof: false,
        }
    }

    /// Advance the read state machine by one step using buffered `raw` bytes.
    /// Returns `true` if it made progress (so the caller should re-check), or
    /// `false` if it needs more bytes from the inner transport.
    fn decode_step(&mut self) -> bool {
        if self.skip_remaining > 0 {
            if self.raw.is_empty() {
                return false;
            }
            let take = self.skip_remaining.min(self.raw.len());
            self.raw.drain(..take);
            self.skip_remaining -= take;
            return true;
        }
        if self.record_remaining > 0 {
            if self.raw.is_empty() {
                return false;
            }
            let take = self.record_remaining.min(self.raw.len());
            self.out.extend_from_slice(&self.raw[..take]);
            self.raw.drain(..take);
            self.record_remaining -= take;
            return true;
        }
        if self.raw.len() < 5 {
            return false;
        }
        // Record header: type | version(2) | length(2). The type is assumed to
        // be application-data (0x17); the length bounds the payload that follows.
        let len = u16::from_be_bytes([self.raw[3], self.raw[4]]) as usize;
        self.raw.drain(..5);
        self.record_remaining = len;
        true
    }
}

impl<S> AsyncRead for ObfsTlsStream<S>
where
    S: AsyncRead + AsyncWrite + Unpin,
{
    fn poll_read(self: Pin<&mut Self>, cx: &mut TaskContext<'_>, buf: &mut ReadBuf<'_>) -> Poll<io::Result<()>> {
        let this = self.get_mut();
        loop {
            if this.out_pos < this.out.len() {
                let n = buf.remaining().min(this.out.len() - this.out_pos);
                buf.put_slice(&this.out[this.out_pos..this.out_pos + n]);
                this.out_pos += n;
                if this.out_pos == this.out.len() {
                    this.out.clear();
                    this.out_pos = 0;
                }
                return Poll::Ready(Ok(()));
            }
            this.out.clear();
            this.out_pos = 0;

            if this.decode_step() {
                continue;
            }
            if this.saw_eof {
                return Poll::Ready(Ok(()));
            }

            let mut tmp = [0u8; 8192];
            let mut rb = ReadBuf::new(&mut tmp);
            ready!(Pin::new(&mut this.inner).poll_read(cx, &mut rb))?;
            let filled = rb.filled();
            if filled.is_empty() {
                this.saw_eof = true;
            } else {
                this.raw.extend_from_slice(filled);
            }
        }
    }
}

impl<S> ObfsTlsStream<S>
where
    S: AsyncRead + AsyncWrite + Unpin,
{
    /// Flush `write_buf` to the inner transport, returning `Ready(Ok(()))` only
    /// once it is fully drained.
    fn poll_flush_write_buf(&mut self, cx: &mut TaskContext<'_>) -> Poll<io::Result<()>> {
        while self.write_off < self.write_buf.len() {
            let n = ready!(Pin::new(&mut self.inner).poll_write(cx, &self.write_buf[self.write_off..]))?;
            if n == 0 {
                return Poll::Ready(Err(io::Error::new(
                    io::ErrorKind::WriteZero,
                    "simple-obfs tls: inner transport accepted no bytes",
                )));
            }
            self.write_off += n;
        }
        self.write_buf.clear();
        self.write_off = 0;
        Poll::Ready(Ok(()))
    }
}

impl<S> AsyncWrite for ObfsTlsStream<S>
where
    S: AsyncRead + AsyncWrite + Unpin,
{
    fn poll_write(self: Pin<&mut Self>, cx: &mut TaskContext<'_>, buf: &[u8]) -> Poll<io::Result<usize>> {
        let this = self.get_mut();
        // Don't accept new data until the previously encoded bytes are flushed.
        ready!(this.poll_flush_write_buf(cx))?;
        if buf.is_empty() {
            return Poll::Ready(Ok(0));
        }

        if this.first_request {
            this.write_buf = build_client_hello(buf, &this.server)?;
            this.first_request = false;
        } else {
            let mut encoded = Vec::with_capacity(buf.len() + (buf.len() / CHUNK_SIZE + 1) * 5);
            for chunk in buf.chunks(CHUNK_SIZE) {
                encoded.extend_from_slice(&APP_DATA_HEADER);
                encoded.extend_from_slice(&(chunk.len() as u16).to_be_bytes());
                encoded.extend_from_slice(chunk);
            }
            this.write_buf = encoded;
        }
        this.write_off = 0;

        // Best-effort flush; any remainder completes on the next poll.
        match this.poll_flush_write_buf(cx) {
            Poll::Ready(Err(e)) => Poll::Ready(Err(e)),
            Poll::Ready(Ok(())) | Poll::Pending => Poll::Ready(Ok(buf.len())),
        }
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut TaskContext<'_>) -> Poll<io::Result<()>> {
        let this = self.get_mut();
        ready!(this.poll_flush_write_buf(cx))?;
        Pin::new(&mut this.inner).poll_flush(cx)
    }

    fn poll_shutdown(self: Pin<&mut Self>, cx: &mut TaskContext<'_>) -> Poll<io::Result<()>> {
        let this = self.get_mut();
        ready!(this.poll_flush_write_buf(cx))?;
        Pin::new(&mut this.inner).poll_shutdown(cx)
    }
}

/// Build the fake `ClientHello` that carries `data` in its `SessionTicket`
/// extension and `server` as the SNI. The byte layout (including the fixed
/// cipher-suite / extension blocks and the length constants) mirrors the
/// clash/mihomo simple-obfs client so real obfs-tls servers accept it.
fn build_client_hello(data: &[u8], server: &str) -> io::Result<Vec<u8>> {
    let mut random = [0u8; 28];
    let mut session_id = [0u8; 32];
    getrandom::fill(&mut random).map_err(|_| io::Error::other("simple-obfs tls: system RNG unavailable"))?;
    getrandom::fill(&mut session_id).map_err(|_| io::Error::other("simple-obfs tls: system RNG unavailable"))?;

    let server = server.as_bytes();
    let dlen = data.len();
    let slen = server.len();
    let mut buf = Vec::with_capacity(220 + dlen + slen);

    // TLS record: Handshake, version TLS 1.0, length.
    buf.extend_from_slice(&[0x16, 0x03, 0x01]);
    buf.extend_from_slice(&((212 + dlen + slen) as u16).to_be_bytes());

    // Handshake: ClientHello, 3-byte length, version TLS 1.2.
    buf.push(0x01);
    buf.push(0x00);
    buf.extend_from_slice(&((208 + dlen + slen) as u16).to_be_bytes());
    buf.extend_from_slice(&[0x03, 0x03]);

    // Random: 4-byte gmt_unix_time + 28 random bytes.
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0) as u32;
    buf.extend_from_slice(&now.to_be_bytes());
    buf.extend_from_slice(&random);

    // Session id (32 bytes).
    buf.push(0x20);
    buf.extend_from_slice(&session_id);

    // Cipher suites (28 bytes) + compression methods (null).
    buf.extend_from_slice(&[
        0x00, 0x1c, 0xc0, 0x2b, 0xc0, 0x2f, 0xcc, 0xa9, 0xcc, 0xa8, 0xcc, 0x14, 0xcc, 0x13, 0xc0, 0x0a, 0xc0, 0x14,
        0xc0, 0x09, 0xc0, 0x13, 0x00, 0x9c, 0x00, 0x35, 0x00, 0x2f, 0x00, 0x0a,
    ]);
    buf.extend_from_slice(&[0x01, 0x00]);

    // Extensions length, then the extensions themselves.
    buf.extend_from_slice(&((79 + dlen + slen) as u16).to_be_bytes());

    // SessionTicket (0x0023): carries the obfuscated payload.
    buf.extend_from_slice(&[0x00, 0x23]);
    buf.extend_from_slice(&(dlen as u16).to_be_bytes());
    buf.extend_from_slice(data);

    // server_name (0x0000): SNI.
    buf.extend_from_slice(&[0x00, 0x00]);
    buf.extend_from_slice(&((slen + 5) as u16).to_be_bytes());
    buf.extend_from_slice(&((slen + 3) as u16).to_be_bytes());
    buf.push(0x00);
    buf.extend_from_slice(&(slen as u16).to_be_bytes());
    buf.extend_from_slice(server);

    // ec_point_formats.
    buf.extend_from_slice(&[0x00, 0x0b, 0x00, 0x04, 0x03, 0x00, 0x01, 0x02]);
    // supported_groups.
    buf.extend_from_slice(&[
        0x00, 0x0a, 0x00, 0x0a, 0x00, 0x08, 0x00, 0x1d, 0x00, 0x17, 0x00, 0x19, 0x00, 0x18,
    ]);
    // signature_algorithms.
    buf.extend_from_slice(&[
        0x00, 0x0d, 0x00, 0x20, 0x00, 0x1e, 0x06, 0x01, 0x06, 0x02, 0x06, 0x03, 0x05, 0x01, 0x05, 0x02, 0x05, 0x03,
        0x04, 0x01, 0x04, 0x02, 0x04, 0x03, 0x03, 0x01, 0x03, 0x02, 0x03, 0x03, 0x02, 0x01, 0x02, 0x02, 0x02, 0x03,
    ]);
    // encrypt_then_mac.
    buf.extend_from_slice(&[0x00, 0x16, 0x00, 0x00]);
    // extended_master_secret.
    buf.extend_from_slice(&[0x00, 0x17, 0x00, 0x00]);

    Ok(buf)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The `ClientHello` must embed the payload in the SessionTicket extension
    /// and the host as SNI, with the extension-length field matching the actual
    /// extension bytes (the field a server can rely on to bound the record).
    #[test]
    fn client_hello_embeds_payload_and_sni() {
        let data = b"the-shadowsocks-salt";
        let server = "www.example.com";
        let hello = build_client_hello(data, server).unwrap();

        assert_eq!(&hello[0..3], &[0x16, 0x03, 0x01]);
        assert_eq!(&hello[5..7], &[0x01, 0x00]);

        // Extension-length field sits after the fixed ClientHello prefix.
        let ext_len = u16::from_be_bytes([hello[108], hello[109]]) as usize;
        assert_eq!(ext_len, 79 + data.len() + server.len());
        assert_eq!(hello.len(), 110 + ext_len);

        // First extension is the SessionTicket carrying the payload.
        assert_eq!(&hello[110..112], &[0x00, 0x23]);
        let ticket_len = u16::from_be_bytes([hello[112], hello[113]]) as usize;
        assert_eq!(ticket_len, data.len());
        assert_eq!(&hello[114..114 + data.len()], data);

        // SNI carries the host verbatim.
        let needle = server.as_bytes();
        assert!(
            hello.windows(needle.len()).any(|w| w == needle),
            "ClientHello should contain the SNI host"
        );
    }
}
