import { useMemo } from 'react'

import { useCurrentEgressIdentity } from '@/hooks/data'
import {
  useProxiesData,
  useRulesData,
} from '@/providers/app-data-context'

import { buildRuntimeSummaryItem } from './runtime-summary/build-runtime-summary-item'
import type { IRenderItem } from './render-list/types'

export const useRuntimeSummaryItem = (mode: string): IRenderItem | null => {
  const { proxies: proxiesData } = useProxiesData()
  const { rules } = useRulesData()
  const { data: currentIdentity } = useCurrentEgressIdentity({
    staleTime: 5_000,
    refetchOnWindowFocus: false,
    refetchOnReconnect: true,
    refetchInterval: 5_000,
    retry: 1,
  })

  return useMemo(
    () =>
      buildRuntimeSummaryItem({
        currentIdentity,
        mode,
        proxiesData,
        rules,
      }),
    [currentIdentity, mode, proxiesData, rules],
  )
}
