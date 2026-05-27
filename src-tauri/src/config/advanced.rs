/**
 * 高级功能配置
 * 
 * 统一管理所有高级功能的配置
 */

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use anyhow::Result;

use crate::anti_probe::AntiProbeConfig;
use crate::multipath::MultipathConfig;
#[cfg(target_os = "linux")]
use crate::xdp::XdpConfig;

/// 高级功能配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdvancedConfig {
    /// 安全防御配置
    #[serde(default)]
    pub security: SecurityConfig,
    
    /// 多路径路由配置
    #[serde(default)]
    pub multipath: MultipathConfig,
    
    /// XDP 代理配置（仅 Linux）
    #[cfg(target_os = "linux")]
    #[serde(default)]
    pub xdp: XdpConfig,
}

impl Default for AdvancedConfig {
    fn default() -> Self {
        Self {
            security: SecurityConfig::default(),
            multipath: MultipathConfig::default(),
            #[cfg(target_os = "linux")]
            xdp: XdpConfig::default(),
        }
    }
}

/// 安全防御配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityConfig {
    /// 启用安全监控
    #[serde(default)]
    pub enabled: bool,
    
    /// 反主动探测配置
    #[serde(default)]
    pub anti_probe: AntiProbeConfig,
    
    /// TLS 指纹名称
    #[serde(default)]
    pub tls_fingerprint: Option<String>,
    
    /// 配置欺骗配置
    #[serde(default)]
    pub config_decoy: ConfigDecoyConfig,
}

impl Default for SecurityConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            anti_probe: AntiProbeConfig::default(),
            tls_fingerprint: None,
            config_decoy: ConfigDecoyConfig::default(),
        }
    }
}

/// 配置欺骗配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigDecoyConfig {
    /// 启用配置欺骗
    #[serde(default)]
    pub enabled: bool,
    
    /// 假配置文件路径
    #[serde(default)]
    pub decoy_path: Option<String>,
    
    /// 加密密钥（从环境变量加载）
    #[serde(skip)]
    pub encryption_key: Option<String>,
}

impl Default for ConfigDecoyConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            decoy_path: Some("config_decoy.yaml".to_string()),
            encryption_key: None,
        }
    }
}

use super::ConfigFile;

// 实现 ConfigFile trait
impl ConfigFile for AdvancedConfig {}

impl AdvancedConfig {
    /// 从文件加载配置（使用 trait 默认实现）
    pub fn load(path: &PathBuf) -> Result<Self> {
        Self::load_from_file(path)
    }

    /// 保存配置到文件（使用 trait 默认实现）
    pub fn save(&self, path: &PathBuf) -> Result<()> {
        self.save_to_file(path)
    }

    /// 验证配置
    pub fn validate(&self) -> Result<()> {
        // 验证反探测配置
        if self.security.anti_probe.enabled {
            if self.security.anti_probe.secret_key.is_empty() {
                return Err(anyhow::anyhow!("反探测密钥不能为空"));
            }
            if self.security.anti_probe.time_window == 0 {
                return Err(anyhow::anyhow!("时间窗口必须大于 0"));
            }
        }

        // 验证多路径配置
        if self.multipath.enabled {
            if self.multipath.node_pools.is_empty() {
                return Err(anyhow::anyhow!("多路径路由需要至少一个节点池"));
            }
            
            for pool in &self.multipath.node_pools {
                if pool.enabled && pool.nodes.is_empty() {
                    return Err(anyhow::anyhow!("节点池 '{}' 没有节点", pool.name));
                }
            }
        }

        // 验证 XDP 配置（Linux）
        #[cfg(target_os = "linux")]
        if self.xdp.enabled {
            if self.xdp.interface.is_empty() {
                return Err(anyhow::anyhow!("XDP 接口不能为空"));
            }
        }

        Ok(())
    }

    /// 合并配置（用于部分更新）
    pub fn merge(&mut self, other: &Self) {
        // 合并安全配置
        if other.security.enabled {
            self.security = other.security.clone();
        }

        // 合并多路径配置
        if other.multipath.enabled {
            self.multipath = other.multipath.clone();
        }

        // 合并 XDP 配置（Linux）
        #[cfg(target_os = "linux")]
        if other.xdp.enabled {
            self.xdp = other.xdp.clone();
        }
    }
}

/// 配置示例生成器
impl AdvancedConfig {
    /// 生成推荐配置
    pub fn recommended() -> Self {
        use crate::multipath::{NodePool, PoolType, SlicingStrategy};

        Self {
            security: SecurityConfig {
                enabled: true,
                anti_probe: AntiProbeConfig {
                    enabled: true,
                    secret_key: "auto-generated".to_string(),
                    time_window: 300,
                    whitelist: Vec::new(),
                    strict_mode: false,
                },
                tls_fingerprint: Some("Chrome 120 (Windows)".to_string()),
                config_decoy: ConfigDecoyConfig {
                    enabled: true,
                    decoy_path: Some("config_decoy.yaml".to_string()),
                    encryption_key: None,
                },
            },
            multipath: MultipathConfig {
                enabled: true,
                strategy: SlicingStrategy::Weighted,
                node_pools: vec![
                    NodePool {
                        name: "通用池".to_string(),
                        pool_type: PoolType::General,
                        nodes: Vec::new(),
                        enabled: true,
                    },
                    NodePool {
                        name: "流媒体专用".to_string(),
                        pool_type: PoolType::Streaming,
                        nodes: Vec::new(),
                        enabled: true,
                    },
                ],
                min_fragment_size: 1024,
                max_fragment_size: 65536,
                reassembly_timeout: 5000,
                session_persistence: true,
            },
            #[cfg(target_os = "linux")]
            xdp: XdpConfig {
                enabled: false,
                interface: "eth0".to_string(),
                mode: crate::xdp::XdpMode::Skb,
                queue_size: 4096,
            },
        }
    }

    /// 生成最小配置（所有功能关闭）
    pub fn minimal() -> Self {
        Self::default()
    }

    /// 生成最大安全配置
    pub fn maximum_security() -> Self {
        let mut config = Self::recommended();
        config.security.anti_probe.strict_mode = true;
        config.security.config_decoy.enabled = true;
        config
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = AdvancedConfig::default();
        assert!(!config.security.enabled);
        assert!(!config.multipath.enabled);
    }

    #[test]
    fn test_recommended_config() {
        let config = AdvancedConfig::recommended();
        assert!(config.security.enabled);
        assert!(config.multipath.enabled);
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_config_validation() {
        let mut config = AdvancedConfig::default();
        config.multipath.enabled = true;
        // 没有节点池，应该验证失败
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_config_merge() {
        let mut config1 = AdvancedConfig::default();
        let config2 = AdvancedConfig::recommended();
        
        config1.merge(&config2);
        assert!(config1.security.enabled);
        assert!(config1.multipath.enabled);
    }
}
