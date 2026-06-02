/**
 * 本地安全监控模块
 *
 * 功能：
 * 1. 本地绑定监控 - 确保端口只绑定到 127.0.0.1
 * 2. 端口冲突检测 - 检测端口占用并支持自动切换
 * 3. 泄漏监控 - 实时监控本地安全状态
 */
use anyhow::{Result, anyhow};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::net::{IpAddr, Ipv4Addr, SocketAddr, TcpListener};
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::sync::RwLock;

use super::firewall::FirewallManager;

/// 本地安全配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocalSecurityConfig {
    /// 绑定地址（强制 127.0.0.1）
    pub bind_address: String,
    /// 端口随机化
    pub port_randomization: bool,
    /// 端口范围
    pub port_range: (u16, u16),
    /// 端口冲突自动切换
    pub auto_switch_on_conflict: bool,
    /// 防火墙自动配置
    pub auto_firewall: bool,
    /// 进程隐蔽
    pub process_stealth: bool,
    /// 泄漏监控
    pub leak_monitoring: bool,
    /// 监控间隔（秒）
    pub monitor_interval: u64,
}

impl Default for LocalSecurityConfig {
    fn default() -> Self {
        Self {
            bind_address: "127.0.0.1".to_string(),
            port_randomization: false,
            port_range: (10800, 10900),
            auto_switch_on_conflict: true,
            auto_firewall: false,
            process_stealth: false,
            leak_monitoring: true,
            monitor_interval: 30,
        }
    }
}

/// 泄漏监控状态
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LeakMonitorStatus {
    /// 本地绑定安全
    pub local_binding_secure: bool,
    /// 防火墙规则生效
    pub firewall_rules_active: bool,
    /// 进程隐蔽
    pub process_hidden: bool,
    /// 外部访问被阻止
    pub external_access_blocked: bool,
    /// 最后检查时间（Unix 时间戳）
    pub last_check_time: i64,
    /// 是否检测到泄漏
    pub leak_detected: bool,
    /// 泄漏类型
    pub leak_type: Option<String>,
    /// 是否自动修复
    pub auto_fix_applied: bool,
}

impl Default for LeakMonitorStatus {
    fn default() -> Self {
        Self {
            local_binding_secure: false,
            firewall_rules_active: false,
            process_hidden: false,
            external_access_blocked: false,
            last_check_time: 0,
            leak_detected: false,
            leak_type: None,
            auto_fix_applied: false,
        }
    }
}

/// 安全错误类型
#[derive(Debug, thiserror::Error)]
pub enum SecurityError {
    #[error("Port {0} is not bound to localhost")]
    NotLocalBinding(u16),

    #[error("Port {0} is already in use")]
    PortConflict(u16),

    #[error("Failed to get network connections: {0}")]
    NetworkError(String),

    #[error("Firewall configuration failed: {0}")]
    FirewallError(String),

    #[error("Security leak detected: {0}")]
    LeakDetected(String),
}

/// 网络连接信息
#[derive(Debug, Clone)]
struct NetworkConnection {
    local_address: String,
    local_port: u16,
    state: String,
    protocol: String,
}

/// 绑定检查缓存
struct BindingCache {
    cache: HashMap<u16, (bool, SystemTime)>,
    ttl: Duration,
}

impl BindingCache {
    fn new(ttl_secs: u64) -> Self {
        Self {
            cache: HashMap::new(),
            ttl: Duration::from_secs(ttl_secs),
        }
    }

    fn get(&self, port: u16) -> Option<bool> {
        if let Some((result, timestamp)) = self.cache.get(&port) {
            if timestamp.elapsed().ok()? < self.ttl {
                return Some(*result);
            }
        }
        None
    }

    fn set(&mut self, port: u16, result: bool) {
        self.cache.insert(port, (result, SystemTime::now()));
    }

    fn clear(&mut self) {
        self.cache.clear();
    }
}

/// 本地安全监控器
pub struct LocalSecurityMonitor {
    config: Arc<RwLock<LocalSecurityConfig>>,
    status: Arc<RwLock<LeakMonitorStatus>>,
    cache: Arc<RwLock<BindingCache>>,
    firewall_manager: Arc<FirewallManager>,
}

impl LocalSecurityMonitor {
    /// 创建新的监控器实例
    pub fn new(config: LocalSecurityConfig) -> Self {
        let firewall_manager = Arc::new(FirewallManager::new(config.clone()));
        Self {
            config: Arc::new(RwLock::new(config)),
            status: Arc::new(RwLock::new(LeakMonitorStatus::default())),
            cache: Arc::new(RwLock::new(BindingCache::new(10))), // 10秒缓存
            firewall_manager,
        }
    }

    /// 获取配置
    pub async fn get_config(&self) -> LocalSecurityConfig {
        self.config.read().await.clone()
    }

    /// 更新配置
    pub async fn update_config(&self, config: LocalSecurityConfig) {
        let mut cfg = self.config.write().await;
        *cfg = config;
        // 清除缓存以使新配置生效
        self.cache.write().await.clear();
    }

    /// 获取状态
    pub async fn get_status(&self) -> LeakMonitorStatus {
        self.status.read().await.clone()
    }

    /// 检查本地绑定是否安全
    ///
    /// 返回 true 表示端口只绑定到 127.0.0.1，false 表示绑定到其他地址
    pub async fn check_local_binding(&self, port: u16) -> Result<bool> {
        // 检查缓存
        if let Some(cached) = self.cache.read().await.get(port) {
            return Ok(cached);
        }

        // 执行实际检查
        let result = self.check_local_binding_impl(port).await?;

        // 更新缓存
        self.cache.write().await.set(port, result);

        Ok(result)
    }

    /// 实际的本地绑定检查实现
    async fn check_local_binding_impl(&self, port: u16) -> Result<bool> {
        let start = std::time::Instant::now();
        let connections = self.get_network_connections().await?;

        // 查找指定端口的监听连接
        let listeners: Vec<_> = connections
            .iter()
            .filter(|c| c.local_port == port && c.state == "LISTEN")
            .collect();

        if listeners.is_empty() {
            // 端口未被监听，认为是安全的
            log::trace!("Port {} not listening, check took {:?}", port, start.elapsed());
            return Ok(true);
        }

        // 检查所有监听是否都绑定到 127.0.0.1
        for listener in listeners {
            if listener.local_address != "127.0.0.1" 
                && listener.local_address != "::1" // IPv6 localhost
                && listener.local_address != "0.0.0.0" // 这个不安全
                && listener.local_address != "::"
            // IPv6 any，不安全
            {
                // 如果绑定到其他地址，检查是否是本地地址
                if !is_localhost(&listener.local_address) {
                    log::warn!(
                        "Port {} bound to non-localhost address: {} (protocol {}), check took {:?}",
                        port,
                        listener.local_address,
                        listener.protocol,
                        start.elapsed()
                    );
                    return Ok(false);
                }
            } else if listener.local_address == "0.0.0.0" || listener.local_address == "::" {
                // 绑定到 0.0.0.0 或 :: 是不安全的
                log::warn!(
                    "Port {} bound to wildcard address: {} (protocol {}), check took {:?}",
                    port,
                    listener.local_address,
                    listener.protocol,
                    start.elapsed()
                );
                return Ok(false);
            }
        }

        log::trace!("Port {} binding secure, check took {:?}", port, start.elapsed());
        Ok(true)
    }

    /// 检查端口冲突
    ///
    /// 返回 true 表示端口被占用，false 表示端口可用
    pub async fn check_port_conflict(&self, port: u16) -> Result<bool> {
        // 尝试绑定端口
        let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), port);

        match TcpListener::bind(addr) {
            Ok(_) => Ok(false), // 端口可用
            Err(_) => Ok(true), // 端口被占用
        }
    }

    /// 查找可用端口
    ///
    /// 在配置的端口范围内查找第一个可用的端口
    pub async fn find_available_port(&self) -> Result<u16> {
        let config = self.config.read().await;
        let (start, end) = config.port_range;

        for port in start..=end {
            if !self.check_port_conflict(port).await? {
                return Ok(port);
            }
        }

        Err(anyhow!("No available port in range {}-{}", start, end))
    }

    /// 执行完整的安全检查
    pub async fn perform_security_check(&self, port: u16) -> Result<LeakMonitorStatus> {
        let config = self.config.read().await.clone();

        // 1. 检查本地绑定
        let binding_check = self.check_local_binding(port).await;
        let (binding_secure, binding_error) = match binding_check {
            Ok(ok) => (ok, None),
            Err(err) => (false, Some(err.to_string())),
        };

        // 1.1 检查端口冲突
        let port_conflict = self.check_port_conflict(port).await.unwrap_or(false);

        // 2. 检查防火墙规则
        let firewall_active = if config.auto_firewall {
            self.firewall_manager.check_firewall_rules(port).await.unwrap_or(false)
        } else {
            false
        };

        // 3. 检查外部访问（TODO: 实现外部访问检查）
        let external_blocked = true; // 暂时返回 true

        // 4. 检查进程隐蔽（TODO: 实现进程检查）
        let process_hidden = config.process_stealth;

        // 5. 确定是否检测到泄漏
        let leak_error = if port_conflict {
            Some(SecurityError::PortConflict(port))
        } else if let Some(err) = binding_error {
            Some(SecurityError::NetworkError(err))
        } else if !binding_secure {
            Some(SecurityError::NotLocalBinding(port))
        } else if config.auto_firewall && !firewall_active {
            Some(SecurityError::FirewallError("Firewall rules inactive".to_string()))
        } else if !external_blocked {
            Some(SecurityError::LeakDetected("External access not blocked".to_string()))
        } else {
            None
        };

        let leak_detected = leak_error.is_some();
        let leak_type = leak_error.as_ref().map(|e| e.to_string());

        // 6. 生成状态
        let status = LeakMonitorStatus {
            local_binding_secure: binding_secure,
            firewall_rules_active: firewall_active,
            process_hidden,
            external_access_blocked: external_blocked,
            last_check_time: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs() as i64,
            leak_detected,
            leak_type,
            auto_fix_applied: false,
        };

        // 7. 更新状态
        let mut current_status = self.status.write().await;
        *current_status = status.clone();

        Ok(status)
    }

    /// 配置防火墙规则
    pub async fn configure_firewall(&self, port: u16) -> Result<()> {
        self.firewall_manager.configure_firewall(port).await
    }

    /// 删除防火墙规则
    pub async fn remove_firewall_rules(&self, port: u16) -> Result<()> {
        self.firewall_manager.remove_firewall_rules(port).await
    }

    /// 获取网络连接信息
    #[cfg(target_os = "windows")]
    async fn get_network_connections(&self) -> Result<Vec<NetworkConnection>> {
        use crate::utils::command::hidden_command;

        let output = hidden_command("netstat")
            .args(&["-ano", "-p", "TCP"])
            .output()
            .map_err(|e| anyhow!("Failed to execute netstat: {}", e))?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut connections = Vec::new();

        for line in stdout.lines().skip(4) {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 4 {
                if let Some((addr, port)) = parse_socket_addr(parts[1]) {
                    connections.push(NetworkConnection {
                        local_address: addr,
                        local_port: port,
                        state: parts[3].to_string(),
                        protocol: "TCP".to_string(),
                    });
                }
            }
        }

        Ok(connections)
    }

    /// 获取网络连接信息（Linux）
    #[cfg(target_os = "linux")]
    async fn get_network_connections(&self) -> Result<Vec<NetworkConnection>> {
        use std::fs;

        let content =
            fs::read_to_string("/proc/net/tcp").map_err(|e| anyhow!("Failed to read /proc/net/tcp: {}", e))?;

        let mut connections = Vec::new();

        for line in content.lines().skip(1) {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 4 {
                if let Some((addr, port)) = parse_hex_socket_addr(parts[1]) {
                    let state = parse_tcp_state(parts[3]);
                    connections.push(NetworkConnection {
                        local_address: addr,
                        local_port: port,
                        state,
                        protocol: "TCP".to_string(),
                    });
                }
            }
        }

        Ok(connections)
    }

    /// 获取网络连接信息（macOS）
    #[cfg(target_os = "macos")]
    async fn get_network_connections(&self) -> Result<Vec<NetworkConnection>> {
        use std::process::Command;

        let output = Command::new("lsof")
            .args(&["-iTCP", "-sTCP:LISTEN", "-n", "-P"])
            .output()
            .map_err(|e| anyhow!("Failed to execute lsof: {}", e))?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut connections = Vec::new();

        for line in stdout.lines().skip(1) {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 9 {
                if let Some((addr, port)) = parse_socket_addr(parts[8]) {
                    connections.push(NetworkConnection {
                        local_address: addr,
                        local_port: port,
                        state: "LISTEN".to_string(),
                        protocol: "TCP".to_string(),
                    });
                }
            }
        }

        Ok(connections)
    }
}

/// 解析套接字地址（格式：127.0.0.1:8080）
fn parse_socket_addr(addr_str: &str) -> Option<(String, u16)> {
    let parts: Vec<&str> = addr_str.rsplitn(2, ':').collect();
    if parts.len() == 2 {
        let port = parts[0].parse::<u16>().ok()?;
        let addr = parts[1].to_string();
        Some((addr, port))
    } else {
        None
    }
}

/// 解析十六进制套接字地址（Linux /proc/net/tcp 格式）
#[cfg(target_os = "linux")]
fn parse_hex_socket_addr(hex_str: &str) -> Option<(String, u16)> {
    let parts: Vec<&str> = hex_str.split(':').collect();
    if parts.len() == 2 {
        let addr_hex = parts[0];
        let port_hex = parts[1];

        // 解析端口
        let port = u16::from_str_radix(port_hex, 16).ok()?;

        // 解析地址（小端序）
        let addr_num = u32::from_str_radix(addr_hex, 16).ok()?;
        let addr = format!(
            "{}.{}.{}.{}",
            addr_num & 0xFF,
            (addr_num >> 8) & 0xFF,
            (addr_num >> 16) & 0xFF,
            (addr_num >> 24) & 0xFF
        );

        Some((addr, port))
    } else {
        None
    }
}

/// 解析 TCP 状态（Linux /proc/net/tcp 格式）
#[cfg(target_os = "linux")]
fn parse_tcp_state(state_hex: &str) -> String {
    match state_hex {
        "0A" => "LISTEN".to_string(),
        "01" => "ESTABLISHED".to_string(),
        "02" => "SYN_SENT".to_string(),
        "03" => "SYN_RECV".to_string(),
        "04" => "FIN_WAIT1".to_string(),
        "05" => "FIN_WAIT2".to_string(),
        "06" => "TIME_WAIT".to_string(),
        "07" => "CLOSE".to_string(),
        "08" => "CLOSE_WAIT".to_string(),
        "09" => "LAST_ACK".to_string(),
        _ => "UNKNOWN".to_string(),
    }
}

/// 检查地址是否为本地地址
fn is_localhost(addr: &str) -> bool {
    addr == "127.0.0.1" || addr == "::1" || addr.starts_with("127.") || addr == "localhost"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_local_binding_check() {
        let config = LocalSecurityConfig::default();
        let monitor = LocalSecurityMonitor::new(config);

        // 测试一个不太可能被占用的端口
        let result = monitor.check_local_binding(65432).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_port_conflict_detection() {
        let config = LocalSecurityConfig::default();
        let monitor = LocalSecurityMonitor::new(config);

        // 绑定一个端口
        let _listener = TcpListener::bind("127.0.0.1:65433").unwrap();

        // 检查冲突
        let conflict = monitor.check_port_conflict(65433).await.unwrap();
        assert!(conflict, "Port should be in conflict");

        // 检查未占用的端口
        let no_conflict = monitor.check_port_conflict(65434).await.unwrap();
        assert!(!no_conflict, "Port should be available");
    }

    #[tokio::test]
    async fn test_find_available_port() {
        let config = LocalSecurityConfig {
            port_range: (65400, 65500),
            ..Default::default()
        };
        let monitor = LocalSecurityMonitor::new(config);

        let port = monitor.find_available_port().await;
        assert!(port.is_ok());

        let port_num = port.unwrap();
        assert!(port_num >= 65400 && port_num <= 65500);
    }

    #[test]
    fn test_parse_socket_addr() {
        let (addr, port) = parse_socket_addr("127.0.0.1:8080").unwrap();
        assert_eq!(addr, "127.0.0.1");
        assert_eq!(port, 8080);

        let (addr, port) = parse_socket_addr("[::1]:8080").unwrap();
        assert_eq!(addr, "[::1]");
        assert_eq!(port, 8080);
    }

    #[test]
    fn test_is_localhost() {
        assert!(is_localhost("127.0.0.1"));
        assert!(is_localhost("127.0.0.2"));
        assert!(is_localhost("::1"));
        assert!(is_localhost("localhost"));
        assert!(!is_localhost("192.168.1.1"));
        assert!(!is_localhost("0.0.0.0"));
    }

    #[tokio::test]
    async fn test_cache_mechanism() {
        let config = LocalSecurityConfig::default();
        let monitor = LocalSecurityMonitor::new(config);

        // 第一次检查（无缓存）
        let start = SystemTime::now();
        let _ = monitor.check_local_binding(65435).await;
        let first_duration = start.elapsed().unwrap();

        // 第二次检查（有缓存）
        let start = SystemTime::now();
        let _ = monitor.check_local_binding(65435).await;
        let second_duration = start.elapsed().unwrap();

        // 缓存应该更快
        assert!(second_duration < first_duration);
    }

    #[tokio::test]
    async fn test_perform_security_check() {
        let config = LocalSecurityConfig::default();
        let monitor = LocalSecurityMonitor::new(config);

        let status = monitor.perform_security_check(65436).await;
        assert!(status.is_ok());

        let status = status.unwrap();
        assert!(status.last_check_time > 0);
    }

    /// 性能测试：确保本地绑定检查延迟 < 10ms
    #[tokio::test]
    async fn bench_local_binding_check() {
        let config = LocalSecurityConfig::default();
        let monitor = LocalSecurityMonitor::new(config);

        // 预热
        let _ = monitor.check_local_binding(65437).await;

        // 测试 10 次取平均值
        let mut total_duration = std::time::Duration::ZERO;
        const ITERATIONS: usize = 10;

        for _ in 0..ITERATIONS {
            let start = std::time::Instant::now();
            let _ = monitor.check_local_binding(65437).await;
            total_duration += start.elapsed();
        }

        let avg_duration = total_duration / ITERATIONS as u32;
        println!("Average check duration: {:?}", avg_duration);

        // 验收标准：平均延迟 < 10ms
        assert!(
            avg_duration.as_millis() < 10,
            "Average check duration {:?} exceeds 10ms threshold",
            avg_duration
        );
    }

    /// 性能测试：缓存命中性能
    #[tokio::test]
    async fn bench_cached_binding_check() {
        let config = LocalSecurityConfig::default();
        let monitor = LocalSecurityMonitor::new(config);

        // 第一次检查（无缓存）
        let start = std::time::Instant::now();
        let _ = monitor.check_local_binding(65438).await;
        let uncached_duration = start.elapsed();

        // 第二次检查（有缓存）
        let start = std::time::Instant::now();
        let _ = monitor.check_local_binding(65438).await;
        let cached_duration = start.elapsed();

        println!("Uncached: {:?}, Cached: {:?}", uncached_duration, cached_duration);

        // 缓存应该显著更快（至少快 50%）
        assert!(
            cached_duration < uncached_duration / 2,
            "Cached check ({:?}) should be at least 50% faster than uncached ({:?})",
            cached_duration,
            uncached_duration
        );

        // 缓存命中应该 < 1ms
        assert!(
            cached_duration.as_micros() < 1000,
            "Cached check duration {:?} exceeds 1ms threshold",
            cached_duration
        );
    }

    /// 测试端口自动切换逻辑
    #[tokio::test]
    async fn test_auto_port_switch() {
        let config = LocalSecurityConfig {
            port_range: (65440, 65450),
            auto_switch_on_conflict: true,
            ..Default::default()
        };
        let monitor = LocalSecurityMonitor::new(config);

        // 占用几个端口
        let _listener1 = TcpListener::bind("127.0.0.1:65440").unwrap();
        let _listener2 = TcpListener::bind("127.0.0.1:65441").unwrap();

        // 查找可用端口
        let available_port = monitor.find_available_port().await.unwrap();

        // 应该找到 65442 或更高的端口
        assert!(available_port >= 65442 && available_port <= 65450);

        // 验证找到的端口确实可用
        let conflict = monitor.check_port_conflict(available_port).await.unwrap();
        assert!(!conflict, "Found port should be available");
    }

    /// 测试并发检查性能
    #[tokio::test]
    async fn bench_concurrent_checks() {
        let config = LocalSecurityConfig::default();
        let monitor = Arc::new(LocalSecurityMonitor::new(config));

        let start = std::time::Instant::now();

        // 并发执行 100 次检查
        let mut handles = vec![];
        for i in 0..100 {
            let monitor_clone = monitor.clone();
            let handle = tokio::spawn(async move { monitor_clone.check_local_binding(65450 + (i % 10)).await });
            handles.push(handle);
        }

        // 等待所有检查完成
        for handle in handles {
            let _ = handle.await;
        }

        let total_duration = start.elapsed();
        let avg_duration = total_duration / 100;

        println!(
            "100 concurrent checks took {:?}, avg {:?}",
            total_duration, avg_duration
        );

        // 并发检查平均延迟应该 < 20ms（考虑到并发开销）
        assert!(
            avg_duration.as_millis() < 20,
            "Average concurrent check duration {:?} exceeds 20ms threshold",
            avg_duration
        );
    }
}
