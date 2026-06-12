use crate::core::runtime_status::ProxyDetectionLocation;
use anyhow::{Context as _, Result, anyhow};
use clash_verge_logging::{Type, logging};
use reqwest::Client;
use serde_json::Value as JsonValue;
use smartstring::alias::String;
use std::net::IpAddr;

const PUBLIC_IP_PLAIN_SOURCE: &str = "https://api.ipify.org";
const PUBLIC_IPV4_PLAIN_SOURCE: &str = "https://api4.ipify.org";

pub const PUBLIC_IP_PROBE_HOSTS: [&str; 2] = [
    "api.ipify.org",
    "api4.ipify.org",
];

#[derive(Debug, Clone, Default)]
pub struct GeoIpInfo {
    pub ip: Option<String>,
    pub country_code: Option<String>,
    pub country: Option<String>,
    pub region: Option<String>,
    pub city: Option<String>,
    pub organization: Option<String>,
    pub asn: Option<u32>,
    pub asn_organization: Option<String>,
    pub isp: Option<String>,
}

fn has_location_identity(info: &GeoIpInfo) -> bool {
    info.country.is_some() || info.country_code.is_some() || info.ip.is_some()
}

fn has_asn_metadata(info: &GeoIpInfo) -> bool {
    info.asn.is_some() || info.asn_organization.is_some() || info.organization.is_some() || info.isp.is_some()
}

fn is_ipv4_address(value: &str) -> bool {
    value.trim().parse::<IpAddr>().is_ok_and(|ip| ip.is_ipv4())
}

pub(super) async fn fetch_json(client: &Client, url: &str) -> Result<JsonValue> {
    let response = client
        .get(url)
        .send()
        .await
        .with_context(|| format!("request failed: {url}"))?
        .error_for_status()
        .with_context(|| format!("request returned error status: {url}"))?;

    response
        .json::<JsonValue>()
        .await
        .with_context(|| format!("failed to parse json: {url}"))
}

pub async fn fetch_public_ip_plain(client: &Client) -> Result<String> {
    match client.get(PUBLIC_IP_PLAIN_SOURCE).send().await {
        Ok(response) if response.status().is_success() => match response.text().await {
            Ok(ip) => {
                let ip = ip.trim();
                if !ip.is_empty() {
                    return Ok(ip.into());
                }
                logging!(
                    warn,
                    Type::Config,
                    "Plain public IP source returned an empty body for {PUBLIC_IP_PLAIN_SOURCE}"
                );
            }
            Err(err) => {
                logging!(
                    warn,
                    Type::Config,
                    "Plain public IP source failed for {PUBLIC_IP_PLAIN_SOURCE}: {err}"
                );
            }
        },
        Ok(response) => {
            logging!(
                warn,
                Type::Config,
                "Plain public IP source returned status {} for {PUBLIC_IP_PLAIN_SOURCE}",
                response.status()
            );
        }
        Err(err) => {
            logging!(
                warn,
                Type::Config,
                "Plain public IP source failed for {PUBLIC_IP_PLAIN_SOURCE}: {err}"
            );
        }
    }

    Err(anyhow!(
        "failed to fetch plain public IP from {PUBLIC_IP_PLAIN_SOURCE}"
    ))
}

pub async fn fetch_public_ipv4_plain(client: &Client) -> Result<String> {
    match client.get(PUBLIC_IPV4_PLAIN_SOURCE).send().await {
        Ok(response) if response.status().is_success() => match response.text().await {
            Ok(ip) => {
                let ip = ip.trim();
                if is_ipv4_address(ip) {
                    return Ok(ip.into());
                }
                logging!(
                    warn,
                    Type::Config,
                    "IPv4 public IP source returned non-IPv4 address {ip} for {PUBLIC_IPV4_PLAIN_SOURCE}"
                );
            }
            Err(err) => {
                logging!(
                    warn,
                    Type::Config,
                    "IPv4 public IP source failed for {PUBLIC_IPV4_PLAIN_SOURCE}: {err}"
                );
            }
        },
        Ok(response) => {
            logging!(
                warn,
                Type::Config,
                "IPv4 public IP source returned status {} for {PUBLIC_IPV4_PLAIN_SOURCE}",
                response.status()
            );
        }
        Err(err) => {
            logging!(
                warn,
                Type::Config,
                "IPv4 public IP source failed for {PUBLIC_IPV4_PLAIN_SOURCE}: {err}"
            );
        }
    }

    Err(anyhow!(
        "failed to fetch IPv4 public IP from {PUBLIC_IPV4_PLAIN_SOURCE}"
    ))
}

async fn lookup_local_ip_info(ip: &str) -> GeoIpInfo {
    match crate::core::ip_reputation::get_ip_reputation_manager()
        .lookup_ip_metadata_record(ip)
        .await
    {
        Ok(record) => GeoIpInfo {
            ip: Some(ip.to_string().into()),
            country_code: record.country_code.map(Into::into),
            country: record.country_name.map(Into::into),
            region: record.region.map(Into::into),
            city: record.city.map(Into::into),
            organization: record.asn_organization.clone().map(Into::into),
            asn: record.asn,
            asn_organization: record.asn_organization.clone().map(Into::into),
            isp: record.asn_organization.map(Into::into),
        },
        Err(error) => {
            logging!(
                warn,
                Type::Config,
                "Local MMDB geo lookup failed for {ip}: {error}"
            );
            GeoIpInfo {
                ip: Some(ip.to_string().into()),
                ..GeoIpInfo::default()
            }
        }
    }
}

pub async fn fetch_public_ip_location(client: &Client) -> Result<GeoIpInfo> {
    let ip = fetch_public_ip_plain(client).await?;
    let mut info = lookup_local_ip_info(&ip).await;

    if info.ip.as_deref().is_some_and(is_ipv4_address) {
        return Ok(info);
    }

    match fetch_public_ipv4_observation(client, Some(info.clone())).await {
        Ok(ipv4_info) => Ok(ipv4_info),
        Err(err) => {
            logging!(
                warn,
                Type::Config,
                "IPv4 public IP observation failed, keeping original public IP: {err}"
            );
            info.ip = Some(ip);
            Ok(info)
        }
    }
}

pub async fn fetch_public_ipv4_observation(client: &Client, base_info: Option<GeoIpInfo>) -> Result<GeoIpInfo> {
    let ip = fetch_public_ipv4_plain(client).await?;
    let mut info = lookup_local_ip_info(&ip).await;

    if !has_location_identity(&info) && !has_asn_metadata(&info) {
        info = base_info.unwrap_or_default();
    }

    info.ip = Some(ip);
    Ok(info)
}

pub async fn fetch_public_ip_observation(client: &Client) -> Result<GeoIpInfo> {
    fetch_public_ip_location(client).await
}

pub async fn fetch_ip_location(_client: &Client, ip: &str) -> GeoIpInfo {
    lookup_local_ip_info(ip).await
}

pub(super) fn build_proxy_detection_location(info: &GeoIpInfo) -> Option<ProxyDetectionLocation> {
    if info.country_code.is_none()
        && info.country.is_none()
        && info.region.is_none()
        && info.city.is_none()
        && info.organization.is_none()
        && info.asn.is_none()
        && info.asn_organization.is_none()
        && info.isp.is_none()
    {
        return None;
    }

    Some(ProxyDetectionLocation {
        country_code: info.country_code.clone(),
        country: info.country.clone(),
        region: info.region.clone(),
        city: info.city.clone(),
        organization: info.organization.clone().or_else(|| info.isp.clone()),
        asn: info.asn,
        asn_organization: info.asn_organization.clone().or_else(|| info.isp.clone()),
    })
}

pub(super) fn has_proxy_detection_location_delta(direct: &GeoIpInfo, proxy: &GeoIpInfo) -> bool {
    if let (Some(direct_country_code), Some(proxy_country_code)) =
        (direct.country_code.as_deref(), proxy.country_code.as_deref())
    {
        return direct_country_code != proxy_country_code;
    }

    if let (Some(direct_country), Some(proxy_country)) = (direct.country.as_deref(), proxy.country.as_deref())
        && direct_country != proxy_country
    {
        return true;
    }

    if let (Some(direct_city), Some(proxy_city)) = (direct.city.as_deref(), proxy.city.as_deref()) {
        return direct_city != proxy_city;
    }

    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn public_ip_source_order_is_stable() {
        assert_eq!(PUBLIC_IP_PLAIN_SOURCE, "https://api.ipify.org");
        assert_eq!(PUBLIC_IPV4_PLAIN_SOURCE, "https://api4.ipify.org");
    }

    fn extract_host(url: &str) -> &str {
        url.trim_start_matches("https://")
            .split('/')
            .next()
            .unwrap_or_default()
    }

    #[test]
    fn public_ip_probe_hosts_match_fetch_sources() {
        let expected_hosts = [PUBLIC_IP_PLAIN_SOURCE, PUBLIC_IPV4_PLAIN_SOURCE]
            .iter()
            .map(|url| extract_host(url))
            .collect::<Vec<_>>();

        assert_eq!(PUBLIC_IP_PROBE_HOSTS.as_slice(), expected_hosts.as_slice());
    }

    #[test]
    fn build_proxy_detection_location_returns_none_when_empty() {
        assert!(build_proxy_detection_location(&GeoIpInfo::default()).is_none());
    }
}
