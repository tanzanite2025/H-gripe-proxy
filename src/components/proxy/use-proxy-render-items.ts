import { useMemo } from 'react'

import {
  useProxiesData,
  useRulesData,
} from '@/providers/app-data-context'
import {
  categorizeProxyGroup,
  getPreferredProxyGroupName,
  isProxyGroupItem,
} from '@/services/proxy-display'

import {
  buildProxyItems,
  GLOBAL_SELECTOR_SECTION_COPY,
  type IRenderItem,
  type ProxyGroup,
  type ProxyItem,
} from './render-list-shared'
import { filterSort } from './use-filter-sort'
import { DEFAULT_STATE, type HeadState } from './use-head-state'

interface UseProxyRenderItemsOptions {
  mode: string
  headStates: Record<string, HeadState>
  col: number
  latencyTimeout: number | undefined
  runtimeSummaryItem: IRenderItem | null
  strategyGroupOverrides: Record<string, string[]>
  managedStrategyGroupNames: string[]
}

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

const buildStrategyPathText = (group: ProxyGroup, memberCount: number) => {
  if (memberCount <= 0) {
    return '自动策略 · 未配置成员'
  }

  return `自动策略 · 当前 ${group.now || '未观测'} · 手动成员 ${memberCount} 个`
}

export const useProxyRenderItems = ({
  mode,
  headStates,
  col,
  latencyTimeout,
  runtimeSummaryItem,
  strategyGroupOverrides,
  managedStrategyGroupNames,
}: UseProxyRenderItemsOptions): IRenderItem[] => {
  const { proxies: proxiesData } = useProxiesData()
  const { rules } = useRulesData()

  return useMemo(() => {
    if (!proxiesData) {
      return []
    }

    const selectionGroupName = getPreferredProxyGroupName({
      proxies: proxiesData,
      rules,
      isGlobalMode: mode === 'global',
    })
    const selectionGroupRecord =
      selectionGroupName === 'GLOBAL'
        ? proxiesData.global
        : proxiesData.records?.[selectionGroupName]
    const selectionGroup = isProxyGroupItem(selectionGroupRecord)
      ? selectionGroupRecord
      : proxiesData.global

    if (!selectionGroup) {
      return runtimeSummaryItem ? [runtimeSummaryItem] : []
    }

    const headState = headStates[selectionGroup.name] || DEFAULT_STATE
    const filterState = {
      matchCase: headState.filterMatchCase,
      matchWholeWord: headState.filterMatchWholeWord,
      useRegularExpression: headState.filterUseRegularExpression,
    }

    const manualProxies = filterSort(
      dedupeByName(proxiesData.proxies || []),
      selectionGroup.name,
      headState.filterText,
      headState.sortType,
      latencyTimeout,
      filterState,
    ) as ProxyItem[]

    const strategyProxies = filterSort(
      dedupeByName(
        managedStrategyGroupNames
          .map((name) => proxiesData.records?.[normalizeName(name)])
          .filter(
            (proxy): proxy is ProxyGroup =>
              isProxyGroupItem(proxy) &&
              categorizeProxyGroup(proxy) === 'strategy',
          )
          .map((proxy) => proxy as unknown as ProxyItem),
      ),
      selectionGroup.name,
      headState.filterText,
      headState.sortType,
      latencyTimeout,
      filterState,
    ) as ProxyItem[]

    const items: IRenderItem[] = runtimeSummaryItem ? [runtimeSummaryItem] : []

    items.push({
      type: 1,
      key: `head-${selectionGroup.name}`,
      group: selectionGroup,
      headState,
    })

    const sections = [
      {
        kind: 'manual' as const,
        copy: GLOBAL_SELECTOR_SECTION_COPY.manual,
        proxies: manualProxies,
      },
      {
        kind: 'strategy' as const,
        copy: GLOBAL_SELECTOR_SECTION_COPY.strategy,
        proxies: strategyProxies,
      },
    ].filter((section) => section.proxies.length > 0)

    if (!sections.length) {
      items.push({
        type: 3,
        key: `empty-${selectionGroup.name}`,
        group: selectionGroup,
        headState,
      })
      return items
    }

    sections.forEach((section) => {
      items.push({
        type: 5,
        key: `section-${selectionGroup.name}-${section.kind}`,
        group: selectionGroup,
        sectionKind: section.kind,
        sectionTitle: section.copy.title,
        sectionDescription: section.copy.description,
      })

      items.push(
        ...buildProxyItems(
          selectionGroup,
          section.kind === 'strategy'
            ? section.proxies.map((proxy) => ({
                ...proxy,
                now: buildStrategyPathText(
                  proxy as unknown as ProxyGroup,
                  strategyGroupOverrides[proxy.name]?.length ?? 0,
                ),
              }))
            : section.proxies,
          col,
          section.kind,
          headState,
        ),
      )
    })

    return items
  }, [
    col,
    headStates,
    latencyTimeout,
    managedStrategyGroupNames,
    mode,
    proxiesData,
    rules,
    runtimeSummaryItem,
    strategyGroupOverrides,
  ])
}
