use anyhow::{Context as _, Result};
use serde_yaml_ng::{Mapping, Value};
use smartstring::alias::String;
use std::net::IpAddr;

#[derive(Debug)]
pub struct NativeValidationReport {
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
}

impl NativeValidationReport {
    pub fn is_valid(&self) -> bool {
        self.errors.is_empty()
    }

    pub fn error_summary(&self) -> String {
        self.errors.join("; ").into()
    }
}

const VALID_MODES: &[&str] = &["rule", "global", "direct"];
const VALID_LOG_LEVELS: &[&str] = &["debug", "info", "warning", "error", "silent"];
const VALID_TUN_STACKS: &[&str] = &["system", "gvisor", "mixed", "lwip"];
const VALID_DNS_MODES: &[&str] = &["normal", "fake-ip", "redir-host", "mapping"];
const VALID_FIND_PROCESS_MODES: &[&str] = &["strict", "off", "always"];
const VALID_PROXY_TYPES: &[&str] = &[
    "ss",
    "ssr",
    "vmess",
    "vless",
    "trojan",
    "hysteria",
    "hysteria2",
    "tuic",
    "wireguard",
    "socks5",
    "http",
    "snell",
    "ssh",
    "direct",
    "dns",
    "reject",
    "reject-drop",
    "compatible",
    "pass",
];
const VALID_GROUP_TYPES: &[&str] = &["select", "url-test", "fallback", "load-balance", "relay"];
const VALID_CACHE_ALGORITHMS: &[&str] = &["lru", "arc"];

pub async fn validate_native(config_path: &str) -> Result<NativeValidationReport> {
    let content = tokio::fs::read_to_string(config_path)
        .await
        .with_context(|| format!("Failed to read config file: {config_path}"))?;

    let mapping: Mapping = serde_yaml_ng::from_str(&content).with_context(|| "YAML syntax error")?;

    let mut report = NativeValidationReport {
        errors: Vec::new(),
        warnings: Vec::new(),
    };

    validate_ports(&mapping, &mut report);
    validate_mode(&mapping, &mut report);
    validate_log_level(&mapping, &mut report);
    validate_bind_address(&mapping, &mut report);
    validate_proxies(&mapping, &mut report);
    validate_proxy_groups(&mapping, &mut report);
    validate_rules(&mapping, &mut report);
    validate_dns(&mapping, &mut report);
    validate_tun(&mapping, &mut report);
    validate_find_process_mode(&mapping, &mut report);
    validate_sniffer(&mapping, &mut report);
    validate_external_controller(&mapping, &mut report);

    Ok(report)
}

fn validate_ports(map: &Mapping, report: &mut NativeValidationReport) {
    for key in &["port", "socks-port", "mixed-port"] {
        if let Some(val) = map.get(*key) {
            match val.as_i64() {
                Some(p) if (0..=65535).contains(&p) => {}
                Some(p) => report
                    .errors
                    .push(format!("{key}: port {p} out of range 0-65535").into()),
                None => report
                    .errors
                    .push(format!("{key}: expected integer, got {}", type_name(val)).into()),
            }
        }
    }
}

fn validate_mode(map: &Mapping, report: &mut NativeValidationReport) {
    if let Some(val) = map.get("mode") {
        if let Some(s) = val.as_str() {
            if !VALID_MODES.contains(&s.to_ascii_lowercase().as_str()) {
                report.errors.push(
                    format!(
                        "mode: invalid value \"{s}\", expected one of: {}",
                        VALID_MODES.join(", ")
                    )
                    .into(),
                );
            }
        } else {
            report
                .errors
                .push(format!("mode: expected string, got {}", type_name(val)).into());
        }
    }
}

fn validate_log_level(map: &Mapping, report: &mut NativeValidationReport) {
    if let Some(val) = map.get("log-level") {
        if let Some(s) = val.as_str() {
            if !VALID_LOG_LEVELS.contains(&s.to_ascii_lowercase().as_str()) {
                report.errors.push(
                    format!(
                        "log-level: invalid value \"{s}\", expected one of: {}",
                        VALID_LOG_LEVELS.join(", ")
                    )
                    .into(),
                );
            }
        } else {
            report
                .errors
                .push(format!("log-level: expected string, got {}", type_name(val)).into());
        }
    }
}

fn validate_bind_address(map: &Mapping, report: &mut NativeValidationReport) {
    if let Some(val) = map.get("bind-address") {
        if let Some(s) = val.as_str() {
            if s != "*" && s.parse::<IpAddr>().is_err() {
                report
                    .errors
                    .push(format!("bind-address: \"{s}\" is not a valid IP address or \"*\"").into());
            }
        }
    }
}

fn validate_proxies(map: &Mapping, report: &mut NativeValidationReport) {
    let Some(proxies) = map.get("proxies") else { return };
    let Some(seq) = proxies.as_sequence() else {
        if !proxies.is_null() {
            report.errors.push("proxies: expected array".into());
        }
        return;
    };

    for (i, item) in seq.iter().enumerate() {
        let Some(proxy) = item.as_mapping() else {
            report.errors.push(format!("proxies[{i}]: expected mapping").into());
            continue;
        };

        if proxy.get("name").and_then(|v| v.as_str()).is_none() {
            report
                .errors
                .push(format!("proxies[{i}]: missing required field \"name\"").into());
        }

        let proxy_type = proxy.get("type").and_then(|v| v.as_str());
        match proxy_type {
            Some(t) => {
                if !VALID_PROXY_TYPES.contains(&t.to_ascii_lowercase().as_str()) {
                    let name = proxy.get("name").and_then(|v| v.as_str()).unwrap_or("?");
                    report
                        .warnings
                        .push(format!("proxies[{i}] \"{name}\": unknown type \"{t}\"").into());
                }
            }
            None => {
                report
                    .errors
                    .push(format!("proxies[{i}]: missing required field \"type\"").into());
            }
        }

        let needs_server = proxy_type.is_some_and(|t| {
            let lower = t.to_ascii_lowercase();
            !matches!(
                lower.as_str(),
                "direct" | "dns" | "reject" | "reject-drop" | "compatible" | "pass"
            )
        });

        if needs_server {
            if proxy.get("server").and_then(|v| v.as_str()).is_none() {
                let name = proxy.get("name").and_then(|v| v.as_str()).unwrap_or("?");
                report
                    .errors
                    .push(format!("proxies[{i}] \"{name}\": missing required field \"server\"").into());
            }
            if proxy.get("port").is_none() {
                let name = proxy.get("name").and_then(|v| v.as_str()).unwrap_or("?");
                report
                    .errors
                    .push(format!("proxies[{i}] \"{name}\": missing required field \"port\"").into());
            }
        }
    }
}

fn validate_proxy_groups(map: &Mapping, report: &mut NativeValidationReport) {
    let Some(groups) = map.get("proxy-groups") else { return };
    let Some(seq) = groups.as_sequence() else {
        if !groups.is_null() {
            report.errors.push("proxy-groups: expected array".into());
        }
        return;
    };

    for (i, item) in seq.iter().enumerate() {
        let Some(group) = item.as_mapping() else {
            report
                .errors
                .push(format!("proxy-groups[{i}]: expected mapping").into());
            continue;
        };

        if group.get("name").and_then(|v| v.as_str()).is_none() {
            report
                .errors
                .push(format!("proxy-groups[{i}]: missing required field \"name\"").into());
        }

        if let Some(t) = group.get("type").and_then(|v| v.as_str()) {
            if !VALID_GROUP_TYPES.contains(&t.to_ascii_lowercase().as_str()) {
                let name = group.get("name").and_then(|v| v.as_str()).unwrap_or("?");
                report.errors.push(
                    format!(
                        "proxy-groups[{i}] \"{name}\": invalid type \"{t}\", expected one of: {}",
                        VALID_GROUP_TYPES.join(", ")
                    )
                    .into(),
                );
            }
        } else {
            report
                .errors
                .push(format!("proxy-groups[{i}]: missing required field \"type\"").into());
        }

        let has_proxies = group
            .get("proxies")
            .and_then(|v| v.as_sequence())
            .is_some_and(|s| !s.is_empty());
        let has_use = group
            .get("use")
            .and_then(|v| v.as_sequence())
            .is_some_and(|s| !s.is_empty());
        let has_include = group.get("include-all").and_then(|v| v.as_bool()).unwrap_or(false)
            || group
                .get("include-all-proxies")
                .and_then(|v| v.as_bool())
                .unwrap_or(false)
            || group
                .get("include-all-providers")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);

        if !has_proxies && !has_use && !has_include {
            let name = group.get("name").and_then(|v| v.as_str()).unwrap_or("?");
            report
                .warnings
                .push(format!("proxy-groups[{i}] \"{name}\": no proxies, use, or include-all specified").into());
        }
    }
}

fn validate_rules(map: &Mapping, report: &mut NativeValidationReport) {
    let Some(rules) = map.get("rules") else { return };
    let Some(seq) = rules.as_sequence() else {
        if !rules.is_null() {
            report.errors.push("rules: expected array".into());
        }
        return;
    };

    for (i, item) in seq.iter().enumerate() {
        let Some(rule_str) = item.as_str() else {
            report.errors.push(format!("rules[{i}]: expected string").into());
            continue;
        };

        let validation = super::rule_engine::validate_rule(rule_str);
        if !validation.valid {
            if let Some(err) = validation.error {
                report.errors.push(format!("rules[{i}]: {err}").into());
            }
        }
    }
}

fn validate_dns(map: &Mapping, report: &mut NativeValidationReport) {
    let Some(dns) = map.get("dns") else { return };
    let Some(dns_map) = dns.as_mapping() else {
        report.errors.push("dns: expected mapping".into());
        return;
    };

    if let Some(mode) = dns_map.get("enhanced-mode") {
        if let Some(s) = mode.as_str() {
            if !VALID_DNS_MODES.contains(&s.to_ascii_lowercase().as_str()) {
                report.errors.push(
                    format!(
                        "dns.enhanced-mode: invalid value \"{s}\", expected one of: {}",
                        VALID_DNS_MODES.join(", ")
                    )
                    .into(),
                );
            }
        }
    }

    if let Some(listen) = dns_map.get("listen") {
        if let Some(s) = listen.as_str() {
            if !s.is_empty() && !s.contains(':') {
                report
                    .warnings
                    .push(format!("dns.listen: \"{s}\" should be in host:port format").into());
            }
        }
    }

    validate_dns_nameserver_array(dns_map, "nameserver", report);
    validate_dns_nameserver_array(dns_map, "fallback", report);
    validate_dns_nameserver_array(dns_map, "default-nameserver", report);

    if let Some(range) = dns_map.get("fake-ip-range") {
        if let Some(s) = range.as_str() {
            if !s.contains('/') {
                report
                    .errors
                    .push(format!("dns.fake-ip-range: \"{s}\" should be in CIDR notation (e.g. 198.18.0.1/16)").into());
            }
        }
    }

    if let Some(algo) = dns_map.get("cache-algorithm") {
        if let Some(s) = algo.as_str() {
            if !VALID_CACHE_ALGORITHMS.contains(&s.to_ascii_lowercase().as_str()) {
                report.errors.push(
                    format!(
                        "dns.cache-algorithm: invalid value \"{s}\", expected one of: {}",
                        VALID_CACHE_ALGORITHMS.join(", ")
                    )
                    .into(),
                );
            }
        }
    }
}

fn validate_dns_nameserver_array(dns_map: &Mapping, key: &str, report: &mut NativeValidationReport) {
    if let Some(val) = dns_map.get(key) {
        if let Some(seq) = val.as_sequence() {
            for (i, item) in seq.iter().enumerate() {
                if item.as_str().is_none() {
                    report.errors.push(format!("dns.{key}[{i}]: expected string").into());
                }
            }
        } else if !val.is_null() {
            report.errors.push(format!("dns.{key}: expected array").into());
        }
    }
}

fn validate_tun(map: &Mapping, report: &mut NativeValidationReport) {
    let Some(tun) = map.get("tun") else { return };
    let Some(tun_map) = tun.as_mapping() else {
        report.errors.push("tun: expected mapping".into());
        return;
    };

    if let Some(stack) = tun_map.get("stack") {
        if let Some(s) = stack.as_str() {
            if !VALID_TUN_STACKS.contains(&s.to_ascii_lowercase().as_str()) {
                report.errors.push(
                    format!(
                        "tun.stack: invalid value \"{s}\", expected one of: {}",
                        VALID_TUN_STACKS.join(", ")
                    )
                    .into(),
                );
            }
        }
    }

    if let Some(mtu) = tun_map.get("mtu") {
        if let Some(n) = mtu.as_u64() {
            if n == 0 || n > 65535 {
                report
                    .errors
                    .push(format!("tun.mtu: {n} out of valid range 1-65535").into());
            }
        } else if !mtu.is_null() {
            report.errors.push("tun.mtu: expected integer".into());
        }
    }
}

fn validate_find_process_mode(map: &Mapping, report: &mut NativeValidationReport) {
    if let Some(val) = map.get("find-process-mode") {
        if let Some(s) = val.as_str() {
            if !VALID_FIND_PROCESS_MODES.contains(&s.to_ascii_lowercase().as_str()) {
                report.errors.push(
                    format!(
                        "find-process-mode: invalid value \"{s}\", expected one of: {}",
                        VALID_FIND_PROCESS_MODES.join(", ")
                    )
                    .into(),
                );
            }
        }
    }
}

fn validate_sniffer(map: &Mapping, report: &mut NativeValidationReport) {
    let Some(sniffer) = map.get("sniffer") else { return };
    let Some(sniffer_map) = sniffer.as_mapping() else {
        report.errors.push("sniffer: expected mapping".into());
        return;
    };

    if let Some(sniff) = sniffer_map.get("sniff") {
        if !sniff.is_mapping() && !sniff.is_null() {
            report.errors.push("sniffer.sniff: expected mapping".into());
        }
    }
}

fn validate_external_controller(map: &Mapping, report: &mut NativeValidationReport) {
    if let Some(val) = map.get("external-controller") {
        if let Some(s) = val.as_str() {
            if !s.is_empty() && !s.contains(':') {
                report
                    .warnings
                    .push(format!("external-controller: \"{s}\" should be in host:port format").into());
            }
        }
    }
}

fn type_name(val: &Value) -> &'static str {
    match val {
        Value::Null => "null",
        Value::Bool(_) => "boolean",
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

    fn make_report(yaml: &str) -> NativeValidationReport {
        let mapping: Mapping = serde_yaml_ng::from_str(yaml).unwrap();
        let mut report = NativeValidationReport {
            errors: Vec::new(),
            warnings: Vec::new(),
        };
        validate_ports(&mapping, &mut report);
        validate_mode(&mapping, &mut report);
        validate_log_level(&mapping, &mut report);
        validate_bind_address(&mapping, &mut report);
        validate_proxies(&mapping, &mut report);
        validate_proxy_groups(&mapping, &mut report);
        validate_rules(&mapping, &mut report);
        validate_dns(&mapping, &mut report);
        validate_tun(&mapping, &mut report);
        validate_find_process_mode(&mapping, &mut report);
        validate_sniffer(&mapping, &mut report);
        validate_external_controller(&mapping, &mut report);
        report
    }

    #[test]
    fn valid_minimal_config() {
        let r = make_report("mixed-port: 7890\nmode: rule\nlog-level: info\n");
        assert!(r.is_valid(), "errors: {:?}", r.errors);
    }

    #[test]
    fn rejects_invalid_port() {
        let r = make_report("mixed-port: 99999\n");
        assert!(!r.is_valid());
        assert!(r.errors.iter().any(|e| e.contains("out of range")));
    }

    #[test]
    fn rejects_invalid_mode() {
        let r = make_report("mode: banana\n");
        assert!(!r.is_valid());
        assert!(r.errors.iter().any(|e| e.contains("mode")));
    }

    #[test]
    fn rejects_proxy_missing_name() {
        let r = make_report("proxies:\n  - type: ss\n    server: 1.2.3.4\n    port: 443\n");
        assert!(!r.is_valid());
        assert!(r.errors.iter().any(|e| e.contains("name")));
    }

    #[test]
    fn rejects_rule_missing_target() {
        let r = make_report("rules:\n  - \"DOMAIN,example.com\"\n");
        assert!(!r.is_valid());
        assert!(r.errors.iter().any(|e| e.contains("TYPE,MATCHER,TARGET")));
    }

    #[test]
    fn accepts_match_rule() {
        let r = make_report("rules:\n  - \"MATCH,DIRECT\"\n");
        assert!(r.is_valid(), "errors: {:?}", r.errors);
    }

    #[test]
    fn rejects_invalid_tun_stack() {
        let r = make_report("tun:\n  stack: banana\n");
        assert!(!r.is_valid());
        assert!(r.errors.iter().any(|e| e.contains("tun.stack")));
    }

    #[test]
    fn rejects_invalid_dns_mode() {
        let r = make_report("dns:\n  enhanced-mode: banana\n");
        assert!(!r.is_valid());
        assert!(r.errors.iter().any(|e| e.contains("dns.enhanced-mode")));
    }
}
