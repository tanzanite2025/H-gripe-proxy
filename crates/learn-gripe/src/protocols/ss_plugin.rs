//! SIP003 plugin transports for Shadowsocks.
//!
//! A SIP003 `plugin` wraps the Shadowsocks byte stream in another transport
//! before the AEAD framing is applied, so the proxy looks like ordinary web
//! traffic. This module maps the clash/mihomo `plugin` + `plugin-opts` fields
//! onto a resolved [`SsPlugin`] and dials it, returning a [`BoxedStream`] that
//! the Shadowsocks layer then runs its salt + AEAD chunks over — exactly as it
//! would over a raw socket.
//!
//! Implemented:
//! - `obfs` (simple-obfs) **http** mode — fake WebSocket-upgrade header.
//! - `v2ray-plugin` **websocket** mode, optionally over TLS — reuses the
//!   kernel's vetted [`ws`](crate::transport::ws) and [`tls`](crate::transport::tls)
//!   transports.
//!
//! Not implemented (rejected rather than mis-framed): simple-obfs `tls`
//! (fake-TLS) mode, and v2ray-plugin non-websocket modes / `mux`.

use anyhow::{Context, Result, anyhow, bail};
use tokio::net::TcpStream;

use crate::config::outbound_opts::PluginOpts;
use crate::outbound::BoxedStream;
use crate::transport::simple_obfs;
use crate::transport::tls::{ClientFingerprint, TlsClientConfig};
use crate::transport::ws::WsTransportConfig;

/// A resolved SIP003 plugin transport for a Shadowsocks outbound.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SsPlugin {
    /// simple-obfs (obfs-local) HTTP mode: a fake WebSocket-upgrade request
    /// frames the stream; `host`/`path` populate the request line and header.
    ObfsHttp { host: String, path: String },
    /// v2ray-plugin websocket mode, optionally over TLS.
    V2rayWebsocket {
        ws: WsTransportConfig,
        tls: Option<TlsClientConfig>,
    },
}

impl SsPlugin {
    /// Resolve a `plugin` name plus its `plugin-opts` into a transport, or
    /// `None` when no plugin is configured. Unsupported plugins/modes are
    /// rejected so traffic is never silently mis-framed.
    pub fn parse(plugin: Option<&str>, opts: Option<&PluginOpts>) -> Result<Option<Self>> {
        let plugin = match plugin.map(str::trim).filter(|s| !s.is_empty()) {
            None => return Ok(None),
            Some(p) => p,
        };
        let default = PluginOpts::default();
        let opts = opts.unwrap_or(&default);

        match plugin {
            "obfs" | "obfs-local" | "simple-obfs" => Self::parse_obfs(opts).map(Some),
            "v2ray-plugin" => Self::parse_v2ray(opts).map(Some),
            other => bail!("shadowsocks: plugin {other:?} not supported (only obfs / v2ray-plugin)"),
        }
    }

    fn parse_obfs(opts: &PluginOpts) -> Result<Self> {
        match opts.mode.as_deref().unwrap_or("http") {
            "http" => Ok(SsPlugin::ObfsHttp {
                host: opts
                    .host
                    .clone()
                    .filter(|s| !s.is_empty())
                    .unwrap_or_else(|| "bing.com".to_string()),
                path: opts
                    .path
                    .clone()
                    .filter(|s| !s.is_empty())
                    .unwrap_or_else(|| "/".to_string()),
            }),
            "tls" => bail!(
                "shadowsocks: simple-obfs tls (fake-TLS) mode not implemented yet \
                 (use obfs http or v2ray-plugin)"
            ),
            other => bail!("shadowsocks: unknown simple-obfs mode {other:?} (use http)"),
        }
    }

    fn parse_v2ray(opts: &PluginOpts) -> Result<Self> {
        match opts.mode.as_deref().unwrap_or("websocket") {
            "websocket" => {}
            other => bail!("shadowsocks: v2ray-plugin mode {other:?} not implemented yet (only websocket)"),
        }
        if opts.mux == Some(true) {
            bail!("shadowsocks: v2ray-plugin mux not implemented yet");
        }

        let ws = WsTransportConfig {
            path: opts
                .path
                .clone()
                .filter(|s| !s.is_empty())
                .unwrap_or_else(|| "/".to_string()),
            host: opts.host.clone().filter(|s| !s.is_empty()),
            headers: opts.headers.clone().unwrap_or_default(),
        };

        let tls = if opts.tls == Some(true) {
            let client_fingerprint = match opts.fingerprint.as_deref() {
                None | Some("") => None,
                Some(value) => {
                    Some(ClientFingerprint::parse(value).map_err(|e| anyhow!("shadowsocks: v2ray-plugin: {e}"))?)
                }
            };
            Some(TlsClientConfig {
                server_name: opts.host.clone().filter(|s| !s.is_empty()),
                alpn: Vec::new(),
                skip_cert_verify: opts.skip_cert_verify.unwrap_or(false),
                client_fingerprint,
            })
        } else {
            None
        };

        Ok(SsPlugin::V2rayWebsocket { ws, tls })
    }

    /// Dial the plugin transport to `server:port` and return a relay-ready byte
    /// stream onto which the Shadowsocks layer writes its salt + AEAD chunks.
    pub async fn connect(&self, server: &str, port: u16) -> Result<BoxedStream> {
        let tcp = TcpStream::connect((server, port))
            .await
            .with_context(|| format!("shadowsocks plugin: dial {server}:{port}"))?;

        match self {
            SsPlugin::ObfsHttp { host, path } => {
                let stream = simple_obfs::connect_http(tcp, host, path).await?;
                Ok(Box::new(stream))
            }
            SsPlugin::V2rayWebsocket { ws, tls } => {
                let secured: BoxedStream = match tls {
                    None => Box::new(tcp),
                    Some(cfg) => Box::new(crate::transport::tls::connect(cfg, server, tcp).await?),
                };
                let stream = crate::transport::ws::connect(secured, server, ws).await?;
                Ok(Box::new(stream))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_plugin_resolves_to_none() {
        assert_eq!(SsPlugin::parse(None, None).unwrap(), None);
        assert_eq!(SsPlugin::parse(Some(""), None).unwrap(), None);
    }

    #[test]
    fn obfs_http_defaults() {
        let plugin = SsPlugin::parse(Some("obfs"), None).unwrap().unwrap();
        assert_eq!(
            plugin,
            SsPlugin::ObfsHttp {
                host: "bing.com".to_string(),
                path: "/".to_string(),
            }
        );
    }

    #[test]
    fn obfs_http_uses_opts() {
        let opts = PluginOpts {
            mode: Some("http".to_string()),
            host: Some("www.example.com".to_string()),
            path: Some("/ray".to_string()),
            ..Default::default()
        };
        let plugin = SsPlugin::parse(Some("obfs-local"), Some(&opts)).unwrap().unwrap();
        assert_eq!(
            plugin,
            SsPlugin::ObfsHttp {
                host: "www.example.com".to_string(),
                path: "/ray".to_string(),
            }
        );
    }

    #[test]
    fn obfs_tls_is_rejected() {
        let opts = PluginOpts {
            mode: Some("tls".to_string()),
            ..Default::default()
        };
        let err = SsPlugin::parse(Some("obfs"), Some(&opts)).unwrap_err();
        assert!(err.to_string().contains("tls"), "got: {err}");
    }

    #[test]
    fn v2ray_websocket_plain() {
        let opts = PluginOpts {
            host: Some("cdn.example.com".to_string()),
            path: Some("/v2".to_string()),
            ..Default::default()
        };
        let plugin = SsPlugin::parse(Some("v2ray-plugin"), Some(&opts)).unwrap().unwrap();
        match plugin {
            SsPlugin::V2rayWebsocket { ws, tls } => {
                assert_eq!(ws.path, "/v2");
                assert_eq!(ws.host.as_deref(), Some("cdn.example.com"));
                assert!(tls.is_none());
            }
            other => panic!("expected websocket, got {other:?}"),
        }
    }

    #[test]
    fn v2ray_websocket_tls() {
        let opts = PluginOpts {
            tls: Some(true),
            host: Some("cdn.example.com".to_string()),
            skip_cert_verify: Some(true),
            ..Default::default()
        };
        let plugin = SsPlugin::parse(Some("v2ray-plugin"), Some(&opts)).unwrap().unwrap();
        match plugin {
            SsPlugin::V2rayWebsocket { tls, .. } => {
                let tls = tls.expect("tls enabled");
                assert_eq!(tls.server_name.as_deref(), Some("cdn.example.com"));
                assert!(tls.skip_cert_verify);
            }
            other => panic!("expected websocket, got {other:?}"),
        }
    }

    #[test]
    fn v2ray_mux_is_rejected() {
        let opts = PluginOpts {
            mux: Some(true),
            ..Default::default()
        };
        let err = SsPlugin::parse(Some("v2ray-plugin"), Some(&opts)).unwrap_err();
        assert!(err.to_string().contains("mux"), "got: {err}");
    }

    #[test]
    fn unknown_plugin_is_rejected() {
        let err = SsPlugin::parse(Some("kcptun"), None).unwrap_err();
        assert!(err.to_string().contains("not supported"), "got: {err}");
    }
}
