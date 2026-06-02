use anyhow::Result;
use serde::{Deserialize, Serialize};
use tauri_plugin_mihomo::MihomoExt as _;
use tauri_plugin_mihomo::models::EgressStatus;

use crate::core::ip_reputation::IpReputation;
use crate::core::runtime_diagnostics::geoip::fetch_public_ip_observation;
use crate::utils::network::{NetworkManager, ProxyType};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum CurrentEgressIdentitySource {
    MihomoEgressStatus,
    MihomoConnectionMetadata,
    PublicIpObservation,
    Unavailable,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CurrentEgressIdentity {
    pub source: CurrentEgressIdentitySource,
    pub proxy_name: Option<String>,
    pub proxy_chain: Vec<String>,
    pub egress_ip: Option<String>,
    pub remote_destination: Option<String>,
    pub destination_asn: Option<String>,
    pub asn_org: Option<String>,
    pub rule: Option<String>,
    pub rule_payload: Option<String>,
    pub updated_at: Option<String>,
    pub reputation: Option<IpReputation>,
    pub message: String,
}

pub async fn build_current_egress_identity(
    app_handle: Option<&tauri::AppHandle>,
) -> Result<CurrentEgressIdentity> {
    if let Some(app_handle) = app_handle {
        if let Ok(Some(identity)) = current_identity_from_mihomo_egress_status(app_handle).await {
            return Ok(identity);
        }

        if let Ok(Some(identity)) = current_identity_from_mihomo_connections(app_handle).await {
            return Ok(identity);
        }
    }

    current_identity_from_public_ip().await
}

async fn current_identity_from_mihomo_egress_status(
    app_handle: &tauri::AppHandle,
) -> Result<Option<CurrentEgressIdentity>> {
    let mihomo = app_handle.mihomo().read().await;
    let status = mihomo.get_egress_status().await?;
    drop(mihomo);

    let Some(mut identity) = current_identity_from_egress_status(&status) else {
        return Ok(None);
    };

    if let Some(ip) = identity.egress_ip.as_deref() {
        identity.reputation = Some(
            crate::feat::get_ip_reputation_manager()
                .inspect_ip_metadata(ip)
                .await?,
        );
    }

    Ok(Some(identity))
}

fn current_identity_from_egress_status(status: &EgressStatus) -> Option<CurrentEgressIdentity> {
    let egress_ip = status.egress_ip.as_deref().and_then(non_empty_string);
    let remote_destination = status
        .remote_destination
        .as_deref()
        .and_then(non_empty_string);
    let proxy_name = status.proxy_name.as_deref().and_then(non_empty_string);
    let proxy_chain = status
        .proxy_chain
        .as_deref()
        .map(split_proxy_chain)
        .unwrap_or_default();
    let destination_asn = status.destination_asn.as_deref().and_then(non_empty_string);
    let asn_org = status.asn_org.as_deref().and_then(non_empty_string);
    let rule = status.rule.as_deref().and_then(non_empty_string);
    let rule_payload = status.rule_payload.as_deref().and_then(non_empty_string);
    let updated_at = status.updated_at.as_deref().and_then(non_empty_string);

    if egress_ip.is_none()
        && proxy_name.is_none()
        && proxy_chain.is_empty()
        && destination_asn.is_none()
        && asn_org.is_none()
        && rule.is_none()
        && rule_payload.is_none()
    {
        return None;
    }

    Some(CurrentEgressIdentity {
        source: CurrentEgressIdentitySource::MihomoEgressStatus,
        proxy_name,
        proxy_chain,
        egress_ip,
        remote_destination,
        destination_asn,
        asn_org,
        rule,
        rule_payload,
        updated_at,
        reputation: None,
        message: "identity derived from Mihomo egress status snapshot".to_string(),
    })
}

async fn current_identity_from_mihomo_connections(
    app_handle: &tauri::AppHandle,
) -> Result<Option<CurrentEgressIdentity>> {
    let mihomo = app_handle.mihomo().read().await;
    let connections = mihomo.get_connections().await?;
    drop(mihomo);

    let Some(connection) = connections.connections.as_ref().and_then(|items| items.first()) else {
        return Ok(None);
    };

    let proxy_chain = connection.chains.clone();
    let proxy_name = proxy_chain.first().cloned().or_else(|| {
        let special = connection.metadata.special_proxy.trim();
        (!special.is_empty()).then(|| special.to_string())
    });
    let destination_asn = non_empty_string(&connection.metadata.destination_ip_asn);

    Ok(Some(CurrentEgressIdentity {
        source: CurrentEgressIdentitySource::MihomoConnectionMetadata,
        proxy_name,
        proxy_chain,
        egress_ip: None,
        remote_destination: None,
        destination_asn,
        asn_org: None,
        rule: None,
        rule_payload: None,
        updated_at: None,
        reputation: None,
        message: "proxy identity derived from Mihomo connection metadata".to_string(),
    }))
}

async fn current_identity_from_public_ip() -> Result<CurrentEgressIdentity> {
    let network_manager = NetworkManager::new();
    let client = network_manager
        .create_request(ProxyType::Localhost, Some(8), None, false)
        .await?;
    let observation = fetch_public_ip_observation(&client).await?;
    let egress_ip = observation.ip.map(|ip| ip.to_string());
    let reputation = match egress_ip.as_deref() {
        Some(ip) => Some(crate::feat::get_ip_reputation_manager().inspect_ip_metadata(ip).await?),
        None => None,
    };

    Ok(CurrentEgressIdentity {
        source: CurrentEgressIdentitySource::PublicIpObservation,
        proxy_name: None,
        proxy_chain: Vec::new(),
        egress_ip,
        remote_destination: None,
        destination_asn: None,
        asn_org: None,
        rule: None,
        rule_payload: None,
        updated_at: None,
        reputation,
        message: "identity derived from current local-core public IP observation".to_string(),
    })
}

fn non_empty_string(value: impl AsRef<str>) -> Option<String> {
    let value = value.as_ref();
    let trimmed = value.trim();
    (!trimmed.is_empty()).then(|| trimmed.to_string())
}

fn split_proxy_chain(value: &str) -> Vec<String> {
    value
        .split("->")
        .filter_map(non_empty_string)
        .collect()
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
            remote_destination: Some("203.0.113.10:443".to_string()),
            proxy_name: Some("HK-Residential".to_string()),
            proxy_chain: Some("HK-Residential -> DIRECT".to_string()),
            destination_asn: Some("AS7922".to_string()),
            asn_org: Some("Comcast Cable Communications, LLC".to_string()),
            rule: Some("MATCH".to_string()),
            rule_payload: Some("".to_string()),
            updated_at: Some("2026-06-02T02:00:00Z".to_string()),
        };

        let identity = current_identity_from_egress_status(&status).unwrap();

        assert_eq!(identity.source, CurrentEgressIdentitySource::MihomoEgressStatus);
        assert_eq!(identity.egress_ip.as_deref(), Some("203.0.113.10"));
        assert_eq!(
            identity.remote_destination.as_deref(),
            Some("203.0.113.10:443")
        );
        assert_eq!(identity.proxy_name.as_deref(), Some("HK-Residential"));
        assert_eq!(
            identity.proxy_chain,
            vec!["HK-Residential".to_string(), "DIRECT".to_string()]
        );
        assert_eq!(identity.destination_asn.as_deref(), Some("AS7922"));
        assert_eq!(identity.asn_org.as_deref(), Some("Comcast Cable Communications, LLC"));
        assert_eq!(identity.rule.as_deref(), Some("MATCH"));
        assert_eq!(identity.rule_payload, None);
        assert_eq!(identity.updated_at.as_deref(), Some("2026-06-02T02:00:00Z"));
    }
}
