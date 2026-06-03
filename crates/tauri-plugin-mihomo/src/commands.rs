use std::collections::HashMap;

use tauri::{State, async_runtime::RwLock, command, ipc::Channel};

use crate::{Result, mihomo::Mihomo, models::*};

#[command]
pub(crate) async fn update_controller(
    state: State<'_, RwLock<Mihomo>>,
    host: Option<String>,
    port: Option<u16>,
) -> Result<()> {
    let mut mihomo = state.write().await;
    mihomo.update_external_host(host);
    mihomo.update_external_port(port);
    drop(mihomo);
    Ok(())
}

#[command]
pub(crate) async fn update_secret(state: State<'_, RwLock<Mihomo>>, secret: Option<String>) -> Result<()> {
    state.write().await.update_secret(secret);
    Ok(())
}

#[command]
pub(crate) async fn get_version(state: State<'_, RwLock<Mihomo>>) -> Result<MihomoVersion> {
    state.read().await.get_version().await
}

#[command]
pub(crate) async fn flush_fakeip(state: State<'_, RwLock<Mihomo>>) -> Result<()> {
    state.read().await.flush_fakeip().await
}

#[command]
pub(crate) async fn flush_dns(state: State<'_, RwLock<Mihomo>>) -> Result<()> {
    state.read().await.flush_dns().await
}

#[command]
pub(crate) async fn get_dns_metrics(state: State<'_, RwLock<Mihomo>>) -> Result<crate::models::DnsMetrics> {
    state.read().await.get_dns_metrics().await
}

#[command]
pub(crate) async fn dns_warmup(state: State<'_, RwLock<Mihomo>>) -> Result<()> {
    state.read().await.dns_warmup().await
}

// connections
#[command]
pub(crate) async fn get_connections(state: State<'_, RwLock<Mihomo>>) -> Result<Connections> {
    state.read().await.get_connections().await
}

#[command]
pub(crate) async fn close_all_connections(state: State<'_, RwLock<Mihomo>>) -> Result<()> {
    state.read().await.close_all_connections().await
}

#[command]
pub(crate) async fn close_connection(state: State<'_, RwLock<Mihomo>>, connection_id: String) -> Result<()> {
    state.read().await.close_connection(&connection_id).await
}

// groups
#[command]
pub(crate) async fn get_groups(state: State<'_, RwLock<Mihomo>>) -> Result<Groups> {
    state.read().await.get_groups().await
}

#[command]
pub(crate) async fn get_group_by_name(state: State<'_, RwLock<Mihomo>>, group_name: String) -> Result<Proxy> {
    state.read().await.get_group_by_name(&group_name).await
}

#[command]
pub(crate) async fn delay_group(
    state: State<'_, RwLock<Mihomo>>,
    group_name: String,
    test_url: String,
    timeout: u32,
    keep_fixed: bool,
) -> Result<HashMap<String, u32>> {
    let fixed = if keep_fixed {
        state.read().await.get_group_by_name(&group_name).await?.fixed
    } else {
        None
    };
    log::debug!("delay group, fixed: {fixed:?}");
    let res = state.read().await.delay_group(&group_name, &test_url, timeout).await?;
    if keep_fixed
        && let Some(fixed) = fixed
        && !fixed.is_empty()
    {
        state.read().await.select_node_for_group(&group_name, &fixed).await?;
    }
    Ok(res)
}

// providers
#[command]
pub(crate) async fn get_proxy_providers(state: State<'_, RwLock<Mihomo>>) -> Result<ProxyProviders> {
    state.read().await.get_proxy_providers().await
}

#[command]
pub(crate) async fn get_proxy_provider_by_name(
    state: State<'_, RwLock<Mihomo>>,
    provider_name: String,
) -> Result<ProxyProvider> {
    state.read().await.get_proxy_provider_by_name(&provider_name).await
}

#[command]
pub(crate) async fn update_proxy_provider(state: State<'_, RwLock<Mihomo>>, provider_name: String) -> Result<()> {
    state.read().await.update_proxy_provider(&provider_name).await
}

#[command]
pub(crate) async fn healthcheck_proxy_provider(state: State<'_, RwLock<Mihomo>>, provider_name: String) -> Result<()> {
    state.read().await.healthcheck_proxy_provider(&provider_name).await
}

#[command]
pub(crate) async fn healthcheck_node_in_provider(
    state: State<'_, RwLock<Mihomo>>,
    provider_name: String,
    proxy_name: String,
    test_url: String,
    timeout: u32,
) -> Result<ProxyDelay> {
    state
        .read()
        .await
        .healthcheck_node_in_provider(&provider_name, &proxy_name, &test_url, timeout)
        .await
}

// proxies
#[command]
pub(crate) async fn get_proxies(state: State<'_, RwLock<Mihomo>>) -> Result<Proxies> {
    state.read().await.get_proxies().await
}

#[command]
pub(crate) async fn get_proxy_by_name(state: State<'_, RwLock<Mihomo>>, proxy_name: String) -> Result<Proxy> {
    state.read().await.get_proxy_by_name(&proxy_name).await
}

#[command]
pub(crate) async fn select_node_for_group(
    state: State<'_, RwLock<Mihomo>>,
    group_name: String,
    node: String,
) -> Result<()> {
    state.read().await.select_node_for_group(&group_name, &node).await
}

#[command]
pub(crate) async fn unfixed_proxy(state: State<'_, RwLock<Mihomo>>, group_name: String) -> Result<()> {
    state.read().await.unfixed_proxy(&group_name).await
}

#[command]
pub(crate) async fn delay_proxy_by_name(
    state: State<'_, RwLock<Mihomo>>,
    proxy_name: String,
    test_url: String,
    timeout: u32,
) -> Result<ProxyDelay> {
    state
        .read()
        .await
        .delay_proxy_by_name(&proxy_name, &test_url, timeout)
        .await
}

// rules
#[command]
pub(crate) async fn get_rules(state: State<'_, RwLock<Mihomo>>) -> Result<Rules> {
    state.read().await.get_rules().await
}

#[command]
pub(crate) async fn disable_rules(state: State<'_, RwLock<Mihomo>>, payload: HashMap<i32, bool>) -> Result<()> {
    state.read().await.disable_rules(&payload).await
}

#[command]
pub(crate) async fn delete_rule(state: State<'_, RwLock<Mihomo>>, index: i32) -> Result<()> {
    state.read().await.delete_rule(index).await
}

#[command]
pub(crate) async fn create_rule(
    state: State<'_, RwLock<Mihomo>>,
    rule_type: String,
    payload: String,
    proxy: String,
    source: Option<String>,
    sub_rule: Option<String>,
    position: Option<String>,
) -> Result<i32> {
    state
        .read()
        .await
        .create_rule(
            &rule_type,
            &payload,
            &proxy,
            source.as_deref(),
            sub_rule.as_deref(),
            position.as_deref(),
        )
        .await
}

#[command]
pub(crate) async fn get_sub_rules(state: State<'_, RwLock<Mihomo>>) -> Result<serde_json::Value> {
    state.read().await.get_sub_rules().await
}

#[command]
pub(crate) async fn delete_sub_rule_by_source(
    state: State<'_, RwLock<Mihomo>>,
    name: String,
    source_prefix: Option<String>,
) -> Result<i32> {
    state
        .read()
        .await
        .delete_sub_rule_by_source(&name, source_prefix.as_deref())
        .await
}

#[command]
pub(crate) async fn get_rule_providers(state: State<'_, RwLock<Mihomo>>) -> Result<RuleProviders> {
    state.read().await.get_rule_providers().await
}

#[command]
pub(crate) async fn update_rule_provider(state: State<'_, RwLock<Mihomo>>, provider_name: String) -> Result<()> {
    state.read().await.update_rule_provider(&provider_name).await
}

// runtime config
#[command]
pub(crate) async fn get_base_config(state: State<'_, RwLock<Mihomo>>) -> Result<BaseConfig> {
    state.read().await.get_base_config().await
}

#[command]
pub(crate) async fn reload_config(state: State<'_, RwLock<Mihomo>>, force: bool, config_path: String) -> Result<()> {
    state.read().await.reload_config(force, &config_path).await
}

#[command]
pub(crate) async fn patch_base_config(state: State<'_, RwLock<Mihomo>>, data: serde_json::Value) -> Result<()> {
    state.read().await.patch_base_config(&data).await
}

#[command]
pub(crate) async fn update_geo(state: State<'_, RwLock<Mihomo>>) -> Result<()> {
    state.read().await.update_geo().await
}

#[command]
pub(crate) async fn restart(state: State<'_, RwLock<Mihomo>>) -> Result<()> {
    state.read().await.restart().await
}

// upgrade
#[command]
pub(crate) async fn upgrade_core(
    state: State<'_, RwLock<Mihomo>>,
    channel: CoreUpdaterChannel,
    force: bool,
) -> Result<()> {
    state.read().await.upgrade_core(channel, force).await
}

#[command]
pub(crate) async fn upgrade_ui(state: State<'_, RwLock<Mihomo>>) -> Result<()> {
    state.read().await.upgrade_ui().await
}

#[command]
pub(crate) async fn upgrade_geo(state: State<'_, RwLock<Mihomo>>) -> Result<()> {
    state.read().await.upgrade_geo().await
}

// mihomo websocket
#[command]
pub(crate) async fn ws_traffic(
    state: State<'_, RwLock<Mihomo>>,
    on_message: Channel<serde_json::Value>,
) -> Result<ConnectionId> {
    state
        .read()
        .await
        .ws_traffic(move |data| {
            let _ = on_message.send(data);
        })
        .await
}

#[command]
pub(crate) async fn ws_memory(
    state: State<'_, RwLock<Mihomo>>,
    on_message: Channel<serde_json::Value>,
) -> Result<ConnectionId> {
    state
        .read()
        .await
        .ws_memory(move |data| {
            let _ = on_message.send(data);
        })
        .await
}

#[command]
pub(crate) async fn ws_connections(
    state: State<'_, RwLock<Mihomo>>,
    on_message: Channel<serde_json::Value>,
) -> Result<ConnectionId> {
    state
        .read()
        .await
        .ws_connections(move |data| {
            let _ = on_message.send(data);
        })
        .await
}

#[command]
pub(crate) async fn ws_logs(
    state: State<'_, RwLock<Mihomo>>,
    level: LogLevel,
    on_message: Channel<serde_json::Value>,
) -> Result<ConnectionId> {
    state
        .read()
        .await
        .ws_logs(level, move |data| {
            let _ = on_message.send(data);
        })
        .await
}

// mihomo 的 websocket 应该只读取数据，没必要发送数据
// #[command]
// pub(crate) async fn ws_send(
//     state: State<'_, RwLock<Mihomo>>,
//     id: u32,
//     message: WebSocketMessage,
// ) -> Result<()> {
//     state.read().await.send(id, message).await
// }

#[command]
pub(crate) async fn ws_disconnect(
    state: State<'_, RwLock<Mihomo>>,
    id: ConnectionId,
    force_timeout: Option<u64>,
) -> Result<()> {
    state.read().await.disconnect(id, force_timeout).await
}

#[command]
pub(crate) async fn clear_all_ws_connections(state: State<'_, RwLock<Mihomo>>) -> Result<()> {
    state.write().await.clear_all_ws_connections().await
}

// engine api
#[command]
pub(crate) async fn get_engine_stats(state: State<'_, RwLock<Mihomo>>) -> Result<crate::models::EngineStats> {
    state.read().await.get_engine_stats().await
}

#[command]
pub(crate) async fn get_top_connections(
    state: State<'_, RwLock<Mihomo>>,
) -> Result<Vec<crate::models::ConnTrafficSnapshot>> {
    state.read().await.get_top_connections().await
}

#[command]
pub(crate) async fn get_buffer_pool_stats(state: State<'_, RwLock<Mihomo>>) -> Result<crate::models::BufferPoolStats> {
    state.read().await.get_buffer_pool_stats().await
}

#[command]
pub(crate) async fn get_rule_traffic(
    state: State<'_, RwLock<Mihomo>>,
) -> Result<std::collections::HashMap<String, crate::models::RuleTrafficSnapshot>> {
    state.read().await.get_rule_traffic().await
}

#[command]
pub(crate) async fn get_egress_status(state: State<'_, RwLock<Mihomo>>) -> Result<crate::models::EgressStatus> {
    state.read().await.get_egress_status().await
}

#[command]
pub(crate) async fn get_tls_fingerprint_stats(
    state: State<'_, RwLock<Mihomo>>,
) -> Result<crate::models::TLSFingerprintStats> {
    state.read().await.get_tls_fingerprint_stats().await
}

#[command]
pub(crate) async fn force_tls_rotation(state: State<'_, RwLock<Mihomo>>) -> Result<crate::models::TLSRotationResult> {
    state.read().await.force_tls_rotation().await
}

#[command]
pub(crate) async fn get_perf_stats(state: State<'_, RwLock<Mihomo>>) -> Result<crate::models::PerfStats> {
    state.read().await.get_perf_stats().await
}

#[command]
pub(crate) async fn get_hot_reload_status(state: State<'_, RwLock<Mihomo>>) -> Result<crate::models::HotReloadStatus> {
    state.read().await.get_hot_reload_status().await
}

#[command]
pub(crate) async fn get_xdp_status(state: State<'_, RwLock<Mihomo>>) -> Result<crate::models::XDPStatus> {
    state.read().await.get_xdp_status().await
}
