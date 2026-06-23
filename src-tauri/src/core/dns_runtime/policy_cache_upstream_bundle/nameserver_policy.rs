use super::{RustDnsPolicyCacheNameserverPolicyEvidence, RustDnsPolicyCacheNameserverPolicyRuleEvidence};
use super::{filter::wildcard_matches, yaml::DnsPolicyBundleConfig};

pub(super) fn evaluate_nameserver_policy(
    config: &DnsPolicyBundleConfig,
    domain: &str,
) -> RustDnsPolicyCacheNameserverPolicyEvidence {
    let mut matched_rules = Vec::new();
    let mut selected_nameservers = Vec::new();

    for entry in &config.nameserver_policy_entries {
        if let Some(rule_type) = matcher_type(&entry.matcher, domain, config) {
            selected_nameservers.extend(entry.nameservers.iter().cloned());
            matched_rules.push(RustDnsPolicyCacheNameserverPolicyRuleEvidence {
                rule_type: rule_type.into(),
                matcher: entry.matcher.clone(),
                selected_nameservers: entry.nameservers.clone(),
            });
        }
    }
    selected_nameservers.sort();
    selected_nameservers.dedup();

    RustDnsPolicyCacheNameserverPolicyEvidence {
        domain: domain.into(),
        selected_nameservers,
        matched_rules,
        evaluated_rule_count: config.nameserver_policy_entries.len(),
    }
}

fn matcher_type(matcher: &str, domain: &str, config: &DnsPolicyBundleConfig) -> Option<String> {
    let matcher = matcher.trim();
    if matcher.eq_ignore_ascii_case(domain) {
        return Some("exact".into());
    }
    if matcher.starts_with("geosite:") {
        return geosite_matches(matcher, domain).then(|| "geosite-canary".into());
    }
    if matcher.starts_with("rule-set:") || matcher.starts_with("rule-provider:") {
        return rule_provider_matches(matcher, domain, config).then(|| "rule-provider-canary".into());
    }
    if matcher.contains('*') && wildcard_matches(matcher, domain) {
        return Some("wildcard".into());
    }
    if wildcard_matches(matcher, domain) {
        return Some("suffix".into());
    }
    None
}

fn geosite_matches(matcher: &str, domain: &str) -> bool {
    let tag = matcher
        .split_once(':')
        .map(|(_, tag)| tag.trim().to_ascii_lowercase())
        .unwrap_or_default();
    !tag.is_empty() && (domain.ends_with(&format!(".{tag}")) || domain == tag)
}

fn rule_provider_matches(matcher: &str, domain: &str, config: &DnsPolicyBundleConfig) -> bool {
    let provider = matcher
        .split_once(':')
        .map(|(_, provider)| provider.trim())
        .unwrap_or_default();
    config
        .rule_provider_payloads
        .get(provider)
        .map(|rules| rules.iter().any(|rule| rule_provider_rule_matches(rule, domain)))
        .unwrap_or(false)
}

fn rule_provider_rule_matches(rule: &str, domain: &str) -> bool {
    let rule = rule.trim();
    let normalized = rule
        .strip_prefix("DOMAIN-SUFFIX,")
        .or_else(|| rule.strip_prefix("domain-suffix,"))
        .map(|suffix| format!("+.{suffix}"))
        .or_else(|| {
            rule.strip_prefix("DOMAIN,")
                .or_else(|| rule.strip_prefix("domain,"))
                .map(ToOwned::to_owned)
        })
        .unwrap_or_else(|| rule.to_owned());
    wildcard_matches(&normalized, domain)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::dns_runtime::policy_cache_upstream_bundle::yaml::{
        DnsPolicyBundleConfig, NameserverPolicyConfigEntry,
    };
    use std::collections::BTreeMap;

    #[test]
    fn matches_rule_provider_payload() {
        let mut payloads = BTreeMap::new();
        payloads.insert("private".into(), vec!["DOMAIN-SUFFIX,internal.test".into()]);
        let config = DnsPolicyBundleConfig {
            fake_ip_range: "198.18.0.1/16".into(),
            fake_ip_filters: Vec::new(),
            fallback_upstreams: Vec::new(),
            fallback_filter_domains: Vec::new(),
            fallback_filter_ipcidrs: Vec::new(),
            nameserver_policy_entries: vec![NameserverPolicyConfigEntry {
                matcher: "rule-set:private".into(),
                nameservers: vec!["127.0.0.1".into()],
            }],
            rule_provider_payloads: payloads,
        };

        let evidence = evaluate_nameserver_policy(&config, "svc.internal.test");

        assert_eq!(evidence.selected_nameservers, vec!["127.0.0.1"]);
        assert_eq!(evidence.matched_rules[0].rule_type, "rule-provider-canary");
    }
}
