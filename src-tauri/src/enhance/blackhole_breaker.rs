/// 黑洞熔断器 enhance 集成
///
/// 在 enhance 管线中，将当前处于 Open 状态的熔断规则
/// 转换为 Mihomo DOMAIN-SUFFIX → REJECT-DROP 规则

use serde_yaml_ng::Mapping;

/// 将熔断器生成的 REJECT-DROP 规则注入到 Mihomo 配置的 rules 前部
pub async fn apply_blackhole_breaker_config(mut config: Mapping) -> Mapping {
    let reject_rules = crate::feat::blackhole_breaker_generate_reject_rules().await;

    if reject_rules.is_empty() {
        return config;
    }

    let rules = config
        .entry("rules".into())
        .or_insert_with(|| serde_yaml_ng::Value::Sequence(Vec::new()));

    if let serde_yaml_ng::Value::Sequence(seq) = rules {
        for (pattern, policy) in reject_rules.iter().rev() {
            // DOMAIN-SUFFIX 匹配 *.xxx.com → xxx.com
            let suffix = if pattern.starts_with("*.") {
                &pattern[2..]
            } else {
                pattern
            };
            let rule_str = format!("DOMAIN-SUFFIX,{suffix},{policy}");
            seq.insert(0, serde_yaml_ng::Value::String(rule_str));
        }

        log::info!(
            "[BlackholeBreaker] 注入了 {} 条 REJECT-DROP 规则",
            reject_rules.len()
        );
    }

    config
}
