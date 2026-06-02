use crate::config::Config;
use crate::core::runtime_diagnostics::geoip::fetch_public_ip_location;
use anyhow::{anyhow, Result};
use reqwest::{Client, Proxy};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

const TOR_OBSERVATION_SOURCE: &str = "shared-geoip-probe";

#[derive(Debug, Clone, serde::Serialize)]
pub struct TorRuntimeStatus {
    pub enabled: bool,
    pub socks_host: String,
    pub socks_port: u16,
    pub control_port: Option<u16>,
    pub use_bridges: bool,
    pub bridge_count: usize,
    pub configured_proxy_url: String,
    pub checked: bool,
    pub status: String,
    pub connected: bool,
    pub circuit_established: bool,
    pub observation_incomplete: bool,
    pub runtime_risk_detected: bool,
    pub confidence: String,
    pub assessment: String,
    pub runtime_risk_type: Vec<String>,
    pub current_ip: Option<String>,
    pub exit_node: Option<String>,
    pub check_method: String,
    pub observation_path: String,
    pub observation_source: Option<String>,
    pub warnings: Vec<String>,
    pub error: Option<String>,
    pub timestamp: u64,
}

#[derive(Debug, Clone)]
struct TorConfig {
    enabled: bool,
    socks_host: String,
    socks_port: u16,
    control_port: Option<u16>,
    use_bridges: bool,
    bridges: Vec<String>,
}

fn current_timestamp_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis() as u64)
        .unwrap_or(0)
}

fn is_local_socks_host(host: &str) -> bool {
    matches!(
        host.trim().to_ascii_lowercase().as_str(),
        "127.0.0.1" | "localhost" | "::1"
    )
}

fn collect_runtime_risk_type(config: &TorConfig) -> Vec<String> {
    let mut runtime_risk_type = Vec::new();

    if !is_local_socks_host(&config.socks_host) {
        runtime_risk_type.push("non-local-socks-endpoint".to_string());
    }

    if config.socks_port == 0 {
        runtime_risk_type.push("invalid-socks-port".to_string());
    }

    if config.use_bridges && config.bridges.is_empty() {
        runtime_risk_type.push("bridges-enabled-without-bridges".to_string());
    }

    runtime_risk_type
}

fn collect_warnings(runtime_risk_type: &[String]) -> Vec<String> {
    let mut warnings = Vec::new();

    for risk in runtime_risk_type {
        match risk.as_str() {
            "non-local-socks-endpoint" => warnings.push(
                "Tor SOCKS5 端点不是本机地址，当前将信任外部 SOCKS 代理。".to_string(),
            ),
            "invalid-socks-port" => {
                warnings.push("Tor SOCKS5 端口为 0，属于无效配置。".to_string())
            }
            "bridges-enabled-without-bridges" => {
                warnings.push("已启用网桥模式，但当前未配置任何网桥".to_string())
            }
            _ => {}
        }
    }

    warnings
}

fn derive_assessment(
    enabled: bool,
    connected: bool,
    runtime_risk_detected: bool,
    observation_incomplete: bool,
) -> &'static str {
    if !enabled {
        return "disabled";
    }

    if connected {
        if runtime_risk_detected {
            return "runtime-risk";
        }

        return "connected";
    }

    if runtime_risk_detected {
        return "runtime-risk";
    }

    if observation_incomplete {
        return "inconclusive";
    }

    "inconclusive"
}

fn derive_confidence(
    enabled: bool,
    connected: bool,
    runtime_risk_detected: bool,
    observation_incomplete: bool,
) -> &'static str {
    if !enabled {
        return "low";
    }

    if connected {
        if runtime_risk_detected {
            return "medium";
        }

        return "high";
    }

    if observation_incomplete {
        return "low";
    }

    if runtime_risk_detected {
        return "medium";
    }

    "low"
}

async fn read_tor_config() -> TorConfig {
    let verge = Config::verge().await.data_arc();

    TorConfig {
        enabled: verge.enable_tor_proxy.unwrap_or(false),
        socks_host: verge
            .tor_socks_host
            .clone()
            .map(|value| value.to_string())
            .filter(|value| !value.trim().is_empty())
            .unwrap_or_else(|| "127.0.0.1".to_string()),
        socks_port: verge.tor_socks_port.unwrap_or(9050),
        control_port: verge.tor_control_port,
        use_bridges: verge.tor_use_bridges.unwrap_or(false),
        bridges: verge
            .tor_bridges
            .clone()
            .unwrap_or_default()
            .into_iter()
            .map(|value| value.to_string())
            .collect(),
    }
}

fn build_display_proxy_url(config: &TorConfig) -> String {
    format!("socks5://{}:{}", config.socks_host, config.socks_port)
}

fn build_probe_proxy_url(config: &TorConfig) -> String {
    format!("socks5h://{}:{}", config.socks_host, config.socks_port)
}

fn build_tor_client(config: &TorConfig) -> Result<Client> {
    let proxy = Proxy::all(build_probe_proxy_url(config))?;

    Ok(Client::builder()
        .tls_backend_rustls()
        .proxy(proxy)
        .redirect(reqwest::redirect::Policy::limited(10))
        .tcp_keepalive(Duration::from_secs(60))
        .timeout(Duration::from_secs(8))
        .connect_timeout(Duration::from_secs(8))
        .user_agent(format!("clash-verge/v{}", env!("CARGO_PKG_VERSION")))
        .build()?)
}

fn build_tor_exit_node(city: Option<&str>, country: Option<&str>) -> Option<String> {
    let city = city.map(str::trim).filter(|item| !item.is_empty());
    let country = country.map(str::trim).filter(|item| !item.is_empty());

    match (city, country) {
        (Some(city), Some(country)) => Some(format!("{city}, {country}")),
        (Some(city), None) => Some(city.to_string()),
        (None, Some(country)) => Some(country.to_string()),
        (None, None) => None,
    }
}

async fn query_tor_exit(client: &Client) -> Result<(String, Option<String>, String)> {
    let info = fetch_public_ip_location(client).await?;
    let ip = info.ip.ok_or_else(|| anyhow!("Tor exit lookup returned no IP"))?;
    let exit_node = build_tor_exit_node(info.city.as_deref(), info.country.as_deref());

    Ok((ip.to_string(), exit_node, TOR_OBSERVATION_SOURCE.to_string()))
}

fn compose_tor_runtime_status(
    config: &TorConfig,
    configured_proxy_url: &str,
    observation_path: &str,
    runtime_risk_type: Vec<String>,
    warnings: Vec<String>,
    checked: bool,
    connected: bool,
    observation_incomplete: bool,
    current_ip: Option<String>,
    exit_node: Option<String>,
    observation_source: Option<String>,
    error: Option<String>,
) -> TorRuntimeStatus {
    let runtime_risk_detected = !runtime_risk_type.is_empty();
    let assessment = derive_assessment(
        config.enabled,
        connected,
        runtime_risk_detected,
        observation_incomplete,
    );
    let confidence = derive_confidence(
        config.enabled,
        connected,
        runtime_risk_detected,
        observation_incomplete,
    );
    let status = if !config.enabled {
        "disabled"
    } else if connected {
        "connected"
    } else {
        "failed"
    };

    TorRuntimeStatus {
        enabled: config.enabled,
        socks_host: config.socks_host.clone(),
        socks_port: config.socks_port,
        control_port: config.control_port,
        use_bridges: config.use_bridges,
        bridge_count: config.bridges.len(),
        configured_proxy_url: configured_proxy_url.to_string(),
        checked,
        status: status.to_string(),
        connected,
        circuit_established: connected,
        observation_incomplete,
        runtime_risk_detected,
        confidence: confidence.to_string(),
        assessment: assessment.to_string(),
        runtime_risk_type,
        current_ip,
        exit_node,
        check_method: "socks5h".to_string(),
        observation_path: observation_path.to_string(),
        observation_source,
        warnings,
        error,
        timestamp: current_timestamp_ms(),
    }
}

async fn probe_tor_runtime_status(config: TorConfig) -> TorRuntimeStatus {
    let configured_proxy_url = build_display_proxy_url(&config);
    let observation_path = "socks5h-exit-probe";

    if !config.enabled {
        return compose_tor_runtime_status(
            &config,
            &configured_proxy_url,
            observation_path,
            Vec::new(),
            Vec::new(),
            false,
            false,
            false,
            None,
            None,
            None,
            None,
        );
    }

    let runtime_risk_type = collect_runtime_risk_type(&config);
    let warnings = collect_warnings(&runtime_risk_type);

    match build_tor_client(&config) {
        Ok(client) => match query_tor_exit(&client).await {
            Ok((current_ip, exit_node, observation_source)) => compose_tor_runtime_status(
                &config,
                &configured_proxy_url,
                observation_path,
                runtime_risk_type,
                warnings,
                true,
                true,
                false,
                Some(current_ip),
                exit_node,
                Some(observation_source),
                None,
            ),
            Err(err) => compose_tor_runtime_status(
                &config,
                &configured_proxy_url,
                observation_path,
                runtime_risk_type,
                warnings,
                true,
                false,
                true,
                None,
                None,
                None,
                Some(format!("Tor 连通性检测失败: {err}")),
            ),
        },
        Err(err) => compose_tor_runtime_status(
            &config,
            &configured_proxy_url,
            observation_path,
            runtime_risk_type,
            warnings,
            false,
            false,
            true,
            None,
            None,
            None,
            Some(format!("Tor SOCKS5 客户端初始化失败: {err}")),
        ),
    }
}

pub async fn build_tor_runtime_status() -> Result<TorRuntimeStatus> {
    let config = read_tor_config().await;
    Ok(probe_tor_runtime_status(config).await)
}
