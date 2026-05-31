/// 黑洞熔断器
///
/// 当出口节点或目标域名的异常指标超过阈值时，
/// 自动将匹配流量导向 Mihomo 的 REJECT-DROP（黑洞），
/// 经过冷却期后进入半开探测，恢复后自动闭合。
///
/// 复用现有基础设施：
/// - Mihomo `REJECT-DROP` 策略作为黑洞出口
/// - `RiskFallbackPolicy::Block` / `EgressFailoverPolicy::Block` 语义对齐
/// - IP 信誉评分作为触发信号之一

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::SystemTime;
use tokio::sync::RwLock;

// ── 熔断器状态 ──────────────────────────────────────────────────────

/// 熔断器状态
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum BreakerState {
    /// 闭合（正常放行）
    Closed,
    /// 断开（黑洞熔断，全部 REJECT-DROP）
    Open,
    /// 半开（允许少量探测流量）
    HalfOpen,
}

impl Default for BreakerState {
    fn default() -> Self {
        Self::Closed
    }
}

impl std::fmt::Display for BreakerState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Closed => write!(f, "Closed"),
            Self::Open => write!(f, "Open"),
            Self::HalfOpen => write!(f, "HalfOpen"),
        }
    }
}

// ── 熔断器配置 ──────────────────────────────────────────────────────

/// 单个熔断规则
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BreakerRule {
    /// 规则 ID
    pub id: String,
    /// 是否启用
    #[serde(default = "default_true")]
    pub enabled: bool,
    /// 匹配目标：域名模式列表
    pub domain_patterns: Vec<String>,
    /// 匹配目标：节点名称模式列表（为空则不限节点）
    #[serde(default)]
    pub node_patterns: Vec<String>,
    /// 触发条件
    pub trigger: BreakerTrigger,
    /// 冷却时长（秒），Open → HalfOpen 的等待时间
    #[serde(default = "default_cooldown")]
    pub cooldown_secs: u64,
    /// 半开探测次数，成功 N 次后闭合
    #[serde(default = "default_probe_count")]
    pub probe_success_count: u8,
    /// 描述
    #[serde(default)]
    pub description: String,
}

fn default_true() -> bool {
    true
}
fn default_cooldown() -> u64 {
    300 // 5 分钟
}
fn default_probe_count() -> u8 {
    3
}

/// 熔断触发条件
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BreakerTrigger {
    /// 连续失败次数阈值
    #[serde(default = "default_fail_threshold")]
    pub consecutive_failures: u32,
    /// 窗口期内失败率（0.0-1.0）
    #[serde(default = "default_failure_rate")]
    pub failure_rate: f64,
    /// 窗口大小（秒）
    #[serde(default = "default_window_secs")]
    pub window_secs: u64,
    /// 窗口内最小请求数（低于此数不触发）
    #[serde(default = "default_min_requests")]
    pub min_requests: u32,
    /// 欺诈评分阈值（IP 信誉欺诈评分 >= 此值触发）
    #[serde(default)]
    pub max_fraud_score: Option<u8>,
}

impl Default for BreakerTrigger {
    fn default() -> Self {
        Self {
            consecutive_failures: default_fail_threshold(),
            failure_rate: default_failure_rate(),
            window_secs: default_window_secs(),
            min_requests: default_min_requests(),
            max_fraud_score: None,
        }
    }
}

fn default_fail_threshold() -> u32 {
    5
}
fn default_failure_rate() -> f64 {
    0.6
}
fn default_window_secs() -> u64 {
    60
}
fn default_min_requests() -> u32 {
    10
}

/// 黑洞熔断器全局配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlackholeBreakerConfig {
    /// 全局启用
    #[serde(default = "default_true")]
    pub enabled: bool,
    /// 熔断规则列表
    #[serde(default)]
    pub rules: Vec<BreakerRule>,
    /// 全局冷却时长（秒），规则未指定时使用
    #[serde(default = "default_cooldown")]
    pub default_cooldown_secs: u64,
    /// 全局半开探测次数
    #[serde(default = "default_probe_count")]
    pub default_probe_success_count: u8,
}

impl Default for BlackholeBreakerConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            rules: get_predefined_breaker_rules(),
            default_cooldown_secs: default_cooldown(),
            default_probe_success_count: default_probe_count(),
        }
    }
}

// ── 熔断器运行时状态 ────────────────────────────────────────────────

/// 单条熔断规则的运行时状态
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BreakerRuntimeState {
    /// 规则 ID
    pub rule_id: String,
    /// 当前状态
    pub state: BreakerState,
    /// 连续失败计数
    pub consecutive_failures: u32,
    /// 窗口内总请求数
    pub window_total: u32,
    /// 窗口内失败请求数
    pub window_failures: u32,
    /// 窗口起始时间
    pub window_start: Option<u64>,
    /// 进入 Open 状态的时间
    pub opened_at: Option<u64>,
    /// 半开探测成功计数
    pub probe_successes: u8,
    /// 半开探测失败计数
    pub probe_failures: u8,
    /// 历史触发次数
    pub trip_count: u32,
    /// 最后一次状态变更时间
    pub last_state_change: u64,
}

impl BreakerRuntimeState {
    fn new(rule_id: &str) -> Self {
        Self {
            rule_id: rule_id.to_string(),
            state: BreakerState::Closed,
            consecutive_failures: 0,
            window_total: 0,
            window_failures: 0,
            window_start: None,
            opened_at: None,
            probe_successes: 0,
            probe_failures: 0,
            trip_count: 0,
            last_state_change: now_secs(),
        }
    }
}

fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

// ── 熔断器管理器 ────────────────────────────────────────────────────

pub struct BlackholeBreakerManager {
    config: Arc<RwLock<BlackholeBreakerConfig>>,
    states: Arc<RwLock<HashMap<String, BreakerRuntimeState>>>,
}

impl BlackholeBreakerManager {
    pub fn new() -> Self {
        Self {
            config: Arc::new(RwLock::new(BlackholeBreakerConfig::default())),
            states: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn get_config(&self) -> BlackholeBreakerConfig {
        self.config.read().await.clone()
    }

    pub async fn update_config(&self, config: BlackholeBreakerConfig) {
        *self.config.write().await = config;
        log::info!("[BlackholeBreaker] 配置已更新");
    }

    /// 记录一次请求结果
    pub async fn record_result(&self, rule_id: &str, success: bool) {
        let config = self.config.read().await;
        let rule = match config.rules.iter().find(|r| r.id == rule_id) {
            Some(r) => r,
            None => return,
        };

        let mut states = self.states.write().await;
        let state = states
            .entry(rule_id.to_string())
            .or_insert_with(|| BreakerRuntimeState::new(rule_id));

        let now = now_secs();

        // 滑动窗口重置
        if state.window_start.is_none()
            || now - state.window_start.unwrap() > rule.trigger.window_secs
        {
            state.window_start = Some(now);
            state.window_total = 0;
            state.window_failures = 0;
        }

        state.window_total += 1;

        if success {
            state.consecutive_failures = 0;
            state.window_failures += 0; // no-op clarity

            match state.state {
                BreakerState::HalfOpen => {
                    state.probe_successes += 1;
                    if state.probe_successes >= rule.probe_success_count {
                        state.state = BreakerState::Closed;
                        state.probe_successes = 0;
                        state.probe_failures = 0;
                        state.last_state_change = now;
                        log::info!("[BlackholeBreaker] 规则 {} 探测成功，恢复闭合", rule_id);
                    }
                }
                _ => {}
            }
        } else {
            state.consecutive_failures += 1;
            state.window_failures += 1;

            match state.state {
                BreakerState::Closed => {
                    if should_trip(state, &rule.trigger) {
                        state.state = BreakerState::Open;
                        state.opened_at = Some(now);
                        state.trip_count += 1;
                        state.last_state_change = now;
                        log::warn!(
                            "[BlackholeBreaker] 规则 {} 触发熔断！连续失败={}, 窗口失败率={:.0}%",
                            rule_id,
                            state.consecutive_failures,
                            rate(state.window_failures, state.window_total) * 100.0
                        );
                    }
                }
                BreakerState::HalfOpen => {
                    state.probe_failures += 1;
                    // 半开探测失败，立即回到 Open
                    state.state = BreakerState::Open;
                    state.opened_at = Some(now);
                    state.probe_successes = 0;
                    state.probe_failures = 0;
                    state.trip_count += 1;
                    state.last_state_change = now;
                    log::warn!("[BlackholeBreaker] 规则 {} 半开探测失败，重新熔断", rule_id);
                }
                BreakerState::Open => {}
            }
        }
    }

    /// 检查某条规则当前是否应熔断（返回 true 表示应黑洞）
    pub async fn should_block(&self, rule_id: &str) -> bool {
        let config = self.config.read().await;
        if !config.enabled {
            return false;
        }

        let rule = match config.rules.iter().find(|r| r.id == rule_id) {
            Some(r) if r.enabled => r,
            _ => return false,
        };

        let mut states = self.states.write().await;
        let state = states
            .entry(rule_id.to_string())
            .or_insert_with(|| BreakerRuntimeState::new(rule_id));

        let now = now_secs();

        // 检查 Open → HalfOpen 转换
        if state.state == BreakerState::Open {
            let cooldown = rule.cooldown_secs;
            if let Some(opened_at) = state.opened_at {
                if now - opened_at >= cooldown {
                    state.state = BreakerState::HalfOpen;
                    state.probe_successes = 0;
                    state.probe_failures = 0;
                    state.last_state_change = now;
                    log::info!("[BlackholeBreaker] 规则 {} 冷却结束，进入半开探测", rule_id);
                }
            }
        }

        state.state == BreakerState::Open
    }

    /// 检查域名是否被任何规则熔断
    pub async fn should_block_domain(&self, domain: &str) -> bool {
        let config = self.config.read().await;
        if !config.enabled {
            return false;
        }

        for rule in &config.rules {
            if !rule.enabled {
                continue;
            }
            if rule.domain_patterns.iter().any(|p| domain_matches(domain, p)) {
                if self.should_block(&rule.id).await {
                    return true;
                }
            }
        }
        false
    }

    /// 检查节点是否被任何规则熔断
    pub async fn should_block_node(&self, node_name: &str) -> bool {
        let config = self.config.read().await;
        if !config.enabled {
            return false;
        }

        for rule in &config.rules {
            if !rule.enabled || rule.node_patterns.is_empty() {
                continue;
            }
            if rule.node_patterns.iter().any(|p| node_matches(node_name, p)) {
                if self.should_block(&rule.id).await {
                    return true;
                }
            }
        }
        false
    }

    /// 获取所有规则的运行时状态
    pub async fn get_all_states(&self) -> Vec<BreakerRuntimeState> {
        let config = self.config.read().await;
        let mut states = self.states.write().await;

        // 确保每条规则都有状态
        for rule in &config.rules {
            states
                .entry(rule.id.clone())
                .or_insert_with(|| BreakerRuntimeState::new(&rule.id));
        }

        // 检查 Open → HalfOpen 转换
        let now = now_secs();
        for rule in &config.rules {
            if let Some(state) = states.get_mut(&rule.id) {
                if state.state == BreakerState::Open {
                    if let Some(opened_at) = state.opened_at {
                        if now - opened_at >= rule.cooldown_secs {
                            state.state = BreakerState::HalfOpen;
                            state.probe_successes = 0;
                            state.probe_failures = 0;
                            state.last_state_change = now;
                        }
                    }
                }
            }
        }

        states.values().cloned().collect()
    }

    /// 手动重置某条规则为 Closed
    pub async fn reset_rule(&self, rule_id: &str) -> Result<()> {
        let mut states = self.states.write().await;
        if let Some(state) = states.get_mut(rule_id) {
            state.state = BreakerState::Closed;
            state.consecutive_failures = 0;
            state.window_total = 0;
            state.window_failures = 0;
            state.opened_at = None;
            state.probe_successes = 0;
            state.probe_failures = 0;
            state.last_state_change = now_secs();
            log::info!("[BlackholeBreaker] 规则 {} 手动重置为闭合", rule_id);
        }
        Ok(())
    }

    /// 手动触发某条规则熔断
    pub async fn trip_rule(&self, rule_id: &str) -> Result<()> {
        let mut states = self.states.write().await;
        let state = states
            .entry(rule_id.to_string())
            .or_insert_with(|| BreakerRuntimeState::new(rule_id));
        let now = now_secs();
        state.state = BreakerState::Open;
        state.opened_at = Some(now);
        state.trip_count += 1;
        state.last_state_change = now;
        log::warn!("[BlackholeBreaker] 规则 {} 手动触发熔断", rule_id);
        Ok(())
    }

    /// 生成当前应注入的 Mihomo REJECT-DROP 规则列表
    /// 返回 (domain_pattern, "REJECT-DROP") 对
    pub async fn generate_reject_rules(&self) -> Vec<(String, String)> {
        let config = self.config.read().await;
        if !config.enabled {
            return Vec::new();
        }

        let mut rules = Vec::new();
        for rule in &config.rules {
            if !rule.enabled {
                continue;
            }
            if self.should_block(&rule.id).await {
                for pattern in &rule.domain_patterns {
                    rules.push((pattern.clone(), "REJECT-DROP".to_string()));
                }
            }
        }
        rules
    }

    /// 根据欺诈评分检查并触发熔断
    /// 当 IP 信誉检测到高欺诈评分时调用，直接与规则的 max_fraud_score 比较
    pub async fn record_fraud_score(&self, domain: &str, fraud_score: u8) {
        let config = self.config.read().await;
        if !config.enabled {
            return;
        }

        for rule in &config.rules {
            if !rule.enabled {
                continue;
            }
            let max_score = match rule.trigger.max_fraud_score {
                Some(s) => s,
                None => continue,
            };
            if fraud_score < max_score {
                continue;
            }
            // 域名匹配
            if !rule.domain_patterns.iter().any(|p| domain_matches(domain, p)) {
                continue;
            }
            // 欺诈评分超阈值，直接触发熔断
            let mut states = self.states.write().await;
            let state = states
                .entry(rule.id.clone())
                .or_insert_with(|| BreakerRuntimeState::new(&rule.id));
            if state.state == BreakerState::Closed {
                let now = now_secs();
                state.state = BreakerState::Open;
                state.opened_at = Some(now);
                state.trip_count += 1;
                state.last_state_change = now;
                log::warn!(
                    "[BlackholeBreaker] 规则 {} 因欺诈评分 {} >= {} 触发熔断",
                    rule.id, fraud_score, max_score
                );
            }
        }
    }
}

// ── 辅助函数 ────────────────────────────────────────────────────────

fn should_trip(state: &BreakerRuntimeState, trigger: &BreakerTrigger) -> bool {
    // 条件1：连续失败次数
    if state.consecutive_failures >= trigger.consecutive_failures {
        return true;
    }

    // 条件2：窗口内失败率
    if state.window_total >= trigger.min_requests {
        let rate = rate(state.window_failures, state.window_total);
        if rate >= trigger.failure_rate {
            return true;
        }
    }

    false
}

fn rate(failures: u32, total: u32) -> f64 {
    if total == 0 {
        0.0
    } else {
        failures as f64 / total as f64
    }
}

/// 域名匹配（复用 session_affinity）
fn domain_matches(domain: &str, pattern: &str) -> bool {
    if pattern.starts_with("*.") {
        let suffix = &pattern[2..];
        domain.ends_with(suffix) || domain == suffix
    } else if pattern.starts_with('*') {
        let suffix = &pattern[1..];
        domain.ends_with(suffix)
    } else {
        domain == pattern
    }
}

/// 节点名称匹配（支持通配符）
fn node_matches(node_name: &str, pattern: &str) -> bool {
    if pattern.starts_with('*') {
        let suffix = &pattern[1..];
        node_name.ends_with(suffix)
    } else {
        node_name.eq_ignore_ascii_case(pattern)
    }
}

/// 预定义熔断规则
pub fn get_predefined_breaker_rules() -> Vec<BreakerRule> {
    vec![
        // AI 服务熔断：欺诈评分过高时黑洞
        BreakerRule {
            id: "ai-fraud-breaker".to_string(),
            enabled: true,
            domain_patterns: vec![
                "*.openai.com".to_string(),
                "*.anthropic.com".to_string(),
                "*.claude.ai".to_string(),
            ],
            node_patterns: vec![],
            trigger: BreakerTrigger {
                consecutive_failures: 3,
                failure_rate: 0.5,
                window_secs: 120,
                min_requests: 5,
                max_fraud_score: Some(80),
            },
            cooldown_secs: 600, // 10 分钟
            probe_success_count: 2,
            description: "AI 服务熔断 — 出口 IP 欺诈评分过高或频繁失败时黑洞".to_string(),
        },
        // 金融服务熔断
        BreakerRule {
            id: "finance-breaker".to_string(),
            enabled: true,
            domain_patterns: vec![
                "*.stripe.com".to_string(),
                "*.paypal.com".to_string(),
            ],
            node_patterns: vec![],
            trigger: BreakerTrigger {
                consecutive_failures: 2,
                failure_rate: 0.4,
                window_secs: 60,
                min_requests: 3,
                max_fraud_score: Some(60),
            },
            cooldown_secs: 900, // 15 分钟
            probe_success_count: 3,
            description: "金融服务熔断 — 严格条件，快速触发，长冷却".to_string(),
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_should_trip_consecutive() {
        let state = BreakerRuntimeState {
            consecutive_failures: 5,
            window_total: 10,
            window_failures: 5,
            ..BreakerRuntimeState::new("test")
        };
        let trigger = BreakerTrigger {
            consecutive_failures: 5,
            failure_rate: 0.8,
            window_secs: 60,
            min_requests: 10,
            max_fraud_score: None,
        };
        assert!(should_trip(&state, &trigger));
    }

    #[test]
    fn test_should_trip_rate() {
        let state = BreakerRuntimeState {
            consecutive_failures: 2,
            window_total: 20,
            window_failures: 14,
            ..BreakerRuntimeState::new("test")
        };
        let trigger = BreakerTrigger {
            consecutive_failures: 10,
            failure_rate: 0.6,
            window_secs: 60,
            min_requests: 10,
            max_fraud_score: None,
        };
        assert!(should_trip(&state, &trigger)); // 14/20 = 0.7 >= 0.6
    }

    #[test]
    fn test_should_not_trip() {
        let state = BreakerRuntimeState {
            consecutive_failures: 1,
            window_total: 5,
            window_failures: 1,
            ..BreakerRuntimeState::new("test")
        };
        let trigger = BreakerTrigger {
            consecutive_failures: 5,
            failure_rate: 0.6,
            window_secs: 60,
            min_requests: 10,
            max_fraud_score: None,
        };
        assert!(!should_trip(&state, &trigger));
    }

    #[tokio::test]
    async fn test_circuit_breaker_lifecycle() {
        let manager = BlackholeBreakerManager::new();

        // 初始应不阻塞
        assert!(!manager.should_block("ai-fraud-breaker").await);

        // 连续失败触发熔断
        for _ in 0..5 {
            manager.record_result("ai-fraud-breaker", false).await;
        }
        assert!(manager.should_block("ai-fraud-breaker").await);

        // 手动重置
        manager.reset_rule("ai-fraud-breaker").await.unwrap();
        assert!(!manager.should_block("ai-fraud-breaker").await);
    }

    #[test]
    fn test_domain_matches() {
        assert!(domain_matches("api.openai.com", "*.openai.com"));
        assert!(domain_matches("openai.com", "*.openai.com"));
        assert!(!domain_matches("example.com", "*.openai.com"));
    }

    #[tokio::test]
    async fn test_fraud_score_trips_breaker() {
        let manager = BlackholeBreakerManager::new();
        let config = BlackholeBreakerConfig {
            enabled: true,
            rules: vec![BreakerRule {
                id: "ai-fraud-breaker".to_string(),
                enabled: true,
                domain_patterns: vec!["*.openai.com".to_string()],
                node_patterns: vec![],
                trigger: BreakerTrigger {
                    consecutive_failures: 100,
                    failure_rate: 1.0,
                    window_secs: 60,
                    min_requests: 100,
                    max_fraud_score: Some(80),
                },
                cooldown_secs: 600,
                probe_success_count: 2,
                description: "test".to_string(),
            }],
            default_cooldown_secs: 600,
            default_probe_success_count: 2,
        };
        manager.update_config(config).await;

        // Below threshold — should not trip
        manager.record_fraud_score("api.openai.com", 70).await;
        assert!(!manager.should_block("ai-fraud-breaker").await);

        // At threshold — should trip
        manager.record_fraud_score("api.openai.com", 85).await;
        assert!(manager.should_block("ai-fraud-breaker").await);
    }
}
