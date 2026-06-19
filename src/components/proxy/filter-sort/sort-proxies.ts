import delayManager from '@/services/delay'
import { resolveDelayTimeout } from '@/services/delay-config'
import type { IProxyItem } from '@/types/proxy'

import type { ProxySortType } from './types'

const getEffectiveTimeout = (latencyTimeout?: number) =>
  resolveDelayTimeout(latencyTimeout)

const categorizeDelay = (
  delay: number,
  effectiveTimeout: number,
): [number, number] => {
  if (!Number.isFinite(delay)) return [3, Number.MAX_SAFE_INTEGER]
  if (delay > 1e5) return [4, delay]
  if (delay === 0 || (delay >= effectiveTimeout && delay <= 1e5)) {
    return [3, delay || effectiveTimeout]
  }
  if (delay < 0) {
    // Sentinel delays (-1, -2, etc.) should always sort after real measurements.
    return [5, Number.MAX_SAFE_INTEGER]
  }
  return [0, delay]
}

export function sortProxies(
  proxies: IProxyItem[],
  groupName: string,
  sortType: ProxySortType,
  latencyTimeout?: number,
) {
  if (!proxies) return []
  if (sortType === 0) return proxies

  const list = proxies.slice()
  const effectiveTimeout = getEffectiveTimeout(latencyTimeout)

  if (sortType === 1) {
    list.sort((left, right) => {
      const leftDelay = delayManager.getDelayFix(left, groupName)
      const rightDelay = delayManager.getDelayFix(right, groupName)
      const [leftRank, leftValue] = categorizeDelay(leftDelay, effectiveTimeout)
      const [rightRank, rightValue] = categorizeDelay(
        rightDelay,
        effectiveTimeout,
      )

      if (leftRank !== rightRank) return leftRank - rightRank
      return leftValue - rightValue
    })

    return list
  }

  list.sort((left, right) => left.name.localeCompare(right.name))
  return list
}
