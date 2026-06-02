import { useState } from 'react'

import { Button } from '@/components/tailwind/Button'
import { Card } from '@/components/tailwind/Card'
import { Switch } from '@/components/tailwind/Switch'
import { TextField } from '@/components/tailwind/TextField'
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

export function IpReputationPanel({ config, onChange }: Props) {
  const [checkIp, setCheckIp] = useState('')
  const [checking, setChecking] = useState(false)
  const [result, setResult] = useState<IpReputation | null>(null)
  const [cacheEntries, setCacheEntries] = useState<IpReputation[]>([])
  const [cacheStats, setCacheStats] = useState<[number, number] | null>(null)
  const [showCache, setShowCache] = useState(false)

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
