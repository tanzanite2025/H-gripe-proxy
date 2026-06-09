import type { ClashMode } from '@/services/clash-mode'

export const PROXY_CHAIN_MODES = ['rule', 'global'] as const

export type ProxyChainMode = (typeof PROXY_CHAIN_MODES)[number]

export const PROXY_CHAIN_MODE_LABELS: Record<ProxyChainMode, string> = {
  rule: '应用规则',
  global: '单选节点',
}

export const PROXY_MODE_SECTION_TITLE = '主模式'

const PROXY_MODE_DESCRIPTIONS: Record<ProxyChainMode, string> = {
  rule:
    '按 Mihomo 规则命中对应链路，核心目标是不断链；下面的节点选择只作为规则命中后的出口承接。',
  global:
    '忽略规则，固定使用你手动选定的单一出口；不会因为延迟或策略自动漂移到别的节点。',
}

export const getProxyModeDescription = (mode: ClashMode | ProxyChainMode) =>
  PROXY_MODE_DESCRIPTIONS[mode === 'global' ? 'global' : 'rule']
