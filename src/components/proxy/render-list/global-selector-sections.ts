import type { HeadState } from '../use-head-state'

import { GLOBAL_SELECTOR_SECTION_COPY } from './section-copy'
import { buildManualSectionProxies } from './section-proxies'
import type { ProxyGroup, ProxyItem, VisibleSectionKind } from './types'

type ProxySection = {
  kind: VisibleSectionKind
  title: string
  description?: string
  proxies: ProxyItem[]
}

interface BuildGlobalSelectorSectionsOptions {
  headState: HeadState
  latencyTimeout: number | undefined
  proxies: ProxyItem[]
  selectionGroup: ProxyGroup
}

export const buildGlobalSelectorSections = ({
  headState,
  latencyTimeout,
  proxies,
  selectionGroup,
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
  ]

  return sections.filter((section) => section.proxies.length > 0)
}
