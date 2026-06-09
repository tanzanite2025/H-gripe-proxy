import { useMemo, useRef } from 'react'

import { useProxiesData } from '@/providers/app-data-context'
import {
  categorizeProxyGroup,
  getDisplayableTopLevelGroups,
  resolveProxyPath,
  splitProxyGroupTargets,
} from '@/services/proxy-display'

import {
  buildDisplayPath,
  buildProxyItems,
  GLOBAL_SELECTOR_SECTION_COPY,
  GROUP_SECTION_COPY,
  MANUAL_PAGE_SECTION_COPY,
  type GroupCache,
  type IRenderItem,
  type ProxyGroup,
  type ProxyItem,
} from './render-list-shared'
import { filterSort } from './use-filter-sort'
import {
  DEFAULT_STATE,
  type HeadState,
} from './use-head-state'

interface UseProxyRenderItemsOptions {
  mode: string
  headStates: Record<string, HeadState>
  col: number
  latencyTimeout: number | undefined
  runtimeSummaryItem: IRenderItem | null
  strategyGroupOverrides: Record<string, string[]>
}

const toStrategyMemberItems = ({
  group,
  names,
  records,
}: {
  group: ProxyGroup
  names: string[]
  records?: Record<string, IProxyItem>
}): ProxyItem[] =>
  names.map((name) => {
    const record = records?.[name]
    if (record) {
      return record as ProxyItem
    }

    return {
      name,
      type: 'Unknown',
      udp: false,
      xudp: false,
      tfo: false,
      mptcp: false,
      smux: false,
      history: [],
      testUrl: group.testUrl,
    } as ProxyItem
  })

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
}: UseProxyRenderItemsOptions): IRenderItem[] => {
  const { proxies: proxiesData } = useProxiesData()
  const groupCacheRef = useRef<Map<string, GroupCache>>(new Map())

  return useMemo(() => {
    if (!proxiesData) return []

    const useRule = mode === 'rule' || mode === 'script'
    const topLevelGroups = getDisplayableTopLevelGroups({
      groups: proxiesData.groups,
      global: proxiesData.global,
    })
    const strategyGroups = topLevelGroups.filter(
      (group) => categorizeProxyGroup(group) === 'strategy',
    )
    const renderGroups = useRule
      ? topLevelGroups
      : [proxiesData.global!, ...strategyGroups]

    const cache = groupCacheRef.current

    const buildGroupItems = (group: ProxyGroup): IRenderItem[] => {
      const headState = headStates[group.name] || DEFAULT_STATE
      const groupKind = categorizeProxyGroup(group)
      const strategyMembers =
        groupKind === 'strategy' ? strategyGroupOverrides[group.name] ?? [] : []
      const strategyMembersSignature = strategyMembers.join('|')
      const path = resolveProxyPath(proxiesData.records, group.name)
      const pathDisplay = buildDisplayPath(path.path.slice(1), proxiesData.records)
      const pathSignature = path.path.join(' -> ')
      const cached = cache.get(group.name)

      if (
        cached &&
        cached.now === group.now &&
        cached.all === group.all &&
        cached.headState === headState &&
        cached.col === col &&
        cached.latencyTimeout === latencyTimeout &&
        cached.pathSignature === pathSignature &&
        cached.strategyMembersSignature === strategyMembersSignature
      ) {
        return cached.items
      }

      const items: IRenderItem[] = [
        {
          type: 0,
          key: group.name,
          group,
          headState,
          memberCount:
            groupKind === 'strategy' ? strategyMembers.length : undefined,
          pathText:
            groupKind === 'strategy'
              ? buildStrategyPathText(group, strategyMembers.length)
              : pathDisplay.join(' -> ') ||
                buildDisplayPath([group.now], proxiesData.records)[0] ||
                group.now,
        },
      ]

      if (headState.open || !useRule) {
        items.push({
          type: 1,
          key: `head-${group.name}`,
          group,
          headState,
        })

        const split = splitProxyGroupTargets(group)
        const strategyMemberTargets =
          groupKind === 'strategy'
            ? toStrategyMemberItems({
                group,
                names: strategyMembers,
                records: proxiesData.records,
              })
            : []

        const canExposeStrategyTargets =
          groupKind !== 'strategy' && split.strategy.length > 0

        const sections = (
          groupKind === 'strategy'
            ? [
                {
                  kind: 'manual' as const,
                  proxies: strategyMemberTargets,
                },
              ]
            : [
                {
                  kind: 'manual' as const,
                  proxies: split.manual,
                },
                ...(canExposeStrategyTargets
                  ? [
                      {
                        kind: 'strategy' as const,
                        proxies: split.strategy as ProxyItem[],
                      },
                    ]
                  : []),
              ]
        )
          .map((section) => ({
            ...section,
            proxies: filterSort(
              section.proxies,
              group.name,
              headState.filterText,
              headState.sortType,
              latencyTimeout,
              {
                matchCase: headState.filterMatchCase,
                matchWholeWord: headState.filterMatchWholeWord,
                useRegularExpression: headState.filterUseRegularExpression,
              },
            ) as ProxyItem[],
          }))
          .filter((section) => section.proxies.length > 0)

        const shouldShowSectionHeaders =
          groupKind !== 'strategy' && sections.length > 1

        if (!sections.length) {
          items.push({
            type: 3,
            key: `empty-${group.name}`,
            group,
            headState,
          })
        } else {
          sections.forEach((section) => {
            if (shouldShowSectionHeaders) {
              const sectionCopy = GLOBAL_SELECTOR_SECTION_COPY[section.kind]

              items.push({
                type: 5,
                key: `section-${group.name}-${section.kind}`,
                group,
                sectionKind: section.kind,
                sectionTitle: sectionCopy.title,
                sectionDescription: sectionCopy.description,
              })
            }

            items.push(
              ...buildProxyItems(
                group,
                section.proxies,
                col,
                section.kind,
                headState,
              ),
            )
          })
        }
      }

      cache.set(group.name, {
        now: group.now,
        all: group.all,
        headState,
        col,
        latencyTimeout,
        pathSignature,
        strategyMembersSignature,
        items,
      })

      return items
    }

    const body = renderGroups.flatMap(buildGroupItems)
    const retList = runtimeSummaryItem ? [runtimeSummaryItem, ...body] : body

    const withPageSections =
      useRule && renderGroups.length > 1
        ? (() => {
            const next: IRenderItem[] = []
            let insertedManual = false

            retList.forEach((item) => {
              if (!insertedManual && item.type === 0) {
                next.push({
                  type: 5,
                  key: 'page-section-manual',
                  group: item.group,
                  sectionKind: 'manual',
                  sectionTitle: MANUAL_PAGE_SECTION_COPY.title,
                  sectionDescription: MANUAL_PAGE_SECTION_COPY.description,
                })
                insertedManual = true
              }

              next.push(item)
            })

            return next
          })()
        : retList

    return !useRule
      ? withPageSections.filter(
          (item, index) =>
            !(index === (runtimeSummaryItem ? 1 : 0) && item.type === 0),
        )
      : withPageSections
  }, [
    col,
    headStates,
    latencyTimeout,
    mode,
    proxiesData,
    runtimeSummaryItem,
    strategyGroupOverrides,
  ])
}
