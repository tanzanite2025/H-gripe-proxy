/**
 * DNS 统计卡片组件
 * 显示 DNS 缓存、健康检查、智能分流、Tor 等统计信息
 */

import {
  RefreshCw as CachedRounded,
  CheckCircle as CheckCircleRounded,
  AlertCircle as ErrorRounded,
  RefreshCw as RefreshRounded,
  AlertTriangle as WarningRounded,
  Router as RouterRounded,
  Shield as VpnLockRounded,
  Zap as ZapRounded,
  Activity as ActivityRounded,
} from 'lucide-react'
import { useEffect, useState } from 'react'

import { Button } from '@/components/tailwind/Button'
import { Card } from '@/components/tailwind/Card'
import { Chip } from '@/components/tailwind/Chip'
import { LinearProgress } from '@/components/tailwind/LinearProgress'
import type { DnsRuntimeStatus } from '@/services/cmds'
import { getDnsMetrics, dnsWarmup, type DnsMetrics } from '@/services/dns-api'

import { buildDnsRuntimeViewModel } from './dns-runtime-view-model'

interface Props {
  runtimeStatus?: DnsRuntimeStatus
  runtimeStatusPending: boolean
  onRefresh: () => void
}

export const DnsStatsCard = ({
  runtimeStatus,
  runtimeStatusPending,
  onRefresh,
}: Props) => {
  if (runtimeStatusPending && !runtimeStatus) {
    return (
      <Card>
        <div className="p-4 min-h-[200px] flex flex-col">
          <div className="mb-2 flex items-center gap-1 text-sm font-semibold">
            <CachedRounded className="h-4 w-4" />
            DNS 当前运行态统计
          </div>
          <div className="flex-1 flex items-center justify-center">
            <div className="text-xs text-muted-foreground">正在加载 DNS 运行态数据...</div>
          </div>
          <LinearProgress />
        </div>
      </Card>
    )
  }

  if (!runtimeStatus) {
    return (
      <Card>
        <div className="p-4 min-h-[200px] flex flex-col">
          <div className="mb-2 flex items-center gap-1 text-sm font-semibold">
            <CachedRounded className="h-4 w-4" />
            DNS 当前运行态统计
          </div>
          <div className="flex-1 flex items-center justify-center">
            <div className="text-sm text-muted-foreground">
              暂时无法读取后端 DNS 运行态统计。
            </div>
          </div>
        </div>
      </Card>
    )
  }

  const runtimeView = buildDnsRuntimeViewModel(runtimeStatus)

  return (
    <Card>
      <div className="p-4">
        <div className="mb-2 flex items-center justify-between">
          <div className="flex items-center gap-1 text-sm font-semibold">
            <CachedRounded className="h-4 w-4" />
            DNS 当前运行态统计
          </div>
          <Button
            size="small"
            startIcon={<RefreshRounded className="h-4 w-4" />}
            onClick={onRefresh}
            disabled={runtimeStatusPending}
          >
            刷新
          </Button>
        </div>

        <div className="mb-2 text-xs text-gray-500 dark:text-gray-400">
          此处展示的是 Rust 后端确认的当前 DNS 运行态统计，不会随未保存的表单编辑即时变化。
        </div>

        {/* DNS 缓存统计 */}
        <div className="mb-2">
          <div className="mb-1 block text-xs text-gray-500 dark:text-gray-400">
            DNS 运行态摘要
          </div>
          <div className="space-y-1">
            <div className="flex justify-between">
              <div className="text-sm">Nameserver 数量</div>
              <div className="text-sm font-bold">
                {runtimeView.nameserverCount}
              </div>
            </div>
            <div className="flex justify-between">
              <div className="text-sm">Fallback 数量</div>
              <div className="text-sm font-bold text-green-600 dark:text-green-400">
                {runtimeView.fallbackCount}
              </div>
            </div>
            <div className="flex justify-between">
              <div className="text-sm">Nameserver Policy 数量</div>
              <div className="text-sm font-bold text-yellow-600 dark:text-yellow-400">
                {runtimeView.routing.policyCount}
              </div>
            </div>
            <div className="flex items-center justify-between">
              <div className="text-sm">Default Nameserver 数量</div>
              <div className="text-sm font-bold">
                {runtimeView.defaultNameserverCount}
              </div>
            </div>
            <div className="flex justify-between">
              <div className="text-sm">当前运行态</div>
              <div className="text-sm font-bold">
                {runtimeView.runtimeDnsInjectedLabel}
              </div>
            </div>
          </div>
        </div>

        <div className="my-2 border-t border-gray-200 dark:border-gray-700" />

        {/* DNS 健康检查统计 */}
        <div className="mb-2">
          <div className="mb-1 block text-xs text-gray-500 dark:text-gray-400">
            DNS 运行态选项
          </div>
          <div className="space-y-1">
            <div className="flex justify-between">
              <div className="text-sm">增强模式</div>
              <div className="text-sm font-bold">
                {runtimeView.enhancedModeLabel}
              </div>
            </div>
            <div className="flex items-center justify-between">
              <div className="text-sm">IPv6</div>
              <Chip
                icon={<CheckCircleRounded className="h-3 w-3" />}
                label={runtimeView.options.ipv6.label}
                size="small"
                color={runtimeView.options.ipv6.color}
              />
            </div>
            <div className="flex items-center justify-between">
              <div className="text-sm">Prefer H3</div>
              <Chip
                icon={<WarningRounded className="h-3 w-3" />}
                label={runtimeView.options.preferH3.label}
                size="small"
                color={runtimeView.options.preferH3.color}
              />
            </div>
            <div className="flex items-center justify-between">
              <div className="text-sm">Use Hosts</div>
              <Chip
                icon={<ErrorRounded className="h-3 w-3" />}
                label={runtimeView.options.useHosts.label}
                size="small"
                color={runtimeView.options.useHosts.color}
              />
            </div>
            <div className="flex justify-between">
              <div className="text-sm">Use System Hosts</div>
              <div className="text-sm font-bold text-primary-600 dark:text-primary-400">
                {runtimeView.options.useSystemHosts.label}
              </div>
            </div>
            <div className="flex justify-between">
              <div className="text-sm">Respect Rules</div>
              <div
                className="max-w-[200px] overflow-hidden text-ellipsis whitespace-nowrap text-sm font-bold"
                title={runtimeView.options.respectRules.label}
              >
                {runtimeView.options.respectRules.label}
              </div>
            </div>
          </div>
        </div>

        <div className="my-2 border-t border-gray-200 dark:border-gray-700" />

        {/* DNS 预解析统计 */}
        <div className="mb-2">
          <div className="mb-1 block text-xs text-gray-500 dark:text-gray-400">
            运行态对齐状态
          </div>
          <div className="space-y-1">
            <div className="flex items-center justify-between">
              <div className="text-sm">dns_config.yaml</div>
              <Chip
                label={runtimeView.dnsConfig.label}
                size="small"
                color={runtimeView.dnsConfig.color}
              />
            </div>
            <div className="flex items-center justify-between">
              <div className="text-sm">DNS 段对齐</div>
              <Chip
                label={runtimeView.runtimeDnsAlignment.label}
                size="small"
                color={runtimeView.runtimeDnsAlignment.color}
              />
            </div>
            <div className="flex items-center justify-between">
              <div className="text-sm">Hosts 段对齐</div>
              <Chip
                label={runtimeView.runtimeHostsAlignment.label}
                size="small"
                color={runtimeView.runtimeHostsAlignment.color}
              />
            </div>
            <div className="flex items-center justify-between">
              <div className="text-sm">整体运行态</div>
              <Chip
                label={runtimeView.runtimeAlignment.label}
                size="small"
                color={runtimeView.runtimeAlignment.color}
              />
            </div>
          </div>
        </div>

        <div className="my-2 border-t border-gray-200 dark:border-gray-700" />

        {/* DNS 智能分流统计 */}
        <div className="mb-2">
          <div className="mb-1 flex items-center gap-0.5 text-xs text-gray-500 dark:text-gray-400">
            <RouterRounded className="h-3.5 w-3.5" />
            DNS 智能分流
          </div>
          <div className="space-y-1">
            <div className="flex items-center justify-between">
              <div className="text-sm">分流模式</div>
              <Chip
                label={runtimeView.routing.modeLabel}
                size="small"
                color={runtimeView.routing.modeColor}
              />
            </div>
            <div className="flex justify-between">
              <div className="text-sm">国内 DNS</div>
              <div
                className="max-w-[180px] overflow-hidden text-ellipsis whitespace-nowrap text-xs font-bold"
                title={runtimeView.routing.domesticDns}
              >
                {runtimeView.routing.domesticDns}
              </div>
            </div>
            <div className="flex justify-between">
              <div className="text-sm">国外 DNS</div>
              <div
                className="max-w-[180px] overflow-hidden text-ellipsis whitespace-nowrap text-xs font-bold"
                title={runtimeView.routing.foreignDns}
              >
                {runtimeView.routing.foreignDns}
              </div>
            </div>
            <div className="flex justify-between">
              <div className="text-sm">策略组数量</div>
              <div className="text-sm font-bold">
                {runtimeView.routing.policyCount}
              </div>
            </div>
          </div>
        </div>

        <div className="my-2 border-t border-gray-200 dark:border-gray-700" />

        {/* Tor 代理统计 */}
        <div className="mb-2">
          <div className="mb-1 flex items-center gap-0.5 text-xs text-gray-500 dark:text-gray-400">
            <VpnLockRounded className="h-3.5 w-3.5" />
            运行态覆盖
          </div>
          <div className="space-y-1">
            <div className="flex items-center justify-between">
              <div className="text-sm">覆盖开关</div>
              <Chip
                label={runtimeView.runtimeOverride.label}
                size="small"
                color={runtimeView.runtimeOverride.color}
              />
            </div>
            <div className="flex justify-between">
              <div className="text-sm">当前来源</div>
              <div
                className="max-w-[180px] overflow-hidden text-ellipsis whitespace-nowrap text-xs font-bold"
                title={runtimeView.runtimeSource}
              >
                {runtimeView.runtimeSource}
              </div>
            </div>
            <div className="flex items-center justify-between">
              <div className="text-sm">当前生效情况</div>
              <Chip
                label={runtimeView.runtimeEffect.label}
                size="small"
                color={runtimeView.runtimeEffect.color}
              />
            </div>
            <div className="flex items-center justify-between">
              <div className="text-sm">已保存产物</div>
              <Chip
                label={runtimeView.savedArtifact.label}
                size="small"
                color={runtimeView.savedArtifact.color}
              />
            </div>
          </div>
        </div>

        <div className="my-2 border-t border-gray-200 dark:border-gray-700" />

        {/* DNS 零泄漏防护统计 */}
        <div>
          <div className="mb-1 flex items-center gap-0.5 text-xs text-gray-500 dark:text-gray-400">
            <RouterRounded className="h-3.5 w-3.5" />
            零泄漏防护
          </div>
          <div className="space-y-1">
            <div className="flex items-center justify-between">
              <div className="text-sm">防护级别</div>
              <Chip
                label={runtimeView.leak.levelLabel}
                size="small"
                color={runtimeView.leak.securityColor}
              />
            </div>
            <div className="flex items-center justify-between">
              <div className="text-sm">安全等级</div>
              <Chip
                label={runtimeView.leak.securityLabel}
                size="small"
                color={runtimeView.leak.securityColor}
              />
            </div>
            <div className="flex items-center justify-between">
              <div className="text-sm">安全状态</div>
              {runtimeView.leak.safe === null ? (
                <Chip label="未知" size="small" color="default" />
              ) : runtimeView.leak.safe ? (
                <Chip icon={<CheckCircleRounded className="h-3 w-3" />} label="安全" size="small" color="success" />
              ) : (
                <Chip icon={<WarningRounded className="h-3 w-3" />} label="不安全" size="small" color="error" />
              )}
            </div>
          </div>
        </div>

        <div className="my-2 border-t border-gray-200 dark:border-gray-700" />

        {/* DNS 性能指标 */}
        <DnsPerformanceMetrics />
      </div>
    </Card>
  )
}

/** DNS 性能实时指标子组件 */
function DnsPerformanceMetrics() {
  const [metrics, setMetrics] = useState<DnsMetrics | null>(null)
  const [loading, setLoading] = useState(false)

  const refresh = async () => {
    try {
      setLoading(true)
      const m = await getDnsMetrics()
      setMetrics(m)
    } catch {
      // metrics unavailable when DNS disabled
    } finally {
      setLoading(false)
    }
  }

  const handleWarmup = async () => {
    try {
      await dnsWarmup()
      // refresh after warmup to see cache fill
      setTimeout(refresh, 2000)
    } catch {
      // ignore
    }
  }

  useEffect(() => {
    refresh()
    const interval = setInterval(refresh, 10000)
    return () => clearInterval(interval)
  }, [])

  const cacheHitRate = metrics?.cache?.hitRate ?? 0
  const avgLatencyMs = metrics?.queries ? Math.round(metrics.queries.avgLatencyUs / 1000) : 0
  const maxLatencyMs = metrics?.queries ? Math.round(metrics.queries.maxLatencyUs / 1000) : 0

  return (
    <div>
      <div className="mb-1 flex items-center justify-between">
        <div className="flex items-center gap-0.5 text-xs text-gray-500 dark:text-gray-400">
          <ActivityRounded className="h-3.5 w-3.5" />
          性能指标
        </div>
        <div className="flex items-center gap-1">
          <Button
            size="small"
            variant="outlined"
            onClick={handleWarmup}
            className="!px-2 !py-0.5 !text-xs"
          >
            <ZapRounded className="mr-1 h-3 w-3" />
            预热
          </Button>
          <Button
            size="small"
            variant="outlined"
            onClick={refresh}
            disabled={loading}
            className="!px-2 !py-0.5 !text-xs"
          >
            <RefreshRounded className={`h-3 w-3 ${loading ? 'animate-spin' : ''}`} />
          </Button>
        </div>
      </div>

      {!metrics ? (
        <div className="text-xs text-gray-400">暂无数据</div>
      ) : (
        <div className="space-y-1">
          {/* 缓存命中率 */}
          <div>
            <div className="flex items-center justify-between text-sm">
              <span>缓存命中率</span>
              <span className="font-mono text-xs">
                {metrics.cache.hit + metrics.cache.miss > 0
                  ? `${(cacheHitRate * 100).toFixed(1)}%`
                  : '-'}
              </span>
            </div>
            <LinearProgress
              value={cacheHitRate * 100}
              color={cacheHitRate > 0.8 ? 'success' : cacheHitRate > 0.5 ? 'warning' : 'error'}
            />
            <div className="mt-0.5 flex justify-between text-xs text-gray-400">
              <span>命中 {metrics.cache.hit}</span>
              <span>未命中 {metrics.cache.miss}</span>
              <span>缓存 {metrics.cache.size} 条</span>
            </div>
          </div>

          {/* 查询统计 */}
          <div className="flex items-center justify-between text-sm">
            <span>查询总数</span>
            <span className="font-mono text-xs">{metrics.queries.total}</span>
          </div>
          <div className="flex items-center justify-between text-sm">
            <span>成功 / 失败</span>
            <span className="font-mono text-xs">
              <span className="text-green-600">{metrics.queries.success}</span>
              {' / '}
              <span className="text-red-500">{metrics.queries.failed}</span>
            </span>
          </div>
          <div className="flex items-center justify-between text-sm">
            <span>平均延迟</span>
            <span className="font-mono text-xs">{avgLatencyMs}ms</span>
          </div>
          <div className="flex items-center justify-between text-sm">
            <span>最大延迟</span>
            <span className="font-mono text-xs">{maxLatencyMs}ms</span>
          </div>

          {/* 服务器状态 */}
          {metrics.servers && metrics.servers.length > 0 && (
            <div className="mt-1">
              <div className="text-xs text-gray-500 dark:text-gray-400">服务器状态</div>
              <div className="mt-0.5 space-y-0.5">
                {metrics.servers.slice(0, 5).map((s) => (
                  <div key={s.server} className="flex items-center justify-between text-xs">
                    <span className="truncate max-w-[140px]" title={s.server}>
                      {s.server}
                    </span>
                    <span className="font-mono">
                      {Math.round(s.avgLatencyUs / 1000)}ms
                      {' '}
                      <span className="text-green-600">{s.successes}</span>
                      {'/'}
                      <span className="text-red-500">{s.failures}</span>
                    </span>
                  </div>
                ))}
                {metrics.servers.length > 5 && (
                  <div className="text-xs text-gray-400">
                    +{metrics.servers.length - 5} 更多
                  </div>
                )}
              </div>
            </div>
          )}

          {/* 污染检测 */}
          {metrics.pollution && metrics.pollution.totalChecked > 0 && (
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
                {metrics.pollution.pollutionRate > 0 &&
                  ` · 污染率 ${(metrics.pollution.pollutionRate * 100).toFixed(1)}%`}
              </div>
              {metrics.pollution.recentPolluted.length > 0 && (
                <div className="mt-0.5 space-y-0.5">
                  {metrics.pollution.recentPolluted.slice(0, 3).map((p, i) => (
                    <div key={i} className="flex items-center justify-between text-xs text-red-400">
                      <span className="truncate max-w-[120px]" title={p.domain}>
                        {p.domain}
                      </span>
                      <span className="font-mono">{p.ip}</span>
                    </div>
                  ))}
                </div>
              )}
            </div>
          )}

          {/* DNS 服务器信任评估 */}
          {metrics.trust && metrics.trust.total > 0 && (
            <div className="mt-1">
              <div className="flex items-center justify-between text-sm">
                <span>服务器安全</span>
                <Chip
                  label={
                    metrics.trust.leakRiskScore < 0.3
                      ? '低风险'
                      : metrics.trust.leakRiskScore < 0.6
                        ? '中风险'
                        : '高风险'
                  }
                  size="small"
                  color={
                    metrics.trust.leakRiskScore < 0.3
                      ? 'success'
                      : metrics.trust.leakRiskScore < 0.6
                        ? 'warning'
                        : 'error'
                  }
                />
              </div>
              <div className="text-xs text-gray-400">
                {metrics.trust.encrypted}/{metrics.trust.total} 加密
                {metrics.trust.unencrypted > 0 && (
                  <span className="text-red-400">
                    {' '}· {metrics.trust.unencrypted} 明文
                  </span>
                )}
              </div>
              {metrics.trust.servers.length > 0 && (
                <div className="mt-0.5 space-y-0.5">
                  {metrics.trust.servers.slice(0, 4).map((s) => (
                    <div key={s.address} className="flex items-center justify-between text-xs">
                      <span className="truncate max-w-[100px]" title={s.address}>
                        {s.address}
                      </span>
                      <span className="flex items-center gap-1 font-mono">
                        <span className={
                          s.trustLevel === 'high' || s.trustLevel === 'maximum'
                            ? 'text-green-600'
                            : s.trustLevel === 'medium'
                              ? 'text-yellow-500'
                              : 'text-red-500'
                        }>
                          {s.protocol}
                        </span>
                        {!s.encrypted && (
                          <span className="text-red-400" title="未加密">⚠</span>
                        )}
                      </span>
                    </div>
                  ))}
                </div>
              )}
            </div>
          )}
        </div>
      )}
    </div>
  )
}
