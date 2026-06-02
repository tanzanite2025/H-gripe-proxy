use anyhow::{Result, anyhow};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::net::IpAddr;
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use tokio::sync::RwLock;

use super::asn_classifier::{self, AsnCategory};
use super::runtime_diagnostics::geoip::fetch_ip_location;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IpReputationConfig {
    pub enabled: bool,
    pub cache_ttl: u64,
    pub routing_rules: Vec<RiskRoutingRule>,
    pub use_local_db: bool,
}

impl Default for IpReputationConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            cache_ttl: 3600,
            routing_rules: get_predefined_routing_rules(),
            use_local_db: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IpReputation {
    pub ip: String,
    pub ip_type: IpType,
    pub asn: String,
    pub asn_org: String,
    pub fraud_score: u8,
    pub risk_level: RiskLevel,
    pub confidence: u8,
    pub evidence: Vec<IpReputationEvidence>,
    pub residential_state: ResidentialVerificationState,
    pub is_proxy: bool,
    pub is_vpn: bool,
    pub is_tor: bool,
    pub country_code: String,
    pub city: Option<String>,
    pub checked_at: SystemTime,
}

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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RiskLevel {
    Low,
    Medium,
    High,
    VeryHigh,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum IpReputationEvidenceKind {
    AsnTable,
    OrgKeyword,
    IpPrefix,
    ReservedIp,
    GeoIp,
    Default,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct IpReputationEvidence {
    pub kind: IpReputationEvidenceKind,
    pub label: String,
    pub weight: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum ResidentialVerificationState {
    NotResidential,
    ObservedResidential,
    VerifiedResidential,
    Unknown,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskRoutingRule {
    pub domain_patterns: Vec<String>,
    pub enabled: bool,
    pub required_ip_type: Option<IpType>,
    pub max_fraud_score: u8,
    pub fallback_policy: RiskFallbackPolicy,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RiskFallbackPolicy {
    Block,
    Warn,
    Allow,
}

pub struct IpReputationManager {
    config: Arc<RwLock<IpReputationConfig>>,
    cache: Arc<RwLock<HashMap<String, IpReputation>>>,
}

impl IpReputationManager {
    pub fn new() -> Self {
        Self {
            config: Arc::new(RwLock::new(IpReputationConfig::default())),
            cache: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn update_config(&self, config: IpReputationConfig) -> Result<()> {
        *self.config.write().await = config;
        log::info!("[IpReputation] config updated");
        Ok(())
    }

    pub async fn inspect_ip_metadata(&self, ip: &str) -> Result<IpReputation> {
        let (cache_ttl, use_local_db) = {
            let config = self.config.read().await;
            (config.cache_ttl, config.use_local_db)
        };

        let cache = self.cache.read().await;
        if let Some(cached) = cache.get(ip) {
            let age = SystemTime::now().duration_since(cached.checked_at).unwrap_or_default();

            if age < Duration::from_secs(cache_ttl) {
                log::debug!("[IpReputation] using cached metadata for {ip}");
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

    pub async fn check_ip_reputation(&self, ip: &str) -> Result<IpReputation> {
        let enabled = self.config.read().await.enabled;

        if !enabled {
            return Ok(self.create_default_reputation(ip));
        }

        let reputation = self.inspect_ip_metadata(ip).await?;

        log::info!(
            "[IpReputation] IP {} type {:?}, fraud score {}",
            ip,
            reputation.ip_type,
            reputation.fraud_score
        );

        Ok(reputation)
    }

    async fn check_ip_local(&self, ip: &str) -> Result<IpReputation> {
        let client = Client::builder()
            .timeout(Duration::from_secs(10))
            .build()
            .unwrap_or_default();

        let geo_info = fetch_ip_location(&client, ip).await;

        let asn_num = geo_info.asn;
        let asn_org = geo_info
            .asn_organization
            .as_deref()
            .or(geo_info.isp.as_deref())
            .or(geo_info.organization.as_deref());

        let asn_info = asn_classifier::get_asn_info(asn_num, asn_org);
        let ip_type = self.resolve_ip_type(ip, &asn_info);
        let evidence = self.build_reputation_evidence(ip, asn_num, asn_org, &asn_info, &ip_type);
        let confidence = self.calculate_confidence(&ip_type, &evidence);
        let residential_state = self.resolve_residential_state(&ip_type, confidence);
        let fraud_score = self.calculate_fraud_score(&ip_type, &asn_info);
        let risk_level = RiskLevel::from_score(fraud_score);

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
            confidence,
            evidence,
            residential_state,
            is_proxy: false,
            is_vpn: false,
            is_tor: false,
            country_code: geo_info
                .country_code
                .map(|s| s.into())
                .unwrap_or_else(|| "Unknown".to_string()),
            city: geo_info.city.map(|s| s.into()),
            checked_at: SystemTime::now(),
        })
    }

    fn resolve_ip_type(&self, ip: &str, asn_info: &asn_classifier::AsnInfo) -> IpType {
        let asn_ip_type = IpType::from(asn_info.category);
        if asn_ip_type == IpType::Unknown {
            self.detect_ip_type(ip)
        } else {
            asn_ip_type
        }
    }

    fn detect_ip_type(&self, ip: &str) -> IpType {
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

        if is_private_or_reserved_ip(ip) {
            return IpType::Unknown;
        }

        IpType::Unknown
    }

    fn calculate_fraud_score(&self, ip_type: &IpType, asn_info: &asn_classifier::AsnInfo) -> u8 {
        match ip_type {
            IpType::Datacenter => {
                let name_lower = asn_info.name.to_lowercase();
                if name_lower.contains("m247") || name_lower.contains("stark") || name_lower.contains("floki") {
                    95
                } else if name_lower.contains("cloudflare")
                    || name_lower.contains("akamai")
                    || name_lower.contains("fastly")
                {
                    70
                } else {
                    85
                }
            }
            IpType::Residential => 15,
            IpType::Mobile => 10,
            IpType::Education => 25,
            IpType::Unknown => 50,
        }
    }

    fn build_reputation_evidence(
        &self,
        ip: &str,
        asn: Option<u32>,
        org_name: Option<&str>,
        asn_info: &asn_classifier::AsnInfo,
        ip_type: &IpType,
    ) -> Vec<IpReputationEvidence> {
        let mut evidence = Vec::new();

        if let Some(asn_num) = asn {
            if asn_classifier::classify_by_asn(asn_num).is_some() {
                evidence.push(IpReputationEvidence {
                    kind: IpReputationEvidenceKind::AsnTable,
                    label: format!("ASN table matched AS{asn_num} as {:?}", ip_type),
                    weight: 70,
                });
            }
        }

        if let Some(org) = org_name {
            if asn_classifier::classify_by_org_name(org) != asn_classifier::AsnCategory::Unknown && asn_info.name == org
            {
                evidence.push(IpReputationEvidence {
                    kind: IpReputationEvidenceKind::OrgKeyword,
                    label: format!("organization name matched classification keywords: {org}"),
                    weight: 45,
                });
            }
        }

        if self.detect_ip_type(ip) != IpType::Unknown && asn_info.category == asn_classifier::AsnCategory::Unknown {
            evidence.push(IpReputationEvidence {
                kind: IpReputationEvidenceKind::IpPrefix,
                label: format!("IP prefix matched known {:?} range", ip_type),
                weight: 35,
            });
        }

        if is_private_or_reserved_ip(ip) {
            evidence.push(IpReputationEvidence {
                kind: IpReputationEvidenceKind::ReservedIp,
                label: "IP is private or reserved; public reputation is inconclusive".to_string(),
                weight: 10,
            });
        }

        if evidence.is_empty() {
            evidence.push(IpReputationEvidence {
                kind: IpReputationEvidenceKind::Default,
                label: "no ASN, organization, or prefix evidence matched".to_string(),
                weight: 15,
            });
        }

        evidence
    }

    fn calculate_confidence(&self, ip_type: &IpType, evidence: &[IpReputationEvidence]) -> u8 {
        let strongest = evidence.iter().map(|item| item.weight).max().unwrap_or(0);
        let support_count = evidence
            .iter()
            .filter(|item| {
                !matches!(
                    item.kind,
                    IpReputationEvidenceKind::Default | IpReputationEvidenceKind::ReservedIp
                )
            })
            .count();
        let support_bonus = support_count.saturating_sub(1).min(2) as u8 * 10;
        let type_penalty = if matches!(ip_type, IpType::Unknown) { 20 } else { 0 };

        strongest
            .saturating_add(support_bonus)
            .saturating_sub(type_penalty)
            .min(95)
    }

    fn resolve_residential_state(&self, ip_type: &IpType, confidence: u8) -> ResidentialVerificationState {
        match ip_type {
            IpType::Residential | IpType::Mobile | IpType::Education if confidence >= 90 => {
                ResidentialVerificationState::VerifiedResidential
            }
            IpType::Residential | IpType::Mobile | IpType::Education => {
                ResidentialVerificationState::ObservedResidential
            }
            IpType::Unknown => ResidentialVerificationState::Unknown,
            IpType::Datacenter => ResidentialVerificationState::NotResidential,
        }
    }

    fn create_default_reputation(&self, ip: &str) -> IpReputation {
        IpReputation {
            ip: ip.to_string(),
            ip_type: IpType::Unknown,
            asn: "Unknown".to_string(),
            asn_org: "Unknown".to_string(),
            fraud_score: 50,
            risk_level: RiskLevel::Medium,
            confidence: 0,
            evidence: vec![IpReputationEvidence {
                kind: IpReputationEvidenceKind::Default,
                label: "IP reputation checks are disabled".to_string(),
                weight: 0,
            }],
            residential_state: ResidentialVerificationState::Unknown,
            is_proxy: false,
            is_vpn: false,
            is_tor: false,
            country_code: "Unknown".to_string(),
            city: None,
            checked_at: SystemTime::now(),
        }
    }

    pub async fn select_node_for_domain(&self, domain: &str, available_nodes: &[(String, String)]) -> Result<String> {
        let config = self.config.read().await;

        if !config.enabled {
            return Ok(first_node_name(available_nodes)?.to_string());
        }

        let rule = config
            .routing_rules
            .iter()
            .find(|r| r.enabled && r.domain_patterns.iter().any(|p| domain_matches(domain, p)));

        if let Some(rule) = rule {
            let mut suitable_nodes = Vec::new();
            let mut all_fraud_scores: Vec<u8> = Vec::new();

            for (node_name, node_ip) in available_nodes {
                match self.check_ip_reputation(node_ip).await {
                    Ok(reputation) => {
                        all_fraud_scores.push(reputation.fraud_score);
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
                                "[IpReputation] node {} rejected: type_match={}, score_match={} (score={}, max={})",
                                node_name,
                                type_match,
                                score_match,
                                reputation.fraud_score,
                                rule.max_fraud_score
                            );
                        }
                    }
                    Err(e) => {
                        log::warn!("[IpReputation] failed to inspect node {node_name}: {e}");
                    }
                }
            }

            if suitable_nodes.is_empty() {
                match rule.fallback_policy {
                    RiskFallbackPolicy::Block => {
                        log::error!("[IpReputation] domain {domain} has no node satisfying reputation requirements");
                        let max_score = all_fraud_scores.iter().max().copied().unwrap_or(100);
                        crate::feat::blackhole_breaker_record_fraud_score(domain, max_score).await;
                        return Err(anyhow!("no node satisfies reputation requirements"));
                    }
                    RiskFallbackPolicy::Warn | RiskFallbackPolicy::Allow => {
                        if matches!(rule.fallback_policy, RiskFallbackPolicy::Warn) {
                            log::warn!("[IpReputation] domain {domain} has no suitable node; using default node");
                        }
                        return Ok(first_node_name(available_nodes)?.to_string());
                    }
                }
            }

            suitable_nodes.sort_by_key(|(_, rep)| rep.fraud_score);
            let selected = &suitable_nodes.first().unwrap().0;

            log::info!(
                "[IpReputation] domain {} selected node {} (fraud score {})",
                domain,
                selected,
                suitable_nodes.first().unwrap().1.fraud_score
            );

            Ok(selected.clone())
        } else {
            Ok(first_node_name(available_nodes)?.to_string())
        }
    }

    pub async fn clear_cache(&self) -> Result<()> {
        let mut cache = self.cache.write().await;
        cache.clear();
        log::info!("[IpReputation] cache cleared");
        Ok(())
    }

    pub async fn get_cache_stats(&self) -> (usize, usize) {
        let cache = self.cache.read().await;
        let cache_ttl = self.config.read().await.cache_ttl;
        let total = cache.len();
        let expired = cache
            .values()
            .filter(|rep| {
                let age = SystemTime::now().duration_since(rep.checked_at).unwrap_or_default();
                age > Duration::from_secs(cache_ttl)
            })
            .count();
        (total, expired)
    }

    pub async fn get_cache_entries(&self) -> Vec<IpReputation> {
        let cache = self.cache.read().await;
        cache.values().cloned().collect()
    }
}

fn first_node_name(available_nodes: &[(String, String)]) -> Result<&str> {
    available_nodes
        .first()
        .map(|(name, _)| name.as_str())
        .ok_or_else(|| anyhow!("no available nodes"))
}

fn is_carrier_grade_nat_ip(octets: [u8; 4]) -> bool {
    octets[0] == 100 && (100..=127).contains(&octets[1])
}

fn is_private_or_reserved_ip(ip: &str) -> bool {
    match ip.parse::<IpAddr>() {
        Ok(IpAddr::V4(addr)) => {
            addr.is_private()
                || addr.is_loopback()
                || addr.is_link_local()
                || addr.is_unspecified()
                || addr.is_broadcast()
                || addr.is_documentation()
                || is_carrier_grade_nat_ip(addr.octets())
        }
        Ok(IpAddr::V6(addr)) => {
            addr.is_loopback() || addr.is_unspecified() || addr.is_unique_local() || addr.is_unicast_link_local()
        }
        Err(_) => true,
    }
}

pub fn matches_ip_type(actual: &IpType, required: &IpType) -> bool {
    match (actual, required) {
        (IpType::Residential, IpType::Residential) => true,
        (IpType::Mobile, IpType::Residential) => true,
        (IpType::Mobile, IpType::Mobile) => true,
        (IpType::Education, IpType::Residential) => true,
        (a, r) => a == r,
    }
}

fn domain_matches(domain: &str, pattern: &str) -> bool {
    if let Some(suffix) = pattern.strip_prefix("*.") {
        domain.ends_with(suffix) || domain == suffix
    } else if let Some(suffix) = pattern.strip_prefix('*') {
        domain.ends_with(suffix)
    } else {
        domain == pattern
    }
}

pub fn get_predefined_routing_rules() -> Vec<RiskRoutingRule> {
    vec![
        RiskRoutingRule {
            domain_patterns: vec!["*.openai.com".to_string(), "*.anthropic.com".to_string()],
            enabled: true,
            required_ip_type: Some(IpType::Residential),
            max_fraud_score: 30,
            fallback_policy: RiskFallbackPolicy::Block,
            description: "AI services require residential IP and fraud score below 30".to_string(),
        },
        RiskRoutingRule {
            domain_patterns: vec!["*.stripe.com".to_string(), "*.paypal.com".to_string()],
            enabled: true,
            required_ip_type: Some(IpType::Residential),
            max_fraud_score: 20,
            fallback_policy: RiskFallbackPolicy::Block,
            description: "Financial services require residential IP and fraud score below 20".to_string(),
        },
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
            description: "Game platforms prefer residential IP and fraud score below 50".to_string(),
        },
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
            description: "Social media prefers fraud score below 70".to_string(),
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_ip_type_detection() {
        let manager = IpReputationManager::new();

        assert_eq!(manager.detect_ip_type("45.76.123.45"), IpType::Datacenter);
        assert_eq!(manager.detect_ip_type("13.52.100.1"), IpType::Datacenter);

        assert_eq!(manager.detect_ip_type("192.168.1.1"), IpType::Unknown);
        assert_eq!(manager.detect_ip_type("172.20.1.1"), IpType::Unknown);
        assert_eq!(manager.detect_ip_type("100.64.1.1"), IpType::Unknown);
    }

    #[tokio::test]
    async fn test_unknown_asn_uses_ip_prefix_fallback() {
        let manager = IpReputationManager::new();
        let unknown_asn = asn_classifier::AsnInfo {
            name: "Unknown".to_string(),
            category: asn_classifier::AsnCategory::Unknown,
        };

        assert_eq!(
            manager.resolve_ip_type("45.76.123.45", &unknown_asn),
            IpType::Datacenter
        );
    }

    #[tokio::test]
    async fn test_known_asn_wins_over_ip_prefix_fallback() {
        let manager = IpReputationManager::new();
        let mobile_asn = asn_classifier::AsnInfo {
            name: "China Mobile".to_string(),
            category: asn_classifier::AsnCategory::Mobile,
        };

        assert_eq!(manager.resolve_ip_type("45.76.123.45", &mobile_asn), IpType::Mobile);
    }

    #[test]
    fn test_residential_reputation_is_observed_not_verified_by_default() {
        let manager = IpReputationManager::new();
        let asn_info = asn_classifier::AsnInfo {
            name: "Comcast Cable".to_string(),
            category: asn_classifier::AsnCategory::Residential,
        };

        let evidence = manager.build_reputation_evidence(
            "203.0.113.10",
            Some(7922),
            Some("Comcast Cable"),
            &asn_info,
            &IpType::Residential,
        );
        let confidence = manager.calculate_confidence(&IpType::Residential, &evidence);

        assert_eq!(
            manager.resolve_residential_state(&IpType::Residential, confidence),
            ResidentialVerificationState::ObservedResidential
        );
        assert!(confidence < 90);
        assert!(
            evidence
                .iter()
                .any(|item| item.kind == IpReputationEvidenceKind::AsnTable)
        );
    }

    #[test]
    fn test_prefix_fallback_has_lower_confidence_than_asn_table() {
        let manager = IpReputationManager::new();
        let unknown_asn = asn_classifier::AsnInfo {
            name: "Unknown".to_string(),
            category: asn_classifier::AsnCategory::Unknown,
        };
        let datacenter_asn = asn_classifier::AsnInfo {
            name: "Amazon AWS".to_string(),
            category: asn_classifier::AsnCategory::Datacenter,
        };

        let prefix_evidence =
            manager.build_reputation_evidence("45.76.123.45", None, None, &unknown_asn, &IpType::Datacenter);
        let asn_evidence = manager.build_reputation_evidence(
            "52.1.1.1",
            Some(16509),
            Some("Amazon AWS"),
            &datacenter_asn,
            &IpType::Datacenter,
        );

        assert!(
            manager.calculate_confidence(&IpType::Datacenter, &asn_evidence)
                > manager.calculate_confidence(&IpType::Datacenter, &prefix_evidence)
        );
        assert!(
            prefix_evidence
                .iter()
                .any(|item| item.kind == IpReputationEvidenceKind::IpPrefix)
        );
    }

    #[tokio::test]
    async fn test_unknown_public_ip_without_asn_metadata_stays_unknown() {
        let manager = IpReputationManager::new();
        let unknown_asn = asn_classifier::AsnInfo {
            name: "Unknown".to_string(),
            category: asn_classifier::AsnCategory::Unknown,
        };

        assert_eq!(manager.resolve_ip_type("8.8.8.8", &unknown_asn), IpType::Unknown);
    }

    #[tokio::test]
    async fn test_fraud_score_calculation() {
        let manager = IpReputationManager::new();
        let dc_info = asn_classifier::AsnInfo {
            name: "Vultr".to_string(),
            category: asn_classifier::AsnCategory::Datacenter,
        };
        let res_info = asn_classifier::AsnInfo {
            name: "Comcast".to_string(),
            category: asn_classifier::AsnCategory::Residential,
        };

        assert_eq!(manager.calculate_fraud_score(&IpType::Datacenter, &dc_info), 85);
        assert_eq!(manager.calculate_fraud_score(&IpType::Residential, &res_info), 15);
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
        assert!(
            rules
                .iter()
                .any(|r| r.domain_patterns.contains(&"*.openai.com".to_string()))
        );
    }
}
