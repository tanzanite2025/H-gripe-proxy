import { useQuery } from '@tanstack/react-query'
import { Globe2, ShieldAlert } from 'lucide-react'

import { useIPInfo } from '@/hooks/data'
import {
  getIpTypeText,
  getResidentialStateText,
  getRiskLevelText,
  ipReputationCheckIp,
  type IpReputation,
} from '@/services/ip-reputation'
import { getCurrentEgressIdentity, getIdentityConsistencyReport } from '@/services/cmds'
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

const consistencyColorMap = {
  good: 'text-green-500',
  warning: 'text-yellow-500',
  danger: 'text-red-500',
  unknown: 'text-gray-500',
} as const

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

const formatEgressTypeSummary = ({
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
  if (!ip) return '未获取'
  if (reputationLoading) return '识别中'
  if (reputationError) return '识别失败'
  if (!reputation) return '未识别'

  return `${getIpTypeText(reputation.ipType)} / ${reputation.confidence}`
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
    data: currentIdentity,
    error: currentIdentityError,
    isLoading: currentIdentityLoading,
  } = useQuery({
    queryKey: ['current-egress-identity'],
    queryFn: getCurrentEgressIdentity,
    staleTime: 30 * 1000,
    gcTime: 5 * 60 * 1000,
    retry: 1,
  })
  const { data: consistencyReport } = useQuery({
    queryKey: ['identity-consistency-report'],
    queryFn: getIdentityConsistencyReport,
    staleTime: 30 * 1000,
    gcTime: 5 * 60 * 1000,
    retry: 1,
  })
  const identityReputation = currentIdentity?.reputation
  const {
    data: reputation,
    error: reputationError,
    isLoading: reputationLoading,
  } = useQuery({
    queryKey: ['ip-reputation-summary', ip],
    queryFn: () => ipReputationCheckIp(ip!),
    enabled: Boolean(ip) && !identityReputation,
    staleTime: 5 * 60 * 1000,
    gcTime: 60 * 60 * 1000,
    retry: 1,
  })
  const effectiveReputation = identityReputation ?? reputation
  const effectiveReputationLoading = currentIdentityLoading || (!identityReputation && reputationLoading)
  const effectiveReputationError = currentIdentityError && reputationError

  const country = ipInfo?.country || 'Unknown'
  const flag = getCountryFlag(ipInfo?.country_code)
  const displayIp = currentIdentity?.public_egress_ip || currentIdentity?.egress_ip || ip
  const location = formatLocation(ipInfo?.city, ipInfo?.region, ipInfo?.country)
  const riskText = formatReputationSummary({
    ip,
    reputation: effectiveReputation,
    reputationLoading: effectiveReputationLoading,
    reputationError: effectiveReputationError,
  })
  const egressTypeText = formatEgressTypeSummary({
    ip: currentIdentity?.public_egress_ip ?? currentIdentity?.egress_ip ?? ip,
    reputation: effectiveReputation,
    reputationLoading: effectiveReputationLoading,
    reputationError: effectiveReputationError,
  })
  const residentialStateText = effectiveReputation
    ? getResidentialStateText(effectiveReputation.residentialState)
    : '未确认'
  const consistencyText = consistencyReport
    ? `${consistencyReport.score} / ${consistencyReport.level}`
    : '未评分'
  const consistencyIssuesText =
    consistencyReport?.issues
      .slice(0, 3)
      .map((issue) => issue.message)
      .join('; ') || '无'
  const identitySourceText =
    currentIdentity?.source === 'mihomoEgressStatus'
      ? '内核出口快照'
      : currentIdentity?.source === 'mihomoConnectionMetadata'
      ? '内核连接元数据'
      : currentIdentity?.source === 'publicIpObservation'
        ? '出口观测'
        : '出口观测'

  const cardTitle = [
    `${country} / ${displayIp || 'Unknown'}`,
    location,
    identitySourceText,
    currentIdentity?.proxy_name || '当前出口',
    `出口类型: ${egressTypeText}`,
    residentialStateText,
    `风险: ${riskText}`,
    `一致性: ${consistencyText}`,
    `问题: ${consistencyIssuesText}`,
  ].join(' | ')

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
      title={cardTitle}
    >
      <span className="the-ip-card__flag" data-tauri-drag-region="true">{flag}</span>
      <span className="the-ip-card__primary" data-tauri-drag-region="true">{country}</span>
      <span className="the-ip-card__muted" data-tauri-drag-region="true">IP</span>
      <span className="the-ip-card__mono" data-tauri-drag-region="true">{displayIp || 'Unknown'}</span>
      <span className="the-ip-card__divider" data-tauri-drag-region="true" />
      <span className="the-ip-card__muted" data-tauri-drag-region="true">位置</span>
      <span className="the-ip-card__value" data-tauri-drag-region="true">{location}</span>
      <span className="the-ip-card__divider" data-tauri-drag-region="true" />
      <ShieldAlert
        className="h-3.5 w-3.5 shrink-0 text-text-secondary"
        data-tauri-drag-region="true"
      />
      <span className="the-ip-card__muted" data-tauri-drag-region="true">出口类型</span>
      <span
        className={cn(
          'the-ip-card__value',
          effectiveReputation ? riskColorMap[effectiveReputation.riskLevel] ?? 'text-gray-500' : undefined,
        )}
        data-tauri-drag-region="true"
      >
        {egressTypeText}
      </span>
      <span className="the-ip-card__divider" data-tauri-drag-region="true" />
      <span className="the-ip-card__muted" data-tauri-drag-region="true">一致性</span>
      <span
        className={cn(
          'the-ip-card__value',
          consistencyReport ? consistencyColorMap[consistencyReport.level] : 'text-gray-500',
        )}
        data-tauri-drag-region="true"
      >
        {consistencyReport ? consistencyReport.score : '--'}
      </span>
    </div>
  )
}
