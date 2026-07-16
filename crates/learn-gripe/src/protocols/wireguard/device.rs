use std::collections::HashMap;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use anyhow::{Context, Result, anyhow, bail};
use boringtun::noise::Tunn;
use boringtun::x25519::{PublicKey, StaticSecret};
use hickory_proto::rr::RecordType;
use tokio::net::UdpSocket;
use tokio::sync::{mpsc, oneshot};

use super::WireGuardOutboundConfig;
use super::device_loop::{CHANNEL_DEPTH, DeviceLoop, PeerTunn};
use super::dns::{build_dns_query, dns_query_id, parse_dns_answer};
use super::stream::{WgTcpStream, WgUdpAssoc};

/// How long to wait for a tunnel-side DNS reply before retransmitting the query.
const DNS_QUERY_TIMEOUT: Duration = Duration::from_millis(800);
/// How many times a tunnel-side DNS query is (re)sent before giving up on a
/// resolver (the first few may be lost while the Noise handshake warms up).
const DNS_QUERY_RETRIES: usize = 5;

/// Registry key fingerprinting the interface key plus every peer endpoint/key,
/// so the same multi-peer config shares one device while a different peer set
/// gets its own.
pub(super) type WgKey = String;

/// Per-config registry of live tunnel devices, so concurrent connections to the
/// same peer share one Noise session + netstack (mirrors the AnyTLS session
/// registry). A device whose command channel has closed (its loop exited) is
/// discarded and rebuilt on the next connect.
static DEVICE_REGISTRY: Mutex<Option<HashMap<WgKey, Arc<WireGuardDevice>>>> = Mutex::new(None);

/// Command sent from a `connect` call into the device's poll loop.
pub(super) enum Command {
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
    pub(super) async fn get_or_create(config: &WireGuardOutboundConfig) -> Result<Arc<Self>> {
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

    pub(super) async fn open_tcp(&self, dst: SocketAddr) -> Result<WgTcpStream> {
        let (reply_tx, reply_rx) = oneshot::channel();
        self.commands
            .send(Command::OpenTcp { dst, reply: reply_tx })
            .await
            .map_err(|_| anyhow!("wireguard: device loop is gone"))?;
        reply_rx
            .await
            .map_err(|_| anyhow!("wireguard: connection to {dst} failed (handshake/connect timeout)"))
    }

    pub(super) async fn open_udp(&self, dst: SocketAddr) -> Result<WgUdpAssoc> {
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
    pub(super) async fn resolve_remote(&self, host: &str, config: &WireGuardOutboundConfig) -> Result<IpAddr> {
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
