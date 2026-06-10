use anyhow::{Result, anyhow};
use reqwest::{Client, Proxy};
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tauri_plugin_mihomo::MihomoExt as _;

use crate::config::{ResidentialProxy, ResidentialProxyType};
use crate::core::ip_reputation::{IpReputation, ResidentialVerificationState};
use crate::core::runtime_diagnostics::geoip::{
    GeoIpInfo, PUBLIC_IP_PROBE_HOSTS, fetch_public_ip_observation,
};

const RESIDENTIAL_VERIFY_GROUP: &str = "VERGE-RES-VERIFY";
const RESIDENTIAL_VERIFY_RULE_SOURCE: &str = "residential-verification";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum ResidentialProxyVerificationStatus {
    Verified,
    Observed,
    Rejected,
    NeedsMihomoProbe,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResidentialProxyVerification {
    pub proxy_name: String,
    pub status: ResidentialProxyVerificationStatus,
    pub egress_ip: Option<String>,
    pub reputation: Option<IpReputation>,
    pub probe_method: ResidentialProbeMethod,
    pub mihomo_proxy_name: Option<String>,
    pub message: String,
    pub checked_at: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum ResidentialProbeMethod {
    DirectProxy,
    MihomoCore,
}

pub async fn verify_residential_proxy(
    proxy: ResidentialProxy,
    app_handle: Option<&tauri::AppHandle>,
) -> Result<ResidentialProxyVerification> {
    let Some(proxy_url) = residential_proxy_url(&proxy) else {
        return verify_residential_proxy_via_mihomo(proxy, app_handle).await;
    };

    let client = Client::builder()
        .timeout(Duration::from_secs(12))
        .connect_timeout(Duration::from_secs(8))
        .proxy(Proxy::all(proxy_url)?)
        .build()?;

    let observation = fetch_public_ip_observation(&client).await?;
    let (egress_ip, reputation) = reputation_from_observation(observation).await?;
    let status = status_from_reputation(&reputation);
    let message = verification_message(status.clone(), &reputation);

    Ok(ResidentialProxyVerification {
        proxy_name: proxy.name,
        status,
        egress_ip: Some(egress_ip.to_string()),
        reputation: Some(reputation),
        probe_method: ResidentialProbeMethod::DirectProxy,
        mihomo_proxy_name: None,
        message,
        checked_at: current_timestamp_ms(),
    })
}

async fn verify_residential_proxy_via_mihomo(
    proxy: ResidentialProxy,
    app_handle: Option<&tauri::AppHandle>,
) -> Result<ResidentialProxyVerification> {
    let mihomo_proxy_name = mihomo_residential_proxy_name(&proxy);
    let (fixed_proxy, probe_rules) = match app_handle {
        Some(handle) => (
            Some(MihomoSelectionGuard::select(handle, RESIDENTIAL_VERIFY_GROUP, &mihomo_proxy_name).await?),
            Some(MihomoProbeRuleGuard::create(handle).await?),
        ),
        None => (None, None),
    };

    let result = observe_mihomo_egress(&proxy, &mihomo_proxy_name).await;

    let mut restore_errors = Vec::new();
    if let Some(guard) = probe_rules {
        if let Err(err) = guard.restore().await {
            restore_errors.push(format!("probe rules: {err}"));
        }
    }

    if let Some(guard) = fixed_proxy {
        if let Err(err) = guard.restore().await {
            restore_errors.push(format!("selection: {err}"));
        }
    }

    if !restore_errors.is_empty() {
        return Err(anyhow!(
            "failed to restore Mihomo verification state: {}",
            restore_errors.join("; ")
        ));
    }

    result
}

struct MihomoProbeRuleGuard<'a> {
    app_handle: &'a tauri::AppHandle,
    rule_indexes: Vec<i32>,
}

impl<'a> MihomoProbeRuleGuard<'a> {
    async fn create(app_handle: &'a tauri::AppHandle) -> Result<Self> {
        let mihomo = app_handle.mihomo().read().await;
        let mut rule_indexes = Vec::new();

        for host in PUBLIC_IP_PROBE_HOSTS {
            let index = mihomo
                .create_rule(
                    "DOMAIN",
                    host,
                    RESIDENTIAL_VERIFY_GROUP,
                    Some(RESIDENTIAL_VERIFY_RULE_SOURCE),
                    None,
                    Some("prepend"),
                )
                .await?;
            if index >= 0 {
                rule_indexes.push(index);
            }
        }

        drop(mihomo);
        Ok(Self {
            app_handle,
            rule_indexes,
        })
    }

    async fn restore(self) -> Result<()> {
        let mihomo = self.app_handle.mihomo().read().await;
        for index in self.rule_indexes.into_iter().rev() {
            mihomo.delete_rule(index).await?;
        }

        Ok(())
    }
}

async fn observe_mihomo_egress(
    proxy: &ResidentialProxy,
    mihomo_proxy_name: &str,
) -> Result<ResidentialProxyVerification> {
    let client = build_mihomo_proxy_client().await?;
    let observation = fetch_public_ip_observation(&client).await?;
    let (egress_ip, reputation) = reputation_from_observation(observation).await?;
    let status = status_from_reputation(&reputation);
    let message = format!(
        "{}; observed through local Mihomo core with {} selected in {}",
        verification_message(status.clone(), &reputation),
        mihomo_proxy_name,
        RESIDENTIAL_VERIFY_GROUP
    );

    Ok(ResidentialProxyVerification {
        proxy_name: proxy.name.clone(),
        status,
        egress_ip: Some(egress_ip),
        reputation: Some(reputation),
        probe_method: ResidentialProbeMethod::MihomoCore,
        mihomo_proxy_name: Some(mihomo_proxy_name.to_string()),
        message,
        checked_at: current_timestamp_ms(),
    })
}

struct MihomoSelectionGuard<'a> {
    app_handle: &'a tauri::AppHandle,
    group_name: String,
    previous_node: Option<String>,
}

impl<'a> MihomoSelectionGuard<'a> {
    async fn select(app_handle: &'a tauri::AppHandle, group_name: &str, node_name: &str) -> Result<Self> {
        let mihomo = app_handle.mihomo().read().await;
        let target = mihomo.get_proxy_by_name(node_name).await?;
        if !target.alive {
            return Err(anyhow!("mihomo proxy {node_name} is not alive"));
        }

        let group = mihomo.get_group_by_name(group_name).await?;
        let previous_node = group.now.clone().filter(|node| !node.is_empty());
        let selectable = group
            .all
            .as_ref()
            .map(|nodes| nodes.iter().any(|node| node == node_name))
            .unwrap_or(false);
        if !selectable {
            return Err(anyhow!("mihomo group {group_name} does not contain {node_name}"));
        }

        mihomo.select_node_for_group(group_name, node_name).await?;
        drop(mihomo);

        Ok(Self {
            app_handle,
            group_name: group_name.to_string(),
            previous_node,
        })
    }

    async fn restore(self) -> Result<()> {
        if let Some(previous_node) = self.previous_node {
            let mihomo = self.app_handle.mihomo().read().await;
            mihomo.select_node_for_group(&self.group_name, &previous_node).await?;
        }

        Ok(())
    }
}

async fn build_mihomo_proxy_client() -> Result<Client> {
    let mixed_port = local_mihomo_mixed_port().await;
    let proxy_url = format!("http://127.0.0.1:{mixed_port}");

    Ok(Client::builder()
        .timeout(Duration::from_secs(12))
        .connect_timeout(Duration::from_secs(8))
        .proxy(Proxy::all(proxy_url)?)
        .build()?)
}

async fn local_mihomo_mixed_port() -> u16 {
    let verge_port = crate::config::Config::verge().await.data_arc().verge_mixed_port;

    match verge_port {
        Some(port) => port,
        None => crate::config::Config::clash().await.data_arc().get_mixed_port(),
    }
}

async fn reputation_from_observation(observation: GeoIpInfo) -> Result<(String, IpReputation)> {
    let egress_ip = observation
        .ip
        .ok_or_else(|| anyhow!("residential proxy egress lookup returned no IP"))?;
    let reputation = crate::feat::get_ip_reputation_manager()
        .inspect_ip_metadata(&egress_ip)
        .await?;

    Ok((egress_ip.to_string(), reputation))
}

pub fn status_from_reputation(reputation: &IpReputation) -> ResidentialProxyVerificationStatus {
    match reputation.residential_state {
        ResidentialVerificationState::VerifiedResidential => ResidentialProxyVerificationStatus::Verified,
        ResidentialVerificationState::ObservedResidential => ResidentialProxyVerificationStatus::Observed,
        ResidentialVerificationState::NotResidential => ResidentialProxyVerificationStatus::Rejected,
        ResidentialVerificationState::Unknown => ResidentialProxyVerificationStatus::Failed,
    }
}

fn verification_message(status: ResidentialProxyVerificationStatus, reputation: &IpReputation) -> String {
    match status {
        ResidentialProxyVerificationStatus::Verified => {
            format!(
                "egress verified as residential with confidence {}",
                reputation.confidence
            )
        }
        ResidentialProxyVerificationStatus::Observed => {
            format!(
                "egress has residential-like evidence with confidence {}",
                reputation.confidence
            )
        }
        ResidentialProxyVerificationStatus::Rejected => {
            format!("egress is not residential-like: {:?}", reputation.ip_type)
        }
        ResidentialProxyVerificationStatus::Failed => "egress reputation is inconclusive".to_string(),
        ResidentialProxyVerificationStatus::NeedsMihomoProbe => {
            "proxy type requires Mihomo-assisted verification".to_string()
        }
    }
}

fn mihomo_residential_proxy_name(proxy: &ResidentialProxy) -> String {
    format!("VERGE-RES-{}", proxy.name)
}

fn residential_proxy_url(proxy: &ResidentialProxy) -> Option<String> {
    let scheme = match proxy.proxy_type {
        ResidentialProxyType::Socks5 => "socks5",
        ResidentialProxyType::Http => "http",
        ResidentialProxyType::Ss | ResidentialProxyType::Vmess | ResidentialProxyType::Trojan => {
            return None;
        }
    };

    let authority = match (&proxy.username, &proxy.password) {
        (Some(username), Some(password)) => format!("{username}:{password}@{}:{}", proxy.server, proxy.port),
        (Some(username), None) => format!("{username}@{}:{}", proxy.server, proxy.port),
        _ => format!("{}:{}", proxy.server, proxy.port),
    };

    Some(format!("{scheme}://{authority}"))
}

fn current_timestamp_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_millis() as u64)
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::ip_reputation::{IpType, RiskLevel};
    use std::time::SystemTime;

    fn proxy(proxy_type: ResidentialProxyType) -> ResidentialProxy {
        ResidentialProxy {
            name: "test-res".to_string(),
            proxy_type,
            server: "127.0.0.1".to_string(),
            port: 1080,
            username: None,
            password: None,
            cipher: None,
            uuid: None,
            trojan_password: None,
            tls: None,
            sni: None,
            skip_cert_verify: None,
            region: None,
            enabled: true,
        }
    }

    fn reputation(state: ResidentialVerificationState) -> IpReputation {
        IpReputation {
            ip: "203.0.113.10".to_string(),
            ip_type: IpType::Residential,
            asn: "AS7922".to_string(),
            asn_org: "Comcast Cable".to_string(),
            fraud_score: 15,
            risk_level: RiskLevel::Low,
            confidence: 80,
            evidence: Vec::new(),
            residential_state: state,
            is_proxy: false,
            is_vpn: false,
            is_tor: false,
            country_code: "US".to_string(),
            city: None,
            timezone: Some("America/New_York".to_string()),
            checked_at: SystemTime::UNIX_EPOCH,
        }
    }

    #[test]
    fn test_direct_probe_supports_http_and_socks5_only() {
        assert_eq!(
            residential_proxy_url(&proxy(ResidentialProxyType::Http)).unwrap(),
            "http://127.0.0.1:1080"
        );
        assert_eq!(
            residential_proxy_url(&proxy(ResidentialProxyType::Socks5)).unwrap(),
            "socks5://127.0.0.1:1080"
        );
        assert!(residential_proxy_url(&proxy(ResidentialProxyType::Ss)).is_none());
        assert!(residential_proxy_url(&proxy(ResidentialProxyType::Vmess)).is_none());
        assert!(residential_proxy_url(&proxy(ResidentialProxyType::Trojan)).is_none());
    }

    #[test]
    fn test_status_from_reputation_preserves_observed_vs_verified() {
        assert_eq!(
            status_from_reputation(&reputation(ResidentialVerificationState::VerifiedResidential)),
            ResidentialProxyVerificationStatus::Verified
        );
        assert_eq!(
            status_from_reputation(&reputation(ResidentialVerificationState::ObservedResidential)),
            ResidentialProxyVerificationStatus::Observed
        );
        assert_eq!(
            status_from_reputation(&reputation(ResidentialVerificationState::NotResidential)),
            ResidentialProxyVerificationStatus::Rejected
        );
    }

    #[test]
    fn test_non_direct_protocols_have_mihomo_proxy_name() {
        assert_eq!(
            mihomo_residential_proxy_name(&proxy(ResidentialProxyType::Vmess)),
            "VERGE-RES-test-res"
        );
    }
}
