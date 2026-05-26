/**
 * DNS 配置工具函数
 * 包含解析、格式化、默认配置等
 */

type NameserverPolicy = Record<string, any>

/**
 * 解析域名服务器策略字符串
 * 格式: "domain1=server1;server2, domain2=server3"
 */
export function parseNameserverPolicy(str: string): NameserverPolicy {
  const result: NameserverPolicy = {}
  if (!str) return result

  const ruleRegex = /\s*([^=]+?)\s*=\s*([^,]+)(?:,|$)/g
  let match: RegExpExecArray | null

  while ((match = ruleRegex.exec(str)) !== null) {
    const [, domainsPart, serversPart] = match

    const domains = [domainsPart.trim()]
    const servers = serversPart.split(';').map((s) => s.trim())

    domains.forEach((domain) => {
      result[domain] = servers
    })
  }

  return result
}

/**
 * 格式化域名服务器策略对象为字符串
 */
export function formatNameserverPolicy(policy: unknown): string {
  if (!policy || typeof policy !== 'object') return ''

  return Object.entries(policy as Record<string, unknown>)
    .map(([domain, servers]) => {
      const serversStr = Array.isArray(servers) ? servers.join(';') : servers
      return `${domain}=${serversStr}`
    })
    .join(', ')
}

/**
 * 格式化 Hosts 对象为字符串
 */
export function formatHosts(hosts: unknown): string {
  if (!hosts || typeof hosts !== 'object') return ''

  const result: string[] = []

  Object.entries(hosts as Record<string, unknown>).forEach(
    ([domain, value]) => {
      if (Array.isArray(value)) {
        const ipsStr = value.join(';')
        result.push(`${domain}=${ipsStr}`)
      } else {
        result.push(`${domain}=${value}`)
      }
    },
  )

  return result.join(', ')
}

/**
 * 解析 Hosts 字符串为对象
 * 格式: "domain1=ip1, domain2=ip2;ip3"
 */
export function parseHosts(str: string): NameserverPolicy {
  const result: NameserverPolicy = {}
  if (!str) return result

  str.split(',').forEach((item) => {
    const parts = item.trim().split('=')
    if (parts.length < 2) return

    const domain = parts[0].trim()
    const valueStr = parts.slice(1).join('=').trim()

    if (valueStr.includes(';')) {
      result[domain] = valueStr
        .split(';')
        .map((s) => s.trim())
        .filter(Boolean)
    } else {
      result[domain] = valueStr
    }
  })

  return result
}

/**
 * 解析逗号分隔的列表字符串
 */
export function parseList(str: string): string[] {
  if (!str?.trim()) return []
  return str
    .split(',')
    .map((item) => item.trim())
    .filter(Boolean)
}

/**
 * 默认 DNS 配置（已优化网络稳定性）
 * 
 * 优化点：
 * 1. 多层 DNS 备份（UDP → DoH → DoT → Fallback）
 * 2. 国内 DNS 优先（降低延迟）
 * 3. 加密 DNS（DoH/DoT）防止 DNS 污染
 * 4. 智能分流（不同域名使用不同 DNS）
 * 5. 完善的 fallback 机制
 */
export const DEFAULT_DNS_CONFIG = {
  enable: true,
  listen: ':53',
  'enhanced-mode': 'fake-ip' as 'fake-ip' | 'redir-host',
  'fake-ip-range': '198.18.0.1/16',
  'fake-ip-filter-mode': 'blacklist' as 'blacklist' | 'whitelist',
  'prefer-h3': false,
  'respect-rules': false,
  'use-hosts': false,
  'use-system-hosts': false,
  ipv6: true,
  'fake-ip-filter': [
    '*.lan',
    '*.local',
    '*.arpa',
    'time.*.com',
    'ntp.*.com',
    '+.market.xiaomi.com',
    'localhost.ptlogin2.qq.com',
    '*.msftncsi.com',
    'www.msftconnecttest.com',
  ],
  // 默认域名服务器（用于解析 DNS 服务器的域名）
  // 优先使用国内快速 DNS
  'default-nameserver': [
    '223.5.5.5', // 阿里 DNS（国内最快）
    '119.29.29.29', // DNSPod（国内稳定）
    '114.114.114.114', // 114 DNS（备用）
    '8.8.8.8', // Google DNS（国际备用）
  ],
  // 主域名服务器（多层备份策略）
  nameserver: [
    // 第一层：国内快速 DNS（UDP，延迟最低 10-30ms）
    '223.5.5.5', // 阿里 DNS
    '119.29.29.29', // DNSPod
    // 第二层：国内 DoH（加密，防污染，延迟 30-50ms）
    'https://dns.alidns.com/dns-query',
    'https://doh.pub/dns-query',
  ],
  // 回退域名服务器（主服务器失败时使用）
  fallback: [
    // 国际 DoH（防污染，延迟 100-300ms）
    'https://dns.google/dns-query',
    'https://cloudflare-dns.com/dns-query',
    // 国际 DoT（备用）
    'tls://dns.google',
  ],
  // 域名服务器策略（针对不同域名使用不同 DNS）
  'nameserver-policy': {
    // 国内域名使用国内 DNS（低延迟）
    'geosite:cn': ['223.5.5.5', '119.29.29.29'],
    // Google 服务使用 Google DNS
    '+.google.com': ['https://dns.google/dns-query'],
    '+.googleapis.com': ['https://dns.google/dns-query'],
    // GitHub 使用国际 DNS
    '+.github.com': ['https://dns.google/dns-query', 'https://cloudflare-dns.com/dns-query'],
    '+.githubusercontent.com': ['https://dns.google/dns-query'],
  },
  // 代理服务器域名解析（用于解析代理服务器地址）
  'proxy-server-nameserver': [
    'https://dns.alidns.com/dns-query',
    'https://doh.pub/dns-query',
    'tls://dns.alidns.com',
  ],
  // 直连域名服务器（用于直连域名）
  'direct-nameserver': [
    '223.5.5.5',
    '119.29.29.29',
    'https://dns.alidns.com/dns-query',
  ],
  'direct-nameserver-follow-policy': false,
  // 回退过滤器（判断是否使用 fallback）
  'fallback-filter': {
    geoip: true,
    'geoip-code': 'CN',
    ipcidr: [
      '240.0.0.0/4', // 保留地址
      '0.0.0.0/32', // 无效地址
      '127.0.0.1/8', // 本地回环
    ],
    domain: [
      '+.google.com',
      '+.googleapis.com',
      '+.facebook.com',
      '+.youtube.com',
      '+.github.com',
      '+.githubusercontent.com',
      '+.twitter.com',
    ],
  },
}

/**
 * DNS 表单值类型
 */
export interface DnsFormValues {
  enable: boolean
  listen: string
  enhancedMode: 'fake-ip' | 'redir-host'
  fakeIpRange: string
  fakeIpFilterMode: 'blacklist' | 'whitelist'
  preferH3: boolean
  respectRules: boolean
  useHosts: boolean
  useSystemHosts: boolean
  ipv6: boolean
  fakeIpFilter: string
  nameserver: string
  fallback: string
  defaultNameserver: string
  proxyServerNameserver: string
  directNameserver: string
  directNameserverFollowPolicy: boolean
  fallbackGeoip: boolean
  fallbackGeoipCode: string
  fallbackIpcidr: string
  fallbackDomain: string
  nameserverPolicy: string
  hosts: string
}

/**
 * 获取默认表单值
 */
export function getDefaultFormValues(): DnsFormValues {
  return {
    enable: DEFAULT_DNS_CONFIG.enable,
    listen: DEFAULT_DNS_CONFIG.listen,
    enhancedMode: DEFAULT_DNS_CONFIG['enhanced-mode'],
    fakeIpRange: DEFAULT_DNS_CONFIG['fake-ip-range'],
    fakeIpFilterMode: DEFAULT_DNS_CONFIG['fake-ip-filter-mode'],
    preferH3: DEFAULT_DNS_CONFIG['prefer-h3'],
    respectRules: DEFAULT_DNS_CONFIG['respect-rules'],
    useHosts: DEFAULT_DNS_CONFIG['use-hosts'],
    useSystemHosts: DEFAULT_DNS_CONFIG['use-system-hosts'],
    ipv6: DEFAULT_DNS_CONFIG.ipv6,
    fakeIpFilter: DEFAULT_DNS_CONFIG['fake-ip-filter'].join(', '),
    defaultNameserver: DEFAULT_DNS_CONFIG['default-nameserver'].join(', '),
    nameserver: DEFAULT_DNS_CONFIG.nameserver.join(', '),
    fallback: DEFAULT_DNS_CONFIG.fallback.join(', '),
    proxyServerNameserver:
      DEFAULT_DNS_CONFIG['proxy-server-nameserver']?.join(', ') || '',
    directNameserver: DEFAULT_DNS_CONFIG['direct-nameserver']?.join(', ') || '',
    directNameserverFollowPolicy:
      DEFAULT_DNS_CONFIG['direct-nameserver-follow-policy'] || false,
    fallbackGeoip: DEFAULT_DNS_CONFIG['fallback-filter'].geoip,
    fallbackGeoipCode: DEFAULT_DNS_CONFIG['fallback-filter']['geoip-code'],
    fallbackIpcidr:
      DEFAULT_DNS_CONFIG['fallback-filter'].ipcidr?.join(', ') || '',
    fallbackDomain:
      DEFAULT_DNS_CONFIG['fallback-filter'].domain?.join(', ') || '',
    nameserverPolicy: '',
    hosts: '',
  }
}

/**
 * 从配置对象生成表单值
 */
export function configToFormValues(config: any): DnsFormValues {
  if (!config) return getDefaultFormValues()

  const dnsConfig = config.dns || {}
  const hostsConfig = config.hosts || {}

  const enhancedMode =
    dnsConfig['enhanced-mode'] || DEFAULT_DNS_CONFIG['enhanced-mode']
  const validEnhancedMode =
    enhancedMode === 'fake-ip' || enhancedMode === 'redir-host'
      ? enhancedMode
      : DEFAULT_DNS_CONFIG['enhanced-mode']

  const fakeIpFilterMode =
    dnsConfig['fake-ip-filter-mode'] ||
    DEFAULT_DNS_CONFIG['fake-ip-filter-mode']
  const validFakeIpFilterMode =
    fakeIpFilterMode === 'blacklist' || fakeIpFilterMode === 'whitelist'
      ? fakeIpFilterMode
      : DEFAULT_DNS_CONFIG['fake-ip-filter-mode']

  return {
    enable: dnsConfig.enable ?? DEFAULT_DNS_CONFIG.enable,
    listen: dnsConfig.listen ?? DEFAULT_DNS_CONFIG.listen,
    enhancedMode: validEnhancedMode,
    fakeIpRange:
      dnsConfig['fake-ip-range'] ?? DEFAULT_DNS_CONFIG['fake-ip-range'],
    fakeIpFilterMode: validFakeIpFilterMode,
    preferH3: dnsConfig['prefer-h3'] ?? DEFAULT_DNS_CONFIG['prefer-h3'],
    respectRules:
      dnsConfig['respect-rules'] ?? DEFAULT_DNS_CONFIG['respect-rules'],
    useHosts: dnsConfig['use-hosts'] ?? DEFAULT_DNS_CONFIG['use-hosts'],
    useSystemHosts:
      dnsConfig['use-system-hosts'] ?? DEFAULT_DNS_CONFIG['use-system-hosts'],
    ipv6: dnsConfig.ipv6 ?? DEFAULT_DNS_CONFIG.ipv6,
    fakeIpFilter:
      dnsConfig['fake-ip-filter']?.join(', ') ??
      DEFAULT_DNS_CONFIG['fake-ip-filter'].join(', '),
    nameserver:
      dnsConfig.nameserver?.join(', ') ??
      DEFAULT_DNS_CONFIG.nameserver.join(', '),
    fallback:
      dnsConfig.fallback?.join(', ') ?? DEFAULT_DNS_CONFIG.fallback.join(', '),
    defaultNameserver:
      dnsConfig['default-nameserver']?.join(', ') ??
      DEFAULT_DNS_CONFIG['default-nameserver'].join(', '),
    proxyServerNameserver:
      dnsConfig['proxy-server-nameserver']?.join(', ') ??
      (DEFAULT_DNS_CONFIG['proxy-server-nameserver']?.join(', ') || ''),
    directNameserver:
      dnsConfig['direct-nameserver']?.join(', ') ??
      (DEFAULT_DNS_CONFIG['direct-nameserver']?.join(', ') || ''),
    directNameserverFollowPolicy:
      dnsConfig['direct-nameserver-follow-policy'] ??
      DEFAULT_DNS_CONFIG['direct-nameserver-follow-policy'],
    fallbackGeoip:
      dnsConfig['fallback-filter']?.geoip ??
      DEFAULT_DNS_CONFIG['fallback-filter'].geoip,
    fallbackGeoipCode:
      dnsConfig['fallback-filter']?.['geoip-code'] ??
      DEFAULT_DNS_CONFIG['fallback-filter']['geoip-code'],
    fallbackIpcidr:
      dnsConfig['fallback-filter']?.ipcidr?.join(', ') ??
      DEFAULT_DNS_CONFIG['fallback-filter'].ipcidr.join(', '),
    fallbackDomain:
      dnsConfig['fallback-filter']?.domain?.join(', ') ??
      DEFAULT_DNS_CONFIG['fallback-filter'].domain.join(', '),
    nameserverPolicy:
      formatNameserverPolicy(dnsConfig['nameserver-policy']) || '',
    hosts: formatHosts(hostsConfig) || '',
  }
}

/**
 * 从表单值生成配置对象
 */
export function formValuesToConfig(values: DnsFormValues): Record<string, any> {
  const config: Record<string, any> = {}

  // 生成 DNS 配置
  const dnsConfig: any = {
    enable: values.enable,
    listen: values.listen,
    'enhanced-mode': values.enhancedMode,
    'fake-ip-range': values.fakeIpRange,
    'fake-ip-filter-mode': values.fakeIpFilterMode,
    'prefer-h3': values.preferH3,
    'respect-rules': values.respectRules,
    'use-hosts': values.useHosts,
    'use-system-hosts': values.useSystemHosts,
    ipv6: values.ipv6,
    'fake-ip-filter': parseList(values.fakeIpFilter),
    'default-nameserver': parseList(values.defaultNameserver),
    nameserver: parseList(values.nameserver),
    'direct-nameserver-follow-policy': values.directNameserverFollowPolicy,
    'fallback-filter': {
      geoip: values.fallbackGeoip,
      'geoip-code': values.fallbackGeoipCode,
      ipcidr: parseList(values.fallbackIpcidr),
      domain: parseList(values.fallbackDomain),
    },
    fallback: parseList(values.fallback),
    'proxy-server-nameserver': parseList(values.proxyServerNameserver),
    'direct-nameserver': parseList(values.directNameserver),
  }

  const policy = parseNameserverPolicy(values.nameserverPolicy)
  if (Object.keys(policy).length > 0) {
    dnsConfig['nameserver-policy'] = policy
  }

  if (Object.keys(dnsConfig).length > 0) {
    config.dns = dnsConfig
  }

  // 生成 Hosts 配置
  const hosts = parseHosts(values.hosts)
  if (Object.keys(hosts).length > 0) {
    config.hosts = hosts
  }

  return config
}
