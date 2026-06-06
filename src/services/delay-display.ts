export interface DelayDisplayUpdate {
  delay: number
  elapsed?: number
  updatedAt: number
}

export interface DelayHistoryEntry {
  delay?: number | null
  time?: string | null
}

export interface DelayDisplayProxy {
  history?: DelayHistoryEntry[] | null
}

const ERROR_DELAY_THRESHOLD = 1e5

export function isSuccessfulDelay(delay: number) {
  return Number.isFinite(delay) && delay > 0 && delay < ERROR_DELAY_THRESHOLD
}

function isFailedDelay(delay: number) {
  return delay === 0 || delay > ERROR_DELAY_THRESHOLD
}

export function getLatestHistoryDelayUpdate(
  proxy: DelayDisplayProxy | null | undefined,
): DelayDisplayUpdate | undefined {
  const history = proxy?.history
  if (!history?.length) return undefined

  const latest = history[history.length - 1]
  const delay = latest?.delay
  if (typeof delay !== 'number' || !Number.isFinite(delay)) {
    return undefined
  }

  const parsedTime =
    typeof latest?.time === 'string' ? Date.parse(latest.time) : Number.NaN

  return {
    delay,
    updatedAt: Number.isNaN(parsedTime) ? 0 : parsedTime,
  }
}

export function resolveDisplayDelayUpdate(
  cached: DelayDisplayUpdate | undefined,
  proxy: DelayDisplayProxy | null | undefined,
): DelayDisplayUpdate | undefined {
  const history = getLatestHistoryDelayUpdate(proxy)

  if (cached?.delay === -2) return cached
  if (cached && isSuccessfulDelay(cached.delay)) return cached

  if (
    history &&
    isSuccessfulDelay(history.delay) &&
    (!cached || isFailedDelay(cached.delay))
  ) {
    return history
  }

  return cached ?? history
}
