use super::{
    constants::*,
    geoip::{build_proxy_detection_location, fetch_public_ip_observation, has_proxy_detection_location_delta},
    helpers::current_timestamp_ms,
    input::{DiagnosticsInput, build_diagnostics_input},
};
use crate::core::{
    ip_reputation::IpReputation, runtime_snapshot::RuntimeSnapshotService, runtime_status::ProxyDetectionResult,
};
use crate::utils::network::{NetworkManager, ProxyType};
use anyhow::Result;
use smartstring::alias::String;

fn build_proxy_detection_assessment(
    direct_observed: bool,
    proxy_observed: bool,
    proxy_effective: bool,
    runtime_risk_detected: bool,
) -> &'static str {
    if direct_observed && proxy_observed {
        if proxy_effective {
            PROXY_DETECTION_ASSESSMENT_EFFECTIVE
        } else {
            PROXY_DETECTION_ASSESSMENT_SAME_EGRESS
        }
    } else if runtime_risk_detected {
        PROXY_DETECTION_ASSESSMENT_RUNTIME_RISK
    } else {
        PROXY_DETECTION_ASSESSMENT_INCONCLUSIVE
    }
}

fn build_proxy_detection_confidence(direct_observed: bool, proxy_observed: bool) -> &'static str {
    if direct_observed && proxy_observed {
        PROXY_DETECTION_CONFIDENCE_HIGH
    } else if direct_observed || proxy_observed {
        PROXY_DETECTION_CONFIDENCE_MEDIUM
    } else {
        PROXY_DETECTION_CONFIDENCE_LOW
    }
}

fn proxy_detection_runtime_risks_from_input(input: &DiagnosticsInput) -> Vec<String> {
    if input.core_running {
        Vec::new()
    } else {
        vec!["core-not-running".into()]
    }
}

fn build_proxy_detection_recommendations(
    core_running: bool,
    proxy_effective: bool,
    ip_changed: bool,
    location_changed: bool,
    runtime_risk_type: &[String],
    observation_path: &str,
    observation_incomplete: bool,
    proxy_reputation: Option<&IpReputation>,
) -> Vec<String> {
    let mut recommendations = Vec::new();

    if proxy_effective {
        if ip_changed {
            recommendations.push("Direct and proxy egress IPs differ; app traffic is using a different exit.".into());
        }

        if location_changed {
            recommendations.push(
                "Direct and proxy egress locations differ; combine this with IP reputation for quality checks.".into(),
            );
        }

        if let Some(reputation) = proxy_reputation {
            recommendations.push(
                format!(
                    "Proxy egress reputation: {:?}, score {}, ASN {}.",
                    reputation.ip_type, reputation.fraud_score, reputation.asn
                )
                .into(),
            );
        }

        if recommendations.is_empty() {
            recommendations.push("Proxy egress differs from direct egress; the proxy path appears effective.".into());
        }

        return recommendations;
    }

    for risk in runtime_risk_type {
        match risk.as_str() {
            "core-not-running" => recommendations
                .push("Local core is not running; start it before testing proxy egress.".into()),
            "direct-egress-unavailable" => recommendations.push(
                "Direct egress could not be observed; check direct access to external IP lookup services."
                    .into(),
            ),
            "local-core-proxy-unreachable" => recommendations.push(
                "Proxy egress through local core could not be observed; check mixed-port, local listener, and core state."
                    .into(),
            ),
            "proxy-reputation-unavailable" => recommendations.push(
                "Proxy egress was observed, but its IP reputation could not be resolved.".into(),
            ),
            _ => {}
        }
    }

    if let Some(reputation) = proxy_reputation {
        recommendations.push(
            format!(
                "Proxy egress reputation: {:?}, score {}, ASN {}.",
                reputation.ip_type, reputation.fraud_score, reputation.asn
            )
            .into(),
        );
    }

    if core_running && observation_path == PROXY_DETECTION_OBSERVATION_DIRECT_VS_CORE_PROXY {
        recommendations.push(
            "Direct and proxy egress were compared but no clear exit change was observed; check rule matching and selected node."
                .into(),
        );
    }

    if observation_incomplete {
        recommendations
            .push("Observation is incomplete; retry when both direct and local-core proxy paths are available.".into());
    }

    if recommendations.is_empty() {
        recommendations
            .push("No clear proxy egress change was observed; check proxy mode, rule path, and current node.".into());
    }

    recommendations
}

fn build_proxy_detection_result_from_observations(
    core_running: bool,
    direct_info: Option<super::geoip::GeoIpInfo>,
    proxy_info: Option<super::geoip::GeoIpInfo>,
    proxy_reputation: Option<IpReputation>,
    mut runtime_risk_type: Vec<String>,
    warnings: Vec<String>,
) -> ProxyDetectionResult {
    runtime_risk_type.sort();
    runtime_risk_type.dedup();

    let direct_observed = direct_info.as_ref().and_then(|info| info.ip.as_ref()).is_some();
    let proxy_observed = proxy_info.as_ref().and_then(|info| info.ip.as_ref()).is_some();
    let checked_via_core_proxy = proxy_observed;

    let observation_path = if direct_observed && proxy_observed {
        PROXY_DETECTION_OBSERVATION_DIRECT_VS_CORE_PROXY
    } else if proxy_observed {
        PROXY_DETECTION_OBSERVATION_CORE_PROXY_ONLY
    } else {
        PROXY_DETECTION_OBSERVATION_DIRECT_ONLY
    };

    let ip_changed = match (
        direct_info.as_ref().and_then(|info| info.ip.as_deref()),
        proxy_info.as_ref().and_then(|info| info.ip.as_deref()),
    ) {
        (Some(direct_ip), Some(proxy_ip)) => direct_ip != proxy_ip,
        _ => false,
    };

    let location_changed = match (direct_info.as_ref(), proxy_info.as_ref()) {
        (Some(direct_info), Some(proxy_info)) => has_proxy_detection_location_delta(direct_info, proxy_info),
        _ => false,
    };

    let proxy_effective = ip_changed || location_changed;
    let observation_incomplete = !(direct_observed && proxy_observed);
    let runtime_risk_detected = !runtime_risk_type.is_empty();
    let assessment =
        build_proxy_detection_assessment(direct_observed, proxy_observed, proxy_effective, runtime_risk_detected);
    let confidence = build_proxy_detection_confidence(direct_observed, proxy_observed);
    let recommendations = build_proxy_detection_recommendations(
        core_running,
        proxy_effective,
        ip_changed,
        location_changed,
        &runtime_risk_type,
        observation_path,
        observation_incomplete,
        proxy_reputation.as_ref(),
    );
    let error = if !direct_observed && !proxy_observed {
        Some("Unable to observe either direct or proxy egress for app traffic".into())
    } else {
        None
    };

    ProxyDetectionResult {
        checked: true,
        core_running,
        direct_observed,
        proxy_observed,
        checked_via_core_proxy,
        proxy_effective,
        ip_changed,
        location_changed,
        observation_incomplete,
        runtime_risk_detected,
        confidence: confidence.into(),
        assessment: assessment.into(),
        runtime_risk_type,
        warnings: warnings.clone(),
        recommendations,
        direct_ip: direct_info.as_ref().and_then(|info| info.ip.clone()),
        proxy_ip: proxy_info.as_ref().and_then(|info| info.ip.clone()),
        direct_location: direct_info
            .as_ref()
            .and_then(|info| build_proxy_detection_location(info)),
        proxy_location: proxy_info
            .as_ref()
            .and_then(|info| build_proxy_detection_location(info)),
        proxy_reputation,
        observation_path: observation_path.into(),
        error,
        timestamp: current_timestamp_ms(),
    }
}

pub async fn build_proxy_detection_result() -> Result<ProxyDetectionResult> {
    let snapshot_service = RuntimeSnapshotService::global();
    let diagnostics_input = build_diagnostics_input(&snapshot_service).await;
    let core_running = diagnostics_input.core_running;
    let network_manager = NetworkManager::new();
    let mut warnings = Vec::new();
    let mut runtime_risk_type = proxy_detection_runtime_risks_from_input(&diagnostics_input);

    let direct_info = match network_manager
        .create_request(ProxyType::None, Some(8), None, false)
        .await
    {
        Ok(client) => match fetch_public_ip_observation(&client).await {
            Ok(info) if info.ip.is_some() => Some(info),
            Ok(_) => {
                warnings.push("Direct egress lookup returned an incomplete result without IP.".into());
                runtime_risk_type.push("direct-egress-unavailable".into());
                None
            }
            Err(err) => {
                warnings.push(format!("Direct egress lookup failed: {err}").into());
                runtime_risk_type.push("direct-egress-unavailable".into());
                None
            }
        },
        Err(err) => {
            warnings.push(format!("Unable to build direct lookup request: {err}").into());
            runtime_risk_type.push("direct-egress-unavailable".into());
            None
        }
    };

    let proxy_info = if core_running {
        match network_manager
            .create_request(ProxyType::Localhost, Some(8), None, false)
            .await
        {
            Ok(client) => match fetch_public_ip_observation(&client).await {
                Ok(info) if info.ip.is_some() => Some(info),
                Ok(_) => {
                    warnings.push("Local-core proxy egress lookup returned an incomplete result without IP.".into());
                    runtime_risk_type.push("local-core-proxy-unreachable".into());
                    None
                }
                Err(err) => {
                    warnings.push(format!("Local-core proxy egress lookup failed: {err}").into());
                    runtime_risk_type.push("local-core-proxy-unreachable".into());
                    None
                }
            },
            Err(err) => {
                warnings.push(format!("Unable to build local-core proxy lookup request: {err}").into());
                runtime_risk_type.push("local-core-proxy-unreachable".into());
                None
            }
        }
    } else {
        warnings.push("Local core is not running; proxy egress cannot be observed.".into());
        None
    };

    let proxy_reputation = if let Some(proxy_ip) = proxy_info.as_ref().and_then(|info| info.ip.as_deref()) {
        match crate::feat::get_ip_reputation_manager()
            .inspect_ip_metadata(proxy_ip)
            .await
        {
            Ok(reputation) => Some(reputation),
            Err(err) => {
                warnings.push(format!("Proxy egress reputation lookup failed: {err}").into());
                runtime_risk_type.push("proxy-reputation-unavailable".into());
                None
            }
        }
    } else {
        None
    };

    Ok(build_proxy_detection_result_from_observations(
        core_running,
        direct_info,
        proxy_info,
        proxy_reputation,
        runtime_risk_type,
        warnings,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::ip_reputation::{IpReputation, IpType, RiskLevel};
    use std::time::SystemTime;

    #[test]
    fn test_proxy_detection_core_state_from_diagnostics_input() {
        let input = DiagnosticsInput {
            core_running: false,
            ..DiagnosticsInput::default()
        };

        let risks = proxy_detection_runtime_risks_from_input(&input);

        assert_eq!(risks, vec!["core-not-running".to_string()]);
    }

    #[test]
    fn test_proxy_detection_result_includes_proxy_reputation() {
        let proxy_reputation = IpReputation {
            ip: "203.0.113.10".to_string(),
            ip_type: IpType::Datacenter,
            asn: "AS16509".to_string(),
            asn_org: "Amazon AWS".to_string(),
            fraud_score: 85,
            risk_level: RiskLevel::High,
            confidence: 70,
            evidence: vec![crate::core::ip_reputation::IpReputationEvidence {
                kind: crate::core::ip_reputation::IpReputationEvidenceKind::AsnTable,
                label: "ASN table matched AS16509 as Datacenter".to_string(),
                weight: 70,
            }],
            residential_state: crate::core::ip_reputation::ResidentialVerificationState::NotResidential,
            is_proxy: false,
            is_vpn: false,
            is_tor: false,
            country_code: "US".to_string(),
            city: Some("Seattle".to_string()),
            timezone: Some("America/Los_Angeles".to_string()),
            checked_at: SystemTime::UNIX_EPOCH,
        };

        let result = build_proxy_detection_result_from_observations(
            true,
            Some(super::super::geoip::GeoIpInfo {
                ip: Some("198.51.100.10".into()),
                country_code: Some("US".into()),
                ..Default::default()
            }),
            Some(super::super::geoip::GeoIpInfo {
                ip: Some("203.0.113.10".into()),
                country_code: Some("US".into()),
                asn: Some(16509),
                asn_organization: Some("Amazon AWS".into()),
                ..Default::default()
            }),
            Some(proxy_reputation),
            Vec::new(),
            Vec::new(),
        );

        let reputation = result.proxy_reputation.expect("proxy reputation");
        assert_eq!(reputation.ip_type, IpType::Datacenter);
        assert_eq!(reputation.fraud_score, 85);
        assert_eq!(reputation.asn, "AS16509");
    }
}
