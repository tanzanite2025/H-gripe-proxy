/**
 * 出口 IP 探测逻辑
 */
use std::time::Duration;

use anyhow::{Result, anyhow};
use tokio::time::Instant;

use super::config::EgressIpProbeResult;

/// 通过 Mihomo 本地 mixed-port 主动探测出口 IP
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

async fn build_proxied_client() -> Result<reqwest::Client> {
    use crate::config::Config;

    let verge_port = Config::verge().await.data_arc().verge_mixed_port;
    let mixed_port = match verge_port {
        Some(port) if port > 0 => port,
        _ => Config::clash().await.data_arc().get_mixed_port(),
    };

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
