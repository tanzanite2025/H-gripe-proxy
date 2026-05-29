use super::{
    constants::*,
    geoip::{fetch_ip_location, fetch_public_ip_location, GeoIpInfo},
    helpers::current_timestamp_ms,
    runtime_state::build_dns_runtime_status,
};
use crate::core::{
    CoreManager,
    manager::RunningMode,
    runtime_status::{DnsLeakServer, DnsLeakTestResult, DnsRuntimeStatus},
};
use crate::utils::network::{NetworkManager, ProxyType};
use anyhow::{anyhow, Result};
use clash_verge_logging::{Type, logging};
use reqwest::Client;
use smartstring::alias::String;
use std::collections::HashSet;

fn parse_dns_server_value(value: &serde_json::Value) -> Option<DnsLeakServer> {
    if let Some(ip) = value.as_str() {
        return Some(DnsLeakServer {
            ip: ip.into(),
            hostname: None,
            country: None,
            city: None,
            isp: None,
        });
    }

    let ip = value
        .get("ip")
        .or_else(|| value.get("address"))
        .and_then(|item| item.as_str())?;

    Some(DnsLeakServer {
        ip: ip.into(),
        hostname: value
            .get("hostname")
            .or_else(|| value.get("name"))
            .and_then(|item| item.as_str())
            .map(Into::into),
        country: value.get("country").and_then(|item| item.as_str()).map(Into::into),
        city: value.get("city").and_then(|item| item.as_str()).map(Into::into),
        isp: value
            .get("isp")
            .or_else(|| value.get("org"))
            .and_then(|item| item.as_str())
            .map(Into::into),
    })
}

fn parse_dns_servers(data: &serde_json::Value) -> Vec<DnsLeakServer> {
    if let Some(items) = data.as_array() {
        return items
            .iter()
            .filter_map(parse_dns_server_value)
            .collect();
    }

    if let Some(items) = data.get("dns_servers").and_then(|item| item.as_array()) {
        return items
            .iter()
            .filter_map(parse_dns_server_value)
            .collect();
    }

    Vec::new()
}

fn dedupe_dns_servers(servers: Vec<DnsLeakServer>) -> Vec<DnsLeakServer> {
    let mut seen = HashSet::new();
    let mut deduped = Vec::new();

    for server in servers {
        if seen.insert(server.ip.clone()) {
            deduped.push(server);
        }
    }

    deduped
}

async fn query_dns_leak_servers(client: &Client) -> Result<Vec<DnsLeakServer>> {
    let mut observed_servers = Vec::new();
    let mut errors: Vec<String> = Vec::new();

    for url in [
        "https://ipleak.net/json/",
        "https://www.dnsleaktest.com/api/query",
    ] {
        match super::geoip::fetch_json(client, url).await {
            Ok(data) => {
                observed_servers.extend(parse_dns_servers(&data));
            }
            Err(err) => {
                logging!(warn, Type::Config, "DNS leak service failed for {url}: {err}");
                errors.push(format!("{url}: {err}").into());
            }
        }
    }

    let servers = dedupe_dns_servers(observed_servers);
    if !servers.is_empty() {
        return Ok(servers);
    }

    if errors.is_empty() {
        Err(anyhow!("failed to fetch DNS server list from leak detection services"))
    } else {
        Err(anyhow!(
            "failed to fetch DNS server list from leak detection services: {}",
            errors.join(" | ")
        ))
    }
}

async fn enrich_dns_servers(client: &Client, servers: Vec<DnsLeakServer>) -> Vec<DnsLeakServer> {
    let mut enriched = Vec::with_capacity(servers.len());

    for mut server in servers {
        if server.country.is_none() || server.city.is_none() || server.isp.is_none() {
            let geo = fetch_ip_location(client, server.ip.as_str()).await;
            if server.country.is_none() {
                server.country = geo.country;
            }
            if server.city.is_none() {
                server.city = geo.city;
            }
            if server.isp.is_none() {
                server.isp = geo.isp;
            }
        }

        enriched.push(server);
    }

    enriched
}

async fn observe_dns_leak_with_client(client: &Client) -> (Option<GeoIpInfo>, Vec<DnsLeakServer>, Vec<String>) {
    let (public_ip_result, dns_servers_result) = tokio::join!(
        fetch_public_ip_location(client),
        query_dns_leak_servers(client)
    );

    let mut warnings = Vec::new();

    let public_ip_info = match public_ip_result {
        Ok(info) => Some(info),
        Err(err) => {
            warnings.push(format!("无法获取当前出口 IP 地理位置: {err}").into());
            None
        }
    };

    let dns_servers = match dns_servers_result {
        Ok(servers) => enrich_dns_servers(client, servers).await,
        Err(err) => {
            warnings.push(format!("无法从外部泄漏检测服务获取 DNS 服务器信息: {err}").into());
            Vec::new()
        }
    };

    (public_ip_info, dns_servers, warnings)
}

fn build_runtime_risk_types(runtime_status: &DnsRuntimeStatus) -> Vec<String> {
    let mut risk_types = Vec::new();

    if runtime_status.derived.default_nameserver_count > 0 {
        risk_types.push("明文 DNS 引导".into());
    }

    if runtime_status.derived.leak_protection_safe == Some(false) {
        risk_types.push("运行态 DNS 防护不足".into());
    }

    if runtime_status.enable_dns_settings && !runtime_status.runtime_matches_saved {
        risk_types.push("运行态 DNS 配置尚未与已保存配置对齐".into());
    }

    risk_types
}

fn build_observed_leak_types(location_mismatch: bool) -> Vec<String> {
    let mut observed_types = Vec::new();

    if location_mismatch {
        observed_types.push("DNS 地理位置不匹配".into());
    }

    observed_types
}

fn build_dns_leak_assessment(
    observed_leak: bool,
    runtime_risk_detected: bool,
    observation_incomplete: bool,
) -> &'static str {
    if observed_leak {
        DNS_LEAK_ASSESSMENT_OBSERVED_LEAK
    } else if runtime_risk_detected {
        DNS_LEAK_ASSESSMENT_RUNTIME_RISK
    } else if observation_incomplete {
        DNS_LEAK_ASSESSMENT_INCONCLUSIVE
    } else {
        DNS_LEAK_ASSESSMENT_SAFE
    }
}

fn build_dns_leak_confidence(observation_path: &str, observation_incomplete: bool) -> &'static str {
    if observation_incomplete {
        DNS_LEAK_CONFIDENCE_LOW
    } else if observation_path == DNS_LEAK_OBSERVATION_CORE_PROXY {
        DNS_LEAK_CONFIDENCE_HIGH
    } else {
        DNS_LEAK_CONFIDENCE_MEDIUM
    }
}

fn build_dns_leak_recommendations(
    observed_leak: bool,
    runtime_risk_detected: bool,
    observation_incomplete: bool,
    location_mismatch: bool,
    runtime_status: &DnsRuntimeStatus,
    dns_location: Option<&str>,
    ip_location: &str,
    observation_path: &str,
    confidence: &str,
) -> Vec<String> {
    let mut recommendations = Vec::new();

    match observation_path {
        DNS_LEAK_OBSERVATION_CORE_PROXY => {}
        DNS_LEAK_OBSERVATION_CORE_PROXY_FALLBACK_DIRECT => {
            recommendations.push("本地 core 代理观测不完整，部分结果已回退到直连检测".into());
        }
        _ => {
            recommendations.push("当前检测未完全经过本地 core 代理，结果可能反映直连网络".into());
        }
    }

    if observation_incomplete {
        recommendations.push(
            match confidence {
                DNS_LEAK_CONFIDENCE_LOW => "外部观测不完整，当前检测置信度较低，建议稍后重试".into(),
                _ => "外部观测存在缺口，建议结合运行态配置与多次测试综合判断".into(),
            },
        );
    }

    if !observed_leak && !runtime_risk_detected && !observation_incomplete {
        recommendations.push("✅ 未检测到明显 DNS 泄漏".into());

        if runtime_status.enable_dns_settings && !runtime_status.runtime_matches_saved {
            recommendations.push("后端运行态尚未与已保存 DNS 配置完全对齐，建议重新应用 DNS 运行时配置".into());
        }

        return recommendations;
    }

    if observed_leak {
        recommendations.push("⚠️ 已观测到 DNS 泄漏迹象".into());
    } else if runtime_risk_detected {
        recommendations.push("⚠️ 当前未观测到明确外部泄漏，但运行态存在 DNS 风险信号".into());
    } else if observation_incomplete {
        recommendations.push("⚠️ 当前未形成完整外部观测，建议在本地 core 运行时重新检测".into());
    }

    if location_mismatch {
        if let Some(dns_location) = dns_location {
            recommendations.push(format!("DNS 服务器位置: {dns_location}").into());
        }
        recommendations.push(format!("出口 IP 位置: {ip_location}").into());
    }

    if runtime_status.derived.default_nameserver_count > 0 {
        recommendations.push("建议移除 default-nameserver 中的明文 DNS，引导也尽量改为加密 DNS".into());
    }

    if runtime_status.derived.leak_protection_safe == Some(false) {
        recommendations.push("建议至少启用基础 DNS 零泄漏防护，必要时切换到严格或偏执级别".into());
    }

    if runtime_status.enable_dns_settings && !runtime_status.runtime_matches_saved {
        recommendations.push("已保存 DNS 配置尚未完全进入运行态，建议重新应用 DNS 运行时配置".into());
    }

    if recommendations.is_empty() {
        recommendations.push("建议检查 DNS 运行态配置与代理出口是否一致".into());
    }

    recommendations
}

pub async fn build_dns_leak_test_result() -> Result<DnsLeakTestResult> {
    let runtime_status = build_dns_runtime_status().await?;
    let core_running = *CoreManager::global().get_running_mode() != RunningMode::NotRunning;
    let network_manager = NetworkManager::new();
    let mut warnings = Vec::new();

    let (public_ip_info, dns_servers, observation_path) = if core_running {
        match network_manager
            .create_request(ProxyType::Localhost, Some(8), None, false)
            .await
        {
            Ok(proxy_client) => {
                let (mut public_ip_info, mut dns_servers, proxy_warnings) =
                    observe_dns_leak_with_client(&proxy_client).await;
                warnings.extend(
                    proxy_warnings
                        .into_iter()
                        .map(|warning| format!("通过本地 core 检测时: {warning}").into()),
                );

                if public_ip_info.is_some() && !dns_servers.is_empty() {
                    (public_ip_info, dns_servers, DNS_LEAK_OBSERVATION_CORE_PROXY)
                } else {
                    let direct_client = network_manager
                        .create_request(ProxyType::None, Some(8), None, false)
                        .await
                        .map_err(|err| anyhow!(err.to_string()))?;
                    let (direct_public_ip_info, direct_dns_servers, direct_warnings) =
                        observe_dns_leak_with_client(&direct_client).await;
                    warnings.extend(
                        direct_warnings
                            .into_iter()
                            .map(|warning| format!("直连回退检测时: {warning}").into()),
                    );

                    if public_ip_info.is_none() {
                        public_ip_info = direct_public_ip_info;
                    }
                    if dns_servers.is_empty() {
                        dns_servers = direct_dns_servers;
                    } else if !direct_dns_servers.is_empty() {
                        dns_servers.extend(direct_dns_servers);
                        dns_servers = dedupe_dns_servers(dns_servers);
                    }

                    (
                        public_ip_info,
                        dns_servers,
                        DNS_LEAK_OBSERVATION_CORE_PROXY_FALLBACK_DIRECT,
                    )
                }
            }
            Err(err) => {
                warnings.push(format!("无法通过本地 core 建立检测请求，已回退直连: {err}").into());

                let direct_client = network_manager
                    .create_request(ProxyType::None, Some(8), None, false)
                    .await
                    .map_err(|inner_err| anyhow!(inner_err.to_string()))?;
                let (public_ip_info, dns_servers, direct_warnings) =
                    observe_dns_leak_with_client(&direct_client).await;
                warnings.extend(
                    direct_warnings
                        .into_iter()
                        .map(|warning| format!("直连检测时: {warning}").into()),
                );

                (
                    public_ip_info,
                    dns_servers,
                    DNS_LEAK_OBSERVATION_DIRECT,
                )
            }
        }
    } else {
        let direct_client = network_manager
            .create_request(ProxyType::None, Some(8), None, false)
            .await
            .map_err(|err| anyhow!(err.to_string()))?;
        let (public_ip_info, dns_servers, direct_warnings) =
            observe_dns_leak_with_client(&direct_client).await;
        warnings.extend(direct_warnings);

        (
            public_ip_info,
            dns_servers,
            DNS_LEAK_OBSERVATION_DIRECT,
        )
    };

    let checked_via_core_proxy = observation_path == DNS_LEAK_OBSERVATION_CORE_PROXY;
    let ip_location = public_ip_info
        .as_ref()
        .and_then(|info| info.country.clone())
        .unwrap_or_else(|| "Unknown".into());
    let dns_countries: Vec<String> = dns_servers
        .iter()
        .filter_map(|server| server.country.clone())
        .collect();
    let dns_location = dns_countries.first().cloned();
    let location_comparable = !dns_countries.is_empty() && ip_location != "Unknown";

    let location_mismatch =
        location_comparable && dns_countries.iter().any(|country| country != &ip_location);

    let runtime_risk_type = build_runtime_risk_types(&runtime_status);
    let observed_leak_type = build_observed_leak_types(location_mismatch);
    let runtime_risk_detected = !runtime_risk_type.is_empty();
    let observed_leak = !observed_leak_type.is_empty();
    let mut leak_type = runtime_risk_type.clone();
    leak_type.extend(observed_leak_type.clone());
    leak_type.sort();
    leak_type.dedup();

    let observation_incomplete = !location_comparable;
    let assessment = build_dns_leak_assessment(
        observed_leak,
        runtime_risk_detected,
        observation_incomplete,
    );
    let confidence = build_dns_leak_confidence(observation_path, observation_incomplete);

    let has_leak = observed_leak;
    let risk_level = if location_mismatch
        && dns_countries.iter().any(|country| country == "China")
        && ip_location != "China"
    {
        "danger".into()
    } else if observed_leak || runtime_risk_detected || observation_incomplete {
        "warning".into()
    } else {
        "safe".into()
    };

    Ok(DnsLeakTestResult {
        has_leak,
        observed_leak,
        runtime_risk_detected,
        observation_incomplete,
        confidence: confidence.into(),
        assessment: assessment.into(),
        leak_type,
        observed_leak_type,
        runtime_risk_type,
        warnings: warnings.clone(),
        recommendations: build_dns_leak_recommendations(
            observed_leak,
            runtime_risk_detected,
            observation_incomplete,
            location_mismatch,
            &runtime_status,
            dns_location.as_deref(),
            ip_location.as_str(),
            observation_path,
            confidence,
        ),
        dns_servers,
        dns_location,
        ip_location,
        location_match: location_comparable && !location_mismatch,
        location_comparable,
        risk_level,
        timestamp: current_timestamp_ms(),
        checked_via_core_proxy,
        observation_path: observation_path.into(),
        error: None,
    })
}
