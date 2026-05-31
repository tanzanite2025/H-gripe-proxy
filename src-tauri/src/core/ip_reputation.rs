use anyhow::{anyhow, Result};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use tokio::sync::RwLock;

use super::asn_classifier::{self, AsnCategory};
use super::runtime_diagnostics::geoip::{fetch_ip_location, GeoIpInfo};

/// IP 信誉度配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IpReputationConfig {
    /// 启用 IP 信誉度检测
    pub enabled: bool,
    /// 缓存时长（秒）
    pub cache_ttl: u64,
    /// 风控等级路由规则
    pub routing_rules: Vec<RiskRoutingRule>,
    /// 使用本地数据库（不调用 API）
    pub use_local_db: bool,
}

impl Default for IpReputationConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            cache_ttl: 3600, // 1小时
            routing_rules: get_predefined_routing_rules(),
            use_local_db: true, // 默认使用本地数据库
        }
    }
}

/// IP 信誉度信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IpReputation {
    /// IP 地址
    pub ip: String,
    /// IP 类型
    pub ip_type: IpType,
    /// ASN
    pub asn: String,
    /// ASN 组织
    pub asn_org: String,
    /// 欺诈评分（0-100）
    pub fraud_score: u8,
    /// 风险等级
    pub risk_level: RiskLevel,
    /// 是否为代理
    pub is_proxy: bool,
    /// 是否为 VPN
    pub is_vpn: bool,
    /// 是否为 Tor
    pub is_tor: bool,
    /// 国家代码
    pub country_code: String,
    /// 城市
    pub city: Option<String>,
    /// 检测时间
    pub checked_at: SystemTime,
}

/// IP 类型
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum IpType {
    Datacenter,
    Residential,
    Mobile,
    Education,
    Unknown,
}

impl From<AsnCategory> for IpType {
    fn from(cat: AsnCategory) -> Self {
        match cat {
            AsnCategory::Datacenter => IpType::Datacenter,
            AsnCategory::Residential => IpType::Residential,
            AsnCategory::Mobile => IpType::Mobile,
            AsnCategory::Education => IpType::Education,
            AsnCategory::Unknown => IpType::Unknown,
        }
    }
}

/// 风险等级
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RiskLevel {
    Low,      // 0-30
    Medium,   // 31-60
    High,     // 61-85
    VeryHigh, // 86-100
}

impl RiskLevel {
    pub fn from_score(score: u8) -> Self {
        match score {
            0..=30 => RiskLevel::Low,
            31..=60 => RiskLevel::Medium,
            61..=85 => RiskLevel::High,
            _ => RiskLevel::VeryHigh,
        }
    }
}

/// 风控等级路由规则
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskRoutingRule {
    /// 域名模式
    pub domain_patterns: Vec<String>,
    /// 是否启用
    pub enabled: bool,
    /// 要求的 IP 类型
    pub required_ip_type: Option<IpType>,
    /// 最大欺诈评分
    pub max_fraud_score: u8,
    /// 故障转移策略
    pub fallback_policy: RiskFallbackPolicy,
    /// 描述
    pub description: String,
}


/// 风控故障转移策略
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RiskFallbackPolicy {
    /// 阻止连接
    Block,
    /// 警告但允许
    Warn,
    /// 允许
    Allow,
}

/// IP 信誉度管理器
pub struct IpReputationManager {
    /// 配置
    config: Arc<RwLock<IpReputationConfig>>,
    /// IP 信誉度缓存
    cache: Arc<RwLock<HashMap<String, IpReputation>>>,
}

impl IpReputationManager {
    /// 创建新的 IP 信誉度管理器
    pub fn new() -> Self {
        Self {
            config: Arc::new(RwLock::new(IpReputationConfig::default())),
            cache: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// 获取配置
    pub async fn get_config(&self) -> Result<IpReputationConfig> {
        Ok(self.config.read().await.clone())
    }

    /// 更新配置
    pub async fn update_config(&self, config: IpReputationConfig) -> Result<()> {
        *self.config.write().await = config;
        log::info!("[IpReputation] 配置已更新");
        Ok(())
    }

    pub async fn inspect_ip_metadata(&self, ip: &str) -> Result<IpReputation> {
        let (cache_ttl, use_local_db) = {
            let config = self.config.read().await;
            (config.cache_ttl, config.use_local_db)
        };

        let cache = self.cache.read().await;
        if let Some(cached) = cache.get(ip) {
            let age = SystemTime::now()
                .duration_since(cached.checked_at)
                .unwrap_or_default();

            if age < Duration::from_secs(cache_ttl) {
                log::debug!("[IpReputation] 使用缓存的 IP 元数据: {}", ip);
                return Ok(cached.clone());
            }
        }
        drop(cache);

        let reputation = if use_local_db {
            self.check_ip_local(ip).await?
        } else {
            self.check_ip_local(ip).await?
        };

        let mut cache = self.cache.write().await;
        cache.insert(ip.to_string(), reputation.clone());

        Ok(reputation)
    }

    /// 检测 IP 信誉度
    pub async fn check_ip_reputation(&self, ip: &str) -> Result<IpReputation> {
        let enabled = self.config.read().await.enabled;

        if !enabled {
            // 如果未启用，返回默认的低风险评估
            return Ok(self.create_default_reputation(ip));
        }

        let reputation = self.inspect_ip_metadata(ip).await?;

        log::info!(
            "[IpReputation] IP {} 信誉度: {:?}, 欺诈评分: {}",
            ip,
            reputation.ip_type,
            reputation.fraud_score
        );

        Ok(reputation)
    }

    /// 本地检测 IP 信誉度（GeoIP + ASN 分类器）
    async fn check_ip_local(&self, ip: &str) -> Result<IpReputation> {
        let client = Client::builder()
            .timeout(Duration::from_secs(10))
            .build()
            .unwrap_or_default();

        let geo_info = fetch_ip_location(&client, ip).await;

        let asn_num = geo_info.asn;
        let asn_org = geo_info.asn_organization.as_deref()
            .or(geo_info.isp.as_deref())
            .or(geo_info.organization.as_deref());

        let asn_info = asn_classifier::get_asn_info(asn_num, asn_org);
        let ip_type = IpType::from(asn_info.category);
        let fraud_score = self.calculate_fraud_score(&ip_type, &asn_info);
        let risk_level = RiskLevel::from_score(fraud_score);

        // ASN 名称优先用查表结果，兜底用 GeoIP 返回
        let asn_display = if asn_info.name != "Unknown" {
            asn_info.name.clone()
        } else {
            asn_org.unwrap_or("Unknown").to_string()
        };

        let asn_str = match asn_num {
            Some(n) => format!("AS{}", n),
            None => "Unknown".to_string(),
        };

        Ok(IpReputation {
            ip: ip.to_string(),
            ip_type,
            asn: asn_str,
            asn_org: asn_display,
            fraud_score,
            risk_level,
            is_proxy: false,
            is_vpn: false,
            is_tor: false,
            country_code: geo_info.country_code.map(|s| s.into()).unwrap_or_else(|| "Unknown".to_string()),
            city: geo_info.city.map(|s| s.into()),
            checked_at: SystemTime::now(),
        })
    }

    /// 检测 IP 类型（ASN 分类器 + IP 前缀兜底）
    fn detect_ip_type(&self, ip: &str) -> IpType {
        // IP 前缀兜底（无 GeoIP 时使用）
        let datacenter_prefixes = [
            "45.76.", "104.238.", "207.246.", "149.28.", // Vultr
            "13.", "52.", "54.", // AWS
            "35.", "34.", "104.154.", // GCP
        ];

        for prefix in datacenter_prefixes {
            if ip.starts_with(prefix) {
                return IpType::Datacenter;
            }
        }

        // RFC1918 / 链路本地
        if ip.starts_with("10.") || ip.starts_with("172.16.") || ip.starts_with("192.168.") || ip.starts_with("127.") {
            return IpType::Unknown;
        }

        // 默认假设住宅
        IpType::Residential
    }

    /// 计算欺诈评分（基于 ASN 分类 + 细分）
    fn calculate_fraud_score(&self, ip_type: &IpType, asn_info: &asn_classifier::AsnInfo) -> u8 {
        match ip_type {
            IpType::Datacenter => {
                // 已知数据中心 ASN 更高分
                let name_lower = asn_info.name.to_lowercase();
                if name_lower.contains("m247") || name_lower.contains("stark") || name_lower.contains("floki") {
                    95 // 已知代理/VPN 托管商
                } else if name_lower.contains("cloudflare") || name_lower.contains("akamai") || name_lower.contains("fastly") {
                    70 // CDN 不算特别危险
                } else {
                    85 // 普通机房
                }
            }
            IpType::Residential => 15,
            IpType::Mobile => 10,
            IpType::Education => 25, // 教育网可能被滥用
            IpType::Unknown => 50,
        }
    }

    /// 创建默认的低风险评估
    fn create_default_reputation(&self, ip: &str) -> IpReputation {
        IpReputation {
            ip: ip.to_string(),
            ip_type: IpType::Unknown,
            asn: "Unknown".to_string(),
            asn_org: "Unknown".to_string(),
            fraud_score: 50,
            risk_level: RiskLevel::Medium,
            is_proxy: false,
            is_vpn: false,
            is_tor: false,
            country_code: "Unknown".to_string(),
            city: None,
            checked_at: SystemTime::now(),
        }
    }


    /// 为域名选择合适的节点（考虑 IP 信誉度）
    pub async fn select_node_for_domain(
        &self,
        domain: &str,
        available_nodes: &[(String, String)], // (node_name, node_ip)
    ) -> Result<String> {
        let config = self.config.read().await;

        if !config.enabled {
            // 未启用，使用默认选择
            return Ok(available_nodes
                .first()
                .ok_or_else(|| anyhow!("没有可用节点"))?
                .0
                .clone());
        }

        // 1. 查找匹配的路由规则
        let rule = config.routing_rules.iter().find(|r| {
            r.enabled
                && r.domain_patterns
                    .iter()
                    .any(|p| domain_matches(domain, p))
        });

        if let Some(rule) = rule {
            // 2. 检测所有节点的 IP 信誉度
            let mut suitable_nodes = Vec::new();
            let mut all_fraud_scores: Vec<u8> = Vec::new();

            for (node_name, node_ip) in available_nodes {
                match self.check_ip_reputation(node_ip).await {
                    Ok(reputation) => {
                        all_fraud_scores.push(reputation.fraud_score);
                        // 检查是否满足要求
                        let type_match = rule
                            .required_ip_type
                            .as_ref()
                            .map(|req| matches_ip_type(&reputation.ip_type, req))
                            .unwrap_or(true);

                        let score_match = reputation.fraud_score <= rule.max_fraud_score;

                        if type_match && score_match {
                            suitable_nodes.push((node_name.clone(), reputation));
                        } else {
                            log::debug!(
                                "[IpReputation] 节点 {} 不满足要求: type_match={}, score_match={} (score={}, max={})",
                                node_name,
                                type_match,
                                score_match,
                                reputation.fraud_score,
                                rule.max_fraud_score
                            );
                        }
                    }
                    Err(e) => {
                        log::warn!("[IpReputation] 检测节点 {} 失败: {}", node_name, e);
                    }
                }
            }

            // 3. 根据故障转移策略处理
            if suitable_nodes.is_empty() {
                match rule.fallback_policy {
                    RiskFallbackPolicy::Block => {
                        log::error!(
                            "[IpReputation] 域名 {} 没有满足信誉度要求的节点，阻止连接",
                            domain
                        );
                        // 通知黑洞熔断器：欺诈评分过高
                        let max_score = all_fraud_scores.iter().max().copied().unwrap_or(100);
                        crate::feat::blackhole_breaker_record_fraud_score(domain, max_score).await;
                        return Err(anyhow!("没有满足信誉度要求的节点"));
                    }
                    RiskFallbackPolicy::Warn => {
                        log::warn!(
                            "[IpReputation] 域名 {} 没有满足要求的节点，使用默认节点",
                            domain
                        );
                        return Ok(available_nodes
                            .first()
                            .ok_or_else(|| anyhow!("没有可用节点"))?
                            .0
                            .clone());
                    }
                    RiskFallbackPolicy::Allow => {
                        return Ok(available_nodes
                            .first()
                            .ok_or_else(|| anyhow!("没有可用节点"))?
                            .0
                            .clone());
                    }
                }
            }

            // 4. 选择信誉度最好的节点（欺诈评分最低）
            suitable_nodes.sort_by_key(|(_, rep)| rep.fraud_score);
            let selected = &suitable_nodes.first().unwrap().0;

            log::info!(
                "[IpReputation] 域名 {} 选择节点 {} (欺诈评分: {})",
                domain,
                selected,
                suitable_nodes.first().unwrap().1.fraud_score
            );

            Ok(selected.clone())
        } else {
            // 没有匹配的规则，使用默认选择
            Ok(available_nodes
                .first()
                .ok_or_else(|| anyhow!("没有可用节点"))?
                .0
                .clone())
        }
    }

    /// 清除缓存
    pub async fn clear_cache(&self) -> Result<()> {
        let mut cache = self.cache.write().await;
        cache.clear();
        log::info!("[IpReputation] 缓存已清除");
        Ok(())
    }

    /// 获取缓存统计
    pub async fn get_cache_stats(&self) -> (usize, usize) {
        let cache = self.cache.read().await;
        let cache_ttl = self.config.read().await.cache_ttl;
        let total = cache.len();
        let expired = cache
            .values()
            .filter(|rep| {
                let age = SystemTime::now()
                    .duration_since(rep.checked_at)
                    .unwrap_or_default();
                age > Duration::from_secs(cache_ttl)
            })
            .count();
        (total, expired)
    }

    /// 获取缓存中所有条目
    pub async fn get_cache_entries(&self) -> Vec<IpReputation> {
        let cache = self.cache.read().await;
        cache.values().cloned().collect()
    }
}

/// 检查 IP 类型是否匹配
pub fn matches_ip_type(actual: &IpType, required: &IpType) -> bool {
    match (actual, required) {
        (IpType::Residential, IpType::Residential) => true,
        (IpType::Mobile, IpType::Residential) => true, // Mobile 也算 Residential
        (IpType::Mobile, IpType::Mobile) => true,
        (IpType::Education, IpType::Residential) => true, // Education 有时也算
        (a, r) => a == r,
    }
}

/// 域名匹配（复用 session_affinity 的函数）
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

/// 获取预定义的风控路由规则
pub fn get_predefined_routing_rules() -> Vec<RiskRoutingRule> {
    vec![
        // AI 服务（极高风控）
        RiskRoutingRule {
            domain_patterns: vec!["*.openai.com".to_string(), "*.anthropic.com".to_string()],
            enabled: true,
            required_ip_type: Some(IpType::Residential),
            max_fraud_score: 30,
            fallback_policy: RiskFallbackPolicy::Block,
            description: "AI 服务 - 必须使用住宅 IP，欺诈评分 < 30".to_string(),
        },
        // 金融服务（极高风控）
        RiskRoutingRule {
            domain_patterns: vec!["*.stripe.com".to_string(), "*.paypal.com".to_string()],
            enabled: true,
            required_ip_type: Some(IpType::Residential),
            max_fraud_score: 20,
            fallback_policy: RiskFallbackPolicy::Block,
            description: "金融服务 - 必须使用住宅 IP，欺诈评分 < 20".to_string(),
        },
        // 游戏平台（高风控）
        RiskRoutingRule {
            domain_patterns: vec![
                "*.steampowered.com".to_string(),
                "*.epicgames.com".to_string(),
                "*.riotgames.com".to_string(),
            ],
            enabled: true,
            required_ip_type: Some(IpType::Residential),
            max_fraud_score: 50,
            fallback_policy: RiskFallbackPolicy::Warn,
            description: "游戏平台 - 建议使用住宅 IP，欺诈评分 < 50".to_string(),
        },
        // 社交媒体（中风控）
        RiskRoutingRule {
            domain_patterns: vec![
                "*.twitter.com".to_string(),
                "*.x.com".to_string(),
                "*.facebook.com".to_string(),
                "*.instagram.com".to_string(),
            ],
            enabled: true,
            required_ip_type: None,
            max_fraud_score: 70,
            fallback_policy: RiskFallbackPolicy::Warn,
            description: "社交媒体 - 欺诈评分 < 70".to_string(),
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_ip_type_detection() {
        let manager = IpReputationManager::new();

        // 测试机房 IP
        assert_eq!(manager.detect_ip_type("45.76.123.45"), IpType::Datacenter);
        assert_eq!(manager.detect_ip_type("13.52.100.1"), IpType::Datacenter);

        // 测试住宅 IP
        assert_eq!(manager.detect_ip_type("192.168.1.1"), IpType::Residential);
    }

    #[tokio::test]
    async fn test_fraud_score_calculation() {
        let manager = IpReputationManager::new();
        let dc_info = asn_classifier::AsnInfo {
            asn: 20473,
            name: "Vultr".to_string(),
            category: asn_classifier::AsnCategory::Datacenter,
        };
        let res_info = asn_classifier::AsnInfo {
            asn: 7922,
            name: "Comcast".to_string(),
            category: asn_classifier::AsnCategory::Residential,
        };

        assert_eq!(
            manager.calculate_fraud_score(&IpType::Datacenter, &dc_info),
            85
        );
        assert_eq!(
            manager.calculate_fraud_score(&IpType::Residential, &res_info),
            15
        );
    }

    #[tokio::test]
    async fn test_check_ip_reputation() {
        let manager = IpReputationManager::new();

        let reputation = manager.check_ip_reputation("45.76.123.45").await.unwrap();
        assert_eq!(reputation.ip_type, IpType::Datacenter);
        assert_eq!(reputation.fraud_score, 85);
    }

    #[tokio::test]
    async fn test_predefined_rules() {
        let rules = get_predefined_routing_rules();

        assert!(!rules.is_empty());
        assert!(rules.iter().any(|r| r
            .domain_patterns
            .contains(&"*.openai.com".to_string())));
    }
}
