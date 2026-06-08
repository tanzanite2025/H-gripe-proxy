use anyhow::Result;
use serde::{Deserialize, Serialize};
use tauri_plugin_mihomo::MihomoExt as _;
use tauri_plugin_mihomo::models::EgressStatus;

use crate::core::{CoreManager, ip_reputation::IpReputation, manager::RunningMode};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum CurrentEgressIdentitySource {
    MihomoEgressStatus,
    Unavailable,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CurrentEgressIdentity {
    pub source: CurrentEgressIdentitySource,
    pub proxy_name: Option<String>,
    pub proxy_chain: Vec<String>,
    pub egress_ip: Option<String>,
    pub public_egress_ip: Option<String>,
    pub proxy_endpoint: Option<String>,
    pub destination_asn: Option<String>,
    pub asn_org: Option<String>,
    pub rule: Option<String>,
    pub rule_payload: Option<String>,
    pub egress_source: Option<String>,
    pub confidence: Option<i64>,
    pub sample_count: Option<i64>,
    pub last_verified_at: Option<String>,
    pub updated_at: Option<String>,
    pub reputation: Option<IpReputation>,
    pub message: String,
}

pub async fn build_current_egress_identity(app_handle: Option<&tauri::AppHandle>) -> Result<CurrentEgressIdentity> {
    let Some(app_handle) = app_handle else {
        return Ok(unavailable_identity("Mihomo app handle is unavailable."));
    };

    match current_identity_from_mihomo_egress_status(app_handle).await {
        Ok(Some(identity)) => Ok(identity),
        Ok(None) => Ok(unavailable_identity(
            "Mihomo has not observed a public egress IP yet. Upgrade the Mihomo core if this persists.",
        )),
        Err(error) => Ok(unavailable_identity(&format!(
            "Failed to query Mihomo egress status: {error}. Upgrade the Mihomo core if this persists."
        ))),
    }
}

async fn current_identity_from_mihomo_egress_status(
    app_handle: &tauri::AppHandle,
) -> Result<Option<CurrentEgressIdentity>> {
    if *CoreManager::global().get_running_mode() == RunningMode::NotRunning {
        return Ok(None);
    }

    crate::feat::ensure_mihomo_core_ready().await?;

    let mihomo = app_handle.mihomo().read().await;
    let status = mihomo.get_egress_status().await?;
    drop(mihomo);

    let Some(mut identity) = current_identity_from_egress_status(&status) else {
        return Ok(None);
    };

    if let Some(ip) = identity.egress_ip.as_deref() {
        identity.reputation = Some(crate::feat::get_ip_reputation_manager().inspect_core_metadata(
            ip,
            identity.destination_asn.as_deref(),
            identity.asn_org.as_deref(),
        ));
    }

    Ok(Some(identity))
}

fn current_identity_from_egress_status(status: &EgressStatus) -> Option<CurrentEgressIdentity> {
    let public_egress_ip = status.public_egress_ip.as_deref().and_then(non_empty_string);
    let egress_ip = public_egress_ip
        .clone()
        .or_else(|| status.egress_ip.as_deref().and_then(non_empty_string));
    let proxy_endpoint = status.proxy_endpoint.as_deref().and_then(non_empty_string);
    let proxy_name = status.proxy_name.as_deref().and_then(non_empty_string);
    let proxy_chain = status.proxy_chain.as_deref().map(split_proxy_chain).unwrap_or_default();
    let destination_asn = status.destination_asn.as_deref().and_then(non_empty_string);
    let asn_org = status.asn_org.as_deref().and_then(non_empty_string);
    let rule = status.rule.as_deref().and_then(non_empty_string);
    let rule_payload = status.rule_payload.as_deref().and_then(non_empty_string);
    let egress_source = status.egress_source.as_deref().and_then(non_empty_string);
    let confidence = status.confidence;
    let sample_count = status.sample_count;
    let last_verified_at = status.last_verified_at.as_deref().and_then(non_empty_string);
    let updated_at = status.updated_at.as_deref().and_then(non_empty_string);

    if egress_ip.is_none() {
        return None;
    }

    Some(CurrentEgressIdentity {
        source: CurrentEgressIdentitySource::MihomoEgressStatus,
        proxy_name,
        proxy_chain,
        egress_ip,
        public_egress_ip,
        proxy_endpoint,
        destination_asn,
        asn_org,
        rule,
        rule_payload,
        egress_source,
        confidence,
        sample_count,
        last_verified_at,
        updated_at,
        reputation: None,
        message: "identity derived from Mihomo egress status snapshot".to_string(),
    })
}

fn unavailable_identity(message: &str) -> CurrentEgressIdentity {
    CurrentEgressIdentity {
        source: CurrentEgressIdentitySource::Unavailable,
        proxy_name: None,
        proxy_chain: Vec::new(),
        egress_ip: None,
        public_egress_ip: None,
        proxy_endpoint: None,
        destination_asn: None,
        asn_org: None,
        rule: None,
        rule_payload: None,
        egress_source: None,
        confidence: None,
        sample_count: None,
        last_verified_at: None,
        updated_at: None,
        reputation: None,
        message: message.to_string(),
    }
}

fn non_empty_string(value: impl AsRef<str>) -> Option<String> {
    let value = value.as_ref();
    let trimmed = value.trim();
    (!trimmed.is_empty()).then(|| trimmed.to_string())
}

fn split_proxy_chain(value: &str) -> Vec<String> {
    value.split("->").filter_map(non_empty_string).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use tauri_plugin_mihomo::models::EgressStatus;

    #[test]
    fn test_non_empty_string_trims_blank_values() {
        assert_eq!(non_empty_string(" AS7922 "), Some("AS7922".to_string()));
        assert_eq!(non_empty_string(""), None);
        assert_eq!(non_empty_string("   "), None);
    }

    #[test]
    fn test_current_identity_prefers_mihomo_egress_status_snapshot() {
        let status = EgressStatus {
            stable: true,
            change_count: 0,
            observed_count: Some(1),
            egress_ip: Some("203.0.113.10".to_string()),
            public_egress_ip: Some("203.0.113.10".to_string()),
            proxy_endpoint: Some("198.51.100.1:443".to_string()),
            proxy_name: Some("HK-Residential".to_string()),
            proxy_chain: Some("HK-Residential -> DIRECT".to_string()),
            destination_asn: Some("AS7922".to_string()),
            asn_org: Some("Comcast Cable Communications, LLC".to_string()),
            rule: Some("MATCH".to_string()),
            rule_payload: Some("".to_string()),
            egress_source: Some("publicProbe".to_string()),
            confidence: Some(90),
            sample_count: Some(1),
            last_verified_at: Some("2026-06-02T02:00:00Z".to_string()),
            updated_at: Some("2026-06-02T02:00:00Z".to_string()),
        };

        let identity = current_identity_from_egress_status(&status).unwrap();

        assert_eq!(identity.source, CurrentEgressIdentitySource::MihomoEgressStatus);
        assert_eq!(identity.egress_ip.as_deref(), Some("203.0.113.10"));
        assert_eq!(identity.public_egress_ip.as_deref(), Some("203.0.113.10"));
        assert_eq!(identity.proxy_endpoint.as_deref(), Some("198.51.100.1:443"));
        assert_eq!(identity.proxy_name.as_deref(), Some("HK-Residential"));
        assert_eq!(
            identity.proxy_chain,
            vec!["HK-Residential".to_string(), "DIRECT".to_string()]
        );
        assert_eq!(identity.destination_asn.as_deref(), Some("AS7922"));
        assert_eq!(identity.asn_org.as_deref(), Some("Comcast Cable Communications, LLC"));
        assert_eq!(identity.rule.as_deref(), Some("MATCH"));
        assert_eq!(identity.rule_payload, None);
        assert_eq!(identity.egress_source.as_deref(), Some("publicProbe"));
        assert_eq!(identity.confidence, Some(90));
        assert_eq!(identity.sample_count, Some(1));
        assert_eq!(identity.last_verified_at.as_deref(), Some("2026-06-02T02:00:00Z"));
        assert_eq!(identity.updated_at.as_deref(), Some("2026-06-02T02:00:00Z"));
    }

    #[test]
    fn test_current_identity_requires_observed_egress_ip() {
        let status = EgressStatus {
            stable: false,
            change_count: 0,
            observed_count: Some(0),
            egress_ip: None,
            public_egress_ip: None,
            proxy_endpoint: Some("198.51.100.1:443".to_string()),
            proxy_name: Some("HK-Node".to_string()),
            proxy_chain: Some("HK-Node -> DIRECT".to_string()),
            destination_asn: Some("AS16509".to_string()),
            asn_org: Some("Amazon AWS".to_string()),
            rule: Some("MATCH".to_string()),
            rule_payload: None,
            egress_source: None,
            confidence: Some(10),
            sample_count: Some(0),
            last_verified_at: None,
            updated_at: None,
        };

        assert!(current_identity_from_egress_status(&status).is_none());
    }
}
