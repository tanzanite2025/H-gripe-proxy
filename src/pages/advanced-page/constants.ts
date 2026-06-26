export const ADVANCED_TAB_IDS = {
  security: 'security',
  securityPolicies: 'security-policies',
  localStealth: 'local-stealth',
  egressIdentity: 'egress-identity',
  sessionAffinity: 'session-affinity',
  appRuntime: 'app-runtime',
  egressMonitor: 'egress-monitor',
  residentialPool: 'residential-pool',
  ipReputation: 'ip-reputation',
  blackholeBreaker: 'blackhole-breaker',
  timezoneSpoof: 'timezone-spoof',
  multipath: 'multipath',
  performance: 'performance',
} as const

export type AdvancedTabId =
  (typeof ADVANCED_TAB_IDS)[keyof typeof ADVANCED_TAB_IDS]

export interface AdvancedTabDefinition {
  id: AdvancedTabId
  label: string
}

export const ADVANCED_TABS: AdvancedTabDefinition[] = [
  { id: ADVANCED_TAB_IDS.security, label: '安全防护' },
  { id: ADVANCED_TAB_IDS.securityPolicies, label: '安全策略' },
  { id: ADVANCED_TAB_IDS.localStealth, label: '本地隐匿' },
  { id: ADVANCED_TAB_IDS.egressIdentity, label: '出口身份' },
  { id: ADVANCED_TAB_IDS.sessionAffinity, label: '会话绑定' },
  { id: ADVANCED_TAB_IDS.appRuntime, label: '应用编排' },
  { id: ADVANCED_TAB_IDS.egressMonitor, label: '出口监控' },
  { id: ADVANCED_TAB_IDS.residentialPool, label: '住宅代理池' },
  { id: ADVANCED_TAB_IDS.ipReputation, label: 'IP 信誉' },
  { id: ADVANCED_TAB_IDS.blackholeBreaker, label: '黑洞熔断' },
  { id: ADVANCED_TAB_IDS.timezoneSpoof, label: '时区伪装' },
  { id: ADVANCED_TAB_IDS.multipath, label: '多路径路由' },
  { id: ADVANCED_TAB_IDS.performance, label: '性能监控' },
]
