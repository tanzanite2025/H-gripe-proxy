import type { CalculatedProxies } from '@/services/proxy-runtime'
import {
  getPreferredProxyGroupName,
  isProxyGroupItem,
} from '@/services/proxy-display'

import type { ProxyGroup } from './types'

interface ResolveSelectionGroupOptions {
  mode: string
  proxiesData: CalculatedProxies
  rules?: any[]
}

export const resolveSelectionGroup = ({
  mode,
  proxiesData,
  rules,
}: ResolveSelectionGroupOptions): ProxyGroup | null => {
  const selectionGroupName = getPreferredProxyGroupName({
    proxies: proxiesData,
    rules,
    isGlobalMode: mode === 'global',
  })
  const selectionGroupRecord =
    selectionGroupName === 'GLOBAL'
      ? proxiesData.global
      : proxiesData.records?.[selectionGroupName]

  if (isProxyGroupItem(selectionGroupRecord)) {
    return selectionGroupRecord
  }

  return proxiesData.global || null
}
