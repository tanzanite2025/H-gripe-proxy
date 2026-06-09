import type { TranslationKey } from '@/types/generated/i18n-keys'

export const proxyStrategyOptions = [
  'select',
  'url-test',
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
  'load-balance': 'proxies.components.enums.strategies.load-balance',
  relay: 'proxies.components.enums.strategies.relay',
}
