import { useMemo } from 'react'

import { useCurrentEgressIdentity } from '@/hooks/data'
import { useProxiesData } from '@/providers/app-data-context'

import type { IRenderItem } from './render-list/types'
import { buildRuntimeSummaryItem } from './runtime-summary/build-runtime-summary-item'

export const useRuntimeSummaryItem = (): IRenderItem | null => {
  const { proxies: proxiesData } = useProxiesData()
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
        proxiesData,
      }),
    [currentIdentity, proxiesData],
  )
}
