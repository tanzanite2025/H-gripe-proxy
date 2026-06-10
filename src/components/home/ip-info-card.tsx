import { Globe2, ShieldAlert } from 'lucide-react'

import { useCurrentEgressIdentity } from '@/hooks/data'
import type {
  CurrentEgressIdentity,
  CurrentEgressIdentitySource,
} from '@/services/cmds/diagnostics'
import {
  getIpTypeText,
  getResidentialStateText,
  getRiskLevelText,
} from '@/services/ip-reputation/presentation'
import type { IpReputation } from '@/services/ip-reputation/model'
import { cn } from '@/utils/cn'

const isIPv4Address = (value: unknown): value is string => {
  if (typeof value !== 'string') return false

  const parts = value.trim().split('.')
  return (
    parts.length === 4 &&
    parts.every((part) => {
      if (!/^\d{1,3}$/.test(part)) return false
      const segmentValue = Number(part)
      return segmentValue >= 0 && segmentValue <= 255
    })
  )
}

const selectDisplayIp = (...candidates: Array<string | null | undefined>) => {
  const validCandidates = candidates.filter((candidate): candidate is string =>
    Boolean(candidate?.trim()),
  )

  return (
    validCandidates.find(isIPv4Address) ??
    validCandidates.find((candidate) => !candidate.includes(':')) ??
    validCandidates[0]
  )
}

const OBSERVED_SOURCES = new Set<CurrentEgressIdentitySource>([
  'mihomoEgressStatus',
  'mihomoProxyProbe',
])

const isObservedSource = (source?: CurrentEgressIdentitySource | null) =>
  Boolean(source && OBSERVED_SOURCES.has(source))

const riskColorMap: Record<IpReputation['riskLevel'], string> = {
  Low: 'text-green-500',
  Medium: 'text-yellow-500',
  High: 'text-orange-500',
  VeryHigh: 'text-red-500',
}

const getRiskColor = (reputation?: IpReputation) =>
  reputation
    ? (riskColorMap[reputation.riskLevel] ?? 'text-gray-500')
    : undefined

const formatReputationSummary = (reputation?: IpReputation) => {
  if (!reputation) return '未评估'
  if (!Number.isFinite(reputation.fraudScore) || !reputation.riskLevel) {
    return '结果异常'
  }

  return `${getRiskLevelText(reputation.riskLevel)} / ${reputation.fraudScore}`
}

const formatEgressTypeSummary = (reputation?: IpReputation) => {
  if (!reputation) return '未识别'

  return `${getIpTypeText(reputation.ipType)} / ${reputation.confidence}`
}

const formatAsnSummary = (
  destinationAsn?: string | null,
  asnOrg?: string | null,
) =>
  [destinationAsn, asnOrg]
    .filter((value): value is string => Boolean(value?.trim()))
    .join(' / ') || '未返回'

const formatUnavailableSummary = (identity?: CurrentEgressIdentity) => {
  const message = identity?.message?.trim()

  if (!message) {
    return '当前内核未返回出口快照'
  }

  const normalizedMessage = message.toLowerCase()

  if (
    normalizedMessage.includes('has not observed a public egress ip yet') &&
    normalizedMessage.includes('proxy probe also did not produce')
  ) {
    return 'Mihomo 暂未观测到出口 IP，主动探测也没有拿到结果'
  }

  if (normalizedMessage.includes('has not observed a public egress ip yet')) {
    return 'Mihomo 暂未观测到出口 IP'
  }

  if (normalizedMessage.includes('failed to query mihomo egress status')) {
    return '读取 Mihomo 出口快照失败'
  }

  return message
}

const formatIdentitySourceText = (source?: CurrentEgressIdentitySource | null) => {
  switch (source) {
    case 'mihomoEgressStatus':
      return 'Mihomo 出口快照'
    case 'mihomoProxyProbe':
      return 'Mihomo 主动探测'
    default:
      return '内核未观测'
  }
}

interface IpInfoCardProps {
  className?: string
}

const EGRESS_IDENTITY_PENDING_REFRESH_INTERVAL = 5 * 1000
const EGRESS_IDENTITY_STABLE_REFRESH_INTERVAL = 30 * 1000

export const IpInfoCard = ({ className }: IpInfoCardProps) => {
  const {
    data: currentIdentity,
    error,
    isLoading,
  } = useCurrentEgressIdentity({
    staleTime: 30 * 1000,
    gcTime: 5 * 60 * 1000,
    refetchOnWindowFocus: false,
    refetchOnReconnect: true,
    refetchInterval: (query) => {
      const cachedIdentity = query.state.data as CurrentEgressIdentity | undefined
      const cachedDisplayIp = selectDisplayIp(
        cachedIdentity?.public_egress_ip,
        cachedIdentity?.egress_ip,
      )

      if (!isObservedSource(cachedIdentity?.source) || !cachedDisplayIp) {
        return EGRESS_IDENTITY_PENDING_REFRESH_INTERVAL
      }

      return EGRESS_IDENTITY_STABLE_REFRESH_INTERVAL
    },
    retry: 1,
  })

  const reputation = currentIdentity?.reputation ?? undefined
  const displayIp = selectDisplayIp(
    currentIdentity?.public_egress_ip,
    currentIdentity?.egress_ip,
  )
  const asnText = formatAsnSummary(
    currentIdentity?.destination_asn,
    currentIdentity?.asn_org,
  )
  const riskText = formatReputationSummary(reputation)
  const egressTypeText = formatEgressTypeSummary(reputation)
  const residentialStateText = reputation
    ? getResidentialStateText(reputation.residentialState)
    : '未确认'
  const identitySourceText = formatIdentitySourceText(currentIdentity?.source)
  const unavailableSummary = formatUnavailableSummary(currentIdentity)

  const cardTitle = [
    `出口 IP: ${displayIp || '未观测'}`,
    `来源: ${identitySourceText}`,
    `节点: ${currentIdentity?.proxy_name || '当前出口'}`,
    `ASN: ${asnText}`,
    `出口类型: ${egressTypeText}`,
    `住宅状态: ${residentialStateText}`,
    `风险: ${riskText}`,
    `规则: ${currentIdentity?.rule || '--'}`,
    `说明: ${currentIdentity?.message || '--'}`,
  ].join(' | ')

  if (isLoading) {
    return (
      <div
        className={cn('the-ip-card', className)}
        data-tauri-drag-region="true"
      >
        <Globe2
          className="h-3.5 w-3.5 shrink-0 text-teal-400"
          data-tauri-drag-region="true"
        />
        <span className="the-ip-card__muted" data-tauri-drag-region="true">
          出口识别中...
        </span>
      </div>
    )
  }

  if (error) {
    return (
      <div
        className={cn('the-ip-card', className)}
        data-tauri-drag-region="true"
      >
        <Globe2
          className="h-3.5 w-3.5 shrink-0 text-red-400"
          data-tauri-drag-region="true"
        />
        <span className="the-ip-card__muted" data-tauri-drag-region="true">
          出口信息获取失败
        </span>
      </div>
    )
  }

  if (!isObservedSource(currentIdentity?.source) || !displayIp) {
    return (
      <div
        className={cn('the-ip-card', className)}
        data-tauri-drag-region="true"
        title={cardTitle}
      >
        <Globe2
          className="h-3.5 w-3.5 shrink-0 text-yellow-500"
          data-tauri-drag-region="true"
        />
        <span className="the-ip-card__muted" data-tauri-drag-region="true">
          出口未观测
        </span>
        <span className="the-ip-card__divider" data-tauri-drag-region="true" />
        <span className="the-ip-card__value" data-tauri-drag-region="true">
          {unavailableSummary}
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
      <span className="the-ip-card__muted" data-tauri-drag-region="true">
        出口 IP
      </span>
      <span className="the-ip-card__mono" data-tauri-drag-region="true">
        {displayIp}
      </span>
      <span className="the-ip-card__divider" data-tauri-drag-region="true" />
      <span className="the-ip-card__muted" data-tauri-drag-region="true">
        ASN
      </span>
      <span className="the-ip-card__value" data-tauri-drag-region="true">
        {asnText}
      </span>
      <span className="the-ip-card__divider" data-tauri-drag-region="true" />
      <ShieldAlert
        className="h-3.5 w-3.5 shrink-0 text-text-secondary"
        data-tauri-drag-region="true"
      />
      <span className="the-ip-card__muted" data-tauri-drag-region="true">
        出口类型
      </span>
      <span
        className={cn('the-ip-card__value', getRiskColor(reputation))}
        data-tauri-drag-region="true"
      >
        {egressTypeText}
      </span>
      <span className="the-ip-card__divider" data-tauri-drag-region="true" />
      <span className="the-ip-card__muted" data-tauri-drag-region="true">
        风险
      </span>
      <span
        className={cn('the-ip-card__value', getRiskColor(reputation))}
        data-tauri-drag-region="true"
      >
        {riskText}
      </span>
    </div>
  )
}
