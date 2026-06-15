import { useQuery, useQueryClient } from '@tanstack/react-query'
import { useEffect } from 'react'

import { subscribeMetrics } from '@/services/connection-metrics'

export interface IMemoryUsageItem {
  inuse: number
  oslimit?: number
}

const FALLBACK_MEMORY_USAGE: IMemoryUsageItem = { inuse: 0 }
const QUERY_KEY = 'rust-memory-usage'

export const useMemoryData = () => {
  const queryClient = useQueryClient()

  useEffect(() => {
    return subscribeMetrics((payload) => {
      queryClient.setQueryData<IMemoryUsageItem>([QUERY_KEY], {
        inuse: payload.metrics.traffic.memory,
      })
    })
  }, [queryClient])

  const response = useQuery<IMemoryUsageItem>({
    queryKey: [QUERY_KEY],
    queryFn: () => FALLBACK_MEMORY_USAGE,
    staleTime: Infinity,
    refetchOnWindowFocus: false,
  })

  const refresh = () => {
    queryClient.setQueryData<IMemoryUsageItem>(
      [QUERY_KEY],
      FALLBACK_MEMORY_USAGE,
    )
  }

  return { response, refreshGetClashMemory: refresh }
}
