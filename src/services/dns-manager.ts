/**
 * DNS 管理器
 * 整合 DNS 缓存、预解析、健康检查、智能分流等功能
 */

import { dnsCacheService, type DnsCacheStats } from './dns-cache'
import { dnsHealthCheckService, type DnsHealthStats } from './dns-health-check'
import { dnsPrefetchService } from './dns-prefetch'
import { dnsSmartRoutingService, type DnsRoutingMode } from './dns-smart-routing'
import { torProxyService } from './tor-proxy'
import { dnsLeakProtectionService, type DnsLeakProtectionLevel } from './dns-leak-protection'
import { dnsQuery } from './dns-api'

interface DnsManagerConfig {
  enableCache: boolean
  enablePrefetch: boolean
  enableHealthCheck: boolean
  enableSmartRouting: boolean // 启用智能分流
  enableTor: boolean // 启用 Tor 支持
  enableLeakProtection: boolean // 启用零泄漏防护
  prefetchInterval: number // 预解析间隔（毫秒）
  healthCheckInterval: number // 健康检查间隔（毫秒）
  routingMode: DnsRoutingMode // DNS 分流模式
  leakProtectionLevel: DnsLeakProtectionLevel // 零泄漏防护级别
}

interface DnsManagerStats {
  cache: DnsCacheStats
  health: DnsHealthStats
  prefetch: {
    commonDomains: number
    accessHistory: number
  }
  routing: {
    mode: DnsRoutingMode
    domesticDns: string
    foreignDns: string
    customRulesCount: number
  }
  tor: {
    enabled: boolean
    connected: boolean
    socksProxy: string
  }
  leakProtection: {
    level: DnsLeakProtectionLevel
    levelName: string
    security: string
    safe: boolean
  }
}

class DnsManager {
  private config: DnsManagerConfig = {
    enableCache: true,
    enablePrefetch: true,
    enableHealthCheck: true,
    enableSmartRouting: true,
    enableTor: false,
    enableLeakProtection: true,
    prefetchInterval: 300000, // 5 分钟
    healthCheckInterval: 60000, // 1 分钟
    routingMode: 'balanced',
    leakProtectionLevel: 'basic',
  }

  private initialized = false

  /**
   * 初始化 DNS 管理器
   */
  async initialize(config?: Partial<DnsManagerConfig>): Promise<void> {
    if (this.initialized) {
      console.log('DNS Manager: already initialized')
      return
    }

    // 合并配置
    if (config) {
      this.config = { ...this.config, ...config }
    }

    console.log('DNS Manager: initializing...', this.config)

    // 初始化智能分流
    if (this.config.enableSmartRouting) {
      dnsSmartRoutingService.setMode(this.config.routingMode)
      console.log('DNS Manager: smart routing enabled')
    }

    // 初始化零泄漏防护
    if (this.config.enableLeakProtection) {
      dnsLeakProtectionService.setLevel(this.config.leakProtectionLevel)
      console.log('DNS Manager: leak protection enabled')
    }

    // 初始化 Tor
    if (this.config.enableTor) {
      torProxyService.enable()
      console.log('DNS Manager: Tor proxy enabled')
    }

    // 初始化 DNS 服务器列表
    if (this.config.enableHealthCheck) {
      this.initializeDnsServers()
      dnsHealthCheckService.startMonitoring(this.config.healthCheckInterval)
    }

    // 启动预解析
    if (this.config.enablePrefetch) {
      // 如果启用智能分流，配置预解析使用 DoH（隐私优先）
      if (this.config.enableSmartRouting && this.config.routingMode === 'privacy') {
        dnsPrefetchService.setConfig({ useDoH: true })
      }
      dnsPrefetchService.startAutoPrefetch(this.config.prefetchInterval)
    }

    this.initialized = true
    console.log('DNS Manager: initialized successfully')
  }

  /**
   * 初始化 DNS 服务器列表
   */
  private initializeDnsServers(): void {
    // 国内 DNS
    dnsHealthCheckService.addServer('223.5.5.5', 'udp')
    dnsHealthCheckService.addServer('119.29.29.29', 'udp')
    dnsHealthCheckService.addServer('114.114.114.114', 'udp')

    // 国内 DoH
    dnsHealthCheckService.addServer('https://dns.alidns.com/dns-query', 'doh')
    dnsHealthCheckService.addServer('https://doh.pub/dns-query', 'doh')

    // 国际 DNS
    dnsHealthCheckService.addServer('8.8.8.8', 'udp')
    dnsHealthCheckService.addServer('1.1.1.1', 'udp')

    // 国际 DoH
    dnsHealthCheckService.addServer('https://dns.google/dns-query', 'doh')
    dnsHealthCheckService.addServer('https://cloudflare-dns.com/dns-query', 'doh')

    console.log('DNS Manager: initialized DNS servers')
  }

  /**
   * 解析域名（带缓存和智能分流）
   */
  async resolve(domain: string): Promise<string> {
    // 检查缓存
    if (this.config.enableCache) {
      const cached = dnsCacheService.get(domain)
      if (cached) {
        console.log(`DNS Manager: cache hit for ${domain} -> ${cached}`)
        return cached
      }
    }

    // 记录访问（用于学习）
    if (this.config.enablePrefetch) {
      dnsPrefetchService.recordAccess(domain)
    }

    // 选择 DNS 配置（智能分流）
    let dnsOptions = {}
    if (this.config.enableSmartRouting) {
      dnsOptions = dnsSmartRoutingService.selectDnsConfig(domain)
      console.log(`DNS Manager: routing ${domain} ->`, dnsOptions)
    }

    // 实际解析（调用后端 API）
    try {
      const result = await dnsQuery(domain, dnsOptions)
      
      if (result.success && result.ip) {
        // 缓存结果
        if (this.config.enableCache) {
          dnsCacheService.set(domain, result.ip)
        }

        console.log(
          `DNS Manager: resolved ${domain} -> ${result.ip} (${result.latency}ms, ${result.protocol})`,
        )
        return result.ip
      } else {
        throw new Error(result.error || 'DNS query failed')
      }
    } catch (err) {
      console.error(`DNS Manager: failed to resolve ${domain}:`, err)
      throw err
    }
  }

  /**
   * 获取最优 DNS 服务器
   */
  getBestDnsServers(count: number = 3): string[] {
    if (!this.config.enableHealthCheck) {
      return []
    }
    return dnsHealthCheckService.getBestServers(count)
  }

  /**
   * 获取统计信息
   */
  getStats(): DnsManagerStats {
    const routingStats = dnsSmartRoutingService.getStats()
    const torStatus = torProxyService.getStatus()
    const leakProtectionStats = dnsLeakProtectionService.getStats()

    return {
      cache: dnsCacheService.getStats(),
      health: dnsHealthCheckService.getStats(),
      prefetch: {
        commonDomains: dnsPrefetchService.getCommonDomains().length,
        accessHistory: dnsPrefetchService.getAccessStats().length,
      },
      routing: {
        mode: routingStats.mode,
        domesticDns: routingStats.domesticDns,
        foreignDns: routingStats.foreignDns,
        customRulesCount: routingStats.customRulesCount,
      },
      tor: {
        enabled: torStatus.enabled,
        connected: torStatus.connected,
        socksProxy: torProxyService.getSocksProxyUrl(),
      },
      leakProtection: {
        level: leakProtectionStats.level,
        levelName: leakProtectionStats.levelName,
        security: leakProtectionStats.security,
        safe: leakProtectionStats.safe,
      },
    }
  }

  /**
   * 清空缓存
   */
  clearCache(): void {
    dnsCacheService.clear()
    console.log('DNS Manager: cache cleared')
  }

  /**
   * 清空访问历史
   */
  clearHistory(): void {
    dnsPrefetchService.clearHistory()
    console.log('DNS Manager: access history cleared')
  }

  /**
   * 重置健康检查
   */
  resetHealthCheck(): void {
    dnsHealthCheckService.resetAllServers()
    console.log('DNS Manager: health check reset')
  }

  /**
   * 更新配置
   */
  updateConfig(config: Partial<DnsManagerConfig>): void {
    const oldConfig = { ...this.config }
    this.config = { ...this.config, ...config }

    // 处理智能分流配置变化
    if (oldConfig.enableSmartRouting !== this.config.enableSmartRouting) {
      if (this.config.enableSmartRouting) {
        dnsSmartRoutingService.setMode(this.config.routingMode)
        console.log('DNS Manager: smart routing enabled')
      } else {
        console.log('DNS Manager: smart routing disabled')
      }
    }

    if (oldConfig.routingMode !== this.config.routingMode) {
      dnsSmartRoutingService.setMode(this.config.routingMode)
      console.log(`DNS Manager: routing mode changed to ${this.config.routingMode}`)
    }

    // 处理 Tor 配置变化
    if (oldConfig.enableTor !== this.config.enableTor) {
      if (this.config.enableTor) {
        torProxyService.enable()
        console.log('DNS Manager: Tor proxy enabled')
      } else {
        torProxyService.disable()
        console.log('DNS Manager: Tor proxy disabled')
      }
    }

    // 处理健康检查配置变化
    if (oldConfig.enableHealthCheck !== this.config.enableHealthCheck) {
      if (this.config.enableHealthCheck) {
        this.initializeDnsServers()
        dnsHealthCheckService.startMonitoring(this.config.healthCheckInterval)
      } else {
        dnsHealthCheckService.stopMonitoring()
      }
    }

    // 处理预解析配置变化
    if (oldConfig.enablePrefetch !== this.config.enablePrefetch) {
      if (this.config.enablePrefetch) {
        dnsPrefetchService.startAutoPrefetch(this.config.prefetchInterval)
      } else {
        dnsPrefetchService.stopAutoPrefetch()
      }
    }

    console.log('DNS Manager: config updated', this.config)
  }

  /**
   * 获取配置
   */
  getConfig(): DnsManagerConfig {
    return { ...this.config }
  }

  /**
   * 设置 DNS 分流模式
   */
  setRoutingMode(mode: DnsRoutingMode): void {
    this.config.routingMode = mode
    dnsSmartRoutingService.setMode(mode)
    console.log(`DNS Manager: routing mode set to ${mode}`)
  }

  /**
   * 启用 Tor
   */
  enableTor(): void {
    this.config.enableTor = true
    torProxyService.enable()
    console.log('DNS Manager: Tor enabled')
  }

  /**
   * 禁用 Tor
   */
  disableTor(): void {
    this.config.enableTor = false
    torProxyService.disable()
    console.log('DNS Manager: Tor disabled')
  }

  /**
   * 获取 Tor 状态
   */
  getTorStatus() {
    return torProxyService.getStatus()
  }

  /**
   * 获取 Tor 配置
   */
  getTorConfig() {
    return torProxyService.getConfig()
  }

  /**
   * 获取智能分流服务
   */
  getSmartRoutingService() {
    return dnsSmartRoutingService
  }

  /**
   * 获取 Tor 服务
   */
  getTorService() {
    return torProxyService
  }

  /**
   * 设置零泄漏防护级别
   */
  setLeakProtectionLevel(level: DnsLeakProtectionLevel): void {
    this.config.leakProtectionLevel = level
    dnsLeakProtectionService.setLevel(level)
    console.log(`DNS Manager: leak protection level set to ${level}`)
  }

  /**
   * 启用零泄漏防护
   */
  enableLeakProtection(): void {
    this.config.enableLeakProtection = true
    dnsLeakProtectionService.setLevel(this.config.leakProtectionLevel)
    console.log('DNS Manager: leak protection enabled')
  }

  /**
   * 禁用零泄漏防护
   */
  disableLeakProtection(): void {
    this.config.enableLeakProtection = false
    dnsLeakProtectionService.setLevel('none')
    console.log('DNS Manager: leak protection disabled')
  }

  /**
   * 获取零泄漏防护服务
   */
  getLeakProtectionService() {
    return dnsLeakProtectionService
  }

  /**
   * 生成零泄漏 Clash DNS 配置
   */
  generateLeakProofDnsConfig(): Record<string, any> {
    return dnsLeakProtectionService.generateClashDnsConfig()
  }

  /**
   * 停止所有服务
   */
  shutdown(): void {
    dnsHealthCheckService.stopMonitoring()
    dnsPrefetchService.stopAutoPrefetch()
    dnsCacheService.stopCleanup()
    torProxyService.disable()

    this.initialized = false
    console.log('DNS Manager: shutdown')
  }
}

// 导出单例
export const dnsManager = new DnsManager()

// 导出类型
export type { DnsManagerConfig, DnsManagerStats }
