import { filterSort } from '../use-filter-sort'
import type { HeadState } from '../use-head-state'
import type { ProxyItem } from './types'

const normalizeName = (value?: string | null) => value?.trim() || ''

const dedupeByName = <T extends { name?: string }>(items: T[]) => {
  const seen = new Set<string>()

  return items.filter((item) => {
    const name = normalizeName(item?.name)
    if (!name || seen.has(name)) {
      return false
    }

    seen.add(name)
    return true
  })
}

const buildFilterState = (headState: HeadState) => ({
  matchCase: headState.filterMatchCase,
  matchWholeWord: headState.filterMatchWholeWord,
  useRegularExpression: headState.filterUseRegularExpression,
})

export const buildManualSectionProxies = ({
  headState,
  latencyTimeout,
  proxies,
  selectionGroupName,
}: {
  headState: HeadState
  latencyTimeout: number | undefined
  proxies: ProxyItem[]
  selectionGroupName: string
}) => {
  return filterSort(
    dedupeByName(proxies),
    selectionGroupName,
    headState.filterText,
    headState.sortType,
    latencyTimeout,
    buildFilterState(headState),
  ) as ProxyItem[]
}
