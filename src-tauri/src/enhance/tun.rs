use serde_yaml_ng::{Mapping, Value};

#[cfg(target_os = "macos")]
use crate::process::AsyncHandler;

use crate::constants::tun as tun_const;
use crate::core::security_policy::{TUN_SECURITY_SUB_RULE, get_security_policy_manager};

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

        // 读取DNS配置
        let dns_key = Value::from("dns");
        let dns_val = config.get(&dns_key);
        let mut dns_val = dns_val.map_or_else(Mapping::new, |val| {
            val.as_mapping().cloned().unwrap_or_else(Mapping::new)
        });
        let ipv6_key = Value::from("ipv6");
        let ipv6_val = config.get(&ipv6_key).and_then(|v| v.as_bool()).unwrap_or(false);

        // 检查现有的 enhanced-mode 设置
        let current_mode = dns_val
            .get(Value::from("enhanced-mode"))
            .and_then(|v| v.as_str())
            .unwrap_or("fake-ip");

        // 只有当 enhanced-mode 是 fake-ip 或未设置时才修改 DNS 配置
        if current_mode == "fake-ip" || !dns_val.contains_key(Value::from("enhanced-mode")) {
            revise!(dns_val, "enable", true);
            revise!(dns_val, "ipv6", ipv6_val);

            if !dns_val.contains_key(Value::from("enhanced-mode")) {
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

        // 当TUN启用时，将修改后的DNS配置写回
        revise!(config, "dns", dns_val);
        ensure_lan_direct_rules_before_match(&mut config);
    } else {
        // TUN未启用时，仅恢复系统DNS，不修改配置文件中的DNS设置
        #[cfg(target_os = "macos")]
        AsyncHandler::spawn(move || async move {
            crate::utils::resolve::dns::restore_public_dns().await;
        });
    }

    // 更新TUN配置
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

    let insert_at = seq
        .iter()
        .position(|rule| is_match_rule(rule))
        .unwrap_or(seq.len());

    for rule in LAN_DIRECT_RULES.iter().rev() {
        seq.insert(insert_at, Value::from(*rule));
    }
}

fn is_match_rule(rule: &Value) -> bool {
    rule.as_str()
        .map(|rule| rule.trim_start().starts_with("MATCH,"))
        .unwrap_or(false)
}

/// Inject tun.rule and sub-rules.tun-security when there are enabled tun_only security policies.
/// This must be called after use_tun so the tun section already exists.
pub async fn apply_tun_security_policy(mut config: Mapping) -> Mapping {
    let manager = get_security_policy_manager();
    let policies = manager.get_policies().await;
    let has_tun_only = policies.iter().any(|p| p.tun_only && p.enabled);

    if has_tun_only {
        // Set tun.rule = "tun-security" so TUN listener uses the dedicated sub-rule list
        let tun_key = Value::from("tun");
        if let Some(tun_val) = config.get_mut(&tun_key) {
            if let Some(tun_map) = tun_val.as_mapping_mut() {
                revise!(tun_map, "rule", TUN_SECURITY_SUB_RULE);
            }
        }

        // Ensure sub-rules has a tun-security entry (empty list, will be filled at runtime)
        let sub_rules_key = Value::from("sub-rules");
        let sub_rules_val = config.get(&sub_rules_key);
        let mut sub_rules_val = sub_rules_val
            .and_then(|v| v.as_mapping().cloned())
            .unwrap_or_else(Mapping::new);

        let tun_security_key = Value::from(TUN_SECURITY_SUB_RULE);
        if !sub_rules_val.contains_key(&tun_security_key) {
            // Insert empty sequence for tun-security sub-rules
            sub_rules_val.insert(tun_security_key, Value::Sequence(Vec::new()));
        }

        revise!(config, "sub-rules", sub_rules_val);

        // Auto-enable sniffer for TUN-only policies: TUN traffic arrives as pure IP,
        // sniffer must extract domain names for rule matching to work.
        let sniffer_key = Value::from("sniffer");
        let mut sniffer_val = config
            .get(&sniffer_key)
            .and_then(|v| v.as_mapping().cloned())
            .unwrap_or_else(Mapping::new);
        revise!(sniffer_val, "enable", true);
        revise!(sniffer_val, "parse-pure-ip", true);
        revise!(sniffer_val, "force-dns-mapping", true);
        revise!(config, "sniffer", sniffer_val);
    }

    config
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
            assert!(
                index < match_index,
                "{expected_rule} should be inserted before MATCH"
            );
        }
    }
}
