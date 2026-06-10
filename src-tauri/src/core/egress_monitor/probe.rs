use std::net::IpAddr;
use std::time::Duration;

use anyhow::{Result, anyhow};
use tokio::time::Instant;

use super::config::EgressIpProbeResult;

pub async fn probe_egress_ip() -> Result<EgressIpProbeResult> {
    let client = build_proxied_client().await?;

    let start = Instant::now();
    let ip = fetch_exit_ip(&client).await?;
    let metadata = lookup_local_metadata(&ip).await;

    Ok(EgressIpProbeResult {
        ip,
        country_code: metadata
            .as_ref()
            .and_then(|item| normalize_country_code(item.country_code.as_deref())),
        city: metadata
            .as_ref()
            .and_then(|item| normalize_optional_string(item.city.as_deref())),
        timezone: metadata
            .as_ref()
            .and_then(|item| normalize_optional_string(item.timezone.as_deref())),
        probed_at_ms: now_ms(),
        latency_ms: start.elapsed().as_millis() as u64,
    })
}

async fn fetch_exit_ip(client: &reqwest::Client) -> Result<String> {
    let plain_ip = crate::core::runtime_diagnostics::geoip::fetch_public_ip_plain(client).await?;
    if is_ip_address(&plain_ip) {
        return Ok(plain_ip.to_string());
    }

    let ipv4_ip = crate::core::runtime_diagnostics::geoip::fetch_public_ipv4_plain(client).await?;
    if is_ip_address(&ipv4_ip) {
        return Ok(ipv4_ip.to_string());
    }

    Err(anyhow!("public IP observation returned no valid IP address"))
}

async fn lookup_local_metadata(ip: &str) -> Option<crate::core::ip_intelligence::IpIntelligenceRecord> {
    crate::feat::get_ip_reputation_manager()
        .lookup_ip_metadata_record(ip)
        .await
        .ok()
}

fn normalize_country_code(value: Option<&str>) -> Option<String> {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty() && !value.eq_ignore_ascii_case("unknown"))
        .map(str::to_ascii_uppercase)
}

fn normalize_optional_string(value: Option<&str>) -> Option<String> {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
}

fn is_ip_address(value: &str) -> bool {
    value.trim().parse::<IpAddr>().is_ok()
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
