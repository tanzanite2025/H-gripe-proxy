use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use tokio::sync::RwLock;

use crate::process::AsyncHandler;

/// 会话绑定配置
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionAffinityConfig {
    /// 启用会话绑定
    pub enabled: bool,
    /// 域名级绑定规则
    #[serde(alias = "domain_rules")]
    pub domain_rules: Vec<DomainBindingRule>,
    /// 进程级绑定规则
    #[serde(alias = "process_rules")]
    pub process_rules: Vec<ProcessBindingRule>,
    /// 连接级绑定配置
    #[serde(alias = "connection_binding")]
    pub connection_binding: ConnectionBindingConfig,
}

impl Default for SessionAffinityConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            domain_rules: get_predefined_rules(),
            process_rules: vec![],
            connection_binding: ConnectionBindingConfig::default(),
        }
    }
}

/// 域名绑定规则
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DomainBindingRule {
    /// 域名模式（支持通配符）
    #[serde(alias = "domain_pattern")]
    pub domain_pattern: String,
    /// 是否启用
    pub enabled: bool,
    /// 绑定的节点名称（None 表示自动选择后绑定）
    #[serde(alias = "bound_node")]
    pub bound_node: Option<String>,
    /// 绑定时长（秒，0 表示永久）
    pub ttl: u64,
    /// 故障转移策略
    #[serde(alias = "fallback_policy")]
    pub fallback_policy: FallbackPolicy,
    /// 描述
    pub description: String,
}

/// 进程绑定规则
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProcessBindingRule {
    /// 进程名称（如 "Steam.exe"）
    #[serde(alias = "process_name")]
    pub process_name: String,
    /// 是否启用
    pub enabled: bool,
    /// 绑定的节点名称
    #[serde(alias = "bound_node")]
    pub bound_node: Option<String>,
    /// 绑定时长（秒）
    pub ttl: u64,
    /// 故障转移策略
    #[serde(alias = "fallback_policy")]
    pub fallback_policy: FallbackPolicy,
    /// 描述
    pub description: String,
}

/// 连接级绑定配置
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConnectionBindingConfig {
    /// 启用连接级绑定
    pub enabled: bool,
    /// 跟踪方式
    #[serde(alias = "track_by")]
    pub track_by: TrackBy,
    /// 超时时间（秒）
    pub timeout: u64,
}

impl Default for ConnectionBindingConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            track_by: TrackBy::SourceIpPort,
            timeout: 3600, // 1小时
        }
    }
}

/// 跟踪方式
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TrackBy {
    /// 源 IP + 端口
    SourceIpPort,
    /// 会话 ID
    SessionId,
}


/// 故障转移策略
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FallbackPolicy {
    /// 手动确认（需要用户手动选择新节点）
    Manual,
    /// 自动重试当前节点
    AutoRetry,
    /// 自动切换到备用节点
    AutoSwitch,
}

/// 节点绑定记录
#[derive(Debug, Clone)]
pub struct NodeBinding {
    /// 节点 ID
    pub node_id: String,
    /// 绑定时间
    pub bound_at: SystemTime,
    /// 过期时间
    pub expires_at: Option<SystemTime>,
    /// 故障转移策略
    pub fallback_policy: FallbackPolicy,
}

/// 连接 ID
#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub struct ConnectionId {
    pub source_ip: String,
    pub source_port: u16,
}

/// 绑定信息（用于前端展示）
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BindingInfo {
    /// 绑定类型
    pub binding_type: String,
    /// 键（域名/进程名/连接ID）
    pub key: String,
    /// 节点 ID
    pub node_id: String,
    /// 绑定时间（Unix 时间戳）
    pub bound_at: u64,
    /// 过期时间（Unix 时间戳，None 表示永久）
    pub expires_at: Option<u64>,
    /// 剩余时间（秒）
    pub remaining_seconds: Option<u64>,
}

/// 会话绑定管理器
pub struct SessionAffinityManager {
    /// 配置
    config: Arc<RwLock<SessionAffinityConfig>>,
    /// 域名 -> 节点绑定
    domain_bindings: Arc<RwLock<HashMap<String, NodeBinding>>>,
    /// 域名规则 -> 节点绑定（用于稳定组手动选择回写）
    domain_rule_bindings: Arc<RwLock<HashMap<String, NodeBinding>>>,
    /// 进程 -> 节点绑定
    process_bindings: Arc<RwLock<HashMap<String, NodeBinding>>>,
    /// 连接 -> 节点绑定
    connection_bindings: Arc<RwLock<HashMap<ConnectionId, NodeBinding>>>,
}

impl SessionAffinityManager {
    /// 创建新的会话绑定管理器
    pub fn new() -> Self {
        Self::new_with_config(SessionAffinityConfig::default())
    }

    pub fn new_with_config(config: SessionAffinityConfig) -> Self {
        Self {
            config: Arc::new(RwLock::new(config)),
            domain_bindings: Arc::new(RwLock::new(HashMap::new())),
            domain_rule_bindings: Arc::new(RwLock::new(HashMap::new())),
            process_bindings: Arc::new(RwLock::new(HashMap::new())),
            connection_bindings: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// 更新配置
    pub async fn update_config(&self, config: SessionAffinityConfig) -> Result<()> {
        *self.config.write().await = config;
        log::info!("[SessionAffinity] 配置已更新");
        Ok(())
    }

    fn create_binding_from_domain_rule(rule: &DomainBindingRule, node_id: String) -> NodeBinding {
        NodeBinding {
            node_id,
            bound_at: SystemTime::now(),
            expires_at: if rule.ttl > 0 {
                Some(SystemTime::now() + Duration::from_secs(rule.ttl))
            } else {
                None
            },
            fallback_policy: rule.fallback_policy.clone(),
        }
    }

    pub async fn record_domain_rule_binding(&self, domain_pattern: &str, node_id: String) -> Result<()> {
        let config = self.config.read().await;

        if !config.enabled {
            return Err(anyhow!("会话绑定未启用"));
        }

        let rule = config
            .domain_rules
            .iter()
            .find(|rule| rule.enabled && rule.domain_pattern == domain_pattern)
            .cloned()
            .ok_or_else(|| anyhow!("未找到匹配的域名绑定规则: {}", domain_pattern))?;

        drop(config);

        let binding = Self::create_binding_from_domain_rule(&rule, node_id.clone());

        self.domain_rule_bindings
            .write()
            .await
            .insert(domain_pattern.to_string(), binding.clone());

        let mut domain_bindings = self.domain_bindings.write().await;
        for (domain, existing_binding) in domain_bindings.iter_mut() {
            if domain_matches(domain, domain_pattern) {
                *existing_binding = binding.clone();
            }
        }

        log::info!(
            "[SessionAffinity] 域名规则 {} 运行态绑定到节点 {}",
            domain_pattern,
            node_id
        );

        Ok(())
    }


    /// 为域名选择节点（考虑会话绑定）
    pub async fn select_node_for_domain(
        &self,
        domain: &str,
        available_nodes: &[String],
    ) -> Result<String> {
        let config = self.config.read().await;

        if !config.enabled {
            // 会话绑定未启用，使用默认选择
            return Ok(available_nodes
                .first()
                .ok_or_else(|| anyhow!("没有可用节点"))?
                .clone());
        }

        // 1. 查找匹配的域名规则
        let rule = config
            .domain_rules
            .iter()
            .find(|r| r.enabled && domain_matches(domain, &r.domain_pattern));

        if let Some(rule) = rule {
            let rule_bindings = self.domain_rule_bindings.read().await;
            if let Some(binding) = rule_bindings.get(&rule.domain_pattern).cloned()
                && !self.is_binding_expired(&binding)
                && available_nodes.contains(&binding.node_id)
            {
                drop(rule_bindings);
                let mut bindings = self.domain_bindings.write().await;
                bindings.insert(domain.to_string(), binding.clone());
                log::info!(
                    "[SessionAffinity] 域名 {} 使用域名规则 {} 的运行态绑定节点 {}",
                    domain,
                    rule.domain_pattern,
                    binding.node_id
                );
                return Ok(binding.node_id);
            }
            drop(rule_bindings);

            // 2. 检查是否已有绑定
            let bindings = self.domain_bindings.read().await;
            if let Some(binding) = bindings.get(domain).cloned() {
                // 检查绑定是否过期
                if !self.is_binding_expired(&binding) {
                    // 检查节点是否仍然可用
                    if available_nodes.contains(&binding.node_id) {
                        log::debug!(
                            "[SessionAffinity] 域名 {} 使用已绑定节点 {}",
                            domain,
                            binding.node_id
                        );
                        return Ok(binding.node_id.clone());
                    } else {
                        // 节点不可用，根据故障转移策略处理
                        drop(bindings); // 释放读锁
                        return self
                            .handle_node_unavailable(domain, &binding, available_nodes)
                            .await;
                    }
                }
            }
            drop(bindings); // 释放读锁

            // 3. 没有绑定或已过期，选择新节点
            let node = if let Some(ref bound_node) = rule.bound_node {
                // 使用指定节点
                if available_nodes.contains(bound_node) {
                    bound_node.clone()
                } else {
                    return Err(anyhow!("指定节点 {} 不可用", bound_node));
                }
            } else {
                // 自动选择节点（使用第一个可用节点）
                available_nodes
                    .first()
                    .ok_or_else(|| anyhow!("没有可用节点"))?
                    .clone()
            };

            // 4. 创建绑定
            let binding = Self::create_binding_from_domain_rule(rule, node.clone());

            // 5. 保存绑定
            let mut bindings = self.domain_bindings.write().await;
            bindings.insert(domain.to_string(), binding);

            log::info!("[SessionAffinity] 域名 {} 绑定到节点 {}", domain, node);

            Ok(node)
        } else {
            // 没有匹配的规则，使用默认选择
            Ok(available_nodes
                .first()
                .ok_or_else(|| anyhow!("没有可用节点"))?
                .clone())
        }
    }

    /// 检查绑定是否过期
    fn is_binding_expired(&self, binding: &NodeBinding) -> bool {
        if let Some(expires_at) = binding.expires_at {
            SystemTime::now() > expires_at
        } else {
            false
        }
    }


    /// 处理节点不可用
    async fn handle_node_unavailable(
        &self,
        domain: &str,
        binding: &NodeBinding,
        available_nodes: &[String],
    ) -> Result<String> {
        match binding.fallback_policy {
            FallbackPolicy::Manual => {
                // 需要用户手动确认
                log::warn!(
                    "[SessionAffinity] 域名 {} 的节点 {} 不可用，需要手动选择新节点",
                    domain,
                    binding.node_id
                );
                Err(anyhow!("节点不可用，需要手动选择新节点"))
            }
            FallbackPolicy::AutoRetry => {
                // 自动重试当前节点（返回错误，让上层重试）
                log::warn!(
                    "[SessionAffinity] 域名 {} 的节点 {} 不可用，正在重试",
                    domain,
                    binding.node_id
                );
                Err(anyhow!("节点不可用，正在重试"))
            }
            FallbackPolicy::AutoSwitch => {
                // 自动切换到备用节点
                let new_node = available_nodes
                    .first()
                    .ok_or_else(|| anyhow!("没有可用节点"))?
                    .clone();

                // 更新绑定
                let mut bindings = self.domain_bindings.write().await;
                let mut new_binding = binding.clone();
                new_binding.node_id = new_node.clone();
                new_binding.bound_at = SystemTime::now();
                bindings.insert(domain.to_string(), new_binding);

                log::warn!(
                    "[SessionAffinity] 域名 {} 自动切换到节点 {}",
                    domain,
                    new_node
                );

                Ok(new_node)
            }
        }
    }

    /// 获取所有绑定信息
    pub async fn get_all_bindings(&self) -> Result<Vec<BindingInfo>> {
        let mut bindings = Vec::new();

        // 域名规则绑定
        let domain_rule_bindings = self.domain_rule_bindings.read().await;
        for (domain_pattern, binding) in domain_rule_bindings.iter() {
            let key = format!("rule:{domain_pattern}");
            bindings.push(self.binding_to_info("domain-rule", &key, binding));
        }

        // 域名绑定
        let domain_bindings = self.domain_bindings.read().await;
        for (domain, binding) in domain_bindings.iter() {
            bindings.push(self.binding_to_info("domain", domain, binding));
        }

        // 进程绑定
        let process_bindings = self.process_bindings.read().await;
        for (process, binding) in process_bindings.iter() {
            bindings.push(self.binding_to_info("process", process, binding));
        }

        // 连接绑定
        let connection_bindings = self.connection_bindings.read().await;
        for (conn_id, binding) in connection_bindings.iter() {
            let key = format!("{}:{}", conn_id.source_ip, conn_id.source_port);
            bindings.push(self.binding_to_info("connection", &key, binding));
        }

        Ok(bindings)
    }

    /// 将绑定转换为信息
    fn binding_to_info(&self, binding_type: &str, key: &str, binding: &NodeBinding) -> BindingInfo {
        let bound_at = binding
            .bound_at
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let (expires_at, remaining_seconds) = if let Some(exp) = binding.expires_at {
            let exp_secs = exp
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();
            let remaining = exp
                .duration_since(SystemTime::now())
                .ok()
                .map(|d| d.as_secs());
            (Some(exp_secs), remaining)
        } else {
            (None, None)
        };

        BindingInfo {
            binding_type: binding_type.to_string(),
            key: key.to_string(),
            node_id: binding.node_id.clone(),
            bound_at,
            expires_at,
            remaining_seconds,
        }
    }

    /// 清除域名绑定
    pub async fn clear_domain_binding(&self, domain: &str) -> Result<()> {
        if let Some(domain_pattern) = domain.strip_prefix("rule:") {
            let mut rule_bindings = self.domain_rule_bindings.write().await;
            rule_bindings.remove(domain_pattern);

            let mut bindings = self.domain_bindings.write().await;
            bindings.retain(|bound_domain, _| !domain_matches(bound_domain, domain_pattern));

            log::info!("[SessionAffinity] 已清除域名规则 {} 的运行态绑定", domain_pattern);
            return Ok(());
        }

        let mut bindings = self.domain_bindings.write().await;
        bindings.remove(domain);
        log::info!("[SessionAffinity] 已清除域名 {} 的绑定", domain);
        Ok(())
    }

    /// 清除所有过期绑定
    pub async fn cleanup_expired_bindings(&self) -> Result<()> {
        // 清理域名绑定
        let mut domain_bindings = self.domain_bindings.write().await;
        domain_bindings.retain(|_, binding| !self.is_binding_expired(binding));

        // 清理域名规则绑定
        let mut domain_rule_bindings = self.domain_rule_bindings.write().await;
        domain_rule_bindings.retain(|_, binding| !self.is_binding_expired(binding));

        // 清理进程绑定
        let mut process_bindings = self.process_bindings.write().await;
        process_bindings.retain(|_, binding| !self.is_binding_expired(binding));

        // 清理连接绑定
        let mut connection_bindings = self.connection_bindings.write().await;
        connection_bindings.retain(|_, binding| !self.is_binding_expired(binding));

        log::debug!("[SessionAffinity] 已清理过期绑定");
        Ok(())
    }
}


/// 检查域名是否匹配规则
pub fn domain_matches(domain: &str, pattern: &str) -> bool {
    // 支持通配符匹配
    // 例如: "*.openai.com" 匹配 "chat.openai.com"

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

/// 获取预定义的会话绑定规则
pub fn get_predefined_rules() -> Vec<DomainBindingRule> {
    vec![
        // AI 服务（极高风控）
        DomainBindingRule {
            domain_pattern: "*.openai.com".to_string(),
            enabled: true,
            bound_node: None,
            ttl: 86400, // 24小时
            fallback_policy: FallbackPolicy::Manual,
            description: "ChatGPT - 必须单节点，24小时内不允许切换".to_string(),
        },
        DomainBindingRule {
            domain_pattern: "*.anthropic.com".to_string(),
            enabled: true,
            bound_node: None,
            ttl: 86400,
            fallback_policy: FallbackPolicy::Manual,
            description: "Claude - 必须单节点".to_string(),
        },
        // 游戏平台（高风控）
        DomainBindingRule {
            domain_pattern: "*.steampowered.com".to_string(),
            enabled: true,
            bound_node: None,
            ttl: 604800, // 7天
            fallback_policy: FallbackPolicy::Manual,
            description: "Steam - 必须单节点，7天内不允许切换".to_string(),
        },
        DomainBindingRule {
            domain_pattern: "*.steamcommunity.com".to_string(),
            enabled: true,
            bound_node: None,
            ttl: 604800,
            fallback_policy: FallbackPolicy::Manual,
            description: "Steam Community - 必须单节点".to_string(),
        },
        DomainBindingRule {
            domain_pattern: "*.epicgames.com".to_string(),
            enabled: true,
            bound_node: None,
            ttl: 604800,
            fallback_policy: FallbackPolicy::Manual,
            description: "Epic Games - 必须单节点".to_string(),
        },
        DomainBindingRule {
            domain_pattern: "*.riotgames.com".to_string(),
            enabled: true,
            bound_node: None,
            ttl: 604800,
            fallback_policy: FallbackPolicy::Manual,
            description: "Riot Games - 必须单节点".to_string(),
        },
        // 金融服务（极高风控）
        DomainBindingRule {
            domain_pattern: "*.stripe.com".to_string(),
            enabled: true,
            bound_node: None,
            ttl: 2592000, // 30天
            fallback_policy: FallbackPolicy::Manual,
            description: "Stripe - 必须单节点，30天内不允许切换".to_string(),
        },
        DomainBindingRule {
            domain_pattern: "*.paypal.com".to_string(),
            enabled: true,
            bound_node: None,
            ttl: 2592000,
            fallback_policy: FallbackPolicy::Manual,
            description: "PayPal - 必须单节点".to_string(),
        },
        // 社交媒体（中风控）
        DomainBindingRule {
            domain_pattern: "*.twitter.com".to_string(),
            enabled: true,
            bound_node: None,
            ttl: 86400,
            fallback_policy: FallbackPolicy::AutoSwitch,
            description: "Twitter - 建议单节点".to_string(),
        },
        DomainBindingRule {
            domain_pattern: "*.x.com".to_string(),
            enabled: true,
            bound_node: None,
            ttl: 86400,
            fallback_policy: FallbackPolicy::AutoSwitch,
            description: "X (Twitter) - 建议单节点".to_string(),
        },
        DomainBindingRule {
            domain_pattern: "*.facebook.com".to_string(),
            enabled: true,
            bound_node: None,
            ttl: 86400,
            fallback_policy: FallbackPolicy::AutoSwitch,
            description: "Facebook - 建议单节点".to_string(),
        },
        DomainBindingRule {
            domain_pattern: "*.instagram.com".to_string(),
            enabled: true,
            bound_node: None,
            ttl: 86400,
            fallback_policy: FallbackPolicy::AutoSwitch,
            description: "Instagram - 建议单节点".to_string(),
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_domain_matches() {
        assert!(domain_matches("chat.openai.com", "*.openai.com"));
        assert!(domain_matches("openai.com", "*.openai.com"));
        assert!(!domain_matches("openai.org", "*.openai.com"));
        assert!(domain_matches("example.com", "example.com"));
        assert!(!domain_matches("sub.example.com", "example.com"));
    }

    #[tokio::test]
    async fn test_session_affinity_basic() {
        let manager = SessionAffinityManager::new();
        let nodes = vec!["node1".to_string(), "node2".to_string()];

        // 第一次选择
        let node1 = manager
            .select_node_for_domain("chat.openai.com", &nodes)
            .await
            .unwrap();

        // 第二次选择应该返回相同节点
        let node2 = manager
            .select_node_for_domain("chat.openai.com", &nodes)
            .await
            .unwrap();

        assert_eq!(node1, node2);
    }

    #[tokio::test]
    async fn test_get_bindings() {
        let manager = SessionAffinityManager::new();
        let nodes = vec!["node1".to_string()];

        // 创建绑定
        manager
            .select_node_for_domain("chat.openai.com", &nodes)
            .await
            .unwrap();

        // 获取绑定
        let bindings = manager.get_all_bindings().await.unwrap();
        assert_eq!(bindings.len(), 1);
        assert_eq!(bindings[0].binding_type, "domain");
        assert_eq!(bindings[0].key, "chat.openai.com");
    }
}


/// 进程检测模块
#[cfg(target_os = "windows")]
pub mod process_detection {
    use anyhow::Result;
    use std::process::Command;

    /// 根据端口获取进程名称
    pub fn get_process_name_by_port(port: u16) -> Result<String> {
        let output = Command::new("netstat")
            .args(&["-ano"])
            .output()?;

        let stdout = String::from_utf8_lossy(&output.stdout);

        // 解析 netstat 输出，找到对应端口的 PID
        for line in stdout.lines() {
            if line.contains(&format!(":{}", port)) && line.contains("ESTABLISHED") {
                // 提取 PID（最后一列）
                if let Some(pid_str) = line.split_whitespace().last() {
                    if let Ok(pid) = pid_str.parse::<u32>() {
                        return get_process_name_by_pid(pid);
                    }
                }
            }
        }

        Err(anyhow::anyhow!("未找到进程"))
    }

    /// 根据 PID 获取进程名称
    fn get_process_name_by_pid(pid: u32) -> Result<String> {
        let output = Command::new("tasklist")
            .args(&["/FI", &format!("PID eq {}", pid), "/FO", "CSV", "/NH"])
            .output()?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        
        // 解析 CSV 输出
        if let Some(line) = stdout.lines().next() {
            // CSV 格式: "进程名","PID","会话名","会话#","内存使用"
            let parts: Vec<&str> = line.split(',').collect();
            if !parts.is_empty() {
                let process_name = parts[0].trim_matches('"');
                return Ok(process_name.to_string());
            }
        }

        Err(anyhow::anyhow!("未找到进程"))
    }
}

#[cfg(target_os = "linux")]
pub mod process_detection {
    use anyhow::Result;
    use std::fs;

    /// 根据端口获取进程名称
    pub fn get_process_name_by_port(port: u16) -> Result<String> {
        // 读取 /proc/net/tcp
        let tcp_content = fs::read_to_string("/proc/net/tcp")?;

        // 解析找到对应端口的 inode
        for line in tcp_content.lines().skip(1) {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() > 9 {
                let local_address = parts[1];
                // 端口是十六进制格式
                let port_hex = format!("{:04X}", port);
                if local_address.ends_with(&format!(":{}", port_hex)) {
                    let inode = parts[9];
                    return find_process_by_inode(inode);
                }
            }
        }

        Err(anyhow::anyhow!("未找到进程"))
    }

    /// 根据 inode 查找进程
    fn find_process_by_inode(inode: &str) -> Result<String> {
        // 遍历 /proc 目录
        for entry in fs::read_dir("/proc")? {
            let entry = entry?;
            let path = entry.path();

            // 只处理数字目录（PID）
            if let Some(pid_str) = path.file_name().and_then(|n| n.to_str()) {
                if pid_str.chars().all(|c| c.is_ascii_digit()) {
                    // 检查 /proc/[pid]/fd/ 下的文件描述符
                    let fd_dir = path.join("fd");
                    if let Ok(entries) = fs::read_dir(fd_dir) {
                        for fd_entry in entries.flatten() {
                            if let Ok(link) = fs::read_link(fd_entry.path()) {
                                if link.to_string_lossy().contains(inode) {
                                    // 读取进程名
                                    let comm_path = path.join("comm");
                                    if let Ok(comm) = fs::read_to_string(comm_path) {
                                        return Ok(comm.trim().to_string());
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        Err(anyhow::anyhow!("未找到进程"))
    }
}

#[cfg(target_os = "macos")]
pub mod process_detection {
    use anyhow::Result;
    use std::process::Command;

    /// 根据端口获取进程名称
    pub fn get_process_name_by_port(port: u16) -> Result<String> {
        let output = Command::new("lsof")
            .args(&["-i", &format!(":{}", port), "-sTCP:ESTABLISHED", "-Fn"])
            .output()?;

        let stdout = String::from_utf8_lossy(&output.stdout);

        // 解析 lsof 输出
        // 格式: p<PID>\nn<进程名>\n...
        let mut process_name = None;
        for line in stdout.lines() {
            if line.starts_with('n') {
                process_name = Some(line[1..].to_string());
                break;
            }
        }

        process_name.ok_or_else(|| anyhow::anyhow!("未找到进程"))
    }
}

impl SessionAffinityManager {
    /// 为进程选择节点（考虑进程绑定）
    pub async fn select_node_for_process(
        &self,
        source_port: u16,
        available_nodes: &[String],
    ) -> Result<String> {
        let config = self.config.read().await;

        if !config.enabled {
            return Ok(available_nodes
                .first()
                .ok_or_else(|| anyhow!("没有可用节点"))?
                .clone());
        }

        // 尝试获取进程名
        let process_name = match process_detection::get_process_name_by_port(source_port) {
            Ok(name) => name,
            Err(_) => {
                // 无法获取进程名，使用默认选择
                return Ok(available_nodes
                    .first()
                    .ok_or_else(|| anyhow!("没有可用节点"))?
                    .clone());
            }
        };

        // 查找匹配的进程规则
        let rule = config
            .process_rules
            .iter()
            .find(|r| r.enabled && r.process_name.eq_ignore_ascii_case(&process_name));

        if let Some(rule) = rule {
            // 检查是否已有绑定
            let bindings = self.process_bindings.read().await;
            if let Some(binding) = bindings.get(&process_name) {
                if !self.is_binding_expired(binding) {
                    if available_nodes.contains(&binding.node_id) {
                        log::debug!(
                            "[SessionAffinity] 进程 {} 使用已绑定节点 {}",
                            process_name,
                            binding.node_id
                        );
                        return Ok(binding.node_id.clone());
                    }
                }
            }
            drop(bindings);

            // 选择新节点
            let node = if let Some(ref bound_node) = rule.bound_node {
                if available_nodes.contains(bound_node) {
                    bound_node.clone()
                } else {
                    return Err(anyhow!("指定节点 {} 不可用", bound_node));
                }
            } else {
                available_nodes
                    .first()
                    .ok_or_else(|| anyhow!("没有可用节点"))?
                    .clone()
            };

            // 创建绑定
            let binding = NodeBinding {
                node_id: node.clone(),
                bound_at: SystemTime::now(),
                expires_at: if rule.ttl > 0 {
                    Some(SystemTime::now() + Duration::from_secs(rule.ttl))
                } else {
                    None
                },
                fallback_policy: rule.fallback_policy.clone(),
            };

            let mut bindings = self.process_bindings.write().await;
            bindings.insert(process_name.clone(), binding);

            log::info!("[SessionAffinity] 进程 {} 绑定到节点 {}", process_name, node);

            Ok(node)
        } else {
            Ok(available_nodes
                .first()
                .ok_or_else(|| anyhow!("没有可用节点"))?
                .clone())
        }
    }
}


impl SessionAffinityManager {
    /// 为连接选择节点（考虑连接级绑定）
    pub async fn select_node_for_connection(
        &self,
        source_ip: &str,
        source_port: u16,
        available_nodes: &[String],
    ) -> Result<String> {
        let config = self.config.read().await;

        if !config.enabled || !config.connection_binding.enabled {
            return Ok(available_nodes
                .first()
                .ok_or_else(|| anyhow!("没有可用节点"))?
                .clone());
        }

        let conn_id = ConnectionId {
            source_ip: source_ip.to_string(),
            source_port,
        };

        // 检查是否已有绑定
        let bindings = self.connection_bindings.read().await;
        if let Some(binding) = bindings.get(&conn_id) {
            if !self.is_binding_expired(binding) {
                if available_nodes.contains(&binding.node_id) {
                    log::debug!(
                        "[SessionAffinity] 连接 {}:{} 使用已绑定节点 {}",
                        source_ip,
                        source_port,
                        binding.node_id
                    );
                    return Ok(binding.node_id.clone());
                }
            }
        }
        drop(bindings);

        // 选择新节点
        let node = available_nodes
            .first()
            .ok_or_else(|| anyhow!("没有可用节点"))?
            .clone();

        // 创建绑定
        let binding = NodeBinding {
            node_id: node.clone(),
            bound_at: SystemTime::now(),
            expires_at: Some(
                SystemTime::now() + Duration::from_secs(config.connection_binding.timeout)
            ),
            fallback_policy: FallbackPolicy::AutoSwitch,
        };

        let mut bindings = self.connection_bindings.write().await;
        bindings.insert(conn_id, binding);

        log::debug!(
            "[SessionAffinity] 连接 {}:{} 绑定到节点 {}",
            source_ip,
            source_port,
            node
        );

        Ok(node)
    }

    /// 启动后台清理任务
    pub fn start_cleanup_task(self: Arc<Self>) {
        AsyncHandler::spawn(move || async move {
            let mut interval = tokio::time::interval(Duration::from_secs(60));
            loop {
                interval.tick().await;
                if let Err(e) = self.cleanup_expired_bindings().await {
                    log::error!("[SessionAffinity] 清理过期绑定失败: {}", e);
                }
            }
        });
    }
}


#[cfg(test)]
#[path = "session_affinity_tests.rs"]
mod integration_tests;
