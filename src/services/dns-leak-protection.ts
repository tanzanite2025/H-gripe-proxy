/**
 * DNS 零泄漏防护服务
 * 确保所有 DNS 查询都通过加密通道，防止 DNS 泄漏
 */

import { testDnsLeak as testDnsLeakCommand } from './cmds'

/**
 * DNS 泄漏防护级别
 */
export type DnsLeakProtectionLevel = 'none' | 'basic' | 'strict' | 'paranoid'

/**
 * DNS 泄漏防护配置
 */
export interface DnsLeakProtectionConfig {
  level: DnsLeakProtectionLevel
  enableFakeIp: boolean // 启用 Fake-IP 模式
  blockPlainDns: boolean // 阻止明文 DNS
  blockSystemDns: boolean // 阻止系统 DNS
  blockIpv6Dns: boolean // 阻止 IPv6 DNS
  forceDoH: boolean // 强制使用 DoH
  forceDoT: boolean // 强制使用 DoT
  enableDnsLeakTest: boolean // 启用 DNS 泄漏测试
}

/**
 * DNS 泄漏测试结果
 */
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

/**
 * DNS 零泄漏防护预设
 */
const DNS_LEAK_PROTECTION_PRESETS: Record<
  DnsLeakProtectionLevel,
  Omit<DnsLeakProtectionConfig, 'level'>
> = {
  // 无防护
  none: {
    enableFakeIp: false,
    blockPlainDns: false,
    blockSystemDns: false,
    blockIpv6Dns: false,
    forceDoH: false,
    forceDoT: false,
    enableDnsLeakTest: false,
  },
  // 基础防护
  basic: {
    enableFakeIp: false,
    blockPlainDns: false,
    blockSystemDns: false,
    blockIpv6Dns: false,
    forceDoH: true,
    forceDoT: false,
    enableDnsLeakTest: true,
  },
  // 严格防护
  strict: {
    enableFakeIp: true,
    blockPlainDns: true,
    blockSystemDns: true,
    blockIpv6Dns: false,
    forceDoH: true,
    forceDoT: false,
    enableDnsLeakTest: true,
  },
  // 偏执防护（最强）
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

class DnsLeakProtectionService {
  private config: DnsLeakProtectionConfig = {
    level: 'basic',
    ...DNS_LEAK_PROTECTION_PRESETS.basic,
  }

  /**
   * 设置防护级别
   */
  setLevel(level: DnsLeakProtectionLevel): void {
    this.config = {
      level,
      ...DNS_LEAK_PROTECTION_PRESETS[level],
    }
    console.log(`DNS leak protection level set to: ${level}`)
  }

  /**
   * 设置自定义配置
   */
  setConfig(config: Partial<DnsLeakProtectionConfig>): void {
    this.config = { ...this.config, ...config }
    console.log('DNS leak protection config updated:', this.config)
  }

  /**
   * 获取当前配置
   */
  getConfig(): DnsLeakProtectionConfig {
    return { ...this.config }
  }

  /**
   * 生成 Clash DNS 配置（零泄漏）
   */
  generateClashDnsConfig(): Record<string, any> {
    const config: Record<string, any> = {
      enable: true,
      listen: '0.0.0.0:53',
      'use-hosts': true,
    }

    // Fake-IP 模式
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

    // IPv6 配置
    if (this.config.blockIpv6Dns) {
      config.ipv6 = false
    } else {
      config.ipv6 = true
    }

    // 不使用明文 DNS 作为 default-nameserver
    if (this.config.blockPlainDns) {
      config['default-nameserver'] = []
    } else {
      config['default-nameserver'] = ['223.5.5.5', '119.29.29.29']
    }

    // Nameserver 配置
    const nameservers: string[] = []
    if (this.config.forceDoH) {
      nameservers.push(
        'https://dns.alidns.com/dns-query',
        'https://doh.pub/dns-query',
        'https://dns.google/dns-query',
        'https://cloudflare-dns.com/dns-query',
      )
    } else if (this.config.forceDoT) {
      nameservers.push(
        'tls://dns.alidns.com:853',
        'tls://dns.google:853',
        'tls://1.1.1.1:853',
      )
    } else {
      nameservers.push('223.5.5.5', '119.29.29.29', '8.8.8.8', '1.1.1.1')
    }
    config.nameserver = nameservers

    // Fallback 配置
    const fallbacks: string[] = []
    if (this.config.forceDoH) {
      fallbacks.push(
        'https://dns.google/dns-query',
        'https://cloudflare-dns.com/dns-query',
        'https://dns.quad9.net/dns-query',
      )
    } else if (this.config.forceDoT) {
      fallbacks.push('tls://dns.google:853', 'tls://1.1.1.1:853', 'tls://9.9.9.9:853')
    } else {
      fallbacks.push('8.8.8.8', '1.1.1.1', '9.9.9.9')
    }
    config.fallback = fallbacks

    // Fallback 过滤器
    config['fallback-filter'] = {
      geoip: true,
      'geoip-code': 'CN',
      ipcidr: ['240.0.0.0/4', '0.0.0.0/32'],
      domain: ['+.google.com', '+.facebook.com', '+.youtube.com', '+.twitter.com', '+.github.com'],
    }

    // Nameserver 策略（分流）
    if (this.config.forceDoH) {
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

  /**
   * 验证 DNS 配置是否安全
   */
  validateDnsConfig(config: Record<string, any>): {
    safe: boolean
    issues: string[]
    recommendations: string[]
  } {
    const issues: string[] = []
    const recommendations: string[] = []

    // 检查是否启用 DNS
    if (!config.enable) {
      issues.push('DNS 未启用')
      recommendations.push('启用 DNS 以防止泄漏')
    }

    // 检查是否使用明文 DNS
    const nameservers = config.nameserver || []
    const hasPlainDns = nameservers.some(
      (ns: string) => !ns.startsWith('https://') && !ns.startsWith('tls://'),
    )
    if (hasPlainDns && this.config.blockPlainDns) {
      issues.push('使用了明文 DNS')
      recommendations.push('使用 DoH 或 DoT 加密 DNS')
    }

    // 检查 default-nameserver
    const defaultNameservers = config['default-nameserver'] || []
    if (defaultNameservers.length > 0 && this.config.blockPlainDns) {
      issues.push('default-nameserver 使用明文 DNS')
      recommendations.push('移除 default-nameserver 或使用加密 DNS')
    }

    // 检查 Fake-IP 模式
    if (!config['enhanced-mode'] || config['enhanced-mode'] !== 'fake-ip') {
      if (this.config.enableFakeIp) {
        issues.push('未启用 Fake-IP 模式')
        recommendations.push('启用 Fake-IP 模式以获得最强防护')
      }
    }

    // 检查 IPv6
    if (config.ipv6 !== false && this.config.blockIpv6Dns) {
      issues.push('IPv6 DNS 可能泄漏')
      recommendations.push('禁用 IPv6 或使用 IPv6 代理')
    }

    return {
      safe: issues.length === 0,
      issues,
      recommendations,
    }
  }

  /**
   * 执行 DNS 泄漏测试
   */
  async testDnsLeak(): Promise<DnsLeakTestResult> {
    try {
      const result = await testDnsLeakCommand()

      return {
        hasLeak: result.has_leak,
        leakType: result.leak_type,
        dnsServers: result.dns_servers.map((server) => ({
          ip: server.ip,
          location: [server.country, server.city].filter(Boolean).join(' · ') || 'Unknown',
          isp: server.isp || 'Unknown',
        })),
        recommendations: result.recommendations,
      }
    } catch (err) {
      console.error('DNS leak test failed:', err)
      throw err
    }
  }

  /**
   * 获取防护级别描述
   */
  getLevelDescription(level: DnsLeakProtectionLevel): {
    name: string
    description: string
    features: string[]
    security: 'low' | 'medium' | 'high' | 'maximum'
  } {
    const descriptions = {
      none: {
        name: '无防护',
        description: '不启用任何 DNS 泄漏防护',
        features: ['使用默认 DNS 配置', '可能存在 DNS 泄漏'],
        security: 'low' as const,
      },
      basic: {
        name: '基础防护',
        description: '使用 DoH 加密 DNS 查询',
        features: ['强制使用 DoH', 'DNS 泄漏测试', '基本隐私保护'],
        security: 'medium' as const,
      },
      strict: {
        name: '严格防护',
        description: '启用 Fake-IP 模式，阻止明文 DNS',
        features: [
          '启用 Fake-IP 模式',
          '阻止明文 DNS',
          '阻止系统 DNS',
          '强制使用 DoH',
          'DNS 泄漏测试',
        ],
        security: 'high' as const,
      },
      paranoid: {
        name: '偏执防护',
        description: '最强防护，阻止所有可能的 DNS 泄漏',
        features: [
          '启用 Fake-IP 模式',
          '阻止明文 DNS',
          '阻止系统 DNS',
          '阻止 IPv6 DNS',
          '强制使用 DoH 和 DoT',
          'DNS 泄漏测试',
        ],
        security: 'maximum' as const,
      },
    }

    return descriptions[level]
  }

  /**
   * 获取统计信息
   */
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

// 导出单例
export const dnsLeakProtectionService = new DnsLeakProtectionService()
