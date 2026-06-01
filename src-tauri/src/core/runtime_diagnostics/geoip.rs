use anyhow::{Context as _, Result, anyhow};
use clash_verge_logging::{Type, logging};
use crate::core::runtime_status::ProxyDetectionLocation;
use reqwest::Client;
use serde_json::Value as JsonValue;
use smartstring::alias::String;

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

fn json_string(value: &JsonValue, key: &str) -> Option<String> {
    value.get(key).and_then(|item| item.as_str()).map(Into::into)
}

fn json_nested_string(value: &JsonValue, key: &str, nested_key: &str) -> Option<String> {
    value
        .get(key)
        .and_then(|item| item.get(nested_key))
        .and_then(|item| item.as_str())
        .map(Into::into)
}

fn parse_json_u32(value: &JsonValue) -> Option<u32> {
    value
        .as_u64()
        .map(|item| item as u32)
        .or_else(|| {
            value
                .as_str()
                .and_then(|item| item.trim_start_matches("AS").parse::<u32>().ok())
        })
}

fn json_u32(value: &JsonValue, key: &str) -> Option<u32> {
    value.get(key).and_then(parse_json_u32)
}

fn json_nested_u32(value: &JsonValue, key: &str, nested_key: &str) -> Option<u32> {
    value
        .get(key)
        .and_then(|item| item.get(nested_key))
        .and_then(parse_json_u32)
}

fn parse_geo_ip_info(data: &JsonValue) -> GeoIpInfo {
    GeoIpInfo {
        ip: json_string(data, "ip"),
        country_code: json_nested_string(data, "data", "country_code")
            .or_else(|| json_nested_string(data, "location", "country_code"))
            .or_else(|| json_nested_string(data, "adcode", "country"))
            .or_else(|| json_string(data, "country_code")),
        country: json_nested_string(data, "data", "country")
            .or_else(|| json_string(data, "country_name"))
            .or_else(|| json_nested_string(data, "location", "country"))
            .or_else(|| json_string(data, "country")),
        region: json_nested_string(data, "data", "province")
            .or_else(|| json_nested_string(data, "location", "state"))
            .or_else(|| json_string(data, "region")),
        city: json_nested_string(data, "data", "city")
            .or_else(|| json_nested_string(data, "location", "city"))
            .or_else(|| json_string(data, "city")),
        organization: json_nested_string(data, "company", "name")
            .or_else(|| json_nested_string(data, "connection", "org"))
            .or_else(|| json_string(data, "organization"))
            .or_else(|| json_string(data, "org"))
            .or_else(|| json_string(data, "isp")),
        asn: json_nested_u32(data, "asn", "asn")
            .or_else(|| json_nested_u32(data, "connection", "asn"))
            .or_else(|| json_u32(data, "asn")),
        asn_organization: json_nested_string(data, "asn", "org")
            .or_else(|| json_string(data, "asn_organization"))
            .or_else(|| json_nested_string(data, "connection", "isp"))
            .or_else(|| json_string(data, "org"))
            .or_else(|| json_string(data, "isp")),
        isp: json_nested_string(data, "data", "isp")
            .or_else(|| json_nested_string(data, "connection", "isp"))
            .or_else(|| json_nested_string(data, "asn", "org"))
            .or_else(|| json_string(data, "organization"))
            .or_else(|| json_string(data, "org")),
    }
}

fn has_location_identity(info: &GeoIpInfo) -> bool {
    info.country.is_some() || info.country_code.is_some() || info.ip.is_some()
}

fn has_asn_metadata(info: &GeoIpInfo) -> bool {
    info.asn.is_some() || info.asn_organization.is_some() || info.organization.is_some() || info.isp.is_some()
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

pub(super) async fn fetch_public_ip_location(client: &Client) -> Result<GeoIpInfo> {
    for url in [
        "https://api.ip.sb/geoip",
        "https://ipapi.co/json",
        "https://ipwho.is/",
    ] {
        match fetch_json(client, url).await {
            Ok(data) => {
                let info = parse_geo_ip_info(&data);
                if info.country.is_some() || info.ip.is_some() {
                    return Ok(info);
                }
            }
            Err(err) => {
                logging!(warn, Type::Config, "DNS leak IP source failed for {url}: {err}");
            }
        }
    }

    Err(anyhow!("failed to fetch public IP location"))
}

pub async fn fetch_ip_location(client: &Client, ip: &str) -> GeoIpInfo {
    let mut fallback: Option<GeoIpInfo> = None;

    for url in [
        format!("https://ipapi.co/{ip}/json/"),
        format!("https://ipwho.is/{ip}"),
    ] {
        match fetch_json(client, &url).await {
            Ok(data) => {
                let info = parse_geo_ip_info(&data);
                if has_location_identity(&info) && has_asn_metadata(&info) {
                    return info;
                }
                if has_location_identity(&info) {
                    fallback.get_or_insert(info);
                }
            }
            Err(err) => {
                logging!(warn, Type::Config, "DNS leak geo lookup failed for {ip} via {url}: {err}");
            }
        }
    }

    fallback.unwrap_or_default()
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

    if let (Some(direct_country), Some(proxy_country)) =
        (direct.country.as_deref(), proxy.country.as_deref())
    {
        if direct_country != proxy_country {
            return true;
        }
    }

    if let (Some(direct_city), Some(proxy_city)) = (direct.city.as_deref(), proxy.city.as_deref()) {
        return direct_city != proxy_city;
    }

    false
}
