import { useQuery } from '@tanstack/react-query'
import { Globe2, ShieldAlert } from 'lucide-react'

import { useIPInfo } from '@/hooks/data'
import {
  getRiskLevelText,
  ipReputationCheckIp,
  type IpReputation,
} from '@/services/ip-reputation'
import { cn } from '@/utils/cn'

const getCountryFlag = (countryCode: string | undefined) => {
  if (!countryCode) return ''
  const codePoints = countryCode
    .toUpperCase()
    .split('')
    .map((char) => 127397 + char.charCodeAt(0))
  return String.fromCodePoint(...codePoints)
}

const riskColorMap: Record<IpReputation['riskLevel'], string> = {
  Low: 'text-green-500',
  Medium: 'text-yellow-500',
  High: 'text-orange-500',
  VeryHigh: 'text-red-500',
}

const formatReputationSummary = ({
  ip,
  reputation,
  reputationLoading,
  reputationError,
}: {
  ip?: string
  reputation?: IpReputation
  reputationLoading: boolean
  reputationError: unknown
}) => {
  if (!ip) return '未获取 IP'
  if (reputationLoading) return '检测中'
  if (reputationError) return '检测失败'
  if (!reputation) return '未检测'
  if (!Number.isFinite(reputation.fraudScore) || !reputation.riskLevel) {
    return '结果异常'
  }

  return `${getRiskLevelText(reputation.riskLevel)} / ${reputation.fraudScore}`
}

const formatLocation = (city?: string, region?: string, country?: string) =>
  [city, region].filter(Boolean).join(', ') || country || 'Unknown'

interface IpInfoCardProps {
  className?: string
}

export const IpInfoCard = ({ className }: IpInfoCardProps) => {
  const { data: ipInfo, error, isLoading } = useIPInfo()
  const ip = ipInfo?.ip
  const {
    data: reputation,
    error: reputationError,
    isLoading: reputationLoading,
  } = useQuery({
    queryKey: ['ip-reputation-summary', ip],
    queryFn: () => ipReputationCheckIp(ip!),
    enabled: Boolean(ip),
    staleTime: 5 * 60 * 1000,
    gcTime: 60 * 60 * 1000,
    retry: 1,
  })

  const country = ipInfo?.country || 'Unknown'
  const flag = getCountryFlag(ipInfo?.country_code)
  const location = formatLocation(ipInfo?.city, ipInfo?.region, ipInfo?.country)
  const riskText = formatReputationSummary({
    ip,
    reputation,
    reputationLoading,
    reputationError,
  })

  if (isLoading) {
    return (
      <div className={cn('the-ip-card', className)} data-tauri-drag-region="true">
        <Globe2
          className="h-3.5 w-3.5 shrink-0 text-teal-400"
          data-tauri-drag-region="true"
        />
        <span className="the-ip-card__muted" data-tauri-drag-region="true">
          IP 检测中...
        </span>
      </div>
    )
  }

  if (error) {
    return (
      <div className={cn('the-ip-card', className)} data-tauri-drag-region="true">
        <Globe2
          className="h-3.5 w-3.5 shrink-0 text-red-400"
          data-tauri-drag-region="true"
        />
        <span className="the-ip-card__muted" data-tauri-drag-region="true">
          IP 获取失败
        </span>
      </div>
    )
  }

  return (
    <div
      className={cn('the-ip-card', className)}
      data-tauri-drag-region="true"
      title={`${country} / ${ip || 'Unknown'} · ${location} · 欺诈: ${riskText}`}
    >
      <span className="the-ip-card__flag" data-tauri-drag-region="true">{flag}</span>
      <span className="the-ip-card__primary" data-tauri-drag-region="true">{country}</span>
      <span className="the-ip-card__muted" data-tauri-drag-region="true">IP</span>
      <span className="the-ip-card__mono" data-tauri-drag-region="true">{ip || 'Unknown'}</span>
      <span className="the-ip-card__divider" data-tauri-drag-region="true" />
      <span className="the-ip-card__muted" data-tauri-drag-region="true">位置</span>
      <span className="the-ip-card__value" data-tauri-drag-region="true">{location}</span>
      <span className="the-ip-card__divider" data-tauri-drag-region="true" />
      <ShieldAlert
        className="h-3.5 w-3.5 shrink-0 text-text-secondary"
        data-tauri-drag-region="true"
      />
      <span className="the-ip-card__muted" data-tauri-drag-region="true">欺诈</span>
      <span
        className={cn(
          'the-ip-card__value',
          reputation ? riskColorMap[reputation.riskLevel] ?? 'text-gray-500' : undefined,
        )}
        data-tauri-drag-region="true"
      >
        {riskText}
      </span>
    </div>
  )
}
