//! v2ray-plugin **quic** mode transport.
//!
//! The upstream `shadowsocks/v2ray-plugin` Go binary's `mode=quic` carries the
//! Shadowsocks byte stream over a standard QUIC (TLS 1.3) connection: TLS is
//! mandatory, the `host` opt is the SNI / certificate name, and each logical
//! Shadowsocks connection rides one bidirectional QUIC stream. The plugin never
//! exposes v2ray's `security` / `header` knobs, so with their `none` defaults
//! the wire is plain QUIC — we reuse the kernel's vetted
//! [`crate::transport::quic`] dialer and only add a [`BoxedStream`] adapter over
//! the stream halves plus per-target connection reuse: QUIC's built-in
//! multiplexing lets many Shadowsocks streams share a single handshake.

use std::collections::HashMap;
use std::io;
use std::pin::Pin;
use std::sync::{Arc, Mutex, OnceLock, Weak};
use std::task::{Context as TaskContext, Poll};

use anyhow::{Context, Result};
use quinn::{RecvStream, SendStream};
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};

use crate::outbound::BoxedStream;
use crate::transport::quic::{self, Congestion, QuicClientParams, QuicConnection};

/// Resolved v2ray-plugin quic-mode parameters.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct V2rayQuicConfig {
    /// TLS SNI / certificate name (`host`). Empty falls back to the dial server.
    pub server_name: String,
    /// ALPN protocols to offer. v2ray-core's TLS layer defaults to
    /// `["h2", "http/1.1"]` when none is configured, which is what the plugin
    /// presents on the QUIC handshake.
    pub alpn: Vec<String>,
    /// Accept any server certificate (`skip-cert-verify`).
    pub skip_cert_verify: bool,
}

/// Identity under which a live QUIC connection may be reused by later dials.
#[derive(Clone, PartialEq, Eq, Hash)]
struct PoolKey {
    server: String,
    port: u16,
    server_name: String,
    alpn: Vec<String>,
    skip_cert_verify: bool,
}

/// Process-wide registry of live QUIC connections keyed by [`PoolKey`]. Entries
/// are weak: a connection lives only while a relay still holds a stream over it,
/// and the registry simply lets a concurrent dial to the same server reuse that
/// handshake instead of opening a second one.
fn pool() -> &'static Mutex<HashMap<PoolKey, Weak<QuicConnection>>> {
    static POOL: OnceLock<Mutex<HashMap<PoolKey, Weak<QuicConnection>>>> = OnceLock::new();
    POOL.get_or_init(|| Mutex::new(HashMap::new()))
}

/// Return a pooled connection for `key` when one is still alive and usable.
fn take_live(key: &PoolKey) -> Option<Arc<QuicConnection>> {
    let map = pool().lock().expect("v2ray-plugin quic pool poisoned");
    let conn = map.get(key)?.upgrade()?;
    // A connection that has begun closing (idle timeout, peer reset) can no
    // longer open streams; let the caller dial a fresh one.
    if conn.connection.close_reason().is_none() {
        Some(conn)
    } else {
        None
    }
}

/// Register `conn` so concurrent dials to the same server can reuse it.
fn store(key: PoolKey, conn: &Arc<QuicConnection>) {
    pool()
        .lock()
        .expect("v2ray-plugin quic pool poisoned")
        .insert(key, Arc::downgrade(conn));
}

/// Dial (or reuse) a QUIC connection to `server:port` and open one bidirectional
/// stream for a Shadowsocks relay to run its salt + AEAD chunks over.
pub async fn connect(cfg: &V2rayQuicConfig, server: &str, port: u16) -> Result<BoxedStream> {
    let server_name = if cfg.server_name.is_empty() {
        server.to_string()
    } else {
        cfg.server_name.clone()
    };
    let key = PoolKey {
        server: server.to_string(),
        port,
        server_name: server_name.clone(),
        alpn: cfg.alpn.clone(),
        skip_cert_verify: cfg.skip_cert_verify,
    };

    let conn = match take_live(&key) {
        Some(conn) => conn,
        None => {
            let params = QuicClientParams {
                server: server.to_string(),
                port,
                server_name,
                alpn: cfg.alpn.clone(),
                skip_cert_verify: cfg.skip_cert_verify,
                congestion: Congestion::Bbr,
                obfs: None,
                port_hop: None,
                zero_rtt: false,
            };
            let conn = Arc::new(
                quic::connect(&params)
                    .await
                    .with_context(|| format!("v2ray-plugin quic: dial {server}:{port}"))?,
            );
            store(key, &conn);
            conn
        }
    };

    let (send, recv) = conn
        .connection
        .open_bi()
        .await
        .context("v2ray-plugin quic: open stream")?;

    Ok(Box::new(QuicBiStream {
        _conn: conn,
        send,
        recv,
    }))
}

/// A relay-ready view over one bidirectional QUIC stream. The owning
/// [`QuicConnection`] (endpoint + connection) is held via `Arc` so its
/// background driver — and the pooled connection — stay alive for as long as any
/// stream over it is in use.
struct QuicBiStream {
    _conn: Arc<QuicConnection>,
    send: SendStream,
    recv: RecvStream,
}

impl AsyncRead for QuicBiStream {
    fn poll_read(mut self: Pin<&mut Self>, cx: &mut TaskContext<'_>, buf: &mut ReadBuf<'_>) -> Poll<io::Result<()>> {
        AsyncRead::poll_read(Pin::new(&mut self.recv), cx, buf)
    }
}

impl AsyncWrite for QuicBiStream {
    fn poll_write(mut self: Pin<&mut Self>, cx: &mut TaskContext<'_>, buf: &[u8]) -> Poll<io::Result<usize>> {
        AsyncWrite::poll_write(Pin::new(&mut self.send), cx, buf)
    }

    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut TaskContext<'_>) -> Poll<io::Result<()>> {
        AsyncWrite::poll_flush(Pin::new(&mut self.send), cx)
    }

    fn poll_shutdown(mut self: Pin<&mut Self>, cx: &mut TaskContext<'_>) -> Poll<io::Result<()>> {
        AsyncWrite::poll_shutdown(Pin::new(&mut self.send), cx)
    }
}
