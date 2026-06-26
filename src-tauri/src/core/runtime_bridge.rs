use anyhow::{Result, anyhow};
use once_cell::sync::Lazy;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Mutex;
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::Duration;
use tauri_plugin_mihomo::models::{
    ConnectionId, CoreUpdaterChannel, LogLevel, Protocol, ProxyDelay, TLSRotationResult,
};

use crate::core::{CoreManager, handle::Handle, runtime_snapshot};

/// Cadence at which the in-process live-connections stream re-pushes a snapshot
/// to refresh live byte counts between structural changes, matching the former
/// Mihomo `/connections` WebSocket tick.
const CONNECTION_STREAM_TICK: Duration = Duration::from_secs(1);

/// Active in-process push streams (live connections and core logs), keyed by
/// the id handed back to the consumer so [`disconnect_runtime_stream`] can stop
/// the right one.
static STREAMS: Lazy<Mutex<HashMap<ConnectionId, tokio::task::JoinHandle<()>>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));
static NEXT_STREAM_ID: AtomicU32 = AtomicU32::new(1);

pub async fn read_runtime_controller_transport() -> Protocol {
    Handle::mihomo().await.protocol.clone()
}

pub async fn measure_runtime_proxy_delay(
    proxy_name: &str,
    test_url: &str,
    timeout: u32,
    group_name: Option<&str>,
) -> Result<ProxyDelay> {
    // Measured in-process by dialing the node's own outbound and timing the
    // probe, replacing the Mihomo controller `/proxies/{name}/delay` call.
    let result = CoreManager::global()
        .measure_runtime_proxy_delay(proxy_name, test_url, timeout)
        .await;
    let detail = Some(format!("proxy={proxy_name};url={test_url};timeout={timeout}"));
    record_runtime_bridge_result("measure-runtime-proxy-delay", result.as_ref().map(|_| ()), detail);
    // A failed probe (timeout / refused) is the UI's `delay == 0` timeout
    // sentinel, not a hard error — the same shape the former Mihomo call had.
    let delay = result.unwrap_or(0);
    if let Some(group_name) = group_name.filter(|value| !value.is_empty()) {
        runtime_snapshot::record_and_persist_runtime_proxy_delay(group_name, proxy_name, delay, test_url);
    }
    Ok(ProxyDelay { delay })
}

pub async fn measure_runtime_group_delay(
    group_name: &str,
    test_url: &str,
    timeout: u32,
) -> Result<HashMap<String, u32>> {
    // Measured in-process by probing every member outbound concurrently,
    // replacing the Mihomo controller `/group/{name}/delay` call.
    let result = CoreManager::global()
        .measure_runtime_group_delay(group_name, test_url, timeout)
        .await;
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

pub async fn update_runtime_rule_provider(provider_name: &str) -> Result<()> {
    let result = Handle::mihomo().await.update_rule_provider(provider_name).await;
    record_runtime_bridge_result(
        "update-runtime-rule-provider",
        result.as_ref().map(|_| ()),
        Some(format!("provider={provider_name}")),
    );
    runtime_snapshot::record_and_persist_runtime_provider_health(
        &format!("rule:{provider_name}"),
        result.is_ok(),
        result.as_ref().err().map(ToString::to_string),
    );
    result?;
    Ok(())
}

pub async fn close_runtime_connection(connection_id: &str) -> Result<()> {
    let result = match connection_id.parse::<u64>() {
        Ok(id) => {
            CoreManager::global().close_runtime_connection(id).await;
            Ok(())
        }
        Err(_) => Err(anyhow!("invalid connection id: {connection_id}")),
    };
    record_runtime_bridge_result(
        "close-runtime-connection",
        result.as_ref().map(|_| ()),
        Some(format!("connection_id={connection_id}")),
    );
    result
}

pub async fn close_all_runtime_connections(reason: &str) -> Result<()> {
    let count = CoreManager::global().close_all_runtime_connections().await;
    record_runtime_bridge_result::<anyhow::Error>(
        "close-all-runtime-connections",
        Ok(()),
        Some(format!("reason={reason};count={count}")),
    );
    Ok(())
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
    F: Fn(Value) + Send + Sync + 'static,
{
    // The live connection stream is now served in-process from the kernel's
    // connection table; there is no external controller WebSocket. Require a
    // running kernel (the connection monitor retries on error), then push a
    // snapshot on every table change plus on a fixed interval so live byte
    // counts refresh between structural changes.
    let mut changes = CoreManager::global()
        .watch_runtime_connections()
        .await
        .ok_or_else(|| anyhow!("kernel not running"))?;

    let connection_id = NEXT_STREAM_ID.fetch_add(1, Ordering::Relaxed);

    let task = tokio::spawn(async move {
        // Emit an initial snapshot so the consumer doesn't wait a full tick.
        emit_connection_snapshot(&on_message).await;
        loop {
            tokio::select! {
                _ = tokio::time::sleep(CONNECTION_STREAM_TICK) => {
                    emit_connection_snapshot(&on_message).await;
                }
                changed = changes.changed() => {
                    // The watch sender is dropped when the kernel stops; end the
                    // stream so the consumer reconnects against the next kernel.
                    if changed.is_err() {
                        break;
                    }
                    emit_connection_snapshot(&on_message).await;
                }
            }
        }
    });

    STREAMS.lock().unwrap().insert(connection_id, task);
    record_runtime_bridge_result::<anyhow::Error>(
        "connect-connections-stream",
        Ok(()),
        Some(format!("connection_id={connection_id}")),
    );
    Ok(connection_id)
}

/// Snapshot the kernel's connection table and hand the consumer a
/// Mihomo-compatible `Connections` JSON value. A stopped kernel yields an empty
/// table (the same shape the consumer saw with no controller).
async fn emit_connection_snapshot<F: Fn(Value) + Sync>(on_message: &F) {
    let table = CoreManager::global().runtime_connections().await.unwrap_or_default();
    let connections = runtime_snapshot::connections_from_kernel(table);
    if let Ok(value) = serde_json::to_value(&connections) {
        on_message(value);
    }
}

pub async fn connect_runtime_log_stream<F>(level: LogLevel, on_message: F) -> Result<ConnectionId>
where
    F: Fn(Value) + Send + 'static,
{
    // Core logs are produced in-process by the kernel via the `log` facade and
    // captured by the app's logger; there is no external controller WebSocket.
    // Subscribe to the in-process core-log broadcast and forward records that
    // pass the requested level threshold as Mihomo-compatible JSON.
    let mut receiver = crate::core::log_stream::subscribe();
    let connection_id = NEXT_STREAM_ID.fetch_add(1, Ordering::Relaxed);

    let task = tokio::spawn(async move {
        loop {
            match receiver.recv().await {
                Ok(record) => {
                    if crate::core::log_stream::level_passes(level, record.level) {
                        on_message(crate::core::log_stream::to_frontend_value(&record));
                    }
                }
                // A lagging consumer drops the oldest records; keep streaming.
                Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => {}
                // The broadcast sender is a process-lifetime static, so this is
                // only reached if the channel is ever torn down; end the stream.
                Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
            }
        }
    });

    STREAMS.lock().unwrap().insert(connection_id, task);
    record_runtime_bridge_result::<anyhow::Error>(
        "connect-log-stream",
        Ok(()),
        Some(format!("level={level};connection_id={connection_id}")),
    );
    Ok(connection_id)
}

pub async fn disconnect_runtime_stream(connection_id: ConnectionId, _close_code: Option<u64>) {
    // Both the live-connections stream and the core-log stream are served
    // in-process now; "disconnect" stops the matching push task. An unknown id
    // (e.g. an already-finished stream) is a no-op.
    let task = STREAMS.lock().unwrap().remove(&connection_id);
    if let Some(task) = task {
        task.abort();
    }
    record_runtime_bridge_result::<anyhow::Error>(
        "disconnect-runtime-stream",
        Ok(()),
        Some(format!("connection_id={connection_id}")),
    );
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
