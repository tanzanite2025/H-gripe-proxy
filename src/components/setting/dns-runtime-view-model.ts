import type { DnsRuntimeStatus } from '@/services/cmds'

export type DnsStatusColor =
  | 'default'
  | 'primary'
  | 'secondary'
  | 'error'
  | 'warning'
  | 'info'
  | 'success'

type DnsStatusBadge = {
  label: string
  color: DnsStatusColor
}

const DEFAULT_EMPTY_LABEL = '未配置'

export const formatDnsRoutingModeLabel = (
  mode: string | null | undefined,
  emptyLabel = DEFAULT_EMPTY_LABEL,
) => {
  switch (mode) {
    case 'speed':
      return '速度优先'
    case 'privacy':
      return '隐私优先'
    case 'balanced':
      return '平衡模式'
    case 'custom':
      return '自定义'
    default:
      return emptyLabel
  }
}

export const getDnsRoutingModeColor = (
  mode: string | null | undefined,
): DnsStatusColor => {
  switch (mode) {
    case 'speed':
      return 'success'
    case 'privacy':
      return 'info'
    case 'balanced':
      return 'warning'
    case 'custom':
      return 'default'
    default:
      return 'default'
  }
}

export const formatDnsLeakProtectionLabel = (
  level: string | null | undefined,
  emptyLabel = DEFAULT_EMPTY_LABEL,
) => {
  switch (level) {
    case 'none':
      return '无防护'
    case 'basic':
      return '基础'
    case 'strict':
      return '严格'
    case 'paranoid':
      return '偏执'
    case 'custom':
      return '自定义'
    default:
      return emptyLabel
  }
}

export const getDnsLeakProtectionColor = (
  level: string | null | undefined,
): DnsStatusColor => {
  switch (level) {
    case 'none':
      return 'error'
    case 'basic':
      return 'warning'
    case 'strict':
      return 'info'
    case 'paranoid':
      return 'success'
    case 'custom':
      return 'default'
    default:
      return 'default'
  }
}

export const formatDnsLeakSecurityLabel = (
  security: string | null | undefined,
  emptyLabel = DEFAULT_EMPTY_LABEL,
) => {
  switch (security) {
    case 'low':
      return '低'
    case 'medium':
      return '中'
    case 'high':
      return '高'
    case 'very-high':
      return '很高'
    case 'maximum':
      return '最高'
    case 'custom':
      return '自定义'
    default:
      return emptyLabel
  }
}

export const getDnsLeakSecurityColor = (
  security: string | null | undefined,
): DnsStatusColor => {
  switch (security) {
    case 'low':
      return 'error'
    case 'medium':
      return 'warning'
    case 'high':
      return 'info'
    case 'very-high':
    case 'maximum':
      return 'success'
    default:
      return 'default'
  }
}

export const formatDnsRuntimeBool = (
  value: boolean | null | undefined,
  enabledLabel = '已开启',
  disabledLabel = '已关闭',
  emptyLabel = DEFAULT_EMPTY_LABEL,
) => {
  if (value === null || value === undefined) {
    return emptyLabel
  }

  return value ? enabledLabel : disabledLabel
}

export const getDnsRuntimeBoolColor = (
  value: boolean | null | undefined,
): DnsStatusColor => {
  if (value === null || value === undefined) {
    return 'default'
  }

  return value ? 'success' : 'warning'
}

const buildPresence = (present: boolean): DnsStatusBadge => ({
  label: present ? '存在' : '缺失',
  color: present ? 'success' : 'warning',
})

const buildAlignment = (aligned: boolean): DnsStatusBadge => ({
  label: aligned ? '已对齐' : '未对齐',
  color: aligned ? 'success' : 'warning',
})

const joinDnsList = (servers: string[]) => servers.join(', ') || DEFAULT_EMPTY_LABEL

export function buildDnsRuntimeViewModel(runtimeStatus: DnsRuntimeStatus) {
  const { snapshot, derived } = runtimeStatus
  const routingMode = derived.routing_mode
  const leakLevel = derived.leak_protection_level
  const leakSecurity = derived.leak_protection_security
  const leakSafe = derived.leak_protection_safe
  const summary = [
    snapshot.enhanced_mode ? `模式 ${snapshot.enhanced_mode}` : null,
    `nameserver ${snapshot.nameserver_count}`,
    `fallback ${snapshot.fallback_count}`,
  ]
    .filter(Boolean)
    .join(' / ')

  return {
    summary,
    nameserverCount: snapshot.nameserver_count,
    fallbackCount: snapshot.fallback_count,
    defaultNameserverCount: derived.default_nameserver_count,
    enhancedModeLabel: snapshot.enhanced_mode ?? DEFAULT_EMPTY_LABEL,
    runtimeDnsPresence: buildPresence(runtimeStatus.runtime_has_dns),
    runtimeHostsPresence: buildPresence(runtimeStatus.runtime_has_hosts),
    runtimeDnsInjectedLabel: runtimeStatus.runtime_has_dns
      ? '运行态已注入 DNS'
      : '运行态未注入 DNS',
    runtimeAlignment: buildAlignment(runtimeStatus.runtime_matches_saved),
    runtimeDnsAlignment: buildAlignment(runtimeStatus.runtime_dns_matches_saved),
    runtimeHostsAlignment: buildAlignment(
      runtimeStatus.runtime_hosts_matches_saved,
    ),
    options: {
      ipv6: {
        label: formatDnsRuntimeBool(snapshot.ipv6, '已开启', '已关闭'),
        color: getDnsRuntimeBoolColor(snapshot.ipv6),
      },
      preferH3: {
        label: formatDnsRuntimeBool(derived.prefer_h3, '已开启', '已关闭'),
        color: getDnsRuntimeBoolColor(derived.prefer_h3),
      },
      useHosts: {
        label: formatDnsRuntimeBool(snapshot.use_hosts, '已开启', '已关闭'),
        color: getDnsRuntimeBoolColor(snapshot.use_hosts),
      },
      useSystemHosts: {
        label: formatDnsRuntimeBool(
          snapshot.use_system_hosts,
          '已开启',
          '已关闭',
        ),
        color: getDnsRuntimeBoolColor(snapshot.use_system_hosts),
      },
      respectRules: {
        label: formatDnsRuntimeBool(
          snapshot.respect_rules,
          '已开启',
          '已关闭',
        ),
        color: getDnsRuntimeBoolColor(snapshot.respect_rules),
      },
    },
    dnsConfig: {
      label: runtimeStatus.dns_config_exists
        ? runtimeStatus.dns_config_valid
          ? '存在且有效'
          : '存在但无效'
        : '不存在',
      color:
        runtimeStatus.dns_config_exists && runtimeStatus.dns_config_valid
          ? 'success'
          : 'warning',
    } satisfies DnsStatusBadge,
    runtimeSource: runtimeStatus.enable_dns_settings
      ? '来自已保存 dns_config.yaml 派生配置'
      : '来自当前基础 runtime 配置',
    runtimeOverride: {
      label: runtimeStatus.enable_dns_settings ? '已启用' : '未启用',
      color: runtimeStatus.enable_dns_settings ? 'success' : 'warning',
    } satisfies DnsStatusBadge,
    runtimeEffect: {
      label: runtimeStatus.runtime_has_dns
        ? '运行态已携带 DNS'
        : '运行态未携带 DNS',
      color: runtimeStatus.runtime_has_dns ? 'success' : 'warning',
    } satisfies DnsStatusBadge,
    savedArtifact: {
      label: runtimeStatus.runtime_matches_saved ? '已生效' : '未完全生效',
      color: runtimeStatus.runtime_matches_saved ? 'success' : 'warning',
    } satisfies DnsStatusBadge,
    routing: {
      mode: routingMode,
      modeLabel: formatDnsRoutingModeLabel(routingMode),
      modeUnknownLabel: formatDnsRoutingModeLabel(routingMode, '未知'),
      modeColor: getDnsRoutingModeColor(routingMode),
      domesticDns: joinDnsList(derived.domestic_dns),
      foreignDns: joinDnsList(derived.foreign_dns),
      domesticDnsConfig: joinDnsList(derived.domestic_dns),
      foreignDnsConfig: joinDnsList(derived.foreign_dns),
      policyCount: snapshot.nameserver_policy_count,
      policyCountLabel: `${snapshot.nameserver_policy_count} 个策略组`,
    },
    leak: {
      level: leakLevel,
      levelLabel: formatDnsLeakProtectionLabel(leakLevel),
      levelUnknownLabel: formatDnsLeakProtectionLabel(leakLevel, '未知'),
      levelColor: getDnsLeakProtectionColor(leakLevel),
      security: leakSecurity,
      securityLabel: formatDnsLeakSecurityLabel(leakSecurity),
      securityUnknownLabel: formatDnsLeakSecurityLabel(leakSecurity, '未知'),
      securityColor: getDnsLeakSecurityColor(leakSecurity),
      safe: leakSafe,
      safeLabel: leakSafe === null ? '未知' : leakSafe ? '安全' : '不安全',
      safeColor: leakSafe === null ? 'default' : leakSafe ? 'success' : 'error',
      features: [
        snapshot.enhanced_mode === 'fake-ip' ? '启用 Fake-IP 模式' : null,
        derived.default_nameserver_plain_count === 0 ? '阻断明文 DNS' : null,
        snapshot.ipv6 === false ? '阻断 IPv6 DNS' : null,
        derived.prefer_h3 ? 'DoH 优先使用 HTTP/3' : null,
        snapshot.respect_rules ? '遵循运行时规则' : null,
      ].filter(Boolean) as string[],
    },
  }
}

export type DnsRuntimeViewModel = ReturnType<typeof buildDnsRuntimeViewModel>
