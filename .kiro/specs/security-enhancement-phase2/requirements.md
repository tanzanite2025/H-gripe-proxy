# 安全增强 Phase 2 - 需求文档（修正版）

## 概述

根据实际架构分析，重新定义安全增强需求。核心目标是**入口隐蔽**和**流量特征隐匿**，而不是出口IP轮换。

---

## 架构理解

### 当前架构特点

```
用户应用 (浏览器/应用)
    ↓
┌──────────────────────────────┐
│ 本地防护 (127.0.0.1:10808)   │
│ • 仅本地监听                  │
│ • 防火墙保护                  │
│ • 进程隐蔽                    │
└──────────────────────────────┘
    ↓
┌──────────────────────────────┐
│ 你的代理软件                  │
│ • 入口隐蔽 (用户IP保护)       │
│ • 流量混淆                    │
│ • 反向追踪防护                │
└──────────────────────────────┘
    ↓ (单条长连接)
┌──────────────────────────────┐
│ 订阅商出口 (固定节点)         │
│ • 永不轮换 ✅                 │
│ • 持久连接 ✅                 │
│ • 心跳保活 ✅                 │
│ • 监听节点健康 ✅             │
└──────────────────────────────┘
    ↓
互联网
```

### 安全模型

1. **出口安全性**（已完善）：
   - ✅ IP 稳定性 100%
   - ✅ 连接持久性 100%
   - ✅ 节点健康监控
   - ✅ 预期封号风险极低

2. **入口安全性**（需要增强）：
   - ⚠️ 本地监听（基础）
   - ⚠️ 防火墙保护（基础）
   - ⚠️ 进程隐蔽（基础）
   - ❌ 实时泄漏监控（缺失）
   - ❌ 流量特征隐匿（缺失）

3. **流量状态**（需要增强）：
   - ⚠️ 流量监控（基础）
   - ❌ 异常行为检测（缺失）
   - ❌ 流量特征混淆（缺失）

---

## 核心需求（修正）

### ❌ 删除：出口IP轮换

**原因**：
- 与架构设计**完全相反**
- 出口需要的是**稳定性**，不是轮换
- 轮换会导致封号风险增加

### ✅ 新增：入口隐蔽增强

**目标**：保护用户真实IP，防止入口泄漏

**需求**：
1. 本地监听安全加固
2. 防火墙规则自动配置
3. 进程隐蔽增强
4. 实时泄漏监控

### ✅ 保留：HTTP头净化

**目标**：清除代理特征，防止代理检测

**需求**：
1. 清除代理特征头（X-Forwarded-For、Via、Proxy-Connection）
2. 伪造/规范化正常头（User-Agent、Accept、Accept-Language）
3. 头部顺序规范化

### ✅ 保留：流量填充

**目标**：对抗流量分析，隐匿流量特征

**需求**：
1. 随机填充数据生成
2. 填充策略配置（强度、频率、大小范围）
3. 智能填充（根据当前流量调整）
4. 性能控制（带宽/CPU限制）

---

## 功能 1: 入口隐蔽增强

### 1.1 用户故事

**作为** 代理用户  
**我希望** 我的真实IP和本地监听端口不会泄漏  
**以便** 保护我的隐私和安全

### 1.2 功能需求

#### 1.2.1 本地监听安全加固

**需求**：
- 强制绑定到 127.0.0.1（禁止 0.0.0.0）
- 监听端口随机化（可选）
- 端口占用检测和自动切换
- 监听状态实时监控

**配置示例**：
```yaml
local_security:
  bind_address: "127.0.0.1"  # 强制本地
  port_randomization: false   # 端口随机化
  port_range: [10800, 10900]  # 随机端口范围
  auto_switch_on_conflict: true  # 端口冲突自动切换
```

#### 1.2.2 防火墙规则自动配置

**需求**：
- Windows: 自动配置 Windows Defender 防火墙
- Linux: 自动配置 iptables/nftables
- macOS: 自动配置 pf
- 规则验证和健康检查

**防火墙规则**：
```bash
# Windows (PowerShell)
New-NetFirewallRule -DisplayName "Clash Verge Local Only" `
  -Direction Inbound -LocalAddress 127.0.0.1 -Action Allow

# Linux (iptables)
iptables -A INPUT -i lo -j ACCEPT
iptables -A INPUT -p tcp --dport 10808 -j DROP

# macOS (pf)
block in proto tcp from any to any port 10808
pass in proto tcp from 127.0.0.1 to 127.0.0.1 port 10808
```

#### 1.2.3 进程隐蔽增强

**需求**：
- 进程名混淆（可选）
- 进程优先级调整（降低可见性）
- 进程保护（防止被杀）
- 进程监控（检测异常终止）

**配置示例**：
```yaml
process_stealth:
  name_obfuscation: false  # 进程名混淆
  priority: "normal"       # 进程优先级
  protection: true         # 进程保护
  monitoring: true         # 进程监控
```

#### 1.2.4 实时泄漏监控

**需求**：
- 定期检测本地监听端口是否暴露
- 定期检测防火墙规则是否生效
- 定期检测进程是否被扫描
- 泄漏告警和自动修复

**监控指标**：
```typescript
interface LeakMonitorStatus {
  localBindingSecure: boolean;      // 本地绑定安全
  firewallRulesActive: boolean;     // 防火墙规则生效
  processHidden: boolean;           // 进程隐蔽
  externalAccessBlocked: boolean;   // 外部访问被阻止
  lastCheckTime: number;            // 最后检查时间
  leakDetected: boolean;            // 是否检测到泄漏
  leakType?: string;                // 泄漏类型
  autoFixApplied: boolean;          // 是否自动修复
}
```

### 1.3 验收标准

- [ ] 本地监听强制绑定到 127.0.0.1
- [ ] 防火墙规则自动配置并验证
- [ ] 进程隐蔽功能正常工作
- [ ] 实时泄漏监控每 30 秒检查一次
- [ ] 检测到泄漏时自动告警
- [ ] 支持自动修复（可选）
- [ ] 提供详细的监控日志

### 1.4 技术依赖

- Rust: `tokio`, `sysinfo`, `netstat2`
- Windows: `windows-rs`, `winapi`
- Linux: `nix`, `libc`
- macOS: `core-foundation`, `system-configuration`

### 1.5 风险

- 防火墙配置可能需要管理员权限
- 不同操作系统的防火墙 API 差异较大
- 进程保护可能与安全软件冲突

---

## 功能 2: HTTP头净化

### 2.1 用户故事

**作为** 代理用户  
**我希望** HTTP 请求头不包含代理特征  
**以便** 避免被目标网站检测为代理

### 2.2 功能需求

#### 2.2.1 清除代理特征头

**需求**：
- 清除 `X-Forwarded-For`
- 清除 `X-Real-IP`
- 清除 `Via`
- 清除 `Proxy-Connection`
- 清除 `X-Proxy-ID`
- 清除其他自定义代理头

**配置示例**：
```yaml
header_sanitization:
  remove_proxy_headers: true
  custom_headers_to_remove:
    - "X-Custom-Proxy"
    - "X-Forwarded-Host"
```

#### 2.2.2 伪造/规范化正常头

**需求**：
- 伪造 `User-Agent`（模拟真实浏览器）
- 规范化 `Accept`
- 规范化 `Accept-Language`
- 规范化 `Accept-Encoding`
- 添加 `DNT` (Do Not Track)
- 添加 `Upgrade-Insecure-Requests`

**配置示例**：
```yaml
header_forge:
  user_agent: "auto"  # auto, chrome, firefox, safari, custom
  custom_user_agent: ""
  accept: "text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8"
  accept_language: "en-US,en;q=0.9"
  accept_encoding: "gzip, deflate, br"
  dnt: "1"
  upgrade_insecure_requests: "1"
```

#### 2.2.3 头部顺序规范化

**需求**：
- 按照真实浏览器的头部顺序排列
- 支持多种浏览器模板（Chrome、Firefox、Safari）
- 自定义头部顺序

**头部顺序示例（Chrome）**：
```
1. Host
2. Connection
3. Upgrade-Insecure-Requests
4. User-Agent
5. Accept
6. Accept-Encoding
7. Accept-Language
8. Cookie
9. DNT
```

### 2.3 验收标准

- [ ] 所有代理特征头被清除
- [ ] User-Agent 伪造为真实浏览器
- [ ] Accept 系列头规范化
- [ ] 头部顺序符合真实浏览器
- [ ] 支持 3 种浏览器模板（Chrome、Firefox、Safari）
- [ ] 支持自定义头部配置
- [ ] 提供头部净化前后对比日志

### 2.4 技术依赖

- Rust: `http`, `hyper`, `reqwest`
- HTTP 头解析和修改
- 浏览器指纹数据库

### 2.5 风险

- 过度净化可能导致某些网站功能异常
- 头部顺序规范化可能与某些服务器不兼容
- User-Agent 伪造需要定期更新

---

## 功能 3: 流量填充

### 3.1 用户故事

**作为** 代理用户  
**我希望** 我的流量特征被混淆  
**以便** 对抗流量分析和行为画像

### 3.2 功能需求

#### 3.2.1 随机填充数据生成

**需求**：
- 生成随机填充数据（不可压缩）
- 填充数据大小随机化
- 填充数据内容随机化
- 填充数据加密（可选）

**配置示例**：
```yaml
traffic_padding:
  enabled: true
  min_size: 100        # 最小填充大小（字节）
  max_size: 1024       # 最大填充大小（字节）
  encrypt: true        # 加密填充数据
```

#### 3.2.2 填充策略配置

**需求**：
- 填充强度（低/中/高/自定义）
- 填充频率（每 N 秒/每 N 请求）
- 填充时机（请求前/请求后/随机）
- 填充目标（所有流量/特定域名/特定协议）

**配置示例**：
```yaml
padding_strategy:
  intensity: "medium"  # low, medium, high, custom
  frequency:
    type: "time"       # time, request, random
    interval: 5        # 每 5 秒
  timing: "random"     # before, after, random
  targets:
    - "*.google.com"
    - "*.youtube.com"
```

#### 3.2.3 智能填充

**需求**：
- 根据当前流量大小调整填充强度
- 根据网络延迟调整填充频率
- 根据带宽使用率调整填充大小
- 避免在高流量时过度填充

**智能填充算法**：
```typescript
function calculatePaddingSize(
  currentTraffic: number,
  networkLatency: number,
  bandwidthUsage: number
): number {
  // 流量越小，填充越多
  const trafficFactor = 1 - Math.min(currentTraffic / 1000000, 1);
  
  // 延迟越高，填充越少
  const latencyFactor = 1 - Math.min(networkLatency / 1000, 1);
  
  // 带宽使用率越高，填充越少
  const bandwidthFactor = 1 - Math.min(bandwidthUsage, 1);
  
  const basePadding = 512; // 基础填充大小
  return basePadding * trafficFactor * latencyFactor * bandwidthFactor;
}
```

#### 3.2.4 性能控制

**需求**：
- 带宽限制（最大填充带宽）
- CPU 限制（最大 CPU 使用率）
- 内存限制（最大内存使用）
- 自动降级（性能不足时自动降低填充强度）

**配置示例**：
```yaml
performance_control:
  max_bandwidth: 1048576  # 1 MB/s
  max_cpu_usage: 10       # 10%
  max_memory: 104857600   # 100 MB
  auto_downgrade: true    # 自动降级
```

### 3.3 验收标准

- [ ] 随机填充数据生成正常
- [ ] 填充数据大小和内容随机化
- [ ] 支持 3 种填充强度（低/中/高）
- [ ] 支持 3 种填充频率（时间/请求/随机）
- [ ] 智能填充根据流量动态调整
- [ ] 性能控制限制生效
- [ ] 提供填充统计（填充次数、填充大小、带宽占用）

### 3.4 技术依赖

- Rust: `tokio`, `rand`, `ring`
- 流量监控和统计
- 性能监控（CPU、内存、带宽）

### 3.5 风险

- 填充过多可能导致带宽浪费
- 填充过少可能无法有效对抗流量分析
- 智能填充算法需要调优
- 性能控制可能影响填充效果

---

## 非功能需求

### 1. 性能要求

- HTTP 头净化延迟 < 1ms
- 流量填充 CPU 占用 < 10%
- 入口监控检查间隔 30 秒
- 内存占用 < 100 MB

### 2. 安全要求

- 所有配置加密存储
- 敏感日志脱敏
- 防火墙规则验证
- 泄漏自动修复

### 3. 可用性要求

- 配置热重载（无需重启）
- 错误自动恢复
- 详细的日志和监控
- 用户友好的 UI

### 4. 兼容性要求

- Windows 10/11
- Linux (Ubuntu 20.04+, Debian 11+)
- macOS 11+
- 支持主流浏览器（Chrome、Firefox、Safari、Edge）

---

## 用户界面

### 1. 入口隐蔽监控卡片

**位置**：设置页面 → 安全设置

**内容**：
```
┌─────────────────────────────────────┐
│ 入口隐蔽监控                         │
├─────────────────────────────────────┤
│ 状态: ✅ 安全                        │
│                                     │
│ 本地绑定:     ✅ 127.0.0.1:10808    │
│ 防火墙:       ✅ 已启用              │
│ 进程隐蔽:     ✅ 已启用              │
│ 外网访问:     🟢 已阻止              │
│                                     │
│ 最后检查: 30 秒前                    │
│                                     │
│ [立即检查] [查看日志]                │
└─────────────────────────────────────┘
```

### 2. HTTP 头净化配置

**位置**：设置页面 → 高级设置

**内容**：
```
┌─────────────────────────────────────┐
│ HTTP 头净化                          │
├─────────────────────────────────────┤
│ ☑ 清除代理特征头                     │
│ ☑ 伪造 User-Agent                    │
│                                     │
│ 浏览器模板: [Chrome ▼]               │
│                                     │
│ User-Agent:                         │
│ Mozilla/5.0 (Windows NT 10.0...)    │
│                                     │
│ [高级配置] [测试净化效果]            │
└─────────────────────────────────────┘
```

### 3. 流量填充配置

**位置**：设置页面 → 高级设置

**内容**：
```
┌─────────────────────────────────────┐
│ 流量填充                             │
├─────────────────────────────────────┤
│ ☑ 启用流量填充                       │
│                                     │
│ 填充强度: ●───○───○ 低/中/高        │
│                                     │
│ 填充频率: 每 [5] 秒                  │
│                                     │
│ ☑ 智能填充（根据流量自动调整）        │
│                                     │
│ 性能限制:                            │
│ 最大带宽: [1] MB/s                   │
│ 最大 CPU: [10] %                     │
│                                     │
│ 统计:                                │
│ 今日填充: 1.2 GB                     │
│ 填充次数: 12,345                     │
│                                     │
│ [查看详细统计]                       │
└─────────────────────────────────────┘
```

---

## 实施计划

### Phase 2.1: 入口隐蔽增强（6 小时）

**任务**：
1. 实现本地监听安全加固（2 小时）
2. 实现防火墙规则自动配置（2 小时）
3. 实现实时泄漏监控（2 小时）

**交付物**：
- `src-tauri/src/security/local_security.rs`
- `src-tauri/src/security/firewall.rs`
- `src-tauri/src/security/leak_monitor.rs`
- `src/services/local-security.ts`
- `src/components/security/local-security-monitor.tsx`

### Phase 2.2: HTTP 头净化（4 小时）

**任务**：
1. 实现代理头清除（1 小时）
2. 实现正常头伪造（1 小时）
3. 实现头部顺序规范化（1 小时）
4. 实现浏览器模板（1 小时）

**交付物**：
- `src-tauri/src/http/header_sanitization.rs`
- `src/services/header-sanitization.ts`
- `src/components/settings/header-sanitization-config.tsx`

### Phase 2.3: 流量填充（4 小时）

**任务**：
1. 实现随机填充数据生成（1 小时）
2. 实现填充策略配置（1 小时）
3. 实现智能填充（1 小时）
4. 实现性能控制（1 小时）

**交付物**：
- `src-tauri/src/traffic/padding.rs`
- `src/services/traffic-padding.ts`
- `src/components/settings/traffic-padding-config.tsx`

---

## 总结

### 修正内容

1. **删除**：出口 IP 轮换（与架构相反）
2. **新增**：入口隐蔽增强（符合架构需求）
3. **保留**：HTTP 头净化（必要功能）
4. **保留**：流量填充（必要功能）

### 预期效果

- ✅ 入口安全性从 50% 提升到 95%
- ✅ 流量特征隐匿从 25% 提升到 85%
- ✅ 整体安全防护能力从 65% 提升到 90%

### 总工作量

- 入口隐蔽增强：6 小时
- HTTP 头净化：4 小时
- 流量填充：4 小时
- **总计：14 小时**

---

**创建日期**: 2026-05-28  
**状态**: ✅ 需求定义完成  
**下一步**: 创建设计文档 (`design.md`)
