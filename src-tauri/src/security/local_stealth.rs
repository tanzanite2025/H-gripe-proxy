/**
 * 本地隐蔽增强模块
 *
 * 功能：
 * 1. 进程隐蔽 - 伪装进程标题，降低特征识别
 * 2. 端口隐蔽 - 端口随机化，避免使用标准端口
 * 3. 防本地发现 - 禁用 mDNS/UPnP 等服务发现协议
 */
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::utils::command::hidden_command;

/// 本地隐蔽配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocalStealthConfig {
    /// 进程隐蔽
    pub process_stealth: ProcessStealthConfig,
    /// 端口隐蔽
    pub port_stealth: PortStealthConfig,
    /// 防本地发现
    pub anti_discovery: AntiDiscoveryConfig,
}

impl Default for LocalStealthConfig {
    fn default() -> Self {
        Self {
            process_stealth: ProcessStealthConfig::default(),
            port_stealth: PortStealthConfig::default(),
            anti_discovery: AntiDiscoveryConfig::default(),
        }
    }
}

// ── 进程隐蔽 ──────────────────────────────────────────────

/// 进程隐蔽配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessStealthConfig {
    /// 是否启用进程隐蔽
    pub enabled: bool,
    /// 伪装的进程标题名
    pub disguise_title: String,
}

impl Default for ProcessStealthConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            disguise_title: "System Service Host".to_string(),
        }
    }
}

/// 进程隐蔽管理器
pub struct ProcessStealthManager {
    config: Arc<RwLock<ProcessStealthConfig>>,
    original_title: Option<String>,
}

impl ProcessStealthManager {
    pub fn new(config: ProcessStealthConfig) -> Self {
        Self {
            config: Arc::new(RwLock::new(config)),
            original_title: None,
        }
    }

    /// 应用进程隐蔽
    pub async fn apply(&mut self) -> Result<(), String> {
        let config = self.config.read().await;
        if !config.enabled {
            return Ok(());
        }

        // 保存原始标题
        self.original_title = Some(self.get_current_title());

        // 设置伪装标题
        self.set_title(&config.disguise_title);

        log::info!("✅ 进程隐蔽已启用，标题伪装为: {}", config.disguise_title);
        Ok(())
    }

    /// 恢复原始标题
    pub async fn restore(&self) {
        if let Some(ref original) = self.original_title {
            self.set_title(original);
            log::info!("✅ 进程标题已恢复");
        }
    }

    /// 更新配置
    pub async fn update_config(&self, config: ProcessStealthConfig) {
        let mut cfg = self.config.write().await;
        *cfg = config;
    }

    /// 获取当前进程标题
    fn get_current_title(&self) -> String {
        // 默认返回空字符串，各平台实现可能不同
        String::new()
    }

    /// 设置进程标题
    fn set_title(&self, title: &str) {
        use windows::Win32::System::Console::SetConsoleTitleW;
        use windows::core::HSTRING;
        let _ = unsafe { SetConsoleTitleW(&HSTRING::from(title)) };
    }
}

// ── 端口隐蔽 ──────────────────────────────────────────────

/// 端口隐蔽配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortStealthConfig {
    /// 是否启用端口随机化
    pub enabled: bool,
    /// 端口范围（随机选取范围）
    pub port_range: (u16, u16),
    /// 避免使用的常见端口列表
    pub avoid_ports: Vec<u16>,
}

impl Default for PortStealthConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            port_range: (20000, 60000),
            avoid_ports: vec![
                // 代理常见端口
                7890, 7891, 7892, 7893, 1080, 8080, 8118, // SOCKS
                1080, 1081, 1082, // HTTP 代理
                3128, 8888, 9090, // Mihomo API
                9090, 9091, // SS
                8388, 8389, // 常见服务
                22, 80, 443, 3306, 5432, 6379, 27017,
            ],
        }
    }
}

/// 端口隐蔽管理器
pub struct PortStealthManager {
    config: Arc<RwLock<PortStealthConfig>>,
    current_port: Arc<RwLock<Option<u16>>>,
}

impl PortStealthManager {
    pub fn new(config: PortStealthConfig) -> Self {
        Self {
            config: Arc::new(RwLock::new(config)),
            current_port: Arc::new(RwLock::new(None)),
        }
    }

    /// 分配一个随机隐蔽端口
    pub async fn allocate_stealth_port(&self) -> Result<u16, String> {
        let (start, end, avoid) = {
            let config = self.config.read().await;
            if !config.enabled {
                return Err("端口隐蔽未启用".to_string());
            }
            (config.port_range.0, config.port_range.1, config.avoid_ports.clone())
        };

        // 预生成随机端口（ThreadRng 不是 Send，必须在 await 之前完成）
        let random_ports: Vec<u16> = {
            use rand::Rng;
            let mut rng = rand::thread_rng();
            (0..100).map(|_| rng.gen_range(start..=end)).collect()
        };

        for port in &random_ports {
            if avoid.contains(port) {
                continue;
            }
            if self.is_port_available(*port) {
                let mut current = self.current_port.write().await;
                *current = Some(*port);
                log::info!("✅ 端口隐蔽已分配随机端口: {}", port);
                return Ok(*port);
            }
        }

        // 随机失败则顺序扫描
        for port in start..=end {
            if avoid.contains(&port) {
                continue;
            }
            if self.is_port_available(port) {
                let mut current = self.current_port.write().await;
                *current = Some(port);
                log::info!("✅ 端口隐蔽已分配顺序端口: {}", port);
                return Ok(port);
            }
        }

        Err(format!("在范围 {}-{} 内无可用端口", start, end))
    }

    /// 获取当前分配的端口
    pub async fn get_current_port(&self) -> Option<u16> {
        *self.current_port.read().await
    }

    /// 更新配置
    pub async fn update_config(&self, config: PortStealthConfig) {
        let mut cfg = self.config.write().await;
        *cfg = config;
    }

    /// 检查端口是否可用
    fn is_port_available(&self, port: u16) -> bool {
        use std::net::{IpAddr, Ipv4Addr, SocketAddr, TcpListener};
        let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), port);
        TcpListener::bind(addr).is_ok()
    }
}

// ── 防本地发现 ──────────────────────────────────────────────

/// 防本地发现配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AntiDiscoveryConfig {
    /// 是否启用防本地发现
    pub enabled: bool,
    /// 禁用 mDNS
    pub disable_mdns: bool,
    /// 禁用 UPnP
    pub disable_upnp: bool,
    /// 禁用 LLMNR
    pub disable_llmnr: bool,
    /// 禁用 NetBIOS over TCP/IP (Windows)
    pub disable_netbios: bool,
    /// 禁用 SSDP (UPnP 发现协议)
    pub disable_ssdp: bool,
}

impl Default for AntiDiscoveryConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            disable_mdns: true,
            disable_upnp: true,
            disable_llmnr: true,
            disable_netbios: true,
            disable_ssdp: true,
        }
    }
}

/// 防本地发现管理器
pub struct AntiDiscoveryManager {
    config: Arc<RwLock<AntiDiscoveryConfig>>,
}

impl AntiDiscoveryManager {
    pub fn new(config: AntiDiscoveryConfig) -> Self {
        Self {
            config: Arc::new(RwLock::new(config)),
        }
    }

    /// 应用防本地发现策略
    pub async fn apply(&self) -> Result<Vec<String>, String> {
        let config = self.config.read().await;
        if !config.enabled {
            return Ok(vec![]);
        }

        let mut results = Vec::new();

        // 禁用 mDNS
        if config.disable_mdns {
            match self.disable_mdns_service().await {
                Ok(_) => results.push("mDNS 已禁用".to_string()),
                Err(e) => results.push(format!("mDNS 禁用失败: {}", e)),
            }
        }

        // 禁用 UPnP
        if config.disable_upnp {
            match self.disable_upnp_service().await {
                Ok(_) => results.push("UPnP 已禁用".to_string()),
                Err(e) => results.push(format!("UPnP 禁用失败: {}", e)),
            }
        }

        // 禁用 LLMNR
        if config.disable_llmnr {
            match self.disable_llmnr_service().await {
                Ok(_) => results.push("LLMNR 已禁用".to_string()),
                Err(e) => results.push(format!("LLMNR 禁用失败: {}", e)),
            }
        }

        // 禁用 NetBIOS
        if config.disable_netbios {
            match self.disable_netbios_service().await {
                Ok(_) => results.push("NetBIOS 已禁用".to_string()),
                Err(e) => results.push(format!("NetBIOS 禁用失败: {}", e)),
            }
        }

        // 禁用 SSDP
        if config.disable_ssdp {
            match self.disable_ssdp_service().await {
                Ok(_) => results.push("SSDP 已禁用".to_string()),
                Err(e) => results.push(format!("SSDP 禁用失败: {}", e)),
            }
        }

        log::info!("✅ 防本地发现策略已应用: {:?}", results);
        Ok(results)
    }

    /// 恢复本地发现服务
    pub async fn restore(&self) -> Result<Vec<String>, String> {
        let mut results = Vec::new();

        results.push(self.restore_mdns_service().await);
        results.push(self.restore_upnp_service().await);
        results.push(self.restore_llmnr_service().await);
        results.push(self.restore_netbios_service().await);
        results.push(self.restore_ssdp_service().await);

        log::info!("✅ 本地发现服务已恢复");
        Ok(results)
    }

    /// 更新配置
    pub async fn update_config(&self, config: AntiDiscoveryConfig) {
        let mut cfg = self.config.write().await;
        *cfg = config;
    }

    // ── Windows 实现 ──

    async fn disable_mdns_service(&self) -> Result<(), String> {
        // Windows: 停止 DNS-SD 服务（Bonjour mDNS）并禁用防火墙规则
        let output = hidden_command("netsh")
            .args(&[
                "advfirewall",
                "firewall",
                "add",
                "rule",
                "name=Block_mDNS_In",
                "dir=in",
                "action=block",
                "protocol=UDP",
                "localport=5353",
            ])
            .output()
            .map_err(|e| e.to_string())?;

        if !output.status.success() {
            return Err(String::from_utf8_lossy(&output.stderr).to_string());
        }

        let output = hidden_command("netsh")
            .args(&[
                "advfirewall",
                "firewall",
                "add",
                "rule",
                "name=Block_mDNS_Out",
                "dir=out",
                "action=block",
                "protocol=UDP",
                "localport=5353",
            ])
            .output()
            .map_err(|e| e.to_string())?;

        if !output.status.success() {
            return Err(String::from_utf8_lossy(&output.stderr).to_string());
        }

        Ok(())
    }

    async fn disable_upnp_service(&self) -> Result<(), String> {
        // Windows: 停止 UPnP Device Host 服务
        let output = hidden_command("net").args(&["stop", "upnphost"]).output();

        // 服务可能未运行，忽略错误
        if let Ok(output) = output {
            log::debug!("UPnP service stop result: {}", String::from_utf8_lossy(&output.stdout));
        }

        // 阻止 SSDP 端口 (1900)
        let output = hidden_command("netsh")
            .args(&[
                "advfirewall",
                "firewall",
                "add",
                "rule",
                "name=Block_SSDP_In",
                "dir=in",
                "action=block",
                "protocol=UDP",
                "localport=1900",
            ])
            .output()
            .map_err(|e| e.to_string())?;

        if !output.status.success() {
            return Err(String::from_utf8_lossy(&output.stderr).to_string());
        }

        Ok(())
    }

    async fn disable_llmnr_service(&self) -> Result<(), String> {
        // Windows: 通过注册表禁用 LLMNR
        // HKLM\SOFTWARE\Policies\Microsoft\Windows NT\DNSClient
        // EnableMulticast = 0
        let output = hidden_command("reg")
            .args(&[
                "add",
                r"HKLM\SOFTWARE\Policies\Microsoft\Windows NT\DNSClient",
                "/v",
                "EnableMulticast",
                "/t",
                "REG_DWORD",
                "/d",
                "0",
                "/f",
            ])
            .output()
            .map_err(|e| e.to_string())?;

        if !output.status.success() {
            return Err(String::from_utf8_lossy(&output.stderr).to_string());
        }

        Ok(())
    }

    async fn disable_netbios_service(&self) -> Result<(), String> {
        // Windows: 通过防火墙阻止 NetBIOS 端口 (137, 138, 139)
        for port in [137, 138, 139] {
            let output = hidden_command("netsh")
                .args(&[
                    "advfirewall",
                    "firewall",
                    "add",
                    "rule",
                    &format!("name=Block_NetBIOS_{}_In", port).to_string(),
                    "dir=in",
                    "action=block",
                    "protocol=UDP",
                    &format!("localport={}", port).to_string(),
                ])
                .output()
                .map_err(|e| e.to_string())?;

            if !output.status.success() {
                return Err(String::from_utf8_lossy(&output.stderr).to_string());
            }
        }

        // TCP 139
        let output = hidden_command("netsh")
            .args(&[
                "advfirewall",
                "firewall",
                "add",
                "rule",
                "name=Block_NetBIOS_139_TCP_In",
                "dir=in",
                "action=block",
                "protocol=TCP",
                "localport=139",
            ])
            .output()
            .map_err(|e| e.to_string())?;

        if !output.status.success() {
            return Err(String::from_utf8_lossy(&output.stderr).to_string());
        }

        Ok(())
    }

    async fn disable_ssdp_service(&self) -> Result<(), String> {
        // 阻止 SSDP 端口 (1900) 出站
        let output = hidden_command("netsh")
            .args(&[
                "advfirewall",
                "firewall",
                "add",
                "rule",
                "name=Block_SSDP_Out",
                "dir=out",
                "action=block",
                "protocol=UDP",
                "localport=1900",
            ])
            .output()
            .map_err(|e| e.to_string())?;

        if !output.status.success() {
            return Err(String::from_utf8_lossy(&output.stderr).to_string());
        }

        Ok(())
    }

    // ── 恢复方法 (Windows) ──

    async fn restore_mdns_service(&self) -> String {
        let output = hidden_command("netsh")
            .args(&["advfirewall", "firewall", "delete", "rule", "name=Block_mDNS_In"])
            .output();
        let output2 = hidden_command("netsh")
            .args(&["advfirewall", "firewall", "delete", "rule", "name=Block_mDNS_Out"])
            .output();
        match (output, output2) {
            (Ok(o1), Ok(o2)) if o1.status.success() && o2.status.success() => "mDNS 已恢复".to_string(),
            _ => "mDNS 恢复失败".to_string(),
        }
    }

    async fn restore_upnp_service(&self) -> String {
        let _ = hidden_command("netsh")
            .args(&["advfirewall", "firewall", "delete", "rule", "name=Block_SSDP_In"])
            .output();
        "UPnP 已恢复".to_string()
    }

    async fn restore_llmnr_service(&self) -> String {
        let output = hidden_command("reg")
            .args(&[
                "delete",
                r"HKLM\SOFTWARE\Policies\Microsoft\Windows NT\DNSClient",
                "/v",
                "EnableMulticast",
                "/f",
            ])
            .output();
        match output {
            Ok(o) if o.status.success() => "LLMNR 已恢复".to_string(),
            _ => "LLMNR 恢复失败".to_string(),
        }
    }

    async fn restore_netbios_service(&self) -> String {
        for port in [137, 138, 139] {
            let _ = hidden_command("netsh")
                .args(&[
                    "advfirewall",
                    "firewall",
                    "delete",
                    "rule",
                    &format!("name=Block_NetBIOS_{}_In", port).to_string(),
                ])
                .output();
        }
        let _ = hidden_command("netsh")
            .args(&[
                "advfirewall",
                "firewall",
                "delete",
                "rule",
                "name=Block_NetBIOS_139_TCP_In",
            ])
            .output();
        "NetBIOS 已恢复".to_string()
    }

    async fn restore_ssdp_service(&self) -> String {
        let _ = hidden_command("netsh")
            .args(&["advfirewall", "firewall", "delete", "rule", "name=Block_SSDP_Out"])
            .output();
        "SSDP 已恢复".to_string()
    }
}

// ── 本地隐蔽总管理器 ──────────────────────────────────────────

/// 本地隐蔽管理器（统一入口）
pub struct LocalStealthManager {
    config: Arc<RwLock<LocalStealthConfig>>,
    process_manager: ProcessStealthManager,
    port_manager: PortStealthManager,
    discovery_manager: AntiDiscoveryManager,
}

impl LocalStealthManager {
    pub fn new(config: LocalStealthConfig) -> Self {
        Self {
            process_manager: ProcessStealthManager::new(config.process_stealth.clone()),
            port_manager: PortStealthManager::new(config.port_stealth.clone()),
            discovery_manager: AntiDiscoveryManager::new(config.anti_discovery.clone()),
            config: Arc::new(RwLock::new(config)),
        }
    }

    /// 应用所有隐蔽策略
    pub async fn apply_all(&mut self) -> Result<StealthApplyResult, String> {
        let mut result = StealthApplyResult::default();

        // 1. 进程隐蔽
        if let Err(e) = self.process_manager.apply().await {
            result.errors.push(format!("进程隐蔽失败: {}", e));
        } else {
            result.process_stealth_applied = true;
        }

        // 2. 端口隐蔽
        if self.config.read().await.port_stealth.enabled {
            match self.port_manager.allocate_stealth_port().await {
                Ok(port) => {
                    result.port_stealth_applied = true;
                    result.allocated_port = Some(port);
                }
                Err(e) => result.errors.push(format!("端口隐蔽失败: {}", e)),
            }
        }

        // 3. 防本地发现
        match self.discovery_manager.apply().await {
            Ok(messages) => {
                result.anti_discovery_applied = true;
                result.discovery_messages = messages;
            }
            Err(e) => result.errors.push(format!("防本地发现失败: {}", e)),
        }

        if result.errors.is_empty() {
            log::info!("✅ 本地隐蔽策略全部应用成功");
        } else {
            log::warn!("⚠️ 本地隐蔽策略部分失败: {:?}", result.errors);
        }

        Ok(result)
    }

    /// 恢复所有隐蔽策略
    pub async fn restore_all(&self) {
        self.process_manager.restore().await;
        let _ = self.discovery_manager.restore().await;
        log::info!("✅ 本地隐蔽策略已全部恢复");
    }

    /// 更新配置
    pub async fn update_config(&mut self, config: LocalStealthConfig) {
        self.process_manager.update_config(config.process_stealth.clone()).await;
        self.port_manager.update_config(config.port_stealth.clone()).await;
        self.discovery_manager
            .update_config(config.anti_discovery.clone())
            .await;
        let mut cfg = self.config.write().await;
        *cfg = config;
    }

    /// 获取端口管理器引用（用于端口分配）
    pub fn port_manager(&self) -> &PortStealthManager {
        &self.port_manager
    }
}

/// 隐蔽策略应用结果
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct StealthApplyResult {
    /// 进程隐蔽是否应用成功
    pub process_stealth_applied: bool,
    /// 端口隐蔽是否应用成功
    pub port_stealth_applied: bool,
    /// 分配的隐蔽端口
    pub allocated_port: Option<u16>,
    /// 防本地发现是否应用成功
    pub anti_discovery_applied: bool,
    /// 防本地发现消息
    pub discovery_messages: Vec<String>,
    /// 错误信息
    pub errors: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = LocalStealthConfig::default();
        assert!(!config.process_stealth.enabled);
        assert!(!config.port_stealth.enabled);
        assert!(!config.anti_discovery.enabled);
    }

    #[test]
    fn test_port_stealth_avoid_list() {
        let config = PortStealthConfig::default();
        assert!(config.avoid_ports.contains(&7890));
        assert!(config.avoid_ports.contains(&9090));
        assert!(config.avoid_ports.contains(&1080));
    }

    #[test]
    fn test_port_stealth_range() {
        let config = PortStealthConfig::default();
        assert_eq!(config.port_range, (20000, 60000));
    }

    #[tokio::test]
    async fn test_port_allocation() {
        let config = PortStealthConfig {
            enabled: true,
            port_range: (50000, 50100),
            ..Default::default()
        };
        let manager = PortStealthManager::new(config);
        let port = manager.allocate_stealth_port().await;
        assert!(port.is_ok());
        let port_num = port.unwrap();
        assert!(port_num >= 50000 && port_num <= 50100);
    }

    #[test]
    fn test_anti_discovery_default() {
        let config = AntiDiscoveryConfig::default();
        assert!(config.disable_mdns);
        assert!(config.disable_upnp);
        assert!(config.disable_llmnr);
        assert!(config.disable_netbios);
        assert!(config.disable_ssdp);
    }
}
