import type { TorRuntimeStatus } from '@/services/cmds'

import type { DnsStatusColor } from './dns-runtime-view-model'

export function formatTorAssessmentLabel(assessment?: string) {
  switch (assessment) {
    case 'connected':
      return '已验证连通'
    case 'runtime-risk':
      return '存在运行风险'
    case 'inconclusive':
      return '结果不确定'
    case 'disabled':
      return '未启用'
    default:
      return assessment || '未知'
  }
}

export function getTorAssessmentColor(assessment?: string): DnsStatusColor {
  switch (assessment) {
    case 'connected':
      return 'success'
    case 'runtime-risk':
      return 'warning'
    case 'inconclusive':
      return 'info'
    case 'disabled':
      return 'default'
    default:
      return 'default'
  }
}

export function formatTorConfidenceLabel(confidence?: string) {
  switch (confidence) {
    case 'high':
      return '高置信度'
    case 'medium':
      return '中置信度'
    case 'low':
      return '低置信度'
    default:
      return confidence || '未知'
  }
}

export function formatTorRuntimeRiskLabel(risk: string) {
  switch (risk) {
    case 'non-local-socks-endpoint':
      return 'SOCKS 端点不是本机地址'
    case 'invalid-socks-port':
      return 'SOCKS 端口无效'
    case 'bridges-enabled-without-bridges':
      return '已启用桥接但未配置 bridge'
    default:
      return risk
  }
}

export function buildTorRuntimeViewModel(
  status: TorRuntimeStatus | undefined,
  fallbackEnabled: boolean,
  pending: boolean,
) {
  const connection = status?.connected
    ? ({
        label: '已连接',
        color: 'success',
        connected: true,
      } satisfies { label: string; color: DnsStatusColor; connected: boolean })
    : pending
      ? ({
          label: '检测中',
          color: 'info',
          connected: false,
        } satisfies { label: string; color: DnsStatusColor; connected: boolean })
      : ({
          label: '未连接',
          color: 'error',
          connected: false,
        } satisfies { label: string; color: DnsStatusColor; connected: boolean })

  const enabled = status?.enabled ?? fallbackEnabled

  return {
    enabled: {
      active: enabled,
      label: enabled ? '已启用' : '未启用',
      color: enabled ? 'success' : 'default',
    } satisfies { active: boolean; label: string; color: DnsStatusColor },
    connection,
    assessment: status?.assessment
      ? {
          label: formatTorAssessmentLabel(status.assessment),
          color: getTorAssessmentColor(status.assessment),
        }
      : null,
    confidence: status?.confidence
      ? {
          label: formatTorConfidenceLabel(status.confidence),
          color: 'info' as const,
        }
      : null,
    runtimeRiskText:
      status?.runtime_risk_type.map(formatTorRuntimeRiskLabel).join('；') || '',
  }
}
