# 安全增强 Phase 2 - 设计文档

## 概述

本文档定义安全增强 Phase 2 的技术设计，包括入口隐蔽增强、HTTP头净化、流量填充三大功能。

---

## 架构设计

### 整体架构

```
┌─────────────────────────────────────────────────────────┐
│                    前端 UI 层                            │
│  LocalSecurityMonitor | HeaderSanitization | Padding    │
└────────────┬────────────────────────────────────────────┘
             │
┌────────────▼────────────────────────────────────────────┐
│                  TypeScript 服务层                       │
│  local-security.ts | header-sanitization.ts |           │
│  traffic-padding.ts                                     │
└────────────┬────────────────────────────────────────────┘
             │ Tauri Commands
┌────────────▼────────────────────────────────────────────┐
│                   Rust 后端层                            │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐  │
│  │ Local        │  │ Header       │  │ Traffic      │  │
│  │ Security     │  │ Sanitization │  │ Padding      │  │
│  └──────────────┘  └──────────────┘  └──────────────┘  │
└─────────────────────────────────────────────────────────┘
             │
┌────────────▼────────────────────────────────────────────┐
│                   系统层                                 │
│  Windows Firewall | iptables | pf | Network Stack       │
└─────────────────────────────────────────────────────────┘
```

### 模块依赖关系

```
LocalSecurity
  ├─ BindingMonitor (本地绑定监控)
  ├─ FirewallManager (防火墙管理)
  │   ├─ WindowsFirewall
  │   ├─ LinuxIptables
  │   └─ MacOSPf
  ├─ ProcessStealth (进程隐蔽)
  └─ LeakMonitor (泄漏监控)

HeaderSanitization
  ├─ ProxyHeaderRemover (代理头清除)
  ├─ HeaderForger (头部伪造)
  ├─ HeaderNormalizer (头部规范化)
  └─ BrowserTemplates (浏览器模板)

TrafficPadding
  ├─ PaddingGenerator (填充生成器)
  ├─ PaddingScheduler (填充调度器)
  ├─ SmartPadding (智能填充)
  └─ PerformanceController (性能控制)
```

---

## 功能 1: 入口隐蔽增强

### 1.1 数据结构

```rust
// src-tauri/src/security/local_security.rs

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
    /// 最后检查时间
    pub last_check_time: i64,
    /// 是否检测到泄漏
    pub leak_detected: bool,
    /// 泄漏类型
    pub leak_type: Option<String>,
    /// 是否自动修复
    pub auto_fix_applied: bool,
}

/// 防火墙规则
#[derive(Debug, Clone)]
pub struct FirewallRule {
    pub name: String,
    pub port: u16,
    pub protocol: Protocol,
    pub action: Action,
}

#[derive(Debug, Clone)]
pub enum Protocol {
    TCP,
    UDP,
}

#[derive(Debug, Clone)]
pub enum Action {
    Allow,
    Block,
}
```

### 1.2 核心算法

#### 1.2.1 本地绑定监控

```rust
/// 检查本地绑定是否安全
pub async fn check_local_binding(port: u16) -> Result<bool> {
    // 1. 获取所有网络连接
    let connections = get_network_connections()?;
    
    // 2. 查找指定端口的监听
    let listeners = connections.iter()
        .filter(|c| c.local_port == port && c.state == "LISTEN")
        .collect::<Vec<_>>();
    
    // 3. 检查是否只绑定到 127.0.0.1
    for listener in listeners {
        if listener.local_address != "127.0.0.1" {
            return Ok(false); // 不安全
        }
    }
    
    Ok(true) // 安全
}
```

#### 1.2.2 防火墙规则配置

**Windows (PowerShell)**:
```rust
pub fn configure_windows_firewall(port: u16) -> Result<()> {
    let rule_name = format!("ClashVerge-LocalOnly-{}", port);
    
    // 删除旧规则
    Command::new("powershell")
        .args(&[
            "-Command",
            &format!("Remove-NetFirewallRule -DisplayName '{}' -ErrorAction SilentlyContinue", rule_name)
        ])
        .output()?;
    
    // 添加新规则：允许本地访问
    Command::new("powershell")
        .args(&[
            "-Command",
            &format!(
                "New-NetFirewallRule -DisplayName '{}' -Direction Inbound -LocalAddress 127.0.0.1 -LocalPort {} -Protocol TCP -Action Allow",
                rule_name, port
            )
        ])
        .output()?;
    
    // 添加阻止规则：阻止外部访问
    Command::new("powershell")
        .args(&[
            "-Command",
            &format!(
                "New-NetFirewallRule -DisplayName '{}-Block' -Direction Inbound -LocalPort {} -Protocol TCP -Action Block -RemoteAddress Any",
                rule_name, port
            )
        ])
        .output()?;
    
    Ok(())
}
```

**Linux (iptables)**:
```rust
pub fn configure_linux_firewall(port: u16) -> Result<()> {
    // 允许本地回环访问
    Command::new("iptables")
        .args(&["-A", "INPUT", "-i", "lo", "-j", "ACCEPT"])
        .output()?;
    
    // 阻止外部访问指定端口
    Command::new("iptables")
        .args(&[
            "-A", "INPUT",
            "-p", "tcp",
            "--dport", &port.to_string(),
            "!", "-i", "lo",
            "-j", "DROP"
        ])
        .output()?;
    
    Ok(())
}
```

**macOS (pf)**:
```rust
pub fn configure_macos_firewall(port: u16) -> Result<()> {
    let rules = format!(
        "block in proto tcp from any to any port {}\n\
         pass in proto tcp from 127.0.0.1 to 127.0.0.1 port {}",
        port, port
    );
    
    // 写入规则文件
    std::fs::write("/etc/pf.anchors/clash_verge", rules)?;
    
    // 加载规则
    Command::new("pfctl")
        .args(&["-f", "/etc/pf.anchors/clash_verge"])
        .output()?;
    
    Ok(())
}
```

#### 1.2.3 泄漏监控循环

```rust
pub async fn start_leak_monitor(config: LocalSecurityConfig) -> Result<()> {
    let interval = Duration::from_secs(config.monitor_interval);
    
    loop {
        tokio::time::sleep(interval).await;
        
        // 1. 检查本地绑定
        let binding_secure = check_local_binding(config.port).await?;
        
        // 2. 检查防火墙规则
        let firewall_active = check_firewall_rules(config.port).await?;
        
        // 3. 检查外部访问
        let external_blocked = check_external_access(config.port).await?;
        
        // 4. 生成状态报告
        let status = LeakMonitorStatus {
            local_binding_secure: binding_secure,
            firewall_rules_active: firewall_active,
            process_hidden: true, // TODO: 实现进程检测
            external_access_blocked: external_blocked,
            last_check_time: chrono::Utc::now().timestamp(),
            leak_detected: !binding_secure || !firewall_active || !external_blocked,
            leak_type: None,
            auto_fix_applied: false,
        };
        
        // 5. 如果检测到泄漏，尝试自动修复
        if status.leak_detected && config.auto_firewall {
            auto_fix_leak(&config).await?;
        }
        
        // 6. 发送状态更新
        emit_status_update(status).await?;
    }
}
```

### 1.3 API 设计

#### Rust Commands

```rust
// src-tauri/src/cmd/local_security.rs

#[tauri::command]
pub async fn local_security_get_config() -> Result<LocalSecurityConfig, String> {
    // 获取配置
}

#[tauri::command]
pub async fn local_security_update_config(
    config: LocalSecurityConfig
) -> Result<(), String> {
    // 更新配置
}

#[tauri::command]
pub async fn local_security_start_monitor() -> Result<(), String> {
    // 启动监控
}

#[tauri::command]
pub async fn local_security_stop_monitor() -> Result<(), String> {
    // 停止监控
}

#[tauri::command]
pub async fn local_security_get_status() -> Result<LeakMonitorStatus, String> {
    // 获取状态
}

#[tauri::command]
pub async fn local_security_check_now() -> Result<LeakMonitorStatus, String> {
    // 立即检查
}

#[tauri::command]
pub async fn local_security_fix_leak() -> Result<(), String> {
    // 修复泄漏
}
```

#### TypeScript Service

```typescript
// src/services/local-security.ts

export interface LocalSecurityConfig {
  bindAddress: string;
  portRandomization: boolean;
  portRange: [number, number];
  autoSwitchOnConflict: boolean;
  autoFirewall: boolean;
  processStealth: boolean;
  leakMonitoring: boolean;
  monitorInterval: number;
}

export interface LeakMonitorStatus {
  localBindingSecure: boolean;
  firewallRulesActive: boolean;
  processHidden: boolean;
  externalAccessBlocked: boolean;
  lastCheckTime: number;
  leakDetected: boolean;
  leakType?: string;
  autoFixApplied: boolean;
}

export async function localSecurityGetConfig(): Promise<LocalSecurityConfig>
export async function localSecurityUpdateConfig(config: LocalSecurityConfig): Promise<void>
export async function localSecurityStartMonitor(): Promise<void>
export async function localSecurityStopMonitor(): Promise<void>
export async function localSecurityGetStatus(): Promise<LeakMonitorStatus>
export async function localSecurityCheckNow(): Promise<LeakMonitorStatus>
export async function localSecurityFixLeak(): Promise<void>
```

---

## 功能 2: HTTP 头净化

### 2.1 数据结构

```rust
// src-tauri/src/http/header_sanitization.rs

/// HTTP 头净化配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeaderSanitizationConfig {
    /// 启用净化
    pub enabled: bool,
    /// 清除代理头
    pub remove_proxy_headers: bool,
    /// 自定义要清除的头
    pub custom_headers_to_remove: Vec<String>,
    /// 伪造 User-Agent
    pub forge_user_agent: bool,
    /// 浏览器模板
    pub browser_template: BrowserTemplate,
    /// 自定义 User-Agent
    pub custom_user_agent: Option<String>,
    /// 规范化 Accept 头
    pub normalize_accept: bool,
    /// 规范化头部顺序
    pub normalize_header_order: bool,
}

/// 浏览器模板
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BrowserTemplate {
    Chrome,
    Firefox,
    Safari,
    Edge,
    Custom,
}

/// 浏览器指纹
#[derive(Debug, Clone)]
pub struct BrowserFingerprint {
    pub user_agent: String,
    pub accept: String,
    pub accept_language: String,
    pub accept_encoding: String,
    pub header_order: Vec<String>,
}
```

### 2.2 核心算法

#### 2.2.1 代理头清除

```rust
/// 代理特征头列表
const PROXY_HEADERS: &[&str] = &[
    "X-Forwarded-For",
    "X-Real-IP",
    "Via",
    "Proxy-Connection",
    "X-Proxy-ID",
    "Forwarded",
    "X-Forwarded-Host",
    "X-Forwarded-Proto",
    "X-Forwarded-Server",
];

/// 清除代理头
pub fn remove_proxy_headers(
    headers: &mut HeaderMap,
    config: &HeaderSanitizationConfig
) -> Result<()> {
    // 清除标准代理头
    if config.remove_proxy_headers {
        for header in PROXY_HEADERS {
            headers.remove(*header);
        }
    }
    
    // 清除自定义代理头
    for header in &config.custom_headers_to_remove {
        headers.remove(header);
    }
    
    Ok(())
}
```

#### 2.2.2 浏览器指纹伪造

```rust
/// 获取浏览器指纹
pub fn get_browser_fingerprint(template: &BrowserTemplate) -> BrowserFingerprint {
    match template {
        BrowserTemplate::Chrome => BrowserFingerprint {
            user_agent: "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36".to_string(),
            accept: "text/html,application/xhtml+xml,application/xml;q=0.9,image/avif,image/webp,image/apng,*/*;q=0.8".to_string(),
            accept_language: "en-US,en;q=0.9".to_string(),
            accept_encoding: "gzip, deflate, br".to_string(),
            header_order: vec![
                "Host".to_string(),
                "Connection".to_string(),
                "Upgrade-Insecure-Requests".to_string(),
                "User-Agent".to_string(),
                "Accept".to_string(),
                "Accept-Encoding".to_string(),
                "Accept-Language".to_string(),
            ],
        },
        BrowserTemplate::Firefox => BrowserFingerprint {
            user_agent: "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:121.0) Gecko/20100101 Firefox/121.0".to_string(),
            accept: "text/html,application/xhtml+xml,application/xml;q=0.9,image/avif,image/webp,*/*;q=0.8".to_string(),
            accept_language: "en-US,en;q=0.5".to_string(),
            accept_encoding: "gzip, deflate, br".to_string(),
            header_order: vec![
                "Host".to_string(),
                "User-Agent".to_string(),
                "Accept".to_string(),
                "Accept-Language".to_string(),
                "Accept-Encoding".to_string(),
                "Connection".to_string(),
                "Upgrade-Insecure-Requests".to_string(),
            ],
        },
        BrowserTemplate::Safari => BrowserFingerprint {
            user_agent: "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/17.1 Safari/605.1.15".to_string(),
            accept: "text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8".to_string(),
            accept_language: "en-US,en;q=0.9".to_string(),
            accept_encoding: "gzip, deflate, br".to_string(),
            header_order: vec![
                "Host".to_string(),
                "Accept".to_string(),
                "User-Agent".to_string(),
                "Accept-Language".to_string(),
                "Accept-Encoding".to_string(),
                "Connection".to_string(),
            ],
        },
        _ => get_browser_fingerprint(&BrowserTemplate::Chrome),
    }
}

/// 应用浏览器指纹
pub fn apply_browser_fingerprint(
    headers: &mut HeaderMap,
    fingerprint: &BrowserFingerprint
) -> Result<()> {
    headers.insert("User-Agent", fingerprint.user_agent.parse()?);
    headers.insert("Accept", fingerprint.accept.parse()?);
    headers.insert("Accept-Language", fingerprint.accept_language.parse()?);
    headers.insert("Accept-Encoding", fingerprint.accept_encoding.parse()?);
    headers.insert("DNT", "1".parse()?);
    headers.insert("Upgrade-Insecure-Requests", "1".parse()?);
    
    Ok(())
}
```

#### 2.2.3 头部顺序规范化

```rust
/// 规范化头部顺序
pub fn normalize_header_order(
    headers: &mut HeaderMap,
    order: &[String]
) -> Result<HeaderMap> {
    let mut ordered_headers = HeaderMap::new();
    
    // 按照指定顺序添加头部
    for header_name in order {
        if let Some(value) = headers.get(header_name) {
            ordered_headers.insert(
                header_name.parse()?,
                value.clone()
            );
        }
    }
    
    // 添加剩余的头部
    for (name, value) in headers.iter() {
        if !ordered_headers.contains_key(name) {
            ordered_headers.insert(name.clone(), value.clone());
        }
    }
    
    Ok(ordered_headers)
}
```

### 2.3 API 设计

#### Rust Commands

```rust
// src-tauri/src/cmd/header_sanitization.rs

#[tauri::command]
pub async fn header_sanitization_get_config() -> Result<HeaderSanitizationConfig, String> {
    // 获取配置
}

#[tauri::command]
pub async fn header_sanitization_update_config(
    config: HeaderSanitizationConfig
) -> Result<(), String> {
    // 更新配置
}

#[tauri::command]
pub async fn header_sanitization_test(
    headers: HashMap<String, String>
) -> Result<HashMap<String, String>, String> {
    // 测试净化效果
}

#[tauri::command]
pub async fn header_sanitization_get_templates() -> Result<Vec<String>, String> {
    // 获取浏览器模板列表
}
```

#### TypeScript Service

```typescript
// src/services/header-sanitization.ts

export interface HeaderSanitizationConfig {
  enabled: boolean;
  removeProxyHeaders: boolean;
  customHeadersToRemove: string[];
  forgeUserAgent: boolean;
  browserTemplate: 'chrome' | 'firefox' | 'safari' | 'edge' | 'custom';
  customUserAgent?: string;
  normalizeAccept: boolean;
  normalizeHeaderOrder: boolean;
}

export async function headerSanitizationGetConfig(): Promise<HeaderSanitizationConfig>
export async function headerSanitizationUpdateConfig(config: HeaderSanitizationConfig): Promise<void>
export async function headerSanitizationTest(headers: Record<string, string>): Promise<Record<string, string>>
export async function headerSanitizationGetTemplates(): Promise<string[]>
```

---

## 功能 3: 流量填充

### 3.1 数据结构

```rust
// src-tauri/src/traffic/padding.rs

/// 流量填充配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrafficPaddingConfig {
    /// 启用填充
    pub enabled: bool,
    /// 最小填充大小（字节）
    pub min_size: usize,
    /// 最大填充大小（字节）
    pub max_size: usize,
    /// 加密填充数据
    pub encrypt: bool,
    /// 填充强度
    pub intensity: PaddingIntensity,
    /// 填充频率
    pub frequency: PaddingFrequency,
    /// 填充时机
    pub timing: PaddingTiming,
    /// 填充目标
    pub targets: Vec<String>,
    /// 智能填充
    pub smart_padding: bool,
    /// 性能控制
    pub performance_control: PerformanceControl,
}

/// 填充强度
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PaddingIntensity {
    Low,
    Medium,
    High,
    Custom(f32),
}

/// 填充频率
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaddingFrequency {
    pub freq_type: FrequencyType,
    pub interval: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FrequencyType {
    Time,      // 每 N 秒
    Request,   // 每 N 请求
    Random,    // 随机
}

/// 填充时机
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PaddingTiming {
    Before,    // 请求前
    After,     // 请求后
    Random,    // 随机
}

/// 性能控制
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceControl {
    /// 最大带宽（字节/秒）
    pub max_bandwidth: usize,
    /// 最大 CPU 使用率（%）
    pub max_cpu_usage: f32,
    /// 最大内存（字节）
    pub max_memory: usize,
    /// 自动降级
    pub auto_downgrade: bool,
}

/// 填充统计
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaddingStats {
    /// 填充次数
    pub padding_count: u64,
    /// 填充总大小（字节）
    pub total_padding_size: u64,
    /// 带宽占用（字节/秒）
    pub bandwidth_usage: f32,
    /// CPU 占用（%）
    pub cpu_usage: f32,
    /// 内存占用（字节）
    pub memory_usage: usize,
}
```

### 3.2 核心算法

#### 3.2.1 随机填充数据生成

```rust
use rand::Rng;
use ring::aead::{Aad, LessSafeKey, Nonce, UnboundKey, AES_256_GCM};

/// 生成随机填充数据
pub fn generate_padding_data(
    size: usize,
    encrypt: bool
) -> Result<Vec<u8>> {
    let mut rng = rand::thread_rng();
    
    // 生成随机数据
    let mut data = vec![0u8; size];
    rng.fill(&mut data[..]);
    
    // 如果需要加密
    if encrypt {
        data = encrypt_padding_data(&data)?;
    }
    
    Ok(data)
}

/// 加密填充数据
fn encrypt_padding_data(data: &[u8]) -> Result<Vec<u8>> {
    // 生成随机密钥
    let mut key_bytes = [0u8; 32];
    rand::thread_rng().fill(&mut key_bytes);
    
    let unbound_key = UnboundKey::new(&AES_256_GCM, &key_bytes)?;
    let key = LessSafeKey::new(unbound_key);
    
    // 生成随机 nonce
    let mut nonce_bytes = [0u8; 12];
    rand::thread_rng().fill(&mut nonce_bytes);
    let nonce = Nonce::assume_unique_for_key(nonce_bytes);
    
    // 加密
    let mut in_out = data.to_vec();
    key.seal_in_place_append_tag(nonce, Aad::empty(), &mut in_out)?;
    
    Ok(in_out)
}
```

#### 3.2.2 智能填充算法

```rust
/// 计算智能填充大小
pub fn calculate_smart_padding_size(
    current_traffic: f32,      // 当前流量（字节/秒）
    network_latency: f32,      // 网络延迟（毫秒）
    bandwidth_usage: f32,      // 带宽使用率（0-1）
    base_size: usize,          // 基础填充大小
) -> usize {
    // 流量越小，填充越多
    let traffic_factor = 1.0 - (current_traffic / 1_000_000.0).min(1.0);
    
    // 延迟越高，填充越少
    let latency_factor = 1.0 - (network_latency / 1000.0).min(1.0);
    
    // 带宽使用率越高，填充越少
    let bandwidth_factor = 1.0 - bandwidth_usage.min(1.0);
    
    // 计算最终填充大小
    let size = (base_size as f32) 
        * traffic_factor 
        * latency_factor 
        * bandwidth_factor;
    
    size.max(0.0) as usize
}
```

#### 3.2.3 填充调度器

```rust
/// 填充调度器
pub struct PaddingScheduler {
    config: TrafficPaddingConfig,
    stats: Arc<Mutex<PaddingStats>>,
    running: Arc<AtomicBool>,
}

impl PaddingScheduler {
    pub fn new(config: TrafficPaddingConfig) -> Self {
        Self {
            config,
            stats: Arc::new(Mutex::new(PaddingStats::default())),
            running: Arc::new(AtomicBool::new(false)),
        }
    }
    
    pub async fn start(&self) -> Result<()> {
        self.running.store(true, Ordering::SeqCst);
        
        match self.config.frequency.freq_type {
            FrequencyType::Time => self.schedule_by_time().await,
            FrequencyType::Request => self.schedule_by_request().await,
            FrequencyType::Random => self.schedule_randomly().await,
        }
    }
    
    async fn schedule_by_time(&self) -> Result<()> {
        let interval = Duration::from_secs(self.config.frequency.interval);
        
        while self.running.load(Ordering::SeqCst) {
            tokio::time::sleep(interval).await;
            self.send_padding().await?;
        }
        
        Ok(())
    }
    
    async fn send_padding(&self) -> Result<()> {
        // 检查性能限制
        if !self.check_performance_limits().await? {
            if self.config.performance_control.auto_downgrade {
                // 自动降级
                self.downgrade_intensity().await?;
            }
            return Ok(());
        }
        
        // 计算填充大小
        let size = if self.config.smart_padding {
            self.calculate_smart_size().await?
        } else {
            rand::thread_rng().gen_range(
                self.config.min_size..=self.config.max_size
            )
        };
        
        // 生成填充数据
        let data = generate_padding_data(size, self.config.encrypt)?;
        
        // 发送填充数据
        self.send_padding_data(&data).await?;
        
        // 更新统计
        self.update_stats(size).await?;
        
        Ok(())
    }
}
```

### 3.3 API 设计

#### Rust Commands

```rust
// src-tauri/src/cmd/traffic_padding.rs

#[tauri::command]
pub async fn traffic_padding_get_config() -> Result<TrafficPaddingConfig, String> {
    // 获取配置
}

#[tauri::command]
pub async fn traffic_padding_update_config(
    config: TrafficPaddingConfig
) -> Result<(), String> {
    // 更新配置
}

#[tauri::command]
pub async fn traffic_padding_start() -> Result<(), String> {
    // 启动填充
}

#[tauri::command]
pub async fn traffic_padding_stop() -> Result<(), String> {
    // 停止填充
}

#[tauri::command]
pub async fn traffic_padding_get_stats() -> Result<PaddingStats, String> {
    // 获取统计
}

#[tauri::command]
pub async fn traffic_padding_reset_stats() -> Result<(), String> {
    // 重置统计
}
```

#### TypeScript Service

```typescript
// src/services/traffic-padding.ts

export interface TrafficPaddingConfig {
  enabled: boolean;
  minSize: number;
  maxSize: number;
  encrypt: boolean;
  intensity: 'low' | 'medium' | 'high' | { custom: number };
  frequency: {
    type: 'time' | 'request' | 'random';
    interval: number;
  };
  timing: 'before' | 'after' | 'random';
  targets: string[];
  smartPadding: boolean;
  performanceControl: {
    maxBandwidth: number;
    maxCpuUsage: number;
    maxMemory: number;
    autoDowngrade: boolean;
  };
}

export interface PaddingStats {
  paddingCount: number;
  totalPaddingSize: number;
  bandwidthUsage: number;
  cpuUsage: number;
  memoryUsage: number;
}

export async function trafficPaddingGetConfig(): Promise<TrafficPaddingConfig>
export async function trafficPaddingUpdateConfig(config: TrafficPaddingConfig): Promise<void>
export async function trafficPaddingStart(): Promise<void>
export async function trafficPaddingStop(): Promise<void>
export async function trafficPaddingGetStats(): Promise<PaddingStats>
export async function trafficPaddingResetStats(): Promise<void>
```

---

## UI 组件设计

### 1. 入口隐蔽监控卡片

```typescript
// src/components/security/local-security-monitor.tsx

export function LocalSecurityMonitor() {
  const [status, setStatus] = useState<LeakMonitorStatus | null>(null);
  const [config, setConfig] = useState<LocalSecurityConfig | null>(null);
  
  useEffect(() => {
    // 加载配置和状态
    loadConfigAndStatus();
    
    // 订阅状态更新
    const unlisten = listen('leak-monitor-status', (event) => {
      setStatus(event.payload);
    });
    
    return () => {
      unlisten.then(fn => fn());
    };
  }, []);
  
  return (
    <Card>
      <CardHeader>
        <CardTitle>入口隐蔽监控</CardTitle>
      </CardHeader>
      <CardContent>
        <div className="space-y-4">
          {/* 状态指示器 */}
          <StatusIndicator status={status} />
          
          {/* 详细信息 */}
          <DetailedInfo status={status} />
          
          {/* 操作按钮 */}
          <div className="flex gap-2">
            <Button onClick={handleCheckNow}>立即检查</Button>
            <Button onClick={handleViewLogs}>查看日志</Button>
          </div>
        </div>
      </CardContent>
    </Card>
  );
}
```

### 2. HTTP 头净化配置

```typescript
// src/components/settings/header-sanitization-config.tsx

export function HeaderSanitizationConfig() {
  const [config, setConfig] = useState<HeaderSanitizationConfig | null>(null);
  const [testResult, setTestResult] = useState<Record<string, string> | null>(null);
  
  const handleTest = async () => {
    const sampleHeaders = {
      'User-Agent': 'Test',
      'X-Forwarded-For': '1.2.3.4',
      'Via': 'proxy',
    };
    
    const result = await headerSanitizationTest(sampleHeaders);
    setTestResult(result);
  };
  
  return (
    <Card>
      <CardHeader>
        <CardTitle>HTTP 头净化</CardTitle>
      </CardHeader>
      <CardContent>
        <div className="space-y-4">
          {/* 启用开关 */}
          <Switch
            checked={config?.enabled}
            onCheckedChange={(checked) => 
              updateConfig({ ...config, enabled: checked })
            }
          />
          
          {/* 浏览器模板选择 */}
          <Select
            value={config?.browserTemplate}
            onValueChange={(value) =>
              updateConfig({ ...config, browserTemplate: value })
            }
          >
            <SelectItem value="chrome">Chrome</SelectItem>
            <SelectItem value="firefox">Firefox</SelectItem>
            <SelectItem value="safari">Safari</SelectItem>
          </Select>
          
          {/* User-Agent 预览 */}
          <TextField
            label="User-Agent"
            value={getUserAgentPreview(config?.browserTemplate)}
            disabled
          />
          
          {/* 测试按钮 */}
          <Button onClick={handleTest}>测试净化效果</Button>
          
          {/* 测试结果 */}
          {testResult && <TestResult result={testResult} />}
        </div>
      </CardContent>
    </Card>
  );
}
```

### 3. 流量填充配置

```typescript
// src/components/settings/traffic-padding-config.tsx

export function TrafficPaddingConfig() {
  const [config, setConfig] = useState<TrafficPaddingConfig | null>(null);
  const [stats, setStats] = useState<PaddingStats | null>(null);
  
  useEffect(() => {
    // 定期更新统计
    const interval = setInterval(async () => {
      const newStats = await trafficPaddingGetStats();
      setStats(newStats);
    }, 1000);
    
    return () => clearInterval(interval);
  }, []);
  
  return (
    <Card>
      <CardHeader>
        <CardTitle>流量填充</CardTitle>
      </CardHeader>
      <CardContent>
        <div className="space-y-4">
          {/* 启用开关 */}
          <Switch
            checked={config?.enabled}
            onCheckedChange={(checked) =>
              updateConfig({ ...config, enabled: checked })
            }
          />
          
          {/* 填充强度滑块 */}
          <div>
            <label>填充强度</label>
            <Slider
              value={getIntensityValue(config?.intensity)}
              onValueChange={(value) =>
                updateConfig({ ...config, intensity: getIntensityFromValue(value) })
              }
              min={0}
              max={2}
              step={1}
            />
            <div className="flex justify-between text-sm">
              <span>低</span>
              <span>中</span>
              <span>高</span>
            </div>
          </div>
          
          {/* 填充频率 */}
          <TextField
            label="填充频率（秒）"
            type="number"
            value={config?.frequency.interval}
            onChange={(e) =>
              updateConfig({
                ...config,
                frequency: { ...config.frequency, interval: Number(e.target.value) }
              })
            }
          />
          
          {/* 智能填充开关 */}
          <Switch
            label="智能填充（根据流量自动调整）"
            checked={config?.smartPadding}
            onCheckedChange={(checked) =>
              updateConfig({ ...config, smartPadding: checked })
            }
          />
          
          {/* 性能限制 */}
          <div className="space-y-2">
            <h4>性能限制</h4>
            <TextField
              label="最大带宽（MB/s）"
              type="number"
              value={config?.performanceControl.maxBandwidth / 1048576}
              onChange={(e) =>
                updatePerformanceControl('maxBandwidth', Number(e.target.value) * 1048576)
              }
            />
            <TextField
              label="最大 CPU（%）"
              type="number"
              value={config?.performanceControl.maxCpuUsage}
              onChange={(e) =>
                updatePerformanceControl('maxCpuUsage', Number(e.target.value))
              }
            />
          </div>
          
          {/* 统计信息 */}
          <div className="space-y-2">
            <h4>统计</h4>
            <div>今日填充: {formatBytes(stats?.totalPaddingSize)}</div>
            <div>填充次数: {stats?.paddingCount.toLocaleString()}</div>
            <div>带宽占用: {formatBytes(stats?.bandwidthUsage)}/s</div>
            <div>CPU 占用: {stats?.cpuUsage.toFixed(1)}%</div>
          </div>
          
          {/* 查看详细统计按钮 */}
          <Button onClick={handleViewDetailedStats}>查看详细统计</Button>
        </div>
      </CardContent>
    </Card>
  );
}
```

---

## 数据流设计

### 1. 入口隐蔽监控数据流

```
┌─────────────────────────────────────────────────────────┐
│                    启动监控                              │
└────────────┬────────────────────────────────────────────┘
             │
┌────────────▼────────────────────────────────────────────┐
│              定时检查循环（每 30 秒）                     │
└────────────┬────────────────────────────────────────────┘
             │
    ┌────────┼────────┬────────────┐
    │        │        │            │
┌───▼───┐ ┌─▼──┐ ┌──▼───┐ ┌──────▼──────┐
│本地绑定│ │防火墙│ │进程  │ │外部访问检测│
│检查   │ │检查 │ │检查  │ │            │
└───┬───┘ └─┬──┘ └──┬───┘ └──────┬──────┘
    │       │       │            │
    └───────┴───────┴────────────┘
             │
┌────────────▼────────────────────────────────────────────┐
│              生成状态报告                                 │
└────────────┬────────────────────────────────────────────┘
             │
        ┌────┴────┐
        │泄漏检测？│
        └────┬────┘
             │
      ┌──────┴──────┐
      │是           │否
      ▼             ▼
┌─────────┐   ┌─────────┐
│自动修复  │   │发送状态  │
│（可选）  │   │更新     │
└─────┬───┘   └─────┬───┘
      │             │
      └──────┬──────┘
             │
┌────────────▼────────────────────────────────────────────┐
│              通知前端 UI                                  │
└─────────────────────────────────────────────────────────┘
```

### 2. HTTP 头净化数据流

```
┌─────────────────────────────────────────────────────────┐
│              HTTP 请求拦截                                │
└────────────┬────────────────────────────────────────────┘
             │
┌────────────▼────────────────────────────────────────────┐
│              解析请求头                                   │
└────────────┬────────────────────────────────────────────┘
             │
┌────────────▼────────────────────────────────────────────┐
│              清除代理特征头                               │
│  • X-Forwarded-For                                      │
│  • Via                                                  │
│  • Proxy-Connection                                     │
└────────────┬────────────────────────────────────────────┘
             │
┌────────────▼────────────────────────────────────────────┐
│              应用浏览器指纹                               │
│  • User-Agent                                           │
│  • Accept                                               │
│  • Accept-Language                                      │
└────────────┬────────────────────────────────────────────┘
             │
┌────────────▼────────────────────────────────────────────┐
│              规范化头部顺序                               │
└────────────┬────────────────────────────────────────────┘
             │
┌────────────▼────────────────────────────────────────────┐
│              发送净化后的请求                             │
└─────────────────────────────────────────────────────────┘
```

### 3. 流量填充数据流

```
┌─────────────────────────────────────────────────────────┐
│              启动填充调度器                               │
└────────────┬────────────────────────────────────────────┘
             │
┌────────────▼────────────────────────────────────────────┐
│              等待触发条件                                 │
│  • 时间间隔                                              │
│  • 请求计数                                              │
│  • 随机触发                                              │
└────────────┬────────────────────────────────────────────┘
             │
┌────────────▼────────────────────────────────────────────┐
│              检查性能限制                                 │
│  • 带宽使用率                                            │
│  • CPU 使用率                                            │
│  • 内存使用量                                            │
└────────────┬────────────────────────────────────────────┘
             │
        ┌────┴────┐
        │超限？    │
        └────┬────┘
             │
      ┌──────┴──────┐
      │是           │否
      ▼             ▼
┌─────────┐   ┌─────────────┐
│自动降级  │   │计算填充大小  │
│（可选）  │   │（智能/固定） │
└─────┬───┘   └─────┬───────┘
      │             │
      └──────┬──────┘
             │
┌────────────▼────────────────────────────────────────────┐
│              生成填充数据                                 │
│  • 随机数据                                              │
│  • 加密（可选）                                          │
└────────────┬────────────────────────────────────────────┘
             │
┌────────────▼────────────────────────────────────────────┐
│              发送填充数据                                 │
└────────────┬────────────────────────────────────────────┘
             │
┌────────────▼────────────────────────────────────────────┐
│              更新统计信息                                 │
└─────────────────────────────────────────────────────────┘
```

---

## 配置存储设计

### 配置文件结构

```yaml
# config/security-phase2.yaml

local_security:
  bind_address: "127.0.0.1"
  port_randomization: false
  port_range: [10800, 10900]
  auto_switch_on_conflict: true
  auto_firewall: true
  process_stealth: true
  leak_monitoring: true
  monitor_interval: 30

header_sanitization:
  enabled: true
  remove_proxy_headers: true
  custom_headers_to_remove:
    - "X-Custom-Proxy"
  forge_user_agent: true
  browser_template: "chrome"
  custom_user_agent: null
  normalize_accept: true
  normalize_header_order: true

traffic_padding:
  enabled: true
  min_size: 100
  max_size: 1024
  encrypt: true
  intensity: "medium"
  frequency:
    type: "time"
    interval: 5
  timing: "random"
  targets:
    - "*.google.com"
    - "*.youtube.com"
  smart_padding: true
  performance_control:
    max_bandwidth: 1048576  # 1 MB/s
    max_cpu_usage: 10.0
    max_memory: 104857600   # 100 MB
    auto_downgrade: true
```

### 配置加密

```rust
use ring::aead::{Aad, LessSafeKey, Nonce, UnboundKey, AES_256_GCM};

/// 加密配置
pub fn encrypt_config(config: &str, key: &[u8]) -> Result<Vec<u8>> {
    let unbound_key = UnboundKey::new(&AES_256_GCM, key)?;
    let key = LessSafeKey::new(unbound_key);
    
    let mut nonce_bytes = [0u8; 12];
    rand::thread_rng().fill(&mut nonce_bytes);
    let nonce = Nonce::assume_unique_for_key(nonce_bytes);
    
    let mut in_out = config.as_bytes().to_vec();
    key.seal_in_place_append_tag(nonce, Aad::empty(), &mut in_out)?;
    
    // 将 nonce 和密文组合
    let mut result = nonce_bytes.to_vec();
    result.extend_from_slice(&in_out);
    
    Ok(result)
}

/// 解密配置
pub fn decrypt_config(encrypted: &[u8], key: &[u8]) -> Result<String> {
    let unbound_key = UnboundKey::new(&AES_256_GCM, key)?;
    let key = LessSafeKey::new(unbound_key);
    
    // 提取 nonce 和密文
    let nonce_bytes = &encrypted[..12];
    let nonce = Nonce::assume_unique_for_key(*nonce_bytes.try_into()?);
    
    let mut in_out = encrypted[12..].to_vec();
    let plaintext = key.open_in_place(nonce, Aad::empty(), &mut in_out)?;
    
    Ok(String::from_utf8(plaintext.to_vec())?)
}
```

---

## 错误处理设计

### 错误类型定义

```rust
// src-tauri/src/error.rs

#[derive(Debug, thiserror::Error)]
pub enum SecurityError {
    #[error("本地绑定检查失败: {0}")]
    BindingCheckFailed(String),
    
    #[error("防火墙配置失败: {0}")]
    FirewallConfigFailed(String),
    
    #[error("权限不足: {0}")]
    PermissionDenied(String),
    
    #[error("泄漏检测失败: {0}")]
    LeakDetectionFailed(String),
    
    #[error("HTTP 头净化失败: {0}")]
    HeaderSanitizationFailed(String),
    
    #[error("流量填充失败: {0}")]
    TrafficPaddingFailed(String),
    
    #[error("配置加载失败: {0}")]
    ConfigLoadFailed(String),
    
    #[error("配置保存失败: {0}")]
    ConfigSaveFailed(String),
    
    #[error("加密失败: {0}")]
    EncryptionFailed(String),
    
    #[error("解密失败: {0}")]
    DecryptionFailed(String),
    
    #[error("IO 错误: {0}")]
    IoError(#[from] std::io::Error),
    
    #[error("序列化错误: {0}")]
    SerializationError(#[from] serde_json::Error),
}

pub type Result<T> = std::result::Result<T, SecurityError>;
```

### 错误恢复策略

```rust
/// 错误恢复策略
pub async fn handle_security_error(error: SecurityError) -> Result<()> {
    match error {
        SecurityError::FirewallConfigFailed(_) => {
            // 防火墙配置失败，尝试手动配置
            log::warn!("防火墙自动配置失败，请手动配置");
            notify_user("防火墙配置失败，请手动配置").await?;
            Ok(())
        }
        
        SecurityError::PermissionDenied(_) => {
            // 权限不足，提示用户
            log::error!("权限不足，某些功能可能无法使用");
            notify_user("需要管理员权限才能配置防火墙").await?;
            Ok(())
        }
        
        SecurityError::LeakDetectionFailed(_) => {
            // 泄漏检测失败，继续运行但记录日志
            log::warn!("泄漏检测失败，将在下次检查时重试");
            Ok(())
        }
        
        SecurityError::ConfigLoadFailed(_) => {
            // 配置加载失败，使用默认配置
            log::warn!("配置加载失败，使用默认配置");
            load_default_config().await?;
            Ok(())
        }
        
        _ => {
            // 其他错误，记录日志并返回
            log::error!("安全功能错误: {}", error);
            Err(error)
        }
    }
}
```

---

## 日志设计

### 日志级别

```rust
// TRACE: 详细的调试信息
log::trace!("检查本地绑定: port={}", port);

// DEBUG: 调试信息
log::debug!("防火墙规则已配置: rule={:?}", rule);

// INFO: 一般信息
log::info!("入口隐蔽监控已启动");

// WARN: 警告信息
log::warn!("检测到潜在泄漏: type={}", leak_type);

// ERROR: 错误信息
log::error!("防火墙配置失败: {}", error);
```

### 日志格式

```
[2026-05-28 12:34:56.789] [INFO] [local_security] 入口隐蔽监控已启动
[2026-05-28 12:35:26.123] [DEBUG] [local_security] 本地绑定检查: port=10808, secure=true
[2026-05-28 12:35:26.456] [DEBUG] [local_security] 防火墙规则检查: active=true
[2026-05-28 12:35:26.789] [INFO] [local_security] 泄漏检查完成: leak_detected=false
[2026-05-28 12:36:00.123] [INFO] [header_sanitization] HTTP 头净化已启用: template=chrome
[2026-05-28 12:36:05.456] [INFO] [traffic_padding] 流量填充已启动: intensity=medium
[2026-05-28 12:36:10.789] [DEBUG] [traffic_padding] 发送填充数据: size=512 bytes
```

### 日志文件

```
logs/
├── security-phase2.log          # 主日志文件
├── security-phase2.log.1        # 轮转日志 1
├── security-phase2.log.2        # 轮转日志 2
└── security-phase2-error.log    # 错误日志
```

---

## 性能优化

### 1. 本地绑定检查优化

```rust
// 使用缓存减少系统调用
pub struct BindingCache {
    cache: Arc<Mutex<HashMap<u16, (bool, Instant)>>>,
    ttl: Duration,
}

impl BindingCache {
    pub async fn check_binding(&self, port: u16) -> Result<bool> {
        let mut cache = self.cache.lock().await;
        
        // 检查缓存
        if let Some((secure, timestamp)) = cache.get(&port) {
            if timestamp.elapsed() < self.ttl {
                return Ok(*secure);
            }
        }
        
        // 缓存过期，重新检查
        let secure = check_local_binding_impl(port).await?;
        cache.insert(port, (secure, Instant::now()));
        
        Ok(secure)
    }
}
```

### 2. HTTP 头净化优化

```rust
// 使用预编译的浏览器指纹
lazy_static! {
    static ref BROWSER_FINGERPRINTS: HashMap<BrowserTemplate, BrowserFingerprint> = {
        let mut map = HashMap::new();
        map.insert(BrowserTemplate::Chrome, get_chrome_fingerprint());
        map.insert(BrowserTemplate::Firefox, get_firefox_fingerprint());
        map.insert(BrowserTemplate::Safari, get_safari_fingerprint());
        map
    };
}

pub fn get_fingerprint(template: &BrowserTemplate) -> &BrowserFingerprint {
    BROWSER_FINGERPRINTS.get(template).unwrap()
}
```

### 3. 流量填充优化

```rust
// 使用对象池减少内存分配
pub struct PaddingDataPool {
    pool: Arc<Mutex<Vec<Vec<u8>>>>,
    max_size: usize,
}

impl PaddingDataPool {
    pub async fn get(&self, size: usize) -> Vec<u8> {
        let mut pool = self.pool.lock().await;
        
        // 尝试从池中获取
        if let Some(mut data) = pool.pop() {
            data.resize(size, 0);
            rand::thread_rng().fill(&mut data[..]);
            return data;
        }
        
        // 池为空，创建新数据
        let mut data = vec![0u8; size];
        rand::thread_rng().fill(&mut data[..]);
        data
    }
    
    pub async fn return_data(&self, data: Vec<u8>) {
        let mut pool = self.pool.lock().await;
        
        // 如果池未满，归还数据
        if pool.len() < self.max_size {
            pool.push(data);
        }
    }
}
```

---

## 测试策略

### 1. 单元测试

```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_local_binding_check() {
        // 测试本地绑定检查
        let result = check_local_binding(10808).await;
        assert!(result.is_ok());
    }
    
    #[test]
    fn test_remove_proxy_headers() {
        // 测试代理头清除
        let mut headers = HeaderMap::new();
        headers.insert("X-Forwarded-For", "1.2.3.4".parse().unwrap());
        headers.insert("User-Agent", "Test".parse().unwrap());
        
        let config = HeaderSanitizationConfig::default();
        remove_proxy_headers(&mut headers, &config).unwrap();
        
        assert!(!headers.contains_key("X-Forwarded-For"));
        assert!(headers.contains_key("User-Agent"));
    }
    
    #[test]
    fn test_generate_padding_data() {
        // 测试填充数据生成
        let data = generate_padding_data(512, false).unwrap();
        assert_eq!(data.len(), 512);
    }
    
    #[test]
    fn test_smart_padding_calculation() {
        // 测试智能填充计算
        let size = calculate_smart_padding_size(
            100_000.0,  // 100 KB/s
            50.0,       // 50ms
            0.5,        // 50%
            512,        // 512 bytes
        );
        assert!(size > 0);
        assert!(size <= 512);
    }
}
```

### 2. 集成测试

```rust
#[cfg(test)]
mod integration_tests {
    use super::*;
    
    #[tokio::test]
    async fn test_leak_monitor_workflow() {
        // 测试完整的泄漏监控流程
        let config = LocalSecurityConfig::default();
        
        // 启动监控
        let handle = tokio::spawn(async move {
            start_leak_monitor(config).await
        });
        
        // 等待一段时间
        tokio::time::sleep(Duration::from_secs(5)).await;
        
        // 获取状态
        let status = local_security_get_status().await.unwrap();
        assert!(!status.leak_detected);
        
        // 停止监控
        handle.abort();
    }
    
    #[tokio::test]
    async fn test_header_sanitization_workflow() {
        // 测试完整的 HTTP 头净化流程
        let config = HeaderSanitizationConfig {
            enabled: true,
            remove_proxy_headers: true,
            browser_template: BrowserTemplate::Chrome,
            ..Default::default()
        };
        
        // 更新配置
        header_sanitization_update_config(config).await.unwrap();
        
        // 测试净化
        let mut headers = HashMap::new();
        headers.insert("X-Forwarded-For".to_string(), "1.2.3.4".to_string());
        headers.insert("User-Agent".to_string(), "Test".to_string());
        
        let result = header_sanitization_test(headers).await.unwrap();
        
        assert!(!result.contains_key("X-Forwarded-For"));
        assert!(result.get("User-Agent").unwrap().contains("Chrome"));
    }
    
    #[tokio::test]
    async fn test_traffic_padding_workflow() {
        // 测试完整的流量填充流程
        let config = TrafficPaddingConfig {
            enabled: true,
            intensity: PaddingIntensity::Low,
            frequency: PaddingFrequency {
                freq_type: FrequencyType::Time,
                interval: 1,
            },
            ..Default::default()
        };
        
        // 更新配置
        traffic_padding_update_config(config).await.unwrap();
        
        // 启动填充
        traffic_padding_start().await.unwrap();
        
        // 等待一段时间
        tokio::time::sleep(Duration::from_secs(3)).await;
        
        // 获取统计
        let stats = traffic_padding_get_stats().await.unwrap();
        assert!(stats.padding_count > 0);
        
        // 停止填充
        traffic_padding_stop().await.unwrap();
    }
}
```

### 3. 性能测试

```rust
#[cfg(test)]
mod performance_tests {
    use super::*;
    use std::time::Instant;
    
    #[test]
    fn bench_local_binding_check() {
        let start = Instant::now();
        for _ in 0..1000 {
            let _ = check_local_binding(10808);
        }
        let elapsed = start.elapsed();
        println!("1000 次本地绑定检查耗时: {:?}", elapsed);
        assert!(elapsed.as_millis() < 1000); // < 1ms per check
    }
    
    #[test]
    fn bench_header_sanitization() {
        let mut headers = HeaderMap::new();
        headers.insert("X-Forwarded-For", "1.2.3.4".parse().unwrap());
        headers.insert("User-Agent", "Test".parse().unwrap());
        
        let config = HeaderSanitizationConfig::default();
        
        let start = Instant::now();
        for _ in 0..10000 {
            let mut h = headers.clone();
            let _ = remove_proxy_headers(&mut h, &config);
        }
        let elapsed = start.elapsed();
        println!("10000 次 HTTP 头净化耗时: {:?}", elapsed);
        assert!(elapsed.as_millis() < 100); // < 0.01ms per sanitization
    }
    
    #[test]
    fn bench_padding_generation() {
        let start = Instant::now();
        for _ in 0..1000 {
            let _ = generate_padding_data(512, false);
        }
        let elapsed = start.elapsed();
        println!("1000 次填充数据生成耗时: {:?}", elapsed);
        assert!(elapsed.as_millis() < 1000); // < 1ms per generation
    }
}
```

---

## 安全考虑

### 1. 权限管理

```rust
/// 检查是否有管理员权限
pub fn check_admin_privileges() -> bool {
    #[cfg(target_os = "windows")]
    {
        use windows::Win32::Security::IsUserAnAdmin;
        unsafe { IsUserAnAdmin().as_bool() }
    }
    
    #[cfg(target_os = "linux")]
    {
        use nix::unistd::Uid;
        Uid::effective().is_root()
    }
    
    #[cfg(target_os = "macos")]
    {
        use nix::unistd::Uid;
        Uid::effective().is_root()
    }
}

/// 请求管理员权限
pub async fn request_admin_privileges() -> Result<()> {
    if check_admin_privileges() {
        return Ok(());
    }
    
    #[cfg(target_os = "windows")]
    {
        // Windows: 使用 UAC 提升权限
        use std::os::windows::process::CommandExt;
        Command::new("powershell")
            .args(&["-Command", "Start-Process", "-Verb", "RunAs"])
            .creation_flags(0x08000000) // CREATE_NO_WINDOW
            .spawn()?;
    }
    
    #[cfg(target_os = "linux")]
    {
        // Linux: 使用 pkexec 或 sudo
        Command::new("pkexec")
            .arg(std::env::current_exe()?)
            .spawn()?;
    }
    
    #[cfg(target_os = "macos")]
    {
        // macOS: 使用 osascript
        Command::new("osascript")
            .args(&[
                "-e",
                "do shell script \"sudo true\" with administrator privileges"
            ])
            .spawn()?;
    }
    
    Ok(())
}
```

### 2. 配置验证

```rust
/// 验证配置安全性
pub fn validate_config(config: &LocalSecurityConfig) -> Result<()> {
    // 验证绑定地址
    if config.bind_address != "127.0.0.1" && config.bind_address != "localhost" {
        return Err(SecurityError::ConfigLoadFailed(
            "绑定地址必须是 127.0.0.1 或 localhost".to_string()
        ));
    }
    
    // 验证端口范围
    if config.port_range.0 < 1024 || config.port_range.1 > 65535 {
        return Err(SecurityError::ConfigLoadFailed(
            "端口范围必须在 1024-65535 之间".to_string()
        ));
    }
    
    // 验证监控间隔
    if config.monitor_interval < 10 || config.monitor_interval > 3600 {
        return Err(SecurityError::ConfigLoadFailed(
            "监控间隔必须在 10-3600 秒之间".to_string()
        ));
    }
    
    Ok(())
}
```

### 3. 敏感数据处理

```rust
/// 脱敏日志输出
pub fn sanitize_log(message: &str) -> String {
    // 移除 IP 地址
    let re_ip = regex::Regex::new(r"\d{1,3}\.\d{1,3}\.\d{1,3}\.\d{1,3}").unwrap();
    let message = re_ip.replace_all(message, "***.***.***.**");
    
    // 移除密钥
    let re_key = regex::Regex::new(r"key=[\w\d]+").unwrap();
    let message = re_key.replace_all(&message, "key=***");
    
    message.to_string()
}

/// 安全清除内存
pub fn secure_zero_memory(data: &mut [u8]) {
    use std::sync::atomic::{compiler_fence, Ordering};
    
    for byte in data.iter_mut() {
        *byte = 0;
    }
    
    // 防止编译器优化掉清零操作
    compiler_fence(Ordering::SeqCst);
}
```

---

## 部署考虑

### 1. 系统要求

**Windows**:
- Windows 10/11
- 管理员权限（配置防火墙）
- PowerShell 5.1+

**Linux**:
- Ubuntu 20.04+ / Debian 11+
- root 权限或 sudo（配置 iptables）
- iptables 或 nftables

**macOS**:
- macOS 11+
- root 权限或 sudo（配置 pf）
- pf 防火墙

### 2. 依赖项

```toml
# Cargo.toml

[dependencies]
tokio = { version = "1.35", features = ["full"] }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
serde_yaml = "0.9"
thiserror = "1.0"
anyhow = "1.0"
log = "0.4"
env_logger = "0.11"
rand = "0.8"
ring = "0.17"
regex = "1.10"
chrono = "0.4"
lazy_static = "1.4"

[target.'cfg(windows)'.dependencies]
windows = { version = "0.52", features = ["Win32_Security"] }
winapi = "0.3"

[target.'cfg(target_os = "linux")'.dependencies]
nix = { version = "0.27", features = ["user"] }
libc = "0.2"

[target.'cfg(target_os = "macos")'.dependencies]
core-foundation = "0.9"
system-configuration = "0.5"
```

### 3. 安装步骤

1. **安装依赖**
   ```bash
   # Windows
   # 无需额外安装
   
   # Linux
   sudo apt-get install iptables
   
   # macOS
   # 无需额外安装（pf 内置）
   ```

2. **配置权限**
   ```bash
   # Linux
   sudo setcap cap_net_admin=eip /path/to/clash-verge
   
   # macOS
   sudo chown root:wheel /path/to/clash-verge
   sudo chmod u+s /path/to/clash-verge
   ```

3. **启动应用**
   ```bash
   # 首次启动需要管理员权限
   sudo ./clash-verge
   ```

---

## 文档清单

### 实现文件

**Rust 后端**:
- `src-tauri/src/security/local_security.rs` - 本地安全模块
- `src-tauri/src/security/firewall.rs` - 防火墙管理
- `src-tauri/src/security/leak_monitor.rs` - 泄漏监控
- `src-tauri/src/http/header_sanitization.rs` - HTTP 头净化
- `src-tauri/src/traffic/padding.rs` - 流量填充
- `src-tauri/src/cmd/local_security.rs` - 本地安全命令
- `src-tauri/src/cmd/header_sanitization.rs` - HTTP 头净化命令
- `src-tauri/src/cmd/traffic_padding.rs` - 流量填充命令

**TypeScript 前端**:
- `src/services/local-security.ts` - 本地安全服务
- `src/services/header-sanitization.ts` - HTTP 头净化服务
- `src/services/traffic-padding.ts` - 流量填充服务
- `src/components/security/local-security-monitor.tsx` - 本地安全监控组件
- `src/components/settings/header-sanitization-config.tsx` - HTTP 头净化配置组件
- `src/components/settings/traffic-padding-config.tsx` - 流量填充配置组件

**配置文件**:
- `config/security-phase2.yaml` - 安全配置文件

**文档**:
- `.kiro/specs/security-enhancement-phase2/requirements.md` - 需求文档
- `.kiro/specs/security-enhancement-phase2/design.md` - 设计文档（本文档）
- `.kiro/specs/security-enhancement-phase2/tasks.md` - 任务分解文档

---

## 总结

本设计文档定义了安全增强 Phase 2 的完整技术方案，包括：

1. **入口隐蔽增强**（6 小时）
   - 本地监听安全加固
   - 防火墙规则自动配置
   - 进程隐蔽增强
   - 实时泄漏监控

2. **HTTP 头净化**（4 小时）
   - 代理头清除
   - 浏览器指纹伪造
   - 头部顺序规范化

3. **流量填充**（4 小时）
   - 随机填充数据生成
   - 智能填充算法
   - 性能控制

**总工作量**: 14 小时

**预期效果**:
- 入口安全性：50% → 95%
- 流量隐匿性：25% → 85%
- 整体防护：65% → 90%

---

**创建日期**: 2026-05-28  
**状态**: ✅ 设计完成  
**下一步**: 创建任务分解文档 (`tasks.md`)
