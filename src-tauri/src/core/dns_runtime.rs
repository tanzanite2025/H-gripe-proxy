use crate::config::Config;
use crate::utils::dirs;
use anyhow::{Context as _, Result, anyhow};
use async_trait::async_trait;
use hickory_proto::rr::Name;
use hickory_resolver::TokioAsyncResolver;
use hickory_resolver::config::*;
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use serde_yaml_ng::{Mapping, Value};
use std::net::{IpAddr, SocketAddr};
use std::str::FromStr;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tokio::fs;

mod default_runtime;
#[cfg(test)]
use default_runtime::default_runtime_execution_superseded_state;
pub use default_runtime::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DnsProtocol {
    Udp,
    Tcp,
    Doh,
    Dot,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum DnsServerProviderKind {
    Cloudflare,
    Google,
    Quad9,
    AliDns,
    DohPub,
    DotPub,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum DnsServerProviderAvailability {
    Ready,
    Experimental,
    Placeholder,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DnsServerProviderEndpointRegistration {
    pub protocol: DnsProtocol,
    pub server: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DnsServerProviderRegistration {
    pub kind: DnsServerProviderKind,
    pub label: String,
    pub availability: DnsServerProviderAvailability,
    pub description: String,
    pub canonical_host: String,
    pub host_aliases: Vec<String>,
    pub bootstrap_ips: Vec<String>,
    pub supported_protocols: Vec<DnsProtocol>,
    pub recommended_servers: Vec<DnsServerProviderEndpointRegistration>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DnsServerProviderHealthReport {
    pub provider_kind: DnsServerProviderKind,
    pub provider_label: String,
    pub server: String,
    pub protocol: String,
    pub test_domain: String,
    pub healthy: bool,
    pub message: String,
    pub latency_ms: Option<u64>,
    pub checked_at: SystemTime,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct DnsServerProviderDefinition {
    kind: DnsServerProviderKind,
    label: &'static str,
    availability: DnsServerProviderAvailability,
    description: &'static str,
    canonical_host: &'static str,
    host_aliases: &'static [&'static str],
    bootstrap_ips: &'static [&'static str],
    supported_protocols: &'static [DnsProtocol],
}

impl DnsServerProviderDefinition {
    fn matches_host(&self, host: &str) -> bool {
        self.host_aliases
            .iter()
            .any(|candidate| candidate.eq_ignore_ascii_case(host))
    }

    fn matches_ip(&self, ip: &IpAddr) -> bool {
        let ip_text = ip.to_string();
        self.bootstrap_ips.iter().any(|candidate| *candidate == ip_text)
    }

    fn supports_protocol(&self, protocol: DnsProtocol) -> bool {
        self.supported_protocols.contains(&protocol)
    }

    fn preferred_ip(&self) -> Option<IpAddr> {
        self.bootstrap_ips.first().and_then(|ip| ip.parse().ok())
    }

    fn default_protocol(&self) -> DnsProtocol {
        self.supported_protocols.first().copied().unwrap_or(DnsProtocol::Udp)
    }

    fn server_for_protocol(&self, protocol: DnsProtocol) -> Option<String> {
        if !self.supports_protocol(protocol) {
            return None;
        }

        match protocol {
            DnsProtocol::Udp => self.preferred_ip().map(|ip| ip.to_string()),
            DnsProtocol::Tcp => self.preferred_ip().map(|ip| format!("tcp://{ip}:53")),
            DnsProtocol::Doh => Some(format!("https://{}/dns-query", self.canonical_host)),
            DnsProtocol::Dot => Some(format!("tls://{}:853", self.canonical_host)),
        }
    }

    fn to_registration(self) -> DnsServerProviderRegistration {
        DnsServerProviderRegistration {
            kind: self.kind,
            label: self.label.to_string(),
            availability: self.availability,
            description: self.description.to_string(),
            canonical_host: self.canonical_host.to_string(),
            host_aliases: self.host_aliases.iter().map(|item| (*item).to_string()).collect(),
            bootstrap_ips: self.bootstrap_ips.iter().map(|item| (*item).to_string()).collect(),
            supported_protocols: self.supported_protocols.to_vec(),
            recommended_servers: self
                .supported_protocols
                .iter()
                .copied()
                .filter_map(|protocol| {
                    self.server_for_protocol(protocol)
                        .map(|server| DnsServerProviderEndpointRegistration { protocol, server })
                })
                .collect(),
        }
    }
}

const ALL_DNS_PROTOCOLS: &[DnsProtocol] = &[DnsProtocol::Udp, DnsProtocol::Tcp, DnsProtocol::Doh, DnsProtocol::Dot];
const DOH_PUB_PROTOCOLS: &[DnsProtocol] = &[DnsProtocol::Udp, DnsProtocol::Tcp, DnsProtocol::Doh];
const DOT_PUB_PROTOCOLS: &[DnsProtocol] = &[DnsProtocol::Udp, DnsProtocol::Tcp, DnsProtocol::Dot];

const CLOUDFLARE_DNS_PROVIDER: DnsServerProviderDefinition = DnsServerProviderDefinition {
    kind: DnsServerProviderKind::Cloudflare,
    label: "Cloudflare DNS",
    availability: DnsServerProviderAvailability::Ready,
    description: "Built-in public DNS provider with UDP, TCP, DoH, and DoT endpoints.",
    canonical_host: "cloudflare-dns.com",
    host_aliases: &[
        "cloudflare-dns.com",
        "one.one.one.one",
        "1dot1dot1dot1.cloudflare-dns.com",
    ],
    bootstrap_ips: &["1.1.1.1", "1.0.0.1"],
    supported_protocols: ALL_DNS_PROTOCOLS,
};

const GOOGLE_DNS_PROVIDER: DnsServerProviderDefinition = DnsServerProviderDefinition {
    kind: DnsServerProviderKind::Google,
    label: "Google Public DNS",
    availability: DnsServerProviderAvailability::Ready,
    description: "Built-in Google public DNS provider with UDP, TCP, DoH, and DoT endpoints.",
    canonical_host: "dns.google",
    host_aliases: &["dns.google"],
    bootstrap_ips: &["8.8.8.8", "8.8.4.4"],
    supported_protocols: ALL_DNS_PROTOCOLS,
};

const QUAD9_DNS_PROVIDER: DnsServerProviderDefinition = DnsServerProviderDefinition {
    kind: DnsServerProviderKind::Quad9,
    label: "Quad9 DNS",
    availability: DnsServerProviderAvailability::Ready,
    description: "Built-in Quad9 DNS provider with UDP, TCP, DoH, and DoT endpoints.",
    canonical_host: "dns.quad9.net",
    host_aliases: &["dns.quad9.net"],
    bootstrap_ips: &["9.9.9.9"],
    supported_protocols: ALL_DNS_PROTOCOLS,
};

const ALIDNS_PROVIDER: DnsServerProviderDefinition = DnsServerProviderDefinition {
    kind: DnsServerProviderKind::AliDns,
    label: "AliDNS",
    availability: DnsServerProviderAvailability::Ready,
    description: "Built-in AliDNS provider with UDP, TCP, DoH, and DoT endpoints.",
    canonical_host: "dns.alidns.com",
    host_aliases: &["dns.alidns.com"],
    bootstrap_ips: &["223.5.5.5", "223.6.6.6"],
    supported_protocols: ALL_DNS_PROTOCOLS,
};

const DOH_PUB_PROVIDER: DnsServerProviderDefinition = DnsServerProviderDefinition {
    kind: DnsServerProviderKind::DohPub,
    label: "DoH.pub",
    availability: DnsServerProviderAvailability::Ready,
    description: "Built-in Tencent DoH provider with UDP, TCP, and DoH endpoints.",
    canonical_host: "doh.pub",
    host_aliases: &["doh.pub"],
    bootstrap_ips: &["119.29.29.29", "120.53.53.53"],
    supported_protocols: DOH_PUB_PROTOCOLS,
};

const DOT_PUB_PROVIDER: DnsServerProviderDefinition = DnsServerProviderDefinition {
    kind: DnsServerProviderKind::DotPub,
    label: "DoT.pub",
    availability: DnsServerProviderAvailability::Ready,
    description: "Built-in Tencent DoT provider with UDP, TCP, and DoT endpoints.",
    canonical_host: "dot.pub",
    host_aliases: &["dot.pub"],
    bootstrap_ips: &["1.12.12.12"],
    supported_protocols: DOT_PUB_PROTOCOLS,
};

// A single provider registry drives hostname bootstrap, TLS server-name canonicalization,
// and the public provider catalog exposed to later UI/config consumers.
const DNS_SERVER_PROVIDERS: [&DnsServerProviderDefinition; 6] = [
    &CLOUDFLARE_DNS_PROVIDER,
    &GOOGLE_DNS_PROVIDER,
    &QUAD9_DNS_PROVIDER,
    &ALIDNS_PROVIDER,
    &DOH_PUB_PROVIDER,
    &DOT_PUB_PROVIDER,
];

const DEFAULT_DNS_HEALTH_CHECK_DOMAIN: &str = "www.google.com";
const DEFAULT_DNS_RUNTIME_TIMEOUT_MS: u64 = 5_000;
const DEFAULT_DNS_RUNTIME_ATTEMPTS: u8 = 2;

fn provider_definitions() -> &'static [&'static DnsServerProviderDefinition] {
    &DNS_SERVER_PROVIDERS
}

pub fn list_dns_server_provider_registrations() -> Vec<DnsServerProviderRegistration> {
    provider_definitions()
        .iter()
        .copied()
        .map(|provider| provider.to_registration())
        .collect()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DnsQueryResult {
    pub domain: String,
    pub ip: String,
    pub latency: u64,
    pub success: bool,
    pub error: Option<String>,
    pub protocol: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DnsHealthCheckResult {
    pub server: String,
    pub latency: u64,
    pub success: bool,
    pub error: Option<String>,
    pub protocol: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum DnsResolverPlanStatus {
    Ready,
    Disabled,
    Rejected,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DnsResolverRuntimeFeaturePlan {
    pub configured: bool,
    pub runtime_applied: bool,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DnsResolverRuntimeProjection {
    pub fake_ip: DnsResolverRuntimeFeaturePlan,
    pub fallback_filter: DnsResolverRuntimeFeaturePlan,
    pub nameserver_policy: DnsResolverRuntimeFeaturePlan,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DnsResolverNameserverPlan {
    pub server: String,
    pub protocol: DnsProtocol,
    pub protocol_name: String,
    pub target: Option<DnsServerProbeTarget>,
    pub runtime_supported: bool,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DnsResolverPlan {
    pub status: DnsResolverPlanStatus,
    pub reason: String,
    pub enabled: Option<bool>,
    pub timeout_ms: u64,
    pub attempts: u8,
    pub nameservers: Vec<DnsResolverNameserverPlan>,
    pub runtime_projection: DnsResolverRuntimeProjection,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DnsRuntimeQueryOptions {
    pub timeout_ms: u64,
    pub attempts: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DnsResolverRuntimeMetrics {
    pub total_queries: u64,
    pub successful_queries: u64,
    pub failed_queries: u64,
    pub total_latency_ms: u64,
    pub last_error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DnsResolverRuntimeQueryReport {
    pub plan: DnsResolverPlan,
    pub domain: String,
    pub result: Option<DnsQueryResult>,
    pub attempted_servers: Vec<String>,
    pub metrics: DnsResolverRuntimeMetrics,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DnsResolverRuntimeProbeSummary {
    pub total_targets: usize,
    pub runtime_supported_targets: usize,
    pub healthy_targets: usize,
    pub failed_targets: usize,
    pub unsupported_targets: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DnsResolverRuntimeProbeTargetReport {
    pub server: String,
    pub protocol: String,
    pub provider_kind: Option<DnsServerProviderKind>,
    pub provider_label: Option<String>,
    pub runtime_supported: bool,
    pub healthy: bool,
    pub latency_ms: Option<u64>,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DnsResolverRuntimeProbeReport {
    pub plan: DnsResolverPlan,
    pub test_domain: String,
    pub targets: Vec<DnsResolverRuntimeProbeTargetReport>,
    pub summary: DnsResolverRuntimeProbeSummary,
    pub metrics: DnsResolverRuntimeMetrics,
    pub warnings: Vec<String>,
}

#[async_trait]
pub trait RustDnsResolverRuntime: Send + Sync {
    async fn query(
        &self,
        nameserver: &DnsResolverNameserverPlan,
        domain: &str,
        options: DnsRuntimeQueryOptions,
    ) -> Result<DnsQueryResult>;
}

#[derive(Debug, Default)]
pub struct HickoryDnsResolverRuntime;

#[async_trait]
impl RustDnsResolverRuntime for HickoryDnsResolverRuntime {
    async fn query(
        &self,
        nameserver: &DnsResolverNameserverPlan,
        domain: &str,
        options: DnsRuntimeQueryOptions,
    ) -> Result<DnsQueryResult> {
        dns_query_with_options(
            domain.to_string(),
            Some(nameserver.server.clone()),
            Some(nameserver.protocol),
            options,
        )
        .await
    }
}

pub struct DnsResolverRuntimeController<R: RustDnsResolverRuntime> {
    runtime: R,
    metrics: Arc<Mutex<DnsResolverRuntimeMetrics>>,
}

impl<R: RustDnsResolverRuntime> DnsResolverRuntimeController<R> {
    pub fn new(runtime: R) -> Self {
        Self {
            runtime,
            metrics: Arc::new(Mutex::new(DnsResolverRuntimeMetrics::default())),
        }
    }

    pub async fn query(&self, plan: DnsResolverPlan, domain: String) -> DnsResolverRuntimeQueryReport {
        let options = DnsRuntimeQueryOptions {
            timeout_ms: plan.timeout_ms,
            attempts: plan.attempts,
        };
        let mut attempted_servers = Vec::new();
        let mut result = None;

        if plan.status == DnsResolverPlanStatus::Ready {
            for nameserver in plan.nameservers.iter().filter(|item| item.runtime_supported) {
                attempted_servers.push(nameserver.server.clone());
                match self.runtime.query(nameserver, &domain, options).await {
                    Ok(query_result) if query_result.success => {
                        self.record_query(&query_result);
                        result = Some(query_result);
                        break;
                    }
                    Ok(query_result) => {
                        self.record_query(&query_result);
                        result = Some(query_result);
                    }
                    Err(error) => {
                        self.record_error(error.to_string());
                    }
                }
            }
        }

        if result.is_none() && plan.status != DnsResolverPlanStatus::Ready {
            self.record_error(plan.reason.clone());
        }

        DnsResolverRuntimeQueryReport {
            plan,
            domain,
            result,
            attempted_servers,
            metrics: self.metrics(),
        }
    }

    pub async fn probe(&self, plan: DnsResolverPlan, test_domain: String) -> DnsResolverRuntimeProbeReport {
        let options = DnsRuntimeQueryOptions {
            timeout_ms: plan.timeout_ms,
            attempts: plan.attempts,
        };
        let mut targets = Vec::new();

        for nameserver in &plan.nameservers {
            if !nameserver.runtime_supported {
                targets.push(unsupported_probe_target_report(nameserver));
                continue;
            }

            match self.runtime.query(nameserver, &test_domain, options).await {
                Ok(result) => {
                    self.record_query(&result);
                    targets.push(success_probe_target_report(nameserver, &result));
                }
                Err(error) => {
                    let message = error.to_string();
                    self.record_error(message.clone());
                    targets.push(error_probe_target_report(nameserver, message));
                }
            }
        }

        if plan.status != DnsResolverPlanStatus::Ready {
            self.record_error(plan.reason.clone());
        }

        let summary = probe_summary(&targets);
        let warnings = probe_warnings(&plan);

        DnsResolverRuntimeProbeReport {
            plan,
            test_domain,
            targets,
            summary,
            metrics: self.metrics(),
            warnings,
        }
    }

    pub fn metrics(&self) -> DnsResolverRuntimeMetrics {
        self.metrics.lock().expect("dns metrics lock poisoned").clone()
    }

    fn record_query(&self, result: &DnsQueryResult) {
        let mut metrics = self.metrics.lock().expect("dns metrics lock poisoned");
        metrics.total_queries += 1;
        metrics.total_latency_ms = metrics.total_latency_ms.saturating_add(result.latency);
        if result.success {
            metrics.successful_queries += 1;
            metrics.last_error = None;
        } else {
            metrics.failed_queries += 1;
            metrics.last_error = result.error.clone();
        }
    }

    fn record_error(&self, error: String) {
        let mut metrics = self.metrics.lock().expect("dns metrics lock poisoned");
        metrics.total_queries += 1;
        metrics.failed_queries += 1;
        metrics.last_error = Some(error);
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DnsServerProbeTarget {
    pub server: String,
    pub protocol: DnsProtocol,
    pub protocol_name: String,
    pub socket_addr: String,
    pub tls_dns_name: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct DnsServerEndpoint {
    socket_addr: SocketAddr,
    tls_dns_name: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct DnsServerParts {
    scheme: Option<String>,
    host: String,
    port: Option<u16>,
}

pub fn build_dns_resolver_plan(yaml: &str) -> Result<DnsResolverPlan> {
    let value: Value = serde_yaml_ng::from_str(yaml).context("YAML syntax error")?;
    let root = value
        .as_mapping()
        .ok_or_else(|| anyhow!("config root must be a YAML mapping"))?;
    let Some(dns) = dns_mapping(root) else {
        return Ok(rejected_resolver_plan("dns config is missing"));
    };

    let mut warnings = Vec::new();
    let enabled = optional_bool(dns, "enable", &mut warnings);
    if enabled == Some(false) {
        return Ok(DnsResolverPlan {
            status: DnsResolverPlanStatus::Disabled,
            reason: "dns.enable is false; Rust resolver runtime stays inactive".into(),
            enabled,
            timeout_ms: DEFAULT_DNS_RUNTIME_TIMEOUT_MS,
            attempts: DEFAULT_DNS_RUNTIME_ATTEMPTS,
            nameservers: Vec::new(),
            runtime_projection: build_runtime_projection(dns),
            warnings,
        });
    }

    let timeout_ms = optional_u64(dns, "timeout", &mut warnings).unwrap_or(DEFAULT_DNS_RUNTIME_TIMEOUT_MS);
    let attempts = optional_u64(dns, "attempts", &mut warnings)
        .and_then(|value| u8::try_from(value).ok())
        .filter(|value| *value > 0)
        .unwrap_or(DEFAULT_DNS_RUNTIME_ATTEMPTS);
    let nameservers = extract_server_values(dns.get("nameserver"), "dns.nameserver", &mut warnings)
        .into_iter()
        .map(build_nameserver_plan)
        .collect::<Vec<_>>();
    let supported_count = nameservers.iter().filter(|item| item.runtime_supported).count();

    let (status, reason) = if nameservers.is_empty() {
        (
            DnsResolverPlanStatus::Rejected,
            "dns.nameserver is empty; no Rust resolver can be built".into(),
        )
    } else if supported_count == 0 {
        (
            DnsResolverPlanStatus::Rejected,
            "dns.nameserver has no runtime-supported targets".into(),
        )
    } else {
        (
            DnsResolverPlanStatus::Ready,
            format!("Rust resolver runtime can query {supported_count} nameserver target(s)"),
        )
    };

    Ok(DnsResolverPlan {
        status,
        reason,
        enabled,
        timeout_ms,
        attempts,
        nameservers,
        runtime_projection: build_runtime_projection(dns),
        warnings,
    })
}

pub async fn dns_runtime_query(yaml: &str, domain: String) -> Result<DnsResolverRuntimeQueryReport> {
    let plan = build_dns_resolver_plan(yaml)?;
    let controller = DnsResolverRuntimeController::new(HickoryDnsResolverRuntime);
    Ok(controller.query(plan, domain).await)
}

pub async fn dns_controlled_runtime_probe(
    yaml: &str,
    test_domain: Option<String>,
) -> Result<DnsResolverRuntimeProbeReport> {
    let plan = build_dns_resolver_plan(yaml)?;
    let test_domain = test_domain
        .as_deref()
        .map(str::trim)
        .filter(|domain| !domain.is_empty())
        .unwrap_or(DEFAULT_DNS_HEALTH_CHECK_DOMAIN)
        .to_string();
    let controller = DnsResolverRuntimeController::new(HickoryDnsResolverRuntime);
    Ok(controller.probe(plan, test_domain).await)
}

fn rejected_resolver_plan(reason: &str) -> DnsResolverPlan {
    DnsResolverPlan {
        status: DnsResolverPlanStatus::Rejected,
        reason: reason.into(),
        enabled: None,
        timeout_ms: DEFAULT_DNS_RUNTIME_TIMEOUT_MS,
        attempts: DEFAULT_DNS_RUNTIME_ATTEMPTS,
        nameservers: Vec::new(),
        runtime_projection: default_runtime_projection(),
        warnings: Vec::new(),
    }
}

fn build_nameserver_plan(server: String) -> DnsResolverNameserverPlan {
    let protocol = infer_dns_protocol(Some(&server), None);
    match plan_dns_server_probe_target(&server) {
        Ok(target) => DnsResolverNameserverPlan {
            server,
            protocol,
            protocol_name: dns_protocol_name(protocol),
            target: Some(target),
            runtime_supported: true,
            reason: "supported by Rust resolver runtime".into(),
        },
        Err(error) => DnsResolverNameserverPlan {
            server,
            protocol,
            protocol_name: dns_protocol_name(protocol),
            target: None,
            runtime_supported: false,
            reason: error.to_string(),
        },
    }
}

fn build_runtime_projection(dns: &Mapping) -> DnsResolverRuntimeProjection {
    let enhanced_mode = dns
        .get("enhanced-mode")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_ascii_lowercase();
    let fake_ip_configured = enhanced_mode == "fake-ip" || dns.contains_key("fake-ip-range");
    let fallback_filter_configured = dns.contains_key("fallback-filter");
    let nameserver_policy_count = dns
        .get("nameserver-policy")
        .and_then(Value::as_mapping)
        .map(Mapping::len)
        .unwrap_or(0);

    DnsResolverRuntimeProjection {
        fake_ip: DnsResolverRuntimeFeaturePlan {
            configured: fake_ip_configured,
            runtime_applied: false,
            reason: "fake-ip remains plan/explain-only in this slice".into(),
        },
        fallback_filter: DnsResolverRuntimeFeaturePlan {
            configured: fallback_filter_configured,
            runtime_applied: false,
            reason: "fallback-filter remains plan/explain-only in this slice".into(),
        },
        nameserver_policy: DnsResolverRuntimeFeaturePlan {
            configured: nameserver_policy_count > 0,
            runtime_applied: false,
            reason: format!(
                "{nameserver_policy_count} nameserver-policy entries remain plan/explain-only in this slice"
            ),
        },
    }
}

fn success_probe_target_report(
    nameserver: &DnsResolverNameserverPlan,
    result: &DnsQueryResult,
) -> DnsResolverRuntimeProbeTargetReport {
    let provider = nameserver.target.as_ref().and_then(provider_for_probe_target);
    DnsResolverRuntimeProbeTargetReport {
        server: nameserver.server.clone(),
        protocol: nameserver.protocol_name.clone(),
        provider_kind: provider.map(|provider| provider.kind),
        provider_label: provider.map(|provider| provider.label.to_string()),
        runtime_supported: true,
        healthy: result.success,
        latency_ms: Some(result.latency),
        message: result
            .error
            .clone()
            .unwrap_or_else(|| format!("resolved {} to {}", result.domain, result.ip)),
    }
}

fn error_probe_target_report(
    nameserver: &DnsResolverNameserverPlan,
    message: String,
) -> DnsResolverRuntimeProbeTargetReport {
    let provider = nameserver.target.as_ref().and_then(provider_for_probe_target);
    DnsResolverRuntimeProbeTargetReport {
        server: nameserver.server.clone(),
        protocol: nameserver.protocol_name.clone(),
        provider_kind: provider.map(|provider| provider.kind),
        provider_label: provider.map(|provider| provider.label.to_string()),
        runtime_supported: true,
        healthy: false,
        latency_ms: None,
        message,
    }
}

fn unsupported_probe_target_report(nameserver: &DnsResolverNameserverPlan) -> DnsResolverRuntimeProbeTargetReport {
    DnsResolverRuntimeProbeTargetReport {
        server: nameserver.server.clone(),
        protocol: nameserver.protocol_name.clone(),
        provider_kind: None,
        provider_label: None,
        runtime_supported: false,
        healthy: false,
        latency_ms: None,
        message: nameserver.reason.clone(),
    }
}

fn provider_for_probe_target(target: &DnsServerProbeTarget) -> Option<&'static DnsServerProviderDefinition> {
    target
        .tls_dns_name
        .as_deref()
        .and_then(find_dns_provider_by_host)
        .or_else(|| {
            target
                .socket_addr
                .parse::<SocketAddr>()
                .ok()
                .and_then(|socket_addr| find_dns_provider_by_ip(&socket_addr.ip()))
        })
}

fn probe_summary(targets: &[DnsResolverRuntimeProbeTargetReport]) -> DnsResolverRuntimeProbeSummary {
    let runtime_supported_targets = targets.iter().filter(|target| target.runtime_supported).count();
    let healthy_targets = targets.iter().filter(|target| target.healthy).count();
    let unsupported_targets = targets.iter().filter(|target| !target.runtime_supported).count();

    DnsResolverRuntimeProbeSummary {
        total_targets: targets.len(),
        runtime_supported_targets,
        healthy_targets,
        failed_targets: runtime_supported_targets.saturating_sub(healthy_targets),
        unsupported_targets,
    }
}

fn probe_warnings(plan: &DnsResolverPlan) -> Vec<String> {
    let mut warnings = plan.warnings.clone();
    append_projection_warning(&mut warnings, "fake-ip", &plan.runtime_projection.fake_ip);
    append_projection_warning(
        &mut warnings,
        "fallback-filter",
        &plan.runtime_projection.fallback_filter,
    );
    append_projection_warning(
        &mut warnings,
        "nameserver-policy",
        &plan.runtime_projection.nameserver_policy,
    );
    warnings
}

fn append_projection_warning(warnings: &mut Vec<String>, feature: &str, feature_plan: &DnsResolverRuntimeFeaturePlan) {
    if feature_plan.configured && !feature_plan.runtime_applied {
        warnings.push(format!("{feature} is plan-only for controlled Rust DNS runtime probe"));
    }
}

fn default_runtime_projection() -> DnsResolverRuntimeProjection {
    DnsResolverRuntimeProjection {
        fake_ip: DnsResolverRuntimeFeaturePlan {
            configured: false,
            runtime_applied: false,
            reason: "fake-ip is not configured".into(),
        },
        fallback_filter: DnsResolverRuntimeFeaturePlan {
            configured: false,
            runtime_applied: false,
            reason: "fallback-filter is not configured".into(),
        },
        nameserver_policy: DnsResolverRuntimeFeaturePlan {
            configured: false,
            runtime_applied: false,
            reason: "nameserver-policy is not configured".into(),
        },
    }
}

fn dns_mapping(root: &Mapping) -> Option<&Mapping> {
    if let Some(dns) = root.get("dns") {
        return dns.as_mapping();
    }

    if root.keys().any(|key| {
        key.as_str()
            .map(|key| {
                matches!(
                    key,
                    "nameserver"
                        | "fallback"
                        | "default-nameserver"
                        | "proxy-server-nameserver"
                        | "direct-nameserver"
                        | "enhanced-mode"
                        | "fake-ip-range"
                        | "fallback-filter"
                        | "nameserver-policy"
                )
            })
            .unwrap_or(false)
    }) {
        return Some(root);
    }

    None
}

fn optional_bool(dns: &Mapping, key: &str, warnings: &mut Vec<String>) -> Option<bool> {
    dns.get(key).and_then(|value| match value.as_bool() {
        Some(value) => Some(value),
        None => {
            warnings.push(format!("dns.{key}: expected boolean, got {}", value_type(value)));
            None
        }
    })
}

fn optional_u64(dns: &Mapping, key: &str, warnings: &mut Vec<String>) -> Option<u64> {
    dns.get(key).and_then(|value| match value.as_u64() {
        Some(value) => Some(value),
        None => {
            warnings.push(format!(
                "dns.{key}: expected unsigned integer, got {}",
                value_type(value)
            ));
            None
        }
    })
}

fn extract_server_values(value: Option<&Value>, path: &str, warnings: &mut Vec<String>) -> Vec<String> {
    match value {
        None | Some(Value::Null) => Vec::new(),
        Some(Value::String(server)) => {
            warnings.push(format!("{path}: expected array, treating string as one server"));
            vec![server.trim().into()]
        }
        Some(Value::Sequence(items)) => items
            .iter()
            .enumerate()
            .filter_map(|(index, item)| match item.as_str() {
                Some(server) => Some(server.trim().into()),
                None => {
                    warnings.push(format!("{path}[{index}]: expected string, got {}", value_type(item)));
                    None
                }
            })
            .collect(),
        Some(other) => {
            warnings.push(format!("{path}: expected array, got {}", value_type(other)));
            Vec::new()
        }
    }
}

fn value_type(value: &Value) -> &'static str {
    match value {
        Value::Null => "null",
        Value::Bool(_) => "boolean",
        Value::Number(_) => "number",
        Value::String(_) => "string",
        Value::Sequence(_) => "array",
        Value::Mapping(_) => "mapping",
        Value::Tagged(_) => "tagged",
    }
}

fn infer_dns_protocol(server: Option<&str>, protocol: Option<DnsProtocol>) -> DnsProtocol {
    if let Some(protocol) = protocol {
        return protocol;
    }

    let Some(server) = server else {
        return DnsProtocol::Udp;
    };

    let normalized = server.trim().to_ascii_lowercase();
    if normalized.starts_with("https://") {
        DnsProtocol::Doh
    } else if normalized.starts_with("tls://") || normalized.starts_with("dot://") {
        DnsProtocol::Dot
    } else if normalized.starts_with("tcp://") {
        DnsProtocol::Tcp
    } else {
        DnsProtocol::Udp
    }
}

fn dns_protocol_name(protocol: DnsProtocol) -> String {
    format!("{protocol:?}")
}

fn default_dns_port(protocol: &DnsProtocol) -> u16 {
    match protocol {
        DnsProtocol::Udp | DnsProtocol::Tcp => 53,
        DnsProtocol::Doh => 443,
        DnsProtocol::Dot => 853,
    }
}

fn resolver_protocol(protocol: DnsProtocol) -> Protocol {
    match protocol {
        DnsProtocol::Udp => Protocol::Udp,
        DnsProtocol::Tcp => Protocol::Tcp,
        DnsProtocol::Doh => Protocol::Https,
        DnsProtocol::Dot => Protocol::Tls,
    }
}

fn parse_dns_server_endpoint(server: &str, protocol: &DnsProtocol) -> Result<DnsServerEndpoint> {
    let parts = parse_dns_server_parts(server)?;
    validate_dns_scheme(parts.scheme.as_deref(), protocol)?;

    let port = parts.port.unwrap_or_else(|| default_dns_port(protocol));
    let ip = parts.host.parse::<IpAddr>().or_else(|_| {
        known_dns_ip_for_host(&parts.host).ok_or_else(|| {
            anyhow!(
                "unsupported DNS hostname `{}`; use a registered DNS provider hostname or an IP address",
                parts.host
            )
        })
    })?;

    let tls_dns_name = match protocol {
        DnsProtocol::Doh | DnsProtocol::Dot => tls_dns_name_for_endpoint(&parts.host, ip),
        DnsProtocol::Udp | DnsProtocol::Tcp => None,
    };

    Ok(DnsServerEndpoint {
        socket_addr: SocketAddr::new(ip, port),
        tls_dns_name,
    })
}

fn parse_dns_server_parts(server: &str) -> Result<DnsServerParts> {
    let trimmed = server.trim();
    if trimmed.is_empty() {
        return Err(anyhow!("DNS server cannot be empty"));
    }

    let (scheme, authority) = if let Some(index) = trimmed.find("://") {
        let scheme = trimmed[..index].to_ascii_lowercase();
        let authority = trimmed[index + 3..].split('/').next().unwrap_or_default().trim();
        (Some(scheme), authority)
    } else {
        (None, trimmed)
    };

    if authority.is_empty() {
        return Err(anyhow!("DNS server host cannot be empty"));
    }

    let (host, port) = split_dns_authority(authority)?;
    if host.is_empty() {
        return Err(anyhow!("DNS server host cannot be empty"));
    }

    Ok(DnsServerParts {
        scheme,
        host: host.to_ascii_lowercase(),
        port,
    })
}

fn split_dns_authority(authority: &str) -> Result<(String, Option<u16>)> {
    if let Some(rest) = authority.strip_prefix('[') {
        let end = rest
            .find(']')
            .ok_or_else(|| anyhow!("invalid bracketed IPv6 DNS server `{authority}`"))?;
        let host = rest[..end].to_string();
        let suffix = &rest[end + 1..];
        let port = if suffix.is_empty() {
            None
        } else {
            let Some(port_text) = suffix.strip_prefix(':') else {
                return Err(anyhow!("invalid DNS server `{authority}`"));
            };
            Some(parse_dns_port(port_text)?)
        };
        return Ok((host, port));
    }

    let colon_count = authority.matches(':').count();
    if colon_count == 1 {
        let (host, port_text) = authority
            .rsplit_once(':')
            .ok_or_else(|| anyhow!("invalid DNS server `{authority}`"))?;
        return Ok((host.to_string(), Some(parse_dns_port(port_text)?)));
    }

    Ok((authority.to_string(), None))
}

fn parse_dns_port(port_text: &str) -> Result<u16> {
    port_text
        .parse::<u16>()
        .map_err(|_| anyhow!("invalid DNS server port `{port_text}`"))
}

fn validate_dns_scheme(scheme: Option<&str>, protocol: &DnsProtocol) -> Result<()> {
    let Some(scheme) = scheme else {
        return Ok(());
    };

    let matched = match protocol {
        DnsProtocol::Udp => scheme == "udp" || scheme == "dns",
        DnsProtocol::Tcp => scheme == "tcp",
        DnsProtocol::Doh => scheme == "https",
        DnsProtocol::Dot => scheme == "tls" || scheme == "dot",
    };

    if matched {
        Ok(())
    } else {
        Err(anyhow!(
            "DNS server scheme `{scheme}` does not match protocol `{protocol:?}`"
        ))
    }
}

fn known_dns_ip_for_host(host: &str) -> Option<IpAddr> {
    find_dns_provider_by_host(host).and_then(DnsServerProviderDefinition::preferred_ip)
}

fn tls_dns_name_for_endpoint(host: &str, ip: IpAddr) -> Option<String> {
    if host.parse::<IpAddr>().is_err() {
        return find_dns_provider_by_host(host)
            .map(|provider| provider.canonical_host.to_string())
            .or_else(|| Some(host.to_string()));
    }

    find_dns_provider_by_ip(&ip).map(|provider| provider.canonical_host.to_string())
}

fn find_dns_provider_by_kind(kind: DnsServerProviderKind) -> Option<&'static DnsServerProviderDefinition> {
    provider_definitions()
        .iter()
        .copied()
        .find(|provider| provider.kind == kind)
}

fn find_dns_provider_by_host(host: &str) -> Option<&'static DnsServerProviderDefinition> {
    provider_definitions()
        .iter()
        .copied()
        .find(|provider| provider.matches_host(host))
}

fn find_dns_provider_by_ip(ip: &IpAddr) -> Option<&'static DnsServerProviderDefinition> {
    provider_definitions()
        .iter()
        .copied()
        .find(|provider| provider.matches_ip(ip))
}

async fn create_resolver(server: Option<String>, protocol: Option<DnsProtocol>) -> Result<TokioAsyncResolver> {
    create_resolver_with_options(
        server,
        protocol,
        DnsRuntimeQueryOptions {
            timeout_ms: DEFAULT_DNS_RUNTIME_TIMEOUT_MS,
            attempts: DEFAULT_DNS_RUNTIME_ATTEMPTS,
        },
    )
    .await
}

async fn create_resolver_with_options(
    server: Option<String>,
    protocol: Option<DnsProtocol>,
    options: DnsRuntimeQueryOptions,
) -> Result<TokioAsyncResolver> {
    let effective_protocol = infer_dns_protocol(server.as_deref(), protocol);

    let Some(server_addr) = server else {
        let mut opts = ResolverOpts::default();
        opts.timeout = Duration::from_millis(options.timeout_ms);
        opts.attempts = options.attempts as usize;
        return Ok(TokioAsyncResolver::tokio(ResolverConfig::default(), opts));
    };

    let endpoint = parse_dns_server_endpoint(&server_addr, &effective_protocol)?;
    let mut config = ResolverConfig::new();
    config.add_name_server(NameServerConfig {
        socket_addr: endpoint.socket_addr,
        protocol: resolver_protocol(effective_protocol),
        tls_dns_name: endpoint.tls_dns_name,
        tls_config: None,
        trust_negative_responses: true,
        bind_addr: None,
    });

    let mut opts = ResolverOpts::default();
    opts.timeout = Duration::from_millis(options.timeout_ms);
    opts.attempts = options.attempts as usize;

    Ok(TokioAsyncResolver::tokio(config, opts))
}

pub async fn dns_query(
    domain: String,
    server: Option<String>,
    protocol: Option<DnsProtocol>,
) -> Result<DnsQueryResult> {
    dns_query_with_options(
        domain,
        server,
        protocol,
        DnsRuntimeQueryOptions {
            timeout_ms: DEFAULT_DNS_RUNTIME_TIMEOUT_MS,
            attempts: DEFAULT_DNS_RUNTIME_ATTEMPTS,
        },
    )
    .await
}

pub async fn dns_query_with_options(
    domain: String,
    server: Option<String>,
    protocol: Option<DnsProtocol>,
    options: DnsRuntimeQueryOptions,
) -> Result<DnsQueryResult> {
    let start = Instant::now();
    let effective_protocol = infer_dns_protocol(server.as_deref(), protocol);
    let protocol_str = if server.is_none() && protocol.is_none() {
        "System".to_string()
    } else {
        dns_protocol_name(effective_protocol)
    };

    let resolver = create_resolver_with_options(server.clone(), Some(effective_protocol), options).await?;
    let name = Name::from_str(&domain)?;

    match resolver.lookup_ip(name).await {
        Ok(response) => {
            let latency = start.elapsed().as_millis() as u64;

            if let Some(ip) = response.iter().next() {
                Ok(DnsQueryResult {
                    domain,
                    ip: ip.to_string(),
                    latency,
                    success: true,
                    error: None,
                    protocol: protocol_str,
                })
            } else {
                Ok(DnsQueryResult {
                    domain,
                    ip: String::new(),
                    latency,
                    success: false,
                    error: Some("No IP address found".to_string()),
                    protocol: protocol_str,
                })
            }
        }
        Err(e) => {
            let latency = start.elapsed().as_millis() as u64;
            Ok(DnsQueryResult {
                domain,
                ip: String::new(),
                latency,
                success: false,
                error: Some(e.to_string()),
                protocol: protocol_str,
            })
        }
    }
}

pub async fn dns_health_check(
    server: String,
    test_domain: Option<String>,
    protocol: Option<DnsProtocol>,
) -> Result<DnsHealthCheckResult> {
    let domain = test_domain.unwrap_or_else(|| DEFAULT_DNS_HEALTH_CHECK_DOMAIN.to_string());
    let start = Instant::now();
    let effective_protocol = infer_dns_protocol(Some(&server), protocol);
    let protocol_str = dns_protocol_name(effective_protocol);

    let resolver = create_resolver(Some(server.clone()), Some(effective_protocol)).await?;
    let name = Name::from_str(&domain)?;

    match resolver.lookup_ip(name).await {
        Ok(_) => {
            let latency = start.elapsed().as_millis() as u64;
            Ok(DnsHealthCheckResult {
                server,
                latency,
                success: true,
                error: None,
                protocol: protocol_str,
            })
        }
        Err(e) => {
            let latency = start.elapsed().as_millis() as u64;
            Ok(DnsHealthCheckResult {
                server,
                latency,
                success: false,
                error: Some(e.to_string()),
                protocol: protocol_str,
            })
        }
    }
}

pub fn plan_dns_server_probe_target(server: &str) -> Result<DnsServerProbeTarget> {
    let effective_protocol = infer_dns_protocol(Some(server), None);
    let endpoint = parse_dns_server_endpoint(server, &effective_protocol)?;

    Ok(DnsServerProbeTarget {
        server: server.trim().into(),
        protocol: effective_protocol,
        protocol_name: dns_protocol_name(effective_protocol),
        socket_addr: endpoint.socket_addr.to_string().into(),
        tls_dns_name: endpoint.tls_dns_name,
    })
}

pub async fn probe_dns_server_provider(
    kind: DnsServerProviderKind,
    protocol: Option<DnsProtocol>,
    test_domain: Option<&str>,
) -> DnsServerProviderHealthReport {
    let checked_at = SystemTime::now();
    let Some(provider) = find_dns_provider_by_kind(kind) else {
        return DnsServerProviderHealthReport {
            provider_kind: kind,
            provider_label: "Unknown DNS Provider".to_string(),
            server: String::new(),
            protocol: String::new(),
            test_domain: test_domain.unwrap_or(DEFAULT_DNS_HEALTH_CHECK_DOMAIN).to_string(),
            healthy: false,
            message: format!("provider {:?} is not registered", kind),
            latency_ms: None,
            checked_at,
        };
    };

    let effective_protocol = protocol.unwrap_or_else(|| provider.default_protocol());
    let test_domain = test_domain.unwrap_or(DEFAULT_DNS_HEALTH_CHECK_DOMAIN).to_string();
    let Some(server) = provider.server_for_protocol(effective_protocol) else {
        return DnsServerProviderHealthReport {
            provider_kind: provider.kind,
            provider_label: provider.label.to_string(),
            server: String::new(),
            protocol: dns_protocol_name(effective_protocol),
            test_domain,
            healthy: false,
            message: format!(
                "provider {} does not support protocol {:?}",
                provider.label, effective_protocol
            ),
            latency_ms: None,
            checked_at,
        };
    };

    match dns_health_check(server.clone(), Some(test_domain.clone()), Some(effective_protocol)).await {
        Ok(result) => DnsServerProviderHealthReport {
            provider_kind: provider.kind,
            provider_label: provider.label.to_string(),
            server,
            protocol: result.protocol,
            test_domain,
            healthy: result.success,
            message: result
                .error
                .unwrap_or_else(|| "provider health check succeeded".to_string()),
            latency_ms: Some(result.latency),
            checked_at,
        },
        Err(error) => DnsServerProviderHealthReport {
            provider_kind: provider.kind,
            provider_label: provider.label.to_string(),
            server,
            protocol: dns_protocol_name(effective_protocol),
            test_domain,
            healthy: false,
            message: error.to_string(),
            latency_ms: None,
            checked_at,
        },
    }
}

pub async fn save_dns_config_mapping(dns_config: &Mapping) -> Result<()> {
    let dns_path = crate::utils::dirs::app_home_dir()?.join(crate::constants::files::DNS_CONFIG);
    let yaml_str = serde_yaml_ng::to_string(dns_config)?;
    fs::write(&dns_path, yaml_str).await?;
    log::info!("[DnsRuntime] DNS config saved to {dns_path:?}");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn doh_url_uses_known_endpoint_and_tls_name() {
        let endpoint = parse_dns_server_endpoint("https://dns.google/dns-query", &DnsProtocol::Doh).unwrap();

        assert_eq!(endpoint.socket_addr.to_string(), "8.8.8.8:443");
        assert_eq!(endpoint.tls_dns_name.as_deref(), Some("dns.google"));
    }

    #[test]
    fn dot_url_uses_known_endpoint_and_tls_name() {
        let endpoint = parse_dns_server_endpoint("tls://dns.quad9.net:853", &DnsProtocol::Dot).unwrap();

        assert_eq!(endpoint.socket_addr.to_string(), "9.9.9.9:853");
        assert_eq!(endpoint.tls_dns_name.as_deref(), Some("dns.quad9.net"));
    }

    #[test]
    fn plain_ipv4_uses_protocol_default_port() {
        let endpoint = parse_dns_server_endpoint("1.1.1.1", &DnsProtocol::Udp).unwrap();

        assert_eq!(endpoint.socket_addr.to_string(), "1.1.1.1:53");
        assert_eq!(endpoint.tls_dns_name, None);
    }

    #[test]
    fn protocol_is_inferred_from_url_scheme_when_omitted() {
        assert_eq!(
            infer_dns_protocol(Some("https://cloudflare-dns.com/dns-query"), None),
            DnsProtocol::Doh
        );
        assert_eq!(infer_dns_protocol(Some("tls://dns.google"), None), DnsProtocol::Dot);
    }

    #[test]
    fn provider_registry_resolves_alias_to_canonical_tls_name() {
        let endpoint = parse_dns_server_endpoint("https://one.one.one.one/dns-query", &DnsProtocol::Doh).unwrap();

        assert_eq!(endpoint.socket_addr.to_string(), "1.1.1.1:443");
        assert_eq!(endpoint.tls_dns_name.as_deref(), Some("cloudflare-dns.com"));
    }

    #[test]
    fn provider_registry_exposes_supported_dns_providers() {
        let providers = provider_definitions();

        assert_eq!(providers.len(), 6);
        assert_eq!(providers[0].kind, DnsServerProviderKind::Cloudflare);
        assert_eq!(providers[0].label, "Cloudflare DNS");
        assert!(providers[0].matches_host("one.one.one.one"));
        assert!(providers[0].matches_ip(&"1.0.0.1".parse().unwrap()));
    }

    #[test]
    fn public_provider_registrations_include_recommended_servers() {
        let providers = list_dns_server_provider_registrations();
        let cloudflare = providers
            .into_iter()
            .find(|provider| provider.kind == DnsServerProviderKind::Cloudflare)
            .expect("cloudflare provider should exist");

        assert_eq!(cloudflare.canonical_host, "cloudflare-dns.com");
        assert_eq!(cloudflare.supported_protocols.len(), 4);
        assert!(cloudflare.recommended_servers.iter().any(
            |server| server.protocol == DnsProtocol::Doh && server.server == "https://cloudflare-dns.com/dns-query"
        ));
        assert!(
            cloudflare
                .recommended_servers
                .iter()
                .any(|server| server.protocol == DnsProtocol::Dot && server.server == "tls://cloudflare-dns.com:853")
        );
    }

    #[test]
    fn probe_target_plan_reuses_runtime_endpoint_resolution() {
        let target = plan_dns_server_probe_target("https://cloudflare-dns.com/dns-query").unwrap();

        assert_eq!(target.protocol, DnsProtocol::Doh);
        assert_eq!(target.protocol_name, "Doh");
        assert_eq!(target.socket_addr, "1.1.1.1:443");
        assert_eq!(target.tls_dns_name.as_deref(), Some("cloudflare-dns.com"));
    }

    #[test]
    fn resolver_plan_uses_nameserver_as_runtime_targets() {
        let plan = build_dns_resolver_plan(
            r#"
dns:
  enable: true
  enhanced-mode: fake-ip
  fake-ip-range: 198.18.0.1/16
  timeout: 3000
  nameserver:
    - 1.1.1.1
    - https://dns.google/dns-query
  fallback-filter:
    geoip: true
  nameserver-policy:
    "+.example.com": 8.8.8.8
"#,
        )
        .unwrap();

        assert_eq!(plan.status, DnsResolverPlanStatus::Ready);
        assert_eq!(plan.timeout_ms, 3000);
        assert_eq!(plan.attempts, DEFAULT_DNS_RUNTIME_ATTEMPTS);
        assert_eq!(plan.nameservers.len(), 2);
        assert!(plan.nameservers.iter().all(|server| server.runtime_supported));
        assert!(plan.runtime_projection.fake_ip.configured);
        assert!(!plan.runtime_projection.fake_ip.runtime_applied);
        assert!(plan.runtime_projection.fallback_filter.configured);
        assert!(plan.runtime_projection.nameserver_policy.configured);
    }

    #[test]
    fn resolver_plan_rejects_missing_nameservers_fail_soft() {
        let plan = build_dns_resolver_plan(
            r#"
dns:
  enable: true
  enhanced-mode: normal
"#,
        )
        .unwrap();

        assert_eq!(plan.status, DnsResolverPlanStatus::Rejected);
        assert!(plan.reason.contains("dns.nameserver is empty"));
    }

    #[test]
    fn resolver_plan_marks_unsupported_nameserver_without_panicking() {
        let plan = build_dns_resolver_plan(
            r#"
dns:
  enable: true
  nameserver:
    - https://unregistered.example/dns-query
"#,
        )
        .unwrap();

        assert_eq!(plan.status, DnsResolverPlanStatus::Rejected);
        assert_eq!(plan.nameservers.len(), 1);
        assert!(!plan.nameservers[0].runtime_supported);
        assert!(plan.nameservers[0].reason.contains("unsupported DNS hostname"));
    }

    struct StaticDnsRuntime;

    #[async_trait::async_trait]
    impl RustDnsResolverRuntime for StaticDnsRuntime {
        async fn query(
            &self,
            nameserver: &DnsResolverNameserverPlan,
            domain: &str,
            _options: DnsRuntimeQueryOptions,
        ) -> Result<DnsQueryResult> {
            Ok(DnsQueryResult {
                domain: domain.to_string(),
                ip: "93.184.216.34".to_string(),
                latency: 12,
                success: true,
                error: None,
                protocol: nameserver.protocol_name.clone(),
            })
        }
    }

    #[tokio::test]
    async fn controlled_probe_summarizes_supported_unsupported_and_plan_only_dns_features() {
        let plan = build_dns_resolver_plan(
            r#"
dns:
  enable: true
  enhanced-mode: fake-ip
  nameserver:
    - 1.1.1.1
    - https://unregistered.example/dns-query
  fallback-filter:
    geoip: true
"#,
        )
        .unwrap();
        let controller = DnsResolverRuntimeController::new(StaticDnsRuntime);
        let report = controller.probe(plan, "example.com".to_string()).await;

        assert_eq!(report.test_domain, "example.com");
        assert_eq!(report.summary.total_targets, 2);
        assert_eq!(report.summary.runtime_supported_targets, 1);
        assert_eq!(report.summary.healthy_targets, 1);
        assert_eq!(report.summary.unsupported_targets, 1);
        assert_eq!(report.metrics.total_queries, 1);
        assert_eq!(report.metrics.successful_queries, 1);
        assert_eq!(report.targets[0].provider_kind, Some(DnsServerProviderKind::Cloudflare));
        assert_eq!(report.targets[0].provider_label.as_deref(), Some("Cloudflare DNS"));
        assert!(report.targets[0].message.contains("resolved example.com"));
        assert!(report.targets[1].message.contains("unsupported DNS hostname"));
        assert!(
            report
                .warnings
                .iter()
                .any(|warning| warning.contains("fake-ip is plan-only"))
        );
        assert!(
            report
                .warnings
                .iter()
                .any(|warning| warning.contains("fallback-filter is plan-only"))
        );
    }

    #[test]
    fn default_runtime_readiness_blocks_plan_only_dns_features() {
        let report = build_dns_default_runtime_readiness_report(
            r#"
dns:
  enable: true
  enhanced-mode: fake-ip
  nameserver:
    - 1.1.1.1
  fallback-filter:
    geoip: true
  nameserver-policy:
    "+.example.com": 1.1.1.1
"#,
            None,
        )
        .unwrap();

        assert_eq!(report.status, DnsDefaultRuntimeReadinessStatus::Blocked);
        assert!(report.blockers.iter().any(|blocker| blocker.contains("fake-ip")));
        assert!(
            report
                .blockers
                .iter()
                .any(|blocker| blocker.contains("fallback-filter"))
        );
        assert!(
            report
                .warnings
                .iter()
                .any(|warning| warning.contains("controlled probe evidence is missing"))
        );
    }

    #[tokio::test]
    async fn default_runtime_readiness_passes_with_supported_nameserver_and_probe() {
        let yaml = r#"
dns:
  enable: true
  enhanced-mode: normal
  nameserver:
    - 1.1.1.1
"#;
        let plan = build_dns_resolver_plan(yaml).unwrap();
        let controller = DnsResolverRuntimeController::new(StaticDnsRuntime);
        let probe = controller.probe(plan, "example.com".to_string()).await;
        let report = build_dns_default_runtime_readiness_report(yaml, Some(probe)).unwrap();

        assert_eq!(report.status, DnsDefaultRuntimeReadinessStatus::Ready);
        assert_eq!(report.summary.failed, 0);
        assert_eq!(report.summary.warnings, 0);
        assert_eq!(report.probe_summary.unwrap().healthy_targets, 1);
    }

    #[test]
    fn default_runtime_readiness_blocks_unsupported_nameserver_coverage() {
        let report = build_dns_default_runtime_readiness_report(
            r#"
dns:
  enable: true
  enhanced-mode: normal
  nameserver:
    - https://unregistered.example/dns-query
"#,
            None,
        )
        .unwrap();

        assert_eq!(report.status, DnsDefaultRuntimeReadinessStatus::Blocked);
        assert!(
            report
                .blockers
                .iter()
                .any(|blocker| blocker.contains("complete Rust resolver plan"))
        );
        assert!(
            report
                .blockers
                .iter()
                .any(|blocker| blocker.contains("not Rust-runtime supported"))
        );
    }

    #[tokio::test]
    async fn default_runtime_shadow_evidence_matches_system_result() {
        let yaml = r#"
dns:
  enable: true
  enhanced-mode: normal
  nameserver:
    - 1.1.1.1
"#;
        let plan = build_dns_resolver_plan(yaml).unwrap();
        let controller = DnsResolverRuntimeController::new(StaticDnsRuntime);
        let probe = controller.probe(plan.clone(), "example.com".to_string()).await;
        let readiness = build_dns_default_runtime_readiness_report(yaml, Some(probe)).unwrap();
        let rust_report = controller.query(plan, "example.com".to_string()).await;
        let system_result = DnsQueryResult {
            domain: "example.com".to_string(),
            ip: "93.184.216.34".to_string(),
            latency: 18,
            success: true,
            error: None,
            protocol: "System".to_string(),
        };

        let report = build_dns_default_runtime_shadow_evidence_report(readiness, rust_report, system_result);

        assert_eq!(report.status, DnsDefaultRuntimeShadowEvidenceStatus::Matched);
        assert!(report.query.ip_match);
        assert!(report.query.mismatch_reason.is_none());
    }

    #[tokio::test]
    async fn default_runtime_shadow_evidence_stays_blocked_when_readiness_blocks() {
        let yaml = r#"
dns:
  enable: true
  enhanced-mode: fake-ip
  nameserver:
    - 1.1.1.1
"#;
        let plan = build_dns_resolver_plan(yaml).unwrap();
        let controller = DnsResolverRuntimeController::new(StaticDnsRuntime);
        let rust_report = controller.query(plan, "example.com".to_string()).await;
        let readiness = build_dns_default_runtime_readiness_report(yaml, None).unwrap();
        let system_result = DnsQueryResult {
            domain: "example.com".to_string(),
            ip: "93.184.216.34".to_string(),
            latency: 18,
            success: true,
            error: None,
            protocol: "System".to_string(),
        };

        let report = build_dns_default_runtime_shadow_evidence_report(readiness, rust_report, system_result);

        assert_eq!(report.status, DnsDefaultRuntimeShadowEvidenceStatus::Blocked);
        assert!(
            report
                .blockers
                .iter()
                .any(|blocker| blocker.contains("cannot prove default runtime replacement"))
        );
    }

    #[tokio::test]
    async fn default_runtime_opt_in_switch_guard_passes_only_as_read_only_preflight() {
        let yaml = r#"
dns:
  enable: true
  enhanced-mode: normal
  nameserver:
    - 1.1.1.1
"#;
        let plan = build_dns_resolver_plan(yaml).unwrap();
        let controller = DnsResolverRuntimeController::new(StaticDnsRuntime);
        let probe = controller.probe(plan.clone(), "example.com".to_string()).await;
        let readiness = build_dns_default_runtime_readiness_report(yaml, Some(probe)).unwrap();
        let rust_report = controller.query(plan, "example.com".to_string()).await;
        let system_result = DnsQueryResult {
            domain: "example.com".to_string(),
            ip: "93.184.216.34".to_string(),
            latency: 18,
            success: true,
            error: None,
            protocol: "System".to_string(),
        };
        let shadow = build_dns_default_runtime_shadow_evidence_report(readiness, rust_report, system_result);

        let report = build_dns_default_runtime_opt_in_switch_guard_report(shadow, true);

        assert_eq!(report.status, DnsDefaultRuntimeOptInSwitchGuardStatus::Ready);
        assert!(report.explicit_opt_in);
        assert!(!report.mutates_runtime);
        assert_eq!(report.activation_mode, "preflightOnly");
        assert!(report.rollback_plan.supported);
    }

    #[tokio::test]
    async fn default_runtime_opt_in_switch_guard_blocks_without_explicit_opt_in() {
        let yaml = r#"
dns:
  enable: true
  enhanced-mode: normal
  nameserver:
    - 1.1.1.1
"#;
        let plan = build_dns_resolver_plan(yaml).unwrap();
        let controller = DnsResolverRuntimeController::new(StaticDnsRuntime);
        let probe = controller.probe(plan.clone(), "example.com".to_string()).await;
        let readiness = build_dns_default_runtime_readiness_report(yaml, Some(probe)).unwrap();
        let rust_report = controller.query(plan, "example.com".to_string()).await;
        let system_result = DnsQueryResult {
            domain: "example.com".to_string(),
            ip: "93.184.216.34".to_string(),
            latency: 18,
            success: true,
            error: None,
            protocol: "System".to_string(),
        };
        let shadow = build_dns_default_runtime_shadow_evidence_report(readiness, rust_report, system_result);

        let report = build_dns_default_runtime_opt_in_switch_guard_report(shadow, false);

        assert_eq!(report.status, DnsDefaultRuntimeOptInSwitchGuardStatus::Blocked);
        assert!(
            report
                .blockers
                .iter()
                .any(|blocker| blocker.contains("explicit user opt-in"))
        );
    }

    #[tokio::test]
    async fn default_runtime_executor_preflight_builds_dry_run_diff_and_audit() {
        let yaml = r#"
dns:
  enable: true
  enhanced-mode: normal
  nameserver:
    - 1.1.1.1
"#;
        let plan = build_dns_resolver_plan(yaml).unwrap();
        let controller = DnsResolverRuntimeController::new(StaticDnsRuntime);
        let probe = controller.probe(plan.clone(), "example.com".to_string()).await;
        let readiness = build_dns_default_runtime_readiness_report(yaml, Some(probe)).unwrap();
        let rust_report = controller.query(plan, "example.com".to_string()).await;
        let system_result = DnsQueryResult {
            domain: "example.com".to_string(),
            ip: "93.184.216.34".to_string(),
            latency: 18,
            success: true,
            error: None,
            protocol: "System".to_string(),
        };
        let shadow = build_dns_default_runtime_shadow_evidence_report(readiness, rust_report, system_result);
        let guard = build_dns_default_runtime_opt_in_switch_guard_report(shadow, true);

        let report = build_dns_default_runtime_opt_in_executor_preflight_report(guard);

        assert_eq!(report.status, DnsDefaultRuntimeExecutorPreflightStatus::Ready);
        assert!(report.dry_run);
        assert!(report.would_mutate_runtime);
        assert!(!report.executed);
        assert!(!report.reload_mihomo);
        assert_eq!(report.mutation_diff.nameserver_targets, vec!["1.1.1.1"]);
        assert!(report.rollback_marker.prepared);
        assert_eq!(report.audit_record.action, "defaultDnsRuntimeOptInExecutorPreflight");
    }

    #[tokio::test]
    async fn default_runtime_executor_preflight_blocks_when_guard_blocks() {
        let yaml = r#"
dns:
  enable: true
  enhanced-mode: fake-ip
  nameserver:
    - 1.1.1.1
"#;
        let plan = build_dns_resolver_plan(yaml).unwrap();
        let controller = DnsResolverRuntimeController::new(StaticDnsRuntime);
        let rust_report = controller.query(plan, "example.com".to_string()).await;
        let readiness = build_dns_default_runtime_readiness_report(yaml, None).unwrap();
        let system_result = DnsQueryResult {
            domain: "example.com".to_string(),
            ip: "93.184.216.34".to_string(),
            latency: 18,
            success: true,
            error: None,
            protocol: "System".to_string(),
        };
        let shadow = build_dns_default_runtime_shadow_evidence_report(readiness, rust_report, system_result);
        let guard = build_dns_default_runtime_opt_in_switch_guard_report(shadow, true);

        let report = build_dns_default_runtime_opt_in_executor_preflight_report(guard);

        assert_eq!(report.status, DnsDefaultRuntimeExecutorPreflightStatus::Blocked);
        assert!(!report.executed);
        assert!(
            report
                .blockers
                .iter()
                .any(|blocker| blocker.contains("opt-in switch guard is not ready"))
        );
    }

    #[tokio::test]
    async fn default_runtime_execution_guard_allows_only_after_persistence_is_prepared() {
        let yaml = r#"
dns:
  enable: true
  enhanced-mode: normal
  nameserver:
    - 1.1.1.1
"#;
        let plan = build_dns_resolver_plan(yaml).unwrap();
        let controller = DnsResolverRuntimeController::new(StaticDnsRuntime);
        let probe = controller.probe(plan.clone(), "example.com".to_string()).await;
        let readiness = build_dns_default_runtime_readiness_report(yaml, Some(probe)).unwrap();
        let rust_report = controller.query(plan, "example.com".to_string()).await;
        let system_result = DnsQueryResult {
            domain: "example.com".to_string(),
            ip: "93.184.216.34".to_string(),
            latency: 18,
            success: true,
            error: None,
            protocol: "System".to_string(),
        };
        let shadow = build_dns_default_runtime_shadow_evidence_report(readiness, rust_report, system_result);
        let guard = build_dns_default_runtime_opt_in_switch_guard_report(shadow, true);
        let preflight = build_dns_default_runtime_opt_in_executor_preflight_report(guard);
        let superseded_state = default_runtime_execution_superseded_state(&preflight);
        let persistence = DnsDefaultRuntimeExecutionPersistence {
            requested: true,
            prepared: true,
            audit_record_path: Some("audit.yaml".into()),
            rollback_marker_path: Some("rollback-marker.yaml".into()),
            superseded_state_path: Some("superseded-state.yaml".into()),
            audit_persisted: true,
            rollback_marker_persisted: true,
            superseded_state_persisted: true,
            errors: Vec::new(),
        };

        let report = build_dns_default_runtime_opt_in_execution_guard_report(preflight, persistence, superseded_state);

        assert_eq!(report.status, DnsDefaultRuntimeExecutionGuardStatus::Ready);
        assert!(report.execution_allowed);
        assert!(report.user_trigger_required);
        assert!(!report.mutates_runtime);
        assert!(!report.executed);
        assert!(!report.reload_mihomo);
    }

    #[tokio::test]
    async fn default_runtime_execution_guard_blocks_without_persistence() {
        let yaml = r#"
dns:
  enable: true
  enhanced-mode: normal
  nameserver:
    - 1.1.1.1
"#;
        let plan = build_dns_resolver_plan(yaml).unwrap();
        let controller = DnsResolverRuntimeController::new(StaticDnsRuntime);
        let probe = controller.probe(plan.clone(), "example.com".to_string()).await;
        let readiness = build_dns_default_runtime_readiness_report(yaml, Some(probe)).unwrap();
        let rust_report = controller.query(plan, "example.com".to_string()).await;
        let system_result = DnsQueryResult {
            domain: "example.com".to_string(),
            ip: "93.184.216.34".to_string(),
            latency: 18,
            success: true,
            error: None,
            protocol: "System".to_string(),
        };
        let shadow = build_dns_default_runtime_shadow_evidence_report(readiness, rust_report, system_result);
        let guard = build_dns_default_runtime_opt_in_switch_guard_report(shadow, true);
        let preflight = build_dns_default_runtime_opt_in_executor_preflight_report(guard);
        let superseded_state = default_runtime_execution_superseded_state(&preflight);
        let persistence = DnsDefaultRuntimeExecutionPersistence {
            requested: true,
            prepared: false,
            audit_record_path: None,
            rollback_marker_path: None,
            superseded_state_path: None,
            audit_persisted: false,
            rollback_marker_persisted: false,
            superseded_state_persisted: false,
            errors: vec!["persistence unavailable".into()],
        };

        let report = build_dns_default_runtime_opt_in_execution_guard_report(preflight, persistence, superseded_state);

        assert_eq!(report.status, DnsDefaultRuntimeExecutionGuardStatus::Blocked);
        assert!(!report.execution_allowed);
        assert!(
            report
                .blockers
                .iter()
                .any(|blocker| blocker.contains("persistence unavailable"))
        );
    }

    #[tokio::test]
    async fn default_runtime_post_execution_verification_passes_with_observed_match_and_drill() {
        let yaml = r#"
dns:
  enable: true
  enhanced-mode: normal
  nameserver:
    - 1.1.1.1
"#;
        let plan = build_dns_resolver_plan(yaml).unwrap();
        let controller = DnsResolverRuntimeController::new(StaticDnsRuntime);
        let probe = controller.probe(plan.clone(), "example.com".to_string()).await;
        let readiness = build_dns_default_runtime_readiness_report(yaml, Some(probe)).unwrap();
        let rust_report = controller.query(plan, "example.com".to_string()).await;
        let system_result = DnsQueryResult {
            domain: "example.com".to_string(),
            ip: "93.184.216.34".to_string(),
            latency: 18,
            success: true,
            error: None,
            protocol: "System".to_string(),
        };
        let observed_evidence = build_dns_default_runtime_shadow_evidence_report(readiness, rust_report, system_result);
        let active_state = DnsDefaultRuntimeActiveState {
            active_runtime: "rustDefaultDnsResolver".into(),
            previous_runtime: "mihomoManagedDefaultDns".into(),
            state: "active".into(),
            execution_event_id: "dns-default-runtime-limited-execution-1".into(),
            activated_at_epoch_seconds: 1,
            rollback_marker_path: Some("rollback-marker.yaml".into()),
            audit_record_path: Some("audit.yaml".into()),
        };
        let execution_record = DnsDefaultRuntimeExecutionRecord {
            event_id: active_state.execution_event_id.clone(),
            action: "defaultDnsRuntimeLimitedOptInExecution".into(),
            status: "executed".into(),
            guard_event_id: "dns-default-runtime-executor-preflight-1".into(),
            previous_runtime: "mihomoManagedDefaultDns".into(),
            candidate_runtime: "rustDefaultDnsResolver".into(),
            created_at_epoch_seconds: 1,
            metadata_verified: true,
            error: None,
        };
        let pre_execution_audit_record = DnsDefaultRuntimeExecutorAuditRecord {
            event_id: execution_record.guard_event_id.clone(),
            action: "defaultDnsRuntimeOptInExecutorPreflight".into(),
            dry_run: true,
            created_at_epoch_seconds: 1,
            guard_status: DnsDefaultRuntimeOptInSwitchGuardStatus::Ready,
            readiness_status: DnsDefaultRuntimeReadinessStatus::Ready,
            shadow_status: DnsDefaultRuntimeShadowEvidenceStatus::Matched,
        };
        let rollback_marker = DnsDefaultRuntimeExecutorRollbackMarker {
            required: true,
            prepared: true,
            strategy: "restoreMihomoManagedDefaultDnsRuntime".into(),
            restores_runtime: true,
            previous_runtime: "mihomoManagedDefaultDns".into(),
            candidate_runtime: "rustDefaultDnsResolver".into(),
        };
        let rollback_drill = build_dns_default_runtime_rollback_drill_report(
            Some(active_state.clone()),
            Some(execution_record.clone()),
            Some(rollback_marker),
            Vec::new(),
        );

        let report = build_dns_default_runtime_post_execution_observed_verification_report(
            Some(active_state),
            Some(execution_record),
            Some(pre_execution_audit_record),
            observed_evidence,
            rollback_drill,
            Vec::new(),
        );

        assert_eq!(
            report.status,
            DnsDefaultRuntimePostExecutionVerificationStatus::Verified
        );
        assert_eq!(
            report.rollback_drill.status,
            DnsDefaultRuntimeRollbackDrillStatus::Ready
        );
        assert!(!report.failure_audit.required);
        assert!(!report.mutates_runtime);
        assert!(!report.reload_mihomo);

        let gate = build_dns_default_runtime_expanded_opt_in_execution_gate_report(report, true);

        assert_eq!(gate.status, DnsDefaultRuntimeExpandedOptInExecutionGateStatus::Ready);
        assert!(gate.expansion_allowed);
        assert!(gate.user_trigger_required);
        assert!(!gate.auto_rollout);
        assert!(!gate.executed);
        assert!(!gate.reload_mihomo);

        let preflight = build_dns_default_runtime_expanded_opt_in_execution_preflight_report(
            gate,
            true,
            true,
            Some("preflight.yaml".into()),
        );

        assert_eq!(
            preflight.status,
            DnsDefaultRuntimeExpandedOptInExecutionPreflightStatus::Ready
        );
        assert!(preflight.would_mutate_runtime);
        assert!(!preflight.mutates_runtime);
        assert!(!preflight.executed);
        assert!(!preflight.reload_mihomo);
        assert!(preflight.preflight_record.mutation_plan.active_profile_write);
        assert!(preflight.preflight_record.mutation_plan.mihomo_reload);

        let expanded_execution =
            build_dns_default_runtime_expanded_opt_in_execution_report(preflight, true, true, Vec::new());

        assert_eq!(
            expanded_execution.status,
            DnsDefaultRuntimeExpandedOptInExecutionStatus::Executed
        );
        assert!(expanded_execution.mutates_runtime);
        assert!(expanded_execution.executed);
        assert!(expanded_execution.reload_mihomo);
        assert!(expanded_execution.rollback_available);
        assert_eq!(
            expanded_execution
                .active_state
                .as_ref()
                .map(|state| state.state.as_str()),
            Some("expandedActiveProfileReloaded")
        );

        let expanded_drill = build_dns_default_runtime_expanded_rollback_drill_report(
            expanded_execution.active_state.clone(),
            Some(expanded_execution.execution_record.clone()),
            Some(expanded_execution.preflight.preflight_record.clone()),
            Vec::new(),
        );

        assert_eq!(
            expanded_drill.status,
            DnsDefaultRuntimeExpandedRollbackDrillStatus::Ready
        );
        assert!(!expanded_drill.auto_rollback);
        assert_eq!(expanded_drill.would_restore_runtime, "mihomoManagedDefaultDns");

        let expanded_plan = build_dns_resolver_plan(yaml).unwrap();
        let expanded_probe = controller.probe(expanded_plan.clone(), "example.com".to_string()).await;
        let expanded_readiness = build_dns_default_runtime_readiness_report(yaml, Some(expanded_probe)).unwrap();
        let expanded_rust_report = controller.query(expanded_plan, "example.com".to_string()).await;
        let expanded_system_result = DnsQueryResult {
            domain: "example.com".to_string(),
            ip: "93.184.216.34".to_string(),
            latency: 18,
            success: true,
            error: None,
            protocol: "System".to_string(),
        };
        let expanded_observed_evidence = build_dns_default_runtime_shadow_evidence_report(
            expanded_readiness,
            expanded_rust_report,
            expanded_system_result,
        );
        let expanded_post_execution = build_dns_default_runtime_expanded_post_execution_observed_verification_report(
            expanded_execution.active_state,
            Some(expanded_execution.execution_record),
            Some(expanded_execution.preflight.preflight_record),
            expanded_observed_evidence,
            expanded_drill,
            Vec::new(),
        );

        assert_eq!(
            expanded_post_execution.status,
            DnsDefaultRuntimeExpandedPostExecutionVerificationStatus::Verified
        );
        assert!(!expanded_post_execution.failure_audit.required);
        assert!(!expanded_post_execution.mutates_runtime);
        assert!(!expanded_post_execution.reload_mihomo);

        let stability_gate = build_dns_default_runtime_expanded_stability_gate_report(expanded_post_execution, true);

        assert_eq!(
            stability_gate.status,
            DnsDefaultRuntimeExpandedStabilityGateStatus::Ready
        );
        assert!(stability_gate.keep_active_allowed);
        assert!(!stability_gate.rollback_recommended);
        assert!(!stability_gate.promotion_allowed);
        assert!(!stability_gate.auto_rollout);
        assert!(!stability_gate.auto_rollback);

        let hold_started_at = stability_gate
            .post_execution
            .active_state
            .as_ref()
            .map(|state| state.activated_at_epoch_seconds)
            .unwrap();
        let hold_policy =
            build_dns_default_runtime_expanded_hold_policy_report(stability_gate, hold_started_at.saturating_add(300));

        assert_eq!(hold_policy.status, DnsDefaultRuntimeExpandedHoldPolicyStatus::Ready);
        assert_eq!(hold_policy.active_age_seconds, Some(300));
        assert!(hold_policy.keep_active_allowed);
        assert!(!hold_policy.next_verification_required);
        assert!(!hold_policy.rollback_recommended);
        assert!(!hold_policy.promotion_allowed);
        assert!(!hold_policy.auto_rollout);
        assert!(!hold_policy.auto_rollback);

        let reverify = build_dns_default_runtime_expanded_reverify_report(
            hold_policy,
            true,
            Some("reverify.yaml".into()),
            Vec::new(),
            hold_started_at.saturating_add(300),
        );

        assert_eq!(reverify.status, DnsDefaultRuntimeExpandedReverifyStatus::Recorded);
        assert!(reverify.reverify_persisted);
        assert_eq!(
            reverify.reverify_record.hold_status,
            DnsDefaultRuntimeExpandedHoldPolicyStatus::Ready
        );
        assert!(reverify.keep_active_allowed);
        assert!(!reverify.next_verification_required);
        assert!(!reverify.rollback_recommended);
        assert!(!reverify.auto_rollout);
        assert!(!reverify.auto_rollback);

        let mut second_record = reverify.reverify_record.clone();
        second_record.event_id = "dns-default-runtime-expanded-reverify-second".into();
        second_record.created_at_epoch_seconds = hold_started_at.saturating_add(600);
        second_record.active_age_seconds = Some(600);
        let history = build_dns_default_runtime_expanded_reverify_history_report(
            vec![reverify.reverify_record.clone(), second_record],
            Vec::new(),
        );

        assert_eq!(history.status, DnsDefaultRuntimeExpandedReverifyHistoryStatus::Ready);
        assert_eq!(history.record_count, 2);
        assert_eq!(history.stable_streak, 2);
        assert!(history.closeout_ready);
        assert!(!history.rollback_recommended);
        assert!(!history.promotion_allowed);
        assert!(!history.auto_rollout);
        assert!(!history.auto_rollback);

        let active_state = reverify.hold_policy.stability_gate.post_execution.active_state.clone();
        let closeout = build_dns_default_runtime_expanded_lifecycle_closeout_report(history, active_state, Vec::new());

        assert_eq!(
            closeout.status,
            DnsDefaultRuntimeExpandedLifecycleCloseoutStatus::Complete
        );
        assert!(closeout.observation_closed);
        assert!(closeout.handoff_ready);
        assert!(!closeout.rollback_recommended);
        assert!(!closeout.promotion_allowed);
        assert!(!closeout.auto_rollout);
        assert!(!closeout.auto_rollback);
    }

    #[test]
    fn default_runtime_rollback_drill_blocks_without_active_state() {
        let report = build_dns_default_runtime_rollback_drill_report(None, None, None, Vec::new());

        assert_eq!(report.status, DnsDefaultRuntimeRollbackDrillStatus::Blocked);
        assert!(!report.would_rollback);
        assert!(!report.auto_rollback);
        assert!(
            report
                .blockers
                .iter()
                .any(|blocker| blocker.contains("active state was not found"))
        );
    }

    #[tokio::test]
    async fn default_runtime_expanded_opt_in_gate_blocks_without_explicit_opt_in() {
        let yaml = r#"
dns:
  enable: true
  enhanced-mode: normal
  nameserver:
    - 1.1.1.1
"#;
        let plan = build_dns_resolver_plan(yaml).unwrap();
        let controller = DnsResolverRuntimeController::new(StaticDnsRuntime);
        let probe = controller.probe(plan.clone(), "example.com".to_string()).await;
        let readiness = build_dns_default_runtime_readiness_report(yaml, Some(probe)).unwrap();
        let rust_report = controller.query(plan, "example.com".to_string()).await;
        let system_result = DnsQueryResult {
            domain: "example.com".to_string(),
            ip: "93.184.216.34".to_string(),
            latency: 18,
            success: true,
            error: None,
            protocol: "System".to_string(),
        };
        let observed_evidence = build_dns_default_runtime_shadow_evidence_report(readiness, rust_report, system_result);
        let rollback_drill = build_dns_default_runtime_rollback_drill_report(None, None, None, Vec::new());
        let post_execution = build_dns_default_runtime_post_execution_observed_verification_report(
            None,
            None,
            None,
            observed_evidence,
            rollback_drill,
            Vec::new(),
        );

        let gate = build_dns_default_runtime_expanded_opt_in_execution_gate_report(post_execution, false);

        assert_eq!(gate.status, DnsDefaultRuntimeExpandedOptInExecutionGateStatus::Blocked);
        assert!(!gate.expansion_allowed);
        assert!(!gate.auto_rollout);
        assert!(gate.blockers.iter().any(|blocker| blocker.contains("explicit opt-in")));
    }
}
