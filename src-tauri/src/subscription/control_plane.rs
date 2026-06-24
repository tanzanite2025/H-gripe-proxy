use super::fetch::{FetchedSubscriptionPayload, fetch_remote_profile};
use crate::{
    config::{Config, PrfOption},
    core::{
        clash_mode::ClashMode,
        handle::Handle,
        mihomo_runtime_guard::{MihomoRuleGuard, MihomoRuntimeRuleSpec, MihomoSelectionGuard},
        runtime_lifecycle,
        runtime_snapshot::read_subscription_control_plane_topology,
    },
    enhance::subscription_update::SUBSCRIPTION_UPDATE_GROUP,
};
use anyhow::{Context as _, Result, anyhow};
use serde_yaml_ng::Value;
use smartstring::alias::String;
use std::net::IpAddr;
use tauri::Url;
use tauri_plugin_mihomo::models::{Proxies, Proxy};

const SUBSCRIPTION_UPDATE_RULE_SOURCE: &str = "subscription-update";

pub async fn subscription_update_uses_dedicated_control_plane() -> bool {
    if runtime_lifecycle::runtime_is_not_running() {
        return false;
    }

    let tun_enabled = Config::verge().await.latest_arc().enable_tun_mode.unwrap_or(false);
    if !tun_enabled {
        return false;
    }

    Config::runtime()
        .await
        .latest_arc()
        .config
        .as_ref()
        .and_then(|config| config.get("mode"))
        .and_then(Value::as_str)
        .and_then(|mode| mode.parse::<ClashMode>().ok())
        .is_some_and(|mode| mode == ClashMode::Global)
}

pub async fn fetch_subscription_update_via_control_plane(
    url: &str,
    option: &PrfOption,
) -> Result<FetchedSubscriptionPayload> {
    let parsed = Url::parse(url).with_context(|| format!("invalid subscription url: {url}"))?;
    let host = parsed
        .host_str()
        .ok_or_else(|| anyhow!("subscription url has no host: {url}"))?;
    let rule_specs = build_subscription_update_rule_specs(host);

    if rule_specs.is_empty() {
        anyhow::bail!("subscription url host cannot be routed through the dedicated Mihomo control plane");
    }

    let app_handle = Handle::app_handle();
    let rule_guard = MihomoRuleGuard::create(
        app_handle,
        &rule_specs,
        Some(SUBSCRIPTION_UPDATE_RULE_SOURCE),
        Some("prepend"),
    )
    .await?;

    let attempt_result = try_control_plane_candidates(url, option, app_handle).await;
    let restore_result = rule_guard.restore().await;

    match (attempt_result, restore_result) {
        (Ok(fetched), Ok(())) => Ok(fetched),
        (Ok(_), Err(err)) => Err(anyhow!(
            "subscription update fetched payload but failed to restore Mihomo runtime rule state: {err}"
        )),
        (Err(err), Ok(())) => Err(err),
        (Err(err), Err(restore_err)) => Err(anyhow!(
            "{err}; failed to restore Mihomo runtime rule state: {restore_err}"
        )),
    }
}

async fn try_control_plane_candidates(
    url: &str,
    option: &PrfOption,
    app_handle: &tauri::AppHandle,
) -> Result<FetchedSubscriptionPayload> {
    let ordered_candidates = resolve_control_plane_candidates(app_handle).await?;
    if ordered_candidates.is_empty() {
        anyhow::bail!("dedicated Mihomo control-plane group has no selectable candidates");
    }

    let mut last_err = None;

    for candidate in ordered_candidates {
        let selection_guard =
            match MihomoSelectionGuard::select(app_handle, SUBSCRIPTION_UPDATE_GROUP, &candidate).await {
                Ok(guard) => guard,
                Err(err) => {
                    last_err = Some(anyhow!("control-plane candidate {candidate} is unavailable: {err}"));
                    continue;
                }
            };

        let fetch_result = fetch_remote_profile(url, Some(option)).await;
        let restore_result = selection_guard.restore().await;

        match (fetch_result, restore_result) {
            (Ok(fetched), Ok(())) => return Ok(fetched),
            (Ok(_), Err(err)) => {
                return Err(anyhow!(
                    "subscription update fetched payload through control-plane candidate {candidate} but failed to restore Mihomo selection: {err}"
                ));
            }
            (Err(err), Ok(())) => {
                last_err = Some(anyhow!("control-plane candidate {candidate} failed: {err:#}"));
            }
            (Err(err), Err(restore_err)) => {
                last_err = Some(anyhow!(
                    "control-plane candidate {candidate} failed: {err:#}; failed to restore Mihomo selection: {restore_err}"
                ));
            }
        }
    }

    Err(last_err.unwrap_or_else(|| anyhow!("dedicated Mihomo control-plane produced no fetch attempts")))
}

async fn resolve_control_plane_candidates(app_handle: &tauri::AppHandle) -> Result<Vec<String>> {
    let (group, proxies) = read_subscription_control_plane_topology(app_handle, SUBSCRIPTION_UPDATE_GROUP)
        .await
        .with_context(|| format!("failed to load dedicated control-plane group {SUBSCRIPTION_UPDATE_GROUP}"))?;

    Ok(order_control_plane_candidates(&group, &proxies))
}

fn build_subscription_update_rule_specs(host: &str) -> Vec<MihomoRuntimeRuleSpec> {
    let host = host.trim();
    if host.is_empty() {
        return Vec::new();
    }

    if let Ok(ip) = host.parse::<IpAddr>() {
        let prefix = if ip.is_ipv4() { 32 } else { 128 };
        return vec![MihomoRuntimeRuleSpec::new(
            "IPCIDR",
            format!("{ip}/{prefix}"),
            SUBSCRIPTION_UPDATE_GROUP,
        )];
    }

    vec![MihomoRuntimeRuleSpec::new(
        "DOMAIN",
        host.to_ascii_lowercase(),
        SUBSCRIPTION_UPDATE_GROUP,
    )]
}

fn order_control_plane_candidates(group: &Proxy, proxies: &Proxies) -> Vec<String> {
    let mut seen = std::collections::HashSet::<std::string::String>::new();
    let mut direct = Vec::new();
    let mut alive = Vec::new();
    let mut degraded = Vec::new();

    for candidate in group.all.as_ref().into_iter().flatten() {
        let trimmed = candidate.trim();
        if trimmed.is_empty() {
            continue;
        }

        let dedupe_key = trimmed.to_ascii_lowercase();
        if !seen.insert(dedupe_key) {
            continue;
        }

        if trimmed.eq_ignore_ascii_case("DIRECT") {
            direct.push(trimmed.to_owned().into());
            continue;
        }

        match proxies.proxies.get(trimmed) {
            Some(proxy) if proxy.alive => alive.push(trimmed.to_owned().into()),
            Some(_) | None => degraded.push(trimmed.to_owned().into()),
        }
    }

    direct.extend(alive);
    direct.extend(degraded);
    direct
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use tauri_plugin_mihomo::models::{DelayHistory, Extra, ProxyType};

    fn runtime_proxy(name: &str, alive: bool) -> Proxy {
        Proxy {
            all: None,
            expected_status: None,
            fixed: None,
            hidden: None,
            icon: None,
            now: None,
            test_url: None,
            id: None,
            alive,
            history: Vec::<DelayHistory>::new(),
            extra: HashMap::<std::string::String, Extra>::new(),
            name: name.to_string(),
            udp: true,
            uot: false,
            proxy_type: ProxyType::Shadowsocks,
            xudp: false,
            tfo: false,
            mptcp: false,
            smux: false,
            interface: std::string::String::new(),
            dialer_proxy: std::string::String::new(),
            routing_mark: 0,
            provider_name: None,
        }
    }

    #[test]
    fn build_subscription_update_rule_specs_supports_domain_and_ip_hosts() {
        let domain_rules = build_subscription_update_rule_specs("Sub.Example.com");
        assert_eq!(domain_rules.len(), 1);
        assert_eq!(domain_rules[0].rule_type, "DOMAIN");
        assert_eq!(domain_rules[0].payload, "sub.example.com");

        let ip_rules = build_subscription_update_rule_specs("203.0.113.10");
        assert_eq!(ip_rules.len(), 1);
        assert_eq!(ip_rules[0].rule_type, "IPCIDR");
        assert_eq!(ip_rules[0].payload, "203.0.113.10/32");
    }

    #[test]
    fn order_control_plane_candidates_prefers_direct_then_alive_nodes() {
        let group = Proxy {
            all: Some(vec![
                "node-b".to_string(),
                "DIRECT".to_string(),
                "node-a".to_string(),
                "node-c".to_string(),
            ]),
            expected_status: None,
            fixed: None,
            hidden: None,
            icon: None,
            now: Some("node-c".to_string()),
            test_url: None,
            id: None,
            alive: true,
            history: Vec::new(),
            extra: HashMap::new(),
            name: SUBSCRIPTION_UPDATE_GROUP.to_string(),
            udp: true,
            uot: false,
            proxy_type: ProxyType::Selector,
            xudp: false,
            tfo: false,
            mptcp: false,
            smux: false,
            interface: std::string::String::new(),
            dialer_proxy: std::string::String::new(),
            routing_mark: 0,
            provider_name: None,
        };
        let proxies = Proxies {
            proxies: HashMap::from([
                ("node-a".to_string(), runtime_proxy("node-a", true)),
                ("node-b".to_string(), runtime_proxy("node-b", false)),
                ("node-c".to_string(), runtime_proxy("node-c", true)),
            ]),
        };

        let ordered = order_control_plane_candidates(&group, &proxies);
        assert_eq!(ordered, vec!["DIRECT", "node-a", "node-c", "node-b"]);
    }
}
