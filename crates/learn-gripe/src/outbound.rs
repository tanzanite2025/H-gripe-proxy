use crate::address::TargetAddr;
use crate::config::OutboundMode;
use crate::socks5;
use crate::trojan;
use crate::vless;
use anyhow::{Context, Result};
use tokio::io::{AsyncRead, AsyncWrite};
use tokio::net::TcpStream;

/// A relay-ready outbound stream. Different outbounds wrap the underlying
/// socket differently (raw TCP, TLS, protocol framing), so the data plane
/// works against this boxed trait object.
pub trait AsyncStream: AsyncRead + AsyncWrite + Unpin + Send {}
impl<T: AsyncRead + AsyncWrite + Unpin + Send> AsyncStream for T {}

/// Boxed outbound stream used by the relay loop.
pub type BoxedStream = Box<dyn AsyncStream>;

/// Establish an outbound connection to `target` according to `mode` and return
/// a stream that is ready for relaying.
pub async fn connect(mode: &OutboundMode, target: &TargetAddr) -> Result<BoxedStream> {
    match mode {
        OutboundMode::Direct => Ok(Box::new(dial_direct(target).await?)),
        OutboundMode::Socks5Upstream { addr } => {
            let mut stream = TcpStream::connect(addr)
                .await
                .with_context(|| format!("connect upstream SOCKS5 {addr}"))?;
            socks5::client_connect(&mut stream, target)
                .await
                .with_context(|| format!("upstream CONNECT to {target}"))?;
            Ok(Box::new(stream))
        }
        OutboundMode::Vless(config) => vless::connect(config, target).await,
        OutboundMode::Trojan(config) => trojan::connect(config, target).await,
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
