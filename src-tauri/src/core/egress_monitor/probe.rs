/**
 * 出口 IP 探测逻辑
 */

use std::time::Duration;

use anyhow::{anyhow, Result};
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

/// 通过多个 GeoIP 服务探测出口 IP 和国家代码
async fn fetch_exit_ip_geo(client: &reqwest::Client) -> Result<(String, Option<String>)> {
    let urls = [
        "https://api.ip.sb/geoip",
        "https://ipapi.co/json",
        "https://ipwho.is/",
    ];

    for url in &urls {
        match client.get(*url).send().await {
            Ok(resp) if resp.status().is_success() => {
                if let Ok(data) = resp.json::<serde_json::Value>().await {
                    let ip = data
                        .get("ip")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string());

                    if let Some(ip) = ip {
                        let country_code = data
                            .get("country_code")
                            .or_else(|| data.get("data").and_then(|d| d.get("country_code")))
                            .or_else(|| data.get("country"))
                            .and_then(|v| v.as_str())
                            .map(|s| s.to_uppercase());
                        return Ok((ip, country_code));
                    }
                }
            }
            Ok(_) => continue,
            Err(_) => continue,
        }
    }

    // 降级：仅获取 IP（不带 Geo）
    let plain_urls = [
        "https://api.ipify.org",
        "https://ifconfig.me/ip",
        "https://icanhazip.com",
    ];

    for url in &plain_urls {
        if let Ok(resp) = client.get(*url).send().await {
            if resp.status().is_success() {
                if let Ok(ip) = resp.text().await {
                    let ip = ip.trim().to_string();
                    if !ip.is_empty() {
                        return Ok((ip, None));
                    }
                }
            }
        }
    }

    Err(anyhow!("所有 IP 探测服务均不可用"))
}

/// 构建走 Mihomo 代理的 HTTP 客户端
async fn build_proxied_client() -> Result<reqwest::Client> {
    use crate::config::Config;

    let verge = Config::verge().await.latest_arc();
    let proxy_enabled = verge.enable_system_proxy.unwrap_or(false)
        || verge.enable_tun_mode.unwrap_or(false);

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
