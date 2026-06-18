export type LogLevel = 'debug' | 'info' | 'warning' | 'error' | 'silent'

export type ClashMode = 'rule' | 'global' | 'direct' | 'script'
export type FindProcessMode = 'always' | 'strict' | 'off'

export interface GeoXUrl {
  geoIp: string
  mmdb: string
  asn: string
  geoSite: string
}

export interface BaseConfig {
  port: number
  socksPort: number
  redirPort: number
  tproxyPort: number
  mixedPort: number
  tun: Record<string, unknown>
  tuicServer: Record<string, unknown>
  ssConfig: string
  vmessConfig: string
  authentication: string[] | null
  skipAuthPrefixes: string[] | null
  lanAllowedIps: string[] | null
  lanDisallowedIps: string[] | null
  allowLan: boolean
  bindAddress: string
  inboundTfo: boolean
  inboundMptcp: boolean
  mode: ClashMode
  unifiedDelay: boolean
  logLevel: LogLevel
  ipv6: boolean
  interfaceName: string
  routingMark: number
  geoxUrl: GeoXUrl
  geoAutoUpdate: boolean
  geoUpdateInterval: number
  geodataMode: boolean
  geodataLoader: string
  geositeMatcher: string
  tcpConcurrent: boolean
  findProcessMode: FindProcessMode
  sniffing: boolean
  globalClientFingerprint: string
  globalUa: string
  etagSupport: boolean
  keepAliveInterval: number
  keepAliveIdle: number
  disableKeepAlive: boolean
}

export interface MihomoVersion {
  meta: boolean
  version: string
}

export interface DelayHistory {
  time: string
  delay: number
}

export interface Proxy {
  all?: string[]
  expectedStatus?: string
  fixed?: string
  hidden?: boolean
  icon?: string
  now?: string
  testUrl?: string
  id?: string
  alive: boolean
  history: DelayHistory[]
  extra: Record<string, Record<string, unknown> | undefined>
  name: string
  udp: boolean
  uot: boolean
  type: string
  xudp: boolean
  tfo: boolean
  mptcp: boolean
  smux: boolean
  interface: string
  dialerProxy: string
  routingMark: number
}

export interface SubscriptionInfo {
  Upload: number
  Download: number
  Total: number
  Expire: number
}

export interface ProxyProvider {
  name: string
  type: string
  vehicleType: string
  proxies: Proxy[]
  testUrl: string
  expectedStatus: string
  updatedAt: string | null
  subscriptionInfo: SubscriptionInfo | null
}

export interface RuleExtra {
  disabled?: boolean
  deleted?: boolean
  hitCount?: number
  missCount?: number
  hitAt?: string
}

export interface Rule {
  index: number
  type: string
  payload: string
  proxy: string
  size: number
  source: string
  extra?: RuleExtra
}

export interface RuleProvider {
  behavior: string
  format: string
  name: string
  ruleCount: number
  type: string
  updatedAt: string
  vehicleType: string
}

export interface Rules {
  rules: Rule[]
  total?: number
  page?: number
  page_size?: number
}

export interface RuleProviders {
  providers: Record<string, RuleProvider | undefined>
}

export interface ProxyDelay {
  delay: number
}

export interface DnsMetrics {
  cache: {
    hit: number
    miss: number
    size: number
    hitRate: number
  }
  queries: {
    total: number
    success: number
    failed: number
    avgLatencyUs: number
    maxLatencyUs: number
  }
  servers: Array<{
    server: string
    queries: number
    successes: number
    failures: number
    avgLatencyUs: number
    lastQuery: string
    lastError?: string | null
  }>
  recent: Array<{
    domain: string
    qType: string
    server: string
    protocol: string
    proxyName?: string | null
    proxyChain?: string | null
    egress?: string | null
    rule?: string | null
    rulePayload?: string | null
    success: boolean
    error?: string | null
    latencyUs: number
    timestamp: string
  }>
  pollution: {
    totalChecked: number
    pollutedCount: number
    pollutionRate: number
    recentPolluted: Array<{
      domain: string
      ip: string
      reason: string
      timestamp: string
    }>
  }
  trust: {
    total: number
    encrypted: number
    unencrypted: number
    byTrustLevel: Record<string, number>
    servers: Array<{
      address: string
      protocol: string
      trustLevel: string
      encrypted: boolean
      description?: string | null
    }>
    leakRiskScore: number
    lastEvaluated: string
  }
}
