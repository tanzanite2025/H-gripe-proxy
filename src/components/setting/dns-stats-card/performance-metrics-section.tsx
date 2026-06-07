import { Activity, RefreshCw, Zap } from 'lucide-react'
import { useEffect, useRef, useState } from 'react'

import { Button } from '@/components/tailwind/Button'
import { Chip } from '@/components/tailwind/Chip'
import { LinearProgress } from '@/components/tailwind/LinearProgress'
import { dnsWarmup, getDnsMetrics, type DnsMetrics } from '@/services/dns-api'

import { DnsSectionHeading, DnsTextRow } from './shared'

const getLeakRiskLabel = (score: number) => {
  if (score < 0.3) return '低风险'
  if (score < 0.6) return '中风险'
  return '高风险'
}

const getLeakRiskColor = (score: number) => {
  if (score < 0.3) return 'success' as const
  if (score < 0.6) return 'warning' as const
  return 'error' as const
}

export function PerformanceMetricsSection() {
  const [metrics, setMetrics] = useState<DnsMetrics | null>(null)
  const [loading, setLoading] = useState(false)
  const warmupTimerRef = useRef<number | null>(null)

  const refresh = async () => {
    try {
      setLoading(true)
      const nextMetrics = await getDnsMetrics()
      setMetrics(nextMetrics)
    } catch {
      // Metrics may be unavailable when DNS is disabled.
    } finally {
      setLoading(false)
    }
  }

  const handleWarmup = async () => {
    try {
      await dnsWarmup()
      if (warmupTimerRef.current !== null) {
        window.clearTimeout(warmupTimerRef.current)
      }
      warmupTimerRef.current = window.setTimeout(() => {
        void refresh()
        warmupTimerRef.current = null
      }, 2000)
    } catch {
      // Ignore warmup failures.
    }
  }

  useEffect(() => {
    void refresh()
    const interval = window.setInterval(() => {
      void refresh()
    }, 10000)

    return () => {
      window.clearInterval(interval)
      if (warmupTimerRef.current !== null) {
        window.clearTimeout(warmupTimerRef.current)
      }
    }
  }, [])

  const cacheHitRate = metrics?.cache?.hitRate ?? 0
  const avgLatencyMs = metrics?.queries
    ? Math.round(metrics.queries.avgLatencyUs / 1000)
    : 0
  const maxLatencyMs = metrics?.queries
    ? Math.round(metrics.queries.maxLatencyUs / 1000)
    : 0

  return (
    <div>
      <div className="mb-1 flex items-center justify-between">
        <DnsSectionHeading
          title="性能指标"
          icon={<Activity className="h-3.5 w-3.5" />}
        />
        <div className="flex items-center gap-1">
          <Button
            size="small"
            variant="outlined"
            onClick={handleWarmup}
            className="!px-2 !py-0.5 !text-xs"
          >
            <Zap className="mr-1 h-3 w-3" />
            预热
          </Button>
          <Button
            size="small"
            variant="outlined"
            onClick={() => {
              void refresh()
            }}
            disabled={loading}
            className="!px-2 !py-0.5 !text-xs"
          >
            <RefreshCw className={`h-3 w-3 ${loading ? 'animate-spin' : ''}`} />
          </Button>
        </div>
      </div>

      {!metrics ? (
        <div className="text-xs text-gray-400">暂无数据</div>
      ) : (
        <div className="space-y-1">
          <div>
            <DnsTextRow
              label="缓存命中率"
              value={
                metrics.cache.hit + metrics.cache.miss > 0
                  ? `${(cacheHitRate * 100).toFixed(1)}%`
                  : '-'
              }
              valueClassName="font-mono text-xs"
            />
            <LinearProgress
              value={cacheHitRate * 100}
              color={
                cacheHitRate > 0.8
                  ? 'success'
                  : cacheHitRate > 0.5
                    ? 'warning'
                    : 'error'
              }
            />
            <div className="mt-0.5 flex justify-between text-xs text-gray-400">
              <span>命中 {metrics.cache.hit}</span>
              <span>未命中 {metrics.cache.miss}</span>
              <span>缓存 {metrics.cache.size} 条</span>
            </div>
          </div>

          <DnsTextRow
            label="查询总数"
            value={metrics.queries.total}
            valueClassName="font-mono text-xs"
          />
          <div className="flex items-center justify-between text-sm">
            <span>成功 / 失败</span>
            <span className="font-mono text-xs">
              <span className="text-green-600">{metrics.queries.success}</span>
              {' / '}
              <span className="text-red-500">{metrics.queries.failed}</span>
            </span>
          </div>
          <DnsTextRow
            label="平均延迟"
            value={`${avgLatencyMs}ms`}
            valueClassName="font-mono text-xs"
          />
          <DnsTextRow
            label="最大延迟"
            value={`${maxLatencyMs}ms`}
            valueClassName="font-mono text-xs"
          />

          {metrics.servers && metrics.servers.length > 0 ? (
            <div className="mt-1">
              <div className="text-xs text-gray-500 dark:text-gray-400">
                服务器状态
              </div>
              <div className="mt-0.5 space-y-0.5">
                {metrics.servers.slice(0, 5).map((server) => (
                  <div
                    key={server.server}
                    className="flex items-center justify-between text-xs"
                  >
                    <span
                      className="max-w-[140px] truncate"
                      title={server.server}
                    >
                      {server.server}
                    </span>
                    <span className="font-mono">
                      {Math.round(server.avgLatencyUs / 1000)}ms{' '}
                      <span className="text-green-600">{server.successes}</span>/
                      <span className="text-red-500">{server.failures}</span>
                    </span>
                  </div>
                ))}
                {metrics.servers.length > 5 ? (
                  <div className="text-xs text-gray-400">
                    +{metrics.servers.length - 5} 更多
                  </div>
                ) : null}
              </div>
            </div>
          ) : null}

          {metrics.pollution && metrics.pollution.totalChecked > 0 ? (
            <div className="mt-1">
              <div className="flex items-center justify-between text-sm">
                <span>污染检测</span>
                <span className="font-mono text-xs">
                  {metrics.pollution.pollutedCount > 0 ? (
                    <span className="text-red-500">
                      {metrics.pollution.pollutedCount} 次污染
                    </span>
                  ) : (
                    <span className="text-green-600">未检测到</span>
                  )}
                </span>
              </div>
              <div className="text-xs text-gray-400">
                已检查 {metrics.pollution.totalChecked} 次响应
                {metrics.pollution.pollutionRate > 0
                  ? ` / 污染率 ${(metrics.pollution.pollutionRate * 100).toFixed(1)}%`
                  : ''}
              </div>
              {metrics.pollution.recentPolluted.length > 0 ? (
                <div className="mt-0.5 space-y-0.5">
                  {metrics.pollution.recentPolluted.slice(0, 3).map((item, index) => (
                    <div
                      key={index}
                      className="flex items-center justify-between text-xs text-red-400"
                    >
                      <span
                        className="max-w-[120px] truncate"
                        title={item.domain}
                      >
                        {item.domain}
                      </span>
                      <span className="font-mono">{item.ip}</span>
                    </div>
                  ))}
                </div>
              ) : null}
            </div>
          ) : null}

          {metrics.trust && metrics.trust.total > 0 ? (
            <div className="mt-1">
              <div className="flex items-center justify-between text-sm">
                <span>解析链路安全</span>
                <Chip
                  label={getLeakRiskLabel(metrics.trust.leakRiskScore)}
                  size="small"
                  color={getLeakRiskColor(metrics.trust.leakRiskScore)}
                />
              </div>
              <div className="text-xs text-gray-400">
                {metrics.trust.encrypted}/{metrics.trust.total} 已加密
                {metrics.trust.unencrypted > 0 ? (
                  <span className="text-red-400">
                    {' '}
                    / {metrics.trust.unencrypted} 明文
                  </span>
                ) : null}
              </div>
              {metrics.trust.servers.length > 0 ? (
                <div className="mt-0.5 space-y-0.5">
                  {metrics.trust.servers.slice(0, 4).map((server) => (
                    <div
                      key={server.address}
                      className="flex items-center justify-between text-xs"
                    >
                      <span
                        className="max-w-[100px] truncate"
                        title={server.address}
                      >
                        {server.address}
                      </span>
                      <span className="flex items-center gap-1 font-mono">
                        <span
                          className={
                            server.trustLevel === 'high' ||
                            server.trustLevel === 'maximum'
                              ? 'text-green-600'
                              : server.trustLevel === 'medium'
                                ? 'text-yellow-500'
                                : 'text-red-500'
                          }
                        >
                          {server.protocol}
                        </span>
                        {!server.encrypted ? (
                          <span className="text-red-400" title="未加密">
                            !
                          </span>
                        ) : null}
                      </span>
                    </div>
                  ))}
                </div>
              ) : null}
            </div>
          ) : null}
        </div>
      )}
    </div>
  )
}
