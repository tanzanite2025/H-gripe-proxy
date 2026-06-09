import { GLOBAL_SELECTOR_SECTION_COPY } from './section-copy'
import {
  buildManualSectionProxies,
  buildStrategySectionProxies,
} from './section-proxies'
import type { ProxyGroup, ProxyItem, VisibleSectionKind } from './types'
import type { HeadState } from '../use-head-state'

type ProxySection = {
  kind: VisibleSectionKind
  title: string
  description?: string
  proxies: ProxyItem[]
}

interface BuildGlobalSelectorSectionsOptions {
  headState: HeadState
  latencyTimeout: number | undefined
  managedStrategyGroupNames: string[]
  proxies: ProxyItem[]
  records?: Record<string, IProxyItem>
  selectionGroup: ProxyGroup
  strategyGroupOverrides: Record<string, string[]>
}

export const buildGlobalSelectorSections = ({
  headState,
  latencyTimeout,
  managedStrategyGroupNames,
  proxies,
  records,
  selectionGroup,
  strategyGroupOverrides,
}: BuildGlobalSelectorSectionsOptions): ProxySection[] => {
  const sections: ProxySection[] = [
    {
      kind: 'manual',
      title: GLOBAL_SELECTOR_SECTION_COPY.manual.title,
      description: GLOBAL_SELECTOR_SECTION_COPY.manual.description,
      proxies: buildManualSectionProxies({
        headState,
        latencyTimeout,
        proxies,
        selectionGroupName: selectionGroup.name,
      }),
    },
    {
      kind: 'strategy',
      title: GLOBAL_SELECTOR_SECTION_COPY.strategy.title,
      description: GLOBAL_SELECTOR_SECTION_COPY.strategy.description,
      proxies: buildStrategySectionProxies({
        headState,
        latencyTimeout,
        managedStrategyGroupNames,
        records,
        selectionGroupName: selectionGroup.name,
        strategyGroupOverrides,
      }),
    },
  ]

  return sections.filter((section) => section.proxies.length > 0)
}
