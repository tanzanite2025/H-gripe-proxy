use anyhow::{Result, anyhow};
use serde_json::Value;
use std::collections::HashMap;
use tauri_plugin_mihomo::models::{
    ConnectionId, CoreUpdaterChannel, LogLevel, Protocol, ProxyDelay, TLSRotationResult,
};

use crate::core::{handle::Handle, runtime_snapshot};

pub async fn read_runtime_controller_transport() -> Protocol {
    Handle::mihomo().await.protocol.clone()
}

pub async fn measure_runtime_proxy_delay(
    proxy_name: &str,
    test_url: &str,
    timeout: u32,
    group_name: Option<&str>,
) -> Result<ProxyDelay> {
    let result = Handle::mihomo()
        .await
        .delay_proxy_by_name(proxy_name, test_url, timeout)
        .await;
    let detail = Some(format!("proxy={proxy_name};url={test_url};timeout={timeout}"));
    record_runtime_bridge_result("measure-runtime-proxy-delay", result.as_ref().map(|_| ()), detail);
    let result = result?;
    if let Some(group_name) = group_name.filter(|value| !value.is_empty()) {
        runtime_snapshot::record_and_persist_runtime_proxy_delay(group_name, proxy_name, result.delay, test_url);
    }
    Ok(result)
}

pub async fn measure_runtime_group_delay(
    group_name: &str,
    test_url: &str,
    timeout: u32,
) -> Result<HashMap<String, u32>> {
    let result = Handle::mihomo().await.delay_group(group_name, test_url, timeout).await;
    record_runtime_bridge_result(
        "measure-runtime-group-delay",
        result.as_ref().map(|_| ()),
        Some(format!("group={group_name};url={test_url};timeout={timeout}")),
    );
    let result = result?;
    for (proxy_name, delay) in &result {
        runtime_snapshot::record_and_persist_runtime_proxy_delay(group_name, proxy_name, *delay, test_url);
    }
    Ok(result)
}

pub async fn update_runtime_proxy_provider(provider_name: &str) -> Result<()> {
    let result = Handle::mihomo().await.update_proxy_provider(provider_name).await;
    record_runtime_bridge_result(
        "update-runtime-proxy-provider",
        result.as_ref().map(|_| ()),
        Some(format!("provider={provider_name}")),
    );
    runtime_snapshot::record_and_persist_runtime_provider_health(
        provider_name,
        result.is_ok(),
        result.as_ref().err().map(ToString::to_string),
    );
    result?;
    Ok(())
}

pub async fn healthcheck_runtime_proxy_provider(provider_name: &str) -> Result<()> {
    let result = Handle::mihomo().await.healthcheck_proxy_provider(provider_name).await;
    record_runtime_bridge_result(
        "healthcheck-runtime-proxy-provider",
        result.as_ref().map(|_| ()),
        Some(format!("provider={provider_name}")),
    );
    runtime_snapshot::record_and_persist_runtime_provider_health(
        provider_name,
        result.is_ok(),
        result.as_ref().err().map(ToString::to_string),
    );
    result?;
    Ok(())
}

pub async fn close_runtime_connection(connection_id: &str) -> Result<()> {
    let result = Handle::mihomo().await.close_connection(connection_id).await;
    record_runtime_bridge_result(
        "close-runtime-connection",
        result.as_ref().map(|_| ()),
        Some(format!("connection_id={connection_id}")),
    );
    result?;
    Ok(())
}

pub async fn close_all_runtime_connections(reason: &str) -> Result<()> {
    let connections = runtime_snapshot::read_runtime_connections().await?;
    let Some(connections) = connections.connections else {
        record_runtime_bridge_result::<anyhow::Error>(
            "close-all-runtime-connections",
            Ok(()),
            Some(format!("reason={reason};count=0")),
        );
        return Ok(());
    };

    let mut errors = Vec::new();
    let count = connections.len();
    for connection in connections {
        if let Err(error) = Handle::mihomo().await.close_connection(&connection.id).await {
            errors.push(format!("{}:{error}", connection.id));
        }
    }

    if errors.is_empty() {
        record_runtime_bridge_result::<anyhow::Error>(
            "close-all-runtime-connections",
            Ok(()),
            Some(format!("reason={reason};count={count}")),
        );
        Ok(())
    } else {
        let error = anyhow!(
            "failed to close {} of {count} runtime connections: {}",
            errors.len(),
            errors.join("; ")
        );
        record_runtime_bridge_result(
            "close-all-runtime-connections",
            Err(&error),
            Some(format!("reason={reason};count={count}")),
        );
        Err(error)
    }
}

pub async fn update_runtime_geo() -> Result<()> {
    let result = Handle::mihomo().await.update_geo().await;
    record_runtime_bridge_result("update-runtime-geo", result.as_ref().map(|_| ()), None);
    result?;
    Ok(())
}

pub async fn upgrade_runtime_core(channel: CoreUpdaterChannel, force: bool) -> Result<()> {
    let detail = Some(format!("channel={channel:?};force={force}"));
    let result = Handle::mihomo().await.upgrade_core(channel, force).await;
    record_runtime_bridge_result("upgrade-runtime-core", result.as_ref().map(|_| ()), detail);
    result?;
    Ok(())
}

pub async fn upgrade_runtime_ui() -> Result<()> {
    let result = Handle::mihomo().await.upgrade_ui().await;
    record_runtime_bridge_result("upgrade-runtime-ui", result.as_ref().map(|_| ()), None);
    result?;
    Ok(())
}

pub async fn upgrade_runtime_geo() -> Result<()> {
    let result = Handle::mihomo().await.upgrade_geo().await;
    record_runtime_bridge_result("upgrade-runtime-geo", result.as_ref().map(|_| ()), None);
    result?;
    Ok(())
}

pub async fn force_runtime_tls_rotation() -> Result<TLSRotationResult> {
    let result = Handle::mihomo().await.force_tls_rotation().await;
    record_runtime_bridge_result("force-runtime-tls-rotation", result.as_ref().map(|_| ()), None);
    Ok(result?)
}

pub async fn connect_runtime_connections_stream<F>(on_message: F) -> Result<ConnectionId>
where
    F: Fn(Value) + Send + 'static,
{
    let result = Handle::mihomo().await.ws_connections(on_message).await;
    record_runtime_bridge_result("connect-connections-stream", result.as_ref().map(|_| ()), None);
    let connection_id = result?;
    Ok(connection_id)
}

pub async fn connect_runtime_log_stream<F>(level: LogLevel, on_message: F) -> Result<ConnectionId>
where
    F: Fn(Value) + Send + 'static,
{
    let result = Handle::mihomo().await.ws_logs(level, on_message).await;
    record_runtime_bridge_result(
        "connect-log-stream",
        result.as_ref().map(|_| ()),
        Some(format!("level={level}")),
    );
    let connection_id = result?;
    Ok(connection_id)
}

pub async fn disconnect_runtime_stream(connection_id: ConnectionId, close_code: Option<u64>) {
    let result = Handle::mihomo().await.disconnect(connection_id, close_code).await;
    record_runtime_bridge_result(
        "disconnect-runtime-stream",
        result.as_ref().map(|_| ()),
        Some(format!("connection_id={connection_id}")),
    );
    if let Err(error) = result {
        log::debug!("failed to disconnect runtime stream {connection_id}: {error}");
    }
}

pub async fn read_runtime_obfuscation_stats() -> Result<Value> {
    let stats = Handle::mihomo().await.get_obfuscation_stats().await?;
    Ok(stats)
}

pub async fn reset_runtime_obfuscation_stats() -> Result<()> {
    let result = Handle::mihomo().await.reset_obfuscation_stats().await;
    record_runtime_bridge_result("reset-obfuscation-stats", result.as_ref().map(|_| ()), None);
    result?;
    Ok(())
}

fn record_runtime_bridge_result<E: std::fmt::Display>(
    kind: &str,
    result: std::result::Result<(), &E>,
    detail: Option<String>,
) {
    runtime_snapshot::record_and_persist_runtime_lifecycle_event(
        kind,
        result.is_ok(),
        result.err().map(ToString::to_string),
        detail,
    );
}
