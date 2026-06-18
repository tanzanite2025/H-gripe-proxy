import dayjs from 'dayjs'

import type { ProxyProvider } from '@/types/mihomo'

export const parseExpire = (expire?: number) => {
  if (!expire) return '-'
  return dayjs(expire * 1000).format('YYYY-MM-DD')
}

export const getProviderProgress = (
  provider: ProxyProvider,
) => {
  const sub = provider.subscriptionInfo
  const upload = sub?.Upload || 0
  const download = sub?.Download || 0
  const total = sub?.Total || 0
  const expire = sub?.Expire || 0
  const progress =
    total > 0
      ? Math.min(Math.round(((download + upload) * 100) / total) + 1, 100)
      : 0

  return {
    sub,
    hasSubInfo: !!sub,
    upload,
    download,
    total,
    expire,
    progress,
  }
}

export const buildUpdatingMap = (keys: string[]) =>
  keys.reduce(
    (acc, key) => {
      acc[key] = true
      return acc
    },
    {} as Record<string, boolean>,
  )
