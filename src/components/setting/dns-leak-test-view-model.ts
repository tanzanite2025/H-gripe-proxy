import type { DnsLeakTestResult } from '@/services/cmds'
import type { DNSLeakResult } from '@/services/dns-leak-detection'

import type { DnsStatusColor } from './dns-runtime-view-model'

type DnsAlertSeverity = 'error' | 'warning' | 'info' | 'success'

export function formatDnsLeakAssessmentLabel(
  assessment: string | null | undefined,
) {
  switch (assessment) {
    case 'observed-leak':
      return '已观察到泄漏'
    case 'runtime-risk':
      return '运行态风险'
    case 'inconclusive':
      return '结果不完整'
    case 'safe':
      return '安全'
    default:
      return '未知'
  }
}

export function getDnsLeakAssessmentColor(
  assessment: string | null | undefined,
): DnsStatusColor {
  switch (assessment) {
    case 'observed-leak':
      return 'error'
    case 'runtime-risk':
      return 'warning'
    case 'inconclusive':
      return 'info'
    case 'safe':
      return 'success'
    default:
      return 'default'
  }
}

export function formatDnsLeakRiskLevelLabel(
  riskLevel: string | null | undefined,
) {
  switch (riskLevel) {
    case 'danger':
      return '高风险'
    case 'warning':
      return '警告'
    case 'safe':
      return '安全'
    default:
      return '未知'
  }
}

export function getDnsLeakRiskLevelColor(
  riskLevel: string | null | undefined,
): DnsStatusColor {
  switch (riskLevel) {
    case 'danger':
      return 'error'
    case 'warning':
      return 'warning'
    case 'safe':
      return 'success'
    default:
      return 'default'
  }
}

export function formatDnsLeakConfidenceLabel(
  confidence: string | null | undefined,
) {
  switch (confidence) {
    case 'high':
      return '高'
    case 'medium':
      return '中'
    case 'low':
      return '低'
    default:
      return '未知'
  }
}

export function getDnsLeakConfidenceColor(
  confidence: string | null | undefined,
): DnsStatusColor {
  switch (confidence) {
    case 'high':
      return 'success'
    case 'medium':
      return 'info'
    case 'low':
      return 'warning'
    default:
      return 'default'
  }
}

export function buildDnsLeakObservationPath(
  observationPath: string | null | undefined,
) {
  switch (observationPath) {
    case 'core-proxy':
      return { label: '通过本地内核代理观测', color: 'success' } satisfies {
        label: string
        color: DnsStatusColor
      }
    case 'core-proxy-fallback-direct':
      return { label: '内核代理失败后直连', color: 'warning' } satisfies {
        label: string
        color: DnsStatusColor
      }
    case 'direct':
      return { label: '直接观测', color: 'warning' } satisfies {
        label: string
        color: DnsStatusColor
      }
    default:
      return { label: '未知路径', color: 'default' } satisfies {
        label: string
        color: DnsStatusColor
      }
  }
}

export function formatDnsLeakStatusMessage(result: {
  observedLeak?: boolean
  runtimeRiskDetected?: boolean
  observationIncomplete?: boolean
}) {
  if (result.observedLeak) {
    return '已观察到外部 DNS 泄漏信号'
  }
  if (result.runtimeRiskDetected) {
    return '当前未直接观察到泄漏，但运行态存在 DNS 风险'
  }
  if (result.observationIncomplete) {
    return '外部观测不完整，结果偏保守'
  }
  return '当前未发现 DNS 泄漏或运行态风险'
}

export function buildDnsLeakTestViewModel(result: DnsLeakTestResult) {
  return {
    assessment: {
      label: formatDnsLeakAssessmentLabel(result.assessment),
      color: getDnsLeakAssessmentColor(result.assessment),
    },
    riskLevel: {
      label: formatDnsLeakRiskLevelLabel(result.risk_level),
      color: getDnsLeakRiskLevelColor(result.risk_level),
    },
    confidence: {
      label: formatDnsLeakConfidenceLabel(result.confidence),
      color: getDnsLeakConfidenceColor(result.confidence),
    },
    observationPath: buildDnsLeakObservationPath(result.observation_path),
    alert: {
      severity: (
        result.assessment === 'observed-leak'
          ? 'error'
          : result.assessment === 'runtime-risk'
            ? 'warning'
            : result.assessment === 'inconclusive'
              ? 'info'
              : 'success'
      ) as DnsAlertSeverity,
      message:
        result.assessment === 'observed-leak'
          ? '检测到 DNS 泄漏'
          : result.assessment === 'runtime-risk'
            ? '未直接观察到泄漏，但运行态存在风险信号'
            : result.assessment === 'inconclusive'
              ? '外部观测不完整，结果仅供参考'
              : '未检测到 DNS 泄漏',
    },
  }
}

export function buildHomeDnsLeakViewModel(result: DNSLeakResult) {
  return {
    assessment: {
      label: formatDnsLeakAssessmentLabel(result.assessment),
      color: getDnsLeakAssessmentColor(result.assessment),
    },
    confidence: {
      label: formatDnsLeakConfidenceLabel(result.confidence),
      color: getDnsLeakConfidenceColor(result.confidence),
    },
    statusMessage: formatDnsLeakStatusMessage(result),
  }
}
