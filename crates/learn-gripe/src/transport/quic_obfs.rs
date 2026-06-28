//! QUIC datagram obfuscation and port hopping for the Hysteria2 outbound.
//!
//! Both features live *below* QUIC, at the UDP datagram boundary, so they are
//! implemented as a [`quinn::AsyncUdpSocket`] wrapper around the runtime's real
//! socket. The wrapper transforms every datagram on the way out and in:
//!
//! - **Packet obfuscation** ([`PacketObfs`]): each outgoing datagram is
//!   XOR-masked with a fresh salt and each incoming one is unmasked, hiding the
//!   QUIC header from a passive censor. Hysteria2 uses Salamander
//!   ([`crate::protocols::salamander`], `obfs: salamander`) and Hysteria v1 uses
//!   XPlus ([`crate::protocols::xplus`], `obfs: <key>`); both share this socket.
//!   Because every datagram carries an independent salt (and therefore a
//!   different post-obfuscation length), GSO/GRO batching is disabled
//!   ([`max_transmit_segments`](AsyncUdpSocket::max_transmit_segments) = 1).
//!
//! - **Port hopping** (`ports: "1000-2000,3000"`): outgoing datagrams are
//!   re-targeted to a server port chosen at random from the configured ranges,
//!   rotated every `hop-interval`. The server is reachable on every port in the
//!   range (typically via a firewall redirect), so hopping spreads the flow
//!   across ports to evade port-based blocking and QoS. quinn still believes it
//!   is talking to a single fixed peer (`canonical`), so incoming datagrams have
//!   their source address normalized back to that peer before quinn sees them.
//!
//! The endpoint hosts exactly one connection (a fresh endpoint is built per
//! dial in [`super::quic::connect`]), which is what makes the unconditional
//! source-address normalization safe.

use std::io;
use std::net::SocketAddr;
use std::pin::Pin;
use std::sync::Arc;
use std::sync::Mutex;
use std::task::{Context, Poll};
use std::time::{Duration, Instant};

use anyhow::{Result, bail};
use quinn::udp::{RecvMeta, Transmit};
use quinn::{AsyncUdpSocket, UdpPoller};

use crate::protocols::salamander::Salamander;
use crate::protocols::xplus::XPlus;

/// Packet obfuscation applied below QUIC, at the UDP datagram boundary. Each
/// variant masks a datagram into `salt || XOR(payload)` and recovers it on the
/// way back; the two variants differ only in salt length and key-derivation
/// hash. Selected per outbound: Hysteria2 -> Salamander, Hysteria v1 -> XPlus.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PacketObfs {
    Salamander(Salamander),
    XPlus(XPlus),
}

impl PacketObfs {
    /// Mask `payload` into a fresh `salt || XOR(payload)` datagram.
    fn obfuscate(&self, payload: &[u8]) -> Vec<u8> {
        match self {
            PacketObfs::Salamander(s) => s.obfuscate(payload),
            PacketObfs::XPlus(x) => x.obfuscate(payload),
        }
    }

    /// Recover a `salt || ciphertext` datagram in place, returning the payload
    /// length, or `None` if it is too short to be valid.
    fn deobfuscate_in_place(&self, buf: &mut [u8]) -> Option<usize> {
        match self {
            PacketObfs::Salamander(s) => s.deobfuscate_in_place(buf),
            PacketObfs::XPlus(x) => x.deobfuscate_in_place(buf),
        }
    }
}

/// Default port-hopping rotation interval when `hop-interval` is unset
/// (matches the reference Hysteria2 client default of 30s).
const DEFAULT_HOP_INTERVAL: Duration = Duration::from_secs(30);

/// Parsed port-hopping configuration: the inclusive port ranges the server
/// listens on plus how often to rotate the active port.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PortHopConfig {
    /// Inclusive `(low, high)` port ranges; a single port is `(p, p)`.
    ranges: Vec<(u16, u16)>,
    /// Total number of ports across all ranges (for uniform selection).
    total: u32,
    interval: Duration,
}

impl PortHopConfig {
    /// Parse a clash/mihomo `ports` spec: comma-separated tokens, each a single
    /// port (`443`) or an inclusive range (`20000-50000`). `hop_interval` is in
    /// seconds (0 / unset → 30s default).
    pub fn parse(spec: &str, hop_interval: Option<u32>) -> Result<Self> {
        let mut ranges = Vec::new();
        let mut total: u32 = 0;
        for token in spec.split(',') {
            let token = token.trim();
            if token.is_empty() {
                continue;
            }
            let (lo, hi) = match token.split_once('-') {
                Some((lo, hi)) => (parse_port(lo)?, parse_port(hi)?),
                None => {
                    let p = parse_port(token)?;
                    (p, p)
                }
            };
            if lo > hi {
                bail!("hysteria2: invalid port range {token:?} (low > high)");
            }
            ranges.push((lo, hi));
            total += u32::from(hi - lo) + 1;
        }
        if ranges.is_empty() {
            bail!("hysteria2: ports {spec:?} contains no valid ports");
        }
        let interval = match hop_interval {
            Some(secs) if secs > 0 => Duration::from_secs(u64::from(secs)),
            _ => DEFAULT_HOP_INTERVAL,
        };
        Ok(Self {
            ranges,
            total,
            interval,
        })
    }

    /// The port at flat index `index` (`0..total`) across the ranges in order.
    fn port_at(&self, mut index: u32) -> u16 {
        for &(lo, hi) in &self.ranges {
            let span = u32::from(hi - lo) + 1;
            if index < span {
                return lo + index as u16;
            }
            index -= span;
        }
        // `index` is always taken modulo `total`, so a range always matches; the
        // final range's top is a safe fallback for the unreachable case.
        self.ranges.last().map(|&(_, hi)| hi).unwrap_or_default()
    }

    /// Pick a uniformly random port across the configured ranges.
    fn random_port(&self) -> u16 {
        self.port_at(random_u32() % self.total)
    }
}

/// Parse a single port token (1..=65535; 0 is not a dialable port).
fn parse_port(value: &str) -> Result<u16> {
    let port: u16 = value
        .trim()
        .parse()
        .map_err(|_| anyhow::anyhow!("hysteria2: invalid port {value:?}"))?;
    if port == 0 {
        bail!("hysteria2: port 0 is not valid");
    }
    Ok(port)
}

/// Live port-hopping state: the currently selected destination port and the
/// instant it should next rotate.
struct PortHopper {
    config: PortHopConfig,
    state: Mutex<HopState>,
}

struct HopState {
    port: u16,
    next_switch: Instant,
}

impl PortHopper {
    fn new(config: PortHopConfig) -> Self {
        let port = config.random_port();
        let next_switch = Instant::now() + config.interval;
        Self {
            config,
            state: Mutex::new(HopState { port, next_switch }),
        }
    }

    /// The destination port to send to right now, rotating to a fresh random
    /// port once the hop interval has elapsed.
    fn current_port(&self) -> u16 {
        let mut state = self.state.lock().expect("port hopper mutex poisoned");
        let now = Instant::now();
        if now >= state.next_switch {
            state.port = self.config.random_port();
            state.next_switch = now + self.config.interval;
        }
        state.port
    }
}

/// A [`quinn::AsyncUdpSocket`] that applies Salamander obfuscation and/or port
/// hopping around an inner runtime socket.
pub struct ObfsHopSocket {
    inner: Arc<dyn AsyncUdpSocket>,
    obfs: Option<PacketObfs>,
    hopper: Option<PortHopper>,
    /// The address quinn dials and associates with the connection. Outgoing
    /// datagrams keep this IP (only the port hops); incoming datagrams are
    /// reported as coming from here so quinn matches the single connection.
    canonical: SocketAddr,
}

impl ObfsHopSocket {
    /// Wrap `inner`, applying `obfs` and/or `port_hop` to datagrams to/from the
    /// `canonical` peer. At least one of `obfs`/`port_hop` is expected to be set
    /// (otherwise the plain socket should be used directly).
    pub fn new(
        inner: Arc<dyn AsyncUdpSocket>,
        canonical: SocketAddr,
        obfs: Option<PacketObfs>,
        port_hop: Option<PortHopConfig>,
    ) -> Self {
        Self {
            inner,
            obfs,
            hopper: port_hop.map(PortHopper::new),
            canonical,
        }
    }
}

impl std::fmt::Debug for ObfsHopSocket {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ObfsHopSocket")
            .field("obfs", &self.obfs.is_some())
            .field("port_hopping", &self.hopper.is_some())
            .field("canonical", &self.canonical)
            .finish()
    }
}

impl AsyncUdpSocket for ObfsHopSocket {
    fn create_io_poller(self: Arc<Self>) -> Pin<Box<dyn UdpPoller>> {
        self.inner.clone().create_io_poller()
    }

    fn try_send(&self, transmit: &Transmit<'_>) -> io::Result<()> {
        // Hop the destination port (keeping the server IP) when port hopping is
        // active; otherwise send where quinn asked.
        let destination = match &self.hopper {
            Some(hopper) => SocketAddr::new(self.canonical.ip(), hopper.current_port()),
            None => transmit.destination,
        };

        match &self.obfs {
            Some(obfs) => {
                // Each datagram gets an independent salt, so GSO is disabled and
                // `contents` is always a single datagram here.
                let obfuscated = obfs.obfuscate(transmit.contents);
                self.inner.try_send(&Transmit {
                    destination,
                    ecn: transmit.ecn,
                    contents: &obfuscated,
                    segment_size: None,
                    src_ip: transmit.src_ip,
                })
            }
            None => self.inner.try_send(&Transmit {
                destination,
                ecn: transmit.ecn,
                contents: transmit.contents,
                segment_size: transmit.segment_size,
                src_ip: transmit.src_ip,
            }),
        }
    }

    fn poll_recv(
        &self,
        cx: &mut Context<'_>,
        bufs: &mut [io::IoSliceMut<'_>],
        meta: &mut [RecvMeta],
    ) -> Poll<io::Result<usize>> {
        let poll = self.inner.poll_recv(cx, bufs, meta);
        let Poll::Ready(Ok(count)) = &poll else {
            return poll;
        };
        for i in 0..*count {
            if let Some(obfs) = &self.obfs {
                let len = meta[i].len;
                match obfs.deobfuscate_in_place(&mut bufs[i][..len]) {
                    Some(new_len) => {
                        meta[i].len = new_len;
                        meta[i].stride = new_len;
                    }
                    // Too short to be a valid obfuscated datagram: present it as
                    // empty so quinn drops it (stride must stay non-zero — quinn
                    // slices the buffer in `stride`-sized chunks).
                    None => {
                        meta[i].len = 0;
                        meta[i].stride = 1;
                    }
                }
            }
            // With port hopping the reply arrives from whichever port the server
            // used; quinn only knows the canonical peer, so normalize the source.
            if self.hopper.is_some() {
                meta[i].addr = self.canonical;
            }
        }
        poll
    }

    fn local_addr(&self) -> io::Result<SocketAddr> {
        self.inner.local_addr()
    }

    fn max_transmit_segments(&self) -> usize {
        if self.obfs.is_some() {
            1
        } else {
            self.inner.max_transmit_segments()
        }
    }

    fn max_receive_segments(&self) -> usize {
        if self.obfs.is_some() {
            1
        } else {
            self.inner.max_receive_segments()
        }
    }

    fn may_fragment(&self) -> bool {
        self.inner.may_fragment()
    }
}

/// Fill a `u32` from the OS CSPRNG.
fn random_u32() -> u32 {
    let mut bytes = [0u8; 4];
    getrandom::fill(&mut bytes).expect("os rng");
    u32::from_be_bytes(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_single_and_range_ports() {
        let cfg = PortHopConfig::parse("443", None).unwrap();
        assert_eq!(cfg.ranges, vec![(443, 443)]);
        assert_eq!(cfg.total, 1);
        assert_eq!(cfg.interval, DEFAULT_HOP_INTERVAL);

        let cfg = PortHopConfig::parse("1000-2000, 3000 ,4000-4002", Some(15)).unwrap();
        assert_eq!(cfg.ranges, vec![(1000, 2000), (3000, 3000), (4000, 4002)]);
        assert_eq!(cfg.total, 1001 + 1 + 3);
        assert_eq!(cfg.interval, Duration::from_secs(15));
    }

    #[test]
    fn rejects_bad_port_specs() {
        assert!(PortHopConfig::parse("", None).is_err());
        assert!(PortHopConfig::parse("0", None).is_err());
        assert!(PortHopConfig::parse("2000-1000", None).is_err());
        assert!(PortHopConfig::parse("70000", None).is_err());
        assert!(PortHopConfig::parse("abc", None).is_err());
    }

    #[test]
    fn zero_hop_interval_falls_back_to_default() {
        let cfg = PortHopConfig::parse("443", Some(0)).unwrap();
        assert_eq!(cfg.interval, DEFAULT_HOP_INTERVAL);
    }

    #[test]
    fn port_at_walks_ranges_in_order() {
        let cfg = PortHopConfig::parse("10-12,20,30-31", None).unwrap();
        // Flat indices: 0->10 1->11 2->12 3->20 4->30 5->31.
        let ports: Vec<u16> = (0..cfg.total).map(|i| cfg.port_at(i)).collect();
        assert_eq!(ports, vec![10, 11, 12, 20, 30, 31]);
    }

    #[test]
    fn random_port_stays_within_ranges() {
        let cfg = PortHopConfig::parse("100-105,200", None).unwrap();
        for _ in 0..1000 {
            let p = cfg.random_port();
            assert!((100..=105).contains(&p) || p == 200, "out-of-range port {p}");
        }
    }

    #[test]
    fn hopper_holds_port_until_interval_elapses() {
        // A long interval means the port never rotates within the test.
        let cfg = PortHopConfig::parse("100-200", Some(3600)).unwrap();
        let hopper = PortHopper::new(cfg);
        let first = hopper.current_port();
        for _ in 0..100 {
            assert_eq!(hopper.current_port(), first, "port must be stable within the interval");
        }
    }
}
