use crate::config::GripeConfig;
use crate::conntrack::{ConnMeta, ConnNetwork, ConnRegistry, ConnTableSnapshot, relay_tracked};
use crate::dns::{FakeIpPool, unmap_fake_ip};
use crate::{http, outbound, socks5, udp};
use anyhow::{Context, Result};
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::sync::{Arc, Mutex};
use tokio::net::{TcpListener, TcpStream, UdpSocket};
use tokio::sync::Notify;
use tokio::task::JoinHandle;

/// Optional fake-IP pool shared with a DNS server. When present, the inbound
/// rewrites a target that is a fake IP back to its original domain before
/// routing (see [`unmap_fake_ip`]).
type FakeIp = Option<Arc<Mutex<FakeIpPool>>>;

/// The learn-gripe kernel. Owns the inbound listener task and exposes a handle
/// to stop it.
pub struct GripeKernel;

impl GripeKernel {
    /// Bind the mixed (SOCKS5 + HTTP) inbound and start serving in a background
    /// task. The listener is bound before returning so callers observe bind
    /// failures synchronously.
    pub async fn start(config: GripeConfig) -> Result<GripeHandle> {
        Self::start_inner(config, None).await
    }

    /// Like [`GripeKernel::start`], but share a fake-IP `pool` (typically the
    /// one a [`crate::DnsServer`] hands out) so connections to a fake IP are
    /// routed by their original domain.
    pub async fn start_with_fake_ip(config: GripeConfig, pool: Arc<Mutex<FakeIpPool>>) -> Result<GripeHandle> {
        Self::start_inner(config, Some(pool)).await
    }

    async fn start_inner(config: GripeConfig, fake_ip: FakeIp) -> Result<GripeHandle> {
        let listener = TcpListener::bind(config.socks_listen)
            .await
            .with_context(|| format!("bind mixed inbound on {}", config.socks_listen))?;
        let local_addr = listener.local_addr().unwrap_or(config.socks_listen);

        let shutdown = Arc::new(Notify::new());
        let registry = Arc::new(ConnRegistry::default());
        let config = Arc::new(config);
        let task_shutdown = shutdown.clone();
        let task_registry = registry.clone();
        let task = tokio::spawn(async move {
            serve(listener, config, task_shutdown, fake_ip, task_registry).await;
        });

        log::info!("learn-gripe mixed (SOCKS5 + HTTP) inbound listening on {local_addr}");
        Ok(GripeHandle {
            local_addr,
            shutdown,
            task,
            registry,
        })
    }
}

/// Handle to a running kernel. Dropping it does not stop the kernel; call
/// [`GripeHandle::shutdown`] for a graceful stop.
#[derive(Debug)]
pub struct GripeHandle {
    local_addr: SocketAddr,
    shutdown: Arc<Notify>,
    task: JoinHandle<()>,
    registry: Arc<ConnRegistry>,
}

impl GripeHandle {
    /// The address the inbound is actually bound to (useful when the config
    /// requested an ephemeral port 0).
    pub fn local_addr(&self) -> SocketAddr {
        self.local_addr
    }

    /// Snapshot the live connection table plus cumulative byte totals. This is
    /// the in-process replacement for the Mihomo controller `/connections`
    /// query.
    pub fn connections(&self) -> ConnTableSnapshot {
        self.registry.snapshot()
    }

    /// Signal the connection with `id` to close. Returns `true` if it was live.
    /// Replaces the Mihomo controller `close_connection` call.
    pub fn close_connection(&self, id: u64) -> bool {
        self.registry.close(id)
    }

    /// Signal every live connection to close, returning the number signalled.
    pub fn close_all_connections(&self) -> usize {
        self.registry.close_all()
    }

    /// Signal the accept loop to stop and wait for it to wind down.
    pub async fn shutdown(self) {
        self.shutdown.notify_waiters();
        self.task.abort();
        let _ = self.task.await;
    }
}

async fn serve(
    listener: TcpListener,
    config: Arc<GripeConfig>,
    shutdown: Arc<Notify>,
    fake_ip: FakeIp,
    registry: Arc<ConnRegistry>,
) {
    loop {
        tokio::select! {
            _ = shutdown.notified() => {
                log::info!("learn-gripe inbound shutting down");
                return;
            }
            accepted = listener.accept() => {
                match accepted {
                    Ok((stream, peer)) => {
                        let config = config.clone();
                        let fake_ip = fake_ip.clone();
                        let registry = registry.clone();
                        tokio::spawn(async move {
                            if let Err(err) = handle_connection(stream, &config, fake_ip, &registry).await {
                                log::debug!("learn-gripe connection from {peer} ended: {err:#}");
                            }
                        });
                    }
                    Err(err) => {
                        log::warn!("learn-gripe accept error: {err:#}");
                    }
                }
            }
        }
    }
}

async fn handle_connection(
    mut inbound: TcpStream,
    config: &GripeConfig,
    fake_ip: FakeIp,
    registry: &Arc<ConnRegistry>,
) -> Result<()> {
    // The inbound is a mixed listener: SOCKS5 (RFC 1928) and HTTP proxy share
    // the same port the way the app's mixed-port does. Peek the first byte
    // (without consuming it) to pick the protocol: 0x05 is the SOCKS version,
    // anything else is the start of an HTTP request line.
    let mut first = [0u8; 1];
    if inbound.peek(&mut first).await? == 0 {
        return Ok(());
    }
    if first[0] != socks5::VERSION {
        return http::handle(inbound, config, fake_ip.as_ref(), registry).await;
    }

    socks5::server_handshake(&mut inbound).await?;
    let (command, mut target) = socks5::read_request(&mut inbound).await?;
    // Resolve a fake IP back to its domain so routing sees the real host.
    if let Some(pool) = &fake_ip {
        target = unmap_fake_ip(pool, target);
    }
    match command {
        socks5::Command::Connect => handle_connect(inbound, target, config, registry).await,
        socks5::Command::UdpAssociate => handle_udp_associate(inbound, config, fake_ip).await,
    }
}

async fn handle_connect(
    mut inbound: TcpStream,
    target: crate::TargetAddr,
    config: &GripeConfig,
    registry: &Arc<ConnRegistry>,
) -> Result<()> {
    let outbound = match outbound::connect(&config.outbound, &target).await {
        Ok(stream) => stream,
        Err(err) => {
            let _ = socks5::write_reply(&mut inbound, socks5::REP_GENERAL_FAILURE).await;
            return Err(err);
        }
    };

    socks5::write_reply(&mut inbound, socks5::REP_SUCCEEDED).await?;

    let meta = ConnMeta::for_target(
        ConnNetwork::Tcp,
        inbound.peer_addr().ok(),
        inbound.local_addr().ok(),
        &config.outbound,
        &target,
    );
    let conn = registry.register(meta);
    relay_tracked(inbound, outbound, &conn)
        .await
        .with_context(|| format!("relay to {target}"))?;
    Ok(())
}

async fn handle_udp_associate(mut inbound: TcpStream, config: &GripeConfig, fake_ip: FakeIp) -> Result<()> {
    if !outbound::supports_udp_associate(&config.outbound) {
        let _ = socks5::write_reply(&mut inbound, socks5::REP_CMD_NOT_SUPPORTED).await;
        anyhow::bail!("udp associate not supported for the configured outbound");
    }

    // Bind the relay socket on the same interface the client reached us on so
    // the address we hand back is routable for it.
    let local_ip = inbound
        .local_addr()
        .map(|addr| addr.ip())
        .unwrap_or(IpAddr::V4(Ipv4Addr::LOCALHOST));
    let relay = match UdpSocket::bind((local_ip, 0)).await {
        Ok(socket) => socket,
        Err(err) => {
            let _ = socks5::write_reply(&mut inbound, socks5::REP_GENERAL_FAILURE).await;
            return Err(err).context("bind udp relay socket");
        }
    };
    let relay_addr = relay.local_addr().context("udp relay local addr")?;
    socks5::write_reply_with_addr(&mut inbound, socks5::REP_SUCCEEDED, relay_addr).await?;

    let mode = Arc::new(config.outbound.clone());
    udp::run_associate(inbound, relay, mode, fake_ip).await
}
