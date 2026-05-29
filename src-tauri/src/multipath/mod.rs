/**
 * 多路径阴影路由模块
 * 
 * 将流量分片到多个节点，降维打击行为分析
 */

use std::collections::HashMap;
use std::sync::Arc;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};

/// 多路径配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MultipathConfig {
    /// 是否启用多路径
    pub enabled: bool,
    /// 分片策略
    pub strategy: SlicingStrategy,
    /// 节点池
    pub node_pools: Vec<NodePool>,
    /// 最小分片大小（字节）
    pub min_fragment_size: usize,
    /// 最大分片大小（字节）
    pub max_fragment_size: usize,
    /// 重组超时（毫秒）
    pub reassembly_timeout: u64,
    /// 是否启用会话保持
    pub session_persistence: bool,
    /// 域名绑定规则
    #[serde(default = "SessionBinding::all_predefined")]
    pub bindings: Vec<SessionBinding>,
}

impl Default for MultipathConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            strategy: SlicingStrategy::RoundRobin,
            node_pools: Vec::new(),
            min_fragment_size: 1024,      // 1KB
            max_fragment_size: 65536,     // 64KB
            reassembly_timeout: 5000,     // 5秒
            session_persistence: true,
            bindings: SessionBinding::all_predefined(),
        }
    }
}

/// 分片策略
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SlicingStrategy {
    /// 轮询（均匀分配）
    RoundRobin,
    /// 随机（完全随机）
    Random,
    /// 加权（根据节点权重）
    Weighted,
    /// 最少连接（选择连接数最少的节点）
    LeastConnections,
    /// 延迟优先（选择延迟最低的节点）
    LatencyBased,
}

/// 节点池
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodePool {
    /// 池名称
    pub name: String,
    /// 池类型（用于会话保持）
    pub pool_type: PoolType,
    /// 节点列表
    pub nodes: Vec<PathNode>,
    /// 是否启用
    pub enabled: bool,
}

/// 池类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PoolType {
    /// 通用池（所有流量）
    General,
    /// 流媒体专用池（Netflix, YouTube 等）
    Streaming,
    /// 游戏专用池（低延迟）
    Gaming,
    /// 下载专用池（高带宽）
    Download,
    /// 社交媒体池（Twitter, Facebook 等）
    Social,
}

/// 路径节点
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PathNode {
    /// 节点名称
    pub name: String,
    /// 服务器地址
    pub server: String,
    /// 端口
    pub port: u16,
    /// 协议
    pub protocol: String,
    /// 权重（1-100）
    pub weight: u8,
    /// 是否启用
    pub enabled: bool,
    /// 地理位置（用于智能选择）
    pub location: Option<String>,
    /// 最大并发连接数
    pub max_connections: Option<u32>,
}

/// 会话绑定规则
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionBinding {
    /// 域名模式（支持通配符）
    pub domain_pattern: String,
    /// 绑定的池类型
    pub pool_type: PoolType,
    /// 是否强制使用单一节点
    pub force_single_node: bool,
    /// 描述
    pub description: String,
}

/// 预定义的会话绑定规则
impl SessionBinding {
    /// 流媒体服务（必须单节点，避免 IP 变化导致封号）
    pub fn streaming_services() -> Vec<Self> {
        vec![
            Self {
                domain_pattern: "*.netflix.com".to_string(),
                pool_type: PoolType::Streaming,
                force_single_node: true,
                description: "Netflix - 必须单节点".to_string(),
            },
            Self {
                domain_pattern: "*.youtube.com".to_string(),
                pool_type: PoolType::Streaming,
                force_single_node: true,
                description: "YouTube - 必须单节点".to_string(),
            },
            Self {
                domain_pattern: "*.hulu.com".to_string(),
                pool_type: PoolType::Streaming,
                force_single_node: true,
                description: "Hulu - 必须单节点".to_string(),
            },
            Self {
                domain_pattern: "*.disneyplus.com".to_string(),
                pool_type: PoolType::Streaming,
                force_single_node: true,
                description: "Disney+ - 必须单节点".to_string(),
            },
            Self {
                domain_pattern: "*.primevideo.com".to_string(),
                pool_type: PoolType::Streaming,
                force_single_node: true,
                description: "Amazon Prime Video - 必须单节点".to_string(),
            },
        ]
    }

    /// 游戏服务（必须单节点，避免延迟波动）
    pub fn gaming_services() -> Vec<Self> {
        vec![
            Self {
                domain_pattern: "*.steampowered.com".to_string(),
                pool_type: PoolType::Gaming,
                force_single_node: true,
                description: "Steam - 必须单节点".to_string(),
            },
            Self {
                domain_pattern: "*.epicgames.com".to_string(),
                pool_type: PoolType::Gaming,
                force_single_node: true,
                description: "Epic Games - 必须单节点".to_string(),
            },
            Self {
                domain_pattern: "*.riotgames.com".to_string(),
                pool_type: PoolType::Gaming,
                force_single_node: true,
                description: "Riot Games - 必须单节点".to_string(),
            },
            Self {
                domain_pattern: "*.blizzard.com".to_string(),
                pool_type: PoolType::Gaming,
                force_single_node: true,
                description: "Blizzard - 必须单节点".to_string(),
            },
        ]
    }

    /// 社交媒体（可以多路径，但建议单节点）
    pub fn social_services() -> Vec<Self> {
        vec![
            Self {
                domain_pattern: "*.twitter.com".to_string(),
                pool_type: PoolType::Social,
                force_single_node: true,
                description: "Twitter - 建议单节点".to_string(),
            },
            Self {
                domain_pattern: "*.facebook.com".to_string(),
                pool_type: PoolType::Social,
                force_single_node: true,
                description: "Facebook - 建议单节点".to_string(),
            },
            Self {
                domain_pattern: "*.instagram.com".to_string(),
                pool_type: PoolType::Social,
                force_single_node: true,
                description: "Instagram - 建议单节点".to_string(),
            },
        ]
    }

    /// 下载服务（可以多路径，提高速度）
    pub fn download_services() -> Vec<Self> {
        vec![
            Self {
                domain_pattern: "*.github.com".to_string(),
                pool_type: PoolType::Download,
                force_single_node: false,
                description: "GitHub - 可多路径".to_string(),
            },
            Self {
                domain_pattern: "*.githubusercontent.com".to_string(),
                pool_type: PoolType::Download,
                force_single_node: false,
                description: "GitHub Raw - 可多路径".to_string(),
            },
        ]
    }

    /// 获取所有预定义规则
    pub fn all_predefined() -> Vec<Self> {
        let mut rules = Vec::new();
        rules.extend(Self::streaming_services());
        rules.extend(Self::gaming_services());
        rules.extend(Self::social_services());
        rules.extend(Self::download_services());
        rules
    }
}

/// 分片信息
#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct Fragment {
    /// 流 ID
    pub stream_id: u64,
    /// 分片序号
    pub sequence: u32,
    /// 总分片数
    pub total_fragments: u32,
    /// 数据
    pub data: Vec<u8>,
    /// 目标节点
    pub target_node: String,
    /// 时间戳
    pub timestamp: u64,
}

/// 流会话
#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct StreamSession {
    /// 会话 ID
    pub session_id: u64,
    /// 目标域名
    pub domain: String,
    /// 绑定的节点（如果启用会话保持）
    pub bound_node: Option<String>,
    /// 绑定的池类型
    pub pool_type: PoolType,
    /// 是否强制单节点
    pub force_single_node: bool,
    /// 创建时间
    pub created_at: u64,
    /// 最后活动时间
    pub last_activity: u64,
}

/// 多路径管理器
pub struct MultipathManager {
    config: Arc<RwLock<MultipathConfig>>,
    #[allow(dead_code)]
    sessions: Arc<RwLock<HashMap<u64, StreamSession>>>,
    #[allow(dead_code)]
    node_stats: Arc<RwLock<HashMap<String, NodeStats>>>,
}

/// 节点统计
#[allow(dead_code)]
#[derive(Debug, Clone, Default)]
pub struct NodeStats {
    pub active_connections: u32,
    pub total_bytes: u64,
    pub avg_latency: u64,
    pub error_count: u32,
}

impl MultipathManager {
    pub fn new() -> Self {
        Self {
            config: Arc::new(RwLock::new(MultipathConfig::default())),
            sessions: Arc::new(RwLock::new(HashMap::new())),
            node_stats: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// 获取配置
    pub fn get_config(&self) -> MultipathConfig {
        self.config.read().clone()
    }

    /// 更新配置
    pub fn update_config(&self, config: MultipathConfig) {
        *self.config.write() = config;
    }

    /// 获取会话绑定规则
    pub fn get_bindings(&self) -> Vec<SessionBinding> {
        self.config.read().bindings.clone()
    }

    /// 添加会话绑定规则
    #[allow(dead_code)]
    pub fn add_binding(&self, binding: SessionBinding) {
        let mut config = self.config.write();
        config.bindings.push(binding);
    }

    /// 删除会话绑定规则
    #[allow(dead_code)]
    pub fn remove_binding(&self, domain_pattern: &str) {
        let mut config = self.config.write();
        config.bindings.retain(|b| b.domain_pattern != domain_pattern);
    }

    /// 检查域名是否需要单节点
    #[allow(dead_code)]
    pub fn should_use_single_node(&self, domain: &str) -> (bool, PoolType) {
        let config = self.config.read();
        Self::resolve_binding(domain, &config.bindings)
    }

    /// 域名匹配（支持通配符）
    fn match_domain(pattern: &str, domain: &str) -> bool {
        if pattern == domain {
            return true;
        }

        if pattern.starts_with("*.") {
            let suffix = &pattern[2..];
            return domain.ends_with(suffix) || domain == suffix;
        }

        false
    }

    fn resolve_binding(domain: &str, bindings: &[SessionBinding]) -> (bool, PoolType) {
        for binding in bindings {
            if Self::match_domain(&binding.domain_pattern, domain) {
                return (binding.force_single_node, binding.pool_type);
            }
        }

        (false, PoolType::General)
    }

    /// 选择节点
    pub fn select_node(&self, domain: &str, session_id: u64) -> Option<String> {
        let config = self.config.read();
        
        if !config.enabled {
            return None;
        }

        let (force_single, pool_type) = Self::resolve_binding(domain, &config.bindings);

        // 检查是否已有会话
        if config.session_persistence || force_single {
            let sessions = self.sessions.read();
            if let Some(session) = sessions.get(&session_id) {
                if let Some(ref node) = session.bound_node {
                    return Some(node.clone());
                }
            }
        }

        // 选择节点池
        let pool = config.node_pools.iter()
            .find(|p| p.enabled && p.pool_type == pool_type)
            .or_else(|| config.node_pools.iter().find(|p| p.enabled && p.pool_type == PoolType::General))?;

        // 根据策略选择节点
        let node = match config.strategy {
            SlicingStrategy::RoundRobin => self.select_round_robin(&pool.nodes),
            SlicingStrategy::Random => self.select_random(&pool.nodes),
            SlicingStrategy::Weighted => self.select_weighted(&pool.nodes),
            SlicingStrategy::LeastConnections => self.select_least_connections(&pool.nodes),
            SlicingStrategy::LatencyBased => self.select_latency_based(&pool.nodes),
        }?;

        // 如果需要单节点，创建会话绑定
        if force_single {
            let mut sessions = self.sessions.write();
            sessions.insert(session_id, StreamSession {
                session_id,
                domain: domain.to_string(),
                bound_node: Some(node.name.clone()),
                pool_type,
                force_single_node: true,
                created_at: Self::current_timestamp(),
                last_activity: Self::current_timestamp(),
            });
        }

        Some(node.name.clone())
    }

    fn select_round_robin(&self, nodes: &[PathNode]) -> Option<PathNode> {
        nodes.iter().find(|n| n.enabled).cloned()
    }

    fn select_random(&self, nodes: &[PathNode]) -> Option<PathNode> {
        use rand::seq::SliceRandom;
        let enabled: Vec<_> = nodes.iter().filter(|n| n.enabled).cloned().collect();
        enabled.choose(&mut rand::thread_rng()).cloned()
    }

    fn select_weighted(&self, nodes: &[PathNode]) -> Option<PathNode> {
        use rand::Rng;
        let enabled: Vec<_> = nodes.iter().filter(|n| n.enabled).cloned().collect();
        let total_weight: u32 = enabled.iter().map(|n| n.weight as u32).sum();
        
        if total_weight == 0 {
            return enabled.first().cloned();
        }

        let mut rng = rand::thread_rng();
        let mut random = rng.gen_range(0..total_weight);

        for node in enabled {
            if random < node.weight as u32 {
                return Some(node);
            }
            random -= node.weight as u32;
        }

        None
    }

    fn select_least_connections(&self, nodes: &[PathNode]) -> Option<PathNode> {
        let stats = self.node_stats.read();
        nodes.iter()
            .filter(|n| n.enabled)
            .min_by_key(|n| {
                stats.get(&n.name)
                    .map(|s| s.active_connections)
                    .unwrap_or(0)
            })
            .cloned()
    }

    fn select_latency_based(&self, nodes: &[PathNode]) -> Option<PathNode> {
        let stats = self.node_stats.read();
        nodes.iter()
            .filter(|n| n.enabled)
            .min_by_key(|n| {
                stats.get(&n.name)
                    .map(|s| s.avg_latency)
                    .unwrap_or(u64::MAX)
            })
            .cloned()
    }

    fn current_timestamp() -> u64 {
        use std::time::{SystemTime, UNIX_EPOCH};
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs()
    }
}

impl Default for MultipathManager {
    fn default() -> Self {
        Self::new()
    }
}
