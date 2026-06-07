export const FINGERPRINT_CATEGORY_LABELS: Record<string, string> = {
  browser: '浏览器',
  mobile: '移动端',
  random: '随机',
  classic: '经典',
}

export const DEFAULT_FINGERPRINT_CATEGORY_ORDER = [
  'browser',
  'mobile',
  'random',
  'classic',
]

export const SNIFFING_TYPES = ['TLS', 'HTTP', 'QUIC'] as const

export const OBFUSCATION_LEVEL_OPTIONS = [
  { value: 'low', label: '低' },
  { value: 'medium', label: '中' },
  { value: 'high', label: '高' },
  { value: 'paranoid', label: '偏执' },
] as const
