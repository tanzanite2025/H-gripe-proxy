import { debugLog } from '@/utils/misc'

import { getCurrentEgressIdentity } from './cmds'
import { getCachedIpInfo, setCachedIpInfo, clearIpCache } from './ip-cache'
import { networkMonitor } from './network-monitor'
import { deduplicator } from './request-deduplicator'

export interface IpInfo {
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

const hasUsableIp = (value: unknown): value is string =>
  typeof value === 'string' && value.trim().length > 0

const parseAsnNumber = (value?: string | null): number => {
  if (!value) return 0
  const normalized = value.trim().toUpperCase().replace(/^AS/, '')
  const parsed = Number(normalized)
  return Number.isFinite(parsed) ? parsed : 0
}

const selectDisplayIp = (...candidates: Array<string | null | undefined>) => {
  const validCandidates = candidates.filter((candidate): candidate is string =>
    hasUsableIp(candidate),
  )

  return validCandidates.find((candidate) => !candidate.includes(':')) ?? validCandidates[0] ?? ''
}

const mapBackendIpInfo = (
  data: Awaited<ReturnType<typeof getCurrentEgressIdentity>>,
): IpInfo & { lastFetchTs: number } => ({
  ip: selectDisplayIp(data.public_egress_ip, data.egress_ip),
  country_code: data.country_code || data.reputation?.countryCode || '',
  country: data.country_code || data.reputation?.countryCode || '',
  region: '',
  city: data.reputation?.city || '',
  organization: data.asn_org || data.reputation?.asnOrg || '',
  asn: parseAsnNumber(data.destination_asn || data.reputation?.asn),
  asn_organization: data.asn_org || data.reputation?.asnOrg || '',
  longitude: 0,
  latitude: 0,
  timezone: data.timezone || data.reputation?.timezone || '',
  lastFetchTs: Date.now(),
})

export const getIpInfo = async (): Promise<IpInfo & { lastFetchTs: number }> =>
  deduplicator.dedupe('ip-info', async () => {
    const cached = getCachedIpInfo()
    if (cached && hasUsableIp(cached.ip)) {
      debugLog('[IpInfo] using cached backend observation')
      return cached
    }

    if (!networkMonitor.isOnline()) {
      throw new Error('网络已断开，无法获取出口 IP 信息')
    }

    const identity = await getCurrentEgressIdentity()
    const ipInfo = mapBackendIpInfo(identity)
    if (!hasUsableIp(ipInfo.ip)) {
      throw new Error(identity.message || '内核未返回有效的出口 IP 信息')
    }

    debugLog('[IpInfo] using current egress identity only')
    setCachedIpInfo(ipInfo)
    return ipInfo
  })

export const clearIpInfoCache = clearIpCache
