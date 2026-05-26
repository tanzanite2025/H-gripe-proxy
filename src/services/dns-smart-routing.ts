/**
 * DNS 智能分流服务
 * 根据域名类型自动选择最优 DNS 服务器和协议
 */

import type { DnsProtocol, DnsQueryOptions } from './dns-api'

/**
 * DNS 分流模式
 */
export type DnsRoutingMode = 'speed' | 'privacy' | 'balanced' | 'custom'

/**
 * DNS 分流配置
 */
export interface DnsRoutingConfig {
  mode: DnsRoutingMode
  // 国内 DNS 配置
  domesticDns: {
    server: string
    protocol: DnsProtocol
  }
  // 国外 DNS 配置
  foreignDns: {
    server: string
    protocol: DnsProtocol
  }
  // 自定义域名规则
  customRules?: Array<{
    pattern: string | RegExp
    server: string
    protocol: DnsProtocol
  }>
}

/**
 * 预定义的 DNS 配置
 */
const DNS_PRESETS = {
  // 速度优先：全部使用国内 UDP DNS
  speed: {
    domesticDns: { server: '223.5.5.5', protocol: 'udp' as DnsProtocol },
    foreignDns: { server: '223.5.5.5', protocol: 'udp' as DnsProtocol },
  },
  // 隐私优先：全部使用 DoH
  privacy: {
    domesticDns: { server: '1.1.1.1', protocol: 'doh' as DnsProtocol },
    foreignDns: { server: '1.1.1.1', protocol: 'doh' as DnsProtocol },
  },
  // 平衡模式：国内 UDP，国外 DoH
  balanced: {
    domesticDns: { server: '223.5.5.5', protocol: 'udp' as DnsProtocol },
    foreignDns: { server: '1.1.1.1', protocol: 'doh' as DnsProtocol },
  },
}

/**
 * 国内域名后缀列表
 */
const DOMESTIC_SUFFIXES = [
  '.cn',
  '.com.cn',
  '.net.cn',
  '.org.cn',
  '.gov.cn',
  '.edu.cn',
  '.mil.cn',
  '.ac.cn',
]

/**
 * 国内常见域名列表
 */
const DOMESTIC_DOMAINS = [
  'baidu.com',
  'taobao.com',
  'tmall.com',
  'qq.com',
  'weibo.com',
  'jd.com',
  'sina.com',
  'sohu.com',
  '163.com',
  '126.com',
  'aliyun.com',
  'tencent.com',
  'bilibili.com',
  'douyin.com',
  'zhihu.com',
  'csdn.net',
  'cnblogs.com',
  'gitee.com',
]

class DnsSmartRoutingService {
  private config: DnsRoutingConfig = {
    mode: 'balanced',
    ...DNS_PRESETS.balanced,
  }

  /**
   * 设置分流模式
   */
  setMode(mode: DnsRoutingMode): void {
    this.config.mode = mode

    if (mode !== 'custom') {
      const preset = DNS_PRESETS[mode]
      this.config.domesticDns = preset.domesticDns
      this.config.foreignDns = preset.foreignDns
    }

    console.log(`DNS routing mode set to: ${mode}`)
  }

  /**
   * 设置自定义配置
   */
  setCustomConfig(config: Partial<DnsRoutingConfig>): void {
    this.config = {
      ...this.config,
      ...config,
      mode: 'custom',
    }
    console.log('DNS routing custom config updated:', this.config)
  }

  /**
   * 获取当前配置
   */
  getConfig(): DnsRoutingConfig {
    return { ...this.config }
  }

  /**
   * 判断是否为国内域名
   */
  isDomesticDomain(domain: string): boolean {
    const lowerDomain = domain.toLowerCase()

    // 检查域名后缀
    if (DOMESTIC_SUFFIXES.some((suffix) => lowerDomain.endsWith(suffix))) {
      return true
    }

    // 检查常见国内域名
    if (DOMESTIC_DOMAINS.some((d) => lowerDomain.includes(d))) {
      return true
    }

    return false
  }

  /**
   * 根据域名选择 DNS 配置
   */
  selectDnsConfig(domain: string): DnsQueryOptions {
    // 检查自定义规则
    if (this.config.customRules) {
      for (const rule of this.config.customRules) {
        if (typeof rule.pattern === 'string') {
          if (domain.includes(rule.pattern)) {
            return {
              server: rule.server,
              protocol: rule.protocol,
            }
          }
        } else if (rule.pattern instanceof RegExp) {
          if (rule.pattern.test(domain)) {
            return {
              server: rule.server,
              protocol: rule.protocol,
            }
          }
        }
      }
    }

    // 根据域名类型选择配置
    const isDomestic = this.isDomesticDomain(domain)
    const dnsConfig = isDomestic ? this.config.domesticDns : this.config.foreignDns

    return {
      server: dnsConfig.server,
      protocol: dnsConfig.protocol,
    }
  }

  /**
   * 添加自定义规则
   */
  addCustomRule(pattern: string | RegExp, server: string, protocol: DnsProtocol): void {
    if (!this.config.customRules) {
      this.config.customRules = []
    }

    this.config.customRules.push({ pattern, server, protocol })
    console.log(`DNS routing rule added: ${pattern} -> ${server} (${protocol})`)
  }

  /**
   * 移除自定义规则
   */
  removeCustomRule(pattern: string | RegExp): void {
    if (!this.config.customRules) return

    const patternStr = pattern instanceof RegExp ? pattern.source : pattern
    this.config.customRules = this.config.customRules.filter((rule) => {
      const rulePatternStr = rule.pattern instanceof RegExp ? rule.pattern.source : rule.pattern
      return rulePatternStr !== patternStr
    })

    console.log(`DNS routing rule removed: ${pattern}`)
  }

  /**
   * 清空自定义规则
   */
  clearCustomRules(): void {
    this.config.customRules = []
    console.log('DNS routing custom rules cleared')
  }

  /**
   * 获取所有自定义规则
   */
  getCustomRules(): Array<{
    pattern: string | RegExp
    server: string
    protocol: DnsProtocol
  }> {
    return this.config.customRules || []
  }

  /**
   * 获取统计信息
   */
  getStats(): {
    mode: DnsRoutingMode
    domesticDns: string
    foreignDns: string
    customRulesCount: number
  } {
    return {
      mode: this.config.mode,
      domesticDns: `${this.config.domesticDns.server} (${this.config.domesticDns.protocol})`,
      foreignDns: `${this.config.foreignDns.server} (${this.config.foreignDns.protocol})`,
      customRulesCount: this.config.customRules?.length || 0,
    }
  }
}

// 导出单例
export const dnsSmartRoutingService = new DnsSmartRoutingService()

// 导出类型
export type { DnsQueryOptions }
