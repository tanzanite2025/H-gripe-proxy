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
//! The kernel multiplexes streams over per-server sessions: each session is one
//! live TLS connection driven by a background task that owns the transport,
//! demultiplexing inbound `cmdPSH`/`cmdFIN` to each logical stream and
//! serialising every stream's writes through the per-session padding shaper. A
//! new outbound connection opens another stream on an existing session to the
//! same server (a fresh `cmdSYN` with the next id) — running concurrently with
//! that session's other streams — and only does a new TLS handshake + auth when
//! no session has a free slot (`MAX_STREAMS_PER_SESSION`). A session also stays
//! registered while idle (no open streams) so a later connection reuses it
//! instead of handshaking, expiring on an idle TTL; once broken or idle-expired
//! it is evicted and its connection closed. Because the shared reader stalls all
//! of a session's streams while one stream's bounded inbound buffer is full,
//! per-session fan-out is capped (matching anytls-go's bounded per-stream pipe).
//!
//! **Padding-scheme traffic shaping** is applied, matching upstream
//! (`anytls-go` `proxy/session/session.go` `writeConn` + `proxy/padding`). The
//! client advertises the default scheme's `padding-md5` and shapes its writes
//! by it: `padding0` zero bytes ride the auth header (packet 0), and the first
//! `stop` "TLS packets" (a packet = one `writeConn` flush) are split/padded to
//! the scheme's per-packet record sizes, inserting `cmdWaste` frames to fill
//! short writes and emitting standalone `cmdWaste` records where the scheme
//! calls for pure padding. Packet 1 is the combined `cmdSettings` +
//! `cmdSYN` + `cmdPSH(target)` flush; packet 2 onward are the relay's data
//! writes. Padding is byte-level shaping the server discards transparently
//! (`cmdWaste` is dropped, frame boundaries are recovered from the length
//! field), so it never affects interop — it only obscures record sizes. A
//! server-pushed `cmdUpdatePaddingScheme` is parsed and stored per server (keyed
//! by `server:port`): the current connection keeps shaping by its own scheme,
//! but subsequent connections to that server advertise and shape by the updated
//! scheme, exactly as anytls-go's per-server `Client` does. A stock server does
//! not push one anyway since the advertised md5 already matches the default.
//! UDP rides sing-box udp-over-tcp v2 (see [`connect_udp`]).

mod frame;
mod padding;
mod session;
mod stream;

#[cfg(test)]
mod tests;

use anyhow::{Context, Result};
use sha2::{Digest, Sha256};
use tokio::io::AsyncWriteExt;

use crate::address::TargetAddr;
use crate::config::outbound_opts::ProxyEntry;
use crate::inbound::socks5;
use crate::outbound::BoxedStream;
use crate::transport::{self, Security, Transport};

use frame::build_auth_header;
use padding::{ServerKey, current_scheme};
use session::{open_on, register_session, spawn_session, take_reusable};
use stream::MuxStream;

/// udp-over-tcp v2 magic destination (sing `common/uot`). A `cmdPSH` to this
/// FQDN tells the server the stream carries UoT-framed datagrams rather than a
/// raw TCP relay.
const UOT_MAGIC_ADDRESS: &str = "sp.v2.udp-over-tcp.arpa";

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
/// auth header (packet 0, padding0 from the scheme), then the padded packet-1
/// flush of `cmdSettings` + `cmdSYN` + `cmdPSH`(target address), and hand back a
/// stream that frames relay traffic as `cmdPSH` and decodes the server's frames.
pub async fn connect(config: &AnyTlsOutboundConfig, target: &TargetAddr) -> Result<BoxedStream> {
    Ok(Box::new(acquire_stream(config, target).await?))
}

/// Acquire an AnyTLS stream to `target`: open another stream on a live
/// registered session for the config's server if one has a free slot (concurrent
/// multiplexing or idle reuse), otherwise establish a new TLS session (handshake
/// then auth, after which the driver writes `cmdSettings` on its first stream).
/// Either way the returned stream has its `cmdSYN` + `cmdPSH`(target) queued and
/// is ready to relay.
async fn acquire_stream(config: &AnyTlsOutboundConfig, target: &TargetAddr) -> Result<MuxStream> {
    let key = ServerKey {
        server: config.server.clone(),
        port: config.port,
    };

    // Reuse path: open another stream on an existing session (its driver assigns
    // the next id and writes `cmdSYN` + `cmdPSH`). On failure the session just
    // broke; release the reserved slot and fall through to a fresh connection.
    if let Some(handle) = take_reusable(&key) {
        match open_on(&handle, target).await {
            Ok(stream) => return Ok(stream),
            Err(_) => handle.release_slot(),
        }
    }

    // New-session path: TLS handshake + auth (packet 0). The spawned driver then
    // writes the padded packet-1 flush of `cmdSettings` + `cmdSYN` +
    // `cmdPSH`(target) when this first stream is opened, as anytls-go does after
    // `OpenStream` clears buffering.
    let scheme = current_scheme(&key);
    let mut transport = transport::establish(&config.server, config.port, &config.security, &config.transport).await?;
    transport
        .write_all(&build_auth_header(&config.password_sha256, &scheme))
        .await
        .context("anytls: send auth header")?;
    let handle = spawn_session(transport, (*scheme).clone(), key.clone());
    register_session(key, handle.clone());
    open_on(&handle, target).await.context("anytls: open first stream")
}

/// Open an AnyTLS outbound for UDP datagrams to `target` via udp-over-tcp v2
/// (sing `common/uot`). The session stream is opened to the UoT magic address;
/// the first application bytes are the UoT *connect* request (`IsConnect=1` +
/// SOCKS5 destination), after which every datagram is framed as `len(u16 BE) |
/// payload` in both directions (connect mode carries no per-packet address).
/// One stream is opened per destination, matching the relay's per-target model.
pub async fn connect_udp(config: &AnyTlsOutboundConfig, target: &TargetAddr) -> Result<BoxedStream> {
    // Open the session stream to the UoT magic address (new or reused), then send
    // the UoT connect request as its first application bytes.
    let magic = TargetAddr::Domain(UOT_MAGIC_ADDRESS.to_string(), 0);
    let mut anytls = acquire_stream(config, &magic).await?;
    // UoT v2 request: IsConnect (1) + SOCKS5-encoded destination. Sent as the
    // stream's first `cmdPSH` payload.
    let mut request = Vec::with_capacity(1 + 1 + 256 + 2);
    request.push(1u8); // IsConnect = true (fixed destination per stream)
    socks5::encode_address(&mut request, target);
    anytls
        .write_all(&request)
        .await
        .context("anytls udp: send uot request")?;
    anytls.flush().await.context("anytls udp: flush uot request")?;
    Ok(Box::new(anytls))
}
