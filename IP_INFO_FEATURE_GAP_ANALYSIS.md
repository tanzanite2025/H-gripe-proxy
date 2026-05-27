# IP 信息功能差距分析

## 📊 现有实现分析

### ✅ 已有功能（不需要重复造轮子）

#### 1. IP 信息查询服务 (`src/services/api.ts`)
**状态**: ✅ 已完善，架构优秀

**现有特性**:
- ✅ **6个国际服务源**，带自动故障转移
  - api.ip.sb/geoip
  - ipapi.co/json
  - api.ipapi.is
  - ipwho.is
  - ip.api.skk.moe/cf-geoip
  - get.geojs.io/v1/ip/geo.json
- ✅ **智能服务选择**: 随机打乱顺序，避免单点故障
- ✅ **服务特定字段映射**: 每个服务有独立的数据转换逻辑
- ✅ **重试机制**: 使用 `asyncRetry` 自动重试失败请求
- ✅ **超时控制**: 基于网络质量的自适应超时
- ✅ **请求去重**: 使用 `deduplicator` 避免重复请求
- ✅ **User-Agent**: 自动生成应用标识

**数据字段**:
```typescript
interface IpInfo {
  ip: string                    // IP 地址
  country_code: string          // 国家代码
  country: string               // 国家名称
  region: string                // 地区/省份
  city: string                  // 城市
  organization: string          // ISP/组织
  asn: number                   // ASN 号码
  asn_organization: string      // ASN 组织
  longitude: number             // 经度
  latitude: number              // 纬度
  timezone: string              // 时区
  lastFetchTs: number           // 最后获取时间戳
}
```

#### 2. IP 信息缓存 (`src/services/ip-cache.ts`)
**状态**: ✅ 已完善

**现有特性**:
- ✅ **30分钟缓存 TTL**: 避免频繁请求
- ✅ **LocalStorage 存储**: 持久化缓存
- ✅ **缓存年龄追踪**: 可查询缓存剩余时间
- ✅ **自动过期清理**: 过期自动删除
- ✅ **错误处理**: 缓存读写失败不影响主流程

#### 3. 自适应网络配置 (`src/services/adaptive-config.ts`)
**状态**: ✅ 已完善

**现有特性**:
- ✅ **网络质量感知**: 根据 good/poor/offline 调整参数
- ✅ **动态超时**: 好网络5秒，弱网络10秒
- ✅ **动态重试**: 好网络2次，弱网络3次
- ✅ **离线保护**: 离线时不发起请求

#### 4. IP 信息卡片 UI (`src/components/home/ip-info-card.tsx`)
**状态**: ✅ 已完善

**现有特性**:
- ✅ **自动刷新**: 300秒倒计时自动刷新
- ✅ **智能刷新策略**:
  - 只在卡片进入视口后才开始倒计时（IntersectionObserver）
  - 窗口隐藏时暂停倒计时（节能）
  - 离线时不刷新
  - 窗口不可见时不刷新
- ✅ **React Query 集成**: 自动缓存和状态管理
- ✅ **手动刷新**: 点击刷新按钮
- ✅ **IP 显示/隐藏切换**: 隐私保护
- ✅ **加载状态**: Skeleton 加载动画
- ✅ **错误处理**: 友好的错误提示和重试按钮

#### 5. IP 信息展示 (`src/components/home/ip-info-card-ui.tsx`)
**状态**: ✅ 已完善

**现有特性**:
- ✅ **国旗表情**: 根据国家代码显示国旗
- ✅ **信息展示**: IP、国家、城市、地区、ASN、ISP、时区、坐标
- ✅ **IP 隐藏**: 点击眼睛图标切换显示/隐藏
- ✅ **自动刷新倒计时**: 显示剩余刷新时间
- ✅ **响应式布局**: 左右分栏，信息清晰
- ✅ **Tailwind CSS**: 已完成 MUI 迁移

---

## ❌ 缺失功能（需要实现）

### 🔴 高优先级（立即实施）

#### 1. 国内 IP 检测服务 ⚠️ 部分缺失
**现状**: 只有 `api.ip.sb` 一个国内可用服务，其他5个都是国外服务

**需要添加**:
- ❌ ipip.net (国内专业 IP 库)
- ❌ vore.top (国内服务)
- ❌ ip.taobao.com (淘宝 IP 库，可能已停用，需验证)
- ❌ ip.ws.126.net (网易 IP 库)

**实施方案**:
```typescript
// 在 src/services/api.ts 的 IP_CHECK_SERVICES 数组中添加
{
  url: 'https://myip.ipip.net/json',
  mapping: (data) => ({
    ip: data.ip || '',
    country_code: data.country_code || '',
    country: data.data?.country || '',
    region: data.data?.province || '',
    city: data.data?.city || '',
    organization: data.data?.isp || '',
    asn: data.data?.asn || 0,
    asn_organization: data.data?.isp || '',
    longitude: data.data?.longitude || 0,
    latitude: data.data?.latitude || 0,
    timezone: data.data?.timezone || '',
  }),
},
{
  url: 'https://api.vore.top/api/IPdata',
  mapping: (data) => ({
    ip: data.ip || data.ipip || '',
    country_code: data.adcode?.country || '',
    country: data.ipip_country || '',
    region: data.ipip_province || '',
    city: data.ipip_city || '',
    organization: data.isp || '',
    asn: 0, // vore.top 不提供 ASN
    asn_organization: data.isp || '',
    longitude: data.ipip_longitude || 0,
    latitude: data.ipip_latitude || 0,
    timezone: '',
  }),
}
```

**优先级**: 🔴 最高（国内用户体验关键）

#### 2. 代理检测 ❌ 完全缺失
**现状**: 无法验证代理是否生效

**需要实现**:
- ❌ 直连 IP 检测
- ❌ 代理 IP 检测
- ❌ IP 地址对比
- ❌ 地理位置对比
- ❌ 代理类型检测（HTTP/HTTPS/SOCKS5）
- ❌ 代理头检测（X-Forwarded-For 等）

**实施方案**:
```typescript
// 新建 src/services/proxy-detection.ts
interface ProxyDetectionResult {
  directIP: string
  proxyIP: string
  isProxyWorking: boolean
  directLocation: Location
  proxyLocation: Location
  locationChanged: boolean
  proxyType?: 'HTTP' | 'HTTPS' | 'SOCKS5'
  proxyHeaders?: Record<string, string>
}

export async function detectProxy(): Promise<ProxyDetectionResult> {
  // 1. 获取直连 IP（通过系统 API 或特殊端点）
  const directIP = await getDirectIP()
  
  // 2. 获取代理 IP（通过当前配置）
  const proxyIP = await getIpInfo()
  
  // 3. 对比结果
  return {
    directIP: directIP.ip,
    proxyIP: proxyIP.ip,
    isProxyWorking: directIP.ip !== proxyIP.ip,
    directLocation: directIP.location,
    proxyLocation: proxyIP.location,
    locationChanged: directIP.country !== proxyIP.country,
  }
}
```

**UI 展示**:
```
┌─────────────────────────────────────┐
│ 🔒 代理状态                          │
├─────────────────────────────────────┤
│ ✅ 代理已生效                        │
│                                     │
│ 直连 IP: 1.2.3.4 (中国 北京)        │
│ 代理 IP: 5.6.7.8 (美国 纽约)        │
│                                     │
│ 位置变化: 中国 → 美国                │
└─────────────────────────────────────┘
```

**优先级**: 🔴 高（用户核心需求）

#### 3. DNS 泄漏检测 ❌ 完全缺失
**现状**: 无法检测 DNS 是否泄漏真实位置

**需要实现**:
- ❌ DNS 服务器检测
- ❌ DNS 地理位置检测
- ❌ DNS 与代理位置对比
- ❌ 泄漏风险评估
- ❌ 修复建议

**实施方案**:
```typescript
// 新建 src/services/dns-leak-detection.ts
interface DNSLeakResult {
  dnsServers: string[]
  isDNSLeaking: boolean
  dnsLocation: string
  ipLocation: string
  locationMatch: boolean
  riskLevel: 'safe' | 'warning' | 'danger'
  recommendations: string[]
}

export async function detectDNSLeak(): Promise<DNSLeakResult> {
  // 1. 查询特殊域名获取 DNS 服务器
  const dnsServers = await queryDNSServers()
  
  // 2. 获取当前 IP 位置
  const ipInfo = await getIpInfo()
  
  // 3. 获取 DNS 服务器位置
  const dnsLocations = await Promise.all(
    dnsServers.map(dns => getIPLocation(dns))
  )
  
  // 4. 判断是否泄漏
  const isLeaking = dnsLocations.some(
    loc => loc.country !== ipInfo.country
  )
  
  return {
    dnsServers,
    isDNSLeaking: isLeaking,
    dnsLocation: dnsLocations[0]?.country || 'Unknown',
    ipLocation: ipInfo.country,
    locationMatch: !isLeaking,
    riskLevel: isLeaking ? 'danger' : 'safe',
    recommendations: isLeaking 
      ? ['启用 DNS over HTTPS', '使用代理的 DNS 服务器']
      : []
  }
}

// 使用 DNS 泄漏检测服务
async function queryDNSServers(): Promise<string[]> {
  const response = await fetch('https://www.dnsleaktest.com/api/query')
  const data = await response.json()
  return data.servers || []
}
```

**UI 展示**:
```
┌─────────────────────────────────────┐
│ 🛡️ DNS 安全检测                     │
├─────────────────────────────────────┤
│ ⚠️  DNS 可能泄漏                     │
│                                     │
│ DNS 服务器: 8.8.8.8 (美国)          │
│ 代理位置: 日本                       │
│                                     │
│ 风险等级: 警告                       │
│                                     │
│ 建议:                                │
│ • 启用 DNS over HTTPS               │
│ • 使用代理的 DNS 服务器              │
└─────────────────────────────────────┘
```

**优先级**: 🔴 高（安全关键）

#### 4. 真实速度测试 ❌ 完全缺失
**现状**: 只有延迟测试（ping），没有下载/上传速度测试

**需要实现**:
- ❌ 下载速度测试
- ❌ 上传速度测试
- ❌ 实时速度显示
- ❌ 速度曲线图
- ❌ 丢包率测试
- ❌ 抖动（jitter）测试

**实施方案**:
```typescript
// 新建 src/services/speed-test.ts
interface SpeedTestResult {
  download: {
    speed: number        // Mbps
    duration: number     // ms
    dataSize: number     // MB
    stability: number    // 0-100
  }
  upload: {
    speed: number
    duration: number
    dataSize: number
    stability: number
  }
  latency: {
    min: number
    max: number
    avg: number
    jitter: number
  }
  packetLoss: {
    sent: number
    received: number
    lossRate: number     // %
  }
}

export class SpeedTestService {
  async testDownloadSpeed(
    onProgress?: (speed: number, progress: number) => void
  ): Promise<SpeedTestResult['download']> {
    // 使用 Cloudflare Speed Test 或自建服务器
    const testFile = 'https://speed.cloudflare.com/__down?bytes=10000000'
    const startTime = performance.now()
    let downloadedBytes = 0
    
    const response = await fetch(testFile)
    const reader = response.body!.getReader()
    
    const speeds: number[] = []
    
    while (true) {
      const { done, value } = await reader.read()
      if (done) break
      
      downloadedBytes += value.length
      
      // 计算实时速度
      const elapsed = performance.now() - startTime
      const speed = (downloadedBytes * 8) / (elapsed / 1000) / 1000000 // Mbps
      speeds.push(speed)
      
      onProgress?.(speed, downloadedBytes / 10000000)
    }
    
    const totalTime = performance.now() - startTime
    const avgSpeed = (downloadedBytes * 8) / (totalTime / 1000) / 1000000
    
    // 计算稳定性（速度方差）
    const stability = calculateStability(speeds)
    
    return {
      speed: avgSpeed,
      duration: totalTime,
      dataSize: downloadedBytes / 1000000,
      stability
    }
  }
  
  async testUploadSpeed(
    onProgress?: (speed: number, progress: number) => void
  ): Promise<SpeedTestResult['upload']> {
    // 上传测试实现
    const testData = new Uint8Array(5000000) // 5MB
    const startTime = performance.now()
    
    const response = await fetch('https://speed.cloudflare.com/__up', {
      method: 'POST',
      body: testData,
    })
    
    const totalTime = performance.now() - startTime
    const avgSpeed = (testData.length * 8) / (totalTime / 1000) / 1000000
    
    return {
      speed: avgSpeed,
      duration: totalTime,
      dataSize: testData.length / 1000000,
      stability: 100
    }
  }
}
```

**UI 展示**:
```
┌─────────────────────────────────────┐
│ ⚡ 网络速度测试                      │
├─────────────────────────────────────┤
│ 下载: ████████░░ 85.3 Mbps          │
│ 上传: ██████░░░░ 62.1 Mbps          │
│                                     │
│ 延迟: 25ms (抖动: 3ms)              │
│ 丢包: 0.1%                          │
│                                     │
│ [开始测试] [停止]                    │
└─────────────────────────────────────┘
```

**优先级**: 🔴 高（用户核心需求）

---

### 🟡 中优先级（1-2周）

#### 5. WebRTC 泄漏检测 ❌ 完全缺失
**需要实现**: 检测 WebRTC 是否泄漏真实 IP

**实施方案**:
```typescript
// 新建 src/services/webrtc-leak-detection.ts
interface WebRTCLeakResult {
  localIPs: string[]
  publicIPs: string[]
  isLeaking: boolean
  leakedIPs: string[]
  riskLevel: 'safe' | 'warning' | 'danger'
}

export async function detectWebRTCLeak(): Promise<WebRTCLeakResult> {
  return new Promise((resolve) => {
    const localIPs: string[] = []
    const publicIPs: string[] = []
    
    const pc = new RTCPeerConnection({
      iceServers: [{ urls: 'stun:stun.l.google.com:19302' }]
    })
    
    pc.createDataChannel('')
    pc.createOffer().then(offer => pc.setLocalDescription(offer))
    
    pc.onicecandidate = (ice) => {
      if (!ice || !ice.candidate) {
        pc.close()
        
        // 判断是否泄漏
        const currentIP = await getIpInfo()
        const isLeaking = publicIPs.some(ip => ip !== currentIP.ip)
        
        resolve({
          localIPs,
          publicIPs,
          isLeaking,
          leakedIPs: isLeaking ? publicIPs : [],
          riskLevel: isLeaking ? 'danger' : 'safe'
        })
        return
      }
      
      const ipMatch = /([0-9]{1,3}\.){3}[0-9]{1,3}/.exec(ice.candidate.candidate)
      if (ipMatch) {
        const ip = ipMatch[0]
        if (ip.startsWith('192.168.') || ip.startsWith('10.')) {
          localIPs.push(ip)
        } else {
          publicIPs.push(ip)
        }
      }
    }
  })
}
```

**优先级**: 🟡 中（安全重要，但不如 DNS 泄漏紧急）

#### 6. 历史记录和对比 ❌ 完全缺失
**需要实现**: 追踪 IP 变化，对比不同时间/代理的信息

**实施方案**:
```typescript
// 新建 src/services/ip-history.ts
interface IPHistoryRecord {
  id: string
  timestamp: number
  ip: string
  location: Location
  isp: string
  proxyName?: string
  proxyType?: string
  speedTest?: SpeedTestResult
  dnsLeak?: DNSLeakResult
  webrtcLeak?: WebRTCLeakResult
  tags: string[]
  notes?: string
}

export class IPHistoryService {
  private readonly STORAGE_KEY = 'clash-verge-ip-history'
  private readonly MAX_RECORDS = 100
  
  async saveRecord(record: Omit<IPHistoryRecord, 'id' | 'timestamp'>): Promise<void> {
    const history = this.getHistory()
    const newRecord: IPHistoryRecord = {
      ...record,
      id: crypto.randomUUID(),
      timestamp: Date.now()
    }
    
    history.unshift(newRecord)
    
    // 限制记录数量
    if (history.length > this.MAX_RECORDS) {
      history.splice(this.MAX_RECORDS)
    }
    
    localStorage.setItem(this.STORAGE_KEY, JSON.stringify(history))
  }
  
  getHistory(): IPHistoryRecord[] {
    const data = localStorage.getItem(this.STORAGE_KEY)
    return data ? JSON.parse(data) : []
  }
  
  compareRecords(id1: string, id2: string): ComparisonResult {
    const history = this.getHistory()
    const record1 = history.find(r => r.id === id1)
    const record2 = history.find(r => r.id === id2)
    
    if (!record1 || !record2) {
      throw new Error('记录不存在')
    }
    
    return {
      ipChanged: record1.ip !== record2.ip,
      locationChanged: record1.location.country !== record2.location.country,
      speedDiff: calculateSpeedDiff(record1.speedTest, record2.speedTest),
      // ... 更多对比
    }
  }
}
```

**优先级**: 🟡 中（用户体验提升）

#### 7. 综合质量评分 ❌ 完全缺失
**需要实现**: 一眼看出网络质量

**实施方案**:
```typescript
// 新建 src/services/quality-score.ts
interface QualityScore {
  overall: number          // 总分 0-100
  latency: number          // 延迟得分
  speed: number            // 速度得分
  stability: number        // 稳定性得分
  security: number         // 安全性得分
  grade: 'A+' | 'A' | 'B' | 'C' | 'D' | 'F'
  recommendations: string[]
}

export function calculateQualityScore(
  speedTest: SpeedTestResult,
  dnsLeak: DNSLeakResult,
  webrtcLeak: WebRTCLeakResult
): QualityScore {
  // 延迟得分 (0-25分)
  const latencyScore = calculateLatencyScore(speedTest.latency.avg)
  
  // 速度得分 (0-35分)
  const speedScore = calculateSpeedScore(
    speedTest.download.speed,
    speedTest.upload.speed
  )
  
  // 稳定性得分 (0-25分)
  const stabilityScore = calculateStabilityScore(
    speedTest.latency.jitter,
    speedTest.packetLoss.lossRate
  )
  
  // 安全性得分 (0-15分)
  const securityScore = calculateSecurityScore(dnsLeak, webrtcLeak)
  
  const overall = latencyScore + speedScore + stabilityScore + securityScore
  
  return {
    overall,
    latency: latencyScore,
    speed: speedScore,
    stability: stabilityScore,
    security: securityScore,
    grade: getGrade(overall),
    recommendations: generateRecommendations({
      speedTest,
      dnsLeak,
      webrtcLeak
    })
  }
}

function getGrade(score: number): QualityScore['grade'] {
  if (score >= 95) return 'A+'
  if (score >= 85) return 'A'
  if (score >= 75) return 'B'
  if (score >= 60) return 'C'
  if (score >= 50) return 'D'
  return 'F'
}
```

**优先级**: 🟡 中（用户体验提升）

#### 8. 趋势图表 ❌ 完全缺失
**需要实现**: 可视化网络性能变化

**实施方案**: 使用 recharts 或 chart.js

**优先级**: 🟡 中（可视化增强）

---

### 🟢 低优先级（长期）

#### 9. 地图可视化 ❌ 完全缺失
**需要实现**: 显示地理位置和路由路径

**实施方案**: 使用 Leaflet.js 或 Mapbox GL

**优先级**: 🟢 低（锦上添花）

#### 10. 专业诊断面板 ❌ 完全缺失
**需要实现**: 完整的网络诊断工具（Traceroute, MTU, 端口扫描等）

**优先级**: 🟢 低（专业用户需求）

#### 11. 智能推荐 ❌ 完全缺失
**需要实现**: 根据场景推荐最优代理

**优先级**: 🟢 低（AI 增强）

---

## 📋 实施优先级总结

### 第一阶段（本周）- 国内服务和基础检测
1. ✅ **添加国内 IP 服务** (ipip.net, vore.top) - 2小时
2. ✅ **代理检测** - 4小时
3. ✅ **DNS 泄漏检测** - 4小时

**预计工作量**: 1-2天

### 第二阶段（下周）- 速度测试
4. ✅ **真实速度测试** (下载/上传) - 8小时
5. ✅ **延迟和丢包测试** - 4小时
6. ✅ **速度测试 UI** - 4小时

**预计工作量**: 2-3天

### 第三阶段（2周后）- 高级功能
7. ✅ **WebRTC 泄漏检测** - 4小时
8. ✅ **历史记录** - 6小时
9. ✅ **综合质量评分** - 4小时
10. ✅ **趋势图表** - 6小时

**预计工作量**: 3-4天

### 第四阶段（长期）- 专业功能
11. ⏸️ **地图可视化** - 8小时
12. ⏸️ **专业诊断面板** - 16小时
13. ⏸️ **智能推荐** - 12小时

**预计工作量**: 5-7天

---

## 🎯 立即开始的任务

### Task 1: 添加国内 IP 服务
**文件**: `src/services/api.ts`

**修改点**: 在 `IP_CHECK_SERVICES` 数组中添加国内服务

**代码位置**: 第 40-120 行

**预计时间**: 2小时

### Task 2: 实现代理检测
**新建文件**: `src/services/proxy-detection.ts`

**新建组件**: `src/components/home/proxy-detection-card.tsx`

**预计时间**: 4小时

### Task 3: 实现 DNS 泄漏检测
**新建文件**: `src/services/dns-leak-detection.ts`

**新建组件**: `src/components/home/dns-leak-card.tsx`

**预计时间**: 4小时

---

## 🚀 准备好开始了吗？

**建议**: 从 Task 1 开始，因为它最简单且影响最大（国内用户体验）

**下一步**: 我可以立即开始实现 Task 1（添加国内 IP 服务），需要我开始吗？
