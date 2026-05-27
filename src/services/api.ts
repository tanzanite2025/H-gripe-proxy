import { getName, getVersion } from '@tauri-apps/api/app'
import { fetch } from '@tauri-apps/plugin-http'
import { asyncRetry } from 'foxts/async-retry'
import { extractErrorMessage } from 'foxts/extract-error-message'
import { once } from 'foxts/once'

import { debugLog } from '@/utils/misc'

import { getIpCheckConfig } from './adaptive-config'
import {
  getCachedIpInfo,
  setCachedIpInfo,
  clearIpCache,
} from './ip-cache'
import { networkMonitor } from './network-monitor'
import { deduplicator } from './request-deduplicator'

const getUserAgentPromise = once(async () => {
  try {
    const [name, version] = await Promise.all([getName(), getVersion()])
    return `${name}/${version}`
  } catch (error) {
    console.debug('Failed to build User-Agent, fallback to default', error)
    return 'clash-verge-optimized'
  }
})
// Get current IP and geolocation information （refactored IP detection with service-specific mappings）
interface IpInfo {
  ip: string
  country_code: string
  country: string
  region: string
  city: string
  organization: string
  asn: number
  asn_organization: string
  longitude: number
  latitude: number
  timezone: string
}

// IP检测服务配置
interface ServiceConfig {
  url: string
  mapping: (data: any) => IpInfo
  timeout?: number // 保留timeout字段（如有需要）
}

// 可用的IP检测服务列表及字段映射
// 包含国内和国际服务，随机打乱顺序以实现负载均衡和故障转移
const IP_CHECK_SERVICES: ServiceConfig[] = [
  // 国内服务 - 优先级高，国内用户访问快
  {
    url: 'https://myip.ipip.net/json',
    mapping: (data) => ({
      ip: data.ip || '',
      country_code: data.country_code || '',
      country: data.data?.country || data.country || '',
      region: data.data?.province || data.region || '',
      city: data.data?.city || data.city || '',
      organization: data.data?.isp || data.isp || '',
      asn: data.data?.asn || data.asn || 0,
      asn_organization: data.data?.isp || data.isp || '',
      longitude: data.data?.longitude || 0,
      latitude: data.data?.latitude || 0,
      timezone: data.data?.timezone || data.timezone || '',
    }),
  },
  {
    url: 'https://api.vore.top/api/IPdata',
    mapping: (data) => ({
      ip: data.ip || data.ipip || '',
      country_code: data.adcode?.country || data.country_code || '',
      country: data.ipip_country || data.country || '',
      region: data.ipip_province || data.province || data.region || '',
      city: data.ipip_city || data.city || '',
      organization: data.isp || data.org || '',
      asn: data.asn || 0,
      asn_organization: data.isp || data.org || '',
      longitude: Number(data.ipip_longitude || data.longitude) || 0,
      latitude: Number(data.ipip_latitude || data.latitude) || 0,
      timezone: data.timezone || '',
    }),
  },
  {
    url: 'https://api.ip.sb/geoip',
    mapping: (data) => ({
      ip: data.ip || '',
      country_code: data.country_code || '',
      country: data.country || '',
      region: data.region || '',
      city: data.city || '',
      organization: data.organization || data.isp || '',
      asn: data.asn || 0,
      asn_organization: data.asn_organization || '',
      longitude: data.longitude || 0,
      latitude: data.latitude || 0,
      timezone: data.timezone || '',
    }),
  },
  // 国际服务 - 备用，全球覆盖好
  {
    url: 'https://ipapi.co/json',
    mapping: (data) => ({
      ip: data.ip || '',
      country_code: data.country_code || '',
      country: data.country_name || '',
      region: data.region || '',
      city: data.city || '',
      organization: data.org || '',
      asn: data.asn ? parseInt(data.asn.replace('AS', '')) : 0,
      asn_organization: data.org || '',
      longitude: data.longitude || 0,
      latitude: data.latitude || 0,
      timezone: data.timezone || '',
    }),
  },
  {
    url: 'https://api.ipapi.is/',
    mapping: (data) => ({
      ip: data.ip || '',
      country_code: data.location?.country_code || '',
      country: data.location?.country || '',
      region: data.location?.state || '',
      city: data.location?.city || '',
      organization: data.asn?.org || data.company?.name || '',
      asn: data.asn?.asn || 0,
      asn_organization: data.asn?.org || '',
      longitude: data.location?.longitude || 0,
      latitude: data.location?.latitude || 0,
      timezone: data.location?.timezone || '',
    }),
  },
  {
    url: 'https://ipwho.is/',
    mapping: (data) => ({
      ip: data.ip || '',
      country_code: data.country_code || '',
      country: data.country || '',
      region: data.region || '',
      city: data.city || '',
      organization: data.connection?.org || data.connection?.isp || '',
      asn: data.connection?.asn || 0,
      asn_organization: data.connection?.isp || '',
      longitude: data.longitude || 0,
      latitude: data.latitude || 0,
      timezone: data.timezone?.id || '',
    }),
  },
  {
    url: 'https://ip.api.skk.moe/cf-geoip',
    mapping: (data) => ({
      ip: data.ip || '',
      country_code: data.country || '',
      country: data.country || '',
      region: data.region || '',
      city: data.city || '',
      organization: data.asOrg || '',
      asn: data.asn || 0,
      asn_organization: data.asOrg || '',
      longitude: data.longitude || 0,
      latitude: data.latitude || 0,
      timezone: data.timezone || '',
    }),
  },
  {
    url: 'https://get.geojs.io/v1/ip/geo.json',
    mapping: (data) => ({
      ip: data.ip || '',
      country_code: data.country_code || '',
      country: data.country || '',
      region: data.region || '',
      city: data.city || '',
      organization: data.organization_name || '',
      asn: data.asn || 0,
      asn_organization: data.organization_name || '',
      longitude: Number(data.longitude) || 0,
      latitude: Number(data.latitude) || 0,
      timezone: data.timezone || '',
    }),
  },
]

// 获取当前IP和地理位置信息
export const getIpInfo = async (): Promise<
  IpInfo & { lastFetchTs: number }
> => {
  // 使用请求去重
  return deduplicator.dedupe('ip-info', async () => {
    // 先尝试从缓存获取
    const cached = getCachedIpInfo()
    if (cached) {
      console.debug('[IpInfo] 使用缓存的IP信息')
      return cached
    }

    // 检查网络状态
    if (!networkMonitor.isOnline()) {
      throw new Error('网络已断开，无法获取IP信息')
    }

    // 根据网络质量获取配置
    const config = getIpCheckConfig()
    if (config.timeout === 0) {
      throw new Error('离线状态，无法获取IP信息')
    }

    const shuffledServices = IP_CHECK_SERVICES.toSorted(
      () => Math.random() - 0.5,
    )
    let lastError: unknown | null = null
    const userAgent = await getUserAgentPromise()
    console.debug(`[IpInfo] 开始IP检测，共 ${IP_CHECK_SERVICES.length} 个服务源（${shuffledServices.slice(0, 3).map(s => new URL(s.url).hostname).join(', ')}...）`)
    console.debug('User-Agent for IP detection:', userAgent)

  for (const service of shuffledServices) {
    debugLog(`尝试IP检测服务: ${service.url}`)

    const timeoutController = new AbortController()
    const timeoutId = setTimeout(() => {
      timeoutController.abort()
    }, service.timeout || config.timeout)

    try {
      return await asyncRetry(
        async (bail) => {
          console.debug('Fetching IP information:', service.url)

          const response = await fetch(service.url, {
            method: 'GET',
            signal: timeoutController.signal,
            connectTimeout: service.timeout || config.timeout,
            headers: {
              'User-Agent': userAgent,
            },
          })

          if (!response.ok) {
            return bail(
              new Error(
                `IP 检测服务出错，状态码: ${response.status} from ${service.url}`,
              ),
            )
          }

          let data: any
          try {
            data = await response.json()
          } catch {
            return bail(new Error(`无法解析 JSON 响应 from ${service.url}`))
          }

          if (data && data.ip) {
            debugLog(`IP检测成功，使用服务: ${service.url}`)
            const ipInfo = Object.assign(service.mapping(data), {
              // use last fetch success timestamp
              lastFetchTs: Date.now(),
            })
            // 保存到缓存
            setCachedIpInfo(ipInfo)
            return ipInfo
          } else {
            return bail(new Error(`无效的响应格式 from ${service.url}`))
          }
        },
        {
          retries: config.retries,
          minTimeout: config.minTimeout,
          maxTimeout: config.maxTimeout,
          randomize: true,
        },
      )
    } catch (error) {
      debugLog(`IP检测服务失败: ${service.url}`, error)
      lastError = error
    } finally {
      clearTimeout(timeoutId)
    }
  }

  if (lastError) {
    throw new Error(
      `所有IP检测服务都失败: ${extractErrorMessage(lastError) || '未知错误'}`,
    )
  } else {
    throw new Error('没有可用的IP检测服务')
  }
  })
}

/**
 * 清除 IP 信息缓存
 */
export const clearIpInfoCache = clearIpCache
