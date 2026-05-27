/**
 * 代理检测服务
 * 检测代理是否生效，对比直连和代理的 IP 信息
 */

import { extractErrorMessage } from 'foxts/extract-error-message'

import { debugLog } from '@/utils/misc'

import { getIpInfo } from './api'

export interface ProxyDetectionResult {
  // 代理状态
  isProxyWorking: boolean
  
  // IP 信息
  directIP?: string
  proxyIP: string
  ipChanged: boolean
  
  // 地理位置信息
  directLocation?: {
    country: string
    country_code: string
    city: string
    region: string
  }
  proxyLocation: {
    country: string
    country_code: string
    city: string
    region: string
  }
  locationChanged: boolean
  
  // 检测时间
  timestamp: number
  
  // 错误信息（如果检测失败）
  error?: string
}

/**
 * 检测代理是否生效
 * 
 * 原理：
 * 1. 获取当前 IP 信息（通过代理）
 * 2. 尝试获取直连 IP 信息（如果可能）
 * 3. 对比两者的 IP 地址和地理位置
 * 
 * 注意：
 * - 在 Tauri 应用中，所有网络请求都会经过系统代理设置
 * - 无法直接获取"真实"直连 IP（除非通过特殊方法）
 * - 因此我们主要依赖用户手动记录的直连 IP，或者检测 IP 是否在常见代理服务器范围
 */
export async function detectProxy(): Promise<ProxyDetectionResult> {
  try {
    debugLog('[ProxyDetection] 开始代理检测')
    
    // 1. 获取当前 IP 信息（通过代理）
    const proxyInfo = await getIpInfo()
    
    debugLog('[ProxyDetection] 当前 IP 信息:', {
      ip: proxyInfo.ip,
      country: proxyInfo.country,
      city: proxyInfo.city,
    })
    
    // 2. 尝试从缓存获取直连 IP（用户之前记录的）
    const directInfo = getStoredDirectIP()
    
    // 3. 判断代理是否生效
    let isProxyWorking = false
    let ipChanged = false
    let locationChanged = false
    
    if (directInfo) {
      // 如果有直连 IP 记录，进行对比
      ipChanged = directInfo.ip !== proxyInfo.ip
      locationChanged = directInfo.country_code !== proxyInfo.country_code
      isProxyWorking = ipChanged || locationChanged
      
      debugLog('[ProxyDetection] 对比结果:', {
        directIP: directInfo.ip,
        proxyIP: proxyInfo.ip,
        ipChanged,
        locationChanged,
        isProxyWorking,
      })
    } else {
      // 如果没有直连 IP 记录，通过启发式方法判断
      // 检查是否是常见的代理服务器特征
      isProxyWorking = detectProxyHeuristics(proxyInfo)
      
      debugLog('[ProxyDetection] 启发式检测结果:', {
        isProxyWorking,
        reason: isProxyWorking ? '检测到代理特征' : '未检测到明显代理特征',
      })
    }
    
    return {
      isProxyWorking,
      directIP: directInfo?.ip,
      proxyIP: proxyInfo.ip,
      ipChanged,
      directLocation: directInfo ? {
        country: directInfo.country,
        country_code: directInfo.country_code,
        city: directInfo.city,
        region: directInfo.region,
      } : undefined,
      proxyLocation: {
        country: proxyInfo.country,
        country_code: proxyInfo.country_code,
        city: proxyInfo.city,
        region: proxyInfo.region,
      },
      locationChanged,
      timestamp: Date.now(),
    }
  } catch (error) {
    debugLog('[ProxyDetection] 检测失败:', error)
    
    // 即使检测失败，也返回基本信息
    return {
      isProxyWorking: false,
      proxyIP: 'Unknown',
      ipChanged: false,
      proxyLocation: {
        country: 'Unknown',
        country_code: 'XX',
        city: 'Unknown',
        region: 'Unknown',
      },
      locationChanged: false,
      timestamp: Date.now(),
      error: extractErrorMessage(error) || '代理检测失败',
    }
  }
}

/**
 * 启发式检测代理特征
 * 
 * 检测方法：
 * 1. 检查 ASN 是否属于常见的 VPS/云服务商
 * 2. 检查 ISP 名称是否包含代理相关关键词
 * 3. 检查 IP 是否在常见的代理 IP 段
 */
function detectProxyHeuristics(ipInfo: any): boolean {
  // 常见的 VPS/云服务商 ASN
  const commonVPSASNs = [
    13335, // Cloudflare
    15169, // Google Cloud
    16509, // Amazon AWS
    8075,  // Microsoft Azure
    14061, // DigitalOcean
    20473, // Choopa (Vultr)
    63949, // Linode
    // 添加更多...
  ]
  
  // 常见的代理/VPN 关键词
  const proxyKeywords = [
    'vpn',
    'proxy',
    'tunnel',
    'relay',
    'anonymous',
    'privacy',
    'secure',
    'cloud',
    'hosting',
    'datacenter',
    'server',
  ]
  
  // 1. 检查 ASN
  if (ipInfo.asn && commonVPSASNs.includes(ipInfo.asn)) {
    debugLog('[ProxyDetection] 检测到常见 VPS ASN:', ipInfo.asn)
    return true
  }
  
  // 2. 检查 ISP 名称
  const orgLower = (ipInfo.organization || '').toLowerCase()
  const asnOrgLower = (ipInfo.asn_organization || '').toLowerCase()
  
  for (const keyword of proxyKeywords) {
    if (orgLower.includes(keyword) || asnOrgLower.includes(keyword)) {
      debugLog('[ProxyDetection] 检测到代理关键词:', keyword)
      return true
    }
  }
  
  // 3. 如果都没检测到，返回 false（可能是直连）
  return false
}

/**
 * 保存直连 IP 信息
 * 用户可以在未启用代理时手动保存直连 IP，用于后续对比
 */
export function saveDirectIP(ipInfo: {
  ip: string
  country: string
  country_code: string
  city: string
  region: string
}): void {
  try {
    localStorage.setItem('clash-verge-direct-ip', JSON.stringify({
      ...ipInfo,
      timestamp: Date.now(),
    }))
    debugLog('[ProxyDetection] 直连 IP 已保存:', ipInfo.ip)
  } catch (error) {
    console.error('[ProxyDetection] 保存直连 IP 失败:', error)
  }
}

/**
 * 获取存储的直连 IP 信息
 */
function getStoredDirectIP(): {
  ip: string
  country: string
  country_code: string
  city: string
  region: string
  timestamp: number
} | null {
  try {
    const stored = localStorage.getItem('clash-verge-direct-ip')
    if (!stored) return null
    
    const data = JSON.parse(stored)
    
    // 检查是否过期（30天）
    const age = Date.now() - data.timestamp
    const maxAge = 30 * 24 * 60 * 60 * 1000 // 30天
    
    if (age > maxAge) {
      debugLog('[ProxyDetection] 直连 IP 记录已过期')
      localStorage.removeItem('clash-verge-direct-ip')
      return null
    }
    
    return data
  } catch (error) {
    console.error('[ProxyDetection] 读取直连 IP 失败:', error)
    return null
  }
}

/**
 * 清除存储的直连 IP 信息
 */
export function clearDirectIP(): void {
  try {
    localStorage.removeItem('clash-verge-direct-ip')
    debugLog('[ProxyDetection] 直连 IP 记录已清除')
  } catch (error) {
    console.error('[ProxyDetection] 清除直连 IP 失败:', error)
  }
}

/**
 * 获取代理检测建议
 */
export function getProxyDetectionAdvice(result: ProxyDetectionResult): string[] {
  const advice: string[] = []
  
  if (result.error) {
    advice.push('代理检测失败，请检查网络连接')
    return advice
  }
  
  if (!result.directIP) {
    advice.push('未记录直连 IP，建议在关闭代理时保存直连 IP 以便对比')
  }
  
  if (result.isProxyWorking) {
    if (result.ipChanged) {
      advice.push('✅ IP 地址已改变，代理正常工作')
    }
    if (result.locationChanged) {
      advice.push('✅ 地理位置已改变，代理正常工作')
    }
  } else {
    advice.push('⚠️ 未检测到明显的代理特征')
    advice.push('可能原因：')
    advice.push('  • 代理未启用')
    advice.push('  • 代理配置错误')
    advice.push('  • 使用了本地代理（IP 未改变）')
  }
  
  return advice
}
