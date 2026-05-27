/**
 * WebRTC 泄漏检测服务
 * 检测 WebRTC 是否泄漏真实 IP 地址
 */

import { extractErrorMessage } from 'foxts/extract-error-message'

import { debugLog } from '@/utils/misc'

import { getIpInfo } from './api'

export interface WebRTCLeakResult {
  // 本地 IP 地址
  localIPs: string[]
  
  // 公网 IP 地址
  publicIPs: string[]
  
  // 当前代理 IP
  proxyIP: string
  
  // 泄漏状态
  isLeaking: boolean
  leakedIPs: string[]
  
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
 * 检测 WebRTC 泄漏
 * 
 * 原理：
 * 1. 使用 WebRTC API 创建 RTCPeerConnection
 * 2. 通过 STUN 服务器获取 ICE 候选
 * 3. 从 ICE 候选中提取 IP 地址
 * 4. 对比提取的 IP 与当前代理 IP
 * 5. 判断是否泄漏真实 IP
 */
export async function detectWebRTCLeak(): Promise<WebRTCLeakResult> {
  try {
    debugLog('[WebRTCLeak] 开始 WebRTC 泄漏检测')
    
    // 1. 获取当前代理 IP
    const ipInfo = await getIpInfo()
    const proxyIP = ipInfo.ip
    
    debugLog('[WebRTCLeak] 当前代理 IP:', proxyIP)
    
    // 2. 使用 WebRTC 获取本地和公网 IP
    const { localIPs, publicIPs } = await getWebRTCIPs()
    
    debugLog('[WebRTCLeak] 检测到的 IP:', { localIPs, publicIPs })
    
    // 3. 判断是否泄漏
    const leakedIPs: string[] = []
    let isLeaking = false
    
    // 检查公网 IP 是否与代理 IP 不同
    for (const ip of publicIPs) {
      if (ip !== proxyIP) {
        leakedIPs.push(ip)
        isLeaking = true
      }
    }
    
    // 4. 评估风险等级
    let riskLevel: WebRTCLeakResult['riskLevel'] = 'safe'
    if (isLeaking) {
      if (publicIPs.length > 0) {
        // 泄漏了公网 IP，风险高
        riskLevel = 'danger'
      } else if (localIPs.length > 0) {
        // 只泄漏了本地 IP，风险中等
        riskLevel = 'warning'
      }
    }
    
    // 5. 生成建议
    const recommendations = generateRecommendations(
      isLeaking,
      localIPs,
      publicIPs,
      leakedIPs
    )
    
    debugLog('[WebRTCLeak] 检测结果:', {
      isLeaking,
      leakedIPs,
      riskLevel,
    })
    
    return {
      localIPs,
      publicIPs,
      proxyIP,
      isLeaking,
      leakedIPs,
      riskLevel,
      recommendations,
      timestamp: Date.now(),
    }
  } catch (error) {
    debugLog('[WebRTCLeak] 检测失败:', error)
    
    return {
      localIPs: [],
      publicIPs: [],
      proxyIP: 'Unknown',
      isLeaking: false,
      leakedIPs: [],
      riskLevel: 'safe',
      recommendations: ['WebRTC 泄漏检测失败，请检查浏览器支持'],
      timestamp: Date.now(),
      error: extractErrorMessage(error) || 'WebRTC 泄漏检测失败',
    }
  }
}

/**
 * 使用 WebRTC API 获取 IP 地址
 */
async function getWebRTCIPs(): Promise<{
  localIPs: string[]
  publicIPs: string[]
}> {
  return new Promise((resolve, reject) => {
    const localIPs = new Set<string>()
    const publicIPs = new Set<string>()
    
    try {
      // 创建 RTCPeerConnection
      const pc = new RTCPeerConnection({
        iceServers: [
          // Google STUN 服务器
          { urls: 'stun:stun.l.google.com:19302' },
          { urls: 'stun:stun1.l.google.com:19302' },
          // Cloudflare STUN 服务器
          { urls: 'stun:stun.cloudflare.com:3478' },
        ],
      })
      
      // 创建数据通道（触发 ICE 收集）
      pc.createDataChannel('')
      
      // 创建 offer
      pc.createOffer()
        .then((offer) => pc.setLocalDescription(offer))
        .catch((error) => {
          debugLog('[WebRTCLeak] 创建 offer 失败:', error)
        })
      
      // 监听 ICE 候选
      pc.onicecandidate = (ice) => {
        if (!ice || !ice.candidate) {
          // ICE 收集完成
          pc.close()
          
          debugLog('[WebRTCLeak] ICE 收集完成:', {
            localIPs: Array.from(localIPs),
            publicIPs: Array.from(publicIPs),
          })
          
          resolve({
            localIPs: Array.from(localIPs),
            publicIPs: Array.from(publicIPs),
          })
          return
        }
        
        // 提取 IP 地址
        const candidate = ice.candidate.candidate
        debugLog('[WebRTCLeak] ICE 候选:', candidate)
        
        // 匹配 IPv4 地址
        const ipv4Match = /([0-9]{1,3}\.){3}[0-9]{1,3}/.exec(candidate)
        if (ipv4Match) {
          const ip = ipv4Match[0]
          
          // 判断是本地 IP 还是公网 IP
          if (isPrivateIP(ip)) {
            localIPs.add(ip)
          } else {
            publicIPs.add(ip)
          }
        }
        
        // 匹配 IPv6 地址（可选）
        const ipv6Match = /(([0-9a-fA-F]{1,4}:){7,7}[0-9a-fA-F]{1,4}|([0-9a-fA-F]{1,4}:){1,7}:|([0-9a-fA-F]{1,4}:){1,6}:[0-9a-fA-F]{1,4}|([0-9a-fA-F]{1,4}:){1,5}(:[0-9a-fA-F]{1,4}){1,2}|([0-9a-fA-F]{1,4}:){1,4}(:[0-9a-fA-F]{1,4}){1,3}|([0-9a-fA-F]{1,4}:){1,3}(:[0-9a-fA-F]{1,4}){1,4}|([0-9a-fA-F]{1,4}:){1,2}(:[0-9a-fA-F]{1,4}){1,5}|[0-9a-fA-F]{1,4}:((:[0-9a-fA-F]{1,4}){1,6})|:((:[0-9a-fA-F]{1,4}){1,7}|:)|fe80:(:[0-9a-fA-F]{0,4}){0,4}%[0-9a-zA-Z]{1,}|::(ffff(:0{1,4}){0,1}:){0,1}((25[0-5]|(2[0-4]|1{0,1}[0-9]){0,1}[0-9])\.){3,3}(25[0-5]|(2[0-4]|1{0,1}[0-9]){0,1}[0-9])|([0-9a-fA-F]{1,4}:){1,4}:((25[0-5]|(2[0-4]|1{0,1}[0-9]){0,1}[0-9])\.){3,3}(25[0-5]|(2[0-4]|1{0,1}[0-9]){0,1}[0-9]))/.exec(
          candidate
        )
        if (ipv6Match) {
          const ip = ipv6Match[0]
          if (isPrivateIPv6(ip)) {
            localIPs.add(ip)
          } else {
            publicIPs.add(ip)
          }
        }
      }
      
      // 设置超时（10秒）
      setTimeout(() => {
        pc.close()
        resolve({
          localIPs: Array.from(localIPs),
          publicIPs: Array.from(publicIPs),
        })
      }, 10000)
    } catch (error) {
      debugLog('[WebRTCLeak] WebRTC API 错误:', error)
      reject(error)
    }
  })
}

/**
 * 判断是否是私有 IP 地址
 */
function isPrivateIP(ip: string): boolean {
  const parts = ip.split('.').map(Number)
  
  // 10.0.0.0 - 10.255.255.255
  if (parts[0] === 10) return true
  
  // 172.16.0.0 - 172.31.255.255
  if (parts[0] === 172 && parts[1] >= 16 && parts[1] <= 31) return true
  
  // 192.168.0.0 - 192.168.255.255
  if (parts[0] === 192 && parts[1] === 168) return true
  
  // 127.0.0.0 - 127.255.255.255 (localhost)
  if (parts[0] === 127) return true
  
  // 169.254.0.0 - 169.254.255.255 (link-local)
  if (parts[0] === 169 && parts[1] === 254) return true
  
  return false
}

/**
 * 判断是否是私有 IPv6 地址
 */
function isPrivateIPv6(ip: string): boolean {
  // fe80::/10 (link-local)
  if (ip.toLowerCase().startsWith('fe80:')) return true
  
  // fc00::/7 (unique local)
  if (ip.toLowerCase().startsWith('fc') || ip.toLowerCase().startsWith('fd')) {
    return true
  }
  
  // ::1 (localhost)
  if (ip === '::1') return true
  
  return false
}

/**
 * 生成建议
 */
function generateRecommendations(
  isLeaking: boolean,
  localIPs: string[],
  publicIPs: string[],
  leakedIPs: string[]
): string[] {
  const recommendations: string[] = []
  
  if (!isLeaking) {
    recommendations.push('✅ WebRTC 未泄漏，您的真实 IP 是安全的')
    return recommendations
  }
  
  recommendations.push('⚠️ 检测到 WebRTC 泄漏')
  
  if (leakedIPs.length > 0) {
    recommendations.push(`泄漏的 IP: ${leakedIPs.join(', ')}`)
  }
  
  recommendations.push('')
  recommendations.push('建议修复方法：')
  
  if (publicIPs.length > 0) {
    recommendations.push('1. 在浏览器中禁用 WebRTC')
    recommendations.push('   • Chrome: 安装 WebRTC Leak Prevent 扩展')
    recommendations.push('   • Firefox: about:config 设置 media.peerconnection.enabled = false')
    recommendations.push('   • Edge: 安装 WebRTC Control 扩展')
    recommendations.push('2. 使用支持 WebRTC 保护的 VPN/代理')
    recommendations.push('3. 使用 Tor 浏览器（内置 WebRTC 保护）')
  } else if (localIPs.length > 0) {
    recommendations.push('1. 检测到本地 IP 泄漏（风险较低）')
    recommendations.push('2. 如需完全隐藏，可禁用 WebRTC')
    recommendations.push('3. 本地 IP 通常不会暴露真实位置')
  }
  
  return recommendations
}

/**
 * 获取 WebRTC 泄漏风险描述
 */
export function getWebRTCLeakRiskDescription(
  riskLevel: WebRTCLeakResult['riskLevel']
): {
  title: string
  description: string
  color: string
} {
  switch (riskLevel) {
    case 'safe':
      return {
        title: '✅ 安全',
        description: 'WebRTC 未泄漏，您的真实 IP 是安全的',
        color: 'text-success',
      }
    case 'warning':
      return {
        title: '⚠️ 警告',
        description: 'WebRTC 泄漏了本地 IP，风险较低',
        color: 'text-warning',
      }
    case 'danger':
      return {
        title: '🔴 危险',
        description: 'WebRTC 泄漏了公网 IP，您的真实 IP 可能暴露',
        color: 'text-error',
      }
  }
}

/**
 * 检查浏览器是否支持 WebRTC
 */
export function isWebRTCSupported(): boolean {
  return (
    typeof RTCPeerConnection !== 'undefined' ||
    typeof (window as any).webkitRTCPeerConnection !== 'undefined' ||
    typeof (window as any).mozRTCPeerConnection !== 'undefined'
  )
}
