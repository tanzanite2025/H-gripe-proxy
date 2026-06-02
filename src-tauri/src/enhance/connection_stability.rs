/**
 * 连接稳定性配置注入
 *
 * - keep-alive-interval: 防止中间设备杀空闲 TCP
 * - keep-alive-idle: TCP Keep Alive 最大空闲时间
 * - unified-delay: 消除延迟测试中的握手差异
 * - tcp-concurrent: DNS 多 IP 并发建连，弱网下首次连接成功率提升
 * - store-selected: 持久化 select 组选择
 * - tolerance: 所有 url-test 组注入容差，防止微小波动导致频繁切换
 * - smux + brutal-opts: 多路复用 + TCP Brutal 拥塞控制（需 advanced.yaml 开启）
 */

#[allow(unused_imports)]
use crate::config::advanced::{BrutalConfig, MultiplexConfig};
use serde_yaml_ng::{Mapping, Value};

pub fn apply_connection_stability(mut config: Mapping) -> Mapping {
    // keep-alive-interval: 30s，弱网下保活 TCP 连接
    if !config.contains_key("keep-alive-interval") {
        config.insert("keep-alive-interval".into(), Value::from(30));
    }

    // keep-alive-idle: 30s，TCP 连接最大空闲时间，配合 interval 保活
    if !config.contains_key("keep-alive-idle") {
        config.insert("keep-alive-idle".into(), Value::from(30));
    }

    // unified-delay: 统一延迟计算，排除握手差异，减少误切换
    if !config.contains_key("unified-delay") {
        config.insert("unified-delay".into(), Value::Bool(true));
    }

    // tcp-concurrent: DNS 解析多 IP 时并发建连，弱网下首次连接成功率显著提升
    if !config.contains_key("tcp-concurrent") {
        config.insert("tcp-concurrent".into(), Value::Bool(true));
    }

    // store-selected: 所有 select 组持久化用户选择
    let mut profile = config
        .get("profile")
        .and_then(Value::as_mapping)
        .cloned()
        .unwrap_or_default();
    if !profile.contains_key("store-selected") {
        profile.insert("store-selected".into(), Value::Bool(true));
        config.insert("profile".into(), Value::Mapping(profile));
    }

    // 为所有 url-test 组注入 tolerance（防止延迟微小波动导致频繁切换节点/IP）
    if let Some(Value::Sequence(groups)) = config.get_mut("proxy-groups") {
        for group in groups {
            if let Some(group_map) = group.as_mapping_mut() {
                let group_type = group_map.get("type").and_then(Value::as_str).unwrap_or("");

                if group_type == "url-test" && !group_map.contains_key("tolerance") {
                    // 延迟差 < 50ms 不切换，避免 IP 频繁变化
                    group_map.insert("tolerance".into(), Value::from(50));
                }
            }
        }
    }

    config
}

/// 根据 advanced.yaml 中的 multiplex 配置，为每个代理节点注入 smux + brutal-opts
/// 从 coordinator 内存配置读取，避免各自 load_default 读磁盘
pub fn apply_multiplex(config: Mapping) -> Mapping {
    let advanced = crate::feat::get_coordinator().get_advanced_config();
    apply_multiplex_config(config, &advanced.multiplex)
}

/// 根据 MultiplexConfig 为每个代理节点注入 smux + brutal-opts
/// 默认关闭，用户需在 advanced.yaml 中设置 multiplex.enabled = true
pub fn apply_multiplex_config(mut config: Mapping, multiplex: &MultiplexConfig) -> Mapping {
    if !multiplex.enabled {
        return config;
    }

    let smux_value = build_smux_value(multiplex);

    if let Some(Value::Sequence(proxies)) = config.get_mut("proxies") {
        for proxy in proxies {
            if let Some(proxy_map) = proxy.as_mapping_mut() {
                // 仅对尚无 smux 配置的节点注入，尊重用户手动配置
                if !proxy_map.contains_key("smux") {
                    proxy_map.insert("smux".into(), smux_value.clone());
                }
            }
        }
    }

    config
}

fn build_smux_value(multiplex: &MultiplexConfig) -> Value {
    let mut smux = Mapping::new();
    smux.insert("enabled".into(), Value::Bool(true));
    smux.insert("protocol".into(), Value::from(multiplex.protocol.as_str()));
    smux.insert("max-connections".into(), Value::from(multiplex.max_connections));
    smux.insert("min-streams".into(), Value::from(multiplex.min_streams));
    if let Some(max_streams) = multiplex.max_streams {
        smux.insert("max-streams".into(), Value::from(max_streams));
    }
    smux.insert("statistic".into(), Value::Bool(multiplex.statistic));
    smux.insert("only-tcp".into(), Value::Bool(multiplex.only_tcp));
    smux.insert("padding".into(), Value::Bool(multiplex.padding));

    // brutal-opts
    let brutal = &multiplex.brutal;
    if brutal.enabled {
        let mut brutal_map = Mapping::new();
        brutal_map.insert("enabled".into(), Value::Bool(true));
        brutal_map.insert("up".into(), Value::from(format!("{} Mbps", brutal.up)));
        brutal_map.insert("down".into(), Value::from(format!("{} Mbps", brutal.down)));
        smux.insert("brutal-opts".into(), Value::Mapping(brutal_map));
    }

    Value::Mapping(smux)
}

#[cfg(test)]
mod tests {
    use super::super::tests::parse_yaml;
    use super::*;

    #[test]
    fn inject_keep_alive_interval_when_missing() {
        let config = Mapping::new();
        let result = apply_connection_stability(config);
        assert_eq!(result.get("keep-alive-interval").and_then(Value::as_i64), Some(30));
    }

    #[test]
    fn inject_keep_alive_idle_when_missing() {
        let config = Mapping::new();
        let result = apply_connection_stability(config);
        assert_eq!(result.get("keep-alive-idle").and_then(Value::as_i64), Some(30));
    }

    #[test]
    fn preserve_existing_keep_alive_idle() {
        let mut config = Mapping::new();
        config.insert("keep-alive-idle".into(), Value::from(60));
        let result = apply_connection_stability(config);
        assert_eq!(result.get("keep-alive-idle").and_then(Value::as_i64), Some(60));
    }

    #[test]
    fn inject_tcp_concurrent_when_missing() {
        let config = Mapping::new();
        let result = apply_connection_stability(config);
        assert_eq!(result.get("tcp-concurrent").and_then(Value::as_bool), Some(true));
    }

    #[test]
    fn preserve_existing_tcp_concurrent() {
        let mut config = Mapping::new();
        config.insert("tcp-concurrent".into(), Value::Bool(false));
        let result = apply_connection_stability(config);
        assert_eq!(result.get("tcp-concurrent").and_then(Value::as_bool), Some(false));
    }

    #[test]
    fn preserve_existing_keep_alive_interval() {
        let mut config = Mapping::new();
        config.insert("keep-alive-interval".into(), Value::from(60));
        let result = apply_connection_stability(config);
        assert_eq!(result.get("keep-alive-interval").and_then(Value::as_i64), Some(60));
    }

    #[test]
    fn inject_unified_delay_when_missing() {
        let config = Mapping::new();
        let result = apply_connection_stability(config);
        assert_eq!(result.get("unified-delay").and_then(Value::as_bool), Some(true));
    }

    #[test]
    fn inject_store_selected_in_profile() {
        let config = Mapping::new();
        let result = apply_connection_stability(config);
        let profile = result
            .get("profile")
            .and_then(Value::as_mapping)
            .expect("profile should exist");
        assert_eq!(profile.get("store-selected").and_then(Value::as_bool), Some(true));
    }

    #[test]
    fn inject_tolerance_for_url_test_groups() {
        let yaml = r#"
proxy-groups:
  - name: "auto"
    type: url-test
    proxies:
      - "node-a"
  - name: "manual"
    type: select
    proxies:
      - "node-a"
"#;
        let config = parse_yaml(yaml);
        let result = apply_connection_stability(config);

        let groups = result
            .get("proxy-groups")
            .and_then(Value::as_sequence)
            .expect("proxy-groups should exist");

        let auto_group = groups[0].as_mapping().unwrap();
        assert_eq!(auto_group.get("tolerance").and_then(Value::as_i64), Some(50));

        let manual_group = groups[1].as_mapping().unwrap();
        assert!(manual_group.get("tolerance").is_none());
    }

    #[test]
    fn preserve_existing_tolerance() {
        let yaml = r#"
proxy-groups:
  - name: "auto"
    type: url-test
    tolerance: 100
    proxies:
      - "node-a"
"#;
        let config = parse_yaml(yaml);
        let result = apply_connection_stability(config);

        let groups = result.get("proxy-groups").and_then(Value::as_sequence).unwrap();
        let auto_group = groups[0].as_mapping().unwrap();
        assert_eq!(auto_group.get("tolerance").and_then(Value::as_i64), Some(100));
    }

    #[test]
    fn multiplex_disabled_does_nothing() {
        let yaml = r#"
proxies:
  - name: "node-a"
    type: ss
"#;
        let config = parse_yaml(yaml);
        let multiplex = MultiplexConfig::default(); // enabled = false
        let result = apply_multiplex_config(config, &multiplex);

        let proxies = result.get("proxies").and_then(Value::as_sequence).unwrap();
        let proxy = proxies[0].as_mapping().unwrap();
        assert!(proxy.get("smux").is_none());
    }

    #[test]
    fn multiplex_enabled_injects_smux() {
        let yaml = r#"
proxies:
  - name: "node-a"
    type: ss
  - name: "node-b"
    type: vmess
"#;
        let config = parse_yaml(yaml);
        let multiplex = MultiplexConfig {
            enabled: true,
            ..MultiplexConfig::recommended()
        };
        let result = apply_multiplex_config(config, &multiplex);

        let proxies = result.get("proxies").and_then(Value::as_sequence).unwrap();
        for proxy in proxies {
            let proxy_map = proxy.as_mapping().unwrap();
            let smux = proxy_map.get("smux").and_then(Value::as_mapping).unwrap();
            assert_eq!(smux.get("enabled").and_then(Value::as_bool), Some(true));
            assert_eq!(smux.get("protocol").and_then(Value::as_str), Some("h2mux"));
            assert_eq!(smux.get("max-connections").and_then(Value::as_i64), Some(4));
            assert_eq!(smux.get("min-streams").and_then(Value::as_i64), Some(4));
            assert_eq!(smux.get("padding").and_then(Value::as_bool), Some(true));
        }
    }

    #[test]
    fn multiplex_respects_existing_smux() {
        let yaml = r#"
proxies:
  - name: "node-a"
    type: ss
    smux:
      enabled: false
"#;
        let config = parse_yaml(yaml);
        let multiplex = MultiplexConfig {
            enabled: true,
            ..MultiplexConfig::recommended()
        };
        let result = apply_multiplex_config(config, &multiplex);

        let proxies = result.get("proxies").and_then(Value::as_sequence).unwrap();
        let proxy = proxies[0].as_mapping().unwrap();
        let smux = proxy.get("smux").and_then(Value::as_mapping).unwrap();
        // 已有 smux 配置应被保留
        assert_eq!(smux.get("enabled").and_then(Value::as_bool), Some(false));
    }

    #[test]
    fn multiplex_with_brutal_injects_brutal_opts() {
        let yaml = r#"
proxies:
  - name: "node-a"
    type: ss
"#;
        let config = parse_yaml(yaml);
        let multiplex = MultiplexConfig {
            enabled: true,
            brutal: BrutalConfig {
                enabled: true,
                up: 50,
                down: 100,
            },
            ..MultiplexConfig::recommended()
        };
        let result = apply_multiplex_config(config, &multiplex);

        let proxies = result.get("proxies").and_then(Value::as_sequence).unwrap();
        let proxy = proxies[0].as_mapping().unwrap();
        let smux = proxy.get("smux").and_then(Value::as_mapping).unwrap();
        let brutal_opts = smux.get("brutal-opts").and_then(Value::as_mapping).unwrap();
        assert_eq!(brutal_opts.get("enabled").and_then(Value::as_bool), Some(true));
        assert_eq!(brutal_opts.get("up").and_then(Value::as_str), Some("50 Mbps"));
        assert_eq!(brutal_opts.get("down").and_then(Value::as_str), Some("100 Mbps"));
    }

    #[test]
    fn multiplex_without_brutal_no_brutal_opts() {
        let yaml = r#"
proxies:
  - name: "node-a"
    type: ss
"#;
        let config = parse_yaml(yaml);
        let multiplex = MultiplexConfig {
            enabled: true,
            brutal: BrutalConfig::default(), // enabled = false
            ..MultiplexConfig::recommended()
        };
        let result = apply_multiplex_config(config, &multiplex);

        let proxies = result.get("proxies").and_then(Value::as_sequence).unwrap();
        let proxy = proxies[0].as_mapping().unwrap();
        let smux = proxy.get("smux").and_then(Value::as_mapping).unwrap();
        assert!(smux.get("brutal-opts").is_none());
    }
}
