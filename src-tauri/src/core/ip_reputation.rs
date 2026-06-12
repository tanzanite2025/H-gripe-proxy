use anyhow::{Result, anyhow};
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use tokio::sync::RwLock;

use super::asn_classifier::{self, AsnCategory};
use super::blackhole_breaker::get_blackhole_breaker_manager;
use super::ip_intelligence::{
    IpIntelligenceProvider, IpIntelligenceProviderConfig, IpIntelligenceProviderHealthReport,
    IpIntelligenceRecord, build_provider, probe_provider,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IpReputationConfig {
    pub enabled: bool,
    pub cache_ttl: u64,
    pub routing_rules: Vec<RiskRoutingRule>,
    #[serde(default)]
    pub metadata_provider: IpIntelligenceProviderConfig,
}

impl Default for IpReputationConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            cache_ttl: 3600,
            routing_rules: get_predefined_routing_rules(),
            metadata_provider: IpIntelligenceProviderConfig::default(),
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
    pub timezone: Option<String>,
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
    MetadataProvider,
    OrgKeyword,
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
    metadata_provider: Arc<RwLock<Option<Arc<dyn IpIntelligenceProvider>>>>,
}

fn sanitize_metadata_provider_config() -> IpIntelligenceProviderConfig {
    IpIntelligenceProviderConfig::default()
}

pub fn normalize_ip_reputation_config(mut config: IpReputationConfig) -> IpReputationConfig {
    config.metadata_provider = sanitize_metadata_provider_config();
    config
}

static IP_REPUTATION_MANAGER: Lazy<Arc<IpReputationManager>> = Lazy::new(|| {
    let config = normalize_ip_reputation_config(
        crate::core::coordinator::get_coordinator()
            .get_advanced_config()
            .ip_reputation,
    );
    Arc::new(IpReputationManager::from_config(config))
});

pub fn get_ip_reputation_manager() -> Arc<IpReputationManager> {
    IP_REPUTATION_MANAGER.clone()
}

pub async fn probe_local_metadata_provider(target_ip: Option<&str>) -> IpIntelligenceProviderHealthReport {
    let local_provider = sanitize_metadata_provider_config();
    probe_provider(&local_provider, target_ip).await
}

impl IpReputationManager {
    pub fn from_config(config: IpReputationConfig) -> Self {
        let config = normalize_ip_reputation_config(config);
        let metadata_provider = build_metadata_provider(&config);

        Self {
            config: Arc::new(RwLock::new(config)),
            cache: Arc::new(RwLock::new(HashMap::new())),
            metadata_provider: Arc::new(RwLock::new(metadata_provider)),
        }
    }

    pub async fn update_config(&self, config: IpReputationConfig) -> Result<()> {
        let config = normalize_ip_reputation_config(config);
        let metadata_provider = build_metadata_provider(&config);
        *self.config.write().await = config;
        *self.metadata_provider.write().await = metadata_provider;
        log::info!("[IpReputation] config updated");
        Ok(())
    }

    pub async fn inspect_ip_metadata(&self, ip: &str) -> Result<IpReputation> {
        let cache_ttl = {
            let config = self.config.read().await;
            config.cache_ttl
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

        let reputation = self.check_ip_local(ip).await?;

        let mut cache = self.cache.write().await;
        cache.insert(ip.to_string(), reputation.clone());

        Ok(reputation)
    }

    pub async fn lookup_ip_metadata_record(&self, ip: &str) -> Result<IpIntelligenceRecord> {
        let provider = self.metadata_provider.read().await.clone();
        let Some(provider) = provider else {
            return Err(anyhow!(
                "Local IP metadata provider is unavailable. Add GeoLite2-ASN.mmdb and GeoLite2-City.mmdb to the app resources."
            ));
        };

        provider.lookup(ip).await
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
        let provider = self.metadata_provider.read().await.clone();
        let Some(provider) = provider else {
            return Ok(self.create_unresolved_metadata_reputation(
                ip,
                "Local IP metadata provider is unavailable. Add GeoLite2-ASN.mmdb and GeoLite2-City.mmdb to the app resources.",
            ));
        };

        match provider.lookup(ip).await {
            Ok(record) => Ok(self.build_reputation_from_metadata(
                ip,
                record.asn,
                record.asn_organization.as_deref(),
                record.country_code.as_deref(),
                record.city.as_deref(),
                record.timezone.as_deref(),
                None,
                Some(&record.provider_label),
            )),
            Err(error) => {
                log::warn!("[IpReputation] metadata provider lookup failed for {ip}: {error}");
                Ok(self
                    .create_unresolved_metadata_reputation(ip, &format!("{} lookup failed: {error}", provider.label())))
            }
        }
    }

    fn build_reputation_from_metadata(
        &self,
        ip: &str,
        asn_num: Option<u32>,
        asn_org: Option<&str>,
        country_code: Option<&str>,
        city: Option<&str>,
        timezone: Option<&str>,
        asn_text: Option<&str>,
        provider_label: Option<&str>,
    ) -> IpReputation {
        let asn_info = asn_classifier::get_asn_info(asn_num, asn_org);
        let ip_type = self.resolve_ip_type(&asn_info);
        let evidence = self.build_reputation_evidence(asn_num, asn_org, &asn_info, &ip_type, provider_label);
        let confidence = self.calculate_confidence(&ip_type, &evidence);
        let residential_state = self.resolve_residential_state(&ip_type, confidence);
        let fraud_score = self.calculate_fraud_score(&ip_type, &asn_info);
        let risk_level = RiskLevel::from_score(fraud_score);

        let asn_display = if asn_info.name != "Unknown" {
            asn_info.name.clone()
        } else {
            asn_org.unwrap_or("Unknown").to_string()
        };

        let asn_str = asn_text
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(|value| value.to_string())
            .or_else(|| asn_num.map(|n| format!("AS{}", n)))
            .unwrap_or_else(|| "Unknown".to_string());

        IpReputation {
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
            country_code: country_code.unwrap_or("Unknown").to_string(),
            city: city.map(str::to_string),
            timezone: timezone.map(str::to_string),
            checked_at: SystemTime::now(),
        }
    }

    fn resolve_ip_type(&self, asn_info: &asn_classifier::AsnInfo) -> IpType {
        IpType::from(asn_info.category)
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
        asn: Option<u32>,
        org_name: Option<&str>,
        asn_info: &asn_classifier::AsnInfo,
        ip_type: &IpType,
        provider_label: Option<&str>,
    ) -> Vec<IpReputationEvidence> {
        let mut evidence = Vec::new();

        if let Some(provider_label) = provider_label
            && (asn.is_some() || org_name.is_some())
        {
            evidence.push(IpReputationEvidence {
                kind: IpReputationEvidenceKind::MetadataProvider,
                label: format!("metadata supplied by {provider_label}"),
                weight: 35,
            });
        }

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

        if evidence.is_empty() {
            evidence.push(IpReputationEvidence {
                kind: IpReputationEvidenceKind::Default,
                label: "no ASN or organization evidence matched".to_string(),
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
            timezone: None,
            checked_at: SystemTime::now(),
        }
    }

    fn create_unresolved_metadata_reputation(&self, ip: &str, message: &str) -> IpReputation {
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
                label: message.to_string(),
                weight: 10,
            }],
            residential_state: ResidentialVerificationState::Unknown,
            is_proxy: false,
            is_vpn: false,
            is_tor: false,
            country_code: "Unknown".to_string(),
            city: None,
            timezone: None,
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
                        get_blackhole_breaker_manager()
                            .record_fraud_score(domain, max_score)
                            .await;
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

fn build_metadata_provider(config: &IpReputationConfig) -> Option<Arc<dyn IpIntelligenceProvider>> {
    let local_metadata_provider = sanitize_metadata_provider_config();

    match build_provider(&local_metadata_provider) {
        Ok(provider) => Some(Arc::from(provider)),
        Err(error) => {
            log::warn!(
                "[IpReputation] failed to initialize local GeoLite2 metadata provider (configured kind {:?} ignored): {}",
                config.metadata_provider.kind,
                error,
            );
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_known_asn_uses_asn_category_directly() {
        let manager = IpReputationManager::from_config(IpReputationConfig::default());
        let mobile_asn = asn_classifier::AsnInfo {
            name: "China Mobile".to_string(),
            category: asn_classifier::AsnCategory::Mobile,
        };

        assert_eq!(manager.resolve_ip_type(&mobile_asn), IpType::Mobile);
    }

    #[test]
    fn test_residential_reputation_is_observed_not_verified_by_default() {
        let manager = IpReputationManager::from_config(IpReputationConfig::default());
        let asn_info = asn_classifier::AsnInfo {
            name: "Comcast Cable".to_string(),
            category: asn_classifier::AsnCategory::Residential,
        };

        let evidence =
            manager.build_reputation_evidence(Some(7922), Some("Comcast Cable"), &asn_info, &IpType::Residential, None);
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
    fn test_unknown_asn_reputation_uses_default_evidence() {
        let manager = IpReputationManager::from_config(IpReputationConfig::default());
        let unknown_asn = asn_classifier::AsnInfo {
            name: "Unknown".to_string(),
            category: asn_classifier::AsnCategory::Unknown,
        };

        let default_evidence = manager.build_reputation_evidence(None, None, &unknown_asn, &IpType::Unknown, None);

        assert_eq!(manager.resolve_ip_type(&unknown_asn), IpType::Unknown);
        assert!(
            default_evidence
                .iter()
                .any(|item| item.kind == IpReputationEvidenceKind::Default)
        );
    }

    #[tokio::test]
    async fn test_unknown_public_ip_without_asn_metadata_stays_unknown() {
        let manager = IpReputationManager::from_config(IpReputationConfig::default());
        let unknown_asn = asn_classifier::AsnInfo {
            name: "Unknown".to_string(),
            category: asn_classifier::AsnCategory::Unknown,
        };

        assert_eq!(manager.resolve_ip_type(&unknown_asn), IpType::Unknown);
    }

    #[tokio::test]
    async fn test_fraud_score_calculation() {
        let manager = IpReputationManager::from_config(IpReputationConfig::default());
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

    #[test]
    fn test_metadata_provider_evidence_is_recorded_when_lookup_returns_asn() {
        let manager = IpReputationManager::from_config(IpReputationConfig::default());
        let asn_info = asn_classifier::AsnInfo {
            name: "Example ISP".to_string(),
            category: asn_classifier::AsnCategory::Unknown,
        };

        let evidence = manager.build_reputation_evidence(
            Some(64500),
            Some("Example ISP"),
            &asn_info,
            &IpType::Unknown,
            Some("GeoLite2 ASN MMDB"),
        );

        assert!(
            evidence
                .iter()
                .any(|item| item.kind == IpReputationEvidenceKind::MetadataProvider)
        );
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
