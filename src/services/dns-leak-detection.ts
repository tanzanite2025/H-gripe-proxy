import { extractErrorMessage } from 'foxts/extract-error-message'

import { debugLog } from '@/utils/misc'

import { testDnsLeak, type DnsMetrics } from './cmds'

export interface DNSLeakResult {
  dnsServers: Array<{
    ip: string
    hostname?: string
    country?: string
    city?: string
    isp?: string
  }>
  isDNSLeaking: boolean
  observedLeak: boolean
  runtimeRiskDetected: boolean
  observationIncomplete: boolean
  assessment: 'safe' | 'observed-leak' | 'runtime-risk' | 'inconclusive' | string
  confidence: 'high' | 'medium' | 'low' | string
  warnings: string[]
  observedLeakType: string[]
  runtimeRiskType: string[]
  dnsMetrics?: DnsMetrics | null
  dnsLocation?: string
  ipLocation: string
  locationMatch: boolean
  locationComparable: boolean
  riskLevel: 'safe' | 'warning' | 'danger'
  recommendations: string[]
  timestamp: number
  error?: string
}

const normalizeStringArray = (value: unknown) =>
  Array.isArray(value) ? value.filter((item): item is string => typeof item === 'string') : []

const normalizeRiskLevel = (
  riskLevel: string | null | undefined,
): DNSLeakResult['riskLevel'] => {
  switch (riskLevel) {
    case 'danger':
      return 'danger'
    case 'warning':
      return 'warning'
    default:
      return 'safe'
  }
}

export async function detectDNSLeak(): Promise<DNSLeakResult> {
  try {
    debugLog('[DNSLeak] 开始 DNS 泄漏检测')
    const result = await testDnsLeak()

    debugLog('[DNSLeak] 检测结果', {
      hasLeak: result.has_leak,
      dnsLocation: result.dns_location,
      ipLocation: result.ip_location,
      riskLevel: result.risk_level,
    })

    return {
      dnsServers: result.dns_servers.map((server) => ({
        ip: server.ip,
        hostname: server.hostname ?? undefined,
        country: server.country ?? undefined,
        city: server.city ?? undefined,
        isp: server.isp ?? undefined,
      })),
      isDNSLeaking: result.has_leak,
      observedLeak: result.observed_leak,
      runtimeRiskDetected: result.runtime_risk_detected,
      observationIncomplete: result.observation_incomplete,
      assessment: result.assessment,
      confidence: result.confidence,
      warnings: normalizeStringArray(result.warnings),
      observedLeakType: normalizeStringArray(result.observed_leak_type).map(
        formatDNSLeakSignal,
      ),
      runtimeRiskType: normalizeStringArray(result.runtime_risk_type).map(
        formatDNSRuntimeRisk,
      ),
      dnsMetrics: result.dns_metrics,
      dnsLocation: result.dns_location ?? '未知',
      ipLocation: result.ip_location || '未知',
      locationMatch: result.location_match,
      locationComparable: result.location_comparable,
      riskLevel: normalizeRiskLevel(result.risk_level),
      recommendations: normalizeStringArray(result.recommendations),
      timestamp: result.timestamp,
      error: result.error ?? undefined,
    }
  } catch (error) {
    debugLog('[DNSLeak] 检测失败', error)

    return {
      dnsServers: [],
      isDNSLeaking: false,
      observedLeak: false,
      runtimeRiskDetected: false,
      observationIncomplete: true,
      assessment: 'inconclusive',
      confidence: 'low',
      warnings: ['DNS 泄漏检测失败，未能完成外部观测。'],
      observedLeakType: [],
      runtimeRiskType: [],
      ipLocation: '未知',
      locationMatch: false,
      locationComparable: false,
      riskLevel: 'warning',
      recommendations: ['请检查网络连接、代理状态和内核运行状态后重试。'],
      timestamp: Date.now(),
      error: extractErrorMessage(error) || 'DNS 泄漏检测失败',
    }
  }
}

export function getDNSLeakRiskDescription(
  riskLevel: DNSLeakResult['riskLevel'],
): {
  title: string
  description: string
  color: string
} {
  switch (riskLevel) {
    case 'safe':
      return {
        title: '安全',
        description: '未观察到 DNS 泄漏，当前解析路径看起来安全。',
        color: 'text-success',
      }
    case 'warning':
      return {
        title: '注意',
        description: '存在运行态风险或观测不完整，建议检查 DNS 配置。',
        color: 'text-warning',
      }
    case 'danger':
      return {
        title: '危险',
        description: '已观察到 DNS 泄漏，真实出口位置可能暴露。',
        color: 'text-error',
      }
  }
}

export function formatDNSRuntimeRisk(type: string): string {
  switch (type) {
    case 'plain-dns-bootstrap':
      return '存在明文 DNS bootstrap，可能绕过加密解析链路'
    case 'dns-protection-insufficient':
      return '当前 DNS 防护级别不足，建议切换到严格或偏执模式'
    case 'system-hosts-enabled':
      return '仍启用 system hosts，可能绕过 Clash DNS 规则'
    case 'runtime-dns-not-synced':
      return '运行时 DNS 配置尚未同步到当前内核'
    case 'core-dns-unencrypted-server':
      return '本地内核报告正在使用未加密 DNS 服务器'
    case 'core-dns-high-risk-score':
      return '本地内核 DNS trust risk score 偏高'
    case 'core-dns-polluted-response':
      return '本地内核检测到 DNS 污染响应'
    case 'core-dns-high-failure-rate':
      return '本地内核 DNS 查询失败率偏高'
    default:
      return type
  }
}

export function formatDNSLeakSignal(type: string): string {
  switch (type) {
    case 'dns-location-mismatch':
      return 'DNS 出口位置与代理出口位置不一致'
    default:
      return type
  }
}
