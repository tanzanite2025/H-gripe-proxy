import type { CalculatedProxies } from '@/services/proxy-runtime'

import { buildProxyItems } from './render-list/utils'
import { buildGlobalSelectorSections } from './render-list/global-selector-sections'
import { resolveSelectionGroup } from './render-list/selection-group'
import type { IRenderItem } from './render-list/types'
import { DEFAULT_STATE, type HeadState } from './use-head-state'

export interface ProxyRenderListBuilderOptions {
  col: number
  headStates: Record<string, HeadState>
  latencyTimeout: number | undefined
  managedStrategyGroupNames: string[]
  mode: string
  proxiesData?: CalculatedProxies
  rules?: any[]
  runtimeSummaryItem: IRenderItem | null
  strategyGroupOverrides: Record<string, string[]>
}

const buildBaseItems = (
  runtimeSummaryItem: IRenderItem | null,
  selectionGroup: IProxyGroupItem,
  headState: HeadState,
) => {
  const items: IRenderItem[] = runtimeSummaryItem ? [runtimeSummaryItem] : []

  items.push({
    type: 1,
    key: `head-${selectionGroup.name}`,
    group: selectionGroup,
    headState,
  })

  return items
}

export const buildProxyRenderList = ({
  col,
  headStates,
  latencyTimeout,
  managedStrategyGroupNames,
  mode,
  proxiesData,
  rules,
  runtimeSummaryItem,
  strategyGroupOverrides,
}: ProxyRenderListBuilderOptions): IRenderItem[] => {
  if (!proxiesData) {
    return []
  }

  const selectionGroup = resolveSelectionGroup({
    mode,
    proxiesData,
    rules,
  })
  if (!selectionGroup) {
    return runtimeSummaryItem ? [runtimeSummaryItem] : []
  }

  const headState = headStates[selectionGroup.name] || DEFAULT_STATE
  const items = buildBaseItems(runtimeSummaryItem, selectionGroup, headState)
  const sections = buildGlobalSelectorSections({
    headState,
    latencyTimeout,
    managedStrategyGroupNames,
    proxies: (proxiesData.proxies || []) as IProxyItem[],
    records: proxiesData.records,
    selectionGroup,
    strategyGroupOverrides,
  })

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
      sectionTitle: section.title,
      sectionDescription: section.description,
    })

    items.push(
      ...buildProxyItems(
        selectionGroup,
        section.proxies,
        col,
        section.kind,
        headState,
      ),
    )
  })

  return items
}
