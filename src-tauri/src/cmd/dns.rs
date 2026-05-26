use serde::{Deserialize, Serialize};
use std::net::{IpAddr, ToSocketAddrs};
use std::time::{Duration, Instant};
use tauri::State;
use tokio::time::timeout;

/// DNS 查询结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DnsQueryResult {
    pub domain: String,
    pub ip: String,
    pub latency: u64, // 毫秒
    pub success: bool,
    pub error: Option<String>,
}

/// DNS 服务器健康检查结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DnsHealthCheckResult {
    pub server: String,
    pub latency: u64,
    pub success: bool,
    pub error: Option<String>,
}

/// DNS 查询（简单实现）
/// 
/// 使用系统 DNS 解析器进行查询
#[tauri::command]
pub async fn dns_query(domain: String) -> Result<DnsQueryResult, String> {
    let start = Instant::now();
    
    // 设置超时时间为 5 秒
    let query_future = tokio::task::spawn_blocking(move || {
        // 使用系统 DNS 解析
        let addrs = format!("{}:0", domain)
            .to_socket_addrs()
            .map_err(|e| e.to_string())?;
        
        // 获取第一个 IP 地址
        let ip = addrs
            .into_iter()
            .next()
            .ok_or_else(|| "No IP address found".to_string())?
            .ip();
        
        Ok::<IpAddr, String>(ip)
    });
    
    match timeout(Duration::from_secs(5), query_future).await {
        Ok(Ok(Ok(ip))) => {
            let latency = start.elapsed().as_millis() as u64;
            Ok(DnsQueryResult {
                domain: domain.clone(),
                ip: ip.to_string(),
                latency,
                success: true,
                error: None,
            })
        }
        Ok(Ok(Err(e))) => Ok(DnsQueryResult {
            domain: domain.clone(),
            ip: String::new(),
            latency: start.elapsed().as_millis() as u64,
            success: false,
            error: Some(e),
        }),
        Ok(Err(e)) => Ok(DnsQueryResult {
            domain: domain.clone(),
            ip: String::new(),
            latency: start.elapsed().as_millis() as u64,
            success: false,
            error: Some(e.to_string()),
        }),
        Err(_) => Ok(DnsQueryResult {
            domain: domain.clone(),
            ip: String::new(),
            latency: 5000,
            success: false,
            error: Some("DNS query timeout".to_string()),
        }),
    }
}

/// DNS 服务器健康检查
/// 
/// 使用指定的测试域名检查 DNS 服务器的健康状态
#[tauri::command]
pub async fn dns_health_check(
    server: String,
    test_domain: Option<String>,
) -> Result<DnsHealthCheckResult, String> {
    let domain = test_domain.unwrap_or_else(|| "www.google.com".to_string());
    let start = Instant::now();
    
    // 注意：这里使用系统 DNS，实际应该使用指定的 DNS 服务器
    // 完整实现需要使用 trust-dns-resolver 或类似库
    let query_future = tokio::task::spawn_blocking(move || {
        let addrs = format!("{}:0", domain)
            .to_socket_addrs()
            .map_err(|e| e.to_string())?;
        
        addrs
            .into_iter()
            .next()
            .ok_or_else(|| "No IP address found".to_string())?;
        
        Ok::<(), String>(())
    });
    
    match timeout(Duration::from_secs(5), query_future).await {
        Ok(Ok(Ok(_))) => {
            let latency = start.elapsed().as_millis() as u64;
            Ok(DnsHealthCheckResult {
                server: server.clone(),
                latency,
                success: true,
                error: None,
            })
        }
        Ok(Ok(Err(e))) => Ok(DnsHealthCheckResult {
            server: server.clone(),
            latency: start.elapsed().as_millis() as u64,
            success: false,
            error: Some(e),
        }),
        Ok(Err(e)) => Ok(DnsHealthCheckResult {
            server: server.clone(),
            latency: start.elapsed().as_millis() as u64,
            success: false,
            error: Some(e.to_string()),
        }),
        Err(_) => Ok(DnsHealthCheckResult {
            server: server.clone(),
            latency: 5000,
            success: false,
            error: Some("Health check timeout".to_string()),
        }),
    }
}

/// 批量 DNS 查询
#[tauri::command]
pub async fn dns_batch_query(domains: Vec<String>) -> Result<Vec<DnsQueryResult>, String> {
    let mut results = Vec::new();
    
    for domain in domains {
        match dns_query(domain).await {
            Ok(result) => results.push(result),
            Err(e) => {
                log::error!("DNS batch query error: {}", e);
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
) -> Result<Vec<DnsHealthCheckResult>, String> {
    let mut results = Vec::new();
    let domain = test_domain.clone();
    
    for server in servers {
        match dns_health_check(server, domain.clone()).await {
            Ok(result) => results.push(result),
            Err(e) => {
                log::error!("DNS batch health check error: {}", e);
            }
        }
    }
    
    Ok(results)
}
