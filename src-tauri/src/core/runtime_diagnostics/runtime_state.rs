use super::helpers::{
    infer_leak_protection_level, infer_routing_mode, mapping_bool, mapping_nested_len,
    mapping_nested_string_list, mapping_sequence_len, mapping_string,
};
use crate::{
    config::Config,
    constants,
    core::runtime_status::{DnsRuntimeDerivedState, DnsRuntimeSnapshot, DnsRuntimeStatus},
    utils::dirs,
};
use anyhow::Result;
use clash_verge_logging::{Type, logging};
use serde_yaml_ng::Mapping;
use tokio::fs;

pub(super) fn build_dns_runtime_derived_state(runtime_config: Option<&Mapping>) -> DnsRuntimeDerivedState {
    let dns_mapping = runtime_config
        .and_then(|config| config.get("dns"))
        .and_then(|value| value.as_mapping());

    match dns_mapping {
        Some(dns_mapping) => {
            let domestic_dns = mapping_nested_string_list(
                dns_mapping,
                "nameserver-policy",
                "geosite:cn",
            );
            let foreign_dns = mapping_nested_string_list(
                dns_mapping,
                "nameserver-policy",
                "geosite:geolocation-!cn",
            );
            let leak_protection_level = infer_leak_protection_level(dns_mapping);
            let leak_protection_security = match leak_protection_level.as_deref() {
                Some("none") => Some("low".into()),
                Some("basic") => Some("medium".into()),
                Some("strict") => Some("high".into()),
                Some("paranoid") => Some("very-high".into()),
                Some("custom") => Some("custom".into()),
                _ => None,
            };
            let leak_protection_safe = match leak_protection_level.as_deref() {
                Some("none") => Some(false),
                Some("basic") | Some("strict") | Some("paranoid") => Some(true),
                Some("custom") | None => None,
                Some(_) => None,
            };

            DnsRuntimeDerivedState {
                routing_mode: infer_routing_mode(&domestic_dns, &foreign_dns),
                domestic_dns,
                foreign_dns,
                default_nameserver_count: mapping_sequence_len(dns_mapping, "default-nameserver"),
                prefer_h3: mapping_bool(dns_mapping, "prefer-h3"),
                leak_protection_level,
                leak_protection_security,
                leak_protection_safe,
            }
        }
        None => DnsRuntimeDerivedState {
            routing_mode: None,
            domestic_dns: Vec::new(),
            foreign_dns: Vec::new(),
            default_nameserver_count: 0,
            prefer_h3: None,
            leak_protection_level: None,
            leak_protection_security: None,
            leak_protection_safe: None,
        },
    }
}

pub(super) fn build_dns_runtime_snapshot(runtime_config: Option<&Mapping>) -> DnsRuntimeSnapshot {
    let dns_mapping = runtime_config
        .and_then(|config| config.get("dns"))
        .and_then(|value| value.as_mapping());

    match dns_mapping {
        Some(dns_mapping) => DnsRuntimeSnapshot {
            enhanced_mode: mapping_string(dns_mapping, "enhanced-mode"),
            ipv6: mapping_bool(dns_mapping, "ipv6"),
            nameserver_count: mapping_sequence_len(dns_mapping, "nameserver"),
            fallback_count: mapping_sequence_len(dns_mapping, "fallback"),
            nameserver_policy_count: mapping_nested_len(dns_mapping, "nameserver-policy"),
            use_hosts: mapping_bool(dns_mapping, "use-hosts"),
            use_system_hosts: mapping_bool(dns_mapping, "use-system-hosts"),
            respect_rules: mapping_bool(dns_mapping, "respect-rules"),
        },
        None => DnsRuntimeSnapshot {
            enhanced_mode: None,
            ipv6: None,
            nameserver_count: 0,
            fallback_count: 0,
            nameserver_policy_count: 0,
            use_hosts: None,
            use_system_hosts: None,
            respect_rules: None,
        },
    }
}

pub async fn build_dns_runtime_status() -> Result<DnsRuntimeStatus> {
    let runtime = Config::runtime().await;
    let runtime = runtime.latest_arc();
    let runtime_config = runtime.config.as_ref();

    let enable_dns_settings = Config::verge()
        .await
        .latest_arc()
        .enable_dns_settings
        .unwrap_or(false);

    let dns_path = dirs::app_home_dir()?.join(constants::files::DNS_CONFIG);
    let dns_config_exists = dns_path.exists();

    let mut dns_config_valid = false;
    let mut saved_dns_mapping: Option<Mapping> = None;
    let mut saved_hosts_mapping: Option<Mapping> = None;

    if dns_config_exists {
        match fs::read_to_string(&dns_path).await {
            Ok(dns_yaml) => match serde_yaml_ng::from_str::<Mapping>(&dns_yaml) {
                Ok(saved_mapping) => {
                    dns_config_valid = true;
                    saved_dns_mapping = saved_mapping
                        .get("dns")
                        .and_then(|value| value.as_mapping())
                        .cloned();
                    saved_hosts_mapping = saved_mapping
                        .get("hosts")
                        .and_then(|value| value.as_mapping())
                        .cloned();
                }
                Err(err) => {
                    logging!(warn, Type::Config, "Failed to parse DNS runtime artifact: {err}");
                }
            },
            Err(err) => {
                logging!(warn, Type::Config, "Failed to read DNS runtime artifact: {err}");
            }
        }
    }

    let runtime_dns_mapping = runtime_config
        .and_then(|config| config.get("dns"))
        .and_then(|value| value.as_mapping())
        .cloned();
    let runtime_hosts_mapping = runtime_config
        .and_then(|config| config.get("hosts"))
        .and_then(|value| value.as_mapping())
        .cloned();

    let runtime_has_dns = runtime_dns_mapping.is_some();
    let runtime_has_hosts = runtime_hosts_mapping.is_some();

    let runtime_dns_matches_saved = saved_dns_mapping
        .as_ref()
        .map(|saved| runtime_dns_mapping.as_ref() == Some(saved))
        .unwrap_or(false);
    let runtime_hosts_matches_saved = saved_hosts_mapping
        .as_ref()
        .map(|saved| runtime_hosts_mapping.as_ref() == Some(saved))
        .unwrap_or(false);

    Ok(DnsRuntimeStatus {
        enable_dns_settings,
        dns_config_exists,
        dns_config_valid,
        runtime_has_dns,
        runtime_has_hosts,
        runtime_dns_matches_saved,
        runtime_hosts_matches_saved,
        runtime_matches_saved: dns_config_valid && runtime_dns_matches_saved && runtime_hosts_matches_saved,
        snapshot: build_dns_runtime_snapshot(runtime_config),
        derived: build_dns_runtime_derived_state(runtime_config),
    })
}
