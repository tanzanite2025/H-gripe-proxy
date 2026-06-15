import { useQuery, useQueryClient } from '@tanstack/react-query'
import { useEffect } from 'react'

import { useTrafficMonitorEnhanced } from '@/hooks/network'
import { subscribeMetrics } from '@/services/connection-metrics'

const FALLBACK_TRAFFIC: ITrafficItem = { up: 0, down: 0 }
const QUERY_KEY = 'rust-traffic-speed'

export const useTrafficData = (options?: { enabled?: boolean }) => {
  const enabled = options?.enabled ?? true

  const {
    graphData: { appendData },
  } = useTrafficMonitorEnhanced({ subscribe: false, enabled })

  const queryClient = useQueryClient()

  useEffect(() => {
    if (!enabled) return

    return subscribeMetrics((payload) => {
      const traffic: ITrafficItem = {
        up: payload.metrics.traffic.uploadSpeed,
        down: payload.metrics.traffic.downloadSpeed,
      }
      queryClient.setQueryData<ITrafficItem>([QUERY_KEY], traffic)
      appendData(traffic)
    })
  }, [queryClient, enabled, appendData])

  const response = useQuery<ITrafficItem>({
    queryKey: [QUERY_KEY],
    queryFn: () => FALLBACK_TRAFFIC,
    staleTime: Infinity,
    refetchOnWindowFocus: false,
  })

  const refresh = () => {
    queryClient.setQueryData<ITrafficItem>([QUERY_KEY], FALLBACK_TRAFFIC)
  }

  return { response, refreshGetClashTraffic: refresh }
}
