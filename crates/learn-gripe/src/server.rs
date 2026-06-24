use crate::config::GripeConfig;
use crate::{outbound, socks5};
use anyhow::{Context, Result};
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::{TcpListener, TcpStream};
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
    let target = socks5::read_connect_request(&mut inbound).await?;

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
