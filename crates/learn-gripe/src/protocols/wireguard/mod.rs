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

mod amnezia;
mod device;
mod device_loop;
mod dns;
mod netstack;
mod parse;
mod stream;

use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};

use anyhow::{Context, Result, anyhow, bail};

use crate::address::TargetAddr;
use crate::config::outbound_opts::ProxyEntry;
use crate::outbound::BoxedStream;

use amnezia::Amnezia;
use device::WgKey;
use parse::{catch_all, hex, parse_allowed_ips, parse_dns_server, parse_key, parse_local_v4, parse_reserved};

pub use device::WireGuardDevice;
pub use stream::{WgTcpStream, WgUdpAssoc};

/// Default tunnel MTU (max inner IP packet); WireGuard adds a 32-byte overhead
/// on top, so the UDP datagram stays within a typical 1500-byte path.
const DEFAULT_MTU: u32 = 1408;

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
