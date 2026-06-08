use crate::{
    config::Config,
    core::{CoreManager, clash_mode::ClashMode, handle, manager::CLASH_LOGGER, tray},
    feat::clean_async,
    process::AsyncHandler,
    utils::{self, dirs},
};
use bytes::BytesMut;
use clash_verge_logging::{Type, logging, logging_error};
use compact_str::CompactString;
use once_cell::sync::Lazy;
use serde_yaml_ng::{Mapping, Value};
use smartstring::alias::String;
use std::sync::Arc;
use std::time::Duration;
use tauri_plugin_mihomo::Error as MihomoError;
use tokio::fs;
use tokio::sync::Mutex;

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

static MIHOMO_RECOVERY_LOCK: Lazy<Mutex<()>> = Lazy::new(|| Mutex::new(()));

async fn probe_mihomo_ipc() -> Result<(), MihomoError> {
    handle::Handle::mihomo().await.get_version().await.map(|_| ())
}

pub async fn ensure_mihomo_core_ready() -> anyhow::Result<()> {
    let _guard = MIHOMO_RECOVERY_LOCK.lock().await;

    handle::Handle::sync_mihomo_controller_state().await?;

    match probe_mihomo_ipc().await {
        Ok(()) => return Ok(()),
        Err(err) if !CoreManager::is_mihomo_ipc_unavailable(&err) => {
            return Err(anyhow::anyhow!("Mihomo IPC probe failed: {err}"));
        }
        Err(err) => {
            let running_mode = CoreManager::global().get_running_mode();
            logging!(
                warn,
                Type::Core,
                "Mihomo IPC is unavailable while checking readiness (mode: {}). Attempting recovery: {}",
                running_mode,
                err
            );

            match &*running_mode {
                crate::core::manager::RunningMode::NotRunning => {
                    CoreManager::global().start_core().await?;
                }
                crate::core::manager::RunningMode::Sidecar | crate::core::manager::RunningMode::Service => {
                    CoreManager::global().restart_core().await?;
                }
            }
        }
    }

    tokio::time::sleep(Duration::from_millis(250)).await;
    handle::Handle::sync_mihomo_controller_state().await?;
    probe_mihomo_ipc()
        .await
        .map_err(|err| anyhow::anyhow!("Mihomo IPC is still unavailable after recovery: {err}"))?;
    handle::Handle::refresh_clash();

    Ok(())
}

/// Restart the Clash core
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

/// Restart the application
pub async fn restart_app() {
    logging!(debug, Type::System, "启动重启应用流程");
    // 设置退出标志
    handle::Handle::global().set_is_exiting();

    utils::server::shutdown_embedded_server();
    Config::apply_all_and_save_file().await;

    logging!(info, Type::System, "开始异步清理资源");
    let cleanup_result = clean_async().await;

    logging!(
        info,
        Type::System,
        "资源清理完成，退出代码: {}",
        if cleanup_result { 0 } else { 1 }
    );

    let app_handle = handle::Handle::app_handle();
    app_handle.restart();
}

fn after_change_clash_mode() {
    AsyncHandler::spawn(move || async {
        if let Err(err) = handle::Handle::mihomo().await.close_all_connections().await {
            logging!(
                error,
                Type::Core,
                "Failed to close connections after clash mode change: {err}"
            );
        }
    });
}

/// Change Clash mode (rule/global/direct)
pub async fn change_clash_mode(mode: ClashMode) -> anyhow::Result<()> {
    let mode = mode.as_str();
    let mut mapping = Mapping::new();
    mapping.insert(Value::from("mode"), Value::from(mode));
    // Convert YAML mapping to JSON Value
    let json_value = serde_json::json!({
        "mode": mode
    });
    logging!(debug, Type::Core, "change clash mode to {mode}");
    match handle::Handle::mihomo().await.patch_base_config(&json_value).await {
        Ok(_) => {
            // 更新订阅
            let clash = Config::clash().await;
            clash.edit_draft(|d| d.patch_config(&mapping));
            clash.apply();

            // 分离数据获取和异步调用
            let clash_data = clash.data_arc();
            clash_data.save_config().await?;
            handle::Handle::refresh_clash();
            tray::Tray::global().update_menu_and_icon().await;

            after_change_clash_mode();
            Ok(())
        }
        Err(err) => {
            logging!(error, Type::Core, "{err}");
            Err(anyhow::anyhow!("{err}"))
        }
    }
}

/// Test delay to a URL through proxy.
/// HTTPS: measures TLS handshake time. HTTP: measures HEAD round-trip time.
pub async fn test_delay(url: String) -> anyhow::Result<u32> {
    use std::sync::Arc;
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

        // frontend treats 0 as timeout
        Ok((start.elapsed().as_millis() as u32).max(1))
    })
    .await
    .unwrap_or(Ok(10000u32))
}

/// 保存 DNS 配置映射到文件
pub async fn save_dns_config_mapping(dns_config: &Mapping) -> anyhow::Result<()> {
    let dns_path = dirs::app_home_dir()?.join(crate::constants::files::DNS_CONFIG);
    let yaml_str = serde_yaml_ng::to_string(dns_config)?;
    fs::write(&dns_path, yaml_str).await?;
    logging!(info, Type::Config, "DNS config saved to {dns_path:?}");
    Ok(())
}

fn build_dns_runtime_patch(saved_config: Mapping) -> Mapping {
    let mut patch = Mapping::new();

    if let Some(dns) = saved_config.get("dns").cloned() {
        patch.insert("dns".into(), dns);
    }

    if let Some(hosts) = saved_config.get("hosts").cloned() {
        patch.insert("hosts".into(), hosts);
    }

    patch
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dns_runtime_patch_uses_root_mapping_without_nested_dns() {
        let mut dns = Mapping::new();
        dns.insert("enable".into(), true.into());
        let mut saved = Mapping::new();
        saved.insert("dns".into(), dns.into());
        saved.insert("hosts".into(), Mapping::new().into());

        let patch = build_dns_runtime_patch(saved);
        let dns_mapping = patch.get("dns").and_then(|value| value.as_mapping()).unwrap();

        assert!(dns_mapping.get("enable").is_some());
        assert!(dns_mapping.get("dns").is_none());
        assert!(patch.get("hosts").is_some());
    }
}

/// 启动核心
pub async fn start_core() -> anyhow::Result<()> {
    CoreManager::global().start_core().await?;
    handle::Handle::refresh_clash();
    Ok(())
}

/// 关闭核心
pub async fn stop_core() -> anyhow::Result<()> {
    logging_error!(Type::Core, Config::profiles().await.data_arc().save_file().await);
    CoreManager::global().stop_core().await?;
    handle::Handle::refresh_clash();
    Ok(())
}

/// 重启核心
pub async fn restart_core() -> anyhow::Result<()> {
    logging_error!(Type::Core, Config::profiles().await.data_arc().save_file().await);
    CoreManager::global().restart_core().await?;
    handle::Handle::refresh_clash();
    Ok(())
}

/// 应用或撤销 DNS 配置
pub async fn apply_dns_config(apply: bool) -> anyhow::Result<()> {
    if apply {
        let dns_path = dirs::app_home_dir()?.join(crate::constants::files::DNS_CONFIG);

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

        let patch = build_dns_runtime_patch(saved_config);

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

/// 获取 Clash 日志
pub async fn get_clash_logs() -> Vec<CompactString> {
    CoreManager::global().get_clash_logs().await.unwrap_or_default()
}

/// 清除 Clash 日志缓存
pub async fn clear_clash_logs() {
    CLASH_LOGGER.clear_logs().await;
}
