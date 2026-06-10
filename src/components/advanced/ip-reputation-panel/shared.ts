import type {
  IdentityConsistencyDrift,
  IdentityConsistencyLevel,
  IdentityConsistencyReport,
  IdentityConsistencySnapshot,
} from '@/services/cmds/diagnostics'

export const consistencyLevelText: Record<IdentityConsistencyLevel, string> = {
  good: '良好',
  warning: '需关注',
  danger: '高风险',
  unknown: '未知',
}

export const consistencyScoreColor: Record<IdentityConsistencyLevel, string> = {
  good: 'text-green-600',
  warning: 'text-yellow-600',
  danger: 'text-red-600',
  unknown: 'text-gray-500',
}

export const consistencyBadgeColor: Record<IdentityConsistencyLevel, string> = {
  good: 'bg-green-100 text-green-700 dark:bg-green-900/30 dark:text-green-400',
  warning:
    'bg-yellow-100 text-yellow-700 dark:bg-yellow-900/30 dark:text-yellow-400',
  danger: 'bg-red-100 text-red-700 dark:bg-red-900/30 dark:text-red-400',
  unknown: 'bg-gray-100 text-gray-700 dark:bg-gray-900/30 dark:text-gray-400',
}

export const driftKindText: Record<IdentityConsistencyDrift['kind'], string> =
  {
    publicEgressIp: '公网出口',
    ipType: 'IP 类型',
    dnsAssessment: 'DNS',
    tlsFingerprint: 'TLS 指纹',
  }

export const formatConsistencyValue = (
  value: string | number | null | undefined,
) => (value === null || value === undefined || value === '' ? '未观测' : String(value))

export const formatProxyChain = (report: IdentityConsistencyReport) =>
  report.proxy_chain.length > 0 ? report.proxy_chain.join(' -> ') : '未观测'

export const formatSnapshotTime = (snapshot: IdentityConsistencySnapshot) =>
  snapshot.observed_at
    ? new Date(snapshot.observed_at).toLocaleString()
    : '未知时间'

export const snapshotSummary = (snapshot: IdentityConsistencySnapshot) => {
  const report = snapshot.report

  return [
    report.public_egress_ip || '未观测出口',
    report.ip_type || '未知类型',
    report.dns_assessment ? `DNS ${report.dns_assessment}` : null,
    report.tls_fingerprint ? `TLS ${report.tls_fingerprint}` : null,
  ]
    .filter(Boolean)
    .join(' / ')
}

export const driftValue = (value: string | null) => value || '未观测'

export const getFraudScoreColor = (score: number) => {
  if (score <= 30) return 'text-green-600'
  if (score <= 60) return 'text-yellow-600'
  if (score <= 85) return 'text-orange-600'
  return 'text-red-600'
}

export const getFraudScoreBg = (score: number) => {
  if (score <= 30) return 'bg-green-100 dark:bg-green-900/30'
  if (score <= 60) return 'bg-yellow-100 dark:bg-yellow-900/30'
  if (score <= 85) return 'bg-orange-100 dark:bg-orange-900/30'
  return 'bg-red-100 dark:bg-red-900/30'
}

export const getIpTypeBadgeClass = (type: string) => {
  const colors: Record<string, string> = {
    Datacenter: 'bg-red-100 text-red-700 dark:bg-red-900/30 dark:text-red-400',
    Residential:
      'bg-green-100 text-green-700 dark:bg-green-900/30 dark:text-green-400',
    Mobile: 'bg-blue-100 text-blue-700 dark:bg-blue-900/30 dark:text-blue-400',
    Education:
      'bg-purple-100 text-purple-700 dark:bg-purple-900/30 dark:text-purple-400',
    Unknown: 'bg-gray-100 text-gray-700 dark:bg-gray-900/30 dark:text-gray-400',
  }

  return colors[type] || colors.Unknown
}
