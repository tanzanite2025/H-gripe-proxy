/**
 * DNS 管理器
 * 整合 DNS 缓存、预解析、健康检查等功能
 */

import { dnsCacheService, type DnsCacheStats } from './dns-cache'
import { dnsHealthCheckService, type DnsHealthStats } from './dns-health-check'
import { dnsPrefetchService } from './dns-prefetch'

interface DnsManagerConfig {
  enableCache: boolean
  enablePrefetch: boolean
  enableHealthCheck: boolean
  prefetchInterval: number // 预解析间隔（毫秒）
  healthCheckInterval: number // 健康检查间隔（毫秒）
}

interface DnsManagerStats {
  cache: DnsCacheStats
  health: DnsHealthStats
  prefetch: {
    commonDomains: number
    accessHistory: number
  }
}

class DnsManager {
  private config: DnsManagerConfig = {
    enableCache: true,
    enablePrefetch: true,
    enableHealthCheck: true,
    prefetchInterval: 300000, // 5 分钟
    healthCheckInterval: 60000, // 1 分钟
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

    // 初始化 DNS 服务器列表
    if (this.config.enableHealthCheck) {
      this.initializeDnsServers()
      dnsHealthCheckService.startMonitoring(this.config.healthCheckInterval)
    }

    // 启动预解析
    if (this.config.enablePrefetch) {
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
   * 解析域名（带缓存）
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

    // 实际解析（需要调用后端 API）
    // const ip = await invoke('dns_query', { domain })

    // 模拟解析（实际应该删除）
    const ip = `192.168.1.${Math.floor(Math.random() * 255)}`

    // 缓存结果
    if (this.config.enableCache) {
      dnsCacheService.set(domain, ip)
    }

    console.log(`DNS Manager: resolved ${domain} -> ${ip}`)
    return ip
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
    return {
      cache: dnsCacheService.getStats(),
      health: dnsHealthCheckService.getStats(),
      prefetch: {
        commonDomains: dnsPrefetchService.getCommonDomains().length,
        accessHistory: dnsPrefetchService.getAccessStats().length,
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

    // 处理配置变化
    if (oldConfig.enableHealthCheck !== this.config.enableHealthCheck) {
      if (this.config.enableHealthCheck) {
        this.initializeDnsServers()
        dnsHealthCheckService.startMonitoring(this.config.healthCheckInterval)
      } else {
        dnsHealthCheckService.stopMonitoring()
      }
    }

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
   * 停止所有服务
   */
  shutdown(): void {
    dnsHealthCheckService.stopMonitoring()
    dnsPrefetchService.stopAutoPrefetch()
    dnsCacheService.stopCleanup()

    this.initialized = false
    console.log('DNS Manager: shutdown')
  }
}

// 导出单例
export const dnsManager = new DnsManager()

// 导出类型
export type { DnsManagerConfig, DnsManagerStats }
