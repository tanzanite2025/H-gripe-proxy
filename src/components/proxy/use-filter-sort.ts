import { useCallback, useEffect, useMemo, useReducer, useRef } from 'react'

import { useVerge } from '@/hooks/system'

import { filterProxies } from './filter-sort/filter-proxies'
import { sortProxies } from './filter-sort/sort-proxies'
import { useGroupDelayRefresh } from './filter-sort/use-group-delay-refresh'
import type { ProxySearchState, ProxySortType } from './filter-sort/types'

export type { ProxySearchState, ProxySortType }

export default function useFilterSort(
  proxies: IProxyItem[],
  groupName: string,
  filterText: string,
  sortType: ProxySortType,
  searchState?: ProxySearchState,
) {
  const { verge } = useVerge()
  const [_, bumpRefresh] = useReducer((count: number) => count + 1, 0)
  const lastInputRef = useRef<{ text: string; sort: ProxySortType } | null>(
    null,
  )
  const debounceTimerRef = useRef<number | null>(null)

  const handleGroupRefresh = useCallback(() => {
    bumpRefresh()
  }, [])

  useGroupDelayRefresh(groupName, handleGroupRefresh)

  const compute = useMemo(() => {
    const filtered = filterProxies(proxies, groupName, filterText, searchState)
    return sortProxies(
      filtered,
      groupName,
      sortType,
      verge?.default_latency_timeout,
    )
  }, [
    proxies,
    groupName,
    filterText,
    sortType,
    searchState,
    verge?.default_latency_timeout,
  ])

  const [result, setResult] = useReducer(
    (_prev: IProxyItem[], next: IProxyItem[]) => next,
    compute,
  )

  useEffect(() => {
    if (debounceTimerRef.current !== null) {
      window.clearTimeout(debounceTimerRef.current)
      debounceTimerRef.current = null
    }

    const previousInput = lastInputRef.current
    const stableInputs =
      previousInput &&
      previousInput.text === filterText &&
      previousInput.sort === sortType

    lastInputRef.current = { text: filterText, sort: sortType }

    const delay = stableInputs ? 0 : 150
    debounceTimerRef.current = window.setTimeout(() => {
      setResult(compute)
      debounceTimerRef.current = null
    }, delay)

    return () => {
      if (debounceTimerRef.current !== null) {
        window.clearTimeout(debounceTimerRef.current)
        debounceTimerRef.current = null
      }
    }
  }, [compute, filterText, sortType])

  return result
}

export function filterSort(
  proxies: IProxyItem[],
  groupName: string,
  filterText: string,
  sortType: ProxySortType,
  latencyTimeout?: number,
  searchState?: ProxySearchState,
) {
  const filtered = filterProxies(proxies, groupName, filterText, searchState)
  return sortProxies(filtered, groupName, sortType, latencyTimeout)
}
