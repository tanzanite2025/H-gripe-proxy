use super::{
    constants::*,
    geoip::{build_proxy_detection_location, fetch_public_ip_location, has_proxy_detection_location_delta},
    helpers::current_timestamp_ms,
};
use crate::core::{
    CoreManager,
    manager::RunningMode,
    runtime_status::ProxyDetectionResult,
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

fn build_proxy_detection_recommendations(
    core_running: bool,
    proxy_effective: bool,
    ip_changed: bool,
    location_changed: bool,
    runtime_risk_type: &[String],
    observation_path: &str,
    observation_incomplete: bool,
) -> Vec<String> {
    let mut recommendations = Vec::new();

    if proxy_effective {
        if ip_changed {
            recommendations.push("已观察到代理前后出口 IP 变化，说明软件自身流量出口已发生切换".into());
        }

        if location_changed {
            recommendations.push("已观察到代理前后地理位置变化，可继续结合目标站点校验出口纯净度".into());
        }

        if recommendations.is_empty() {
            recommendations.push("已观察到代理前后出口差异，当前代理检测结果可视为有效".into());
        }

        return recommendations;
    }

    for risk in runtime_risk_type {
        match risk.as_str() {
            "core-not-running" => {
                recommendations.push("当前本地 core 未运行，请先启动代理核心后再检测软件出口".into())
            }
            "direct-egress-unavailable" => {
                recommendations.push("未能观测到直连出口，请检查当前网络是否允许直连访问外部 IP 观测服务".into())
            }
            "local-core-proxy-unreachable" => recommendations.push(
                "未能通过本地 core 代理观测出口，请检查 mixed-port、本地监听和核心运行状态".into(),
            ),
            _ => {}
        }
    }

    if core_running && observation_path == PROXY_DETECTION_OBSERVATION_DIRECT_VS_CORE_PROXY {
        recommendations.push(
            "已完成直连与本地 core 出口对比，但未观察到明显出口变化，请检查当前规则命中、节点选择和上游出口纯净度"
                .into(),
        );
    }

    if observation_incomplete {
        recommendations.push("当前观测不完整，请在直连与本地 core 两条路径都可用时重新检测".into());
    }

    if recommendations.is_empty() {
        recommendations.push("未观察到明确的代理出口变化，请检查代理模式、规则链路和当前节点出口".into());
    }

    recommendations
}

pub async fn build_proxy_detection_result() -> Result<ProxyDetectionResult> {
    let core_running = *CoreManager::global().get_running_mode() != RunningMode::NotRunning;
    let network_manager = NetworkManager::new();
    let mut warnings = Vec::new();
    let mut runtime_risk_type = Vec::new();

    let direct_info = match network_manager
        .create_request(ProxyType::None, Some(8), None, false)
        .await
    {
        Ok(client) => match fetch_public_ip_location(&client).await {
            Ok(info) if info.ip.is_some() => Some(info),
            Ok(_) => {
                warnings.push("直连出口观测返回了不完整结果，缺少 IP 字段".into());
                runtime_risk_type.push("direct-egress-unavailable".into());
                None
            }
            Err(err) => {
                warnings.push(format!("直连出口观测失败: {err}").into());
                runtime_risk_type.push("direct-egress-unavailable".into());
                None
            }
        },
        Err(err) => {
            warnings.push(format!("无法建立直连观测请求: {err}").into());
            runtime_risk_type.push("direct-egress-unavailable".into());
            None
        }
    };

    let proxy_info = if core_running {
        match network_manager
            .create_request(ProxyType::Localhost, Some(8), None, false)
            .await
        {
            Ok(client) => match fetch_public_ip_location(&client).await {
                Ok(info) if info.ip.is_some() => Some(info),
                Ok(_) => {
                    warnings.push("本地 core 代理出口观测返回了不完整结果，缺少 IP 字段".into());
                    runtime_risk_type.push("local-core-proxy-unreachable".into());
                    None
                }
                Err(err) => {
                    warnings.push(format!("本地 core 代理出口观测失败: {err}").into());
                    runtime_risk_type.push("local-core-proxy-unreachable".into());
                    None
                }
            },
            Err(err) => {
                warnings.push(format!("无法建立本地 core 代理观测请求: {err}").into());
                runtime_risk_type.push("local-core-proxy-unreachable".into());
                None
            }
        }
    } else {
        warnings.push("当前本地 core 未运行，无法观测软件代理出口".into());
        runtime_risk_type.push("core-not-running".into());
        None
    };

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
        (Some(direct_info), Some(proxy_info)) => {
            has_proxy_detection_location_delta(direct_info, proxy_info)
        }
        _ => false,
    };

    let proxy_effective = ip_changed || location_changed;
    let observation_incomplete = !(direct_observed && proxy_observed);
    let runtime_risk_detected = !runtime_risk_type.is_empty();
    let assessment = build_proxy_detection_assessment(
        direct_observed,
        proxy_observed,
        proxy_effective,
        runtime_risk_detected,
    );
    let confidence = build_proxy_detection_confidence(direct_observed, proxy_observed);
    let recommendations = build_proxy_detection_recommendations(
        core_running,
        proxy_effective,
        ip_changed,
        location_changed,
        &runtime_risk_type,
        observation_path,
        observation_incomplete,
    );
    let error = if !direct_observed && !proxy_observed {
        Some("无法观测到软件流量的直连或代理出口".into())
    } else {
        None
    };

    Ok(ProxyDetectionResult {
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
        observation_path: observation_path.into(),
        error,
        timestamp: current_timestamp_ms(),
    })
}
