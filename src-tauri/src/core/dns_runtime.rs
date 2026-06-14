use anyhow::{Result, anyhow};
use hickory_proto::rr::Name;
use hickory_resolver::TokioAsyncResolver;
use hickory_resolver::config::*;
use serde::{Deserialize, Serialize};
use serde_yaml_ng::Mapping;
use std::net::{IpAddr, SocketAddr};
use std::str::FromStr;
use std::time::{Duration, Instant, SystemTime};
use tokio::fs;

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
    let effective_protocol = infer_dns_protocol(server.as_deref(), protocol);

    let Some(server_addr) = server else {
        return Ok(TokioAsyncResolver::tokio(
            ResolverConfig::default(),
            ResolverOpts::default(),
        ));
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
    opts.timeout = Duration::from_secs(5);
    opts.attempts = 2;

    Ok(TokioAsyncResolver::tokio(config, opts))
}

pub async fn dns_query(
    domain: String,
    server: Option<String>,
    protocol: Option<DnsProtocol>,
) -> Result<DnsQueryResult> {
    let start = Instant::now();
    let effective_protocol = infer_dns_protocol(server.as_deref(), protocol);
    let protocol_str = if server.is_none() && protocol.is_none() {
        "System".to_string()
    } else {
        dns_protocol_name(effective_protocol)
    };

    let resolver = create_resolver(server.clone(), Some(effective_protocol)).await?;
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
}
