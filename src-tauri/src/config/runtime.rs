use serde_yaml_ng::{Mapping, Value};
use smartstring::alias::String;
use std::collections::{HashMap, HashSet};

use crate::{
    core::clash_mode::ClashMode,
    enhance::{apply_stable_egress_policy, field::use_keys},
};

const PATCH_CONFIG_INNER: [&str; 5] = ["allow-lan", "ipv6", "log-level", "unified-delay", "tunnels"];
const PATCH_CONFIG_MODE: &str = "mode";

#[derive(Default, Clone)]
pub struct IRuntime {
    pub config: Option<Mapping>,
    // 记录在订阅中（包括merge和script生成的）出现过的keys
    // 这些keys不一定都生效
    pub exists_keys: HashSet<String>,
    // TODO 或许可以用 FixMap 来存储以提升效率
    pub chain_logs: HashMap<String, Vec<(String, String)>>,
}

impl IRuntime {
    #[inline]
    pub fn new() -> Self {
        Self::default()
    }

    // 这里只更改 allow-lan | ipv6 | log-level | tun | tunnels
    #[inline]
    pub fn patch_config(&mut self, patch: &Mapping) {
        let config = if let Some(config) = self.config.as_mut() {
            config
        } else {
            return;
        };

        for key in PATCH_CONFIG_INNER.iter() {
            if let Some(value) = patch.get(key) {
                config.insert((*key).into(), value.clone());
            }
        }

        if let Some(mode) = patch
            .get(PATCH_CONFIG_MODE)
            .and_then(Value::as_str)
            .and_then(|mode| mode.parse::<ClashMode>().ok())
        {
            config.insert(PATCH_CONFIG_MODE.into(), mode.as_str().into());
        }

        let patch_tun = patch.get("tun");
        if let Some(patch_tun_value) = patch_tun {
            let mut tun = config
                .get("tun")
                .and_then(|val| val.as_mapping())
                .cloned()
                .unwrap_or_else(Mapping::new);

            if let Some(patch_tun_mapping) = patch_tun_value.as_mapping() {
                for key in use_keys(patch_tun_mapping) {
                    if let Some(value) = patch_tun_mapping.get(key.as_str()) {
                        tun.insert(Value::from(key.as_str()), value.clone());
                    }
                }
            }

            config.insert("tun".into(), Value::from(tun));
        }
    }

    #[inline]
    pub fn patch_dns_runtime_config(&mut self, patch: &Mapping) {
        let config = if let Some(config) = self.config.as_mut() {
            config
        } else {
            return;
        };

        for key in ["dns", "hosts"] {
            if let Some(value) = patch.get(key) {
                if matches!(value, Value::Null) {
                    config.remove(key);
                } else {
                    config.insert(key.into(), value.clone());
                }
            }
        }
    }

    #[inline]
    pub fn patch_adapter_egress_runtime_config(&mut self, patch: &Mapping) {
        let config = if let Some(config) = self.config.as_mut() {
            config
        } else {
            return;
        };

        if let Some(groups) = patch.get("proxy-groups").and_then(Value::as_sequence) {
            let group_names = groups
                .iter()
                .filter_map(|group| {
                    group
                        .as_mapping()
                        .and_then(|mapping| mapping.get("name"))
                        .and_then(Value::as_str)
                        .map(|name| name.to_string())
                })
                .collect::<HashSet<_>>();
            let existing = config
                .get("proxy-groups")
                .and_then(Value::as_sequence)
                .cloned()
                .unwrap_or_default();
            let mut merged = existing
                .into_iter()
                .filter(|group| {
                    group
                        .as_mapping()
                        .and_then(|mapping| mapping.get("name"))
                        .and_then(Value::as_str)
                        .map(|name| !group_names.contains(name))
                        .unwrap_or(true)
                })
                .collect::<Vec<_>>();
            merged.extend(groups.iter().cloned());
            config.insert("proxy-groups".into(), Value::Sequence(merged));
        }

        if let Some(rules) = patch.get("rules").and_then(Value::as_sequence) {
            let rule_identities = rules
                .iter()
                .filter_map(Value::as_str)
                .filter_map(adapter_egress_rule_identity)
                .collect::<HashSet<_>>();
            let existing = config
                .get("rules")
                .and_then(Value::as_sequence)
                .cloned()
                .unwrap_or_default();
            let mut merged = rules.clone();
            merged.extend(existing.into_iter().filter(|rule| {
                rule.as_str()
                    .and_then(adapter_egress_rule_identity)
                    .map(|identity| !rule_identities.contains(&identity))
                    .unwrap_or(true)
            }));
            config.insert("rules".into(), Value::Sequence(merged));
        }
    }

    #[inline]
    pub fn replace_adapter_egress_runtime_config(&mut self, patch: &Mapping) {
        let config = if let Some(config) = self.config.as_mut() {
            config
        } else {
            return;
        };

        for key in ["proxy-groups", "rules"] {
            match patch.get(key) {
                Some(Value::Null) => {
                    config.remove(key);
                }
                Some(value) => {
                    config.insert(key.into(), value.clone());
                }
                None => {}
            }
        }
    }

    /// 更新链式代理配置
    ///
    /// 该函数更新 `proxies` 和 `proxy-groups` 配置，并处理链式代理的修改或(传入 None )删除。
    ///
    /// 配置示例：
    ///
    /// ```json
    /// {
    ///     "proxies": [
    ///         {
    ///             "name": "入口节点",
    ///             "type": "xxx",
    ///             "server": "xxx",
    ///             "port": "xxx",
    ///             "ports": "xxx",
    ///             "password": "xxx",
    ///             "skip-cert-verify": "xxx"
    ///         },
    ///         {
    ///             "name": "hop_node_1_xxxx",
    ///             "type": "xxx",
    ///             "server": "xxx",
    ///             "port": "xxx",
    ///             "ports": "xxx",
    ///             "password": "xxx",
    ///             "skip-cert-verify": "xxx",
    ///             "dialer-proxy": "入口节点"
    ///         },
    ///         {
    ///             "name": "出口节点",
    ///             "type": "xxx",
    ///             "server": "xxx",
    ///             "port": "xxx",
    ///             "ports": "xxx",
    ///             "password": "xxx",
    ///             "skip-cert-verify": "xxx",
    ///             "dialer-proxy": "hop_node_1_xxxx"
    ///         }
    ///     ],
    ///     "proxy-groups": [
    ///         {
    ///             "name": "proxy_chain",
    ///             "type": "select",
    ///             "proxies": ["出口节点"]
    ///         }
    ///     ]
    /// }
    /// ```
    #[inline]
    pub fn update_proxy_chain_config(&mut self, proxy_chain_config: Option<Value>) {
        let config = if let Some(config) = self.config.as_mut() {
            config
        } else {
            return;
        };

        if let Some(Value::Sequence(proxies)) = config.get_mut("proxies") {
            proxies.iter_mut().for_each(|proxy| {
                if let Some(proxy) = proxy.as_mapping_mut()
                    && proxy.get("dialer-proxy").is_some()
                {
                    proxy.remove("dialer-proxy");
                }
            });
        }

        if let Some(Value::Sequence(dialer_proxies)) = proxy_chain_config
            && let Some(Value::Sequence(proxies)) = config.get_mut("proxies")
        {
            for (i, dialer_proxy) in dialer_proxies.iter().enumerate() {
                if let Some(Value::Mapping(proxy)) =
                    proxies.iter_mut().find(|proxy| proxy.get("name") == Some(dialer_proxy))
                    && i != 0
                    && let Some(dialer_proxy) = dialer_proxies.get(i - 1)
                {
                    proxy.insert("dialer-proxy".into(), dialer_proxy.to_owned());
                }
            }
        }

        let stabilized = apply_stable_egress_policy(config.clone());
        *config = stabilized;
    }
}

fn adapter_egress_rule_identity(rule: &str) -> Option<String> {
    let mut segments = rule.split(',');
    let matcher = segments.next()?.trim();
    let value = segments.next()?.trim();
    if matcher.is_empty() || value.is_empty() {
        None
    } else {
        Some(format!("{matcher},{value}").into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn patch_config_applies_supported_clash_mode_to_runtime_config() {
        let mut runtime = IRuntime {
            config: Some(Mapping::new()),
            ..IRuntime::default()
        };
        let mut patch = Mapping::new();
        patch.insert("mode".into(), " GLOBAL ".into());

        runtime.patch_config(&patch);

        let config = runtime.config.as_ref().unwrap();
        assert_eq!(config.get("mode").and_then(Value::as_str), Some("global"));
    }

    #[test]
    fn patch_config_ignores_unsupported_clash_mode_in_runtime_config() {
        let mut config = Mapping::new();
        config.insert("mode".into(), "rule".into());
        let mut runtime = IRuntime {
            config: Some(config),
            ..IRuntime::default()
        };
        let mut patch = Mapping::new();
        patch.insert("mode".into(), "script".into());

        runtime.patch_config(&patch);

        let config = runtime.config.as_ref().unwrap();
        assert_eq!(config.get("mode").and_then(Value::as_str), Some("rule"));
    }

    #[test]
    fn patch_dns_runtime_config_updates_dns_and_hosts() {
        let mut runtime = IRuntime {
            config: Some(Mapping::new()),
            ..IRuntime::default()
        };
        let mut dns = Mapping::new();
        dns.insert("enable".into(), true.into());
        dns.insert(
            "nameserver".into(),
            Value::Sequence(vec!["1.1.1.1".into(), "8.8.8.8".into()]),
        );
        let mut hosts = Mapping::new();
        hosts.insert("example.test".into(), "127.0.0.1".into());
        let mut patch = Mapping::new();
        patch.insert("dns".into(), Value::Mapping(dns));
        patch.insert("hosts".into(), Value::Mapping(hosts));

        runtime.patch_dns_runtime_config(&patch);

        let config = runtime.config.as_ref().unwrap();
        assert!(config.get("dns").and_then(Value::as_mapping).is_some());
        assert!(config.get("hosts").and_then(Value::as_mapping).is_some());
    }

    #[test]
    fn patch_dns_runtime_config_can_remove_dns_and_hosts_for_rollback() {
        let mut config = Mapping::new();
        config.insert("dns".into(), Value::Mapping(Mapping::new()));
        config.insert("hosts".into(), Value::Mapping(Mapping::new()));
        let mut runtime = IRuntime {
            config: Some(config),
            ..IRuntime::default()
        };
        let mut patch = Mapping::new();
        patch.insert("dns".into(), Value::Null);
        patch.insert("hosts".into(), Value::Null);

        runtime.patch_dns_runtime_config(&patch);

        let config = runtime.config.as_ref().unwrap();
        assert!(config.get("dns").is_none());
        assert!(config.get("hosts").is_none());
    }

    #[test]
    fn patch_adapter_egress_runtime_config_replaces_app_group_and_rule_identity() {
        let mut config = Mapping::new();
        config.insert(
            "proxy-groups".into(),
            serde_yaml_ng::from_str::<Value>(
                r#"
- name: app-demo
  type: select
  proxies:
    - old-node
- name: keep
  type: select
  proxies:
    - keep-node
"#,
            )
            .unwrap(),
        );
        config.insert(
            "rules".into(),
            Value::Sequence(vec![
                "PROCESS-NAME,browser.exe,old-target".into(),
                "DOMAIN,example.com,DIRECT".into(),
            ]),
        );
        let mut runtime = IRuntime {
            config: Some(config),
            ..IRuntime::default()
        };
        let patch = serde_yaml_ng::from_str::<Value>(
            r#"
proxy-groups:
  - name: app-demo
    type: select
    proxies:
      - new-node
rules:
  - PROCESS-NAME,browser.exe,app-demo
"#,
        )
        .unwrap()
        .as_mapping()
        .cloned()
        .unwrap();

        runtime.patch_adapter_egress_runtime_config(&patch);

        let config = runtime.config.as_ref().unwrap();
        let groups = config.get("proxy-groups").and_then(Value::as_sequence).unwrap();
        let group_names = groups
            .iter()
            .filter_map(|group| {
                group
                    .as_mapping()
                    .and_then(|mapping| mapping.get("name"))
                    .and_then(Value::as_str)
            })
            .collect::<Vec<_>>();
        assert_eq!(group_names, vec!["keep", "app-demo"]);
        let rules = config.get("rules").and_then(Value::as_sequence).unwrap();
        assert_eq!(
            rules.iter().filter_map(Value::as_str).collect::<Vec<_>>(),
            vec!["PROCESS-NAME,browser.exe,app-demo", "DOMAIN,example.com,DIRECT"]
        );
    }

    #[test]
    fn replace_adapter_egress_runtime_config_restores_previous_sequences() {
        let mut runtime = IRuntime {
            config: Some(Mapping::new()),
            ..IRuntime::default()
        };
        let patch = serde_yaml_ng::from_str::<Value>(
            r#"
proxy-groups:
  - name: previous
    type: select
    proxies:
      - node-a
rules:
  - PROCESS-NAME,app.exe,DIRECT
"#,
        )
        .unwrap()
        .as_mapping()
        .cloned()
        .unwrap();

        runtime.replace_adapter_egress_runtime_config(&patch);

        let config = runtime.config.as_ref().unwrap();
        assert_eq!(
            config
                .get("rules")
                .and_then(Value::as_sequence)
                .and_then(|rules| rules.first())
                .and_then(Value::as_str),
            Some("PROCESS-NAME,app.exe,DIRECT")
        );
    }
}
