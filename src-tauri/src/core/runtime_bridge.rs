use anyhow::{Result, anyhow};
use clash_dtos::{ConnectionId, CoreUpdaterChannel, LogLevel, ProxyDelay, TLSRotationResult};
use once_cell::sync::Lazy;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Mutex;
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::Duration;

use crate::core::{CoreManager, runtime_snapshot};

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

/// The control plane runs fully in-process (the kernel is compiled into the app
/// over `learn-gripe`); there is no external Mihomo controller socket to probe,
/// so the transport is reported as `in-process` rather than the former
/// `http`/`local-socket`/`auto` controller protocol.
pub fn read_runtime_controller_transport() -> &'static str {
    "in-process"
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
    // Refreshed in-process (download + validate + atomic replace + reload),
    // replacing the Mihomo controller `/providers/proxies/{name}` update call.
    let result = CoreManager::global().update_proxy_provider(provider_name).await;
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
    // Probed in-process by dialing each provider node, replacing the Mihomo
    // controller `/providers/proxies/{name}/healthcheck` call.
    let result = CoreManager::global().healthcheck_proxy_provider(provider_name).await;
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
    // Refreshed in-process, replacing the Mihomo controller
    // `/providers/rules/{name}` update call.
    let result = CoreManager::global().update_rule_provider(provider_name).await;
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
    let result = CoreManager::global().update_geo().await;
    record_runtime_bridge_result("update-runtime-geo", result.as_ref().map(|_| ()), None);
    result?;
    Ok(())
}

pub async fn upgrade_runtime_core(channel: CoreUpdaterChannel, force: bool) -> Result<()> {
    // In the pure-Rust runtime the kernel is compiled into the app (`learn-gripe`),
    // so there is no external Mihomo binary to download and hot-swap; kernel
    // upgrades ship through the application updater. This is an in-process no-op
    // that records the request for parity with the former Mihomo controller
    // `/upgrade` call instead of routing to a controller that no longer exists.
    let detail = Some(format!(
        "channel={channel:?};force={force};no external core in pure-Rust runtime"
    ));
    record_runtime_bridge_result::<anyhow::Error>("upgrade-runtime-core", Ok(()), detail);
    Ok(())
}

pub async fn upgrade_runtime_ui() -> Result<()> {
    // The dashboard UI is bundled with the app, so there is no external panel to
    // download; UI upgrades ship through the application updater. In-process
    // no-op recorded for parity with the former Mihomo controller `/upgrade/ui`
    // call.
    record_runtime_bridge_result::<anyhow::Error>(
        "upgrade-runtime-ui",
        Ok(()),
        Some("no external dashboard in pure-Rust runtime".into()),
    );
    Ok(())
}

pub async fn upgrade_runtime_geo() -> Result<()> {
    // `upgrade_geo` is semantically identical to `update_geo` (download +
    // reload the local geo databases); the in-process path serves both.
    let result = CoreManager::global().update_geo().await;
    record_runtime_bridge_result("upgrade-runtime-geo", result.as_ref().map(|_| ()), None);
    result?;
    Ok(())
}

pub async fn force_runtime_tls_rotation() -> Result<TLSRotationResult> {
    // Recorded in-process by the kernel's obfuscation counters; learn-gripe
    // re-rolls random fingerprints per dial and pins concrete ones to per-proxy
    // config, so this has no on-the-wire effect and only marks a rotation event
    // for telemetry parity. Replaces the Mihomo controller
    // `/engine/obfuscation/tls/rotate` call.
    let new_fingerprint = CoreManager::global().force_runtime_tls_rotation().await;
    record_runtime_bridge_result::<anyhow::Error>(
        "force-runtime-tls-rotation",
        Ok(()),
        Some(format!("fingerprint={new_fingerprint}")),
    );
    Ok(TLSRotationResult { new_fingerprint })
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

/// Read the kernel's in-process client-obfuscation stats (TLS ClientHello
/// fingerprint shaping), shaped like the former Mihomo
/// `/engine/obfuscation/stats` payload so the consumer parses it unchanged.
/// Reports empty stats when the kernel is not running. The kernel does no
/// payload padding and never re-keys a live session, so the byte/active/padding
/// counters that the external Go kernel exposed are reported as zero.
pub async fn read_runtime_obfuscation_stats() -> Result<Value> {
    let stats = CoreManager::global()
        .runtime_obfuscation_stats()
        .await
        .unwrap_or_default();
    Ok(serde_json::json!({
        "obfuscation": {
            "totalObfuscatedConns": stats.total_obfuscated_conns,
            "activeConns": 0,
            "totalWriteBytes": 0,
            "totalWriteCount": 0,
            "totalPaddingBytes": 0,
            "tlsRotationCount": stats.tls_rotation_count,
        },
        "tls": {
            "currentFingerprint": stats.current_tls_fingerprint,
        },
    }))
}

pub async fn reset_runtime_obfuscation_stats() -> Result<()> {
    CoreManager::global().reset_runtime_obfuscation_stats().await;
    record_runtime_bridge_result::<anyhow::Error>("reset-obfuscation-stats", Ok(()), None);
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn controller_transport_is_in_process() {
        // The pure-Rust runtime has no external Mihomo controller socket; the
        // probe must report the in-process surface instead of the former
        // http/local-socket/auto controller protocol.
        assert_eq!(read_runtime_controller_transport(), "in-process");
    }
}
