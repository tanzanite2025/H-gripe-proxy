/**
 * DNS 缓存服务
 * 减少 DNS 查询次数，提高响应速度
 */

interface DnsCacheEntry {
  ip: string
  timestamp: number
  ttl: number // 缓存时间（秒）
  hitCount: number // 命中次数
}

interface DnsCacheStats {
  totalQueries: number
  cacheHits: number
  cacheMisses: number
  hitRate: number
  cacheSize: number
}

class DnsCacheService {
  private cache = new Map<string, DnsCacheEntry>()
  private readonly DEFAULT_TTL = 300 // 5分钟
  private readonly MAX_CACHE_SIZE = 1000 // 最大缓存条目数
  private cleanupInterval: ReturnType<typeof setInterval> | null = null

  // 统计数据
  private stats = {
    totalQueries: 0,
    cacheHits: 0,
    cacheMisses: 0,
  }

  constructor() {
    this.startCleanup()
  }

  /**
   * 获取缓存的 IP
   */
  get(domain: string): string | null {
    this.stats.totalQueries++

    const entry = this.cache.get(domain)
    if (!entry) {
      this.stats.cacheMisses++
      return null
    }

    const now = Date.now()
    const age = (now - entry.timestamp) / 1000

    // 检查是否过期
    if (age > entry.ttl) {
      this.cache.delete(domain)
      this.stats.cacheMisses++
      return null
    }

    // 命中缓存
    entry.hitCount++
    this.stats.cacheHits++
    return entry.ip
  }

  /**
   * 设置缓存
   */
  set(domain: string, ip: string, ttl: number = this.DEFAULT_TTL): void {
    // 检查缓存大小限制
    if (this.cache.size >= this.MAX_CACHE_SIZE && !this.cache.has(domain)) {
      this.evictLeastUsed()
    }

    this.cache.set(domain, {
      ip,
      timestamp: Date.now(),
      ttl,
      hitCount: 0,
    })
  }

  /**
   * 删除缓存
   */
  delete(domain: string): boolean {
    return this.cache.delete(domain)
  }

  /**
   * 检查缓存是否存在且有效
   */
  has(domain: string): boolean {
    const entry = this.cache.get(domain)
    if (!entry) return false

    const now = Date.now()
    const age = (now - entry.timestamp) / 1000

    if (age > entry.ttl) {
      this.cache.delete(domain)
      return false
    }

    return true
  }

  /**
   * 清除过期缓存
   */
  cleanup(): void {
    const now = Date.now()
    const expiredDomains: string[] = []

    for (const [domain, entry] of this.cache.entries()) {
      const age = (now - entry.timestamp) / 1000
      if (age > entry.ttl) {
        expiredDomains.push(domain)
      }
    }

    for (const domain of expiredDomains) {
      this.cache.delete(domain)
    }

    if (expiredDomains.length > 0) {
      console.log(`DNS cache cleanup: removed ${expiredDomains.length} expired entries`)
    }
  }

  /**
   * 淘汰最少使用的缓存条目
   */
  private evictLeastUsed(): void {
    let leastUsedDomain: string | null = null
    let leastHitCount = Number.POSITIVE_INFINITY

    for (const [domain, entry] of this.cache.entries()) {
      if (entry.hitCount < leastHitCount) {
        leastHitCount = entry.hitCount
        leastUsedDomain = domain
      }
    }

    if (leastUsedDomain) {
      this.cache.delete(leastUsedDomain)
      console.log(`DNS cache eviction: removed ${leastUsedDomain} (hit count: ${leastHitCount})`)
    }
  }

  /**
   * 清空所有缓存
   */
  clear(): void {
    this.cache.clear()
    this.stats = {
      totalQueries: 0,
      cacheHits: 0,
      cacheMisses: 0,
    }
    console.log('DNS cache cleared')
  }

  /**
   * 获取缓存统计信息
   */
  getStats(): DnsCacheStats {
    const hitRate =
      this.stats.totalQueries > 0
        ? (this.stats.cacheHits / this.stats.totalQueries) * 100
        : 0

    return {
      totalQueries: this.stats.totalQueries,
      cacheHits: this.stats.cacheHits,
      cacheMisses: this.stats.cacheMisses,
      hitRate: Math.round(hitRate * 100) / 100,
      cacheSize: this.cache.size,
    }
  }

  /**
   * 获取所有缓存条目
   */
  getAll(): Array<{ domain: string; ip: string; age: number; ttl: number; hitCount: number }> {
    const now = Date.now()
    const entries: Array<{ domain: string; ip: string; age: number; ttl: number; hitCount: number }> = []

    for (const [domain, entry] of this.cache.entries()) {
      const age = Math.round((now - entry.timestamp) / 1000)
      entries.push({
        domain,
        ip: entry.ip,
        age,
        ttl: entry.ttl,
        hitCount: entry.hitCount,
      })
    }

    return entries.sort((a, b) => b.hitCount - a.hitCount)
  }

  /**
   * 启动定期清理
   */
  private startCleanup(): void {
    if (this.cleanupInterval) return

    // 每分钟清理一次过期缓存
    this.cleanupInterval = setInterval(() => {
      this.cleanup()
    }, 60000)
  }

  /**
   * 停止定期清理
   */
  stopCleanup(): void {
    if (this.cleanupInterval) {
      clearInterval(this.cleanupInterval)
      this.cleanupInterval = null
    }
  }

  /**
   * 重置统计数据
   */
  resetStats(): void {
    this.stats = {
      totalQueries: 0,
      cacheHits: 0,
      cacheMisses: 0,
    }
  }
}

// 导出单例
export const dnsCacheService = new DnsCacheService()

// 导出类型
export type { DnsCacheStats }
