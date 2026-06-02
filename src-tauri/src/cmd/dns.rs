use crate::cmd::{CmdResult, StringifyErr};
use crate::core::dns_runtime::{
    DnsHealthCheckResult, DnsProtocol, DnsQueryResult, dns_health_check as build_dns_health_check,
    dns_query as build_dns_query,
};
use log::error;

/// DNS 查询
///
/// 支持自定义 DNS 服务器和协议（UDP/TCP/DoH/DoT）
#[tauri::command]
pub async fn dns_query(
    domain: String,
    server: Option<String>,
    protocol: Option<DnsProtocol>,
) -> CmdResult<DnsQueryResult> {
    build_dns_query(domain, server, protocol).await.stringify_err()
}

/// DNS 服务器健康检查
///
/// 使用指定的测试域名检查 DNS 服务器的健康状态
#[tauri::command]
pub async fn dns_health_check(
    server: String,
    test_domain: Option<String>,
    protocol: Option<DnsProtocol>,
) -> CmdResult<DnsHealthCheckResult> {
    build_dns_health_check(server, test_domain, protocol)
        .await
        .stringify_err()
}

/// 批量 DNS 查询
#[tauri::command]
pub async fn dns_batch_query(
    domains: Vec<String>,
    server: Option<String>,
    protocol: Option<DnsProtocol>,
) -> CmdResult<Vec<DnsQueryResult>> {
    let mut results = Vec::new();

    for domain in domains {
        match build_dns_query(domain, server.clone(), protocol.clone()).await {
            Ok(result) => results.push(result),
            Err(e) => {
                error!("DNS batch query error: {}", e);
            }
        }
    }

    Ok(results)
}

/// 批量 DNS 健康检查
#[tauri::command]
pub async fn dns_batch_health_check(
    servers: Vec<String>,
    test_domain: Option<String>,
    protocol: Option<DnsProtocol>,
) -> CmdResult<Vec<DnsHealthCheckResult>> {
    let mut results = Vec::new();
    let domain = test_domain.clone();

    for server in servers {
        match build_dns_health_check(server, domain.clone(), protocol.clone()).await {
            Ok(result) => results.push(result),
            Err(e) => {
                error!("DNS batch health check error: {}", e);
            }
        }
    }

    Ok(results)
}
