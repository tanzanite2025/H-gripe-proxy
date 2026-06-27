//! End-to-end proof for the simple-obfs **tls** (fake-TLS) plugin mode:
//! SOCKS5 client -> gripe inbound -> Shadowsocks outbound (over obfs-tls) ->
//! fake obfs-tls server -> shared fake SS server.
//!
//! The fake server is an *independent* implementation of the simple-obfs TLS
//! framing: it parses the client's fake `ClientHello`, recovers the embedded
//! payload from the `SessionTicket` extension, replies with a fixed 105-byte
//! fake handshake, and then exchanges TLS application-data records. The
//! recovered payload plus the de-framed record stream are handed to the shared
//! Shadowsocks AEAD server in [`common`], so this proves the obfs-tls framing
//! composes correctly with the real Shadowsocks layer.
//!
//! The `ClientHello` is parsed using the (correct) extension-length field rather
//! than the outer record/handshake length constants, so the server locates the
//! SessionTicket robustly even though clash-compatible clients leave those outer
//! lengths slightly off.

mod common;

use std::io;
use std::net::{Ipv4Addr, SocketAddr};
use std::pin::Pin;
use std::task::{Context as TaskContext, Poll, ready};

use common::{assert_relays, serve_shadowsocks, ss_plugin_config};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, ReadBuf};
use tokio::net::{TcpListener, TcpStream};

/// Largest payload per TLS application-data record (matches the client).
const CHUNK_SIZE: usize = 1 << 14;
const APP_DATA_HEADER: [u8; 3] = [0x17, 0x03, 0x03];
/// Byte offset of the 2-byte extensions-length field within the `ClientHello`.
const EXTENSIONS_LEN_OFFSET: usize = 108;

// --- fake obfs-tls server --------------------------------------------------

/// Read the full `ClientHello`, returning `(embedded_payload, leftover)` where
/// `embedded_payload` is the SessionTicket data and `leftover` is any byte that
/// arrived after the `ClientHello` (the start of the application-data stream).
async fn read_client_hello(tcp: &mut TcpStream) -> Option<(Vec<u8>, Vec<u8>)> {
    let mut buf = Vec::new();
    let mut tmp = [0u8; 2048];

    // Enough to read the extensions-length field.
    while buf.len() < EXTENSIONS_LEN_OFFSET + 2 {
        let n = tcp.read(&mut tmp).await.ok()?;
        if n == 0 {
            return None;
        }
        buf.extend_from_slice(&tmp[..n]);
    }
    let ext_len = u16::from_be_bytes([buf[EXTENSIONS_LEN_OFFSET], buf[EXTENSIONS_LEN_OFFSET + 1]]) as usize;
    let ch_end = EXTENSIONS_LEN_OFFSET + 2 + ext_len;

    while buf.len() < ch_end {
        let n = tcp.read(&mut tmp).await.ok()?;
        if n == 0 {
            return None;
        }
        buf.extend_from_slice(&tmp[..n]);
    }

    // Walk the extension list and pull out the SessionTicket (0x0023) data.
    let mut embedded = None;
    let mut i = EXTENSIONS_LEN_OFFSET + 2;
    while i + 4 <= ch_end {
        let etype = u16::from_be_bytes([buf[i], buf[i + 1]]);
        let elen = u16::from_be_bytes([buf[i + 2], buf[i + 3]]) as usize;
        let data_start = i + 4;
        if data_start + elen > ch_end {
            break;
        }
        if etype == 0x0023 {
            embedded = Some(buf[data_start..data_start + elen].to_vec());
        }
        i = data_start + elen;
    }

    let leftover = buf[ch_end..].to_vec();
    Some((embedded?, leftover))
}

async fn serve_obfs_tls(mut tcp: TcpStream) {
    let Some((embedded, leftover)) = read_client_hello(&mut tcp).await else {
        return;
    };
    serve_shadowsocks(ObfsTlsServerStream::new(tcp, embedded, leftover)).await;
}

/// Server-side simple-obfs TLS adapter: presents the de-framed Shadowsocks byte
/// stream to [`serve_shadowsocks`]. Reads strip TLS application-data record
/// headers (seeded with the `ClientHello` payload); the first write emits the
/// fixed fake handshake response before the data records.
struct ObfsTlsServerStream {
    inner: TcpStream,
    first_response: bool,

    write_buf: Vec<u8>,
    write_off: usize,

    raw: Vec<u8>,
    record_remaining: usize,
    out: Vec<u8>,
    out_pos: usize,
    saw_eof: bool,
}

impl ObfsTlsServerStream {
    fn new(inner: TcpStream, embedded: Vec<u8>, leftover: Vec<u8>) -> Self {
        Self {
            inner,
            first_response: true,
            write_buf: Vec::new(),
            write_off: 0,
            raw: leftover,
            record_remaining: 0,
            out: embedded,
            out_pos: 0,
            saw_eof: false,
        }
    }

    fn decode_step(&mut self) -> bool {
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
        let len = u16::from_be_bytes([self.raw[3], self.raw[4]]) as usize;
        self.raw.drain(..5);
        self.record_remaining = len;
        true
    }

    fn poll_flush_write_buf(&mut self, cx: &mut TaskContext<'_>) -> Poll<io::Result<()>> {
        while self.write_off < self.write_buf.len() {
            let n = ready!(Pin::new(&mut self.inner).poll_write(cx, &self.write_buf[self.write_off..]))?;
            if n == 0 {
                return Poll::Ready(Err(io::Error::new(io::ErrorKind::WriteZero, "wrote zero")));
            }
            self.write_off += n;
        }
        self.write_buf.clear();
        self.write_off = 0;
        Poll::Ready(Ok(()))
    }
}

/// The fixed 105-byte fake handshake response (ServerHello + ChangeCipherSpec +
/// Finished). The client skips exactly this many bytes, so the contents are
/// inert filler shaped as valid-looking TLS records.
fn server_handshake() -> Vec<u8> {
    let mut out = Vec::with_capacity(105);
    // ServerHello record (79 bytes): record header + 74-byte handshake body.
    out.extend_from_slice(&[0x16, 0x03, 0x03, 0x00, 0x4a]);
    out.extend_from_slice(&[0x02, 0x00, 0x00, 0x46, 0x03, 0x03]);
    out.extend_from_slice(&[0u8; 32]); // random
    out.push(0x20);
    out.extend_from_slice(&[0u8; 32]); // session id
    out.extend_from_slice(&[0xc0, 0x2f]); // cipher suite
    out.push(0x00); // compression
    // ChangeCipherSpec record (6 bytes).
    out.extend_from_slice(&[0x14, 0x03, 0x03, 0x00, 0x01, 0x01]);
    // Finished record (20 bytes): header + 15-byte opaque body.
    out.extend_from_slice(&[0x16, 0x03, 0x03, 0x00, 0x0f]);
    out.extend_from_slice(&[0u8; 15]);
    debug_assert_eq!(out.len(), 105);
    out
}

fn encode_records(data: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(data.len() + (data.len() / CHUNK_SIZE + 1) * 5);
    for chunk in data.chunks(CHUNK_SIZE) {
        out.extend_from_slice(&APP_DATA_HEADER);
        out.extend_from_slice(&(chunk.len() as u16).to_be_bytes());
        out.extend_from_slice(chunk);
    }
    out
}

impl AsyncRead for ObfsTlsServerStream {
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

impl AsyncWrite for ObfsTlsServerStream {
    fn poll_write(self: Pin<&mut Self>, cx: &mut TaskContext<'_>, buf: &[u8]) -> Poll<io::Result<usize>> {
        let this = self.get_mut();
        ready!(this.poll_flush_write_buf(cx))?;
        if buf.is_empty() {
            return Poll::Ready(Ok(0));
        }

        let mut encoded = Vec::new();
        if this.first_response {
            encoded.extend_from_slice(&server_handshake());
            this.first_response = false;
        }
        encoded.extend_from_slice(&encode_records(buf));
        this.write_buf = encoded;
        this.write_off = 0;

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

async fn spawn_obfs_tls_server() -> SocketAddr {
    let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        while let Ok((tcp, _)) = listener.accept().await {
            tokio::spawn(serve_obfs_tls(tcp));
        }
    });
    addr
}

// --- tests -----------------------------------------------------------------

#[tokio::test]
async fn relays_through_obfs_tls() {
    let server = spawn_obfs_tls_server().await;
    let plugin = "plugin: obfs\nplugin-opts:\n  mode: tls\n  host: www.bing.com\n";
    assert_relays(
        ss_plugin_config(server, plugin),
        b"hello shadowsocks over simple-obfs tls",
    )
    .await;
}

#[tokio::test]
async fn relays_large_payload_over_obfs_tls() {
    // Larger than one 16 KiB TLS record to exercise multi-record chunking in
    // both directions.
    let server = spawn_obfs_tls_server().await;
    let plugin = "plugin: obfs\nplugin-opts:\n  mode: tls\n  host: www.bing.com\n";
    let payload: Vec<u8> = (0..50_000u32).map(|i| (i % 251) as u8).collect();
    assert_relays(ss_plugin_config(server, plugin), &payload).await;
}
