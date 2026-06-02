use anyhow::{Result, anyhow};
use hickory_proto::rr::Name;
use hickory_resolver::TokioAsyncResolver;
use hickory_resolver::config::*;
use serde::{Deserialize, Serialize};
use std::net::{IpAddr, SocketAddr};
use std::str::FromStr;
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DnsProtocol {
    Udp,
    Tcp,
    Doh,
    Dot,
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
                "unsupported DNS hostname `{}`; use a known DNS provider hostname or an IP address",
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
    let ip = match host {
        "cloudflare-dns.com" | "one.one.one.one" | "1dot1dot1dot1.cloudflare-dns.com" => "1.1.1.1",
        "dns.google" => "8.8.8.8",
        "dns.quad9.net" => "9.9.9.9",
        "dns.alidns.com" => "223.5.5.5",
        "doh.pub" => "119.29.29.29",
        "dot.pub" => "1.12.12.12",
        _ => return None,
    };

    ip.parse().ok()
}

fn tls_dns_name_for_endpoint(host: &str, ip: IpAddr) -> Option<String> {
    if host.parse::<IpAddr>().is_err() {
        return Some(host.to_string());
    }

    match ip.to_string().as_str() {
        "1.1.1.1" | "1.0.0.1" => Some("cloudflare-dns.com".to_string()),
        "8.8.8.8" | "8.8.4.4" => Some("dns.google".to_string()),
        "9.9.9.9" => Some("dns.quad9.net".to_string()),
        "223.5.5.5" | "223.6.6.6" => Some("dns.alidns.com".to_string()),
        "119.29.29.29" | "120.53.53.53" => Some("doh.pub".to_string()),
        "1.12.12.12" => Some("dot.pub".to_string()),
        _ => None,
    }
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
    let domain = test_domain.unwrap_or_else(|| "www.google.com".to_string());
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
}
