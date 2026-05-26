/**
 * DNS 统计卡片组件
 * 显示 DNS 缓存、健康检查、智能分流、Tor 等统计信息
 */

import {
  CachedRounded,
  CheckCircleRounded,
  ErrorRounded,
  RefreshRounded,
  WarningRounded,
  RouterRounded,
  VpnLockRounded,
} from '@mui/icons-material'
import {
  Box,
  Button,
  Card,
  CardContent,
  Chip,
  Divider,
  LinearProgress,
  Stack,
  Typography,
} from '@mui/material'
import { useEffect, useState } from 'react'
import { useTranslation } from 'react-i18next'

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
        <Box sx={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', mb: 2 }}>
          <Typography variant="h6" sx={{ display: 'flex', alignItems: 'center', gap: 1 }}>
            <CachedRounded />
            DNS 统计
          </Typography>
          <Button
            size="small"
            startIcon={<RefreshRounded />}
            onClick={() => void loadStats()}
            disabled={loading}
          >
            刷新
          </Button>
        </Box>

        {/* DNS 缓存统计 */}
        <Box sx={{ mb: 3 }}>
          <Typography variant="subtitle2" color="text.secondary" sx={{ mb: 1 }}>
            DNS 缓存
          </Typography>
          <Stack spacing={1}>
            <Box sx={{ display: 'flex', justifyContent: 'space-between' }}>
              <Typography variant="body2">总查询次数</Typography>
              <Typography variant="body2" sx={{ fontWeight: 'bold' }}>
                {cache.totalQueries}
              </Typography>
            </Box>
            <Box sx={{ display: 'flex', justifyContent: 'space-between' }}>
              <Typography variant="body2">缓存命中</Typography>
              <Typography variant="body2" sx={{ fontWeight: 'bold' }} color="success.main">
                {cache.cacheHits}
              </Typography>
            </Box>
            <Box sx={{ display: 'flex', justifyContent: 'space-between' }}>
              <Typography variant="body2">缓存未命中</Typography>
              <Typography variant="body2" sx={{ fontWeight: 'bold' }} color="warning.main">
                {cache.cacheMisses}
              </Typography>
            </Box>
            <Box sx={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
              <Typography variant="body2">命中率</Typography>
              <Box sx={{ display: 'flex', alignItems: 'center', gap: 1 }}>
                <LinearProgress
                  variant="determinate"
                  value={cache.hitRate}
                  sx={{ width: 100, height: 6, borderRadius: 3 }}
                  color={cache.hitRate > 70 ? 'success' : cache.hitRate > 40 ? 'warning' : 'error'}
                />
                <Typography variant="body2" sx={{ fontWeight: 'bold' }}>
                  {cache.hitRate.toFixed(1)}%
                </Typography>
              </Box>
            </Box>
            <Box sx={{ display: 'flex', justifyContent: 'space-between' }}>
              <Typography variant="body2">缓存大小</Typography>
              <Typography variant="body2" sx={{ fontWeight: 'bold' }}>
                {cache.cacheSize} / 1000
              </Typography>
            </Box>
            <Button
              size="small"
              variant="outlined"
              color="warning"
              onClick={handleClearCache}
              sx={{ mt: 1 }}
            >
              清空缓存
            </Button>
          </Stack>
        </Box>

        <Divider sx={{ my: 2 }} />

        {/* DNS 健康检查统计 */}
        <Box sx={{ mb: 3 }}>
          <Typography variant="subtitle2" color="text.secondary" sx={{ mb: 1 }}>
            DNS 健康检查
          </Typography>
          <Stack spacing={1}>
            <Box sx={{ display: 'flex', justifyContent: 'space-between' }}>
              <Typography variant="body2">总服务器数</Typography>
              <Typography variant="body2" sx={{ fontWeight: 'bold' }}>
                {health.totalServers}
              </Typography>
            </Box>
            <Box sx={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
              <Typography variant="body2">健康</Typography>
              <Chip
                icon={<CheckCircleRounded />}
                label={health.healthyServers}
                size="small"
                color="success"
              />
            </Box>
            <Box sx={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
              <Typography variant="body2">降级</Typography>
              <Chip
                icon={<WarningRounded />}
                label={health.degradedServers}
                size="small"
                color="warning"
              />
            </Box>
            <Box sx={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
              <Typography variant="body2">故障</Typography>
              <Chip
                icon={<ErrorRounded />}
                label={health.downServers}
                size="small"
                color="error"
              />
            </Box>
            <Box sx={{ display: 'flex', justifyContent: 'space-between' }}>
              <Typography variant="body2">平均延迟</Typography>
              <Typography variant="body2" sx={{ fontWeight: 'bold' }} color="primary.main">
                {health.averageLatency}ms
              </Typography>
            </Box>
            <Box sx={{ display: 'flex', justifyContent: 'space-between' }}>
              <Typography variant="body2">最优服务器</Typography>
              <Typography variant="body2" sx={{ fontWeight: 'bold', maxWidth: 200, overflow: 'hidden', textOverflow: 'ellipsis' }}>
                {health.bestServer || 'N/A'}
              </Typography>
            </Box>
            <Button
              size="small"
              variant="outlined"
              onClick={handleResetHealth}
              sx={{ mt: 1 }}
            >
              重置健康检查
            </Button>
          </Stack>
        </Box>

        <Divider sx={{ my: 2 }} />

        {/* DNS 预解析统计 */}
        <Box sx={{ mb: 3 }}>
          <Typography variant="subtitle2" color="text.secondary" sx={{ mb: 1 }}>
            DNS 预解析
          </Typography>
          <Stack spacing={1}>
            <Box sx={{ display: 'flex', justifyContent: 'space-between' }}>
              <Typography variant="body2">常用域名数</Typography>
              <Typography variant="body2" sx={{ fontWeight: 'bold' }}>
                {prefetch.commonDomains}
              </Typography>
            </Box>
            <Box sx={{ display: 'flex', justifyContent: 'space-between' }}>
              <Typography variant="body2">访问历史数</Typography>
              <Typography variant="body2" sx={{ fontWeight: 'bold' }}>
                {prefetch.accessHistory}
              </Typography>
            </Box>
          </Stack>
        </Box>

        <Divider sx={{ my: 2 }} />

        {/* DNS 智能分流统计 */}
        <Box sx={{ mb: 3 }}>
          <Typography variant="subtitle2" color="text.secondary" sx={{ mb: 1, display: 'flex', alignItems: 'center', gap: 1 }}>
            <RouterRounded fontSize="small" />
            DNS 智能分流
          </Typography>
          <Stack spacing={1}>
            <Box sx={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
              <Typography variant="body2">分流模式</Typography>
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
            </Box>
            <Box sx={{ display: 'flex', justifyContent: 'space-between' }}>
              <Typography variant="body2">国内 DNS</Typography>
              <Typography variant="body2" sx={{ fontWeight: 'bold', fontSize: '0.75rem', maxWidth: 180, overflow: 'hidden', textOverflow: 'ellipsis' }}>
                {routing.domesticDns}
              </Typography>
            </Box>
            <Box sx={{ display: 'flex', justifyContent: 'space-between' }}>
              <Typography variant="body2">国外 DNS</Typography>
              <Typography variant="body2" sx={{ fontWeight: 'bold', fontSize: '0.75rem', maxWidth: 180, overflow: 'hidden', textOverflow: 'ellipsis' }}>
                {routing.foreignDns}
              </Typography>
            </Box>
            {routing.customRulesCount > 0 && (
              <Box sx={{ display: 'flex', justifyContent: 'space-between' }}>
                <Typography variant="body2">自定义规则</Typography>
                <Typography variant="body2" sx={{ fontWeight: 'bold' }}>
                  {routing.customRulesCount} 条
                </Typography>
              </Box>
            )}
          </Stack>
        </Box>

        <Divider sx={{ my: 2 }} />

        {/* Tor 代理统计 */}
        <Box sx={{ mb: 3 }}>
          <Typography variant="subtitle2" color="text.secondary" sx={{ mb: 1, display: 'flex', alignItems: 'center', gap: 1 }}>
            <VpnLockRounded fontSize="small" />
            Tor 代理
          </Typography>
          <Stack spacing={1}>
            <Box sx={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
              <Typography variant="body2">状态</Typography>
              {tor.enabled ? (
                <Chip
                  icon={tor.connected ? <CheckCircleRounded /> : <WarningRounded />}
                  label={tor.connected ? '已连接' : '未连接'}
                  size="small"
                  color={tor.connected ? 'success' : 'warning'}
                />
              ) : (
                <Chip label="未启用" size="small" />
              )}
            </Box>
            {tor.enabled && (
              <Box sx={{ display: 'flex', justifyContent: 'space-between' }}>
                <Typography variant="body2">SOCKS5 代理</Typography>
                <Typography variant="body2" sx={{ fontWeight: 'bold', fontSize: '0.75rem' }}>
                  {tor.socksProxy}
                </Typography>
              </Box>
            )}
          </Stack>
        </Box>

        <Divider sx={{ my: 2 }} />

        {/* DNS 零泄漏防护统计 */}
        <Box>
          <Typography variant="subtitle2" color="text.secondary" sx={{ mb: 1, display: 'flex', alignItems: 'center', gap: 1 }}>
            <RouterRounded fontSize="small" />
            零泄漏防护
          </Typography>
          <Stack spacing={1}>
            <Box sx={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
              <Typography variant="body2">防护级别</Typography>
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
            </Box>
            <Box sx={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
              <Typography variant="body2">安全状态</Typography>
              {leakProtection.safe ? (
                <Chip icon={<CheckCircleRounded />} label="安全" size="small" color="success" />
              ) : (
                <Chip icon={<WarningRounded />} label="不安全" size="small" color="error" />
              )}
            </Box>
          </Stack>
        </Box>
      </CardContent>
    </Card>
  )
}
