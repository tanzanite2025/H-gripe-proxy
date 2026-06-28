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
//! UDP relay reuses the same authenticated connection but carries datagrams as
//! QUIC datagram frames (the server advertises support with `Hysteria-UDP: true`
//! in the auth response):
//! ```text
//! UDP datagram: SESSION(4) PACKET(2) FRAG_ID(1) FRAG_COUNT(1)
//!               varint(addr_len) addr  PAYLOAD
//! ```
//! A payload too large for one QUIC datagram is split across fragments that
//! share a packet id; reassembly lives in [`crate::protocols::quic_udp`].
//!
//! Scope: TCP relay plus UDP relay over a single connection. **Salamander
//! packet obfuscation** (`obfs: salamander`) and **port hopping** (`ports`) are
//! implemented below QUIC in [`crate::transport::quic_obfs`]. **0-RTT**
//! (`reduce-rtt`) sends the HTTP/3 authentication and `TCPRequest` as early data
//! once a TLS session ticket is cached; the auth header carries no exporter
//! secret, so it is 0-RTT-safe. `congestion-controller` is honored as a local
//! send-rate choice; `up`/`down` bandwidth caps are not sent (server-side BBR is
//! used).

use std::io;
use std::pin::Pin;
use std::sync::Mutex;
use std::sync::atomic::{AtomicU16, Ordering};
use std::task::{Context as TaskContext, Poll};

use anyhow::{Context, Result, anyhow, bail};
use bytes::Bytes;
use h3::client::SendRequest;
use quinn::{Connection, Endpoint, RecvStream, SendStream};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, ReadBuf};

use crate::address::TargetAddr;
use crate::config::outbound_opts::ProxyEntry;
use crate::outbound::BoxedStream;
use crate::protocols::quic_udp::{self, Reassembler};
use crate::protocols::salamander::Salamander;
use crate::transport::quic::{self, Congestion, QuicClientParams};
use crate::transport::quic_obfs::{PacketObfs, PortHopConfig};

/// `obfs` value selecting Salamander packet obfuscation (the only mode mihomo /
/// Hysteria2 define).
const OBFS_SALAMANDER: &str = "salamander";

/// Hysteria2 `TCPRequest` frame id.
const FRAME_TCP_REQUEST: u64 = 0x401;
/// HyOK: the HTTP/3 status a server returns on successful authentication.
const STATUS_AUTH_OK: u16 = 233;
/// Cap on server-sent `TCPResponse` message / padding lengths (defensive).
const MAX_RESPONSE_FIELD: u64 = 64 * 1024;
/// Fixed-size prefix of a UDP datagram: SESSION(4) PACKET(2) FRAG_ID(1) FRAG_COUNT(1).
const UDP_HEADER_PREFIX: usize = 8;

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
    /// Salamander packet obfuscation derived from `obfs: salamander` +
    /// `obfs-password`, or `None` when obfuscation is off.
    pub obfs: Option<Salamander>,
    /// Port hopping derived from `ports` (+ `hop-interval`), or `None` to always
    /// dial the configured `port`.
    pub port_hop: Option<PortHopConfig>,
    /// Attempt a 0-RTT handshake (`reduce-rtt`): send authentication and the
    /// `TCPRequest` as early data on a resumed connection.
    pub reduce_rtt: bool,
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

        // Salamander obfuscation XOR-masks every QUIC datagram (see
        // `transport::quic_obfs`); only the `salamander` mode is defined, and it
        // requires a non-empty `obfs-password`.
        let obfs = match opts.obfs.as_deref().filter(|s| !s.is_empty()) {
            None => None,
            Some(mode) if mode.eq_ignore_ascii_case(OBFS_SALAMANDER) => {
                let password = opts
                    .obfs_password
                    .clone()
                    .filter(|s| !s.is_empty())
                    .context("hysteria2: obfs salamander requires obfs-password")?;
                Some(Salamander::new(password.into_bytes()))
            }
            Some(mode) => bail!("hysteria2: obfs {mode:?} not supported (only \"salamander\")"),
        };

        // Port hopping spreads datagrams across a range of server ports; the
        // configured `port` stays the address quinn associates with the peer.
        let port_hop = match opts.ports.as_deref().filter(|s| !s.is_empty()) {
            None => None,
            Some(spec) => Some(PortHopConfig::parse(spec, opts.hop_interval)?),
        };

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
            obfs,
            port_hop,
            reduce_rtt: opts.reduce_rtt.unwrap_or(false),
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
            obfs: self.obfs.clone().map(PacketObfs::Salamander),
            port_hop: self.port_hop.clone(),
            zero_rtt: self.reduce_rtt,
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

    // With `reduce-rtt` the QUIC connection may be a 0-RTT one. The HTTP/3 `/auth`
    // POST and the `TCPRequest` are both idempotent and carry no exporter secret,
    // so they are safe to send as early data: `tcp_handshake` issues those writes
    // immediately, which `quinn` flushes in the 0-RTT first flight, and only the
    // response reads wait for the handshake. If the server rejects 0-RTT the
    // early streams are reset (so the attempt errors), so confirm acceptance and
    // rebuild the handshake on the established 1-RTT connection.
    let handshake = match quic.zero_rtt {
        Some(accepted) => {
            let attempt = tcp_handshake(&connection, config, target).await;
            if accepted.await {
                attempt.context("hysteria2: 0-RTT handshake")?
            } else {
                drop(attempt);
                tcp_handshake(&connection, config, target)
                    .await
                    .context("hysteria2: handshake after 0-RTT rejection")?
            }
        }
        None => tcp_handshake(&connection, config, target)
            .await
            .context("hysteria2: handshake")?,
    };

    Ok(Box::new(Hysteria2Stream {
        _endpoint: quic.endpoint,
        _connection: connection,
        _h3_send: handshake.h3_send,
        send: handshake.send,
        recv: handshake.recv,
    }))
}

/// The live stream handles produced by a Hysteria2 TCP handshake.
struct TcpHandshake {
    /// HTTP/3 request handle kept alive for the connection's lifetime.
    h3_send: SendRequest<h3_quinn::OpenStreams, Bytes>,
    send: SendStream,
    recv: RecvStream,
}

/// Authenticate over HTTP/3 and open the proxy `TCPRequest` stream for `target`.
/// All writes happen before any response read, so on a 0-RTT connection the
/// request rides as early data while only the reads await the handshake.
async fn tcp_handshake(
    connection: &Connection,
    config: &Hysteria2OutboundConfig,
    target: &TargetAddr,
) -> Result<TcpHandshake> {
    let (h3_send, _udp_enabled) = authenticate(connection.clone(), config)
        .await
        .context("hysteria2: authenticate")?;

    let (mut send, mut recv) = connection.open_bi().await.context("hysteria2: open proxy stream")?;
    let request = encode_tcp_request(target);
    send.write_all(&request).await.context("hysteria2: send TCPRequest")?;
    read_tcp_response(&mut recv)
        .await
        .context("hysteria2: read TCPResponse")?;

    Ok(TcpHandshake { h3_send, send, recv })
}

/// Connect a Hysteria2 UDP relay session for `target`. Authenticates over
/// HTTP/3 (requiring the server to advertise UDP support), then carries each
/// datagram as one or more QUIC datagram frames addressed to `target`.
pub async fn connect_udp(config: &Hysteria2OutboundConfig, target: &TargetAddr) -> Result<Hysteria2Udp> {
    let quic = quic::connect(&config.quic_params())
        .await
        .context("hysteria2: QUIC connect")?;
    let connection = quic.connection.clone();

    // UDP relay rides QUIC datagrams that need the established 1-RTT keys, so
    // wait out any 0-RTT handshake before authenticating.
    if let Some(accepted) = quic.zero_rtt {
        accepted.await;
    }
    let (h3_send, udp_enabled) = authenticate(quic.connection, config)
        .await
        .context("hysteria2: authenticate")?;
    if !udp_enabled {
        bail!("hysteria2: server does not allow UDP relay (Hysteria-UDP: false)");
    }

    Ok(Hysteria2Udp {
        _endpoint: quic.endpoint,
        connection,
        _h3_send: h3_send,
        address: address_string(target),
        session_id: random_u32(),
        next_packet_id: AtomicU16::new(0),
        reassembler: Mutex::new(Reassembler::new()),
    })
}

/// Authenticate over HTTP/3 and return the live `SendRequest` handle (which the
/// caller must keep alive for the connection's lifetime — dropping the last one
/// closes the QUIC connection) plus whether the server advertised UDP relay
/// support via the `Hysteria-UDP` response header.
async fn authenticate(
    connection: Connection,
    config: &Hysteria2OutboundConfig,
) -> Result<(SendRequest<h3_quinn::OpenStreams, Bytes>, bool)> {
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
    // The server reports UDP availability with `Hysteria-UDP: true|false`; treat
    // an absent header as enabled (lenient), only an explicit `false` disables it.
    let udp_enabled = response
        .headers()
        .get("hysteria-udp")
        .map(|value| !value.as_bytes().eq_ignore_ascii_case(b"false"))
        .unwrap_or(true);
    Ok((send_request, udp_enabled))
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

/// Number of bytes [`put_varint`] uses to encode `value`.
fn varint_len(value: u64) -> usize {
    if value < (1 << 6) {
        1
    } else if value < (1 << 14) {
        2
    } else if value < (1 << 30) {
        4
    } else {
        8
    }
}

/// Read a QUIC variable-length integer from `data` at `*pos`, advancing `*pos`.
/// Returns `None` if the buffer is truncated.
fn read_varint_slice(data: &[u8], pos: &mut usize) -> Option<u64> {
    let first = *data.get(*pos)?;
    let len = 1usize << (first >> 6);
    if *pos + len > data.len() {
        return None;
    }
    let mut value = (first & 0x3f) as u64;
    for &b in &data[*pos + 1..*pos + len] {
        value = (value << 8) | b as u64;
    }
    *pos += len;
    Some(value)
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

/// A random 32-bit UDP session id.
fn random_u32() -> u32 {
    let mut bytes = [0u8; 4];
    getrandom::fill(&mut bytes).expect("os rng");
    u32::from_be_bytes(bytes)
}

/// Encode a UDP payload into one or more Hysteria2 datagram fragments, each
/// sized to fit within `max_datagram`. Every fragment repeats the session id,
/// packet id, and address; the payload is split across the `FRAG_COUNT`
/// fragments.
fn encode_udp_datagrams(
    session_id: u32,
    packet_id: u16,
    address: &str,
    payload: &[u8],
    max_datagram: usize,
) -> Result<Vec<Bytes>> {
    let overhead = UDP_HEADER_PREFIX + varint_len(address.len() as u64) + address.len();
    let chunk_size = max_datagram
        .checked_sub(overhead)
        .filter(|n| *n > 0)
        .ok_or_else(|| anyhow!("hysteria2: QUIC datagram too small for UDP header ({max_datagram} bytes)"))?;

    let chunks = quic_udp::fragments(payload, chunk_size);
    let frag_count =
        u8::try_from(chunks.len()).map_err(|_| anyhow!("hysteria2: UDP payload needs too many fragments"))?;

    let mut datagrams = Vec::with_capacity(chunks.len());
    for (frag_id, chunk) in chunks.into_iter().enumerate() {
        let mut buf = Vec::with_capacity(overhead + chunk.len());
        buf.extend_from_slice(&session_id.to_be_bytes());
        buf.extend_from_slice(&packet_id.to_be_bytes());
        buf.push(frag_id as u8);
        buf.push(frag_count);
        put_varint(&mut buf, address.len() as u64);
        buf.extend_from_slice(address.as_bytes());
        buf.extend_from_slice(chunk);
        datagrams.push(Bytes::from(buf));
    }
    Ok(datagrams)
}

/// Parse an inbound Hysteria2 UDP datagram into `(packet_id, frag_id,
/// frag_count, payload)`. The session id and address are skipped (a session has
/// one fixed target here). Returns `None` for a malformed datagram.
fn parse_udp_datagram(data: &[u8]) -> Option<(u16, u8, u8, Vec<u8>)> {
    if data.len() < UDP_HEADER_PREFIX {
        return None;
    }
    let packet_id = u16::from_be_bytes([data[4], data[5]]);
    let frag_id = data[6];
    let frag_count = data[7];
    let mut pos = UDP_HEADER_PREFIX;
    let addr_len = read_varint_slice(data, &mut pos)? as usize;
    pos = pos.checked_add(addr_len).filter(|p| *p <= data.len())?;
    Some((packet_id, frag_id, frag_count, data[pos..].to_vec()))
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

/// A Hysteria2 UDP relay session bound to a single target. Datagrams to/from the
/// target are carried as QUIC datagram frames over the authenticated connection.
///
/// The owning [`Endpoint`], [`Connection`], and HTTP/3 [`SendRequest`] are held
/// so the connection stays alive for the session's lifetime.
pub struct Hysteria2Udp {
    _endpoint: Endpoint,
    connection: Connection,
    _h3_send: SendRequest<h3_quinn::OpenStreams, Bytes>,
    /// The fixed target address (`host:port`) all datagrams in this session carry.
    address: String,
    session_id: u32,
    next_packet_id: AtomicU16,
    reassembler: Mutex<Reassembler>,
}

impl Hysteria2Udp {
    /// Send one UDP datagram to the session target, fragmenting if it does not
    /// fit a single QUIC datagram.
    pub async fn send(&self, payload: &[u8]) -> Result<()> {
        let max = self
            .connection
            .max_datagram_size()
            .ok_or_else(|| anyhow!("hysteria2: peer does not allow QUIC datagrams (UDP relay unavailable)"))?;
        let packet_id = self.next_packet_id.fetch_add(1, Ordering::Relaxed);
        for datagram in encode_udp_datagrams(self.session_id, packet_id, &self.address, payload, max)? {
            self.connection
                .send_datagram(datagram)
                .map_err(|e| anyhow!("hysteria2: send UDP datagram: {e}"))?;
        }
        Ok(())
    }

    /// Receive the next fully reassembled UDP datagram from the target.
    pub async fn recv(&self) -> Result<Vec<u8>> {
        loop {
            let datagram = self
                .connection
                .read_datagram()
                .await
                .context("hysteria2: read UDP datagram")?;
            let Some((packet_id, frag_id, frag_count, payload)) = parse_udp_datagram(&datagram) else {
                continue;
            };
            if let Some(full) = self
                .reassembler
                .lock()
                .expect("reassembler mutex poisoned")
                .accept(packet_id, frag_id, frag_count, payload)
            {
                return Ok(full);
            }
        }
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
    fn salamander_obfs_requires_password() {
        let yaml = "name: h\ntype: hysteria2\nserver: example.com\nport: 443\npassword: pw\nobfs: salamander\n";
        let err = Hysteria2OutboundConfig::from_proxy(&parse_entry(yaml)).unwrap_err();
        assert!(err.to_string().contains("obfs-password"), "got: {err}");
    }

    #[test]
    fn salamander_obfs_is_accepted_with_password() {
        let yaml = "name: h\ntype: hysteria2\nserver: example.com\nport: 443\npassword: pw\n\
                    obfs: Salamander\nobfs-password: s3cret\n";
        let cfg = Hysteria2OutboundConfig::from_proxy(&parse_entry(yaml)).unwrap();
        assert_eq!(cfg.obfs, Some(Salamander::new(b"s3cret".to_vec())));
        assert!(cfg.port_hop.is_none());
    }

    #[test]
    fn unknown_obfs_is_rejected() {
        let yaml = "name: h\ntype: hysteria2\nserver: example.com\nport: 443\npassword: pw\n\
                    obfs: rprx\nobfs-password: x\n";
        let err = Hysteria2OutboundConfig::from_proxy(&parse_entry(yaml)).unwrap_err();
        assert!(err.to_string().contains("not supported"), "got: {err}");
    }

    #[test]
    fn ports_enable_port_hopping() {
        let yaml = "name: h\ntype: hysteria2\nserver: example.com\nport: 443\npassword: pw\n\
                    ports: \"30000-40000,50000\"\nhop-interval: 20\n";
        let cfg = Hysteria2OutboundConfig::from_proxy(&parse_entry(yaml)).unwrap();
        assert!(cfg.port_hop.is_some(), "ports should enable hopping");
    }

    #[test]
    fn invalid_ports_are_rejected() {
        let yaml = "name: h\ntype: hysteria2\nserver: example.com\nport: 443\npassword: pw\nports: \"abc\"\n";
        let err = Hysteria2OutboundConfig::from_proxy(&parse_entry(yaml)).unwrap_err();
        assert!(err.to_string().contains("port"), "got: {err}");
    }

    #[test]
    fn missing_password_is_rejected() {
        let yaml = "name: h\ntype: hysteria2\nserver: example.com\nport: 443\n";
        let err = Hysteria2OutboundConfig::from_proxy(&parse_entry(yaml)).unwrap_err();
        assert!(err.to_string().contains("password"), "got: {err}");
    }

    #[test]
    fn reduce_rtt_defaults_off_and_parses() {
        let base = "name: h\ntype: hysteria2\nserver: example.com\nport: 443\npassword: pw\n";
        assert!(
            !Hysteria2OutboundConfig::from_proxy(&parse_entry(base))
                .unwrap()
                .reduce_rtt
        );
        let yaml = format!("{base}reduce-rtt: true\n");
        assert!(
            Hysteria2OutboundConfig::from_proxy(&parse_entry(&yaml))
                .unwrap()
                .reduce_rtt
        );
    }

    #[test]
    fn udp_datagram_round_trips_single_fragment() {
        let datagrams = encode_udp_datagrams(0xDEAD_BEEF, 7, "example.com:443", b"hello", 1200).unwrap();
        assert_eq!(datagrams.len(), 1);
        // Header: session(4) packet(2) frag_id(1) frag_count(1).
        let d = &datagrams[0];
        assert_eq!(&d[0..4], &0xDEAD_BEEFu32.to_be_bytes());
        assert_eq!(&d[4..6], &7u16.to_be_bytes());
        assert_eq!(d[6], 0); // frag id
        assert_eq!(d[7], 1); // frag count
        let (packet_id, frag_id, frag_count, payload) = parse_udp_datagram(d).unwrap();
        assert_eq!((packet_id, frag_id, frag_count), (7, 0, 1));
        assert_eq!(payload, b"hello");
    }

    #[test]
    fn udp_datagram_fragments_and_reassembles() {
        // A small max datagram forces several fragments; every fragment repeats
        // the address, and reassembly restores the original payload in order.
        let payload: Vec<u8> = (0..1000u32).map(|i| i as u8).collect();
        let datagrams = encode_udp_datagrams(1, 42, "1.2.3.4:53", &payload, 64).unwrap();
        assert!(datagrams.len() > 1, "expected fragmentation");

        let mut reassembler = Reassembler::new();
        let mut recovered = None;
        for d in &datagrams {
            let (packet_id, frag_id, frag_count, frag_payload) = parse_udp_datagram(d).unwrap();
            assert_eq!(packet_id, 42);
            assert_eq!(frag_count as usize, datagrams.len());
            if let Some(full) = reassembler.accept(packet_id, frag_id, frag_count, frag_payload) {
                recovered = Some(full);
            }
        }
        assert_eq!(recovered, Some(payload));
    }

    #[test]
    fn udp_datagram_rejects_oversized_header() {
        // A max datagram smaller than the header leaves no room for payload.
        let err = encode_udp_datagrams(1, 0, "example.com:443", b"x", 4).unwrap_err();
        assert!(err.to_string().contains("too small"), "got: {err}");
    }
}
