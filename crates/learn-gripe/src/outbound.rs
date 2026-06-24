use crate::address::TargetAddr;
use crate::config::OutboundMode;
use crate::socks5;
use anyhow::{Context, Result};
use tokio::net::TcpStream;

/// Establish an outbound TCP connection to `target` according to `mode` and
/// return a stream that is ready for relaying.
pub async fn connect(mode: &OutboundMode, target: &TargetAddr) -> Result<TcpStream> {
    match mode {
        OutboundMode::Direct => dial_direct(target).await,
        OutboundMode::Socks5Upstream { addr } => {
            let mut stream = TcpStream::connect(addr)
                .await
                .with_context(|| format!("connect upstream SOCKS5 {addr}"))?;
            socks5::client_connect(&mut stream, target)
                .await
                .with_context(|| format!("upstream CONNECT to {target}"))?;
            Ok(stream)
        }
    }
}

async fn dial_direct(target: &TargetAddr) -> Result<TcpStream> {
    match target {
        TargetAddr::Ip(addr) => TcpStream::connect(addr)
            .await
            .with_context(|| format!("direct connect {addr}")),
        TargetAddr::Domain(host, port) => TcpStream::connect((host.as_str(), *port))
            .await
            .with_context(|| format!("direct connect {host}:{port}")),
    }
}
