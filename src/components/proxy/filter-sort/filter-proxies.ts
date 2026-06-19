import delayManager from '@/services/delay'
import type { IProxyItem } from '@/types/proxy'
import { compileStringMatcher } from '@/utils/validation'

import type { ProxySearchState } from './types'

const DELAY_FILTER_REGEX = /delay([=<>])(\d+|timeout|error)/i
const TYPE_FILTER_REGEX = /type=(.*)/i

export function filterProxies(
  proxies: IProxyItem[],
  groupName: string,
  filterText: string,
  searchState?: ProxySearchState,
) {
  const query = filterText.trim()
  if (!query) return proxies

  const delayFilter = DELAY_FILTER_REGEX.exec(query)
  if (delayFilter) {
    const symbol = delayFilter[1]
    const rawValue = delayFilter[2].toLowerCase()
    const value =
      rawValue === 'error' ? 1e5 : rawValue === 'timeout' ? 3000 : +rawValue

    return proxies.filter((proxy) => {
      const delay = delayManager.getDelayFix(proxy, groupName)

      if (delay < 0) return false
      if (symbol === '=' && rawValue === 'error') return delay >= 1e5
      if (symbol === '=' && rawValue === 'timeout') {
        return delay < 1e5 && delay >= 3000
      }
      if (symbol === '=') return delay == value
      if (symbol === '<') return delay <= value
      if (symbol === '>') return delay >= value
      return false
    })
  }

  const typeFilter = TYPE_FILTER_REGEX.exec(query)
  if (typeFilter) {
    const type = typeFilter[1].toLowerCase()
    return proxies.filter((proxy) => proxy.type.toLowerCase().includes(type))
  }

  const {
    matchCase = false,
    matchWholeWord = false,
    useRegularExpression = false,
  } = searchState ?? {}
  const compiled = compileStringMatcher(query, {
    matchCase,
    matchWholeWord,
    useRegularExpression,
  })

  if (!compiled.isValid) return []
  return proxies.filter((proxy) => compiled.matcher(proxy.name))
}
