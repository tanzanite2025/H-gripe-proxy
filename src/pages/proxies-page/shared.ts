import type { ClashMode } from '@/services/clash-mode'

export const PROXY_CHAIN_MODES = ['rule', 'global'] as const

export type ProxyChainMode = (typeof PROXY_CHAIN_MODES)[number]

export const PROXY_CHAIN_MODE_LABELS: Record<ProxyChainMode, string> = {
  rule: '应用规则',
  global: '统一出口',
}

export const PROXY_MODE_SECTION_TITLE = '出口模式'

const PROXY_MODE_DESCRIPTIONS: Record<ProxyChainMode, string> = {
  rule:
    '先按 Mihomo 规则命中分组，再落到你在下方选中的出口。选中策略池时，规则命中后仍由该池自动挑选成员。',
  global:
    '忽略规则，所有流量统一走你在下方选中的出口。这里的出口可以是单节点，也可以是策略池。',
}

export const getProxyModeDescription = (mode: ClashMode | ProxyChainMode) =>
  PROXY_MODE_DESCRIPTIONS[mode === 'global' ? 'global' : 'rule']
