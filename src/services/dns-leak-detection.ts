import { extractErrorMessage } from 'foxts/extract-error-message'

import { debugLog } from '@/utils/misc'

import { testDnsLeak } from './cmds'

export interface DNSLeakResult {
  // DNS 服务器信息
  dnsServers: Array<{
    ip: string
    hostname?: string
    country?: string
    city?: string
    isp?: string
  }>
  
  // 泄漏状态
  isDNSLeaking: boolean
  observedLeak: boolean
  runtimeRiskDetected: boolean
  observationIncomplete: boolean
  assessment: 'safe' | 'observed-leak' | 'runtime-risk' | 'inconclusive' | string
  confidence: 'high' | 'medium' | 'low' | string
  warnings: string[]
  observedLeakType: string[]
  runtimeRiskType: string[]
  
  // 位置信息
  dnsLocation?: string  // DNS 服务器所在国家
  ipLocation: string    // 当前 IP 所在国家
  locationMatch: boolean
  locationComparable: boolean
  
  // 风险等级
  riskLevel: 'safe' | 'warning' | 'danger'
  
  // 建议
  recommendations: string[]
  
  // 检测时间
  timestamp: number
  
  // 错误信息
  error?: string
}

/**
 * 检测 DNS 泄漏
 * 
 * 原理：
 * 1. 查询特殊域名，获取 DNS 服务器 IP
 * 2. 获取当前 IP 的地理位置
 * 3. 查询 DNS 服务器的地理位置
 * 4. 对比两者是否一致
 * 
 * 如果 DNS 服务器位置与代理位置不一致，说明 DNS 泄漏
 */
export async function detectDNSLeak(): Promise<DNSLeakResult> {
  try {
    debugLog('[DNSLeak] 开始 DNS 泄漏检测')
    const result = await testDnsLeak()
    
    debugLog('[DNSLeak] 检测结果:', {
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
      warnings: result.warnings,
      observedLeakType: result.observed_leak_type,
      runtimeRiskType: result.runtime_risk_type,
      dnsLocation: result.dns_location ?? 'Unknown',
      ipLocation: result.ip_location,
      locationMatch: result.location_match,
      locationComparable: result.location_comparable,
      riskLevel:
        result.risk_level === 'danger'
          ? 'danger'
          : result.risk_level === 'warning'
            ? 'warning'
            : 'safe',
      recommendations: result.recommendations,
      timestamp: result.timestamp,
      error: result.error ?? undefined,
    }
  } catch (error) {
    debugLog('[DNSLeak] 检测失败:', error)
    
    return {
      dnsServers: [],
      isDNSLeaking: false,
      observedLeak: false,
      runtimeRiskDetected: false,
      observationIncomplete: true,
      assessment: 'inconclusive',
      confidence: 'low',
      warnings: ['DNS 泄漏检测失败，未能获取完整外部观测'],
      observedLeakType: [],
      runtimeRiskType: [],
      ipLocation: 'Unknown',
      locationMatch: true,
      locationComparable: false,
      riskLevel: 'safe',
      recommendations: ['DNS 泄漏检测失败，请检查网络连接'],
      timestamp: Date.now(),
      error: extractErrorMessage(error) || 'DNS 泄漏检测失败',
    }
  }
}

/**
 * 获取 DNS 泄漏风险描述
 */
export function getDNSLeakRiskDescription(riskLevel: DNSLeakResult['riskLevel']): {
  title: string
  description: string
  color: string
} {
  switch (riskLevel) {
    case 'safe':
      return {
        title: '✅ 安全',
        description: 'DNS 未泄漏，您的 DNS 请求是安全的',
        color: 'text-success',
      }
    case 'warning':
      return {
        title: '⚠️ 警告',
        description: 'DNS 可能泄漏，建议检查配置',
        color: 'text-warning',
      }
    case 'danger':
      return {
        title: '🔴 危险',
        description: 'DNS 严重泄漏，您的真实位置可能暴露',
        color: 'text-error',
      }
  }
}
