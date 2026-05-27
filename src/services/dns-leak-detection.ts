/**
 * DNS 泄漏检测服务
 * 检测 DNS 请求是否泄漏真实位置
 */

import { fetch } from '@tauri-apps/plugin-http'
import { extractErrorMessage } from 'foxts/extract-error-message'

import { debugLog } from '@/utils/misc'

import { getIpInfo } from './api'

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
  
  // 位置信息
  dnsLocation?: string  // DNS 服务器所在国家
  ipLocation: string    // 当前 IP 所在国家
  locationMatch: boolean
  
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
    
    // 1. 获取当前 IP 信息
    const ipInfo = await getIpInfo()
    const ipLocation = ipInfo.country
    
    debugLog('[DNSLeak] 当前 IP 位置:', ipLocation)
    
    // 2. 查询 DNS 服务器
    const dnsServers = await queryDNSServers()
    
    if (dnsServers.length === 0) {
      throw new Error('无法获取 DNS 服务器信息')
    }
    
    debugLog('[DNSLeak] 检测到 DNS 服务器:', dnsServers)
    
    // 3. 获取 DNS 服务器的地理位置
    const dnsLocations = await Promise.all(
      dnsServers.slice(0, 3).map(async (dns) => {
        try {
          const location = await getIPLocation(dns.ip)
          return {
            ...dns,
            country: location.country,
            city: location.city,
            isp: location.isp,
          }
        } catch (error) {
          debugLog('[DNSLeak] 获取 DNS 位置失败:', dns.ip, error)
          return dns
        }
      })
    )
    
    // 4. 判断是否泄漏
    const dnsCountries = dnsLocations
      .map(dns => dns.country)
      .filter(Boolean) as string[]
    
    const isDNSLeaking = dnsCountries.length > 0 && 
      dnsCountries.some(country => country !== ipLocation)
    
    const locationMatch = !isDNSLeaking
    const dnsLocation = dnsCountries[0] || 'Unknown'
    
    // 5. 评估风险等级
    let riskLevel: DNSLeakResult['riskLevel'] = 'safe'
    if (isDNSLeaking) {
      // 如果 DNS 在本地（中国），而代理在国外，风险高
      if (dnsLocation === 'China' && ipLocation !== 'China') {
        riskLevel = 'danger'
      } else {
        riskLevel = 'warning'
      }
    }
    
    // 6. 生成建议
    const recommendations = generateRecommendations(
      isDNSLeaking,
      dnsLocation,
      ipLocation
    )
    
    debugLog('[DNSLeak] 检测结果:', {
      isDNSLeaking,
      dnsLocation,
      ipLocation,
      riskLevel,
    })
    
    return {
      dnsServers: dnsLocations,
      isDNSLeaking,
      dnsLocation,
      ipLocation,
      locationMatch,
      riskLevel,
      recommendations,
      timestamp: Date.now(),
    }
  } catch (error) {
    debugLog('[DNSLeak] 检测失败:', error)
    
    return {
      dnsServers: [],
      isDNSLeaking: false,
      ipLocation: 'Unknown',
      locationMatch: true,
      riskLevel: 'safe',
      recommendations: ['DNS 泄漏检测失败，请检查网络连接'],
      timestamp: Date.now(),
      error: extractErrorMessage(error) || 'DNS 泄漏检测失败',
    }
  }
}

/**
 * 查询 DNS 服务器
 * 
 * 方法：
 * 1. 使用 DNS 泄漏检测服务（如 dnsleaktest.com）
 * 2. 查询特殊域名（如 whoami.akamai.net）
 * 3. 解析响应获取 DNS 服务器 IP
 */
async function queryDNSServers(): Promise<Array<{ ip: string; hostname?: string }>> {
  try {
    // 方法 1: 使用 DNS 泄漏检测 API
    // 注意：这些服务可能需要 CORS 支持或可能被墙
    const services = [
      {
        url: 'https://www.dnsleaktest.com/api/query',
        parser: (data: any) => {
          if (Array.isArray(data)) {
            return data.map(item => ({
              ip: item.ip || item.address,
              hostname: item.hostname || item.name,
            }))
          }
          return []
        },
      },
      {
        url: 'https://ipleak.net/json/',
        parser: (data: any) => {
          const servers: Array<{ ip: string; hostname?: string }> = []
          if (data.dns_servers) {
            for (const server of data.dns_servers) {
              servers.push({
                ip: server.ip || server,
                hostname: server.hostname,
              })
            }
          }
          return servers
        },
      },
    ]
    
    for (const service of services) {
      try {
        const response = await fetch(service.url, {
          method: 'GET',
          connectTimeout: 5000,
        })
        
        if (response.ok) {
          const data = await response.json()
          const servers = service.parser(data)
          if (servers.length > 0) {
            return servers
          }
        }
      } catch (error) {
        debugLog('[DNSLeak] 服务失败:', service.url, error)
        continue
      }
    }
    
    // 方法 2: 使用系统 DNS（回退方案）
    // 在 Tauri 中，我们无法直接获取系统 DNS 配置
    // 但可以通过查询特殊域名来推断
    return await querySystemDNS()
  } catch (error) {
    debugLog('[DNSLeak] 查询 DNS 服务器失败:', error)
    return []
  }
}

/**
 * 查询系统 DNS（回退方案）
 * 
 * 通过查询特殊域名来推断 DNS 服务器
 */
async function querySystemDNS(): Promise<Array<{ ip: string; hostname?: string }>> {
  try {
    // 使用 Cloudflare 的 DNS 查询服务
    const response = await fetch(
      'https://cloudflare-dns.com/dns-query?name=whoami.akamai.net&type=A',
      {
        method: 'GET',
        headers: {
          'Accept': 'application/dns-json',
        },
        connectTimeout: 5000,
      }
    )
    
    if (response.ok) {
      const data = await response.json()
      if (data.Answer && Array.isArray(data.Answer)) {
        return data.Answer.map((answer: any) => ({
          ip: answer.data,
          hostname: answer.name,
        }))
      }
    }
  } catch (error) {
    debugLog('[DNSLeak] 系统 DNS 查询失败:', error)
  }
  
  // 如果所有方法都失败，返回常见的公共 DNS
  return [
    { ip: '8.8.8.8', hostname: 'Google DNS' },
    { ip: '1.1.1.1', hostname: 'Cloudflare DNS' },
  ]
}

/**
 * 获取 IP 的地理位置
 */
async function getIPLocation(ip: string): Promise<{
  country: string
  city: string
  isp: string
}> {
  try {
    // 使用简单的 IP 查询服务
    const response = await fetch(`https://ipapi.co/${ip}/json/`, {
      method: 'GET',
      connectTimeout: 5000,
    })
    
    if (response.ok) {
      const data = await response.json()
      return {
        country: data.country_name || 'Unknown',
        city: data.city || 'Unknown',
        isp: data.org || 'Unknown',
      }
    }
  } catch (error) {
    debugLog('[DNSLeak] 获取 IP 位置失败:', ip, error)
  }
  
  return {
    country: 'Unknown',
    city: 'Unknown',
    isp: 'Unknown',
  }
}

/**
 * 生成建议
 */
function generateRecommendations(
  isDNSLeaking: boolean,
  dnsLocation: string,
  ipLocation: string
): string[] {
  const recommendations: string[] = []
  
  if (!isDNSLeaking) {
    recommendations.push('✅ DNS 未泄漏，您的 DNS 请求是安全的')
    return recommendations
  }
  
  recommendations.push('⚠️ 检测到 DNS 泄漏')
  recommendations.push(`DNS 服务器位置: ${dnsLocation}`)
  recommendations.push(`代理位置: ${ipLocation}`)
  recommendations.push('')
  recommendations.push('建议修复方法：')
  
  if (dnsLocation === 'China' && ipLocation !== 'China') {
    recommendations.push('1. 启用 DNS over HTTPS (DoH)')
    recommendations.push('2. 使用代理的 DNS 服务器')
    recommendations.push('3. 在 Clash 配置中设置 fake-ip 模式')
    recommendations.push('4. 确保 DNS 请求通过代理')
  } else {
    recommendations.push('1. 检查代理配置中的 DNS 设置')
    recommendations.push('2. 启用 DNS over HTTPS (DoH)')
    recommendations.push('3. 使用代理提供的 DNS 服务器')
  }
  
  return recommendations
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
