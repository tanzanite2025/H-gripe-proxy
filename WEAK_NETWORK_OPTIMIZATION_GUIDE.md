# 弱网环境优化指南

## 📊 当前状态分析

**分析时间：** 2026-05-27  
**项目：** Clash Verge Clean  
**分析范围：** 网络请求、重试机制、超时配置、用户体验

---

## 🔍 现有机制评估

### 1. 网络请求层 (`services/api.ts`)

#### ✅ 已有的优化

**IP 检测服务：**
```typescript
// 多服务降级策略
const IP_CHECK_SERVICES = [
  'https://api.ip.sb/geoip',
  'https://ipapi.co/json',
  'https://api.ipapi.is/',
  'https://ipwho.is/',
  'https://ip.api.skk.moe/cf-geoip',
  'https://get.geojs.io/v1/ip/geo.json',
]

// 重试机制
await asyncRetry(fetchFunction, {
  retries: 2,           // 最多重试2次
  minTimeout: 1000,     // 最小重试间隔1秒
  maxTimeout: 4000,     // 最大重试间隔4秒
  randomize: true,      // 随机化重试间隔（避免雷鸣效应）
})

// 超时控制
const serviceTimeout = 5000  // 5秒超时
```

**优点：**
- ✅ 多服务降级（6个备用服务）
- ✅ 随机化服务顺序（避免单点故障）
- ✅ 指数退避重试
- ✅ 超时控制
- ✅ AbortController 支持

#### ⚠️ 存在的问题

1. **超时时间偏长**
   - 5秒超时 × 2次重试 × 6个服务 = 最长60秒
   - 弱网环境下用户等待时间过长

2. **缺少快速失败机制**
   - 没有"快速失败"选项
   - 无法根据网络状况动态调整

3. **缺少缓存策略**
   - IP信息变化不频繁，可以缓存
   - 没有离线降级方案


---

### 2. 数据获取层 (SWR/React Query)

#### ✅ 已有的配置

**SWR 配置 (`services/config.ts`)：**
```typescript
const SWR_NOT_SMART = {
  revalidateOnFocus: false,
  revalidateOnReconnect: false,  // ❌ 禁用了重连时重新验证
  revalidateIfStale: false,
  errorRetryCount: 2,
  dedupingInterval: 1500,
  errorRetryInterval: 3000,
}

const SWR_EXTERNAL_API = {
  errorRetryCount: 1,
  errorRetryInterval: 30_000,  // 30秒重试间隔
}
```

**React Query 配置 (`services/query-client.ts`)：**
```typescript
{
  staleTime: 2000,
  retry: 3,
  retryDelay: 5000,
  refetchOnWindowFocus: false,
}
```

#### ⚠️ 存在的问题

1. **禁用了重连时重新验证**
   - `revalidateOnReconnect: false` 
   - 网络恢复后不会自动刷新数据

2. **重试间隔固定**
   - 没有指数退避
   - 弱网环境下可能过于频繁

3. **缺少网络状态感知**
   - 不区分在线/离线状态
   - 不根据网络质量调整策略


---

### 3. 延迟测试 (`services/delay.ts`)

#### ✅ 已有的优化

```typescript
// 超时控制
const timeoutPromise = new Promise((resolve) => {
  setTimeout(() => resolve({ delay: 0 }), timeout)
})
const result = await Promise.race([
  delayProxyByName(name, url, timeout),
  timeoutPromise,
])

// 批量测试并发控制
const actualConcurrency = Math.min(concurrency, names.length, 10)

// 随机延迟避免雷鸣效应
await new Promise((resolve) =>
  setTimeout(resolve, Math.random() * 200)
)

// 最小加载时间（UX优化）
if (elapsedTime < 500) {
  await new Promise((resolve) => setTimeout(resolve, 500 - elapsedTime))
}
```

**优点：**
- ✅ 超时控制
- ✅ 并发限制（最多10个）
- ✅ 随机延迟避免雷鸣效应
- ✅ 最小加载时间（避免闪烁）

#### ⚠️ 存在的问题

1. **默认超时时间过长**
   - 10秒超时对弱网环境过长
   - 批量测试时总时间过长

2. **缺少智能调度**
   - 不根据历史延迟优先测试
   - 不跳过已知不可用的节点

3. **缺少取消机制**
   - 用户无法中途取消批量测试
   - 切换页面时测试仍在后台运行


---

## 🎯 优化建议

### 优先级 1：网络状态感知（高优先级）

#### 1.1 添加网络状态监听

**目标：** 根据网络状态动态调整策略

**实现方案：**

```typescript
// src/services/network-monitor.ts
class NetworkMonitor {
  private online = navigator.onLine
  private quality: 'good' | 'poor' | 'offline' = 'good'
  private listeners = new Set<(status: NetworkStatus) => void>()

  constructor() {
    // 监听在线/离线事件
    window.addEventListener('online', this.handleOnline)
    window.addEventListener('offline', this.handleOffline)
    
    // 定期检测网络质量
    this.startQualityCheck()
  }

  private async startQualityCheck() {
    setInterval(async () => {
      if (!this.online) {
        this.quality = 'offline'
        return
      }

      // 使用小文件测试网络质量
      const start = Date.now()
      try {
        await fetch('https://cp.cloudflare.com/generate_204', {
          method: 'HEAD',
          signal: AbortSignal.timeout(3000),
        })
        const latency = Date.now() - start
        
        this.quality = latency < 500 ? 'good' : 'poor'
      } catch {
        this.quality = 'poor'
      }

      this.notifyListeners()
    }, 30000) // 每30秒检测一次
  }

  getQuality() {
    return this.quality
  }

  isOnline() {
    return this.online
  }

  subscribe(listener: (status: NetworkStatus) => void) {
    this.listeners.add(listener)
    return () => this.listeners.delete(listener)
  }
}

export const networkMonitor = new NetworkMonitor()
```

**收益：**
- ✅ 离线时停止无意义的请求
- ✅ 弱网时降低请求频率
- ✅ 网络恢复时自动重试


#### 1.2 动态调整超时和重试策略

**目标：** 根据网络质量调整参数

**实现方案：**

```typescript
// src/services/adaptive-config.ts
import { networkMonitor } from './network-monitor'

export const getAdaptiveConfig = () => {
  const quality = networkMonitor.getQuality()

  switch (quality) {
    case 'good':
      return {
        timeout: 5000,
        retries: 2,
        retryDelay: 1000,
        concurrency: 10,
      }
    case 'poor':
      return {
        timeout: 10000,      // 弱网时延长超时
        retries: 3,          // 增加重试次数
        retryDelay: 3000,    // 延长重试间隔
        concurrency: 3,      // 降低并发数
      }
    case 'offline':
      return {
        timeout: 0,
        retries: 0,
        retryDelay: 0,
        concurrency: 0,
      }
  }
}

// 在 api.ts 中使用
export const getIpInfo = async () => {
  const config = getAdaptiveConfig()
  
  if (config.timeout === 0) {
    // 离线状态，使用缓存
    return getCachedIpInfo()
  }

  // 使用动态配置
  return await asyncRetry(fetchFunction, {
    retries: config.retries,
    minTimeout: config.retryDelay,
    maxTimeout: config.retryDelay * 3,
  })
}
```

**收益：**
- ✅ 弱网时更宽容的超时设置
- ✅ 好网时更快的响应
- ✅ 离线时避免无效请求


---

### 优先级 2：缓存策略（高优先级）

#### 2.1 IP 信息缓存

**目标：** 减少不必要的网络请求

**实现方案：**

```typescript
// src/services/ip-cache.ts
interface CachedIpInfo {
  data: IpInfo
  timestamp: number
}

const IP_CACHE_KEY = 'clash-verge-ip-info'
const CACHE_TTL = 30 * 60 * 1000 // 30分钟

export const getCachedIpInfo = (): IpInfo | null => {
  try {
    const cached = localStorage.getItem(IP_CACHE_KEY)
    if (!cached) return null

    const { data, timestamp }: CachedIpInfo = JSON.parse(cached)
    
    // 检查是否过期
    if (Date.now() - timestamp > CACHE_TTL) {
      localStorage.removeItem(IP_CACHE_KEY)
      return null
    }

    return data
  } catch {
    return null
  }
}

export const setCachedIpInfo = (data: IpInfo) => {
  const cached: CachedIpInfo = {
    data,
    timestamp: Date.now(),
  }
  localStorage.setItem(IP_CACHE_KEY, JSON.stringify(cached))
}

// 在 api.ts 中使用
export const getIpInfo = async (): Promise<IpInfo> => {
  // 先尝试从缓存获取
  const cached = getCachedIpInfo()
  if (cached) {
    console.debug('使用缓存的IP信息')
    return cached
  }

  // 缓存未命中，发起请求
  const data = await fetchIpInfo()
  
  // 保存到缓存
  setCachedIpInfo(data)
  
  return data
}
```

**收益：**
- ✅ 减少90%的IP检测请求
- ✅ 离线时仍可显示上次的IP信息
- ✅ 提升页面加载速度


#### 2.2 延迟测试结果缓存优化

**目标：** 智能使用历史数据

**实现方案：**

```typescript
// 在 delay.ts 中优化
class DelayManager {
  // 扩展缓存TTL
  private CACHE_TTL = 60 * 60 * 1000 // 从30分钟延长到60分钟

  // 添加持久化缓存
  private saveCacheToDisk() {
    const cacheData = Array.from(this.cache.entries())
    localStorage.setItem('delay-cache', JSON.stringify(cacheData))
  }

  private loadCacheFromDisk() {
    try {
      const cached = localStorage.getItem('delay-cache')
      if (cached) {
        const entries = JSON.parse(cached)
        this.cache = new Map(entries)
      }
    } catch (error) {
      console.error('加载延迟缓存失败', error)
    }
  }

  // 智能测试：优先使用缓存
  async checkDelayWithCache(name: string, group: string, timeout: number) {
    const cached = this.getDelayUpdate(name, group)
    
    // 如果缓存有效且不是错误状态，直接返回
    if (cached && cached.delay > 0 && cached.delay < 1e5) {
      const age = Date.now() - cached.updatedAt
      
      // 缓存未过期，直接使用
      if (age < this.CACHE_TTL) {
        console.debug(`使用缓存的延迟数据: ${name}, ${cached.delay}ms`)
        return cached
      }
    }

    // 缓存无效或过期，重新测试
    return await this.checkDelay(name, group, timeout)
  }
}
```

**收益：**
- ✅ 减少重复的延迟测试
- ✅ 应用重启后保留历史数据
- ✅ 弱网时优先使用缓存


---

### 优先级 3：请求优化（中优先级）

#### 3.1 请求去重

**目标：** 避免重复请求

**实现方案：**

```typescript
// src/services/request-deduplicator.ts
class RequestDeduplicator {
  private pending = new Map<string, Promise<any>>()

  async dedupe<T>(key: string, fn: () => Promise<T>): Promise<T> {
    // 如果已有相同请求在进行中，直接返回
    if (this.pending.has(key)) {
      console.debug(`请求去重: ${key}`)
      return this.pending.get(key)!
    }

    // 创建新请求
    const promise = fn().finally(() => {
      this.pending.delete(key)
    })

    this.pending.set(key, promise)
    return promise
  }
}

export const deduplicator = new RequestDeduplicator()

// 在 api.ts 中使用
export const getIpInfo = async () => {
  return deduplicator.dedupe('ip-info', async () => {
    // 原有的获取逻辑
    return await fetchIpInfo()
  })
}
```

**收益：**
- ✅ 避免同时发起多个相同请求
- ✅ 减少服务器压力
- ✅ 节省带宽

#### 3.2 请求优先级队列

**目标：** 关键请求优先

**实现方案：**

```typescript
// src/services/request-queue.ts
type Priority = 'high' | 'normal' | 'low'

interface QueuedRequest {
  fn: () => Promise<any>
  priority: Priority
  resolve: (value: any) => void
  reject: (error: any) => void
}

class RequestQueue {
  private queue: QueuedRequest[] = []
  private running = 0
  private maxConcurrent = 6

  async enqueue<T>(
    fn: () => Promise<T>,
    priority: Priority = 'normal'
  ): Promise<T> {
    return new Promise((resolve, reject) => {
      this.queue.push({ fn, priority, resolve, reject })
      this.queue.sort((a, b) => {
        const priorityMap = { high: 0, normal: 1, low: 2 }
        return priorityMap[a.priority] - priorityMap[b.priority]
      })
      this.process()
    })
  }

  private async process() {
    if (this.running >= this.maxConcurrent || this.queue.length === 0) {
      return
    }

    this.running++
    const request = this.queue.shift()!

    try {
      const result = await request.fn()
      request.resolve(result)
    } catch (error) {
      request.reject(error)
    } finally {
      this.running--
      this.process()
    }
  }
}

export const requestQueue = new RequestQueue()
```

**收益：**
- ✅ 关键请求（如配置更新）优先执行
- ✅ 非关键请求（如IP检测）可以延后
- ✅ 避免请求拥塞


---

### 优先级 4：用户体验优化（中优先级）

#### 4.1 加载状态优化

**目标：** 让用户知道发生了什么

**实现方案：**

```typescript
// src/components/base/network-status-indicator.tsx
export const NetworkStatusIndicator = () => {
  const [status, setStatus] = useState(networkMonitor.getQuality())

  useEffect(() => {
    return networkMonitor.subscribe((newStatus) => {
      setStatus(newStatus.quality)
    })
  }, [])

  if (status === 'offline') {
    return (
      <Alert severity="error">
        网络已断开，部分功能不可用
      </Alert>
    )
  }

  if (status === 'poor') {
    return (
      <Alert severity="warning">
        网络较慢，请耐心等待
      </Alert>
    )
  }

  return null
}

// 在延迟测试时显示进度
export const DelayTestProgress = ({ total, completed }) => {
  const progress = (completed / total) * 100

  return (
    <Box>
      <LinearProgress variant="determinate" value={progress} />
      <Typography variant="caption">
        正在测试延迟... {completed}/{total}
      </Typography>
    </Box>
  )
}
```

**收益：**
- ✅ 用户知道网络状态
- ✅ 减少焦虑感
- ✅ 提供明确的反馈

#### 4.2 骨架屏和占位符

**目标：** 减少白屏时间

**实现方案：**

```typescript
// src/components/base/skeleton-card.tsx
export const ProxyCardSkeleton = () => {
  return (
    <Card>
      <Skeleton variant="text" width="60%" height={30} />
      <Skeleton variant="rectangular" width="100%" height={60} />
      <Skeleton variant="text" width="40%" height={20} />
    </Card>
  )
}

// 在组件中使用
export const ProxyList = () => {
  const { data, isLoading } = useProxies()

  if (isLoading) {
    return (
      <>
        {Array.from({ length: 5 }).map((_, i) => (
          <ProxyCardSkeleton key={i} />
        ))}
      </>
    )
  }

  return data.map(proxy => <ProxyCard key={proxy.name} proxy={proxy} />)
}
```

**收益：**
- ✅ 减少感知加载时间
- ✅ 提供视觉连续性
- ✅ 更好的用户体验


#### 4.3 取消机制

**目标：** 让用户可以中断长时间操作

**实现方案：**

```typescript
// 在 delay.ts 中添加取消支持
class DelayManager {
  private abortControllers = new Map<string, AbortController>()

  async checkListDelay(
    nameList: string[],
    group: string,
    timeout: number,
    concurrency = 36,
  ) {
    // 创建 AbortController
    const controller = new AbortController()
    this.abortControllers.set(group, controller)

    try {
      // 原有的批量测试逻辑
      // 在每个请求中检查是否已取消
      if (controller.signal.aborted) {
        throw new Error('测试已取消')
      }

      await this.checkDelay(currName, group, timeout)
    } finally {
      this.abortControllers.delete(group)
    }
  }

  cancelGroupTest(group: string) {
    const controller = this.abortControllers.get(group)
    if (controller) {
      controller.abort()
      console.debug(`取消组延迟测试: ${group}`)
    }
  }
}

// 在 UI 中添加取消按钮
export const DelayTestButton = ({ group }) => {
  const [testing, setTesting] = useState(false)

  const handleTest = async () => {
    setTesting(true)
    try {
      await delayManager.checkListDelay(names, group, timeout)
    } finally {
      setTesting(false)
    }
  }

  const handleCancel = () => {
    delayManager.cancelGroupTest(group)
    setTesting(false)
  }

  return (
    <>
      {testing ? (
        <Button onClick={handleCancel} color="error">
          取消测试
        </Button>
      ) : (
        <Button onClick={handleTest}>
          测试延迟
        </Button>
      )}
    </>
  )
}
```

**收益：**
- ✅ 用户可以中断长时间操作
- ✅ 避免资源浪费
- ✅ 提升控制感


---

### 优先级 5：高级优化（低优先级）

#### 5.1 预连接和 DNS 预解析

**目标：** 减少连接建立时间

**实现方案：**

```html
<!-- 在 index.html 中添加 -->
<head>
  <!-- DNS 预解析 -->
  <link rel="dns-prefetch" href="https://api.ip.sb" />
  <link rel="dns-prefetch" href="https://ipapi.co" />
  <link rel="dns-prefetch" href="https://cp.cloudflare.com" />
  
  <!-- 预连接 -->
  <link rel="preconnect" href="https://api.ip.sb" />
  <link rel="preconnect" href="https://ipapi.co" />
</head>
```

**收益：**
- ✅ 减少 DNS 查询时间（50-200ms）
- ✅ 减少 TCP 握手时间（50-200ms）
- ✅ 总计可节省 100-400ms

#### 5.2 Service Worker 缓存

**目标：** 离线可用

**实现方案：**

```typescript
// public/sw.js
const CACHE_NAME = 'clash-verge-v1'
const STATIC_ASSETS = [
  '/',
  '/index.html',
  '/assets/index.js',
  '/assets/index.css',
]

self.addEventListener('install', (event) => {
  event.waitUntil(
    caches.open(CACHE_NAME).then((cache) => {
      return cache.addAll(STATIC_ASSETS)
    })
  )
})

self.addEventListener('fetch', (event) => {
  event.respondWith(
    caches.match(event.request).then((response) => {
      // 缓存命中，返回缓存
      if (response) {
        return response
      }

      // 缓存未命中，发起网络请求
      return fetch(event.request).then((response) => {
        // 缓存新资源
        if (response.status === 200) {
          const responseClone = response.clone()
          caches.open(CACHE_NAME).then((cache) => {
            cache.put(event.request, responseClone)
          })
        }
        return response
      })
    })
  )
})
```

**收益：**
- ✅ 离线时应用仍可访问
- ✅ 静态资源秒开
- ✅ 减少带宽消耗


#### 5.3 智能预加载

**目标：** 预测用户行为，提前加载

**实现方案：**

```typescript
// src/services/prefetch.ts
class PrefetchManager {
  private prefetched = new Set<string>()

  // 预加载代理组数据
  prefetchProxyGroup(groupName: string) {
    if (this.prefetched.has(groupName)) return

    // 在空闲时预加载
    requestIdleCallback(() => {
      queryClient.prefetchQuery({
        queryKey: ['proxy-group', groupName],
        queryFn: () => fetchProxyGroup(groupName),
      })
      this.prefetched.add(groupName)
    })
  }

  // 鼠标悬停时预加载
  onProxyCardHover(proxyName: string) {
    requestIdleCallback(() => {
      queryClient.prefetchQuery({
        queryKey: ['proxy-detail', proxyName],
        queryFn: () => fetchProxyDetail(proxyName),
      })
    })
  }
}

export const prefetchManager = new PrefetchManager()

// 在组件中使用
export const ProxyCard = ({ proxy }) => {
  return (
    <Card
      onMouseEnter={() => prefetchManager.onProxyCardHover(proxy.name)}
    >
      {/* ... */}
    </Card>
  )
}
```

**收益：**
- ✅ 点击时数据已准备好
- ✅ 感觉更快
- ✅ 利用空闲时间

#### 5.4 HTTP/2 和 HTTP/3

**目标：** 使用更高效的协议

**实现方案：**

```typescript
// 在 Tauri 配置中启用 HTTP/2
// tauri.conf.json
{
  "tauri": {
    "allowlist": {
      "http": {
        "all": true,
        "request": true,
        "scope": ["https://**"]
      }
    }
  }
}

// 确保使用的 fetch API 支持 HTTP/2
// Tauri 的 @tauri-apps/plugin-http 默认支持 HTTP/2
```

**收益：**
- ✅ 多路复用（减少连接数）
- ✅ 头部压缩（减少带宽）
- ✅ 服务器推送（更快加载）


---

## 📊 优化效果预估

### 场景 1：正常网络（延迟 < 100ms）

| 操作 | 优化前 | 优化后 | 改善 |
|------|--------|--------|------|
| IP 检测 | 2-5秒 | 0.1秒（缓存） | **95%** ↓ |
| 延迟测试（单个） | 1-2秒 | 0.5-1秒 | **50%** ↓ |
| 延迟测试（批量100个） | 30-60秒 | 15-30秒 | **50%** ↓ |
| 页面加载 | 1-2秒 | 0.5-1秒 | **50%** ↓ |

### 场景 2：弱网环境（延迟 500-1000ms）

| 操作 | 优化前 | 优化后 | 改善 |
|------|--------|--------|------|
| IP 检测 | 10-30秒 | 0.1秒（缓存） | **99%** ↓ |
| 延迟测试（单个） | 5-10秒 | 3-8秒 | **30%** ↓ |
| 延迟测试（批量100个） | 超时 | 60-120秒 | **可完成** ✅ |
| 页面加载 | 5-10秒 | 2-5秒 | **50%** ↓ |

### 场景 3：离线状态

| 操作 | 优化前 | 优化后 | 改善 |
|------|--------|--------|------|
| IP 检测 | 失败 | 显示缓存 | **可用** ✅ |
| 延迟测试 | 失败 | 显示历史 | **可用** ✅ |
| 页面加载 | 失败 | 正常显示 | **可用** ✅ |
| 配置查看 | 失败 | 正常显示 | **可用** ✅ |

---

## 🎯 实施路线图

### 第一阶段：基础优化（1-2天）

**目标：** 快速见效的优化

1. ✅ 添加 IP 信息缓存（30分钟 TTL）
2. ✅ 优化延迟测试缓存（60分钟 TTL）
3. ✅ 添加请求去重
4. ✅ 添加网络状态指示器

**预期收益：**
- 正常网络：50% 性能提升
- 弱网环境：30% 性能提升
- 离线状态：基本可用


### 第二阶段：智能优化（3-5天）

**目标：** 根据网络状况自适应

1. ✅ 实现网络状态监听
2. ✅ 动态调整超时和重试策略
3. ✅ 实现请求优先级队列
4. ✅ 添加取消机制
5. ✅ 优化 SWR 配置（启用 revalidateOnReconnect）

**预期收益：**
- 正常网络：70% 性能提升
- 弱网环境：60% 性能提升
- 离线状态：完全可用

### 第三阶段：高级优化（5-7天）

**目标：** 极致的用户体验

1. ✅ 添加骨架屏和占位符
2. ✅ 实现智能预加载
3. ✅ 添加 DNS 预解析和预连接
4. ✅ 实现 Service Worker 缓存
5. ✅ 优化延迟测试调度算法

**预期收益：**
- 正常网络：80% 性能提升
- 弱网环境：70% 性能提升
- 离线状态：完全可用 + 秒开

---

## 🔧 具体实施步骤

### 步骤 1：创建网络监控服务

```bash
# 创建文件
touch src/services/network-monitor.ts

# 实现网络状态监听
# 参考上面的 NetworkMonitor 类
```

### 步骤 2：添加 IP 信息缓存

```bash
# 创建文件
touch src/services/ip-cache.ts

# 修改 api.ts
# 添加缓存逻辑
```

### 步骤 3：优化延迟测试

```bash
# 修改 delay.ts
# 1. 添加持久化缓存
# 2. 添加取消机制
# 3. 优化批量测试调度
```

### 步骤 4：更新 SWR 配置

```bash
# 修改 config.ts
# 启用 revalidateOnReconnect
```

### 步骤 5：添加 UI 反馈

```bash
# 创建组件
touch src/components/base/network-status-indicator.tsx
touch src/components/base/skeleton-card.tsx

# 在关键页面使用
```


---

## 📝 配置建议

### 推荐的超时配置

```typescript
// 根据网络质量的推荐配置
export const TIMEOUT_CONFIG = {
  good: {
    ipCheck: 5000,        // IP检测：5秒
    delayTest: 5000,      // 延迟测试：5秒
    configFetch: 10000,   // 配置获取：10秒
    apiCall: 8000,        // API调用：8秒
  },
  poor: {
    ipCheck: 10000,       // IP检测：10秒
    delayTest: 10000,     // 延迟测试：10秒
    configFetch: 20000,   // 配置获取：20秒
    apiCall: 15000,       // API调用：15秒
  },
  offline: {
    ipCheck: 0,           // 不请求
    delayTest: 0,         // 不请求
    configFetch: 0,       // 不请求
    apiCall: 0,           // 不请求
  },
}
```

### 推荐的重试配置

```typescript
export const RETRY_CONFIG = {
  good: {
    count: 2,             // 重试2次
    delay: 1000,          // 间隔1秒
    backoff: 2,           // 指数退避系数
  },
  poor: {
    count: 3,             // 重试3次
    delay: 3000,          // 间隔3秒
    backoff: 1.5,         // 指数退避系数
  },
  offline: {
    count: 0,             // 不重试
    delay: 0,
    backoff: 1,
  },
}
```

### 推荐的缓存配置

```typescript
export const CACHE_CONFIG = {
  ipInfo: {
    ttl: 30 * 60 * 1000,      // 30分钟
    staleWhileRevalidate: true,
  },
  delayTest: {
    ttl: 60 * 60 * 1000,      // 60分钟
    staleWhileRevalidate: true,
  },
  proxyList: {
    ttl: 5 * 60 * 1000,       // 5分钟
    staleWhileRevalidate: false,
  },
  config: {
    ttl: 10 * 60 * 1000,      // 10分钟
    staleWhileRevalidate: true,
  },
}
```


---

## 🧪 测试建议

### 1. 弱网模拟测试

**Chrome DevTools 网络限速：**
```
1. 打开 DevTools (F12)
2. 切换到 Network 标签
3. 选择 "Slow 3G" 或 "Fast 3G"
4. 测试各项功能
```

**推荐测试场景：**
- ✅ Slow 3G (400ms RTT, 400kbps 下载)
- ✅ Fast 3G (150ms RTT, 1.6Mbps 下载)
- ✅ 离线模式
- ✅ 间歇性断网（切换在线/离线）

### 2. 性能指标监控

**关键指标：**
```typescript
// 监控首次内容绘制 (FCP)
performance.mark('fcp')

// 监控最大内容绘制 (LCP)
new PerformanceObserver((list) => {
  const entries = list.getEntries()
  const lastEntry = entries[entries.length - 1]
  console.log('LCP:', lastEntry.renderTime || lastEntry.loadTime)
}).observe({ entryTypes: ['largest-contentful-paint'] })

// 监控首次输入延迟 (FID)
new PerformanceObserver((list) => {
  const entries = list.getEntries()
  entries.forEach((entry) => {
    console.log('FID:', entry.processingStart - entry.startTime)
  })
}).observe({ entryTypes: ['first-input'] })
```

**目标值：**
- FCP < 1.8秒
- LCP < 2.5秒
- FID < 100ms
- TTI < 3.8秒

### 3. 用户体验测试

**测试清单：**
- [ ] 弱网环境下页面是否可用
- [ ] 离线状态下是否显示缓存数据
- [ ] 网络恢复后是否自动刷新
- [ ] 长时间操作是否可以取消
- [ ] 是否有明确的加载状态提示
- [ ] 错误信息是否友好
- [ ] 重试是否自动进行


---

## 💡 最佳实践总结

### DO ✅

1. **使用缓存**
   - 缓存不常变化的数据（IP信息、延迟结果）
   - 使用 stale-while-revalidate 策略
   - 持久化关键缓存到 localStorage

2. **智能重试**
   - 使用指数退避
   - 添加随机抖动（避免雷鸣效应）
   - 根据网络质量调整重试策略

3. **超时控制**
   - 所有网络请求都设置超时
   - 根据网络质量动态调整
   - 使用 AbortController 支持取消

4. **用户反馈**
   - 显示网络状态
   - 显示加载进度
   - 提供取消按钮
   - 使用骨架屏

5. **请求优化**
   - 请求去重
   - 优先级队列
   - 并发控制
   - 智能预加载

### DON'T ❌

1. **不要阻塞 UI**
   - 不要在主线程做耗时操作
   - 使用 Web Worker 处理大数据
   - 使用 requestIdleCallback 延迟非关键任务

2. **不要过度请求**
   - 不要同时发起大量请求
   - 不要频繁轮询
   - 不要重复请求相同数据

3. **不要忽略错误**
   - 不要吞掉错误
   - 不要显示技术性错误信息
   - 不要让用户不知所措

4. **不要假设网络良好**
   - 不要使用过短的超时
   - 不要放弃重试
   - 不要忽略离线状态

5. **不要牺牲用户体验**
   - 不要无限等待
   - 不要没有反馈
   - 不要强制刷新


---

## 📚 参考资源

### 网络优化

- [Web.dev - Network Reliability](https://web.dev/reliable/)
- [MDN - Network Information API](https://developer.mozilla.org/en-US/docs/Web/API/Network_Information_API)
- [Google - Offline Cookbook](https://web.dev/offline-cookbook/)

### 性能优化

- [Web Vitals](https://web.dev/vitals/)
- [React Performance Optimization](https://react.dev/learn/render-and-commit)
- [SWR Documentation](https://swr.vercel.app/)

### 用户体验

- [Material Design - Loading](https://m3.material.io/foundations/interaction/states/loading)
- [Nielsen Norman Group - Response Times](https://www.nngroup.com/articles/response-times-3-important-limits/)

---

## 🎯 总结

### 核心要点

1. **网络状态感知是基础**
   - 监听在线/离线事件
   - 检测网络质量
   - 动态调整策略

2. **缓存是关键**
   - 减少不必要的请求
   - 提供离线能力
   - 提升响应速度

3. **用户体验优先**
   - 明确的状态反馈
   - 可取消的长操作
   - 友好的错误提示

4. **智能重试和超时**
   - 指数退避
   - 随机抖动
   - 动态调整

### 预期收益

**性能提升：**
- 正常网络：50-80% 性能提升
- 弱网环境：30-70% 性能提升
- 离线状态：从不可用到完全可用

**用户体验：**
- 更快的响应速度
- 更少的等待时间
- 更好的容错能力
- 更清晰的状态反馈

### 下一步行动

1. **立即实施**（第一阶段）
   - 添加 IP 信息缓存
   - 优化延迟测试缓存
   - 添加请求去重

2. **短期计划**（第二阶段）
   - 实现网络状态监听
   - 动态调整策略
   - 添加取消机制

3. **长期规划**（第三阶段）
   - Service Worker
   - 智能预加载
   - 极致优化

---

**文档创建时间：** 2026-05-27  
**文档版本：** v1.0  
**适用范围：** Clash Verge Clean 项目  
**维护者：** 开发团队
