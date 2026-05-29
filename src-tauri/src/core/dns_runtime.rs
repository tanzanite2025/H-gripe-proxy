use anyhow::Result;
use hickory_proto::rr::Name;
use hickory_resolver::config::*;
use hickory_resolver::TokioAsyncResolver;
use serde::{Deserialize, Serialize};
use std::str::FromStr;
use std::time::Instant;

/// DNS 协议类型
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DnsProtocol {
    Udp,
    Tcp,
    Doh, // DNS over HTTPS
    Dot, // DNS over TLS
}

/// DNS 查询结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DnsQueryResult {
    pub domain: String,
    pub ip: String,
    pub latency: u64, // 毫秒
    pub success: bool,
    pub error: Option<String>,
    pub protocol: String,
}

/// DNS 服务器健康检查结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DnsHealthCheckResult {
    pub server: String,
    pub latency: u64,
    pub success: bool,
    pub error: Option<String>,
    pub protocol: String,
}

/// 创建 DNS 解析器
async fn create_resolver(
    server: Option<String>,
    protocol: Option<DnsProtocol>,
) -> Result<TokioAsyncResolver> {
    let protocol = protocol.unwrap_or(DnsProtocol::Udp);

    // 如果没有指定服务器，使用系统默认
    if server.is_none() {
        return Ok(TokioAsyncResolver::tokio(
            ResolverConfig::default(),
            ResolverOpts::default(),
        ));
    }

    let server_addr = server.unwrap();
    let mut config = ResolverConfig::new();

    match protocol {
        DnsProtocol::Udp => {
            // UDP DNS (标准 DNS，端口 53)
            let socket_addr = if server_addr.contains(':') {
                server_addr.clone()
            } else {
                format!("{}:53", server_addr)
            };

            config.add_name_server(NameServerConfig {
                socket_addr: socket_addr.parse()?,
                protocol: Protocol::Udp,
                tls_dns_name: None,
                tls_config: None,
                trust_negative_responses: true,
                bind_addr: None,
            });
        }
        DnsProtocol::Tcp => {
            // TCP DNS (端口 53)
            let socket_addr = if server_addr.contains(':') {
                server_addr.clone()
            } else {
                format!("{}:53", server_addr)
            };

            config.add_name_server(NameServerConfig {
                socket_addr: socket_addr.parse()?,
                protocol: Protocol::Tcp,
                tls_dns_name: None,
                tls_config: None,
                trust_negative_responses: true,
                bind_addr: None,
            });
        }
        DnsProtocol::Doh => {
            // DNS over HTTPS (端口 443)
            let socket_addr = if server_addr.contains(':') {
                server_addr.clone()
            } else {
                format!("{}:443", server_addr)
            };

            // 从 IP 地址提取 TLS DNS 名称
            let tls_dns_name = match server_addr.as_str() {
                "1.1.1.1" | "1.0.0.1" => Some("cloudflare-dns.com".to_string()),
                "8.8.8.8" | "8.8.4.4" => Some("dns.google".to_string()),
                "9.9.9.9" => Some("dns.quad9.net".to_string()),
                _ => None,
            };

            config.add_name_server(NameServerConfig {
                socket_addr: socket_addr.parse()?,
                protocol: Protocol::Https,
                tls_dns_name,
                tls_config: None,
                trust_negative_responses: true,
                bind_addr: None,
            });
        }
        DnsProtocol::Dot => {
            // DNS over TLS (端口 853)
            let socket_addr = if server_addr.contains(':') {
                server_addr.clone()
            } else {
                format!("{}:853", server_addr)
            };

            // 从 IP 地址提取 TLS DNS 名称
            let tls_dns_name = match server_addr.as_str() {
                "1.1.1.1" | "1.0.0.1" => Some("cloudflare-dns.com".to_string()),
                "8.8.8.8" | "8.8.4.4" => Some("dns.google".to_string()),
                "9.9.9.9" => Some("dns.quad9.net".to_string()),
                _ => None,
            };

            config.add_name_server(NameServerConfig {
                socket_addr: socket_addr.parse()?,
                protocol: Protocol::Tls,
                tls_dns_name,
                tls_config: None,
                trust_negative_responses: true,
                bind_addr: None,
            });
        }
    }

    let mut opts = ResolverOpts::default();
    opts.timeout = std::time::Duration::from_secs(5);
    opts.attempts = 2;

    Ok(TokioAsyncResolver::tokio(config, opts))
}

/// DNS 查询
/// 支持自定义 DNS 服务器和协议（UDP/TCP/DoH/DoT）
pub async fn dns_query(
    domain: String,
    server: Option<String>,
    protocol: Option<DnsProtocol>,
) -> Result<DnsQueryResult> {
    let start = Instant::now();
    let protocol_str = protocol
        .as_ref()
        .map(|p| format!("{:?}", p))
        .unwrap_or_else(|| "System".to_string());

    // 创建解析器
    let resolver = create_resolver(server.clone(), protocol.clone()).await?;

    // 解析域名
    let name = Name::from_str(&domain)?;

    match resolver.lookup_ip(name).await {
        Ok(response) => {
            let latency = start.elapsed().as_millis() as u64;

            // 获取第一个 IP 地址
            if let Some(ip) = response.iter().next() {
                Ok(DnsQueryResult {
                    domain: domain.clone(),
                    ip: ip.to_string(),
                    latency,
                    success: true,
                    error: None,
                    protocol: protocol_str,
                })
            } else {
                Ok(DnsQueryResult {
                    domain: domain.clone(),
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
                domain: domain.clone(),
                ip: String::new(),
                latency,
                success: false,
                error: Some(e.to_string()),
                protocol: protocol_str,
            })
        }
    }
}

/// DNS 服务器健康检查
/// 使用指定的测试域名检查 DNS 服务器的健康状态
pub async fn dns_health_check(
    server: String,
    test_domain: Option<String>,
    protocol: Option<DnsProtocol>,
) -> Result<DnsHealthCheckResult> {
    let domain = test_domain.unwrap_or_else(|| "www.google.com".to_string());
    let start = Instant::now();
    let protocol_str = protocol
        .as_ref()
        .map(|p| format!("{:?}", p))
        .unwrap_or_else(|| "Udp".to_string());

    // 创建解析器
    let resolver = create_resolver(Some(server.clone()), protocol.clone()).await?;

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
