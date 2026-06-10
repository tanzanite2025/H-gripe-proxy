import { getBaseConfig, patchBaseConfig, updateGeo } from 'tauri-plugin-mihomo-api'

import { getGeoDataUpdateTime } from '@/services/cmds'

export const GEO_UPDATE_INTERVAL_OPTIONS = [
  6,
  12,
  24,
  48,
  72,
  168,
] as const

const getLatestGeoUpdateTimestamp = (value: {
  mmdb: number | null
  geoip: number | null
  asn: number | null
  city: number | null
  geosite: number | null
}) =>
  [value.mmdb, value.geoip, value.asn, value.city, value.geosite]
    .filter((item): item is number => item != null)
    .sort((left, right) => right - left)[0] ?? null

export const formatGeoLastUpdateLabel = (timestamp: number | null) => {
  if (!timestamp) {
    return ''
  }

  const diff = Date.now() - timestamp
  const hours = Math.floor(diff / 3600000)
  const days = Math.floor(hours / 24)

  if (days > 0) {
    return `${days} 天前`
  }
  if (hours > 0) {
    return `${hours} 小时前`
  }

  return '刚刚'
}

export async function loadGeoSettings() {
  const [baseConfig, updateTime] = await Promise.all([
    getBaseConfig(),
    getGeoDataUpdateTime(),
  ])

  return {
    autoUpdate: baseConfig.geoAutoUpdate,
    interval: baseConfig.geoUpdateInterval,
    lastUpdateLabel: formatGeoLastUpdateLabel(
      getLatestGeoUpdateTimestamp(updateTime),
    ),
  }
}

export async function triggerGeoUpdate() {
  await updateGeo()
  const updateTime = await getGeoDataUpdateTime()

  return (
    formatGeoLastUpdateLabel(getLatestGeoUpdateTimestamp(updateTime)) || '刚刚'
  )
}

export async function saveGeoAutoUpdate(enabled: boolean) {
  await patchBaseConfig({ 'geo-auto-update': enabled })
}

export async function saveGeoUpdateInterval(hours: number) {
  await patchBaseConfig({ 'geo-update-interval': hours })
}
