# DNS 优化快速参考指南

## 快速导航

| 功能 | 文件位置 | 说明 |
|------|----------|------|
| DNS 统计显示 | `src/components/setting/dns-stats-card.tsx` | 显示缓存、健康检查、分流、Tor 统计 |
| DNS 分流配置 | `src/components/setting/dns-routing-card.tsx` | 4 种分流模式选择器 |
| Tor 配置 | `src/components/setting/tor-config-card.tsx` | Tor 代理配置界面 |
| DNS 管理器 | `src/services/dns-manager.ts` | 整合所有 DNS 服务 |
| DNS 智能分流 | `src/services/dns-smart-routing.ts` | 智能分流服务 |
| Tor 代理 | `src/services/tor-proxy.ts` | Tor 代理服务 |
| DNS 缓存 | `src/services/dns-cache.ts` | DNS 缓存服务 |
| DNS 预解析 | `src/services/dns-prefetch.ts` | DNS 预解析服务 |
| DNS 健康检查 | `src/services/dns-health-check.ts` | DNS 健康检查服务 |
| DNS API | `src/services/dns-api.ts` | 前端 API 包装器 |
| DNS 后端 | `src-tauri/src/cmd/dns.rs` | Rust DNS 命令模块 |

---

## DNS 分流模式

### 速度优先模式

```typescript
dnsSmartRoutingService.setMode('speed')
```

- **国内 DNS**: 223.5.5.5 (UDP)
- **国外 DNS**: 223.5.5.5 (UDP)
- **延迟**: 10-30ms
- **隐私**: 低
- **适用场景**: 需要最快速度，不关心隐私

### 平衡模式（推荐）

```typescript
dnsSmartRoutingService.setMode('balanced')
```

- **国内 DNS**: 223.5.5.5 (UDP)
- **国外 DNS**: 1.1.1.1 (DoH)
- **延迟**: 20-40ms
- **隐私**: 中
- **适用场景**: 日常使用，平衡速度和隐私

### 隐私优先模式

```typescript
dnsSmartRoutingService.setMode('privacy')
```

- **国内 DNS**: 1.1.1.1 (DoH)
- **国外 DNS**: 1.1.1.1 (DoH)
- **延迟**: 30-80ms
- **隐私**: 高
- **适用场景**: 需要最强隐私保护

### 自定义模式

```typescript
dnsSmartRoutingService.setCustomConfig({
  mode: 'custom',
  domesticDns: { server: '223.5.5.5', protocol: 'udp' },
  foreignDns: { server: '8.8.8.8', protocol: 'doh' },
  customRules: [
    { pattern: 'github.com', server: '1.1.1.1', protocol: 'doh' },
  ],
})
```

---

## Tor 代理配置

### 启用 Tor

```typescript
torProxyService.enable({
  socksHost: '127.0.0.1',
  socksPort: 9050,
})
```

### 禁用 Tor

```typescript
torProxyService.disable()
```

### 获取 SOCKS5 代理地址

```typescript
const socksUrl = torProxyService.getSocksProxyUrl()
// 返回: "socks5://127.0.0.1:9050"
```

### 检查连接状态

```typescript
const isConnected = await torProxyService.checkConnection()
```

---

## DNS 管理器使用

### 初始化

```typescript
await dnsManager.initialize({
  enableCache: true,
  enablePrefetch: true,
  enableHealthCheck: true,
  enableSmartRouting: true,
  enableTor: false,
  routingMode: 'balanced',
})
```

### 解析域名

```typescript
const ip = await dnsManager.resolve('example.com')
```

### 获取统计信息

```typescript
const stats = dnsManager.getStats()
console.log(stats.cache.hitRate) // 缓存命中率
console.log(stats.health.averageLatency) // 平均延迟
console.log(stats.routing.mode) // 分流模式
console.log(stats.tor.enabled) // Tor 状态
```

### 清空缓存

```typescript
dnsManager.clearCache()
```

### 重置健康检查

```typescript
dnsManager.resetHealthCheck()
```

---

## DNS 缓存服务

### 获取缓存

```typescript
const ip = dnsCacheService.get('example.com')
```

### 设置缓存

```typescript
dnsCacheService.set('example.com', '1.2.3.4', 300) // TTL 300 秒
```

### 清空缓存

```typescript
dnsCacheService.clear()
```

### 获取统计

```typescript
const stats = dnsCacheService.getStats()
console.log(stats.hitRate) // 命中率
console.log(stats.cacheSize) // 缓存大小
```

---

## DNS 预解析服务

### 记录访问

```typescript
dnsPrefetchService.recordAccess('example.com')
```

### 预解析域名

```typescript
await dnsPrefetchService.prefetch('example.com')
```

### 批量预解析

```typescript
await dnsPrefetchService.prefetchBatch(['example.com', 'github.com'])
```

### 启动自动预解析

```typescript
dnsPrefetchService.startAutoPrefetch(300000) // 每 5 分钟
```

### 停止自动预解析

```typescript
dnsPrefetchService.stopAutoPrefetch()
```

---

## DNS 健康检查服务

### 添加服务器

```typescript
dnsHealthCheckService.addServer('8.8.8.8', 'udp')
dnsHealthCheckService.addServer('https://dns.google/dns-query', 'doh')
```

### 检查服务器

```typescript
await dnsHealthCheckService.checkServer('8.8.8.8')
```

### 获取最优服务器

```typescript
const bestServers = dnsHealthCheckService.getBestServers(3)
```

### 启动监控

```typescript
dnsHealthCheckService.startMonitoring(60000) // 每 1 分钟
```

### 停止监控

```typescript
dnsHealthCheckService.stopMonitoring()
```

---

## DNS 后端 API

### DNS 查询

```typescript
const result = await dnsQuery('example.com', {
  server: '8.8.8.8',
  protocol: 'udp',
})
console.log(result.ip) // IP 地址
console.log(result.latency) // 延迟（毫秒）
```

### 健康检查

```typescript
const result = await dnsHealthCheck('8.8.8.8', 'google.com', 'udp')
console.log(result.healthy) // 是否健康
console.log(result.latency) // 延迟（毫秒）
```

### 批量查询

```typescript
const results = await dnsBatchQuery(
  ['example.com', 'github.com'],
  '8.8.8.8',
  'udp',
)
```

### 批量健康检查

```typescript
const results = await dnsBatchHealthCheck(
  ['8.8.8.8', '1.1.1.1'],
  'google.com',
  'udp',
)
```

---

## UI 组件使用

### DNS 统计卡片

```tsx
import { DnsStatsCard } from '@/components/setting/dns-stats-card'

<DnsStatsCard />
```

**显示内容**:
- DNS 缓存统计（总查询、命中率、缓存大小）
- DNS 健康检查统计（健康/降级/故障服务器、平均延迟）
- DNS 预解析统计（常用域名数、访问历史数）
- DNS 智能分流统计（分流模式、国内/国外 DNS、自定义规则）
- Tor 代理统计（状态、连接状态、SOCKS5 地址）

### DNS 分流卡片

```tsx
import { DnsRoutingCard } from '@/components/setting/dns-routing-card'

<DnsRoutingCard />
```

**功能**:
- 4 种分流模式切换（速度/平衡/隐私/自定义）
- 当前配置显示
- 性能提示

### Tor 配置卡片

```tsx
import { TorConfigCard } from '@/components/setting/tor-config-card'

<TorConfigCard />
```

**功能**:
- Tor 启用/禁用开关
- SOCKS5 配置（主机、端口）
- 连接状态显示
- 使用说明（可折叠）

---

## 常见问题

### Q: 如何切换 DNS 分流模式？

A: 在设置页面的 "DNS 智能分流" 卡片中点击对应的模式按钮即可。

### Q: 如何启用 Tor 代理？

A: 在设置页面的 "Tor 代理" 卡片中打开开关，并确保本地 Tor 服务运行在 127.0.0.1:9050。

### Q: 如何查看 DNS 缓存命中率？

A: 在设置页面的 "DNS 统计" 卡片中查看 "缓存命中率" 指标。

### Q: 如何清空 DNS 缓存？

A: 在设置页面的 "DNS 统计" 卡片中点击 "清空缓存" 按钮。

### Q: 如何添加自定义 DNS 规则？

A: 使用代码添加：
```typescript
dnsSmartRoutingService.addCustomRule(
  'github.com',
  '1.1.1.1',
  'doh',
)
```

### Q: 如何检查 DNS 服务器健康状态？

A: 在设置页面的 "DNS 统计" 卡片中查看 "DNS 健康检查" 部分。

---

## 性能优化建议

### 1. 选择合适的分流模式

- **国内用户**: 推荐使用 "平衡模式"
- **需要速度**: 使用 "速度优先模式"
- **需要隐私**: 使用 "隐私优先模式"

### 2. 启用 DNS 缓存

```typescript
dnsManager.updateConfig({ enableCache: true })
```

### 3. 启用 DNS 预解析

```typescript
dnsManager.updateConfig({ enablePrefetch: true })
```

### 4. 启用健康检查

```typescript
dnsManager.updateConfig({ enableHealthCheck: true })
```

### 5. 调整缓存 TTL

```typescript
dnsCacheService.setConfig({ defaultTtl: 600 }) // 10 分钟
```

---

## 调试技巧

### 查看 DNS 查询日志

```typescript
// 在浏览器控制台中
console.log(dnsManager.getStats())
```

### 查看缓存内容

```typescript
// 在浏览器控制台中
console.log(dnsCacheService.getStats())
```

### 查看健康检查结果

```typescript
// 在浏览器控制台中
console.log(dnsHealthCheckService.getStats())
```

### 测试 DNS 查询

```typescript
// 在浏览器控制台中
await dnsManager.resolve('example.com')
```

---

## 相关文档

- [DNS Config 组件重构](./DNS_CONFIG_REFACTOR_COMPLETE.md)
- [DNS 稳定性优化 - 阶段 1](./DNS_STABILITY_OPTIMIZATION_PHASE1_COMPLETE.md)
- [DNS 稳定性优化 - 阶段 2](./DNS_STABILITY_OPTIMIZATION_PHASE2_COMPLETE.md)
- [DoH/DoT 实现](./DOH_DOT_IMPLEMENTATION_COMPLETE.md)
- [DNS 后端集成](./DNS_BACKEND_INTEGRATION_COMPLETE.md)
- [DNS 智能分流和 Tor](./DNS_SMART_ROUTING_TOR_COMPLETE.md)
- [DNS UI 集成](./DNS_UI_INTEGRATION_COMPLETE.md)
- [项目总结](./DNS_OPTIMIZATION_PROJECT_SUMMARY.md)

---

**最后更新**: 2026-05-27
