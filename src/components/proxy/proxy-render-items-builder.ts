import type { CalculatedProxies } from '@/services/proxy-runtime'
import type { IProxyGroupItem, IProxyItem } from '@/types/proxy'

import { buildGlobalSelectorSections } from './render-list/global-selector-sections'
import { resolveSelectionGroup } from './render-list/selection-group'
import type { IRenderItem } from './render-list/types'
import { buildProxyItems } from './render-list/utils'
import { DEFAULT_STATE, type HeadState } from './use-head-state'

export interface ProxyRenderListBuilderOptions {
  col: number
  headStates: Record<string, HeadState>
  latencyTimeout: number | undefined
  proxiesData?: CalculatedProxies
  runtimeSummaryItem: IRenderItem | null
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
  proxiesData,
  runtimeSummaryItem,
}: ProxyRenderListBuilderOptions): IRenderItem[] => {
  if (!proxiesData) {
    return []
  }

  const selectionGroup = resolveSelectionGroup({ proxiesData })
  if (!selectionGroup) {
    return runtimeSummaryItem ? [runtimeSummaryItem] : []
  }

  const headState = headStates[selectionGroup.name] || DEFAULT_STATE
  const items = buildBaseItems(runtimeSummaryItem, selectionGroup, headState)
  const sections = buildGlobalSelectorSections({
    headState,
    latencyTimeout,
    proxies: (proxiesData.proxies || []) as IProxyItem[],
    selectionGroup,
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
