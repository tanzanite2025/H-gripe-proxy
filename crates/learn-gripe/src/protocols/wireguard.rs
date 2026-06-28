//! WireGuard outbound data plane.
//!
//! Unlike the per-target proxy outbounds (Trojan/VLESS/Snell/…), WireGuard is
//! not a stream proxy: it is an L3 encrypted tunnel carrying arbitrary IP
//! packets to one peer. To relay a TCP connection we run a **userspace TCP/IP
//! stack** (smoltcp, already vendored for the TUN inbound) bound to the address
//! the peer assigned us; each relayed connection is a smoltcp socket whose IP
//! packets are sealed by WireGuard and sent to the peer over a real UDP socket,
//! and whose inbound packets come from decrypting the peer's UDP datagrams.
//! This mirrors sing-box / wireguard-go's userspace `netstack`.
//!
//! The Noise_IKpsk2 handshake, transport-data sealing, rekey/cookie/keepalive
//! timers — the error-prone protocol state machine — are delegated to the
//! vetted `boringtun` crate (`noise::Tunn`), which deliberately ships no
//! network or tunnel stack. We own only the orchestration: UDP I/O, the smoltcp
//! netstack, per-connection bridging, and the per-config device registry. This
//! is the same "delegate the wire codec, own the plumbing" split used for
//! rustls / quinn / smoltcp / hickory elsewhere in the kernel.
//!
//! Scope (this module): **TCP + UDP relay** (IPv4/IPv6 inner targets) over one
//! or more peers. Each relayed UDP association is a userspace smoltcp UDP socket
//! bound inside the same per-config device, so its datagrams ride the Noise
//! tunnel exactly like the TCP flows. Tunnel-side DNS (`remote-dns-resolve`) is
//! supported: a domain target is resolved by querying the configured `dns`
//! resolvers over the tunnel (UDP/53) rather than the host resolver. Multi-peer
//! is supported: the top-level peer plus any `peers` entries each run their own
//! Noise session + UDP endpoint, and an inner packet is routed to the peer with
//! the longest matching `allowed-ips` prefix. AmneziaWG obfuscation
//! (`amnezia-wg-option`) is supported: junk packets precede each handshake,
//! handshake messages carry random prefix padding (`S1`/`S2`), and the 4-byte
//! WireGuard message-type header is rewritten (`H1`-`H4`). boringtun still
//! produces standard messages; the obfuscation is applied to its bytes on the
//! way out and reversed before decapsulation, so the Noise engine is unchanged.

use std::collections::HashMap;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::task::{Context as TaskContext, Poll, Waker};
use std::time::{Duration, Instant};

use anyhow::{Context, Result, anyhow, bail};
use boringtun::noise::{Tunn, TunnResult};
use boringtun::x25519::{PublicKey, StaticSecret};
use hickory_proto::op::{Message, MessageType, OpCode, Query};
use hickory_proto::rr::rdata::{A, AAAA};
use hickory_proto::rr::{Name, RData, RecordType};
use smoltcp::iface::{Config as IfaceConfig, Interface, SocketHandle, SocketSet};
use smoltcp::socket::{tcp, udp};
use smoltcp::time::Instant as SmolInstant;
use smoltcp::wire::{HardwareAddress, IpAddress, IpCidr, IpEndpoint};
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};
use tokio::net::UdpSocket;
use tokio::sync::{Notify, mpsc, oneshot};

use crate::address::TargetAddr;
use crate::config::outbound_opts::{AmneziaWgOption, ProxyEntry};
use crate::outbound::BoxedStream;

/// Default tunnel MTU (max inner IP packet); WireGuard adds a 32-byte overhead
/// on top, so the UDP datagram stays within a typical 1500-byte path.
const DEFAULT_MTU: u32 = 1408;
/// Per-flow bridge channel depth (in chunks).
const CHANNEL_DEPTH: usize = 64;
/// Per-flow smoltcp socket buffer size (each direction).
const FLOW_BUFFER: usize = 64 * 1024;
/// Number of in-flight datagram slots per direction for a UDP flow's smoltcp
/// packet buffer (each datagram needs one metadata slot).
const UDP_META_SLOTS: usize = 64;
/// How long to wait for a relayed TCP connection to reach `Established` (covers
/// the WireGuard handshake plus the inner TCP handshake).
const CONNECT_TIMEOUT: Duration = Duration::from_secs(10);
/// Upper bound on how long the device poll loop sleeps between wakeups.
const MAX_POLL_SLEEP: Duration = Duration::from_millis(250);
/// Wall-clock cadence at which `Tunn::update_timers` is driven (rekey / keepalive
/// / handshake retransmit). boringtun expects this every ~100-250ms; we tick it
/// from the loop top rather than only the timeout arm so steady relay traffic
/// cannot starve it.
const TIMER_TICK: Duration = Duration::from_millis(120);
/// How long to wait for a tunnel-side DNS reply before retransmitting the query.
const DNS_QUERY_TIMEOUT: Duration = Duration::from_millis(800);
/// How many times a tunnel-side DNS query is (re)sent before giving up on a
/// resolver (the first few may be lost while the Noise handshake warms up).
const DNS_QUERY_RETRIES: usize = 5;

/// A single inner-destination prefix routed to a peer (one `allowed-ips` entry).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AllowedIp {
    V4(Ipv4Addr, u8),
    V6(Ipv6Addr, u8),
}

impl AllowedIp {
    /// Prefix length in bits; longer prefixes win when routing an inner packet.
    fn prefix(&self) -> u8 {
        match self {
            AllowedIp::V4(_, p) | AllowedIp::V6(_, p) => *p,
        }
    }

    /// Whether `ip` falls inside this prefix.
    fn contains(&self, ip: IpAddr) -> bool {
        match (self, ip) {
            (AllowedIp::V4(net, prefix), IpAddr::V4(ip)) => prefix_match(&net.octets(), &ip.octets(), *prefix),
            (AllowedIp::V6(net, prefix), IpAddr::V6(ip)) => prefix_match(&net.octets(), &ip.octets(), *prefix),
            _ => false,
        }
    }
}

/// Compare the leading `prefix` bits of two equal-length addresses.
fn prefix_match(net: &[u8], ip: &[u8], prefix: u8) -> bool {
    let prefix = prefix as usize;
    let full = prefix / 8;
    if net[..full] != ip[..full] {
        return false;
    }
    let rem = prefix % 8;
    if rem == 0 {
        return true;
    }
    let mask = 0xffu8 << (8 - rem);
    (net[full] & mask) == (ip[full] & mask)
}

/// One WireGuard peer: endpoint, key material, transport `reserved` tag, and the
/// inner prefixes routed to it.
#[derive(Debug, Clone, PartialEq, Eq)]
struct PeerConfig {
    server: String,
    port: u16,
    public_key: [u8; 32],
    preshared_key: Option<[u8; 32]>,
    reserved: [u8; 3],
    allowed_ips: Vec<AllowedIp>,
}

/// Standard WireGuard message sizes (boringtun emits these). Used on the RX
/// side to identify an obfuscated message by its `(padding + size, header)`
/// signature, since AmneziaWG hides the message type behind `H1`-`H4`.
const MSG_INIT_SIZE: usize = 148;
const MSG_RESP_SIZE: usize = 92;
const MSG_COOKIE_SIZE: usize = 64;
/// Smallest transport message (16-byte header + 16-byte Poly1305 tag of an
/// empty keepalive). Transport packets are variable length, so they are matched
/// by `H4` header + this minimum rather than an exact size.
const MSG_TRANSPORT_MIN: usize = 32;

/// Standard WireGuard message-type bytes (byte 0 of a boringtun message).
const TYPE_INIT: u8 = 1;
const TYPE_RESPONSE: u8 = 2;
const TYPE_COOKIE: u8 = 3;
const TYPE_TRANSPORT: u8 = 4;

/// Parsed AmneziaWG obfuscation parameters, shared by every peer in the device.
/// `s1`/`s2` are byte counts prepended to the handshake initiation / response;
/// `h1`-`h4` are the `u32` values written over the 4-byte message-type header of
/// the initiation / response / cookie / transport messages.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Amnezia {
    jc: u32,
    jmin: u32,
    jmax: u32,
    s1: usize,
    s2: usize,
    h1: u32,
    h2: u32,
    h3: u32,
    h4: u32,
}

impl Amnezia {
    /// Parse and validate the `amnezia-wg-option` block. Unset numeric fields
    /// default to 0 (that obfuscation is skipped). `h1`-`h4` must all be set and
    /// mutually distinct so the RX side can recover the message type, and must
    /// not collide with the standard type bytes (1-4). `jmin <= jmax` when junk
    /// packets are enabled.
    fn from_opts(o: &AmneziaWgOption) -> Result<Self> {
        let am = Amnezia {
            jc: o.jc.unwrap_or(0),
            jmin: o.jmin.unwrap_or(0),
            jmax: o.jmax.unwrap_or(0),
            s1: o.s1.unwrap_or(0) as usize,
            s2: o.s2.unwrap_or(0) as usize,
            h1: o.h1.unwrap_or(0),
            h2: o.h2.unwrap_or(0),
            h3: o.h3.unwrap_or(0),
            h4: o.h4.unwrap_or(0),
        };
        let headers = [am.h1, am.h2, am.h3, am.h4];
        if headers.iter().any(|h| *h == 0) {
            bail!("wireguard: `amnezia-wg-option` requires h1, h2, h3 and h4 to all be set");
        }
        for (i, h) in headers.iter().enumerate() {
            if *h <= TYPE_TRANSPORT as u32 {
                bail!(
                    "wireguard: `amnezia-wg-option` h{} must be > 4 (avoid the standard message types)",
                    i + 1
                );
            }
            if headers[i + 1..].contains(h) {
                bail!("wireguard: `amnezia-wg-option` h1-h4 must be distinct");
            }
        }
        if am.jc > 0 && am.jmin > am.jmax {
            bail!("wireguard: `amnezia-wg-option` requires jmin <= jmax");
        }
        Ok(am)
    }

    /// A fingerprint folded into the device registry key so configs differing
    /// only in obfuscation get their own device.
    fn fingerprint(&self) -> String {
        format!(
            "{}:{}:{}:{}:{}:{:x}:{:x}:{:x}:{:x}",
            self.jc, self.jmin, self.jmax, self.s1, self.s2, self.h1, self.h2, self.h3, self.h4
        )
    }

    /// A random junk-packet length in `[jmin, jmax]`.
    fn junk_len(&self) -> usize {
        if self.jmax <= self.jmin {
            return self.jmin as usize;
        }
        let span = (self.jmax - self.jmin + 1) as u64;
        (self.jmin as u64 + random_u64() % span) as usize
    }

    /// The `(header, padding)` pair applied to a message of standard `type`.
    fn transform(&self, std_type: u8) -> Option<(u32, usize)> {
        match std_type {
            TYPE_INIT => Some((self.h1, self.s1)),
            TYPE_RESPONSE => Some((self.h2, self.s2)),
            TYPE_COOKIE => Some((self.h3, 0)),
            TYPE_TRANSPORT => Some((self.h4, 0)),
            _ => None,
        }
    }
}

/// Fill `buf` with random bytes (used for junk packets and S1/S2 padding).
fn fill_random(buf: &mut [u8]) {
    if getrandom::fill(buf).is_err() {
        // The system RNG being unavailable is fatal elsewhere (key/index gen);
        // here we degrade to a cheap fallback so obfuscation never panics.
        for b in buf.iter_mut() {
            *b = 0;
        }
    }
}

/// A random `u64` from the system RNG (falls back to 0 if unavailable).
fn random_u64() -> u64 {
    let mut b = [0u8; 8];
    let _ = getrandom::fill(&mut b);
    u64::from_le_bytes(b)
}

/// Send a boringtun-produced datagram to the peer. With AmneziaWG configured
/// this sends `jc` random junk packets ahead of a handshake initiation, prepends
/// the `S1`/`S2` random padding, and rewrites the 4-byte message-type header
/// (`H1`-`H4`); otherwise it just stamps the `reserved` tag. `out` is the raw
/// WireGuard message from boringtun (byte 0 is the standard type 1-4).
async fn send_obfuscated(amnezia: &Option<Amnezia>, udp: &UdpSocket, reserved: [u8; 3], out: &mut [u8]) {
    let Some(am) = amnezia else {
        apply_reserved(out, reserved);
        let _ = udp.send(out).await;
        return;
    };
    if out.len() < 4 {
        let _ = udp.send(out).await;
        return;
    }
    let std_type = out[0];
    // Junk packets precede every handshake initiation.
    if std_type == TYPE_INIT {
        for _ in 0..am.jc {
            let mut junk = vec![0u8; am.junk_len()];
            fill_random(&mut junk);
            let _ = udp.send(&junk).await;
        }
    }
    let Some((header, pad)) = am.transform(std_type) else {
        let _ = udp.send(out).await;
        return;
    };
    let mut buf = vec![0u8; pad + out.len()];
    if pad > 0 {
        fill_random(&mut buf[..pad]);
    }
    buf[pad..].copy_from_slice(out);
    buf[pad..pad + 4].copy_from_slice(&header.to_le_bytes());
    let _ = udp.send(&buf).await;
}

/// Reverse AmneziaWG obfuscation on a received datagram in place: identify the
/// message by its `(padding, size, H-header)` signature, drop the `S1`/`S2`
/// prefix, and restore the standard 4-byte message-type header so boringtun can
/// parse it. Returns the deobfuscated length, or `None` to drop the datagram
/// (a junk packet or anything unrecognised).
fn deobfuscate(am: &Amnezia, buf: &mut [u8]) -> Option<usize> {
    let size = buf.len();
    let header_at = |off: usize| {
        buf.get(off..off + 4)
            .map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
    };

    let (pad, std_type) = if size == am.s1 + MSG_INIT_SIZE && header_at(am.s1) == Some(am.h1) {
        (am.s1, TYPE_INIT)
    } else if size == am.s2 + MSG_RESP_SIZE && header_at(am.s2) == Some(am.h2) {
        (am.s2, TYPE_RESPONSE)
    } else if size == MSG_COOKIE_SIZE && header_at(0) == Some(am.h3) {
        (0, TYPE_COOKIE)
    } else if size >= MSG_TRANSPORT_MIN && header_at(0) == Some(am.h4) {
        (0, TYPE_TRANSPORT)
    } else {
        return None;
    };

    if pad > 0 {
        buf.copy_within(pad.., 0);
    }
    let n = size - pad;
    buf[0] = std_type;
    buf[1] = 0;
    buf[2] = 0;
    buf[3] = 0;
    Some(n)
}

/// Parsed WireGuard outbound configuration. The interface-level fields (key,
/// assigned address, MTU, DNS) are shared; `peers` lists one or more peers, each
/// with its own Noise session, endpoint, and `allowed-ips`. Index 0 is the
/// top-level peer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WireGuardOutboundConfig {
    pub server: String,
    pub port: u16,
    private_key: [u8; 32],
    local_v4: Option<Ipv4Addr>,
    local_v6: Option<Ipv6Addr>,
    mtu: u32,
    keepalive: Option<u16>,
    /// Resolve domain targets via DNS sent through the tunnel to `dns_servers`.
    remote_dns_resolve: bool,
    /// Resolver socket addresses reachable inside the tunnel (port 53 default).
    dns_servers: Vec<SocketAddr>,
    peers: Vec<PeerConfig>,
    /// AmneziaWG obfuscation, applied uniformly to every peer when set.
    amnezia: Option<Amnezia>,
}

impl WireGuardOutboundConfig {
    pub fn from_proxy(entry: &ProxyEntry) -> Result<Self> {
        let opts = &entry.options;
        let server = opts
            .server
            .clone()
            .ok_or_else(|| anyhow!("wireguard: missing `server`"))?;
        let port = opts.port.ok_or_else(|| anyhow!("wireguard: missing `port`"))?;
        let private_key = parse_key(
            opts.private_key
                .as_deref()
                .ok_or_else(|| anyhow!("wireguard: missing `private-key`"))?,
        )
        .context("wireguard: invalid `private-key`")?;
        let public_key = parse_key(
            opts.public_key
                .as_deref()
                .ok_or_else(|| anyhow!("wireguard: missing `public-key`"))?,
        )
        .context("wireguard: invalid `public-key`")?;
        let preshared_key = match opts.pre_shared_key.as_deref() {
            Some(psk) => Some(parse_key(psk).context("wireguard: invalid `pre-shared-key`")?),
            None => None,
        };

        let local_v4 = match opts.ip.as_deref() {
            Some(ip) => Some(parse_local_v4(ip).with_context(|| format!("wireguard: invalid `ip` {ip:?}"))?),
            None => None,
        };
        let local_v6 = match opts.ipv6.as_deref() {
            Some(ip) => Some(
                ip.trim()
                    .split('/')
                    .next()
                    .unwrap_or("")
                    .parse::<Ipv6Addr>()
                    .with_context(|| format!("wireguard: invalid `ipv6` {ip:?}"))?,
            ),
            None => None,
        };
        if local_v4.is_none() && local_v6.is_none() {
            bail!("wireguard: at least one of `ip` / `ipv6` (the assigned tunnel address) is required");
        }

        let reserved = parse_reserved(opts.reserved.as_deref())?;

        let keepalive = opts.persistent_keepalive.and_then(|k| {
            if k == 0 {
                None
            } else {
                Some(k.min(u16::MAX as u32) as u16)
            }
        });

        let mtu = opts.mtu.filter(|m| *m >= 576).unwrap_or(DEFAULT_MTU);

        let dns_servers = match &opts.dns {
            Some(list) => list
                .iter()
                .map(|s| parse_dns_server(s).with_context(|| format!("wireguard: invalid `dns` entry {s:?}")))
                .collect::<Result<Vec<_>>>()?,
            None => Vec::new(),
        };
        let remote_dns_resolve = opts.remote_dns_resolve.unwrap_or(false);
        if remote_dns_resolve && dns_servers.is_empty() {
            bail!("wireguard: `remote-dns-resolve` requires at least one `dns` resolver");
        }

        let amnezia = match &opts.amnezia_wg_option {
            Some(o) => Some(Amnezia::from_opts(o)?),
            None => None,
        };

        // The top-level peer; its `allowed-ips` defaults to a catch-all so a
        // single-peer tunnel carries everything.
        let top_allowed = match &opts.allowed_ips {
            Some(list) => parse_allowed_ips(list)?,
            None => catch_all(),
        };
        let mut peers = vec![PeerConfig {
            server: server.clone(),
            port,
            public_key,
            preshared_key,
            reserved,
            allowed_ips: top_allowed,
        }];

        // Additional `peers` entries each need an explicit endpoint, key, and
        // `allowed-ips` (routing across multiple peers must be unambiguous).
        if let Some(extra) = &opts.peers {
            for (i, p) in extra.iter().enumerate() {
                let server = p
                    .server
                    .clone()
                    .ok_or_else(|| anyhow!("wireguard: `peers[{i}]` missing `server`"))?;
                let port = p
                    .port
                    .ok_or_else(|| anyhow!("wireguard: `peers[{i}]` missing `port`"))?;
                let public_key = parse_key(
                    p.public_key
                        .as_deref()
                        .ok_or_else(|| anyhow!("wireguard: `peers[{i}]` missing `public-key`"))?,
                )
                .with_context(|| format!("wireguard: `peers[{i}]` invalid `public-key`"))?;
                let preshared_key = match p.pre_shared_key.as_deref() {
                    Some(psk) => Some(
                        parse_key(psk).with_context(|| format!("wireguard: `peers[{i}]` invalid `pre-shared-key`"))?,
                    ),
                    None => None,
                };
                let reserved = parse_reserved(p.reserved.as_deref())
                    .with_context(|| format!("wireguard: `peers[{i}]` invalid `reserved`"))?;
                let allowed = match &p.allowed_ips {
                    Some(list) if !list.is_empty() => parse_allowed_ips(list)?,
                    _ => bail!("wireguard: `peers[{i}]` requires non-empty `allowed-ips`"),
                };
                peers.push(PeerConfig {
                    server,
                    port,
                    public_key,
                    preshared_key,
                    reserved,
                    allowed_ips: allowed,
                });
            }
        }

        Ok(Self {
            server,
            port,
            private_key,
            local_v4,
            local_v6,
            mtu,
            keepalive,
            remote_dns_resolve,
            dns_servers,
            peers,
            amnezia,
        })
    }

    fn registry_key(&self) -> WgKey {
        let mut peers: Vec<String> = self
            .peers
            .iter()
            .map(|p| format!("{}:{}:{}", p.server, p.port, hex(&p.public_key)))
            .collect();
        peers.sort();
        let awg = self.amnezia.map(|a| a.fingerprint()).unwrap_or_default();
        format!("{}|{}|{}", hex(&self.private_key), peers.join(","), awg)
    }
}

/// Connect a relayed TCP stream to `target` through the configured WireGuard
/// tunnel, reusing (or lazily building) the per-config device.
pub async fn connect(config: &WireGuardOutboundConfig, target: &TargetAddr) -> Result<BoxedStream> {
    let device = WireGuardDevice::get_or_create(config).await?;
    let dst = resolve_target(config, &device, target).await?;
    let stream = device.open_tcp(dst).await?;
    Ok(Box::new(stream) as BoxedStream)
}

/// Open a relayed UDP association to `target` through the configured WireGuard
/// tunnel, reusing (or lazily building) the per-config device. Each association
/// is a userspace smoltcp UDP socket; datagrams to the resolved destination ride
/// the Noise tunnel like the TCP flows.
pub async fn connect_udp(config: &WireGuardOutboundConfig, target: &TargetAddr) -> Result<WgUdpAssoc> {
    let device = WireGuardDevice::get_or_create(config).await?;
    let dst = resolve_target(config, &device, target).await?;
    device.open_udp(dst).await
}

/// Resolve a relayed target to a literal socket address. A domain is resolved
/// over the tunnel (DNS sent to a `dns` resolver through the device) when
/// `remote-dns-resolve` is set, otherwise by the host resolver.
async fn resolve_target(
    config: &WireGuardOutboundConfig,
    device: &WireGuardDevice,
    target: &TargetAddr,
) -> Result<SocketAddr> {
    match target {
        TargetAddr::Ip(addr) => Ok(*addr),
        TargetAddr::Domain(host, port) => {
            if config.remote_dns_resolve && !config.dns_servers.is_empty() {
                let ip = device
                    .resolve_remote(host, config)
                    .await
                    .with_context(|| format!("wireguard: tunnel DNS resolve {host}"))?;
                Ok(SocketAddr::new(ip, *port))
            } else {
                tokio::net::lookup_host((host.as_str(), *port))
                    .await
                    .with_context(|| format!("wireguard: resolve {host}:{port}"))?
                    .next()
                    .ok_or_else(|| anyhow!("wireguard: no addresses for {host}:{port}"))
            }
        }
    }
}

/// Registry key fingerprinting the interface key plus every peer endpoint/key,
/// so the same multi-peer config shares one device while a different peer set
/// gets its own.
type WgKey = String;

/// Per-config registry of live tunnel devices, so concurrent connections to the
/// same peer share one Noise session + netstack (mirrors the AnyTLS session
/// registry). A device whose command channel has closed (its loop exited) is
/// discarded and rebuilt on the next connect.
static DEVICE_REGISTRY: Mutex<Option<HashMap<WgKey, Arc<WireGuardDevice>>>> = Mutex::new(None);

/// Command sent from a `connect` call into the device's poll loop.
enum Command {
    OpenTcp {
        dst: SocketAddr,
        reply: oneshot::Sender<WgTcpStream>,
    },
    OpenUdp {
        dst: SocketAddr,
        reply: oneshot::Sender<WgUdpAssoc>,
    },
}

/// Handle to a running WireGuard tunnel device: just the command channel into
/// its poll loop task.
pub struct WireGuardDevice {
    commands: mpsc::Sender<Command>,
}

impl WireGuardDevice {
    async fn get_or_create(config: &WireGuardOutboundConfig) -> Result<Arc<Self>> {
        let key = config.registry_key();
        {
            let mut guard = DEVICE_REGISTRY.lock().expect("wireguard device registry");
            let map = guard.get_or_insert_with(HashMap::new);
            if let Some(device) = map.get(&key) {
                if !device.commands.is_closed() {
                    return Ok(device.clone());
                }
                map.remove(&key);
            }
        }

        let device = Arc::new(Self::spawn(config).await?);
        let mut guard = DEVICE_REGISTRY.lock().expect("wireguard device registry");
        let map = guard.get_or_insert_with(HashMap::new);
        // Another task may have raced us; prefer the existing live device.
        if let Some(existing) = map.get(&key) {
            if !existing.commands.is_closed() {
                return Ok(existing.clone());
            }
        }
        map.insert(key, device.clone());
        Ok(device)
    }

    /// Dial every peer's UDP endpoint, build a Noise tunnel per peer plus the
    /// shared smoltcp interface, and spawn the poll loop.
    async fn spawn(config: &WireGuardOutboundConfig) -> Result<Self> {
        let mut peers = Vec::with_capacity(config.peers.len());
        for peer in &config.peers {
            let endpoint = tokio::net::lookup_host((peer.server.as_str(), peer.port))
                .await
                .with_context(|| format!("wireguard: resolve peer {}:{}", peer.server, peer.port))?
                .next()
                .ok_or_else(|| anyhow!("wireguard: no addresses for peer {}:{}", peer.server, peer.port))?;

            let bind: SocketAddr = if endpoint.is_ipv4() {
                (Ipv4Addr::UNSPECIFIED, 0).into()
            } else {
                (Ipv6Addr::UNSPECIFIED, 0).into()
            };
            let udp = UdpSocket::bind(bind).await.context("wireguard: bind UDP socket")?;
            udp.connect(endpoint)
                .await
                .with_context(|| format!("wireguard: connect UDP to {endpoint}"))?;

            let mut index = [0u8; 4];
            getrandom::fill(&mut index).map_err(|_| anyhow!("wireguard: system RNG unavailable"))?;
            let tunn = Tunn::new(
                StaticSecret::from(config.private_key),
                PublicKey::from(peer.public_key),
                peer.preshared_key,
                config.keepalive,
                u32::from_le_bytes(index),
                None,
            );

            peers.push(PeerTunn {
                tunn,
                udp,
                reserved: peer.reserved,
                allowed_ips: peer.allowed_ips.clone(),
            });
        }

        let (commands_tx, commands_rx) = mpsc::channel::<Command>(CHANNEL_DEPTH);
        let loop_state = DeviceLoop::new(peers, config, commands_rx);
        tokio::spawn(loop_state.run());

        Ok(Self { commands: commands_tx })
    }

    async fn open_tcp(&self, dst: SocketAddr) -> Result<WgTcpStream> {
        let (reply_tx, reply_rx) = oneshot::channel();
        self.commands
            .send(Command::OpenTcp { dst, reply: reply_tx })
            .await
            .map_err(|_| anyhow!("wireguard: device loop is gone"))?;
        reply_rx
            .await
            .map_err(|_| anyhow!("wireguard: connection to {dst} failed (handshake/connect timeout)"))
    }

    async fn open_udp(&self, dst: SocketAddr) -> Result<WgUdpAssoc> {
        let (reply_tx, reply_rx) = oneshot::channel();
        self.commands
            .send(Command::OpenUdp { dst, reply: reply_tx })
            .await
            .map_err(|_| anyhow!("wireguard: device loop is gone"))?;
        reply_rx
            .await
            .map_err(|_| anyhow!("wireguard: UDP association to {dst} failed"))
    }

    /// Resolve `host` to an IP by querying the configured `dns` resolvers over
    /// the tunnel (UDP/53). Tries each resolver in turn, and `A`/`AAAA` in the
    /// order implied by the assigned tunnel address family. UDP has no
    /// retransmit, so each query is resent a few times (covering the Noise
    /// handshake warm-up) before moving on.
    async fn resolve_remote(&self, host: &str, config: &WireGuardOutboundConfig) -> Result<IpAddr> {
        let mut rtypes: Vec<RecordType> = Vec::new();
        if config.local_v4.is_some() {
            rtypes.push(RecordType::A);
        }
        if config.local_v6.is_some() {
            rtypes.push(RecordType::AAAA);
        }
        if rtypes.is_empty() {
            rtypes.push(RecordType::A);
        }

        for server in &config.dns_servers {
            for &rtype in &rtypes {
                let assoc = match self.open_udp(*server).await {
                    Ok(assoc) => assoc,
                    Err(_) => continue,
                };
                let id = dns_query_id();
                let query = build_dns_query(host, rtype, id)?;
                for _ in 0..DNS_QUERY_RETRIES {
                    if assoc.send(&query).await.is_err() {
                        break;
                    }
                    match tokio::time::timeout(DNS_QUERY_TIMEOUT, assoc.recv()).await {
                        // A response for our query: either an answer (done) or a
                        // negative/empty reply (stop retrying this record type).
                        Ok(Ok(resp)) => match parse_dns_answer(&resp, id, rtype) {
                            Some(ip) => return Ok(ip),
                            None => break,
                        },
                        // recv error: association is gone, try the next resolver.
                        Ok(Err(_)) => break,
                        // Timed out: fall through to retransmit (handshake may
                        // still be warming up).
                        Err(_) => {}
                    }
                }
            }
        }
        bail!("wireguard: no DNS answer for {host} from configured resolvers")
    }
}

/// Wakers parked by streams whose write channel filled, woken once the loop has
/// drained their bytes into the smoltcp sockets.
type WriterWakers = Arc<Mutex<Vec<Waker>>>;

/// One peer's tunnel state owned by the poll loop: its Noise session, dedicated
/// UDP endpoint, transport `reserved` tag, and the inner prefixes routed to it.
struct PeerTunn {
    tunn: Tunn,
    udp: UdpSocket,
    reserved: [u8; 3],
    allowed_ips: Vec<AllowedIp>,
}

/// State owned by the per-device poll loop.
struct DeviceLoop {
    /// One entry per configured peer; index 0 is the top-level peer.
    peers: Vec<PeerTunn>,
    mtu: usize,
    local_v4: Option<Ipv4Addr>,
    local_v6: Option<Ipv6Addr>,
    commands: mpsc::Receiver<Command>,
    flows: Vec<WgFlow>,
    udp_flows: Vec<WgUdpFlow>,
    next_port: u16,
    wake: Arc<Notify>,
    writer_wakers: WriterWakers,
    /// AmneziaWG obfuscation applied to every peer's UDP I/O when set.
    amnezia: Option<Amnezia>,
}

/// Bridge state for one relayed TCP flow, owned by the poll loop.
struct WgFlow {
    handle: SocketHandle,
    /// Caller -> socket bytes.
    write_rx: mpsc::Receiver<Vec<u8>>,
    /// Socket -> caller bytes; dropped to signal EOF to the caller.
    read_tx: Option<mpsc::Sender<Vec<u8>>>,
    /// Caller bytes not yet accepted by the socket send buffer.
    pending: Vec<u8>,
    pending_off: usize,
    /// We have closed the socket's write side (caller half-closed).
    write_closed: bool,
    /// Pending connect result; resolved once the socket reaches `Established`.
    connect_reply: Option<oneshot::Sender<WgTcpStream>>,
    /// The stream handed to the caller once connected.
    stream_slot: Option<WgTcpStream>,
    deadline: Instant,
}

/// Bridge state for one relayed UDP association, owned by the poll loop. Unlike
/// TCP there is no connection state: datagrams flow to a fixed `remote` and one
/// datagram maps to one inner UDP packet.
struct WgUdpFlow {
    handle: SocketHandle,
    /// Fixed inner destination for this association.
    remote: IpEndpoint,
    /// Caller -> socket datagrams.
    write_rx: mpsc::Receiver<Vec<u8>>,
    /// Socket -> caller datagrams.
    read_tx: mpsc::Sender<Vec<u8>>,
    /// A datagram accepted from the caller but not yet handed to the send buffer.
    pending: Option<Vec<u8>>,
}

impl DeviceLoop {
    fn new(peers: Vec<PeerTunn>, config: &WireGuardOutboundConfig, commands: mpsc::Receiver<Command>) -> Self {
        Self {
            peers,
            mtu: config.mtu as usize,
            local_v4: config.local_v4,
            local_v6: config.local_v6,
            commands,
            flows: Vec::new(),
            udp_flows: Vec::new(),
            next_port: 1024,
            wake: Arc::new(Notify::new()),
            writer_wakers: Arc::new(Mutex::new(Vec::new())),
            amnezia: config.amnezia,
        }
    }

    async fn run(mut self) {
        let start = Instant::now();
        let mut phy = WgPhy::new(self.mtu);
        let mut iface = build_interface(&mut phy, smol_now(start), self.local_v4, self.local_v6);
        let mut sockets = SocketSet::new(Vec::new());
        // One receive buffer per peer (each peer has its own UDP socket).
        let mut udp_bufs: Vec<Vec<u8>> = (0..self.peers.len()).map(|_| vec![0u8; 65535]).collect();
        let mut scratch = vec![0u8; 65535 + 32];

        // Kick each peer's handshake proactively so the first SYN has a session
        // to ride instead of waiting for smoltcp's first retransmit.
        for idx in 0..self.peers.len() {
            let reserved = self.peers[idx].reserved;
            if let TunnResult::WriteToNetwork(out) =
                self.peers[idx].tunn.format_handshake_initiation(&mut scratch, false)
            {
                send_obfuscated(&self.amnezia, &self.peers[idx].udp, reserved, out).await;
            }
        }

        // Next wall-clock instant at which the WireGuard timers must be driven.
        let mut next_timer = Instant::now() + TIMER_TICK;

        loop {
            let now = smol_now(start);
            iface.poll(now, &mut phy, &mut sockets);
            self.service_flows(&mut sockets, &mut iface);
            self.service_udp_flows(&mut sockets);
            self.wake_writers();
            self.encapsulate_tx(&mut phy, &mut scratch).await;

            // Drive rekey / keepalive / handshake-retransmit on a steady cadence
            // regardless of `select!` readiness. Folding this into the timeout
            // arm alone lets a busy tunnel (the `udp.recv`/`wake` arms always
            // ready) starve the timers, so a long-lived but bursty session could
            // miss its rekey and die; this gate fires on schedule under load.
            if Instant::now() >= next_timer {
                self.drive_timers(&mut scratch).await;
                next_timer = Instant::now() + TIMER_TICK;
            }

            // Wake by `next_timer` at the latest so the gate above runs on time.
            let timer_wait = next_timer.saturating_duration_since(Instant::now());
            let delay = iface
                .poll_delay(smol_now(start), &sockets)
                .map(|d| Duration::from_micros(d.total_micros()))
                .map_or(MAX_POLL_SLEEP, |d| d.min(MAX_POLL_SLEEP))
                .min(timer_wait);

            tokio::select! {
                _ = self.wake.notified() => {}
                cmd = self.commands.recv() => match cmd {
                    Some(cmd) => self.handle_command(cmd, &mut sockets, &mut iface),
                    None => return,
                },
                (idx, res) = recv_any(&self.peers, &mut udp_bufs) => {
                    if let Ok(n) = res {
                        self.decapsulate_rx(idx, n, &mut udp_bufs, &mut phy, &mut scratch).await;
                    }
                }
                _ = tokio::time::sleep(delay) => {}
            }
        }
    }

    /// Open a smoltcp client socket to `dst`, wire its bridge channels, and stash
    /// the caller's stream to hand over once it connects.
    fn handle_command(&mut self, cmd: Command, sockets: &mut SocketSet, iface: &mut Interface) {
        let (dst, reply) = match cmd {
            Command::OpenTcp { dst, reply } => (dst, reply),
            Command::OpenUdp { dst, reply } => return self.handle_open_udp(dst, reply, sockets),
        };
        let remote = IpEndpoint::new(ip_address(dst.ip()), dst.port());
        let mut sock = tcp::Socket::new(
            tcp::SocketBuffer::new(vec![0u8; FLOW_BUFFER]),
            tcp::SocketBuffer::new(vec![0u8; FLOW_BUFFER]),
        );
        let local_port = self.alloc_port();
        if sock.connect(iface.context(), remote, local_port).is_err() {
            return; // dropping `reply` reports the failure to the caller
        }
        let handle = sockets.add(sock);

        let (write_tx, write_rx) = mpsc::channel::<Vec<u8>>(CHANNEL_DEPTH);
        let (read_tx, read_rx) = mpsc::channel::<Vec<u8>>(CHANNEL_DEPTH);
        let stream = WgTcpStream {
            write_tx: Some(write_tx),
            read_rx,
            wake: self.wake.clone(),
            writer_wakers: self.writer_wakers.clone(),
            leftover: Vec::new(),
            leftover_pos: 0,
        };

        self.flows.push(WgFlow {
            handle,
            write_rx,
            read_tx: Some(read_tx),
            pending: Vec::new(),
            pending_off: 0,
            write_closed: false,
            connect_reply: Some(reply),
            stream_slot: Some(stream),
            deadline: Instant::now() + CONNECT_TIMEOUT,
        });
    }

    /// Open a smoltcp UDP socket bound to a local port for datagrams destined to
    /// `dst`, wire its bridge channels, and hand the association back. Unlike
    /// TCP there is no connect handshake, so the association is returned
    /// immediately; datagrams sent before the Noise handshake completes are
    /// dropped (UDP is lossy).
    fn handle_open_udp(&mut self, dst: SocketAddr, reply: oneshot::Sender<WgUdpAssoc>, sockets: &mut SocketSet) {
        let remote = IpEndpoint::new(ip_address(dst.ip()), dst.port());
        let mut sock = udp::Socket::new(
            udp::PacketBuffer::new(vec![udp::PacketMetadata::EMPTY; UDP_META_SLOTS], vec![0u8; FLOW_BUFFER]),
            udp::PacketBuffer::new(vec![udp::PacketMetadata::EMPTY; UDP_META_SLOTS], vec![0u8; FLOW_BUFFER]),
        );
        let local_port = self.alloc_port();
        if sock.bind(local_port).is_err() {
            return; // dropping `reply` reports the failure to the caller
        }
        let handle = sockets.add(sock);

        let (write_tx, write_rx) = mpsc::channel::<Vec<u8>>(CHANNEL_DEPTH);
        let (read_tx, read_rx) = mpsc::channel::<Vec<u8>>(CHANNEL_DEPTH);
        let assoc = WgUdpAssoc {
            write_tx,
            read_rx: tokio::sync::Mutex::new(read_rx),
            wake: self.wake.clone(),
        };
        self.udp_flows.push(WgUdpFlow {
            handle,
            remote,
            write_rx,
            read_tx,
            pending: None,
        });
        let _ = reply.send(assoc);
    }

    fn alloc_port(&mut self) -> u16 {
        let port = self.next_port;
        self.next_port = self.next_port.checked_add(1).unwrap_or(1024);
        port
    }

    /// Move datagrams between each UDP flow's smoltcp socket and its bridge
    /// channels, dropping (rather than stalling) when a buffer is full, and reap
    /// flows whose caller association has been dropped.
    fn service_udp_flows(&mut self, sockets: &mut SocketSet) {
        let mut done: Vec<usize> = Vec::new();
        for (idx, flow) in self.udp_flows.iter_mut().enumerate() {
            let sock = sockets.get_mut::<udp::Socket>(flow.handle);
            let mut reap = false;

            // caller -> socket
            loop {
                if flow.pending.is_none() {
                    match flow.write_rx.try_recv() {
                        Ok(buf) => flow.pending = Some(buf),
                        Err(mpsc::error::TryRecvError::Empty) => break,
                        Err(mpsc::error::TryRecvError::Disconnected) => {
                            reap = true;
                            break;
                        }
                    }
                }
                if !sock.can_send() {
                    break;
                }
                let Some(buf) = flow.pending.take() else { break };
                match sock.send_slice(&buf, flow.remote) {
                    Ok(()) => {}
                    // Send buffer is full: retry this datagram next turn.
                    Err(udp::SendError::BufferFull) => {
                        flow.pending = Some(buf);
                        break;
                    }
                    // No route to the destination: drop the datagram.
                    Err(udp::SendError::Unaddressable) => {}
                }
            }

            // socket -> caller
            while sock.can_recv() {
                let payload = match sock.recv() {
                    Ok((data, _meta)) => data.to_vec(),
                    Err(_) => break,
                };
                match flow.read_tx.try_send(payload) {
                    Ok(()) => {}
                    // Caller is draining slowly: drop this reply (UDP is lossy).
                    Err(mpsc::error::TrySendError::Full(_)) => break,
                    // Caller association dropped: reap the flow.
                    Err(mpsc::error::TrySendError::Closed(_)) => {
                        reap = true;
                        break;
                    }
                }
            }

            if reap {
                done.push(idx);
            }
        }

        for idx in done.into_iter().rev() {
            let flow = self.udp_flows.swap_remove(idx);
            sockets.remove(flow.handle);
        }
    }

    /// Move bytes between each flow's smoltcp socket and its bridge channels,
    /// resolve pending connects, and reap finished flows.
    fn service_flows(&mut self, sockets: &mut SocketSet, _iface: &mut Interface) {
        let mut done: Vec<usize> = Vec::new();
        for (idx, flow) in self.flows.iter_mut().enumerate() {
            let sock = sockets.get_mut::<tcp::Socket>(flow.handle);

            // Resolve the pending connect once established (or fail it).
            if flow.connect_reply.is_some() {
                if sock.state() == tcp::State::Established {
                    if let (Some(reply), Some(stream)) = (flow.connect_reply.take(), flow.stream_slot.take()) {
                        let _ = reply.send(stream);
                    }
                } else if Instant::now() >= flow.deadline || is_dead(sock.state()) {
                    flow.connect_reply = None; // dropping the sender fails the connect
                    flow.stream_slot = None;
                    done.push(idx);
                    continue;
                } else {
                    continue; // still connecting; no data bridging yet
                }
            }

            // caller -> socket
            loop {
                if flow.pending_off >= flow.pending.len() {
                    flow.pending.clear();
                    flow.pending_off = 0;
                    if flow.write_closed {
                        break;
                    }
                    match flow.write_rx.try_recv() {
                        Ok(buf) => flow.pending = buf,
                        Err(mpsc::error::TryRecvError::Empty) => break,
                        Err(mpsc::error::TryRecvError::Disconnected) => {
                            // Caller dropped/half-closed: FIN the socket once flushed.
                            if !flow.write_closed {
                                sock.close();
                                flow.write_closed = true;
                            }
                            break;
                        }
                    }
                }
                if !sock.can_send() {
                    break;
                }
                match sock.send_slice(&flow.pending[flow.pending_off..]) {
                    Ok(0) => break,
                    Ok(n) => flow.pending_off += n,
                    Err(_) => break,
                }
            }

            // socket -> caller
            if let Some(tx) = &flow.read_tx {
                while sock.can_recv() {
                    match tx.try_reserve() {
                        Ok(permit) => {
                            let data = sock.recv(|buf| (buf.len(), buf.to_vec())).unwrap_or_default();
                            if data.is_empty() {
                                break;
                            }
                            permit.send(data);
                        }
                        Err(_) => break,
                    }
                }
            }

            // Peer FIN and everything drained -> signal EOF to the caller.
            if !sock.may_recv() && !sock.can_recv() {
                flow.read_tx = None;
            }

            if sock.state() == tcp::State::Closed {
                done.push(idx);
            }
        }

        for idx in done.into_iter().rev() {
            let flow = self.flows.swap_remove(idx);
            sockets.remove(flow.handle);
        }
    }

    fn wake_writers(&self) {
        let mut wakers = self.writer_wakers.lock().expect("wireguard writer wakers");
        for waker in wakers.drain(..) {
            waker.wake();
        }
    }

    /// Pick the peer an inner packet destined for `dst` should ride: the one
    /// whose `allowed-ips` has the longest prefix matching `dst`. Ties keep the
    /// earlier (lower-index) peer. Returns `None` when no peer claims `dst`.
    fn route(&self, dst: IpAddr) -> Option<usize> {
        let mut best: Option<(usize, u8)> = None;
        for (i, peer) in self.peers.iter().enumerate() {
            for allowed in &peer.allowed_ips {
                if allowed.contains(dst) {
                    let prefix = allowed.prefix();
                    if best.is_none_or(|(_, b)| prefix > b) {
                        best = Some((i, prefix));
                    }
                }
            }
        }
        best.map(|(i, _)| i)
    }

    /// Encapsulate every IP packet smoltcp queued and send it to the peer that
    /// claims the packet's destination (longest `allowed-ips` match). Packets
    /// claimed by no peer are dropped.
    async fn encapsulate_tx(&mut self, phy: &mut WgPhy, scratch: &mut [u8]) {
        while let Some(pkt) = phy.tx.pop_front() {
            let idx = match packet_dst_ip(&pkt).and_then(|dst| self.route(dst)) {
                Some(idx) => idx,
                None => continue,
            };
            let reserved = self.peers[idx].reserved;
            match self.peers[idx].tunn.encapsulate(&pkt, scratch) {
                TunnResult::WriteToNetwork(out) => {
                    send_obfuscated(&self.amnezia, &self.peers[idx].udp, reserved, out).await;
                }
                TunnResult::Err(_) | TunnResult::Done => {}
                // encapsulate only ever yields WriteToNetwork / Done / Err.
                _ => {}
            }
        }
    }

    /// Decapsulate one datagram received on peer `idx`'s socket, feeding
    /// decrypted IP packets to smoltcp and flushing any handshake/cookie
    /// responses back to that peer.
    async fn decapsulate_rx(
        &mut self,
        idx: usize,
        n: usize,
        udp_bufs: &mut [Vec<u8>],
        phy: &mut WgPhy,
        scratch: &mut [u8],
    ) {
        let reserved = self.peers[idx].reserved;
        // Reverse AmneziaWG obfuscation (or just clear `reserved`) before the
        // datagram reaches boringtun. A junk / unrecognised packet yields `None`
        // and is dropped.
        let n = match &self.amnezia {
            Some(am) => match deobfuscate(am, &mut udp_bufs[idx][..n]) {
                Some(m) => m,
                None => return,
            },
            None => {
                clear_reserved(&mut udp_bufs[idx][..n]);
                n
            }
        };
        // First call parses the datagram; subsequent calls with an empty slice
        // flush queued network writes until `Done`.
        let mut first = true;
        loop {
            let datagram: &[u8] = if first { &udp_bufs[idx][..n] } else { &[] };
            match self.peers[idx].tunn.decapsulate(None, datagram, scratch) {
                TunnResult::WriteToNetwork(out) => {
                    send_obfuscated(&self.amnezia, &self.peers[idx].udp, reserved, out).await;
                    first = false;
                }
                TunnResult::WriteToTunnelV4(pkt, _) | TunnResult::WriteToTunnelV6(pkt, _) => {
                    phy.rx.push_back(pkt.to_vec());
                    self.wake.notify_one();
                    break;
                }
                TunnResult::Done | TunnResult::Err(_) => break,
            }
        }
    }

    /// Drive rekey / keepalive / handshake retransmit timers for every peer,
    /// flushing each packet `update_timers` wants to emit this tick. boringtun
    /// yields at most one packet per call, so the bounded drain just covers the
    /// case where more than one timer is simultaneously due; a repeat call at
    /// the same instant returns `Done` and ends the loop.
    async fn drive_timers(&mut self, scratch: &mut [u8]) {
        for idx in 0..self.peers.len() {
            let reserved = self.peers[idx].reserved;
            for _ in 0..4 {
                match self.peers[idx].tunn.update_timers(scratch) {
                    TunnResult::WriteToNetwork(out) => {
                        send_obfuscated(&self.amnezia, &self.peers[idx].udp, reserved, out).await;
                    }
                    _ => break,
                }
            }
        }
    }
}

/// Recv on whichever peer socket is ready first, returning `(peer_index,
/// result)`. Each socket recvs into its own buffer; pending sockets register
/// their waker so the loop is rescheduled when any becomes readable.
async fn recv_any(peers: &[PeerTunn], bufs: &mut [Vec<u8>]) -> (usize, std::io::Result<usize>) {
    std::future::poll_fn(|cx| {
        for (i, (peer, buf)) in peers.iter().zip(bufs.iter_mut()).enumerate() {
            let mut rb = ReadBuf::new(&mut buf[..]);
            match peer.udp.poll_recv(cx, &mut rb) {
                Poll::Ready(Ok(())) => return Poll::Ready((i, Ok(rb.filled().len()))),
                Poll::Ready(Err(e)) => return Poll::Ready((i, Err(e))),
                Poll::Pending => {}
            }
        }
        Poll::Pending
    })
    .await
}

/// Read the destination address of an inner IP packet (IPv4 or IPv6) for
/// `allowed-ips` routing. Returns `None` for a truncated/unknown packet.
fn packet_dst_ip(pkt: &[u8]) -> Option<IpAddr> {
    match pkt.first()? >> 4 {
        4 if pkt.len() >= 20 => {
            let octets: [u8; 4] = pkt[16..20].try_into().ok()?;
            Some(IpAddr::V4(Ipv4Addr::from(octets)))
        }
        6 if pkt.len() >= 40 => {
            let octets: [u8; 16] = pkt[24..40].try_into().ok()?;
            Some(IpAddr::V6(Ipv6Addr::from(octets)))
        }
        _ => None,
    }
}

/// A relayed TCP stream over the tunnel: channel-backed `AsyncRead`/`AsyncWrite`
/// bridged to a smoltcp socket inside the device loop.
pub struct WgTcpStream {
    /// Caller -> loop bytes; dropped on shutdown to half-close the socket.
    write_tx: Option<mpsc::Sender<Vec<u8>>>,
    /// Loop -> caller bytes; closed (EOF) on peer FIN or device shutdown.
    read_rx: mpsc::Receiver<Vec<u8>>,
    wake: Arc<Notify>,
    writer_wakers: WriterWakers,
    leftover: Vec<u8>,
    leftover_pos: usize,
}

impl AsyncRead for WgTcpStream {
    fn poll_read(self: Pin<&mut Self>, cx: &mut TaskContext<'_>, buf: &mut ReadBuf<'_>) -> Poll<std::io::Result<()>> {
        let this = self.get_mut();
        loop {
            if this.leftover_pos < this.leftover.len() {
                let n = buf.remaining().min(this.leftover.len() - this.leftover_pos);
                buf.put_slice(&this.leftover[this.leftover_pos..this.leftover_pos + n]);
                this.leftover_pos += n;
                return Poll::Ready(Ok(()));
            }
            match this.read_rx.poll_recv(cx) {
                Poll::Ready(Some(data)) => {
                    this.leftover = data;
                    this.leftover_pos = 0;
                }
                Poll::Ready(None) => return Poll::Ready(Ok(())), // EOF
                Poll::Pending => return Poll::Pending,
            }
        }
    }
}

impl AsyncWrite for WgTcpStream {
    fn poll_write(self: Pin<&mut Self>, cx: &mut TaskContext<'_>, buf: &[u8]) -> Poll<std::io::Result<usize>> {
        let this = self.get_mut();
        let Some(tx) = &this.write_tx else {
            return Poll::Ready(Err(std::io::ErrorKind::BrokenPipe.into()));
        };
        if buf.is_empty() {
            return Poll::Ready(Ok(0));
        }
        let take = buf.len().min(FLOW_BUFFER);
        match tx.try_send(buf[..take].to_vec()) {
            Ok(()) => {
                this.wake.notify_one();
                Poll::Ready(Ok(take))
            }
            Err(mpsc::error::TrySendError::Full(_)) => {
                this.writer_wakers
                    .lock()
                    .expect("wireguard writer wakers")
                    .push(cx.waker().clone());
                this.wake.notify_one();
                Poll::Pending
            }
            Err(mpsc::error::TrySendError::Closed(_)) => Poll::Ready(Err(std::io::ErrorKind::BrokenPipe.into())),
        }
    }

    fn poll_flush(self: Pin<&mut Self>, _cx: &mut TaskContext<'_>) -> Poll<std::io::Result<()>> {
        Poll::Ready(Ok(()))
    }

    fn poll_shutdown(self: Pin<&mut Self>, _cx: &mut TaskContext<'_>) -> Poll<std::io::Result<()>> {
        let this = self.get_mut();
        // Dropping the write sender signals half-close to the loop, which FINs
        // the socket once buffered bytes are flushed.
        if this.write_tx.take().is_some() {
            this.wake.notify_one();
        }
        Poll::Ready(Ok(()))
    }
}

/// A relayed UDP association over the tunnel: a channel pair bridged to a
/// smoltcp UDP socket inside the device loop, sending to one fixed destination.
/// `send`/`recv` mirror the other protocols' UDP associations so the shared UDP
/// egress loop can drive it.
pub struct WgUdpAssoc {
    /// Caller -> loop datagrams.
    write_tx: mpsc::Sender<Vec<u8>>,
    /// Loop -> caller datagrams.
    read_rx: tokio::sync::Mutex<mpsc::Receiver<Vec<u8>>>,
    wake: Arc<Notify>,
}

impl WgUdpAssoc {
    /// Queue `payload` as one datagram to the association's destination. A full
    /// queue drops the datagram (UDP is lossy) rather than blocking the relay.
    pub async fn send(&self, payload: &[u8]) -> Result<()> {
        match self.write_tx.try_send(payload.to_vec()) {
            Ok(()) | Err(mpsc::error::TrySendError::Full(_)) => {
                self.wake.notify_one();
                Ok(())
            }
            Err(mpsc::error::TrySendError::Closed(_)) => bail!("wireguard udp: device loop is gone"),
        }
    }

    /// Receive the next reply datagram from the destination.
    pub async fn recv(&self) -> Result<Vec<u8>> {
        let mut rx = self.read_rx.lock().await;
        rx.recv()
            .await
            .ok_or_else(|| anyhow!("wireguard udp: device loop closed"))
    }
}

// --- smoltcp in-memory device --------------------------------------------------

/// In-memory smoltcp [`Device`](smoltcp::phy::Device) backed by two frame
/// queues: `tx` holds IP packets the stack wants encrypted + sent to the peer,
/// `rx` holds decrypted IP packets from the peer waiting to enter the stack.
struct WgPhy {
    rx: std::collections::VecDeque<Vec<u8>>,
    tx: std::collections::VecDeque<Vec<u8>>,
    mtu: usize,
}

impl WgPhy {
    fn new(mtu: usize) -> Self {
        Self {
            rx: std::collections::VecDeque::new(),
            tx: std::collections::VecDeque::new(),
            mtu,
        }
    }
}

struct PhyRxToken {
    buf: Vec<u8>,
}

struct PhyTxToken<'a> {
    tx: &'a mut std::collections::VecDeque<Vec<u8>>,
}

impl smoltcp::phy::Device for WgPhy {
    type RxToken<'a> = PhyRxToken;
    type TxToken<'a> = PhyTxToken<'a>;

    fn receive(&mut self, _t: SmolInstant) -> Option<(Self::RxToken<'_>, Self::TxToken<'_>)> {
        let buf = self.rx.pop_front()?;
        Some((PhyRxToken { buf }, PhyTxToken { tx: &mut self.tx }))
    }

    fn transmit(&mut self, _t: SmolInstant) -> Option<Self::TxToken<'_>> {
        Some(PhyTxToken { tx: &mut self.tx })
    }

    fn capabilities(&self) -> smoltcp::phy::DeviceCapabilities {
        let mut caps = smoltcp::phy::DeviceCapabilities::default();
        caps.medium = smoltcp::phy::Medium::Ip;
        caps.max_transmission_unit = self.mtu;
        caps
    }
}

impl smoltcp::phy::RxToken for PhyRxToken {
    fn consume<R, F: FnOnce(&[u8]) -> R>(self, f: F) -> R {
        f(&self.buf)
    }
}

impl smoltcp::phy::TxToken for PhyTxToken<'_> {
    fn consume<R, F: FnOnce(&mut [u8]) -> R>(self, len: usize, f: F) -> R {
        let mut buf = vec![0u8; len];
        let result = f(&mut buf);
        self.tx.push_back(buf);
        result
    }
}

/// Build the userspace interface, assigning the peer-given tunnel address(es) at
/// prefix 0 so every inner destination is treated as on-link (the tunnel is the
/// only egress) while replies still source from our assigned address.
fn build_interface(
    phy: &mut WgPhy,
    now: SmolInstant,
    local_v4: Option<Ipv4Addr>,
    local_v6: Option<Ipv6Addr>,
) -> Interface {
    let config = IfaceConfig::new(HardwareAddress::Ip);
    let mut iface = Interface::new(config, phy, now);
    iface.set_any_ip(true);
    iface.update_ip_addrs(|addrs| {
        if let Some(v4) = local_v4 {
            let _ = addrs.push(IpCidr::new(IpAddress::Ipv4(v4), 0));
        }
        if let Some(v6) = local_v6 {
            let _ = addrs.push(IpCidr::new(IpAddress::Ipv6(v6), 0));
        }
    });
    if let Some(v4) = local_v4 {
        let _ = iface.routes_mut().add_default_ipv4_route(v4);
    }
    if let Some(v6) = local_v6 {
        let _ = iface.routes_mut().add_default_ipv6_route(v6);
    }
    iface
}

fn smol_now(start: Instant) -> SmolInstant {
    SmolInstant::from_micros(start.elapsed().as_micros() as i64)
}

fn ip_address(ip: IpAddr) -> IpAddress {
    match ip {
        IpAddr::V4(v4) => IpAddress::Ipv4(v4),
        IpAddr::V6(v6) => IpAddress::Ipv6(v6),
    }
}

fn is_dead(state: tcp::State) -> bool {
    matches!(state, tcp::State::Closed | tcp::State::TimeWait | tcp::State::Closing)
}

/// Stamp the 3-byte WireGuard `reserved` field (bytes 1..4 of the message
/// header) on an outgoing datagram. A no-op for the default all-zero value.
fn apply_reserved(datagram: &mut [u8], reserved: [u8; 3]) {
    if reserved != [0u8; 3] && datagram.len() >= 4 {
        datagram[1..4].copy_from_slice(&reserved);
    }
}

/// Zero the `reserved` field before handing a received datagram to boringtun
/// (which validates those bytes are zero).
fn clear_reserved(datagram: &mut [u8]) {
    if datagram.len() >= 4 {
        datagram[1] = 0;
        datagram[2] = 0;
        datagram[3] = 0;
    }
}

/// Lowercase hex of a byte slice, used to fingerprint keys in the registry key.
fn hex(bytes: &[u8]) -> String {
    let mut s = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        s.push_str(&format!("{b:02x}"));
    }
    s
}

/// Parse the optional 3-byte `reserved` field (defaults to all-zero).
fn parse_reserved(bytes: Option<&[u8]>) -> Result<[u8; 3]> {
    match bytes {
        Some(bytes) => {
            if bytes.len() != 3 {
                bail!("wireguard: `reserved` must be exactly 3 bytes, got {}", bytes.len());
            }
            Ok([bytes[0], bytes[1], bytes[2]])
        }
        None => Ok([0u8; 3]),
    }
}

/// The default `allowed-ips` for a lone peer: route every inner destination.
fn catch_all() -> Vec<AllowedIp> {
    vec![
        AllowedIp::V4(Ipv4Addr::UNSPECIFIED, 0),
        AllowedIp::V6(Ipv6Addr::UNSPECIFIED, 0),
    ]
}

/// Parse a list of `allowed-ips` CIDR entries (`10.0.0.0/24`, `::/0`, or a bare
/// address meaning a host route).
fn parse_allowed_ips(list: &[String]) -> Result<Vec<AllowedIp>> {
    let mut out = Vec::with_capacity(list.len());
    for entry in list {
        out.push(parse_allowed_ip(entry).with_context(|| format!("wireguard: invalid `allowed-ips` entry {entry:?}"))?);
    }
    Ok(out)
}

fn parse_allowed_ip(entry: &str) -> Result<AllowedIp> {
    let entry = entry.trim();
    let (addr, prefix) = match entry.split_once('/') {
        Some((addr, prefix)) => (addr, Some(prefix)),
        None => (entry, None),
    };
    let ip = addr.parse::<IpAddr>().map_err(|_| anyhow!("not an IP/CIDR"))?;
    match ip {
        IpAddr::V4(v4) => {
            let p = match prefix {
                Some(p) => p.parse::<u8>().map_err(|_| anyhow!("bad prefix"))?,
                None => 32,
            };
            if p > 32 {
                bail!("IPv4 prefix {p} > 32");
            }
            Ok(AllowedIp::V4(v4, p))
        }
        IpAddr::V6(v6) => {
            let p = match prefix {
                Some(p) => p.parse::<u8>().map_err(|_| anyhow!("bad prefix"))?,
                None => 128,
            };
            if p > 128 {
                bail!("IPv6 prefix {p} > 128");
            }
            Ok(AllowedIp::V6(v6, p))
        }
    }
}

/// Parse a base64-encoded 32-byte WireGuard key.
fn parse_key(value: &str) -> Result<[u8; 32]> {
    let bytes = base64_decode(value.trim())?;
    if bytes.len() != 32 {
        bail!("expected a 32-byte key, decoded {} bytes", bytes.len());
    }
    let mut key = [0u8; 32];
    key.copy_from_slice(&bytes);
    Ok(key)
}

/// Parse the assigned IPv4 tunnel address (`ip`), accepting an optional CIDR
/// suffix (`10.0.0.2/32`).
fn parse_local_v4(value: &str) -> Result<Ipv4Addr> {
    value
        .trim()
        .split('/')
        .next()
        .unwrap_or("")
        .parse::<Ipv4Addr>()
        .map_err(|_| anyhow!("not an IPv4 address"))
}

/// Parse a `dns` resolver entry: a bare IP (port defaults to 53) or `ip:port`
/// (bracketed for IPv6).
fn parse_dns_server(value: &str) -> Result<SocketAddr> {
    let value = value.trim();
    if let Ok(addr) = value.parse::<SocketAddr>() {
        return Ok(addr);
    }
    let ip = value
        .parse::<IpAddr>()
        .map_err(|_| anyhow!("not an IP address or `ip:port`"))?;
    Ok(SocketAddr::new(ip, 53))
}

/// A fresh DNS transaction id. Per-query randomness mostly matters for spoofing
/// resistance; here each query rides a dedicated tunnel UDP socket, so this just
/// lets us reject a stale datagram from a prior retransmit.
fn dns_query_id() -> u16 {
    let mut bytes = [0u8; 2];
    let _ = getrandom::fill(&mut bytes);
    u16::from_ne_bytes(bytes)
}

/// Encode a recursive DNS query for `host` / `rtype` with transaction id `id`.
fn build_dns_query(host: &str, rtype: RecordType, id: u16) -> Result<Vec<u8>> {
    let fqdn = if host.ends_with('.') {
        host.to_string()
    } else {
        format!("{host}.")
    };
    let name = Name::from_utf8(&fqdn).with_context(|| format!("invalid DNS name {host:?}"))?;
    let mut msg = Message::new();
    msg.set_id(id);
    msg.set_message_type(MessageType::Query);
    msg.set_op_code(OpCode::Query);
    msg.set_recursion_desired(true);
    msg.add_query(Query::query(name, rtype));
    msg.to_vec().context("encode DNS query")
}

/// Extract the first address of the requested family from a DNS response, after
/// checking the transaction id matches. Returns `None` for a mismatched id or a
/// response with no usable answer (e.g. NODATA / NXDOMAIN).
fn parse_dns_answer(resp: &[u8], id: u16, rtype: RecordType) -> Option<IpAddr> {
    let msg = Message::from_vec(resp).ok()?;
    if msg.id() != id {
        return None;
    }
    for answer in msg.answers() {
        match (rtype, answer.data()) {
            (RecordType::A, Some(RData::A(A(ip)))) => return Some(IpAddr::V4(*ip)),
            (RecordType::AAAA, Some(RData::AAAA(AAAA(ip)))) => return Some(IpAddr::V6(*ip)),
            _ => {}
        }
    }
    None
}

/// Decode standard or URL-safe Base64 (padding / whitespace ignored).
fn base64_decode(input: &str) -> Result<Vec<u8>> {
    fn sextet(c: u8) -> Option<u8> {
        match c {
            b'A'..=b'Z' => Some(c - b'A'),
            b'a'..=b'z' => Some(c - b'a' + 26),
            b'0'..=b'9' => Some(c - b'0' + 52),
            b'+' | b'-' => Some(62),
            b'/' | b'_' => Some(63),
            _ => None,
        }
    }
    let mut out = Vec::with_capacity(input.len() / 4 * 3);
    let mut acc = 0u32;
    let mut bits = 0u32;
    for &c in input.as_bytes() {
        if c == b'=' || c.is_ascii_whitespace() {
            continue;
        }
        let v = sextet(c).ok_or_else(|| anyhow!("invalid base64 character {:?}", c as char))?;
        acc = (acc << 6) | u32::from(v);
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push((acc >> bits) as u8);
        }
    }
    Ok(out)
}
