/**
 * 代理组清理
 *
 * 移除 proxy-groups 中引用了不存在的代理/组/provider 的条目，
 * 避免运行时因找不到节点而报错。
 */
use serde_yaml_ng::{Mapping, Value};
use smartstring::alias::String;
use std::collections::HashSet;

pub fn cleanup_proxy_groups(mut config: Mapping) -> Mapping {
    const BUILTIN_POLICIES: &[&str] = &["DIRECT", "REJECT", "REJECT-DROP", "PASS"];

    let proxy_names = config
        .get("proxies")
        .and_then(|v| v.as_sequence())
        .map(|seq| {
            seq.iter()
                .filter_map(|item| match item {
                    Value::Mapping(map) => map
                        .get("name")
                        .and_then(Value::as_str)
                        .map(|name| name.to_owned().into()),
                    Value::String(name) => Some(name.to_owned().into()),
                    _ => None,
                })
                .collect::<HashSet<String>>()
        })
        .unwrap_or_default();

    let group_names = config
        .get("proxy-groups")
        .and_then(|v| v.as_sequence())
        .map(|seq| {
            seq.iter()
                .filter_map(|item| {
                    item.as_mapping()
                        .and_then(|map| map.get("name"))
                        .and_then(Value::as_str)
                        .map(std::convert::Into::into)
                })
                .collect::<HashSet<String>>()
        })
        .unwrap_or_default();

    let provider_names = config
        .get("proxy-providers")
        .and_then(Value::as_mapping)
        .map(|map| {
            map.keys()
                .filter_map(Value::as_str)
                .map(std::convert::Into::into)
                .collect::<HashSet<String>>()
        })
        .unwrap_or_default();

    let mut allowed_names = proxy_names;
    allowed_names.extend(group_names);
    allowed_names.extend(provider_names.iter().cloned());
    allowed_names.extend(BUILTIN_POLICIES.iter().map(|p| (*p).into()));

    if let Some(Value::Sequence(groups)) = config.get_mut("proxy-groups") {
        for group in groups {
            if let Some(group_map) = group.as_mapping_mut() {
                normalize_group_healthcheck_url(group_map);
                let mut has_valid_provider = false;

                if let Some(Value::Sequence(uses)) = group_map.get_mut("use") {
                    uses.retain(|provider| match provider {
                        Value::String(name) => {
                            let exists = provider_names.contains(name.as_str());
                            has_valid_provider = has_valid_provider || exists;
                            exists
                        }
                        _ => false,
                    });
                }

                if let Some(Value::Sequence(proxies)) = group_map.get_mut("proxies") {
                    proxies.retain(|proxy| match proxy {
                        Value::String(name) => allowed_names.contains(name.as_str()) || has_valid_provider,
                        _ => true,
                    });
                }
            }
        }
    }

    normalize_provider_healthcheck_urls(&mut config);

    config
}

fn normalize_group_healthcheck_url(group_map: &mut Mapping) {
    let Some(url) = group_map.get("url").and_then(Value::as_str) else {
        return;
    };

    if let Some(normalized) = normalize_generate_204_url(url) {
        group_map.insert("url".into(), Value::from(normalized.as_str()));
    }
}

fn normalize_provider_healthcheck_urls(config: &mut Mapping) {
    let Some(Value::Mapping(providers)) = config.get_mut("proxy-providers") else {
        return;
    };

    for (_, provider) in providers.iter_mut() {
        let Some(provider_map) = provider.as_mapping_mut() else {
            continue;
        };
        let Some(Value::Mapping(health_check)) = provider_map.get_mut("health-check") else {
            continue;
        };
        let Some(url) = health_check.get("url").and_then(Value::as_str) else {
            continue;
        };

        if let Some(normalized) = normalize_generate_204_url(url) {
            health_check.insert("url".into(), Value::from(normalized.as_str()));
        }
    }
}

fn normalize_generate_204_url(url: &str) -> Option<String> {
    let trimmed = url.trim();
    let rest = trimmed.strip_prefix("http://")?;

    if !rest.contains("/generate_204") {
        return None;
    }

    Some(format!("https://{rest}").into())
}

#[cfg(test)]
mod tests {
    use super::super::tests::parse_yaml;
    use super::*;

    #[test]
    fn remove_missing_proxies_from_groups() {
        let yaml = r#"
proxies:
  - name: "alive-node"
    type: ss
proxy-groups:
  - name: "manual"
    type: select
    proxies:
      - "alive-node"
      - "missing-node"
      - "DIRECT"
  - name: "nested"
    type: select
    proxies:
      - "manual"
      - "ghost"
"#;
        let config = parse_yaml(yaml);
        let config = cleanup_proxy_groups(config);

        let groups = config
            .get("proxy-groups")
            .and_then(Value::as_sequence)
            .cloned()
            .expect("proxy-groups should be a sequence");

        let manual_group = groups
            .iter()
            .find(|g| g.get("name").and_then(Value::as_str) == Some("manual"))
            .and_then(|g| g.as_mapping())
            .expect("manual group should exist");

        let proxies = manual_group
            .get("proxies")
            .and_then(Value::as_sequence)
            .expect("proxies should be a sequence");
        assert_eq!(proxies.len(), 2);
        assert!(proxies.iter().any(|p| p.as_str() == Some("alive-node")));
        assert!(proxies.iter().any(|p| p.as_str() == Some("DIRECT")));

        let nested_group = groups
            .iter()
            .find(|g| g.get("name").and_then(Value::as_str) == Some("nested"))
            .and_then(|g| g.as_mapping())
            .expect("nested group should exist");

        let nested_proxies = nested_group
            .get("proxies")
            .and_then(Value::as_sequence)
            .expect("proxies should be a sequence");
        assert_eq!(nested_proxies.len(), 1);
        assert_eq!(nested_proxies[0].as_str(), Some("manual"));
    }

    #[test]
    fn keep_provider_backed_groups_intact() {
        let yaml = r#"
proxy-providers:
  providerA:
    type: http
    url: https://example.com
    path: ./providerA.yaml
proxies: []
proxy-groups:
  - name: "manual"
    type: select
    use:
      - "providerA"
      - "ghostProvider"
    proxies:
      - "dynamic-node"
      - "DIRECT"
"#;
        let config = parse_yaml(yaml);
        let config = cleanup_proxy_groups(config);

        let groups = config
            .get("proxy-groups")
            .and_then(Value::as_sequence)
            .cloned()
            .expect("proxy-groups should be a sequence");

        let manual_group = groups
            .iter()
            .find(|g| g.get("name").and_then(Value::as_str) == Some("manual"))
            .and_then(|g| g.as_mapping())
            .expect("manual group should exist");

        let uses = manual_group
            .get("use")
            .and_then(Value::as_sequence)
            .expect("use should be a sequence");
        assert_eq!(uses.len(), 1);
        assert_eq!(uses[0].as_str(), Some("providerA"));

        let proxies = manual_group
            .get("proxies")
            .and_then(Value::as_sequence)
            .expect("proxies should be a sequence");
        assert_eq!(proxies.len(), 2);
        assert!(proxies.iter().any(|p| p.as_str() == Some("dynamic-node")));
        assert!(proxies.iter().any(|p| p.as_str() == Some("DIRECT")));
    }

    #[test]
    fn prune_invalid_provider_and_proxies_without_provider() {
        let yaml = r#"
proxy-groups:
  - name: "manual"
    type: select
    use:
      - "ghost-provider"
    proxies:
      - "ghost-node"
      - "DIRECT"
"#;
        let config = parse_yaml(yaml);
        let config = cleanup_proxy_groups(config);

        let groups = config
            .get("proxy-groups")
            .and_then(Value::as_sequence)
            .cloned()
            .expect("proxy-groups should be a sequence");

        let manual_group = groups
            .iter()
            .find(|g| g.get("name").and_then(Value::as_str) == Some("manual"))
            .and_then(|g| g.as_mapping())
            .expect("manual group should exist");

        let uses = manual_group
            .get("use")
            .and_then(Value::as_sequence)
            .expect("use should be a sequence");
        assert_eq!(uses.len(), 0);

        let proxies = manual_group
            .get("proxies")
            .and_then(Value::as_sequence)
            .expect("proxies should be a sequence");
        assert_eq!(proxies.len(), 1);
        assert_eq!(proxies[0].as_str(), Some("DIRECT"));
    }

    #[test]
    fn upgrade_plain_http_generate_204_group_urls() {
        let yaml = r#"
proxy-groups:
  - name: "auto"
    type: url-test
    url: http://www.gstatic.com/generate_204
    proxies:
      - "DIRECT"
"#;
        let config = parse_yaml(yaml);
        let config = cleanup_proxy_groups(config);

        let groups = config
            .get("proxy-groups")
            .and_then(Value::as_sequence)
            .cloned()
            .expect("proxy-groups should be a sequence");
        let auto_group = groups[0].as_mapping().expect("group should be a mapping");

        assert_eq!(
            auto_group.get("url").and_then(Value::as_str),
            Some("https://www.gstatic.com/generate_204")
        );
    }

    #[test]
    fn upgrade_plain_http_generate_204_provider_healthcheck_urls() {
        let yaml = r#"
proxy-providers:
  providerA:
    type: http
    url: https://example.com/sub
    path: ./providerA.yaml
    health-check:
      enable: true
      url: http://cp.cloudflare.com/generate_204
proxy-groups: []
"#;
        let config = parse_yaml(yaml);
        let config = cleanup_proxy_groups(config);

        let providers = config
            .get("proxy-providers")
            .and_then(Value::as_mapping)
            .expect("proxy-providers should be a mapping");
        let provider = providers
            .get("providerA")
            .and_then(Value::as_mapping)
            .expect("providerA should exist");
        let health_check = provider
            .get("health-check")
            .and_then(Value::as_mapping)
            .expect("health-check should exist");

        assert_eq!(
            health_check.get("url").and_then(Value::as_str),
            Some("https://cp.cloudflare.com/generate_204")
        );
    }
}
