//! Kernel DNS server: fake-IP allocation and upstream forwarding.
//!
//! Two modes, selected per server:
//! - [`DnsMode::FakeIp`]: answer `A` queries with a synthetic IP drawn from a
//!   CIDR pool, recording a bidirectional `domain <-> ip` mapping. The router
//!   can later recover the original hostname from a fake IP (reverse lookup),
//!   so connections to a fake IP route by domain. `AAAA` queries get an empty
//!   `NOERROR` answer so clients fall back to the fake `A` (matching Clash's
//!   fake-ip behaviour); other record types are refused with `NotImp`.
//! - [`DnsMode::Forward`]: forward the query verbatim to an upstream resolver
//!   over UDP and return its response unchanged.
//!
//! We own the product logic (pool allocation, mapping, mode selection) and
//! delegate only the DNS wire format to `hickory-proto`.

use std::collections::HashMap;
use std::net::{Ipv4Addr, Ipv6Addr, SocketAddr};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use anyhow::{Context, Result, anyhow, bail};
use hickory_proto::op::{Message, MessageType, OpCode, ResponseCode};
use hickory_proto::rr::rdata::A;
use hickory_proto::rr::{RData, Record, RecordType};
use tokio::net::UdpSocket;
use tokio::sync::Notify;
use tokio::task::JoinHandle;
use tokio::time::timeout;

use crate::address::TargetAddr;

/// TTL handed out for fake-IP answers. Short, since the mapping is an internal
/// routing handle rather than a real record clients should cache.
const FAKE_IP_TTL: u32 = 1;

/// Max UDP DNS payload we read/forward. Large enough for EDNS responses.
const MAX_DNS_UDP: usize = 4096;

/// How long to wait for an upstream resolver to answer a forwarded query.
const UPSTREAM_TIMEOUT: Duration = Duration::from_secs(5);

/// CIDR the fake-IP pool draws synthetic addresses from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FakeIpConfig {
    base: u32,
    capacity: u32,
}

impl FakeIpConfig {
    /// Build a pool config from a network base and prefix length. The network
    /// (`.0`) and broadcast (`.255…`) addresses are reserved, so usable
    /// capacity is `2^(32-prefix) - 2`. `prefix` must be in `1..=30` and
    /// `network` must already be masked to the prefix.
    pub fn new(network: Ipv4Addr, prefix: u8) -> Result<Self> {
        if !(1..=30).contains(&prefix) {
            bail!("fake-ip prefix /{prefix} out of range (expected 1..=30)");
        }
        let base = u32::from(network);
        let host_bits = 32 - u32::from(prefix);
        let size = 1u32 << host_bits;
        let mask = !(size - 1);
        if base & mask != base {
            bail!("fake-ip network {network} is not aligned to /{prefix}");
        }
        Ok(Self {
            base,
            // Reserve network + broadcast.
            capacity: size - 2,
        })
    }
}

impl Default for FakeIpConfig {
    /// `198.18.0.0/16`, the conventional Clash fake-IP range.
    fn default() -> Self {
        Self {
            base: u32::from(Ipv4Addr::new(198, 18, 0, 0)),
            capacity: (1u32 << 16) - 2,
        }
    }
}

/// A bidirectional `domain <-> fake IP` allocator over a CIDR. Allocation walks
/// the range cyclically; when it wraps, the oldest mapping on a slot is evicted
/// and its IP reused (an LRU-by-allocation-order ring).
#[derive(Debug)]
pub struct FakeIpPool {
    config: FakeIpConfig,
    cursor: u32,
    domain_to_ip: HashMap<String, Ipv4Addr>,
    ip_to_domain: HashMap<Ipv4Addr, String>,
}

impl FakeIpPool {
    pub fn new(config: FakeIpConfig) -> Self {
        Self {
            config,
            cursor: 0,
            domain_to_ip: HashMap::new(),
            ip_to_domain: HashMap::new(),
        }
    }

    /// Number of distinct fake IPs the pool can hold before it must recycle.
    pub fn capacity(&self) -> u32 {
        self.config.capacity
    }

    /// Allocate (or return the existing) fake IP for `domain`. Names are matched
    /// case-insensitively. Always succeeds: once full it recycles the slot the
    /// cursor lands on.
    pub fn allocate(&mut self, domain: &str) -> Ipv4Addr {
        let key = domain.to_ascii_lowercase();
        if let Some(&ip) = self.domain_to_ip.get(&key) {
            return ip;
        }
        // First usable host is base + 1 (base itself is the network address).
        let ip = Ipv4Addr::from(self.config.base + 1 + self.cursor);
        if let Some(old) = self.ip_to_domain.remove(&ip) {
            self.domain_to_ip.remove(&old);
        }
        self.ip_to_domain.insert(ip, key.clone());
        self.domain_to_ip.insert(key, ip);
        self.cursor = (self.cursor + 1) % self.config.capacity;
        ip
    }

    /// Reverse lookup: the domain a fake IP currently maps to, if any.
    pub fn domain_for(&self, ip: Ipv4Addr) -> Option<&str> {
        self.ip_to_domain.get(&ip).map(String::as_str)
    }

    /// Whether `ip` falls inside this pool's CIDR (the network/broadcast ends
    /// included), i.e. it could be a fake IP this pool hands out.
    pub fn contains(&self, ip: Ipv4Addr) -> bool {
        let value = u32::from(ip);
        value >= self.config.base && value < self.config.base + self.config.capacity + 2
    }
}

/// Rewrite a fake IP back to its original domain so routing and the outbound
/// see the real host. If `target` is an IPv4 address this pool handed out,
/// return `TargetAddr::Domain` with the recorded host (preserving the port);
/// otherwise return `target` unchanged. This is what makes fake-IP useful: a
/// client connects to the synthetic IP, and the kernel routes by the domain the
/// rules were written against.
pub fn unmap_fake_ip(pool: &Mutex<FakeIpPool>, target: TargetAddr) -> TargetAddr {
    if let TargetAddr::Ip(SocketAddr::V4(addr)) = &target
        && let Ok(pool) = pool.lock()
        && let Some(domain) = pool.domain_for(*addr.ip())
    {
        return TargetAddr::Domain(domain.to_string(), addr.port());
    }
    target
}

/// How a [`DnsServer`] answers queries.
#[derive(Clone)]
pub enum DnsMode {
    /// Forward every query to `upstream` over UDP and relay its reply verbatim.
    Forward { upstream: SocketAddr },
    /// Answer address queries from a shared fake-IP pool. The caller keeps the
    /// `Arc` so it can perform reverse lookups (e.g. from the routing path).
    FakeIp { pool: Arc<Mutex<FakeIpPool>> },
}

impl DnsMode {
    /// Convenience constructor for a fake-IP mode backed by a fresh pool.
    pub fn fake_ip(config: FakeIpConfig) -> (Self, Arc<Mutex<FakeIpPool>>) {
        let pool = Arc::new(Mutex::new(FakeIpPool::new(config)));
        (Self::FakeIp { pool: pool.clone() }, pool)
    }
}

/// Configuration for a kernel DNS server.
#[derive(Clone)]
pub struct DnsConfig {
    /// UDP address the DNS server listens on.
    pub listen: SocketAddr,
    /// How queries are answered.
    pub mode: DnsMode,
}

/// The kernel DNS server. Owns a UDP listener task.
pub struct DnsServer;

impl DnsServer {
    /// Bind the DNS UDP socket and start serving in a background task. The
    /// socket is bound before returning so callers observe bind failures
    /// synchronously.
    pub async fn start(config: DnsConfig) -> Result<DnsHandle> {
        let socket = UdpSocket::bind(config.listen)
            .await
            .with_context(|| format!("bind DNS inbound on {}", config.listen))?;
        let local_addr = socket.local_addr().unwrap_or(config.listen);

        let shutdown = Arc::new(Notify::new());
        let task_shutdown = shutdown.clone();
        let mode = Arc::new(config.mode);
        let task = tokio::spawn(async move {
            serve(socket, mode, task_shutdown).await;
        });

        log::info!("learn-gripe DNS inbound listening on {local_addr}");
        Ok(DnsHandle {
            local_addr,
            shutdown,
            task,
        })
    }
}

/// Handle to a running DNS server. Call [`DnsHandle::shutdown`] to stop it.
#[derive(Debug)]
pub struct DnsHandle {
    local_addr: SocketAddr,
    shutdown: Arc<Notify>,
    task: JoinHandle<()>,
}

impl DnsHandle {
    /// The address the DNS server is actually bound to (resolves an ephemeral
    /// port 0 to the chosen port).
    pub fn local_addr(&self) -> SocketAddr {
        self.local_addr
    }

    /// Stop the server task and wait for it to wind down.
    pub async fn shutdown(self) {
        self.shutdown.notify_waiters();
        self.task.abort();
        let _ = self.task.await;
    }
}

async fn serve(socket: UdpSocket, mode: Arc<DnsMode>, shutdown: Arc<Notify>) {
    let socket = Arc::new(socket);
    let mut buf = vec![0u8; MAX_DNS_UDP];
    loop {
        tokio::select! {
            _ = shutdown.notified() => {
                log::info!("learn-gripe DNS inbound shutting down");
                return;
            }
            res = socket.recv_from(&mut buf) => {
                let (n, peer) = match res {
                    Ok(v) => v,
                    Err(_) => continue,
                };
                let query = buf[..n].to_vec();
                let mode = mode.clone();
                let socket = socket.clone();
                tokio::spawn(async move {
                    match answer_query(&query, &mode).await {
                        Ok(response) => {
                            let _ = socket.send_to(&response, peer).await;
                        }
                        Err(err) => log::debug!("learn-gripe dns: dropped query from {peer}: {err:#}"),
                    }
                });
            }
        }
    }
}

/// Produce the response bytes for one query under `mode`. Public so other
/// inbounds (e.g. the TUN stack answering DNS over the virtual interface) can
/// reuse the exact same fake-IP/forward logic instead of duplicating it.
pub async fn answer_query(query: &[u8], mode: &DnsMode) -> Result<Vec<u8>> {
    match mode {
        DnsMode::Forward { upstream } => forward(query, *upstream).await,
        DnsMode::FakeIp { pool } => build_fake_ip_response(query, pool),
    }
}

/// Forward a raw query to an upstream resolver over UDP and return its reply.
async fn forward(query: &[u8], upstream: SocketAddr) -> Result<Vec<u8>> {
    let bind: SocketAddr = match upstream {
        SocketAddr::V4(_) => (Ipv4Addr::UNSPECIFIED, 0).into(),
        SocketAddr::V6(_) => (Ipv6Addr::UNSPECIFIED, 0).into(),
    };
    let socket = UdpSocket::bind(bind).await.context("bind dns upstream socket")?;
    socket
        .connect(upstream)
        .await
        .with_context(|| format!("connect dns upstream {upstream}"))?;
    socket.send(query).await.context("send to dns upstream")?;

    let mut buf = vec![0u8; MAX_DNS_UDP];
    let n = timeout(UPSTREAM_TIMEOUT, socket.recv(&mut buf))
        .await
        .with_context(|| format!("dns upstream {upstream} timed out"))?
        .context("recv from dns upstream")?;
    buf.truncate(n);
    Ok(buf)
}

/// Build a fake-IP response for `query`: synthesize an `A` answer per `A` query
/// from the pool, return an empty `NOERROR` for `AAAA`, and `NotImp` otherwise.
fn build_fake_ip_response(query: &[u8], pool: &Arc<Mutex<FakeIpPool>>) -> Result<Vec<u8>> {
    let request = Message::from_vec(query).context("parse dns query")?;

    let mut response = Message::new();
    response.set_id(request.id());
    response.set_message_type(MessageType::Response);
    response.set_op_code(OpCode::Query);
    response.set_recursion_desired(request.recursion_desired());
    response.set_recursion_available(true);
    response.set_response_code(ResponseCode::NoError);

    for query in request.queries() {
        response.add_query(query.clone());
        match query.query_type() {
            RecordType::A => {
                let domain = query.name().to_ascii();
                let domain = domain.trim_end_matches('.');
                let ip = {
                    let mut pool = pool.lock().map_err(|_| anyhow!("fake-ip pool poisoned"))?;
                    pool.allocate(domain)
                };
                response.add_answer(Record::from_rdata(query.name().clone(), FAKE_IP_TTL, RData::A(A(ip))));
            }
            // No AAAA answer: clients fall back to the fake A (Clash behaviour).
            RecordType::AAAA => {}
            _ => {
                response.set_response_code(ResponseCode::NotImp);
            }
        }
    }

    response.to_vec().context("serialize dns response")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn allocate_is_stable_and_reverses() {
        let mut pool = FakeIpPool::new(FakeIpConfig::default());
        let first = pool.allocate("example.com");
        // First usable host of 198.18.0.0/16 is 198.18.0.1.
        assert_eq!(first, Ipv4Addr::new(198, 18, 0, 1));
        // Same domain (any case) returns the same IP.
        assert_eq!(pool.allocate("EXAMPLE.com"), first);
        // A different domain takes the next slot.
        assert_eq!(pool.allocate("other.test"), Ipv4Addr::new(198, 18, 0, 2));
        // Reverse lookup recovers the (lowercased) domain.
        assert_eq!(pool.domain_for(first), Some("example.com"));
        assert!(pool.contains(first));
        assert!(!pool.contains(Ipv4Addr::new(10, 0, 0, 1)));
    }

    #[test]
    fn pool_recycles_when_full() {
        // /30 -> 4 addresses -> 2 usable (.1, .2).
        let config = FakeIpConfig::new(Ipv4Addr::new(198, 18, 0, 0), 30).unwrap();
        let mut pool = FakeIpPool::new(config);
        assert_eq!(pool.capacity(), 2);
        let a = pool.allocate("a.test");
        let b = pool.allocate("b.test");
        assert_eq!(a, Ipv4Addr::new(198, 18, 0, 1));
        assert_eq!(b, Ipv4Addr::new(198, 18, 0, 2));
        // Third allocation wraps and evicts a.test's slot.
        let c = pool.allocate("c.test");
        assert_eq!(c, a);
        assert_eq!(pool.domain_for(a), Some("c.test"));
        assert_eq!(pool.domain_for(b), Some("b.test"));
    }

    #[test]
    fn unmap_rewrites_only_pool_ips() {
        let pool = Mutex::new(FakeIpPool::new(FakeIpConfig::default()));
        let fake = pool.lock().unwrap().allocate("example.com");

        // A fake IP becomes its domain, keeping the port.
        let mapped = unmap_fake_ip(&pool, TargetAddr::Ip(SocketAddr::from((fake, 443))));
        assert_eq!(mapped, TargetAddr::Domain("example.com".to_string(), 443));

        // An IP outside the pool is left untouched.
        let real = SocketAddr::from((Ipv4Addr::new(1, 1, 1, 1), 80));
        assert_eq!(unmap_fake_ip(&pool, TargetAddr::Ip(real)), TargetAddr::Ip(real));

        // A domain target is left untouched.
        let domain = TargetAddr::Domain("other.test".to_string(), 53);
        assert_eq!(unmap_fake_ip(&pool, domain.clone()), domain);

        // A pool IP with no mapping (never allocated) is left as-is.
        let unmapped = SocketAddr::from((Ipv4Addr::new(198, 18, 5, 5), 22));
        assert_eq!(unmap_fake_ip(&pool, TargetAddr::Ip(unmapped)), TargetAddr::Ip(unmapped));
    }

    #[test]
    fn rejects_misaligned_or_oversized_prefix() {
        assert!(FakeIpConfig::new(Ipv4Addr::new(198, 18, 0, 1), 16).is_err());
        assert!(FakeIpConfig::new(Ipv4Addr::new(198, 18, 0, 0), 31).is_err());
        assert!(FakeIpConfig::new(Ipv4Addr::new(198, 18, 0, 0), 0).is_err());
    }

    #[test]
    fn fake_ip_response_answers_a_and_empties_aaaa() {
        use hickory_proto::op::Query;
        use hickory_proto::rr::Name;
        use std::str::FromStr;

        let (mode, pool) = DnsMode::fake_ip(FakeIpConfig::default());
        let DnsMode::FakeIp { pool: _ } = &mode else {
            panic!("expected fake-ip mode");
        };

        let mut request = Message::new();
        request.set_id(0x1234);
        request.set_message_type(MessageType::Query);
        request.set_op_code(OpCode::Query);
        request.add_query(Query::query(Name::from_str("example.com.").unwrap(), RecordType::A));
        let bytes = request.to_vec().unwrap();

        let response = build_fake_ip_response(&bytes, &pool).unwrap();
        let parsed = Message::from_vec(&response).unwrap();
        assert_eq!(parsed.id(), 0x1234);
        assert_eq!(parsed.answers().len(), 1);
        match parsed.answers()[0].data() {
            Some(RData::A(A(ip))) => {
                assert_eq!(*ip, Ipv4Addr::new(198, 18, 0, 1));
                assert_eq!(pool.lock().unwrap().domain_for(*ip), Some("example.com"));
            }
            other => panic!("expected A answer, got {other:?}"),
        }

        // AAAA query -> NOERROR with no answers.
        let mut aaaa = Message::new();
        aaaa.set_id(7);
        aaaa.add_query(Query::query(Name::from_str("example.com.").unwrap(), RecordType::AAAA));
        let parsed = Message::from_vec(&build_fake_ip_response(&aaaa.to_vec().unwrap(), &pool).unwrap()).unwrap();
        assert_eq!(parsed.response_code(), ResponseCode::NoError);
        assert!(parsed.answers().is_empty());
    }
}
