/**
 * 标准区域代理池
 *
 * 为常见地区（TW/HK/JP/SG/US/KR）自动创建 load-balance + sticky-sessions 组，
 * 确保同源同目标流量锁定同一节点，避免 IP 频繁变化。
 */

use regex::Regex;
use serde_yaml_ng::{Mapping, Sequence, Value};
use smartstring::alias::String;

const STANDARD_REGION_POOL_PREFIX: &str = "VERGE-REGION-";
const STANDARD_REGION_POOL_URL: &str = "https://www.gstatic.com/generate_204";
const STANDARD_REGION_POOL_INTERVAL: u64 = 300;
const STANDARD_REGION_POOL_TIMEOUT: u64 = 5_000;
const STANDARD_REGION_POOL_MAX_FAILED_TIMES: u64 = 3;

#[derive(Clone, Copy)]
struct StandardRegionPoolSpec {
    code: &'static str,
    filter: &'static str,
}

impl StandardRegionPoolSpec {
    fn group_name(self) -> std::string::String {
        format!("{STANDARD_REGION_POOL_PREFIX}{}-AUTO", self.code)
    }

    fn matches(self, value: &str) -> bool {
        Regex::new(self.filter)
            .map(|re| re.is_match(value))
            .unwrap_or(false)
    }
}

const STANDARD_REGION_POOL_SPECS: [StandardRegionPoolSpec; 6] = [
    StandardRegionPoolSpec {
        code: "TW",
        filter: r"(?i)(台湾|台灣|臺灣|taiwan|taipei|台北|新北|taichung|台中|tainan|台南|kaohsiung|高雄|\bTW\b)",
    },
    StandardRegionPoolSpec {
        code: "HK",
        filter: r"(?i)(香港|hong[\s_-]?kong|\bHK\b)",
    },
    StandardRegionPoolSpec {
        code: "JP",
        filter: r"(?i)(日本|japan|tokyo|osaka|sapporo|\bJP\b)",
    },
    StandardRegionPoolSpec {
        code: "SG",
        filter: r"(?i)(新加坡|singapore|\bSG\b)",
    },
    StandardRegionPoolSpec {
        code: "US",
        filter: r"(?i)(美国|美國|united[\s_-]?states|america|los[\s_-]?angeles|san[\s_-]?jose|seattle|new[\s_-]?york|ashburn|\bUS\b)",
    },
    StandardRegionPoolSpec {
        code: "KR",
        filter: r"(?i)(韩国|韓國|korea|seoul|busan|\bKR\b)",
    },
];

pub fn append_standard_region_pools(mut config: Mapping) -> Mapping {
    let proxy_names = config
        .get("proxies")
        .and_then(Value::as_sequence)
        .map(|seq| {
            seq.iter()
                .filter_map(|item| match item {
                    Value::Mapping(map) => map.get("name").and_then(Value::as_str).map(String::from),
                    Value::String(name) => Some(String::from(name.as_str())),
                    _ => None,
                })
                .collect::<Vec<String>>()
        })
        .unwrap_or_default();

    let provider_names = config
        .get("proxy-providers")
        .and_then(Value::as_mapping)
        .map(|map| {
            map.keys()
                .filter_map(Value::as_str)
                .map(String::from)
                .collect::<Vec<String>>()
        })
        .unwrap_or_default();

    if proxy_names.is_empty() && provider_names.is_empty() {
        return config;
    }

    let mut groups = config
        .get("proxy-groups")
        .and_then(Value::as_sequence)
        .cloned()
        .unwrap_or_default();

    groups.retain(|group| {
        group
            .get("name")
            .and_then(Value::as_str)
            .map(|name| !name.starts_with(STANDARD_REGION_POOL_PREFIX))
            .unwrap_or(true)
    });

    for spec in STANDARD_REGION_POOL_SPECS {
        let static_matches = proxy_names
            .iter()
            .filter(|name| spec.matches(name))
            .cloned()
            .collect::<Vec<String>>();

        if static_matches.is_empty() && provider_names.is_empty() {
            continue;
        }

        let mut group = Mapping::new();
        let group_name = spec.group_name();
        group.insert("name".into(), Value::from(group_name.as_str()));
        // 使用 load-balance + sticky-sessions：同源同目标锁定节点，IP 不变
        group.insert("type".into(), "load-balance".into());
        group.insert("strategy".into(), "sticky-sessions".into());
        group.insert("url".into(), STANDARD_REGION_POOL_URL.into());
        group.insert("interval".into(), STANDARD_REGION_POOL_INTERVAL.into());
        group.insert("timeout".into(), STANDARD_REGION_POOL_TIMEOUT.into());
        group.insert(
            "max-failed-times".into(),
            STANDARD_REGION_POOL_MAX_FAILED_TIMES.into(),
        );
        group.insert("lazy".into(), true.into());
        group.insert("hidden".into(), true.into());

        if !static_matches.is_empty() {
            group.insert(
                "proxies".into(),
                Value::Sequence(
                    static_matches
                        .into_iter()
                        .map(|name| Value::from(name.as_str()))
                        .collect::<Sequence>(),
                ),
            );
        }

        if !provider_names.is_empty() {
            group.insert(
                "use".into(),
                Value::Sequence(
                    provider_names
                        .iter()
                        .map(|name| Value::from(name.as_str()))
                        .collect::<Sequence>(),
                ),
            );
            group.insert("filter".into(), spec.filter.into());
        }

        groups.push(Value::Mapping(group));
    }

    config.insert("proxy-groups".into(), Value::Sequence(groups));
    config
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::tests::parse_yaml;

    #[test]
    fn append_standard_region_pool_from_static_proxies() {
        let yaml = r#"
proxies:
  - name: "台湾-01"
    type: ss
  - name: "香港-01"
    type: ss
proxy-groups: []
"#;
        let config = parse_yaml(yaml);
        let config = append_standard_region_pools(config);

        let groups = config
            .get("proxy-groups")
            .and_then(Value::as_sequence)
            .cloned()
            .expect("proxy-groups should be a sequence");

        let tw_group = groups
            .iter()
            .find(|g| g.get("name").and_then(Value::as_str) == Some("VERGE-REGION-TW-AUTO"))
            .and_then(|g| g.as_mapping())
            .expect("TW region pool should exist");

        let proxies = tw_group
            .get("proxies")
            .and_then(Value::as_sequence)
            .expect("TW region pool proxies should exist");
        assert_eq!(proxies.len(), 1);
        assert_eq!(proxies[0].as_str(), Some("台湾-01"));
        assert_eq!(tw_group.get("hidden").and_then(Value::as_bool), Some(true));
    }

    #[test]
    fn append_standard_region_pool_from_providers() {
        let yaml = r#"
proxy-providers:
  providerA:
    type: http
    url: https://example.com
    path: ./providerA.yaml
proxy-groups: []
"#;
        let config = parse_yaml(yaml);
        let config = append_standard_region_pools(config);

        let groups = config
            .get("proxy-groups")
            .and_then(Value::as_sequence)
            .cloned()
            .expect("proxy-groups should be a sequence");

        let tw_group = groups
            .iter()
            .find(|g| g.get("name").and_then(Value::as_str) == Some("VERGE-REGION-TW-AUTO"))
            .and_then(|g| g.as_mapping())
            .expect("TW region pool should exist");

        let uses = tw_group
            .get("use")
            .and_then(Value::as_sequence)
            .expect("TW region pool use should exist");
        assert_eq!(uses.len(), 1);
        assert_eq!(uses[0].as_str(), Some("providerA"));
        assert!(tw_group.get("filter").and_then(Value::as_str).is_some());
    }

    #[test]
    fn no_pools_when_no_proxies_or_providers() {
        let yaml = "proxy-groups: []\n";
        let config = parse_yaml(yaml);
        let config = append_standard_region_pools(config);

        let groups = config
            .get("proxy-groups")
            .and_then(Value::as_sequence)
            .cloned()
            .unwrap_or_default();
        assert!(groups.is_empty());
    }

    #[test]
    fn replace_existing_region_pools() {
        let yaml = r#"
proxies:
  - name: "日本-01"
    type: ss
proxy-groups:
  - name: "VERGE-REGION-JP-AUTO"
    type: fallback
    proxies:
      - "old-node"
"#;
        let config = parse_yaml(yaml);
        let config = append_standard_region_pools(config);

        let groups = config
            .get("proxy-groups")
            .and_then(Value::as_sequence)
            .cloned()
            .expect("proxy-groups should be a sequence");

        // 应该只有一个 JP 组（替换了旧的）
        let jp_count = groups
            .iter()
            .filter(|g| g.get("name").and_then(Value::as_str) == Some("VERGE-REGION-JP-AUTO"))
            .count();
        assert_eq!(jp_count, 1);

        let jp_group = groups
            .iter()
            .find(|g| g.get("name").and_then(Value::as_str) == Some("VERGE-REGION-JP-AUTO"))
            .and_then(|g| g.as_mapping())
            .unwrap();
        // 新组应该是 load-balance 而非 fallback
        assert_eq!(jp_group.get("type").and_then(Value::as_str), Some("load-balance"));
    }
}
