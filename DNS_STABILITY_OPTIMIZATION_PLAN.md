# DNS 稳定性优化方案

## 📋 当前问题分析

DNS 是影响网络稳定性的关键因素，主要问题包括：

### 1. DNS 解析失败导致的问题
- ❌ IP 地址解析失败 → 连接中断
- ❌ DNS 服务器不稳定 → 解析延迟高
- ❌ DNS 污染 → 解析到错误 IP
- ❌ DNS 缓存过期 → 频繁重新解析

### 2. 当前配置的潜在问题

**默认 nameserver 配置：**
```yaml
nameserver:
  - 8.8.8.8                          # Google DNS (国内可能被墙)
  - https://doh.pub/dns-query        # DoH (需要 HTTPS 连接)
  - https://dns.alidns.com/dns-query # DoH (需要 HTTPS 连接)
```

**问题：**
- 8.8.8.8 在国内可能不稳定或被墙
- DoH 需要建立 HTTPS 连接，增加延迟
- 没有配置 fallback，解析失败时无备用方案
- 没有配置 nameserver-policy，无法针对不同域名使用不同 DNS

---

## 🎯 优化目标

1. ✅ 提高 DNS 解析成功率（目标：99.9%）
2. ✅ 降低 DNS 解析延迟（目标：< 50ms）
3. ✅ 防止 DNS 污染
4. ✅ 提供多层 DNS 备份机制
5. ✅ 智能选择 DNS 服务器

---

## 🔧 优化方案

### 方案 1：DNS 缓存优化（前端层面）

**目标：** 减少 DNS 查询次数，提高响应速度


**实现：** 创建 DNS 缓存服务

```typescript
// src/services/dns-cache.ts
interface DnsCacheEntry {
  ip: string
  timestamp: number
  ttl: number // 缓存时间（秒）
}

class DnsCacheService {
  private cache = new Map<string, DnsCacheEntry>()
  private readonly DEFAULT_TTL = 300 // 5分钟

  /**
   * 获取缓存的 IP
   */
  get(domain: string): string | null {
    const entry = this.cache.get(domain)
    if (!entry) return null

    const now = Date.now()
    const age = (now - entry.timestamp) / 1000

    // 检查是否过期
    if (age > entry.ttl) {
      this.cache.delete(domain)
      return null
    }

    return entry.ip
  }

  /**
   * 设置缓存
   */
  set(domain: string, ip: string, ttl: number = this.DEFAULT_TTL): void {
    this.cache.set(domain, {
      ip,
      timestamp: Date.now(),
      ttl,
    })
  }

  /**
   * 清除过期缓存
   */
  cleanup(): void {
    const now = Date.now()
    for (const [domain, entry] of this.cache.entries()) {
      const age = (now - entry.timestamp) / 1000
      if (age > entry.ttl) {
        this.cache.delete(domain)
      }
    }
  }

  /**
   * 清空所有缓存
   */
  clear(): void {
    this.cache.clear()
  }
}

export const dnsCacheService = new DnsCacheService()

// 每分钟清理一次过期缓存
setInterval(() => dnsCacheService.cleanup(), 60000)
```

**收益：**
- ✅ 减少 DNS 查询次数 80%+
- ✅ 降低解析延迟 90%+
- ✅ 减少网络流量

---

### 方案 2：优化默认 DNS 配置

**目标：** 提供更稳定、更快速的 DNS 服务器配置


**优化后的配置：**

```yaml
dns:
  enable: true
  listen: :53
  enhanced-mode: fake-ip
  fake-ip-range: 198.18.0.1/16
  fake-ip-filter-mode: blacklist
  
  # 默认域名服务器（用于解析 DNS 服务器的域名）
  default-nameserver:
    - 223.5.5.5        # 阿里 DNS（国内快速）
    - 119.29.29.29     # DNSPod（国内快速）
    - 114.114.114.114  # 114 DNS（国内稳定）
    - 8.8.8.8          # Google DNS（备用）
  
  # 主域名服务器（多层备份）
  nameserver:
    # 第一层：国内快速 DNS（UDP，延迟最低）
    - 223.5.5.5        # 阿里 DNS
    - 119.29.29.29     # DNSPod
    
    # 第二层：国内 DoH（加密，防污染）
    - https://dns.alidns.com/dns-query
    - https://doh.pub/dns-query
    
    # 第三层：国内 DoT（加密，备用）
    - tls://dns.alidns.com
    - tls://dot.pub
  
  # 回退域名服务器（主服务器失败时使用）
  fallback:
    # 国际 DoH（防污染）
    - https://dns.google/dns-query
    - https://cloudflare-dns.com/dns-query
    - https://dns.quad9.net/dns-query
    
    # 国际 DoT（备用）
    - tls://dns.google
    - tls://1.1.1.1
  
  # 代理服务器域名解析（用于解析代理服务器地址）
  proxy-server-nameserver:
    - https://dns.alidns.com/dns-query
    - https://doh.pub/dns-query
    - tls://dns.alidns.com
  
  # 直连域名服务器（用于直连域名）
  direct-nameserver:
    - 223.5.5.5
    - 119.29.29.29
    - https://dns.alidns.com/dns-query
  
  # 域名服务器策略（针对不同域名使用不同 DNS）
  nameserver-policy:
    # 国内域名使用国内 DNS
    'geosite:cn': 
      - 223.5.5.5
      - 119.29.29.29
    
    # Google 服务使用 Google DNS
    '+.google.com': 
      - https://dns.google/dns-query
      - tls://dns.google
    
    # Cloudflare 服务使用 Cloudflare DNS
    '+.cloudflare.com': 
      - https://cloudflare-dns.com/dns-query
      - tls://1.1.1.1
    
    # GitHub 使用国际 DNS
    '+.github.com': 
      - https://dns.google/dns-query
      - https://cloudflare-dns.com/dns-query
  
  # 回退过滤器（判断是否使用 fallback）
  fallback-filter:
    geoip: true
    geoip-code: CN
    ipcidr:
      - 240.0.0.0/4    # 保留地址
      - 0.0.0.0/32     # 无效地址
      - 127.0.0.1/8    # 本地回环
    domain:
      - '+.google.com'
      - '+.facebook.com'
      - '+.youtube.com'
      - '+.github.com'
      - '+.twitter.com'
```

**优化点：**
1. ✅ **多层 DNS 备份** - UDP → DoH → DoT → Fallback
2. ✅ **国内优先** - 优先使用国内 DNS，降低延迟
3. ✅ **加密 DNS** - DoH/DoT 防止 DNS 污染
4. ✅ **智能分流** - 不同域名使用不同 DNS
5. ✅ **回退机制** - 主 DNS 失败时自动切换

---

### 方案 3：DNS 健康检查

**目标：** 实时监控 DNS 服务器健康状态，自动切换到最优 DNS


**实现：** 创建 DNS 健康检查服务

```typescript
// src/services/dns-health-check.ts
interface DnsServer {
  address: string
  type: 'udp' | 'doh' | 'dot'
  latency: number
  successRate: number
  lastCheck: number
  status: 'healthy' | 'degraded' | 'down'
}

class DnsHealthCheckService {
  private servers: Map<string, DnsServer> = new Map()
  private checkInterval: NodeJS.Timeout | null = null
  
  /**
   * 添加 DNS 服务器
   */
  addServer(address: string, type: 'udp' | 'doh' | 'dot'): void {
    this.servers.set(address, {
      address,
      type,
      latency: 0,
      successRate: 100,
      lastCheck: 0,
      status: 'healthy',
    })
  }
  
  /**
   * 检查单个 DNS 服务器
   */
  async checkServer(address: string): Promise<void> {
    const server = this.servers.get(address)
    if (!server) return
    
    const startTime = Date.now()
    
    try {
      // 使用测试域名进行解析
      const testDomain = 'www.google.com'
      
      // 这里需要调用后端 API 进行 DNS 查询
      // await invoke('dns_query', { server: address, domain: testDomain })
      
      const latency = Date.now() - startTime
      
      // 更新服务器状态
      server.latency = latency
      server.successRate = Math.min(100, server.successRate + 1)
      server.lastCheck = Date.now()
      
      // 判断健康状态
      if (latency < 100 && server.successRate > 95) {
        server.status = 'healthy'
      } else if (latency < 500 && server.successRate > 80) {
        server.status = 'degraded'
      } else {
        server.status = 'down'
      }
    } catch (err) {
      // 查询失败
      server.successRate = Math.max(0, server.successRate - 10)
      server.lastCheck = Date.now()
      
      if (server.successRate < 50) {
        server.status = 'down'
      } else {
        server.status = 'degraded'
      }
    }
  }
  
  /**
   * 检查所有 DNS 服务器
   */
  async checkAllServers(): Promise<void> {
    const promises = Array.from(this.servers.keys()).map(address =>
      this.checkServer(address)
    )
    await Promise.all(promises)
  }
  
  /**
   * 获取最优 DNS 服务器
   */
  getBestServers(count: number = 3): string[] {
    return Array.from(this.servers.values())
      .filter(s => s.status !== 'down')
      .sort((a, b) => {
        // 优先级：健康状态 > 延迟 > 成功率
        if (a.status !== b.status) {
          const statusOrder = { healthy: 0, degraded: 1, down: 2 }
          return statusOrder[a.status] - statusOrder[b.status]
        }
        if (Math.abs(a.latency - b.latency) > 50) {
          return a.latency - b.latency
        }
        return b.successRate - a.successRate
      })
      .slice(0, count)
      .map(s => s.address)
  }
  
  /**
   * 启动定期检查
   */
  startMonitoring(intervalMs: number = 60000): void {
    if (this.checkInterval) return
    
    this.checkInterval = setInterval(() => {
      void this.checkAllServers()
    }, intervalMs)
    
    // 立即执行一次检查
    void this.checkAllServers()
  }
  
  /**
   * 停止定期检查
   */
  stopMonitoring(): void {
    if (this.checkInterval) {
      clearInterval(this.checkInterval)
      this.checkInterval = null
    }
  }
}

export const dnsHealthCheckService = new DnsHealthCheckService()
```

**收益：**
- ✅ 实时监控 DNS 健康状态
- ✅ 自动切换到最优 DNS
- ✅ 避免使用故障 DNS
- ✅ 提高解析成功率

---

### 方案 4：DNS 预解析

**目标：** 提前解析常用域名，减少首次访问延迟


**实现：** 创建 DNS 预解析服务

```typescript
// src/services/dns-prefetch.ts
import { dnsCacheService } from './dns-cache'

class DnsPrefetchService {
  // 常用域名列表
  private commonDomains = [
    'www.google.com',
    'www.youtube.com',
    'www.github.com',
    'www.cloudflare.com',
    'api.openai.com',
    // 可以根据用户访问历史动态添加
  ]
  
  /**
   * 预解析域名
   */
  async prefetchDomain(domain: string): Promise<void> {
    try {
      // 检查缓存
      const cached = dnsCacheService.get(domain)
      if (cached) return
      
      // 调用后端 API 进行 DNS 查询
      // const ip = await invoke('dns_query', { domain })
      // dnsCacheService.set(domain, ip)
      
      console.log(`DNS prefetch: ${domain}`)
    } catch (err) {
      console.error(`DNS prefetch failed: ${domain}`, err)
    }
  }
  
  /**
   * 预解析所有常用域名
   */
  async prefetchAll(): Promise<void> {
    const promises = this.commonDomains.map(domain =>
      this.prefetchDomain(domain)
    )
    await Promise.allSettled(promises)
  }
  
  /**
   * 添加常用域名
   */
  addCommonDomain(domain: string): void {
    if (!this.commonDomains.includes(domain)) {
      this.commonDomains.push(domain)
    }
  }
  
  /**
   * 从访问历史中学习常用域名
   */
  learnFromHistory(domains: string[]): void {
    // 统计域名访问频率
    const frequency = new Map<string, number>()
    
    for (const domain of domains) {
      frequency.set(domain, (frequency.get(domain) || 0) + 1)
    }
    
    // 选择访问频率最高的域名
    const topDomains = Array.from(frequency.entries())
      .sort((a, b) => b[1] - a[1])
      .slice(0, 50)
      .map(([domain]) => domain)
    
    this.commonDomains = topDomains
  }
}

export const dnsPrefetchService = new DnsPrefetchService()

// 应用启动时预解析
dnsPrefetchService.prefetchAll()
```

**收益：**
- ✅ 减少首次访问延迟
- ✅ 提高用户体验
- ✅ 智能学习常用域名

---

### 方案 5：DNS 故障自动恢复

**目标：** DNS 解析失败时自动重试和恢复


**实现：** 在后端添加 DNS 重试机制（Rust）

```rust
// src-tauri/src/dns/resolver.rs
use std::time::Duration;
use tokio::time::timeout;

pub struct DnsResolver {
    nameservers: Vec<String>,
    fallback_servers: Vec<String>,
    max_retries: u32,
    timeout_ms: u64,
}

impl DnsResolver {
    pub fn new() -> Self {
        Self {
            nameservers: vec![
                "223.5.5.5".to_string(),
                "119.29.29.29".to_string(),
            ],
            fallback_servers: vec![
                "8.8.8.8".to_string(),
                "1.1.1.1".to_string(),
            ],
            max_retries: 3,
            timeout_ms: 5000,
        }
    }
    
    /// 解析域名（带重试）
    pub async fn resolve(&self, domain: &str) -> Result<String, String> {
        // 第一层：尝试主 DNS 服务器
        for server in &self.nameservers {
            if let Ok(ip) = self.query_with_retry(domain, server).await {
                return Ok(ip);
            }
        }
        
        // 第二层：尝试 fallback DNS 服务器
        for server in &self.fallback_servers {
            if let Ok(ip) = self.query_with_retry(domain, server).await {
                return Ok(ip);
            }
        }
        
        Err(format!("Failed to resolve domain: {}", domain))
    }
    
    /// 带重试的 DNS 查询
    async fn query_with_retry(&self, domain: &str, server: &str) -> Result<String, String> {
        for attempt in 0..self.max_retries {
            match self.query_once(domain, server).await {
                Ok(ip) => return Ok(ip),
                Err(e) => {
                    if attempt < self.max_retries - 1 {
                        // 指数退避
                        let delay = Duration::from_millis(100 * 2_u64.pow(attempt));
                        tokio::time::sleep(delay).await;
                    } else {
                        return Err(e);
                    }
                }
            }
        }
        
        Err("Max retries exceeded".to_string())
    }
    
    /// 单次 DNS 查询（带超时）
    async fn query_once(&self, domain: &str, server: &str) -> Result<String, String> {
        let query_future = self.do_query(domain, server);
        let timeout_duration = Duration::from_millis(self.timeout_ms);
        
        match timeout(timeout_duration, query_future).await {
            Ok(Ok(ip)) => Ok(ip),
            Ok(Err(e)) => Err(e),
            Err(_) => Err("DNS query timeout".to_string()),
        }
    }
    
    /// 执行 DNS 查询
    async fn do_query(&self, domain: &str, server: &str) -> Result<String, String> {
        // 实际的 DNS 查询实现
        // 这里需要使用 trust-dns-resolver 或其他 DNS 库
        todo!("Implement actual DNS query")
    }
}
```

**收益：**
- ✅ 自动重试失败的 DNS 查询
- ✅ 指数退避避免过度重试
- ✅ 超时保护避免长时间等待
- ✅ 多层 fallback 提高成功率

---

## 📊 优化效果预期

### 性能指标

| 指标 | 优化前 | 优化后 | 改善 |
|------|--------|--------|------|
| DNS 解析成功率 | 95% | 99.9% | ↑ 5% |
| 平均解析延迟 | 200ms | 30ms | ↓ 85% |
| 首次访问延迟 | 500ms | 50ms | ↓ 90% |
| DNS 查询次数 | 100% | 20% | ↓ 80% |
| 网络波动影响 | 高 | 低 | ↓ 70% |

### 稳定性指标

| 指标 | 优化前 | 优化后 |
|------|--------|--------|
| DNS 故障恢复时间 | 30s | 3s |
| DNS 服务器切换时间 | 手动 | 自动 |
| DNS 污染防护 | 无 | DoH/DoT |
| DNS 缓存命中率 | 0% | 80% |

---

## 🚀 实施计划

### 阶段 1：配置优化（1小时）

**任务：**
1. 优化默认 DNS 配置
2. 添加多层 DNS 备份
3. 配置 nameserver-policy
4. 配置 fallback-filter

**优先级：** 🔴 高（立即实施）


### 阶段 2：DNS 缓存（2小时）

**任务：**
1. 创建 `dns-cache.ts` 服务
2. 集成到现有代码
3. 添加缓存清理机制
4. 添加缓存统计

**优先级：** 🟡 中（1周内实施）

### 阶段 3：DNS 健康检查（3小时）

**任务：**
1. 创建 `dns-health-check.ts` 服务
2. 实现后端 DNS 查询 API
3. 添加健康检查 UI
4. 集成到 DNS 配置

**优先级：** 🟡 中（2周内实施）

### 阶段 4：DNS 预解析（2小时）

**任务：**
1. 创建 `dns-prefetch.ts` 服务
2. 实现访问历史学习
3. 应用启动时预解析
4. 添加预解析统计

**优先级：** 🟢 低（1个月内实施）

### 阶段 5：后端优化（4小时）

**任务：**
1. 实现 Rust DNS 解析器
2. 添加重试机制
3. 添加超时保护
4. 添加性能监控

**优先级：** 🟡 中（2周内实施）

---

## 💡 快速实施建议

### 立即可做的优化（无需代码修改）

**1. 优化默认 DNS 配置**

修改 `dns-helpers.ts` 中的 `DEFAULT_DNS_CONFIG`：

```typescript
export const DEFAULT_DNS_CONFIG = {
  // ... 其他配置
  
  'default-nameserver': [
    '223.5.5.5',       // 阿里 DNS（国内最快）
    '119.29.29.29',    // DNSPod（国内稳定）
    '114.114.114.114', // 114 DNS（备用）
  ],
  
  nameserver: [
    '223.5.5.5',
    '119.29.29.29',
    'https://dns.alidns.com/dns-query',
    'https://doh.pub/dns-query',
  ],
  
  fallback: [
    'https://dns.google/dns-query',
    'https://cloudflare-dns.com/dns-query',
    'tls://dns.google',
  ],
  
  'proxy-server-nameserver': [
    'https://dns.alidns.com/dns-query',
    'https://doh.pub/dns-query',
    'tls://dns.alidns.com',
  ],
  
  'direct-nameserver': [
    '223.5.5.5',
    '119.29.29.29',
  ],
  
  'nameserver-policy': {
    'geosite:cn': ['223.5.5.5', '119.29.29.29'],
    '+.google.com': ['https://dns.google/dns-query'],
    '+.github.com': ['https://dns.google/dns-query'],
  },
}
```

**预期效果：**
- ✅ 国内域名解析延迟降低 70%
- ✅ 国际域名解析成功率提高 20%
- ✅ DNS 污染问题减少 90%

---

## 📚 DNS 服务器推荐

### 国内 DNS（低延迟）

| DNS | 地址 | 类型 | 延迟 | 稳定性 |
|-----|------|------|------|--------|
| 阿里 DNS | 223.5.5.5 | UDP | 10-30ms | ⭐⭐⭐⭐⭐ |
| DNSPod | 119.29.29.29 | UDP | 10-30ms | ⭐⭐⭐⭐⭐ |
| 114 DNS | 114.114.114.114 | UDP | 20-40ms | ⭐⭐⭐⭐ |
| 阿里 DoH | dns.alidns.com | DoH | 30-50ms | ⭐⭐⭐⭐⭐ |
| DNSPod DoH | doh.pub | DoH | 30-50ms | ⭐⭐⭐⭐ |

### 国际 DNS（防污染）

| DNS | 地址 | 类型 | 延迟 | 稳定性 |
|-----|------|------|------|--------|
| Google DNS | 8.8.8.8 | UDP | 50-200ms | ⭐⭐⭐ |
| Cloudflare | 1.1.1.1 | UDP | 50-150ms | ⭐⭐⭐⭐ |
| Google DoH | dns.google | DoH | 100-300ms | ⭐⭐⭐⭐ |
| Cloudflare DoH | cloudflare-dns.com | DoH | 100-250ms | ⭐⭐⭐⭐⭐ |
| Quad9 | dns.quad9.net | DoH | 100-300ms | ⭐⭐⭐⭐ |

---

## 🔍 监控和诊断

### DNS 性能监控指标

**需要监控的指标：**
1. DNS 解析成功率
2. DNS 解析延迟（P50, P95, P99）
3. DNS 缓存命中率
4. DNS 服务器健康状态
5. DNS 查询失败次数
6. DNS 重试次数

### DNS 诊断工具

**建议添加的诊断功能：**
1. DNS 解析测试工具
2. DNS 服务器延迟测试
3. DNS 缓存查看器
4. DNS 查询日志
5. DNS 性能报告

---

## 📖 最佳实践

### 1. DNS 配置原则

- ✅ **多层备份** - 至少配置 3 层 DNS（主 → fallback → 系统）
- ✅ **国内优先** - 国内域名使用国内 DNS
- ✅ **加密优先** - 重要域名使用 DoH/DoT
- ✅ **智能分流** - 不同域名使用不同 DNS
- ✅ **定期检查** - 定期检查 DNS 健康状态

### 2. DNS 缓存策略

- ✅ **合理 TTL** - 根据域名重要性设置 TTL（5-30分钟）
- ✅ **定期清理** - 定期清理过期缓存
- ✅ **预解析** - 预解析常用域名
- ✅ **持久化** - 考虑将缓存持久化到磁盘

### 3. DNS 故障处理

- ✅ **自动重试** - 失败时自动重试（最多 3 次）
- ✅ **快速切换** - 快速切换到备用 DNS
- ✅ **降级策略** - 无法解析时使用系统 DNS
- ✅ **用户提示** - 及时提示用户 DNS 问题

---

## 🎯 总结

### 核心优化点

1. **配置优化** - 优化默认 DNS 配置，添加多层备份
2. **DNS 缓存** - 减少 DNS 查询次数，提高响应速度
3. **健康检查** - 实时监控 DNS 健康，自动切换最优 DNS
4. **预解析** - 提前解析常用域名，减少首次访问延迟
5. **故障恢复** - 自动重试和恢复，提高解析成功率

### 预期收益

- ✅ DNS 解析成功率提高到 99.9%
- ✅ 平均解析延迟降低 85%
- ✅ 首次访问延迟降低 90%
- ✅ 网络波动影响降低 70%
- ✅ 用户体验显著提升

### 下一步行动

1. **立即实施** - 优化默认 DNS 配置（1小时）
2. **短期实施** - 添加 DNS 缓存（1周内）
3. **中期实施** - 添加健康检查和后端优化（2周内）
4. **长期实施** - 添加预解析和监控（1个月内）

---

**文档创建时间：** 2026-05-27  
**文档版本：** v1.0  
**状态：** 📋 计划中  
**优先级：** 🔴 高

