import type { ResidentialProxy, ResidentialProxyType } from '@/services/coordinator'
import type { ResidentialProxyVerification } from '@/services/ip-reputation'

export const PROXY_TYPES: { value: ResidentialProxyType; label: string }[] = [
  { value: 'socks5', label: 'SOCKS5' },
  { value: 'http', label: 'HTTP' },
  { value: 'ss', label: 'Shadowsocks' },
  { value: 'vmess', label: 'VMess' },
  { value: 'trojan', label: 'Trojan' },
]

export const REGION_OPTIONS = [
  { value: '', label: '未指定' },
  { value: 'US', label: '美国' },
  { value: 'JP', label: '日本' },
  { value: 'SG', label: '新加坡' },
  { value: 'DE', label: '德国' },
  { value: 'GB', label: '英国' },
  { value: 'KR', label: '韩国' },
  { value: 'HK', label: '香港' },
  { value: 'TW', label: '台湾' },
  { value: 'AU', label: '澳大利亚' },
]

export function emptyProxy(): ResidentialProxy {
  return {
    name: '',
    proxyType: 'socks5',
    server: '',
    port: 1080,
    enabled: true,
  }
}

export function getVerificationLabel(
  verification: ResidentialProxyVerification,
): string {
  switch (verification.status) {
    case 'verified':
      return '已验证'
    case 'observed':
      return `观测为住宅 ${verification.reputation?.confidence ?? 0}`
    case 'rejected':
      return '非住宅'
    case 'needsMihomoProbe':
      return '待内核验证'
    default:
      return '验证失败'
  }
}

export function getVerificationColor(
  verification: ResidentialProxyVerification,
): 'success' | 'warning' | 'error' | 'default' | 'info' {
  switch (verification.status) {
    case 'verified':
      return 'success'
    case 'observed':
      return 'info'
    case 'rejected':
      return 'error'
    case 'needsMihomoProbe':
      return 'warning'
    default:
      return 'default'
  }
}
