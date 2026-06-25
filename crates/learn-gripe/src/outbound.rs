use crate::address::TargetAddr;
use crate::config::OutboundMode;
use crate::socks5;
use crate::trojan;
use crate::vless;
use crate::vmess;
use anyhow::{Context, Result, bail};
use std::future::Future;
use std::pin::Pin;
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
///
/// Boxed future so a [`OutboundMode::Routed`] outbound can recurse into the
/// selected sub-outbound.
pub fn connect<'a>(
    mode: &'a OutboundMode,
    target: &'a TargetAddr,
) -> Pin<Box<dyn Future<Output = Result<BoxedStream>> + Send + 'a>> {
    Box::pin(async move {
        match mode {
            OutboundMode::Direct => Ok(Box::new(dial_direct(target).await?) as BoxedStream),
            OutboundMode::Reject => bail!("connection to {target} rejected by rule"),
            OutboundMode::Socks5Upstream { addr } => {
                let mut stream = TcpStream::connect(addr)
                    .await
                    .with_context(|| format!("connect upstream SOCKS5 {addr}"))?;
                socks5::client_connect(&mut stream, target)
                    .await
                    .with_context(|| format!("upstream CONNECT to {target}"))?;
                Ok(Box::new(stream) as BoxedStream)
            }
            OutboundMode::Vless(config) => vless::connect(config, target).await,
            OutboundMode::Trojan(config) => trojan::connect(config, target).await,
            OutboundMode::Vmess(config) => vmess::connect(config, target).await,
            OutboundMode::Routed(router) => connect(router.select(target), target).await,
        }
    })
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
