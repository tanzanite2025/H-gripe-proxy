# 系统集成架构梳理

## 问题分析

### 1. 潜在重复功能

检查发现以下可能的重复：

#### ❌ 没有重复
- **代理链**：`enhance/chain.rs` 是现有功能，`multipath` 是新增的多路径路由，两者功能不同
- **配置加密**：`config/encrypt.rs` 是配置文件加密，`security/config_decoy.rs` 是配置欺骗，功能不同
- **备份**：`core/backup.rs` 是配置备份，`module/auto_backup.rs` 是自动备份，功能互补

### 2. 模块串联问题

当前各模块独立运行，需要建立统一的协调机制。

---

## 统一架构设计

### 核心协调器

创建一个中央协调器，统一管理所有高级功能。

```
┌─────────────────────────────────────────────────────────┐
│              Clash Verge 核心协调器                       │
│  ┌───────────────────────────────────────────────────┐  │
│  │           配置管理层（Config Layer）                │  │
│  │  - 统一配置存储                                     │  │
│  │  - 配置验证                                         │  │
│  │  - 配置同步                                         │  │
│  └───────────────────────────────────────────────────┘  │
│                          ↓                               │
│  ┌───────────────────────────────────────────────────┐  │
│  │         安全防御层（Security Layer）                │  │
│  │  ┌─────────────┬─────────────┬─────────────────┐  │  │
│  │  │ 反主动探测   │ TLS 指纹    │ 内生欺骗陷阱     │  │  │
│  │  │ (Anti-Probe)│ (Parrot)    │ (Honeypot)      │  │  │
│  │  └─────────────┴─────────────┴─────────────────┘  │  │
│  └───────────────────────────────────────────────────┘  │
│                          ↓                               │
│  ┌───────────────────────────────────────────────────┐  │
│  │         路由决策层（Routing Layer）                 │  │
│  │  ┌─────────────┬─────────────┬─────────────────┐  │  │
│  │  │ 多路径路由   │ 代理链      │ 规则匹配         │  │  │
│  │  │ (Multipath) │ (Chain)     │ (Rules)         │  │  │
│  │  └─────────────┴─────────────┴─────────────────┘  │  │
│  └───────────────────────────────────────────────────┘  │
│                          ↓                               │
│  ┌───────────────────────────────────────────────────┐  │
│  │         数据平面层（Data Plane Layer）              │  │
│  │  ┌─────────────┬─────────────┬─────────────────┐  │  │
│  │  │ XDP 代理    │ 传统代理     │ 混淆加密         │  │  │
│  │  │ (XDP)       │ (TUN/TAP)   │ (Obfuscation)   │  │  │
│  │  └─────────────┴─────────────┴─────────────────┘  │  │
│  └───────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────┘
```

---

## 模块集成方案

### 1. 创建核心协调器

**文件**：`src-tauri/src/core/coordinator.rs`

```rust
/// 核心协调器
pub struct CoreCoordinator {
    // 安全模块
    security_monitor: Arc<SecurityMonitor>,
    anti_probe: Arc<AntiProbeService>,
    tls_fingerprint: Arc<TlsFingerprintService>,
    
    // 路由模块
    multipath_manager: Arc<MultipathManager>,
    
    // XDP 模块（Linux）
    #[cfg(target_os = "linux")]
    xdp_manager: Arc<XdpManager>,
    
    // 配置
    config: Arc<RwLock<CoordinatorConfig>>,
}

impl CoreCoordinator {
    /// 初始化所有模块
    pub fn initialize(&self) -> Result<()> {
        // 1. 启动安全监控
        if self.config.read().security_enabled {
            self.security_monitor.start();
        }
        
        // 2. 配置反探测
        if self.config.read().anti_probe_enabled {
            // 配置反探测
        }
        
        // 3. 设置 TLS 指纹
        if let Some(ref fingerprint) = self.config.read().tls_fingerprint {
            self.tls_fingerprint.set_fingerprint(fingerprint.clone());
        }
        
        // 4. 启动多路径路由
        if self.config.read().multipath_enabled {
            // 配置多路径
        }
        
        // 5. 启动 XDP（Linux）
        #[cfg(target_os = "linux")]
        if self.config.read().xdp_enabled {
            self.xdp_manager.start()?;
        }
        
        Ok(())
    }
    
    /// 处理连接请求
    pub fn handle_connection(&self, request: ConnectionRequest) -> Result<ConnectionDecision> {
        // 1. 安全检查
        if self.security_monitor.is_compromised() {
            return Err("安全状态已被破坏".into());
        }
        
        // 2. 反探测验证
        if self.config.read().anti_probe_enabled {
            if !self.anti_probe.verify_handshake(&request.client_ip, &request.token) {
                return Ok(ConnectionDecision::Reject);
            }
        }
        
        // 3. 路由决策
        let route = if self.config.read().multipath_enabled {
            // 多路径路由
            self.multipath_manager.select_node(&request.domain, request.session_id)
        } else {
            // 传统路由
            None
        };
        
        // 4. 返回决策
        Ok(ConnectionDecision::Accept {
            route,
            tls_fingerprint: self.tls_fingerprint.get_fingerprint().cloned(),
        })
    }
}
```

### 2. 配置统一管理

**文件**：`src-tauri/src/config/advanced.rs`

```rust
/// 高级功能配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdvancedConfig {
    /// 安全防御
    pub security: SecurityConfig,
    
    /// 多路径路由
    pub multipath: MultipathConfig,
    
    /// XDP 代理
    #[cfg(target_os = "linux")]
    pub xdp: XdpConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityConfig {
    /// 启用安全监控
    pub enabled: bool,
    
    /// 反主动探测
    pub anti_probe: AntiProbeConfig,
    
    /// TLS 指纹
    pub tls_fingerprint: Option<String>,
    
    /// 配置欺骗
    pub config_decoy: ConfigDecoyConfig,
}
```

### 3. 前端统一入口

**文件**：`src/pages/advanced.tsx`

```tsx
/**
 * 高级功能统一配置页面
 */
export default function AdvancedPage() {
  const [tabValue, setTabValue] = useState(0)

  return (
    <BasePage title="高级功能">
      <Tabs value={tabValue} onChange={(_, v) => setTabValue(v)}>
        <Tab label="安全防御" />
        <Tab label="多路径路由" />
        <Tab label="XDP 代理" />
        <Tab label="性能监控" />
      </Tabs>

      <TabPanel value={tabValue} index={0}>
        <SecurityConfig />
      </TabPanel>

      <TabPanel value={tabValue} index={1}>
        <MultipathConfig />
      </TabPanel>

      <TabPanel value={tabValue} index={2}>
        <XdpConfig />
      </TabPanel>

      <TabPanel value={tabValue} index={3}>
        <PerformanceMonitor />
      </TabPanel>
    </BasePage>
  )
}
```

---

## 数据流集成

### 连接处理流程

```
用户请求
    ↓
┌─────────────────────────────────────┐
│ 1. 安全检查                          │
│    - 反调试检测                      │
│    - 内存蜜罐检查                    │
│    - 配置欺骗状态                    │
└─────────────────────────────────────┘
    ↓ (通过)
┌─────────────────────────────────────┐
│ 2. 反探测验证                        │
│    - 握手暗号验证                    │
│    - 白名单检查                      │
└─────────────────────────────────────┘
    ↓ (通过)
┌─────────────────────────────────────┐
│ 3. TLS 指纹应用                      │
│    - 选择指纹                        │
│    - 应用到连接                      │
└─────────────────────────────────────┘
    ↓
┌─────────────────────────────────────┐
│ 4. 路由决策                          │
│    - 检查会话绑定                    │
│    - 多路径 or 单路径                │
│    - 选择节点                        │
└─────────────────────────────────────┘
    ↓
┌─────────────────────────────────────┐
│ 5. 数据传输                          │
│    - XDP 快速路径 (Linux)            │
│    - 传统路径 (其他平台)             │
│    - 混淆加密                        │
└─────────────────────────────────────┘
    ↓
目标服务器
```

---

## 配置优先级

### 1. 安全优先

```
安全检查失败 → 立即拒绝/自毁
    ↓
反探测失败 → 拒绝连接
    ↓
正常路由
```

### 2. 性能优先

```
Linux + XDP 可用 → 使用 XDP
    ↓
多路径可用 → 使用多路径
    ↓
传统代理
```

### 3. 兼容性优先

```
流媒体/游戏 → 强制单节点
    ↓
其他服务 → 根据配置
```

---

## 实现步骤

### Phase 1: 创建协调器（1 天）

1. ✅ 创建 `core/coordinator.rs`
2. ✅ 实现模块初始化
3. ✅ 实现连接处理流程

### Phase 2: 配置整合（1 天）

1. ✅ 创建 `config/advanced.rs`
2. ✅ 整合所有高级功能配置
3. ✅ 实现配置验证

### Phase 3: 前端整合（1 天）

1. ✅ 创建统一配置页面
2. ✅ 整合所有配置组件
3. ✅ 实现状态同步

### Phase 4: 测试验证（1 天）

1. ✅ 单元测试
2. ✅ 集成测试
3. ✅ 性能测试

---

## 配置示例

### 完整配置

```yaml
# 高级功能配置
advanced:
  # 安全防御
  security:
    enabled: true
    
    # 反主动探测
    anti_probe:
      enabled: true
      secret_key: "auto-generated"
      time_window: 300
      whitelist: []
      strict_mode: false
    
    # TLS 指纹
    tls_fingerprint: "Chrome 120 (Windows)"
    
    # 配置欺骗
    config_decoy:
      enabled: true
      decoy_path: "config_decoy.yaml"
  
  # 多路径路由
  multipath:
    enabled: true
    strategy: "Weighted"
    session_persistence: true
    
    node_pools:
      - name: "流媒体专用"
        pool_type: "Streaming"
        enabled: true
        nodes: [...]
      
      - name: "通用池"
        pool_type: "General"
        enabled: true
        nodes: [...]
  
  # XDP 代理（Linux）
  xdp:
    enabled: false  # 默认关闭
    interface: "eth0"
    mode: "Skb"
```

---

## 监控和调试

### 统一监控面板

```tsx
<PerformanceMonitor>
  <SecurityStatus />
  <MultipathStats />
  <XdpStats />
  <ConnectionStats />
</PerformanceMonitor>
```

### 日志集成

```rust
// 统一日志格式
log::info!("[Coordinator] 初始化完成");
log::info!("[Security] 安全监控已启动");
log::info!("[AntiProbe] 验证通过: {}", client_ip);
log::info!("[Multipath] 选择节点: {}", node_name);
log::info!("[XDP] 数据包处理: {} packets", count);
```

---

## 总结

### 当前状态

✅ **无重复功能** - 所有模块功能独立且互补  
❌ **缺少串联** - 各模块独立运行，需要协调器  
❌ **配置分散** - 需要统一配置管理  

### 需要实现

1. **核心协调器** - 统一管理所有模块
2. **配置整合** - 统一配置存储和验证
3. **前端整合** - 统一配置界面
4. **数据流串联** - 建立完整的处理流程

### 优先级

1. **高优先级**：核心协调器（必须）
2. **中优先级**：配置整合（重要）
3. **低优先级**：前端美化（可选）
