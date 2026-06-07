import type { TranslationKey } from '@/types/generated/i18n-keys'

export const builtinProxyPolicies = ['DIRECT', 'REJECT', 'REJECT-DROP', 'PASS']

export const proxyStrategyOptions = [
  'select',
  'url-test',
  'fallback',
  'load-balance',
  'relay',
] as const

export const excludeTypeOptions = [
  'Direct',
  'Reject',
  'RejectDrop',
  'Compatible',
  'Pass',
  'Dns',
  'Shadowsocks',
  'ShadowsocksR',
  'Snell',
  'Socks5',
  'Http',
  'Vmess',
  'Vless',
  'Trojan',
  'Hysteria',
  'Hysteria2',
  'WireGuard',
  'Tuic',
  'Mieru',
  'Masque',
  'AnyTLS',
  'Sudoku',
  'Relay',
  'Selector',
  'Fallback',
  'URLTest',
  'LoadBalance',
  'Ssh',
]

export const PROXY_STRATEGY_LABEL_KEYS: Record<string, TranslationKey> = {
  select: 'proxies.components.enums.strategies.select',
  'url-test': 'proxies.components.enums.strategies.url-test',
  fallback: 'proxies.components.enums.strategies.fallback',
  'load-balance': 'proxies.components.enums.strategies.load-balance',
  relay: 'proxies.components.enums.strategies.relay',
}

export const PROXY_POLICY_LABEL_KEYS: Record<string, TranslationKey> =
  builtinProxyPolicies.reduce(
    (acc, policy) => {
      acc[policy] =
        `proxies.components.enums.policies.${policy}` as TranslationKey
      return acc
    },
    {} as Record<string, TranslationKey>,
  )
