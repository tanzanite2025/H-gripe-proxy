import type { ChangeEvent, KeyboardEvent } from 'react'

import { Button } from '@/components/tailwind/Button'
import { Card } from '@/components/tailwind/Card'
import { TextField } from '@/components/tailwind/TextField'
import {
  getIpTypeText,
  getRiskLevelColor,
  getRiskLevelText,
} from '@/services/ip-reputation/presentation'
import type { IpReputation } from '@/services/ip-reputation/model'

import {
  getFraudScoreBg,
  getFraudScoreColor,
  getIpTypeBadgeClass,
} from './shared'

interface IpReputationLookupCardProps {
  enabled: boolean
  checkIp: string
  checking: boolean
  result: IpReputation | null
  onCheckIpChange: (value: string) => void
  onCheck: () => void | Promise<void>
}

export function IpReputationLookupCard({
  enabled,
  checkIp,
  checking,
  result,
  onCheckIpChange,
  onCheck,
}: IpReputationLookupCardProps) {
  if (!enabled) return null

  return (
    <Card>
      <div className="space-y-4">
        <h3 className="text-sm font-semibold">调试查询</h3>
        <div className="flex gap-2">
          <TextField
            placeholder="输入 IP 地址，例如 45.76.123.45"
            value={checkIp}
            onChange={(event: ChangeEvent<HTMLInputElement>) =>
              onCheckIpChange(event.target.value)
            }
            onKeyDown={(
              event: KeyboardEvent<HTMLInputElement | HTMLTextAreaElement>,
            ) => {
              if (event.key === 'Enter') {
                void onCheck()
              }
            }}
            fullWidth
          />
          <Button
            onClick={() => void onCheck()}
            disabled={checking || !checkIp.trim()}
          >
            {checking ? '查询中...' : '查询'}
          </Button>
        </div>

        {result && (
          <div className={`rounded-lg p-4 ${getFraudScoreBg(result.fraudScore)}`}>
            <div className="grid grid-cols-2 gap-4 md:grid-cols-4">
              <div>
                <p className="text-xs text-gray-500">IP 地址</p>
                <p className="text-sm font-medium font-mono">{result.ip}</p>
              </div>
              <div>
                <p className="text-xs text-gray-500">IP 类型</p>
                <span
                  className={`mt-1 inline-block rounded px-2 py-0.5 text-xs font-medium ${getIpTypeBadgeClass(result.ipType)}`}
                >
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
                <p
                  className={`text-2xl font-bold ${getFraudScoreColor(result.fraudScore)}`}
                >
                  {result.fraudScore}
                </p>
                <p
                  className={`text-xs ${getRiskLevelColor(result.riskLevel)}`}
                >
                  {getRiskLevelText(result.riskLevel)}
                </p>
              </div>
            </div>

            <div className="mt-3 grid grid-cols-2 gap-4 border-t border-gray-200 pt-3 dark:border-gray-700 md:grid-cols-3">
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
                <span
                  className={`text-xs ${
                    result.isProxy ? 'font-medium text-red-500' : 'text-gray-400'
                  }`}
                >
                  代理: {result.isProxy ? '是' : '否'}
                </span>
                <span
                  className={`text-xs ${
                    result.isVpn ? 'font-medium text-red-500' : 'text-gray-400'
                  }`}
                >
                  VPN: {result.isVpn ? '是' : '否'}
                </span>
                <span
                  className={`text-xs ${
                    result.isTor ? 'font-medium text-red-500' : 'text-gray-400'
                  }`}
                >
                  Tor: {result.isTor ? '是' : '否'}
                </span>
              </div>
            </div>
          </div>
        )}
      </div>
    </Card>
  )
}
