/**
 * IP 信息缓存服务
 * 缓存 IP 检测结果，减少不必要的网络请求
 */

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
  lastFetchTs: number
}

interface CachedIpInfo {
  data: IpInfo
  timestamp: number
}

const IP_CACHE_KEY = 'clash-verge-ip-info'
const CACHE_TTL = 30 * 60 * 1000 // 30分钟

const isString = (value: unknown): value is string => typeof value === 'string'
const isNumber = (value: unknown): value is number =>
  typeof value === 'number' && Number.isFinite(value)

const isValidIpInfo = (value: unknown): value is IpInfo => {
  if (!value || typeof value !== 'object') return false

  const data = value as Partial<IpInfo>
  return (
    isString(data.ip) &&
    data.ip.trim().length > 0 &&
    isString(data.country_code) &&
    isString(data.country) &&
    isString(data.region) &&
    isString(data.city) &&
    isString(data.organization) &&
    isNumber(data.asn) &&
    isString(data.asn_organization) &&
    isNumber(data.longitude) &&
    isNumber(data.latitude) &&
    isString(data.timezone) &&
    isNumber(data.lastFetchTs)
  )
}

/**
 * 获取缓存的 IP 信息
 */
export const getCachedIpInfo = (): IpInfo | null => {
  try {
    const cached = localStorage.getItem(IP_CACHE_KEY)
    if (!cached) {
      console.debug('[IpCache] 缓存未命中')
      return null
    }

    const { data, timestamp }: CachedIpInfo = JSON.parse(cached)

    if (!isValidIpInfo(data) || !isNumber(timestamp)) {
      console.debug('[IpCache] 缓存格式无效，已清除')
      localStorage.removeItem(IP_CACHE_KEY)
      return null
    }

    // 检查是否过期
    const age = Date.now() - timestamp
    if (age > CACHE_TTL) {
      console.debug(`[IpCache] 缓存已过期 (${Math.round(age / 1000)}秒)`)
      localStorage.removeItem(IP_CACHE_KEY)
      return null
    }

    console.debug(
      `[IpCache] 缓存命中，剩余有效期: ${Math.round((CACHE_TTL - age) / 1000)}秒`,
    )
    return data
  } catch (error) {
    console.error('[IpCache] 读取缓存失败', error)
    return null
  }
}

/**
 * 保存 IP 信息到缓存
 */
export const setCachedIpInfo = (data: IpInfo): void => {
  try {
    const cached: CachedIpInfo = {
      data,
      timestamp: Date.now(),
    }
    localStorage.setItem(IP_CACHE_KEY, JSON.stringify(cached))
    console.debug('[IpCache] IP 信息已缓存')
  } catch (error) {
    console.error('[IpCache] 保存缓存失败', error)
  }
}

/**
 * 清除 IP 信息缓存
 */
export const clearIpCache = (): void => {
  try {
    localStorage.removeItem(IP_CACHE_KEY)
    console.debug('[IpCache] 缓存已清除')
  } catch (error) {
    console.error('[IpCache] 清除缓存失败', error)
  }
}

/**
 * 获取缓存的年龄（毫秒）
 */
export const getCacheAge = (): number | null => {
  try {
    const cached = localStorage.getItem(IP_CACHE_KEY)
    if (!cached) return null

    const { timestamp }: CachedIpInfo = JSON.parse(cached)
    return Date.now() - timestamp
  } catch {
    return null
  }
}
