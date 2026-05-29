# 安全增强 Phase 3 - 防封号核心功能设计文档

## 🎯 战略定位

**核心目标**: 防止账号被服务商封禁（对抗商业风控）  
**设计哲学**: "让自己看起来像一个正常的当地居民"，而不是"让流量无法被识别"

### 与 Phase 2 的区别

| 维度 | Phase 2 | Phase 3 |
|------|---------|---------|
| **对抗目标** | GFW 网络审查 | 商业风控系统 |
| **核心关注** | 流量特征、协议识别 | 行为一致性、IP信誉 |
| **技术手段** | 流量混淆、协议伪装 | 固定节点、高信誉IP、出口身份一致性 |
| **优先级** | P1-P2 | **P0（最高）** |

---

## 📋 Phase 3 任务清单

### Task 1: 会话绑定系统（4小时）⭐⭐⭐⭐⭐
**核心价值**: 防止 IP 频繁跳动导致封号

**子任务**:
1. 域名级绑定（如 `*.openai.com` 固定到特定节点）
2. 进程级绑定（如 `Steam.exe` 固定到特定节点）
3. 连接级绑定（基于源 IP+端口跟踪）
4. UI 配置界面

**文件**:
- `src-tauri/src/core/session_affinity.rs`
- `src-tauri/src/cmd/session_affinity.rs`
- `src/services/session-affinity.ts`
- `src/components/security/session-affinity-config.tsx`

### Task 2: IP 信誉度系统（6小时）⭐⭐⭐⭐⭐
**核心价值**: 根据服务风控等级选择合适的 IP 类型

**子任务**:
1. 集成 IP 信誉度 API（IPQualityScore、MaxMind）
2. 实现节点信誉度标注（Datacenter/ISP/Mobile）
3. 实现风控等级路由规则
4. UI 节点信誉度展示

**文件**:
- `src-tauri/src/core/ip_reputation.rs`
- `src-tauri/src/cmd/ip_reputation.rs`
- `src/services/ip-reputation.ts`
- `src/components/proxy/ip-reputation-badge.tsx`

### Task 3: 代理级出口身份管理（4小时）⭐⭐⭐⭐⭐
**核心价值**: 由代理软件统一为应用、快捷方式和业务会话分配稳定出口身份，做到同一主体始终映射到同一出口画像

**子任务**:
1. 设计出口身份画像（节点偏好、IP 信誉约束、DNS/TLS/会话策略）
2. 设计应用/快捷方式映射规则（`process_name`、`exe_path`、`shortcut_id`）
3. 实现统一出口决策器（协调会话绑定与 IP 信誉度选择）
4. 提供统一配置界面与运行态分配观测能力

**文件**:
- `src-tauri/src/core/egress_identity.rs`
- `src-tauri/src/cmd/egress_identity.rs`
- `src/services/egress-identity.ts`
- `src/components/advanced/egress-identity-panel.tsx`
- `src-tauri/src/config/advanced.rs`
- `src-tauri/src/core/coordinator.rs`

**总计**: 14小时

---

## 🏗️ 架构设计

### 整体架构

```
┌─────────────────────────────────────────────────────────┐
│                    前端 UI 层                            │
│  SessionAffinityConfig | IpReputationBadge |            │
│  EgressIdentityPanel                                    │
└────────────┬────────────────────────────────────────────┘
             │
┌────────────▼────────────────────────────────────────────┐
│                  TypeScript 服务层                       │
│  coordinator.ts | session-affinity.ts |                 │
│  ip-reputation.ts | egress-identity.ts                  │
└────────────┬────────────────────────────────────────────┘
             │ Tauri Commands
┌────────────▼────────────────────────────────────────────┐
│                   Rust 后端层                            │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐  │
│  │ Session      │  │ IP           │  │ Egress       │  │
│  │ Affinity     │  │ Reputation   │  │ Identity     │  │
│  └──────────────┘  └──────────────┘  └──────────────┘  │
└─────────────────────────────────────────────────────────┘
             │
┌────────────▼────────────────────────────────────────────┐
│                   集成层                                 │
│  AdvancedConfig + CoreCoordinator (统一协调)            │
└─────────────────────────────────────────────────────────┘


---

## 📐 Task 1: 会话绑定系统详细设计

### 1.1 数据结构

```rust
// src-tauri/src/core/session_affinity.rs

/// 会话绑定配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionAffinityConfig {
    /// 启用会话绑定
    pub enabled: bool,
    /// 域名级绑定规则
    pub domain_rules: Vec<DomainBindingRule>,
    /// 进程级绑定规则
    pub process_rules: Vec<ProcessBindingRule>,
    /// 连接级绑定配置
    pub connection_binding: ConnectionBindingConfig,
}

/// 域名绑定规则
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DomainBindingRule {
    /// 域名模式（支持通配符）
    pub domain_pattern: String,
    /// 是否启用
    pub enabled: bool,
    /// 绑定的节点名称（None 表示自动选择后绑定）
    pub bound_node: Option<String>,
    /// 绑定时长（秒，0 表示永久）
    pub ttl: u64,
    /// 故障转移策略
    pub fallback_policy: FallbackPolicy,
    /// 描述
    pub description: String,
}

/// 进程绑定规则
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessBindingRule {
    /// 进程名称（如 "Steam.exe"）
    pub process_name: String,
    /// 是否启用
    pub enabled: bool,
    /// 绑定的节点名称
    pub bound_node: Option<String>,
    /// 绑定时长（秒）
    pub ttl: u64,
    /// 故障转移策略
    pub fallback_policy: FallbackPolicy,
    /// 描述
    pub description: String,
}

/// 连接级绑定配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionBindingConfig {
    /// 启用连接级绑定
    pub enabled: bool,
    /// 跟踪方式
    pub track_by: TrackBy,
    /// 超时时间（秒）
    pub timeout: u64,
}

/// 跟踪方式
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TrackBy {
    /// 源 IP + 端口
    SourceIpPort,
    /// 会话 ID
    SessionId,
}

/// 故障转移策略
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FallbackPolicy {
    /// 手动确认（需要用户手动选择新节点）
    Manual,
    /// 自动重试当前节点
    AutoRetry,
    /// 自动切换到备用节点
    AutoSwitch,
}

/// 节点绑定记录
#[derive(Debug, Clone)]
pub struct NodeBinding {
    /// 节点 ID
    pub node_id: String,
    /// 绑定时间
    pub bound_at: SystemTime,
    /// 过期时间
    pub expires_at: Option<SystemTime>,
    /// 故障转移策略
    pub fallback_policy: FallbackPolicy,
}

/// 会话绑定管理器
pub struct SessionAffinityManager {
    /// 配置
    config: Arc<RwLock<SessionAffinityConfig>>,
    /// 域名 -> 节点绑定
    domain_bindings: Arc<RwLock<HashMap<String, NodeBinding>>>,
    /// 进程 -> 节点绑定
    process_bindings: Arc<RwLock<HashMap<String, NodeBinding>>>,
    /// 连接 -> 节点绑定
    connection_bindings: Arc<RwLock<HashMap<ConnectionId, NodeBinding>>>,
}

/// 连接 ID
#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub struct ConnectionId {
    pub source_ip: String,
    pub source_port: u16,
}
```

### 1.2 核心算法

#### 1.2.1 域名匹配算法

```rust
/// 检查域名是否匹配规则
pub fn domain_matches(domain: &str, pattern: &str) -> bool {
    // 支持通配符匹配
    // 例如: "*.openai.com" 匹配 "chat.openai.com"
    
    if pattern.starts_with("*.") {
        let suffix = &pattern[2..];
        domain.ends_with(suffix) || domain == suffix
    } else if pattern.starts_with("*") {
        let suffix = &pattern[1..];
        domain.ends_with(suffix)
    } else {
        domain == pattern
    }
}
```


#### 1.2.2 节点选择与绑定

```rust
impl SessionAffinityManager {
    /// 为域名选择节点（考虑会话绑定）
    pub async fn select_node_for_domain(
        &self,
        domain: &str,
        available_nodes: &[String],
    ) -> Result<String> {
        let config = self.config.read().await;
        
        // 1. 查找匹配的域名规则
        let rule = config.domain_rules.iter()
            .find(|r| r.enabled && domain_matches(domain, &r.domain_pattern));
        
        if let Some(rule) = rule {
            // 2. 检查是否已有绑定
            let bindings = self.domain_bindings.read().await;
            if let Some(binding) = bindings.get(domain) {
                // 检查绑定是否过期
                if !self.is_binding_expired(binding) {
                    // 检查节点是否仍然可用
                    if available_nodes.contains(&binding.node_id) {
                        return Ok(binding.node_id.clone());
                    } else {
                        // 节点不可用，根据故障转移策略处理
                        return self.handle_node_unavailable(
                            domain,
                            binding,
                            available_nodes,
                        ).await;
                    }
                }
            }
            
            // 3. 没有绑定或已过期，选择新节点
            let node = if let Some(ref bound_node) = rule.bound_node {
                // 使用指定节点
                if available_nodes.contains(bound_node) {
                    bound_node.clone()
                } else {
                    return Err("指定节点不可用".into());
                }
            } else {
                // 自动选择节点（使用第一个可用节点）
                available_nodes.first()
                    .ok_or("没有可用节点")?
                    .clone()
            };
            
            // 4. 创建绑定
            let binding = NodeBinding {
                node_id: node.clone(),
                bound_at: SystemTime::now(),
                expires_at: if rule.ttl > 0 {
                    Some(SystemTime::now() + Duration::from_secs(rule.ttl))
                } else {
                    None
                },
                fallback_policy: rule.fallback_policy.clone(),
            };
            
            // 5. 保存绑定
            let mut bindings = self.domain_bindings.write().await;
            bindings.insert(domain.to_string(), binding);
            
            log::info!("[SessionAffinity] 域名 {} 绑定到节点 {}", domain, node);
            
            Ok(node)
        } else {
            // 没有匹配的规则，使用默认选择
            Ok(available_nodes.first()
                .ok_or("没有可用节点")?
                .clone())
        }
    }
    
    /// 检查绑定是否过期
    fn is_binding_expired(&self, binding: &NodeBinding) -> bool {
        if let Some(expires_at) = binding.expires_at {
            SystemTime::now() > expires_at
        } else {
            false
        }
    }
    
    /// 处理节点不可用
    async fn handle_node_unavailable(
        &self,
        domain: &str,
        binding: &NodeBinding,
        available_nodes: &[String],
    ) -> Result<String> {
        match binding.fallback_policy {
            FallbackPolicy::Manual => {
                // 需要用户手动确认
                Err("节点不可用，需要手动选择新节点".into())
            }
            FallbackPolicy::AutoRetry => {
                // 自动重试当前节点（返回错误，让上层重试）
                Err("节点不可用，正在重试".into())
            }
            FallbackPolicy::AutoSwitch => {
                // 自动切换到备用节点
                let new_node = available_nodes.first()
                    .ok_or("没有可用节点")?
                    .clone();
                
                // 更新绑定
                let mut bindings = self.domain_bindings.write().await;
                let mut new_binding = binding.clone();
                new_binding.node_id = new_node.clone();
                new_binding.bound_at = SystemTime::now();
                bindings.insert(domain.to_string(), new_binding);
                
                log::warn!("[SessionAffinity] 域名 {} 自动切换到节点 {}", domain, new_node);
                
                Ok(new_node)
            }
        }
    }
}
```

#### 1.2.3 进程检测

```rust
#[cfg(target_os = "windows")]
pub fn get_process_name_by_port(port: u16) -> Result<String> {
    use std::process::Command;
    
    let output = Command::new("netstat")
        .args(&["-ano"])
        .output()?;
    
    let stdout = String::from_utf8_lossy(&output.stdout);
    
    // 解析 netstat 输出，找到对应端口的 PID
    for line in stdout.lines() {
        if line.contains(&format!(":{}", port)) {
            // 提取 PID
            if let Some(pid) = extract_pid_from_line(line) {
                // 根据 PID 获取进程名
                return get_process_name_by_pid(pid);
            }
        }
    }
    
    Err("未找到进程".into())
}

#[cfg(target_os = "linux")]
pub fn get_process_name_by_port(port: u16) -> Result<String> {
    use std::fs;
    
    // 读取 /proc/net/tcp
    let tcp_content = fs::read_to_string("/proc/net/tcp")?;
    
    // 解析找到对应端口的 inode
    for line in tcp_content.lines().skip(1) {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() > 9 {
            let local_address = parts[1];
            if local_address.ends_with(&format!(":{:04X}", port)) {
                let inode = parts[9];
                // 根据 inode 查找进程
                return find_process_by_inode(inode);
            }
        }
    }
    
    Err("未找到进程".into())
}
```


### 1.3 预定义规则

```rust
/// 获取预定义的会话绑定规则
pub fn get_predefined_rules() -> Vec<DomainBindingRule> {
    vec![
        // AI 服务（极高风控）
        DomainBindingRule {
            domain_pattern: "*.openai.com".to_string(),
            enabled: true,
            bound_node: None,
            ttl: 86400, // 24小时
            fallback_policy: FallbackPolicy::Manual,
            description: "ChatGPT - 必须单节点，24小时内不允许切换".to_string(),
        },
        DomainBindingRule {
            domain_pattern: "*.anthropic.com".to_string(),
            enabled: true,
            bound_node: None,
            ttl: 86400,
            fallback_policy: FallbackPolicy::Manual,
            description: "Claude - 必须单节点".to_string(),
        },
        
        // 游戏平台（高风控）
        DomainBindingRule {
            domain_pattern: "*.steampowered.com".to_string(),
            enabled: true,
            bound_node: None,
            ttl: 604800, // 7天
            fallback_policy: FallbackPolicy::Manual,
            description: "Steam - 必须单节点，7天内不允许切换".to_string(),
        },
        DomainBindingRule {
            domain_pattern: "*.epicgames.com".to_string(),
            enabled: true,
            bound_node: None,
            ttl: 604800,
            fallback_policy: FallbackPolicy::Manual,
            description: "Epic Games - 必须单节点".to_string(),
        },
        
        // 金融服务（极高风控）
        DomainBindingRule {
            domain_pattern: "*.stripe.com".to_string(),
            enabled: true,
            bound_node: None,
            ttl: 2592000, // 30天
            fallback_policy: FallbackPolicy::Manual,
            description: "Stripe - 必须单节点，30天内不允许切换".to_string(),
        },
        DomainBindingRule {
            domain_pattern: "*.paypal.com".to_string(),
            enabled: true,
            bound_node: None,
            ttl: 2592000,
            fallback_policy: FallbackPolicy::Manual,
            description: "PayPal - 必须单节点".to_string(),
        },
        
        // 社交媒体（中风控）
        DomainBindingRule {
            domain_pattern: "*.twitter.com".to_string(),
            enabled: true,
            bound_node: None,
            ttl: 86400,
            fallback_policy: FallbackPolicy::AutoSwitch,
            description: "Twitter - 建议单节点".to_string(),
        },
        DomainBindingRule {
            domain_pattern: "*.x.com".to_string(),
            enabled: true,
            bound_node: None,
            ttl: 86400,
            fallback_policy: FallbackPolicy::AutoSwitch,
            description: "X (Twitter) - 建议单节点".to_string(),
        },
    ]
}
```

### 1.4 API 设计

#### Rust Commands

```rust
// src-tauri/src/cmd/session_affinity.rs

#[tauri::command]
pub async fn session_affinity_get_config(
    state: State<'_, Arc<SessionAffinityManager>>
) -> Result<SessionAffinityConfig, String> {
    state.get_config().await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn session_affinity_update_config(
    config: SessionAffinityConfig,
    state: State<'_, Arc<SessionAffinityManager>>
) -> Result<(), String> {
    state.update_config(config).await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn session_affinity_get_bindings(
    state: State<'_, Arc<SessionAffinityManager>>
) -> Result<Vec<BindingInfo>, String> {
    state.get_all_bindings().await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn session_affinity_clear_binding(
    domain: String,
    state: State<'_, Arc<SessionAffinityManager>>
) -> Result<(), String> {
    state.clear_domain_binding(&domain).await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn session_affinity_get_predefined_rules() -> Result<Vec<DomainBindingRule>, String> {
    Ok(get_predefined_rules())
}
```

#### TypeScript Service

```typescript
// src/services/session-affinity.ts

export interface SessionAffinityConfig {
  enabled: boolean;
  domainRules: DomainBindingRule[];
  processRules: ProcessBindingRule[];
  connectionBinding: ConnectionBindingConfig;
}

export interface DomainBindingRule {
  domainPattern: string;
  enabled: boolean;
  boundNode?: string;
  ttl: number;
  fallbackPolicy: 'manual' | 'autoRetry' | 'autoSwitch';
  description: string;
}

export interface ProcessBindingRule {
  processName: string;
  enabled: boolean;
  boundNode?: string;
  ttl: number;
  fallbackPolicy: 'manual' | 'autoRetry' | 'autoSwitch';
  description: string;
}

export interface ConnectionBindingConfig {
  enabled: boolean;
  trackBy: 'sourceIpPort' | 'sessionId';
  timeout: number;
}

export interface BindingInfo {
  type: 'domain' | 'process' | 'connection';
  key: string;
  nodeId: string;
  boundAt: number;
  expiresAt?: number;
}

export async function sessionAffinityGetConfig(): Promise<SessionAffinityConfig>
export async function sessionAffinityUpdateConfig(config: SessionAffinityConfig): Promise<void>
export async function sessionAffinityGetBindings(): Promise<BindingInfo[]>
export async function sessionAffinityClearBinding(domain: string): Promise<void>
export async function sessionAffinityGetPredefinedRules(): Promise<DomainBindingRule[]>
```

---

## 📐 Task 2: IP 信誉度系统详细设计

### 2.1 数据结构

```rust
// src-tauri/src/core/ip_reputation.rs

/// IP 信誉度配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IpReputationConfig {
    /// 启用 IP 信誉度检测
    pub enabled: bool,
    /// API 提供商
    pub providers: Vec<ReputationProvider>,
    /// 缓存时长（秒）
    pub cache_ttl: u64,
    /// 风控等级路由规则
    pub routing_rules: Vec<RiskRoutingRule>,
}

/// 信誉度提供商
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ReputationProvider {
    /// IPQualityScore
    IpQualityScore { api_key: String },
    /// MaxMind GeoIP2
    MaxMind { account_id: String, license_key: String },
    /// IPHub
    IpHub { api_key: String },
    /// 本地数据库
    LocalDatabase { db_path: String },
}

/// IP 信誉度信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IpReputation {
    /// IP 地址
    pub ip: String,
    /// IP 类型
    pub ip_type: IpType,
    /// ASN
    pub asn: String,
    /// ASN 组织
    pub asn_org: String,
    /// 欺诈评分（0-100）
    pub fraud_score: u8,
    /// 风险等级
    pub risk_level: RiskLevel,
    /// 是否为代理
    pub is_proxy: bool,
    /// 是否为 VPN
    pub is_vpn: bool,
    /// 是否为 Tor
    pub is_tor: bool,
    /// 国家代码
    pub country_code: String,
    /// 城市
    pub city: Option<String>,
    /// 检测时间
    pub checked_at: SystemTime,
}

/// IP 类型
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum IpType {
    /// 机房 IP
    Datacenter,
    /// ISP/住宅 IP
    Residential,
    /// 移动网络 IP
    Mobile,
    /// 未知
    Unknown,
}

/// 风险等级
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RiskLevel {
    Low,      // 0-30
    Medium,   // 31-60
    High,     // 61-85
    VeryHigh, // 86-100
}
```


/// 风控等级路由规则
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskRoutingRule {
    /// 域名模式
    pub domain_patterns: Vec<String>,
    /// 是否启用
    pub enabled: bool,
    /// 要求的 IP 类型
    pub required_ip_type: Option<IpType>,
    /// 最大欺诈评分
    pub max_fraud_score: u8,
    /// 故障转移策略
    pub fallback_policy: RiskFallbackPolicy,
    /// 描述
    pub description: String,
}

/// 风控故障转移策略
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RiskFallbackPolicy {
    /// 阻止连接
    Block,
    /// 警告但允许
    Warn,
    /// 允许
    Allow,
}

/// IP 信誉度管理器
pub struct IpReputationManager {
    /// 配置
    config: Arc<RwLock<IpReputationConfig>>,
    /// IP 信誉度缓存
    cache: Arc<RwLock<HashMap<String, IpReputation>>>,
    /// API 提供商
    providers: Vec<Box<dyn ReputationProvider + Send + Sync>>,
}
```

### 2.2 核心算法

#### 2.2.1 IP 信誉度检测

```rust
impl IpReputationManager {
    /// 检测 IP 信誉度
    pub async fn check_ip_reputation(&self, ip: &str) -> Result<IpReputation> {
        // 1. 检查缓存
        let cache = self.cache.read().await;
        if let Some(cached) = cache.get(ip) {
            let age = SystemTime::now()
                .duration_since(cached.checked_at)
                .unwrap_or_default();
            
            if age < Duration::from_secs(self.config.read().await.cache_ttl) {
                return Ok(cached.clone());
            }
        }
        drop(cache);

        // 2. 从 API 提供商获取
        let mut reputation = None;
        for provider in &self.providers {
            match provider.check_ip(ip).await {
                Ok(rep) => {
                    reputation = Some(rep);
                    break;
                }
                Err(e) => {
                    log::warn!("[IpReputation] 提供商检测失败: {}", e);
                    continue;
                }
            }
        }

        let reputation = reputation.ok_or_else(|| anyhow!("所有提供商都失败"))?;

        // 3. 更新缓存
        let mut cache = self.cache.write().await;
        cache.insert(ip.to_string(), reputation.clone());

        Ok(reputation)
    }

    /// 为域名选择合适的节点（考虑 IP 信誉度）
    pub async fn select_node_for_domain(
        &self,
        domain: &str,
        available_nodes: &[(String, String)], // (node_name, node_ip)
    ) -> Result<String> {
        let config = self.config.read().await;

        // 1. 查找匹配的路由规则
        let rule = config.routing_rules.iter()
            .find(|r| r.enabled && r.domain_patterns.iter()
                .any(|p| domain_matches(domain, p)));

        if let Some(rule) = rule {
            // 2. 检测所有节点的 IP 信誉度
            let mut suitable_nodes = Vec::new();
            
            for (node_name, node_ip) in available_nodes {
                match self.check_ip_reputation(node_ip).await {
                    Ok(reputation) => {
                        // 检查是否满足要求
                        let type_match = rule.required_ip_type.as_ref()
                            .map(|req| matches_ip_type(&reputation.ip_type, req))
                            .unwrap_or(true);
                        
                        let score_match = reputation.fraud_score <= rule.max_fraud_score;

                        if type_match && score_match {
                            suitable_nodes.push((node_name.clone(), reputation));
                        }
                    }
                    Err(e) => {
                        log::warn!("[IpReputation] 检测节点 {} 失败: {}", node_name, e);
                    }
                }
            }

            // 3. 根据故障转移策略处理
            if suitable_nodes.is_empty() {
                match rule.fallback_policy {
                    RiskFallbackPolicy::Block => {
                        return Err(anyhow!("没有满足信誉度要求的节点"));
                    }
                    RiskFallbackPolicy::Warn => {
                        log::warn!("[IpReputation] 没有满足要求的节点，使用默认节点");
                        return Ok(available_nodes.first()
                            .ok_or_else(|| anyhow!("没有可用节点"))?
                            .0.clone());
                    }
                    RiskFallbackPolicy::Allow => {
                        return Ok(available_nodes.first()
                            .ok_or_else(|| anyhow!("没有可用节点"))?
                            .0.clone());
                    }
                }
            }

            // 4. 选择信誉度最好的节点
            suitable_nodes.sort_by_key(|(_, rep)| rep.fraud_score);
            Ok(suitable_nodes.first().unwrap().0.clone())
        } else {
            // 没有匹配的规则，使用默认选择
            Ok(available_nodes.first()
                .ok_or_else(|| anyhow!("没有可用节点"))?
                .0.clone())
        }
    }
}

fn matches_ip_type(actual: &IpType, required: &IpType) -> bool {
    match (actual, required) {
        (IpType::Residential, IpType::Residential) => true,
        (IpType::Mobile, IpType::Residential) => true, // Mobile 也算 Residential
        (IpType::Mobile, IpType::Mobile) => true,
        _ => false,
    }
}
```

#### 2.2.2 预定义路由规则

```rust
/// 获取预定义的风控路由规则
pub fn get_predefined_routing_rules() -> Vec<RiskRoutingRule> {
    vec![
        // AI 服务（极高风控）
        RiskRoutingRule {
            domain_patterns: vec![
                "*.openai.com".to_string(),
                "*.anthropic.com".to_string(),
            ],
            enabled: true,
            required_ip_type: Some(IpType::Residential),
            max_fraud_score: 30,
            fallback_policy: RiskFallbackPolicy::Block,
            description: "AI 服务 - 必须使用住宅 IP，欺诈评分 < 30".to_string(),
        },
        
        // 金融服务（极高风控）
        RiskRoutingRule {
            domain_patterns: vec![
                "*.stripe.com".to_string(),
                "*.paypal.com".to_string(),
            ],
            enabled: true,
            required_ip_type: Some(IpType::Residential),
            max_fraud_score: 20,
            fallback_policy: RiskFallbackPolicy::Block,
            description: "金融服务 - 必须使用住宅 IP，欺诈评分 < 20".to_string(),
        },
        
        // 游戏平台（高风控）
        RiskRoutingRule {
            domain_patterns: vec![
                "*.steampowered.com".to_string(),
                "*.epicgames.com".to_string(),
                "*.riotgames.com".to_string(),
            ],
            enabled: true,
            required_ip_type: Some(IpType::Residential),
            max_fraud_score: 50,
            fallback_policy: RiskFallbackPolicy::Warn,
            description: "游戏平台 - 建议使用住宅 IP，欺诈评分 < 50".to_string(),
        },
        
        // 社交媒体（中风控）
        RiskRoutingRule {
            domain_patterns: vec![
                "*.twitter.com".to_string(),
                "*.x.com".to_string(),
                "*.facebook.com".to_string(),
                "*.instagram.com".to_string(),
            ],
            enabled: true,
            required_ip_type: None,
            max_fraud_score: 70,
            fallback_policy: RiskFallbackPolicy::Warn,
            description: "社交媒体 - 欺诈评分 < 70".to_string(),
        },
    ]
}
```

### 2.3 API 设计

#### Rust Commands

```rust
// src-tauri/src/cmd/ip_reputation.rs

#[tauri::command]
pub async fn ip_reputation_get_config() -> Result<IpReputationConfig, String>

#[tauri::command]
pub async fn ip_reputation_update_config(
    config: IpReputationConfig
) -> Result<(), String>

#[tauri::command]
pub async fn ip_reputation_check_ip(ip: String) -> Result<IpReputation, String>

#[tauri::command]
pub async fn ip_reputation_get_predefined_rules() -> Result<Vec<RiskRoutingRule>, String>

#[tauri::command]
pub async fn ip_reputation_select_node_for_domain(
    domain: String,
    available_nodes: Vec<(String, String)>
) -> Result<String, String>
```

#### TypeScript Service

```typescript
// src/services/ip-reputation.ts

export interface IpReputationConfig {
  enabled: boolean;
  providers: ReputationProvider[];
  cacheTtl: number;
  routingRules: RiskRoutingRule[];
}

export interface IpReputation {
  ip: string;
  ipType: 'Datacenter' | 'Residential' | 'Mobile' | 'Unknown';
  asn: string;
  asnOrg: string;
  fraudScore: number;
  riskLevel: 'Low' | 'Medium' | 'High' | 'VeryHigh';
  isProxy: boolean;
  isVpn: boolean;
  isTor: boolean;
  countryCode: string;
  city?: string;
  checkedAt: number;
}

export async function ipReputationGetConfig(): Promise<IpReputationConfig>
export async function ipReputationUpdateConfig(config: IpReputationConfig): Promise<void>
export async function ipReputationCheckIp(ip: string): Promise<IpReputation>
export async function ipReputationGetPredefinedRules(): Promise<RiskRoutingRule[]>
export async function ipReputationSelectNodeForDomain(
  domain: string,
  availableNodes: [string, string][]
): Promise<string>
```

---

## 📐 Task 3: 代理级出口身份管理详细设计

### 3.1 核心目标

Task 3 不再以浏览器环境伪装为中心，而是把代理软件本身升级为**唯一出口身份分配器**。核心目标是：

1. **唯一决策源**：所有应用、快捷方式、域名和业务会话的出口选择都由代理软件内部统一决策。
2. **身份一致性**：同一主体在一段时间内始终映射到同一出口画像，而不是只绑定单个节点。
3. **策略组合化**：出口身份不仅包含节点，还包含 IP 信誉要求、DNS 策略、TLS 指纹和故障转移策略。
4. **为后续软件快捷方式直连奠基**：未来从软件内直接启动目标应用时，可以稳定复用同一出口身份。

这里的“唯一性”不是浏览器指纹唯一，而是：

- 同一个 `shortcut_id` -> 同一个 `profile_id`
- 同一个 `process_name` / `exe_path` -> 同一个出口画像
- 同一个高风控域名 -> 在同一身份画像内保持稳定节点与稳定策略

### 3.2 设计原则

1. **唯一事实源**：配置只存放在 `AdvancedConfig.egress_identity`，统一持久化到 `advanced.yaml`。
2. **不做浏览器特化**：不把 WebRTC、Canvas、扩展注入作为 Task 3 主线。
3. **复用已有能力**：复用 `SessionAffinityManager`、`IpReputationManager`、TLS 指纹与 DNS/TUN 相关能力。
4. **分层职责清晰**：`EgressIdentityManager` 负责统一编排，不重复实现已有模块的细节能力。
5. **先做可执行 MVP**：优先建立画像、映射、选择、观测四件套，再逐步扩展更复杂的启动托管能力。

### 3.3 数据结构

```rust
// src-tauri/src/core/egress_identity.rs

/// 代理级出口身份配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EgressIdentityConfig {
    /// 总开关
    pub enabled: bool,
    /// 默认画像 ID
    pub default_profile: Option<String>,
    /// 可选的出口身份画像
    pub profiles: Vec<EgressIdentityProfile>,
    /// 应用规则（按进程/路径匹配）
    pub app_rules: Vec<AppEgressRule>,
    /// 快捷方式规则（未来软件内直启映射）
    pub shortcut_rules: Vec<ShortcutEgressRule>,
}

/// 单个出口身份画像
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EgressIdentityProfile {
    /// 唯一 ID
    pub id: String,
    /// 展示名称
    pub name: String,
    /// 是否启用
    pub enabled: bool,
    /// 优先节点名单
    pub preferred_nodes: Vec<String>,
    /// 优先节点池名单
    pub preferred_pools: Vec<String>,
    /// 所需 IP 类型（复用 Task 2）
    pub required_ip_type: Option<IpType>,
    /// 最大欺诈评分
    pub max_fraud_score: Option<u8>,
    /// DNS 策略
    pub dns_policy: DnsPolicy,
    /// TLS 指纹名称（复用现有指纹库）
    pub tls_fingerprint: Option<String>,
    /// 会话稳定策略
    pub session_policy: IdentitySessionPolicy,
    /// 失败时如何处理
    pub failover_policy: EgressFailoverPolicy,
    /// 描述
    pub description: String,
}

/// 应用映射规则
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppEgressRule {
    /// 进程名（如 Steam.exe）
    pub process_name: Option<String>,
    /// 可执行文件路径（更高精度）
    pub exe_path: Option<String>,
    /// 可选域名约束
    pub domains: Vec<String>,
    /// 命中的画像 ID
    pub profile_id: String,
    /// 优先级（数值越小越高）
    pub priority: u32,
    /// 是否启用
    pub enabled: bool,
}

/// 快捷方式映射规则
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShortcutEgressRule {
    /// 软件内部快捷方式 ID
    pub shortcut_id: String,
    /// 命中的画像 ID
    pub profile_id: String,
    /// 是否启用
    pub enabled: bool,
}

/// DNS 策略
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DnsPolicy {
    /// DNS 工作模式
    pub mode: DnsMode,
    /// 是否强制远端 DNS
    pub force_remote_dns: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DnsMode {
    /// 沿用全局设置
    Inherit,
    /// 强制 fake-ip / tun hijack 路线
    Hijack,
    /// 强制远端解析
    Remote,
}

/// 会话稳定策略
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IdentitySessionPolicy {
    /// 是否强制单节点粘性
    pub strict_affinity: bool,
    /// 可选 TTL 覆盖（秒）
    pub ttl_override: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EgressFailoverPolicy {
    /// 直接阻止
    Block,
    /// 手动确认
    Manual,
    /// 自动切换
    AutoSwitch,
}

/// 运行时解析输入
#[derive(Debug, Clone)]
pub struct EgressSelectionContext {
    pub shortcut_id: Option<String>,
    pub process_name: Option<String>,
    pub exe_path: Option<String>,
    pub domain: Option<String>,
    pub available_nodes: Vec<String>,
}

/// 运行时解析输出
#[derive(Debug, Clone, Serialize)]
pub struct ResolvedEgressIdentity {
    pub profile_id: String,
    pub selected_node: String,
    pub dns_mode: DnsMode,
    pub tls_fingerprint: Option<String>,
    pub matched_by: String,
}
```

### 3.4 核心算法

```rust
impl EgressIdentityManager {
    pub async fn resolve(&self, ctx: EgressSelectionContext) -> Result<ResolvedEgressIdentity> {
        // 1. 匹配画像：shortcut_id > exe_path > process_name > 默认画像
        let profile = self.match_profile(&ctx)?;

        // 2. 过滤候选节点：优先节点 / 节点池约束
        let candidate_nodes = self.filter_candidates(&profile, &ctx.available_nodes)?;

        // 3. 应用 IP 信誉约束（复用 IpReputationManager）
        let candidate_nodes = self
            .apply_ip_reputation_constraints(&profile, candidate_nodes)
            .await?;

        // 4. 选择稳定节点（复用 SessionAffinityManager）
        let selected_node = self
            .apply_session_affinity(&profile, &ctx, &candidate_nodes)
            .await?;

        // 5. 组合输出的出口身份
        Ok(ResolvedEgressIdentity {
            profile_id: profile.id.clone(),
            selected_node,
            dns_mode: profile.dns_policy.mode.clone(),
            tls_fingerprint: profile.tls_fingerprint.clone(),
            matched_by: self.last_match_reason().to_string(),
        })
    }
}
```

核心流程说明：

1. **先定画像**：先判断“谁在访问”，再决定“怎么出站”。
2. **再筛节点**：根据画像限制节点池、固定节点和 IP 风险阈值。
3. **最后粘性选择**：对剩余节点应用会话绑定，保证行为连续性。
4. **返回完整身份**：不仅返回节点，还返回 DNS/TLS 等需要联动的策略。

### 3.5 与现有架构的集成方式

#### 3.5.1 配置入口

Task 3 必须直接进入统一配置链：

```rust
pub struct AdvancedConfig {
    pub security: SecurityConfig,
    pub multipath: MultipathConfig,
    pub session_affinity: SessionAffinityConfig,
    pub egress_identity: EgressIdentityConfig,
    pub xdp: XdpConfig,
}
```

关键约束：

- 不创建 `egress_identity.yaml`
- 不提供独立配置持久化链路
- 前端只通过 `get_advanced_config` / `save_advanced_config` 读写

#### 3.5.2 运行时编排

`CoreCoordinator` 成为唯一总入口：

- `SessionAffinityManager`：负责节点粘性
- `IpReputationManager`：负责信誉筛选
- `TlsFingerprintService`：负责指纹配置
- `EgressIdentityManager`：负责统一解析与调度

保存高级配置时，协调器应同步应用：

1. 更新 `SessionAffinityManager`
2. 更新 `IpReputationManager`
3. 更新 `EgressIdentityManager`
4. 将画像中的 `tls_fingerprint` 与 DNS 策略下发到运行时

### 3.6 API 设计

#### Rust Commands

配置读写不再单独开放命令，而是使用统一高级配置接口。Task 3 单独开放的命令只保留**运行态诊断/预览**：

```rust
// src-tauri/src/cmd/egress_identity.rs

#[tauri::command]
pub async fn egress_identity_preview_match(
    process_name: Option<String>,
    exe_path: Option<String>,
    shortcut_id: Option<String>,
    domain: Option<String>,
) -> Result<ResolvedEgressIdentity, String>

#[tauri::command]
pub async fn egress_identity_get_active_assignments() -> Result<Vec<ResolvedEgressIdentity>, String>

#[tauri::command]
pub async fn egress_identity_clear_assignment(key: String) -> Result<(), String>
```

#### TypeScript Service

```typescript
// src/services/egress-identity.ts

export interface EgressPreviewRequest {
  processName?: string
  exePath?: string
  shortcutId?: string
  domain?: string
}

export interface ResolvedEgressIdentity {
  profileId: string
  selectedNode: string
  dnsMode: 'Inherit' | 'Hijack' | 'Remote'
  tlsFingerprint?: string
  matchedBy: string
}

export async function egressIdentityPreviewMatch(
  request: EgressPreviewRequest
): Promise<ResolvedEgressIdentity>

export async function egressIdentityGetActiveAssignments(): Promise<ResolvedEgressIdentity[]>
export async function egressIdentityClearAssignment(key: string): Promise<void>
```

### 3.7 前端 UI 设计

UI 仍然挂在 `AdvancedPage`，但必须保持**受控模式**：

- `EgressIdentityPanel`
  - 编辑 `localConfig.egress_identity`
  - 不自己调用独立保存接口
- `ActiveAssignmentsPanel`
  - 展示运行时匹配结果
  - 支持清理当前分配

面板最少包含四个区域：

1. **画像列表**：定义出口身份画像
2. **应用规则**：按进程名 / 路径映射画像
3. **快捷方式规则**：为未来软件内直启留入口
4. **活动分配**：查看当前谁命中了哪个画像和节点

### 3.8 分阶段实施方案

#### 第一阶段（当前就可以做）

1. 在 `AdvancedConfig` 中加入 `egress_identity`
2. 实现 `EgressIdentityConfig` / `EgressIdentityManager` 基础结构
3. 接入 `Coordinator` 保存与应用流程
4. 完成 `AdvancedPage` 的配置面板与预览命令

#### 第二阶段（与运行时进一步接轨）

1. 用真实进程信息驱动 `resolve()`
2. 将解析结果写入活动分配表
3. 与现有会话绑定 / IP 信誉度联动完成闭环

#### 第三阶段（未来增强）

1. 软件内部快捷方式直启与绑定
2. 更强的应用识别（签名、路径、参数）
3. 更复杂的画像继承与分层策略

### 3.9 明确不纳入 Task 3 主线的内容

以下内容不再作为本阶段主线目标：

- 浏览器扩展
- WebRTC 专项防护
- Canvas/WebGL 指纹随机化
- 仅浏览器场景下的环境注入

这些能力即使未来存在，也应视为附属增强，而不是代理软件出口身份管理的主框架。

---

**创建时间**: 2025-05-28  
**最后更新**: 2025-05-28  
**作者**: Kiro AI Assistant  
**状态**: 设计中
