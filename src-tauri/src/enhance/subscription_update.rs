use serde_yaml_ng::{Mapping, Sequence, Value};
use std::collections::HashSet;

pub(crate) const SUBSCRIPTION_UPDATE_GROUP: &str = "VERGE-SUB-UPDATE";

pub(crate) fn apply_subscription_update_control_plane(mut config: Mapping) -> Mapping {
    let mut groups = config
        .get("proxy-groups")
        .and_then(Value::as_sequence)
        .cloned()
        .unwrap_or_default();

    groups.retain(|group| {
        group
            .get("name")
            .and_then(Value::as_str)
            .is_none_or(|name| name != SUBSCRIPTION_UPDATE_GROUP)
    });

    let mut control_plane_group = Mapping::new();
    control_plane_group.insert("name".into(), Value::from(SUBSCRIPTION_UPDATE_GROUP));
    control_plane_group.insert("type".into(), Value::from("select"));
    control_plane_group.insert("hidden".into(), Value::Bool(true));
    control_plane_group.insert("proxies".into(), Value::Sequence(build_control_plane_members(&config)));
    let provider_names = collect_provider_names(&config);
    if !provider_names.is_empty() {
        control_plane_group.insert("use".into(), Value::Sequence(provider_names));
    }
    groups.push(Value::Mapping(control_plane_group));

    let mut profile = config
        .get("profile")
        .and_then(Value::as_mapping)
        .cloned()
        .unwrap_or_default();
    if !profile.contains_key("store-selected") {
        profile.insert("store-selected".into(), Value::Bool(true));
    }

    config.insert("profile".into(), Value::Mapping(profile));
    config.insert("proxy-groups".into(), Value::Sequence(groups));
    config
}

fn build_control_plane_members(config: &Mapping) -> Sequence {
    let mut members = Sequence::new();
    let mut seen = HashSet::<std::string::String>::new();

    members.push(Value::from("DIRECT"));
    seen.insert("direct".to_string());

    if let Some(Value::Sequence(proxies)) = config.get("proxies") {
        for proxy in proxies {
            let Some(name) = (match proxy {
                Value::Mapping(map) => map.get("name").and_then(Value::as_str),
                Value::String(name) => Some(name.as_str()),
                _ => None,
            }) else {
                continue;
            };

            let trimmed = name.trim();
            if trimmed.is_empty() {
                continue;
            }

            let dedupe_key = trimmed.to_ascii_lowercase();
            if seen.insert(dedupe_key) {
                members.push(Value::from(trimmed));
            }
        }
    }

    members
}

fn collect_provider_names(config: &Mapping) -> Sequence {
    config
        .get("proxy-providers")
        .and_then(Value::as_mapping)
        .map(|providers| {
            providers
                .keys()
                .filter_map(Value::as_str)
                .map(Value::from)
                .collect::<Sequence>()
        })
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_yaml(yaml: &str) -> Mapping {
        serde_yaml_ng::from_str(yaml).expect("Failed to parse test yaml")
    }

    #[test]
    fn injects_subscription_update_group_with_direct_and_runtime_proxies() {
        let config = parse_yaml(
            r#"
proxies:
  - name: "node-a"
    type: ss
  - name: "node-b"
    type: vmess
proxy-groups:
  - name: "Main"
    type: select
    proxies:
      - "node-a"
"#,
        );

        let config = apply_subscription_update_control_plane(config);
        let groups = config
            .get("proxy-groups")
            .and_then(Value::as_sequence)
            .expect("proxy-groups should exist");
        let group = groups
            .iter()
            .find(|group| group.get("name").and_then(Value::as_str) == Some(SUBSCRIPTION_UPDATE_GROUP))
            .and_then(Value::as_mapping)
            .expect("subscription update group should exist");

        assert_eq!(group.get("hidden").and_then(Value::as_bool), Some(true));
        assert_eq!(
            group
                .get("proxies")
                .and_then(Value::as_sequence)
                .expect("proxies should exist"),
            &vec![Value::from("DIRECT"), Value::from("node-a"), Value::from("node-b")]
        );
    }

    #[test]
    fn replaces_legacy_subscription_update_group_without_duplication() {
        let config = parse_yaml(
            r#"
proxies:
  - name: "node-a"
    type: ss
proxy-groups:
  - name: "VERGE-SUB-UPDATE"
    type: select
    proxies:
      - "ghost"
"#,
        );

        let config = apply_subscription_update_control_plane(config);
        let groups = config
            .get("proxy-groups")
            .and_then(Value::as_sequence)
            .expect("proxy-groups should exist");

        assert_eq!(
            groups
                .iter()
                .filter(|group| group.get("name").and_then(Value::as_str) == Some(SUBSCRIPTION_UPDATE_GROUP))
                .count(),
            1
        );
    }

    #[test]
    fn injects_provider_uses_into_subscription_update_group() {
        let config = parse_yaml(
            r#"
proxy-providers:
  provider-a:
    type: http
    url: https://example.com/sub
    path: ./provider-a.yaml
proxy-groups: []
"#,
        );

        let config = apply_subscription_update_control_plane(config);
        let groups = config
            .get("proxy-groups")
            .and_then(Value::as_sequence)
            .expect("proxy-groups should exist");
        let group = groups
            .iter()
            .find(|group| group.get("name").and_then(Value::as_str) == Some(SUBSCRIPTION_UPDATE_GROUP))
            .and_then(Value::as_mapping)
            .expect("subscription update group should exist");

        assert_eq!(
            group.get("use").and_then(Value::as_sequence),
            Some(&vec![Value::from("provider-a")])
        );
    }
}
