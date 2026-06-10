use serde_yaml_ng::Mapping;
use smartstring::alias::String;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::config::{
    DOMESTIC_DOH_NAMESERVERS, DOMESTIC_PLAIN_NAMESERVERS, FOREIGN_DOH_NAMESERVERS,
};

pub(super) fn mapping_bool(mapping: &Mapping, key: &str) -> Option<bool> {
    mapping.get(key).and_then(|value| value.as_bool())
}

pub(super) fn mapping_string(mapping: &Mapping, key: &str) -> Option<String> {
    mapping.get(key).and_then(|value| value.as_str()).map(Into::into)
}

pub(super) fn mapping_sequence_len(mapping: &Mapping, key: &str) -> usize {
    mapping
        .get(key)
        .and_then(|value| value.as_sequence())
        .map(|items| items.len())
        .unwrap_or(0)
}

fn is_encrypted_dns_endpoint(value: &str) -> bool {
    let normalized = value.trim().to_ascii_lowercase();
    normalized.starts_with("https://")
        || normalized.starts_with("tls://")
        || normalized.starts_with("quic://")
        || normalized.starts_with("h3://")
}

pub(super) fn mapping_plain_dns_sequence_len(mapping: &Mapping, key: &str) -> usize {
    mapping
        .get(key)
        .and_then(|value| value.as_sequence())
        .map(|items| {
            items
                .iter()
                .filter_map(|item| item.as_str())
                .filter(|item| !is_encrypted_dns_endpoint(item))
                .count()
        })
        .unwrap_or(0)
}

pub(super) fn mapping_nested_string_list(mapping: &Mapping, key: &str, nested_key: &str) -> Vec<String> {
    mapping
        .get(key)
        .and_then(|value| value.as_mapping())
        .and_then(|nested| nested.get(nested_key))
        .and_then(|value| value.as_sequence())
        .map(|items| items.iter().filter_map(|item| item.as_str().map(Into::into)).collect())
        .unwrap_or_default()
}

pub(super) fn same_string_list(values: &[String], expected: &[&str]) -> bool {
    values.len() == expected.len()
        && values
            .iter()
            .zip(expected.iter())
            .all(|(value, expected)| value.as_str() == *expected)
}

pub(super) fn infer_routing_mode(domestic_dns: &[String], foreign_dns: &[String]) -> Option<String> {
    if domestic_dns.is_empty() && foreign_dns.is_empty() {
        return None;
    }

    if same_string_list(domestic_dns, FOREIGN_DOH_NAMESERVERS)
        && same_string_list(foreign_dns, FOREIGN_DOH_NAMESERVERS)
    {
        return Some("privacy".into());
    }

    if (same_string_list(domestic_dns, DOMESTIC_PLAIN_NAMESERVERS)
        && same_string_list(foreign_dns, DOMESTIC_PLAIN_NAMESERVERS))
        || (same_string_list(domestic_dns, DOMESTIC_DOH_NAMESERVERS)
            && same_string_list(foreign_dns, DOMESTIC_DOH_NAMESERVERS))
    {
        return Some("speed".into());
    }

    if (same_string_list(domestic_dns, DOMESTIC_PLAIN_NAMESERVERS)
        && same_string_list(foreign_dns, FOREIGN_DOH_NAMESERVERS))
        || (same_string_list(domestic_dns, DOMESTIC_DOH_NAMESERVERS)
            && same_string_list(foreign_dns, FOREIGN_DOH_NAMESERVERS))
    {
        return Some("balanced".into());
    }

    Some("custom".into())
}

pub(super) fn infer_leak_protection_level(dns_mapping: &Mapping) -> Option<String> {
    let enhanced_mode = mapping_string(dns_mapping, "enhanced-mode");
    let ipv6 = mapping_bool(dns_mapping, "ipv6").unwrap_or(true);
    let default_nameserver_plain_count = mapping_plain_dns_sequence_len(dns_mapping, "default-nameserver");
    let has_fake_ip_range = dns_mapping.get("fake-ip-range").is_some();
    let all_nameservers_encrypted = mapping_plain_dns_sequence_len(dns_mapping, "nameserver") == 0
        && mapping_plain_dns_sequence_len(dns_mapping, "fallback") == 0
        && mapping_plain_dns_sequence_len(dns_mapping, "proxy-server-nameserver") == 0;

    match enhanced_mode.as_deref() {
        Some("fake-ip")
            if all_nameservers_encrypted && default_nameserver_plain_count == 0 && !ipv6 && has_fake_ip_range =>
        {
            Some("paranoid".into())
        }
        Some("fake-ip") if all_nameservers_encrypted && default_nameserver_plain_count == 0 && has_fake_ip_range => {
            Some("strict".into())
        }
        Some("redir-host") if all_nameservers_encrypted => Some("basic".into()),
        Some("redir-host") => Some("none".into()),
        Some(_) => Some("custom".into()),
        None => None,
    }
}

pub(super) fn mapping_nested_len(mapping: &Mapping, key: &str) -> usize {
    mapping
        .get(key)
        .and_then(|value| value.as_mapping())
        .map(|items| items.len())
        .unwrap_or(0)
}

pub(super) fn current_timestamp_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis() as u64)
        .unwrap_or(0)
}
