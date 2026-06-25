use crate::config::GripeConfig;
use crate::{outbound, socks5, udp};
use anyhow::{Context, Result};
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::sync::Arc;
use tokio::net::{TcpListener, TcpStream, UdpSocket};
use tokio::sync::Notify;
use tokio::task::JoinHandle;

/// The learn-gripe kernel. Owns the inbound listener task and exposes a handle
/// to stop it.
pub struct GripeKernel;

impl GripeKernel {
    /// Bind the SOCKS5 inbound and start serving in a background task. The
    /// listener is bound before returning so callers observe bind failures
    /// synchronously.
    pub async fn start(config: GripeConfig) -> Result<GripeHandle> {
        let listener = TcpListener::bind(config.socks_listen)
            .await
            .with_context(|| format!("bind SOCKS5 inbound on {}", config.socks_listen))?;
        let local_addr = listener.local_addr().unwrap_or(config.socks_listen);

        let shutdown = Arc::new(Notify::new());
        let config = Arc::new(config);
        let task_shutdown = shutdown.clone();
        let task = tokio::spawn(async move {
            serve(listener, config, task_shutdown).await;
        });

        log::info!("learn-gripe SOCKS5 inbound listening on {local_addr}");
        Ok(GripeHandle {
            local_addr,
            shutdown,
            task,
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
}

impl GripeHandle {
    /// The address the inbound is actually bound to (useful when the config
    /// requested an ephemeral port 0).
    pub fn local_addr(&self) -> SocketAddr {
        self.local_addr
    }

    /// Signal the accept loop to stop and wait for it to wind down.
    pub async fn shutdown(self) {
        self.shutdown.notify_waiters();
        self.task.abort();
        let _ = self.task.await;
    }
}

async fn serve(listener: TcpListener, config: Arc<GripeConfig>, shutdown: Arc<Notify>) {
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
                        tokio::spawn(async move {
                            if let Err(err) = handle_connection(stream, &config).await {
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

async fn handle_connection(mut inbound: TcpStream, config: &GripeConfig) -> Result<()> {
    socks5::server_handshake(&mut inbound).await?;
    let (command, target) = socks5::read_request(&mut inbound).await?;
    match command {
        socks5::Command::Connect => handle_connect(inbound, target, config).await,
        socks5::Command::UdpAssociate => handle_udp_associate(inbound, config).await,
    }
}

async fn handle_connect(mut inbound: TcpStream, target: crate::TargetAddr, config: &GripeConfig) -> Result<()> {
    let mut outbound = match outbound::connect(&config.outbound, &target).await {
        Ok(stream) => stream,
        Err(err) => {
            let _ = socks5::write_reply(&mut inbound, socks5::REP_GENERAL_FAILURE).await;
            return Err(err);
        }
    };

    socks5::write_reply(&mut inbound, socks5::REP_SUCCEEDED).await?;

    tokio::io::copy_bidirectional(&mut inbound, &mut outbound)
        .await
        .with_context(|| format!("relay to {target}"))?;
    Ok(())
}

async fn handle_udp_associate(mut inbound: TcpStream, config: &GripeConfig) -> Result<()> {
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
    udp::run_associate(inbound, relay, mode).await
}
