//! AnyTLS outbound.
//!
//! AnyTLS ("any TLS") rides a normal TLS connection and runs a small session
//! layer on top whose purpose is traffic-shaping: it multiplexes logical
//! streams inside one TLS connection and (optionally) pads records so their
//! sizes do not leak the proxied protocol. The transport (tcp/ws/grpc/…) and
//! security (tls/reality) layers are provided by [`crate::transport`] via the
//! shared [`crate::transport::build_layers`]; this module is purely the AnyTLS
//! session framing on top. AnyTLS is TLS-by-default (the whole point), so
//! security defaults to TLS unless overridden.
//!
//! Wire format (after the TLS handshake completes), per the upstream spec
//! (`anytls/anytls-go` `docs/protocol.md`):
//!
//! 1. **Authentication** — the client immediately sends
//!    `SHA256(password) (32) | padding0_len (u16 BE) | padding0`.
//! 2. **Session frames** — `cmd(1) | streamId(u32 BE) | len(u16 BE) | data`.
//!    The client must send `cmdSettings` first, then opens a stream with
//!    `cmdSYN`, writes the proxy target as a SOCKS5 address (RFC 1928 §5) in a
//!    `cmdPSH`, and relays the payload as further `cmdPSH` frames. `cmdFIN`
//!    marks EOF. A v2 server answers `cmdSYN` with `cmdSYNACK` (empty = ok, data
//!    = error text) and `cmdSettings` with `cmdServerSettings`.
//!
//! The kernel opens one TLS connection (one session, one stream) per outbound
//! connection — analogous to Trojan — rather than pooling/reusing sessions; a
//! fresh session always creating a new stream is conformant (it is just the
//! "no idle session" branch of the reuse rule). Padding-scheme traffic shaping
//! is not applied (padding0 is empty and no `cmdWaste` frames are emitted); it
//! is an anti-detection nicety, not required for the relay to interoperate, and
//! the advertised `padding-md5` matches the upstream default so a stock server
//! does not bother pushing an update. UDP (sing-box udp-over-tcp v2) is not
//! implemented yet, so AnyTLS serves TCP only.

use std::collections::VecDeque;
use std::io;
use std::pin::Pin;
use std::task::{Context as TaskContext, Poll, ready};

use anyhow::{Context, Result};
use md5::Md5;
use sha2::{Digest, Sha256};
use tokio::io::{AsyncRead, AsyncWrite, AsyncWriteExt, ReadBuf};

use crate::address::TargetAddr;
use crate::config::outbound_opts::ProxyEntry;
use crate::inbound::socks5;
use crate::outbound::BoxedStream;
use crate::transport::{self, Security, Transport};

// Session-layer commands (anytls protocol, "since version 1" + "since version 2").
const CMD_WASTE: u8 = 0;
const CMD_SYN: u8 = 1;
const CMD_PSH: u8 = 2;
const CMD_FIN: u8 = 3;
const CMD_SETTINGS: u8 = 4;
const CMD_ALERT: u8 = 5;
const CMD_UPDATE_PADDING_SCHEME: u8 = 6;
const CMD_SYNACK: u8 = 7;
const CMD_HEART_REQUEST: u8 = 8;
const CMD_HEART_RESPONSE: u8 = 9;
const CMD_SERVER_SETTINGS: u8 = 10;

const FRAME_HEADER_LEN: usize = 7;
/// The single stream opened per session. anytls stream ids are monotonic within
/// a session; with one stream per connection it is always the first id.
const STREAM_ID: u32 = 1;
/// Cap on a single `cmdPSH` payload (the frame length field is a `u16`); a
/// comfortable margin keeps frames small without excessive overhead.
const MAX_PSH_CHUNK: usize = 8192;

/// Implemented protocol version reported in `cmdSettings` (`v=2`).
const PROTOCOL_VERSION: u8 = 2;
/// `client` identifier reported in `cmdSettings` (real name, per the spec —
/// spoofing it is pointless).
const CLIENT_NAME: &str = concat!("learn-gripe/", env!("CARGO_PKG_VERSION"));

/// The upstream default padding scheme (anytls-go `proxy/padding/padding.go`).
/// We do not apply it, but advertise its md5 so a stock server does not push an
/// update. No trailing newline — it must hash identically to the upstream bytes.
const DEFAULT_PADDING_SCHEME: &str = "stop=8\n\
0=30-30\n\
1=100-400\n\
2=400-500,c,500-1000,c,500-1000,c,500-1000,c,500-1000\n\
3=9-9,500-1000\n\
4=500-1000\n\
5=500-1000\n\
6=500-1000\n\
7=500-1000";

/// Fully-resolved AnyTLS outbound parameters.
///
/// `security` and `transport` are orthogonal layers (see [`crate::transport`]).
/// The password is pre-hashed into its 32-byte `SHA256` form — exactly the
/// on-wire authenticator — so the dial path never touches the raw secret again.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AnyTlsOutboundConfig {
    pub server: String,
    pub port: u16,
    pub password_sha256: [u8; 32],
    pub security: Security,
    pub transport: Transport,
}

impl AnyTlsOutboundConfig {
    /// Build an outbound config from a parsed `anytls` proxy entry.
    pub fn from_proxy(entry: &ProxyEntry) -> Result<Self> {
        let opts = &entry.options;
        let server = opts
            .server
            .clone()
            .filter(|s| !s.is_empty())
            .context("anytls: missing server")?;
        let port = opts.port.context("anytls: missing port")?;
        let password = opts
            .password
            .as_deref()
            .filter(|s| !s.is_empty())
            .context("anytls: missing password")?;
        let password_sha256 = Sha256::digest(password.as_bytes()).into();

        // AnyTLS always rides TLS; security and transport are orthogonal to the
        // session framing and are built by the shared layer helper.
        let (security, transport) = transport::build_layers(opts, "anytls", true, false)?;

        Ok(Self {
            server,
            port,
            password_sha256,
            security,
            transport,
        })
    }
}

/// Connect an AnyTLS outbound to `target`: establish the TLS transport, send the
/// auth + `cmdSettings` + `cmdSYN` + `cmdPSH`(target address), and hand back a
/// stream that frames relay traffic as `cmdPSH` and decodes the server's frames.
pub async fn connect(config: &AnyTlsOutboundConfig, target: &TargetAddr) -> Result<BoxedStream> {
    let mut stream = transport::establish(&config.server, config.port, &config.security, &config.transport).await?;
    let hello = build_client_hello(&config.password_sha256, target);
    stream
        .write_all(&hello)
        .await
        .context("anytls: send auth + open stream")?;
    Ok(Box::new(AnyTlsStream::new(stream)))
}

/// The lowercase-hex md5 of the advertised padding scheme, for `cmdSettings`.
fn padding_scheme_md5() -> String {
    let digest = Md5::digest(DEFAULT_PADDING_SCHEME.as_bytes());
    let mut out = String::with_capacity(32);
    for byte in digest {
        out.push_str(&format!("{byte:02x}"));
    }
    out
}

/// Append one session frame (`cmd | streamId | len | data`) to `buf`.
fn push_frame(buf: &mut Vec<u8>, cmd: u8, stream_id: u32, data: &[u8]) {
    buf.push(cmd);
    buf.extend_from_slice(&stream_id.to_be_bytes());
    buf.extend_from_slice(&(data.len() as u16).to_be_bytes());
    buf.extend_from_slice(data);
}

/// Build the bytes the client sends right after the TLS handshake: the auth
/// header (no padding0), then `cmdSettings`, `cmdSYN`, and the `cmdPSH` carrying
/// the SOCKS5-encoded proxy target.
fn build_client_hello(password_sha256: &[u8; 32], target: &TargetAddr) -> Vec<u8> {
    let mut buf = Vec::with_capacity(32 + 2 + 64 + FRAME_HEADER_LEN * 2 + 64);
    // Authentication: SHA256(password) then a zero-length padding0.
    buf.extend_from_slice(password_sha256);
    buf.extend_from_slice(&0u16.to_be_bytes());

    let settings = format!(
        "v={PROTOCOL_VERSION}\nclient={CLIENT_NAME}\npadding-md5={}",
        padding_scheme_md5()
    );
    push_frame(&mut buf, CMD_SETTINGS, 0, settings.as_bytes());

    push_frame(&mut buf, CMD_SYN, STREAM_ID, &[]);

    let mut addr = Vec::with_capacity(1 + 256 + 2);
    socks5::encode_address(&mut addr, target);
    push_frame(&mut buf, CMD_PSH, STREAM_ID, &addr);
    buf
}

/// Session-layer stream over the TLS transport: relay writes become `cmdPSH`
/// frames; reads strip the framing and surface only this stream's `cmdPSH`
/// payload, handling the control frames (`cmdSYNACK`/`cmdFIN`/`cmdAlert`/
/// `cmdHeartRequest`/padding) transparently.
struct AnyTlsStream<S> {
    inner: S,
    /// Outgoing framed bytes pending write to the inner transport (user `cmdPSH`
    /// frames plus any control replies such as `cmdHeartResponse`). Frames are
    /// appended whole, so draining never splits a frame.
    out: VecDeque<u8>,
    /// Raw bytes read from the inner transport not yet parsed into frames.
    read_raw: Vec<u8>,
    /// Decoded `cmdPSH` payload pending delivery to the reader.
    plain: Vec<u8>,
    plain_pos: usize,
    eof: bool,
    fin_sent: bool,
}

impl<S> AnyTlsStream<S> {
    fn new(inner: S) -> Self {
        Self {
            inner,
            out: VecDeque::new(),
            read_raw: Vec::new(),
            plain: Vec::new(),
            plain_pos: 0,
            eof: false,
            fin_sent: false,
        }
    }
}

impl<S: AsyncWrite + Unpin> AnyTlsStream<S> {
    /// Flush queued outgoing frames to the inner transport.
    fn poll_drain(&mut self, cx: &mut TaskContext<'_>) -> Poll<io::Result<()>> {
        while !self.out.is_empty() {
            let (front, _) = self.out.as_slices();
            let n = ready!(Pin::new(&mut self.inner).poll_write(cx, front))?;
            if n == 0 {
                return Poll::Ready(Err(io::Error::new(io::ErrorKind::WriteZero, "anytls: write zero")));
            }
            self.out.drain(..n);
        }
        Poll::Ready(Ok(()))
    }
}

impl<S: AsyncRead + AsyncWrite + Unpin> AsyncRead for AnyTlsStream<S> {
    fn poll_read(self: Pin<&mut Self>, cx: &mut TaskContext<'_>, buf: &mut ReadBuf<'_>) -> Poll<io::Result<()>> {
        let this = self.get_mut();
        // Best-effort flush of any queued control replies (e.g. heart responses
        // produced while parsing). Errors/pending here do not block the read.
        let _ = this.poll_drain(cx);

        loop {
            if this.plain_pos < this.plain.len() {
                let n = buf.remaining().min(this.plain.len() - this.plain_pos);
                buf.put_slice(&this.plain[this.plain_pos..this.plain_pos + n]);
                this.plain_pos += n;
                return Poll::Ready(Ok(()));
            }
            if this.eof {
                return Poll::Ready(Ok(()));
            }

            // Need a full frame: a 7-byte header, then its `len` body bytes.
            let need = if this.read_raw.len() < FRAME_HEADER_LEN {
                FRAME_HEADER_LEN
            } else {
                FRAME_HEADER_LEN + u16::from_be_bytes([this.read_raw[5], this.read_raw[6]]) as usize
            };
            if this.read_raw.len() < need {
                let mut scratch = [0u8; 4096];
                let mut read_buf = ReadBuf::new(&mut scratch);
                ready!(Pin::new(&mut this.inner).poll_read(cx, &mut read_buf))?;
                let filled = read_buf.filled();
                if filled.is_empty() {
                    // Peer closed the transport; treat as end of this stream.
                    this.eof = true;
                    return Poll::Ready(Ok(()));
                }
                this.read_raw.extend_from_slice(filled);
                continue;
            }

            let cmd = this.read_raw[0];
            let stream_id =
                u32::from_be_bytes([this.read_raw[1], this.read_raw[2], this.read_raw[3], this.read_raw[4]]);
            let len = u16::from_be_bytes([this.read_raw[5], this.read_raw[6]]) as usize;
            let data: Vec<u8> = this.read_raw[FRAME_HEADER_LEN..FRAME_HEADER_LEN + len].to_vec();
            this.read_raw.drain(..FRAME_HEADER_LEN + len);

            match cmd {
                CMD_PSH if stream_id == STREAM_ID => {
                    this.plain = data;
                    this.plain_pos = 0;
                }
                CMD_FIN if stream_id == STREAM_ID => {
                    this.eof = true;
                    return Poll::Ready(Ok(()));
                }
                CMD_SYNACK if !data.is_empty() => {
                    return Poll::Ready(Err(io::Error::new(
                        io::ErrorKind::ConnectionRefused,
                        format!("anytls: stream rejected: {}", String::from_utf8_lossy(&data)),
                    )));
                }
                CMD_ALERT => {
                    return Poll::Ready(Err(io::Error::other(format!(
                        "anytls: server alert: {}",
                        String::from_utf8_lossy(&data)
                    ))));
                }
                CMD_HEART_REQUEST => {
                    push_frame_deque(&mut this.out, CMD_HEART_RESPONSE, stream_id, &[]);
                    let _ = this.poll_drain(cx);
                }
                // Padding, settings, padding-scheme updates, heart responses, the
                // stream's own SYN/SYNACK(ok), and frames for other streams carry
                // nothing this single-stream relay needs: read past them.
                CMD_WASTE
                | CMD_SETTINGS
                | CMD_SERVER_SETTINGS
                | CMD_UPDATE_PADDING_SCHEME
                | CMD_HEART_RESPONSE
                | CMD_SYN
                | CMD_SYNACK
                | CMD_PSH
                | CMD_FIN => {}
                _ => {}
            }
        }
    }
}

impl<S: AsyncWrite + Unpin> AsyncWrite for AnyTlsStream<S> {
    fn poll_write(self: Pin<&mut Self>, cx: &mut TaskContext<'_>, buf: &[u8]) -> Poll<io::Result<usize>> {
        let this = self.get_mut();
        ready!(this.poll_drain(cx))?;
        if buf.is_empty() {
            return Poll::Ready(Ok(0));
        }
        let take = buf.len().min(MAX_PSH_CHUNK);
        push_frame_deque(&mut this.out, CMD_PSH, STREAM_ID, &buf[..take]);
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
        if !this.fin_sent {
            push_frame_deque(&mut this.out, CMD_FIN, STREAM_ID, &[]);
            this.fin_sent = true;
        }
        ready!(this.poll_drain(cx))?;
        Pin::new(&mut this.inner).poll_shutdown(cx)
    }
}

/// Append one session frame to a `VecDeque` (the outgoing queue variant of
/// [`push_frame`]).
fn push_frame_deque(buf: &mut VecDeque<u8>, cmd: u8, stream_id: u32, data: &[u8]) {
    buf.push_back(cmd);
    buf.extend(stream_id.to_be_bytes());
    buf.extend((data.len() as u16).to_be_bytes());
    buf.extend(data.iter().copied());
}

#[cfg(test)]
mod tests {
    use std::net::{Ipv4Addr, SocketAddr};

    use super::*;
    use crate::config::outbound_opts::ProxyEntry;
    use crate::transport::tls::ClientFingerprint;

    fn parse_entry(yaml: &str) -> ProxyEntry {
        serde_yaml_ng::from_str(yaml).expect("parse proxy entry")
    }

    #[test]
    fn padding_md5_matches_upstream_default_scheme() {
        // Cross-checked against `md5sum` of anytls-go's default padding scheme.
        assert_eq!(padding_scheme_md5(), "75cff2ad89aadf5e257059ee571ebe11");
    }

    #[test]
    fn client_hello_carries_auth_settings_syn_and_target() {
        let password_sha256: [u8; 32] = Sha256::digest(b"secret").into();
        let target = TargetAddr::Domain("example.com".to_string(), 443);
        let hello = build_client_hello(&password_sha256, &target);

        // Auth: SHA256(password) then a zero padding0 length.
        assert_eq!(&hello[..32], &password_sha256);
        assert_eq!(&hello[32..34], &0u16.to_be_bytes());

        // cmdSettings frame (sid 0) with v / client / padding-md5.
        let mut pos = 34;
        assert_eq!(hello[pos], CMD_SETTINGS);
        assert_eq!(&hello[pos + 1..pos + 5], &0u32.to_be_bytes());
        let settings_len = u16::from_be_bytes([hello[pos + 5], hello[pos + 6]]) as usize;
        let settings = &hello[pos + FRAME_HEADER_LEN..pos + FRAME_HEADER_LEN + settings_len];
        let settings = std::str::from_utf8(settings).unwrap();
        assert!(settings.contains("v=2"), "{settings}");
        assert!(
            settings.contains("padding-md5=75cff2ad89aadf5e257059ee571ebe11"),
            "{settings}"
        );
        pos += FRAME_HEADER_LEN + settings_len;

        // cmdSYN frame for the stream, no data.
        assert_eq!(hello[pos], CMD_SYN);
        assert_eq!(&hello[pos + 1..pos + 5], &STREAM_ID.to_be_bytes());
        assert_eq!(&hello[pos + 5..pos + 7], &0u16.to_be_bytes());
        pos += FRAME_HEADER_LEN;

        // cmdPSH frame carrying the SOCKS5-encoded target.
        assert_eq!(hello[pos], CMD_PSH);
        assert_eq!(&hello[pos + 1..pos + 5], &STREAM_ID.to_be_bytes());
        let addr_len = u16::from_be_bytes([hello[pos + 5], hello[pos + 6]]) as usize;
        let mut expected = Vec::new();
        socks5::encode_address(&mut expected, &target);
        assert_eq!(
            &hello[pos + FRAME_HEADER_LEN..pos + FRAME_HEADER_LEN + addr_len],
            &expected[..]
        );
    }

    #[test]
    fn defaults_to_tls_security() {
        let yaml = "name: a\ntype: anytls\nserver: example.com\nport: 443\npassword: secret\n";
        let cfg = AnyTlsOutboundConfig::from_proxy(&parse_entry(yaml)).unwrap();
        assert!(matches!(cfg.security, Security::Tls(_)));
        assert!(matches!(cfg.transport, Transport::Tcp));
        assert_eq!(cfg.password_sha256, <[u8; 32]>::from(Sha256::digest(b"secret")));
    }

    #[test]
    fn missing_password_is_rejected() {
        let yaml = "name: a\ntype: anytls\nserver: example.com\nport: 443\n";
        let err = AnyTlsOutboundConfig::from_proxy(&parse_entry(yaml)).unwrap_err();
        assert!(err.to_string().contains("password"), "got: {err}");
    }

    #[test]
    fn missing_server_is_rejected() {
        let yaml = "name: a\ntype: anytls\nport: 443\npassword: secret\n";
        let err = AnyTlsOutboundConfig::from_proxy(&parse_entry(yaml)).unwrap_err();
        assert!(err.to_string().contains("server"), "got: {err}");
    }

    #[test]
    fn sni_and_skip_cert_verify_flow_into_tls() {
        let yaml = "name: a\ntype: anytls\nserver: example.com\nport: 443\npassword: secret\n\
             sni: real.example\nskip-cert-verify: true\nclient-fingerprint: chrome\n";
        let cfg = AnyTlsOutboundConfig::from_proxy(&parse_entry(yaml)).unwrap();
        match cfg.security {
            Security::Tls(tls) => {
                assert_eq!(tls.server_name.as_deref(), Some("real.example"));
                assert!(tls.skip_cert_verify);
                assert_eq!(tls.client_fingerprint, Some(ClientFingerprint::Chrome));
            }
            other => panic!("expected TLS security, got {other:?}"),
        }
    }
}
