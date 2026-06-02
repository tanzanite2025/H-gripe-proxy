use super::{
    constants::*,
    geoip::{GeoIpInfo, fetch_ip_location, fetch_public_ip_location},
    helpers::current_timestamp_ms,
    input::{DiagnosticsInput, build_diagnostics_input},
    runtime_state::build_dns_runtime_status,
};
use crate::core::{
    runtime_snapshot::RuntimeSnapshotService,
    runtime_status::{DnsLeakServer, DnsLeakTestResult, DnsRuntimeStatus},
};
use crate::utils::network::{NetworkManager, ProxyType};
use anyhow::{Result, anyhow};
use clash_verge_logging::{Type, logging};
use reqwest::Client;
use smartstring::alias::String;
use std::collections::HashSet;
use tauri_plugin_mihomo::models::DnsMetrics;

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
        return items.iter().filter_map(parse_dns_server_value).collect();
    }

    if let Some(items) = data.get("dns_servers").and_then(|item| item.as_array()) {
        return items.iter().filter_map(parse_dns_server_value).collect();
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

    for url in ["https://ipleak.net/json/", "https://www.dnsleaktest.com/api/query"] {
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
    let (public_ip_result, dns_servers_result) =
        tokio::join!(fetch_public_ip_location(client), query_dns_leak_servers(client));

    let mut warnings = Vec::new();

    let public_ip_info = match public_ip_result {
        Ok(info) => Some(info),
        Err(err) => {
            warnings.push(format!("failed to fetch current egress IP location: {err}").into());
            None
        }
    };

    let dns_servers = match dns_servers_result {
        Ok(servers) => enrich_dns_servers(client, servers).await,
        Err(err) => {
            warnings.push(format!("failed to fetch DNS server info from leak detection services: {err}").into());
            Vec::new()
        }
    };

    (public_ip_info, dns_servers, warnings)
}

fn build_metrics_risk_types(metrics: Option<&DnsMetrics>) -> Vec<String> {
    let Some(metrics) = metrics else {
        return Vec::new();
    };

    let mut risk_types = Vec::new();

    if metrics.trust.unencrypted > 0 {
        risk_types.push("core-dns-unencrypted-server".into());
    }

    if metrics.trust.leak_risk_score >= 0.7 {
        risk_types.push("core-dns-high-risk-score".into());
    }

    if metrics.pollution.polluted_count > 0 {
        risk_types.push("core-dns-polluted-response".into());
    }

    if metrics.queries.total >= 5 {
        let failure_rate = metrics.queries.failed as f64 / metrics.queries.total as f64;
        if failure_rate >= 0.25 {
            risk_types.push("core-dns-high-failure-rate".into());
        }
    }

    risk_types.sort();
    risk_types.dedup();
    risk_types
}

fn build_runtime_risk_types(runtime_status: &DnsRuntimeStatus, metrics: Option<&DnsMetrics>) -> Vec<String> {
    let mut risk_types = Vec::new();

    if runtime_status.derived.default_nameserver_plain_count > 0 {
        risk_types.push("plain-dns-bootstrap".into());
    }

    if runtime_status.derived.leak_protection_safe == Some(false) {
        risk_types.push("dns-protection-insufficient".into());
    }

    if matches!(
        runtime_status.derived.leak_protection_level.as_deref(),
        Some("strict" | "paranoid")
    ) && runtime_status.snapshot.use_system_hosts == Some(true)
    {
        risk_types.push("system-hosts-enabled".into());
    }

    if runtime_status.enable_dns_settings && !runtime_status.runtime_matches_saved {
        risk_types.push("runtime-dns-not-synced".into());
    }

    risk_types.extend(build_metrics_risk_types(metrics));
    risk_types.sort();
    risk_types.dedup();
    risk_types
}

fn build_runtime_risk_types_from_input(runtime_status: &DnsRuntimeStatus, input: &DiagnosticsInput) -> Vec<String> {
    build_runtime_risk_types(runtime_status, input.dns_metrics.as_ref())
}

fn build_observed_leak_types(location_mismatch: bool) -> Vec<String> {
    let mut observed_types = Vec::new();

    if location_mismatch {
        observed_types.push("dns-location-mismatch".into());
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

fn build_dns_leak_confidence(
    observation_path: &str,
    observation_incomplete: bool,
    metrics: Option<&DnsMetrics>,
) -> &'static str {
    let has_core_dns_observation = metrics
        .map(|metrics| metrics.queries.total > 0 || !metrics.recent.is_empty())
        .unwrap_or(false);

    if observation_incomplete && !has_core_dns_observation {
        DNS_LEAK_CONFIDENCE_LOW
    } else if observation_path == DNS_LEAK_OBSERVATION_CORE_PROXY {
        DNS_LEAK_CONFIDENCE_HIGH
    } else if has_core_dns_observation {
        DNS_LEAK_CONFIDENCE_MEDIUM
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
    metrics: Option<&DnsMetrics>,
    dns_location: Option<&str>,
    ip_location: &str,
    observation_path: &str,
    confidence: &str,
) -> Vec<String> {
    let mut recommendations = Vec::new();

    match observation_path {
        DNS_LEAK_OBSERVATION_CORE_PROXY => {}
        DNS_LEAK_OBSERVATION_CORE_PROXY_FALLBACK_DIRECT => {
            recommendations.push(
                "Local-core DNS observation was incomplete; some results fell back to direct observation.".into(),
            );
        }
        _ => {
            recommendations.push("DNS leak test did not fully run through the local core proxy path.".into());
        }
    }

    if observation_incomplete {
        recommendations.push(match confidence {
            DNS_LEAK_CONFIDENCE_LOW => {
                "External DNS observation is incomplete; retry later for a stronger result.".into()
            }
            _ => "External DNS observation has gaps; combine this with runtime DNS state.".into(),
        });
    }

    if !observed_leak && !runtime_risk_detected && !observation_incomplete {
        recommendations.push("No obvious DNS leak was observed.".into());
        return recommendations;
    }

    if observed_leak {
        recommendations.push("Observed DNS leak signal detected.".into());
    } else if runtime_risk_detected {
        recommendations.push("No external leak was observed, but runtime DNS risk signals exist.".into());
    } else if observation_incomplete {
        recommendations.push("External DNS observation is incomplete; retry while local core is running.".into());
    }

    if location_mismatch {
        if let Some(dns_location) = dns_location {
            recommendations.push(format!("DNS server location: {dns_location}").into());
        }
        recommendations.push(format!("Egress IP location: {ip_location}").into());
    }

    if runtime_status.derived.default_nameserver_plain_count > 0 {
        recommendations.push("Remove plain DNS from default-nameserver or switch bootstrap to encrypted DNS.".into());
    }

    if let Some(metrics) = metrics {
        if metrics.trust.unencrypted > 0 {
            recommendations.push(
                format!(
                    "Local core reports {} unencrypted DNS server(s) in use.",
                    metrics.trust.unencrypted
                )
                .into(),
            );
        }
        if metrics.trust.leak_risk_score >= 0.7 {
            recommendations.push(
                format!(
                    "Local core DNS trust risk score is {:.2}; inspect the server list exported by the core.",
                    metrics.trust.leak_risk_score
                )
                .into(),
            );
        }
        if metrics.pollution.polluted_count > 0 {
            recommendations.push(
                format!(
                    "Local core detected {} polluted DNS response(s).",
                    metrics.pollution.polluted_count
                )
                .into(),
            );
        }
        if metrics.queries.total >= 5 {
            let failure_rate = metrics.queries.failed as f64 / metrics.queries.total as f64;
            if failure_rate >= 0.25 {
                recommendations.push(
                    format!(
                        "Local core DNS failure rate is {:.0}%; check resolver health and routing.",
                        failure_rate * 100.0
                    )
                    .into(),
                );
            }
        }
    }

    if runtime_status.derived.leak_protection_safe == Some(false) {
        recommendations
            .push("Enable at least basic DNS leak protection; use strict or paranoid for sensitive exits.".into());
    }

    if matches!(
        runtime_status.derived.leak_protection_level.as_deref(),
        Some("strict" | "paranoid")
    ) && runtime_status.snapshot.use_system_hosts == Some(true)
    {
        recommendations.push("Disable system hosts in strict or paranoid DNS protection.".into());
    }

    if runtime_status.enable_dns_settings && !runtime_status.runtime_matches_saved {
        recommendations.push("Saved DNS config is not fully applied to runtime; re-apply DNS settings.".into());
    }

    if recommendations.is_empty() {
        recommendations.push("Check whether DNS runtime config matches the selected proxy egress.".into());
    }

    recommendations
}

pub async fn build_dns_leak_test_result() -> Result<DnsLeakTestResult> {
    let runtime_status = build_dns_runtime_status().await?;
    let snapshot_service = RuntimeSnapshotService::global();
    let diagnostics_input = build_diagnostics_input(&snapshot_service).await;
    let core_running = diagnostics_input.core_running;
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
                        .map(|warning| format!("while checking through local core: {warning}").into()),
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
                            .map(|warning| format!("while checking through direct fallback: {warning}").into()),
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
                warnings
                    .push(format!("failed to create local-core DNS check request; fell back to direct: {err}").into());

                let direct_client = network_manager
                    .create_request(ProxyType::None, Some(8), None, false)
                    .await
                    .map_err(|inner_err| anyhow!(inner_err.to_string()))?;
                let (public_ip_info, dns_servers, direct_warnings) = observe_dns_leak_with_client(&direct_client).await;
                warnings.extend(
                    direct_warnings
                        .into_iter()
                        .map(|warning| format!("while checking directly: {warning}").into()),
                );

                (public_ip_info, dns_servers, DNS_LEAK_OBSERVATION_DIRECT)
            }
        }
    } else {
        let direct_client = network_manager
            .create_request(ProxyType::None, Some(8), None, false)
            .await
            .map_err(|err| anyhow!(err.to_string()))?;
        let (public_ip_info, dns_servers, direct_warnings) = observe_dns_leak_with_client(&direct_client).await;
        warnings.extend(direct_warnings);

        (public_ip_info, dns_servers, DNS_LEAK_OBSERVATION_DIRECT)
    };

    let checked_via_core_proxy = observation_path == DNS_LEAK_OBSERVATION_CORE_PROXY;
    let ip_location = public_ip_info
        .as_ref()
        .and_then(|info| info.country.clone())
        .unwrap_or_else(|| "Unknown".into());
    let dns_countries: Vec<String> = dns_servers.iter().filter_map(|server| server.country.clone()).collect();
    let dns_location = dns_countries.first().cloned();
    let location_comparable = !dns_countries.is_empty() && ip_location != "Unknown";

    let location_mismatch = location_comparable && dns_countries.iter().any(|country| country != &ip_location);

    let runtime_risk_type = build_runtime_risk_types_from_input(&runtime_status, &diagnostics_input);
    let observed_leak_type = build_observed_leak_types(location_mismatch);
    let runtime_risk_detected = !runtime_risk_type.is_empty();
    let observed_leak = !observed_leak_type.is_empty();
    let mut leak_type = runtime_risk_type.clone();
    leak_type.extend(observed_leak_type.clone());
    leak_type.sort();
    leak_type.dedup();

    let observation_incomplete = !location_comparable;
    let assessment = build_dns_leak_assessment(observed_leak, runtime_risk_detected, observation_incomplete);
    let confidence = build_dns_leak_confidence(
        observation_path,
        observation_incomplete,
        diagnostics_input.dns_metrics.as_ref(),
    );

    let has_leak = observed_leak;
    let risk_level =
        if location_mismatch && dns_countries.iter().any(|country| country == "China") && ip_location != "China" {
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
            diagnostics_input.dns_metrics.as_ref(),
            dns_location.as_deref(),
            ip_location.as_str(),
            observation_path,
            confidence,
        ),
        dns_servers,
        dns_metrics: diagnostics_input.dns_metrics,
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::runtime_status::{DnsRuntimeDerivedState, DnsRuntimeSnapshot, DnsRuntimeStatus};
    use std::collections::HashMap;
    use tauri_plugin_mihomo::models::{
        DnsCacheStats, DnsPollutionStats, DnsQueryStats, DnsServerClassification, DnsTrustSummary,
    };

    fn runtime_status_for_risk(snapshot: DnsRuntimeSnapshot, derived: DnsRuntimeDerivedState) -> DnsRuntimeStatus {
        DnsRuntimeStatus {
            enable_dns_settings: true,
            dns_config_exists: true,
            dns_config_valid: true,
            runtime_has_dns: true,
            runtime_has_hosts: true,
            runtime_dns_matches_saved: true,
            runtime_hosts_matches_saved: true,
            runtime_matches_saved: true,
            snapshot,
            derived,
        }
    }

    #[test]
    fn test_runtime_risk_flags_system_hosts_in_strict_dns() {
        let runtime_status = runtime_status_for_risk(
            DnsRuntimeSnapshot {
                enhanced_mode: Some("fake-ip".into()),
                ipv6: Some(true),
                nameserver_count: 2,
                fallback_count: 2,
                nameserver_policy_count: 2,
                use_hosts: Some(true),
                use_system_hosts: Some(true),
                respect_rules: Some(true),
            },
            DnsRuntimeDerivedState {
                routing_mode: Some("balanced".into()),
                domestic_dns: vec![],
                foreign_dns: vec![],
                default_nameserver_count: 0,
                default_nameserver_plain_count: 0,
                prefer_h3: Some(true),
                leak_protection_level: Some("strict".into()),
                leak_protection_security: Some("high".into()),
                leak_protection_safe: Some(true),
            },
        );

        let risks = build_runtime_risk_types(&runtime_status, None);

        assert!(risks.iter().any(|risk| risk == "system-hosts-enabled"));
    }

    #[test]
    fn test_encrypted_bootstrap_does_not_count_as_plain_dns_risk() {
        let runtime_status = runtime_status_for_risk(
            DnsRuntimeSnapshot {
                enhanced_mode: Some("fake-ip".into()),
                ipv6: Some(true),
                nameserver_count: 2,
                fallback_count: 2,
                nameserver_policy_count: 2,
                use_hosts: Some(true),
                use_system_hosts: Some(false),
                respect_rules: Some(true),
            },
            DnsRuntimeDerivedState {
                routing_mode: Some("balanced".into()),
                domestic_dns: vec![],
                foreign_dns: vec![],
                default_nameserver_count: 2,
                default_nameserver_plain_count: 0,
                prefer_h3: Some(false),
                leak_protection_level: Some("strict".into()),
                leak_protection_security: Some("high".into()),
                leak_protection_safe: Some(true),
            },
        );

        let risks = build_runtime_risk_types(&runtime_status, None);

        assert!(!risks.iter().any(|risk| risk == "plain-dns-bootstrap"));
    }

    #[test]
    fn test_core_dns_metrics_drive_runtime_risk_types() {
        let metrics = DnsMetrics {
            cache: DnsCacheStats {
                hit: 0,
                miss: 0,
                size: 0,
                hit_rate: 0.0,
            },
            queries: DnsQueryStats {
                total: 8,
                success: 5,
                failed: 3,
                avg_latency_us: 10,
                max_latency_us: 20,
            },
            servers: vec![],
            recent: vec![],
            pollution: DnsPollutionStats {
                total_checked: 2,
                polluted_count: 1,
                pollution_rate: 0.5,
                recent_polluted: vec![],
            },
            trust: DnsTrustSummary {
                total: 2,
                encrypted: 1,
                unencrypted: 1,
                by_trust_level: HashMap::new(),
                servers: vec![DnsServerClassification {
                    address: "223.5.5.5".into(),
                    protocol: "UDP".into(),
                    trust_level: "untrusted".into(),
                    encrypted: false,
                    description: None,
                }],
                leak_risk_score: 0.8,
                last_evaluated: "now".into(),
            },
        };

        let risks = build_metrics_risk_types(Some(&metrics));

        assert!(risks.iter().any(|risk| risk == "core-dns-unencrypted-server"));
        assert!(risks.iter().any(|risk| risk == "core-dns-high-risk-score"));
        assert!(risks.iter().any(|risk| risk == "core-dns-polluted-response"));
        assert!(risks.iter().any(|risk| risk == "core-dns-high-failure-rate"));
    }

    #[test]
    fn test_runtime_risk_types_use_diagnostics_input_metrics() {
        let metrics = DnsMetrics {
            cache: DnsCacheStats {
                hit: 0,
                miss: 0,
                size: 0,
                hit_rate: 0.0,
            },
            queries: DnsQueryStats {
                total: 10,
                success: 10,
                failed: 0,
                avg_latency_us: 10,
                max_latency_us: 20,
            },
            servers: vec![],
            recent: vec![],
            pollution: DnsPollutionStats {
                total_checked: 0,
                polluted_count: 0,
                pollution_rate: 0.0,
                recent_polluted: vec![],
            },
            trust: DnsTrustSummary {
                total: 1,
                encrypted: 0,
                unencrypted: 1,
                by_trust_level: HashMap::new(),
                servers: vec![],
                leak_risk_score: 0.8,
                last_evaluated: "2026-06-02T00:00:00Z".into(),
            },
        };
        let input = DiagnosticsInput {
            core_running: true,
            dns_metrics: Some(metrics),
            ..DiagnosticsInput::default()
        };
        let runtime_status = runtime_status_for_risk(
            DnsRuntimeSnapshot {
                enhanced_mode: Some("fake-ip".into()),
                ipv6: Some(true),
                nameserver_count: 2,
                fallback_count: 2,
                nameserver_policy_count: 2,
                use_hosts: Some(true),
                use_system_hosts: Some(false),
                respect_rules: Some(true),
            },
            DnsRuntimeDerivedState {
                routing_mode: Some("balanced".into()),
                domestic_dns: vec![],
                foreign_dns: vec![],
                default_nameserver_count: 0,
                default_nameserver_plain_count: 0,
                prefer_h3: Some(false),
                leak_protection_level: Some("strict".into()),
                leak_protection_security: Some("high".into()),
                leak_protection_safe: Some(true),
            },
        );

        let risks = build_runtime_risk_types_from_input(&runtime_status, &input);

        assert!(risks.iter().any(|risk| risk == "core-dns-unencrypted-server"));
        assert!(risks.iter().any(|risk| risk == "core-dns-high-risk-score"));
    }
}
