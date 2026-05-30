/**
 * 高级功能配置
 * 
 * 统一管理所有高级功能的配置
 */

use serde::{Deserialize, Serialize};
use serde_yaml_ng::{Mapping, Value};
use std::path::PathBuf;
use anyhow::Result;

use super::ConfigFile;
use crate::anti_probe::AntiProbeConfig;
use crate::core::egress_identity::EgressIdentityConfig;
use crate::core::egress_monitor::EgressMonitorConfig;
use crate::core::session_affinity::SessionAffinityConfig;
use crate::multipath::MultipathConfig;
use crate::traffic::{TrafficPaddingConfig, TrafficObfuscationConfig};
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

    /// 会话绑定配置
    #[serde(default)]
    pub session_affinity: SessionAffinityConfig,

    #[serde(default)]
    pub egress_identity: EgressIdentityConfig,

    #[serde(default)]
    pub dns: DnsAdvancedConfig,

    /// 出口 IP 监控配置
    #[serde(default)]
    pub egress_monitor: EgressMonitorConfig,

    /// 流量混淆配置（新）
    #[serde(default)]
    pub traffic_obfuscation: TrafficObfuscationConfig,

    /// 流量填充配置（旧，保留兼容）
    #[serde(default)]
    pub traffic_padding: TrafficPaddingConfig,

    /// 多路复用配置（smux + brutal，推荐但默认关闭）
    #[serde(default)]
    pub multiplex: MultiplexConfig,

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
            session_affinity: SessionAffinityConfig::default(),
            egress_identity: EgressIdentityConfig::default(),
            dns: DnsAdvancedConfig::default(),
            egress_monitor: EgressMonitorConfig::default(),
            traffic_obfuscation: TrafficObfuscationConfig::default(),
            traffic_padding: TrafficPaddingConfig::default(),
            multiplex: MultiplexConfig::default(),
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

    /// 自毁配置
    #[serde(default)]
    pub self_destruct: SelfDestructConfig,

    /// 内存蜜罐配置
    #[serde(default)]
    pub honeypot: HoneypotConfig,
}

impl Default for SecurityConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            anti_probe: AntiProbeConfig::default(),
            tls_fingerprint: None,
            config_decoy: ConfigDecoyConfig::default(),
            self_destruct: SelfDestructConfig::default(),
            honeypot: HoneypotConfig::default(),
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

/// 自毁配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SelfDestructConfig {
    /// 启用自毁机制
    #[serde(default)]
    pub enabled: bool,
    /// 是否清除内存中的密钥
    #[serde(default = "default_true")]
    pub clear_memory: bool,
    /// 是否删除配置文件
    #[serde(default)]
    pub delete_configs: bool,
    /// 是否删除日志文件
    #[serde(default = "default_true")]
    pub delete_logs: bool,
    /// 是否立即退出程序
    #[serde(default = "default_true")]
    pub exit_immediately: bool,
}

impl Default for SelfDestructConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            clear_memory: true,
            delete_configs: false,
            delete_logs: true,
            exit_immediately: true,
        }
    }
}

/// 内存蜜罐配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HoneypotConfig {
    /// 启用内存蜜罐
    #[serde(default)]
    pub enabled: bool,
    /// 蜜罐令牌数量
    #[serde(default = "default_honeypot_token_count")]
    pub token_count: usize,
    /// 监控间隔（秒）
    #[serde(default = "default_honeypot_interval")]
    pub monitor_interval_secs: u64,
}

impl Default for HoneypotConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            token_count: 10,
            monitor_interval_secs: 2,
        }
    }
}

fn default_honeypot_token_count() -> usize { 10 }
fn default_honeypot_interval() -> u64 { 2 }
fn default_true() -> bool { true }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DnsAdvancedConfig {
    #[serde(default = "dns_bool_default_true")]
    pub enable_cache: bool,
    #[serde(default = "dns_bool_default_true")]
    pub enable_prefetch: bool,
    #[serde(default = "dns_bool_default_true")]
    pub enable_health_check: bool,
    #[serde(default = "dns_prefetch_interval_default")]
    pub prefetch_interval: u64,
    #[serde(default = "dns_health_check_interval_default")]
    pub health_check_interval: u64,
    #[serde(default)]
    pub routing_mode: DnsRoutingMode,
    #[serde(default)]
    pub leak_protection_level: DnsLeakProtectionLevel,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum DnsRoutingMode {
    Speed,
    Privacy,
    Balanced,
    Custom,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum DnsLeakProtectionLevel {
    None,
    Basic,
    Strict,
    Paranoid,
}

fn dns_bool_default_true() -> bool {
    true
}

fn dns_prefetch_interval_default() -> u64 {
    300_000
}

fn dns_health_check_interval_default() -> u64 {
    60_000
}

impl Default for DnsRoutingMode {
    fn default() -> Self {
        Self::Balanced
    }
}

impl Default for DnsLeakProtectionLevel {
    fn default() -> Self {
        Self::Basic
    }
}

impl Default for DnsAdvancedConfig {
    fn default() -> Self {
        Self {
            enable_cache: true,
            enable_prefetch: true,
            enable_health_check: true,
            prefetch_interval: dns_prefetch_interval_default(),
            health_check_interval: dns_health_check_interval_default(),
            routing_mode: DnsRoutingMode::default(),
            leak_protection_level: DnsLeakProtectionLevel::default(),
        }
    }
}

impl DnsAdvancedConfig {
    pub fn validate(&self) -> Result<()> {
        if self.prefetch_interval == 0 {
            return Err(anyhow::anyhow!("DNS 预解析间隔必须大于 0"));
        }

        if self.health_check_interval == 0 {
            return Err(anyhow::anyhow!("DNS 健康检查间隔必须大于 0"));
        }

        Ok(())
    }

    pub fn to_dns_config_mapping(&self) -> Mapping {
        let using_fake_ip = matches!(
            self.leak_protection_level,
            DnsLeakProtectionLevel::Strict | DnsLeakProtectionLevel::Paranoid
        );
        let force_doh = matches!(
            self.leak_protection_level,
            DnsLeakProtectionLevel::Basic
                | DnsLeakProtectionLevel::Strict
                | DnsLeakProtectionLevel::Paranoid
        );
        let block_plain_dns = matches!(
            self.leak_protection_level,
            DnsLeakProtectionLevel::Strict | DnsLeakProtectionLevel::Paranoid
        );
        let block_ipv6_dns = matches!(self.leak_protection_level, DnsLeakProtectionLevel::Paranoid);

        let domestic_plain = vec!["223.5.5.5", "119.29.29.29"];
        let domestic_doh = vec![
            "https://dns.alidns.com/dns-query",
            "https://doh.pub/dns-query",
        ];
        let foreign_plain = vec!["1.1.1.1", "8.8.8.8", "9.9.9.9"];
        let foreign_doh = vec![
            "https://dns.google/dns-query",
            "https://cloudflare-dns.com/dns-query",
            "https://dns.quad9.net/dns-query",
        ];

        let effective_mode = match self.routing_mode {
            DnsRoutingMode::Custom => DnsRoutingMode::Balanced,
            _ => self.routing_mode.clone(),
        };

        let (cn_servers, foreign_servers) = match effective_mode {
            DnsRoutingMode::Speed => {
                if force_doh {
                    (domestic_doh.clone(), domestic_doh.clone())
                } else {
                    (domestic_plain.clone(), domestic_plain.clone())
                }
            }
            DnsRoutingMode::Privacy => (foreign_doh.clone(), foreign_doh.clone()),
            DnsRoutingMode::Balanced | DnsRoutingMode::Custom => {
                if force_doh {
                    (domestic_doh.clone(), foreign_doh.clone())
                } else {
                    (domestic_plain.clone(), foreign_doh.clone())
                }
            }
        };

        let mut nameserver = Vec::<String>::new();
        for server in cn_servers.iter().chain(foreign_servers.iter()) {
            if !nameserver.iter().any(|existing| existing == server) {
                nameserver.push((*server).to_string());
            }
        }

        let fallback = if force_doh {
            foreign_doh
                .iter()
                .map(|item| (*item).to_string())
                .collect::<Vec<String>>()
        } else {
            foreign_plain
                .iter()
                .map(|item| (*item).to_string())
                .collect::<Vec<String>>()
        };

        let mut dns_mapping = Mapping::new();
        dns_mapping.insert("enable".into(), Value::Bool(true));
        dns_mapping.insert("listen".into(), Value::String(":53".into()));
        dns_mapping.insert("respect-rules".into(), Value::Bool(true));
        dns_mapping.insert("use-hosts".into(), Value::Bool(true));
        dns_mapping.insert("use-system-hosts".into(), Value::Bool(true));
        dns_mapping.insert("prefer-h3".into(), Value::Bool(force_doh));
        dns_mapping.insert("ipv6".into(), Value::Bool(!block_ipv6_dns));
        dns_mapping.insert(
            "enhanced-mode".into(),
            Value::String(if using_fake_ip {
                "fake-ip".into()
            } else {
                "redir-host".into()
            }),
        );

        if using_fake_ip {
            dns_mapping.insert("fake-ip-range".into(), Value::String("198.18.0.1/16".into()));
            dns_mapping.insert(
                "fake-ip-filter".into(),
                Value::Sequence(
                    [
                        "*.lan",
                        "localhost.ptlogin2.qq.com",
                        "+.stun.*.*",
                        "+.stun.*.*.*",
                        "+.stun.*.*.*.*",
                        "+.stun.*.*.*.*.*",
                        "*.n.n.srv.nintendo.net",
                        "+.stun.playstation.net",
                        "xbox.*.*.microsoft.com",
                        "*.*.xboxlive.com",
                        "*.msftncsi.com",
                        "*.msftconnecttest.com",
                        "WORKGROUP",
                    ]
                    .into_iter()
                    .map(Value::from)
                    .collect(),
                ),
            );
        }

        dns_mapping.insert(
            "default-nameserver".into(),
            Value::Sequence(
                if block_plain_dns {
                    Vec::<Value>::new()
                } else {
                    domestic_plain.into_iter().map(Value::from).collect()
                },
            ),
        );
        dns_mapping.insert(
            "nameserver".into(),
            Value::Sequence(nameserver.iter().map(|item| Value::from(item.as_str())).collect()),
        );
        dns_mapping.insert(
            "fallback".into(),
            Value::Sequence(fallback.iter().map(|item| Value::from(item.as_str())).collect()),
        );

        let mut fallback_filter = Mapping::new();
        fallback_filter.insert("geoip".into(), Value::Bool(true));
        fallback_filter.insert("geoip-code".into(), Value::String("CN".into()));
        fallback_filter.insert(
            "ipcidr".into(),
            Value::Sequence(["240.0.0.0/4", "0.0.0.0/32"].into_iter().map(Value::from).collect()),
        );
        fallback_filter.insert(
            "domain".into(),
            Value::Sequence(
                [
                    "+.google.com",
                    "+.facebook.com",
                    "+.youtube.com",
                    "+.twitter.com",
                    "+.github.com",
                ]
                .into_iter()
                .map(Value::from)
                .collect(),
            ),
        );
        dns_mapping.insert("fallback-filter".into(), Value::Mapping(fallback_filter));

        let mut nameserver_policy = Mapping::new();
        nameserver_policy.insert(
            "geosite:cn".into(),
            Value::Sequence(cn_servers.iter().map(|item| Value::from(*item)).collect()),
        );
        nameserver_policy.insert(
            "geosite:geolocation-!cn".into(),
            Value::Sequence(foreign_servers.iter().map(|item| Value::from(*item)).collect()),
        );
        dns_mapping.insert("nameserver-policy".into(), Value::Mapping(nameserver_policy));

        let mut root = Mapping::new();
        root.insert("dns".into(), Value::Mapping(dns_mapping));
        root.insert("hosts".into(), Value::Mapping(Mapping::new()));
        root
    }
}

/// 多路复用配置（smux + brutal，推荐但默认关闭）
/// 需要服务端支持，开启后弱网环境下可显著提升连接稳定性和吞吐
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MultiplexConfig {
    /// 是否启用 smux 多路复用注入
    #[serde(default)]
    pub enabled: bool,
    /// 多路复用协议：h2mux / smux / yamux
    #[serde(default = "MultiplexConfig::default_protocol")]
    pub protocol: String,
    /// 最大连接数（与 max-streams 冲突）
    #[serde(default = "MultiplexConfig::default_max_connections")]
    pub max_connections: u32,
    /// 最小流数量
    #[serde(default = "MultiplexConfig::default_min_streams")]
    pub min_streams: u32,
    /// 最大流数量（与 max-connections / min-streams 冲突）
    #[serde(default)]
    pub max_streams: Option<u32>,
    /// 是否在面板显示底层连接统计
    #[serde(default)]
    pub statistic: bool,
    /// 仅 TCP 走多路复用，UDP 直连
    #[serde(default)]
    pub only_tcp: bool,
    /// 启用填充
    #[serde(default)]
    pub padding: bool,
    /// TCP Brutal 拥塞控制（需服务端支持）
    #[serde(default)]
    pub brutal: BrutalConfig,
}

impl MultiplexConfig {
    fn default_protocol() -> String { "h2mux".to_string() }
    fn default_max_connections() -> u32 { 4 }
    fn default_min_streams() -> u32 { 4 }

    /// 推荐配置（默认关闭，用户主动开启时使用）
    pub fn recommended() -> Self {
        Self {
            enabled: false,
            protocol: "h2mux".to_string(),
            max_connections: 4,
            min_streams: 4,
            max_streams: None,
            statistic: true,
            only_tcp: false,
            padding: true,
            brutal: BrutalConfig::recommended(),
        }
    }
}

impl Default for MultiplexConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            protocol: Self::default_protocol(),
            max_connections: Self::default_max_connections(),
            min_streams: Self::default_min_streams(),
            max_streams: None,
            statistic: false,
            only_tcp: false,
            padding: false,
            brutal: BrutalConfig::default(),
        }
    }
}

/// TCP Brutal 拥塞控制配置（需服务端支持）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrutalConfig {
    /// 是否启用 TCP Brutal
    #[serde(default)]
    pub enabled: bool,
    /// 上传带宽（Mbps）
    #[serde(default = "BrutalConfig::default_up")]
    pub up: u32,
    /// 下载带宽（Mbps）
    #[serde(default = "BrutalConfig::default_down")]
    pub down: u32,
}

impl BrutalConfig {
    fn default_up() -> u32 { 20 }
    fn default_down() -> u32 { 50 }

    /// 推荐配置
    pub fn recommended() -> Self {
        Self {
            enabled: false,
            up: 20,
            down: 50,
        }
    }
}

impl Default for BrutalConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            up: Self::default_up(),
            down: Self::default_down(),
        }
    }
}

// 实现 ConfigFile trait
impl ConfigFile for AdvancedConfig {}

impl AdvancedConfig {
    /// 获取 advanced.yaml 默认路径
    pub fn default_path() -> Result<PathBuf> {
        crate::utils::dirs::app_home_dir()
            .map(|dir| dir.join("advanced.yaml"))
    }

    /// 从默认路径加载配置，文件不存在则返回默认值
    pub fn load_default() -> Self {
        Self::default_path()
            .ok()
            .and_then(|path| Self::load(&path).ok())
            .unwrap_or_default()
    }

    /// 保存配置到默认路径
    pub fn save_default(&self) -> Result<()> {
        let path = Self::default_path()?;
        self.save_to_file(&path)
    }

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

        self.egress_identity.validate()?;
        self.egress_monitor.validate()?;
        self.dns.validate()?;
        self.traffic_obfuscation.validate()
            .map_err(|e| anyhow::anyhow!("流量混淆配置错误: {}", e))?;

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

        // 合并会话绑定配置
        if other.session_affinity.enabled {
            self.session_affinity = other.session_affinity.clone();
        }

        if other.egress_identity.enabled {
            self.egress_identity = other.egress_identity.clone();
        }

        self.dns = other.dns.clone();

        if other.egress_monitor.enabled {
            self.egress_monitor = other.egress_monitor.clone();
        }

        if other.traffic_obfuscation.enabled {
            self.traffic_obfuscation = other.traffic_obfuscation.clone();
        } else if other.traffic_padding.enabled {
            // 兼容旧配置：traffic_padding -> traffic_obfuscation
            self.traffic_obfuscation = TrafficObfuscationConfig::from_legacy_padding(&other.traffic_padding);
        }

        if other.traffic_padding.enabled {
            self.traffic_padding = other.traffic_padding.clone();
        }

        if other.multiplex.enabled {
            self.multiplex = other.multiplex.clone();
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
        use crate::multipath::{NodePool, PoolType, SessionBinding, SlicingStrategy};

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
                self_destruct: SelfDestructConfig::default(),
                honeypot: HoneypotConfig::default(),
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
                bindings: SessionBinding::all_predefined(),
            },
            session_affinity: SessionAffinityConfig::default(),
            egress_identity: EgressIdentityConfig::recommended(),
            dns: DnsAdvancedConfig::default(),
            egress_monitor: EgressMonitorConfig {
                enabled: true,
                probe_interval_secs: 120,
                auto_rebind_on_change: false,
                notify_on_change: true,
                probe_timeout_secs: 10,
                watch_poll_interval_secs: 30,
                watch_debounce_secs: 10,
                rebind_strategy: crate::core::egress_monitor::RebindStrategyType::Smart,
            },
            traffic_obfuscation: TrafficObfuscationConfig {
                enabled: true,
                profile: crate::traffic::ObfuscationProfile::Conservative,
                ..TrafficObfuscationConfig::default()
            },
            traffic_padding: TrafficPaddingConfig {
                enabled: true,
                ..TrafficPaddingConfig::default()
            },
            multiplex: MultiplexConfig {
                enabled: false,
                ..MultiplexConfig::recommended()
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
