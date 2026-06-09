import { categorizeProxyGroup, isProxyGroupItem } from '@/services/proxy-display'

import { filterSort } from '../use-filter-sort'
import type { HeadState } from '../use-head-state'
import type { ProxyGroup, ProxyItem } from './types'

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

const buildStrategyPathText = (group: ProxyGroup, memberCount: number) => {
  if (memberCount <= 0) {
    return '自动策略 · 未配置成员'
  }

  return `自动策略 · 当前 ${group.now || '未观测'} · 手动成员 ${memberCount} 个`
}

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

export const buildStrategySectionProxies = ({
  headState,
  latencyTimeout,
  managedStrategyGroupNames,
  records,
  selectionGroupName,
  strategyGroupOverrides,
}: {
  headState: HeadState
  latencyTimeout: number | undefined
  managedStrategyGroupNames: string[]
  records?: Record<string, IProxyItem>
  selectionGroupName: string
  strategyGroupOverrides: Record<string, string[]>
}) => {
  const strategyGroups = dedupeByName(
    managedStrategyGroupNames
      .map(
        (name) =>
          records?.[normalizeName(name)] as
            | IProxyItem
            | IProxyGroupItem
            | undefined,
      )
      .filter(
        (proxy): proxy is ProxyGroup =>
          isProxyGroupItem(proxy) && categorizeProxyGroup(proxy) === 'strategy',
      ),
  )

  const filtered = filterSort(
    strategyGroups.map((group) => group as unknown as ProxyItem),
    selectionGroupName,
    headState.filterText,
    headState.sortType,
    latencyTimeout,
    buildFilterState(headState),
  ) as ProxyItem[]

  return filtered.map((proxy) => ({
    ...proxy,
    now: buildStrategyPathText(
      proxy as unknown as ProxyGroup,
      strategyGroupOverrides[proxy.name]?.length ?? 0,
    ),
  }))
}
