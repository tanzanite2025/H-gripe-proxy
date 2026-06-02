/**
 * 防火墙管理模块
 *
 * 功能：
 * 1. Windows 防火墙配置 - 使用 PowerShell
 * 2. Linux 防火墙配置 - 使用 iptables
 * 3. macOS 防火墙配置 - 使用 pf
 */
use anyhow::{Result, anyhow};
use serde::{Deserialize, Serialize};
#[cfg(any(target_os = "linux", target_os = "macos"))]
use std::process::Command;
use std::sync::Arc;
use tokio::sync::RwLock;

use super::local_security::LocalSecurityConfig;
#[cfg(target_os = "windows")]
use crate::utils::command::hidden_command;

/// 防火墙规则
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FirewallRule {
    pub name: String,
    pub port: u16,
    pub protocol: Protocol,
    pub action: Action,
}

/// 协议类型
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Protocol {
    TCP,
    UDP,
}

impl Protocol {
    pub fn as_str(&self) -> &str {
        match self {
            Protocol::TCP => "TCP",
            Protocol::UDP => "UDP",
        }
    }
}

/// 动作类型
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Action {
    Allow,
    Block,
}

impl Action {
    pub fn as_str(&self) -> &str {
        match self {
            Action::Allow => "Allow",
            Action::Block => "Block",
        }
    }
}

/// 防火墙管理器
pub struct FirewallManager {
    config: Arc<RwLock<LocalSecurityConfig>>,
}

impl FirewallManager {
    /// 创建新的防火墙管理器
    pub fn new(config: LocalSecurityConfig) -> Self {
        Self {
            config: Arc::new(RwLock::new(config)),
        }
    }

    /// 配置防火墙规则
    ///
    /// 为指定端口配置防火墙规则：
    /// - 允许本地访问（127.0.0.1）
    /// - 阻止外部访问
    pub async fn configure_firewall(&self, port: u16) -> Result<()> {
        let cfg = self.config.read().await;
        log::info!(
            "Configuring firewall rules for port {} (auto_firewall={}, bind={})",
            port,
            cfg.auto_firewall,
            cfg.bind_address
        );

        // 检查权限
        if !self.check_permissions().await? {
            return Err(anyhow!(
                "Insufficient permissions to configure firewall. Please run as administrator/root."
            ));
        }

        // 先删除旧规则
        let _ = self.remove_firewall_rules(port).await;

        // 根据平台配置防火墙（示例规则，使用 TCP/Allow 语义）
        let allow_rule = FirewallRule {
            name: format!("ClashVerge-Allow-{}", port),
            port,
            protocol: Protocol::TCP,
            action: Action::Allow,
        };
        log::debug!(
            "Applying firewall rule: {} {} {}",
            allow_rule.name,
            allow_rule.protocol.as_str(),
            allow_rule.action.as_str()
        );

        // 根据平台配置防火墙
        #[cfg(target_os = "windows")]
        self.configure_windows_firewall(port).await?;

        #[cfg(target_os = "linux")]
        self.configure_linux_firewall(port).await?;

        #[cfg(target_os = "macos")]
        self.configure_macos_firewall(port).await?;

        log::info!("Firewall rules configured successfully for port {}", port);
        Ok(())
    }

    /// 删除防火墙规则
    pub async fn remove_firewall_rules(&self, port: u16) -> Result<()> {
        log::info!("Removing firewall rules for port {}", port);

        #[cfg(target_os = "windows")]
        self.remove_windows_firewall_rules(port).await?;

        #[cfg(target_os = "linux")]
        self.remove_linux_firewall_rules(port).await?;

        #[cfg(target_os = "macos")]
        self.remove_macos_firewall_rules(port).await?;

        log::info!("Firewall rules removed successfully for port {}", port);
        Ok(())
    }

    /// 检查防火墙规则是否生效
    pub async fn check_firewall_rules(&self, port: u16) -> Result<bool> {
        #[cfg(target_os = "windows")]
        return self.check_windows_firewall_rules(port).await;

        #[cfg(target_os = "linux")]
        return self.check_linux_firewall_rules(port).await;

        #[cfg(target_os = "macos")]
        return self.check_macos_firewall_rules(port).await;
    }

    /// 检查是否有足够的权限配置防火墙
    async fn check_permissions(&self) -> Result<bool> {
        #[cfg(target_os = "windows")]
        {
            // Windows: 检查是否以管理员身份运行
            let output = hidden_command("net").args(&["session"]).output()?;
            Ok(output.status.success())
        }

        #[cfg(target_os = "linux")]
        {
            // Linux: 检查是否为 root 或有 sudo 权限
            let output = Command::new("id").args(&["-u"]).output()?;
            let uid = String::from_utf8_lossy(&output.stdout).trim().to_string();
            Ok(uid == "0")
        }

        #[cfg(target_os = "macos")]
        {
            // macOS: 检查是否为 root 或有 sudo 权限
            let output = Command::new("id").args(&["-u"]).output()?;
            let uid = String::from_utf8_lossy(&output.stdout).trim().to_string();
            Ok(uid == "0")
        }
    }

    // ==================== Windows 实现 ====================

    #[cfg(target_os = "windows")]
    async fn configure_windows_firewall(&self, port: u16) -> Result<()> {
        let rule_name = format!("ClashVerge-LocalOnly-{}", port);
        let rule_name_block = format!("ClashVerge-LocalOnly-{}-Block", port);

        // 添加允许规则：允许本地访问
        let allow_cmd = format!(
            "New-NetFirewallRule -DisplayName '{}' -Direction Inbound -LocalAddress 127.0.0.1 -LocalPort {} -Protocol TCP -Action Allow -Profile Any",
            rule_name, port
        );

        let output = hidden_command("powershell")
            .args(&["-Command", &allow_cmd])
            .output()
            .map_err(|e| anyhow!("Failed to execute PowerShell: {}", e))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow!("Failed to create allow rule: {}", stderr));
        }

        // 添加阻止规则：阻止外部访问
        let block_cmd = format!(
            "New-NetFirewallRule -DisplayName '{}' -Direction Inbound -LocalPort {} -Protocol TCP -Action Block -RemoteAddress Any -Profile Any",
            rule_name_block, port
        );

        let output = hidden_command("powershell")
            .args(&["-Command", &block_cmd])
            .output()
            .map_err(|e| anyhow!("Failed to execute PowerShell: {}", e))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            log::warn!("Failed to create block rule (may already exist): {}", stderr);
        }

        Ok(())
    }

    #[cfg(target_os = "windows")]
    async fn remove_windows_firewall_rules(&self, port: u16) -> Result<()> {
        let rule_name = format!("ClashVerge-LocalOnly-{}", port);
        let rule_name_block = format!("ClashVerge-LocalOnly-{}-Block", port);

        // 删除允许规则
        let remove_allow_cmd = format!(
            "Remove-NetFirewallRule -DisplayName '{}' -ErrorAction SilentlyContinue",
            rule_name
        );

        let _ = hidden_command("powershell")
            .args(&["-Command", &remove_allow_cmd])
            .output();

        // 删除阻止规则
        let remove_block_cmd = format!(
            "Remove-NetFirewallRule -DisplayName '{}' -ErrorAction SilentlyContinue",
            rule_name_block
        );

        let _ = hidden_command("powershell")
            .args(&["-Command", &remove_block_cmd])
            .output();

        Ok(())
    }

    #[cfg(target_os = "windows")]
    async fn check_windows_firewall_rules(&self, port: u16) -> Result<bool> {
        let rule_name = format!("ClashVerge-LocalOnly-{}", port);

        let check_cmd = format!(
            "Get-NetFirewallRule -DisplayName '{}' -ErrorAction SilentlyContinue",
            rule_name
        );

        let output = hidden_command("powershell")
            .args(&["-Command", &check_cmd])
            .output()
            .map_err(|e| anyhow!("Failed to execute PowerShell: {}", e))?;

        Ok(output.status.success() && !output.stdout.is_empty())
    }

    // ==================== Linux 实现 ====================

    #[cfg(target_os = "linux")]
    async fn configure_linux_firewall(&self, port: u16) -> Result<()> {
        // 允许回环接口
        let allow_loopback = Command::new("iptables")
            .args(&["-A", "INPUT", "-i", "lo", "-j", "ACCEPT"])
            .output()
            .map_err(|e| anyhow!("Failed to execute iptables: {}", e))?;

        if !allow_loopback.status.success() {
            let stderr = String::from_utf8_lossy(&allow_loopback.stderr);
            log::warn!("Failed to add loopback rule (may already exist): {}", stderr);
        }

        // 阻止外部访问指定端口
        let block_external = Command::new("iptables")
            .args(&[
                "-A",
                "INPUT",
                "-p",
                "tcp",
                "--dport",
                &port.to_string(),
                "!",
                "-i",
                "lo",
                "-j",
                "DROP",
            ])
            .output()
            .map_err(|e| anyhow!("Failed to execute iptables: {}", e))?;

        if !block_external.status.success() {
            let stderr = String::from_utf8_lossy(&block_external.stderr);
            return Err(anyhow!("Failed to add block rule: {}", stderr));
        }

        // 保存规则（Debian/Ubuntu）
        let _ = Command::new("iptables-save").output();

        Ok(())
    }

    #[cfg(target_os = "linux")]
    async fn remove_linux_firewall_rules(&self, port: u16) -> Result<()> {
        // 删除阻止规则
        let _ = Command::new("iptables")
            .args(&[
                "-D",
                "INPUT",
                "-p",
                "tcp",
                "--dport",
                &port.to_string(),
                "!",
                "-i",
                "lo",
                "-j",
                "DROP",
            ])
            .output();

        Ok(())
    }

    #[cfg(target_os = "linux")]
    async fn check_linux_firewall_rules(&self, port: u16) -> Result<bool> {
        let output = Command::new("iptables")
            .args(&["-L", "INPUT", "-n"])
            .output()
            .map_err(|e| anyhow!("Failed to execute iptables: {}", e))?;

        let stdout = String::from_utf8_lossy(&output.stdout);

        // 检查是否存在针对该端口的规则
        Ok(stdout.contains(&format!("dpt:{}", port)))
    }

    // ==================== macOS 实现 ====================

    #[cfg(target_os = "macos")]
    async fn configure_macos_firewall(&self, port: u16) -> Result<()> {
        let rules = format!(
            "# ClashVerge firewall rules\n\
             block in proto tcp from any to any port {}\n\
             pass in proto tcp from 127.0.0.1 to 127.0.0.1 port {}",
            port, port
        );

        // 写入规则文件
        std::fs::write("/etc/pf.anchors/clash_verge", rules).map_err(|e| anyhow!("Failed to write pf rules: {}", e))?;

        // 加载规则
        let output = Command::new("pfctl")
            .args(&["-f", "/etc/pf.anchors/clash_verge"])
            .output()
            .map_err(|e| anyhow!("Failed to execute pfctl: {}", e))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow!("Failed to load pf rules: {}", stderr));
        }

        // 启用 pf
        let _ = Command::new("pfctl").args(&["-e"]).output();

        Ok(())
    }

    #[cfg(target_os = "macos")]
    async fn remove_macos_firewall_rules(&self, _port: u16) -> Result<()> {
        // 删除规则文件
        let _ = std::fs::remove_file("/etc/pf.anchors/clash_verge");

        // 重新加载 pf 配置
        let _ = Command::new("pfctl").args(&["-f", "/etc/pf.conf"]).output();

        Ok(())
    }

    #[cfg(target_os = "macos")]
    async fn check_macos_firewall_rules(&self, port: u16) -> Result<bool> {
        // 检查规则文件是否存在
        if !std::path::Path::new("/etc/pf.anchors/clash_verge").exists() {
            return Ok(false);
        }

        // 读取规则文件
        let content = std::fs::read_to_string("/etc/pf.anchors/clash_verge")
            .map_err(|e| anyhow!("Failed to read pf rules: {}", e))?;

        // 检查是否包含该端口的规则
        Ok(content.contains(&format!("port {}", port)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_firewall_manager_creation() {
        let config = LocalSecurityConfig::default();
        let manager = FirewallManager::new(config);

        // 验证管理器创建成功
        assert!(manager.config.read().await.bind_address == "127.0.0.1");
    }

    #[tokio::test]
    async fn test_protocol_as_str() {
        assert_eq!(Protocol::TCP.as_str(), "TCP");
        assert_eq!(Protocol::UDP.as_str(), "UDP");
    }

    #[tokio::test]
    async fn test_action_as_str() {
        assert_eq!(Action::Allow.as_str(), "Allow");
        assert_eq!(Action::Block.as_str(), "Block");
    }

    #[tokio::test]
    async fn test_firewall_rule_creation() {
        let rule = FirewallRule {
            name: "test-rule".to_string(),
            port: 8080,
            protocol: Protocol::TCP,
            action: Action::Allow,
        };

        assert_eq!(rule.name, "test-rule");
        assert_eq!(rule.port, 8080);
        assert_eq!(rule.protocol.as_str(), "TCP");
        assert_eq!(rule.action.as_str(), "Allow");
    }

    #[tokio::test]
    async fn test_check_permissions() {
        let config = LocalSecurityConfig::default();
        let manager = FirewallManager::new(config);

        // 检查权限（可能失败，取决于运行环境）
        let result = manager.check_permissions().await;
        assert!(result.is_ok());
    }

    // 注意：以下测试需要管理员/root权限，在CI环境中可能失败

    #[tokio::test]
    #[ignore] // 需要管理员权限
    async fn test_configure_firewall() {
        let config = LocalSecurityConfig::default();
        let manager = FirewallManager::new(config);

        let port = 65500;
        let result = manager.configure_firewall(port).await;

        // 如果有权限，应该成功
        if manager.check_permissions().await.unwrap_or(false) {
            assert!(result.is_ok());

            // 清理
            let _ = manager.remove_firewall_rules(port).await;
        }
    }

    #[tokio::test]
    #[ignore] // 需要管理员权限
    async fn test_check_firewall_rules() {
        let config = LocalSecurityConfig::default();
        let manager = FirewallManager::new(config);

        let port = 65501;

        // 配置规则
        if manager.check_permissions().await.unwrap_or(false) {
            let _ = manager.configure_firewall(port).await;

            // 检查规则
            let exists = manager.check_firewall_rules(port).await.unwrap_or(false);
            assert!(exists);

            // 清理
            let _ = manager.remove_firewall_rules(port).await;
        }
    }

    #[tokio::test]
    #[ignore] // 需要管理员权限
    async fn test_remove_firewall_rules() {
        let config = LocalSecurityConfig::default();
        let manager = FirewallManager::new(config);

        let port = 65502;

        if manager.check_permissions().await.unwrap_or(false) {
            // 配置规则
            let _ = manager.configure_firewall(port).await;

            // 删除规则
            let result = manager.remove_firewall_rules(port).await;
            assert!(result.is_ok());

            // 验证规则已删除
            let exists = manager.check_firewall_rules(port).await.unwrap_or(true);
            assert!(!exists);
        }
    }
}
