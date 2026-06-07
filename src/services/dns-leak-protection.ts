import { testDnsLeak as testDnsLeakCommand } from './cmds'

export type DnsLeakProtectionLevel = 'none' | 'basic' | 'strict' | 'paranoid'

export interface DnsLeakProtectionConfig {
  level: DnsLeakProtectionLevel
  enableFakeIp: boolean
  blockPlainDns: boolean
  blockSystemDns: boolean
  blockIpv6Dns: boolean
  forceDoH: boolean
  forceDoT: boolean
  enableDnsLeakTest: boolean
}

export interface DnsLeakTestResult {
  hasLeak: boolean
  leakType: string[]
  dnsServers: Array<{
    ip: string
    location: string
    isp: string
  }>
  recommendations: string[]
}

const DNS_LEAK_PROTECTION_PRESETS: Record<
  DnsLeakProtectionLevel,
  Omit<DnsLeakProtectionConfig, 'level'>
> = {
  none: {
    enableFakeIp: false,
    blockPlainDns: false,
    blockSystemDns: false,
    blockIpv6Dns: false,
    forceDoH: false,
    forceDoT: false,
    enableDnsLeakTest: false,
  },
  basic: {
    enableFakeIp: false,
    blockPlainDns: false,
    blockSystemDns: false,
    blockIpv6Dns: false,
    forceDoH: true,
    forceDoT: false,
    enableDnsLeakTest: true,
  },
  strict: {
    enableFakeIp: true,
    blockPlainDns: true,
    blockSystemDns: true,
    blockIpv6Dns: false,
    forceDoH: true,
    forceDoT: false,
    enableDnsLeakTest: true,
  },
  paranoid: {
    enableFakeIp: true,
    blockPlainDns: true,
    blockSystemDns: true,
    blockIpv6Dns: true,
    forceDoH: true,
    forceDoT: true,
    enableDnsLeakTest: true,
  },
}

const PLAIN_NAMESERVERS = ['223.5.5.5', '119.29.29.29', '8.8.8.8', '1.1.1.1']
const DOH_NAMESERVERS = [
  'https://dns.alidns.com/dns-query',
  'https://doh.pub/dns-query',
  'https://dns.google/dns-query',
  'https://cloudflare-dns.com/dns-query',
]
const DOT_NAMESERVERS = [
  'tls://dns.alidns.com:853',
  'tls://dns.google:853',
  'tls://1.1.1.1:853',
]
const PLAIN_FALLBACKS = ['8.8.8.8', '1.1.1.1', '9.9.9.9']
const DOH_FALLBACKS = [
  'https://dns.google/dns-query',
  'https://cloudflare-dns.com/dns-query',
  'https://dns.quad9.net/dns-query',
]
const DOT_FALLBACKS = ['tls://dns.google:853', 'tls://1.1.1.1:853', 'tls://9.9.9.9:853']

class DnsLeakProtectionService {
  private config: DnsLeakProtectionConfig = {
    level: 'basic',
    ...DNS_LEAK_PROTECTION_PRESETS.basic,
  }

  setLevel(level: DnsLeakProtectionLevel): void {
    this.config = {
      level,
      ...DNS_LEAK_PROTECTION_PRESETS[level],
    }
    console.log(`DNS leak protection level set to: ${level}`)
  }

  setConfig(config: Partial<DnsLeakProtectionConfig>): void {
    this.config = { ...this.config, ...config }
    console.log('DNS leak protection config updated:', this.config)
  }

  getConfig(): DnsLeakProtectionConfig {
    return { ...this.config }
  }

  generateClashDnsConfig(): Record<string, any> {
    const config: Record<string, any> = {
      enable: true,
      listen: '0.0.0.0:53',
      'use-hosts': true,
    }

    if (this.config.enableFakeIp) {
      config['enhanced-mode'] = 'fake-ip'
      config['fake-ip-range'] = '198.18.0.1/16'
      config['fake-ip-filter'] = [
        '*.lan',
        'localhost.ptlogin2.qq.com',
        '+.stun.*.*',
        '+.stun.*.*.*',
        '+.stun.*.*.*.*',
        '+.stun.*.*.*.*.*',
        '*.n.n.srv.nintendo.net',
        '+.stun.playstation.net',
        'xbox.*.*.microsoft.com',
        '*.*.xboxlive.com',
        '*.msftncsi.com',
        '*.msftconnecttest.com',
        'WORKGROUP',
      ]
    } else {
      config['enhanced-mode'] = 'redir-host'
    }

    config.ipv6 = !this.config.blockIpv6Dns
    config['default-nameserver'] = this.config.blockPlainDns
      ? []
      : ['223.5.5.5', '119.29.29.29']

    const preferDoH = this.config.forceDoH
    const preferDoT = !preferDoH && this.config.forceDoT

    config.nameserver = preferDoH
      ? DOH_NAMESERVERS
      : preferDoT
        ? DOT_NAMESERVERS
        : PLAIN_NAMESERVERS

    config.fallback = preferDoH
      ? DOH_FALLBACKS
      : preferDoT
        ? DOT_FALLBACKS
        : PLAIN_FALLBACKS

    config['fallback-filter'] = {
      geoip: true,
      'geoip-code': 'CN',
      ipcidr: ['240.0.0.0/4', '0.0.0.0/32'],
      domain: [
        '+.google.com',
        '+.facebook.com',
        '+.youtube.com',
        '+.twitter.com',
        '+.github.com',
      ],
    }

    if (preferDoH) {
      config['nameserver-policy'] = {
        'geosite:cn': ['https://dns.alidns.com/dns-query', 'https://doh.pub/dns-query'],
        'geosite:geolocation-!cn': [
          'https://dns.google/dns-query',
          'https://cloudflare-dns.com/dns-query',
        ],
      }
    }

    return config
  }

  validateDnsConfig(config: Record<string, any>): {
    safe: boolean
    issues: string[]
    recommendations: string[]
  } {
    const issues: string[] = []
    const recommendations: string[] = []

    if (!config.enable) {
      issues.push('DNS 未启用')
      recommendations.push('启用 DNS 以减少泄漏风险')
    }

    const nameservers = Array.isArray(config.nameserver) ? config.nameserver : []
    const hasPlainDns = nameservers.some(
      (ns: string) => !ns.startsWith('https://') && !ns.startsWith('tls://'),
    )
    if (hasPlainDns && this.config.blockPlainDns) {
      issues.push('仍在使用明文 DNS')
      recommendations.push('改用 DoH 或 DoT 加密 DNS')
    }

    const defaultNameservers = Array.isArray(config['default-nameserver'])
      ? config['default-nameserver']
      : []
    if (defaultNameservers.length > 0 && this.config.blockPlainDns) {
      issues.push('default-nameserver 仍使用明文 DNS')
      recommendations.push('移除 default-nameserver 或替换为加密 DNS')
    }

    if (this.config.enableFakeIp && config['enhanced-mode'] !== 'fake-ip') {
      issues.push('未启用 Fake-IP 模式')
      recommendations.push('启用 Fake-IP 模式以获得更完整的防护')
    }

    if (config.ipv6 !== false && this.config.blockIpv6Dns) {
      issues.push('IPv6 DNS 可能泄漏')
      recommendations.push('禁用 IPv6 或确保 IPv6 也经过代理')
    }

    return {
      safe: issues.length === 0,
      issues,
      recommendations,
    }
  }

  async testDnsLeak(): Promise<DnsLeakTestResult> {
    try {
      const result = await testDnsLeakCommand()

      return {
        hasLeak: result.has_leak,
        leakType: result.leak_type,
        dnsServers: result.dns_servers.map((server) => ({
          ip: server.ip,
          location: [server.country, server.city].filter(Boolean).join(' / ') || '未知',
          isp: server.isp || '未知',
        })),
        recommendations: result.recommendations,
      }
    } catch (error) {
      console.error('DNS leak test failed:', error)
      throw error
    }
  }

  getLevelDescription(level: DnsLeakProtectionLevel): {
    name: string
    description: string
    features: string[]
    security: 'low' | 'medium' | 'high' | 'maximum'
  } {
    const descriptions = {
      none: {
        name: '无防护',
        description: '不主动阻断 DNS 泄漏，仅适合临时排障。',
        features: ['使用默认 DNS 行为', '可能存在 DNS 泄漏'],
        security: 'low' as const,
      },
      basic: {
        name: '基础防护',
        description: '优先使用 DoH，在兼容性和防护之间做平衡。',
        features: ['优先使用 DoH', '支持 DNS 泄漏测试', '保留基础兼容性'],
        security: 'medium' as const,
      },
      strict: {
        name: '严格防护',
        description: '启用 Fake-IP 并阻断明文 DNS，适合长期日常使用。',
        features: [
          '启用 Fake-IP 模式',
          '阻断明文 DNS',
          '阻断系统 DNS',
          '强制优先使用 DoH',
          '支持 DNS 泄漏测试',
        ],
        security: 'high' as const,
      },
      paranoid: {
        name: '偏执防护',
        description: '同时收紧 IPv6 DNS 和系统 DNS，安全性最高。',
        features: [
          '启用 Fake-IP 模式',
          '阻断明文 DNS',
          '阻断系统 DNS',
          '阻断 IPv6 DNS',
          '优先使用 DoH，保留 DoT 意图配置',
          '支持 DNS 泄漏测试',
        ],
        security: 'maximum' as const,
      },
    }

    return descriptions[level]
  }

  getStats(): {
    level: DnsLeakProtectionLevel
    levelName: string
    security: string
    features: string[]
    safe: boolean
  } {
    const description = this.getLevelDescription(this.config.level)
    return {
      level: this.config.level,
      levelName: description.name,
      security: description.security,
      features: description.features,
      safe: this.config.level !== 'none',
    }
  }
}

export const dnsLeakProtectionService = new DnsLeakProtectionService()
