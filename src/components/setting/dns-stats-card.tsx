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
} from 'lucide-react'
import { useEffect, useState } from 'react'
import { useTranslation } from 'react-i18next'

import { Button } from '@/components/tailwind/Button'
import { Card, CardContent } from '@/components/tailwind/Card'
import { Chip } from '@/components/tailwind/Chip'
import { LinearProgress } from '@/components/tailwind/LinearProgress'
import { dnsManager, type DnsManagerStats } from '@/services/dns-manager'

export const DnsStatsCard = () => {
  const { t } = useTranslation()
  const [stats, setStats] = useState<DnsManagerStats | null>(null)
  const [loading, setLoading] = useState(false)

  // 加载统计信息
  const loadStats = async () => {
    try {
      setLoading(true)
      const data = dnsManager.getStats()
      setStats(data)
    } catch (err) {
      console.error('Failed to load DNS stats', err)
    } finally {
      setLoading(false)
    }
  }

  // 初始化和定期刷新
  useEffect(() => {
    void loadStats()

    // 每 5 秒刷新一次
    const interval = setInterval(() => {
      void loadStats()
    }, 5000)

    return () => clearInterval(interval)
  }, [])

  // 清空缓存
  const handleClearCache = () => {
    dnsManager.clearCache()
    void loadStats()
  }

  // 重置健康检查
  const handleResetHealth = () => {
    dnsManager.resetHealthCheck()
    void loadStats()
  }

  if (!stats) {
    return (
      <Card>
        <CardContent>
          <LinearProgress />
        </CardContent>
      </Card>
    )
  }

  const { cache, health, prefetch, routing, tor, leakProtection } = stats

  return (
    <Card>
      <CardContent>
        <div className="mb-2 flex items-center justify-between">
          <div className="flex items-center gap-1 text-sm font-semibold">
            <CachedRounded className="h-4 w-4" />
            DNS 统计
          </div>
          <Button
            size="small"
            startIcon={<RefreshRounded className="h-4 w-4" />}
            onClick={() => void loadStats()}
            disabled={loading}
          >
            刷新
          </Button>
        </div>

        {/* DNS 缓存统计 */}
        <div className="mb-2">
          <div className="mb-1 block text-xs text-gray-500 dark:text-gray-400">
            DNS 缓存
          </div>
          <div className="space-y-1">
            <div className="flex justify-between">
              <div className="text-sm">总查询次数</div>
              <div className="text-sm font-bold">
                {cache.totalQueries}
              </div>
            </div>
            <div className="flex justify-between">
              <div className="text-sm">缓存命中</div>
              <div className="text-sm font-bold text-green-600 dark:text-green-400">
                {cache.cacheHits}
              </div>
            </div>
            <div className="flex justify-between">
              <div className="text-sm">缓存未命中</div>
              <div className="text-sm font-bold text-yellow-600 dark:text-yellow-400">
                {cache.cacheMisses}
              </div>
            </div>
            <div className="flex items-center justify-between">
              <div className="text-sm">命中率</div>
              <div className="flex items-center gap-1">
                <LinearProgress
                  variant="determinate"
                  value={cache.hitRate}
                  className="h-1.5 w-[100px] rounded-full"
                  color={cache.hitRate > 70 ? 'success' : cache.hitRate > 40 ? 'warning' : 'error'}
                />
                <div className="text-sm font-bold">
                  {cache.hitRate.toFixed(1)}%
                </div>
              </div>
            </div>
            <div className="flex justify-between">
              <div className="text-sm">缓存大小</div>
              <div className="text-sm font-bold">
                {cache.cacheSize} / 1000
              </div>
            </div>
            <Button
              size="small"
              variant="outlined"
              color="warning"
              onClick={handleClearCache}
              className="mt-1"
            >
              清空缓存
            </Button>
          </div>
        </div>

        <div className="my-2 border-t border-gray-200 dark:border-gray-700" />

        {/* DNS 健康检查统计 */}
        <div className="mb-2">
          <div className="mb-1 block text-xs text-gray-500 dark:text-gray-400">
            DNS 健康检查
          </div>
          <div className="space-y-1">
            <div className="flex justify-between">
              <div className="text-sm">总服务器数</div>
              <div className="text-sm font-bold">
                {health.totalServers}
              </div>
            </div>
            <div className="flex items-center justify-between">
              <div className="text-sm">健康</div>
              <Chip
                icon={<CheckCircleRounded className="h-3 w-3" />}
                label={health.healthyServers}
                size="small"
                color="success"
              />
            </div>
            <div className="flex items-center justify-between">
              <div className="text-sm">降级</div>
              <Chip
                icon={<WarningRounded className="h-3 w-3" />}
                label={health.degradedServers}
                size="small"
                color="warning"
              />
            </div>
            <div className="flex items-center justify-between">
              <div className="text-sm">故障</div>
              <Chip
                icon={<ErrorRounded className="h-3 w-3" />}
                label={health.downServers}
                size="small"
                color="error"
              />
            </div>
            <div className="flex justify-between">
              <div className="text-sm">平均延迟</div>
              <div className="text-sm font-bold text-primary-600 dark:text-primary-400">
                {health.averageLatency}ms
              </div>
            </div>
            <div className="flex justify-between">
              <div className="text-sm">最优服务器</div>
              <div 
                className="max-w-[200px] overflow-hidden text-ellipsis whitespace-nowrap text-sm font-bold"
                title={health.bestServer || 'N/A'}
              >
                {health.bestServer || 'N/A'}
              </div>
            </div>
            <Button
              size="small"
              variant="outlined"
              onClick={handleResetHealth}
              className="mt-1"
            >
              重置健康检查
            </Button>
          </div>
        </div>

        <div className="my-2 border-t border-gray-200 dark:border-gray-700" />

        {/* DNS 预解析统计 */}
        <div className="mb-2">
          <div className="mb-1 block text-xs text-gray-500 dark:text-gray-400">
            DNS 预解析
          </div>
          <div className="space-y-1">
            <div className="flex justify-between">
              <div className="text-sm">常用域名数</div>
              <div className="text-sm font-bold">
                {prefetch.commonDomains}
              </div>
            </div>
            <div className="flex justify-between">
              <div className="text-sm">访问历史数</div>
              <div className="text-sm font-bold">
                {prefetch.accessHistory}
              </div>
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
                label={
                  routing.mode === 'speed'
                    ? '速度优先'
                    : routing.mode === 'privacy'
                      ? '隐私优先'
                      : routing.mode === 'balanced'
                        ? '平衡模式'
                        : '自定义'
                }
                size="small"
                color={
                  routing.mode === 'speed'
                    ? 'success'
                    : routing.mode === 'privacy'
                      ? 'info'
                      : routing.mode === 'balanced'
                        ? 'warning'
                        : 'default'
                }
              />
            </div>
            <div className="flex justify-between">
              <div className="text-sm">国内 DNS</div>
              <div 
                className="max-w-[180px] overflow-hidden text-ellipsis whitespace-nowrap text-xs font-bold"
                title={routing.domesticDns}
              >
                {routing.domesticDns}
              </div>
            </div>
            <div className="flex justify-between">
              <div className="text-sm">国外 DNS</div>
              <div 
                className="max-w-[180px] overflow-hidden text-ellipsis whitespace-nowrap text-xs font-bold"
                title={routing.foreignDns}
              >
                {routing.foreignDns}
              </div>
            </div>
            {routing.customRulesCount > 0 && (
              <div className="flex justify-between">
                <div className="text-sm">自定义规则</div>
                <div className="text-sm font-bold">
                  {routing.customRulesCount} 条
                </div>
              </div>
            )}
          </div>
        </div>

        <div className="my-2 border-t border-gray-200 dark:border-gray-700" />

        {/* Tor 代理统计 */}
        <div className="mb-2">
          <div className="mb-1 flex items-center gap-0.5 text-xs text-gray-500 dark:text-gray-400">
            <VpnLockRounded className="h-3.5 w-3.5" />
            Tor 代理
          </div>
          <div className="space-y-1">
            <div className="flex items-center justify-between">
              <div className="text-sm">状态</div>
              {tor.enabled ? (
                <Chip
                  icon={tor.connected ? <CheckCircleRounded className="h-3 w-3" /> : <WarningRounded className="h-3 w-3" />}
                  label={tor.connected ? '已连接' : '未连接'}
                  size="small"
                  color={tor.connected ? 'success' : 'warning'}
                />
              ) : (
                <Chip label="未启用" size="small" />
              )}
            </div>
            {tor.enabled && (
              <div className="flex justify-between">
                <div className="text-sm">SOCKS5 代理</div>
                <div 
                  className="max-w-[150px] overflow-hidden text-ellipsis whitespace-nowrap text-xs font-bold"
                  title={tor.socksProxy}
                >
                  {tor.socksProxy}
                </div>
              </div>
            )}
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
                label={leakProtection.levelName}
                size="small"
                color={
                  leakProtection.security === 'low'
                    ? 'error'
                    : leakProtection.security === 'medium'
                      ? 'warning'
                      : leakProtection.security === 'high'
                        ? 'info'
                        : 'success'
                }
              />
            </div>
            <div className="flex items-center justify-between">
              <div className="text-sm">安全状态</div>
              {leakProtection.safe ? (
                <Chip icon={<CheckCircleRounded className="h-3 w-3" />} label="安全" size="small" color="success" />
              ) : (
                <Chip icon={<WarningRounded className="h-3 w-3" />} label="不安全" size="small" color="error" />
              )}
            </div>
          </div>
        </div>
      </CardContent>
    </Card>
  )
}
