import type { CalculatedProxies } from '@/services/proxy-runtime'
import {
  getPreferredProxyGroupName,
  isProxyGroupItem,
} from '@/services/proxy-display'

import type { ProxyGroup } from './types'

interface ResolveSelectionGroupOptions {
  proxiesData: CalculatedProxies
}

export const resolveSelectionGroup = ({
  proxiesData,
}: ResolveSelectionGroupOptions): ProxyGroup | null => {
  const selectionGroupName = getPreferredProxyGroupName({ proxies: proxiesData })
  const selectionGroupRecord =
    selectionGroupName === 'GLOBAL'
      ? proxiesData.global
      : proxiesData.records?.[selectionGroupName]

  if (isProxyGroupItem(selectionGroupRecord)) {
    return selectionGroupRecord
  }

  return proxiesData.global || null
}
