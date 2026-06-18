use crate::config::Config;
use crate::config::IVerge;
use crate::core::clash_mode::ClashMode;
use crate::core::{
    CoreManager, handle, manager::CLASH_LOGGER,
    stable_egress::sync_runtime_stable_egress_selection as core_sync_stable_egress, tray::Tray,
};
use bytes::BytesMut;
use clash_verge_logging::{Type, logging};
use compact_str::CompactString;
use once_cell::sync::Lazy;
use serde_yaml_ng::{Mapping, Value};
use smartstring::alias::String;
use std::env;
use std::sync::Arc;
use tauri::Emitter as _;
use tauri_plugin_clipboard_manager::ClipboardExt as _;
use tokio::fs;

#[allow(clippy::expect_used)]
static TLS_CONFIG: Lazy<Arc<rustls::ClientConfig>> = Lazy::new(|| {
    let root_store = rustls::RootCertStore::from_iter(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());
    let config = rustls::ClientConfig::builder_with_provider(Arc::new(rustls::crypto::ring::default_provider()))
        .with_safe_default_protocol_versions()
        .expect("Failed to set TLS versions")
        .with_root_certificates(root_store)
        .with_no_client_auth();
    Arc::new(config)
});

pub async fn change_clash_mode(mode: ClashMode) -> anyhow::Result<()> {
    let mode = mode.as_str();
    let mut mapping = Mapping::new();
    mapping.insert(Value::from("mode"), Value::from(mode));
    let json_value = serde_json::json!({ "mode": mode });

    logging!(debug, Type::Core, "change clash mode to {mode}");
    match handle::Handle::mihomo().await.patch_base_config(&json_value).await {
        Ok(_) => {
            let clash = Config::clash().await;
            clash.edit_draft(|d| d.patch_config(&mapping));
            clash.apply();

            let clash_data = clash.data_arc();
            clash_data.save_config().await?;
            handle::Handle::refresh_clash();
            crate::core::tray::Tray::global().update_menu_and_icon().await;

            crate::process::AsyncHandler::spawn(move || async {
                if let Err(err) = handle::Handle::mihomo().await.close_all_connections().await {
                    logging!(
                        error,
                        Type::Core,
                        "Failed to close connections after clash mode change: {err}"
                    );
                }
            });
            Ok(())
        }
        Err(err) => {
            logging!(error, Type::Core, "{err}");
            Err(anyhow::anyhow!("{err}"))
        }
    }
}

pub async fn toggle_system_proxy() -> bool {
    let verge = Config::verge().await;
    let current = verge.latest_arc().enable_system_proxy.unwrap_or(false);
    let requested = !current;

    match crate::app::config::patch_verge(
        &IVerge {
            enable_system_proxy: Some(requested),
            ..IVerge::default()
        },
        false,
    )
    .await
    {
        Ok(_) => {
            handle::Handle::refresh_verge();
            requested
        }
        Err(err) => {
            logging!(error, Type::ProxyMode, "{err}");
            current
        }
    }
}

pub async fn toggle_tun_mode(not_save_file: Option<bool>) -> bool {
    let current = Config::verge().await.latest_arc().enable_tun_mode.unwrap_or(false);
    let enable = !current;

    match crate::app::config::patch_verge(
        &IVerge {
            enable_tun_mode: Some(enable),
            ..IVerge::default()
        },
        not_save_file.unwrap_or(false),
    )
    .await
    {
        Ok(_) => {
            handle::Handle::refresh_verge();
            enable
        }
        Err(err) => {
            logging!(error, Type::ProxyMode, "{err}");
            current
        }
    }
}

pub async fn copy_clash_env() {
    let env_ip = env::var("CLASH_VERGE_OPTIMIZED_IP")
        .or_else(|_| env::var("CLASH_VERGE_REV_IP"))
        .ok();
    let verge_cfg = Config::verge().await.latest_arc();
    let ip = env_ip
        .as_deref()
        .unwrap_or_else(|| verge_cfg.proxy_host.as_deref().unwrap_or("127.0.0.1"));

    let app_handle = handle::Handle::app_handle();
    let port = verge_cfg.verge_mixed_port.unwrap_or(7897);
    let http_proxy = format!("http://{ip}:{port}");
    let socks5_proxy = format!("socks5://{ip}:{port}");

    let clipboard = app_handle.clipboard();

    let default_env = {
        #[cfg(not(target_os = "windows"))]
        {
            "bash"
        }
        #[cfg(target_os = "windows")]
        {
            "powershell"
        }
    };
    let env_type = verge_cfg.env_type.as_deref().unwrap_or(default_env);

    let export_text = match env_type {
        "bash" => format!("export https_proxy={http_proxy} http_proxy={http_proxy} all_proxy={socks5_proxy}"),
        "cmd" => format!("set http_proxy={http_proxy}\r\nset https_proxy={http_proxy}"),
        "powershell" => format!("$env:HTTP_PROXY=\"{http_proxy}\"; $env:HTTPS_PROXY=\"{http_proxy}\""),
        "nushell" => format!("load-env {{ http_proxy: \"{http_proxy}\", https_proxy: \"{http_proxy}\" }}"),
        "fish" => format!("set -x http_proxy {http_proxy}; set -x https_proxy {http_proxy}"),
        _ => {
            logging!(error, Type::ProxyMode, "copy_clash_env: Invalid env type! {env_type}");
            return;
        }
    };

    if clipboard.write_text(&export_text).is_err() {
        logging!(error, Type::ProxyMode, "Failed to write to clipboard");
    }
}

pub async fn restart_clash_core() {
    match CoreManager::global().restart_core().await {
        Ok(_) => {
            handle::Handle::refresh_clash();
            handle::Handle::notice_message("set_config::ok", "ok");
        }
        Err(err) => {
            handle::Handle::notice_message("set_config::error", format!("{err}"));
            logging!(error, Type::Core, "{err}");
        }
    }
}

pub async fn restart_app() {
    let _ = crate::app::window::prepare_shutdown().await;
    let app_handle = handle::Handle::app_handle();
    app_handle.restart();
}

pub async fn test_delay(url: String) -> anyhow::Result<u32> {
    use std::time::Duration;
    use tokio::io::{AsyncReadExt as _, AsyncWriteExt as _};
    use tokio::net::TcpStream;
    use tokio::time::Instant;

    let parsed = tauri::Url::parse(&url)?;
    let is_https = parsed.scheme() == "https";
    let host = parsed
        .host_str()
        .ok_or_else(|| anyhow::anyhow!("Invalid URL: no host"))?
        .to_string();
    let port = parsed.port().unwrap_or(if is_https { 443 } else { 80 });

    let verge = Config::verge().await.latest_arc();
    let proxy_enabled = verge.enable_system_proxy.unwrap_or(false) || verge.enable_tun_mode.unwrap_or(false);
    let proxy_port = if proxy_enabled {
        Some(match verge.verge_mixed_port {
            Some(p) => p,
            None => Config::clash().await.data_arc().get_mixed_port(),
        })
    } else {
        None
    };

    tokio::time::timeout(Duration::from_secs(10), async {
        let start = Instant::now();
        let mut buf = BytesMut::with_capacity(1024);

        if is_https {
            let stream = match proxy_port {
                Some(pp) => {
                    let mut s = TcpStream::connect(format!("127.0.0.1:{pp}")).await?;
                    s.write_all(format!("CONNECT {host}:{port} HTTP/1.1\r\nHost: {host}:{port}\r\n\r\n").as_bytes())
                        .await?;
                    s.read_buf(&mut buf).await?;
                    if !buf.windows(3).any(|w| w == b"200") {
                        return Err(anyhow::anyhow!("Proxy CONNECT failed"));
                    }
                    s
                }
                None => TcpStream::connect(format!("{host}:{port}")).await?,
            };
            let connector = tokio_rustls::TlsConnector::from(Arc::clone(&TLS_CONFIG));
            let server_name = rustls::pki_types::ServerName::try_from(host.as_str())
                .map_err(|_| anyhow::anyhow!("Invalid DNS name: {host}"))?
                .to_owned();
            connector.connect(server_name, stream).await?;
        } else {
            let (mut stream, req) = match proxy_port {
                Some(pp) => (
                    TcpStream::connect(format!("127.0.0.1:{pp}")).await?,
                    format!("HEAD {url} HTTP/1.1\r\nHost: {host}\r\nConnection: close\r\n\r\n"),
                ),
                None => (
                    TcpStream::connect(format!("{host}:{port}")).await?,
                    format!("HEAD / HTTP/1.1\r\nHost: {host}\r\nConnection: close\r\n\r\n"),
                ),
            };
            stream.write_all(req.as_bytes()).await?;
            let _ = stream.read(&mut buf).await?;
        }

        Ok((start.elapsed().as_millis() as u32).max(1))
    })
    .await
    .unwrap_or(Ok(10000u32))
}

pub async fn apply_dns_config(apply: bool) -> anyhow::Result<()> {
    if apply {
        let dns_path = crate::utils::dirs::app_home_dir()?.join(crate::constants::files::DNS_CONFIG);

        if !dns_path.exists() {
            logging!(warn, Type::Config, "DNS config file not found");
            anyhow::bail!("DNS config file not found");
        }

        let dns_yaml = fs::read_to_string(&dns_path).await.map_err(|e| {
            logging!(error, Type::Config, "Failed to read DNS config: {e}");
            e
        })?;

        let saved_config = serde_yaml_ng::from_str::<Mapping>(&dns_yaml).map_err(|e| {
            logging!(error, Type::Config, "Failed to parse DNS config: {e}");
            e
        })?;

        logging!(info, Type::Config, "Applying DNS config from file");

        let mut patch = Mapping::new();
        if let Some(dns) = saved_config.get("dns").cloned() {
            patch.insert("dns".into(), dns);
        }
        if let Some(hosts) = saved_config.get("hosts").cloned() {
            patch.insert("hosts".into(), hosts);
        }

        Config::runtime().await.edit_draft(|d| {
            d.patch_config(&patch);
        });

        CoreManager::global().update_config_checked().await.map_err(|err| {
            let err = format!("Failed to apply config with DNS: {err}");
            logging!(error, Type::Config, "{err}");
            anyhow::anyhow!("{err}")
        })?;

        logging!(info, Type::Config, "DNS config successfully applied");
    } else {
        logging!(info, Type::Config, "DNS settings disabled, regenerating config");

        CoreManager::global().update_config_checked().await.map_err(|err| {
            let err = format!("Failed to apply regenerated config: {err}");
            logging!(error, Type::Config, "{err}");
            anyhow::anyhow!("{err}")
        })?;

        logging!(info, Type::Config, "Config regenerated successfully");
    }

    handle::Handle::refresh_clash();
    Ok(())
}

pub async fn get_clash_logs() -> Vec<CompactString> {
    CoreManager::global().get_clash_logs().await.unwrap_or_default()
}

pub async fn clear_clash_logs() {
    CLASH_LOGGER.clear_logs().await;
}

pub async fn start_core() -> anyhow::Result<()> {
    CoreManager::global().start_core().await?;
    handle::Handle::refresh_clash();
    Ok(())
}

pub async fn stop_core() -> anyhow::Result<()> {
    clash_verge_logging::logging_error!(
        clash_verge_logging::Type::Core,
        Config::profiles().await.data_arc().save_file().await
    );
    CoreManager::global().stop_core().await?;
    handle::Handle::refresh_clash();
    Ok(())
}

pub async fn restart_core() -> anyhow::Result<()> {
    clash_verge_logging::logging_error!(
        clash_verge_logging::Type::Core,
        Config::profiles().await.data_arc().save_file().await
    );
    CoreManager::global().restart_core().await?;
    handle::Handle::refresh_clash();
    Ok(())
}

pub async fn sync_runtime_stable_egress_selection() -> anyhow::Result<()> {
    crate::core::coordinator::sync_coordinator_from_advanced_config_async().await?;

    let runtime_config = Config::runtime().await.latest_arc().config.clone();
    let Some(runtime_config) = runtime_config else {
        return Ok(());
    };

    let coordinator = crate::core::coordinator::get_coordinator();
    let session_affinity_manager = crate::core::session_affinity::get_session_affinity_manager();
    let ip_reputation_manager = crate::core::ip_reputation::get_ip_reputation_manager();

    core_sync_stable_egress(
        &coordinator,
        &session_affinity_manager,
        &ip_reputation_manager,
        &runtime_config,
    )
    .await
}

pub async fn switch_proxy_node(group_name: &str, proxy_name: &str) {
    for attempt in 1..=2 {
        match handle::Handle::mihomo()
            .await
            .select_node_for_group(group_name, proxy_name)
            .await
        {
            Ok(_) => {
                crate::core::runtime_snapshot::record_and_persist_runtime_proxy_selection(group_name, proxy_name);
                logging!(
                    info,
                    Type::Tray,
                    "Switched proxy node on attempt {}: {} -> {}",
                    attempt,
                    group_name,
                    proxy_name
                );

                if let Err(error) = sync_runtime_stable_egress_selection().await {
                    logging!(
                        warn,
                        Type::Tray,
                        "Failed to sync stable egress selection after switching {} -> {}: {}",
                        group_name,
                        proxy_name,
                        error
                    );
                }

                let _ = handle::Handle::app_handle().emit("verge://refresh-proxy-config", ());
                let _ = Tray::global().update_menu().await;
                return;
            }
            Err(err) => {
                logging!(
                    error,
                    Type::Tray,
                    "Failed to switch proxy node on attempt {}: {} -> {}, error: {:?}",
                    attempt,
                    group_name,
                    proxy_name,
                    err
                );
            }
        }
    }
}
