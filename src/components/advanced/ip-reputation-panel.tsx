import { useState } from 'react'
import { useQuery } from '@tanstack/react-query'

import { Button } from '@/components/tailwind/Button'
import { Card } from '@/components/tailwind/Card'
import { Switch } from '@/components/tailwind/Switch'
import { TextField } from '@/components/tailwind/TextField'
import {
  getIdentityConsistencyDriftReport,
  getIdentityConsistencyHistory,
  getIdentityConsistencyReport,
  type IdentityConsistencyDrift,
  type IdentityConsistencyLevel,
  type IdentityConsistencyReport,
  type IdentityConsistencySnapshot,
} from '@/services/cmds'
import {
  type IpReputation,
  type IpReputationConfig,
  ipReputationCheckIp,
  ipReputationClearCache,
  ipReputationGetCacheEntries,
  ipReputationGetCacheStats,
  getIpTypeText,
  getRiskLevelText,
  getRiskLevelColor,
} from '@/services/ip-reputation'

interface Props {
  config: IpReputationConfig
  onChange: (config: IpReputationConfig) => void
}

const consistencyLevelText: Record<IdentityConsistencyLevel, string> = {
  good: '良好',
  warning: '需关注',
  danger: '高风险',
  unknown: '未知',
}

const consistencyScoreColor: Record<IdentityConsistencyLevel, string> = {
  good: 'text-green-600',
  warning: 'text-yellow-600',
  danger: 'text-red-600',
  unknown: 'text-gray-500',
}

const consistencyBadgeColor: Record<IdentityConsistencyLevel, string> = {
  good: 'bg-green-100 text-green-700 dark:bg-green-900/30 dark:text-green-400',
  warning: 'bg-yellow-100 text-yellow-700 dark:bg-yellow-900/30 dark:text-yellow-400',
  danger: 'bg-red-100 text-red-700 dark:bg-red-900/30 dark:text-red-400',
  unknown: 'bg-gray-100 text-gray-700 dark:bg-gray-900/30 dark:text-gray-400',
}

const formatConsistencyValue = (value: string | number | null | undefined) =>
  value === null || value === undefined || value === '' ? '未观测' : String(value)

const formatProxyChain = (report: IdentityConsistencyReport) =>
  report.proxy_chain.length > 0 ? report.proxy_chain.join(' -> ') : '未观测'

const formatSnapshotTime = (snapshot: IdentityConsistencySnapshot) =>
  snapshot.observed_at ? new Date(snapshot.observed_at).toLocaleString() : '未知时间'

const snapshotSummary = (snapshot: IdentityConsistencySnapshot) => {
  const report = snapshot.report
  return [
    report.public_egress_ip || '未观测出口',
    report.ip_type || '未知类型',
    report.dns_assessment ? `DNS ${report.dns_assessment}` : null,
    report.tls_fingerprint ? `TLS ${report.tls_fingerprint}` : null,
  ].filter(Boolean).join(' / ')
}

const driftKindText: Record<IdentityConsistencyDrift['kind'], string> = {
  publicEgressIp: '公网出口',
  ipType: 'IP 类型',
  dnsAssessment: 'DNS',
  tlsFingerprint: 'TLS 指纹',
}

const driftValue = (value: string | null) => value || '未观测'

export function IpReputationPanel({ config, onChange }: Props) {
  const [checkIp, setCheckIp] = useState('')
  const [checking, setChecking] = useState(false)
  const [result, setResult] = useState<IpReputation | null>(null)
  const [cacheEntries, setCacheEntries] = useState<IpReputation[]>([])
  const [cacheStats, setCacheStats] = useState<[number, number] | null>(null)
  const [showCache, setShowCache] = useState(false)
  const {
    data: consistencyReport,
    error: consistencyError,
    isFetching: consistencyFetching,
    refetch: refetchConsistencyReport,
  } = useQuery({
    queryKey: ['identity-consistency-report'],
    queryFn: getIdentityConsistencyReport,
    staleTime: 30 * 1000,
    gcTime: 5 * 60 * 1000,
    retry: 1,
  })
  const {
    data: consistencyHistory = [],
    refetch: refetchConsistencyHistory,
  } = useQuery({
    queryKey: ['identity-consistency-history'],
    queryFn: getIdentityConsistencyHistory,
    staleTime: 30 * 1000,
    gcTime: 5 * 60 * 1000,
    retry: 1,
  })
  const {
    data: consistencyDriftReport,
    refetch: refetchConsistencyDriftReport,
  } = useQuery({
    queryKey: ['identity-consistency-drift-report'],
    queryFn: getIdentityConsistencyDriftReport,
    staleTime: 30 * 1000,
    gcTime: 5 * 60 * 1000,
    retry: 1,
  })

  const handleRefreshConsistency = async () => {
    await refetchConsistencyReport()
    await refetchConsistencyHistory()
    await refetchConsistencyDriftReport()
  }

  const handleCheck = async () => {
    if (!checkIp.trim()) return
    setChecking(true)
    try {
      const rep = await ipReputationCheckIp(checkIp.trim())
      setResult(rep)
    } catch (_e: any) {
      setResult(null)
    } finally {
      setChecking(false)
    }
  }

  const handleRefreshCache = async () => {
    const [stats, entries] = await Promise.all([
      ipReputationGetCacheStats(),
      ipReputationGetCacheEntries(),
    ])
    setCacheStats(stats)
    setCacheEntries(entries)
    setShowCache(true)
  }

  const handleClearCache = async () => {
    await ipReputationClearCache()
    setCacheStats(null)
    setCacheEntries([])
  }

  const handleToggleEnabled = (enabled: boolean) => {
    onChange({ ...config, enabled })
  }

  const handleUpdateTtl = (value: string) => {
    const ttl = parseInt(value, 10)
    if (!isNaN(ttl) && ttl > 0) {
      onChange({ ...config, cacheTtl: ttl })
    }
  }

  const fraudScoreColor = (score: number) => {
    if (score <= 30) return 'text-green-600'
    if (score <= 60) return 'text-yellow-600'
    if (score <= 85) return 'text-orange-600'
    return 'text-red-600'
  }

  const fraudScoreBg = (score: number) => {
    if (score <= 30) return 'bg-green-100 dark:bg-green-900/30'
    if (score <= 60) return 'bg-yellow-100 dark:bg-yellow-900/30'
    if (score <= 85) return 'bg-orange-100 dark:bg-orange-900/30'
    return 'bg-red-100 dark:bg-red-900/30'
  }

  const ipTypeBadge = (type: string) => {
    const colors: Record<string, string> = {
      Datacenter: 'bg-red-100 text-red-700 dark:bg-red-900/30 dark:text-red-400',
      Residential: 'bg-green-100 text-green-700 dark:bg-green-900/30 dark:text-green-400',
      Mobile: 'bg-blue-100 text-blue-700 dark:bg-blue-900/30 dark:text-blue-400',
      Education: 'bg-purple-100 text-purple-700 dark:bg-purple-900/30 dark:text-purple-400',
      Unknown: 'bg-gray-100 text-gray-700 dark:bg-gray-900/30 dark:text-gray-400',
    }
    return colors[type] || colors.Unknown
  }

  return (
    <div className="space-y-4">
      <Card>
        <div className="space-y-4">
          <div className="flex items-start justify-between gap-3">
            <div>
              <h3 className="text-sm font-semibold">当前出口一致性</h3>
              <p className="text-xs text-gray-500 mt-1">
                汇总出口 IP、节点链路、DNS、TLS 指纹和 IP 风险，用于判断当前节点身份是否一致。
              </p>
            </div>
            <Button
              onClick={() => void handleRefreshConsistency()}
              variant="outlined"
              size="sm"
              disabled={consistencyFetching}
            >
              {consistencyFetching ? '刷新中...' : '刷新'}
            </Button>
          </div>

          {consistencyError ? (
            <div className="rounded-lg border border-red-200 bg-red-50 p-3 text-xs text-red-600 dark:border-red-900/40 dark:bg-red-950/20">
              一致性报告获取失败
            </div>
          ) : consistencyReport ? (
            <>
              <div className="grid grid-cols-1 md:grid-cols-4 gap-4">
                <div>
                  <p className="text-xs text-gray-500">一致性评分</p>
                  <p className={`text-2xl font-bold ${consistencyScoreColor[consistencyReport.level]}`}>
                    {consistencyReport.score}
                  </p>
                  <span className={`inline-block mt-1 px-2 py-0.5 rounded text-xs font-medium ${consistencyBadgeColor[consistencyReport.level]}`}>
                    {consistencyLevelText[consistencyReport.level]}
                  </span>
                </div>
                <div>
                  <p className="text-xs text-gray-500">公网出口</p>
                  <p className="text-sm font-mono font-medium">
                    {formatConsistencyValue(consistencyReport.public_egress_ip)}
                  </p>
                  <p className="text-xs text-gray-400">
                    {formatConsistencyValue(consistencyReport.egress_source)}
                    {consistencyReport.egress_confidence !== null
                      ? ` / ${consistencyReport.egress_confidence}`
                      : ''}
                  </p>
                </div>
                <div>
                  <p className="text-xs text-gray-500">IP 类型</p>
                  <p className="text-sm font-medium">
                    {formatConsistencyValue(consistencyReport.ip_type)}
                  </p>
                  <p className="text-xs text-gray-400">
                    {formatConsistencyValue(consistencyReport.residential_state)}
                  </p>
                </div>
                <div>
                  <p className="text-xs text-gray-500">DNS / TLS</p>
                  <p className="text-sm">
                    {formatConsistencyValue(consistencyReport.dns_assessment)}
                  </p>
                  <p className="text-xs text-gray-400">
                    {formatConsistencyValue(consistencyReport.tls_fingerprint)}
                  </p>
                </div>
              </div>

              <div className="grid grid-cols-1 md:grid-cols-2 gap-4 pt-3 border-t border-gray-200 dark:border-gray-700">
                <div>
                  <p className="text-xs text-gray-500">节点链路</p>
                  <p className="text-sm font-medium break-all">
                    {formatProxyChain(consistencyReport)}
                  </p>
                </div>
                <div>
                  <p className="text-xs text-gray-500">主要问题</p>
                  {consistencyReport.issues.length > 0 ? (
                    <div className="mt-1 space-y-1">
                      {consistencyReport.issues.slice(0, 4).map((issue) => (
                        <div key={`${issue.kind}-${issue.message}`} className="flex items-start gap-2 text-xs">
                          <span className={`mt-0.5 h-2 w-2 shrink-0 rounded-full ${issue.severity === 'danger' ? 'bg-red-500' : issue.severity === 'warning' ? 'bg-yellow-500' : 'bg-gray-400'}`} />
                          <span className="text-gray-600 dark:text-gray-300">{issue.message}</span>
                        </div>
                      ))}
                    </div>
                  ) : (
                    <p className="text-sm text-green-600">暂无一致性问题</p>
                  )}
                </div>
              </div>

              {consistencyDriftReport && (
                <div className="pt-3 border-t border-gray-200 dark:border-gray-700">
                  <div className="flex items-center justify-between gap-3">
                    <p className="text-xs text-gray-500">身份漂移</p>
                    <span className={consistencyDriftReport.stable ? 'text-xs text-green-600' : 'text-xs text-yellow-600'}>
                      {consistencyDriftReport.stable
                        ? '最近快照稳定'
                        : `检测到 ${consistencyDriftReport.drift_count} 项变化`}
                    </span>
                  </div>
                  {!consistencyDriftReport.stable && (
                    <div className="mt-2 space-y-1">
                      {consistencyDriftReport.drifts.slice(0, 4).map((drift) => (
                        <div
                          key={`${drift.kind}-${drift.from || 'none'}-${drift.to || 'none'}`}
                          className="rounded bg-yellow-50 px-2 py-1.5 text-xs text-yellow-800 dark:bg-yellow-950/20 dark:text-yellow-300"
                        >
                          <span className="font-medium">{driftKindText[drift.kind]}</span>
                          <span className="mx-1">{driftValue(drift.from)}</span>
                          <span>{'->'}</span>
                          <span className="mx-1">{driftValue(drift.to)}</span>
                        </div>
                      ))}
                    </div>
                  )}
                </div>
              )}

              {consistencyHistory.length > 0 && (
                <div className="pt-3 border-t border-gray-200 dark:border-gray-700">
                  <p className="text-xs text-gray-500">最近快照</p>
                  <div className="mt-2 space-y-1">
                    {consistencyHistory.slice(0, 3).map((snapshot) => (
                      <div
                        key={`${snapshot.observed_at}-${snapshot.report.public_egress_ip || 'unknown'}`}
                        className="flex items-center justify-between gap-3 rounded bg-gray-50 px-2 py-1.5 text-xs dark:bg-gray-900/30"
                      >
                        <span className="font-mono text-gray-500">
                          {formatSnapshotTime(snapshot)}
                        </span>
                        <span className="truncate text-gray-600 dark:text-gray-300">
                          {snapshotSummary(snapshot)}
                        </span>
                        <span className={consistencyScoreColor[snapshot.report.level]}>
                          {snapshot.report.score}
                        </span>
                      </div>
                    ))}
                  </div>
                </div>
              )}
            </>
          ) : (
            <div className="rounded-lg border border-gray-200 bg-gray-50 p-3 text-xs text-gray-500 dark:border-gray-800 dark:bg-gray-900/30">
              一致性报告加载中...
            </div>
          )}
        </div>
      </Card>

      {/* 全局开关 + 配置 */}
      <Card>
        <div className="space-y-4">
          <div className="flex items-center justify-between">
            <div>
              <h3 className="text-sm font-semibold">IP 信誉数据库</h3>
              <p className="text-xs text-gray-500 mt-1">
                为当前节点/当前出口身份识别提供底层证据，手动 IP 查询仅用于调试
              </p>
            </div>
            <Switch checked={config.enabled} onCheckedChange={handleToggleEnabled} />
          </div>

          {config.enabled && (
            <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
              <TextField
                label="缓存 TTL（秒）"
                type="number"
                value={String(config.cacheTtl)}
                onChange={(e: React.ChangeEvent<HTMLInputElement>) => handleUpdateTtl(e.target.value)}
                helperText="IP 信誉结果的缓存时长"
              />
              <div className="flex items-end gap-2">
                <Button onClick={handleRefreshCache} variant="outlined" size="sm">
                  查看缓存
                </Button>
                <Button onClick={handleClearCache} variant="outlined" size="sm">
                  清除缓存
                </Button>
              </div>
            </div>
          )}
        </div>
      </Card>

      {/* 调试查询 */}
      {config.enabled && (
        <Card>
          <div className="space-y-4">
            <h3 className="text-sm font-semibold">调试查询</h3>
            <div className="flex gap-2">
              <TextField
                placeholder="输入 IP 地址，如 45.76.123.45"
                value={checkIp}
                onChange={(e: React.ChangeEvent<HTMLInputElement>) => setCheckIp(e.target.value)}
                onKeyDown={(e: React.KeyboardEvent) => e.key === 'Enter' && handleCheck()}
                fullWidth
              />
              <Button onClick={handleCheck} disabled={checking || !checkIp.trim()}>
                {checking ? '查询中...' : '查询'}
              </Button>
            </div>

            {/* 查询结果 */}
            {result && (
              <div className={`rounded-lg p-4 ${fraudScoreBg(result.fraudScore)}`}>
                <div className="grid grid-cols-2 md:grid-cols-4 gap-4">
                  <div>
                    <p className="text-xs text-gray-500">IP 地址</p>
                    <p className="text-sm font-mono font-medium">{result.ip}</p>
                  </div>
                  <div>
                    <p className="text-xs text-gray-500">IP 类型</p>
                    <span className={`inline-block mt-1 px-2 py-0.5 rounded text-xs font-medium ${ipTypeBadge(result.ipType)}`}>
                      {getIpTypeText(result.ipType)}
                    </span>
                  </div>
                  <div>
                    <p className="text-xs text-gray-500">ASN</p>
                    <p className="text-sm font-medium">{result.asn}</p>
                    <p className="text-xs text-gray-400">{result.asnOrg}</p>
                  </div>
                  <div>
                    <p className="text-xs text-gray-500">欺诈评分</p>
                    <p className={`text-2xl font-bold ${fraudScoreColor(result.fraudScore)}`}>
                      {result.fraudScore}
                    </p>
                    <p className={`text-xs ${getRiskLevelColor(result.riskLevel)}`}>
                      {getRiskLevelText(result.riskLevel)}
                    </p>
                  </div>
                </div>

                <div className="grid grid-cols-2 md:grid-cols-3 gap-4 mt-3 pt-3 border-t border-gray-200 dark:border-gray-700">
                  <div>
                    <p className="text-xs text-gray-500">国家</p>
                    <p className="text-sm">{result.countryCode}</p>
                  </div>
                  {result.city && (
                    <div>
                      <p className="text-xs text-gray-500">城市</p>
                      <p className="text-sm">{result.city}</p>
                    </div>
                  )}
                  <div className="flex gap-3">
                    <span className={`text-xs ${result.isProxy ? 'text-red-500 font-medium' : 'text-gray-400'}`}>
                      代理: {result.isProxy ? '是' : '否'}
                    </span>
                    <span className={`text-xs ${result.isVpn ? 'text-red-500 font-medium' : 'text-gray-400'}`}>
                      VPN: {result.isVpn ? '是' : '否'}
                    </span>
                    <span className={`text-xs ${result.isTor ? 'text-red-500 font-medium' : 'text-gray-400'}`}>
                      Tor: {result.isTor ? '是' : '否'}
                    </span>
                  </div>
                </div>
              </div>
            )}
          </div>
        </Card>
      )}

      {/* 缓存列表 */}
      {showCache && config.enabled && (
        <Card>
          <div className="space-y-3">
            <div className="flex items-center justify-between">
              <h3 className="text-sm font-semibold">信誉缓存</h3>
              {cacheStats && (
                <p className="text-xs text-gray-500">
                  共 {cacheStats[0]} 条，过期 {cacheStats[1]} 条
                </p>
              )}
            </div>

            {cacheEntries.length === 0 ? (
              <p className="text-xs text-gray-400 text-center py-4">暂无缓存数据</p>
            ) : (
              <div className="overflow-x-auto">
                <table className="w-full text-xs">
                  <thead>
                    <tr className="border-b border-gray-200 dark:border-gray-700">
                      <th className="text-left py-2 px-2">IP</th>
                      <th className="text-left py-2 px-2">类型</th>
                      <th className="text-left py-2 px-2">ASN</th>
                      <th className="text-center py-2 px-2">评分</th>
                      <th className="text-left py-2 px-2">风险</th>
                      <th className="text-left py-2 px-2">国家</th>
                    </tr>
                  </thead>
                  <tbody>
                    {cacheEntries.map((entry) => (
                      <tr key={entry.ip} className="border-b border-gray-100 dark:border-gray-800">
                        <td className="py-1.5 px-2 font-mono">{entry.ip}</td>
                        <td className="py-1.5 px-2">
                          <span className={`px-1.5 py-0.5 rounded text-[10px] font-medium ${ipTypeBadge(entry.ipType)}`}>
                            {getIpTypeText(entry.ipType)}
                          </span>
                        </td>
                        <td className="py-1.5 px-2 text-gray-500">{entry.asn}</td>
                        <td className={`py-1.5 px-2 text-center font-medium ${fraudScoreColor(entry.fraudScore)}`}>
                          {entry.fraudScore}
                        </td>
                        <td className={`py-1.5 px-2 ${getRiskLevelColor(entry.riskLevel)}`}>
                          {getRiskLevelText(entry.riskLevel)}
                        </td>
                        <td className="py-1.5 px-2">{entry.countryCode}</td>
                      </tr>
                    ))}
                  </tbody>
                </table>
              </div>
            )}
          </div>
        </Card>
      )}
    </div>
  )
}
