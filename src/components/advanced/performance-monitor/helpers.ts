import type { ChipProps } from '@/components/tailwind'
import type { CoordinatorStatus } from '@/services/coordinator'

export interface MonitorMetric {
  label: string
  value: string | number
  color?: ChipProps['color']
}

export interface MonitorCardViewModel {
  title: string
  badgeLabel: string
  badgeColor: ChipProps['color']
  metrics: MonitorMetric[]
  description?: string
}

export interface MonitorRecommendation {
  message: string
  tone: 'info' | 'success'
}

export function buildMonitorCards(
  status: CoordinatorStatus,
): MonitorCardViewModel[] {
  const domainPatternAssignments =
    status.runtimeState.stableEgressBackwrite.domainPatternAssignments.length
  const domainRuleBindings =
    status.runtimeState.stableEgressBackwrite.domainRuleBindings.length

  const cards: MonitorCardViewModel[] = [
    {
      title: '核心协调器',
      badgeLabel: status.initialized ? '已初始化' : '未初始化',
      badgeColor: status.initialized ? 'success' : 'error',
      metrics: [
        {
          label: '状态',
          value: status.initialized ? '运行正常' : '尚未初始化',
          color: status.initialized ? 'success' : 'error',
        },
      ],
    },
    {
      title: '安全监控',
      badgeLabel: status.securityCompromised
        ? '存在风险'
        : status.securityEnabled
          ? '已启用'
          : '未启用',
      badgeColor: status.securityCompromised
        ? 'error'
        : status.securityEnabled
          ? 'success'
          : 'warning',
      metrics: [
        {
          label: '状态',
          value: status.securityCompromised
            ? '检测到异常'
            : status.securityEnabled
              ? '运行中'
              : '未启用',
          color: status.securityCompromised
            ? 'error'
            : status.securityEnabled
              ? 'success'
              : 'warning',
        },
      ],
    },
    {
      title: '反主动探测',
      badgeLabel: status.antiProbeEnabled ? '已启用' : '未启用',
      badgeColor: status.antiProbeEnabled ? 'success' : 'warning',
      metrics: [
        {
          label: '状态',
          value: status.antiProbeEnabled ? '已开启' : '未开启',
          color: status.antiProbeEnabled ? 'success' : 'warning',
        },
      ],
    },
    {
      title: 'TLS 指纹',
      badgeLabel: status.tlsFingerprint ? '已设置' : '未设置',
      badgeColor: status.tlsFingerprint ? 'success' : 'warning',
      metrics: [
        {
          label: '当前指纹',
          value: status.tlsFingerprint ?? '未设置',
          color: status.tlsFingerprint ? 'success' : 'warning',
        },
      ],
    },
    {
      title: '出口身份管理',
      badgeLabel: status.egressIdentityEnabled ? '已启用' : '未启用',
      badgeColor: status.egressIdentityEnabled ? 'success' : 'warning',
      metrics: [
        {
          label: '状态',
          value: status.egressIdentityEnabled ? '已开启' : '未开启',
          color: status.egressIdentityEnabled ? 'success' : 'warning',
        },
        {
          label: '活跃 assignment',
          value: status.egressIdentityActiveAssignments,
          color: 'info',
        },
        {
          label: 'domain-pattern 回写',
          value: domainPatternAssignments,
          color: 'secondary',
        },
      ],
    },
    {
      title: '会话绑定',
      badgeLabel: status.sessionAffinityEnabled ? '已启用' : '未启用',
      badgeColor: status.sessionAffinityEnabled ? 'success' : 'warning',
      metrics: [
        {
          label: '状态',
          value: status.sessionAffinityEnabled ? '已开启' : '未开启',
          color: status.sessionAffinityEnabled ? 'success' : 'warning',
        },
        {
          label: '活跃绑定',
          value: status.sessionAffinityActiveBindings,
          color: 'info',
        },
        {
          label: 'domain-rule 回写',
          value: domainRuleBindings,
          color: 'secondary',
        },
      ],
    },
    {
      title: '稳定出口回写',
      badgeLabel:
        domainPatternAssignments > 0 || domainRuleBindings > 0
          ? '活跃'
          : '空闲',
      badgeColor:
        domainPatternAssignments > 0 || domainRuleBindings > 0
          ? 'success'
          : 'warning',
      metrics: [
        {
          label: 'domain-pattern',
          value: domainPatternAssignments,
          color: 'secondary',
        },
        {
          label: 'domain-rule',
          value: domainRuleBindings,
          color: 'secondary',
        },
      ],
      description:
        '这里汇总稳定出口策略对 egress identity 与 session affinity 产生的运行态回写结果。',
    },
    {
      title: '多路径路由',
      badgeLabel: status.multipathEnabled ? '已启用' : '未启用',
      badgeColor: status.multipathEnabled ? 'success' : 'warning',
      metrics: [
        {
          label: '状态',
          value: status.multipathEnabled ? '已开启' : '未开启',
          color: status.multipathEnabled ? 'success' : 'warning',
        },
      ],
    },
    {
      title: '流量混淆',
      badgeLabel: status.trafficObfuscationEnabled ? '已启用' : '未启用',
      badgeColor: status.trafficObfuscationEnabled ? 'success' : 'warning',
      metrics: [
        {
          label: '状态',
          value: status.trafficObfuscationEnabled ? '已开启' : '未开启',
          color: status.trafficObfuscationEnabled ? 'success' : 'warning',
        },
      ],
    },
  ]

  return cards
}

export function buildRecommendations(
  status: CoordinatorStatus,
): MonitorRecommendation[] {
  const recommendations: MonitorRecommendation[] = []

  if (!status.securityEnabled) {
    recommendations.push({
      message: '建议启用安全监控，先把运行态最基础的防护能力打开。',
      tone: 'info',
    })
  }

  if (!status.antiProbeEnabled) {
    recommendations.push({
      message: '建议启用反主动探测，降低入口被主动扫描和探测识别的概率。',
      tone: 'info',
    })
  }

  if (!status.tlsFingerprint) {
    recommendations.push({
      message: '建议设置 TLS 指纹，避免出口握手长期暴露为固定特征。',
      tone: 'info',
    })
  }

  if (!status.egressIdentityEnabled) {
    recommendations.push({
      message: '建议启用出口身份管理，统一应用、快捷方式和会话的出口画像。',
      tone: 'info',
    })
  }

  if (!status.sessionAffinityEnabled) {
    recommendations.push({
      message: '建议启用会话绑定，让稳定出口选择持续映射到域名、进程和连接会话。',
      tone: 'info',
    })
  }

  if (!status.trafficObfuscationEnabled) {
    recommendations.push({
      message: '建议按场景启用流量混淆，减少出口特征过于稳定时的识别风险。',
      tone: 'info',
    })
  }

  if (
    status.securityEnabled &&
    status.antiProbeEnabled &&
    status.tlsFingerprint &&
    status.egressIdentityEnabled &&
    status.sessionAffinityEnabled &&
    status.multipathEnabled
  ) {
    recommendations.push({
      message: '当前关键高级能力已经形成完整组合，运行态处于较理想的防护与调度状态。',
      tone: 'success',
    })
  }

  return recommendations
}
