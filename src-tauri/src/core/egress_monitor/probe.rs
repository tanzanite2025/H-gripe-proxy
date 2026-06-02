/**
 * 出口 IP 探测逻辑
 */
use std::time::Duration;

use anyhow::{Result, anyhow};
use tokio::time::Instant;

use super::config::EgressIpProbeResult;

/// 通过 Mihomo 代理探测出口 IP
pub async fn probe_egress_ip() -> Result<EgressIpProbeResult> {
    let client = build_proxied_client().await?;

    let start = Instant::now();
    let (ip, country_code) = fetch_exit_ip_geo(&client).await?;

    Ok(EgressIpProbeResult {
        ip,
        country_code,
        probed_at_ms: now_ms(),
        latency_ms: start.elapsed().as_millis() as u64,
    })
}

async fn fetch_exit_ip_geo(client: &reqwest::Client) -> Result<(String, Option<String>)> {
    let info = crate::core::runtime_diagnostics::geoip::fetch_public_ip_observation(client).await?;
    let ip = info.ip.ok_or_else(|| anyhow!("public IP observation returned no IP"))?;

    Ok((
        ip.to_string(),
        info.country_code.map(|code| code.to_uppercase().to_string()),
    ))
}

/// 构建走 Mihomo 代理的 HTTP 客户端
async fn build_proxied_client() -> Result<reqwest::Client> {
    use crate::config::Config;

    let verge = Config::verge().await.latest_arc();
    let proxy_enabled = verge.enable_system_proxy.unwrap_or(false) || verge.enable_tun_mode.unwrap_or(false);

    if !proxy_enabled {
        return Err(anyhow!("代理未启用，无法探测出口 IP"));
    }

    let mixed_port = verge.verge_mixed_port.unwrap_or_else(|| {
        // fallback: 从 clash 配置读取
        7897
    });

    let proxy_url = format!("http://127.0.0.1:{mixed_port}");

    let client = reqwest::Client::builder()
        .proxy(reqwest::Proxy::all(&proxy_url)?)
        .timeout(Duration::from_secs(15))
        .no_proxy()
        .build()?;

    Ok(client)
}

pub fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}
