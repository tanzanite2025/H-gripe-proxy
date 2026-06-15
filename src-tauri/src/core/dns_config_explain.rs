use super::dns_runtime::{plan_dns_server_probe_target, DnsServerProbeTarget};
use anyhow::{anyhow, Context as _, Result};
use serde::Serialize;
use serde_yaml_ng::{Mapping, Value};
use std::collections::BTreeSet;

const DEFAULT_DNS_PROBE_DOMAIN: &str = "www.google.com";
const SERVER_SECTIONS: &[&str] = &[
    "nameserver",
    "fallback",
    "default-nameserver",
    "proxy-server-nameserver",
    "direct-nameserver",
];
const VALID_DNS_MODES: &[&str] = &["normal", "fake-ip", "redir-host", "mapping"];

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DnsConfigExplainReport {
    pub valid: bool,
    pub explanation: String,
    pub enabled: Option<bool>,
    pub enhanced_mode: Option<String>,
    pub fake_ip_range: Option<String>,
    pub server_sections: Vec<DnsConfigServerSection>,
    pub nameserver_policy_count: usize,
    pub fallback_filter_keys: Vec<String>,
    pub probe_plan: DnsConfigProbePlan,
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DnsConfigServerSection {
    pub key: String,
    pub server_count: usize,
    pub probeable_count: usize,
    pub skipped_count: usize,
    pub servers: Vec<DnsConfigServerExplain>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DnsConfigServerExplain {
    pub section: String,
    pub policy_key: Option<String>,
    pub server: String,
    pub probeable: bool,
    pub reason: String,
    pub target: Option<DnsServerProbeTarget>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum DnsConfigProbePlanStatus {
    Ready,
    Skipped,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DnsConfigProbePlan {
    pub status: DnsConfigProbePlanStatus,
    pub reason: String,
    pub test_domain: String,
    pub target_count: usize,
    pub targets: Vec<DnsServerProbeTarget>,
    pub skipped: Vec<DnsConfigProbeSkipped>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DnsConfigProbeSkipped {
    pub section: String,
    pub policy_key: Option<String>,
    pub server: String,
    pub reason: String,
}

pub fn explain_dns_config(
    yaml: &str,
    test_domain: Option<&str>,
) -> Result<DnsConfigExplainReport> {
    let value: Value = serde_yaml_ng::from_str(yaml).context("YAML syntax error")?;
    let root = value
        .as_mapping()
        .ok_or_else(|| anyhow!("config root must be a YAML mapping"))?;
    let dns = dns_mapping(root);
    let test_domain = normalize_test_domain(test_domain);

    let mut errors = Vec::new();
    let mut warnings = Vec::new();
    let Some(dns) = dns else {
        let probe_plan = DnsConfigProbePlan {
            status: DnsConfigProbePlanStatus::Skipped,
            reason: "dns config is missing".into(),
            test_domain,
            target_count: 0,
            targets: Vec::new(),
            skipped: Vec::new(),
        };

        return Ok(DnsConfigExplainReport {
            valid: true,
            explanation: "dns config is missing; no Rust DNS probes can be planned".into(),
            enabled: None,
            enhanced_mode: None,
            fake_ip_range: None,
            server_sections: Vec::new(),
            nameserver_policy_count: 0,
            fallback_filter_keys: Vec::new(),
            probe_plan,
            errors,
            warnings,
        });
    };

    let enabled = optional_bool(dns, "enable", &mut warnings);
    let enhanced_mode = optional_string(dns, "enhanced-mode", &mut warnings);
    let fake_ip_range = optional_string(dns, "fake-ip-range", &mut warnings);
    validate_enhanced_mode(enhanced_mode.as_deref(), &fake_ip_range, &mut warnings);

    let mut server_sections = Vec::new();
    for key in SERVER_SECTIONS {
        server_sections.push(build_server_section(dns, key, &mut errors, &mut warnings));
    }

    let nameserver_policy_count = dns
        .get("nameserver-policy")
        .and_then(Value::as_mapping)
        .map(|mapping| mapping.len())
        .unwrap_or(0);
    let policy_section = build_nameserver_policy_section(dns, &mut errors, &mut warnings);
    server_sections.push(policy_section);

    let fallback_filter_keys = fallback_filter_keys(dns, &mut warnings);
    let probe_plan = build_probe_plan(&server_sections, test_domain);
    warn_for_missing_nameservers(&server_sections, &mut warnings);

    let total_servers = server_sections
        .iter()
        .map(|section| section.server_count)
        .sum::<usize>();
    let probeable_count = probe_plan.target_count;
    let skipped_count = probe_plan.skipped.len();
    let issue_count = errors.len() + warnings.len();
    let explanation = format!(
        "dns config has {total_servers} server reference(s), {probeable_count} probe target(s), {skipped_count} skipped server(s), and {issue_count} issue(s)"
    );

    Ok(DnsConfigExplainReport {
        valid: errors.is_empty(),
        explanation,
        enabled,
        enhanced_mode,
        fake_ip_range,
        server_sections,
        nameserver_policy_count,
        fallback_filter_keys,
        probe_plan,
        errors,
        warnings,
    })
}

pub fn plan_dns_probe(yaml: &str, test_domain: Option<&str>) -> Result<DnsConfigProbePlan> {
    Ok(explain_dns_config(yaml, test_domain)?.probe_plan)
}

fn dns_mapping(root: &Mapping) -> Option<&Mapping> {
    if let Some(dns) = root.get("dns") {
        return dns.as_mapping();
    }

    if root.keys().any(|key| {
        key.as_str()
            .map(|key| SERVER_SECTIONS.contains(&key) || key == "enhanced-mode" || key == "nameserver-policy")
            .unwrap_or(false)
    }) {
        return Some(root);
    }

    None
}

fn normalize_test_domain(test_domain: Option<&str>) -> String {
    test_domain
        .map(str::trim)
        .filter(|domain| !domain.is_empty())
        .unwrap_or(DEFAULT_DNS_PROBE_DOMAIN)
        .into()
}

fn optional_bool(dns: &Mapping, key: &str, warnings: &mut Vec<String>) -> Option<bool> {
    dns.get(key).and_then(|value| match value.as_bool() {
        Some(value) => Some(value),
        None => {
            warnings.push(format!("dns.{key}: expected boolean, got {}", value_type(value)));
            None
        }
    })
}

fn optional_string(dns: &Mapping, key: &str, warnings: &mut Vec<String>) -> Option<String> {
    dns.get(key).and_then(|value| match value.as_str() {
        Some(value) => Some(value.into()),
        None => {
            warnings.push(format!("dns.{key}: expected string, got {}", value_type(value)));
            None
        }
    })
}

fn validate_enhanced_mode(
    mode: Option<&str>,
    fake_ip_range: &Option<String>,
    warnings: &mut Vec<String>,
) {
    let Some(mode) = mode else {
        return;
    };
    let normalized = mode.to_ascii_lowercase();

    if !VALID_DNS_MODES.contains(&normalized.as_str()) {
        warnings.push(format!(
            "dns.enhanced-mode: invalid value `{mode}`, expected one of: {}",
            VALID_DNS_MODES.join(", ")
        ));
    }

    if normalized == "fake-ip" && fake_ip_range.as_deref().unwrap_or_default().is_empty() {
        warnings.push("dns.fake-ip-range: fake-ip mode should define a fake-ip-range".into());
    }
}

fn build_server_section(
    dns: &Mapping,
    key: &str,
    errors: &mut Vec<String>,
    warnings: &mut Vec<String>,
) -> DnsConfigServerSection {
    let servers = extract_server_values(dns.get(key), &format!("dns.{key}"), errors, warnings)
        .into_iter()
        .map(|server| explain_server(key, None, server))
        .collect::<Vec<_>>();

    summarize_server_section(key, servers)
}

fn build_nameserver_policy_section(
    dns: &Mapping,
    errors: &mut Vec<String>,
    warnings: &mut Vec<String>,
) -> DnsConfigServerSection {
    let key = "nameserver-policy";
    let Some(value) = dns.get(key) else {
        return summarize_server_section(key, Vec::new());
    };
    let Some(policy_map) = value.as_mapping() else {
        errors.push(format!("dns.{key}: expected mapping, got {}", value_type(value)));
        return summarize_server_section(key, Vec::new());
    };

    let mut servers = Vec::new();
    for (policy_key, value) in policy_map {
        let policy_key = value_key(policy_key).unwrap_or_else(|| "<non-string-policy>".into());
        let path = format!("dns.{key}.{policy_key}");
        let values = extract_server_values(Some(value), &path, errors, warnings);
        servers.extend(
            values
                .into_iter()
                .map(|server| explain_server(key, Some(policy_key.clone()), server)),
        );
    }

    summarize_server_section(key, servers)
}

fn summarize_server_section(
    key: &str,
    servers: Vec<DnsConfigServerExplain>,
) -> DnsConfigServerSection {
    let probeable_count = servers.iter().filter(|server| server.probeable).count();
    let server_count = servers.len();

    DnsConfigServerSection {
        key: key.into(),
        server_count,
        probeable_count,
        skipped_count: server_count.saturating_sub(probeable_count),
        servers,
    }
}

fn extract_server_values(
    value: Option<&Value>,
    path: &str,
    errors: &mut Vec<String>,
    warnings: &mut Vec<String>,
) -> Vec<String> {
    match value {
        None | Some(Value::Null) => Vec::new(),
        Some(Value::String(server)) => {
            warnings.push(format!("{path}: expected array, treating string as one server"));
            vec![server.trim().into()]
        }
        Some(Value::Sequence(items)) => items
            .iter()
            .enumerate()
            .filter_map(|(index, item)| match item.as_str() {
                Some(server) => Some(server.trim().into()),
                None => {
                    errors.push(format!("{path}[{index}]: expected string, got {}", value_type(item)));
                    None
                }
            })
            .collect(),
        Some(other) => {
            errors.push(format!("{path}: expected array, got {}", value_type(other)));
            Vec::new()
        }
    }
}

fn explain_server(section: &str, policy_key: Option<String>, server: String) -> DnsConfigServerExplain {
    if server.trim().is_empty() {
        return DnsConfigServerExplain {
            section: section.into(),
            policy_key,
            server,
            probeable: false,
            reason: "server is empty".into(),
            target: None,
        };
    }

    match plan_dns_server_probe_target(&server) {
        Ok(target) => DnsConfigServerExplain {
            section: section.into(),
            policy_key,
            server,
            probeable: true,
            reason: "probe target ready".into(),
            target: Some(target),
        },
        Err(err) => DnsConfigServerExplain {
            section: section.into(),
            policy_key,
            server,
            probeable: false,
            reason: err.to_string(),
            target: None,
        },
    }
}

fn fallback_filter_keys(dns: &Mapping, warnings: &mut Vec<String>) -> Vec<String> {
    let Some(value) = dns.get("fallback-filter") else {
        return Vec::new();
    };
    let Some(map) = value.as_mapping() else {
        warnings.push(format!(
            "dns.fallback-filter: expected mapping, got {}",
            value_type(value)
        ));
        return Vec::new();
    };

    map.keys().filter_map(value_key).collect()
}

fn build_probe_plan(
    server_sections: &[DnsConfigServerSection],
    test_domain: String,
) -> DnsConfigProbePlan {
    let mut seen = BTreeSet::new();
    let mut targets = Vec::new();
    let mut skipped = Vec::new();

    for server in server_sections.iter().flat_map(|section| section.servers.iter()) {
        if let Some(target) = &server.target {
            let dedupe_key = format!("{}|{}", target.protocol_name, target.server);
            if seen.insert(dedupe_key) {
                targets.push(target.clone());
            }
        } else if !server.server.is_empty() {
            skipped.push(DnsConfigProbeSkipped {
                section: server.section.clone(),
                policy_key: server.policy_key.clone(),
                server: server.server.clone(),
                reason: server.reason.clone(),
            });
        }
    }

    let target_count = targets.len();
    let (status, reason) = if target_count == 0 {
        (
            DnsConfigProbePlanStatus::Skipped,
            "no probeable DNS server targets".into(),
        )
    } else {
        (
            DnsConfigProbePlanStatus::Ready,
            format!("planned {target_count} DNS server probe(s)"),
        )
    };

    DnsConfigProbePlan {
        status,
        reason,
        test_domain,
        target_count,
        targets,
        skipped,
    }
}

fn warn_for_missing_nameservers(
    server_sections: &[DnsConfigServerSection],
    warnings: &mut Vec<String>,
) {
    let has_nameserver = server_sections
        .iter()
        .find(|section| section.key == "nameserver")
        .map(|section| section.server_count > 0)
        .unwrap_or(false);

    if !has_nameserver {
        warnings.push("dns.nameserver: no primary nameserver configured".into());
    }
}

fn value_key(value: &Value) -> Option<String> {
    value.as_str().map(Into::into)
}

fn value_type(value: &Value) -> &'static str {
    match value {
        Value::Null => "null",
        Value::Bool(_) => "bool",
        Value::Number(_) => "number",
        Value::String(_) => "string",
        Value::Sequence(_) => "array",
        Value::Mapping(_) => "mapping",
        Value::Tagged(_) => "tagged",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn explains_probeable_dns_servers() {
        let yaml = r#"
dns:
  enable: true
  enhanced-mode: fake-ip
  fake-ip-range: 198.18.0.1/16
  nameserver:
    - https://dns.alidns.com/dns-query
  fallback:
    - tls://cloudflare-dns.com:853
"#;

        let report = explain_dns_config(yaml, Some("example.com")).unwrap();

        assert!(report.valid);
        assert_eq!(report.enhanced_mode.as_deref(), Some("fake-ip"));
        assert_eq!(report.probe_plan.status, DnsConfigProbePlanStatus::Ready);
        assert_eq!(report.probe_plan.test_domain, "example.com");
        assert_eq!(report.probe_plan.target_count, 2);
        assert!(report.probe_plan.targets.iter().any(|target| target.protocol_name == "Doh"));
        assert!(report.probe_plan.targets.iter().any(|target| target.protocol_name == "Dot"));
    }

    #[test]
    fn policy_servers_are_included_in_probe_plan() {
        let yaml = r#"
dns:
  nameserver:
    - 223.5.5.5
  nameserver-policy:
    geosite:cn:
      - https://dns.alidns.com/dns-query
    geosite:geolocation-!cn: tls://cloudflare-dns.com:853
"#;

        let report = explain_dns_config(yaml, None).unwrap();

        assert!(report.valid);
        assert_eq!(report.nameserver_policy_count, 2);
        assert_eq!(report.probe_plan.target_count, 3);
        assert!(
            report
                .server_sections
                .iter()
                .find(|section| section.key == "nameserver-policy")
                .unwrap()
                .servers
                .iter()
                .any(|server| server.policy_key.as_deref() == Some("geosite:cn"))
        );
    }

    #[test]
    fn unknown_dns_hostname_is_fail_soft() {
        let yaml = r#"
dns:
  nameserver:
    - https://unknown.example/dns-query
"#;

        let report = explain_dns_config(yaml, None).unwrap();

        assert!(report.valid);
        assert_eq!(report.probe_plan.status, DnsConfigProbePlanStatus::Skipped);
        assert_eq!(report.probe_plan.skipped.len(), 1);
        assert!(report.probe_plan.skipped[0].reason.contains("unsupported DNS hostname"));
    }

    #[test]
    fn missing_dns_config_skips_probe_plan() {
        let report = explain_dns_config("mode: rule\n", None).unwrap();

        assert!(report.valid);
        assert_eq!(report.probe_plan.status, DnsConfigProbePlanStatus::Skipped);
        assert_eq!(report.explanation, "dns config is missing; no Rust DNS probes can be planned");
    }

    #[test]
    fn invalid_server_section_reports_error_without_panic() {
        let yaml = r#"
dns:
  enhanced-mode: invalid
  nameserver: 1
"#;

        let report = explain_dns_config(yaml, None).unwrap();

        assert!(!report.valid);
        assert!(report.errors.iter().any(|error| error.contains("dns.nameserver")));
        assert!(
            report
                .warnings
                .iter()
                .any(|warning| warning.contains("dns.enhanced-mode"))
        );
    }
}
