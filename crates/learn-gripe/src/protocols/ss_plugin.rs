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
//! - `obfs` (simple-obfs) **http** and **tls** (fake-TLS) modes.
//! - `v2ray-plugin` **websocket** mode, optionally over TLS — reuses the
//!   kernel's vetted [`ws`](crate::transport::ws) and [`tls`](crate::transport::tls)
//!   transports — plus its `v2ray-http-upgrade` variant
//!   ([`httpupgrade`](crate::transport::httpupgrade)) and `mux` framing
//!   ([`v2ray_mux`](crate::transport::v2ray_mux)).
//! - `v2ray-plugin` **quic** mode over standard QUIC + TLS
//!   ([`v2ray_quic`](crate::transport::v2ray_quic)).

use anyhow::{Context, Result, anyhow, bail};
use tokio::net::TcpStream;

use crate::config::outbound_opts::PluginOpts;
use crate::outbound::BoxedStream;
use crate::transport::httpupgrade::HttpUpgradeTransportConfig;
use crate::transport::simple_obfs;
use crate::transport::tls::{ClientFingerprint, TlsClientConfig};
use crate::transport::v2ray_mux::V2rayMux;
use crate::transport::v2ray_quic::V2rayQuicConfig;
use crate::transport::ws::WsTransportConfig;

/// A resolved SIP003 plugin transport for a Shadowsocks outbound.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SsPlugin {
    /// simple-obfs (obfs-local) HTTP mode: a fake WebSocket-upgrade request
    /// frames the stream; `host`/`path` populate the request line and header.
    ObfsHttp { host: String, path: String },
    /// simple-obfs (obfs-local) TLS mode: a fake TLS 1.2 handshake frames the
    /// stream; `host` is sent as the SNI.
    ObfsTls { host: String },
    /// v2ray-plugin, optionally over TLS and/or wrapped in mux.cool framing.
    /// The concrete transport (`websocket` / `v2ray-http-upgrade` / `quic`) is
    /// chosen by [`V2rayStream`].
    V2ray {
        stream: V2rayStream,
        /// Outer TLS for the TCP-based transports. Always `None` for
        /// [`V2rayStream::Quic`], which carries TLS inside the QUIC handshake.
        tls: Option<TlsClientConfig>,
        /// Wrap the stream in mux.cool framing (`mux: true`). Never set for
        /// [`V2rayStream::Quic`] (QUIC multiplexes natively).
        mux: bool,
    },
}

/// The concrete v2ray-plugin transport carrying the Shadowsocks stream.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum V2rayStream {
    /// `mode: websocket` (the default): a WebSocket handshake.
    Websocket(WsTransportConfig),
    /// `mode: websocket` with `v2ray-http-upgrade: true`: V2Ray's leaner
    /// HTTP-Upgrade handshake instead of a full WebSocket one.
    HttpUpgrade(HttpUpgradeTransportConfig),
    /// `mode: quic`: standard QUIC + TLS.
    Quic(V2rayQuicConfig),
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
            "tls" => Ok(SsPlugin::ObfsTls {
                host: opts
                    .host
                    .clone()
                    .filter(|s| !s.is_empty())
                    .unwrap_or_else(|| "bing.com".to_string()),
            }),
            other => bail!("shadowsocks: unknown simple-obfs mode {other:?} (use http or tls)"),
        }
    }

    fn parse_v2ray(opts: &PluginOpts) -> Result<Self> {
        let mux = opts.mux == Some(true);
        match opts.mode.as_deref().unwrap_or("websocket") {
            "websocket" => Self::parse_v2ray_websocket(opts, mux),
            "quic" => {
                // `mux` multiplexes several Shadowsocks streams onto one TCP
                // transport; QUIC already multiplexes natively, so the upstream
                // plugin only offers `mux` in websocket mode.
                if mux {
                    bail!("shadowsocks: v2ray-plugin mux is websocket-only (quic multiplexes natively)");
                }
                let quic = V2rayQuicConfig {
                    // Empty falls back to the dial server at connect time.
                    server_name: opts.host.clone().filter(|s| !s.is_empty()).unwrap_or_default(),
                    // v2ray-core's TLS layer offers these by default when no ALPN
                    // is configured, which is what the plugin presents.
                    alpn: vec!["h2".to_string(), "http/1.1".to_string()],
                    skip_cert_verify: opts.skip_cert_verify.unwrap_or(false),
                };
                Ok(SsPlugin::V2ray {
                    stream: V2rayStream::Quic(quic),
                    tls: None,
                    mux: false,
                })
            }
            other => bail!("shadowsocks: v2ray-plugin mode {other:?} not supported (use websocket or quic)"),
        }
    }

    fn parse_v2ray_websocket(opts: &PluginOpts, mux: bool) -> Result<Self> {
        let path = opts
            .path
            .clone()
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| "/".to_string());
        let host = opts.host.clone().filter(|s| !s.is_empty());
        let headers = opts.headers.clone().unwrap_or_default();

        // `v2ray-http-upgrade` swaps the WebSocket handshake for V2Ray's leaner
        // HTTP-Upgrade one over the same (optionally TLS-secured) socket. The
        // `*-fast-open` payload-piggyback optimization is wire-compatible to
        // omit, so we accept the flag and simply send the request first.
        let stream = if opts.v2ray_http_upgrade == Some(true) {
            V2rayStream::HttpUpgrade(HttpUpgradeTransportConfig { path, host, headers })
        } else {
            V2rayStream::Websocket(WsTransportConfig { path, host, headers })
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
                ech: None,
            })
        } else {
            None
        };

        Ok(SsPlugin::V2ray { stream, tls, mux })
    }

    /// Dial the plugin transport to `server:port` and return a relay-ready byte
    /// stream onto which the Shadowsocks layer writes its salt + AEAD chunks.
    pub async fn connect(&self, server: &str, port: u16) -> Result<BoxedStream> {
        // QUIC dials its own UDP socket; every other transport rides a TCP one.
        if let SsPlugin::V2ray {
            stream: V2rayStream::Quic(cfg),
            ..
        } = self
        {
            return crate::transport::v2ray_quic::connect(cfg, server, port).await;
        }

        let tcp = TcpStream::connect((server, port))
            .await
            .with_context(|| format!("shadowsocks plugin: dial {server}:{port}"))?;

        match self {
            SsPlugin::ObfsHttp { host, path } => {
                let stream = simple_obfs::connect_http(tcp, host, path).await?;
                Ok(Box::new(stream))
            }
            SsPlugin::ObfsTls { host } => {
                let stream = simple_obfs::connect_tls(tcp, host).await?;
                Ok(Box::new(stream))
            }
            SsPlugin::V2ray { stream, tls, mux } => {
                let secured: BoxedStream = match tls {
                    None => Box::new(tcp),
                    Some(cfg) => Box::new(crate::transport::tls::connect(cfg, server, tcp).await?),
                };
                let framed: BoxedStream = match stream {
                    V2rayStream::Websocket(ws) => Box::new(crate::transport::ws::connect(secured, server, ws).await?),
                    V2rayStream::HttpUpgrade(hu) => {
                        Box::new(crate::transport::httpupgrade::connect(secured, server, hu).await?)
                    }
                    // Handled above; never dialed over TCP.
                    V2rayStream::Quic(_) => unreachable!("quic dialed before the TCP path"),
                };
                if *mux {
                    Ok(Box::new(V2rayMux::new(framed)))
                } else {
                    Ok(framed)
                }
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
    fn obfs_tls_is_parsed() {
        let opts = PluginOpts {
            mode: Some("tls".to_string()),
            host: Some("www.example.com".to_string()),
            ..Default::default()
        };
        let plugin = SsPlugin::parse(Some("obfs"), Some(&opts)).unwrap().unwrap();
        assert_eq!(
            plugin,
            SsPlugin::ObfsTls {
                host: "www.example.com".to_string(),
            }
        );
    }

    #[test]
    fn obfs_unknown_mode_is_rejected() {
        let opts = PluginOpts {
            mode: Some("quic".to_string()),
            ..Default::default()
        };
        let err = SsPlugin::parse(Some("obfs"), Some(&opts)).unwrap_err();
        assert!(err.to_string().contains("unknown simple-obfs mode"), "got: {err}");
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
            SsPlugin::V2ray {
                stream: V2rayStream::Websocket(ws),
                tls,
                mux,
            } => {
                assert_eq!(ws.path, "/v2");
                assert_eq!(ws.host.as_deref(), Some("cdn.example.com"));
                assert!(tls.is_none());
                assert!(!mux);
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
            SsPlugin::V2ray { tls, .. } => {
                let tls = tls.expect("tls enabled");
                assert_eq!(tls.server_name.as_deref(), Some("cdn.example.com"));
                assert!(tls.skip_cert_verify);
            }
            other => panic!("expected websocket, got {other:?}"),
        }
    }

    #[test]
    fn v2ray_mux_is_accepted() {
        let opts = PluginOpts {
            mux: Some(true),
            ..Default::default()
        };
        let plugin = SsPlugin::parse(Some("v2ray-plugin"), Some(&opts)).unwrap().unwrap();
        match plugin {
            SsPlugin::V2ray {
                stream: V2rayStream::Websocket(_),
                mux,
                ..
            } => assert!(mux),
            other => panic!("expected websocket+mux, got {other:?}"),
        }
    }

    #[test]
    fn v2ray_http_upgrade_is_parsed() {
        let opts = PluginOpts {
            host: Some("cdn.example.com".to_string()),
            path: Some("/up".to_string()),
            v2ray_http_upgrade: Some(true),
            ..Default::default()
        };
        let plugin = SsPlugin::parse(Some("v2ray-plugin"), Some(&opts)).unwrap().unwrap();
        match plugin {
            SsPlugin::V2ray {
                stream: V2rayStream::HttpUpgrade(hu),
                ..
            } => {
                assert_eq!(hu.path, "/up");
                assert_eq!(hu.host.as_deref(), Some("cdn.example.com"));
            }
            other => panic!("expected http-upgrade, got {other:?}"),
        }
    }

    #[test]
    fn v2ray_quic_is_parsed() {
        let opts = PluginOpts {
            mode: Some("quic".to_string()),
            host: Some("cdn.example.com".to_string()),
            skip_cert_verify: Some(true),
            ..Default::default()
        };
        let plugin = SsPlugin::parse(Some("v2ray-plugin"), Some(&opts)).unwrap().unwrap();
        match plugin {
            SsPlugin::V2ray {
                stream: V2rayStream::Quic(quic),
                tls,
                mux,
            } => {
                assert_eq!(quic.server_name, "cdn.example.com");
                assert_eq!(quic.alpn, vec!["h2".to_string(), "http/1.1".to_string()]);
                assert!(quic.skip_cert_verify);
                assert!(tls.is_none());
                assert!(!mux);
            }
            other => panic!("expected quic, got {other:?}"),
        }
    }

    #[test]
    fn v2ray_quic_with_mux_is_rejected() {
        let opts = PluginOpts {
            mode: Some("quic".to_string()),
            mux: Some(true),
            ..Default::default()
        };
        let err = SsPlugin::parse(Some("v2ray-plugin"), Some(&opts)).unwrap_err();
        assert!(err.to_string().contains("websocket-only"), "got: {err}");
    }

    #[test]
    fn v2ray_unknown_mode_is_rejected() {
        let opts = PluginOpts {
            mode: Some("grpc".to_string()),
            ..Default::default()
        };
        let err = SsPlugin::parse(Some("v2ray-plugin"), Some(&opts)).unwrap_err();
        assert!(err.to_string().contains("not supported"), "got: {err}");
    }

    #[test]
    fn unknown_plugin_is_rejected() {
        let err = SsPlugin::parse(Some("kcptun"), None).unwrap_err();
        assert!(err.to_string().contains("not supported"), "got: {err}");
    }
}
