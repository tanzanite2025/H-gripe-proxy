/**
 * DNS 健康检查服务
 * 实时监控 DNS 服务器健康状态，自动切换到最优 DNS
 */

import { dnsHealthCheck, type DnsProtocol } from './dns-api'

interface DnsServer {
  address: string
  type: 'udp' | 'doh' | 'dot'
  latency: number
  successRate: number
  lastCheck: number
  status: 'healthy' | 'degraded' | 'down'
  consecutiveFailures: number
}

interface DnsHealthStats {
  totalServers: number
  healthyServers: number
  degradedServers: number
  downServers: number
  averageLatency: number
  bestServer: string | null
}

class DnsHealthCheckService {
  private servers = new Map<string, DnsServer>()
  private checkInterval: ReturnType<typeof setInterval> | null = null
  private readonly TEST_DOMAIN = 'www.google.com'
  private readonly MAX_CONSECUTIVE_FAILURES = 3

  /**
   * 添加 DNS 服务器
   */
  addServer(address: string, type: 'udp' | 'doh' | 'dot' = 'udp'): void {
    if (this.servers.has(address)) {
      console.log(`DNS health check: server ${address} already exists`)
      return
    }

    this.servers.set(address, {
      address,
      type,
      latency: 0,
      successRate: 100,
      lastCheck: 0,
      status: 'healthy',
      consecutiveFailures: 0,
    })

    console.log(`DNS health check: added server ${address} (${type})`)
  }

  /**
   * 移除 DNS 服务器
   */
  removeServer(address: string): boolean {
    const removed = this.servers.delete(address)
    if (removed) {
      console.log(`DNS health check: removed server ${address}`)
    }
    return removed
  }

  /**
   * 检查单个 DNS 服务器
   */
  async checkServer(address: string): Promise<void> {
    const server = this.servers.get(address)
    if (!server) return

    try {
      // 将服务器类型转换为协议类型
      const protocol: DnsProtocol = server.type === 'udp' ? 'udp' : server.type

      // 调用后端 API 进行 DNS 健康检查
      const result = await dnsHealthCheck(address, this.TEST_DOMAIN, protocol)

      if (result.success) {
        // 更新服务器状态
        server.latency = result.latency
        server.successRate = Math.min(100, server.successRate + 2)
        server.lastCheck = Date.now()
        server.consecutiveFailures = 0

        // 判断健康状态
        if (result.latency < 100 && server.successRate > 95) {
          server.status = 'healthy'
        } else if (result.latency < 500 && server.successRate > 80) {
          server.status = 'degraded'
        } else {
          server.status = 'down'
        }

        console.log(
          `DNS health check: ${address} - ${server.status} (${result.latency}ms, ${server.successRate.toFixed(1)}%, ${result.protocol})`,
        )
      } else {
        // 查询失败
        server.successRate = Math.max(0, server.successRate - 10)
        server.lastCheck = Date.now()
        server.consecutiveFailures++

        // 判断健康状态
        if (server.consecutiveFailures >= this.MAX_CONSECUTIVE_FAILURES || server.successRate < 50) {
          server.status = 'down'
        } else {
          server.status = 'degraded'
        }

        console.error(
          `DNS health check: ${address} - failed (${server.consecutiveFailures} consecutive failures) - ${result.error || 'unknown error'}`,
        )
      }
    } catch (err) {
      // 查询失败
      server.successRate = Math.max(0, server.successRate - 10)
      server.lastCheck = Date.now()
      server.consecutiveFailures++

      // 判断健康状态
      if (server.consecutiveFailures >= this.MAX_CONSECUTIVE_FAILURES || server.successRate < 50) {
        server.status = 'down'
      } else {
        server.status = 'degraded'
      }

      console.error(
        `DNS health check: ${address} - failed (${server.consecutiveFailures} consecutive failures)`,
        err,
      )
    }
  }

  /**
   * 检查所有 DNS 服务器
   */
  async checkAllServers(): Promise<void> {
    if (this.servers.size === 0) {
      console.log('DNS health check: no servers to check')
      return
    }

    console.log(`DNS health check: checking ${this.servers.size} servers`)

    const promises = Array.from(this.servers.keys()).map((address) =>
      this.checkServer(address),
    )

    await Promise.allSettled(promises)

    const stats = this.getStats()
    console.log(
      `DNS health check: completed (${stats.healthyServers} healthy, ${stats.degradedServers} degraded, ${stats.downServers} down)`,
    )
  }

  /**
   * 获取最优 DNS 服务器
   */
  getBestServers(count: number = 3): string[] {
    return Array.from(this.servers.values())
      .filter((s) => s.status !== 'down')
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
      .map((s) => s.address)
  }

  /**
   * 获取服务器状态
   */
  getServerStatus(address: string): DnsServer | null {
    return this.servers.get(address) || null
  }

  /**
   * 获取所有服务器状态
   */
  getAllServers(): DnsServer[] {
    return Array.from(this.servers.values()).sort((a, b) => {
      const statusOrder = { healthy: 0, degraded: 1, down: 2 }
      return statusOrder[a.status] - statusOrder[b.status]
    })
  }

  /**
   * 获取健康统计信息
   */
  getStats(): DnsHealthStats {
    const servers = Array.from(this.servers.values())
    const healthyServers = servers.filter((s) => s.status === 'healthy')
    const degradedServers = servers.filter((s) => s.status === 'degraded')
    const downServers = servers.filter((s) => s.status === 'down')

    const averageLatency =
      healthyServers.length > 0
        ? healthyServers.reduce((sum, s) => sum + s.latency, 0) / healthyServers.length
        : 0

    const bestServer = healthyServers.length > 0 ? healthyServers[0].address : null

    return {
      totalServers: servers.length,
      healthyServers: healthyServers.length,
      degradedServers: degradedServers.length,
      downServers: downServers.length,
      averageLatency: Math.round(averageLatency),
      bestServer,
    }
  }

  /**
   * 启动定期检查
   */
  startMonitoring(intervalMs: number = 60000): void {
    if (this.checkInterval) {
      console.log('DNS health check: monitoring already started')
      return
    }

    // 立即执行一次检查
    void this.checkAllServers()

    // 定期检查（默认 1 分钟）
    this.checkInterval = setInterval(() => {
      void this.checkAllServers()
    }, intervalMs)

    console.log(`DNS health check: monitoring started (interval: ${intervalMs}ms)`)
  }

  /**
   * 停止定期检查
   */
  stopMonitoring(): void {
    if (this.checkInterval) {
      clearInterval(this.checkInterval)
      this.checkInterval = null
      console.log('DNS health check: monitoring stopped')
    }
  }

  /**
   * 重置所有服务器状态
   */
  resetAllServers(): void {
    for (const server of this.servers.values()) {
      server.latency = 0
      server.successRate = 100
      server.lastCheck = 0
      server.status = 'healthy'
      server.consecutiveFailures = 0
    }
    console.log('DNS health check: all servers reset')
  }

  /**
   * 清空所有服务器
   */
  clear(): void {
    this.servers.clear()
    console.log('DNS health check: all servers cleared')
  }
}

// 导出单例
export const dnsHealthCheckService = new DnsHealthCheckService()

// 导出类型
export type { DnsServer, DnsHealthStats }
