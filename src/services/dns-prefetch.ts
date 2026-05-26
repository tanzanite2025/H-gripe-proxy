/**
 * DNS 预解析服务
 * 提前解析常用域名，减少首次访问延迟
 */

import { dnsCacheService } from './dns-cache'
import { dnsQuery } from './dns-api'

interface DomainFrequency {
  domain: string
  count: number
  lastAccess: number
}

class DnsPrefetchService {
  // 常用域名列表（初始默认）
  private commonDomains = [
    'www.google.com',
    'www.youtube.com',
    'www.github.com',
    'api.github.com',
    'raw.githubusercontent.com',
    'www.cloudflare.com',
    'api.openai.com',
    'www.wikipedia.org',
    'www.reddit.com',
    'www.twitter.com',
  ]

  // 域名访问历史（用于学习）
  private accessHistory = new Map<string, DomainFrequency>()
  private readonly MAX_HISTORY_SIZE = 500
  private prefetchInterval: ReturnType<typeof setInterval> | null = null

  /**
   * 预解析域名
   */
  async prefetchDomain(domain: string): Promise<void> {
    try {
      // 检查缓存
      if (dnsCacheService.has(domain)) {
        return
      }

      // 调用后端 API 进行 DNS 查询
      const result = await dnsQuery(domain)

      if (result.success && result.ip) {
        // 缓存查询结果
        dnsCacheService.set(domain, result.ip)
        console.log(`DNS prefetch: ${domain} -> ${result.ip} (${result.latency}ms)`)
      } else {
        console.warn(`DNS prefetch failed: ${domain} - ${result.error || 'unknown error'}`)
      }
    } catch (err) {
      console.error(`DNS prefetch failed: ${domain}`, err)
    }
  }

  /**
   * 预解析所有常用域名
   */
  async prefetchAll(): Promise<void> {
    console.log(`DNS prefetch: starting for ${this.commonDomains.length} domains`)

    const promises = this.commonDomains.map((domain) =>
      this.prefetchDomain(domain),
    )

    const results = await Promise.allSettled(promises)

    const succeeded = results.filter((r) => r.status === 'fulfilled').length
    const failed = results.filter((r) => r.status === 'rejected').length

    console.log(`DNS prefetch: completed (${succeeded} succeeded, ${failed} failed)`)
  }

  /**
   * 添加常用域名
   */
  addCommonDomain(domain: string): void {
    if (!this.commonDomains.includes(domain)) {
      this.commonDomains.push(domain)
      console.log(`DNS prefetch: added common domain ${domain}`)
    }
  }

  /**
   * 移除常用域名
   */
  removeCommonDomain(domain: string): void {
    const index = this.commonDomains.indexOf(domain)
    if (index !== -1) {
      this.commonDomains.splice(index, 1)
      console.log(`DNS prefetch: removed common domain ${domain}`)
    }
  }

  /**
   * 记录域名访问
   */
  recordAccess(domain: string): void {
    const existing = this.accessHistory.get(domain)

    if (existing) {
      existing.count++
      existing.lastAccess = Date.now()
    } else {
      // 检查历史记录大小限制
      if (this.accessHistory.size >= this.MAX_HISTORY_SIZE) {
        this.evictOldestAccess()
      }

      this.accessHistory.set(domain, {
        domain,
        count: 1,
        lastAccess: Date.now(),
      })
    }
  }

  /**
   * 淘汰最旧的访问记录
   */
  private evictOldestAccess(): void {
    let oldestDomain: string | null = null
    let oldestTime = Number.POSITIVE_INFINITY

    for (const [domain, freq] of this.accessHistory.entries()) {
      if (freq.lastAccess < oldestTime) {
        oldestTime = freq.lastAccess
        oldestDomain = domain
      }
    }

    if (oldestDomain) {
      this.accessHistory.delete(oldestDomain)
    }
  }

  /**
   * 从访问历史中学习常用域名
   */
  learnFromHistory(): void {
    if (this.accessHistory.size === 0) {
      console.log('DNS prefetch: no access history to learn from')
      return
    }

    // 获取访问频率最高的域名
    const topDomains = Array.from(this.accessHistory.values())
      .sort((a, b) => {
        // 综合考虑访问次数和最近访问时间
        const scoreA = a.count * 0.7 + (Date.now() - a.lastAccess) / 86400000 * 0.3
        const scoreB = b.count * 0.7 + (Date.now() - b.lastAccess) / 86400000 * 0.3
        return scoreB - scoreA
      })
      .slice(0, 50)
      .map((freq) => freq.domain)

    // 合并默认域名和学习到的域名
    const defaultDomains = [
      'www.google.com',
      'www.github.com',
      'www.cloudflare.com',
    ]

    this.commonDomains = [
      ...defaultDomains,
      ...topDomains.filter((d) => !defaultDomains.includes(d)),
    ]

    console.log(`DNS prefetch: learned ${topDomains.length} common domains from history`)
  }

  /**
   * 获取常用域名列表
   */
  getCommonDomains(): string[] {
    return [...this.commonDomains]
  }

  /**
   * 获取访问历史统计
   */
  getAccessStats(): Array<{ domain: string; count: number; lastAccess: Date }> {
    return Array.from(this.accessHistory.values())
      .map((freq) => ({
        domain: freq.domain,
        count: freq.count,
        lastAccess: new Date(freq.lastAccess),
      }))
      .sort((a, b) => b.count - a.count)
  }

  /**
   * 启动定期预解析
   */
  startAutoPrefetch(intervalMs: number = 300000): void {
    if (this.prefetchInterval) return

    // 立即执行一次
    void this.prefetchAll()

    // 定期预解析（默认 5 分钟）
    this.prefetchInterval = setInterval(() => {
      // 先学习访问历史
      this.learnFromHistory()
      // 然后预解析
      void this.prefetchAll()
    }, intervalMs)

    console.log(`DNS prefetch: auto prefetch started (interval: ${intervalMs}ms)`)
  }

  /**
   * 停止定期预解析
   */
  stopAutoPrefetch(): void {
    if (this.prefetchInterval) {
      clearInterval(this.prefetchInterval)
      this.prefetchInterval = null
      console.log('DNS prefetch: auto prefetch stopped')
    }
  }

  /**
   * 清空访问历史
   */
  clearHistory(): void {
    this.accessHistory.clear()
    console.log('DNS prefetch: access history cleared')
  }
}

// 导出单例
export const dnsPrefetchService = new DnsPrefetchService()
