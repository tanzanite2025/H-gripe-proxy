//! Hysteria2 outbound (QUIC data plane).
//!
//! Hysteria2 runs a proxy over a single TLS-encrypted QUIC connection, but
//! authenticates with an **HTTP/3** request before any proxying:
//!
//! 1. Dial the server over QUIC (TLS 1.3, ALPN `h3`) via
//!    [`crate::transport::quic`].
//! 2. Authenticate over HTTP/3: send `POST https://<authority>/auth` carrying
//!    `Hysteria-Auth: <password>`, `Hysteria-CC-RX: 0` (let the server pick its
//!    own congestion controller, i.e. BBR), and a random `Hysteria-Padding`.
//!    A `233` (HyOK) response means success. The HTTP/3 wire codec (QPACK,
//!    framing) is delegated to the vetted [`h3`]/[`h3_quinn`] crates.
//! 3. For each target, open a **raw** QUIC bidirectional stream (not HTTP/3),
//!    send a `TCPRequest` (frame id `0x401`, address string `host:port`,
//!    padding), read the `TCPResponse` status, then relay raw bytes.
//!
//! The proxy streams share the QUIC connection with the HTTP/3 connection, so
//! the returned stream keeps the HTTP/3 [`SendRequest`] handle alive: dropping
//! the last one makes `h3` close the whole QUIC connection (`H3_NO_ERROR`),
//! which would tear down the proxy stream.
//!
//! Wire format (QUIC varints, big-endian):
//! ```text
//! TCPRequest:   varint(0x401)  varint(addr_len) addr  varint(pad_len) pad
//! TCPResponse:  STATUS(1)      varint(msg_len)  msg   varint(pad_len) pad
//!               ; STATUS 0x00 = OK, otherwise the connect failed
//! ```
//!
//! Scope: TCP relay over a single connection. UDP relay (QUIC datagrams),
//! Salamander packet obfuscation (`obfs: salamander`), port hopping (`ports`),
//! and 0-RTT are not implemented; `obfs` is rejected at config-build time rather
//! than mis-dialed. `congestion-controller` is honored as a local send-rate
//! choice; `up`/`down` bandwidth caps are not sent (server-side BBR is used).

use std::io;
use std::pin::Pin;
use std::task::{Context as TaskContext, Poll};

use anyhow::{Context, Result, bail};
use bytes::Bytes;
use h3::client::SendRequest;
use quinn::{Connection, Endpoint, RecvStream, SendStream};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, ReadBuf};

use crate::address::TargetAddr;
use crate::config::outbound_opts::ProxyEntry;
use crate::outbound::BoxedStream;
use crate::transport::quic::{self, Congestion, QuicClientParams};

/// Hysteria2 `TCPRequest` frame id.
const FRAME_TCP_REQUEST: u64 = 0x401;
/// HyOK: the HTTP/3 status a server returns on successful authentication.
const STATUS_AUTH_OK: u16 = 233;
/// Cap on server-sent `TCPResponse` message / padding lengths (defensive).
const MAX_RESPONSE_FIELD: u64 = 64 * 1024;

/// Fully-resolved Hysteria2 outbound parameters.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Hysteria2OutboundConfig {
    pub server: String,
    pub port: u16,
    /// Authentication string sent in the `Hysteria-Auth` header.
    pub password: String,
    /// TLS SNI / certificate name and HTTP/3 `:authority` (`sni`, falling back
    /// to `servername`/server).
    pub server_name: String,
    pub alpn: Vec<String>,
    pub skip_cert_verify: bool,
    pub congestion: Congestion,
}

impl Hysteria2OutboundConfig {
    /// Build an outbound config from a parsed `hysteria2` proxy entry.
    pub fn from_proxy(entry: &ProxyEntry) -> Result<Self> {
        let opts = &entry.options;
        let server = opts
            .server
            .clone()
            .filter(|s| !s.is_empty())
            .context("hysteria2: missing server")?;
        let port = opts.port.context("hysteria2: missing port")?;
        let password = opts
            .password
            .clone()
            .filter(|s| !s.is_empty())
            .context("hysteria2: missing password")?;

        // Salamander obfuscation wraps the QUIC datagrams themselves; it needs a
        // custom UDP socket and is not implemented, so reject rather than dial a
        // connection the server will silently drop.
        if let Some(obfs) = opts.obfs.as_deref().filter(|s| !s.is_empty()) {
            bail!("hysteria2: obfs {obfs:?} not implemented yet (salamander packet obfuscation)");
        }

        // SNI precedence: explicit `sni`, then `servername`, then the dial host.
        let server_name = opts
            .sni
            .clone()
            .or_else(|| opts.servername.clone())
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| server.clone());

        // QUIC always carries ALPN; Hysteria2 uses "h3".
        let alpn = match &opts.alpn {
            Some(list) if !list.is_empty() => list.clone(),
            _ => vec!["h3".to_string()],
        };

        let congestion = opts
            .congestion_controller
            .as_deref()
            .map(Congestion::parse)
            .unwrap_or(Congestion::Bbr);

        Ok(Self {
            server,
            port,
            password,
            server_name,
            alpn,
            skip_cert_verify: opts.skip_cert_verify.unwrap_or(false),
            congestion,
        })
    }

    fn quic_params(&self) -> QuicClientParams {
        QuicClientParams {
            server: self.server.clone(),
            port: self.port,
            server_name: self.server_name.clone(),
            alpn: self.alpn.clone(),
            skip_cert_verify: self.skip_cert_verify,
            congestion: self.congestion,
        }
    }
}

/// Connect a Hysteria2 outbound to `target` and return a relay-ready stream. The
/// QUIC connection is authenticated over HTTP/3 and the `TCPRequest`/response
/// handshake is complete, so the caller relays payload bytes directly.
pub async fn connect(config: &Hysteria2OutboundConfig, target: &TargetAddr) -> Result<BoxedStream> {
    let quic = quic::connect(&config.quic_params())
        .await
        .context("hysteria2: QUIC connect")?;
    // The proxy stream and the HTTP/3 auth share one QUIC connection; `quinn`
    // connection handles are cheap clones over the same connection.
    let connection = quic.connection.clone();

    let h3_send = authenticate(quic.connection, config)
        .await
        .context("hysteria2: authenticate")?;

    let (mut send, mut recv) = connection.open_bi().await.context("hysteria2: open proxy stream")?;
    let request = encode_tcp_request(target);
    send.write_all(&request).await.context("hysteria2: send TCPRequest")?;
    read_tcp_response(&mut recv)
        .await
        .context("hysteria2: read TCPResponse")?;

    Ok(Box::new(Hysteria2Stream {
        _endpoint: quic.endpoint,
        _connection: connection,
        _h3_send: h3_send,
        send,
        recv,
    }))
}

/// Authenticate over HTTP/3 and return the live `SendRequest` handle, which the
/// caller must keep alive for the connection's lifetime (dropping the last one
/// closes the QUIC connection).
async fn authenticate(
    connection: Connection,
    config: &Hysteria2OutboundConfig,
) -> Result<SendRequest<h3_quinn::OpenStreams, Bytes>> {
    let h3_conn = h3_quinn::Connection::new(connection);
    let (mut driver, mut send_request) = h3::client::new(h3_conn).await.context("hysteria2: HTTP/3 handshake")?;
    // The HTTP/3 connection must be driven for the auth response to arrive; the
    // driver resolves (and the task ends) once the connection closes.
    tokio::spawn(async move {
        let _ = std::future::poll_fn(|cx| driver.poll_close(cx)).await;
    });

    let uri = format!("https://{}/auth", config.server_name);
    let request = http::Request::builder()
        .method(http::Method::POST)
        .uri(&uri)
        .header("Hysteria-Auth", &config.password)
        .header("Hysteria-CC-RX", "0")
        .header("Hysteria-Padding", random_padding())
        .body(())
        .context("hysteria2: build /auth request")?;

    let mut stream = send_request
        .send_request(request)
        .await
        .context("hysteria2: send /auth request")?;
    stream.finish().await.context("hysteria2: finish /auth request")?;
    let response = stream.recv_response().await.context("hysteria2: recv /auth response")?;
    let status = response.status().as_u16();
    if status != STATUS_AUTH_OK {
        bail!("hysteria2: authentication rejected (HTTP status {status}, expected {STATUS_AUTH_OK})");
    }
    Ok(send_request)
}

/// Encode a `TCPRequest`: frame id, target `host:port`, and random padding.
fn encode_tcp_request(target: &TargetAddr) -> Vec<u8> {
    let address = address_string(target);
    let padding = random_padding();
    let mut buf = Vec::with_capacity(8 + address.len() + padding.len());
    put_varint(&mut buf, FRAME_TCP_REQUEST);
    put_varint(&mut buf, address.len() as u64);
    buf.extend_from_slice(address.as_bytes());
    put_varint(&mut buf, padding.len() as u64);
    buf.extend_from_slice(padding.as_bytes());
    buf
}

/// Read and validate a `TCPResponse` header, leaving `recv` positioned at the
/// relayed payload.
async fn read_tcp_response(recv: &mut RecvStream) -> Result<()> {
    let mut status = [0u8; 1];
    recv.read_exact(&mut status).await.context("read status")?;

    let msg_len = read_varint(recv).await.context("read message length")?;
    if msg_len > MAX_RESPONSE_FIELD {
        bail!("hysteria2: TCPResponse message too long ({msg_len} bytes)");
    }
    let mut message = vec![0u8; msg_len as usize];
    recv.read_exact(&mut message).await.context("read message")?;

    let pad_len = read_varint(recv).await.context("read padding length")?;
    if pad_len > MAX_RESPONSE_FIELD {
        bail!("hysteria2: TCPResponse padding too long ({pad_len} bytes)");
    }
    let mut padding = vec![0u8; pad_len as usize];
    recv.read_exact(&mut padding).await.context("read padding")?;

    if status[0] != 0x00 {
        bail!(
            "hysteria2: server rejected connect (status {}): {}",
            status[0],
            String::from_utf8_lossy(&message)
        );
    }
    Ok(())
}

/// Format a target as a Hysteria2 address string (`host:port`; IPv6 bracketed).
fn address_string(target: &TargetAddr) -> String {
    match target {
        TargetAddr::Ip(addr) => addr.to_string(),
        TargetAddr::Domain(host, port) => format!("{host}:{port}"),
    }
}

/// Append a QUIC variable-length integer (RFC 9000 §16) to `buf`.
fn put_varint(buf: &mut Vec<u8>, value: u64) {
    if value < (1 << 6) {
        buf.push(value as u8);
    } else if value < (1 << 14) {
        buf.extend_from_slice(&((value as u16) | 0x4000).to_be_bytes());
    } else if value < (1 << 30) {
        buf.extend_from_slice(&((value as u32) | 0x8000_0000).to_be_bytes());
    } else {
        buf.extend_from_slice(&(value | 0xC000_0000_0000_0000).to_be_bytes());
    }
}

/// Read a QUIC variable-length integer from an async byte stream.
async fn read_varint<R: AsyncRead + Unpin>(reader: &mut R) -> io::Result<u64> {
    let mut first = [0u8; 1];
    reader.read_exact(&mut first).await?;
    let len = 1usize << (first[0] >> 6);
    let mut value = (first[0] & 0x3f) as u64;
    let mut rest = [0u8; 7];
    reader.read_exact(&mut rest[..len - 1]).await?;
    for &b in &rest[..len - 1] {
        value = (value << 8) | b as u64;
    }
    Ok(value)
}

/// A short random alphanumeric padding string (printable, so it doubles as a
/// valid HTTP header value).
fn random_padding() -> String {
    const CHARSET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789";
    let mut seed = [0u8; 64];
    getrandom::fill(&mut seed).expect("os rng");
    let len = 16 + (seed[0] as usize % 48);
    seed[1..=len]
        .iter()
        .map(|b| CHARSET[*b as usize % CHARSET.len()] as char)
        .collect()
}

/// A relay-ready stream over a Hysteria2 proxy QUIC stream.
///
/// The owning [`Endpoint`], the QUIC [`Connection`], and the HTTP/3
/// [`SendRequest`] are held so their drivers / the connection stay alive for the
/// relay's lifetime; reads and writes delegate to the QUIC stream halves.
struct Hysteria2Stream {
    _endpoint: Endpoint,
    _connection: Connection,
    _h3_send: SendRequest<h3_quinn::OpenStreams, Bytes>,
    send: SendStream,
    recv: RecvStream,
}

impl AsyncRead for Hysteria2Stream {
    fn poll_read(mut self: Pin<&mut Self>, cx: &mut TaskContext<'_>, buf: &mut ReadBuf<'_>) -> Poll<io::Result<()>> {
        AsyncRead::poll_read(Pin::new(&mut self.recv), cx, buf)
    }
}

impl AsyncWrite for Hysteria2Stream {
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

#[cfg(test)]
mod tests {
    use std::net::SocketAddr;

    use super::*;
    use crate::config::outbound_opts::ProxyEntry;

    fn parse_entry(yaml: &str) -> ProxyEntry {
        serde_yaml_ng::from_str(yaml).expect("parse proxy entry")
    }

    #[test]
    fn encodes_varint_boundaries() {
        let mut buf = Vec::new();
        put_varint(&mut buf, 0x401);
        // 0x401 = 1025 fits in the 2-byte form (0x4000 | 0x0401).
        assert_eq!(buf, vec![0x44, 0x01]);

        let mut small = Vec::new();
        put_varint(&mut small, 37);
        assert_eq!(small, vec![37]);
    }

    #[tokio::test]
    async fn varint_round_trips() {
        for value in [0u64, 1, 63, 64, 1025, 16383, 16384, 1_000_000, 1 << 30] {
            let mut buf = Vec::new();
            put_varint(&mut buf, value);
            let mut cursor = std::io::Cursor::new(buf);
            assert_eq!(read_varint(&mut cursor).await.unwrap(), value);
        }
    }

    #[test]
    fn encodes_tcp_request_header() {
        let target = TargetAddr::Domain("example.com".to_string(), 443);
        let buf = encode_tcp_request(&target);
        assert_eq!(&buf[..2], &[0x44, 0x01]); // frame id 0x401
        assert_eq!(buf[2], "example.com:443".len() as u8); // address length varint (<64)
        assert_eq!(&buf[3..3 + 15], b"example.com:443");
    }

    #[test]
    fn ipv6_address_is_bracketed() {
        let target = TargetAddr::Ip("[::1]:443".parse::<SocketAddr>().unwrap());
        assert_eq!(address_string(&target), "[::1]:443");
    }

    #[test]
    fn padding_is_printable_and_bounded() {
        for _ in 0..32 {
            let pad = random_padding();
            assert!((16..64).contains(&pad.len()));
            assert!(pad.bytes().all(|b| b.is_ascii_alphanumeric()));
        }
    }

    #[test]
    fn parses_minimal_config_with_defaults() {
        let yaml = "name: h\ntype: hysteria2\nserver: example.com\nport: 443\npassword: secret\n";
        let cfg = Hysteria2OutboundConfig::from_proxy(&parse_entry(yaml)).unwrap();
        assert_eq!(cfg.server, "example.com");
        assert_eq!(cfg.port, 443);
        assert_eq!(cfg.password, "secret");
        assert_eq!(cfg.server_name, "example.com");
        assert_eq!(cfg.alpn, vec!["h3".to_string()]);
        assert_eq!(cfg.congestion, Congestion::Bbr);
        assert!(!cfg.skip_cert_verify);
    }

    #[test]
    fn honors_sni_alpn_and_skip_verify() {
        let yaml = "name: h\ntype: hysteria2\nserver: 1.2.3.4\nport: 8443\npassword: pw\n\
                    sni: hidden.example\nalpn:\n  - h3\nskip-cert-verify: true\ncongestion-controller: cubic\n";
        let cfg = Hysteria2OutboundConfig::from_proxy(&parse_entry(yaml)).unwrap();
        assert_eq!(cfg.server_name, "hidden.example");
        assert_eq!(cfg.alpn, vec!["h3".to_string()]);
        assert_eq!(cfg.congestion, Congestion::Cubic);
        assert!(cfg.skip_cert_verify);
    }

    #[test]
    fn obfs_is_rejected() {
        let yaml = "name: h\ntype: hysteria2\nserver: example.com\nport: 443\npassword: pw\nobfs: salamander\n";
        let err = Hysteria2OutboundConfig::from_proxy(&parse_entry(yaml)).unwrap_err();
        assert!(err.to_string().contains("obfs"), "got: {err}");
    }

    #[test]
    fn missing_password_is_rejected() {
        let yaml = "name: h\ntype: hysteria2\nserver: example.com\nport: 443\n";
        let err = Hysteria2OutboundConfig::from_proxy(&parse_entry(yaml)).unwrap_err();
        assert!(err.to_string().contains("password"), "got: {err}");
    }
}
