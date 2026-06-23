use super::constants::DEFAULT_FAKE_IP_RANGE;
use anyhow::{Context as _, Result, anyhow};
use serde_yaml_ng::{Mapping, Value};
use smartstring::alias::String;
use std::collections::BTreeMap;

#[derive(Debug, Clone)]
pub(super) struct DnsPolicyBundleConfig {
    pub(super) fake_ip_range: String,
    pub(super) fake_ip_filters: Vec<String>,
    pub(super) fallback_upstreams: Vec<String>,
    pub(super) fallback_filter_domains: Vec<String>,
    pub(super) fallback_filter_ipcidrs: Vec<String>,
    pub(super) nameserver_policy_entries: Vec<NameserverPolicyConfigEntry>,
    pub(super) rule_provider_payloads: BTreeMap<String, Vec<String>>,
}

#[derive(Debug, Clone)]
pub(super) struct NameserverPolicyConfigEntry {
    pub(super) matcher: String,
    pub(super) nameservers: Vec<String>,
}

pub(super) fn parse_dns_policy_bundle_config(yaml: &str) -> Result<DnsPolicyBundleConfig> {
    let value: Value = serde_yaml_ng::from_str(yaml).context("YAML syntax error")?;
    let root = value
        .as_mapping()
        .ok_or_else(|| anyhow!("config root must be a YAML mapping"))?;
    let dns = root
        .get("dns")
        .and_then(Value::as_mapping)
        .ok_or_else(|| anyhow!("dns config is missing"))?;
    let fallback_filter = dns
        .get("fallback-filter")
        .and_then(Value::as_mapping)
        .cloned()
        .unwrap_or_default();

    Ok(DnsPolicyBundleConfig {
        fake_ip_range: dns
            .get("fake-ip-range")
            .and_then(Value::as_str)
            .unwrap_or(DEFAULT_FAKE_IP_RANGE)
            .trim()
            .into(),
        fake_ip_filters: values_as_strings(dns.get("fake-ip-filter")),
        fallback_upstreams: values_as_strings(dns.get("fallback")),
        fallback_filter_domains: values_as_strings(fallback_filter.get("domain")),
        fallback_filter_ipcidrs: values_as_strings(fallback_filter.get("ipcidr")),
        nameserver_policy_entries: parse_nameserver_policy(dns.get("nameserver-policy")),
        rule_provider_payloads: parse_rule_provider_payloads(root.get("rule-providers")),
    })
}

fn parse_nameserver_policy(value: Option<&Value>) -> Vec<NameserverPolicyConfigEntry> {
    value
        .and_then(Value::as_mapping)
        .map(|mapping| {
            mapping
                .iter()
                .filter_map(|(key, value)| {
                    let matcher = key.as_str()?.trim();
                    if matcher.is_empty() {
                        return None;
                    }
                    let nameservers = values_as_strings(Some(value));
                    if nameservers.is_empty() {
                        return None;
                    }
                    Some(NameserverPolicyConfigEntry {
                        matcher: matcher.into(),
                        nameservers,
                    })
                })
                .collect()
        })
        .unwrap_or_default()
}

fn parse_rule_provider_payloads(value: Option<&Value>) -> BTreeMap<String, Vec<String>> {
    value
        .and_then(Value::as_mapping)
        .map(|mapping| {
            mapping
                .iter()
                .filter_map(|(name, provider)| {
                    let name = name.as_str()?.trim();
                    let provider = provider.as_mapping()?;
                    let payload = values_as_strings(provider.get("payload"));
                    if name.is_empty() || payload.is_empty() {
                        return None;
                    }
                    Some((name.into(), payload))
                })
                .collect()
        })
        .unwrap_or_default()
}

fn values_as_strings(value: Option<&Value>) -> Vec<String> {
    match value {
        Some(Value::Sequence(values)) => values
            .iter()
            .filter_map(Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(Into::into)
            .collect(),
        Some(Value::String(value)) => {
            let value = value.trim();
            if value.is_empty() {
                Vec::new()
            } else {
                vec![value.into()]
            }
        }
        _ => Vec::new(),
    }
}

trait MappingExt {
    fn get(&self, key: &str) -> Option<&Value>;
}

impl MappingExt for Mapping {
    fn get(&self, key: &str) -> Option<&Value> {
        self.get(Value::String(key.into()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_bundle_dns_fields() {
        let config = parse_dns_policy_bundle_config(
            r#"
dns:
  fake-ip-range: 198.18.0.1/16
  fake-ip-filter:
    - "*.lan"
  fallback:
    - udp://127.0.0.1:5353
  fallback-filter:
    domain:
      - "+.example.com"
  nameserver-policy:
    "geosite:cn":
      - 223.5.5.5
rule-providers:
  private:
    payload:
      - DOMAIN-SUFFIX,internal.test
"#,
        )
        .unwrap();

        assert_eq!(config.fake_ip_filters, vec!["*.lan"]);
        assert_eq!(config.fallback_upstreams, vec!["udp://127.0.0.1:5353"]);
        assert_eq!(config.nameserver_policy_entries.len(), 1);
        assert!(config.rule_provider_payloads.contains_key("private"));
    }
}
