import { Card } from '@/components/tailwind/Card'
import type { IpReputation } from '@/services/ip-reputation/model'
import {
  getIpTypeText,
  getRiskLevelColor,
  getRiskLevelText,
} from '@/services/ip-reputation/presentation'

import { getFraudScoreColor, getIpTypeBadgeClass } from './shared'

interface IpReputationCacheCardProps {
  enabled: boolean
  visible: boolean
  stats: [number, number] | null
  entries: IpReputation[]
}

export function IpReputationCacheCard({
  enabled,
  visible,
  stats,
  entries,
}: IpReputationCacheCardProps) {
  if (!enabled || !visible) return null

  return (
    <Card>
      <div className="space-y-3">
        <div className="flex items-center justify-between">
          <h3 className="text-sm font-semibold">信誉缓存</h3>
          {stats && (
            <p className="text-xs text-gray-500">
              共 {stats[0]} 条，过期 {stats[1]} 条
            </p>
          )}
        </div>

        {entries.length === 0 ? (
          <p className="py-4 text-center text-xs text-gray-400">暂无缓存数据</p>
        ) : (
          <div className="overflow-x-auto">
            <table className="w-full text-xs">
              <thead>
                <tr className="border-b border-gray-200 dark:border-gray-700">
                  <th className="px-2 py-2 text-left">IP</th>
                  <th className="px-2 py-2 text-left">类型</th>
                  <th className="px-2 py-2 text-left">ASN</th>
                  <th className="px-2 py-2 text-center">评分</th>
                  <th className="px-2 py-2 text-left">风险</th>
                  <th className="px-2 py-2 text-left">国家</th>
                </tr>
              </thead>
              <tbody>
                {entries.map((entry) => (
                  <tr
                    key={entry.ip}
                    className="border-b border-gray-100 dark:border-gray-800"
                  >
                    <td className="px-2 py-1.5 font-mono">{entry.ip}</td>
                    <td className="px-2 py-1.5">
                      <span
                        className={`rounded px-1.5 py-0.5 text-[10px] font-medium ${getIpTypeBadgeClass(entry.ipType)}`}
                      >
                        {getIpTypeText(entry.ipType)}
                      </span>
                    </td>
                    <td className="px-2 py-1.5 text-gray-500">{entry.asn}</td>
                    <td
                      className={`px-2 py-1.5 text-center font-medium ${getFraudScoreColor(entry.fraudScore)}`}
                    >
                      {entry.fraudScore}
                    </td>
                    <td
                      className={`px-2 py-1.5 ${getRiskLevelColor(entry.riskLevel)}`}
                    >
                      {getRiskLevelText(entry.riskLevel)}
                    </td>
                    <td className="px-2 py-1.5">{entry.countryCode}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        )}
      </div>
    </Card>
  )
}
