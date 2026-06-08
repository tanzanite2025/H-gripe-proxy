use serde_yaml_ng::{Mapping, Value};

#[cfg(target_os = "macos")]
use crate::process::AsyncHandler;

use crate::constants::tun as tun_const;

const LAN_DIRECT_RULES: [&str; 3] = [
    "IP-CIDR,10.0.0.0/8,DIRECT,no-resolve",
    "IP-CIDR,172.16.0.0/12,DIRECT,no-resolve",
    "IP-CIDR,192.168.0.0/16,DIRECT,no-resolve",
];

macro_rules! revise {
    ($map: expr, $key: expr, $val: expr) => {
        let ret_key = Value::String($key.into());
        $map.insert(ret_key, Value::from($val));
    };
}

// if key not exists then append value
#[allow(unused_macros)]
macro_rules! append {
    ($map: expr, $key: expr, $val: expr) => {
        let ret_key = Value::String($key.into());
        if !$map.contains_key(&ret_key) {
            $map.insert(ret_key, Value::from($val));
        }
    };
}

pub fn use_tun(mut config: Mapping, enable: bool) -> Mapping {
    let tun_key = Value::from("tun");
    let tun_val = config.get(&tun_key);
    let mut tun_val = tun_val.map_or_else(Mapping::new, |val| {
        val.as_mapping().cloned().unwrap_or_else(Mapping::new)
    });

    if enable {
        append!(tun_val, "stack", tun_const::DEFAULT_STACK);
        append!(tun_val, "auto-route", true);
        #[cfg(target_os = "windows")]
        append!(tun_val, "strict-route", true);
        #[cfg(not(target_os = "windows"))]
        append!(tun_val, "strict-route", false);
        append!(tun_val, "auto-detect-interface", true);
        append!(tun_val, "dns-hijack", tun_const::DNS_HIJACK);

        let dns_key = Value::from("dns");
        let dns_val = config.get(&dns_key);
        let mut dns_val = dns_val.map_or_else(Mapping::new, |val| {
            val.as_mapping().cloned().unwrap_or_else(Mapping::new)
        });
        let ipv6_key = Value::from("ipv6");
        let ipv6_val = config.get(&ipv6_key).and_then(|v| v.as_bool()).unwrap_or(false);

        let current_mode = dns_val
            .get(Value::from("enhanced-mode"))
            .and_then(|v| v.as_str())
            .unwrap_or("fake-ip");
        let has_enhanced_mode = dns_val.contains_key(Value::from("enhanced-mode"));

        #[cfg(target_os = "windows")]
        let force_fake_ip = true;
        #[cfg(not(target_os = "windows"))]
        let force_fake_ip = false;

        if force_fake_ip || current_mode == "fake-ip" || !has_enhanced_mode {
            revise!(dns_val, "enable", true);
            revise!(dns_val, "ipv6", ipv6_val);

            if force_fake_ip || !has_enhanced_mode {
                revise!(dns_val, "enhanced-mode", "fake-ip");
            }

            if !dns_val.contains_key(Value::from("fake-ip-range")) {
                revise!(dns_val, "fake-ip-range", "198.18.0.1/16");
            }

            #[cfg(target_os = "macos")]
            {
                AsyncHandler::spawn(move || async move {
                    crate::utils::resolve::dns::restore_public_dns().await;
                    crate::utils::resolve::dns::set_public_dns("114.114.114.114".to_string()).await;
                });
            }
        }

        #[cfg(target_os = "windows")]
        normalize_windows_tun_dns(&mut dns_val);

        revise!(config, "dns", dns_val);
        ensure_lan_direct_rules_before_match(&mut config);
    } else {
        #[cfg(target_os = "macos")]
        AsyncHandler::spawn(move || async move {
            crate::utils::resolve::dns::restore_public_dns().await;
        });
    }

    revise!(tun_val, "enable", enable);
    revise!(config, "tun", tun_val);

    config
}

fn ensure_lan_direct_rules_before_match(config: &mut Mapping) {
    let rules_key = Value::from("rules");
    let Some(rules) = config.get_mut(&rules_key) else {
        config.insert(
            rules_key,
            Value::Sequence(LAN_DIRECT_RULES.iter().map(|rule| Value::from(*rule)).collect()),
        );
        return;
    };

    let Some(seq) = rules.as_sequence_mut() else {
        return;
    };

    seq.retain(|rule| {
        rule.as_str()
            .is_none_or(|rule| !LAN_DIRECT_RULES.iter().any(|lan_rule| rule == *lan_rule))
    });

    let insert_at = seq.iter().position(is_match_rule).unwrap_or(seq.len());

    for rule in LAN_DIRECT_RULES.iter().rev() {
        seq.insert(insert_at, Value::from(*rule));
    }
}

fn is_match_rule(rule: &Value) -> bool {
    rule.as_str()
        .map(|rule| rule.trim_start().starts_with("MATCH,"))
        .unwrap_or(false)
}

#[cfg(target_os = "windows")]
fn normalize_windows_tun_dns(dns: &mut Mapping) {
    let respect_rules = dns.get("respect-rules").and_then(Value::as_bool).unwrap_or(false);

    if !respect_rules {
        return;
    }

    if let Some(domestic_nameservers) = nested_sequence(dns, "nameserver-policy", "geosite:cn")
        .filter(|values| !values.is_empty())
        .or_else(|| Some(default_domestic_nameservers()))
    {
        dns.insert("nameserver".into(), Value::Sequence(domestic_nameservers));
    }

    if !has_non_empty_sequence(dns, "fallback")
        && let Some(foreign_nameservers) =
            nested_sequence(dns, "nameserver-policy", "geosite:geolocation-!cn").filter(|values| !values.is_empty())
    {
        dns.insert("fallback".into(), Value::Sequence(foreign_nameservers));
    }
}

#[cfg(target_os = "windows")]
fn nested_sequence(mapping: &Mapping, key: &str, nested_key: &str) -> Option<Vec<Value>> {
    mapping
        .get(key)
        .and_then(Value::as_mapping)
        .and_then(|mapping| mapping.get(nested_key))
        .and_then(Value::as_sequence)
        .map(|values| values.to_vec())
}

#[cfg(target_os = "windows")]
fn has_non_empty_sequence(mapping: &Mapping, key: &str) -> bool {
    mapping
        .get(key)
        .and_then(Value::as_sequence)
        .map(|values| !values.is_empty())
        .unwrap_or(false)
}

#[cfg(target_os = "windows")]
fn default_domestic_nameservers() -> Vec<Value> {
    ["https://dns.alidns.com/dns-query", "https://doh.pub/dns-query"]
        .into_iter()
        .map(Value::from)
        .collect()
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::use_tun;
    use serde_yaml_ng::{Mapping, Value};

    fn parse_yaml(yaml: &str) -> Mapping {
        serde_yaml_ng::from_str(yaml).expect("test yaml should parse")
    }

    #[test]
    fn tun_enabled_inserts_private_lan_direct_rules_before_match() {
        let config = parse_yaml(
            r#"
rules:
  - DOMAIN-SUFFIX,example.com,Proxy
  - MATCH,GLOBAL
"#,
        );

        let config = use_tun(config, true);
        let rules = config
            .get("rules")
            .and_then(Value::as_sequence)
            .expect("rules should be a sequence");
        let rules = rules.iter().filter_map(Value::as_str).collect::<Vec<_>>();
        let match_index = rules
            .iter()
            .position(|rule| rule.trim_start().starts_with("MATCH,"))
            .expect("test config should contain MATCH");

        for expected_rule in [
            "IP-CIDR,10.0.0.0/8,DIRECT,no-resolve",
            "IP-CIDR,172.16.0.0/12,DIRECT,no-resolve",
            "IP-CIDR,192.168.0.0/16,DIRECT,no-resolve",
        ] {
            let index = rules
                .iter()
                .position(|rule| *rule == expected_rule)
                .unwrap_or_else(|| panic!("{expected_rule} should be injected"));
            assert!(index < match_index, "{expected_rule} should be inserted before MATCH");
        }
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn tun_enabled_on_windows_forces_fake_ip_and_prefers_domestic_nameserver() {
        let config = parse_yaml(
            r#"
ipv6: true
dns:
  enable: true
  respect-rules: true
  enhanced-mode: redir-host
  nameserver:
    - https://dns.alidns.com/dns-query
    - https://dns.google/dns-query
  nameserver-policy:
    geosite:cn:
      - https://dns.alidns.com/dns-query
      - https://doh.pub/dns-query
    geosite:geolocation-!cn:
      - https://dns.google/dns-query
"#,
        );

        let config = use_tun(config, true);
        let dns = config
            .get("dns")
            .and_then(Value::as_mapping)
            .expect("dns should be a mapping");

        assert_eq!(dns.get("enhanced-mode").and_then(Value::as_str), Some("fake-ip"));
        assert_eq!(dns.get("fake-ip-range").and_then(Value::as_str), Some("198.18.0.1/16"));

        let nameserver = dns
            .get("nameserver")
            .and_then(Value::as_sequence)
            .expect("nameserver should be a sequence")
            .iter()
            .filter_map(Value::as_str)
            .collect::<Vec<_>>();
        assert_eq!(
            nameserver,
            vec!["https://dns.alidns.com/dns-query", "https://doh.pub/dns-query"]
        );
    }
}
